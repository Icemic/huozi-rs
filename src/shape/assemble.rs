//! 把布局收集到的图形片段定形为顶点。
//!
//! 同一个 authored range 的片段先在整段上定形，再按各自的排版单元切成段：相邻片段在上一段的
//! 右边界处切开，接缝两侧正好衔接，既不重叠也不留缝；整段的首尾两端不设边界，因此圆角、描边、
//! 阴影与端帽都不会被切掉。每段自带顶点，拼起来就是完整图形，逐字推进时图形跟着长。
//!
//! 着重号按字符独立出现，每个点自身就是完整图形，不参与合并。

use std::collections::HashMap;

use tiqian::core::geometry::Rect;
use tiqian::core::layout_queries::RichTextCornerRadii;
use tiqian::core::text_model::{DecorationKind, RichTextLinePattern};

use crate::Huozi;
use crate::glyph_vertices::{DecorationShape, DecorationWave, ShapeVertices};

use super::MAX_BACKGROUND_RADIUS;
use super::paint::ShapePaint;
use super::vertices::{LineEnds, ShapeBatch, ShapeLevel};
use super::{ShapeKey, ShapeTemplate};

/// 片段在整段图形中的归属。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FragmentGroup {
    /// 同一个 authored range 的片段合并成一条完整图形。
    Range(u32),
    /// 该片段自身就是一个完整图形，不与任何片段合并。
    Solo,
}

/// 分组键：归属加视觉行；跨行时每行独立成形。
pub(crate) type GroupKey = (FragmentGroup, u32);

/// 分组键；`Solo` 片段用下标占住行的位置，保证互不合并。
fn group_keys(groups: &[FragmentGroup], rows: &[u32]) -> Vec<GroupKey> {
    groups
        .iter()
        .enumerate()
        .map(|(index, group)| match group {
            FragmentGroup::Range(_) => (*group, rows[index]),
            FragmentGroup::Solo => (*group, index as u32),
        })
        .collect()
}

/// 各片段在整段图形中的横向裁剪窗口。
///
/// 相邻片段在上一段的右边界处衔接，因此接缝两侧既不重叠也不留缝；整段的首尾不设边界，图形的
/// 圆角、描边、阴影与端帽因此不会被切掉。
pub(crate) fn fragment_windows(keys: &[GroupKey], rights: &[f32]) -> Vec<(f32, f32)> {
    let mut totals: HashMap<GroupKey, usize> = HashMap::new();
    for key in keys {
        *totals.entry(*key).or_default() += 1;
    }
    let mut seen: HashMap<GroupKey, usize> = HashMap::new();
    let mut previous_right: HashMap<GroupKey, f32> = HashMap::new();
    let mut windows = Vec::with_capacity(keys.len());
    for (index, key) in keys.iter().enumerate() {
        let position = seen.entry(*key).or_default();
        let left = if *position == 0 {
            f32::NEG_INFINITY
        } else {
            previous_right[key]
        };
        *position += 1;
        let right = if *position == totals[key] {
            f32::INFINITY
        } else {
            rights[index]
        };
        previous_right.insert(*key, rights[index]);
        windows.push((left, right));
    }
    windows
}

/// 按分组键求各组的整段几何。
fn group_bounds(keys: &[GroupKey], geometry: &[[f32; 4]]) -> HashMap<GroupKey, [f32; 4]> {
    let mut bounds: HashMap<GroupKey, [f32; 4]> = HashMap::new();
    for (key, geometry) in keys.iter().zip(geometry) {
        bounds
            .entry(*key)
            .and_modify(|current| {
                current[0] = current[0].min(geometry[0]);
                current[1] = current[1].min(geometry[1]);
                current[2] = current[2].max(geometry[2]);
                current[3] = current[3].max(geometry[3]);
            })
            .or_insert(*geometry);
    }
    bounds
}

/// 待定形的一段背景。
pub(crate) struct BackgroundFragment {
    pub(crate) group: FragmentGroup,
    pub(crate) row: u32,
    pub(crate) col: u32,
    pub(crate) rect: Rect,
    pub(crate) corner_radii: RichTextCornerRadii,
    pub(crate) paint: ShapePaint,
}

