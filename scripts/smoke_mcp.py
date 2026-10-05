#!/usr/bin/env python3
"""Exercise the real stdio server with an isolated temporary database."""
import json
import base64
import fcntl
import os
from pathlib import Path
import selectors
import subprocess
import sys
import tempfile

binary = str(Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/sparkpad").resolve())

with tempfile.TemporaryDirectory(prefix="sparkpad-") as tmp:
    env = {**os.environ, "SPARKPAD_DB": str(Path(tmp) / "notes.sqlite3")}
    proc = subprocess.Popen([binary, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, text=True, bufsize=1)
    selector = selectors.DefaultSelector()
    selector.register(proc.stdout, selectors.EVENT_READ)
    sequence = 0
    second = None
    session_dir = Path(env["SPARKPAD_DB"] + ".mcp-sessions")

    def active_sessions():
        count = 0
        for path in session_dir.glob("*.lock"):
            with path.open("r+") as lock:
                try:
                    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    count += 1
        return count


    def send(data):
        proc.stdin.write(json.dumps(data, ensure_ascii=False) + "\n")
        proc.stdin.flush()

    def receive():
        assert selector.select(5), "MCP response timed out"
        line = proc.stdout.readline()
        assert line, "MCP server closed stdout"
        return json.loads(line)

    def request(method, params=None):
        global sequence
        sequence += 1
        send({"jsonrpc": "2.0", "id": sequence, "method": method, "params": params or {}})
        response = receive()
        assert response["id"] == sequence, response
        return response

    def tool(name, args=None, error=False):
        result = request("tools/call", {"name": name, "arguments": args or {}})["result"]
        assert result["isError"] == error, result
        return result if error else result["structuredContent"]

    try:
        assert active_sessions() == 0
        init = request("initialize", {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "smoke", "version": "1"}})
        assert init["result"]["protocolVersion"] == "2025-11-25"
        assert init["result"]["serverInfo"]["name"] == "sparkpad"
        assert active_sessions() == 0, "initialize alone is not an established session"
        send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        assert request("ping")["result"] == {}
        assert active_sessions() == 1
        second = subprocess.Popen([binary, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, text=True, bufsize=1)
        second.stdin.write(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-11-25"}}) + "\n")
        second.stdin.flush()
        assert json.loads(second.stdout.readline())["id"] == 1
        second.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
        second.stdin.write(json.dumps({"jsonrpc": "2.0", "id": 2, "method": "ping"}) + "\n")
        second.stdin.flush()
        assert json.loads(second.stdout.readline())["id"] == 2
        assert active_sessions() == 2, "two agent connections must coexist"
        assert len(request("tools/list")["result"]["tools"]) == 25
        assert tool("list_notes")["notes"] == []
        parent = tool("create_note", {"title":"Parent", "markdown":"Root"})
        destination = tool("create_note", {"title":"Destination", "markdown":""})
        child = tool("create_note", {"title":"Child", "markdown":"Child body", "parent_id":parent["id"]})
        grandchild = tool("create_note", {"title":"Grandchild", "markdown":"Deep", "parent_id":child["id"]})
        assert grandchild["parent_id"] == child["id"]
        tool("move_note", {"id":parent["id"], "parent_id":grandchild["id"], "expected_revision":1}, error=True)
        tool("delete_note", {"id":parent["id"], "expected_revision":1}, error=True)
        moved = tool("move_note", {"id":child["id"], "parent_id":destination["id"], "expected_revision":1})
        assert moved["parent_id"] == destination["id"] and moved["revision"] == 2
        tool("move_note", {"id":child["id"], "parent_id":None, "expected_revision":1}, error=True)
        moved = tool("move_note", {"id":child["id"], "parent_id":None, "expected_revision":2})
        assert moved["parent_id"] is None and moved["revision"] == 3
        assert tool("get_note", {"id":grandchild["id"]})["parent_id"] == child["id"]
        tool("delete_note", {"id":child["id"], "expected_revision":3}, error=True)
        for data in (grandchild, parent, destination, moved):
            tool("delete_note", {"id":data["id"], "expected_revision":data["revision"]})
        assert tool("list_notes")["notes"] == []
        group = tool("create_sidebar_group", {"title":"Favoritos"})
        grouped = tool("create_note", {"title":"Grouped", "markdown":"Preserved", "group_id":group["id"]})
        child = tool("create_note", {"title":"Nested", "markdown":"Child", "parent_id":grouped["id"]})
        assert tool("list_sidebar_groups")["memberships"][0]["id"] == grouped["id"]
        tool("create_note", {"title":"Invalid", "markdown":"", "parent_id":grouped["id"], "group_id":group["id"]}, error=True)
        tool("create_note", {"title":"Invalid", "markdown":"", "group_id":"missing"}, error=True)
        tool("rename_sidebar_group", {"id":group["id"], "title":"Trabalho"})
        assert tool("list_sidebar_groups")["groups"][0]["title"] == "Trabalho"
        assert tool("get_note", {"id":grouped["id"]}) == grouped
        promoted = tool("move_note_to_group", {"id":child["id"], "group_id":group["id"], "expected_revision":1})
        assert promoted["parent_id"] is None and promoted["revision"] == 2
        tool("move_note_to_group", {"id":child["id"], "group_id":None, "expected_revision":1}, error=True)
        assert tool("delete_sidebar_group", {"id":group["id"]})["notes_preserved"]
        assert tool("list_sidebar_groups") == {"groups":[], "memberships":[]}
        assert tool("get_note", {"id":grouped["id"]}) == grouped
        assert tool("get_note", {"id":child["id"]}) == promoted
        for n in [grouped,promoted]: tool("delete_note", {"id":n["id"], "expected_revision":n["revision"]})
        assert tool("list_notes")["notes"] == []
        # All new tools run through the installed protocol against an isolated DB.
        group = tool("create_sidebar_group", {"title":"Visual tests"})
        a = tool("create_note", {"title":"A", "markdown":"# Heading\n\nKeep **bold**\n\n```rust\nlet x = 1;\n```", "group_id":group["id"]})
        b = tool("create_note", {"title":"B", "markdown":"B", "group_id":group["id"]})
        c = tool("create_note", {"title":"Child", "markdown":"Child", "parent_id":a["id"]})
        tree = tool("get_sidebar_tree")
        assert tree["groups"][0]["page_ids"] == [a["id"], b["id"]]
        assert next(n for n in tree["pages"] if n["id"]==a["id"])["child_ids"] == [c["id"]]
        tree = tool("reorder_sidebar", {"kind":"notes", "group_id":group["id"], "ids":[b["id"],a["id"]]})
        assert tree["groups"][0]["page_ids"] == [b["id"],a["id"]]
        tool("reorder_sidebar", {"kind":"notes", "group_id":group["id"], "ids":[a["id"]]}, error=True)
        assert tool("get_sidebar_tree")["groups"][0]["page_ids"] == [b["id"],a["id"]]
        tool("reorder_sidebar", {"kind":"notes", "parent_id":a["id"], "ids":[c["id"]]})
        tool("reorder_sidebar", {"kind":"groups", "ids":[group["id"]]})
        tool("reorder_sidebar", {"kind":"groups", "parent_id":a["id"], "ids":[group["id"]]}, error=True)
        rocket = tool("search_page_icons", {"kind":"emoji", "query":"foguete"})
        assert any(e["value"]=="🚀" for e in rocket["items"])
        emoji = tool("search_page_icons", {"kind":"emoji", "limit":1})
        assert emoji["total"] == 3944 and emoji["next_offset"] == 1
        assert len(tool("search_page_icons", {"kind":"icon", "query":"heart"})["items"]) > 0
        tool("set_note_icon", {"id":a["id"], "kind":"emoji", "value":"🚀"})
        assert tool("get_note_presentation", {"id":a["id"]})["icon"] == "🚀"
        tool("set_note_icon", {"id":a["id"], "kind":"icon", "value":"iconoir/regular/heart.svg"})
        assert tool("get_note_presentation", {"id":a["id"]})["icon"] == "sparkpad:icon:svg:iconoir/regular/heart.svg"
        tool("set_note_icon", {"id":a["id"], "kind":"icon", "value":"../missing.svg"}, error=True)
        assert tool("list_cover_presets")["presets"][0]["id"] == "coral"
        tool("set_note_cover", {"id":a["id"], "kind":"preset", "value":"lavender"})
        assert tool("get_note_presentation", {"id":a["id"]})["cover"] == "sparkpad:cover:lavender"
        tool("set_note_cover", {"id":a["id"], "kind":"preset", "value":"missing"}, error=True)
        image = Path(tmp)/"source.png"
        image.write_bytes(base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg=="))
        icon = tool("set_note_icon", {"id":a["id"], "kind":"image", "value":str(image)})
        cover = tool("set_note_cover", {"id":a["id"], "kind":"image", "value":str(image)})
        image.unlink()
        assert Path(icon["icon"].removeprefix("sparkpad:icon:image:")).is_file()
        assert Path(cover["cover"]).is_file()
        assert tool("get_note", {"id":a["id"]}) == a, "presentation must preserve content and revision"
        tool("set_note_icon", {"id":a["id"], "value":None})
        tool("set_note_cover", {"id":a["id"], "value":None})
        assert tool("get_note_presentation", {"id":a["id"]}) == {"id":a["id"],"icon":None,"cover":None,"group_id":group["id"]}
        defaults = tool("get_preferences")
        prefs = tool("update_preferences", {"theme":"light", "content_font":"serif", "opacity":80, "content_width":65, "sidebar_width":240, "sidebar":False, "always_on_top":False, "font_size":24})
        assert prefs["theme"] == "light" and prefs["content_width"] == 65 and prefs["content_font"] == "serif"
        tool("update_preferences", {"theme":"dark", "opacity":101}, error=True)
        assert tool("get_preferences") == prefs, "invalid preference must not partially commit"
        tool("update_preferences", defaults)
        blocks = tool("get_note_blocks", {"id":a["id"]})
        assert [n["kind"]["type"] for n in blocks["blocks"]] == ["heading", "paragraph", "code"]
        blocks = tool("edit_note_block", {"id":a["id"], "action":"insert", "index":1, "markdown":"> Quote", "expected_revision":1})
        assert blocks["blocks"][1]["kind"]["type"] == "quote" and blocks["revision"] == 2
        tool("edit_note_block", {"id":a["id"], "action":"delete", "index":1, "expected_revision":1}, error=True)
        blocks = tool("edit_note_block", {"id":a["id"], "action":"move", "index":1, "target_index":3, "expected_revision":2})
        assert blocks["blocks"][1]["markdown"] == "Keep **bold**" and blocks["blocks"][3]["kind"]["type"] == "quote"
        blocks = tool("edit_note_block", {"id":a["id"], "action":"update", "index":3, "markdown":"- [x] Done", "expected_revision":3})
        assert blocks["blocks"][3]["kind"] == {"type":"task", "checked":True}
        tool("edit_note_block", {"id":a["id"], "action":"update", "index":0, "markdown":"# One\n\nTwo", "expected_revision":4}, error=True)
        blocks = tool("edit_note_block", {"id":a["id"], "action":"delete", "index":3, "expected_revision":4})
        assert blocks["revision"] == 5 and tool("get_note", {"id":a["id"]})["markdown"] == a["markdown"]
        tool("delete_note", {"id":c["id"], "expected_revision":1})
        tool("delete_note", {"id":a["id"], "expected_revision":5})
        tool("delete_note", {"id":b["id"], "expected_revision":1})
        tool("delete_sidebar_group", {"id":group["id"]})
        note = tool("create_note", {"title": "Entrevista 🦀", "markdown": "# Abertura\n\nOlá, **Rust**!\n\n- Pergunta"})
        assert tool("get_note", {"id": note["id"]}) == note
        revised = tool("patch_note", {"id": note["id"], "old_text": "Pergunta", "new_text": "Próxima pergunta", "expected_revision": 1})
        assert revised["revision"] == 2
        tool("update_note", {"id": note["id"], "title": "Stale", "markdown": "Overwrite", "expected_revision": 1}, error=True)
        revised = tool("update_note", {"id": note["id"], "title": "Novo título", "markdown": "## Fechamento\n\nObrigado!", "expected_revision": 2})
        assert revised["revision"] == 3
        assert tool("select_note", {"id": note["id"]}) == revised
        assert tool("show_panel")["requested"]
        assert tool("list_notes")["notes"] == [revised]
        tool("patch_note", {"id": note["id"], "old_text": "", "new_text": "x", "expected_revision": 3}, error=True)
        assert request("unknown")["error"]["code"] == -32601
        assert request("tools/call", {"name": "unknown"})["error"]["code"] == -32602
        proc.stdin.write("invalid-json\n")
        proc.stdin.flush()
        assert receive()["error"]["code"] == -32700
        assert tool("delete_note", {"id": note["id"], "expected_revision": 3})["deleted"]
        assert tool("list_notes")["notes"] == []
        tool("get_note", {"id": note["id"]}, error=True)
        proc.stdin.close()
        assert proc.wait(timeout=5) == 0
        assert not proc.stderr.read(), "Unexpected stderr"
        assert active_sessions() == 1, "closing one client must preserve the other connection"
        second.kill()
        second.wait(timeout=5)
        assert active_sessions() == 0, "crashes must release presence without a stale connected indicator"
        print("MCP smoke test passed: 25 tools, presentation, catalogs, preferences, sibling ordering, blocks, Unicode, revisions, conflicts, notifications, errors, hierarchy, cycles, subtree moves, session presence, crash detection and clean shutdown.")
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        if second is not None and second.poll() is None:
            second.kill()
            second.wait()
        selector.close()
