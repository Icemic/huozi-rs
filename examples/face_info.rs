use std::env;
use std::fs;
use ttf_parser::Face;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <font-file.ttf> <character>", args[0]);
        std::process::exit(1);
    }

    let font_path = &args[1];
    let character = args[2].chars().next().expect("Invalid character");

    let data = fs::read(font_path).expect("Failed to read font file");
    let face = Face::parse(&data, 0).expect("Failed to parse font");

    let glyph_id = face.glyph_index(character);
    if let Some(glyph_id) = glyph_id {
        let advance = face.glyph_hor_advance(glyph_id).unwrap_or(0);
        let bbox = face.glyph_bounding_box(glyph_id);
        let upem = face.units_per_em();

        println!("Character: {}", character);
        println!("Codepoint: U+{:04X}", character as u32);
        println!("Glyph ID: {:?}", glyph_id.0);
        println!(
            "Advance Width: {} units ({} em)",
            advance,
            advance as f32 / upem as f32
        );
        if let Some(bb) = bbox {
            println!(
                "Bounding Box: xMin={}, yMin={}, xMax={}, yMax={}",
                bb.x_min, bb.y_min, bb.x_max, bb.y_max
            );
        } else {
            println!("Bounding Box: not available");
        }
        println!("Units per EM: {}", upem);
    } else {
        println!("Glyph not found in font for character '{}'", character);
    }
}
