use std::time::{Duration, Instant};

use huozi::charsets::{ASCII, CHS, CJK_SYMBOL};
use huozi::layout::{ColorSpace, LayoutStyle};
use huozi::parser::{Segment, TextStyle};
use image::RgbaImage;

const BENCHMARK_SAMPLES: usize = 10;

fn main() {
    let font_data = std::fs::read("examples/assets/SourceHanSansSC-Regular.otf").unwrap();
    let text = ASCII
        .chars()
        .chain(CJK_SYMBOL.chars())
        .chain(CHS.chars())
        .take(1024)
        .collect::<String>();
    let segments = vec![Segment::dummy(&text)];
    let layout_style = LayoutStyle::default();
    let text_style = TextStyle::default();
    let mut samples = Vec::with_capacity(BENCHMARK_SAMPLES);
    let mut texture_huozi = None;

    for sample_index in 0..=BENCHMARK_SAMPLES {
        let mut huozi = huozi::Huozi::new(vec![huozi::FontSource::new(font_data.clone())]).unwrap();
        let started_at = Instant::now();
        huozi
            .layout_plain(&segments, &layout_style, &text_style, ColorSpace::SRGB)
            .unwrap();
        let elapsed = started_at.elapsed();

        if sample_index > 0 {
            samples.push(elapsed);
        }
        if sample_index == BENCHMARK_SAMPLES {
            texture_huozi = Some(huozi);
        }
    }

    samples.sort_unstable();
    let median = samples[BENCHMARK_SAMPLES / 2];
    let total = samples.iter().sum::<Duration>();
    println!(
        "SDF cold-cache benchmark: {} samples, median {:?}, min {:?}, max {:?}, mean {:?}",
        BENCHMARK_SAMPLES,
        median,
        samples[0],
        samples[BENCHMARK_SAMPLES - 1],
        total / BENCHMARK_SAMPLES as u32,
    );

    // copy red channel to green and blue channel, then fill alpha channel with 255 for easier viewing
    let texture_huozi = texture_huozi.unwrap();
    let texture = texture_huozi.texture_pixels();
    let mut pixels = texture.pixels().to_vec();
    let checksum = pixels.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
    });
    let non_zero_pixels = pixels.iter().filter(|&&value| value != 0).count();
    println!(
        "SDF texture checksum: {checksum:016x}, non-zero bytes: {non_zero_pixels}",
    );
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
