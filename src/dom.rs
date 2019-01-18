pub mod parser;

use std::collections::{BTreeMap, HashSet};
use std::fmt;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Node {
    Text(String),
    Element(ElementData),
}

type AttrMap = BTreeMap<String, String>;

impl Node {
    pub fn element(tag_name: String, attrs: AttrMap, children: Vec<Node>) -> Node {
        Node::Element(ElementData {
            tag_name,
            attrs,
            children,
        })
    }

    pub fn children(&self) -> &[Node] {
        match self {
            Node::Text(_) => &[],
            Node::Element(data) => &data.children,
        }
    }

    pub fn simple_name(&self) -> &str {
        match self {
            Node::Text(s) => &s,
            Node::Element(data) => &data.tag_name,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct ElementData {
    pub tag_name: String,
    pub attrs: AttrMap,
    pub children: Vec<Node>,
}

impl ElementData {
    pub fn id(&self) -> Option<&str> {
        self.attrs.get("id").map(|s| s.as_str())
    }

    pub fn classes(&self) -> HashSet<&str> {
        self.attrs
            .get("class")
            .map(|cl| cl.split(' ').collect())
            .unwrap_or_default()
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Text(s) => write!(f, r#""{}""#, s),
            Node::Element(data) => {
                let mut s = vec![data.tag_name.clone()];
                s.extend(data.attrs.iter().map(|(k, v)| format!("{}={}", k, v)));
                if f.alternate() {
                    s.extend(data.children.iter().map(|n| format!("{:#}", n)));
                }
                write!(f, "({})", s.join(" "))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::btreemap;

    #[test]
    fn text_display() {
        let text = Node::Text("hello".to_string());
        assert_eq!(format!("{}", text), r#""hello""#);
    }

    #[test]
    fn node_display() {
        let div = Node::element("div".to_string(), AttrMap::new(), vec![]);
        assert_eq!(format!("{}", div), "(div)");

        let text = Node::Text("hello".to_string());

        let attrs = btreemap! {
            "id".to_string() => "foo".to_string(),
            "class".to_string() => "bar".to_string(),
        };

        let div = Node::element("div".to_string(), attrs, vec![text]);
        // assert_eq!(format!("{}", div), r#"(div id=foo class=bar "hello")"#);
        assert_eq!(format!("{:#}", div), r#"(div class=bar id=foo "hello")"#);

        let body = Node::element("body".to_string(), AttrMap::new(), vec![div]);
        assert_eq!(
            format!("{:#}", body),
            r#"(body (div class=bar id=foo "hello"))"#
        );
    }

}
