//! Optional sync worker. Nothing in the editor waits for this thread or the network.
use crate::{store::Store, sync_storage};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sparkpad_sync::{decode, encode, Message, PROTOCOL_VERSION};
use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tungstenite::{
    client::IntoClientRequest, stream::MaybeTlsStream, Error as WsError, Message as WsMessage,
    WebSocket,
};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub url: String,
    pub token: String,
}
pub fn config_path(db: &Path) -> PathBuf {
    db.with_extension("sync.json")
}
pub fn read_config(db: &Path) -> Option<Config> {
    serde_json::from_slice(&std::fs::read(config_path(db)).ok()?).ok()
}
pub fn write_config(db: &Path, config: Option<&Config>) -> Result<()> {
    let path = config_path(db);
    if let Some(config) = config {
        let url = tungstenite::http::Uri::try_from(config.url.as_str())
            .context("Enter a valid server URL")?;
        anyhow::ensure!(
            matches!(url.scheme_str(), Some("http" | "https")) && url.host().is_some(),
            "Use an http:// or https:// server URL"
        );
        anyhow::ensure!(
            config.token.len() >= 32 && config.token.len() <= 512,
            "Enter the server connection key"
        );
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let staging = path.with_extension("sync.json.tmp");
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        use std::io::Write;
        let mut file = options.open(&staging)?;
        file.write_all(&serde_json::to_vec(config)?)?;
        file.sync_all()?;
        std::fs::rename(staging, path)?;
    } else if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
pub fn validate_connection(config: &Config) -> Result<()> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .new_agent();
    let mut response = agent
        .get(format!("{}/api/session", config.url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", config.token))
        .call()
        .context("Cannot connect. Check the server address and connection key.")?;
    let value: serde_json::Value = response.body_mut().read_json()?;
    anyhow::ensure!(
        value.get("version").and_then(|v| v.as_u64()) == Some(PROTOCOL_VERSION as u64),
        "Server protocol version is incompatible"
    );
    Ok(())
}
pub struct Worker {
    stop: Arc<AtomicBool>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
pub fn start(path: &Path) -> Worker {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let path = path.to_owned();
    std::thread::Builder::new()
        .name("sparkpad-sync".into())
        .spawn(move || {
            let lock_path = path.with_extension("sync.lock");
            if let Some(parent) = lock_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let Ok(lock) = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(lock_path)
            else {
                return;
            };
            // GUI and MCP processes share the same database; only one of them performs network sync.
            while !stopped.load(Ordering::Relaxed) {
                if lock.try_lock().is_err() {
                    std::thread::sleep(Duration::from_millis(400));
                    continue;
                }
                run(&path, &stopped);
                let _ = lock.unlock();
            }
        })
        .expect("Start sync worker");
    Worker { stop }
}
fn status(store: &Store, value: &str) {
    if store.setting("sync_status").ok().flatten().as_deref() != Some(value) {
        let _ = store.set_setting("sync_status", value);
    }
}
fn run(path: &Path, stop: &AtomicBool) {
    let Ok(store) = Store::open(path) else {
        std::thread::sleep(Duration::from_secs(1));
        return;
    };
    let mut retry = 1;
    while !stop.load(Ordering::Relaxed) {
        let Some(config) = read_config(path) else {
            let _ = sync_storage::disable(&store);
            status(&store, "local");
            std::thread::sleep(Duration::from_millis(400));
            continue;
        };
        if !sync_storage::enabled(&store) {
            if sync_storage::enable(&store).is_err() {
                continue;
            }
        }
        status(&store, "connecting");
        let result = session(&store, &config, stop);
        if let Err(error) = &result {
            let _ = store.set_setting("sync_last_error", &format!("{error:#}"));
            status(&store, "offline");
        } else {
            retry = 1;
        }
        let until = Instant::now() + Duration::from_secs(retry);
        while Instant::now() < until
            && !stop.load(Ordering::Relaxed)
            && read_config(path).as_ref() == Some(&config)
        {
            // Keep local changes durable even while reconnecting. No network calls occur here.
            let _ = sync_storage::drain(&store);
            std::thread::sleep(Duration::from_millis(100));
        }
        retry = (retry * 2).min(30);
    }
}
type Socket = WebSocket<MaybeTlsStream<std::net::TcpStream>>;
fn send(socket: &mut Socket, message: &Message) -> Result<()> {
    socket.send(WsMessage::Text(serde_json::to_string(message)?.into()))?;
    Ok(())
}
fn session(store: &Store, config: &Config, stop: &AtomicBool) -> Result<()> {
    sync_storage::drain(store)?;
    let base = config.url.trim_end_matches('/');
    let ws_url = format!(
        "{}/sync",
        base.replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1)
    );
    let mut request = ws_url.into_client_request()?;
    request
        .headers_mut()
        .insert("Authorization", format!("Bearer {}", config.token).parse()?);
    let (mut socket, _) = tungstenite::connect(request)?;
    match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => {
            stream.set_read_timeout(Some(Duration::from_millis(50)))?;
            stream.set_write_timeout(Some(Duration::from_secs(5)))?;
            stream.set_nodelay(true)?;
        }
        MaybeTlsStream::Rustls(stream) => {
            stream
                .sock
                .set_read_timeout(Some(Duration::from_millis(50)))?;
            stream
                .sock
                .set_write_timeout(Some(Duration::from_secs(5)))?;
            stream.sock.set_nodelay(true)?;
        }
        _ => {}
    }
    send(
        &mut socket,
        &Message::Auth {
            token: config.token.clone(),
            version: PROTOCOL_VERSION,
        },
    )?;
    let mut sent = HashMap::new();
    let mut requested = HashSet::new();
    let mut assets = HashSet::new();
    let mut last_config = Instant::now();
    let mut last_assets = Instant::now() - Duration::from_secs(5);
    let asset_root = store.mcp_sessions_dir.with_extension("assets");
    std::fs::create_dir_all(&asset_root)?;
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .new_agent();
    let mut authenticated = false;
    while !stop.load(Ordering::Relaxed) {
        if last_config.elapsed() > Duration::from_millis(400) {
            last_config = Instant::now();
            if read_config(&store.path).as_ref() != Some(config) {
                let _ = socket.close(None);
                return Ok(());
            }
        }
        sync_storage::drain(store)?;
        if authenticated {
            let pending=sync_storage::outbox(store)?;
            if last_assets.elapsed() > Duration::from_secs(2) || !pending.is_empty() {
                last_assets = Instant::now();
                for name in sync_storage::asset_names(store)? {
                    if assets.contains(&name) {
                        continue;
                    }
                    let path = asset_root.join(&name);
                    let url = format!("{base}/api/assets/{name}");
                    let auth = format!("Bearer {}", config.token);
                    if path.exists() {
                        agent
                            .put(&url)
                            .header("Authorization", &auth)
                            .send(std::fs::read(&path)?)
                            .context("Cannot upload image")?;
                    } else {
                        let bytes = agent
                            .get(&url)
                            .header("Authorization", &auth)
                            .call()?
                            .body_mut()
                            .with_config()
                            .limit(10_000_001)
                            .read_to_vec()?;
                        use sha2::{Digest, Sha256};
                        anyhow::ensure!(
                            bytes.len() <= 10_000_000
                                && name.starts_with(&format!("{:x}", Sha256::digest(&bytes))),
                            "Invalid server image"
                        );
                        let staging = path.with_extension("tmp");
                        std::fs::write(&staging, bytes)?;
                        std::fs::rename(staging, path)?;
                        // Prompt a redraw once an image referenced by a page becomes available.
                        store.set_setting(
                            "sync_assets_changed",
                            &uuid::Uuid::new_v4().to_string(),
                        )?;
                    }
                    assets.insert(name);
                }
            }
            for (doc, data, digest) in pending {
                if sent.get(&doc) == Some(&digest) {
                    continue;
                }
                if !requested.contains(&doc) {
                    send(
                        &mut socket,
                        &Message::Sync {
                            doc: doc.clone(),
                            vector: encode(&sync_storage::vector(store, &doc)?),
                        },
                    )?;
                    requested.insert(doc.clone());
                }
                send(
                    &mut socket,
                    &Message::Update {
                        doc: doc.clone(),
                        data: encode(&data),
                    },
                )?;
                sent.insert(doc, digest);
            }
        }
        match socket.read() {
            Ok(WsMessage::Text(text)) => match serde_json::from_str::<Message>(&text)? {
                Message::Manifest { docs } => {
                    authenticated = true;
                    status(store, "connected");
                    for doc in docs.into_iter().chain(sync_storage::documents(store)?) {
                        if !sparkpad_sync::valid_doc(&doc) || !requested.insert(doc.clone()) {
                            continue;
                        }
                        send(
                            &mut socket,
                            &Message::Sync {
                                doc: doc.clone(),
                                vector: encode(&sync_storage::vector(store, &doc)?),
                            },
                        )?;
                    }
                }
                Message::Update { doc, data } if sparkpad_sync::valid_doc(&doc) => {
                    sync_storage::receive(store, &doc, &decode(&data)?)?;
                    if !requested.contains(&doc) {
                        send(
                            &mut socket,
                            &Message::Sync {
                                doc: doc.clone(),
                                vector: encode(&sync_storage::vector(store, &doc)?),
                            },
                        )?;
                        requested.insert(doc);
                    }
                }
                Message::Sync { doc, vector } if sparkpad_sync::valid_doc(&doc) => {
                    let data = sync_storage::delta(store, &doc, &decode(&vector)?)?;
                    if data.len() > 2 {
                        send(
                            &mut socket,
                            &Message::Update {
                                doc,
                                data: encode(&data),
                            },
                        )?;
                    }
                }
                Message::Ack { doc, digest } => {
                    sync_storage::acknowledge(store, &doc, &digest)?;
                }
                Message::Ping => send(&mut socket, &Message::Pong)?,
                Message::Pong => {}
                Message::Error { message } => anyhow::bail!("Server rejected sync: {message}"),
                _ => anyhow::bail!("Unexpected sync message"),
            },
            Ok(WsMessage::Ping(data)) => {
                socket.send(WsMessage::Pong(data))?;
            }
            Ok(WsMessage::Pong(_)) => {}
            Ok(WsMessage::Close(_)) => anyhow::bail!("Server closed the connection"),
            Ok(_) => {}
            Err(WsError::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(e.into()),
        }
    }
    let _ = socket.close(None);
    Ok(())
}

pub fn cli(path: &Path, args: Vec<String>) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("connect") => {
            let url=args.get(1).context("Usage: sparkpad sync connect https://your-server (connection key on stdin or SPARKPAD_SYNC_TOKEN)")?.trim_end_matches('/').to_owned();
            let token = if let Ok(token) = std::env::var("SPARKPAD_SYNC_TOKEN") {
                token
            } else {
                let mut token = String::new();
                std::io::stdin().read_line(&mut token)?;
                token.trim().to_owned()
            };
            let config = Config { url, token };
            validate_connection(&config)?;
            write_config(path, Some(&config))?;
            println!("Server connected. Notes stay local and sync in the background.");
        }
        Some("disconnect") => {
            write_config(path, None)?;
            sync_storage::disable(&Store::open(path)?)?;
            println!("Local mode enabled. Downloaded notes are kept on this device.");
        }
        Some("status") => {
            let store = Store::open(path)?;
            println!(
                "{}",
                store
                    .setting("sync_status")?
                    .unwrap_or_else(|| "local".into())
            );
            if let Some(config) = read_config(path) {
                println!("Server: {}", config.url);
            }
        }
        Some("run") => {
            println!("Sync worker running; press Ctrl+C to stop.");
            loop {
                std::thread::sleep(Duration::from_secs(60));
            }
        }
        _ => anyhow::bail!("Usage: sparkpad sync connect URL | disconnect | status | run"),
    }
    Ok(())
}
