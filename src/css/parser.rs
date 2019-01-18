// use combine::parser::char::{char, letter, space, spaces};
use combine::parser::char;
use combine::parser::char::{digit, letter, spaces};
use combine::parser::item;
use combine::*;

use crate::css::*;
use crate::prelude::*;

def_parser! {
    pub fn stylesheet() -> Stylesheet {
        sep_by(rule(), spaces()).map(|rules| Stylesheet { rules })
    }
}

def_parser! {
    fn rule() -> Rule {
        (selectors(),
         spaces(),
         char::char('{'),
         spaces(),
         declarations(),
         spaces(),
         char::char('}'),
        ).map(|(selectors, _, _, _, declarations, _, _)| Rule {
            selectors: SortedSelectors::new(selectors),
            declarations
        })
    }
}

def_parser! {
    pub fn selectors() -> Vec<Selector> {
        sep_by(selector(), (char::char(','), spaces()))
    }
}

def_parser! {
    fn selector() -> Selector {
        simple_selector().map(Selector::Simple)
    }
}

enum SimpleSelectorPart {
    Universal,
    TagName(String),
    Id(String),
    Class(String),
}

def_parser! {
    fn simple_selector() -> SimpleSelector {
        simple_selector_part().and(simple_selector()).map(|(x, mut xs)| {
            match x {
                SimpleSelectorPart::Universal => {
                }
                SimpleSelectorPart::TagName(s) => {
                    assert!(xs.tag_name.is_none());
                    xs.tag_name = Some(s);
                }
                SimpleSelectorPart::Id(s) => {
                    assert!(xs.id.is_none());
                    xs.id = Some(s);
                }
                SimpleSelectorPart::Class(s) => {
                    xs.classes.insert(s);
                }
            }
            xs
        }).or(item::value(Default::default()))
    }
}

def_parser! {
    fn simple_selector_part() -> SimpleSelectorPart {
        char::char('*').map(|_| SimpleSelectorPart::Universal)
            .or(tag_name().map(SimpleSelectorPart::TagName))
            .or(id().map(SimpleSelectorPart::Id))
                .or(class().map(SimpleSelectorPart::Class))
    }
}

def_parser! {
    fn tag_name() -> String {
        many1(letter())
    }
}

def_parser! {
    fn id() -> String {
        (char::char('#'), identifier()).map(|(_, id)| id)
    }
}

def_parser! {
    fn class() -> String {
        (char::char('.'), identifier()).map(|(_, class)| class)
    }
}

def_parser! {
    fn declarations() -> Vec<Declaration> {
        sep_by(declaration(), (char::char(';'), spaces()))
    }
}

def_parser! {
    fn declaration() -> Declaration {
        (
            identifier(),
            char::char(':'),
            spaces(),
            value()
        ).map(|(name, _, _, value)| {
            Declaration {
                name,
                value
            }

        })
    }
}

def_parser! {
    fn identifier() -> String {
        (letter(), many(char::alpha_num())).map(|(x, mut xs): (char, String)| {
            xs.insert(0, x);
            xs
        })
    }
}

def_parser! {
    fn value() -> Value {
        // starts with [a-z] => keyword
        // starts with [0-9] => Length
        // starts with [#] => ColorValue
        keyword_string().map(Value::Keyword)
            .or(length().map(|(n, px)| Value::Length(n, px)))
            .or(color().map(Value::ColorValue))
    }
}

def_parser! {
    fn keyword_string() -> String {
        (letter(), many(char::alpha_num().or(item::token('-')))).map(|(x, mut xs): (char, String)| {
            xs.insert(0, x);
            xs
        })
    }
}

def_parser! {
    fn length() -> (f32, Unit) {
        // Todo: Supprt floating point number.
        (many1(digit()), char::string("px")).map(|(digits, _): (String, _)| {
            (digits.parse().unwrap(), Unit::Px)
        })
    }
}