/// 待定形的一段下划线或删除线。
pub(crate) struct LineFragment {
    pub(crate) group: FragmentGroup,
    pub(crate) row: u32,
    pub(crate) col: u32,
    pub(crate) left: f32,
    pub(crate) right: f32,
    pub(crate) line_y: f32,
    pub(crate) thickness: f32,
    pub(crate) pattern: RichTextLinePattern,
    pub(crate) paint: ShapePaint,
}

/// 待定形的一段 CLREQ 装饰。
pub(crate) struct DecorationFragment {
    pub(crate) group: FragmentGroup,
    pub(crate) row: u32,
    pub(crate) col: u32,
    pub(crate) kind: DecorationKind,
    pub(crate) shape: DecorationShape,
    pub(crate) thickness: f32,
    pub(crate) wave: Option<DecorationWave>,
    pub(crate) paint: ShapePaint,
}

/// 按连续范围把背景定形成各段自带顶点的图形。
pub(crate) fn shape_backgrounds(
    huozi: &mut Huozi,
    fragments: &[BackgroundFragment],
) -> Vec<ShapeVertices> {
    let groups = fragments.iter().map(|f| f.group).collect::<Vec<_>>();
    let rows = fragments.iter().map(|f| f.row).collect::<Vec<_>>();
    let keys = group_keys(&groups, &rows);
    let geometry = fragments
        .iter()
        .map(|f| [f.rect.left, f.rect.top, f.rect.right, f.rect.bottom])
        .collect::<Vec<_>>();
    let rights = fragments.iter().map(|f| f.rect.right).collect::<Vec<_>>();
    let bounds = group_bounds(&keys, &geometry);
    let windows = fragment_windows(&keys, &rights);

    let mut result = Vec::with_capacity(fragments.len());
    for (index, fragment) in fragments.iter().enumerate() {
        let target = bounds[&keys[index]];
        let radii = fragment.corner_radii;
        let corner = radii
            .top_left
            .max(radii.top_right)
            .max(radii.bottom_right)
            .max(radii.bottom_left)
            .clamp(0.0, MAX_BACKGROUND_RADIUS as f32)
            .round();
        let template = huozi.shape_template(ShapeKey::Rect {
            radius: corner as u32,
        });
        let mut levels = Levels::new(windows[index]);
        levels.each(&fragment.paint, |batch, level, offset| {
            batch.nine_slice(
                template,
                corner,
                [
                    target[0] + offset[0],
                    target[1] + offset[1],
                    target[2] + offset[0],
                    target[3] + offset[1],
                ],
                level,
            );
        });
        result.push(levels.finish());
    }
    result
}

/// 按连续范围把下划线与删除线定形成各段自带顶点的图形。
pub(crate) fn shape_lines(huozi: &mut Huozi, fragments: &[LineFragment]) -> Vec<ShapeVertices> {
    let groups = fragments.iter().map(|f| f.group).collect::<Vec<_>>();
    let rows = fragments.iter().map(|f| f.row).collect::<Vec<_>>();
    let keys = group_keys(&groups, &rows);
    let geometry = fragments
        .iter()
        .map(|f| [f.left, 0.0, f.right, 0.0])
        .collect::<Vec<_>>();
    let rights = fragments.iter().map(|f| f.right).collect::<Vec<_>>();
    let bounds = group_bounds(&keys, &geometry);
    let windows = fragment_windows(&keys, &rights);

    let mut result = Vec::with_capacity(fragments.len());
    for (index, fragment) in fragments.iter().enumerate() {
        let [left, _, right, _] = bounds[&keys[index]];
        let band = huozi.shape_template(ShapeKey::Band);
        let dot = matches!(fragment.pattern, RichTextLinePattern::Dotted { .. })
            .then(|| huozi.shape_template(ShapeKey::Dot));
        let mut levels = Levels::new(windows[index]);
        levels.each(&fragment.paint, |batch, level, _| {
            draw_line_pattern(
                batch,
                band,
                dot,
                &fragment.pattern,
                left,
                right,
                fragment.line_y,
                fragment.thickness,
                level,
            );
        });
        result.push(levels.finish());
    }
    result
}

