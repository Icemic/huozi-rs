use std::time::SystemTime;

use huozi::charsets::{ASCII, CHS, CJK_SYMBOL};

fn main() {
    let font_data = std::fs::read("examples/assets/SourceHanSansSC-Regular.otf").unwrap();
    let mut huozi = huozi::Huozi::new(font_data);
    // for this demo, just load the first 1024 characters, it will completely fill the red channel.
    let t = SystemTime::now();

    huozi.preload(
        &ASCII
            .chars()
            .into_iter()
            .chain(CJK_SYMBOL.chars().into_iter())
            .chain(CHS.chars().into_iter())
            .take(1024)
            .collect::<String>(),
    );

    println!(
        "SDF texture preloaded 1024 characters in {}ms",
        SystemTime::now().duration_since(t).unwrap().as_millis()
    );

    // copy red channel to green and blue channel, then fill alpha channel with 255 for easier viewing
    let mut img = huozi.texture_image().clone();
    img.chunks_exact_mut(4).for_each(|chunk| {
        chunk[0] = chunk[0];
        chunk[1] = chunk[0];
        chunk[2] = chunk[0];
        chunk[3] = 255;
    });
    img.save("texture_dump.png").unwrap();
}
