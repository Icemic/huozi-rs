use csscolorparser::Color;
use log::warn;
use std::collections::HashMap;
use tiqian::core::geometry::{Rect, ScalarOffset, TextRange};
use tiqian::core::layout_model::{Glyph, LayoutResult};
use tiqian::core::layout_queries::{
    RichTextCornerRadii, RichTextLayerClusterSegment, positioned_clusters,
};
use tiqian::core::text_model::{
    DecorationKind, RichTextLayerKind, RichTextPaint, RichTextSemantic, RubyKind,
    TextStyle as TiqianTextStyle,
};

use crate::Huozi;
use crate::constant::{
    CUTOFF, EDGE_SMOOTHING_HALF_WIDTH, FILL_THRESHOLD_BIAS, FONT_SIZE, GRID_SIZE, RADIUS,
};
use crate::glyph_vertices::{
    BackgroundVertices, DecorationShape, DecorationVertices, DecorationWave, InlineObjectVertices,
    LineVertices, TextRole, TextVertices, UnitVertices,
};
use crate::huozi::Glyph as AtlasGlyph;
use crate::parser::SegmentId;
use crate::shape::{
    BackgroundFragment, DecorationFragment, FragmentGroup, LineFragment, ShapePaint, ShapeShadow,
    ShapeStroke, shape_backgrounds, shape_decorations, shape_lines,
};

use super::color_space::{ColorSpace, get_color_value};
use super::tiqian_input::HuoziSourceMap;
use super::{Interaction, InteractionArea, SegmentGlyphSpan, Vertex};

/// 默认正文填充色，与 parser 的 `TextStyle` 默认值一致。
const DEFAULT_FILL_ARGB: i32 = 0xFF1E_1E23_u32 as i32;
/// 装饰线宽相对字号的倍率，以及最小逻辑像素宽度。
const DECORATION_STROKE_EM: f32 = 1.0 / 16.0;
const DECORATION_STROKE_MIN: f32 = 1.0;
/// 书名号波浪的波周期与振幅相对字号的倍率。
const BOOK_TITLE_WAVELENGTH_EM: f32 = 0.4;
const BOOK_TITLE_AMPLITUDE_EM: f32 = 0.06;

/// 单段布局的输出片段。坐标已加上该段在文档中的纵向偏移。
#[derive(Default)]
pub(crate) struct ParagraphOutput {
    pub(crate) glyphs: Vec<UnitVertices>,
    /// 与 `glyphs` 等长，记录每个元素所属的输入 segment。
    pub(crate) glyph_segments: Vec<Option<SegmentId>>,
    pub(crate) interactions: Vec<Interaction>,
    pub(crate) width: f32,
    /// 已输出的视觉行数。
    pub(crate) visual_lines: u32,
    /// 已输出内容在该段内的本地高度：最后一个可见行的 bottom。
    pub(crate) height: f32,
}

pub(crate) struct HuoziTiqianOutputAdapter;

