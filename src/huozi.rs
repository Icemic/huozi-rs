use anyhow::Result;
use image::{DynamicImage, RgbaImage};
use log::{info, warn};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::Path;

use crate::constant::{BUFFER, CUTOFF, FONT_SIZE, GRID_SIZE, RADIUS, TEXTURE_SIZE};
use crate::font_extractor::{GlyphExtractor, GlyphExtractorTrait, GlyphMetrics};
#[cfg(feature = "sdf")]
use crate::sdf::TinySDF;

#[derive(Debug, Clone, Default)]
pub struct Glyph {
    pub ch: char,
    pub metrics: GlyphMetrics,
    pub page: i32,
    pub index: u32,
    pub u_min: f32,
    pub u_max: f32,
    pub v_min: f32,
    pub v_max: f32,
}

pub struct Huozi {
    #[cfg(feature = "sdf")]
    extractor: GlyphExtractor,
    #[cfg(feature = "sdf")]
    tiny_sdf: TinySDF,
    image: RgbaImage,
    cache: lru::LruCache<char, Glyph>,
    next_grid_index: u32,
    /// increase this flag when the cache is changed
    image_version: u64,
}

impl Huozi {
    pub fn new(font_data: Vec<u8>) -> Self {
        let extractor = GlyphExtractor::new(font_data, FONT_SIZE as f32);

        info!("font metrics: {:?}", extractor.font_metrics());

        let mut image = DynamicImage::new_rgba8(TEXTURE_SIZE, TEXTURE_SIZE).to_rgba8();

        image.fill(0);

        #[cfg(feature = "sdf")]
        let tiny_sdf = TinySDF::new(GRID_SIZE as u32, BUFFER as u32, RADIUS, CUTOFF);

        let cache = LruCache::new(
            NonZeroUsize::new((TEXTURE_SIZE / GRID_SIZE as u32).pow(2) as usize * 4).unwrap(),
        );

        Self {
            #[cfg(feature = "sdf")]
            extractor,
            #[cfg(feature = "sdf")]
            tiny_sdf,
            image,
            cache,
            next_grid_index: 0,
            image_version: 0,
        }
    }

    #[cfg(feature = "sdf")]
    pub fn get_glyph(&mut self, ch: char) -> &Glyph {
        if self.cache.contains(&ch) {
            self.cache.get(&ch).unwrap()
        } else {
            if !self.extractor.exist(ch) {
                warn!("glyph for character `{}` ({:#x}) is not found in the current font or font fallbacks, which may lead to unexpected rendering result.", ch, ch as u16);
            }

            let (bitmap, metrics) = self.extractor.get_bitmap_and_metrics(ch);

            let (bitmap, width, height) =
                self.tiny_sdf
                    .calculate(&bitmap, metrics.width, metrics.height);

            // get a zero-valued Glyph and push to cache, which may expire a exising glyph
            let glyph = Glyph {
                ch,
                metrics,
                page: 0,
                index: 0,
                u_min: 0.,
                u_max: 0.,
                v_min: 0.,
                v_max: 0.,
            };

            let grid_size = GRID_SIZE as i32;

            let line_count = self.image.width() as i32 / grid_size;

            let (page, index_in_page) = if let Some((_, expired_glyph)) = self.cache.push(ch, glyph)
            {
                (expired_glyph.page, expired_glyph.index)
            } else {
                let page = self.next_grid_index as i32 / (line_count * line_count);
                let index_in_page = self.next_grid_index as i32 % (line_count * line_count);

                self.next_grid_index += 1;

                (page, index_in_page as u32)
            };

            // the next empty texture block, aligned by grid size
            let grid_x = grid_size * (index_in_page as i32 % line_count as i32);
            let grid_y = grid_size * (index_in_page as i32 / line_count as i32);

            let offset_x = grid_x + (GRID_SIZE / 2. - width as f64 / 2.).ceil() as i32;
            let offset_y = grid_y + (GRID_SIZE / 2. - height as f64 / 2.).ceil() as i32;

            let len = bitmap.len() as i32;

            for i in 0..len {
                let x = i % (width as i32) + offset_x;
                let y = i / (width as i32) + offset_y;

                if x < grid_x || x >= grid_x + grid_size || y <= grid_y || y >= grid_y + grid_size {
                    continue;
                }

                let v = bitmap[i as usize];

                let pixel = self.image.get_pixel_mut(x as u32, y as u32);
                pixel.0[page as usize] = v;
            }

            let texture_width = self.image.width() as f32;
            let glyph = self.cache.get_mut(&ch).unwrap();
            glyph.page = page;
            glyph.index = index_in_page;
            glyph.u_min = grid_x as f32 / texture_width;
            glyph.v_min = grid_y as f32 / texture_width;
            glyph.u_max = (grid_x + grid_size) as f32 / texture_width;
            glyph.v_max = (grid_y + grid_size) as f32 / texture_width;

            self.image_version += 1;

            glyph
        }
    }

    #[cfg(feature = "sdf")]
    pub fn preload(&mut self, charset: &str) {
        for (i, ch) in charset.chars().enumerate() {
            if i >= 4096 {
                warn!(
                    "The charset to preload has {} characters, which exceeds the limit of 4096, and the excess will be ignored.",
                    charset.len()
                );
                break;
            }

            self.get_glyph(ch);
        }
    }

    #[cfg(feature = "sdf")]
    pub fn image_version(&self) -> u64 {
        self.image_version
    }

    #[cfg(feature = "sdf")]
    pub fn texture_image(&self) -> &RgbaImage {
        &self.image
    }

    #[cfg(feature = "sdf")]
    pub fn dump_texture_to<Q>(&self, path: Q) -> Result<()>
    where
        Q: AsRef<Path>,
    {
        self.image.save(path)?;

        Ok(())
    }
}
