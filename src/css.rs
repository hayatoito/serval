pub mod parser;
use lazy_static::*;

// use ordered_float::OrderedFloat;
use std::collections::BTreeSet;

// https://limpet.net/mbrubeck/2014/08/13/toy-layout-engine-3-css.html

// pub type Num = OrderedFloat<f32>;
// pub type Num = f32;

#[derive(Debug, PartialEq)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, PartialEq)]
pub struct Rule {
    // TODO: Assert selectors should br sorted by their specifities
    // pub selectors: Vec<Selector>,
    pub selectors: SortedSelectors,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, PartialEq)]
pub struct SortedSelectors {
    pub selectors: Vec<Selector>,
}

impl SortedSelectors {
    pub fn new(mut selectors: Vec<Selector>) -> SortedSelectors {
        selectors.sort_by(|a, b| b.specifity().cmp(&a.specifity()));
        SortedSelectors { selectors }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    Simple(SimpleSelector),
}

impl Selector {
    #[cfg(test)]
    pub(crate) fn tag(tag_name: &str) -> Selector {
        Selector::Simple(SimpleSelector::tag(tag_name))
    }

    #[cfg(test)]
    pub(crate) fn id(id: &str) -> Selector {
        Selector::Simple(SimpleSelector::id(id))
    }

    #[cfg(test)]
    pub(crate) fn class(classes: &[&str]) -> Selector {
        Selector::Simple(SimpleSelector::class(classes))
    }

    #[cfg(test)]
    pub(crate) fn universal() -> Selector {
        Selector::Simple(Default::default())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub classes: BTreeSet<String>,
}

impl SimpleSelector {
    #[cfg(test)]
    pub(crate) fn tag(tag_name: &str) -> SimpleSelector {
        SimpleSelector {
            tag_name: Some(tag_name.to_string()),
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn id(id: &str) -> SimpleSelector {
        SimpleSelector {
            id: Some(id.to_string()),
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn class(classes: &[&str]) -> SimpleSelector {
        SimpleSelector {
            classes: classes.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn universal() -> SimpleSelector {
        Default::default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
}

impl Declaration {
    #[cfg(test)]
    pub(crate) fn color(rgb: Rgb) -> Declaration {
        Declaration {
            name: "color".to_string(),
            value: Value::color(rgb),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    ColorValue(Color),
}

impl Value {
    #[cfg(test)]
    pub(crate) fn color((r, g, b): Rgb) -> Value {
        Value::ColorValue(Color { r, g, b })
    }

    pub fn keyword_auto() -> &'static Value {
        lazy_static! {
            static ref AUTO: Value = Value::Keyword("auto".to_string());
        }
        &AUTO
    }

    pub fn length_zero() -> &'static Value {
        lazy_static! {
            static ref LENGTH_ZERO: Value = Value::Length(0.0, Unit::Px);
        }
        &LENGTH_ZERO
    }

    pub fn to_px(&self) -> f32 {
        match *self {
            Value::Length(px, Unit::Px) => px,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unit {
    Px,
}

pub type Rgb = (u8, u8, u8);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    // pub a: u8, // alpha
}

pub type Specifity = (usize, usize, usize);

impl Selector {
    pub fn specifity(&self) -> Specifity {
        let Selector::Simple(ref simple) = *self;
        let a = if simple.id.is_some() { 1 } else { 0 };
        let b = simple.classes.len();
        let c = if simple.tag_name.is_some() { 1 } else { 0 };
        (a, b, c)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::btreeset;

    #[test]
    fn sorted_selectors_test() {
        let selectors = vec![
            Selector::tag("div"),
            Selector::Simple(Default::default()),
            Selector::id("foo"),
            Selector::Simple(SimpleSelector {
                classes: btreeset! { "class1".to_string(), "class2".to_string() },
                ..Default::default()
            }),
            Selector::Simple(SimpleSelector {
                tag_name: Some("div".to_string()),
                classes: btreeset! { "class2".to_string() },
                ..Default::default()
            }),
            Selector::class(&["class1"]),
        ];

        let sorted_selectors = SortedSelectors::new(selectors.clone());
        assert_eq!(
            sorted_selectors.selectors,
            vec![
                Selector::id("foo"),
                Selector::Simple(SimpleSelector {
                    classes: btreeset! { "class1".to_string(), "class2".to_string() },
                    ..Default::default()
                }),
                Selector::Simple(SimpleSelector {
                    tag_name: Some("div".to_string()),
                    classes: btreeset! { "class2".to_string() },
                    ..Default::default()
                }),
                Selector::class(&["class1"]),
                Selector::tag("div"),
                Selector::Simple(Default::default()),
            ]
        );
    }

}
