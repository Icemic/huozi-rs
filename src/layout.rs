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

use crate::constant::{ASCENT, FONT_SIZE, GRID_SIZE};

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

    let mut vertexes = vec![];
    let mut indices = vec![];

    'out: for section in text_sections {
        let style = &section.style;
        let text = &section.text;

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

            let h_advance = metrics.h_advance as f64;

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

            // scale by font size, 48 is the texture font size when the grid size is 64.
            let offset_x =
                (current_x - GRID_SIZE / 2. + metrics.width as f64 / 2. + metrics.x_min as f64)
                    / FONT_SIZE
                    * style.font_size;
            let offset_y = (current_y - GRID_SIZE / 2. + metrics.height as f64 / 2. + ASCENT
                - metrics.y_max as f64)
                / FONT_SIZE
                * style.font_size;

            let actual_grid_size = GRID_SIZE / FONT_SIZE * style.font_size;

            // calculate four vertexes without multiplying with transform matrix

            let tx = offset_x / viewport_width * 2.;
            let ty = 1. - offset_y / viewport_height * 2.;

            let w1 = 0.;
            let w0 = actual_grid_size / viewport_width * 2.;
            let h1 = -1. * actual_grid_size / viewport_height * 2.;
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

            let vertex_index_offset = vertexes.len() as u16;

            vertexes.push(Vertex {
                position: [p0x as f32, p0y as f32, 0.0],
                tex_coords: [ch.u_min, ch.v_max],
                page: ch.page,
            });
            vertexes.push(Vertex {
                position: [p1x as f32, p1y as f32, 0.0],
                tex_coords: [ch.u_max, ch.v_max],
                page: ch.page,
            });
            vertexes.push(Vertex {
                position: [p2x as f32, p2y as f32, 0.0],
                tex_coords: [ch.u_max, ch.v_min],
                page: ch.page,
            });
            vertexes.push(Vertex {
                position: [p3x as f32, p3y as f32, 0.0],
                tex_coords: [ch.u_min, ch.v_min],
                page: ch.page,
            });

            indices.extend([
                vertex_index_offset + 0,
                vertex_index_offset + 1,
                vertex_index_offset + 2,
                vertex_index_offset + 0,
                vertex_index_offset + 2,
                vertex_index_offset + 3,
            ]);

            current_x += h_advance;
        }
    }

    (vertexes, indices)
}
