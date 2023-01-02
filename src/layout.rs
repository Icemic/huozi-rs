mod color;
mod text_section;
mod text_style;
mod layout_style;

pub use color::*;
pub use text_section::*;
pub use text_style::*;
pub use layout_style::*;

pub fn calculate_layout(layout_style: &LayoutStyle, text_sections: &Vec<TextSection>) {}
