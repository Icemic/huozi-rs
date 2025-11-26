use csscolorparser::Color;

pub enum ColorSpace {
    Linear,
    SRGB,
}

pub(super) fn get_color_value(color: &Color, color_space: &ColorSpace) -> [f32; 4] {
    match color_space {
        ColorSpace::Linear => color.to_linear_rgba(),
        ColorSpace::SRGB => color.to_array(),
    }
}