impl HuoziTiqianOutputAdapter {
    /// 把一段 Tiqian 布局结果转成绘制元素，并按 `row_offset`、`y_offset` 平移到文档坐标。
    ///
    /// 可见行只保留完整落入 `remaining_height` 的连续行；调用方应传入剩余的文档高度。
    pub(crate) fn adapt(
        huozi: &mut Huozi,
        result: &LayoutResult,
        source_map: &HuoziSourceMap,
        color_space: &ColorSpace,
        row_offset: u32,
        y_offset: f32,
        remaining_height: f32,
    ) -> ParagraphOutput {
        let positioned_clusters = positioned_clusters(result);
        let mut glyphs_by_cluster_range = HashMap::new();
        for glyph in result.glyph_runs.iter().flat_map(|run| &run.glyphs) {
            glyphs_by_cluster_range
                .entry(glyph.cluster_range)
                .or_insert_with(Vec::new)
                .push(glyph);
        }
        let visible_line_count = result
            .lines
            .iter()
            .take_while(|line| line.bottom <= remaining_height)
            .count();
        let mut output = ParagraphOutput::default();
        if visible_line_count == 0 {
            return output;
        }
        output.visual_lines = visible_line_count as u32;
        output.width = result.size.width.max(0.0);
        output.height = result.lines[visible_line_count - 1].bottom.max(0.0);

        let mut visible: HashMap<TextRange, VisibleCluster> = HashMap::new();
        let mut columns_by_line = vec![0_u32; visible_line_count];
        let mut text_style_cursor = 0;
        let mut rich_text_cursor = 0;
        let mut interaction_clusters = Vec::new();

        for (positioned_index, positioned) in positioned_clusters.iter().enumerate() {
            let line_index = positioned.line_index as usize;
            if line_index >= visible_line_count {
                continue;
            }
            let col = columns_by_line[line_index];
            columns_by_line[line_index] += 1;
            let row = row_offset + positioned.line_index as u32;
            let rect = shifted(positioned.rect(), y_offset);
            visible.insert(positioned.range, VisibleCluster { row, col, rect });
            interaction_clusters.push(InteractionCluster {
                range: positioned.range,
                rect,
                row,
                col,
            });
            let line_ends_after_cluster = positioned_clusters
                .get(positioned_index + 1)
                .is_none_or(|next| next.line_index != positioned.line_index);
            let cluster = &result.clusters[positioned.cluster_index as usize];
            if cluster.synthetic_kind.is_none()
                && let Some(glyphs) = glyphs_by_cluster_range.get(&positioned.range)
            {
                let segment_id = source_segment_id(source_map, positioned.range);
                let style = text_style_for_range(result, positioned.range, &mut text_style_cursor);
                let paints = text_paints_for_range(result, positioned.range, &mut rich_text_cursor);

                for glyph in glyphs {
                    if glyph.id != 0 && glyph.bounds.is_none() {
                        continue;
                    }
                    let Some(face) = glyph.render_font_face.as_ref() else {
                        warn!(
                            "skip glyph {} without a replay FontFaceId for cluster {:?}",
                            glyph.id, glyph.cluster_range
                        );
                        continue;
                    };
                    let atlas_glyph = huozi.get_glyph_by_id(face, glyph.id);
                    let text = glyph_vertices_for_glyph(
                        glyph,
                        &atlas_glyph,
                        positioned.draw_x + glyph.x,
                        positioned.baseline + glyph.y + y_offset,
                        row,
                        col,
                        positioned.top + y_offset,
                        positioned.bottom + y_offset,
                        style,
                        paints,
                        color_space,
                        TextRole::Body,
                    );
                    output.glyphs.push(UnitVertices::Text(text));
                    output.glyph_segments.push(segment_id.clone());
                }
            }

            if !line_ends_after_cluster {
                continue;
            }
            let line = &result.lines[line_index];
            if line.hyphen_glyphs.is_empty() || line.range.is_empty() {
                continue;
            }
            let hyphen_range = TextRange::new(line.range.end() - 1, line.range.end());
            let hyphen_segment_id = source_segment_id(source_map, hyphen_range);
            let style = text_style_for_range(result, hyphen_range, &mut text_style_cursor);
            let paints = text_paints_for_range(result, hyphen_range, &mut rich_text_cursor);
            for glyph in &line.hyphen_glyphs {
                if glyph.id != 0 && glyph.bounds.is_none() {
                    continue;
                }
                let Some(face) = glyph.render_font_face.as_ref() else {
                    warn!(
                        "skip line-end hyphen glyph {} without a replay FontFaceId",
                        glyph.id
                    );
                    continue;
                };
                let atlas_glyph = huozi.get_glyph_by_id(face, glyph.id);
                let text = glyph_vertices_for_glyph(
                    glyph,
                    &atlas_glyph,
                    line.indent + line.visual_width + glyph.x,
                    line.baseline + glyph.y + y_offset,
                    row,
                    col,
                    line.top + y_offset,
                    line.bottom + y_offset,
                    style,
                    paints,
                    color_space,
                    TextRole::Body,
                );
                output.glyphs.push(UnitVertices::Text(text));
                output.glyph_segments.push(hyphen_segment_id.clone());
            }
        }

        let mut continuity_fallback = u32::MAX;
        append_backgrounds(
            &mut output,
            huozi,
            result,
            source_map,
            &visible,
            color_space,
            &mut continuity_fallback,
        );
        append_lines(
            &mut output,
            huozi,
            result,
            source_map,
            &visible,
            color_space,
            &mut continuity_fallback,
        );
        append_decorations(
            &mut output,
            huozi,
            result,
            source_map,
            &visible,
            color_space,
            &mut continuity_fallback,
        );
        append_inline_objects(&mut output, result, source_map, &visible);
        append_annotations(
            &mut output,
            huozi,
            result,
            source_map,
            &visible,
            color_space,
        );

        output.interactions = interactions_from_tiqian_input(&interaction_clusters, result);
        // 各类元素是分组追加的，这里统一排成逐字显示顺序，使输出满足"前缀即可见状态"的保证。
        output.sort_into_display_order();
        output
    }
}

impl ParagraphOutput {
    /// 把 `glyphs` 与 `glyph_segments` 排成逐字显示顺序。
    ///
    /// 顺序是 `(row, col)` 为主序，同一排版单元内按绘制层排列：背景在文字之下，线条与装饰在其上，
    /// 注音跟随基文之后，行内对象最后。调用方取 `glyphs[..end]` 就是"打印到第 `end` 个元素"的完整
    /// 可见状态，不需要自己重排；渲染方仍按固定 paint layer 提交，与数组顺序无关。
    ///
    /// 行尾连字符与注音沿用其基文单元的 `(row, col)`，稳定排序保留它们的原始相对次序（正文、
    /// 连字符、注音）。同一位置内的元素属于同一 authored range，`glyph_segments` 随之重排后仍然
    /// 连续。
    fn sort_into_display_order(&mut self) {
        if self.glyphs.len() <= 1 {
            return;
        }
        // `order[target] = source`：目标位置应取的源下标。
        let mut order = (0..self.glyphs.len()).collect::<Vec<_>>();
        order.sort_by_key(|&index| display_order_key(&self.glyphs[index]));
        permute_pair(&mut self.glyphs, &mut self.glyph_segments, &order);
    }
}

