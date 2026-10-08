use crate::store::Store;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

const PROTOCOLS: &[&str] = &["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"];

fn tool(
    name: &str,
    description: &str,
    properties: Value,
    required: &[&str],
    read_only: bool,
) -> Value {
    json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":read_only,"destructiveHint": name == "delete_note", "openWorldHint":false}})
}
fn tools() -> Vec<Value> {
    let string = json!({"type":"string"});
    let revision = json!({"type":"integer","minimum":1});
    vec![
        tool("get_sidebar_tree", "Read the complete ordered Sparkpad sidebar, including groups, root pages, descendants, icons, covers and selected note. Flat nodes with ordered child_ids avoid a depth limit. IDs in root_ids, group page_ids and child_ids are in display order.", json!({}), &[], true),
        tool("reorder_sidebar", "Reorder all siblings in one scope atomically. kind=groups orders root group headers; kind=notes orders ungrouped roots, a parent's children, or a group's root pages. Supply every current sibling ID exactly once; get_sidebar_tree first. Does not alter Markdown or revisions.", json!({"kind":{"enum":["notes","groups"]},"ids":{"type":"array","items":string,"uniqueItems":true},"parent_id":{"type":["string","null"]},"group_id":{"type":["string","null"]}}), &["kind","ids"], false),
        tool("get_note_presentation", "Read a page's emoji, Iconoir icon or uploaded image, cover and root group membership. Does not load or alter Markdown.", json!({"id":string}), &["id"], true),
        tool("set_note_icon", "Set a page icon: kind=emoji with a full emoji sequence; kind=icon with an Iconoir asset path from search_page_icons; kind=image with an absolute local PNG/JPG/WebP/GIF/SVG path (max 10 MB), copied into owned storage. value=null removes the icon. Markdown and content revision are unchanged.", json!({"id":string,"kind":{"enum":["emoji","icon","image"]},"value":{"type":["string","null"]}}), &["id","value"], false),
        tool("set_note_cover", "Set a full-width cover: kind=preset with an ID from list_cover_presets; kind=image with an absolute local image path (max 10 MB), copied into owned storage. value=null removes the cover. Existing icon is preserved; Markdown and content revision are unchanged.", json!({"id":string,"kind":{"enum":["preset","image"]},"value":{"type":["string","null"]}}), &["id","value"], false),
        tool("search_page_icons", "Search the same complete Unicode emoji and Iconoir catalogs as the desktop picker. Emoji search supports Portuguese and English. Paginated results contain the exact kind and value to pass to set_note_icon.", json!({"kind":{"enum":["emoji","icon"]},"query":string,"offset":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":200}}), &["kind"], true),
        tool("list_cover_presets", "List available solid-color and gradient cover presets and their stable IDs.", json!({}), &[], true),
        tool("get_preferences", "Read Sparkpad appearance and window preferences. Percentages: opacity 25–100, content_width 40–100; pixels: font_size 14–40, sidebar_width 180–420. Content fonts: sans (NV Legible Next), serif (Libron), mono (JetBrains Mono).", json!({}), &[], true),
        tool("update_preferences", "Atomically update supplied preferences; omit fields to preserve them. Updates the running app within its sync interval, or on next launch. Font choice changes content/title only. No note content is changed.", json!({"opacity":{"type":"number","minimum":25,"maximum":100},"content_width":{"type":"number","minimum":40,"maximum":100},"font_size":{"type":"number","minimum":14,"maximum":40},"sidebar_width":{"type":"number","minimum":180,"maximum":420},"sidebar":{"type":"boolean"},"always_on_top":{"type":"boolean"},"theme":{"enum":["dark","light"]},"content_font":{"enum":["sans","serif","mono"]}}), &[], false),
        tool("get_note_blocks", "Read native Markdown blocks with zero-based indices, type, visible text, Markdown and note revision. Block indices are valid only at this revision. Formatting, code, quotes, lists and task checkboxes use Markdown.", json!({"id":string}), &["id"], true),
        tool("edit_note_block", "Insert, update, delete or move a native block at expected_revision. Insert index is 0..block_count; update/delete/move index is an existing block. insert/update require Markdown representing exactly one block. move requires target_index, the final zero-based index. Untouched blocks and inline formatting are preserved. Read again on conflict.", json!({"id":string,"action":{"enum":["insert","update","delete","move"]},"index":{"type":"integer","minimum":0},"target_index":{"type":"integer","minimum":0},"markdown":string,"expected_revision":revision}), &["id","action","index","expected_revision"], false),
        tool("list_notes", "List local notes, including IDs and revisions.", json!({}), &[], true),
        tool("list_sidebar_groups", "List root sidebar organizers and their root-page memberships. Groups are not pages and never contain Markdown or other groups.", json!({}), &[], true),
        tool("create_sidebar_group", "Create a root-only sidebar organizer, such as Favorites. Groups are manual folders, not dynamic views.", json!({"title":string}), &["title"], false),
        tool("rename_sidebar_group", "Rename a sidebar organizer without modifying its pages.", json!({"id":string,"title":string}), &["id","title"], false),
        tool("delete_sidebar_group", "Delete only the organizer. Its pages return to the ungrouped root; notes and descendants are preserved.", json!({"id":string}), &["id"], false),
        tool("move_note_to_group", "Move a page and its descendants to a root sidebar group, or ungroup it with null. Promotes child pages to root atomically. Groups cannot be nested.", json!({"id":string,"group_id":{"type":["string","null"]},"expected_revision":revision}), &["id","group_id","expected_revision"], false),
        tool("get_note", "Read a note's Markdown and revision before editing.", json!({"id":string}), &["id"], true),
        tool("create_note", "Create a Markdown note. Optional parent_id creates a child page; group_id creates a root page in a sidebar group. Do not combine parent_id and group_id. Child pages can themselves have children without a depth limit.", json!({"title":string,"markdown":string,"parent_id":string,"group_id":string}), &["title","markdown"], false),
        tool("update_note", "Replace title and Markdown. expected_revision prevents overwriting edits made in the panel or by other agents. Read again on conflict.", json!({"id":string,"title":string,"markdown":string,"expected_revision":revision}), &["id","title","markdown","expected_revision"], false),
        tool("patch_note", "Replace exactly one occurrence of text in a note, preserving the rest. Fails on missing or ambiguous text or a revision conflict.", json!({"id":string,"old_text":string,"new_text":string,"expected_revision":revision}), &["id","old_text","new_text","expected_revision"], false),
        tool("move_note", "Move a note under parent_id, or to the root with null. Preserves descendants and rejects cycles and revision conflicts.", json!({"id":string,"parent_id":{"type":["string","null"]},"expected_revision":revision}), &["id","parent_id","expected_revision"], false),
        tool("delete_note", "Permanently delete a note at its current revision. include_children=true explicitly deletes its entire subtree atomically; false (default) rejects pages with children.", json!({"id":string,"expected_revision":revision,"include_children":{"type":"boolean","default":false}}), &["id","expected_revision"], false),
        tool("select_note", "Choose the note displayed in the floating panel. Works when the app is running or next time it opens.", json!({"id":string}), &["id"], false),
        tool("show_panel", "Show the floating panel when the desktop app is running.", json!({}), &[], false),
    ]
}
fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("Missing string argument: {key}"))
}
fn revision(args: &Value) -> Result<i64> {
    args.get("expected_revision")
        .and_then(Value::as_i64)
        .filter(|r| *r > 0)
        .context("expected_revision must be a positive integer")
}
fn index_arg(a: &Value, key: &str) -> Result<usize> {
    a.get(key).and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok())
        .with_context(|| format!("{key} must be a non-negative integer"))
}
fn optional_id<'a>(a: &'a Value, key: &str) -> Result<Option<&'a str>> {
    match a.get(key) {None|Some(Value::Null)=>Ok(None),Some(Value::String(s))=>Ok(Some(s)),_=>bail!("{key} must be an ID or null")}
}
fn presentation(db: &Store, id: &str) -> Result<Value> {
    let (icon,cover,group_id)=db.page_metadata(id)?;
    Ok(json!({"id":id,"icon":icon,"cover":cover,"group_id":group_id}))
}
fn sidebar_tree(db: &Store) -> Result<Value> {
    let notes=db.list_summaries()?;
    let groups=db.list_groups()?;
    let covers=db.list_covers()?;
    let mut children:std::collections::HashMap<&str,Vec<&str>>=std::collections::HashMap::new();
    let mut grouped:std::collections::HashMap<&str,Vec<&str>>=std::collections::HashMap::new();
    let mut roots=Vec::new();
    for n in &notes {
        if let Some(parent)=n.parent_id.as_deref() {children.entry(parent).or_default().push(&n.id);}
        else if let Some(group)=n.group_id.as_deref() {grouped.entry(group).or_default().push(&n.id);}
        else {roots.push(&n.id);}
    }
    let nodes:Vec<_>=notes.iter().map(|n| {
        json!({"id":n.id,"title":n.title,"parent_id":n.parent_id,"group_id":n.group_id,"revision":n.revision,"icon":n.icon,"cover":covers.get(&n.id).cloned().flatten(),"child_ids":children.get(n.id.as_str()).cloned().unwrap_or_default()})
    }).collect();
    let groups:Vec<_>=groups.iter().map(|g| json!({"id":g.id,"title":g.title,"page_ids":grouped.get(g.id.as_str()).cloned().unwrap_or_default()})).collect();
    Ok(json!({"root_ids":roots,"groups":groups,"pages":nodes,"selected_note_id":db.setting("selected_note")?}))
}
fn blocks(note: &crate::store::Note) -> Value {
    let doc=crate::blocks::Document::parse(&note.markdown);
    let blocks:Vec<_>=doc.blocks.iter().enumerate().map(|(index,b)| {
        use crate::blocks::Kind;
        let kind=match &b.kind {Kind::Paragraph=>json!({"type":"paragraph"}),Kind::Heading(n)=>json!({"type":"heading","level":n}),
            Kind::Bullet=>json!({"type":"bullet"}),Kind::Number(n)=>json!({"type":"numbered","number":n}),Kind::Task(done)=>json!({"type":"task","checked":done}),
            Kind::Quote=>json!({"type":"quote"}),Kind::Code(lang)=>json!({"type":"code","language":lang}),Kind::Divider=>json!({"type":"divider"}),Kind::Image{url,title}=>json!({"type":"image","url":url,"title":title,"width_percent":crate::blocks::image_width(title.as_deref())}),Kind::Table=>json!({"type":"table"}),Kind::Source=>json!({"type":"source"})};
        json!({"index":index,"kind":kind,"text":b.text,"markdown":b.markdown()})
    }).collect();
    json!({"id":note.id,"revision":note.revision,"blocks":blocks})
}
fn call(db: &Store, name: &str, a: &Value) -> Result<Value> {
    let schema = tools()
        .into_iter()
        .find(|t| t["name"] == name)
        .context("Unknown tool")?;
    let args = a.as_object().context("arguments must be an object")?;
    for key in args.keys() {
        if schema["inputSchema"]["properties"].get(key).is_none() {
            bail!("Unknown argument: {key}");
        }
    }
    Ok(match name {
        "get_sidebar_tree"=>sidebar_tree(db)?,
        "reorder_sidebar"=>{
            let groups=match str_arg(a,"kind")? {"groups"=>true,"notes"=>false,_=>bail!("kind must be notes or groups")};
            let ids=a["ids"].as_array().context("ids must be an array")?.iter().map(|v|v.as_str().map(str::to_owned).context("ids must contain strings")).collect::<Result<Vec<_>>>()?;
            db.reorder_sidebar(&ids,groups,optional_id(a,"parent_id")?,optional_id(a,"group_id")?)?;
            sidebar_tree(db)?
        },
        "get_note_presentation"=>presentation(db,str_arg(a,"id")?)?,
        "set_note_icon"|"set_note_cover"=>{
            let id=str_arg(a,"id")?;db.get(id)?;
            let value=match a.get("value") {Some(Value::Null)=>None,Some(Value::String(v))=>Some(v.as_str()),_=>bail!("value must be a string or null")};
            if let Some(value)=value {
                let kind=str_arg(a,"kind")?;
                match (name,kind) {
                    ("set_note_icon","emoji")=>{
                        if !crate::icon_catalog::emojis().iter().any(|e|e.value==value) {bail!("Choose one complete emoji sequence from search_page_icons")}
                        db.set_icon(id,Some(value))?;
                    },
                    ("set_note_icon","icon")=>{
                        if !crate::icon_catalog::icon_paths().iter().any(|p|p==value) {bail!("Unknown Iconoir asset; search_page_icons first")}
                        db.set_icon(id,Some(&format!("{}{value}",crate::icon_catalog::SVG_PREFIX)))?;
                    },
                    (_,"image")=>{
                        let path=std::path::Path::new(value);
                        if !path.is_absolute() {bail!("Image path must be absolute and local; download remote images first")}
                        if name=="set_note_icon" {db.import_icon(id,path)?;} else {db.import_cover(id,path)?;}
                    },
                    ("set_note_cover","preset")=>{
                        let preset=crate::covers::PRESETS.iter().find(|p|p.id==value).context("Unknown cover preset; list_cover_presets first")?;
                        db.set_cover(id,Some(&crate::covers::stored_value(preset)))?;
                    },
                    _=>bail!("Invalid presentation kind"),
                }
            } else if name=="set_note_icon" {db.set_icon(id,None)?;} else {db.set_cover(id,None)?;}
            presentation(db,id)?
        },
        "search_page_icons"=>{
            let offset=if a.get("offset").is_some() {index_arg(a,"offset")?} else {0};
            let limit=if a.get("limit").is_some() {index_arg(a,"limit")?} else {50};
            if !(1..=200).contains(&limit) {bail!("limit must be between 1 and 200")}
            let query=match a.get("query") {None=>"",Some(Value::String(s))=>s,_=>bail!("query must be a string")};
            let query=crate::icon_catalog::fold(query);
            let matches:Vec<_>=match str_arg(a,"kind")? {
                "emoji"=>crate::icon_catalog::emojis().iter().filter(|e|e.search.contains(&query)||e.value==query)
                    .map(|e|json!({"kind":"emoji","value":e.value,"name":e.name,"category":e.category,"tone":e.tone})).collect(),
                "icon"=>crate::icon_catalog::icon_paths().iter().filter(|p|p.contains(&query.replace(' ',"-")))
                    .map(|p|json!({"kind":"icon","value":p})).collect(),
                _=>bail!("kind must be emoji or icon"),
            };
            let total=matches.len();let items:Vec<_>=matches.into_iter().skip(offset).take(limit).collect();
            let next=offset.checked_add(items.len()).filter(|n|*n<total);
            json!({"items":items,"total":total,"next_offset":next})
        },
        "list_cover_presets"=>json!({"presets":crate::covers::PRESETS.iter().map(|p|json!({"id":p.id,"name":p.name,"colors":p.colors.map(|c|format!("#{c:06x}")),"angle":p.angle})).collect::<Vec<_>>()}),
        "get_preferences"=>json!(crate::preferences::Preferences::read(db)?),
        "update_preferences"=>json!(crate::preferences::Preferences::update(db,a)?),
        "get_note_blocks"=>blocks(&db.get(str_arg(a,"id")?)?),
        "edit_note_block"=>{
            let note=db.get(str_arg(a,"id")?)?;let expected=revision(a)?;
            if note.revision!=expected {bail!("Note changed externally (revision conflict)")}
            let mut doc=crate::blocks::Document::parse(&note.markdown);
            let index=index_arg(a,"index")?;
            let action=str_arg(a,"action")?;
            if index>doc.blocks.len() || (index==doc.blocks.len() && action!="insert") {bail!("Block index out of range")}
            match action {
                "insert"|"update"=>{
                    if a.get("target_index").is_some() {bail!("target_index is only valid for move")}
                    let mut fragment=crate::blocks::Document::parse(str_arg(a,"markdown")?);
                    if fragment.blocks.len()!=1 {bail!("markdown must represent exactly one block")}
                    let mut block=fragment.blocks.remove(0);
                    if action=="update" {block.before=doc.blocks[index].before.clone();doc.blocks[index]=block;}
                    else {
                        block.before=if index==0 {String::new()} else {"\n\n".into()};
                        if index==0 && !doc.blocks.is_empty() {doc.blocks[0].before="\n\n".into();}
                        doc.blocks.insert(index,block);
                    }
                },
                "delete"=>{
                    if a.get("markdown").is_some() || a.get("target_index").is_some() {bail!("delete does not accept markdown or target_index")}
                    doc.remove(index);
                },
                "move"=>{
                    if a.get("markdown").is_some() {bail!("move does not accept markdown")}
                    let target=index_arg(a,"target_index")?;
                    if target>=doc.blocks.len() {bail!("target_index out of range")}
                    if index!=target {
                        let mut block=doc.blocks.remove(index);block.before=if target==0 {String::new()} else {"\n\n".into()};
                        doc.blocks.insert(target,block);
                        doc.blocks[0].before.clear();
                        if target==0 && doc.blocks.len()>1 {doc.blocks[1].before="\n\n".into();}
                    }
                },
                _=>bail!("Unknown block action"),
            }
            let updated=db.update(&note.id,&note.title,&doc.markdown(),expected)?;
            blocks(&updated)
        },
        "list_notes" => json!({"notes":db.list()?}),
        "list_sidebar_groups" => json!({"groups":db.list_groups()?,"memberships":db.list_summaries()?.into_iter().filter(|n| n.group_id.is_some()).collect::<Vec<_>>()}),
        "create_sidebar_group" => json!(db.create_group(str_arg(a,"title")?)?),
        "rename_sidebar_group" => {db.rename_group(str_arg(a,"id")?,str_arg(a,"title")?)?;json!({"renamed":true})},
        "delete_sidebar_group" => {db.delete_group(str_arg(a,"id")?)?;json!({"deleted":true,"notes_preserved":true})},
        "move_note_to_group" => {
            let group=match a.get("group_id") {
                Some(Value::Null) => None,
                Some(Value::String(id)) => Some(id.as_str()),
                _ => bail!("group_id must be a group ID or null"),
            };
            json!(db.move_to_group(str_arg(a,"id")?,group,revision(a)?)?)
        },
        "get_note" => json!(db.get(str_arg(a, "id")?)?),
        "create_note" => {
            let parent = match a.get("parent_id") {
                None => None,
                Some(Value::String(id)) => Some(id.as_str()),
                _ => bail!("parent_id must be a string when creating a note"),
            };
            if let Some(group)=a.get("group_id") {
                if parent.is_some() {bail!("A page cannot have both parent_id and group_id")}
                let group=group.as_str().context("group_id must be a string")?;
                json!(db.create_in_group(str_arg(a,"title")?,str_arg(a,"markdown")?,group)?)
            } else {json!(db.create_child(str_arg(a, "title")?, str_arg(a, "markdown")?, parent)?)}
        }
        "move_note" => {
            let parent = match a.get("parent_id") {
                Some(Value::Null) => None,
                Some(Value::String(id)) => Some(id.as_str()),
                _ => bail!("parent_id must be a note ID or null"),
            };
            json!(db.move_note(str_arg(a,"id")?, parent, revision(a)?)?)
        },
        "update_note" => json!(db.update(
            str_arg(a, "id")?,
            str_arg(a, "title")?,
            str_arg(a, "markdown")?,
            revision(a)?
        )?),
        "patch_note" => {
            let note = db.get(str_arg(a, "id")?)?;
            let old = str_arg(a, "old_text")?;
            if old.is_empty() || note.markdown.matches(old).count() != 1 {
                bail!("old_text must match exactly one non-empty occurrence")
            }
            let markdown = note.markdown.replacen(old, str_arg(a, "new_text")?, 1);
            json!(db.update(&note.id, &note.title, &markdown, revision(a)?)?)
        }
        "delete_note" => {
            let include_children=match a.get("include_children") {None=>false,Some(Value::Bool(value))=>*value,_=>bail!("include_children must be a boolean")};
            if include_children {db.delete_subtree(str_arg(a,"id")?,revision(a)?)?;}
            else {db.delete(str_arg(a, "id")?, revision(a)?)?;}
            json!({"deleted":true})
        }
        "select_note" => json!(db.select(str_arg(a, "id")?)?),
        "show_panel" => {
            db.set_setting("panel_request", &uuid::Uuid::new_v4().to_string())?;
            json!({"requested":true})
        }
        _ => bail!("Unknown tool"),
    })
}
fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
fn dispatch(db: &Store, request: Value) -> Option<Value> {
    if request["jsonrpc"] != "2.0" || !request["method"].is_string() {
        return Some(error(
            request.get("id").cloned().unwrap_or(Value::Null),
            -32600,
            "Invalid JSON-RPC request",
        ));
    }
    let id = request.get("id")?.clone(); // Notifications never produce stdout replies.
    let p = &request["params"];
    let result = match request["method"].as_str().unwrap() {
        "initialize" => {
            let requested = p["protocolVersion"].as_str().unwrap_or("");
            let protocol = if PROTOCOLS.contains(&requested) {
                requested
            } else {
                "2025-11-25"
            };
            json!({"protocolVersion":protocol,"capabilities":{"tools":{}},"serverInfo":{"name":"sparkpad","title":"Sparkpad","version":env!("CARGO_PKG_VERSION")},"instructions":"Sparkpad: minimal native Markdown notes with unlimited nested pages, root-only sidebar groups, manual sibling ordering, rich blocks, emoji/Iconoir/uploaded icons, image/color/gradient covers and appearance preferences. get_sidebar_tree reads the complete ordered hierarchy; create_note with parent_id makes a child or group_id makes a grouped root. move_note and move_note_to_group preserve descendants. reorder_sidebar requires every sibling ID exactly once. Read notes/blocks before modifying and use expected_revision to preserve concurrent edits. Presentation changes do not change content revisions. Catalog tools return exact supported icon and cover values; local images are copied into managed storage. update_preferences applies theme, content font/width/size, opacity, pinning and sidebar controls. select_note selects the page; show_panel reveals the app. All changes share the desktop SQLite database and sync while the app runs."})
        }
        "ping" => json!({}),
        "tools/list" => json!({"tools":tools()}),
        "tools/call" => {
            let Some(name) = p["name"].as_str() else {
                return Some(error(id, -32602, "Missing tool name"));
            };
            if !tools().iter().any(|t| t["name"] == name) {
                return Some(error(id, -32602, "Unknown tool"));
            }
            let args = p.get("arguments").cloned().unwrap_or(json!({}));
            match call(db, name, &args) {
                Ok(data) => {
                    json!({"content":[{"type":"text","text":data.to_string()}],"structuredContent":data,"isError":false})
                }
                Err(e) => json!({"content":[{"type":"text","text":e.to_string()}],"isError":true}),
            }
        }
        _ => return Some(error(id, -32601, "Method not found")),
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}
pub fn serve(db: Store) -> Result<()> {
    let mut initialized = false;
    let mut session = None;
    let stdin = io::stdin();
    let mut out = io::stdout().lock();
    // MCP stdio uses one JSON-RPC message per line; stdout is reserved for the protocol.
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str(&line) {
            Ok(request) => {
                let request: Value = request;
                if initialized && session.is_none()
                    && request["jsonrpc"] == "2.0"
                    && request["method"] == "notifications/initialized"
                    && request.get("id").is_none()
                {
                    session = Some(crate::mcp_presence::Session::start(db.mcp_sessions_dir())?);
                }
                let is_initialize = request["method"] == "initialize";
                let reply = dispatch(&db, request);
                if is_initialize && reply.as_ref().is_some_and(|r| r.get("result").is_some()) {
                    initialized = true;
                }
                reply
            },
            Err(_) => Some(error(Value::Null, -32700, "Parse error")),
        };
        if let Some(reply) = reply {
            serde_json::to_writer(&mut out, &reply)?;
            writeln!(out)?;
            out.flush()?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delete_subtree_requires_explicit_opt_in_and_a_current_revision()->Result<()> {
        let db=Store::open(std::path::Path::new(":memory:"))?;let parent=db.create("Parent","Body")?;let child=db.create_child("Child","Nested body",Some(&parent.id))?;
        let args=json!({"id":parent.id,"expected_revision":parent.revision});
        assert!(call(&db,"delete_note",&args).is_err());
        assert!(call(&db,"delete_note",&json!({"id":parent.id,"expected_revision":parent.revision,"include_children":"yes"})).is_err());
        assert!(db.get(&child.id).is_ok());
        assert!(call(&db,"delete_note",&json!({"id":parent.id,"expected_revision":parent.revision+1,"include_children":true})).is_err());
        assert_eq!(call(&db,"delete_note",&json!({"id":parent.id,"expected_revision":parent.revision,"include_children":true}))?,json!({"deleted":true}));
        assert!(db.get(&child.id).is_err());Ok(())
    }
    #[test]
    fn ordered_tree_survives_reopen_and_rejects_stale_siblings() -> Result<()> {
        let path=std::env::temp_dir().join(format!("sparkpad-order-{}.sqlite3",uuid::Uuid::new_v4()));
        let db=Store::open(&path)?;
        let a=db.create("A", "Keep **formatting**")?;
        let b=db.create("B", "B")?;
        let c=db.create_child("Child", "Child",Some(&a.id))?;
        let g=db.create_group("Group")?;let h=db.create_group("Other")?;
        call(&db,"reorder_sidebar",&json!({"kind":"notes","ids":[b.id,a.id]}))?;
        call(&db,"reorder_sidebar",&json!({"kind":"groups","ids":[h.id,g.id]}))?;
        assert_eq!(sidebar_tree(&db)?["root_ids"],json!([b.id,a.id]));
        let new=db.create("New", "New")?;
        assert!(call(&db,"reorder_sidebar",&json!({"kind":"notes","ids":[a.id,b.id]})).is_err());
        assert!(call(&db,"reorder_sidebar",&json!({"kind":"notes","ids":[a.id,a.id,new.id]})).is_err());
        assert_eq!(db.get(&a.id)?,a);assert_eq!(db.get(&c.id)?,c);
        drop(db);
        let db=Store::open(&path)?;
        assert_eq!(sidebar_tree(&db)?["root_ids"],json!([b.id,a.id,new.id]));
        assert_eq!(sidebar_tree(&db)?["groups"][0]["id"],h.id);
        assert!(call(&db,"get_note_presentation",&json!({"id":"missing"})).is_err());
        drop(db);let _=std::fs::remove_file(path);
        Ok(())
    }
    #[test]
    fn protocol_and_patch_conflicts() -> Result<()> {
        let db = Store::open(std::path::Path::new(":memory:"))?;
        assert!(dispatch(
            &db,
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .is_none());
        let init = dispatch(&db,json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}})).unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
        let n = db.create("Test", "Olá world")?;
        let updated = call(
            &db,
            "patch_note",
            &json!({"id":n.id,"old_text":"world","new_text":"Rust","expected_revision":1}),
        )?;
        assert_eq!(updated["markdown"], "Olá Rust");
        let response = dispatch(&db,json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"update_note","arguments":{"id":n.id,"title":"Test","markdown":"Stale","expected_revision":1}}})).unwrap();
        assert_eq!(response["result"]["isError"], true);
        assert!(call(
            &db,
            "patch_note",
            &json!({"id":n.id,"old_text":"","new_text":"X","expected_revision":2})
        )
        .is_err());
        assert!(call(&db, "list_notes", &json!({"unexpected":1})).is_err());
        Ok(())
    }
}
