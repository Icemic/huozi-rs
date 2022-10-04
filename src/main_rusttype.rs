use image::{DynamicImage, Rgba};
use rusttype::{point, Font, Scale};

fn main() {
    // Load the font
    let font_data = std::fs::read("assets/WenQuanYiMicroHei.ttf").unwrap();
    // This only succeeds if collection consists of one font
    let font = Font::try_from_bytes(&font_data).expect("Error constructing Font");

    // The font size to use
    let scale = Scale::uniform(64.0);

    // The text to render
    let text = "这M是testfga一段文字。";

    // Use a dark red colour
    let colour = (255, 255, 127);

    let v_metrics = font.v_metrics(scale);

    println!("{:?}", v_metrics);

    // layout the glyphs in a line with 20 pixels padding
    let glyphs: Vec<_> = font
        .layout(text, scale, point(20.0, 20.0 + v_metrics.ascent))
        .collect();

    // work out the layout size
    let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
    println!("height: {}", glyphs_height);
    let glyphs_width = {
        let min_x = glyphs
            .first()
            .map(|g| g.pixel_bounding_box().unwrap().min.x)
            .unwrap();
        let max_x = glyphs
            .last()
            .map(|g| g.pixel_bounding_box().unwrap().max.x)
            .unwrap();
        (max_x - min_x) as u32
    };

    // Create a new rgba image with some padding
    let mut image = DynamicImage::new_rgba8(glyphs_width + 40, glyphs_height + 40).to_rgba();

    // Loop through the glyphs in the text, positing each one on a line
    for glyph in glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            // Draw the glyph into the image per-pixel by using the draw closure
            glyph.draw(|x, y, v| {
                image.put_pixel(
                    // Offset the position by the glyph bounding box
                    x + bounding_box.min.x as u32,
                    y + bounding_box.min.y as u32,
                    // Turn the coverage into an alpha value
                    Rgba([colour.0, colour.1, colour.2, (v * 255.0) as u8]),
                )
            });
        }
    }

    // Save the image to a png file
    image.save("rusttype.png").unwrap();
    println!("Generated: image_example.png");
}
