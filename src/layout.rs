use crate::css;
use crate::style::{Display, StyledNode};

use crate::prelude::*;

use log::*;

fn nearly_equal(a: f32, b: f32) -> bool {
    (a - b).abs() < std::f32::EPSILON
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    fn expanded_by(&self, edge: EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
}

impl std::fmt::Display for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}) [{}x{}]",
            self.x, self.y, self.width, self.height
        )
    }
}

// TODO: Rename this?
// LayoutBox => BoxNode,
// Dimensions => LayoutBox?
#[derive(Clone, Copy, Default, Debug)]
pub struct Dimensions {
    /// Position of the content area relative to the document origin:
    pub content: Rect,
    // Surrounding edges:
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl std::fmt::Display for Dimensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (padding: {}, border: {}, margin: {})",
            self.content, self.padding, self.border, self.margin
        )
    }
}

impl Dimensions {
    fn padding_box(&self) -> Rect {
        self.content.expanded_by(self.padding)
    }
    pub(crate) fn border_box(&self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }
    fn margin_box(&self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl std::fmt::Display for EdgeSizes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if nearly_equal(self.left, self.right)
            && nearly_equal(self.right, self.top)
            && nearly_equal(self.top, self.bottom)
        {
            write!(f, "{}", self.top)
        } else if nearly_equal(self.left, self.right) && nearly_equal(self.top, self.bottom) {
            write!(f, "{} {}", self.top, self.right)
        } else if nearly_equal(self.left, self.right) {
            write!(f, "{} {} {}", self.top, self.right, self.bottom)
        } else {
            write!(
                f,
                "{} {} {} {}",
                self.top, self.right, self.bottom, self.left
            )
        }
    }
}

pub struct LayoutBox<'a> {
    pub(crate) dimensions: Dimensions,
    pub(crate) box_type: BoxType<'a>,
    pub(crate) children: Vec<LayoutBox<'a>>,
}

impl std::fmt::Display for LayoutBox<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            self.fmt_alternate(f, 0)
        } else {
            write!(f, "{} {}", self.box_type, self.dimensions)
        }
    }
}

impl<'a> LayoutBox<'a> {
    fn fmt_alternate(&self, f: &mut std::fmt::Formatter<'_>, indent: usize) -> std::fmt::Result {
        debug_assert!(f.alternate());
        writeln!(
            f,
            "{:spaces$}{} {}",
            "",
            self.box_type,
            self.dimensions,
            spaces = indent
        )?;
        for child in &self.children {
            child.fmt_alternate(f, indent + 2)?;
        }
        Ok(())
    }

    fn new(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
        LayoutBox {
            box_type: match style_node.display() {
                Display::Block => BoxType::BlockNode(style_node),
                Display::Inline => BoxType::InlineNode(style_node),
                Display::None => unreachable!(),
            },
            dimensions: Default::default(),
            children: Default::default(),
        }
    }