/// 一个绘制元素在逐字显示顺序中的键：`(row, col, 显示次序)`。
fn display_order_key(element: &UnitVertices) -> (u32, u32, u8) {
    match element {
        UnitVertices::Background(background) => (background.row, background.col, 0),
        UnitVertices::Text(text) => (
            text.row,
            text.col,
            match text.role {
                TextRole::Body => 1,
                TextRole::Ruby | TextRole::Bopomofo => 3,
            },
        ),
        UnitVertices::Line(line) => (line.row, line.col, 2),
        UnitVertices::Decoration(decoration) => (decoration.row, decoration.col, 2),
        UnitVertices::InlineObject(object) => (object.row, object.col, 4),
    }
}

/// 按 `order[target] = source` 原地同步重排两个等长序列。
///
/// 用循环置换原地完成，不复制元素。
fn permute_pair<T, U>(first: &mut [T], second: &mut [U], order: &[usize]) {
    debug_assert_eq!(first.len(), second.len());
    debug_assert_eq!(first.len(), order.len());
    let mut visited = vec![false; first.len()];
    for start in 0..first.len() {
        if visited[start] {
            continue;
        }
        let mut current = start;
        loop {
            visited[current] = true;
            let source = order[current];
            if visited[source] {
                break;
            }
            first.swap(current, source);
            second.swap(current, source);
            current = source;
        }
    }
}

/// 把一段结果中的背景片段转成绘制元素。
///
/// 同一个连续范围在整段上定形，再按排版单元切成段：每段自带顶点，拼起来是完整的圆角矩形。
fn append_backgrounds(
    output: &mut ParagraphOutput,
    huozi: &mut Huozi,
    result: &LayoutResult,
    source_map: &HuoziSourceMap,
    visible: &HashMap<TextRange, VisibleCluster>,
    color_space: &ColorSpace,
    continuity_fallback: &mut u32,
) {
    let mut fragments = Vec::new();
    let mut segments = Vec::new();
    for fragment in result.rich_text_layer_cluster_segments() {
        let Some(layer) = fragment.span.layers.first() else {
            continue;
        };
        if !matches!(layer.kind, RichTextLayerKind::Background { .. }) {
            continue;
        }
        let Some(position) = position_of(result, visible, fragment.cluster_index) else {
            continue;
        };
        fragments.push(BackgroundFragment {
            group: FragmentGroup::Range(continuity_id(layer.id, continuity_fallback)),
            row: position.row,
            col: position.col,
            rect: fragment_rect(&fragment),
            corner_radii: fragment.corner_radii.unwrap_or(default_corner_radii()),
            paint: shape_paint(&layer.paints, color_space),
        });
        segments.push(segment_of_cluster(
            result,
            source_map,
            fragment.cluster_index,
        ));
    }
    let vertices = shape_backgrounds(huozi, &fragments);
    for (fragment, vertices) in fragments.into_iter().zip(vertices) {
        output
            .glyphs
            .push(UnitVertices::Background(BackgroundVertices {
                rect: fragment.rect,
                corner_radii: fragment.corner_radii,
                vertices,
                row: fragment.row,
                col: fragment.col,
            }));
    }
    output.glyph_segments.extend(segments);
}

/// 把一段结果中的下划线与删除线片段转成绘制元素。
fn append_lines(
    output: &mut ParagraphOutput,
    huozi: &mut Huozi,
    result: &LayoutResult,
    source_map: &HuoziSourceMap,
    visible: &HashMap<TextRange, VisibleCluster>,
    color_space: &ColorSpace,
    continuity_fallback: &mut u32,
) {
    let mut fragments = Vec::new();
    let mut segments = Vec::new();
    for fragment in result.rich_text_layer_cluster_segments() {
        let Some(layer) = fragment.span.layers.first() else {
            continue;
        };
        let line = match &layer.kind {
            RichTextLayerKind::Underline { line } | RichTextLayerKind::LineThrough { line } => line,
            _ => continue,
        };
        let Some(position) = position_of(result, visible, fragment.cluster_index) else {
            continue;
        };
        fragments.push(LineFragment {
            group: FragmentGroup::Range(continuity_id(layer.id, continuity_fallback)),
            row: position.row,
            col: position.col,
            left: fragment.left,
            right: fragment.right,
            line_y: fragment.line_y.unwrap_or(fragment.baseline),
            thickness: line.thickness,
            pattern: line.pattern.clone(),
            paint: shape_paint(&layer.paints, color_space),
        });
        segments.push(segment_of_cluster(
            result,
            source_map,
            fragment.cluster_index,
        ));
    }
    let vertices = shape_lines(huozi, &fragments);
    for (fragment, vertices) in fragments.into_iter().zip(vertices) {
        output.glyphs.push(UnitVertices::Line(LineVertices {
            left: fragment.left,
            right: fragment.right,
            line_y: fragment.line_y,
            thickness: fragment.thickness,
            pattern: fragment.pattern,
            vertices,
            row: fragment.row,
            col: fragment.col,
        }));
    }
    output.glyph_segments.extend(segments);
}

