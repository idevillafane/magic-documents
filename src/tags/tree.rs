use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Hierarchical tag structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagNode {
    #[allow(dead_code)]
    pub name: String,
    pub children: BTreeMap<String, TagNode>,
}

impl TagNode {
    pub fn new(name: String) -> Self {
        TagNode {
            name,
            children: BTreeMap::new(),
        }
    }

    pub fn insert_path(&mut self, path: &[String]) {
        if path.is_empty() {
            return;
        }
        let first = &path[0];
        let node = self
            .children
            .entry(first.clone())
            .or_insert_with(|| TagNode::new(first.clone()));
        if path.len() > 1 {
            node.insert_path(&path[1..]);
        }
    }

    pub fn get_children_names(&self) -> Vec<String> {
        self.children.keys().cloned().collect()
    }

    pub fn get_child(&self, name: &str) -> Option<&TagNode> {
        self.children.get(name)
    }
}
