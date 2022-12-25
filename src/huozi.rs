use std::num::NonZeroUsize;

use image::{DynamicImage, Rgba, RgbaImage};
use log::warn;
use lru::LruCache;

use crate::{FontdueExtractor, Glyph, GlyphExtractor, TinySDF};

pub struct Huozi {
    extractor: FontdueExtractor,
    tiny_sdf: TinySDF,
    image: RgbaImage,
    cache: lru::LruCache<char, Glyph>,
    grid_size: u32,
}

impl Huozi {
    pub fn new(font_data: Vec<u8>) -> Self {
        let buffer = 8;
        let radius = 17.;
        let cutoff = 0.25;

        let grid_size = 64;
        let font_size = grid_size - buffer * 2;

        let texture_size = 2048;

        let extractor = FontdueExtractor::new(font_data, font_size as f32);

        let mut image = DynamicImage::new_rgba8(texture_size, texture_size).to_rgba8();

        image.fill(0);

        let tiny_sdf = TinySDF::new(grid_size, buffer, radius, cutoff);

        let cache =
            LruCache::new(NonZeroUsize::new((texture_size / grid_size).pow(2) as usize).unwrap());

        Self {
            extractor,
            tiny_sdf,
            image,
            cache,
            grid_size,
        }
    }

    pub fn preload(&mut self, charset: &str) {
        let line_count = self.image.width() / self.grid_size;

        for (i, ch) in charset.chars().enumerate() {
            if i >= 4096 {
                warn!(
                    "The charset to preload has {} characters, which exceeds the limit of 4096, and the excess will be ignored.",
                    charset.len()
                );
                break;
            }

            let (bitmap, metrics) = self.extractor.get_bitmap_and_metrics(ch);

            let block_x = self.grid_size as i32 * (i as i32 % line_count as i32);
            let block_y = self.grid_size as i32 * (i as i32 / line_count as i32);

            let (bitmap, width, height) =
                self.tiny_sdf
                    .calculate(&bitmap, metrics.width, metrics.height);

            let offset_x = block_x + (self.grid_size as f64 / 2. - width as f64 / 2.).ceil() as i32;
            let offset_y =
                block_y + (self.grid_size as f64 / 2. - height as f64 / 2.).ceil() as i32;

            let len = bitmap.len() as i32;

            for i in 0..len {
                let x = i % (width as i32) + offset_x;
                let y = i / (width as i32) + offset_y;

                if x < block_x
                    || x >= block_x + self.grid_size as i32
                    || y <= block_y
                    || y >= block_y + self.grid_size as i32
                {
                    continue;
                }

                let v = bitmap[i as usize];

                self.image
                    .put_pixel(x as u32, y as u32, Rgba([v, v, v, 255]));
            }
        }
    }
}