/// 把一段结果中的着重号与 CLREQ 装饰片段转成绘制元素。
fn append_decorations(
    output: &mut ParagraphOutput,
    huozi: &mut Huozi,
    result: &LayoutResult,
    source_map: &HuoziSourceMap,
    visible: &HashMap<TextRange, VisibleCluster>,
    color_space: &ColorSpace,
    continuity_fallback: &mut u32,
) {
    let decoration_paints = decoration_paint_layers(result);
    let mut fragments = Vec::new();
    let mut segments = Vec::new();
    for decision in &result.debug.decoration_decisions {
        if !decision.applied || decision.dot_diameter <= 0.0 {
            continue;
        }
        let Some(position) = visible.get(&decision.cluster_range).copied() else {
            continue;
        };
        fragments.push(DecorationFragment {
            // 着重号按字符独立出现，每个点自身就是一个完整图形。
            group: FragmentGroup::Solo,
            row: position.row,
            col: position.col,
            kind: decision.kind,
            shape: DecorationShape::Dot {
                center_x: decision.anchor_x,
                center_y: decision.anchor_y,
                diameter: decision.dot_diameter,
            },
            thickness: 0.0,
            wave: None,
            paint: decoration_shape_paint(
                &decoration_paints,
                decision.kind,
                decision.id,
                decision.cluster_range,
                color_space,
            ),
        });
        segments.push(segment_of_source_range(
            result,
            source_map,
            decision.cluster_range,
        ));
    }

    for fragment in result.decoration_cluster_segments() {
        let Some(position) = position_of(result, visible, fragment.cluster_index) else {
            continue;
        };
        let cluster_range = result
            .clusters
            .get(fragment.cluster_index as usize)
            .map(|cluster| cluster.range)
            .unwrap_or(fragment.range);
        let font_size = font_size_at(result, fragment.range.start());
        let thickness = (font_size * DECORATION_STROKE_EM).max(DECORATION_STROKE_MIN);
        let wave = (fragment.kind == DecorationKind::BookTitle).then_some(DecorationWave {
            wavelength: font_size * BOOK_TITLE_WAVELENGTH_EM,
            amplitude: font_size * BOOK_TITLE_AMPLITUDE_EM,
        });
        fragments.push(DecorationFragment {
            group: FragmentGroup::Range(continuity_id(fragment.id, continuity_fallback)),
            row: position.row,
            col: position.col,
            kind: fragment.kind,
            shape: DecorationShape::Frame {
                left: fragment.left,
                top: fragment.top,
                right: fragment.right,
                bottom: fragment.bottom,
                open_start: fragment.open_start,
                open_end: fragment.open_end,
            },
            thickness,
            wave,
            paint: decoration_shape_paint(
                &decoration_paints,
                fragment.kind,
                fragment.id,
                cluster_range,
                color_space,
            ),
        });
        segments.push(segment_of_cluster(
            result,
            source_map,
            fragment.cluster_index,
        ));
    }

    let vertices = shape_decorations(huozi, &fragments);
    for (fragment, vertices) in fragments.into_iter().zip(vertices) {
        output
            .glyphs
            .push(UnitVertices::Decoration(DecorationVertices {
                kind: fragment.kind,
                shape: fragment.shape,
                thickness: fragment.thickness,
                wave: fragment.wave,
                vertices,
                row: fragment.row,
                col: fragment.col,
            }));
    }
    output.glyph_segments.extend(segments);
}

/// 追加 ruby 与 bopomofo 注音 glyph。
///
/// 注音重放 Tiqian 已 shaping 的 glyph 与最终位置，不从注音文本再次 shaping；`row`、`col` 取自
/// `base_range` 覆盖的最后一个 positioned cluster。
fn append_annotations(
    output: &mut ParagraphOutput,
    huozi: &mut Huozi,
    result: &LayoutResult,
    source_map: &HuoziSourceMap,
    visible: &HashMap<TextRange, VisibleCluster>,
    color_space: &ColorSpace,
) {
    for decision in &result.debug.ruby_decisions {
        let Some(position) = base_position(result, visible, decision.base_range) else {
            continue;
        };
        let segment_id = source_segment_id(source_map, decision.base_range);
        let paints = annotation_paints(result, decision.base_range, RubyKind::Pinyin);
        let style = annotation_text_style(
            decision.font_size,
            &decision.font_families,
            &decision.locale,
            decision.font_weight,
        );
        let origin_x = decision.center_x - decision.width / 2.0;
        for glyph in &decision.glyphs {
            let Some(atlas_glyph) = annotation_atlas_glyph(huozi, glyph) else {
                continue;
            };
            output
                .glyphs
                .push(UnitVertices::Text(glyph_vertices_for_glyph(
                    glyph,
                    &atlas_glyph,
                    origin_x + glyph.x,
                    decision.baseline_y + glyph.y,
                    position.row,
                    position.col,
                    position.rect.top,
                    position.rect.bottom,
                    &style,
                    paints,
                    color_space,
                    TextRole::Ruby,
                )));
            output.glyph_segments.push(segment_id.clone());
        }
    }

    for decision in &result.debug.bopomofo_decisions {
        let Some(position) = base_position(result, visible, decision.base_range) else {
            continue;
        };
        let segment_id = source_segment_id(source_map, decision.base_range);
        let paints = annotation_paints(result, decision.base_range, RubyKind::Bopomofo);
        for placement in &decision.placements {
            let style = annotation_text_style(
                placement.font_size,
                &decision.font_families,
                &decision.locale,
                decision.font_weight,
            );
            for glyph in &placement.glyphs {
                let Some(atlas_glyph) = annotation_atlas_glyph(huozi, glyph) else {
                    continue;
                };
                output
                    .glyphs
                    .push(UnitVertices::Text(glyph_vertices_for_glyph(
                        glyph,
                        &atlas_glyph,
                        placement.draw_x + glyph.x,
                        placement.baseline_y + glyph.y,
                        position.row,
                        position.col,
                        position.rect.top,
                        position.rect.bottom,
                        &style,
                        paints,
                        color_space,
                        TextRole::Bopomofo,
                    )));
                output.glyph_segments.push(segment_id.clone());
            }
        }
    }
}

