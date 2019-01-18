// use super::css::{Rule, Selector, SimpleSelector, Specificity, Stylesheet, Value};
use super::css;
// use super::dom::{ElementData, Node, NodeType};
use super::dom;
use super::dom::Node;
use std::collections::HashMap;

pub type CssPropertyMap = HashMap<String, css::Value>;

#[derive(PartialEq)]
pub enum Display {
    Inline,
    Block,
    None,
}

pub struct StyledNode<'a> {
    pub node: &'a Node,
    pub css_specified_values: CssPropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

impl<'a> StyledNode<'a> {
    pub fn value(&'a self, name: &str) -> Option<&'a css::Value> {
        self.css_specified_values.get(name)
    }

    pub fn lookup(
        &'a self,
        name: &str,
        fallback_name: &str,
        default: &'a css::Value,
    ) -> &'a css::Value {
        self.value(name)
            .unwrap_or_else(|| self.value(fallback_name).unwrap_or_else(|| default))
    }

    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(css::Value::Keyword(s)) => match s.as_str() {
                "block" => Display::Block,
                "none" => Display::None,
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }
}

pub fn style_tree<'a>(root: &'a Node, stylesheet: &'a css::Stylesheet) -> StyledNode<'a> {
    StyledNode {
        node: root,
        css_specified_values: match root {
            Node::Element(data) => css_specified_values(data, stylesheet),
            Node::Text(_) => HashMap::new(),
        },
        children: root
            .children()
            .iter()
            .map(|child| style_tree(child, stylesheet))
            .collect(),
    }
}

fn css_specified_values(elem: &dom::ElementData, stylesheet: &css::Stylesheet) -> CssPropertyMap {
    let mut values = HashMap::new();
    let mut rules = matching_rules(elem, stylesheet);

    // Go through the rules from lowest to highest specificity.
    rules.sort_by(|&(a, _), &(b, _)| a.cmp(&b));
    for (_, rule) in rules {
        for declaration in &rule.declarations {
            values.insert(declaration.name.clone(), declaration.value.clone());
        }
    }
    values
}

type MatchedRule<'a> = (css::Specifity, &'a css::Rule);

fn matching_rules<'a>(
    elem: &'a dom::ElementData,
    stylesheet: &'a css::Stylesheet,
) -> Vec<MatchedRule<'a>> {
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(elem, rule))
        .collect()
}

fn match_rule<'a>(elem: &dom::ElementData, rule: &'a css::Rule) -> Option<MatchedRule<'a>> {
    match_selectors(elem, &rule.selectors).map(|selector| (selector.specifity(), rule))
}

fn match_selectors<'a>(
    elem: &dom::ElementData,
    sorted_selectors: &'a css::SortedSelectors,
) -> Option<&'a css::Selector> {
    // Find the first (most specific) matching selector.
    sorted_selectors
        .selectors
        .iter()
        .find(|selector| matches(elem, *selector))
}

fn matches(elem: &dom::ElementData, selector: &css::Selector) -> bool {
    match selector {
        css::Selector::Simple(simple_selector) => matches_simple_selector(elem, simple_selector),
    }
}

