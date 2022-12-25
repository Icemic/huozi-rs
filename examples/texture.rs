use huozi::charsets::{ASCII, CHS, CJK_SYMBOL};

fn main() {
    let font_data = std::fs::read("assets/SourceHanSansCN-Normal.otf").unwrap();
    let mut huozi = huozi::Huozi::new(font_data);
    huozi.preload(ASCII);
    huozi.preload(CJK_SYMBOL);
    huozi.preload(CHS);
    huozi.dump_texture_to("texture_dump.png").unwrap();
}
