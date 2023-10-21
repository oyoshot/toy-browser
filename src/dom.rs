use std::collections::{HashMap, HashSet};

pub type AttrMap = HashMap<String, String>;

#[derive(Debug, PartialEq)]
pub struct Node {
    // data common to all nodes
    pub childlen: Vec<Node>,

    // data specific to each node type
    pub node_type: NodeType,
}

#[derive(Debug, PartialEq)]
pub enum NodeType {
    Text(String),
    Element(ElementData),
}

#[derive(Debug, PartialEq)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: AttrMap,
}

pub fn text(data: String) -> Node {
    Node {
        childlen: Vec::new(),
        node_type: NodeType::Text(data),
    }
}

pub fn elem(name: String, attrs: AttrMap, children: Vec<Node>) -> Node {
    Node {
        childlen: children,
        node_type: NodeType::Element(ElementData {
            tag_name: name,
            attributes: attrs,
        }),
    }
}

impl ElementData {
    pub fn id(&self) -> Option<&String> {
        self.attributes.get("id")
    }

    pub fn classes(&self) -> HashSet<&str> {
        match self.attributes.get("class") {
            Some(classlist) => classlist.split(' ').collect(),
            None => HashSet::new(),
        }
    }
}
