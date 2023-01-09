pub use csscolorparser::*;

pub trait ColorExt {
    fn to_linear_rgba_f32(&self) -> [f32; 4];
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
}
