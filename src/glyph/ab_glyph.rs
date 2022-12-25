use ab_glyph::{point, Font, FontRef, FontVec, PxScale, ScaleFont};
use image::{DynamicImage, Rgba, RgbaImage};

use super::common::Glyph;

const WIDTH: usize = 800;
const HEIGHT: usize = 128;

pub struct GlyphExtractor {
    font: FontVec,
    font_size: f32,
    scale: PxScale,
}

impl GlyphExtractor {
    pub fn new(font_data: Vec<u8>, font_size: f32) -> Self {
        let font = FontVec::try_from_vec(font_data).unwrap();
        let scale =
            PxScale::from(font_size * font.height_unscaled() / font.units_per_em().unwrap());
        Self {
            font,
            font_size,
            scale,
        }
    }
    pub fn set_font_size(&mut self, font_size: f32) {
        let scale = PxScale::from(
            font_size * self.font.height_unscaled() / self.font.units_per_em().unwrap(),
        );
        self.font_size = font_size;
        self.scale = scale;
    }
    pub fn get_glyph(&self, ch: char) {
        let font = self.font.as_scaled(self.scale);
        let glyph = font.glyph_id(ch).with_scale(self.scale);
        let r = font.glyph_bounds(&glyph);
        let h_advance = font.h_advance(glyph.id);
        let h_bearing = font.h_side_bearing(glyph.id);

        println!(
            "h_advance: {}, h_bearing: {}, width: {}\nx min: {}, x max: {}",
            h_advance,
            h_bearing,
            r.width(),
            r.min.x,
            r.max.x
        )
    }
}

/// Feature "png" needed for save_to_file() and it's disabled by default for optional dependencies
/// cargo run --example mondrian --features png
pub fn main() {
    let mut image = DynamicImage::new_rgba8(800, 800).to_rgba8();

    let font_size = 48.;

    let font_data = std::fs::read("assets/SourceHanSansCN-Normal.otf").unwrap();
    let font = FontRef::try_from_slice(&font_data).unwrap();

    println!("units per em: {}", font.units_per_em().unwrap());

    let scale = PxScale::from(font_size * font.height_unscaled() / font.units_per_em().unwrap());
    // let scale = font.pt_to_px_scale(20.).unwrap();
    let font = font.as_scaled(scale);

    println!(
        "ascent: {}\ndescent: {}\nheight: {}\nlineGap: {}",
        font.ascent(),
        font.descent(),
        font.height(),
        font.line_gap()
    );

    let s = "这M是testfga一段文字。";
    let mut i = 0;

    let mut before = None;
    let mut pos_x = 0.;
    for c in s.chars() {
        draw_char(
            font.font,
            c,
            before,
            &mut image,
            font.scale().x,
            &mut pos_x,
            0.,
        );
        before = Some(c);
        i += 1;
    }

    image.save("output.png").unwrap();
}

fn draw_char(
    font: &FontRef,
    char: char,
    before_char: Option<char>,
    image: &mut RgbaImage,
    font_size: f32,
    pos_x: &mut f32,
    pos_y: f32,
) {
    let font = font.as_scaled(font_size);
    // Get a glyph for 'q' with a scale & position.
    let q_glyph = font
        .glyph_id(char)
        .with_scale_and_position(font_size, point(*pos_x, pos_y));

    let bound_color = (255, 255, 255);
    let q_rect = {
        let r = font.glyph_bounds(&q_glyph);
        println!(
            "h_advance {:?}, v_advance {:?}, h_bearing: {}, char: {}, height: {}",
            font.h_advance(q_glyph.id),
            font.v_advance(q_glyph.id),
            font.h_side_bearing(q_glyph.id),
            char,
            r.max.y - r.min.y
        );

        (
            r.min.x as f64,
            r.min.y as f64 + pos_y as f64,
            r.max.x as f64,
            r.max.y as f64 + pos_y as f64,
        )
    };
    // ctx.stroke(&q_rect, &bound_color, 1.0);

    let kern = if let Some(before_char) = before_char {
        let q_glyph_before = font
            .glyph_id(before_char)
            .with_scale_and_position(font_size, point(*pos_x, pos_y));

        let p = font.kern(q_glyph_before.id, q_glyph.id);

        println!("kern: {}", p);

        p
    } else {
        0.
    };

    let h_advance = font.h_advance(q_glyph.id);

    let colour = (255, 255, 127);

    if let Some(q) = font.outline_glyph(q_glyph) {
        // println!("{:?}", q.px_bounds());
        q.draw(|x, y, c| {
            let x = (x as f64 + *pos_x as f64).ceil() as u32;
            let y = (y as f64 + pos_y as f64).ceil() as u32;

            let c = if c.abs() < f32::EPSILON {
                0.
            } else if 1. - c.abs() < f32::EPSILON {
                1.
            } else {
                c as f64
            };

            image.put_pixel(
                // Offset the position by the glyph bounding box
                x,
                y,
                // Turn the coverage into an alpha value
                Rgba([colour.0, colour.1, colour.2, (c * 255.0) as u8]),
            );
        });
    }

    *pos_x += h_advance;
}
