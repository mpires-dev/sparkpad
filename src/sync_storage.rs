//! Durable CRDT snapshots, a transactional outbox, and a SQLite editor projection.
use crate::store::{Note, Store};
use anyhow::{Context, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use sparkpad_sync::{combine_updates, encode, Document, WORKSPACE};
use std::collections::{BTreeMap, BTreeSet};

pub fn initialize(store: &Store) -> Result<()> {
    store.conn.execute_batch("CREATE TABLE IF NOT EXISTS sync_control(id INTEGER PRIMARY KEY,enabled INTEGER NOT NULL,applying INTEGER NOT NULL);
    INSERT OR IGNORE INTO sync_control VALUES(1,0,0);
    CREATE TABLE IF NOT EXISTS sync_dirty(kind TEXT,id TEXT,PRIMARY KEY(kind,id));
    CREATE TABLE IF NOT EXISTS sync_documents(id TEXT PRIMARY KEY,state BLOB NOT NULL,projection TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS sync_outbox(doc TEXT PRIMARY KEY,data BLOB NOT NULL,digest TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS sync_versions(note_id TEXT,revision INTEGER,state BLOB NOT NULL,PRIMARY KEY(note_id,revision));")?;
    for (table, column, kind) in [
        ("notes", "id", "note"),
        ("note_presentation", "note_id", "note"),
        ("sidebar_membership", "note_id", "note"),
        ("sidebar_note_order", "note_id", "note"),
        ("sidebar_groups", "id", "group"),
        ("sidebar_group_order", "group_id", "group"),
    ] {
        for (event, row) in [("INSERT", "NEW"), ("UPDATE", "NEW"), ("DELETE", "OLD")] {
            store.conn.execute_batch(&format!(
                "CREATE TRIGGER IF NOT EXISTS sync_{table}_{event} AFTER {event} ON {table}
            WHEN (SELECT enabled=1 AND applying=0 FROM sync_control WHERE id=1)
            BEGIN INSERT OR IGNORE INTO sync_dirty VALUES('{kind}',{row}.{column}); END;"
            ))?;
        }
    }
    Ok(())
}
pub fn enabled(store: &Store) -> bool {
    store
        .conn
        .query_row("SELECT enabled FROM sync_control WHERE id=1", [], |r| {
            r.get::<_, bool>(0)
        })
        .unwrap_or(false)
}
pub fn enable(store: &Store) -> Result<()> {
    store.conn.execute_batch(
        "BEGIN IMMEDIATE; UPDATE sync_control SET enabled=1 WHERE id=1;
    INSERT OR IGNORE INTO sync_dirty SELECT 'note',id FROM notes;
    INSERT OR IGNORE INTO sync_dirty SELECT 'group',id FROM sidebar_groups; COMMIT;",
    )?;
    Ok(())
}
pub fn disable(store: &Store) -> Result<()> {
    store.conn.execute(
        "UPDATE sync_control SET enabled=0 WHERE id=1 AND enabled<>0",
        [],
    )?;
    Ok(())
}
fn actor(store: &Store) -> Result<u64> {
    if let Some(value) = store.setting("sync_actor")? {
        return Ok(value.parse()?);
    }
    let id = (uuid::Uuid::new_v4().as_u128() as u64) & ((1u64 << 53) - 1);
    store.set_setting("sync_actor", &id.to_string())?;
    Ok(id)
}
fn load(store: &Store, id: &str) -> Result<Document> {
    let state: Option<Vec<u8>> = store
        .conn
        .query_row("SELECT state FROM sync_documents WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    Document::load(actor(store)?, state.as_deref().unwrap_or_default())
}
fn projection(store: &Store, id: &str) -> Result<String> {
    Ok(store
        .conn
        .query_row(
            "SELECT projection FROM sync_documents WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or_default())
}
fn persist(
    store: &Store,
    id: &str,
    doc: &Document,
    project: &str,
    previous: &[u8],
    queue: bool,
) -> Result<()> {
    let state = doc.state();
    store.conn.execute("INSERT INTO sync_documents VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET state=excluded.state,projection=excluded.projection",params![id,state,project])?;
    if queue {
        let update = doc.delta(previous)?;
        if update.len() > 2 {
            let old: Option<Vec<u8>> = store
                .conn
                .query_row("SELECT data FROM sync_outbox WHERE doc=?1", [id], |r| {
                    r.get(0)
                })
                .optional()?;
            let update = if let Some(old) = old {
                combine_updates(&old, &update)?
            } else {
                update
            };
            let digest = format!("{:x}", Sha256::digest(encode(&update).as_bytes()));
            store.conn.execute("INSERT INTO sync_outbox VALUES(?1,?2,?3) ON CONFLICT(doc) DO UPDATE SET data=excluded.data,digest=excluded.digest",params![id,update,digest])?;
        }
    }
    if id != WORKSPACE {
        if let Ok(note) = store.get(id) {
            if note.markdown == doc.text("body")
                && note.title == {
                    let title = doc.text("title");
                    if title.trim().is_empty() {
                        "Untitled".to_owned()
                    } else {
                        title
                    }
                }
            {
                store.conn.execute(
                    "INSERT OR REPLACE INTO sync_versions VALUES(?1,?2,?3)",
                    params![id, note.revision, doc.state()],
                )?;
                store.conn.execute("DELETE FROM sync_versions WHERE note_id=?1 AND revision NOT IN (SELECT revision FROM sync_versions WHERE note_id=?1 ORDER BY revision DESC LIMIT 32)",[id])?;
            }
        }
    }
    Ok(())
}
fn portable(store: &Store, value: &str) -> Result<String> {
    let raw = value.strip_prefix("sparkpad:icon:image:").unwrap_or(value);
    let path = std::path::Path::new(raw);
    if !path.is_absolute() {
        return Ok(value.to_owned());
    }
    let root = store.mcp_sessions_dir.with_extension("assets");
    let Ok(path) = path.canonicalize() else {
        return Ok(String::new());
    };
    let Ok(root) = root.canonicalize() else {
        return Ok(String::new());
    };
    if !path.starts_with(&root) {
        return Ok(String::new());
    }
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("png");
    anyhow::ensure!(
        ["png", "jpg", "jpeg", "svg", "webp", "gif"].contains(&ext),
        "Unsupported image"
    );
    let bytes = std::fs::read(&path)?;
    anyhow::ensure!(bytes.len() <= 10_000_000, "Image too large");
    let name = format!("{:x}.{ext}", Sha256::digest(&bytes));
    let target = root.join(&name);
    if !target.exists() {
        std::fs::write(target, bytes)?;
    }
    Ok(format!("asset:{name}"))
}
fn materialize(store: &Store, value: &str, icon: bool) -> String {
    if let Some(name) = value.strip_prefix("asset:") {
        // Never let remote metadata escape the image directory.
        let valid = name.split_once('.').is_some_and(|(h, e)| {
            h.len() == 64
                && h.bytes().all(|b| b.is_ascii_hexdigit())
                && ["png", "jpg", "jpeg", "svg", "webp", "gif"].contains(&e)
        });
        if !valid {
            return String::new();
        }
        let path = store
            .mcp_sessions_dir
            .with_extension("assets")
            .join(name)
            .to_string_lossy()
            .into_owned();
        if icon {
            format!("sparkpad:icon:image:{path}")
        } else {
            path
        }
    } else {
        value.to_owned()
    }
}
fn drain_inner(store: &Store) -> Result<()> {
    let dirty: Vec<(String, String)> = store
        .conn
        .prepare("SELECT kind,id FROM sync_dirty")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    if dirty.is_empty() {
        return Ok(());
    }
    let workspace = load(store, WORKSPACE)?;
    let vector = workspace.vector();
    let old: BTreeMap<String, String> =
        serde_json::from_str(&projection(store, WORKSPACE)?).unwrap_or_default();
    let mut changed = BTreeMap::new();
    let mut observed = old.clone();
    for (kind, id) in dirty {
        let prefix = format!("{kind}/{id}/");
        let mut fields = BTreeMap::new();
        if kind == "note" {
            if let Ok(note) = store.get(&id) {
                let doc = load(store, &id)?;
                let previous = doc.vector();
                let old_body: serde_json::Value = serde_json::from_str(&projection(store, &id)?)
                    .unwrap_or(serde_json::Value::Null);
                if old_body.get("body").and_then(|v| v.as_str()) != Some(&note.markdown) {
                    doc.set_text("body", &note.markdown);
                }
                if old_body.get("title").and_then(|v| v.as_str()) != Some(&note.title) {
                    doc.set_text("title", &note.title);
                }
                persist(
                    store,
                    &id,
                    &doc,
                    &serde_json::json!({"body":note.markdown,"title":note.title}).to_string(),
                    &previous,
                    true,
                )?;
                let (icon, cover, group) = store.page_metadata(&id)?;
                let position: Option<i64> = store
                    .conn
                    .query_row(
                        "SELECT position FROM sidebar_note_order WHERE note_id=?1",
                        [&id],
                        |r| r.get(0),
                    )
                    .optional()?;
                for (k, v) in [
                    ("deleted", "false".into()),
                    ("parent", note.parent_id.unwrap_or_default()),
                    ("group", group.unwrap_or_default()),
                    ("icon", portable(store, &icon.unwrap_or_default())?),
                    ("cover", portable(store, &cover.unwrap_or_default())?),
                    ("position", position.unwrap_or(i64::MAX).to_string()),
                    ("created", note.created_at.to_string()),
                ] {
                    fields.insert(k, v);
                }
            } else {
                fields.insert("deleted", "true".into());
            }
        } else {
            let title: Option<String> = store
                .conn
                .query_row("SELECT title FROM sidebar_groups WHERE id=?1", [&id], |r| {
                    r.get(0)
                })
                .optional()?;
            if let Some(title) = title {
                let position: Option<i64> = store
                    .conn
                    .query_row(
                        "SELECT position FROM sidebar_group_order WHERE group_id=?1",
                        [&id],
                        |r| r.get(0),
                    )
                    .optional()?;
                fields.insert("deleted", "false".into());
                fields.insert("title", title);
                fields.insert("position", position.unwrap_or(i64::MAX).to_string());
            } else {
                fields.insert("deleted", "true".into());
            }
        }
        for (k, v) in fields {
            let key = format!("{prefix}{k}");
            if old.get(&key) != Some(&v) {
                changed.insert(key.clone(), v.clone());
            }
            observed.insert(key, v);
        }
    }
    workspace.set_values(&changed);
    persist(
        store,
        WORKSPACE,
        &workspace,
        &serde_json::to_string(&observed)?,
        &vector,
        true,
    )?;
    store.conn.execute("DELETE FROM sync_dirty", [])?;
    Ok(())
}
pub fn drain(store: &Store) -> Result<()> {
    if !enabled(store) {
        return Ok(());
    }
    let tx = rusqlite::Transaction::new_unchecked(&store.conn, TransactionBehavior::Immediate)?;
    drain_inner(store)?;
    tx.commit()?;
    Ok(())
}
pub fn documents(store: &Store) -> Result<Vec<String>> {
    Ok(store
        .conn
        .prepare("SELECT id FROM sync_documents")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}
pub fn vector(store: &Store, id: &str) -> Result<Vec<u8>> {
    Ok(load(store, id)?.vector())
}
pub fn delta(store: &Store, id: &str, vector: &[u8]) -> Result<Vec<u8>> {
    load(store, id)?.delta(vector)
}
pub fn outbox(store: &Store) -> Result<Vec<(String, Vec<u8>, String)>> {
    Ok(store
        .conn
        .prepare("SELECT doc,data,digest FROM sync_outbox")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?)
}
pub fn acknowledge(store: &Store, id: &str, digest: &str) -> Result<()> {
    store.conn.execute(
        "DELETE FROM sync_outbox WHERE doc=?1 AND digest=?2",
        params![id, digest],
    )?;
    Ok(())
}
pub fn asset_names(store: &Store) -> Result<Vec<String>> {
    let mut names:BTreeSet<String>=load(store,WORKSPACE)?.values().values().filter_map(|v|crate::document_images::name(v).map(str::to_owned)).collect();
    let bodies:Vec<String>=store.conn.prepare("SELECT markdown FROM notes WHERE instr(markdown,'asset:')>0")?.query_map([],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?;
    for body in bodies {names.extend(crate::document_images::references(&body));}
    Ok(names.into_iter().collect())
}

fn project_note(store: &Store, id: &str, doc: &Document) -> Result<()> {
    if store.get(id).is_err() {
        return Ok(());
    }
    let title = doc.text("title");
    let body = doc.text("body");
    anyhow::ensure!(
        title.len() <= 512 && body.len() <= 2_000_000,
        "Note exceeds editor limits"
    );
    let title = if title.trim().is_empty() {
        "Untitled"
    } else {
        &title
    };
    store.conn.execute("UPDATE notes SET title=?2,markdown=?3,revision=revision+1,updated_at=unixepoch() WHERE id=?1 AND (title<>?2 OR markdown<>?3)",params![id,title,body])?;
    persist(
        store,
        id,
        doc,
        &serde_json::json!({"title":title,"body":body}).to_string(),
        &doc.vector(),
        false,
    )?;
    Ok(())
}
fn project_workspace(store: &Store, doc: &Document) -> Result<()> {
    let values = doc.values();
    let field = |prefix: &str, key: &str| {
        values
            .get(&format!("{prefix}/{key}"))
            .cloned()
            .unwrap_or_default()
    };
    let notes: BTreeSet<String> = values
        .keys()
        .filter_map(|k| {
            k.strip_prefix("note/")?
                .split_once('/')
                .map(|(id, _)| id.to_owned())
        })
        .filter(|id| sparkpad_sync::valid_doc(id) && id != WORKSPACE)
        .collect();
    let groups: BTreeSet<String> = values
        .keys()
        .filter_map(|k| {
            k.strip_prefix("group/")?
                .split_once('/')
                .map(|(id, _)| id.to_owned())
        })
        .filter(|id| sparkpad_sync::valid_doc(id) && id != WORKSPACE)
        .collect();
    for id in &groups {
        let prefix = format!("group/{id}");
        if field(&prefix, "deleted") == "true" {
            store
                .conn
                .execute("DELETE FROM sidebar_groups WHERE id=?1", [id])?;
            continue;
        }
        store.conn.execute("INSERT INTO sidebar_groups VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET title=excluded.title",params![id,field(&prefix,"title")])?;
        store.conn.execute(
            "INSERT OR REPLACE INTO sidebar_group_order VALUES(?1,?2)",
            params![
                id,
                field(&prefix, "position")
                    .parse::<i64>()
                    .unwrap_or(i64::MAX)
            ],
        )?;
    }
    let live: BTreeSet<String> = notes
        .iter()
        .filter(|id| field(&format!("note/{id}"), "deleted") != "true")
        .cloned()
        .collect();
    for id in &live {
        let prefix = format!("note/{id}");
        store.conn.execute(
            "INSERT OR IGNORE INTO notes(id,title,markdown,created_at) VALUES(?1,'Untitled','',?2)",
            params![id, field(&prefix, "created").parse::<i64>().unwrap_or(0)],
        )?;
    }
    let mut parents: BTreeMap<String, String> = live
        .iter()
        .map(|id| (id.clone(), field(&format!("note/{id}"), "parent")))
        .collect();
    for id in &live {
        let mut chain: Vec<String> = Vec::new();
        let mut node = id.clone();
        while live.contains(&node) {
            if let Some(index) = chain.iter().position(|n| n == &node) {
                let root = chain[index..].iter().min().unwrap().clone();
                parents.insert(root, String::new());
                break;
            }
            chain.push(node.clone());
            node = parents.get(&node).cloned().unwrap_or_default();
        }
    }
    for id in &live {
        let prefix = format!("note/{id}");
        let parent = parents.get(id).filter(|p| live.contains(*p));
        store.conn.execute(
            "UPDATE notes SET parent_id=?2,revision=revision+1 WHERE id=?1 AND parent_id IS NOT ?2",
            params![id, parent],
        )?;
        let group = field(&prefix, "group");
        store
            .conn
            .execute("DELETE FROM sidebar_membership WHERE note_id=?1", [id])?;
        if parent.is_none()
            && groups.contains(&group)
            && field(&format!("group/{group}"), "deleted") != "true"
        {
            store.conn.execute(
                "INSERT INTO sidebar_membership VALUES(?1,?2)",
                params![id, group],
            )?;
        }
        let icon = materialize(store, &field(&prefix, "icon"), true);
        let cover = materialize(store, &field(&prefix, "cover"), false);
        store.conn.execute("INSERT INTO note_presentation VALUES(?1,?2,?3) ON CONFLICT(note_id) DO UPDATE SET icon=excluded.icon,cover=excluded.cover",params![id,if icon.is_empty(){None}else{Some(icon)},if cover.is_empty(){None}else{Some(cover)}])?;
        store.conn.execute(
            "INSERT OR REPLACE INTO sidebar_note_order VALUES(?1,?2)",
            params![
                id,
                field(&prefix, "position")
                    .parse::<i64>()
                    .unwrap_or(i64::MAX)
            ],
        )?;
        let state: bool = store.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sync_documents WHERE id=?1)",
            [id],
            |r| r.get(0),
        )?;
        if state {
            project_note(store, id, &load(store, id)?)?;
        }
    }
    for id in notes.difference(&live) {
        store
            .conn
            .execute("UPDATE notes SET parent_id=NULL WHERE parent_id=?1", [id])?;
    }
    for id in notes.difference(&live) {
        store.conn.execute("DELETE FROM notes WHERE id=?1", [id])?;
    }
    persist(
        store,
        WORKSPACE,
        doc,
        &serde_json::to_string(&values)?,
        &doc.vector(),
        false,
    )?;
    Ok(())
}
pub fn receive(store: &Store, id: &str, update: &[u8]) -> Result<()> {
    anyhow::ensure!(sparkpad_sync::valid_doc(id), "Invalid document ID");
    let tx = rusqlite::Transaction::new_unchecked(&store.conn, TransactionBehavior::Immediate)?;
    drain_inner(store)?;
    let doc = load(store, id)?;
    let before = doc.state();
    doc.apply(update)?;
    if before == doc.state() {
        tx.commit()?;
        return Ok(());
    }
    store
        .conn
        .execute("UPDATE sync_control SET applying=1 WHERE id=1", [])?;
    persist(
        store,
        id,
        &doc,
        &projection(store, id)?,
        &doc.vector(),
        false,
    )?;
    if id == WORKSPACE {
        project_workspace(store, &doc)?;
    } else {
        project_note(store, id, &doc)?;
    }
    store
        .conn
        .execute("UPDATE sync_control SET applying=0 WHERE id=1", [])?;
    tx.commit()?;
    Ok(())
}
pub fn editor_state_at(store: &Store, id: &str, revision: i64) -> Result<Option<Vec<u8>>> {
    if !enabled(store) {
        return Ok(None);
    }
    Ok(store
        .conn
        .query_row(
            "SELECT state FROM sync_versions WHERE note_id=?1 AND revision=?2",
            params![id, revision],
            |r| r.get(0),
        )
        .optional()?)
}
pub fn save_editor_update(store: &Store, id: &str, update: &[u8]) -> Result<Note> {
    let tx = rusqlite::Transaction::new_unchecked(&store.conn, TransactionBehavior::Immediate)?;
    drain_inner(store)?;
    store.get(id)?;
    let doc = load(store, id)?;
    let previous = doc.vector();
    if !update.is_empty() {
        doc.apply(update)?;
    }
    store
        .conn
        .execute("UPDATE sync_control SET applying=1 WHERE id=1", [])?;
    project_note(store, id, &doc)?;
    persist(store, id, &doc, &projection(store, id)?, &previous, true)?;
    store
        .conn
        .execute("UPDATE sync_control SET applying=0 WHERE id=1", [])?;
    let note = store.get(id)?;
    tx.commit()?;
    Ok(note)
}
pub fn merge_save(store: &Store, id: &str, title: &str, body: &str, revision: i64) -> Result<Note> {
    let state = editor_state_at(store, id, revision)?
        .context("This note changed before synchronization was enabled; reload before editing")?;
    let doc = Document::load(
        (uuid::Uuid::new_v4().as_u128() as u64) & ((1 << 53) - 1),
        &state,
    )?;
    let vector = doc.vector();
    doc.set_text("title", title);
    doc.set_text("body", body);
    save_editor_update(store, id, &doc.delta(&vector)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "sparkpad-sync-test-{}/notes.sqlite3",
            uuid::Uuid::new_v4()
        ));
        Store::open(&path).unwrap()
    }
    fn exchange(a: &Store, b: &Store) {
        drain(a).unwrap();
        for (id, update, hash) in outbox(a).unwrap() {
            receive(b, &id, &update).unwrap();
            acknowledge(a, &id, &hash).unwrap();
        }
    }
    #[test]
    fn offline_concurrent_changes_and_deletion_converge() {
        let a = store();
        let b = store();
        enable(&a).unwrap();
        enable(&b).unwrap();
        let note = a.create("Example", "Hello 🌱 world\n- [ ] Task").unwrap();
        exchange(&a, &b);
        assert_eq!(b.get(&note.id).unwrap().markdown, note.markdown);
        let na = a.get(&note.id).unwrap();
        let nb = b.get(&note.id).unwrap();
        a.update(
            &note.id,
            "Example",
            "Hello 🌱 brave world\n- [ ] Task",
            na.revision,
        )
        .unwrap();
        b.update(
            &note.id,
            "Example",
            "Hello 🌱 world\n- [x] Task",
            nb.revision,
        )
        .unwrap();
        drain(&a).unwrap();
        drain(&b).unwrap();
        exchange(&a, &b);
        exchange(&b, &a);
        assert_eq!(
            a.get(&note.id).unwrap().markdown,
            "Hello 🌱 brave world\n- [x] Task"
        );
        assert_eq!(
            a.get(&note.id).unwrap().markdown,
            b.get(&note.id).unwrap().markdown
        );
        a.conn
            .execute("DELETE FROM notes WHERE id=?1", [&note.id])
            .unwrap();
        let nb = b.get(&note.id).unwrap();
        b.update(&note.id, "Example", &(nb.markdown + "!"), nb.revision)
            .unwrap();
        drain(&b).unwrap();
        exchange(&a, &b);
        exchange(&b, &a);
        assert!(a.get(&note.id).is_err());
        assert!(b.get(&note.id).is_err());
    }
    #[test]
    fn metadata_edits_preserve_other_fields_and_images_are_portable() {
        let a = store();
        let b = store();
        enable(&a).unwrap();
        enable(&b).unwrap();
        let note = a.create("Example", "Text").unwrap();
        exchange(&a, &b);
        a.set_cover(&note.id, Some("sparkpad:cover:forest"))
            .unwrap();
        b.set_icon(&note.id, Some("🌱")).unwrap();
        drain(&a).unwrap();
        drain(&b).unwrap();
        exchange(&a, &b);
        exchange(&b, &a);
        assert_eq!(
            a.presentation(&note.id).unwrap(),
            (Some("🌱".into()), Some("sparkpad:cover:forest".into()))
        );
        assert_eq!(
            a.presentation(&note.id).unwrap(),
            b.presentation(&note.id).unwrap()
        );
        let source = a.path.with_extension("svg");
        std::fs::write(&source, b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>").unwrap();
        a.import_icon(&note.id, &source).unwrap();
        drain(&a).unwrap();
        let names = asset_names(&a).unwrap();
        assert_eq!(names.len(), 1);
        assert!(a
            .mcp_sessions_dir
            .with_extension("assets")
            .join(&names[0])
            .exists());
        exchange(&a, &b);
        let icon = b.presentation(&note.id).unwrap().0.unwrap();
        assert!(icon.starts_with("sparkpad:icon:image:"));
        assert!(!icon.contains(a.path.parent().unwrap().to_str().unwrap()));
    }
    #[test]
    fn stale_ack_and_editor_save_preserve_remote_edits() {
        let a = store();
        enable(&a).unwrap();
        let note = a.create("Example", "Hello world").unwrap();
        drain(&a).unwrap();
        let pending = outbox(&a).unwrap();
        let old = pending.iter().find(|p| p.0 == note.id).unwrap().2.clone();
        let base = editor_state_at(&a, &note.id, note.revision)
            .unwrap()
            .unwrap();
        let remote = Document::load(123, &base).unwrap();
        remote.set_text("body", "Hello world!");
        receive(&a, &note.id, &remote.state()).unwrap();
        merge_save(&a, &note.id, "Example", "Hello brave world", note.revision).unwrap();
        assert_eq!(a.get(&note.id).unwrap().markdown, "Hello brave world!");
        acknowledge(&a, &note.id, &old).unwrap();
        assert!(outbox(&a).unwrap().iter().any(|p| p.0 == note.id));
        let reopened = Store::open(&a.path).unwrap();
        assert_eq!(
            reopened.get(&note.id).unwrap().markdown,
            "Hello brave world!"
        );
    }
}
