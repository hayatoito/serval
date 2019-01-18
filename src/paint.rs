use crate::css;
use crate::css::Color;
use crate::layout::*;
use crate::prelude::*;
use log::*;
use std::path::Path;

type DisplayList = Vec<DisplayCommand>;

#[derive(Debug)]
enum DisplayCommand {
    SolidColor(Color, Rect),
}

fn build_display_list(layout_root: &LayoutBox<'_>) -> DisplayList {
    let mut list = Vec::new();
    render_layout_box(&mut list, layout_root);
    list
}

fn render_layout_box(list: &mut DisplayList, layout_box: &LayoutBox<'_>) {
    render_background(list, layout_box);
    render_borders(list, layout_box);

    for child in &layout_box.children {
        render_layout_box(list, child);
    }
}

fn render_background(list: &mut DisplayList, layout_box: &LayoutBox<'_>) {
    if let Some(color) = get_color(layout_box, "background") {
        list.push(DisplayCommand::SolidColor(
            color,
            layout_box.dimensions.border_box(),
        ))
    }
}

fn render_borders(list: &mut DisplayList, layout_box: &LayoutBox<'_>) {
    let color = match get_color(layout_box, "border-color") {
        Some(color) => color,
        _ => return,
    };

    let d = &layout_box.dimensions;
    let border_box = d.border_box();

    // Left border
    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: d.border.left,
            height: border_box.height,
        },
    ));

    // Right border
    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x + border_box.width - d.border.right,
            y: border_box.y,
            width: d.border.right,
            height: border_box.height,
        },
    ));

    // Top border
    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: border_box.width,
            height: d.border.top,
        },
    ));

    // Bottom border
    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x,
            y: border_box.y + border_box.height - d.border.bottom,
            width: border_box.width,
            height: d.border.bottom,
        },
    ));
}

fn get_color(layout_box: &'_ LayoutBox<'_>, name: &str) -> Option<Color> {
    match layout_box.box_type {
        BoxType::BlockNode(style_node) | BoxType::InlineNode(style_node) => {
            match style_node.value(name) {
                Some(css::Value::ColorValue(color)) => Some(*color),
                _ => None,
            }
        }
        BoxType::AnonymousBlock => None,
    }
}

trait Canvas {
    // fn new(width: usize, height: usize) -> Self;
    fn paint_item(&mut self, item: &DisplayCommand);
    fn paint(&mut self, layout_root: &LayoutBox<'_>) {
        let display_list = build_display_list(layout_root);
        debug!("display_list: {:?}", display_list);
        for item in display_list {
            self.paint_item(&item);
        }
    }
    // fn save_as(&self, file: impl AsRef<std::path::Path>) -> Result<()>;
    fn save_as(&self, file: &std::path::Path) -> Result<()>;
}

struct PixelCanvas {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
}

impl PixelCanvas {
    fn new(width: usize, height: usize) -> Self {
        let white = Color { r: 0, g: 0, b: 0 };
        PixelCanvas {
            pixels: vec![white; width * height],
            width,
            height,
        }
    }

    fn clamp(&self, f: f32) -> usize {
        f.clamp(0.0, self.width as f32) as usize
    }
}

impl Canvas for PixelCanvas {
    fn paint_item(&mut self, item: &DisplayCommand) {
        match item {
            DisplayCommand::SolidColor(color, rect) => {
                debug!("painting: color: {:?}, rect: {:?}", color, rect);
                let x0 = self.clamp(rect.x);
                let y0 = self.clamp(rect.y);
                let x1 = self.clamp(rect.x + rect.width);
                let y1 = self.clamp(rect.y + rect.height);
                for y in y0..y1 {
                    for x in x0..x1 {
                        self.pixels[x + y * self.width] = *color;
                    }
                }
            }
        }
    }

    // fn save_as(&self, file: impl AsRef<std::path::Path>) -> Result<()> {
    fn save_as(&self, file: &std::path::Path) -> Result<()> {
        debug!("save_as_png: {}", file.display());
        let (w, h) = (self.width as u32, self.height as u32);
        let img = image::ImageBuffer::from_fn(w, h, move |x, y| {
            let color = self.pixels[(y * w + x) as usize];
            image::Pixel::from_channels(color.r, color.g, color.b, 255)
        });
        image::ImageRgba8(img).save(file)?;
        Ok(())
    }
}

struct WebCanvas {
    width: usize,
    height: usize,
    commands: Vec<String>,
}

impl WebCanvas {
    fn new(width: usize, height: usize) -> Self {
        WebCanvas {
            width,
            height,
            commands: vec![],
        }
    }
}

impl Canvas for WebCanvas {
    fn paint_item(&mut self, item: &DisplayCommand) {
        match item {
            DisplayCommand::SolidColor(color, rect) => {
                debug!("painting: color: {:?}, rect: {:?}", color, rect);
                self.commands.push(format!(
                    "ctx.fillStyle = 'rgb({},{},{},{})';",
                    color.r,
                    color.g,
                    color.b,
                    255, // TODO: Use color.alpha
                ));
                self.commands.push(format!(
                    "ctx.fillRect({}, {}, {}, {});",
                    rect.x, rect.y, rect.width, rect.height
                ));
            }
        }
    }

    // fn save_as(&self, file: impl AsRef<std::path::Path>) -> Result<()> {
    fn save_as(&self, file: &std::path::Path) -> Result<()> {
        debug!("save_as_html: {}", file.display());
        let commands = self.commands.join("\n");
        let html = format!(
            r###"
<doctype html>
<html>
<canvas id="canvas" width="{}" height="{}"></canvas>
<script>
const canvas = document.querySelector('#canvas');
const ctx = canvas.getContext('2d');
{};
</script>
"###,
            self.width, self.height, commands
        );
        std::fs::write(file, html)?;
        Ok(())
    }
}

pub fn paint_and_save(
    html: &str,
    stylesheet: &str,
    output_file: impl AsRef<Path>,
    format: &str,
) -> Result<()> {
    let node = crate::dom::parser::parse_html(html)?;
    debug!("parsed html: {:?}", node);
    let stylesheet = css::parser::parse_stylesheet(&stylesheet)?;
    debug!("parsed stylesheet: {:?}", stylesheet);

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

    let rect = Rect {
        width: 800.0,
        height: 800.0,
        ..Default::default()
    };

    let mut canvas: Box<dyn Canvas> = match format {
        "png" => Box::new(PixelCanvas::new(rect.width as usize, rect.height as usize)),
        "canvas" => Box::new(WebCanvas::new(rect.width as usize, rect.height as usize)),
        _ => {
            unreachable!();
        }
    };

    canvas.paint(&layout_tree);
    canvas.save_as(output_file.as_ref())?;
    println!("saved as: {}", output_file.as_ref().display());

    Ok(())
}

// pub fn dump_png
