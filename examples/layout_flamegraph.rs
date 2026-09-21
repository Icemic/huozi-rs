use std::hint::black_box;
use std::time::{Duration, Instant};

use csscolorparser::Color;
use huozi::layout::{ColorSpace, LayoutStyle};
use huozi::parser::{Segment, ShadowStyle, StrokeStyle, TextStyle};
use huozi::{FontSource, FontSourceKind, Huozi};

const DEFAULT_ITERATIONS: u32 = 10_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Fixed,
    Edit,
    Profile,
}

const DEFAULT_TEXT: &str = r#"一个简单的中日韩文字排印引擎，为游戏富文本特别设计。
A simple typography engine for CJK languages, especially designed for game rich-text.
huózì 活字 gM 123.!""?;:-_/+=<>==
CJK 标点——⸺，。：；“”？、《》「」【】
中文“引号”与western “quote”虽然是同一个字符，但需要渲染成不同的样子。
[locale=zh-hans]骨直肩示[/locale] [locale=zh-hant]骨直肩示[/locale] [locale=zh-hk]骨直肩示[/locale] [locale=ja-jp]骨直肩示[/locale] [locale=ko-kr]骨直肩示[/locale]
[font="思源黑体 VF"]思源黑体[/font] / [font="思源宋体 VF"]思源宋体[/font]
[font="思源黑体 VF"][weight=400]常规[/weight] / [bold]粗体[/bold][/font]
[font="Inter Variable"]Inter Normal / [italic]Inter Italic[/italic][/font]
"#;

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .map(|value| value.parse().expect("迭代次数必须是正整数"))
        .unwrap_or(DEFAULT_ITERATIONS);
    assert!(iterations > 0, "迭代次数必须大于零");
    let mode = match std::env::args().nth(2).as_deref() {
        None | Some("fixed") => Mode::Fixed,
        Some("edit") => Mode::Edit,
        Some("profile") => Mode::Profile,
        Some(_) => panic!("模式必须是 fixed、edit 或 profile"),
    };

    let font_sources = vec![
        FontSource::new(include_bytes!("../resources/fonts/InterVariable.ttf").to_vec())
            .with_kind(FontSourceKind::Latin),
        FontSource::new(include_bytes!("../resources/fonts/InterVariable-Italic.ttf").to_vec())
            .with_kind(FontSourceKind::Latin),
        FontSource::new(include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf").to_vec())
            .with_kind(FontSourceKind::Cjk),
    ];
    #[cfg(feature = "woff")]
    let font_sources = {
        let mut font_sources = font_sources;
        font_sources.push(
            FontSource::new(
                include_bytes!("../resources/fonts/SourceHanSerif-VF.otf.woff2").to_vec(),
            )
            .with_kind(FontSourceKind::Cjk),
        );
        font_sources
    };
    let mut huozi = Huozi::new(font_sources).expect("无法初始化 Huozi 字体管理器");
    let mut input_text = DEFAULT_TEXT.to_owned();
    let profile_inputs = (mode == Mode::Profile).then(|| {
        (0..iterations)
            .map(|iteration| format!("{DEFAULT_TEXT}{iteration:08x}"))
            .collect::<Vec<_>>()
    });
    let layout_style = LayoutStyle {
        box_width: Some(1280.0),
        box_height: Some(600.0),
        line_height: 1.5,
        indent: 0.0,
        ..LayoutStyle::default()
    };
    let text_style = TextStyle {
        font_size: 24.0,
        fill_color: Color::new(1.0, 1.0, 1.0, 1.0),
        font_weight: 400,
        stroke: Some(StrokeStyle {
            stroke_width: 1.0,
            stroke_color: Color::new(0.0, 0.0, 0.0, 1.0),
        }),
        shadow: Some(ShadowStyle {
            shadow_offset_x: 1.5,
            shadow_offset_y: 1.5,
            shadow_blur: 0.0,
            shadow_width: 0.4,
            shadow_color: Color::new(1.0, 0.25, 0.6, 1.0),
        }),
        ..TextStyle::default()
    };

    let warmup_summary = {
        let segments = vec![Segment::dummy(&input_text)];
        let (glyphs, spans, width, height) = huozi
            .layout_parse(
                &segments,
                &layout_style,
                &text_style,
                ColorSpace::SRGB,
                None,
            )
            .expect("预热排版失败");
        black_box((glyphs.len(), spans.len(), width, height))
    };

    let wall_started_at = Instant::now();
    let mut layout_elapsed = Duration::ZERO;
    let mut last_output = None;
    for iteration in 0..iterations {
        if mode == Mode::Edit {
            input_text.push('a');
        }
        let text = profile_inputs
            .as_ref()
            .map_or(input_text.as_str(), |inputs| {
                inputs[iteration as usize].as_str()
            });
        let segments = vec![Segment::dummy(text)];
        let layout_started_at = Instant::now();
        last_output = Some(
            huozi
                .layout_parse(
                    black_box(&segments),
                    black_box(&layout_style),
                    black_box(&text_style),
                    ColorSpace::SRGB,
                    None,
                )
                .expect("排版失败"),
        );
        layout_elapsed += layout_started_at.elapsed();
    }
    let wall_elapsed = wall_started_at.elapsed();
    let (glyphs, spans, width, height) = black_box(last_output.unwrap());
    let output_summary = (glyphs.len(), spans.len(), width, height);
    if mode == Mode::Fixed {
        assert_eq!(output_summary, warmup_summary, "重复排版的输出摘要发生变化");
    }
    let mode_name = match mode {
        Mode::Fixed => "fixed",
        Mode::Edit => "edit",
        Mode::Profile => "profile",
    };

    println!(
        "完成 {iterations} 次 {mode_name} 排版：排版耗时 {layout_elapsed:?}，平均每次 {:?}，墙钟耗时 {wall_elapsed:?}；最后一次输出 {} 个字形、{} 个来源区间，尺寸 {}×{}",
        layout_elapsed / iterations,
        glyphs.len(),
        spans.len(),
        width,
        height,
    );
}
