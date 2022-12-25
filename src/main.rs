mod charsets;
mod glyph;
mod sdf;

use image::{DynamicImage, Rgba};

use crate::charsets::{ASCII, CHS, CJK_SYMBOL};
use crate::glyph::GlyphExtractor;
use crate::sdf::TinySDF;

fn main() {
    let font_data = std::fs::read("assets/SourceHanSansCN-Normal.otf").unwrap();
    let mut huozi = huozi::Huozi::new(font_data);
    huozi.preload(ASCII);
    huozi.preload(CJK_SYMBOL);
    huozi.preload(CHS);
    huozi.dump_texture_to("aaa.png").unwrap();
}

fn main11() {
    let env = env_logger::Env::default().default_filter_or("huozi=debug");
    env_logger::init_from_env(env);

    let buffer = 8;
    let radius = 17.;
    let cutoff = 0.25;

    let grid_size = 64;
    let font_size = grid_size - buffer * 2;

    // let cutoff = 0.25;
    // let grid_size = 64;
    // let font_size = (grid_size as f64 * 4. / 5.).ceil() as u32;
    // let buffer = (grid_size as f64 / 10.).ceil() as u32;
    // let radius = (grid_size as f64 * 4. / 15.).ceil();

    // glyph::ab_glyph::main();
    // let font_data = std::fs::read("assets/PingFang.ttc").unwrap();
    // let font_data = std::fs::read("assets/consolas.ttf").unwrap();
    // let font_data = std::fs::read("assets/WenQuanYiMicroHei.ttf").unwrap();
    let font_data = std::fs::read("assets/SourceHanSansCN-Normal.otf").unwrap();

    // let extractor = glyph::ab_glyph::GlyphExtractor::new(font_data.clone(), 100.);
    let extractor2 = glyph::FontdueExtractor::new(font_data.clone(), font_size as f32);
    // let extractor3 = glyph::rusttype::GlyphExtractor::new(font_data.clone(), 100.);
    // extractor.get_glyph('i');
    // extractor2.get_glyph('i');
    // let x = extractor2.font_metrics();

    let mut image = DynamicImage::new_rgba8(2048, 2048).to_rgba8();
    assert_eq!(
        image.width() % grid_size as u32,
        0,
        "width must be divided by font size."
    );
    assert_eq!(
        image.height() % grid_size as u32,
        0,
        "height must be divided by font size."
    );
    image.fill(0);
    image
        .chunks_mut(4)
        .for_each(|chunk| *chunk.last_mut().unwrap() = 255);

    // let s = "泽材灭逐莫笔亡鲜词";
    // let s = "泽材灭逐亡 fox agM啊。;；“”\"\"";
    // let s = "gM字;";

    let line_count = image.width() / grid_size as u32;

    let mut tiny_sdf = TinySDF::new(grid_size, buffer, radius, cutoff);

    for (i, ch) in (ASCII.to_string() + &CJK_SYMBOL.to_string() + &CHS.to_string())
        .as_str()
        .chars()
        .enumerate()
    {
        if i >= 1024 {
            break;
        }
        let (bitmap, metrics) = extractor2.get_bitmap_and_metrics(ch);

        let block_x = grid_size as i32 * (i as i32 % line_count as i32);
        let block_y = grid_size as i32 * (i as i32 / line_count as i32);

        // let offset_x = block_x
        //     + (grid_size as f64 / 2. - metrics.h_advance as f64 / 2. + metrics.x_min as f64).ceil()
        //         as i32;
        // let offset_y = block_y
        //     + (grid_size as f64 / 2. - font_size as f64 / 2. + ascent as f64
        //         - metrics.y_max as f64)
        //         .ceil() as i32;

        let (bitmap, width, height) = tiny_sdf.calculate(&bitmap, metrics.width, metrics.height);

        let offset_x = block_x + (grid_size as f64 / 2. - width as f64 / 2.).ceil() as i32;
        let offset_y = block_y + (grid_size as f64 / 2. - height as f64 / 2.).ceil() as i32;

        // println!(
        //     "{} {} {} {} {}",
        //     metrics.y_max,
        //     metrics.y_min,
        //     metrics.height,
        //     ascent - metrics.y_max,
        //     offset_y
        // );

        let len = bitmap.len() as i32;

        // println!("{} {} {}", metrics.y_max, metrics.y_min, metrics.height);

        for i in 0..len {
            let x = i % (width as i32) + offset_x;
            let y = i / (width as i32) + offset_y;

            // if x <= 0 || x >= image.width() as i32 || y <= 0 || y >= image.height() as i32 {
            //     continue;
            // }
            if x < block_x
                || x >= block_x + grid_size as i32
                || y <= block_y
                || y >= block_y + grid_size as i32
            {
                continue;
            }

            let v = bitmap[i as usize];

            image.put_pixel(x as u32, y as u32, Rgba([v, v, v, 255]));
        }
    }

    image.save("output.png").unwrap();
}
