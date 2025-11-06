use csscolorparser::Color;
use huozi::layout::*;

pub fn text_style_default() -> TextStyle {
    TextStyle {
        fill_color: Color::new(1.0, 1.0, 1.0, 1.0),
        stroke: Some(stroke_default()),
        shadow: Some(shadow_default()),
        ..TextStyle::default()
    }
}

pub fn stroke_default() -> StrokeStyle {
    StrokeStyle {
        stroke_width: 1.0,
        stroke_color: Color::new(0.0, 0.0, 0.0, 1.0),
    }
}

pub fn shadow_default() -> ShadowStyle {
    ShadowStyle {
        shadow_offset_x: 1.0,
        shadow_offset_y: 1.0,
        shadow_blur: 0.0,
        shadow_width: 0.4,
        shadow_color: Color::new(1.0, 0.25, 0.6, 1.0),
    }
}