    fn new_with_anonymous_block() -> LayoutBox<'a> {
        LayoutBox {
            box_type: BoxType::AnonymousBlock,
            dimensions: Default::default(),
            children: Default::default(),
        }
    }

    pub fn get_inline_container(&mut self) -> &mut Self {
        match self.box_type {
            BoxType::InlineNode(_) | BoxType::AnonymousBlock => self,
            BoxType::BlockNode(_) => {
                match self.children.last() {
                    Some(LayoutBox {
                        box_type: BoxType::AnonymousBlock,
                        ..
                    }) => {} // Re-use the last anonymouse block
                    _ => self.children.push(LayoutBox::new_with_anonymous_block()),
                }
                self.children.last_mut().unwrap()
            }
        }
    }

    pub fn layout(&mut self, containing_block: &Dimensions) {
        debug!("layout: {}", self);
        match self.box_type {
            BoxType::BlockNode(_) => self.layout_block(containing_block),
            // TODO: Implement this.
            // See https://www.w3.org/TR/css-inline-3/
            // https://drafts.csswg.org/css-inline-3/
            BoxType::InlineNode(_) => unimplemented!(),
            BoxType::AnonymousBlock => unimplemented!(),
        }
    }

    // https://limpet.net/mbrubeck/2014/09/17/toy-layout-engine-6-block.html
    fn layout_block(&mut self, containing_block: &Dimensions) {
        debug!("layout_block: {}", self);
        // Child width can depend on parent width, so we need to calculate
        // this box's width before laying out its children.
        self.calculate_block_width(containing_block);

        // Determine where the box is located within its container.
        self.calculate_block_position(containing_block);

        // Recursively lay out the children of this box.
        self.layout_block_children();

        // Parent height can depend on child height, so `calculate_height`
        // must be called *after* the children are laid out.
        self.calculate_block_height();
    }

    fn get_style_node(&self) -> &'a StyledNode<'a> {
        match self.box_type {
            BoxType::BlockNode(node) | BoxType::InlineNode(node) => node,
            BoxType::AnonymousBlock => unreachable!("Anonymous block has no style node"),
        }
    }

    fn calculate_block_width(&mut self, containing_block: &Dimensions) {
        debug!("calculate_block_width: {}", self);
        let style = self.get_style_node();
        let width = style.value("width").unwrap_or(&css::Value::keyword_auto());

        let zero = css::Value::length_zero();

        let mut margin_left = style.lookup("margin-left", "margin", zero);
        let mut margin_right = style.lookup("margin-right", "margin", zero);

        let border_left = style.lookup("border-left-width", "border-width", zero);
        let border_right = style.lookup("border-right-width", "border-width", zero);

        let padding_left = style.lookup("padding-left", "padding", zero);
        let padding_right = style.lookup("padding-right", "padding", zero);

        let total: f32 = [
            margin_left,
            margin_right,
            border_left,
            border_right,
            padding_left,
            padding_right,
            width,
        ]
        .iter()
        .map(|v| v.to_px())
        .sum();

        // println!("total: {}", total);

        let auto = css::Value::keyword_auto();
        if width != auto && total > containing_block.content.width {
            if margin_left == auto {
                margin_left = zero;
            }
            if margin_right == auto {
                margin_right = zero;
            }
        }

        let underflow = containing_block.content.width - total;

        // println!("underflow: {}", underflow);

        let d = &mut self.dimensions;
        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();
        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();

        match (width == auto, margin_left == auto, margin_right == auto) {
            (false, false, false) => {
                d.content.width = width.to_px();
                d.margin.left = margin_left.to_px();
                d.margin.right = margin_right.to_px() + underflow;
            }
            (false, false, true) => {
                d.content.width = width.to_px();
                d.margin.left = margin_left.to_px();
                d.margin.right = underflow;
            }
            (false, true, false) => {
                d.content.width = width.to_px();
                d.margin.left = underflow;
                d.margin.right = margin_right.to_px();
            }
            (false, true, true) => {
                d.content.width = width.to_px();
                d.margin.left = underflow / 2.0;
                d.margin.right = underflow / 2.0;
            }
            (true, _, _) => {
                if margin_left == auto {
                    margin_left = zero;
                }
                if margin_right == auto {
                    margin_right = zero;
                }
                if underflow >= 0.0 {
                    d.content.width = underflow;
                    d.margin.left = margin_left.to_px();
                    d.margin.right = margin_right.to_px();
                } else {
                    d.content.width = 0.0;
                    d.margin.left = margin_left.to_px();
                    d.margin.right = margin_right.to_px() + underflow;
                }
            }
        }
        // debug_assert_eq!(
        //     d.content.width + d.margin.left + d.margin.right,
        //     containing_block.content.width
        // );
    }

    fn calculate_block_position(&mut self, containing_block: &Dimensions) {
        debug!("calculate_block_position: {}", self);
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        let zero = css::Value::length_zero();

        d.margin.top = style.lookup("margin-top", "margin", zero).to_px();
        d.margin.bottom = style.lookup("margin-bottom", "margin", zero).to_px();

        d.border.top = style
            .lookup("border-top-width", "border-width", zero)
            .to_px();
        d.border.bottom = style
            .lookup("border-bottom-width", "border-width", zero)
            .to_px();

        d.padding.top = style.lookup("padding-top-width", "padding", zero).to_px();
        d.padding.bottom = style
            .lookup("padding-bottom-width", "padding", zero)
            .to_px();

        d.content.x = containing_block.content.x + d.margin.left + d.border.left + d.padding.left;

        // TODO: [2018-08-21 Tue] Understand this later.
        // - When containing_block.content.height is calculated?
        // - Why is containing_block.content.height added here?
        /* A :  (y: 0), {margin-top: 10px}
           B <- { margin-top: 10px }
           C <- { margin-top: 10px }

        */

        //
        // layout A:
        //

        d.content.y = containing_block.content.y
            + containing_block.content.height
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }

    fn layout_block_children(&mut self) {
        debug!("layout_block_children: {}", self);
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(d);
            d.content.height += child.dimensions.margin_box().height;
            debug!("d.content.height => : {}", d.content.height);
        }
    }

    fn calculate_block_height(&mut self) {
        if let Some(css::Value::Length(h, css::Unit::Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = *h;
        }
    }
}

