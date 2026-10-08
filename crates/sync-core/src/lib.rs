//! Shared, network-independent CRDT documents and the versioned sync protocol.
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::collections::BTreeMap;
use yrs::{
    updates::decoder::Decode, Doc, GetString, Map, Options, ReadTxn, StateVector, Text, Transact,
    Update,
};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_FRAME: usize = 16 * 1024 * 1024;
pub const WORKSPACE: &str = "workspace";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Auth { token: String, version: u32 },
    Manifest { docs: Vec<String> },
    Sync { doc: String, vector: String },
    Update { doc: String, data: String },
    Ack { doc: String, digest: String },
    Error { message: String },
    Ping,
    Pong,
}
pub fn encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}
pub fn decode(value: &str) -> Result<Vec<u8>> {
    Ok(STANDARD.decode(value)?)
}
pub fn valid_doc(id: &str) -> bool {
    id == WORKSPACE || (id.len() == 36 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'))
}

pub struct Document {
    pub doc: Doc,
}
impl Document {
    pub fn new(client: u64) -> Self {
        Self {
            doc: Doc::with_options(Options {
                client_id: yrs::ClientID::new(client),
                offset_kind: yrs::OffsetKind::Utf16,
                ..Default::default()
            }),
        }
    }
    pub fn load(client: u64, state: &[u8]) -> Result<Self> {
        let doc = Self::new(client);
        if !state.is_empty() {
            doc.apply(state)?;
        }
        Ok(doc)
    }
    pub fn apply(&self, update: &[u8]) -> Result<()> {
        self.doc
            .transact_mut()
            .apply_update(Update::decode_v1(update)?)
            .context("Invalid CRDT update")?;
        Ok(())
    }
    pub fn state(&self) -> Vec<u8> {
        self.doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default())
    }
    pub fn vector(&self) -> Vec<u8> {
        self.doc.transact().state_vector().encode_v1()
    }
    pub fn delta(&self, vector: &[u8]) -> Result<Vec<u8>> {
        let vector = StateVector::decode_v1(vector)?;
        Ok(self.doc.transact().encode_state_as_update_v1(&vector))
    }
    pub fn text(&self, name: &str) -> String {
        self.doc
            .get_or_insert_text(name)
            .get_string(&self.doc.transact())
    }
    /// Apply character edits, keeping unchanged text and concurrent insertions intact.
    pub fn set_text(&self, name: &str, new: &str) {
        self.edit_text(name, new, false);
    }
    pub fn set_local_text(&self, name: &str, new: &str) {
        self.edit_text(name, new, true);
    }
    fn edit_text(&self, name: &str, new: &str, local: bool) {
        let text = self.doc.get_or_insert_text(name);
        let old = text.get_string(&self.doc.transact());
        if old == new {
            return;
        }
        let prefix = old
            .chars()
            .zip(new.chars())
            .take_while(|(a, b)| a == b)
            .count();
        let old_start = old
            .char_indices()
            .nth(prefix)
            .map(|(i, _)| i)
            .unwrap_or(old.len());
        let new_start = new
            .char_indices()
            .nth(prefix)
            .map(|(i, _)| i)
            .unwrap_or(new.len());
        let suffix = old[old_start..]
            .chars()
            .rev()
            .zip(new[new_start..].chars().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let old_end = old.len()
            - old[old_start..]
                .chars()
                .rev()
                .take(suffix)
                .map(char::len_utf8)
                .sum::<usize>();
        let new_end = new.len()
            - new[new_start..]
                .chars()
                .rev()
                .take(suffix)
                .map(char::len_utf8)
                .sum::<usize>();
        let mut pos = old[..old_start].encode_utf16().count() as u32;
        let mut tx = if local {
            self.doc.transact_mut_with(self.doc.client_id())
        } else {
            self.doc.transact_mut()
        };
        let old_middle = &old[old_start..old_end];
        let new_middle = &new[new_start..new_end];
        if old_middle.is_empty() {
            text.insert(&mut tx, pos, new_middle);
            return;
        }
        if new_middle.is_empty() {
            text.remove_range(&mut tx, pos, old_middle.encode_utf16().count() as u32);
            return;
        }
        let diff = TextDiff::configure()
            .timeout(std::time::Duration::from_millis(25))
            .diff_chars(old_middle, new_middle);
        let mut previous = ChangeTag::Equal;
        let mut buffer = String::new();
        let flush = |tag: ChangeTag,
                     buffer: &mut String,
                     pos: &mut u32,
                     tx: &mut yrs::TransactionMut<'_>| {
            let len = buffer.encode_utf16().count() as u32;
            if len == 0 {
                return;
            }
            match tag {
                ChangeTag::Equal => *pos += len,
                ChangeTag::Delete => text.remove_range(tx, *pos, len),
                ChangeTag::Insert => {
                    text.insert(tx, *pos, buffer);
                    *pos += len;
                }
            }
            buffer.clear();
        };
        for change in diff.iter_all_changes() {
            if change.tag() != previous {
                flush(previous, &mut buffer, &mut pos, &mut tx);
                previous = change.tag();
            }
            buffer.push_str(change.value());
        }
        flush(previous, &mut buffer, &mut pos, &mut tx);
    }
    pub fn values(&self) -> BTreeMap<String, String> {
        let map = self.doc.get_or_insert_map("values");
        let tx = self.doc.transact();
        map.iter(&tx)
            .filter_map(|(k, v)| match v {
                yrs::Out::Any(yrs::Any::String(s)) => Some((k.to_owned(), s.to_string())),
                _ => None,
            })
            .collect()
    }
    pub fn set_values(&self, values: &BTreeMap<String, String>) {
        let map = self.doc.get_or_insert_map("values");
        let old = self.values();
        let mut tx = self.doc.transact_mut();
        for (key, value) in values {
            if old.get(key) != Some(value) {
                map.insert(&mut tx, key.as_str(), value.as_str());
            }
        }
    }
}
use yrs::updates::encoder::Encode;