/// 按连续范围把 CLREQ 装饰定形成各段自带顶点的图形。
pub(crate) fn shape_decorations(
    huozi: &mut Huozi,
    fragments: &[DecorationFragment],
) -> Vec<ShapeVertices> {
    let groups = fragments.iter().map(|f| f.group).collect::<Vec<_>>();
    let rows = fragments.iter().map(|f| f.row).collect::<Vec<_>>();
    let keys = group_keys(&groups, &rows);
    let geometry = fragments
        .iter()
        .map(|f| shape_bounds(&f.shape))
        .collect::<Vec<_>>();
    let rights = geometry.iter().map(|bounds| bounds[2]).collect::<Vec<_>>();
    let bounds = group_bounds(&keys, &geometry);
    let windows = fragment_windows(&keys, &rights);

    let mut result = Vec::with_capacity(fragments.len());
    for (index, fragment) in fragments.iter().enumerate() {
        let target = bounds[&keys[index]];
        let thickness = fragment.thickness.max(1.0);
        let mut levels = Levels::new(windows[index]);
        match fragment.shape {
            // 着重号按字符独立出现，每个点自身就是一个完整图形。
            DecorationShape::Dot {
                center_x,
                center_y,
                diameter,
            } => {
                let template = huozi.shape_template(ShapeKey::Dot);
                levels.each(&fragment.paint, |batch, level, _| {
                    batch.dot(template, center_x, center_y, diameter, level);
                });
            }
            DecorationShape::Frame {
                open_start,
                open_end,
                ..
            } => match fragment.kind {
                DecorationKind::BookTitle => {
                    if let Some(wave) = fragment.wave {
                        let band = huozi.shape_template(ShapeKey::Band);
                        let template = huozi.shape_template(ShapeKey::Wave);
                        levels.each(&fragment.paint, |batch, level, _| {
                            batch.wave(
                                template,
                                band,
                                target[0],
                                target[2],
                                target[1],
                                wave.wavelength,
                                level,
                            );
                        });
                    }
                }
                DecorationKind::ProperNoun => {
                    let band = huozi.shape_template(ShapeKey::Band);
                    levels.each(&fragment.paint, |batch, level, _| {
                        batch.horizontal_line(
                            band,
                            target[0],
                            target[2],
                            target[1],
                            thickness,
                            LineEnds::Closed,
                            level,
                        );
                    });
                }
                // 示亡号是一个边框：用九宫格一次成形，四角是模板的 L 形角块。若用四条线段拼接，
                // 四角会因两条边各自向外扩而重叠，或多出可见的小段。
                _ => {
                    let template = huozi.shape_template(ShapeKey::Frame {
                        open_start,
                        open_end,
                    });
                    levels.each(&fragment.paint, |batch, level, _| {
                        batch.frame(template, target, thickness, level);
                    });
                }
            },
        }
        result.push(levels.finish());
    }
    result
}

/// 一个图形片段的三层顶点累积器。
struct Levels {
    shadow: ShapeBatch,
    stroke: ShapeBatch,
    fill: ShapeBatch,
}

impl Levels {
    fn new(window: (f32, f32)) -> Self {
        Self {
            shadow: ShapeBatch::with_window(window),
            stroke: ShapeBatch::with_window(window),
            fill: ShapeBatch::with_window(window),
        }
    }

    /// 依次交出阴影、描边、填充三层。
    ///
    /// 第三项是几何相对图形边界的平移量，只有阴影会用到它。没有的层直接跳过，因此调用方只需要
    /// 写一遍几何。
    fn each(
        &mut self,
        paint: &ShapePaint,
        mut emit: impl FnMut(&mut ShapeBatch, ShapeLevel, [f32; 2]),
    ) {
        let stroke_width = paint.stroke.map_or(0.0, |stroke| stroke.width);
        if let Some(shadow) = paint.shadow {
            emit(
                &mut self.shadow,
                ShapeLevel::shadow(
                    shadow.color,
                    shadow.blur_radius,
                    stroke_width,
                    shadow.spread_radius,
                ),
                [shadow.offset_x, shadow.offset_y],
            );
        }
        if let Some(stroke) = paint.stroke {
            emit(
                &mut self.stroke,
                ShapeLevel::stroke(stroke.color, stroke.width),
                [0.0, 0.0],
            );
        }
        emit(&mut self.fill, ShapeLevel::fill(paint.fill), [0.0, 0.0]);
    }

