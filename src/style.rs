use std::collections::HashMap;

use crate::{
    css::{Rule, Selector, SimpleSelector, Specificity, Stylesheet, Value},
    dom::{ElementData, Node, NodeType},
};

pub type PropertyMap = HashMap<String, Value>;

#[derive(Debug, PartialEq)]
pub struct StyledNode<'a> {
    pub node: &'a Node,
    pub specified_values: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

pub enum Display {
    Inline,
    Block,
    None,
}

impl<'a> StyledNode<'a> {
    pub fn value(&self, name: &str) -> Option<Value> {
        self.specified_values.get(name).cloned()
    }

    pub fn lookup(&self, name: &str, fallback: &str, default: &Value) -> Value {
        self.value(name)
            .unwrap_or_else(|| self.value(fallback).unwrap_or_else(|| default.clone()))
    }

    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(Value::Keyword(s)) => match &*s {
                "block" => Display::Block,
                "none" => Display::None,
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }
}

pub fn style_tree<'a>(root: &'a Node, stylesheet: &'a Stylesheet) -> StyledNode<'a> {
    StyledNode {
        node: root,
        specified_values: match root.node_type {
            NodeType::Element(ref elem) => specified_values(elem, stylesheet),
            NodeType::Text(_) => HashMap::new(),
        },
        children: root
            .childlen
            .iter()
            .map(|child| style_tree(child, stylesheet))
            .collect(),
    }
}

fn specified_values(elem: &ElementData, stylesheet: &Stylesheet) -> PropertyMap {
    let mut values = HashMap::new();
    let mut rules = matching_rules(elem, stylesheet);

    rules.sort_by(|&(a, _), &(b, _)| a.cmp(&b));
    for (_, rule) in rules {
        for declaration in &rule.declarations {
            values.insert(declaration.name.clone(), declaration.value.clone());
        }
    }
    values
}

type MatchRule<'a> = (Specificity, &'a Rule);

fn matching_rules<'a>(elem: &ElementData, stylesheet: &'a Stylesheet) -> Vec<MatchRule<'a>> {
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(elem, rule))
        .collect()
}

fn match_rule<'a>(elem: &ElementData, rule: &'a Rule) -> Option<MatchRule<'a>> {
    rule.selectors
        .iter()
        .find(|selector| matchs(elem, *selector))
        .map(|selector| (selector.specificity(), rule))
}

fn matchs(elem: &ElementData, selector: &Selector) -> bool {
    match *selector {
        Selector::Simple(ref simple_selector) => matchs_simple_selector(elem, simple_selector),
    }
}

fn matchs_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    if selector.tag_name.iter().any(|name| elem.tag_name != *name) {
        return false;
    }

    if selector.id.iter().any(|id| elem.id() != Some(id)) {
        return false;
    }

    let elem_classes = elem.classes();
    if selector
        .class
        .iter()
        .any(|class| !elem_classes.contains(&**class))
    {
        return false;
    }

    true
}

mod tests {
    use std::collections::HashMap;

    use crate::{
        css::{self, Value},
        dom::text,
        html,
        style::{style_tree, StyledNode},
    };

    #[test]
    fn test_style_tree_overwrite() {
        let html_source = String::from(r#"<p class="name">Hello</p>"#);

        let css_source = String::from(
            r#"
        p {
            color: #cccccc;
        }

        p.name {
            color: #cc0000;
        }
        "#,
        );
        let root = html::parse(html_source);
        let css = css::parse(css_source);

        let mut specified_values = HashMap::new();
        specified_values.insert(
            String::from("color"),
            Value::ColorValue(css::Color {
                r: 204,
                g: 0,
                b: 0,
                a: 255,
            }),
        );
        let text = text(String::from("Hello"));
        let expected = StyledNode {
            node: &root,
            specified_values: specified_values,
            children: vec![StyledNode {
                node: &text,
                specified_values: HashMap::new(),
                children: vec![],
            }],
        };
        assert_eq!(expected, style_tree(&root, &css));
    }
}
