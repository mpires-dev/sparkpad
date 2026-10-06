//! Pure drop planning: indexed ancestry, no database reads or tree rebuilds on pointer movement.
use crate::{
    note_tree::{NoteTree, TreeRow},
    store::NoteSummary,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Before,
    After,
    Inside,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destination {
    pub owner: String,
    pub parent: Option<String>,
    pub group: Option<String>,
    pub anchor: Option<String>,
    pub edge: Edge,
    pub depth: usize,
}
/// Indentation is capped visually; interpret horizontal movement relative to the
/// current level so deeply nested pages are not accidentally promoted hundreds of levels.
pub fn desired_depth(depth: usize, x: f32) -> usize {
    let indent = (depth as f32 * 12.).min(72.);
    let outdent = ((indent - (x - 8.)).max(0.) / 12.).ceil() as usize;
    depth.saturating_sub(outdent)
}
pub fn plan(
    tree: &NoteTree,
    notes: &[NoteSummary],
    source: &str,
    row: TreeRow,
    vertical: f32,
    depth: usize,
) -> Option<Destination> {
    if row.spacer {
        return None;
    }
    if let Some(group) = row.group {
        return Some(Destination {
            owner: format!(
                "{}{}",
                if row.empty { "empty:" } else { "" },
                tree.groups[group].id
            ),
            parent: None,
            group: Some(tree.groups[group].id.clone()),
            anchor: None,
            edge: Edge::Inside,
            depth: 0,
        });
    }
    let mut index = row.index;
    let edge = if row.empty || (0.25..=0.75).contains(&vertical) {
        Edge::Inside
    } else if vertical < 0.25 {
        Edge::Before
    } else {
        Edge::After
    };
    let target_depth = if edge == Edge::Inside {
        row.depth + usize::from(!row.empty)
    } else {
        depth.min(row.depth)
    };
    if edge != Edge::Inside {
        for _ in target_depth..row.depth {
            index = tree.note_index(notes[index].parent_id.as_deref()?)?;
        }
    }
    let note = &notes[index];
    if source == note.id {
        return None;
    }
    let parent = if edge == Edge::Inside {
        Some(note.id.clone())
    } else {
        note.parent_id.clone()
    };
    let mut ancestor = parent.as_deref();
    while let Some(id) = ancestor {
        if id == source {
            return None;
        }
        ancestor = notes[tree.note_index(id)?].parent_id.as_deref();
    }
    let display = if edge == Edge::After {
        tree.subtree_end(index).unwrap_or(row)
    } else if edge == Edge::Before {
        TreeRow {
            index,
            empty: false,
            ..row
        }
    } else {
        row
    };
    let owner = format!(
        "{}{}",
        if display.empty { "empty:" } else { "" },
        notes[display.index].id
    );
    Some(Destination {
        owner,
        group: if parent.is_none() {
            note.group_id.clone()
        } else {
            None
        },
        parent,
        anchor: if edge == Edge::Inside {
            None
        } else {
            Some(note.id.clone())
        },
        edge,
        depth: target_depth,
    })
}
#[cfg(test)]
mod tests {
    use super::{plan, Edge};
    use crate::{note_tree::NoteTree, store::NoteSummary};
    #[test]
    fn capped_indentation_keeps_deep_pages_at_their_level() {
        assert_eq!(super::desired_depth(2000, 110.), 2000);
        assert_eq!(super::desired_depth(2000, 68.), 1999);
        assert_eq!(super::desired_depth(2, 50.), 2);
        assert_eq!(super::desired_depth(2, 20.), 1);
        assert_eq!(super::desired_depth(0, 0.), 0);
    }
    #[test]
    fn drop_zones_reorder_nest_outdent_and_reject_cycles() {
        let notes: Vec<_> = (0..4)
            .map(|i| NoteSummary {
                id: i.to_string(),
                title: i.to_string(),
                icon: None,
                revision: 1,
                group_id: None,
                parent_id: match i {
                    1 => Some("0".into()),
                    2 => Some("1".into()),
                    _ => None,
                },
            })
            .collect();
        let mut tree = NoteTree::default();
        tree.expanded.extend(["0".into(), "1".into()]);
        tree.set_notes(&notes);
        let row = *tree
            .rows
            .iter()
            .find(|r| r.is_page() && r.index == 2)
            .unwrap();
        let before = plan(&tree, &notes, "3", row, 0.1, 2).unwrap();
        assert_eq!(before.edge, Edge::Before);
        assert_eq!(before.parent.as_deref(), Some("1"));
        let out = plan(&tree, &notes, "3", row, 0.9, 0).unwrap();
        assert_eq!(out.anchor.as_deref(), Some("0"));
        assert_eq!(out.parent, None);
        assert_eq!(out.depth, 0);
        let inside = plan(&tree, &notes, "3", row, 0.5, 0).unwrap();
        assert_eq!(inside.parent.as_deref(), Some("2"));
        assert_eq!(inside.depth, 3);
        assert!(plan(&tree, &notes, "0", row, 0.5, 2).is_none());
        assert!(plan(&tree, &notes, "1", row, 0.1, 2).is_none());
        assert!(plan(&tree, &notes, "2", row, 0.5, 2).is_none());
    }
}