fn matches_simple_selector(elem: &dom::ElementData, selector: &css::SimpleSelector) -> bool {
    // Check type selector
    if !selector.tag_name.iter().all(|name| elem.tag_name == *name) {
        return false;
    }

    // Check ID selector
    if !selector.id.iter().all(|id| elem.id() == Some(id)) {
        return false;
    }

    // Check class selectors
    let elem_classes = elem.classes();
    if !selector
        .classes
        .iter()
        .all(|class| elem_classes.contains(&**class))
    {
        return false;
    }

    // We didn't find any non-matching selector components.
    true
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::css;
    use crate::dom;
    use maplit::*;

    #[test]
    fn simple_selector_match_test() {
        let div_selector = css::SimpleSelector {
            tag_name: Some("div".to_string()),
            ..Default::default()
        };

        let div_elem = dom::ElementData {
            tag_name: "div".to_string(),
            ..Default::default()
        };

        let p_elem = dom::ElementData {
            tag_name: "p".to_string(),
            ..Default::default()
        };

        assert!(matches_simple_selector(&div_elem, &div_selector));
        assert!(!matches_simple_selector(&p_elem, &div_selector));

        let class_foo_selector = css::SimpleSelector {
            classes: btreeset! { "foo".to_string() },
            ..Default::default()
        };

        assert!(!matches_simple_selector(&div_elem, &class_foo_selector));
        assert!(!matches_simple_selector(&p_elem, &class_foo_selector));

        let div_class_foo_elem = dom::ElementData {
            tag_name: "div".to_string(),
            attrs: btreemap! {
                "class".to_string() => "foo".to_string()
            },
            ..Default::default()
        };

        let div_class_bar_elem = dom::ElementData {
            tag_name: "div".to_string(),
            attrs: btreemap! {
                "class".to_string() => "bar".to_string()
            },
            ..Default::default()
        };

        let div_class_foo_bar_elem = dom::ElementData {
            tag_name: "div".to_string(),
            attrs: btreemap! {
                "class".to_string() => "foo bar".to_string()
            },
            ..Default::default()
        };

        assert!(matches_simple_selector(
            &div_class_foo_elem,
            &class_foo_selector
        ));

        assert!(!matches_simple_selector(
            &div_class_bar_elem,
            &class_foo_selector
        ));

        assert!(matches_simple_selector(
            &div_class_foo_bar_elem,
            &class_foo_selector
        ));

        let universal_selector: css::SimpleSelector = Default::default();
        assert!(matches_simple_selector(&div_elem, &universal_selector));

        let div_id_foo_elem = dom::ElementData {
            tag_name: "div".to_string(),
            attrs: btreemap! {
                "id".to_string() => "foo".to_string()
            },
            ..Default::default()
        };

        assert!(matches_simple_selector(
            &div_id_foo_elem,
            &css::SimpleSelector {
                id: Some("foo".to_string()),
                ..Default::default()
            }
        ));

        assert!(!matches_simple_selector(
            &div_id_foo_elem,
            &css::SimpleSelector {
                id: Some("xxx".to_string()),
                ..Default::default()
            }
        ));

        let div_id_foo_class1_class2_elem = dom::ElementData {
            tag_name: "div".to_string(),
            attrs: btreemap! {
                "id".to_string() => "foo".to_string(),
                "class".to_string() => "class1 class2".to_string(),
            },
            ..Default::default()
        };

        assert!(matches_simple_selector(
            &div_id_foo_class1_class2_elem,
            &css::SimpleSelector {
                id: Some("foo".to_string()),
                classes: btreeset! {"class1".to_string()},
                ..Default::default()
            }
        ));

        assert!(!matches_simple_selector(
            &div_id_foo_class1_class2_elem,
            &css::SimpleSelector {
                id: Some("foo".to_string()),
                classes: btreeset! {"class1 classxx".to_string()},
                ..Default::default()
            }
        ));
    }

    #[test]
    fn match_selectors_test() {
        let div = dom::ElementData {
            tag_name: "div".to_string(),
            ..Default::default()
        };

        assert!(match_selectors(&div, &css::SortedSelectors::new(vec![])).is_none());
        assert!(match_selectors(
            &div,
            &css::SortedSelectors::new(vec![css::Selector::id("XXX")]),
        )
        .is_none());

        assert_eq!(
            match_selectors(
                &div,
                &css::SortedSelectors::new(vec![css::Selector::universal()]),
            ),
            Some(&css::Selector::universal())
        );

        let elem = dom::ElementData {
            tag_name: "div".to_string(),
            attrs: btreemap! {
                "id".to_string() => "foo".to_string(),
                "class".to_string() => "class1 class2".to_string(),
            },
            ..Default::default()
        };

        assert_eq!(
            match_selectors(
                &elem,
                &css::SortedSelectors::new(vec![
                    css::Selector::tag("div"),
                    css::Selector::class(&["class1"]),
                    css::Selector::id("foo"),
                ])
            ),
            Some(&css::Selector::id("foo")),
            "id should win"
        );

        assert_eq!(
            match_selectors(
                &elem,
                &css::SortedSelectors::new(vec![
                    css::Selector::tag("div"),
                    css::Selector::class(&["class1"]),
                ])
            ),
            Some(&css::Selector::class(&["class1"])),
            "class should win"
        );

        assert_eq!(
            match_selectors(
                &elem,
                &css::SortedSelectors::new(vec![
                    css::Selector::class(&["class1"]),
                    css::Selector::class(&["class1", "class2"]),
                    css::Selector::class(&["class2"]),
                ])
            ),
            Some(&css::Selector::class(&["class1", "class2"])),
            "More classes should win"
        );
    }

    #[test]
    fn matching_rules_test() {
        let stylesheet = css::Stylesheet {
            rules: vec![
                css::Rule {
                    selectors: css::SortedSelectors::new(vec![css::Selector::tag("div")]),
                    declarations: vec![css::Declaration::color((0, 0, 0))],
                },
                css::Rule {
                    selectors: css::SortedSelectors::new(vec![css::Selector::tag("foo")]),
                    declarations: vec![css::Declaration::color((1, 1, 1))],
                },
                css::Rule {
                    selectors: css::SortedSelectors::new(vec![css::Selector::tag("div")]),
                    declarations: vec![css::Declaration::color((2, 2, 2))],
                },
            ],
        };

        let div = dom::ElementData {
            tag_name: "div".to_string(),
            ..Default::default()
        };

        let matched_declarations = matching_rules(&div, &stylesheet)
            .into_iter()
            .map(|(_speficity, rule)| &rule.declarations)
            .collect::<Vec<_>>();

        assert_eq!(
            matched_declarations,
            vec![
                &vec![css::Declaration::color((0, 0, 0))],
                &vec![css::Declaration::color((2, 2, 2))],
            ]
        );
    }

    #[test]
    fn css_specified_values_test() {
        let stylesheet = css::Stylesheet {
            rules: vec![
                css::Rule {
                    selectors: css::SortedSelectors::new(vec![css::Selector::tag("div")]),
                    declarations: vec![css::Declaration::color((0, 0, 0))],
                },
                css::Rule {
                    selectors: css::SortedSelectors::new(vec![css::Selector::id("foo")]),
                    declarations: vec![css::Declaration::color((1, 1, 1))],
                },
                css::Rule {
                    selectors: css::SortedSelectors::new(vec![css::Selector::id("foo")]),
                    declarations: vec![css::Declaration::color((2, 2, 2))],
                },
                css::Rule {
                    selectors: css::SortedSelectors::new(vec![css::Selector::tag("div")]),
                    declarations: vec![css::Declaration::color((3, 3, 3))],
                },
            ],
        };

        let div = dom::ElementData {
            tag_name: "div".to_string(),
            attrs: btreemap! {
                "id".to_string() => "foo".to_string()
            },
            ..Default::default()
        };

        let values = css_specified_values(&div, &stylesheet);
        assert_eq!(
            values,
            hashmap! { "color".to_string() => css::Value::color((2, 2, 2)) }
        );
    }
}
