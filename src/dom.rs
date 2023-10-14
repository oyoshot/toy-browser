use std::collections::HashMap;

pub type AttrMap = HashMap<String, String>;

#[derive(Debug, PartialEq)]
pub struct Node {
    // data common to all nodes
    childlen: Vec<Node>,

    // data specific to each node type
    node_type: NodeType,
}

#[derive(Debug, PartialEq)]
enum NodeType {
    Text(String),
    Element(ElementData),
}

#[derive(Debug, PartialEq)]
struct ElementData {
    tag_name: String,
    attributes: AttrMap,
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
