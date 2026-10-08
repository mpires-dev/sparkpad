use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub markdown: String,
    pub parent_id: Option<String>,
    pub revision: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct NoteSummary {
    pub group_id: Option<String>,
    pub icon: Option<String>,
    pub id: String,
    pub title: String,
    pub parent_id: Option<String>,
    pub revision: i64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SidebarGroup { pub id: String, pub title: String }

pub struct Store {
    pub(crate) conn: Connection,
    pub(crate) path: PathBuf,
    pub(crate) mcp_sessions_dir: PathBuf,
}

pub fn default_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("SPARKPAD_DB").or_else(|| std::env::var_os("INTERVIEW_COMPANION_DB")) {
        return Ok(path.into());
    }
    let home = std::env::var_os("HOME").context("HOME is missing")?;
    Ok(storage_path(&PathBuf::from(home)))
}

fn storage_path(home: &Path) -> PathBuf {
    let current=home.join("Library/Application Support/Sparkpad/notes.sqlite3");
    let legacy=home.join("Library/Application Support/Interview Companion/notes.sqlite3");
    // Keep existing clients and all notes on the same database during the rename.
    if !current.exists() && legacy.exists() { legacy } else { current }
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, markdown TEXT NOT NULL,
                revision INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                updated_at INTEGER NOT NULL DEFAULT (unixepoch())
            );
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )?;
        // Migrate existing notes in place: all old notes remain root pages.
        let transaction = rusqlite::Transaction::new_unchecked(&conn, rusqlite::TransactionBehavior::Immediate)?;
        let has_parent = {
            let mut stmt = transaction.prepare("PRAGMA table_info(notes)")?;
            let columns = stmt.query_map([], |r| r.get::<_, String>(1))?;
            columns.collect::<rusqlite::Result<Vec<_>>>()?.iter().any(|c| c == "parent_id")
        };
        if !has_parent {
            transaction.execute_batch("ALTER TABLE notes ADD COLUMN parent_id TEXT REFERENCES notes(id) ON DELETE RESTRICT;")?;
        }
        transaction.execute_batch("CREATE INDEX IF NOT EXISTS notes_parent ON notes(parent_id);")?;
        transaction.execute_batch("CREATE INDEX IF NOT EXISTS notes_parent_idx ON notes(parent_id);
            CREATE TABLE IF NOT EXISTS note_presentation (
            note_id TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
            icon TEXT, cover TEXT);")?;
        transaction.execute_batch("CREATE TABLE IF NOT EXISTS sidebar_groups (
            id TEXT PRIMARY KEY, title TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS sidebar_membership (
            note_id TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
            group_id TEXT NOT NULL REFERENCES sidebar_groups(id) ON DELETE CASCADE);
            CREATE INDEX IF NOT EXISTS sidebar_membership_group ON sidebar_membership(group_id);")?;
        transaction.execute_batch("CREATE TABLE IF NOT EXISTS sidebar_note_order (
            note_id TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE, position INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS sidebar_group_order (
            group_id TEXT PRIMARY KEY REFERENCES sidebar_groups(id) ON DELETE CASCADE, position INTEGER NOT NULL);")?;
        transaction.commit()?;
        let mut directory_name = path.file_name().unwrap_or_default().to_os_string();
        directory_name.push(".mcp-sessions");
        let mcp_sessions_dir = path.with_file_name(directory_name);
        let store = Self { conn, path: path.to_owned(), mcp_sessions_dir };
        crate::sync_storage::initialize(&store)?;
        Ok(store)
    }
    pub fn mcp_sessions_dir(&self) -> &Path {
        &self.mcp_sessions_dir
    }
    pub fn list(&self) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare("SELECT id,title,markdown,revision,created_at,updated_at,parent_id FROM notes ORDER BY updated_at DESC,rowid DESC")?;
        let rows = stmt.query_map([], Self::row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
        Ok(Note {
            id: row.get(0)?,
            title: row.get(1)?,
            markdown: row.get(2)?,
            revision: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            parent_id: row.get(6)?,
        })
    }
    pub fn get(&self, id: &str) -> Result<Note> {
        self.conn
            .query_row(
                "SELECT id,title,markdown,revision,created_at,updated_at,parent_id FROM notes WHERE id=?1",
                [id],
                Self::row,
            )
            .optional()?
            .context("Note not found")
    }
    pub fn create(&self, title: &str, markdown: &str) -> Result<Note> {
        self.create_child(title, markdown, None)
    }
    pub fn create_child(&self, title: &str, markdown: &str, parent_id: Option<&str>) -> Result<Note> {
        Self::validate(title, markdown)?;
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO notes(id,title,markdown,parent_id) VALUES(?1,?2,?3,?4)",
            params![id, title, markdown, parent_id],
        )?;
        self.get(&id)
    }
    /// Sidebar metadata only: never load Markdown for every row or every frame.
    pub fn list_summaries(&self) -> Result<Vec<NoteSummary>> {
        let mut stmt = self.conn.prepare("SELECT n.id,n.title,n.parent_id,n.revision,p.icon,CASE WHEN n.parent_id IS NULL THEN m.group_id ELSE NULL END FROM notes n LEFT JOIN note_presentation p ON p.note_id=n.id LEFT JOIN sidebar_membership m ON m.note_id=n.id LEFT JOIN sidebar_note_order o ON o.note_id=n.id ORDER BY COALESCE(o.position,9223372036854775807),n.title COLLATE NOCASE,n.id")?;
        let rows = stmt.query_map([], |row| Ok(NoteSummary {
            id: row.get(0)?, title: row.get(1)?, parent_id: row.get(2)?, revision: row.get(3)?, icon: row.get(4)?, group_id: row.get(5)?,
        }))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn list_groups(&self) -> Result<Vec<SidebarGroup>> {
        let mut stmt=self.conn.prepare("SELECT g.id,g.title FROM sidebar_groups g LEFT JOIN sidebar_group_order o ON o.group_id=g.id ORDER BY COALESCE(o.position,9223372036854775807),g.rowid")?;
        let rows=stmt.query_map([],|r| Ok(SidebarGroup {id:r.get(0)?,title:r.get(1)?}))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn create_in_group(&self, title: &str, markdown: &str, group: &str) -> Result<Note> {
        Self::validate(title,markdown)?;
        let tx=rusqlite::Transaction::new_unchecked(&self.conn,rusqlite::TransactionBehavior::Immediate)?;
        let id=uuid::Uuid::new_v4().to_string();
        tx.execute("INSERT INTO notes(id,title,markdown,parent_id) VALUES(?1,?2,?3,NULL)",params![id,title,markdown])?;
        tx.execute("INSERT INTO sidebar_membership(note_id,group_id) VALUES(?1,?2)",params![id,group])?;
        tx.commit()?;self.get(&id)
    }
    pub fn create_group(&self, title: &str) -> Result<SidebarGroup> {
        Self::validate(title, "")?;
        let group=SidebarGroup {id:uuid::Uuid::new_v4().to_string(),title:title.trim().to_owned()};
        self.conn.execute("INSERT INTO sidebar_groups(id,title) VALUES(?1,?2)",params![group.id,group.title])?;
        Ok(group)
    }
    pub fn rename_group(&self, id: &str, title: &str) -> Result<()> {
        Self::validate(title, "")?;
        if self.conn.execute("UPDATE sidebar_groups SET title=?2 WHERE id=?1",params![id,title.trim()])?==0 {
            bail!("Group not found")
        }
        Ok(())
    }
    /// Delete the organizer only. Its pages return to the ungrouped sidebar root.
    pub fn delete_group(&self, id: &str) -> Result<()> {
        if self.conn.execute("DELETE FROM sidebar_groups WHERE id=?1",[id])?==0 {bail!("Group not found")}
        Ok(())
    }
    /// A group holds root pages only. Moving a child here promotes its whole subtree,
    /// atomically; grouping an existing root page never changes its content revision.
    pub fn move_to_group(&self, id: &str, group: Option<&str>, expected_revision: i64) -> Result<Note> {
        let tx=rusqlite::Transaction::new_unchecked(&self.conn,rusqlite::TransactionBehavior::Immediate)?;
        let (parent,revision): (Option<String>,i64)=tx.query_row("SELECT parent_id,revision FROM notes WHERE id=?1",[id],|r| Ok((r.get(0)?,r.get(1)?)))
            .optional()?.context("Note not found")?;
        if revision!=expected_revision {bail!("Note changed externally (revision conflict)")}
        if let Some(group)=group {
            let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sidebar_groups WHERE id=?1)",[group],|r| r.get(0))?;
            if !exists {bail!("Group not found")}
        }
        if parent.is_some() {tx.execute("UPDATE notes SET parent_id=NULL,revision=revision+1,updated_at=unixepoch() WHERE id=?1",[id])?;}
        tx.execute("DELETE FROM sidebar_membership WHERE note_id=?1",[id])?;
        tx.execute("DELETE FROM sidebar_note_order WHERE note_id=?1",[id])?;
        if let Some(group)=group {tx.execute("INSERT INTO sidebar_membership(note_id,group_id) VALUES(?1,?2)",params![id,group])?;}
        tx.commit()?;
        self.get(id)
    }
    pub fn presentation(&self, id: &str) -> Result<(Option<String>, Option<String>)> {
        Ok(self.conn.query_row("SELECT icon,cover FROM note_presentation WHERE note_id=?1", [id],
            |r| Ok((r.get(0)?,r.get(1)?))).optional()?.unwrap_or_default())
    }
    pub fn list_covers(&self) -> Result<std::collections::HashMap<String,Option<String>>> {
        Ok(self.conn.prepare("SELECT note_id,cover FROM note_presentation")?
            .query_map([],|r| Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?)
    }
    pub fn page_metadata(&self, id: &str) -> Result<(Option<String>,Option<String>,Option<String>)> {
        self.conn.query_row("SELECT p.icon,p.cover,CASE WHEN n.parent_id IS NULL THEN m.group_id ELSE NULL END
            FROM notes n LEFT JOIN note_presentation p ON p.note_id=n.id
            LEFT JOIN sidebar_membership m ON m.note_id=n.id WHERE n.id=?1",[id],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?.context("Note not found")
    }
    pub fn import_icon(&self, id: &str, source: &Path) -> Result<String> {
        self.import_image(id,source,true)
    }
    pub fn import_cover(&self, id: &str, source: &Path) -> Result<String> {
        self.import_image(id,source,false)
    }
    fn import_image(&self, id: &str, source: &Path, icon: bool) -> Result<String> {
        use std::io::Read;
        self.get(id)?;
        let ext=source.extension().and_then(|s| s.to_str()).unwrap_or_default().to_ascii_lowercase();
        if !["png","jpg","jpeg","webp","gif","svg"].contains(&ext.as_str()) {bail!("Escolha uma imagem PNG, JPG, WebP, GIF ou SVG.")}
        let mut bytes=Vec::new();std::fs::File::open(source)?.take(10_000_001).read_to_end(&mut bytes)?;
        if bytes.len()>10_000_000 {bail!("A imagem deve ter no máximo 10 MB.")}
        let valid=match ext.as_str() {
            "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "jpg"|"jpeg" => bytes.starts_with(&[0xff,0xd8,0xff]),
            "gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
            "webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12)==Some(b"WEBP"),
            "svg" => std::str::from_utf8(&bytes).is_ok_and(|s| s.contains("<svg")),
            _ => false,
        };
        if !valid {bail!("O arquivo não é uma imagem válida nesse formato.")}
        let dir=self.mcp_sessions_dir.with_extension("assets");std::fs::create_dir_all(&dir)?;
        let path=dir.join(format!("{}-{}.{}",if icon {"icon"} else {"cover"},uuid::Uuid::new_v4(),ext));
        std::fs::write(&path,bytes)?;
        let local=std::fs::canonicalize(&path)?.to_string_lossy().into_owned();
        let value=if icon {format!("sparkpad:icon:image:{local}")} else {local};
        let saved=if icon {self.set_icon(id,Some(&value))} else {self.set_cover(id,Some(&value))};
        if let Err(error)=saved {let _=std::fs::remove_file(path);return Err(error);}
        Ok(value)
    }
    pub fn set_icon(&self, id: &str, icon: Option<&str>) -> Result<()> {
        self.conn.execute("INSERT INTO note_presentation(note_id,icon) VALUES(?1,?2)
            ON CONFLICT(note_id) DO UPDATE SET icon=excluded.icon",params![id,icon])?;
        Ok(())
    }
    pub fn set_cover(&self, id: &str, cover: Option<&str>) -> Result<()> {
        self.conn.execute("INSERT INTO note_presentation(note_id,cover) VALUES(?1,?2)
            ON CONFLICT(note_id) DO UPDATE SET cover=excluded.cover",params![id,cover])?;
        Ok(())
    }
    pub fn data_version(&self) -> Result<i64> {
        Ok(self.conn.query_row("PRAGMA data_version", [], |row| row.get(0))?)
    }
    pub fn move_note(&self, id: &str, parent_id: Option<&str>, expected_revision: i64) -> Result<Note> {
        let transaction = rusqlite::Transaction::new_unchecked(&self.conn, rusqlite::TransactionBehavior::Immediate)?;
        // Walk ancestors iteratively, holding the write transaction so two clients
        // cannot race to create a cycle. There is no application depth limit.
        let mut ancestor = parent_id.map(str::to_owned);
        let mut visited = std::collections::HashSet::new();
        while let Some(parent) = ancestor {
            if parent == id || !visited.insert(parent.clone()) {
                bail!("A note cannot be moved into itself or one of its descendants")
            }
            ancestor = transaction.query_row("SELECT parent_id FROM notes WHERE id=?1", [&parent], |row| row.get::<_, Option<String>>(0))
                .optional()?.context("Parent note not found")?;
        }
        let changed = transaction.execute("UPDATE notes SET parent_id=?2,revision=revision+1,updated_at=unixepoch() WHERE id=?1 AND revision=?3", params![id,parent_id,expected_revision])?;
        if changed == 0 { bail!("Note changed externally or was deleted (revision conflict)") }
        transaction.execute("DELETE FROM sidebar_membership WHERE note_id=?1",[id])?;
        transaction.execute("DELETE FROM sidebar_note_order WHERE note_id=?1",[id])?;
        transaction.commit()?;
        self.get(id)
    }
    fn validate(title: &str, markdown: &str) -> Result<()> {
        if title.trim().is_empty() || title.len() > 512 {
            bail!("Title must contain 1–512 bytes")
        }
        if markdown.len() > 2_000_000 {
            bail!("Note exceeds 2 MB")
        }
        Ok(())
    }
    pub fn update(
        &self,
        id: &str,
        title: &str,
        markdown: &str,
        expected_revision: i64,
    ) -> Result<Note> {
        Self::validate(title, markdown)?;
        let changed = self.conn.execute("UPDATE notes SET title=?2,markdown=?3,revision=revision+1,updated_at=unixepoch() WHERE id=?1 AND revision=?4", params![id,title,markdown,expected_revision])?;
        if changed == 0 {
            if crate::sync_storage::enabled(self) && self.get(id).is_ok() {
                return crate::sync_storage::merge_save(self,id,title,markdown,expected_revision);
            }
            bail!("Note changed externally or was deleted. Read it again before updating (revision conflict).")
        }
        self.get(id)
    }
    /// Delete a confirmed page subtree atomically, deepest children first.
    pub fn delete_subtree(&self,id:&str,expected_revision:i64)->Result<()> {
        let tx=rusqlite::Transaction::new_unchecked(&self.conn,rusqlite::TransactionBehavior::Immediate)?;
        let revision:Option<i64>=tx.query_row("SELECT revision FROM notes WHERE id=?1",[id],|r|r.get(0)).optional()?;
        if revision!=Some(expected_revision) {bail!("Note changed externally or was deleted (revision conflict)");}
        let ids:Vec<String>=tx.prepare("WITH RECURSIVE descendants(id,depth) AS (SELECT id,0 FROM notes WHERE id=?1 UNION ALL SELECT n.id,d.depth+1 FROM notes n JOIN descendants d ON n.parent_id=d.id) SELECT id FROM descendants ORDER BY depth DESC")?.query_map([id],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?;
        for child in ids {tx.execute("DELETE FROM notes WHERE id=?1",[child])?;}
        tx.commit()?;Ok(())
    }
    /// Move and position a page in one transaction; preserve the subtree and Markdown.
    pub fn relocate_note(&self,id:&str,parent:Option<&str>,group:Option<&str>,anchor:Option<&str>,before:bool,expected_revision:i64)->Result<Note> {
        if parent.is_some() && group.is_some() {bail!("Choose a parent or a group");}
        let tx=rusqlite::Transaction::new_unchecked(&self.conn,rusqlite::TransactionBehavior::Immediate)?;
        let current:Option<(Option<String>,i64,Option<String>)>=tx.query_row("SELECT n.parent_id,n.revision,m.group_id FROM notes n LEFT JOIN sidebar_membership m ON m.note_id=n.id WHERE n.id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let (old_parent,revision,old_group)=current.context("Note not found")?;
        if revision!=expected_revision {bail!("Note changed externally (revision conflict)");}
        let mut ancestor=parent.map(str::to_owned);
        while let Some(next)=ancestor {
            if next==id {bail!("A note cannot be moved into itself or its descendants");}
            ancestor=tx.query_row("SELECT parent_id FROM notes WHERE id=?1",[next],|r|r.get::<_,Option<String>>(0)).optional()?.context("Parent note not found")?;
        }
        if let Some(group)=group {tx.query_row("SELECT id FROM sidebar_groups WHERE id=?1",[group],|r|r.get::<_,String>(0)).optional()?.context("Group not found")?;}
        let mut siblings:Vec<String>=tx.prepare("SELECT n.id FROM notes n LEFT JOIN sidebar_membership m ON m.note_id=n.id LEFT JOIN sidebar_note_order o ON o.note_id=n.id WHERE n.id!=?3 AND (n.parent_id=?1 OR (n.parent_id IS NULL AND ?1 IS NULL)) AND (?1 IS NOT NULL OR m.group_id=?2 OR (m.group_id IS NULL AND ?2 IS NULL)) ORDER BY COALESCE(o.position,9223372036854775807),n.title COLLATE NOCASE,n.id")?.query_map(params![parent,group,id],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?;
        let position=if let Some(anchor)=anchor {siblings.iter().position(|s|s==anchor).context("Drop destination changed; try again")?+usize::from(!before)} else {siblings.len()};
        siblings.insert(position,id.to_owned());
        if old_parent.as_deref()!=parent || old_group.as_deref()!=group {
            if old_parent.as_deref()!=parent {tx.execute("UPDATE notes SET parent_id=?2,revision=revision+1,updated_at=unixepoch() WHERE id=?1",params![id,parent])?;}
            tx.execute("DELETE FROM sidebar_membership WHERE note_id=?1",[id])?;
            if let Some(group)=group {tx.execute("INSERT INTO sidebar_membership(note_id,group_id) VALUES(?1,?2)",params![id,group])?;}
        }
        for (i,sibling) in siblings.iter().enumerate() {tx.execute("INSERT INTO sidebar_note_order(note_id,position) VALUES(?1,?2) ON CONFLICT(note_id) DO UPDATE SET position=excluded.position",params![sibling,i as i64])?;}
        tx.commit()?;self.get(id)
    }
    pub fn delete(&self, id: &str, expected_revision: i64) -> Result<()> {
        let transaction = rusqlite::Transaction::new_unchecked(&self.conn, rusqlite::TransactionBehavior::Immediate)?;
        let has_children: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM notes WHERE parent_id=?1)", [id], |r| r.get(0))?;
        if has_children { bail!("This note has child notes. Move or delete its children first.") }
        let changed = self.conn.execute(
            "DELETE FROM notes WHERE id=?1 AND revision=?2",
            params![id, expected_revision],
        )?;
        if changed == 0 {
            bail!("Note changed externally or was deleted (revision conflict)")
        }
        transaction.commit()?;
        Ok(())
    }
    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value])?;
        Ok(())
    }
    pub fn set_settings(&self, values: &[(&str, String)]) -> Result<()> {
        let tx=self.conn.unchecked_transaction()?;
        for (key,value) in values {
            tx.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value])?;
        }
        tx.commit()?;
        Ok(())
    }
    /// Full sibling snapshots prevent stale clients from silently dropping new items.
    pub fn reorder_sidebar(&self, ids: &[String], groups: bool, parent: Option<&str>, group: Option<&str>) -> Result<()> {
        if (groups && (parent.is_some() || group.is_some())) || (parent.is_some() && group.is_some()) {
            bail!("Groups exist only at root; choose one parent_id or group_id for pages")
        }
        let tx=rusqlite::Transaction::new_unchecked(&self.conn,rusqlite::TransactionBehavior::Immediate)?;
        if let Some(id)=parent {tx.query_row("SELECT id FROM notes WHERE id=?1",[id],|r|r.get::<_,String>(0)).optional()?.context("Parent note not found")?;}
        if let Some(id)=group {tx.query_row("SELECT id FROM sidebar_groups WHERE id=?1",[id],|r|r.get::<_,String>(0)).optional()?.context("Group not found")?;}
        let current:Vec<String>=if groups {
            tx.prepare("SELECT id FROM sidebar_groups")?.query_map([],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?
        } else {
            tx.prepare("SELECT n.id FROM notes n LEFT JOIN sidebar_membership m ON m.note_id=n.id
                WHERE (n.parent_id=?1 OR (n.parent_id IS NULL AND ?1 IS NULL))
                AND (?1 IS NOT NULL OR m.group_id=?2 OR (m.group_id IS NULL AND ?2 IS NULL))")?
                .query_map(params![parent,group],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?
        };
        let provided:std::collections::HashSet<_>=ids.iter().collect();
        if provided.len()!=ids.len() || ids.len()!=current.len() || !current.iter().all(|id| provided.contains(id)) {
            bail!("ids must contain every current sibling exactly once; refresh get_sidebar_tree on conflict")
        }
        let sql=if groups {"INSERT INTO sidebar_group_order(group_id,position) VALUES(?1,?2) ON CONFLICT(group_id) DO UPDATE SET position=excluded.position"}
            else {"INSERT INTO sidebar_note_order(note_id,position) VALUES(?1,?2) ON CONFLICT(note_id) DO UPDATE SET position=excluded.position"};
        for (position,id) in ids.iter().enumerate() {tx.execute(sql,params![id,position as i64])?;}
        tx.commit()?;Ok(())
    }
    pub fn select(&self, id: &str) -> Result<Note> {
        let note = self.get(id)?;
        self.set_setting("selected_note", id)?;
        Ok(note)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_relocation_is_atomic_ordered_and_preserves_subtrees()->Result<()> {
        let db=Store::open(Path::new(":memory:"))?;
        let a=db.create("A","Markdown A")?;let b=db.create("B","Markdown B")?;
        let child=db.create_child("Child","Nested content",Some(&a.id))?;
        let group=db.create_group("Work")?;
        assert!(db.relocate_note(&a.id,Some(&child.id),None,None,false,a.revision).is_err());
        assert!(db.relocate_note(&a.id,None,Some(&group.id),Some(&b.id),true,a.revision).is_err());
        assert_eq!(db.get(&a.id)?,a);assert!(db.list_summaries()?.iter().all(|n|n.group_id.is_none()));
        assert_eq!(db.relocate_note(&b.id,None,None,Some(&a.id),true,b.revision)?,b);
        assert_eq!(db.list_summaries()?[0].id,b.id);
        assert_eq!(db.relocate_note(&a.id,None,Some(&group.id),None,false,a.revision)?,a);
        assert_eq!(db.get(&child.id)?,child);
        let nested=db.relocate_note(&a.id,Some(&b.id),None,None,false,a.revision)?;
        assert_eq!(nested.markdown,a.markdown);assert_eq!(nested.parent_id.as_deref(),Some(b.id.as_str()));
        assert!(db.relocate_note(&a.id,None,None,None,false,a.revision).is_err());
        assert_eq!(db.get(&a.id)?,nested);
        assert!(db.delete_subtree(&a.id,a.revision).is_err());assert_eq!(db.get(&child.id)?,child);
        db.delete_subtree(&a.id,nested.revision)?;
        assert!(db.get(&child.id).is_err());assert!(db.get(&a.id).is_err());assert_eq!(db.get(&b.id)?,b);
        Ok(())
    }
    #[test]
    fn sparkpad_storage_preserves_existing_installations() -> Result<()> {
        let home=std::env::temp_dir().join(format!("sparkpad-storage-{}",uuid::Uuid::new_v4()));
        let current=home.join("Library/Application Support/Sparkpad/notes.sqlite3");
        let legacy=home.join("Library/Application Support/Interview Companion/notes.sqlite3");
        assert_eq!(storage_path(&home),current);
        let db=Store::open(&legacy)?;
        let note=db.create("Original note","Preserved content")?;
        assert_eq!(storage_path(&home),legacy);
        assert_eq!(Store::open(&storage_path(&home))?.get(&note.id)?,note);
        drop(db);
        Store::open(&current)?;
        assert_eq!(storage_path(&home),current);
        std::fs::remove_dir_all(home)?;
        Ok(())
    }

    #[test]
    fn uploaded_icons_are_owned_persistent_and_do_not_modify_notes() -> Result<()> {
        let dir=std::env::temp_dir().join(format!("sparkpad-icon-upload-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir)?;
        let db_path=dir.join("notes.sqlite3");let source=dir.join("Original.PNG");
        // A valid one-pixel PNG, with no access to the user's actual image files.
        let bytes:Vec<u8>="89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000b49444154789c636000020000050001a5f645400000000049454e44ae426082".as_bytes().chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(),16).unwrap()).collect();
        std::fs::write(&source,&bytes)?;
        let db=Store::open(&db_path)?;let note=db.create("Icon note","**Original body**")?;
        let value=db.import_icon(&note.id,&source)?;
        let owned=PathBuf::from(value.strip_prefix("sparkpad:icon:image:").unwrap());
        assert_ne!(owned,source);assert_eq!(std::fs::read(&owned)?,bytes);
        std::fs::remove_file(&source)?;assert!(owned.exists());
        assert_eq!(db.get(&note.id)?,note);
        let invalid=dir.join("Invalid.png");std::fs::write(&invalid,"Not an image")?;
        assert!(db.import_icon(&note.id,&invalid).is_err());
        assert_eq!(db.presentation(&note.id)?.0.as_deref(),Some(value.as_str()));
        let large=dir.join("Large.png");let file=std::fs::File::create(&large)?;file.set_len(10_000_001)?;
        assert!(db.import_icon(&note.id,&large).is_err());
        assert!(db.import_icon("missing",&owned).is_err());
        drop(db);let db=Store::open(&db_path)?;
        assert_eq!(db.presentation(&note.id)?.0.as_deref(),Some(value.as_str()));assert_eq!(db.get(&note.id)?,note);
        db.set_icon(&note.id,None)?;assert_eq!(db.presentation(&note.id)?.0,None);
        drop(db);std::fs::remove_dir_all(dir)?;Ok(())
    }

    #[test]
    fn sidebar_groups_are_root_only_and_preserve_pages() -> Result<()> {
        let dir=std::env::temp_dir().join(format!("sparkpad-groups-{}",uuid::Uuid::new_v4()));
        let path=dir.join("notes.sqlite3");
        let db=Store::open(&path)?;
        let root=db.create("Original","# Untouched")?;
        let child=db.create_child("Child","Child body",Some(&root.id))?;
        let grandchild=db.create_child("Grandchild","Deep",Some(&child.id))?;
        let group=db.create_group("Favorites")?;
        assert!(db.get(&group.id).is_err(),"groups must not be pages");
        assert!(db.create_group("   ").is_err());
        assert_eq!(db.move_to_group(&root.id,Some(&group.id),root.revision)?,root);
        assert_eq!(db.get(&child.id)?,child);
        assert_eq!(db.list_summaries()?.iter().find(|n| n.id==root.id).unwrap().group_id.as_deref(),Some(group.id.as_str()));
        db.rename_group(&group.id,"Work")?;
        assert_eq!(db.get(&root.id)?,root);
        assert!(db.move_to_group(&child.id,Some("missing"),child.revision).is_err());
        assert_eq!(db.get(&child.id)?,child,"failed move must not promote child");
        assert!(db.move_to_group(&root.id,Some(&group.id),999).is_err());
        let promoted=db.move_to_group(&child.id,Some(&group.id),child.revision)?;
        assert_eq!(promoted.parent_id,None);assert_eq!(promoted.revision,child.revision+1);
        assert_eq!(db.get(&grandchild.id)?,grandchild);
        assert_eq!(promoted.markdown,child.markdown);
        let nested=db.move_note(&child.id,Some(&root.id),promoted.revision)?;
        assert_eq!(db.list_summaries()?.iter().find(|n| n.id==child.id).unwrap().group_id,None);
        let count=db.list()?.len();
        assert!(db.create_in_group("Invalid","Untouched","missing").is_err());
        assert_eq!(db.list()?.len(),count,"invalid group creation must not leave an orphan page");
        let inside=db.create_in_group("New grouped note","New body",&group.id)?;
        drop(db);
        let db=Store::open(&path)?;
        assert_eq!(db.list_groups()?,vec![SidebarGroup {id:group.id.clone(),title:"Work".into()}]);
        assert_eq!(db.list_summaries()?.iter().filter(|n| n.group_id.as_deref()==Some(group.id.as_str())).count(),2);
        db.delete_group(&group.id)?;
        assert!(db.list_groups()?.is_empty());
        assert!(db.list_summaries()?.iter().all(|n| n.group_id.is_none()));
        assert_eq!(db.get(&root.id)?,root);assert_eq!(db.get(&inside.id)?,inside);
        assert_eq!(db.get(&child.id)?,nested);assert_eq!(db.get(&grandchild.id)?,grandchild);
        drop(db);std::fs::remove_dir_all(dir)?;Ok(())
    }

    #[test]
    fn page_presentation_preserves_document_and_isolates_notes() -> Result<()> {
        let temp=std::env::temp_dir().join(format!("sparkpad-page-{}.sqlite3",uuid::Uuid::new_v4()));
        let (first,second);
        {
            let db=Store::open(&temp)?;
            first=db.create("Title","**Untouched body**")?;
            second=db.create("Other","Other body")?;
            db.set_icon(&first.id,Some("🎤"))?;
            db.set_cover(&first.id,Some("/tmp/cover.png"))?;
            assert!(db.set_icon("missing",Some("📄")).is_err());
            assert_eq!(db.get(&first.id)?,first);
            assert_eq!(db.presentation(&second.id)?,(None,None));
        }
        {
            let db=Store::open(&temp)?;
            assert_eq!(db.presentation(&first.id)?,(Some("🎤".into()),Some("/tmp/cover.png".into())));
            assert_eq!(db.list_summaries()?.iter().find(|n| n.id==first.id).unwrap().icon.as_deref(),Some("🎤"));
            db.set_icon(&first.id,None)?;
            assert_eq!(db.presentation(&first.id)?.1,Some("/tmp/cover.png".into()));
            db.delete(&first.id,first.revision)?;
            assert_eq!(db.presentation(&first.id)?,(None,None));
            assert_eq!(db.get(&second.id)?,second);
        }
        std::fs::remove_file(temp)?;
        Ok(())
    }

    #[test]
    fn nesting_moves_cycles_and_revision_conflicts() -> Result<()> {
        let db = Store::open(Path::new(":memory:"))?;
        let root = db.create("Root", "Untouched")?;
        let child = db.create_child("Child", "Body", Some(&root.id))?;
        let grandchild = db.create_child("Grandchild", "Deep", Some(&child.id))?;
        assert_eq!(db.get(&grandchild.id)?.parent_id, Some(child.id.clone()));
        assert!(db.create_child("Orphan", "", Some("missing")).is_err());
        assert!(db.move_note(&root.id, Some(&grandchild.id),1).is_err());
        assert!(db.move_note(&root.id, Some(&root.id),1).is_err());
        assert!(db.move_note(&child.id, Some("missing"),1).is_err());
        assert!(db.delete(&root.id,1).is_err());
        let detached = db.move_note(&child.id,None,1)?;
        assert_eq!(detached.revision,2);
        assert_eq!(detached.parent_id,None);
        assert_eq!(detached.markdown,"Body");
        assert_eq!(db.get(&grandchild.id)?.parent_id,Some(child.id.clone()));
        assert!(db.move_note(&child.id, Some(&root.id),1).is_err());
        db.delete(&root.id,1)?;
        assert!(db.delete(&child.id,2).is_err());
        db.delete(&grandchild.id,1)?;
        db.delete(&child.id,2)?;
        assert!(db.list_summaries()?.is_empty());
        Ok(())
    }

    #[test]
    fn deep_hierarchy_has_no_application_depth_limit() -> Result<()> {
        let db=Store::open(Path::new(":memory:"))?;
        let root=db.create("Root","")?;
        let mut parent=root.id.clone();
        for i in 0..1500 { parent=db.create_child(&format!("Depth {i}"),"",Some(&parent))?.id; }
        assert_eq!(db.list_summaries()?.len(),1501);
        assert!(db.move_note(&root.id,Some(&parent),1).is_err());
        assert_eq!(db.get(&root.id)?.parent_id,None);
        db.move_note(&parent,None,1)?;
        Ok(())
    }
    #[test]
    fn migrates_legacy_database_and_serializes_cross_client_moves() -> Result<()> {
        let directory=std::env::temp_dir().join(format!("sparkpad-migration-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory)?;
        let path=directory.join("notes.sqlite3");
        {
            let legacy=Connection::open(&path)?;
            legacy.execute_batch("CREATE TABLE notes(id TEXT PRIMARY KEY,title TEXT NOT NULL,markdown TEXT NOT NULL,revision INTEGER NOT NULL DEFAULT 1,created_at INTEGER NOT NULL DEFAULT 0,updated_at INTEGER NOT NULL DEFAULT 0); INSERT INTO notes(id,title,markdown) VALUES('legacy','Original','Preserved Markdown');")?;
        }
        {
            let a=Store::open(&path)?;
            let b=Store::open(&path)?;
            let old=a.get("legacy")?;
            assert_eq!(old.parent_id,None);
            assert_eq!(old.markdown,"Preserved Markdown");
            let other=a.create("Other","")?;
            let generation=a.data_version()?;
            b.move_note("legacy",Some(&other.id),1)?;
            assert_ne!(a.data_version()?,generation);
            assert!(a.move_note(&other.id,Some("legacy"),1).is_err());
            assert!(a.update("legacy","Stale","Lost",1).is_err());
        }
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }
    #[test]
    fn persistence_and_concurrent_edits() -> Result<()> {
        let path = std::env::temp_dir().join(format!("sparkpad-{}.sqlite3", uuid::Uuid::new_v4()));
        {
            let a = Store::open(&path)?;
            let b = Store::open(&path)?;
            let n = a.create("Interview", "# Olá\n\n**Rust** 🦀")?;
            a.select(&n.id)?;
            let updated = b.update(&n.id, "Interview", "Agent edit", 1)?;
            assert_eq!(updated.revision, 2);
            assert!(a.update(&n.id, "Stale", "Lost update", 1).is_err());
            assert_eq!(a.get(&n.id)?.markdown, "Agent edit");
            assert!(a.delete(&n.id, 1).is_err());
            assert_eq!(a.setting("selected_note")?, Some(n.id.clone()));
            a.delete(&n.id, 2)?;
            assert!(b.list()?.is_empty());
            a.create("Persisted", "Olá")?;
        }
        assert_eq!(Store::open(&path)?.list()?[0].markdown, "Olá");
        std::fs::remove_file(path)?;
        Ok(())
    }
}
