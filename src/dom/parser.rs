use super::{AttrMap, Node};

use crate::prelude::*;
use combine::parser::char::{char, letter, space, spaces};
use combine::*;

#[derive(PartialEq, Eq, Clone, Debug)]
struct Attribute {
    key: String,
    value: String,
}

def_parser! {
    fn text() -> Node {
        between(
            char('"'),
            char('"'),
            many(letter().or(space())) // TODO: Use satisfy: |c| c != '"'
        ).map(Node::Text)
    }
}

#[test]
fn text_test() {
    assert_eq!(
        text().parse(r#""hello""#),
        Ok((Node::Text("hello".to_string()), ""))
    );
}

def_parser! {
    fn attribute() -> Attribute {
        many1(letter()).skip(char('=')).and(many1(letter())).map(|(k, v)| {
            Attribute {
                key: k,
                value: v
            }
        })
    }
}

#[test]
fn attribute_test() {
    assert_eq!(
        attribute().parse("id=foo"),
        Ok((
            Attribute {
                key: "id".to_string(),
                value: "foo".to_string(),
            },
            ""
        ))
    );
}

// See combine_test.rs / sexp parser
def_parser! {
    pub fn node() -> Node {
        text().or(
            between(
                char('('),
                char(')'),
                element(),
            )
        )
    }
}

def_parser! {
    fn element() -> Node {
        many1(letter()).skip(spaces())
            .and(element_attributes_nodes())
            .map(|(name, (attributes, nodes))| {
                Node::element(name, attributes, nodes)
            })
    }
}

// def_parser!{
//     fn element_attributes_nodes() -> (Vec<Attribute>, Vec<Node>) {
//         attribute().skip(spaces()).and(element_attributes_nodes()).map(|(a, (mut attributes, nodes))| {
//             // TODO: Use append, and reverse vector
//             attributes.insert(0, a);
//             (attributes, nodes)
//         }).or(element_nodes().map(|nodes| (vec![], nodes)))
//     }
// }

def_parser! {
    fn element_attributes_nodes() -> (AttrMap, Vec<Node>) {
        attribute().skip(spaces()).and(element_attributes_nodes()).map(|(a, (mut attributes, nodes))| {
            attributes.insert(a.key, a.value);
            (attributes, nodes)
        }).or(element_nodes().map(|nodes| (AttrMap::new(), nodes)))
    }
}

def_parser! {
    fn element_nodes() -> Vec<Node> {
        sep_by(node(), spaces())
    }
}

pub fn parse_html(html: &str) -> Result<Node> {
    Ok(node().parse(html).map_err(EngineError::from)?.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    use combine::Parser;
    // use crate::dom::attrmap;
    use maplit::btreemap;

    // use crate::parse::assert_parse;

    #[test]
    fn node_test() {
        assert_parse!(
            node(),
            "(p)",
            Node::element("p".to_string(), AttrMap::new(), vec![])
        );

        let hello_text = Node::Text("hello".to_string());

        assert_parse!(node(), r#""hello""#, hello_text.clone());

        let id_attr = btreemap! { "id".to_string() => "foo".to_string()};

        let id_and_class_attrs = btreemap! {
            "id".to_string() => "foo".to_string(),
        "class".to_string() => "bar".to_string()};

        assert_parse!(
            node(),
            "(p id=foo)",
            Node::element(
                "p".to_string(),
                btreemap! { "id".to_string() => "foo".to_string()},
                vec![]
            )
        );

        assert_parse!(
            node(),
            "(p id=foo class=bar)",
            Node::element("p".to_string(), id_and_class_attrs, vec![])
        );

        let p_hello = Node::element("p".to_string(), AttrMap::new(), vec![hello_text.clone()]);

        assert_parse!(node(), r#"(p "hello")"#, p_hello.clone());

        assert_parse!(
            node(),
            r#"(p id=foo "hello")"#,
            Node::element("p".to_string(), id_attr.clone(), vec![hello_text.clone()])
        );

        let div_p_hello = Node::element("div".to_string(), AttrMap::new(), vec![p_hello.clone()]);
        assert_parse!(node(), r#"(div (p "hello"))"#, div_p_hello);

        let div2 = Node::element(
            "div".to_string(),
            AttrMap::new(),
            vec![p_hello.clone(), p_hello.clone()],
        );
        assert_parse!(node(), r#"(div (p "hello") (p "hello"))"#, div2);

        assert_parse_fail!(node(), r#"("hello")"#);
        assert_parse_fail!(node(), "()");
        assert_parse_fail!(node(), "(id=foo)");
        assert_parse_fail!(node(), r#"(p "hello" id=foo)"#);
        assert_parse_fail!(node(), r#"(p (p) id=foo)"#);
        assert_parse_fail!(node(), "(p (p)");
        assert_parse_fail!(node(), "p");
    }

}
