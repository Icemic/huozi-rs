mod color;
mod layout_style;
mod text_section;
mod text_style;
mod vertex;

use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
pub use color::*;
pub use layout_style::*;
use log::warn;
pub use text_section::*;
pub use text_style::*;
pub use vertex::*;

use crate::{
    constant::{ASCENT, FONT_SIZE, GAMMA_COEFFICIENT, GRID_SIZE, VIEWPORT_HEIGHT, VIEWPORT_WIDTH},
    glyph_vertices::GlyphVertices,
    parser::{parse, Element},
    Huozi,
};

pub enum ColorSpace {
    Linear,
    SRGB,
}

impl Huozi {
    pub(self) fn parse_text_recursive(
        &self,
        elements: Vec<Element>,
        current_style: &TextStyle,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<Vec<TextSection>, String> {
        let mut sections = vec![];
        for element in elements {
            match element {
                Element::Text(text) => {
                    sections.push(TextSection {
                        text: text.to_string(),
                        style: current_style.clone(),
                    });
                }
                Element::Block(block) => {
                    let mut style = current_style.clone();
                    if block.value.is_some() {
                        let value = &block.value.unwrap();
                        match block.tag.as_str() {
                            "size" => {
                                style.font_size = parse_str(value, style.font_size);
                            }
                            "color" | "fillColor" => {
                                style.fill_color = parse_str(value, style.fill_color);
                            }
                            "lineHeight" => {
                                style.line_height = parse_str(value, style.line_height);
                            }
                            "indent" => {
                                style.indent = parse_str(value, style.indent);
                            }
                            "stroke" => {
                                style.stroke = parse_str_optional(value, style.stroke);
                            }
                            "strokeColor" => {
                                if style.stroke.is_none() {
                                    style.stroke = Some(StrokeStyle::default());
                                }
                                let stroke = style.stroke.as_mut().unwrap();
                                stroke.stroke_color = parse_str(value, stroke.stroke_color.clone());
                            }
                            "strokeWidth" => {
                                if style.stroke.is_none() {
                                    style.stroke = Some(StrokeStyle::default());
                                }
                                let stroke = style.stroke.as_mut().unwrap();
                                stroke.stroke_width = parse_str(value, stroke.stroke_width);
                            }
                            "shadow" => {
                                style.shadow = parse_str_optional(value, style.shadow);
                            }
                            "shadowOffsetX" => {
                                if style.shadow.is_none() {
                                    style.shadow = Some(ShadowStyle::default());
                                }
                                let shadow = style.shadow.as_mut().unwrap();
                                shadow.shadow_offset_x = parse_str(value, shadow.shadow_offset_x);
                            }
                            "shadowOffsetY" => {
                                if style.shadow.is_none() {
                                    style.shadow = Some(ShadowStyle::default());
                                }
                                let shadow = style.shadow.as_mut().unwrap();
                                shadow.shadow_offset_y = parse_str(value, shadow.shadow_offset_y);
                            }
                            "shadowWidth" => {
                                if style.shadow.is_none() {
                                    style.shadow = Some(ShadowStyle::default());
                                }
                                let shadow = style.shadow.as_mut().unwrap();
                                shadow.shadow_width = parse_str(value, shadow.shadow_width);
                            }
                            "shadowBlur" => {
                                if style.shadow.is_none() {
                                    style.shadow = Some(ShadowStyle::default());
                                }
                                let shadow = style.shadow.as_mut().unwrap();
                                shadow.shadow_blur = parse_str(value, shadow.shadow_blur);
                            }
                            "shadowColor" => {
                                if style.shadow.is_none() {
                                    style.shadow = Some(ShadowStyle::default());
                                }
                                let shadow = style.shadow.as_mut().unwrap();
                                shadow.shadow_color = parse_str(value, shadow.shadow_color.clone());
                            }
                            _ => {
                                warn!("unrecognized tag `{}`, ignored.", block.tag);
                            }
                        };
                    } else {
                        if let Some(style_prefabs) = style_prefabs {
                            if let Some(style_prefab) = style_prefabs.get(&block.tag) {
                                style = style_prefab.clone();
                            }
                        } else {
                            warn!("unrecognized tag `{}`, ignored.", block.tag)
                        }
                    }
                    let inner_sections =
                        self.parse_text_recursive(block.inner, &style, style_prefabs)?;
                    sections.extend(inner_sections);
                }
            }
        }

        Ok(sections)
    }
    pub fn parse_text(
        &self,
        text: &str,
        initial_text_style: &TextStyle,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<Vec<TextSection>, String> {
        let elements = parse(text)?;
        self.parse_text_recursive(elements, initial_text_style, style_prefabs)
    }
    #[cfg(feature = "parser")]
    pub fn layout_parse(
        &mut self,
        text: &str,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
        color_space: ColorSpace,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<(Vec<GlyphVertices>, u32, u32), String> {
        let text_sections = self.parse_text(text, initial_text_style, style_prefabs)?;
        Ok(self.layout(&layout_style, &text_sections, color_space))
    }

    pub fn layout<'a, T: AsRef<Vec<TextSection>>>(
        &mut self,
        layout_style: &LayoutStyle,
        text_sections: T,
        color_space: ColorSpace,
    ) -> (Vec<GlyphVertices>, u32, u32) {
        let mut total_width: f64 = 0.;
        let mut total_height: f64 = 0.;

        let text_sections = text_sections.as_ref();

        let mut current_x = text_sections
            .first()
            .and_then(|f| Some(f.style.indent * FONT_SIZE))
            .unwrap_or(0.);
        let mut current_y = 0.;

        let mut current_col: u32 = 0;
        let mut current_row: u32 = 0;

        let max_width = layout_style.box_width;
        let max_height = layout_style.box_height;

        // preallocate memory for  vertices and indices
        let mut glyph_vertices_vec =
            Vec::with_capacity(text_sections.iter().map(|s| s.text.len()).sum());

        'out: for section in text_sections {
            let style = &section.style;
            let text = &section.text;

            let buffer = 0.745;
            let gamma = 0.;
            let fill_color = get_color_value(&style.fill_color, &color_space);

            let StrokeStyle {
                stroke_color,
                stroke_width,
            } = style.stroke.clone().unwrap_or_default();
            let stroke_color = get_color_value(&stroke_color, &color_space);

            let ShadowStyle {
                shadow_color,
                shadow_offset_x,
                shadow_offset_y,
                shadow_blur,
                shadow_width,
            } = style.shadow.clone().unwrap_or_default();
            let shadow_color = get_color_value(&shadow_color, &color_space);

            // total size of this section in FONT_SIZE, so it must be scaled to font size later.
            let mut total_width_of_section: f64 = 0.;
            let mut total_height_of_section: f64 = 0.;

            for ch in text.chars() {
                let glyph = self.get_glyph(ch);
                let metrics = &glyph.metrics;

                // handles line break
                if glyph.ch == '\n' || glyph.ch == '\r' {
                    // update actual width
                    total_width_of_section = total_width_of_section.max(current_x);
                    // reset x
                    current_x = style.indent * FONT_SIZE;
                    // use original font size (when grid size is 64), it will be scaled in offset_y later.
                    current_y += FONT_SIZE * style.line_height;

                    current_col = 0;
                    current_row += 1;

                    // if text overflows the box, ignore the rest characters
                    if current_y / FONT_SIZE * style.font_size >= max_height {
                        break 'out;
                    }

                    // update actual height to current_y with additional a line
                    total_height_of_section = current_y + FONT_SIZE * style.line_height;

                    continue;
                }

                let mut h_advance = metrics.h_advance as f64;

                // check text overflow
                if (current_x + h_advance) / FONT_SIZE * style.font_size >= max_width {
                    // update actual width to max width
                    total_width_of_section = max_width * FONT_SIZE / style.font_size;
                    // reset x
                    current_x = 0.;
                    // use original font size (when grid size is 64), it will be scaled in offset_y later.
                    current_y += FONT_SIZE * style.line_height;

                    current_col = 0;
                    current_row += 1;

                    // if text overflows the box, ignore the rest characters
                    if current_y / FONT_SIZE * style.font_size >= max_height {
                        break 'out;
                    }

                    // update actual height to current_y with additional a line
                    total_height_of_section = current_y + FONT_SIZE * style.line_height;
                }

                let x_scale = metrics.x_scale.unwrap_or(1.) as f64;
                let y_scale = metrics.y_scale.unwrap_or(1.) as f64;

                let actual_width = metrics.width as f64 / x_scale;
                let actual_height = metrics.height as f64 / y_scale;

                let mut grid_scale_ratio_w = 1.;
                let grid_scale_ratio_h = 1.;
                let actual_scale_ratio = style.font_size / FONT_SIZE;

                // scale character letting width fulfills font size.
                // don't know why em/two-em dash have to do so.
                if glyph.ch == '—' || glyph.ch == '―' {
                    grid_scale_ratio_w = FONT_SIZE / actual_width as f64;
                    h_advance = FONT_SIZE;
                } else if glyph.ch == '⸺' {
                    grid_scale_ratio_w = FONT_SIZE * 2. / actual_width as f64;
                    h_advance = FONT_SIZE * 2.;
                } else if glyph.ch == '–' {
                    grid_scale_ratio_w = FONT_SIZE / 2. / actual_width as f64;
                    h_advance = FONT_SIZE / 2.;
                } else if glyph.ch == '⸻' {
                    grid_scale_ratio_w = FONT_SIZE * 3. / actual_width as f64;
                    h_advance = FONT_SIZE * 3.;
                }

                // scale by font size, 48 is the texture font size when the grid size is 64.
                let offset_x = current_x * actual_scale_ratio
                    - (GRID_SIZE / 2. / x_scale - actual_width / 2. - metrics.x_min as f64)
                        * actual_scale_ratio
                        * grid_scale_ratio_w;
                let offset_y = (current_y) * actual_scale_ratio
                    - (GRID_SIZE / 2. / y_scale - actual_height / 2. - ASCENT
                        + metrics.y_max as f64)
                        * actual_scale_ratio
                        * grid_scale_ratio_h;

                let actual_grid_size_w =
                    GRID_SIZE * actual_scale_ratio * grid_scale_ratio_w / x_scale;
                let actual_grid_size_h =
                    GRID_SIZE * actual_scale_ratio * grid_scale_ratio_h / y_scale;

                // calculate four vertices without multiplying with transform matrix

                let tx = offset_x / VIEWPORT_WIDTH;
                let ty = offset_y / VIEWPORT_HEIGHT;

                let w1 = 0.;
                let w0 = actual_grid_size_w / VIEWPORT_WIDTH;
                let h1 = 0.;
                let h0 = actual_grid_size_h / VIEWPORT_HEIGHT;

                // left top
                let p0x = w1 + tx;
                let p0y = h1 + ty;

                // left bottom
                let p1x = w1 + tx;
                let p1y = h0 + ty;

                // right top
                let p2x = w0 + tx;
                let p2y = h0 + ty;

                // right bottom
                let p3x = w0 + tx;
                let p3y = h1 + ty;

                let mut vertices_fill = Vec::with_capacity(4);
                let mut vertices_stroke = Vec::with_capacity(4);
                let mut vertices_shadow = Vec::with_capacity(4);
                let mut indices = Vec::with_capacity(4);

                vertices_fill.extend([
                    Vertex {
                        position: [p0x as f32, p0y as f32, 0.0],
                        tex_coords: [glyph.u_min, glyph.v_min],
                        page: glyph.page,
                        buffer,
                        gamma,
                        color: fill_color,
                    },
                    Vertex {
                        position: [p1x as f32, p1y as f32, 0.0],
                        tex_coords: [glyph.u_min, glyph.v_max],
                        page: glyph.page,
                        buffer,
                        gamma,
                        color: fill_color,
                    },
                    Vertex {
                        position: [p2x as f32, p2y as f32, 0.0],
                        tex_coords: [glyph.u_max, glyph.v_max],
                        page: glyph.page,
                        buffer,
                        gamma,
                        color: fill_color,
                    },
                    Vertex {
                        position: [p3x as f32, p3y as f32, 0.0],
                        tex_coords: [glyph.u_max, glyph.v_min],
                        page: glyph.page,
                        buffer,
                        gamma,
                        color: fill_color,
                    },
                ]);

                indices.extend([0, 1, 2, 0, 2, 3]);

                // insert vertices for stroke

                if style.stroke.is_some() {
                    // awesome magic number and algorithm, not sure why...
                    let buffer = 0.7
                        - GAMMA_COEFFICIENT * stroke_width
                            / 2.
                            / (style.font_size / FONT_SIZE) as f32;
                    vertices_stroke.extend([
                        Vertex {
                            position: [p0x as f32, p0y as f32, 0.0],
                            tex_coords: [glyph.u_min, glyph.v_min],
                            page: glyph.page,
                            buffer,
                            gamma,
                            color: stroke_color,
                        },
                        Vertex {
                            position: [p1x as f32, p1y as f32, 0.0],
                            tex_coords: [glyph.u_min, glyph.v_max],
                            page: glyph.page,
                            buffer,
                            gamma,
                            color: stroke_color,
                        },
                        Vertex {
                            position: [p2x as f32, p2y as f32, 0.0],
                            tex_coords: [glyph.u_max, glyph.v_max],
                            page: glyph.page,
                            buffer,
                            gamma,
                            color: stroke_color,
                        },
                        Vertex {
                            position: [p3x as f32, p3y as f32, 0.0],
                            tex_coords: [glyph.u_max, glyph.v_min],
                            page: glyph.page,
                            buffer,
                            gamma,
                            color: stroke_color,
                        },
                    ]);
                }

                // insert vertices for shadow

                if style.shadow.is_some() {
                    // awesome magic number and algorithm, not sure why...
                    let buffer = 0.7
                        - GAMMA_COEFFICIENT * shadow_width
                            / 2.
                            / (style.font_size / FONT_SIZE) as f32;
                    let gamma =
                        GAMMA_COEFFICIENT * shadow_blur / 2. / (style.font_size / FONT_SIZE) as f32;
                    let offset_x = shadow_offset_x / VIEWPORT_WIDTH as f32 * 2.;
                    let offset_y = shadow_offset_y / VIEWPORT_HEIGHT as f32 * 2.;
                    vertices_shadow.extend([
                        Vertex {
                            position: [p0x as f32 + offset_x, p0y as f32 + offset_y, 0.0],
                            tex_coords: [glyph.u_min, glyph.v_min],
                            page: glyph.page,
                            buffer,
                            gamma,
                            color: shadow_color,
                        },
                        Vertex {
                            position: [p1x as f32 + offset_x, p1y as f32 + offset_y, 0.0],
                            tex_coords: [glyph.u_min, glyph.v_max],
                            page: glyph.page,
                            buffer,
                            gamma,
                            color: shadow_color,
                        },
                        Vertex {
                            position: [p2x as f32 + offset_x, p2y as f32 + offset_y, 0.0],
                            tex_coords: [glyph.u_max, glyph.v_max],
                            page: glyph.page,
                            buffer,
                            gamma,
                            color: shadow_color,
                        },
                        Vertex {
                            position: [p3x as f32 + offset_x, p3y as f32 + offset_y, 0.0],
                            tex_coords: [glyph.u_max, glyph.v_min],
                            page: glyph.page,
                            buffer,
                            gamma,
                            color: shadow_color,
                        },
                    ]);
                }

                let glyph_vertices = GlyphVertices {
                    fill: vertices_fill,
                    stroke: vertices_stroke,
                    shadow: vertices_shadow,
                    indices,
                    col: current_col,
                    row: current_row,
                    x: current_x.round() as u32,
                    y: current_y.round() as u32,
                    width: h_advance.round() as u32,
                    height: (FONT_SIZE * style.line_height).round() as u32,
                };

                glyph_vertices_vec.push(glyph_vertices);

                current_x += h_advance;
                current_col += 1;
            }

            // in case of the last line without line break
            total_width_of_section = total_width_of_section.max(current_x);
            total_height_of_section = current_y + FONT_SIZE * style.line_height;

            // update total size
            total_width = total_width.max(total_width_of_section / FONT_SIZE * style.font_size);
            total_height += total_height_of_section / FONT_SIZE * style.font_size;
        }

        (
            glyph_vertices_vec,
            total_width.round() as u32,
            total_height.round() as u32,
        )
    }
}

fn parse_str<T: FromStr>(str: &str, fallback: T) -> T {
    str.parse::<T>().unwrap_or_else(|_| {
        warn!(
            "cannot parse string value `{}` to type `{}`.",
            str,
            std::any::type_name::<T>()
        );
        fallback
    })
}

fn parse_str_optional<T: FromStr>(str: &str, fallback: Option<T>) -> Option<T> {
    str.parse::<T>()
        .and_then(|v| Ok(Some(v)))
        .unwrap_or_else(|_| {
            warn!(
                "cannot parse string value `{}` to type `{}`.",
                str,
                std::any::type_name::<T>()
            );
            fallback
        })
}

#[inline]
fn get_color_value(color: &Color, color_space: &ColorSpace) -> [f32; 4] {
    match color_space {
        ColorSpace::Linear => color.to_linear_rgba_f32(),
        ColorSpace::SRGB => color.to_srgb_rgba_f32(),
    }
}
