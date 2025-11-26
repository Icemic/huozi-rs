mod color_space;
mod glyph_span;
mod layout_style;
mod vertex;

use std::collections::HashMap;

use crate::parser::*;
use anyhow::Result;

pub use self::color_space::*;
pub use self::glyph_span::*;
pub use self::layout_style::*;
pub use self::vertex::*;

use crate::{
    constant::{ASCENT, FONT_SIZE, GAMMA_COEFFICIENT, GRID_SIZE, VIEWPORT_HEIGHT, VIEWPORT_WIDTH},
    glyph_vertices::GlyphVertices,
    parser::parse,
    Huozi,
};

impl Huozi {
    /// Parse the text into text spans.
    pub fn parse_text(
        &self,
        segments: &Vec<Segment>,
        initial_text_style: &TextStyle,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<Vec<TextSpan>, String> {
        let elements = segments
            .iter()
            .map(|segment| parse(segment))
            .collect::<Result<Vec<Vec<Element>>, String>>()?
            .into_iter()
            .flatten()
            .collect();
        to_spans(elements, initial_text_style, style_prefabs)
    }

    /// Parse the text with custom open and close tag characters.
    pub fn parse_text_with<const OPEN: char, const CLOSE: char>(
        &self,
        segments: &Vec<Segment>,
        initial_text_style: &TextStyle,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<Vec<TextSpan>, String> {
        let elements = segments
            .iter()
            .map(|segment| parse_with::<OPEN, CLOSE>(segment))
            .collect::<Result<Vec<Vec<Element>>, String>>()?
            .into_iter()
            .flatten()
            .collect();
        to_spans(elements, initial_text_style, style_prefabs)
    }

    /// Parse the text into text spans, then layout into glyph vertices.
    pub fn layout_parse(
        &mut self,
        segments: &Vec<Segment>,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
        color_space: ColorSpace,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<(Vec<GlyphVertices>, Vec<SegmentGlyphSpan>, u32, u32), String> {
        let text_spans = self.parse_text(segments, initial_text_style, style_prefabs)?;
        Ok(self.layout(&layout_style, &text_spans, color_space))
    }

    /// Parse the text with custom open and close tag characters, then layout into glyph vertices.
    pub fn layout_parse_with<const OPEN: char, const CLOSE: char>(
        &mut self,
        segments: &Vec<Segment>,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
        color_space: ColorSpace,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<(Vec<GlyphVertices>, Vec<SegmentGlyphSpan>, u32, u32), String> {
        let text_spans =
            self.parse_text_with::<OPEN, CLOSE>(segments, initial_text_style, style_prefabs)?;
        Ok(self.layout(&layout_style, &text_spans, color_space))
    }

    /// Layout the given text spans into glyph vertices according to the layout style and color space.
    pub fn layout<'a, T: AsRef<Vec<TextSpan>>>(
        &mut self,
        layout_style: &LayoutStyle,
        text_spans: T,
        color_space: ColorSpace,
    ) -> (Vec<GlyphVertices>, Vec<SegmentGlyphSpan>, u32, u32) {
        let mut total_width: f64 = 0.;
        let mut total_height: f64 = 0.;

        let first_run = text_spans
            .as_ref()
            .first()
            .and_then(|span| span.runs.first());

        let mut current_x = first_run
            .and_then(|f| Some(f.style.indent * FONT_SIZE))
            .unwrap_or(0.);
        let mut current_y = 0.;

        let mut current_col: u32 = 0;
        let mut current_row: u32 = 0;

        let max_width = layout_style.box_width;
        let max_height = layout_style.box_height;

        let mut glyph_vertices_vec = vec![];
        let mut segment_glyph_spans = vec![];
        let mut current_segment_id: Option<SegmentId> = None;
        let mut current_segment_range_start: usize = 0;

        for span in text_spans.as_ref() {
            let text_runs = &span.runs;

            // preallocate memory for vertices and indices
            glyph_vertices_vec.reserve(text_runs.iter().map(|s| s.text.len()).sum());

            'out: for run in text_runs {
                let style = &run.style;
                let text = &run.text;
                let segment_id = &run.source_range.segment_id;

                if segment_id != &current_segment_id {
                    // save previous segment span
                    if let Some(seg_id) = &current_segment_id {
                        segment_glyph_spans.push(SegmentGlyphSpan {
                            segment_id: seg_id.clone(),
                            glyph_range: current_segment_range_start..glyph_vertices_vec.len(),
                        });
                    }
                    // start a new segment span
                    current_segment_id = segment_id.clone();
                    current_segment_range_start = glyph_vertices_vec.len() - 1;
                }

                // Buffer value depends on color space due to gamma correction
                // Linear 0.5 corresponds to SRGB 0.735357, using precise theoretical values
                let buffer = match color_space {
                    ColorSpace::Linear => 0.5, // Industry standard for linear space (Mapbox, etc.)
                    ColorSpace::SRGB => 0.735357, // Precise theoretical conversion of Linear 0.5
                };
                // set a value larger than 1. means do not remove inner part
                // we need a value slightly larger than 1.0 to avoid the effect of anti-aliasing
                // 2.0 should be enough
                let fill_buffer = 2.;
                // 0.6 is a magic number, to enable anti-aliasing
                let gamma = GAMMA_COEFFICIENT * 0.6 / 2. / (style.font_size / FONT_SIZE) as f32;
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

                // total size of this run in FONT_SIZE, so it must be scaled to font size later.
                let mut total_width_of_run: f64 = 0.;
                let mut _total_height_of_run: f64 = 0.;

                for ch in text.chars() {
                    let glyph = self.get_glyph(ch);
                    let metrics = &glyph.metrics;

                    // handles line break
                    if glyph.ch == '\n' || glyph.ch == '\r' {
                        // update actual width
                        total_width_of_run = total_width_of_run.max(current_x);
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
                        _total_height_of_run = current_y + FONT_SIZE * style.line_height;

                        continue;
                    }

                    let mut h_advance = metrics.h_advance as f64;

                    // check text overflow
                    if (current_x + h_advance) / FONT_SIZE * style.font_size >= max_width {
                        // update actual width to max width
                        total_width_of_run = max_width * FONT_SIZE / style.font_size;
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
                        _total_height_of_run = current_y + FONT_SIZE * style.line_height;
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
                        - (GRID_SIZE * glyph.grid_count as f64 / 2. / x_scale
                            - actual_width / 2.
                            - metrics.x_min as f64)
                            * actual_scale_ratio
                            * grid_scale_ratio_w;
                    let offset_y = (current_y) * actual_scale_ratio
                        - (GRID_SIZE / 2. / y_scale - actual_height / 2. - ASCENT
                            + metrics.y_max as f64)
                            * actual_scale_ratio
                            * grid_scale_ratio_h;

                    let actual_grid_size_w = GRID_SIZE
                        * glyph.grid_count as f64
                        * actual_scale_ratio
                        * grid_scale_ratio_w
                        / x_scale;
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
                            fill_buffer,
                            gamma,
                            color: fill_color,
                        },
                        Vertex {
                            position: [p1x as f32, p1y as f32, 0.0],
                            tex_coords: [glyph.u_min, glyph.v_max],
                            page: glyph.page,
                            buffer,
                            fill_buffer,
                            gamma,
                            color: fill_color,
                        },
                        Vertex {
                            position: [p2x as f32, p2y as f32, 0.0],
                            tex_coords: [glyph.u_max, glyph.v_max],
                            page: glyph.page,
                            buffer,
                            fill_buffer,
                            gamma,
                            color: fill_color,
                        },
                        Vertex {
                            position: [p3x as f32, p3y as f32, 0.0],
                            tex_coords: [glyph.u_max, glyph.v_min],
                            page: glyph.page,
                            buffer,
                            fill_buffer,
                            gamma,
                            color: fill_color,
                        },
                    ]);

                    indices.extend([0, 1, 2, 0, 2, 3]);

                    // insert vertices for stroke

                    if style.stroke.is_some() {
                        // Stroke uses a different base buffer for visual effect
                        // Original algorithm used 0.7 in SRGB space for better stroke visibility
                        let base_buffer = match color_space {
                            ColorSpace::Linear => 0.448, // Precise conversion of SRGB 0.7
                            ColorSpace::SRGB => 0.7,     // Original empirically tuned value
                        };
                        let fill_buffer = buffer;
                        let buffer = base_buffer
                            - GAMMA_COEFFICIENT * stroke_width
                                / 2.
                                / (style.font_size / FONT_SIZE) as f32
                                * x_scale as f32
                                / grid_scale_ratio_w as f32;

                        // avoid minus (buffer - gamma) value passed to shader
                        let buffer = buffer.max(gamma);

                        vertices_stroke.extend([
                            Vertex {
                                position: [p0x as f32, p0y as f32, 0.0],
                                tex_coords: [glyph.u_min, glyph.v_min],
                                page: glyph.page,
                                buffer,
                                fill_buffer,
                                gamma,
                                color: stroke_color,
                            },
                            Vertex {
                                position: [p1x as f32, p1y as f32, 0.0],
                                tex_coords: [glyph.u_min, glyph.v_max],
                                page: glyph.page,
                                buffer,
                                fill_buffer,
                                gamma,
                                color: stroke_color,
                            },
                            Vertex {
                                position: [p2x as f32, p2y as f32, 0.0],
                                tex_coords: [glyph.u_max, glyph.v_max],
                                page: glyph.page,
                                buffer,
                                fill_buffer,
                                gamma,
                                color: stroke_color,
                            },
                            Vertex {
                                position: [p3x as f32, p3y as f32, 0.0],
                                tex_coords: [glyph.u_max, glyph.v_min],
                                page: glyph.page,
                                buffer,
                                fill_buffer,
                                gamma,
                                color: stroke_color,
                            },
                        ]);
                    }

                    // insert vertices for shadow

                    if style.shadow.is_some() {
                        // Shadow uses a different base buffer for visual effect
                        // Original algorithm used 0.7 in SRGB space for better shadow visibility
                        let base_buffer = match color_space {
                            ColorSpace::Linear => 0.448, // Precise conversion of SRGB 0.7
                            ColorSpace::SRGB => 0.7,     // Original empirically tuned value
                        };
                        // For shadow, if fill alpha is 0, which means no fill, so we do not draw shadow either,
                        // or else there should be shadow.
                        let fill_buffer = if fill_color[3] > 0.0 {
                            fill_buffer
                        } else {
                            buffer
                        };
                        let buffer = base_buffer
                            - GAMMA_COEFFICIENT * shadow_width
                                / 2.
                                / (style.font_size / FONT_SIZE) as f32
                                * x_scale as f32
                                / grid_scale_ratio_w as f32;
                        let gamma = GAMMA_COEFFICIENT * shadow_blur
                            / 2.
                            / (style.font_size / FONT_SIZE * 2.) as f32
                            * x_scale as f32
                            / grid_scale_ratio_w as f32;

                        // avoid minus (buffer - gamma) value passed to shader
                        let buffer = buffer.max(gamma);

                        let offset_x = shadow_offset_x / VIEWPORT_WIDTH as f32 * 2.;
                        let offset_y = shadow_offset_y / VIEWPORT_HEIGHT as f32 * 2.;
                        vertices_shadow.extend([
                            Vertex {
                                position: [p0x as f32 + offset_x, p0y as f32 + offset_y, 0.0],
                                tex_coords: [glyph.u_min, glyph.v_min],
                                page: glyph.page,
                                buffer,
                                fill_buffer,
                                gamma,
                                color: shadow_color,
                            },
                            Vertex {
                                position: [p1x as f32 + offset_x, p1y as f32 + offset_y, 0.0],
                                tex_coords: [glyph.u_min, glyph.v_max],
                                page: glyph.page,
                                buffer,
                                fill_buffer,
                                gamma,
                                color: shadow_color,
                            },
                            Vertex {
                                position: [p2x as f32 + offset_x, p2y as f32 + offset_y, 0.0],
                                tex_coords: [glyph.u_max, glyph.v_max],
                                page: glyph.page,
                                buffer,
                                fill_buffer,
                                gamma,
                                color: shadow_color,
                            },
                            Vertex {
                                position: [p3x as f32 + offset_x, p3y as f32 + offset_y, 0.0],
                                tex_coords: [glyph.u_max, glyph.v_min],
                                page: glyph.page,
                                buffer,
                                fill_buffer,
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
                        scale_ratio: actual_scale_ratio as f32,
                    };

                    glyph_vertices_vec.push(glyph_vertices);

                    current_x += h_advance;
                    current_col += 1;
                }

                // in case of the last line without line break
                total_width_of_run = total_width_of_run.max(current_x);
                _total_height_of_run = current_y + FONT_SIZE * style.line_height;

                // update total size
                total_width = total_width.max(total_width_of_run / FONT_SIZE * style.font_size);
                total_height += _total_height_of_run / FONT_SIZE * style.font_size;
            }
        }

        (
            glyph_vertices_vec,
            segment_glyph_spans,
            total_width.round() as u32,
            total_height.round() as u32,
        )
    }
}
