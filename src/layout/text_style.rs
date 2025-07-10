use std::str::FromStr;

use csscolorparser::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TextStyle {
    // pub font_face: Font
    pub font_size: f64,
    pub fill_color: Color,
    pub line_height: f64,
    pub indent: f64,
    pub stroke: Option<StrokeStyle>,
    pub shadow: Option<ShadowStyle>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 32.,
            fill_color: Color::new(0., 0., 0., 1.),
            line_height: 1.5,
            indent: 0.,
            stroke: None,
            shadow: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StrokeStyle {
    pub stroke_color: Color,
    pub stroke_width: f32,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            stroke_color: Color::new(1.0, 1.0, 1.0, 1.),
            stroke_width: 3.,
        }
    }
}

impl FromStr for StrokeStyle {
    type Err = String;

    fn from_str(_: &str) -> Result<Self, Self::Err> {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShadowStyle {
    pub shadow_color: Color,
    pub shadow_offset_x: f32,
    pub shadow_offset_y: f32,
    pub shadow_blur: f32,
    pub shadow_width: f32,
}

impl Default for ShadowStyle {
    fn default() -> Self {
        Self {
            shadow_color: Color::new(0.5, 0.5, 0.5, 0.8),
            shadow_offset_x: 1.,
            shadow_offset_y: 1.,
            shadow_blur: 8.,
            shadow_width: 3.,
        }
    }
}

impl FromStr for ShadowStyle {
    type Err = String;

    fn from_str(_: &str) -> Result<Self, Self::Err> {
        unimplemented!()
    }
}
