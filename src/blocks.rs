//! Native block document. Markdown remains the storage and MCP interchange format.
use markdown::{mdast::Node, ParseOptions};
use std::ops::Range;
use uuid::Uuid;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Marks {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub underline: bool,
    pub code: bool,
    pub link: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub marks: Marks,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Paragraph,
    Heading(u8),
    Bullet,
    Number(u32),
    Task(bool),
    Quote,
    Code(String),
    Divider,
    Source,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub id: String,
    pub kind: Kind,
    pub text: String,
    pub spans: Vec<Span>,
    original: Option<String>,
    pub before: String,
}
impl Block {
    pub fn slice(&self, range: Range<usize>) -> Self {
        let mut block = self.clone();
        let a = self.text[..range.start].chars().count();
        let z = self.text[..range.end].chars().count();
        block.assign(self.chars()[a..z].to_vec());
        block.id = Uuid::new_v4().to_string();
        block
    }
    pub fn append_marked(&mut self, text: &str, marks: &Marks) {
        let mut chars = self.chars();
        chars.extend(text.chars().map(|ch| (ch, marks.clone())));
        self.assign(chars);
    }
    pub fn new(kind: Kind, text: String) -> Self {
        let len = text.len();
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            text,
            spans: vec![Span {
                range: 0..len,
                marks: Marks::default(),
            }],
            original: None,
            before: "\n\n".into(),
        }
    }
    pub fn invalidate(&mut self) {
        self.original = None;
    }
    fn chars(&self) -> Vec<(char, Marks)> {
        self.text
            .char_indices()
            .map(|(i, c)| {
                (
                    c,
                    self.spans
                        .iter()
                        .find(|s| s.range.contains(&i))
                        .map(|s| s.marks.clone())
                        .unwrap_or_default(),
                )
            })
            .collect()
    }
    fn assign(&mut self, chars: Vec<(char, Marks)>) {
        self.text.clear();
        self.spans.clear();
        for (c, marks) in chars {
            let start = self.text.len();
            self.text.push(c);
            let end = self.text.len();
            if let Some(last) = self.spans.last_mut().filter(|s| s.marks == marks) {
                last.range.end = end;
            } else {
                self.spans.push(Span {
                    range: start..end,
                    marks,
                });
            }
        }
        self.invalidate();
    }
    /// A Unicode-aware splice: retain existing inline styles and inherit the insertion style.
    pub fn edit(&mut self, value: String) {
        if value == self.text {
            return;
        }
        let old = self.chars();
        let new: Vec<char> = value.chars().collect();
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a.0 == **b).count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(new[prefix..].iter().rev())
            .take_while(|(a, b)| a.0 == **b)
            .count();
        let marks = old
            .get(prefix)
            .or_else(|| prefix.checked_sub(1).and_then(|i| old.get(i)))
            .map(|c| c.1.clone())
            .unwrap_or_default();
        let mut chars = old[..prefix].to_vec();
        chars.extend(
            new[prefix..new.len() - suffix]
                .iter()
                .map(|c| (*c, marks.clone())),
        );
        chars.extend_from_slice(&old[old.len() - suffix..]);
        self.assign(chars);
    }
    pub fn is_formatted(&self, range: Range<usize>, style: &str) -> bool {
        let selected: Vec<_> = self
            .spans
            .iter()
            .filter(|s| s.range.start < range.end && s.range.end > range.start)
            .collect();
        !selected.is_empty()
            && selected.iter().all(|s| match style {
                "bold" => s.marks.bold,
                "italic" => s.marks.italic,
                "strike" => s.marks.strike,
                "underline" => s.marks.underline,
                _ => s.marks.code,
            })
    }
    pub fn format(&mut self, range: Range<usize>, style: &str) {
        if range.is_empty() {
            return;
        }
        let selected: Vec<_> = self
            .spans
            .iter()
            .filter(|s| s.range.start < range.end && s.range.end > range.start)
            .collect();
        let all = !selected.is_empty()
            && selected.iter().all(|s| match style {
                "bold" => s.marks.bold,
                "italic" => s.marks.italic,
                "strike" => s.marks.strike,
                "underline" => s.marks.underline,
                _ => s.marks.code,
            });
        let mut offset = 0;
        let chars = self
            .chars()
            .into_iter()
            .map(|(c, mut m)| {
                let start = offset;
                offset += c.len_utf8();
                if range.contains(&start) {
                    match style {
                        "bold" => m.bold = !all,
                        "italic" => m.italic = !all,
                        "strike" => m.strike = !all,
                        "underline" => m.underline = !all,
                        _ => m.code = !all,
                    }
                }
                (c, m)
            })
            .collect();
        self.assign(chars);
    }
    pub fn markdown(&self) -> String {
        if let Some(raw) = &self.original {
            return raw.clone();
        }
        if let Kind::Code(lang) = &self.kind {
            let fence = "`".repeat(
                self.text
                    .split('\n')
                    .map(|l| l.chars().take_while(|c| *c == '`').count())
                    .max()
                    .unwrap_or(0)
                    .max(2)
                    + 1,
            );
            return format!("{fence}{lang}\n{}\n{fence}", self.text);
        }
        if self.kind == Kind::Source {
            return self.text.clone();
        }
        if self.kind == Kind::Divider {
            return "---".into();
        }
        let content = encode_spans(&self.text, &self.spans, 0);
        match &self.kind {
            Kind::Heading(n) => format!("{} {content}", "#".repeat(*n as usize)),
            Kind::Bullet => format!("- {}", content.replace('\n', "\n  ")),
            Kind::Number(n) => format!(
                "{n}. {}",
                content.replace('\n', &format!("\n{}", " ".repeat(n.to_string().len() + 2)))
            ),
            Kind::Task(done) => format!(
                "- [{}] {}",
                if *done { "x" } else { " " },
                content.replace('\n', "\n  ")
            ),
            Kind::Quote => content
                .lines()
                .map(|l| format!("> {l}"))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => content,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
    trailing: String,
}
impl Document {
    pub fn from_blocks(mut blocks: Vec<Block>) -> Self {
        if blocks.is_empty() { blocks.push(Block::new(Kind::Paragraph, String::new())); }
        blocks[0].before.clear();
        Self { blocks, trailing: String::new() }
    }
    pub fn fragment(&self, start: (usize, usize), end: (usize, usize)) -> Vec<Block> {
        let (a, z) = if start <= end { (start, end) } else { (end, start) };
        (a.0..=z.0).filter_map(|i| {
            let block = &self.blocks[i];
            let start = if i == a.0 { a.1 } else { 0 };
            let end = if i == z.0 { z.1 } else { block.text.len() };
            if a.0 != z.0 && ((i == a.0 && start == block.text.len()) || (i == z.0 && end == 0)) { None }
            else { Some(block.slice(start..end)) }
        }).collect()
    }
    /// Paste native blocks while preserving rich text on both sides of the selection.
    pub fn replace_fragment(&mut self, start: (usize, usize), end: (usize, usize), mut fragment: Vec<Block>) -> (usize, usize) {
        let (a, z) = if start <= end { (start, end) } else { (end, start) };
        if fragment.is_empty() { return self.replace_selection(a, z, ""); }
        let prefix = self.blocks[a.0].slice(0..a.1);
        let suffix = self.blocks[z.0].slice(z.1..self.blocks[z.0].text.len());
        let original_kind = self.blocks[a.0].kind.clone();
        let before = self.blocks[a.0].before.clone();
        for block in &mut fragment { block.id = Uuid::new_v4().to_string(); block.invalidate(); }
        let merge_prefix = !prefix.text.is_empty() && matches!(fragment[0].kind, Kind::Paragraph);
        let merge_suffix = !suffix.text.is_empty() && matches!(fragment.last().unwrap().kind, Kind::Paragraph);
        if merge_prefix {
            let mut chars = prefix.chars(); chars.extend(fragment[0].chars());
            fragment[0].assign(chars); fragment[0].kind = original_kind;
        } else if !prefix.text.is_empty() { fragment.insert(0, prefix); }
        let caret = (a.0 + fragment.len() - 1, fragment.last().unwrap().text.len());
        if merge_suffix {
            let block = fragment.last_mut().unwrap();
            let mut chars = block.chars(); chars.extend(suffix.chars()); block.assign(chars);
        } else if !suffix.text.is_empty() { fragment.push(suffix); }
        fragment[0].before = before;
        self.blocks.splice(a.0..=z.0, fragment);
        caret
    }
    pub fn parse(source: &str) -> Self {
        let root = markdown::to_mdast(source, &ParseOptions::gfm()).ok();
        let mut nodes: Vec<(&Node, Kind)> = Vec::new();
        if let Some(children) = root.as_ref().and_then(Node::children) {
            for node in children {
                if let Node::List(list) = node {
                    for (i, item) in list.children.iter().enumerate() {
                        let simple = item
                            .children()
                            .is_some_and(|c| c.len() == 1 && matches!(c[0], Node::Paragraph(_)));
                        let kind = if !simple {
                            Kind::Source
                        } else if let Node::ListItem(li) = item {
                            li.checked.map(Kind::Task).unwrap_or_else(|| {
                                if list.ordered {
                                    Kind::Number(list.start.unwrap_or(1) + i as u32)
                                } else {
                                    Kind::Bullet
                                }
                            })
                        } else {
                            Kind::Source
                        };
                        nodes.push((item, kind));
                    }
                } else {
                    nodes.push((
                        node,
                        match node {
                            Node::Heading(h) => Kind::Heading(h.depth),
                            Node::Paragraph(_) => Kind::Paragraph,
                            Node::Blockquote(q)
                                if q.children.len() == 1
                                    && matches!(q.children[0], Node::Paragraph(_)) =>
                            {
                                Kind::Quote
                            }
                            Node::Code(c) => Kind::Code(c.lang.clone().unwrap_or_default()),
                            Node::ThematicBreak(_) => Kind::Divider,
                            _ => Kind::Source,
                        },
                    ));
                }
            }
        }
        let mut blocks = Vec::new();
        let mut end = 0;
        for (node, kind) in nodes {
            let Some(p) = node.position() else { continue };
            let start = p.start.offset;
            let stop = p.end.offset;
            let kind = if kind != Kind::Source && unsupported_inline(node) {
                Kind::Source
            } else {
                kind
            };
            let mut block = Block::new(kind.clone(), String::new());
            block.before = source[end..start].into();
            block.original = Some(source[start..stop].into());
            match node {
                Node::Code(c) => block.text = c.value.clone(),
                _ if kind == Kind::Source => block.text = source[start..stop].into(),
                _ => inline(node, Marks::default(), &mut block),
            }
            if block.spans.is_empty() {
                block.spans.push(Span {
                    range: 0..block.text.len(),
                    marks: Marks::default(),
                });
            }
            blocks.push(block);
            end = stop;
        }
        if blocks.is_empty() {
            let mut b = Block::new(Kind::Paragraph, source.into());
            b.before = String::new();
            b.original = Some(source.into());
            blocks.push(b);
            end = source.len();
        }
        Self {
            blocks,
            trailing: source[end..].into(),
        }
    }
    pub fn markdown(&self) -> String {
        let mut s = String::new();
        for block in &self.blocks {
            s.push_str(&block.before);
            s.push_str(&block.markdown());
        }
        s.push_str(&self.trailing);
        s
    }
    /// Replace one continuous selection while retaining the unselected rich text.
    pub fn replace_selection(
        &mut self,
        start: (usize, usize),
        end: (usize, usize),
        text: &str,
    ) -> (usize, usize) {
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let first = self.blocks[start.0].chars();
        let last = self.blocks[end.0].chars();
        let a = self.blocks[start.0].text[..start.1].chars().count();
        let z = self.blocks[end.0].text[..end.1].chars().count();
        let marks = first
            .get(a)
            .or_else(|| a.checked_sub(1).and_then(|i| first.get(i)))
            .map(|c| c.1.clone())
            .unwrap_or_default();
        let mut chars = first[..a].to_vec();
        chars.extend(text.chars().map(|c| (c, marks.clone())));
        chars.extend_from_slice(&last[z..]);
        self.blocks[start.0].assign(chars);
        self.blocks.drain(start.0 + 1..end.0 + 1);
        if self.blocks[start.0].text.is_empty() && !matches!(self.blocks[start.0].kind, Kind::Code(_)) {
            self.blocks[start.0].kind = Kind::Paragraph;
        }
        (start.0, start.1 + text.len())
    }
    pub fn split(&mut self, index: usize, range: Range<usize>) -> usize {
        let b = &mut self.blocks[index];
        if b.text.is_empty()
            && matches!(
                b.kind,
                Kind::Bullet | Kind::Number(_) | Kind::Task(_) | Kind::Quote
            )
        {
            b.kind = Kind::Paragraph;
            b.invalidate();
            return index;
        }
        let chars = b.chars();
        let a = b.text[..range.start].chars().count();
        let z = b.text[..range.end].chars().count();
        let kind = match b.kind {
            Kind::Heading(_) | Kind::Divider => Kind::Paragraph,
            Kind::Task(_) => Kind::Task(false),
            Kind::Number(n) => Kind::Number(n + 1),
            _ => b.kind.clone(),
        };
        let mut next = Block::new(kind, String::new());
        next.assign(chars[z..].to_vec());
        if matches!(b.kind, Kind::Bullet | Kind::Number(_) | Kind::Task(_)) {
            next.before = "\n".into();
        }
        b.assign(chars[..a].to_vec());
        self.blocks.insert(index + 1, next);
        index + 1
    }
    pub fn merge_previous(&mut self, index: usize) -> Option<(usize, usize)> {
        if index == 0 || index >= self.blocks.len() {
            return None;
        }
        if matches!(self.blocks[index].kind, Kind::Source | Kind::Divider)
            || matches!(self.blocks[index - 1].kind, Kind::Source | Kind::Divider)
        {
            return None;
        }
        let removed = self.blocks.remove(index);
        let target = &mut self.blocks[index - 1];
        let cursor = target.text.len();
        let mut chars = target.chars();
        chars.extend(removed.chars());
        target.assign(chars);
        Some((index - 1, cursor))
    }
    /// Move a stable block to the insertion slot before/after another block.
    pub fn move_to(&mut self, id: &str, target: &str, after: bool) -> bool {
        let Some(from) = self.blocks.iter().position(|b| b.id == id) else {
            return false;
        };
        let Some(to) = self.blocks.iter().position(|b| b.id == target) else {
            return false;
        };
        let slot = to + usize::from(after);
        let destination = if from < slot { slot - 1 } else { slot };
        if from == destination {
            return false;
        }
        let before: Vec<_> = self.blocks.iter().map(|b| b.before.clone()).collect();
        let block = self.blocks.remove(from);
        self.blocks.insert(destination, block);
        for (i, block) in self.blocks.iter_mut().enumerate() {
            // Keep the document's leading whitespace. Interior boundaries must also work
            // when a list item is moved next to a paragraph or a heading.
            block.before = if i == 0 {
                before[0].clone()
            } else {
                "\n\n".into()
            };
        }
        true
    }
    pub fn remove(&mut self, index: usize) {
        let before = self.blocks[index].before.clone();
        self.blocks.remove(index);
        if let Some(next) = self.blocks.get_mut(index) {
            next.before = before;
        }
        if self.blocks.is_empty() {
            let mut b = Block::new(Kind::Paragraph, String::new());
            b.before = String::new();
            self.blocks.push(b);
        }
    }
}
fn encode_spans(text: &str, spans: &[Span], level: usize) -> String {
    if level == 6 {
        let mut out = String::new();
        for span in spans {
            for c in text[span.range.clone()].chars() {
                if c == '\n' {
                    out.push_str("  \n");
                    continue;
                }
                if "\\`*_[]<>~#|.!()+-=".contains(c) {
                    out.push('\\');
                }
                out.push(c);
            }
        }
        return out;
    }
    fn key(m: &Marks, level: usize) -> String {
        match level {
            0 => m.link.clone().unwrap_or_default(),
            1 => m.bold.to_string(),
            2 => m.italic.to_string(),
            3 => m.strike.to_string(),
            4 => m.underline.to_string(),
            _ => m.code.to_string(),
        }
    }
    let mut out = String::new();
    let mut start = 0;
    while start < spans.len() {
        let value = key(&spans[start].marks, level);
        let mut end = start + 1;
        while end < spans.len() && key(&spans[end].marks, level) == value {
            end += 1;
        }
        let group = &spans[start..end];
        let enabled = if level == 0 {
            !value.is_empty()
        } else {
            value == "true"
        };
        let mut inner = if level == 5 && enabled {
            group
                .iter()
                .map(|s| &text[s.range.clone()])
                .collect::<String>()
        } else {
            encode_spans(text, group, level + 1)
        };
        if enabled {
            if level == 0 {
                inner = format!(
                    "[{inner}]({})",
                    value.replace(' ', "%20").replace(')', "%29")
                );
            } else if level == 4 {
                inner = format!("<u>{inner}</u>");
            } else if level == 5 {
                let mut max = 0;
                let mut run = 0;
                for c in inner.chars() {
                    if c == '`' {
                        run += 1;
                        max = max.max(run);
                    } else {
                        run = 0;
                    }
                }
                let fence = "`".repeat(max + 1);
                let pad = if inner.starts_with('`')
                    || inner.ends_with('`')
                    || (inner.starts_with(' ') && inner.ends_with(' ') && !inner.trim().is_empty())
                {
                    " "
                } else {
                    ""
                };
                inner = format!("{fence}{pad}{inner}{pad}{fence}");
            } else {
                let lead = inner.len() - inner.trim_start().len();
                let tail = inner.trim_end().len();
                if lead < tail {
                    let delimiter = match level {
                        1 => "**",
                        2 => "*",
                        _ => "~~",
                    };
                    inner = format!(
                        "{}{delimiter}{}{delimiter}{}",
                        &inner[..lead],
                        &inner[lead..tail],
                        &inner[tail..]
                    );
                }
            }
        }
        out.push_str(&inner);
        start = end;
    }
    out
}
fn unsupported_inline(node: &Node) -> bool {
    matches!(
        node,
        Node::Image(_)
            | Node::ImageReference(_)
            | Node::LinkReference(_)
            | Node::FootnoteReference(_)
            | Node::InlineMath(_)
    ) || matches!(node, Node::Html(h) if !matches!(h.value.as_str(), "<u>" | "</u>"))
        || node
            .children()
            .is_some_and(|children| children.iter().any(unsupported_inline))
}
fn inline(node: &Node, mut marks: Marks, block: &mut Block) {
    let value = match node {
        Node::Text(t) => Some(t.value.replace('\n', " ")),
        Node::InlineCode(t) => {
            marks.code = true;
            Some(t.value.clone())
        }
        Node::Strong(_) => {
            marks.bold = true;
            None
        }
        Node::Emphasis(_) => {
            marks.italic = true;
            None
        }
        Node::Delete(_) => {
            marks.strike = true;
            None
        }
        Node::Link(l) => {
            marks.link = Some(l.url.clone());
            None
        }
        Node::Break(_) => Some("\n".into()),
        // Keep uncommon inline constructs as source blocks instead of deleting their data.
        Node::Image(i) => Some(format!("![{}]({})", i.alt, i.url)),
        Node::Html(h) => Some(h.value.clone()),
        _ => None,
    };
    if let Some(value) = value {
        let start = block.text.len();
        block.text.push_str(&value);
        block.spans.push(Span {
            range: start..block.text.len(),
            marks,
        });
    } else if let Some(children) = node.children() {
        let mut underline_depth = usize::from(marks.underline);
        for child in children {
            if let Node::Html(h) = child {
                match h.value.as_str() {
                    "<u>" => {
                        underline_depth += 1;
                        continue;
                    }
                    "</u>" => {
                        underline_depth = underline_depth.saturating_sub(1);
                        continue;
                    }
                    _ => {}
                }
            }
            let mut child_marks = marks.clone();
            child_marks.underline = underline_depth > 0;
            inline(child, child_marks, block);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuous_selection_preserves_unicode_styles_and_surrounding_blocks() {
        let source = "**Olá** 💚 fim\n\n## título\n\núltimo **ação** ok\n\nfora";
        for reversed in [false, true] {
            let mut d = Document::parse(source);
            let first_id = d.blocks[0].id.clone();
            let outside_id = d.blocks[3].id.clone();
            let a = (0, "Olá ".len());
            let z = (2, "último ação".len());
            let (a, z) = if reversed { (z, a) } else { (a, z) };
            let caret = d.replace_selection(a, z, "novo");
            assert_eq!(caret, (0, "Olá novo".len()));
            assert_eq!(d.markdown(), "**Olá** novo ok\n\nfora");
            assert_eq!(d.blocks[0].id, first_id);
            assert_eq!(d.blocks[1].id, outside_id);
        }
    }
    #[test]
    fn deleting_complete_document_leaves_one_editable_paragraph() {
        let mut d = Document::parse("# Título\n\n💚 ação\n\n- item");
        let last = d.blocks.len() - 1;
        d.replace_selection((0, 0), (last, d.blocks[last].text.len()), "");
        assert_eq!(d.blocks.len(), 1);
        assert_eq!(d.blocks[0].kind, Kind::Paragraph);
        assert_eq!(d.markdown(), "");
    }
    #[test]
    fn underline_roundtrip_with_other_marks_and_unicode() {
        let mut d = Document::parse("Olá **ação** e vídeo");
        d.blocks[0].format(0.."Olá ação".len(), "underline");
        let exported = d.markdown();
        assert!(exported.contains("<u>"));
        let again = Document::parse(&exported);
        assert_eq!(again.blocks[0].kind, Kind::Paragraph);
        assert_eq!(again.blocks[0].text, d.blocks[0].text);
        assert_eq!(again.blocks[0].chars(), d.blocks[0].chars());
        d.blocks[0].format(0.."Olá ação".len(), "underline");
        assert!(!d.markdown().contains("<u>"));
    }
    #[test]
    fn drag_moves_preserve_identity_styles_and_block_boundaries() {
        let mut d = Document::parse("# título\n\nparágrafo **forte**\n\n- item\n- último");
        let first = d.blocks[0].id.clone();
        let last = d.blocks[3].id.clone();
        assert!(d.move_to(&first, &last, true));
        assert_eq!(d.blocks[3].id, first);
        assert_eq!(d.blocks[3].kind, Kind::Heading(1));
        assert!(d.markdown().starts_with("parágrafo **forte**"));
        assert_eq!(Document::parse(&d.markdown()).blocks.len(), 4);
        assert!(!d.move_to(&first, &first, false));
        let head = d.blocks[0].id.clone();
        assert!(d.move_to(&first, &head, false));
        assert_eq!(d.blocks[0].id, first);
        assert_eq!(
            Document::parse(&d.markdown()).blocks[0].kind,
            Kind::Heading(1)
        );
        assert!(!d.move_to(&head, &first, true));
    }
    #[test]
    fn lossless_import() {
        for s in [
            "\n# Olá **mundo**\n\n- a\n- *b*\n\n> oi\n",
            "- item\n  - nested\n\n| a | b |\n| - | - |\n| c | d |",
            "```rs\nlet a = 1;\n```",
            "",
            "a\n\nb\n\n",
        ] {
            assert_eq!(Document::parse(s).markdown(), s);
        }
    }
    #[test]
    fn unicode_edit_preserves_marks() {
        let mut d = Document::parse("Olá **ação 💚** e *vídeo*");
        assert_eq!(d.blocks[0].text, "Olá ação 💚 e vídeo");
        d.blocks[0].edit("Olá ação 💚! e vídeo".into());
        let exported = d.markdown();
        let again = Document::parse(&exported);
        assert_eq!(again.blocks[0].text, d.blocks[0].text);
        assert!(exported.contains("**ação 💚**"));
        assert!(exported.contains("*vídeo*"));
    }
    #[test]
    fn split_merge_styles() {
        let mut d = Document::parse("# Olá **mundo**");
        let n = d.split(0, 5..5);
        assert_eq!(d.blocks[n].kind, Kind::Paragraph);
        assert_eq!(d.blocks[n].text, "mundo");
        let (i, p) = d.merge_previous(n).unwrap();
        assert_eq!((i, p), (0, 5));
        assert_eq!(d.blocks[0].text, "Olá mundo");
        assert_eq!(d.markdown(), "# Olá **mundo**");
    }
    #[test]
    fn task_and_empty_list_exit() {
        let mut d = Document::parse("- [x] fazer");
        d.split(0, 5..5);
        assert_eq!(d.blocks[1].kind, Kind::Task(false));
        d.split(1, 0..0);
        assert_eq!(d.blocks[1].kind, Kind::Paragraph);
    }
    #[test]
    fn selection_format_and_literal_markdown() {
        let mut d = Document::parse("Olá mundo");
        d.blocks[0].format(5..10, "bold");
        assert_eq!(d.markdown(), "Olá **mundo**");
        d.blocks[0].format(5..10, "bold");
        d.blocks[0].edit("Olá *literal*".into());
        let m = d.markdown();
        assert_eq!(Document::parse(&m).blocks[0].text, "Olá *literal*");
    }
    #[test]
    fn unsupported_blocks_survive_adjacent_edits() {
        let source = "Abertura\n\n| A | B |\n| - | - |\n| um | dois |\n\n![imagem](file.png)\n\n- pai\n  - filho\n";
        let mut d = Document::parse(source);
        d.blocks[0].edit("Nova abertura".into());
        assert_eq!(
            d.markdown(),
            source.replacen("Abertura", "Nova abertura", 1)
        );
        assert!(d.blocks.iter().skip(1).all(|b| b.kind == Kind::Source));
    }
    #[test]
    fn nested_marks_round_trip() {
        for source in [
            "**forte *e itálico* fim**",
            "~~risco~~ e [link **forte**](https://example.com)",
            "`código` e normal",
        ] {
            let mut d = Document::parse(source);
            let text = format!("{}!", d.blocks[0].text);
            d.blocks[0].edit(text.clone());
            let imported = Document::parse(&d.markdown());
            assert_eq!(imported.blocks[0].text, text);
            assert_eq!(imported.blocks[0].chars(), d.blocks[0].chars());
        }
    }
    #[test]
    fn line_breaks_follow_markdown_semantics() {
        let soft = Document::parse("primeira\nsegunda");
        assert_eq!(soft.blocks[0].text, "primeira segunda");
        assert_eq!(soft.markdown(), "primeira\nsegunda");
        let hard = Document::parse("primeira  \nsegunda");
        assert_eq!(hard.blocks[0].text, "primeira\nsegunda");
        let mut edited = Document::parse("primeira");
        edited.blocks[0].edit("primeira\nsegunda".into());
        assert_eq!(
            Document::parse(&edited.markdown()).blocks[0].text,
            "primeira\nsegunda"
        );
    }
    #[test]
    fn deleting_last_keeps_editable_block() {
        let mut d = Document::parse("a");
        d.remove(0);
        assert_eq!(d.blocks.len(), 1);
        assert_eq!(d.markdown(), "");
    }
}
