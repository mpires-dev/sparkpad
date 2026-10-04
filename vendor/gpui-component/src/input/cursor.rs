use std::ops::{Range, RangeBounds};

/// A selection in the text, represented by start and end byte indices.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    pub start: usize,
    pub end: usize,
}

/// FORK (Fennel): unidade de texto em que a seleção por mouse cresce, no espírito do
/// `TextGranularity` do Blink.
///
/// Definida pelo número de cliques e **preservada** depois: é o que faz o arraste e o shift+clique
/// que vêm em seguida grudarem na fronteira da palavra (ou da linha), como no navegador, em vez de
/// andarem de caractere em caractere. Ver `vendor/gpui-component/PATCHES.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionGranularity {
    /// 1 clique — sem arredondamento (comportamento original do crate).
    #[default]
    Character,
    /// 2 cliques — a seleção gruda nas fronteiras de palavra.
    Word,
    /// 3+ cliques — a seleção gruda nas fronteiras de parágrafo (trecho entre `\n`).
    Paragraph,
}

impl Selection {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Clears the selection, setting start and end to 0.
    pub fn clear(&mut self) {
        self.start = 0;
        self.end = 0;
    }

    /// Checks if the given offset is within the selection range.
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }
}

impl From<Range<usize>> for Selection {
    fn from(value: Range<usize>) -> Self {
        Self::new(value.start, value.end)
    }
}
impl From<Selection> for Range<usize> {
    fn from(value: Selection) -> Self {
        value.start..value.end
    }
}
impl RangeBounds<usize> for Selection {
    fn start_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Included(&self.start)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Excluded(&self.end)
    }
}

#[cfg(test)]
mod tests {
    use crate::input::Position;

    #[test]
    fn test_line_column_from_to() {
        assert_eq!(
            Position::new(1, 2),
            Position {
                line: 1,
                character: 2
            }
        );
    }
}
