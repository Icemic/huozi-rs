use anyhow::Result;
use image::{DynamicImage, RgbaImage};
use log::{debug, warn, info};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::Path;

use crate::glyph::{FontdueExtractor, GlyphExtractor, GlyphMetrics};
use crate::layout::{
    calculate_layout, Color, LayoutDirection, LayoutStyle, TextSection, TextStyle, Vertex,
};
use crate::sdf::TinySDF;

#[derive(Debug, Clone, Default)]
pub struct Glyph {
    pub metrics: GlyphMetrics,
    pub page: i32,
    pub index: u32,
    pub u_min: f32,
    pub u_max: f32,
    pub v_min: f32,
    pub v_max: f32,
}

pub struct Huozi {
    extractor: FontdueExtractor,
    tiny_sdf: TinySDF,
    image: RgbaImage,
    cache: lru::LruCache<char, Glyph>,
    grid_size: u32,
    next_grid_index: u32,
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

        info!("font metrics: {:?}", extractor.font_metrics());

        let mut image = DynamicImage::new_rgba8(texture_size, texture_size).to_rgba8();

        image.fill(0);

        let tiny_sdf = TinySDF::new(grid_size, buffer, radius, cutoff);

        let cache = LruCache::new(
            NonZeroUsize::new((texture_size / grid_size).pow(2) as usize * 4).unwrap(),
        );

        Self {
            extractor,
            tiny_sdf,
            image,
            cache,
            grid_size,
            next_grid_index: 0,
        }
    }

    pub fn get_glyph(&mut self, ch: char) -> &Glyph {
        if self.cache.contains(&ch) {
            self.cache.get(&ch).unwrap()
        } else {
            let (bitmap, metrics) = self.extractor.get_bitmap_and_metrics(ch);

            let (bitmap, width, height) =
                self.tiny_sdf
                    .calculate(&bitmap, metrics.width, metrics.height);

            // get a zero-valued Glyph and push to cache, which may expire a exising glyph
            let glyph = Glyph {
                metrics,
                page: 0,
                index: 0,
                u_min: 0.,
                u_max: 0.,
                v_min: 0.,
                v_max: 0.,
            };

            let line_count = self.image.width() / self.grid_size;

            let (page, index_in_page) = if let Some((_, expired_glyph)) = self.cache.push(ch, glyph)
            {
                (expired_glyph.page, expired_glyph.index)
            } else {
                let page = self.next_grid_index / (line_count * line_count);
                let index_in_page = self.next_grid_index % (line_count * line_count);

                self.next_grid_index += 1;

                (page as i32, index_in_page)
            };

            // the next empty texture block, aligned by grid size
            let grid_x = self.grid_size as i32 * (index_in_page as i32 % line_count as i32);
            let grid_y = self.grid_size as i32 * (index_in_page as i32 / line_count as i32);

            let offset_x = grid_x + (self.grid_size as f64 / 2. - width as f64 / 2.).ceil() as i32;
            let offset_y = grid_y + (self.grid_size as f64 / 2. - height as f64 / 2.).ceil() as i32;

            let len = bitmap.len() as i32;

            for i in 0..len {
                let x = i % (width as i32) + offset_x;
                let y = i / (width as i32) + offset_y;

                if x < grid_x
                    || x >= grid_x + self.grid_size as i32
                    || y <= grid_y
                    || y >= grid_y + self.grid_size as i32
                {
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
            glyph.u_max = (grid_x + self.grid_size as i32) as f32 / texture_width;
            glyph.v_max = (grid_y + self.grid_size as i32) as f32 / texture_width;

            glyph
        }
    }

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

    pub fn texture_image(&self) -> &RgbaImage {
        &self.image
    }

    pub fn dump_texture_to<Q>(&self, path: Q) -> Result<()>
    where
        Q: AsRef<Path>,
    {
        self.image.save(path)?;

        Ok(())
    }

    pub fn layout<'a>(&mut self, text: &'a str) -> (Vec<Vertex>, Vec<u16>) {
        let mut section = vec![];

        for ch in text.chars() {
            let glyph = self.get_glyph(ch).clone();
            section.push(glyph);
        }

        calculate_layout(
            &LayoutStyle {
                direction: LayoutDirection::Horizontal,
                box_width: 800,
                box_height: 200,
                glyph_grid_size: 48,
            },
            &[TextSection {
                text: section,
                style: TextStyle {
                    font_size: 48,
                    fill_color: Color::from_html("#fff").unwrap(),
                    stroke: None,
                    shadow: None,
                },
            }],
        )
    }
}
