//! 图形呈现的公开行为：分段完整性与顶点契约。
//!
//! 这里只用公开 API。需要库内模板度量或分段函数的验证放在 `src/shape/` 对应文件里。

use huozi::{
    FontSource, Huozi,
    glyph_vertices::{BackgroundVertices, DecorationShape, DecorationVertices, UnitVertices},
    layout::{ColorSpace, LayoutStyle, RichTextLayoutOutput, Vertex},
    parser::{Segment, TextStyle},
};

const TEST_FONT: &[u8] = include_bytes!("../resources/fonts/SourceHanSansSC-VF.otf");

fn setup() -> Huozi {
    Huozi::new(vec![FontSource::new(TEST_FONT.to_vec())]).unwrap()
}

fn layout(huozi: &mut Huozi, text: &str, layout_style: &LayoutStyle) -> RichTextLayoutOutput {
    huozi
        .layout_parse(
            &vec![Segment::dummy(text)],
            layout_style,
            &TextStyle::default(),
            ColorSpace::SRGB,
            None,
        )
        .unwrap()
}

/// 元素里全部顶点的横向跨度。
fn span(vertices: &[Vertex]) -> (f32, f32) {
    vertices
        .iter()
        .fold((f32::MAX, f32::MIN), |(left, right), vertex| {
            (left.min(vertex.position[0]), right.max(vertex.position[0]))
        })
}

fn backgrounds(output: &RichTextLayoutOutput) -> Vec<&BackgroundVertices> {
    output
        .glyphs
        .iter()
        .filter_map(|element| match element {
            UnitVertices::Background(background) => Some(background),
            _ => None,
        })
        .collect()
}

fn decorations(output: &RichTextLayoutOutput) -> Vec<&DecorationVertices> {
    output
        .glyphs
        .iter()
        .filter_map(|element| match element {
            UnitVertices::Decoration(decoration) => Some(decoration),
            _ => None,
        })
        .collect()
}

fn shape_left(shape: &DecorationShape) -> f32 {
    match *shape {
        DecorationShape::Frame { left, .. } => left,
        DecorationShape::Dot {
            center_x, diameter, ..
        } => center_x - diameter / 2.0,
    }
}

fn shape_right(shape: &DecorationShape) -> f32 {
    match *shape {
        DecorationShape::Frame { right, .. } => right,
        DecorationShape::Dot {
            center_x, diameter, ..
        } => center_x + diameter / 2.0,
    }
}

/// 自带顶点的片段跨多个排版单元时，整段的首尾仍保留模板边距。
///
/// 若按片段矩形裁剪，描边与阴影会被切掉一截。
#[test]
fn background_fragments_keep_their_outer_margin() {
    let mut huozi = setup();
    let output = layout(
        &mut huozi,
        "[background color=\"#1E293B\" radius=8 paddingX=4 strokeColor=\"#F97316\" strokeWidth=1]甲乙[/background]",
        &LayoutStyle::default(),
    );
    let fragments = backgrounds(&output);
    assert_eq!(fragments.len(), 2, "两个 cluster 各有一段");

    let (first_left, first_right) = span(&fragments[0].vertices.fill);
    let (second_left, second_right) = span(&fragments[1].vertices.fill);
    let fragment_left = fragments[0].rect.left;
    let fragment_right = fragments[1].rect.right;
    assert!(
        first_left < fragment_left,
        "整段左端被切：{first_left} 未超出 {fragment_left}"
    );
    assert!(
        second_right > fragment_right,
        "整段右端被切：{second_right} 未超出 {fragment_right}"
    );

    // 描边层同样完整。
    let (stroke_left, _) = span(&fragments[0].vertices.stroke);
    let (_, stroke_right) = span(&fragments[1].vertices.stroke);
    assert!(stroke_left < fragment_left, "描边左端被切");
    assert!(stroke_right > fragment_right, "描边右端被切");

    // 两段在接缝处衔接，中间不能有缝。
    assert!(
        second_left <= first_right + 0.01,
        "接缝处留缝：{first_right} 与 {second_left}"
    );
}

/// 书名号的两端都必须有端帽，且端帽超出整段范围。
#[test]
fn book_title_keeps_both_end_caps() {
    let mut huozi = setup();
    let output = layout(
        &mut huozi,
        "[bookTitle]书名号[/bookTitle]",
        &LayoutStyle::default(),
    );
    let fragments = decorations(&output);
    assert!(fragments.len() >= 2, "书名号按 cluster 拆成多段");

    let group_left = fragments
        .iter()
        .map(|fragment| shape_left(&fragment.shape))
        .fold(f32::MAX, f32::min);
    let group_right = fragments
        .iter()
        .map(|fragment| shape_right(&fragment.shape))
        .fold(f32::MIN, f32::max);

    // 端帽中心落在端面上，形状向外延伸半个端帽加边距，因此整段两侧都应超出形状范围。
    let (left, right) = fragments
        .iter()
        .fold((f32::MAX, f32::MIN), |acc, fragment| {
            let (l, r) = span(&fragment.vertices.fill);
            (acc.0.min(l), acc.1.max(r))
        });
    assert!(
        left < group_left,
        "左端端帽被丢弃：{left} 未超出 {group_left}"
    );
    assert!(
        right > group_right,
        "右端端帽被丢弃：{right} 未超出 {group_right}"
    );
}

/// 图形顶点不携带额外平滑半宽，只有阴影层随 blur 增加。
#[test]
fn shapes_do_not_carry_extra_edge_smoothing() {
    let mut huozi = setup();
    let output = layout(
        &mut huozi,
        "[background color=\"#FFF3BF\" radius=8 shadowColor=\"#000000\" shadowBlur=2]甲[/background][underline color=\"#1677FF\" thickness=1]乙[/underline]",
        &LayoutStyle::default(),
    );
    for element in &output.glyphs {
        let (fill, shadow) = match element {
            UnitVertices::Background(background) => (
                Some(&background.vertices.fill),
                Some(&background.vertices.shadow),
            ),
            UnitVertices::Line(line) => (Some(&line.vertices.fill), Some(&line.vertices.shadow)),
            UnitVertices::Decoration(decoration) => (
                Some(&decoration.vertices.fill),
                Some(&decoration.vertices.shadow),
            ),
            _ => continue,
        };
        for vertex in fill.into_iter().flatten() {
            assert_eq!(vertex.gamma, 0.0, "图形填充不应携带额外平滑半宽");
        }
        for vertex in shadow.into_iter().flatten() {
            assert!(vertex.gamma > 0.0, "阴影应带模糊半宽");
        }
    }
}

/// 文字顶点必须排在它自己的位置上，且带完整的四边形。
#[test]
fn text_elements_carry_their_own_vertices() {
    let mut huozi = setup();
    let output = layout(&mut huozi, "甲", &LayoutStyle::default());
    let text = output
        .glyphs
        .iter()
        .find_map(|element| match element {
            UnitVertices::Text(text) => Some(text),
            _ => None,
        })
        .expect("示例文本产生了文字");
    assert_eq!(text.fill.len(), 4);
    assert!(text.shadow.is_none());
    assert!(text.stroke.is_none());
}
