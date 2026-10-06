//! Portable HTML clipboard interchange; unsupported content never executes or loads assets.
use crate::blocks::{Block, Kind, Marks};
use html5ever::{parse_document, tendril::TendrilSink};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
fn safe_link(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.starts_with("https://") || value.starts_with("http://") || value.starts_with("mailto:") || value.starts_with('#')
}
fn inline_html(block: &Block) -> String {
    block.spans.iter().map(|span| {
        let mut text = escape(&block.text[span.range.clone()]).replace('\n', "<br>");
        let marks = &span.marks;
        if marks.code { text = format!("<code>{text}</code>"); }
        if marks.bold { text = format!("<strong>{text}</strong>"); }
        if marks.italic { text = format!("<em>{text}</em>"); }
        if marks.underline { text = format!("<u>{text}</u>"); }
        if marks.strike { text = format!("<s>{text}</s>"); }
        if let Some(link) = marks.link.as_ref().filter(|link| safe_link(link)) { text = format!("<a href=\"{}\">{text}</a>", escape(link)); }
        text
    }).collect()
}
pub fn to_html(blocks: &[Block]) -> String {
    let mut html = String::from("<meta charset=\"utf-8\"><div>");
    let mut list: Option<(&str, u32)> = None;
    for block in blocks {
        let target = match block.kind { Kind::Bullet | Kind::Task(_) => Some(("ul", 0)), Kind::Number(n) => Some(("ol", n)), _ => None };
        let keep_list = match (list, target) { (Some(("ul", _)), Some(("ul", _))) => true, (Some(("ol", last)), Some(("ol", next))) => next == last + 1, _ => false };
        if !keep_list {
            if let Some((tag, _)) = list.take() { html.push_str(&format!("</{tag}>")); }
            if let Some((tag, number)) = target {
                html.push_str(&if tag == "ol" { format!("<ol start=\"{number}\">") } else { "<ul>".into() });
            }
        }
        list = target;
        let text = inline_html(block);
        match &block.kind {
            Kind::Heading(level) => html.push_str(&format!("<h{level}>{text}</h{level}>")),
            Kind::Bullet | Kind::Number(_) => html.push_str(&format!("<li>{text}</li>")),
            Kind::Task(checked) => html.push_str(&format!("<li data-checked=\"{checked}\"><input type=\"checkbox\" disabled{}>{text}</li>", if *checked { " checked" } else { "" })),
            Kind::Quote => html.push_str(&format!("<blockquote>{text}</blockquote>")),
            Kind::Code(language) => html.push_str(&format!("<pre><code class=\"language-{}\">{}</code></pre>", escape(language), escape(&block.text))),
            Kind::Divider => html.push_str("<hr>"),
            _ => html.push_str(&format!("<p>{}</p>", if text.is_empty() { "<br>" } else { &text })),
        }
    }
    if let Some((tag, _)) = list { html.push_str(&format!("</{tag}>")); }
    html.push_str("</div>"); html
}
struct Builder { blocks: Vec<Block>, current: Option<Block>, lists: Vec<Option<u32>> }
impl Builder {
    fn flush(&mut self, empty: bool) {
        if let Some(mut block) = self.current.take() {
            if !matches!(block.kind, Kind::Code(_)) {
                let end = block.text.trim_end_matches(' ').len();
                block = block.slice(0..end);
            }
            if empty || !block.text.is_empty() || matches!(block.kind, Kind::Divider) {
                block.before = if matches!(block.kind, Kind::Bullet | Kind::Number(_) | Kind::Task(_)) { "\n" } else { "\n\n" }.into();
                self.blocks.push(block);
            }
        }
    }
    fn text(&mut self, value: &str, marks: &Marks, pre: bool) {
        if self.current.is_none() && value.trim().is_empty() { return; }
        let block = self.current.get_or_insert_with(|| Block::new(Kind::Paragraph, String::new()));
        let mut text = String::new();
        let mut space = block.text.is_empty() || block.text.ends_with(' ');
        for ch in value.chars() {
            if !pre && (ch.is_whitespace() || ch == '\u{a0}') {
                if !space { text.push(' '); } space = true;
            } else { text.push(ch); space = false; }
        }
        block.append_marked(&text, marks);
    }
    fn visit(&mut self, node: &Handle, inherited: &Marks, pre: bool, depth: usize) {
        if depth > 256 { return; }
        let NodeData::Element { name, attrs, .. } = &node.data else {
            if let NodeData::Text { contents } = &node.data { self.text(&contents.borrow(), inherited, pre); }
            else { for child in node.children.borrow().iter() { self.visit(child, inherited, pre, depth + 1); } }
            return;
        };
        let tag = name.local.as_ref();
        if matches!(tag, "script" | "style" | "head" | "iframe" | "object" | "svg" | "img") { return; }
        let attributes = attrs.borrow();
        let attr = |key: &str| attributes.iter().find(|a| a.name.local.as_ref() == key).map(|a| a.value.to_string());
        if tag == "br" {
            let block = self.current.get_or_insert_with(|| Block::new(Kind::Paragraph, String::new()));
            if !block.text.is_empty() { block.append_marked("\n", inherited); } return;
        }
        if tag == "input" {
            if attr("type").as_deref() == Some("checkbox") {
                if let Some(block) = &mut self.current { block.kind = Kind::Task(attr("checked").is_some()); }
            } return;
        }
        let mut marks = inherited.clone();
        match tag {
            "b" | "strong" => marks.bold = true, "i" | "em" => marks.italic = true,
            "u" => marks.underline = true, "s" | "del" | "strike" => marks.strike = true,
            "code" if !pre => marks.code = true,
            "a" => marks.link = attr("href").filter(|v| safe_link(v)), _ => {},
        }
        if let Some(style) = attr("style") {
            for declaration in style.split(';') {
                if let Some((key, value)) = declaration.split_once(':') {
                    let value = value.trim().to_ascii_lowercase();
                    match key.trim().to_ascii_lowercase().as_str() {
                        "font-weight" => { if value == "bold" || value.parse::<u16>().is_ok_and(|weight| weight >= 600) { marks.bold = true; } else if value == "normal" || value == "400" { marks.bold = false; } },
                        "font-style" if value == "italic" => marks.italic = true,
                        "text-decoration" | "text-decoration-line" => { marks.underline |= value.contains("underline"); marks.strike |= value.contains("line-through"); },
                        "font-family" if value.contains("monospace") => marks.code = true,
                        _ => {},
                    }
                }
            }
        }
        let kind = match tag {
            "h1" => Some(Kind::Heading(1)), "h2" => Some(Kind::Heading(2)), "h3" => Some(Kind::Heading(3)),
            "h4" => Some(Kind::Heading(4)), "h5" => Some(Kind::Heading(5)), "h6" => Some(Kind::Heading(6)),
            "p" => Some(Kind::Paragraph), "blockquote" => Some(Kind::Quote),
            "pre" => {
                let language = node.children.borrow().iter().find_map(|child| {
                    if let NodeData::Element { attrs, .. } = &child.data {
                        attrs.borrow().iter().find(|a| a.name.local.as_ref() == "class").and_then(|a| a.value.split_whitespace().find_map(|class| class.strip_prefix("language-").map(str::to_owned)))
                    } else { None }
                }).unwrap_or_default(); Some(Kind::Code(language))
            },
            "li" => Some(match self.lists.last_mut() { Some(Some(number)) => { let value = *number; *number = number.saturating_add(1); Kind::Number(value) }, _ => Kind::Bullet }),
            "hr" => Some(Kind::Divider), _ => None,
        };
        let boundary = kind.is_some() || matches!(tag, "div" | "section" | "article" | "ul" | "ol");
        // <p> inside a list item / quote is an inline wrapper, not a second block.
        let contained = matches!(tag, "p" | "div") && self.current.as_ref().is_some_and(|block| matches!(block.kind, Kind::Bullet | Kind::Number(_) | Kind::Task(_) | Kind::Quote));
        if boundary && !contained { self.flush(false); }
        if let Some(kind) = kind.clone().filter(|_| !contained) { self.current = Some(Block::new(kind, String::new())); }
        if tag == "ul" || tag == "ol" { self.lists.push(if tag == "ol" { Some(attr("start").and_then(|v| v.parse().ok()).unwrap_or(1)) } else { None }); }
        if let Some(checked) = attr("data-checked") { if let Some(block) = &mut self.current { block.kind = Kind::Task(checked == "true"); } }
        let pre = pre || tag == "pre";
        if pre { marks.code = false; }
        for child in node.children.borrow().iter() { self.visit(child, &marks, pre, depth + 1); }
        if tag == "ul" || tag == "ol" { self.lists.pop(); }
        if boundary && !contained { self.flush(kind.is_some()); }
    }
}
pub fn from_html(html: &str) -> Vec<Block> {
    if html.len() > 10 * 1024 * 1024 { return Vec::new(); }
    let dom = parse_document(RcDom::default(), Default::default()).one(html);
    let mut builder = Builder { blocks: Vec::new(), current: None, lists: Vec::new() };
    builder.visit(&dom.document, &Marks::default(), false, 0); builder.flush(false); builder.blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::Document;
    #[test]
    fn browser_html_preserves_blocks_styles_entities_and_code() {
        let blocks = from_html("<b style='font-weight:normal'><h2>Career &amp; AI</h2><p>Hello <span style='font-weight:700;font-style:italic;text-decoration:underline line-through'>ação 💚</span> <code>x &lt; 3</code> <a href='https://example.com'>link</a></p><ol start='3'><li><p>one</p></li><li>two</li></ol><blockquote><p>quote</p></blockquote><pre><code class='language-rust'>  let x = 1;\n  x + 2</code></pre><script>bad()</script></b>");
        assert_eq!(blocks.len(), 6);
        assert_eq!(blocks[0].kind, Kind::Heading(2)); assert_eq!(blocks[0].text, "Career & AI");
        let rich = &blocks[1]; let start = rich.text.find("ação").unwrap();
        let marks = &rich.spans.iter().find(|span| span.range.contains(&start)).unwrap().marks;
        assert!(marks.bold && marks.italic && marks.underline && marks.strike);
        assert_eq!(blocks[2].kind, Kind::Number(3)); assert_eq!(blocks[3].kind, Kind::Number(4));
        assert_eq!(blocks[4].kind, Kind::Quote); assert_eq!(blocks[5].kind, Kind::Code("rust".into()));
        assert_eq!(blocks[5].text, "  let x = 1;\n  x + 2");
        assert!(!blocks.iter().any(|b| b.text.contains("bad()")));
        let roundtrip = from_html(&to_html(&blocks));
        for (a, b) in blocks.iter().zip(roundtrip.iter()) { assert_eq!((&a.kind, &a.text, &a.spans), (&b.kind, &b.text, &b.spans)); }
    }
    #[test]
    fn paste_preserves_surrounding_unicode_styles_and_undo_snapshot() {
        let mut doc = Document::parse("**antes 💚** depois\n\nOutro");
        let original = doc.clone();
        let caret = doc.replace_fragment((0, "antes 💚".len()), (0, "antes 💚".len()), from_html("<p><em>inserido</em></p><h2>Título</h2>"));
        assert_eq!(doc.blocks[0].text, "antes 💚inserido");
        assert!(doc.blocks[0].is_formatted(0.."antes 💚".len(), "bold"));
        assert_eq!(doc.blocks[1].kind, Kind::Heading(2)); assert_eq!(caret, (1, "Título".len()));
        assert_eq!(doc.blocks[2].text, " depois"); assert_eq!(original.blocks[0].text, "antes 💚 depois");
    }
    #[test]
    fn partial_copy_retains_only_selected_marks_and_safe_links() {
        let doc = Document::parse("before **ação** after\n\n## Heading");
        let fragment = doc.fragment((0, 7), (0, 7 + "ação".len()));
        let html = to_html(&fragment);
        assert!(html.contains("<strong>ação</strong>")); assert!(!html.contains("before"));
        let unsafe_html = from_html("<p><a href='javascript:alert(1)'>safe text</a><u>underlined</u></p>");
        assert!(unsafe_html[0].spans.iter().all(|span| span.marks.link.is_none()));
    }
}