/// 查询注音 glyph 的图集项；缺少可回放字体身份时记录 warning 并跳过该 glyph。
fn annotation_atlas_glyph(huozi: &mut Huozi, glyph: &Glyph) -> Option<AtlasGlyph> {
    if glyph.id != 0 && glyph.bounds.is_none() {
        return None;
    }
    let Some(face) = glyph.render_font_face.as_ref() else {
        warn!(
            "skip annotation glyph {} without a replay FontFaceId for cluster {:?}",
            glyph.id, glyph.cluster_range
        );
        return None;
    };
    Some(huozi.get_glyph_by_id(face, glyph.id))
}

/// 返回注音基文范围对应的最后一个可见 cluster 位置。
fn base_position(
    result: &LayoutResult,
    visible: &HashMap<TextRange, VisibleCluster>,
    base_range: TextRange,
) -> Option<VisibleCluster> {
    let mut found = None;
    for cluster in &result.clusters {
        if cluster.range.start() < base_range.start() {
            continue;
        }
        if cluster.range.start() >= base_range.end() {
            break;
        }
        if let Some(position) = visible.get(&cluster.range) {
            found = Some(*position);
        }
    }
    found
}

/// 查询注音 layer 的 paint。
fn annotation_paints<'a>(
    result: &'a LayoutResult,
    base_range: TextRange,
    kind: RubyKind,
) -> &'a [RichTextPaint] {
    result
        .rich_text_annotation_layers(base_range, kind)
        .first()
        .map_or(&[], |layer| layer.paints.as_slice())
}

/// 为注音 glyph 构造只用于缩放与阈值的文字样式。
fn annotation_text_style(
    font_size: f32,
    font_families: &[String],
    locale: &str,
    font_weight: i32,
) -> TiqianTextStyle {
    TiqianTextStyle::builder()
        .font_families(font_families.to_vec())
        .font_size(font_size)
        .locale(locale.to_owned())
        .font_weight(font_weight)
        .build()
}

/// 把一段结果中的行内对象转成绘制元素。
fn append_inline_objects(
    output: &mut ParagraphOutput,
    result: &LayoutResult,
    source_map: &HuoziSourceMap,
    visible: &HashMap<TextRange, VisibleCluster>,
) {
    for decision in &result.debug.inline_object_decisions {
        let Some(object) = result
            .input
            .inline_objects
            .iter()
            .find(|object| object.range == decision.range)
        else {
            continue;
        };
        let Some(position) = position_of(result, visible, decision.cluster_index) else {
            continue;
        };
        output
            .glyphs
            .push(UnitVertices::InlineObject(InlineObjectVertices {
                rect: position.rect,
                id: object.id.clone(),
                alt: result
                    .input
                    .content
                    .text
                    .slice_text(decision.range)
                    .as_str()
                    .to_owned(),
                row: position.row,
                col: position.col,
            }));
        output.glyph_segments.push(segment_of_cluster(
            result,
            source_map,
            decision.cluster_index,
        ));
    }
}

/// 一个已输出 cluster 在文档坐标中的位置与矩形。
#[derive(Clone, Copy)]
struct VisibleCluster {
    row: u32,
    col: u32,
    rect: Rect,
}

/// 返回可见片段对应的位置与矩形；不可见或未登记的 cluster 返回 `None`。
fn position_of(
    result: &LayoutResult,
    visible: &HashMap<TextRange, VisibleCluster>,
    cluster_index: i32,
) -> Option<VisibleCluster> {
    visible
        .get(&result.clusters.get(cluster_index as usize)?.range)
        .copied()
}

/// 返回 cluster 对应的输入 segment。
fn segment_of_cluster(
    result: &LayoutResult,
    source_map: &HuoziSourceMap,
    cluster_index: i32,
) -> Option<SegmentId> {
    result
        .clusters
        .get(cluster_index as usize)
        .and_then(|cluster| source_segment_id(source_map, cluster.range))
}

/// 返回 source range 所属的输入 segment。
fn segment_of_source_range(
    _result: &LayoutResult,
    source_map: &HuoziSourceMap,
    range: TextRange,
) -> Option<SegmentId> {
    source_segment_id(source_map, range)
}

/// 未提供圆角时的零圆角回退值。
fn default_corner_radii() -> RichTextCornerRadii {
    RichTextCornerRadii {
        top_left: 0.0,
        top_right: 0.0,
        bottom_right: 0.0,
        bottom_left: 0.0,
    }
}