/// Rebase local edits on a concurrently edited version, including undo snapshots.
pub fn merge_text(base: &str, local: &str, remote: &str) -> Result<String> {
    if local == base {
        return Ok(remote.to_owned());
    }
    if remote == base || local == remote {
        return Ok(local.to_owned());
    }
    let seed = Document::new(1);
    seed.set_text("body", base);
    let a = Document::load(2, &seed.state())?;
    let b = Document::load(3, &seed.state())?;
    a.set_text("body", local);
    b.set_text("body", remote);
    a.apply(&b.state())?;
    Ok(a.text("body"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_concurrent_unicode_edits_converge_and_replays_are_safe() {
        let seed = Document::new(1);
        seed.set_text("body", "Hello 🌱 world\n- [ ] Task\n");
        let a = Document::load(2, &seed.state()).unwrap();
        let b = Document::load(3, &seed.state()).unwrap();
        a.set_text("body", "Hello 🌱 brave world\n- [ ] Task\n");
        b.set_text("body", "Hello 🌱 world\n- [x] Task\n");
        let da = a.state();
        let db = b.state();
        a.apply(&db).unwrap();
        b.apply(&da).unwrap();
        assert_eq!(a.text("body"), "Hello 🌱 brave world\n- [x] Task\n");
        assert_eq!(a.text("body"), b.text("body"));
        a.apply(&db).unwrap();
        assert_eq!(a.text("body"), b.text("body"));
        let restarted = Document::load(2, &a.state()).unwrap();
        assert_eq!(a.text("body"), restarted.text("body"));
    }
    #[test]
    fn rebase_undo_preserves_remote_work() {
        assert_eq!(
            merge_text("Hello world", "Hello world", "Hello world!").unwrap(),
            "Hello world!"
        );
        assert_eq!(
            merge_text("Hello brave world", "Hello world", "Hello brave world!").unwrap(),
            "Hello world!"
        );
    }
    #[test]
    fn live_undo_tracks_only_own_edits() {
        let seed = Document::new(1);
        seed.set_text("body", "Hello 🌱 world");
        let mut local = CollaborativeText::load(2, &seed.state()).unwrap();
        let remote = Document::load(3, &seed.state()).unwrap();
        local.edit("Hello 🌱 brave world");
        remote.set_text("body", "Hello 🌱 world!");
        local.document.apply(&remote.state()).unwrap();
        assert_eq!(local.document.text("body"), "Hello 🌱 brave world!");
        assert!(local.undo(false));
        assert_eq!(local.document.text("body"), "Hello 🌱 world!");
        assert!(local.undo(true));
        assert_eq!(local.document.text("body"), "Hello 🌱 brave world!");
    }
    #[test]
    fn merged_outbox_retains_dependent_increments() {
        let doc = Document::new(1);
        doc.set_text("body", "Hello");
        let seed = doc.state();
        let v = doc.vector();
        doc.set_text("body", "Hello 🌱");
        let a = doc.delta(&v).unwrap();
        let v = doc.vector();
        doc.set_text("body", "Hello 🌱!");
        let b = doc.delta(&v).unwrap();
        let peer = Document::load(2, &seed).unwrap();
        peer.apply(&combine_updates(&a, &b).unwrap()).unwrap();
        assert_eq!(peer.text("body"), "Hello 🌱!");
    }
    #[test]
    fn incremental_payload_does_not_resend_the_note() {
        let doc = Document::new(1);
        let body = "note ".repeat(20_000);
        doc.set_text("body", &body);
        let vector = doc.vector();
        doc.set_text("body", &(body + "!"));
        assert!(doc.delta(&vector).unwrap().len() < 128);
    }
}

pub fn combine_updates(a: &[u8], b: &[u8]) -> Result<Vec<u8>> {
    Ok(yrs::merge_updates_v1([a, b])?)
}

/// Live editor undo only tracks operations from this editor's origin, never remote changes.
pub struct CollaborativeText {
    pub document: Document,
    undo: yrs::UndoManager,
}
impl CollaborativeText {
    pub fn load(client: u64, state: &[u8]) -> Result<Self> {
        let document = Document::load(client, state)?;
        let text = document.doc.get_or_insert_text("body");
        let mut undo = yrs::UndoManager::with_options(yrs::undo::Options::default());
        undo.expand_scope(&document.doc, &text);
        undo.include_origin(document.doc.client_id());
        Ok(Self { document, undo })
    }
    pub fn edit(&mut self, text: &str) {
        self.document.set_local_text("body", text);
    }
    pub fn boundary(&mut self) {
        self.undo.reset();
    }
    pub fn undo(&mut self, redo: bool) -> bool {
        if redo {
            self.undo.redo_blocking()
        } else {
            self.undo.undo_blocking()
        }
    }
    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }
}

/// Keep UTF-8 cursor/selection positions attached to the same surrounding text.
pub fn transform_cursor(old: &str, new: &str, position: usize) -> usize {
    let position = position.min(old.len());
    if old == new {
        return position;
    }
    let mut a = 0;
    let mut b = 0;
    let diff = TextDiff::configure()
        .timeout(std::time::Duration::from_millis(20))
        .diff_chars(old, new);
    for change in diff.iter_all_changes() {
        let len = change.value().len();
        match change.tag() {
            ChangeTag::Insert => b += len,
            ChangeTag::Delete => {
                if a + len > position {
                    return b;
                }
                a += len;
            }
            ChangeTag::Equal => {
                if a + len > position {
                    return b + position - a;
                }
                a += len;
                b += len;
            }
        }
    }
    b.min(new.len())
}
