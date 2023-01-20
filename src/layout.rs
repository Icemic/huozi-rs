mod color;
mod layout_style;
mod text_section;
mod text_style;
mod vertex;

pub use color::*;
pub use layout_style::*;
pub use text_section::*;
pub use text_style::*;
pub use vertex::*;

use crate::constant::{ASCENT, FONT_SIZE, GAMMA_COEFFICIENT, GRID_SIZE};

pub fn calculate_layout(
    layout_style: &LayoutStyle,
    text_sections: &[TextSection],
) -> (Vec<Vertex>, Vec<u16>) {
    let mut current_x = text_sections
        .first()
        .and_then(|f| Some(f.style.indent * FONT_SIZE))
        .unwrap_or(0.);
    let mut current_y = 0.;

    // assume a standard size of screen
    let viewport_width = layout_style.viewport_width;
    let viewport_height = layout_style.viewport_height;

    let max_width = layout_style.box_width;
    let max_height = layout_style.box_height;

    let mut vertices_fill = vec![];
    let mut indices_fill = vec![];

    let mut vertices_stroke = vec![];
    let mut indices_stroke = vec![];

    let mut vertices_shadow = vec![];
    let mut indices_shadow = vec![];

    'out: for section in text_sections {
        let style = &section.style;
        let text = &section.text;

        let buffer = 0.74;
        let gamma = 0.;
        let fill_color = style.fill_color.to_linear_rgba_f32();

        let StrokeStyle {
            stroke_color,
            stroke_width,
        } = style.stroke.clone().unwrap_or_default();
        let stroke_color = stroke_color.to_linear_rgba_f32();

        let ShadowStyle {
            shadow_color,
            shadow_offset_x,
            shadow_offset_y,
            shadow_blur,
            shadow_width,
        } = style.shadow.clone().unwrap_or_default();
        let shadow_color = shadow_color.to_linear_rgba_f32();

        for ch in text {
            let metrics = &ch.metrics;

            if ch.ch == '\n' || ch.ch == '\r' {
                current_x = style.indent * FONT_SIZE;
                // use original font size (when grid size is 64), it will be scaled in offset_y later.
                current_y += FONT_SIZE * style.line_height;

                // if text overflows the box, ignore the rest characters
                if current_y / FONT_SIZE * style.font_size >= max_height {
                    break 'out;
                }

                continue;
            }

            let mut h_advance = metrics.h_advance as f64;

            // check text overflow
            if (current_x + h_advance) / FONT_SIZE * style.font_size >= max_width {
                current_x = 0.;
                // use original font size (when grid size is 64), it will be scaled in offset_y later.
                current_y += FONT_SIZE * style.line_height;

                // if text overflows the box, ignore the rest characters
                if current_y / FONT_SIZE * style.font_size >= max_height {
                    break 'out;
                }
            }

            let x_scale = metrics.x_scale.unwrap_or(1.) as f64;
            let y_scale = metrics.y_scale.unwrap_or(1.) as f64;

            let actual_width = metrics.width as f64 / x_scale;
            let actual_height = metrics.height as f64 / y_scale;

            let mut grid_scale_ratio_w = 1.;
            let mut grid_scale_ratio_h = 1.;
            let actual_scale_ratio = style.font_size / FONT_SIZE;

            // scale character letting width fulfills font size.
            // don't know why em/two-em dash have to do so.
            if ch.ch == '—' || ch.ch == '―' {
                grid_scale_ratio_w = FONT_SIZE / actual_width as f64;
                h_advance = FONT_SIZE;
            } else if ch.ch == '⸺' {
                grid_scale_ratio_w = FONT_SIZE * 2. / actual_width as f64;
                h_advance = FONT_SIZE * 2.;
            } else if ch.ch == '–' {
                grid_scale_ratio_w = FONT_SIZE / 2. / actual_width as f64;
                h_advance = FONT_SIZE / 2.;
            } else if ch.ch == '⸻' {
                grid_scale_ratio_w = FONT_SIZE * 3. / actual_width as f64;
                h_advance = FONT_SIZE * 3.;
            }

            // scale by font size, 48 is the texture font size when the grid size is 64.
            let offset_x = current_x * actual_scale_ratio
                - (GRID_SIZE / 2. / x_scale - actual_width / 2. - metrics.x_min as f64)
                    * actual_scale_ratio
                    * grid_scale_ratio_w;
            let offset_y = (current_y) * actual_scale_ratio
                - (GRID_SIZE / 2. / y_scale - actual_height / 2. - ASCENT + metrics.y_max as f64)
                    * actual_scale_ratio
                    * grid_scale_ratio_h;

            let mut actual_grid_size_w =
                GRID_SIZE * actual_scale_ratio * grid_scale_ratio_w / x_scale;
            let mut actual_grid_size_h =
                GRID_SIZE * actual_scale_ratio * grid_scale_ratio_h / y_scale;

            // calculate four vertices without multiplying with transform matrix

            let tx = offset_x / viewport_width * 2.;
            let ty = 1. - offset_y / viewport_height * 2.;

            let w1 = 0.;
            let w0 = actual_grid_size_w / viewport_width * 2.;
            let h1 = -1. * actual_grid_size_h / viewport_height * 2.;
            let h0 = 0.;

            // left top
            let p0x = w1 + tx - 1.;
            let p0y = h1 + ty;

            // left bottom
            let p1x = w0 + tx - 1.;
            let p1y = h1 + ty;

            // right top
            let p2x = w0 + tx - 1.;
            let p2y = h0 + ty;

            // right bottom
            let p3x = w1 + tx - 1.;
            let p3y = h0 + ty;

            // insert vertices for fill
            let vertex_index_offset = vertices_fill.len() as u16;

            vertices_fill.push(Vertex {
                position: [p0x as f32, p0y as f32, 0.0],
                tex_coords: [ch.u_min, ch.v_max],
                page: ch.page,
                buffer,
                gamma,
                color: fill_color,
            });
            vertices_fill.push(Vertex {
                position: [p1x as f32, p1y as f32, 0.0],
                tex_coords: [ch.u_max, ch.v_max],
                page: ch.page,
                buffer,
                gamma,
                color: fill_color,
            });
            vertices_fill.push(Vertex {
                position: [p2x as f32, p2y as f32, 0.0],
                tex_coords: [ch.u_max, ch.v_min],
                page: ch.page,
                buffer,
                gamma,
                color: fill_color,
            });
            vertices_fill.push(Vertex {
                position: [p3x as f32, p3y as f32, 0.0],
                tex_coords: [ch.u_min, ch.v_min],
                page: ch.page,
                buffer,
                gamma,
                color: fill_color,
            });

            indices_fill.extend([
                vertex_index_offset + 0,
                vertex_index_offset + 1,
                vertex_index_offset + 2,
                vertex_index_offset + 0,
                vertex_index_offset + 2,
                vertex_index_offset + 3,
            ]);

            // insert vertices for stroke

            if style.stroke.is_some() {
                let vertex_index_offset = vertices_stroke.len() as u16;
                // awesome magic number and algorithm, not sure why...
                let buffer = 0.7
                    - GAMMA_COEFFICIENT * stroke_width / 2. / (style.font_size / FONT_SIZE) as f32;
                vertices_stroke.push(Vertex {
                    position: [p0x as f32, p0y as f32, 0.0],
                    tex_coords: [ch.u_min, ch.v_max],
                    page: ch.page,
                    buffer,
                    gamma,
                    color: stroke_color,
                });
                vertices_stroke.push(Vertex {
                    position: [p1x as f32, p1y as f32, 0.0],
                    tex_coords: [ch.u_max, ch.v_max],
                    page: ch.page,
                    buffer,
                    gamma,
                    color: stroke_color,
                });
                vertices_stroke.push(Vertex {
                    position: [p2x as f32, p2y as f32, 0.0],
                    tex_coords: [ch.u_max, ch.v_min],
                    page: ch.page,
                    buffer,
                    gamma,
                    color: stroke_color,
                });
                vertices_stroke.push(Vertex {
                    position: [p3x as f32, p3y as f32, 0.0],
                    tex_coords: [ch.u_min, ch.v_min],
                    page: ch.page,
                    buffer,
                    gamma,
                    color: stroke_color,
                });

                indices_stroke.extend([
                    vertex_index_offset + 0,
                    vertex_index_offset + 1,
                    vertex_index_offset + 2,
                    vertex_index_offset + 0,
                    vertex_index_offset + 2,
                    vertex_index_offset + 3,
                ]);
            }

            // insert vertices for shadow

            if style.shadow.is_some() {
                let vertex_index_offset = vertices_shadow.len() as u16;
                // awesome magic number and algorithm, not sure why...
                let buffer = 0.7
                    - GAMMA_COEFFICIENT * shadow_width / 2. / (style.font_size / FONT_SIZE) as f32;
                let gamma =
                    GAMMA_COEFFICIENT * shadow_blur / 2. / (style.font_size / FONT_SIZE) as f32;
                let offset_x = shadow_offset_x / viewport_width as f32 * 2.;
                let offset_y = -shadow_offset_y / viewport_height as f32 * 2.;
                vertices_shadow.push(Vertex {
                    position: [p0x as f32 + offset_x, p0y as f32 + offset_y, 0.0],
                    tex_coords: [ch.u_min, ch.v_max],
                    page: ch.page,
                    buffer,
                    gamma,
                    color: shadow_color,
                });
                vertices_shadow.push(Vertex {
                    position: [p1x as f32 + offset_x, p1y as f32 + offset_y, 0.0],
                    tex_coords: [ch.u_max, ch.v_max],
                    page: ch.page,
                    buffer,
                    gamma,
                    color: shadow_color,
                });
                vertices_shadow.push(Vertex {
                    position: [p2x as f32 + offset_x, p2y as f32 + offset_y, 0.0],
                    tex_coords: [ch.u_max, ch.v_min],
                    page: ch.page,
                    buffer,
                    gamma,
                    color: shadow_color,
                });
                vertices_shadow.push(Vertex {
                    position: [p3x as f32 + offset_x, p3y as f32 + offset_y, 0.0],
                    tex_coords: [ch.u_min, ch.v_min],
                    page: ch.page,
                    buffer,
                    gamma,
                    color: shadow_color,
                });

                indices_shadow.extend([
                    vertex_index_offset + 0,
                    vertex_index_offset + 1,
                    vertex_index_offset + 2,
                    vertex_index_offset + 0,
                    vertex_index_offset + 2,
                    vertex_index_offset + 3,
                ]);
            }

            current_x += h_advance;
        }
    }

    let indices_offset_fill = (vertices_shadow.len() + vertices_stroke.len()) as u16;
    let indices_offset_stroke = vertices_shadow.len() as u16;

    indices_fill
        .iter_mut()
        .for_each(|v| *v += indices_offset_fill);
    indices_stroke
        .iter_mut()
        .for_each(|v| *v += indices_offset_stroke);

    vertices_shadow.append(&mut vertices_stroke);
    vertices_shadow.append(&mut vertices_fill);
    indices_shadow.append(&mut indices_stroke);
    indices_shadow.append(&mut indices_fill);

    (vertices_shadow, indices_shadow)
}