def_parser! {
    fn color() -> Color {
        (char::char('#'),
         count(3, hex_pair())).map(|(_, rgb): (_, Vec<u8>)| {
             Color {
                 r: rgb[0],
                 g: rgb[1],
                 b: rgb[2],
             }
         })
    }
}

def_parser! {
    fn hex_pair() -> u8 {
        count(2, char::hex_digit()).map(|hex: String| {
            u8::from_str_radix(&hex, 16).unwrap()
        })
    }
}

pub fn parse_stylesheet(sheet: &str) -> Result<Stylesheet> {
    Ok(stylesheet()
        .parse(sheet.trim())
        .map_err(EngineError::from)?
        .0)
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::css;
    use crate::css::parser;
    use maplit::btreeset;

    fn color((r, g, b): Rgb) -> Color {
        Color {
            r,
            g,
            b,
            // a: 0,
        }
    }

    #[test]
    #[ignore]
    fn stylesheet_test() {
        assert_parse!(
            parser::stylesheet(),
            "div { color: #000000 }",
            Stylesheet {
                rules: vec![Rule {
                    selectors: SortedSelectors::new(vec![Selector::tag("div")]),
                    declarations: vec![Declaration::color((0, 0, 0))],
                }],
            }
        );
    }

    #[test]
    fn rule_test() {
        assert_parse!(
            parser::rule(),
            "div { color: #000000 }",
            Rule {
                selectors: SortedSelectors::new(vec![Selector::tag("div")]),
                declarations: vec![Declaration::color((0, 0, 0))],
            }
        );
        assert_parse!(
            parser::rule(),
            "* { display: block }",
            Rule {
                selectors: SortedSelectors::new(vec![Selector::Simple(
                    css::SimpleSelector::universal(),
                )]),
                declarations: vec![Declaration {
                    name: "display".to_string(),
                    value: css::Value::Keyword("block".to_string()),
                }],
            }
        );
        assert_parse!(
            parser::rule(),
            "div, p { color: #000000; color: #010203 }",
            Rule {
                selectors: SortedSelectors::new(vec![Selector::tag("div"), Selector::tag("p")]),
                declarations: vec![Declaration::color((0, 0, 0)), Declaration::color((1, 2, 3))],
            }
        );
        assert_parse!(
            parser::rule(),
            "p, div { color: #000000; color: #010203 }",
            Rule {
                selectors: SortedSelectors::new(vec![Selector::tag("p"), Selector::tag("div")]),
                declarations: vec![Declaration::color((0, 0, 0)), Declaration::color((1, 2, 3))],
            }
        );
        assert_parse!(
            parser::rule(),
            "p, #foo { color: #000000; color: #010203 }",
            Rule {
                selectors: SortedSelectors::new(vec![Selector::id("foo"), Selector::tag("p")]),
                declarations: vec![Declaration::color((0, 0, 0)), Declaration::color((1, 2, 3))],
            }
        );
    }

    #[test]
    fn selectors_test() {
        assert_parse!(parser::selectors(), "div", vec![Selector::tag("div")]);
        assert_parse!(
            parser::selectors(),
            "div  ",
            vec![Selector::tag("div")],
            "  "
        );
        assert_parse!(
            parser::selectors(),
            "div, div",
            vec![Selector::tag("div"), Selector::tag("div")]
        );
        assert_parse!(
            parser::selectors(),
            "div, p",
            vec![Selector::tag("div"), Selector::tag("p")]
        );
        assert_parse!(
            parser::selectors(),
            "div, #foo",
            vec![Selector::tag("div"), Selector::id("foo")]
        );
        assert_parse!(
            parser::selectors(),
            "div, #foo, .class1",
            vec![
                Selector::tag("div"),
                Selector::id("foo"),
                Selector::class(&["class1"]),
            ]
        );
    }

    #[test]
    fn selector_test() {
        assert_parse!(parser::selector(), "div", Selector::tag("div"));
    }

    #[test]
    fn simple_selector_test() {
        assert_parse!(parser::simple_selector(), "div", SimpleSelector::tag("div"));
        assert_parse!(parser::simple_selector(), "#foo", SimpleSelector::id("foo"));
        assert_parse!(
            parser::simple_selector(),
            ".class1",
            SimpleSelector::class(&["class1"])
        );

        assert_parse!(
            parser::simple_selector(),
            ".class1.class2",
            SimpleSelector::class(&["class1", "class2"])
        );

        assert_parse!(
            parser::simple_selector(),
            "div#foo",
            SimpleSelector {
                tag_name: Some("div".to_string()),
                id: Some("foo".to_string()),
                ..Default::default()
            }
        );

        assert_parse!(
            parser::simple_selector(),
            "div#foo.class1.class2",
            SimpleSelector {
                tag_name: Some("div".to_string()),
                id: Some("foo".to_string()),
                classes: btreeset! { "class1".to_string(), "class2".to_string() },
            }
        );
    }

    #[test]
    fn universal_selector_test() {
        assert_parse!(parser::simple_selector(), "*", Default::default());
        assert_parse!(
            parser::simple_selector(),
            "*#foo",
            SimpleSelector {
                id: Some("foo".to_string()),
                ..Default::default()
            }
        )
    }

    #[test]
    fn tag_name_test() {
        assert_parse!(parser::tag_name(), "div", "div".to_string());
    }

    #[test]
    fn identifier_test() {
        let mut parser = parser::identifier();
        assert_parse!(parser, "div", "div".to_string());
        assert_parse!(parser, "d123", "d123".to_string());
        assert_parse_fail!(parser, "123");
    }

    #[test]
    fn keyword_string_test() {
        let mut parser = parser::keyword_string();
        assert_parse!(parser, "div", "div".to_string());
        assert_parse!(parser, "d123", "d123".to_string());
        assert_parse!(parser, "abc-def", "abc-def".to_string());
        assert_parse_fail!(parser, "123");
        assert_parse_fail!(parser, "-");
    }

    #[test]
    fn length_test() {
        let mut parser = parser::length();
        assert_parse!(parser, "1px", (1.0, Unit::Px));
        assert_parse!(parser, "123px", (123.0, Unit::Px));
        // TODO: Support minus value.
        // assert_eq!(parser.parse("-123px"), Ok(((-123, Unit::Px), "")));
        assert_parse_fail!(parser, "1");
        assert_parse_fail!(parser, "apx");
        assert_parse_fail!(parser, "1pz");
    }

    #[test]
    fn value_test() {
        let mut parser = parser::value();
        assert_parse!(parser, "div", Value::Keyword("div".to_string()));
        assert_parse!(parser, "1px", Value::Length(1.0, Unit::Px));
        assert_parse!(parser, "#000000", Value::ColorValue(color((0, 0, 0))));
    }

    #[test]
    fn declarations_test() {
        let mut parser = parser::declarations();
        assert_parse!(parser, "color: #00000", vec![Declaration::color((0, 0, 0))]);
        assert_parse!(
            parser,
            "color: #00000; color: #00000",
            vec![Declaration::color((0, 0, 0)), Declaration::color((0, 0, 0))]
        );
    }

    #[test]
    fn declaration_test() {
        assert_parse!(
            parser::declaration(),
            "color: #00000",
            Declaration::color((0, 0, 0))
        );
    }

    #[test]
    fn hex_pair_test() {
        assert_eq!(u8::from_str_radix("00", 16).unwrap(), 0);
        assert_eq!(u8::from_str_radix("ff", 16).unwrap(), 255);
        assert_parse!(parser::hex_pair(), "00", 0);
        assert_parse!(parser::hex_pair(), "ff", 255);
    }

    #[test]
    fn color_test() {
        assert_parse!(parser::color(), "#00000", color((0, 0, 0)));
    }

}