pub(crate) enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    AnonymousBlock,
}

impl std::fmt::Display for BoxType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoxType::BlockNode(style_node) => write!(f, "{}(block)", style_node.node.simple_name()),
            BoxType::InlineNode(style_node) => {
                write!(f, "{}(inline)", style_node.node.simple_name())
            }
            BoxType::AnonymousBlock => write!(f, "(anonymous)"),
        }
    }
}

pub fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    let mut root = LayoutBox::new(style_node);
    for child in &style_node.children {
        match child.display() {
            Display::Block => root.children.push(build_layout_tree(child)),
            Display::Inline => root
                .get_inline_container()
                .children
                .push(build_layout_tree(child)),
            Display::None => {
                // skip
            }
        }
    }
    root
}

pub fn dump_layout(html: &str, stylesheet: &str) -> Result<String> {
    debug!("parsing html:\n{}", html);
    let node = crate::dom::parser::parse_html(html)?;
    debug!("parsed: {:?}", node);

    debug!("parsing stylesheet:\n{}", stylesheet);
    let stylesheet = css::parser::parse_stylesheet(&stylesheet)?;
    debug!("parsed: {:?}", stylesheet);

    let style_tree = crate::style::style_tree(&node, &stylesheet);
    let mut layout_tree = build_layout_tree(&style_tree);
    let viewport = Dimensions {
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 0.0,
        },
        ..Default::default()
    };
    layout_tree.layout(&viewport);
    Ok(format!("{:#}", layout_tree))
}