fn fragment_rect(fragment: &RichTextLayerClusterSegment) -> Rect {
    Rect {
        left: fragment.left,
        top: fragment.top,
        right: fragment.right,
        bottom: fragment.bottom,
    }
}

fn shifted(rect: Rect, y_offset: f32) -> Rect {
    Rect {
        top: rect.top + y_offset,
        bottom: rect.bottom + y_offset,
        ..rect
    }
}

/// 读取片段所属 layer 给出的连续范围身份；未声明身份时按从大到小分配，避免与真实身份相撞。
fn continuity_id(id: Option<u32>, fallback: &mut u32) -> u32 {
    match id {
        Some(id) => id,
        None => {
            *fallback -= 1;
            *fallback
        }
    }
}

/// 把 Tiqian 的 ARGB paint 转成后端无关的绘制参数。
fn shape_paint(paints: &[RichTextPaint], color_space: &ColorSpace) -> ShapePaint {
    let fill = paints
        .iter()
        .find_map(|paint| match paint {
            RichTextPaint::Fill { argb } => Some(argb_color(*argb, color_space)),
            _ => None,
        })
        .unwrap_or_else(|| argb_color(DEFAULT_FILL_ARGB, color_space));
    let stroke = paints.iter().find_map(|paint| match paint {
        RichTextPaint::Stroke { argb, width } => Some(ShapeStroke {
            color: argb_color(*argb, color_space),
            width: *width,
        }),
        _ => None,
    });
    let shadow = paints.iter().find_map(|paint| match paint {
        RichTextPaint::Shadow {
            argb,
            offset_x,
            offset_y,
            blur_radius,
            spread_radius,
        } => Some(ShapeShadow {
            color: argb_color(*argb, color_space),
            offset_x: *offset_x,
            offset_y: *offset_y,
            blur_radius: *blur_radius,
            spread_radius: *spread_radius,
        }),
        _ => None,
    });
    ShapePaint {
        fill,
        stroke,
        shadow,
    }
}

/// 按输入 segment 收集每个 segment 在 `glyphs` 中占据的连续下标范围。
pub(crate) fn segment_glyph_spans(glyph_segments: &[Option<SegmentId>]) -> Vec<SegmentGlyphSpan> {
    let mut spans = Vec::new();
    let mut current: Option<SegmentId> = None;
    let mut start = 0;
    for (index, segment_id) in glyph_segments.iter().enumerate() {
        if *segment_id == current {
            continue;
        }
        if let Some(segment_id) = current.take() {
            spans.push(SegmentGlyphSpan {
                segment_id,
                glyph_range: start..index,
            });
        }
        current = segment_id.clone();
        start = index;
    }
    if let Some(segment_id) = current {
        spans.push(SegmentGlyphSpan {
            segment_id,
            glyph_range: start..glyph_segments.len(),
        });
    }
    spans
}

/// 返回指定 source offset 的生效字号；找不到时退回段落默认字号。
fn font_size_at(result: &LayoutResult, offset: ScalarOffset) -> f32 {
    let spans = &result.input.content.spans;
    let index = spans.partition_point(|span| span.range.end() <= offset);
    spans
        .get(index)
        .filter(|span| span.range.start() <= offset && span.range.end() > offset)
        .map_or(result.input.text_style.font_size, |span| {
            span.style.font_size
        })
}

/// 把一个 CRLEQ 装饰层及其范围与身份记录下来，供装饰片段查找 paint。
struct DecorationPaintLayer<'a> {
    kind: DecorationKind,
    id: Option<u32>,
    range: TextRange,
    paints: &'a [RichTextPaint],
}

fn decoration_paint_layers(result: &LayoutResult) -> Vec<DecorationPaintLayer<'_>> {
    result
        .input
        .rich_text
        .iter()
        .flat_map(|span| {
            span.layers
                .iter()
                .filter_map(move |layer| match layer.kind {
                    RichTextLayerKind::Decoration { kind } => Some(DecorationPaintLayer {
                        kind,
                        id: layer.id,
                        range: span.range,
                        paints: &layer.paints,
                    }),
                    _ => None,
                })
        })
        .collect()
}

