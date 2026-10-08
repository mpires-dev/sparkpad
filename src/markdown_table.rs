use crate::blocks::{Block, Kind};
use markdown::mdast::{AlignKind, Node};
use std::ops::Range;

#[derive(Clone)]
pub struct MarkdownTable {
    pub rows: Vec<Vec<Block>>,
    pub align: Vec<AlignKind>,
    ranges: Vec<Vec<Range<usize>>>,
    missing: Vec<Vec<bool>>,
}
impl MarkdownTable {
    pub fn parse(source: &str) -> Option<Self> {
        let root = markdown::to_mdast(source, &markdown::ParseOptions::gfm()).ok()?;
        let Node::Table(table) = root.children()?.first()? else {
            return None;
        };
        let cols = table.align.len();
        let mut rows = Vec::new();
        let mut ranges = Vec::new();
        let mut missing = Vec::new();
        for row in &table.children {
            let mut cells = Vec::new();
            let mut positions = Vec::new();
            let mut omitted = Vec::new();
            for node in row.children()?.iter().take(cols) {
                let p = node.position()?;
                let children = node.children()?;
                let (start, end) =
                    if let (Some(first), Some(last)) = (children.first(), children.last()) {
                        (first.position()?.start.offset, last.position()?.end.offset)
                    } else {
                        let raw = &source[p.start.offset..p.end.offset];
                        let trimmed = raw
                            .trim_start()
                            .strip_prefix('|')
                            .unwrap_or(raw.trim_start())
                            .trim_start();
                        let start = p.start.offset + raw.len() - trimmed.len();
                        (start, start)
                    };
                cells.push(crate::blocks::table_cell(node, &source[start..end]));
                positions.push(start..end);
                omitted.push(false);
            }
            let end = row.position()?.end.offset;
            while cells.len() < cols {
                cells.push(Block::new(Kind::Paragraph, String::new()));
                positions.push(end..end);
                omitted.push(true);
            }
            rows.push(cells);
            ranges.push(positions);
            missing.push(omitted);
        }
        Some(Self {
            rows,
            align: table.align.clone(),
            ranges,
            missing,
        })
    }
    /// Replace only the edited cell, preserving the rest of the original Markdown.
    pub fn replace_cell(&self, source: &str, row: usize, column: usize, cell: &Block) -> String {
        let mut text = source.to_owned();
        if let Some(range) = self.ranges.get(row).and_then(|r| r.get(column)) {
            // Short GFM rows omit trailing cells; materialize them on the first edit.
            if self.missing[row][column] {
                let mut next = self.clone();
                next.rows[row][column] = cell.clone();
                return next.markdown();
            }
            text.replace_range(range.clone(), &escape_cell(&cell.markdown()));
        }
        text
    }
    /// Update one cell and shift subsequent byte ranges without parsing all rows.
    /// Short rows, whitespace-only cells and pasted line breaks retain the parser fallback.
    pub fn edit_cell(&mut self,source:&str,row:usize,column:usize,cell:&Block)->String{
        let next=self.replace_cell(source,row,column,cell);
        let Some(range)=self.ranges.get(row).and_then(|r|r.get(column)).cloned()else{return next};
        if self.missing[row][column]||cell.text.trim().is_empty()||cell.text.trim()!=cell.text||cell.text.contains(['\n','\r']){
            if let Some(model)=Self::parse(&next){*self=model;}return next;
        }
        let replacement=escape_cell(&cell.markdown());let delta=replacement.len() as isize-range.len() as isize;
        self.rows[row][column]=cell.clone();self.ranges[row][column]=range.start..range.start+replacement.len();
        for (r,ranges) in self.ranges.iter_mut().enumerate(){for (c,span) in ranges.iter_mut().enumerate(){if (r,c)>(row,column){span.start=span.start.checked_add_signed(delta).unwrap();span.end=span.end.checked_add_signed(delta).unwrap();}}}
        next
    }
    pub fn add_row(&mut self) {
        self.rows.push(
            (0..self.align.len())
                .map(|_| Block::new(Kind::Paragraph, String::new()))
                .collect(),
        );
    }
    pub fn add_column(&mut self) {
        self.align.push(AlignKind::None);
        for row in &mut self.rows {
            row.push(Block::new(Kind::Paragraph, String::new()));
        }
    }
    pub fn remove_row(&mut self, row: usize) {
        if row > 0 && row < self.rows.len() {
            self.rows.remove(row);
        }
    }
    pub fn remove_column(&mut self, col: usize) {
        if self.align.len() > 1 && col < self.align.len() {
            self.align.remove(col);
            for row in &mut self.rows {
                row.remove(col);
            }
        }
    }
    pub fn markdown(&self) -> String {
        let mut lines = Vec::new();
        for (i, row) in self.rows.iter().enumerate() {
            lines.push(format!(
                "| {} |",
                row.iter()
                    .map(|c| escape_cell(&c.markdown()))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
            if i == 0 {
                lines.push(format!(
                    "| {} |",
                    self.align
                        .iter()
                        .map(|a| match a {
                            AlignKind::Left => ":---",
                            AlignKind::Right => "---:",
                            AlignKind::Center => ":---:",
                            AlignKind::None => "---",
                        })
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
            }
        }
        lines.join("\n")
    }
}
fn escape_cell(value: &str) -> String {
    let mut result = String::new();
    let mut backslashes = 0;
    let value = value.replace("  \n", "<br>").replace("\\\n", "<br>");
    for c in value.chars() {
        if c == '|' && backslashes % 2 == 0 {
            result.push('\\');
        }
        if c == '\n' || c == '\r' {
            result.push_str("<br>");
        } else {
            result.push(c);
        }
        backslashes = if c == '\\' { backslashes + 1 } else { 0 };
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incremental_cells_preserve_subsequent_ranges(){
        let mut source="| Name | Value |\n| --- | --- |\n| **café** | 🌱 |\n| | last |".to_owned();let mut model=MarkdownTable::parse(&source).unwrap();
        for (r,c,text) in [(1,0,"larger café"),(2,1,"x | y"),(2,0,"filled"),(0,0,"N"),(1,1,""),(1,1," spaced "),(2,1,"   "),(2,1,"refilled")]{let mut cell=model.rows[r][c].clone();cell.edit(text.into());source=model.edit_cell(&source,r,c,&cell);let parsed=MarkdownTable::parse(&source).unwrap();assert_eq!(model.ranges,parsed.ranges);for (a,b)in model.rows.iter().flatten().zip(parsed.rows.iter().flatten()){assert_eq!(a.text,b.text);assert_eq!(a.markdown(),b.markdown());}}
    }

    #[test]
    fn visual_edit_preserves_other_cells_and_styles() {
        let source = "| Name | Value |\n| :--- | ---: |\n| **Café** | `a\\|b` |";
        let table = MarkdownTable::parse(source).unwrap();
        assert_eq!(table.rows[1][0].text, "Café");
        assert_eq!(table.rows[1][1].text, "a|b");
        let mut cell = table.rows[1][0].clone();
        cell.edit("Café 🌱".into());
        let next = table.replace_cell(source, 1, 0, &cell);
        assert!(next.contains("**Café 🌱**"));
        assert!(next.contains("`a\\|b`"));
        assert!(next.contains("| :--- | ---: |"));
        let parsed = MarkdownTable::parse(&next).unwrap();
        assert_eq!(parsed.rows[1][0].text, "Café 🌱");
    }
    #[test]
    fn structure_and_empty_cells_roundtrip() {
        let mut table = MarkdownTable::parse("| A | B |\n| --- | --- |\n| | |").unwrap();
        table.add_row();
        table.add_column();
        table.rows[2][2].edit("x | y".into());
        let source = table.markdown();
        let again = MarkdownTable::parse(&source).unwrap();
        assert_eq!(again.rows[2][2].text, "x | y");
        table.remove_row(0);
        assert_eq!(table.rows.len(), 3);
        table.remove_row(1);
        table.remove_column(0);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.align.len(), 2);
        assert_eq!(
            MarkdownTable::parse(&table.markdown()).unwrap().rows[1][1].text,
            "x | y"
        );
    }
    #[test]
    fn multiline_cells_remain_in_one_row() {
        let source = "| A | B |\n| --- | --- |\n| First<br>Second | Last |";
        let table = MarkdownTable::parse(source).unwrap();
        assert_eq!(table.rows[1][0].text, "First\nSecond");
        let mut cell = table.rows[1][0].clone();
        cell.edit("First\nSecond\nThird".into());
        let next = table.replace_cell(source, 1, 0, &cell);
        assert_eq!(
            MarkdownTable::parse(&next).unwrap().rows[1][0].text,
            "First\nSecond\nThird"
        );
    }
    #[test]
    fn omitted_cells_are_editable() {
        let s = "| A | B | C |\n| --- | --- | --- |\n| one |";
        let t = MarkdownTable::parse(s).unwrap();
        let mut cell = t.rows[1][2].clone();
        cell.edit("three".into());
        let next = t.replace_cell(s, 1, 2, &cell);
        assert_eq!(
            MarkdownTable::parse(&next).unwrap().rows[1][2].text,
            "three"
        );
    }
}
