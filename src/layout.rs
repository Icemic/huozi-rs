mod color;
mod layout_style;
mod text_section;
mod text_style;
mod vertex;

pub use color::*;
pub use layout_style::*;
use log::debug;
pub use text_section::*;
pub use text_style::*;
pub use vertex::*;

pub fn calculate_layout(
    layout_style: &LayoutStyle,
    text_sections: &[TextSection],
) -> (Vec<Vertex>, Vec<u16>) {
    let mut current_x = 0.;
    let mut current_y = 0.;

    // assume a standard size of screen
    let viewport_width = 2048.;
    let viewport_height = 2048.;

    let ascent = 56;

    let grid_size = layout_style.glyph_grid_size as f64;
    let max_width = layout_style.box_width as i32;
    let max_height = layout_style.box_height as i32;
    let grid_ratio = grid_size / 64.;

    let mut vertexes = vec![];
    let mut indices = vec![];

    'out: for section in text_sections {
        let style = &section.style;
        let text = &section.text;

        for ch in text {
            let metrics = &ch.metrics;

            let offset_x =
                current_x - grid_size / 2. + metrics.width as f64 / 2. + metrics.x_min as f64;
            let offset_y = current_y - grid_size / 2. + metrics.height as f64 / 2. + ascent as f64
                - metrics.y_max as f64;
            let width = grid_size / grid_ratio;
            // line-height: 1
            let height = grid_size / grid_ratio;

            // calculate four vertexes without multiplying with transform matrix

            let tx = offset_x / viewport_width * 2.;
            let ty = 1. - offset_y / viewport_height * 2.;

            let w1 = 0.;
            let w0 = width / viewport_width * 2.;
            let h1 = -1. * height / viewport_height * 2.;
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

            current_x += metrics.h_advance as f64;

            if current_x >= max_width as f64 {
                current_x = 0.;
                current_y += grid_size;

                // if text overflows the box, ignore the rest characters
                if current_y >= max_height as f64 {
                    break 'out;
                }
            }
        }
    }

    (vertexes, indices)
}