/// 查找装饰元素的 paint：优先取范围覆盖它的同身份层，退而取同身份层的第一个。
fn decoration_shape_paint(
    layers: &[DecorationPaintLayer<'_>],
    kind: DecorationKind,
    id: Option<u32>,
    range: TextRange,
    color_space: &ColorSpace,
) -> ShapePaint {
    let mut fallback = None;
    for layer in layers {
        if layer.kind != kind || layer.id != id {
            continue;
        }
        if layer.range.start() <= range.start() && layer.range.end() >= range.end() {
            return shape_paint(layer.paints, color_space);
        }
        fallback.get_or_insert(layer.paints);
    }
    shape_paint(fallback.unwrap_or(&[]), color_space)
}

struct InteractionCluster {
    range: TextRange,
    rect: Rect,
    row: u32,
    col: u32,
}

struct IdentifiedRange {
    id: String,
    range: TextRange,
    /// 同范围的链接包裹对象时，对象作为后进入的内层元素。
    is_object: bool,
}

fn interactions_from_tiqian_input(
    clusters: &[InteractionCluster],
    result: &LayoutResult,
) -> Vec<Interaction> {
    let mut identified_ranges = result
        .input
        .inline_objects
        .iter()
        .filter_map(|object| {
            object.id.as_ref().map(|id| IdentifiedRange {
                id: id.clone(),
                range: object.range,
                is_object: true,
            })
        })
        .chain(result.input.rich_text.iter().flat_map(|span| {
            span.semantics
                .iter()
                .filter_map(move |semantic| match semantic {
                    RichTextSemantic::Link { id: Some(id), .. } => Some(IdentifiedRange {
                        id: id.clone(),
                        range: span.range,
                        is_object: false,
                    }),
                    RichTextSemantic::Link { id: None, .. } | RichTextSemantic::TechnicalInline => {
                        None
                    }
                })
        }))
        .collect::<Vec<_>>();
    identified_ranges.sort_by_key(|interaction| {
        (
            interaction.range.start(),
            std::cmp::Reverse(interaction.range.end()),
            interaction.is_object,
        )
    });
    let mut ordered_ranges = Vec::with_capacity(identified_ranges.len());
    let mut open_ranges = Vec::new();
    for interaction in identified_ranges {
        while open_ranges
            .last()
            .is_some_and(|parent: &IdentifiedRange| interaction.range.end() > parent.range.end())
        {
            if let Some(parent) = open_ranges.pop() {
                ordered_ranges.push(parent);
            }
        }
        open_ranges.push(interaction);
    }
    while let Some(interaction) = open_ranges.pop() {
        ordered_ranges.push(interaction);
    }

    ordered_ranges
        .into_iter()
        .filter_map(|interaction| {
            let start = clusters
                .partition_point(|cluster| cluster.range.end() <= interaction.range.start());
            let areas = clusters[start..]
                .iter()
                .take_while(|cluster| cluster.range.start() < interaction.range.end())
                .map(|cluster| InteractionArea {
                    rect: cluster.rect,
                    row: cluster.row,
                    col: cluster.col,
                })
                .collect::<Vec<_>>();
            (!areas.is_empty()).then(|| Interaction {
                id: interaction.id,
                areas,
            })
        })
        .collect()
}

/// 返回显示文本范围内所属的输入 segment。
///
/// `HuoziSourceMap.entries` 按显示范围有序，因此可以二分定位；调用方不需要维护游标。
fn source_segment_id(
    source_map: &HuoziSourceMap,
    range: tiqian::core::geometry::TextRange,
) -> Option<SegmentId> {
    let entries = &source_map.entries;
    let index = entries.partition_point(|entry| entry.display_range.end() <= range.start());
    entries.get(index).and_then(|entry| {
        (entry.display_range.start() <= range.start() && entry.display_range.end() >= range.end())
            .then(|| entry.source_range.segment_id.clone())
            .flatten()
    })
}

fn text_style_for_range<'a>(
    result: &'a LayoutResult,
    range: tiqian::core::geometry::TextRange,
    cursor: &mut usize,
) -> &'a TiqianTextStyle {
    while result
        .input
        .content
        .spans
        .get(*cursor)
        .is_some_and(|span| span.range.end() <= range.start())
    {
        *cursor += 1;
    }
    result
        .input
        .content
        .spans
        .get(*cursor)
        .and_then(|span| {
            (span.range.start() <= range.start() && span.range.end() >= range.end())
                .then_some(&span.style)
        })
        .unwrap_or(&result.input.text_style)
}

fn text_paints_for_range<'a>(
    result: &'a LayoutResult,
    range: tiqian::core::geometry::TextRange,
    cursor: &mut usize,
) -> &'a [RichTextPaint] {
    while result
        .input
        .rich_text
        .get(*cursor)
        .is_some_and(|span| span.range.end() <= range.start())
    {
        *cursor += 1;
    }
    result
        .input
        .rich_text
        .get(*cursor)
        .filter(|span| span.range.start() <= range.start() && span.range.end() >= range.end())
        .and_then(|span| {
            span.layers
                .iter()
                .find(|layer| matches!(layer.kind, RichTextLayerKind::Text))
        })
        .map_or(&[], |layer| layer.paints.as_slice())
}

