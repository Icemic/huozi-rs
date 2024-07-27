pub use csscolorparser::{parse as parse_color, Color, ParseColorError};

pub trait ColorExt {
    fn to_linear_rgba_f32(&self) -> [f32; 4];
    fn to_srgb_rgba_f32(&self) -> [f32; 4];
}

impl ColorExt for Color {
    fn to_linear_rgba_f32(&self) -> [f32; 4] {
        let color = self.to_linear_rgba();
        [
            color.0 as f32,
            color.1 as f32,
            color.2 as f32,
            color.3 as f32,
        ]
    }
    fn to_srgb_rgba_f32(&self) -> [f32; 4] {
        [self.r as f32, self.g as f32, self.b as f32, self.a as f32]
    }
}
