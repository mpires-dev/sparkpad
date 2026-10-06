use crate::{
    note_tree::{NoteTree, TreeRow},
    store::NoteSummary,
};
use empire_ui::motion::Tween;
use std::collections::{HashMap, HashSet};

pub struct AnimatedRow {
    pub key: String,
    pub row: TreeRow,
    pub amount: Tween,
    sample: f32,
}
#[derive(Default)]
pub struct TreeMotion {
    pub rows: Vec<AnimatedRow>,
}
impl TreeMotion {
    pub fn sync(&mut self, tree: &NoteTree, notes: &[NoteSummary], animate: bool) {
        let key = |i: usize, r: &TreeRow| {
            if r.spacer {
                format!("space:{}", tree.groups[tree.rows[i + 1].group.unwrap()].id)
            } else if let Some(g) = r.group {
                format!(
                    "{}group:{}",
                    if r.empty { "empty:" } else { "" },
                    tree.groups[g].id
                )
            } else {
                format!(
                    "{}page:{}",
                    if r.empty { "empty:" } else { "" },
                    notes[r.index].id
                )
            }
        };
        let targets: Vec<_> = tree
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| (key(i, r), *r))
            .collect();
        if !animate {
            self.rows = targets
                .into_iter()
                .map(|(key, row)| AnimatedRow {
                    key,
                    row,
                    amount: Tween::new(1.),
                    sample: 1.,
                })
                .collect();
            return;
        }
        let wanted: HashSet<_> = targets.iter().map(|(k, _)| k.clone()).collect();
        let previous = std::mem::take(&mut self.rows);
        let mut old = HashMap::new();
        let mut exits: HashMap<Option<String>, Vec<AnimatedRow>> = HashMap::new();
        let mut anchor = None;
        for mut entry in previous {
            let visible = wanted.contains(&entry.key);
            entry.amount.set(if visible { 1. } else { 0. }, 180);
            if visible {
                anchor = Some(entry.key.clone());
                old.insert(entry.key.clone(), entry);
            } else {
                exits.entry(anchor.clone()).or_default().push(entry);
            }
        }
        self.rows.extend(exits.remove(&None).unwrap_or_default());
        for (key, row) in targets {
            let mut entry = old.remove(&key).unwrap_or_else(|| {
                let mut amount = Tween::new(0.);
                amount.set(1., 180);
                AnimatedRow {
                    key: key.clone(),
                    row,
                    amount,
                    sample: 0.,
                }
            });
            entry.row = row;
            self.rows.push(entry);
            self.rows
                .extend(exits.remove(&Some(key)).unwrap_or_default());
        }
    }
    pub fn advance(&mut self) -> bool {
        let before = self.rows.len();
        self.rows
            .retain(|r| r.amount.target() != 0. || r.amount.moving());
        before != self.rows.len()
    }
    // Invalidate only heights that changed; keep the rest of the virtual list cached.
    pub fn changed_ranges(&mut self) -> Vec<std::ops::Range<usize>> {
        let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
        for (i, row) in self.rows.iter_mut().enumerate() {
            let value = row.amount.value();
            if value != row.sample {
                row.sample = value;
                if let Some(last) = ranges.last_mut().filter(|r| r.end == i) {
                    last.end = i + 1;
                } else {
                    ranges.push(i..i + 1);
                }
            }
        }
        ranges
    }
    pub fn moving(&self) -> bool {
        self.rows.iter().any(|r| r.amount.moving())
    }
}

#[cfg(test)]
mod tests {
    use super::TreeMotion;
    use crate::{note_tree::NoteTree, store::NoteSummary};
    use empire_ui::motion::Tween;
    use std::time::{Duration, Instant};
    #[test]
    fn reversing_a_transition_keeps_its_current_value() {
        let now = Instant::now();
        let duration = Duration::from_millis(200);
        let mut tween = Tween::new(0.);
        tween.set_at(1., duration, now);
        let midpoint = now + Duration::from_millis(80);
        let before = tween.value_at(midpoint);
        assert!(before > 0. && before < 1.);
        tween.set_at(0., duration, midpoint);
        assert_eq!(before, tween.value_at(midpoint));
        assert_eq!(tween.value_at(midpoint + duration), 0.);
        let mut settled = Tween::new(1.);
        settled.set(1., 200);
        assert!(!settled.moving());
    }
    #[test]
    fn collapsing_reversing_and_refreshing_preserve_row_identity() {
        let notes: Vec<_> = (0..3)
            .map(|i| NoteSummary {
                id: i.to_string(),
                title: i.to_string(),
                parent_id: if i == 0 { None } else { Some("0".into()) },
                group_id: None,
                icon: None,
                revision: 1,
            })
            .collect();
        let mut tree = NoteTree::default();
        tree.set_notes(&notes);
        tree.toggle("0", &notes);
        let mut motion = TreeMotion::default();
        motion.sync(&tree, &notes, false);
        let keys: Vec<_> = motion.rows.iter().map(|r| r.key.clone()).collect();
        tree.toggle("0", &notes);
        motion.sync(&tree, &notes, true);
        assert_eq!(
            motion
                .rows
                .iter()
                .map(|r| r.key.clone())
                .collect::<Vec<_>>(),
            keys
        );
        assert_eq!(motion.rows[1].amount.target(), 0.);
        tree.toggle("0", &notes);
        motion.sync(&tree, &notes, true);
        assert_eq!(
            motion
                .rows
                .iter()
                .map(|r| r.key.clone())
                .collect::<Vec<_>>(),
            keys
        );
        assert!(motion.rows.iter().all(|r| r.amount.target() == 1.));
        tree.toggle("0", &notes);
        motion.sync(&tree, &notes, false);
        assert_eq!(motion.rows.len(), 1);
        assert!(!motion.moving());
        assert!(motion.changed_ranges().is_empty());
    }
}
