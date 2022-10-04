use ab_glyph::{point, Font, FontRef, Glyph, PxScale, ScaleFont};
use piet::{
    kurbo::{Line, Point, Rect},
    Color, RenderContext,
};
use piet_common::{CoreGraphicsContext, Device};
use image::{DynamicImage, Rgba};


const WIDTH: usize = 800;
const HEIGHT: usize = 128;

/// Feature "png" needed for save_to_file() and it's disabled by default for optional dependencies
/// cargo run --example mondrian --features png
fn main() {
    let mut device = Device::new().unwrap();
    let mut bitmap = device.bitmap_target(WIDTH, HEIGHT, 1.0).unwrap();
    let mut ctx = bitmap.render_context();

    let font_size = 64.;

    let font_data = std::fs::read("assets/WenQuanYiMicroHei.ttf").unwrap();
    let font = FontRef::try_from_slice(&font_data).unwrap();

    println!("units per em: {}", font.units_per_em().unwrap());

    let scale = PxScale::from(font_size * font.height_unscaled() / font.units_per_em().unwrap());
    // let scale = font.pt_to_px_scale(48.).unwrap();
    let font = font.as_scaled(scale);

    println!(
        "ascent: {}\ndescent: {}\nheight: {}\nlineGap: {}",
        font.ascent(),
        font.descent(),
        font.height(),
        font.line_gap()
    );

    let q_line = Rect {
        x0: 0. - 1.,
        y0: font.ascent() as f64 - 1.,
        x1: WIDTH as f64,
        y1: font.ascent() as f64,
    };

    let bound_color = Color::from_hex_str("#fff").unwrap();
    ctx.fill(&q_line, &bound_color);
    

    let s = "这M是testfga一段文字。";
    let mut i = 0;

    let mut before = None;
    let mut pos_x = 0.;
    for c in s.chars() {
        draw_char(
            font.font,
            c,
            before,
            &mut ctx,
            font.scale().x,
            &mut pos_x,
            0.,
        );
        before = Some(c);
        i += 1;
    }

    ctx.finish().unwrap();
    std::mem::drop(ctx);

    bitmap
        .save_to_file("temp-image.png")
        .expect("file save error");
}

fn draw_char(
    font: &FontRef,
    char: char,
    before_char: Option<char>,
    ctx: &mut CoreGraphicsContext,
    font_size: f32,
    pos_x: &mut f32,
    pos_y: f32,
) {
    let font = font.as_scaled(font_size);
    // Get a glyph for 'q' with a scale & position.
    let q_glyph = font
        .glyph_id(char)
        .with_scale_and_position(font_size, point(*pos_x, pos_y));

    let bound_color = Color::from_hex_str("#fff").unwrap();
    let q_rect = {
        let r = font.glyph_bounds(&q_glyph);
        // println!(
        //     "glyph width: {} height: {}",
        //     r.max.x - r.min.x,
        //     r.max.y - r.min.y
        // );
        println!(
            "h_advance {:?}, v_advance {:?}, h_bearing: {}, char: {}, height: {}",
            font.h_advance(q_glyph.id),
            font.v_advance(q_glyph.id),
            font.h_side_bearing(q_glyph.id),
            char,
            r.max.y - r.min.y
        );
        Rect {
            x0: r.min.x as f64,
            y0: r.min.y as f64 + pos_y as f64,
            x1: r.max.x as f64,
            y1: r.max.y as f64 + pos_y as f64,
        }
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

    if let Some(q) = font.outline_glyph(q_glyph) {
        // println!("{:?}", q.px_bounds());
        q.draw(|x, y, c| {
            let rect = Rect {
                x0: (x as f64 - 1. + *pos_x as f64).ceil(),
                y0: (y as f64 - 1. + pos_y as f64).ceil(),
                x1: (x as f64 + *pos_x as f64).ceil(),
                y1: (y as f64 + pos_y as f64).ceil(),
            };
            let c = if c.abs() < f32::EPSILON {
                0.
            } else if 1. - c.abs() < f32::EPSILON {
                1.
            } else {
                c as f64
            };
            let color = Color::rgba8(255, 255, 127, (c * 255.0) as u8);
            ctx.fill(&rect, &color);
        });
    }

    *pos_x += h_advance;
}