fn glyph_vertices_for_glyph(
    glyph: &Glyph,
    atlas_glyph: &AtlasGlyph,
    origin_x: f32,
    origin_y: f32,
    row: u32,
    col: u32,
    line_top: f32,
    line_bottom: f32,
    style: &TiqianTextStyle,
    paints: &[RichTextPaint],
    color_space: &ColorSpace,
    role: TextRole,
) -> TextVertices {
    let x_scale = atlas_glyph.metrics.x_scale.unwrap_or(1.0);
    let y_scale = atlas_glyph.metrics.y_scale.unwrap_or(1.0);
    let bitmap_width = atlas_glyph.metrics.width as f32 / x_scale;
    let bitmap_height = atlas_glyph.metrics.height as f32 / y_scale;
    let scale_ratio = style.font_size / FONT_SIZE;
    let quad_left = origin_x
        - (GRID_SIZE * atlas_glyph.grid_width as f32 / 2.0 / x_scale
            - bitmap_width / 2.0
            - atlas_glyph.metrics.x_min)
            * scale_ratio;
    let quad_top = origin_y
        - (GRID_SIZE * atlas_glyph.grid_height as f32 / 2.0 / y_scale - bitmap_height / 2.0
            + atlas_glyph.metrics.y_max)
            * scale_ratio;
    let quad_width = GRID_SIZE * atlas_glyph.grid_width as f32 * scale_ratio / x_scale;
    let quad_height = GRID_SIZE * atlas_glyph.grid_height as f32 * scale_ratio / y_scale;
    let fill_paint = paints.iter().find_map(|paint| match paint {
        RichTextPaint::Fill { argb } => Some(*argb),
        _ => None,
    });
    let stroke_paint = paints.iter().find_map(|paint| match paint {
        RichTextPaint::Stroke { argb, width } => Some((*argb, *width)),
        _ => None,
    });
    let shadow_paint = paints.iter().find_map(|paint| match paint {
        RichTextPaint::Shadow {
            argb,
            offset_x,
            offset_y,
            blur_radius,
            spread_radius,
        } => Some((*argb, *offset_x, *offset_y, *blur_radius, *spread_radius)),
        _ => None,
    });
    let fill_color = fill_paint
        .map(|argb| argb_color(argb, color_space))
        .unwrap_or_else(|| argb_color(DEFAULT_FILL_ARGB, color_space));
    let buffer = 1.0 - CUTOFF - FILL_THRESHOLD_BIAS;
    let fill_buffer = 2.0;
    let threshold_per_logical_pixel = x_scale / (RADIUS * scale_ratio);
    let edge_gamma = EDGE_SMOOTHING_HALF_WIDTH * threshold_per_logical_pixel;
    let synthetic_embolden = glyph
        .render_font_face
        .as_ref()
        .and_then(|face| face.synthesis().embolden_em())
        .unwrap_or(0.0)
        * style.font_size;
    let fill_threshold = buffer - synthetic_embolden * threshold_per_logical_pixel;
    let fill = quad_vertices(
        quad_left,
        quad_top,
        quad_width,
        quad_height,
        atlas_glyph,
        fill_threshold,
        fill_buffer,
        edge_gamma,
        fill_color,
    );
    let stroke = stroke_paint.map(|(argb, width)| {
        let stroke_buffer = fill_threshold - width * threshold_per_logical_pixel;
        quad_vertices(
            quad_left,
            quad_top,
            quad_width,
            quad_height,
            atlas_glyph,
            stroke_buffer,
            fill_threshold,
            edge_gamma,
            argb_color(argb, color_space),
        )
    });
    let shadow = shadow_paint.map(|(argb, offset_x, offset_y, blur, spread)| {
        let stroke_width = stroke_paint.map_or(0.0, |(_, width)| width);
        let shadow_buffer = fill_threshold - (stroke_width + spread) * threshold_per_logical_pixel;
        let shadow_gamma = edge_gamma + blur * threshold_per_logical_pixel;
        quad_vertices(
            quad_left + offset_x,
            quad_top + offset_y,
            quad_width,
            quad_height,
            atlas_glyph,
            shadow_buffer,
            fill_buffer,
            shadow_gamma,
            argb_color(argb, color_space),
        )
    });

    TextVertices {
        shadow,
        stroke,
        fill,
        col,
        row,
        x: origin_x.max(0.0).round() as u32,
        y: line_top.max(0.0).round() as u32,
        width: glyph.advance.max(0.0).round() as u32,
        height: (line_bottom - line_top).max(0.0).round() as u32,
        scale_ratio,
        role,
    }
}

fn quad_vertices(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    glyph: &AtlasGlyph,
    buffer: f32,
    fill_buffer: f32,
    gamma: f32,
    color: [f32; 4],
) -> [Vertex; 4] {
    [
        vertex(
            left,
            top,
            glyph.u_min,
            glyph.v_min,
            glyph.page,
            buffer,
            fill_buffer,
            gamma,
            color,
        ),
        vertex(
            left,
            top + height,
            glyph.u_min,
            glyph.v_max,
            glyph.page,
            buffer,
            fill_buffer,
            gamma,
            color,
        ),
        vertex(
            left + width,
            top + height,
            glyph.u_max,
            glyph.v_max,
            glyph.page,
            buffer,
            fill_buffer,
            gamma,
            color,
        ),
        vertex(
            left + width,
            top,
            glyph.u_max,
            glyph.v_min,
            glyph.page,
            buffer,
            fill_buffer,
            gamma,
            color,
        ),
    ]
}

fn vertex(
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    page: i32,
    buffer: f32,
    fill_buffer: f32,
    gamma: f32,
    color: [f32; 4],
) -> Vertex {
    Vertex {
        position: [x, y, 0.0],
        tex_coords: [u, v],
        page,
        buffer,
        fill_buffer,
        gamma,
        color,
    }
}

fn argb_color(argb: i32, color_space: &ColorSpace) -> [f32; 4] {
    let [alpha, red, green, blue] = (argb as u32).to_be_bytes();
    get_color_value(&Color::from_rgba8(red, green, blue, alpha), color_space)
}
