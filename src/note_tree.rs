//! Indexed hierarchy + flat visible rows, following Zed's virtualized project panel.
use crate::store::{NoteSummary,SidebarGroup};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug)]
pub struct TreeRow {
    pub index: usize,
    pub group: Option<usize>,
    pub spacer: bool,
    pub depth: usize,
    pub has_children: bool,
}
#[derive(Default)]
pub struct NoteTree {
    pub expanded: HashSet<String>,
    pub rows: Vec<TreeRow>,
    pub groups: Vec<SidebarGroup>,
    grouped_roots: Vec<Vec<usize>>,
    by_id: HashMap<String, usize>,
    children: Vec<Vec<usize>>,
    roots: Vec<usize>,
}
impl NoteTree {
    pub fn set_notes(&mut self, notes: &[NoteSummary]) {
        self.by_id = notes.iter().enumerate().map(|(i,n)| (n.id.clone(),i)).collect();
        self.children = vec![Vec::new(); notes.len()];
        self.roots.clear();
        self.grouped_roots=vec![Vec::new();self.groups.len()];
        let groups:HashMap<_,_>=self.groups.iter().enumerate().map(|(i,g)| (g.id.as_str(),i)).collect();
        for (i,n) in notes.iter().enumerate() {
            if let Some(parent) = n.parent_id.as_ref().and_then(|id| self.by_id.get(id)) {
                self.children[*parent].push(i);
            } else if let Some(group)=n.group_id.as_deref().and_then(|id| groups.get(id)) {
                self.grouped_roots[*group].push(i);
            } else { self.roots.push(i); }
        }
        self.expanded.retain(|id| self.by_id.contains_key(id) || groups.contains_key(id.as_str()));
        self.rebuild(notes);
    }
    pub fn rebuild(&mut self, notes: &[NoteSummary]) {
        self.rows.clear();
        let mut sections=vec![(None,self.roots.clone())];
        sections.extend(self.grouped_roots.iter().enumerate().map(|(i,roots)| (Some(i),roots.clone())));
        for (group,roots) in sections {
            if let Some(group)=group {
                // Uniform virtualized rows keep accurate scroll/drop geometry. Reserve
                // one empty row between sections instead of changing header heights.
                if !self.rows.is_empty() {
                    self.rows.push(TreeRow {index:0,group:None,spacer:true,depth:0,has_children:false});
                }
                self.rows.push(TreeRow {index:0,group:Some(group),spacer:false,depth:0,has_children:!roots.is_empty()});
                if !self.expanded.contains(&self.groups[group].id) {continue;}
            }
            let mut stack:Vec<_>=roots.into_iter().rev().map(|i| (i,0)).collect();
            while let Some((index,depth))=stack.pop() {
                self.rows.push(TreeRow {index,group:None,spacer:false,depth,has_children:!self.children[index].is_empty()});
                if self.expanded.contains(&notes[index].id) {
                    stack.extend(self.children[index].iter().rev().map(|i| (*i,depth+1)));
                }
            }
        }
    }
    pub fn toggle(&mut self, id: &str, notes: &[NoteSummary]) {
        if !self.expanded.remove(id) { self.expanded.insert(id.to_string()); }
        self.rebuild(notes);
    }
    pub fn reveal(&mut self, id: &str, notes: &[NoteSummary]) -> Option<usize> {
        let mut current = self.by_id.get(id).copied()?;
        let mut changed = false;
        while let Some(parent) = notes[current].parent_id.as_ref().and_then(|id| self.by_id.get(id)).copied() {
            changed |= self.expanded.insert(notes[parent].id.clone());
            current = parent;
        }
        if let Some(group)=notes[current].group_id.as_ref() {
            changed |= self.expanded.insert(group.clone());
        }
        if changed { self.rebuild(notes); }
        self.rows.iter().position(|row| !row.spacer && row.group.is_none() && notes[row.index].id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn group_spacing_keeps_reveal_and_virtual_row_indices_correct() {
        let mut tree=NoteTree::default();
        tree.groups=vec![SidebarGroup {id:"a".into(),title:"Favorites".into()},SidebarGroup {id:"b".into(),title:"Work".into()}];
        tree.expanded.extend(["a".to_owned(),"b".to_owned()]);
        let notes:Vec<_>=(0..3).map(|i| NoteSummary {group_id:match i {1=>Some("a".into()),2=>Some("b".into()),_=>None},icon:None,id:i.to_string(),title:i.to_string(),revision:1,parent_id:None}).collect();
        tree.set_notes(&notes);
        assert_eq!(tree.rows.len(),7);
        assert!(tree.rows[1].spacer && tree.rows[4].spacer);
        assert_eq!(tree.reveal("0",&notes),Some(0));
        assert_eq!(tree.reveal("2",&notes),Some(6));
        tree.toggle("a",&notes);
        assert_eq!(tree.reveal("2",&notes),Some(5));
        assert_eq!(tree.reveal("1",&notes),Some(3));
    }

    #[test]
    fn grouped_pages_are_virtualized_and_reveal_their_section() {
        let group=SidebarGroup {id:"group".into(),title:"Favorites".into()};
        let notes:Vec<_>=(0..10_000).map(|i| NoteSummary {group_id:if i==0 {Some(group.id.clone())} else {None},icon:None,id:i.to_string(),title:i.to_string(),revision:1,parent_id:if i==0 {None} else {Some((i-1).to_string())}}).collect();
        let mut tree=NoteTree::default();tree.groups.push(group);tree.set_notes(&notes);
        assert_eq!(tree.rows.len(),1);assert_eq!(tree.rows[0].group,Some(0));
        assert_eq!(tree.reveal("9999",&notes),Some(10000));
        assert_eq!(tree.rows.len(),10001);assert_eq!(tree.rows.last().unwrap().depth,9999);
        tree.toggle("group",&notes);assert_eq!(tree.rows.len(),1);
        tree.toggle("group",&notes);assert_eq!(tree.rows.len(),10001);
        assert!(tree.rows.iter().skip(1).all(|r| r.group.is_none()));
    }
    #[test]
    fn deep_tree_is_iterative_and_collapsed_rows_are_omitted() {
        let notes: Vec<_> = (0..20_000).map(|i| NoteSummary { icon: None, group_id: None,
            id:i.to_string(), title:format!("Note {i}"), revision:1,
            parent_id: if i==0 {None} else {Some((i-1).to_string())},
        }).collect();
        let mut tree = NoteTree::default();
        tree.set_notes(&notes);
        assert_eq!(tree.rows.len(),1);
        assert_eq!(tree.reveal("19999", &notes),Some(19999));
        assert_eq!(tree.rows.last().unwrap().depth,19999);
        tree.toggle("0", &notes);
        assert_eq!(tree.rows.len(),1);
        tree.toggle("0", &notes);
        assert_eq!(tree.rows.len(),20000);
    }
}
