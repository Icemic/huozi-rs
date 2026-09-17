use std::time::SystemTime;

use huozi::charsets::{ASCII, CHS, CJK_SYMBOL};
use huozi::layout::{ColorSpace, LayoutStyle};
use huozi::parser::{Segment, TextStyle};
use image::RgbaImage;

fn main() {
    let font_data = std::fs::read("examples/assets/SourceHanSansSC-Regular.otf").unwrap();
    let mut huozi = huozi::Huozi::new(vec![huozi::FontSource::new(font_data)]).unwrap();
    // for this demo, just load the first 1024 characters, it will completely fill the red channel.
    let t = SystemTime::now();

    let text = ASCII
        .chars()
        .chain(CJK_SYMBOL.chars())
        .chain(CHS.chars())
        .take(1024)
        .collect::<String>();
    huozi
        .layout_plain(
            &vec![Segment::dummy(&text)],
            &LayoutStyle::default(),
            &TextStyle::default(),
            ColorSpace::SRGB,
        )
        .unwrap();

    println!(
        "SDF texture generated from 1024 shaped characters in {}ms",
        SystemTime::now().duration_since(t).unwrap().as_millis()
    );

    // copy red channel to green and blue channel, then fill alpha channel with 255 for easier viewing
    let texture = huozi.texture_pixels();
    let mut pixels = texture.pixels().to_vec();
    pixels.chunks_exact_mut(4).for_each(|chunk| {
        chunk[1] = chunk[0];
        chunk[2] = chunk[0];
        chunk[3] = 255;
    });
    RgbaImage::from_raw(texture.width(), texture.height(), pixels)
        .unwrap()
        .save("texture_dump.png")
        .unwrap();
}
