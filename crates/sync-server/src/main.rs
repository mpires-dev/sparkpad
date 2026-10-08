use axum::{
    body::Bytes,
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        DefaultBodyLimit, Path as RoutePath, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use sparkpad_sync::{decode, encode, valid_doc, Document, Message, MAX_FRAME, PROTOCOL_VERSION};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use subtle::ConstantTimeEq;
use tokio::sync::broadcast;
use tower_http::services::{ServeDir, ServeFile};

type Failure = (StatusCode, &'static str);
#[derive(Clone)]
struct Server {
    db: Arc<Mutex<Connection>>,
    token_hash: [u8; 32],
    events: broadcast::Sender<Message>,
    assets: PathBuf,
}
impl Server {
    fn authorized(&self, token: &str) -> bool {
        let digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        bool::from(digest.ct_eq(&self.token_hash))
    }
    fn check_headers(&self, headers: &HeaderMap) -> Result<(), Failure> {
        let token = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if self.authorized(token) {
            Ok(())
        } else {
            Err((StatusCode::UNAUTHORIZED, "Authentication required"))
        }
    }
    fn manifest(&self) -> anyhow::Result<Vec<String>> {
        Ok(self
            .db
            .lock()
            .unwrap()
            .prepare("SELECT id FROM documents ORDER BY id")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }
    fn sync(&self, id: &str, vector: &str) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        let state: Option<Vec<u8>> = self
            .db
            .lock()
            .unwrap()
            .query_row("SELECT state FROM documents WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        let doc = Document::load(0, state.as_deref().unwrap_or(&[]))?;
        Ok((doc.delta(&decode(vector)?)?, doc.vector()))
    }
    fn update(&self, id: &str, data: &str) -> anyhow::Result<bool> {
        let incoming = decode(data)?;
        anyhow::ensure!(incoming.len() <= 8 * 1024 * 1024, "Update too large");
        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;
        let old: Option<Vec<u8>> = tx
            .query_row("SELECT state FROM documents WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        let doc = Document::load(0, old.as_deref().unwrap_or(&[]))?;
        doc.apply(&incoming)?;
        let state = doc.state();
        anyhow::ensure!(state.len() <= 32 * 1024 * 1024, "Document too large");
        if old.as_ref() == Some(&state) {
            return Ok(false);
        }
        tx.execute("INSERT INTO documents(id,state) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET state=excluded.state", params![id,state])?;
        tx.commit()?;
        Ok(true)
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"service":"sparkpad","version":PROTOCOL_VERSION,"status":"ok"}))
}
async fn identity(
    State(state): State<Server>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, Failure> {
    state.check_headers(&headers)?;
    Ok(Json(serde_json::json!({"version":PROTOCOL_VERSION})))
}
async fn websocket(
    State(state): State<Server>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    // Browser clients authenticate in their first frame; desktop clients also send a header.
    if headers.contains_key("authorization") && state.check_headers(&headers).is_err() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    ws.max_message_size(MAX_FRAME)
        .max_frame_size(MAX_FRAME)
        .on_upgrade(move |socket| session(socket, state))
}
async fn session(mut socket: WebSocket, state: Server) {
    let auth = tokio::time::timeout(Duration::from_secs(5), socket.recv()).await;
    let allowed = match auth {
        Ok(Some(Ok(WsMessage::Text(text)))) => {
            matches!(serde_json::from_str::<Message>(&text), Ok(Message::Auth {token,version}) if version == PROTOCOL_VERSION && state.authorized(&token))
        }
        _ => false,
    };
    if !allowed {
        let _ = socket.send(WsMessage::Close(None)).await;
        return;
    }
    // Subscribe before taking the manifest so updates during the handshake cannot be missed.
    let mut events = state.events.subscribe();
    let initial = {
        let s = state.clone();
        tokio::task::spawn_blocking(move || s.manifest()).await
    };
    let docs = match initial {
        Ok(Ok(docs)) => docs,
        _ => return,
    };
    let (mut sink, mut stream) = socket.split();
    if send(&mut sink, &Message::Manifest { docs }).await.is_err() {
        return;
    }
    let mut heartbeat = tokio::time::interval(Duration::from_secs(20));
    loop {
        tokio::select! {
            incoming = stream.next() => {
                let message = match incoming {
                    Some(Ok(WsMessage::Text(text))) => match serde_json::from_str::<Message>(&text) { Ok(m) => m, Err(_) => break },
                    Some(Ok(WsMessage::Ping(bytes))) => { if sink.send(WsMessage::Pong(bytes)).await.is_err() { break; } continue; },
                    Some(Ok(WsMessage::Pong(_))) => continue,
                    _ => break,
                };
                match message {
                    Message::Sync {doc,vector} if valid_doc(&doc) && vector.len() <= 128 * 1024 => {
                        let s = state.clone(); let id = doc.clone();
                        match tokio::task::spawn_blocking(move || s.sync(&id, &vector)).await {
                            Ok(Ok((update,vector))) => {
                                if send(&mut sink, &Message::Update {doc:doc.clone(),data:encode(&update)}).await.is_err() { break; }
                                if send(&mut sink, &Message::Sync {doc,vector:encode(&vector)}).await.is_err() { break; }
                            },
                            _ => { let _ = send(&mut sink, &Message::Error {message:"Invalid sync request".into()}).await; break; }
                        }
                    }
                    Message::Update {doc,data} if valid_doc(&doc) && data.len() <= MAX_FRAME * 3 / 4 => {
                        let s = state.clone(); let id = doc.clone(); let payload = data.clone();
                        match tokio::task::spawn_blocking(move || s.update(&id, &payload)).await {
                            Ok(Ok(changed)) => {
                                // Acknowledgments follow the durable SQLite commit.
                                let digest = format!("{:x}", Sha256::digest(data.as_bytes()));
                                if send(&mut sink, &Message::Ack {doc:doc.clone(),digest}).await.is_err() { break; }
                                if changed { let _ = state.events.send(Message::Update {doc,data}); }
                            }
                            _ => { let _ = send(&mut sink, &Message::Error {message:"Invalid document update".into()}).await; break; }
                        }
                    }
                    Message::Ping => { if send(&mut sink, &Message::Pong).await.is_err() {break;} },
                    Message::Pong => {},
                    _ => break,
                }
            }
            update = events.recv() => match update {
                Ok(message) => if send(&mut sink, &message).await.is_err() { break; },
                Err(_) => break, // Slow clients reconnect and recover missing updates by state vector.
            },
            _ = heartbeat.tick() => if sink.send(WsMessage::Ping(Bytes::new())).await.is_err() { break; },
        }
    }
}
async fn send<S>(sink: &mut S, message: &Message) -> Result<(), ()>
where
    S: futures_util::Sink<WsMessage> + Unpin,
{
    sink.send(WsMessage::Text(
        serde_json::to_string(message).map_err(|_| ())?.into(),
    ))
    .await
    .map_err(|_| ())
}
fn asset_path(root: &Path, name: &str) -> Result<PathBuf, Failure> {
    let (digest, ext) = name
        .split_once('.')
        .ok_or((StatusCode::BAD_REQUEST, "Invalid asset"))?;
    if digest.len() != 64
        || !digest.bytes().all(|b| b.is_ascii_hexdigit())
        || !["png", "jpg", "jpeg", "svg", "webp", "gif"].contains(&ext)
    {
        return Err((StatusCode::BAD_REQUEST, "Invalid asset"));
    }
    Ok(root.join(name))
}
async fn upload_asset(
    State(state): State<Server>,
    RoutePath(name): RoutePath<String>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<StatusCode, Failure> {
    state.check_headers(&headers)?;
    let path = asset_path(&state.assets, &name)?;
    if bytes.len() > 10_000_000 {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "Image exceeds 10 MB"));
    }
    if !name.starts_with(&format!("{:x}", Sha256::digest(&bytes))) {
        return Err((StatusCode::BAD_REQUEST, "Asset digest does not match"));
    }
    // Each upload has a separate staging path; rename publishes only a complete file.
    let staging = state.assets.join(format!(
        ".upload-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    tokio::fs::write(&staging, &bytes)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Cannot store asset"))?;
    tokio::fs::rename(&staging, &path)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Cannot store asset"))?;
    Ok(StatusCode::NO_CONTENT)
}
async fn download_asset(
    State(state): State<Server>,
    RoutePath(name): RoutePath<String>,
    headers: HeaderMap,
) -> Result<Response, Failure> {
    state.check_headers(&headers)?;
    let path = asset_path(&state.assets, &name)?;
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Asset not found"))?;
    Ok((
        [
            (
                "content-type",
                match name.rsplit('.').next().unwrap_or("") {
                    "svg" => "image/svg+xml",
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "webp" => "image/webp",
                    _ => "application/octet-stream",
                },
            ),
            ("x-content-type-options", "nosniff"),
            ("cache-control", "private, max-age=31536000, immutable"),
        ],
        bytes,
    )
        .into_response())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().any(|arg| arg == "--healthcheck") {
        use std::io::{Read, Write};
        let address = std::net::SocketAddr::from(([127, 0, 0, 1], 7348));
        let mut stream = std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream
            .write_all(b"GET /health HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
        let mut reply = String::new();
        stream.read_to_string(&mut reply)?;
        anyhow::ensure!(
            reply.starts_with("HTTP/1.0 200") || reply.starts_with("HTTP/1.1 200"),
            "Healthcheck failed"
        );
        return Ok(());
    }
    let token = std::env::var("SPARKPAD_SERVER_TOKEN")
        .expect("Set SPARKPAD_SERVER_TOKEN (at least 32 characters)");
    anyhow::ensure!(
        token.len() >= 32,
        "Server token must have at least 32 characters"
    );
    let root = PathBuf::from(
        std::env::var("SPARKPAD_SERVER_DATA").unwrap_or_else(|_| "./sparkpad-data".into()),
    );
    std::fs::create_dir_all(root.join("assets"))?;
    let db = Connection::open(root.join("sync.sqlite3"))?;
    db.busy_timeout(Duration::from_secs(5))?;
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS documents(id TEXT PRIMARY KEY,state BLOB NOT NULL);")?;
    let (events, _) = broadcast::channel(1024);
    let state = Server {
        db: Arc::new(Mutex::new(db)),
        token_hash: Sha256::digest(token.as_bytes()).into(),
        events,
        assets: root.join("assets"),
    };
    let web = std::env::var("SPARKPAD_SERVER_WEB").unwrap_or_else(|_| "apps/sync-web/dist".into());
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/session", get(identity))
        .route("/sync", get(websocket))
        .route("/api/assets/{name}", get(download_asset).put(upload_asset))
        .layer(DefaultBodyLimit::max(10_000_000))
        .fallback_service(
            ServeDir::new(&web)
                .not_found_service(ServeFile::new(Path::new(&web).join("index.html"))),
        )
        .with_state(state);
    let address = std::env::var("SPARKPAD_SERVER_BIND").unwrap_or_else(|_| "127.0.0.1:7348".into());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!("Sparkpad sync server listening on {address}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn server() -> Server {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE documents(id TEXT PRIMARY KEY,state BLOB NOT NULL)")
            .unwrap();
        let (events, _) = broadcast::channel(8);
        Server {
            db: Arc::new(Mutex::new(db)),
            token_hash: Sha256::digest(b"a-test-key-only").into(),
            events,
            assets: PathBuf::new(),
        }
    }
    #[test]
    fn durable_snapshot_recovers_concurrent_edits_and_rejects_invalid_update() {
        let server = server();
        let id = "00000000-0000-4000-8000-000000000000";
        let seed = Document::new(1);
        seed.set_text("body", "Hello 🌱 world");
        server.update(id, &encode(&seed.state())).unwrap();
        let a = Document::load(2, &seed.state()).unwrap();
        let b = Document::load(3, &seed.state()).unwrap();
        a.set_text("body", "Hello 🌱 brave world");
        b.set_text("body", "Hello 🌱 world!");
        server.update(id, &encode(&a.state())).unwrap();
        server.update(id, &encode(&b.state())).unwrap();
        assert!(server.update(id, "invalid-base64!").is_err());
        let (state, _) = server
            .sync(id, &encode(&Document::new(4).vector()))
            .unwrap();
        assert_eq!(
            Document::load(4, &state).unwrap().text("body"),
            "Hello 🌱 brave world!"
        );
        assert!(!server.update(id, &encode(&b.state())).unwrap());
        assert!(!server.authorized("wrong"));
        assert!(server.authorized("a-test-key-only"));
    }
}
