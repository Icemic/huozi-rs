use csscolorparser::Color;
use log::warn;
use std::collections::HashMap;
use tiqian::core::geometry::TextRange;
use tiqian::core::layout_model::LayoutResult;
use tiqian::core::layout_queries::positioned_clusters;
use tiqian::core::text_model::{RichTextLayerKind, RichTextPaint, TextStyle as TiqianTextStyle};

use crate::Huozi;
use crate::constant::{FONT_SIZE, GAMMA_COEFFICIENT, GRID_SIZE, VIEWPORT_HEIGHT, VIEWPORT_WIDTH};
use crate::glyph_vertices::GlyphVertices;
use crate::huozi::Glyph;
use crate::parser::SegmentId;

use super::color_space::{ColorSpace, get_color_value};
use super::tiqian_input::HuoziSourceMap;
use super::{SegmentGlyphSpan, Vertex};

pub(crate) struct HuoziTiqianOutputAdapter;

impl HuoziTiqianOutputAdapter {
    pub(crate) fn adapt(
        huozi: &mut Huozi,
        result: &LayoutResult,
        source_map: &HuoziSourceMap,
        color_space: &ColorSpace,
    ) -> (Vec<GlyphVertices>, Vec<SegmentGlyphSpan>, u32, u32) {
        warn_unsupported_annotation_geometry(result);

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
            .take_while(|line| line.bottom <= result.input.constraints.max_height())
            .count();
        let mut glyph_vertices = Vec::new();
        let mut segment_glyph_spans = Vec::new();
        let mut current_segment_id = None;
        let mut current_segment_start = 0;
        let mut columns_by_line = vec![0_u32; visible_line_count];
        let mut source_map_cursor = 0;
        let mut text_style_cursor = 0;
        let mut rich_text_cursor = 0;

        for (positioned_index, positioned) in positioned_clusters.iter().enumerate() {
            let line_index = positioned.line_index as usize;
            if line_index >= visible_line_count {
                continue;
            }
            let line_ends_after_cluster = positioned_clusters
                .get(positioned_index + 1)
                .is_none_or(|next| next.line_index != positioned.line_index);
            let cluster = &result.clusters[positioned.cluster_index as usize];
            if cluster.synthetic_kind.is_none()
                && let Some(glyphs) = glyphs_by_cluster_range.get(&positioned.range)
            {
                let segment_id =
                    source_segment_id(source_map, positioned.range, &mut source_map_cursor);
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
                    let col = columns_by_line[line_index];
                    columns_by_line[line_index] += 1;
                    let glyph_vertices_item = glyph_vertices_for_glyph(
                        glyph,
                        &atlas_glyph,
                        positioned.draw_x + glyph.x,
                        positioned.baseline + glyph.y,
                        positioned.line_index as u32,
                        col,
                        positioned.top,
                        positioned.bottom,
                        style,
                        paints,
                        color_space,
                    );

                    update_segment_glyph_spans(
                        &mut segment_glyph_spans,
                        &mut current_segment_id,
                        &mut current_segment_start,
                        segment_id.clone(),
                        glyph_vertices.len(),
                    );
                    glyph_vertices.push(glyph_vertices_item);
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
            let hyphen_segment_id =
                source_segment_id(source_map, hyphen_range, &mut source_map_cursor);
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
                let col = columns_by_line[line_index];
                columns_by_line[line_index] += 1;
                let glyph_vertices_item = glyph_vertices_for_glyph(
                    glyph,
                    &atlas_glyph,
                    line.indent + line.visual_width + glyph.x,
                    line.baseline + glyph.y,
                    line_index as u32,
                    col,
                    line.top,
                    line.bottom,
                    style,
                    paints,
                    color_space,
                );

                update_segment_glyph_spans(
                    &mut segment_glyph_spans,
                    &mut current_segment_id,
                    &mut current_segment_start,
                    hyphen_segment_id.clone(),
                    glyph_vertices.len(),
                );
                glyph_vertices.push(glyph_vertices_item);
            }
        }

        if let Some(segment_id) = current_segment_id {
            segment_glyph_spans.push(SegmentGlyphSpan {
                segment_id,
                glyph_range: current_segment_start..glyph_vertices.len(),
            });
        }

        let height = if visible_line_count == 0 {
            0
        } else {
            result.lines[visible_line_count - 1].bottom.max(0.0).round() as u32
        };
        (
            glyph_vertices,
            segment_glyph_spans,
            result.size.width.max(0.0).round() as u32,
            height,
        )
    }
}

fn warn_unsupported_annotation_geometry(result: &LayoutResult) {
    if !result.debug.ruby_decisions.is_empty()
        || !result.debug.bopomofo_decisions.is_empty()
        || !result.debug.decoration_segments.is_empty()
        || !result.debug.decoration_decisions.is_empty()
    {
        warn!(
            "HuoziTiqianOutputAdapter skips ruby, bopomofo, emphasis, underline, and line-through geometry"
        );
    }
}