pub fn dump_layout_as_json(html: &str, stylesheet: &str) -> Result<String> {
    debug!("parsing html:\n{}", html);
    let node = crate::dom::parser::parse_html(html)?;
    debug!("parsed: {:?}", node);

    debug!("parsing stylesheet:\n{}", stylesheet);
    let stylesheet = css::parser::parse_stylesheet(&stylesheet)?;
    debug!("parsed: {:?}", stylesheet);

    let style_tree = crate::style::style_tree(&node, &stylesheet);
    let mut layout_tree = build_layout_tree(&style_tree);
    let viewport = Dimensions {
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 0.0,
        },
        ..Default::default()
    };
    layout_tree.layout(&viewport);
    // TODO: json
    Ok(format!("{:#}", layout_tree))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::css;
    use crate::dom;
    use crate::style;
    use crate::style::StyledNode;
    use combine::*;
    use maplit::*;

    #[test]
    fn calculate_block_width_test() {
        fn px(px: f32) -> css::Value {
            css::Value::Length(px, css::Unit::Px)
        }

        fn keyword(s: &str) -> css::Value {
            css::Value::Keyword(s.to_string())
        }

        fn auto() -> css::Value {
            keyword("auto")
        }

        fn assert_box_width(
            (width, margin_left, margin_right): (css::Value, css::Value, css::Value),
            (expected_width, expected_margin_left, expected_margin_right): (f32, f32, f32),
        ) {
            let node = dom::Node::Element(Default::default());
            let style_node = StyledNode {
                node: &node,
                css_specified_values: hashmap! {
                    "display".to_string() => keyword("block"),
                    "width".to_string() => width,
                    "margin-left".to_string() => margin_left,
                    "margin-right".to_string() => margin_right,
                },
                children: vec![],
            };
            let mut layout_box = LayoutBox::new(&style_node);
            let containing_block = Dimensions {
                content: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 0.0,
                },
                ..Default::default()
            };
            layout_box.calculate_block_width(&containing_block);
            assert_eq!(layout_box.dimensions.content.width, expected_width);
            assert_eq!(layout_box.dimensions.margin.left, expected_margin_left);
            assert_eq!(layout_box.dimensions.margin.right, expected_margin_right);
        }

        assert_box_width((px(1.0), px(0.0), px(0.0)), (1.0, 0.0, 9.0));
        assert_box_width((px(1.0), px(2.0), px(0.0)), (1.0, 2.0, 7.0));
        assert_box_width((px(1.0), px(2.0), px(10.0)), (1.0, 2.0, 7.0));
        // overflow
        assert_box_width((px(1.0), px(10.0), px(10.0)), (1.0, 10.0, -1.0));
        assert_box_width((px(20.0), px(10.0), px(10.0)), (20.0, 10.0, -20.0));

        assert_box_width((auto(), px(0.0), px(0.0)), (10.0, 0.0, 0.0));
        assert_box_width((auto(), px(1.0), px(0.0)), (9.0, 1.0, 0.0));
        assert_box_width((auto(), px(1.0), px(2.0)), (7.0, 1.0, 2.0));
        // overflow
        assert_box_width((auto(), px(1.0), px(20.0)), (0.0, 1.0, 9.0));
        assert_box_width((auto(), px(20.0), px(1.0)), (0.0, 20.0, -10.0));

        assert_box_width((px(1.0), auto(), px(2.0)), (1.0, 7.0, 2.0));
        assert_box_width((px(1.0), px(2.0), auto()), (1.0, 2.0, 7.0));
        assert_box_width((px(1.0), auto(), auto()), (1.0, 4.5, 4.5));
        // overflow
        assert_box_width((px(11.0), auto(), auto()), (11.0, 0.0, -1.0));
        assert_box_width((px(11.0), auto(), px(1.0)), (11.0, 0.0, -1.0));
        assert_box_width((px(11.0), px(1.0), auto()), (11.0, 1.0, -2.0));

        assert_box_width((auto(), auto(), px(1.0)), (9.0, 0.0, 1.0));
        assert_box_width((auto(), px(1.0), auto()), (9.0, 1.0, 0.0));

        assert_box_width((auto(), px(11.0), auto()), (0.0, 11.0, -1.0));
        assert_box_width((auto(), auto(), px(11.0)), (0.0, 0.0, 10.0));
    }

    fn layout<'a>(style_tree: &'a style::StyledNode<'a>) -> LayoutBox<'a> {
        let mut layout_tree = build_layout_tree(style_tree);
        let window = Dimensions {
            content: Rect {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 0.0,
            },
            ..Default::default()
        };
        layout_tree.layout(&window);
        layout_tree
    }

    #[test]
    fn layout_test() {
        let node = dom::parser::node()
            .parse("(p id=foo class=bar (div) (div))")
            .unwrap()
            .0;
        let stylesheet = css::parser::stylesheet()
            .parse("* { display: block } div { margin: 10px }")
            .unwrap()
            .0;
        let style_tree = style::style_tree(&node, &stylesheet);
        let layout_tree = layout(&style_tree);
        // assert_eq!(format!("{:#}", layout_tree), "layouttree-dayo");

        assert_eq!(layout_tree.dimensions.content.width, 800.0);
        assert_eq!(layout_tree.dimensions.content.height, 40.0);

        assert_eq!(layout_tree.children[0].dimensions.content.width, 780.0);
        assert_eq!(layout_tree.children[0].dimensions.content.height, 0.0);
        assert_eq!(layout_tree.children[0].dimensions.content.x, 10.0);
        assert_eq!(layout_tree.children[0].dimensions.content.y, 10.0);

        assert_eq!(layout_tree.children[1].dimensions.content.width, 780.0);
        assert_eq!(layout_tree.children[1].dimensions.content.height, 0.0);
        assert_eq!(layout_tree.children[1].dimensions.content.x, 10.0);
        assert_eq!(layout_tree.children[1].dimensions.content.y, 30.0);
    }

    fn assert_layout_dump(html: &str, css: &str, expected: &str) -> Result<()> {
        assert_eq!(
            dump_layout(html.trim(), css.trim())?.trim(),
            expected.trim()
        );
        Ok(())
    }

    #[test]
    fn layout_dump_test() {
        let html = r"(div (div (div (div (div (div (div))))))))";
        let css = r"
* {
  display: block;
  padding: 12px
}
";
        let layout = r"
div(block) (12, 12) [776x144] (padding: 12, border: 0, margin: 0)
  div(block) (24, 24) [752x120] (padding: 12, border: 0, margin: 0)
    div(block) (36, 36) [728x96] (padding: 12, border: 0, margin: 0)
      div(block) (48, 48) [704x72] (padding: 12, border: 0, margin: 0)
        div(block) (60, 60) [680x48] (padding: 12, border: 0, margin: 0)
          div(block) (72, 72) [656x24] (padding: 12, border: 0, margin: 0)
            div(block) (84, 84) [632x0] (padding: 12, border: 0, margin: 0)
";
        assert_layout_dump(html, css, layout).unwrap();
    }

}
