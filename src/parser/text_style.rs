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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut style = StrokeStyle::default();
        let parts: Vec<&str> = s.split_ascii_whitespace().collect();

        match parts.as_slice() {
            [] => Err("empty stroke style".to_string()),

            [one] => {
                if let Ok(color) = one.parse::<Color>() {
                    style.stroke_color = color;
                    Ok(style)
                } else {
                    style.stroke_width = parse_valid_f32(one, "stroke width")?;
                    Ok(style)
                }
            }

            [color, width] => {
                style.stroke_color = color
                    .parse::<Color>()
                    .map_err(|_| format!("invalid stroke color `{color}`"))?;
                style.stroke_width = parse_valid_f32(width, "stroke width")?;
                Ok(style)
            }

            _ => Err(format!(
                "invalid stroke style `{s}`, expected `<color>`, `<width>`, or `<color> <width>`"
            )),
        }
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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut style = ShadowStyle::default();
        let parts: Vec<&str> = s.split_ascii_whitespace().collect();

        match parts.as_slice() {
            [] => Err("empty shadow style".to_string()),

            [x, y] => {
                style.shadow_offset_x = parse_f32(x, "shadow offset x")?;
                style.shadow_offset_y = parse_f32(y, "shadow offset y")?;
                Ok(style)
            }

            [x, y, blur] => {
                style.shadow_offset_x = parse_f32(x, "shadow offset x")?;
                style.shadow_offset_y = parse_f32(y, "shadow offset y")?;
                style.shadow_blur = parse_valid_f32(blur, "shadow blur")?;
                Ok(style)
            }

            [x, y, blur, color] => {
                style.shadow_offset_x = parse_f32(x, "shadow offset x")?;
                style.shadow_offset_y = parse_f32(y, "shadow offset y")?;
                style.shadow_blur = parse_valid_f32(blur, "shadow blur")?;
                style.shadow_color = color
                    .parse::<Color>()
                    .map_err(|_| format!("invalid shadow color `{color}`"))?;
                Ok(style)
            }

            [x, y, blur, color, width] => {
                style.shadow_offset_x = parse_f32(x, "shadow offset x")?;
                style.shadow_offset_y = parse_f32(y, "shadow offset y")?;
                style.shadow_blur = parse_valid_f32(blur, "shadow blur")?;
                style.shadow_color = color
                    .parse::<Color>()
                    .map_err(|_| format!("invalid shadow color `{color}`"))?;
                style.shadow_width = parse_valid_f32(width, "shadow width")?;
                Ok(style)
            }

            _ => Err(format!(
                "invalid shadow style `{s}`, expected `<x> <y> [blur] [color] [width]`"
            )),
        }
    }
}

fn parse_f32(s: &str, name: &str) -> Result<f32, String> {
    s.parse::<f32>()
        .map_err(|_| format!("invalid {name} `{s}`"))
}

fn parse_valid_f32(s: &str, name: &str) -> Result<f32, String> {
    let value = parse_f32(s, name)?;

    if !value.is_finite() {
        return Err(format!("invalid {name} `{s}`: expected a finite number"));
    }

    if value < 0.0 {
        return Err(format!("invalid {name} `{s}`: expected a non-negative number"));
    }

    Ok(value)
    // may be used for style.stroke_width, style.shadow_blur, style.shadow_width, to prevent NaN, neg, infty, etc.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stroke_style_width() {
        let style = "2.5".parse::<StrokeStyle>().unwrap();
        assert_eq!(style.stroke_width, 2.5);
    }

    #[test]
    fn parse_stroke_style_color_and_width() {
        let style = "#196883 2".parse::<StrokeStyle>().unwrap();
        assert_eq!(style.stroke_width, 2.0);
    }

    #[test]
    fn parse_shadow_style_offset() {
        let style = "1 -2".parse::<ShadowStyle>().unwrap();
        assert_eq!(style.shadow_offset_x, 1.0);
        assert_eq!(style.shadow_offset_y, -2.0);
    }

    #[test]
    fn parse_shadow_style_all() {
        let style = "1 2 8 #196883 3".parse::<ShadowStyle>().unwrap();
        assert_eq!(style.shadow_offset_x, 1.0);
        assert_eq!(style.shadow_offset_y, 2.0);
        assert_eq!(style.shadow_blur, 8.0);
        assert_eq!(style.shadow_width, 3.0);
    }

    #[test]
    fn parse_stroke_style_invalid() {
        assert!("Gan Ren Chui".parse::<StrokeStyle>().is_err());
    }

    #[test]
    fn parse_stroke_style_negative_width() {
        assert!("red -1".parse::<StrokeStyle>().is_err());
    }

    #[test]
    fn parse_shadow_style_negative_blur() {
        assert!("1 2 -3".parse::<ShadowStyle>().is_err());
    }

    #[test]
    fn parse_shadow_style_nan_width() {
        assert!("1 2 3 red NaN".parse::<ShadowStyle>().is_err());
    }
}