fn source_segment_id(
    source_map: &HuoziSourceMap,
    range: tiqian::core::geometry::TextRange,
    cursor: &mut usize,
) -> Option<SegmentId> {
    while source_map
        .entries
        .get(*cursor)
        .is_some_and(|entry| entry.display_range.end() <= range.start())
    {
        *cursor += 1;
    }
    source_map
        .entries
        .get(*cursor)
        .and_then(|entry| {
            (entry.display_range.start() <= range.start()
                && entry.display_range.end() >= range.end())
            .then(|| entry.source_range.segment_id.clone())
        })
        .flatten()
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
    glyph: &tiqian::core::layout_model::Glyph,
    atlas_glyph: &Glyph,
    origin_x: f32,
    origin_y: f32,
    row: u32,
    col: u32,
    line_top: f32,
    line_bottom: f32,
    style: &TiqianTextStyle,
    paints: &[RichTextPaint],
    color_space: &ColorSpace,
) -> GlyphVertices {
    let x_scale = atlas_glyph.metrics.x_scale.unwrap_or(1.0);
    let y_scale = atlas_glyph.metrics.y_scale.unwrap_or(1.0);
    let bitmap_width = atlas_glyph.metrics.width as f32 / x_scale;
    let bitmap_height = atlas_glyph.metrics.height as f32 / y_scale;
    let scale_ratio = style.font_size / FONT_SIZE as f32;
    let quad_left = origin_x
        - (GRID_SIZE as f32 * atlas_glyph.grid_width as f32 / 2.0 / x_scale
            - bitmap_width / 2.0
            - atlas_glyph.metrics.x_min)
            * scale_ratio;
    let quad_top = origin_y
        - (GRID_SIZE as f32 * atlas_glyph.grid_height as f32 / 2.0 / y_scale - bitmap_height / 2.0
            + atlas_glyph.metrics.y_max)
            * scale_ratio;
    let quad_width = GRID_SIZE as f32 * atlas_glyph.grid_width as f32 * scale_ratio / x_scale;
    let quad_height = GRID_SIZE as f32 * atlas_glyph.grid_height as f32 * scale_ratio / y_scale;
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
        .unwrap_or_else(|| argb_color(0xFF1E_1E23_u32 as i32, color_space));
    let buffer = match color_space {
        ColorSpace::Linear => 0.5,
        ColorSpace::SRGB => 0.735357,
    };
    let fill_buffer = 2.0;
    let gamma = GAMMA_COEFFICIENT * 0.6 / 2.0 / scale_ratio;
    let fill = quad_vertices(
        quad_left,
        quad_top,
        quad_width,
        quad_height,
        atlas_glyph,
        buffer,
        fill_buffer,
        gamma,
        fill_color,
    );
    let stroke = stroke_paint.map(|(argb, width)| {
        let base_buffer = match color_space {
            ColorSpace::Linear => 0.448,
            ColorSpace::SRGB => 0.7,
        };
        let stroke_buffer =
            (base_buffer - GAMMA_COEFFICIENT * width / 2.0 / scale_ratio * x_scale).max(gamma);
        quad_vertices(
            quad_left,
            quad_top,
            quad_width,
            quad_height,
            atlas_glyph,
            stroke_buffer,
            buffer,
            gamma,
            argb_color(argb, color_space),
        )
    });
    let shadow = shadow_paint.map(|(argb, offset_x, offset_y, blur, spread)| {
        let base_buffer = match color_space {
            ColorSpace::Linear => 0.448,
            ColorSpace::SRGB => 0.7,
        };
        let shadow_gamma = GAMMA_COEFFICIENT * blur / 2.0 / (scale_ratio * 2.0) * x_scale;
        let shadow_buffer = (base_buffer
            - GAMMA_COEFFICIENT * spread / 2.0 / scale_ratio * x_scale)
            .max(shadow_gamma);
        let shadow_fill_buffer = if fill_color[3] > 0.0 {
            fill_buffer
        } else {
            buffer
        };
        quad_vertices(
            quad_left + offset_x / VIEWPORT_WIDTH as f32 * 2.0,
            quad_top + offset_y / VIEWPORT_HEIGHT as f32 * 2.0,
            quad_width,
            quad_height,
            atlas_glyph,
            shadow_buffer,
            shadow_fill_buffer,
            shadow_gamma,
            argb_color(argb, color_space),
        )
    });

    GlyphVertices {
        shadow,
        stroke,
        fill,
        indices: [0, 1, 2, 0, 2, 3],
        col,
        row,
        x: origin_x.max(0.0).round() as u32,
        y: line_top.max(0.0).round() as u32,
        width: glyph.advance.max(0.0).round() as u32,
        height: (line_bottom - line_top).max(0.0).round() as u32,
        scale_ratio,
    }
}

fn quad_vertices(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    glyph: &Glyph,
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

fn update_segment_glyph_spans(
    segment_glyph_spans: &mut Vec<SegmentGlyphSpan>,
    current_segment_id: &mut Option<SegmentId>,
    current_segment_start: &mut usize,
    next_segment_id: Option<SegmentId>,
    glyph_index: usize,
) {
    if next_segment_id == *current_segment_id {
        return;
    }
    if let Some(segment_id) = current_segment_id.take() {
        segment_glyph_spans.push(SegmentGlyphSpan {
            segment_id,
            glyph_range: *current_segment_start..glyph_index,
        });
    }
    *current_segment_id = next_segment_id;
    *current_segment_start = glyph_index;
}
