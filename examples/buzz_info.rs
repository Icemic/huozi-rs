use rustybuzz::Language;
use rustybuzz::{Face, UnicodeBuffer};
use std::env;
use std::fs;
use std::str::FromStr;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <font-file> <character> <lang>", args[0]);
        std::process::exit(1);
    }

    let font_path = &args[1];
    let ch = args[2].to_string();
    let lang = &args[3];

    let font_data = fs::read(font_path).expect("Failed to read font");
    let face = Face::from_slice(&font_data, 0).expect("Failed to parse font");

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(&ch);
    buffer.set_language(Language::from_str(lang).unwrap());

    // let features = &[Feature::new(Tag::from_bytes(&b"fwid"), 1, 0..999999)];
    // let features = &[Feature::new(Tag::from_bytes(&b"halt"), 1, 0..999999)];
    let features = &[];

    let glyph_buffer = rustybuzz::shape(&face, features, buffer);

    println!("Character: '{}'", ch);
    println!("Language: '{}'", lang);
    for (i, glyph) in glyph_buffer.glyph_infos().iter().enumerate() {
        let pos = &glyph_buffer.glyph_positions()[i];
        println!(
            "Glyph ID: {} | Advance X: {} | Offset: ({}, {})",
            glyph.glyph_id, pos.x_advance, pos.x_offset, pos.y_offset
        );
    }
}