    fn finish(mut self) -> ShapeVertices {
        ShapeVertices {
            shadow: self.shadow.take(),
            stroke: self.stroke.take(),
            fill: self.fill.take(),
        }
    }
}

/// 装饰片段的外接矩形。
fn shape_bounds(shape: &DecorationShape) -> [f32; 4] {
    match *shape {
        DecorationShape::Frame {
            left,
            top,
            right,
            bottom,
            ..
        } => [left, top, right, bottom],
        DecorationShape::Dot {
            center_x,
            center_y,
            diameter,
        } => {
            let half = diameter / 2.0;
            [
                center_x - half,
                center_y - half,
                center_x + half,
                center_y + half,
            ]
        }
    }
}

/// 画一段线型图案。
#[allow(clippy::too_many_arguments)]
fn draw_line_pattern(
    batch: &mut ShapeBatch,
    band: ShapeTemplate,
    dot: Option<ShapeTemplate>,
    pattern: &RichTextLinePattern,
    left: f32,
    right: f32,
    line_y: f32,
    thickness: f32,
    level: ShapeLevel,
) {
    match pattern {
        RichTextLinePattern::Solid => {
            batch.horizontal_line(
                band,
                left,
                right,
                line_y,
                thickness,
                LineEnds::Closed,
                level,
            );
        }
        RichTextLinePattern::Dashed {
            dash_length,
            gap_length,
        } => {
            let step = dash_length + gap_length;
            if step <= 0.0 {
                return;
            }
            let mut x = left;
            while x < right {
                let dash_end = (x + dash_length).min(right);
                // 每个 dash 是同一个 authored range 的碎片，两端按固定节距裁断，不加端帽。
                batch.horizontal_line(
                    band,
                    x,
                    dash_end,
                    line_y,
                    thickness,
                    LineEnds::Clipped,
                    level,
                );
                x += step;
            }
        }
        RichTextLinePattern::Dotted { gap_length } => {
            let Some(dot) = dot else {
                return;
            };
            let step = thickness + gap_length;
            if step <= 0.0 {
                return;
            }
            let mut center = left + thickness / 2.0;
            while center + thickness / 2.0 <= right {
                batch.dot(dot, center, line_y, thickness, level);
                center += step;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 分段的边界必须落在接缝上，整段的首尾不设边界。
    ///
    /// 窗口若取片段自己的矩形，图形的圆角、描边、阴影与端帽都会被切掉。
    #[test]
    fn fragment_windows_cut_at_seams_only() {
        let keys = [
            (FragmentGroup::Range(7), 0),
            (FragmentGroup::Range(7), 0),
            (FragmentGroup::Range(7), 0),
        ];
        let rights = [10.0, 20.0, 30.0];
        let windows = fragment_windows(&keys, &rights);

        assert_eq!(windows[0], (f32::NEG_INFINITY, 10.0));
        assert_eq!(windows[1], (10.0, 20.0));
        assert_eq!(windows[2], (20.0, f32::INFINITY));
    }

    /// 跨行时每行独立成形，不会把两行接成一条。
    #[test]
    fn fragment_windows_split_by_row() {
        let keys = [(FragmentGroup::Range(7), 0), (FragmentGroup::Range(7), 1)];
        let rights = [10.0, 20.0];
        let windows = fragment_windows(&keys, &rights);

        assert_eq!(windows[0], (f32::NEG_INFINITY, f32::INFINITY));
        assert_eq!(windows[1], (f32::NEG_INFINITY, f32::INFINITY));
    }
}
