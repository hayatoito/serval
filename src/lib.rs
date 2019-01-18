#[macro_use]
pub mod prelude;

pub mod css;
pub mod dom;
pub mod layout;
pub mod paint;
pub mod style;

pub use crate::dom::parser::parse_html;
pub use crate::layout::dump_layout;
pub use crate::paint::paint_and_save;
