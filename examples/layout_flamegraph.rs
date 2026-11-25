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

const DEFAULT_TEXT: &str = r##"一个功能完善的中日韩文字排印引擎，为[shadow offsetX=1.5 offsetY=1.5 blur=0 width=0.4 color="rgba(255, 64, 153, 1.0)"]游戏富文本[/shadow]特别设计。[link id="demo-interaction" target="https://example.com/interaction"]点击这里显示交互提示[/link]
这是[wave]一段 [wave=false]让 [wave=true]平面[/wave] 设计师[/wave] 灵魂[/wave] / [shake]震颤[/shake] / [glow color="#FFD580"]的[/glow] / [glitch]文字[/glitch]
[font="思源黑体 VF"][cluster]ffi é[/cluster] [wave][glow]á́ ffi[/glow][/wave][/font]：连字和组合字符整簇显现，嵌套效果一起生效。
A fully functional typography engine for CJK languages, especially designed for game rich-text.
huózì 活字 gM 123.!""?;:-_/+=<>==
CJK 标点——⸺，。：；“”？、《》「」【】
中文[color=yellow]“[/color]引号[color=yellow]”[/color]与western [color=yellow]“[/color]quote[color=yellow]”[/color]虽然是同一个字符，但需要渲染成不同的样子。
[locale=zh-hans]骨直肩示[/locale] [locale=zh-hant]骨直肩示[/locale] [locale=zh-hk]骨直肩示[/locale] [locale=ja-jp]骨直肩示[/locale] [locale=ko-kr]骨直肩示[/locale]
[font="思源黑体 VF"]思源黑体[/font] / [font="Source Han Serif VF"]思源宋体[/font] ｜ [font="思源黑体 VF"][weight=400]常规[/weight] / [bold]粗体[/bold][/font] ｜ [font="Inter Variable"]Inter Normal / [italic]Inter Italic[/italic][/font]
[font="獅尾圓體SC"][weight=400]常规ABCxyz[/weight] / [bold]仿粗体ABCxyz[/bold] / [italic]仿斜体ABCxyz[/italic] / [bold][italic]仿粗斜体ABCxyz[/italic][/bold][/font]
[background color="#FFF3BF" paddingX=6 paddingY=2 radius=8]跨多个字格与行内空隙的圆角背景[/background]与[background color="rgba(56, 189, 248, 0.6)" paddingX=4 paddingY=2 radius=8 strokeColor="#0EA5E9" strokeWidth=1]相邻的另一个背景[/background]。
[background color="#1E293B" paddingX=6 paddingY=2 radius=8 shadowColor="#000000" shadowOffsetX=2 shadowOffsetY=2 shadowBlur=2 shadowWidth=1]带描边与阴影的背景[/background]与[code paddingX=4 paddingY=2 radius=8]cargo test[/code]共用同一套图形。
[underline color="#1677FF" thickness=1]实线下划线[/underline] ⁄ [underline color="#1677FF" thickness=1 pattern=dashed dashLength=3 gapLength=2]虚线下划线[/underline] ⁄ [underline color="#1677FF" thickness=1 pattern=dotted gapLength=2]点线下划线[/underline] ⁄ [lineThrough color="#94A3B8" thickness=1]删除线[/lineThrough]
[ruby text="tí qiàn"][bopomofo text="ㄊㄧˊ"]提[/bopomofo][bopomofo text="ㄑㄧㄢˋ"]椠[/bopomofo][/ruby]
[emphasis]着重号[/emphasis]、[mourning]示亡号[/mourning]、[properNoun]专名号[/properNoun]与[bookTitle]书名号[/bookTitle]
[link id="demo-rich-link" target="https://example.com/rich"]带背景的跨行链接=[/link]，[object id="demo-rich-object" alt="图标" width=24 ascent=18 descent=6 /]滚木滚木滚木
第一段结束。
[br /][br /]空段之后是新的一段。
"##;

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
            .with_kind(FontSourceKind::Western),
        FontSource::new(include_bytes!("../resources/fonts/InterVariable-Italic.ttf").to_vec())
            .with_kind(FontSourceKind::Western),
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
        let output = huozi
            .layout_parse(
                &segments,
                &layout_style,
                &text_style,
                ColorSpace::SRGB,
                None,
            )
            .expect("预热排版失败");
        black_box((
            output.glyphs.len(),
            output.segment_glyph_spans.len(),
            output.width,
            output.height,
        ))
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
    let output = black_box(last_output.unwrap());
    let output_summary = (
        output.glyphs.len(),
        output.segment_glyph_spans.len(),
        output.width,
        output.height,
    );
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
        output.glyphs.len(),
        output.segment_glyph_spans.len(),
        output.width,
        output.height,
    );
}
