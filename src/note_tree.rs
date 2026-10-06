//! Indexed hierarchy + flat visible rows, following Zed's virtualized project panel.
use crate::store::{NoteSummary,SidebarGroup};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug)]
pub struct TreeRow {
    pub index: usize,
    pub group: Option<usize>,
    pub spacer: bool,
    pub empty: bool,
    pub depth: usize,
    pub has_children: bool,
}
impl TreeRow {
    pub fn is_page(&self) -> bool { !self.spacer && !self.empty && self.group.is_none() }
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
    subtree_ends:Vec<usize>,
}
impl NoteTree {
    pub fn note_index(&self,id:&str)->Option<usize> {self.by_id.get(id).copied()}
    pub fn subtree_count(&self,id:&str)->usize {
        let mut stack:Vec<_>=self.note_index(id).into_iter().collect();let mut count=0;
        while let Some(i)=stack.pop() {count+=1;stack.extend(self.children[i].iter().copied());}
        count
    }
    pub fn last_root(&self)->Option<usize> {self.roots.last().copied()}
    pub fn subtree_end(&self,index:usize)->Option<TreeRow> {self.subtree_ends.get(index).and_then(|i|self.rows.get(*i)).copied()}


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
                // Spacing belongs to the preceding section's visible content, so
                // collapsed groups leave their headers adjacent.
                // Keep uniform rows for accurate virtualized scroll/drop geometry.
                if self.rows.last().is_some_and(|row| row.is_page() || row.empty) {
                    self.rows.push(TreeRow {index:0,group:None,spacer:true,empty:false,depth:0,has_children:false});
                }
                self.rows.push(TreeRow {index:0,group:Some(group),spacer:false,empty:false,depth:0,has_children:!roots.is_empty()});
                if !self.expanded.contains(&self.groups[group].id) {continue;}
                if roots.is_empty() {
                    self.rows.push(TreeRow {index:0,group:Some(group),spacer:false,empty:true,depth:0,has_children:false});
                }
            }
            let mut stack:Vec<_>=roots.into_iter().rev().map(|i| (i,0)).collect();
            while let Some((index,depth))=stack.pop() {
                self.rows.push(TreeRow {index,group:None,spacer:false,empty:false,depth,has_children:!self.children[index].is_empty()});
                if self.expanded.contains(&notes[index].id) {
                    if self.children[index].is_empty() {
                        self.rows.push(TreeRow {index,group:None,spacer:false,empty:true,depth:depth+1,has_children:false});
                    }
                    stack.extend(self.children[index].iter().rev().map(|i| (*i,depth+1)));
                }
            }
        }
        self.subtree_ends=vec![0;notes.len()];
        let mut stack:Vec<(usize,usize)>=Vec::new();
        for i in (0..self.rows.len()).rev() {
            let row=self.rows[i];
            if row.group.is_some() || row.spacer {stack.clear();continue;}
            let mut end=i;
            while stack.last().is_some_and(|(depth,_)|*depth>row.depth) {end=end.max(stack.pop().unwrap().1);}
            if row.is_page() {self.subtree_ends[row.index]=end;}
            stack.push((row.depth,end));
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
        self.rows.iter().position(|row| row.is_page() && notes[row.index].id == id)
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
        assert_eq!(tree.rows.len(),5);
        assert_eq!(tree.rows[2].group,Some(0));
        assert_eq!(tree.rows[3].group,Some(1));
        assert_eq!(tree.reveal("2",&notes),Some(4));
        tree.toggle("b",&notes);
        assert_eq!(tree.rows.len(),4);
        assert_eq!(tree.rows[2].group,Some(0));
        assert_eq!(tree.rows[3].group,Some(1));
        assert_eq!(tree.reveal("1",&notes),Some(3));
        assert!(tree.rows[4].spacer);
        assert_eq!(tree.rows[5].group,Some(1));
    }

    #[test]
    fn empty_groups_show_placeholder_only_when_expanded() {
        let mut tree=NoteTree::default();
        tree.groups=vec![SidebarGroup {id:"empty".into(),title:"Empty".into()},SidebarGroup {id:"full".into(),title:"Full".into()}];
        let notes=vec![NoteSummary {id:"page".into(),title:"Page".into(),parent_id:None,group_id:Some("full".into()),icon:None,revision:1}];
        tree.set_notes(&notes);
        assert_eq!(tree.rows.len(),2);
        tree.toggle("empty",&notes);
        assert_eq!(tree.rows.len(),4);
        assert!(tree.rows[1].empty && tree.rows[2].spacer);
        assert_eq!(tree.rows[1].group,Some(0));
        tree.toggle("full",&notes);
        assert_eq!(tree.rows.len(),5);
        tree.toggle("empty",&notes);
        assert_eq!(tree.rows.len(),3);
        assert!(tree.rows.iter().all(|r|!r.spacer));
        tree.toggle("full",&notes);
        assert_eq!(tree.rows.len(),2);
    }
    #[test]
    fn empty_page_placeholder_is_not_a_selectable_page_and_updates_with_children() {
        let mut tree=NoteTree::default();
        let mut notes=vec![NoteSummary {id:"parent".into(),title:"Parent".into(),parent_id:None,group_id:None,icon:None,revision:1}];
        tree.set_notes(&notes);tree.toggle("parent",&notes);
        assert_eq!(tree.rows.len(),2);
        assert!(tree.rows[1].empty && !tree.rows[1].is_page());
        assert_eq!(tree.rows[1].depth,1);
        assert_eq!(tree.reveal("parent",&notes),Some(0));
        notes.push(NoteSummary {id:"child".into(),title:"Child".into(),parent_id:Some("parent".into()),group_id:None,icon:None,revision:1});
        tree.set_notes(&notes);
        assert_eq!(tree.rows.len(),2);
        assert!(tree.rows[1].is_page());
        assert_eq!(tree.reveal("child",&notes),Some(1));
        notes.pop();tree.set_notes(&notes);
        assert!(tree.rows[1].empty);
        tree.toggle("parent",&notes);assert_eq!(tree.rows.len(),1);
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
