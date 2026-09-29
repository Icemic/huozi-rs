//! 从形状模板拼出最终顶点。
//!
//! 每种图形都从一个常驻模板取形状，再按目标尺寸拼成四边形：圆角矩形与示亡号用九宫格、线段用三
//! 宫格、圆点整体等比缩放、波浪周期重复并在末端裁剪。阈值换算把"目标像素"表达成顶点携带的归一化
//! SDF 值，因此调用方只需要说"描边外扩多少像素"。

use crate::constant::{CUTOFF, FILL_THRESHOLD_BIAS, RADIUS};
use crate::layout::Vertex;

use super::ShapeTemplate;
use super::atlas::{
    BAND_TEMPLATE_THICKNESS, DOT_TEMPLATE_DIAMETER, FRAME_TEMPLATE_NOTCH, FRAME_TEMPLATE_THICKNESS,
    SDF_SCALE, WAVE_TEMPLATE_AMPLITUDE, WAVE_TEMPLATE_PERIOD, WAVE_TEMPLATE_THICKNESS,
    frame_corner, shape_margin, shape_u, texel_uv,
};

/// 端帽沿线段方向的总长度（目标像素），中心落在端面上。
///
/// 抗锯齿过渡带约为一个目标像素，因此形状必须在端面两侧各深半个过渡带，端面处的覆盖率才与线段
/// 主体一致（过渡带内的小形状会被冲淡）。中心落在端面上还有一个好处：端面外侧自然形成抗锯齿
/// 渐隐，不需要额外几何。
const LINE_CAP_LENGTH: f32 = 2.4;

/// 线段两端的处理方式。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineEnds {
    /// 两端是真实端点：几何向外扩一圈并用 1:1 端帽把描边与阴影收口。
    Closed,
    /// 两端是图案片段的裁剪边界：几何正好覆盖 `left..right`，不向外扩，也不加端帽。
    Clipped,
}

/// 一层 paint 的 SDF 参数与颜色。
///
/// 距离参数以目标像素为单位：只表达"外缘、内缘相对图形边界外扩多少像素"，与图形最终的缩放比无
/// 关。归一化阈值由 [`Self::thresholds`] 按"目标像素每 texel"换算。
#[derive(Clone, Copy)]
pub(crate) struct ShapeLevel {
    /// 外缘相对图形边界的外扩量；负值表示向图形内部收缩。
    outer: f32,
    /// 内缘相对图形边界的外扩量；`None` 表示不扣除内缘。
    inner: Option<f32>,
    /// 额外边缘平滑半宽。
    blur: f32,
    color: [f32; 4],
}

impl ShapeLevel {
    /// 填充：外缘就是图形边界，不扣除内缘。
    pub(crate) fn fill(color: [f32; 4]) -> Self {
        Self {
            outer: 0.0,
            inner: None,
            blur: 0.0,
            color,
        }
    }

    /// 描边：外缘外扩 `width`，内缘为图形边界。
    pub(crate) fn stroke(color: [f32; 4], width: f32) -> Self {
        Self {
            outer: -width,
            inner: Some(0.0),
            blur: 0.0,
            color,
        }
    }

    /// 阴影：外缘在外扩 `stroke_width + spread` 处，过渡半宽额外含 `blur`。
    pub(crate) fn shadow(color: [f32; 4], blur: f32, stroke_width: f32, spread: f32) -> Self {
        Self {
            outer: -(stroke_width + spread),
            inner: None,
            blur,
            color,
        }
    }

    /// 把目标像素参数换算成顶点携带的归一化 SDF 阈值。
    ///
    /// `scale` 是一个模板 texel 对应多少个目标像素；`RADIUS` 是 SDF 在模板 texel 尺度上的扩散
    /// 半径，因此一个归一化距离单位对应 `RADIUS` 个模板 texel。
    ///
    /// 顶点 gamma 只表达阴影的额外模糊；图形自身的边缘抗锯齿完全交给 shader 的逐像素 SDF 梯度。
    /// 模板是解析距离场且已过采样，梯度项给出的半宽（约一个目标像素）已经足够；再叠加固定半宽会把
    /// 斜坡推过形状端点，在 1～2px 宽的线条两端留下可见短段。
    fn thresholds(self, scale: f32) -> ShapeThresholds {
        let per_pixel = 1.0 / (RADIUS * scale);
        let base = fill_threshold();
        ShapeThresholds {
            buffer: base + self.outer * per_pixel,
            fill_buffer: self.inner.map_or(2.0, |inner| base + inner * per_pixel),
            gamma: self.blur * per_pixel,
        }
    }
}

/// 与文字路径一致的填充阈值。
fn fill_threshold() -> f32 {
    1.0 - CUTOFF - FILL_THRESHOLD_BIAS
}

/// 写入顶点的一组归一化 SDF 阈值。
#[derive(Clone, Copy)]
struct ShapeThresholds {
    buffer: f32,
    fill_buffer: f32,
    gamma: f32,
}

/// 一个绘制层的顶点累积器。
///
/// 只保存顶点：每 4 个连续顶点构成一个四边形，顺序为左上、左下、右下、右上。
///
/// 可以带一个横向窗口：图形只在落进窗口时写入，写入时按窗口裁掉两侧并线性插值 UV。同一个图形
/// 切成多段后拼起来仍是完整图形，因此逐字显示时每一段可以独立成为一个绘制元素。
#[derive(Default)]
pub(crate) struct ShapeBatch {
    pub(crate) vertices: Vec<Vertex>,
    /// 横向窗口 `(left, right)`，`None` 表示不裁剪。
    window: Option<(f32, f32)>,
}

impl ShapeBatch {
    /// 建一个带横向窗口的累积器。
    pub(crate) fn with_window(window: (f32, f32)) -> Self {
        Self {
            window: Some(window),
            ..Self::default()
        }
    }

    /// 把这个累积器的顶点取走。
    pub(crate) fn take(&mut self) -> Vec<Vertex> {
        std::mem::take(&mut self.vertices)
    }

    /// 用圆角矩形模板画一个九宫格矩形：四角保持比例，边与中心只沿各自方向拉伸。
    ///
    /// `target` 是图形的形状边界；模板位图在形状之外还有一圈形状边距（描边与阴影需要占用它），
    /// 因此实际几何从 `target` 向外扩一圈，四角分块固定为「圆角半径 + 边距」。在逻辑像素尺度上，
    /// 角块与其 UV 都是 1:1 的，圆弧既不会被拉伸，也不会被采样成折线。
    pub(crate) fn nine_slice(
        &mut self,
        template: ShapeTemplate,
        corner: f32,
        target: [f32; 4],
        level: ShapeLevel,
    ) {
        let [left, top, right, bottom] = target;
        let width = right - left;
        let height = bottom - top;
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let margin = shape_margin();
        let [texel_u, texel_v] = texel_uv(template);
        // 角块边长固定为「圆角半径 + 边距」：超出部分只在中块拉伸，圆角本身不会变形。
        let corner_x = corner.clamp(0.0, width / 2.0) + margin;
        let corner_y = corner.clamp(0.0, height / 2.0) + margin;
        let outer_left = left - margin;
        let outer_top = top - margin;
        let outer_right = right + margin;
        let outer_bottom = bottom + margin;
        let first_right = outer_left + corner_x;
        let second_right = (outer_right - corner_x).max(first_right);
        let first_bottom = outer_top + corner_y;
        let second_bottom = (outer_bottom - corner_y).max(first_bottom);
        let u_first = template.u_min + corner_x * SDF_SCALE * texel_u;
        let u_second = (template.u_max - corner_x * SDF_SCALE * texel_u).max(u_first);
        let v_first = template.v_min + corner_y * SDF_SCALE * texel_v;
        let v_second = (template.v_max - corner_y * SDF_SCALE * texel_v).max(v_first);

        // (目标起点, 目标终点, UV 起点, UV 终点)；九块共享相同的边界坐标，不会出现可见缝隙。
        let columns = [
            (outer_left, first_right, template.u_min, u_first),
            (first_right, second_right, u_first, u_second),
            (second_right, outer_right, u_second, template.u_max),
        ];
        let rows = [
            (outer_top, first_bottom, template.v_min, v_first),
            (first_bottom, second_bottom, v_first, v_second),
            (second_bottom, outer_bottom, v_second, template.v_max),
        ];
        for (row_top, row_bottom, v_min, v_max) in rows {
            for (column_left, column_right, u_min, u_max) in columns {
                self.quad(
                    [column_left, row_top, column_right, row_bottom],
                    [u_min, v_min, u_max, v_max],
                    template.page,
                    level,
                    1.0 / SDF_SCALE,
                );
            }
        }
    }

    /// 用边框模板画一个矩形框（示亡号）。
    ///
    /// 九宫格切分：四角是 1:1 的 L 形角块，上中与下中只沿横向拉伸，左中与右中只沿纵向拉伸，中心不
    /// 绘制。四角因此由模板一次成形，不会出现两条边各自外扩而在四角重叠或各缺一块。
    ///
    /// 角块的深度是「线宽 + 空腔侧深度」：描边从形状边界向外扩，在空腔那一侧就是向空腔内扩，这段
    /// 区域必须由 1:1 的角块提供；中块只沿一个方向拉伸，覆盖不到它。
    ///
    /// `target` 是边框的形状边界，线宽为 `thickness`；模板形状盒按线宽等比缩放，因此线宽相对空腔
    /// 的比例保持不变。
    pub(crate) fn frame(
        &mut self,
        template: ShapeTemplate,
        target: [f32; 4],
        thickness: f32,
        level: ShapeLevel,
    ) {
        let [left, top, right, bottom] = target;
        let width = right - left;
        let height = bottom - top;
        if width <= 0.0 || height <= 0.0 || thickness <= 0.0 {
            return;
        }
        let scale = thickness / FRAME_TEMPLATE_THICKNESS;
        let margin = shape_margin() * scale;
        let [texel_u, texel_v] = texel_uv(template);
        let uv_corner_u = (shape_margin() + FRAME_TEMPLATE_THICKNESS + FRAME_TEMPLATE_NOTCH)
            * SDF_SCALE
            * texel_u;
        let uv_corner_v = (shape_margin() + FRAME_TEMPLATE_THICKNESS + FRAME_TEMPLATE_NOTCH)
            * SDF_SCALE
            * texel_v;
        let corner_shape = frame_corner(thickness);
        let corner_x = corner_shape.min(width / 2.0) + margin;
        let corner_y = corner_shape.min(height / 2.0) + margin;
        let outer_left = left - margin;
        let outer_top = top - margin;
        let outer_right = right + margin;
        let outer_bottom = bottom + margin;
        let first_right = outer_left + corner_x;
        let second_right = (outer_right - corner_x).max(first_right);
        let first_bottom = outer_top + corner_y;
        let second_bottom = (outer_bottom - corner_y).max(first_bottom);
        let u_first = template.u_min + uv_corner_u;
        let u_second = (template.u_max - uv_corner_u).max(u_first);
        let v_first = template.v_min + uv_corner_v;
        let v_second = (template.v_max - uv_corner_v).max(v_first);

        let columns = [
            (outer_left, first_right, template.u_min, u_first),
            (first_right, second_right, u_first, u_second),
            (second_right, outer_right, u_second, template.u_max),
        ];
        let rows = [
            (outer_top, first_bottom, template.v_min, v_first),
            (first_bottom, second_bottom, v_first, v_second),
            (second_bottom, outer_bottom, v_second, template.v_max),
        ];
        for (row_index, (row_top, row_bottom, v_min, v_max)) in rows.into_iter().enumerate() {
            for (column_index, (column_left, column_right, u_min, u_max)) in
                columns.iter().copied().enumerate()
            {
                // 中心块是框的空腔，不绘制。
                if row_index == 1 && column_index == 1 {
                    continue;
                }
                self.quad(
                    [column_left, row_top, column_right, row_bottom],
                    [u_min, v_min, u_max, v_max],
                    template.page,
                    level,
                    scale / SDF_SCALE,
                );
            }
        }
    }

    /// 用线带模板画一条水平线段。
    ///
    /// [`LineEnds::Closed`] 采用三宫格：两端是 1:1 的端帽，中段沿长度方向拉伸。端帽分别覆盖形状
    /// 外侧的形状边距（描边外扩与阴影需要占用它）与形状内侧半个线宽的端面距离场，因此端面闭合；
    /// 若目标横向只覆盖形状边界，描边环在两端就是敞口的。
    ///
    /// [`LineEnds::Clipped`] 把 `left..right` 直接映射到模板形状区，两端是被裁掉的，用于虚线、点线
    /// 这类图案片段：它们是同一个 authored range 的碎片而不是独立形状，逐个加端帽会让相邻片段的
    /// 描边在间隙里相接。
    pub(crate) fn horizontal_line(
        &mut self,
        template: ShapeTemplate,
        left: f32,
        right: f32,
        center_y: f32,
        thickness: f32,
        ends: LineEnds,
        level: ShapeLevel,
    ) {
        if right <= left || thickness <= 0.0 {
            return;
        }
        let scale = thickness / BAND_TEMPLATE_THICKNESS;
        let half_height = template.height as f32 / SDF_SCALE / 2.0 * scale;
        let top = center_y - half_height;
        let bottom = center_y + half_height;
        let [shape_u_min, shape_u_max] = shape_u(template);
        if ends == LineEnds::Clipped {
            self.quad(
                [left, top, right, bottom],
                [shape_u_min, template.v_min, shape_u_max, template.v_max],
                template.page,
                level,
                scale / SDF_SCALE,
            );
            return;
        }

        let [texel_u, _] = texel_uv(template);
        let inward = (thickness / 2.0).min((right - left) / 2.0);
        let outward = shape_margin();
        let cap = outward + inward;
        let cap_u = (cap * SDF_SCALE * texel_u).min((template.u_max - template.u_min) / 2.0);
        let outer_left = left - outward;
        let outer_right = right + outward;

        let columns = [
            (
                outer_left,
                outer_left + cap,
                template.u_min,
                template.u_min + cap_u,
            ),
            (
                outer_left + cap,
                outer_right - cap,
                template.u_min + cap_u,
                template.u_max - cap_u,
            ),
            (
                outer_right - cap,
                outer_right,
                template.u_max - cap_u,
                template.u_max,
            ),
        ];
        for (column_left, column_right, u_min, u_max) in columns {
            self.quad(
                [column_left, top, column_right, bottom],
                [u_min, template.v_min, u_max, template.v_max],
                template.page,
                level,
                scale / SDF_SCALE,
            );
        }
    }

    /// 用圆点模板画一个圆点。
    ///
    /// 模板位图整体按目标直径等比缩放，因此圆点边界、描边外扩与阴影距离都与目标像素一致。
    pub(crate) fn dot(
        &mut self,
        template: ShapeTemplate,
        center_x: f32,
        center_y: f32,
        diameter: f32,
        level: ShapeLevel,
    ) {
        if diameter <= 0.0 {
            return;
        }
        let scale = diameter / DOT_TEMPLATE_DIAMETER;
        let half_width = template.width as f32 / SDF_SCALE / 2.0 * scale;
        let half_height = template.height as f32 / SDF_SCALE / 2.0 * scale;
        self.quad(
            [
                center_x - half_width,
                center_y - half_height,
                center_x + half_width,
                center_y + half_height,
            ],
            [
                template.u_min,
                template.v_min,
                template.u_max,
                template.v_max,
            ],
            template.page,
            level,
            scale / SDF_SCALE,
        );
    }

    /// 在线段端点处补一个与线段方向对齐的端帽。
    ///
    /// `angle` 是从端点指向线段内侧的方向（弧度）。形状的中心落在 `(end_x, end_y)` 上，长
    /// [`LINE_CAP_LENGTH`]、宽 `thickness`，长度方向 1:1 映射、厚度方向按线宽等比缩放，因此端帽的
    /// 粗细与线段一致，端面垂直于端点处的切线。
    ///
    /// 周期模板（波浪线）两侧的距离场表示曲线继续延伸，端点处是被硬切开的；这个端帽把端点补成
    /// 垂直于切线的端面。形状必须跨越端面两侧：只向内侧延伸的短端帽在过渡带里会被冲淡，端点比
    /// 线条主体更淡，看起来像断开。
    pub(crate) fn line_cap(
        &mut self,
        template: ShapeTemplate,
        end_x: f32,
        end_y: f32,
        angle: f32,
        thickness: f32,
        level: ShapeLevel,
    ) {
        if thickness <= 0.0 {
            return;
        }
        // 端帽是斜置的小矩形，按中心是否落进窗口决定取捨；它本身只有几像素宽，边界处不会产生
        // 可感知的差异。
        if let Some((window_left, window_right)) = self.window {
            if end_x < window_left || end_x >= window_right {
                return;
            }
        }
        let scale = thickness / BAND_TEMPLATE_THICKNESS;
        let [texel_u, _] = texel_uv(template);
        let margin = shape_margin();
        let half = LINE_CAP_LENGTH / 2.0;
        let half_height = (BAND_TEMPLATE_THICKNESS / 2.0 + margin) * scale;
        let u_min = template.u_min;
        let u_max = template.u_min + (margin + LINE_CAP_LENGTH) * SDF_SCALE * texel_u;
        let thresholds = level.thresholds(scale / SDF_SCALE);
        let (sin, cos) = angle.sin_cos();
        // `offset` 是沿向外方向（即 -angle）的距离，`side` 是垂直于线段方向的距离。
        let point = |offset: f32, side: f32| {
            [
                end_x - cos * offset - sin * side,
                end_y - sin * offset + cos * side,
            ]
        };
        for (offset, side, u, v) in [
            (half + margin, -half_height, u_min, template.v_min),
            (half + margin, half_height, u_min, template.v_max),
            (-half, half_height, u_max, template.v_max),
            (-half, -half_height, u_max, template.v_min),
        ] {
            let [x, y] = point(offset, side);
            self.vertices
                .push(vertex(x, y, u, v, template.page, thresholds, level.color));
        }
    }

    /// 用波浪模板画一段波浪线，按 `wavelength` 重复并在 `right` 处裁剪。
    ///
    /// 相位锚点是 `left`：已绘制的周期不会因为右端延长而移动。每个完整周期使用模板形状区的一个
    /// 完整波周期，末周期同时裁剪目标几何与模板 UV；模板按目标波周期等比缩放，因此振幅与线宽
    /// 跟随 Tiqian 给出的字号比例。两端用 `band` 模板补上跟随切线的端帽。
    pub(crate) fn wave(
        &mut self,
        template: ShapeTemplate,
        band: ShapeTemplate,
        left: f32,
        right: f32,
        center_y: f32,
        wavelength: f32,
        level: ShapeLevel,
    ) {
        if right <= left || wavelength <= 0.0 {
            return;
        }
        let scale = wavelength / WAVE_TEMPLATE_PERIOD;
        let half_height = template.height as f32 / SDF_SCALE / 2.0 * scale;
        let [shape_u_min, shape_u_max] = shape_u(template);
        let mut x = left;
        while x < right {
            let next = (x + wavelength).min(right);
            let ratio = (next - x) / wavelength;
            self.quad(
                [x, center_y - half_height, next, center_y + half_height],
                [
                    shape_u_min,
                    template.v_min,
                    shape_u_min + ratio * (shape_u_max - shape_u_min),
                    template.v_max,
                ],
                template.page,
                level,
                scale / SDF_SCALE,
            );
            x = next;
        }

        let amplitude = WAVE_TEMPLATE_AMPLITUDE * scale;
        let slope = amplitude * std::f32::consts::TAU / wavelength;
        let thickness = WAVE_TEMPLATE_THICKNESS * scale;
        // 左端：向内方向是 +x，中线在相位零点上。
        self.line_cap(band, left, center_y, slope.atan(), thickness, level);
        // 右端：向内方向是 -x；中线的纵向位置与斜率都取该处相位。
        let cycles = (right - left) / wavelength;
        let phase = std::f32::consts::TAU * cycles.fract();
        let end_y = center_y + amplitude * phase.sin();
        let end_slope = slope * phase.cos();
        self.line_cap(
            band,
            right,
            end_y,
            (-end_slope).atan2(-1.0),
            thickness,
            level,
        );
    }

    fn quad(&mut self, target: [f32; 4], uv: [f32; 4], page: i32, level: ShapeLevel, scale: f32) {
        let [mut left, top, mut right, bottom] = target;
        let [mut u_min, v_min, mut u_max, v_max] = uv;
        if let Some((window_left, window_right)) = self.window {
            if right <= window_left || left >= window_right {
                return;
            }
            let span = right - left;
            let uv_span = u_max - u_min;
            let clipped_left = left.max(window_left);
            let clipped_right = right.min(window_right);
            // UV 按目标坐标的同一比例插值，因此相邻两段在接缝处的采样完全一致。
            u_min += uv_span * (clipped_left - left) / span;
            u_max = u_min + uv_span * (clipped_right - clipped_left) / span;
            left = clipped_left;
            right = clipped_right;
        }
        let thresholds = level.thresholds(scale);
        self.vertices.extend([
            vertex(left, top, u_min, v_min, page, thresholds, level.color),
            vertex(left, bottom, u_min, v_max, page, thresholds, level.color),
            vertex(right, bottom, u_max, v_max, page, thresholds, level.color),
            vertex(right, top, u_max, v_min, page, thresholds, level.color),
        ]);
    }
}

fn vertex(
    x: f32,
    y: f32,
    u: f32,
    v: f32,
    page: i32,
    thresholds: ShapeThresholds,
    color: [f32; 4],
) -> Vertex {
    Vertex {
        position: [x, y, 0.0],
        tex_coords: [u, v],
        page,
        buffer: thresholds.buffer,
        fill_buffer: thresholds.fill_buffer,
        gamma: thresholds.gamma,
        color,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FontSource;
    use crate::Huozi;
    use crate::glyph_vertices::{
        BackgroundVertices, DecorationShape, DecorationVertices, UnitVertices,
    };
    use crate::layout::{ColorSpace, LayoutStyle, RichTextLayoutOutput};
    use crate::parser::{Segment, TextStyle};
    use crate::shape::ShapeKey;
    use crate::shape::atlas::frame_margin;

    const TEST_FONT: &[u8] = include_bytes!("../../resources/fonts/SourceHanSansSC-VF.otf");

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

    /// 线带模板的两端必须真正闭合。
    ///
    /// 若距离函数只随 y 变化，模板两端边距里仍会被算作「在带内」，几何向外扩多少就会实画出多少。
    #[test]
    fn band_template_closes_its_ends() {
        let mut huozi = setup();
        let template = huozi.shape_template(ShapeKey::Band);
        let atlas = huozi.texture_pixels();
        let x0 = (template.u_min * atlas.width() as f32).round() as usize;
        let y0 = (template.v_min * atlas.height() as f32).round() as usize;
        let center = y0 + template.height as usize / 2;
        let value = |offset: usize| {
            atlas.pixels()
                [(center * atlas.width() as usize + x0 + offset) * 4 + template.page as usize]
                as f32
                / 255.0
        };

        let threshold = fill_threshold();
        let width = template.width as usize;
        assert!(value(0) < threshold, "线带模板左端之外仍被判为在形状内");
        assert!(
            value(width - 1) < threshold,
            "线带模板右端之外仍被判为在形状内"
        );
        let inside = (shape_margin() * SDF_SCALE) as usize + 1;
        assert!(value(inside) >= threshold, "线带形状内部应为实心");
    }

    /// 圆角矩形的角块与模板 1:1，圆弧的切点落在中块边界上。
    #[test]
    fn background_corner_block_keeps_the_arc_unstretched() {
        let mut huozi = setup();
        let output = layout(
            &mut huozi,
            "[background color=\"#FFF3BF\" radius=8 paddingX=6]甲[/background]",
            &LayoutStyle::default(),
        );
        let fragments = backgrounds(&output);
        let fill = &fragments[0].vertices.fill;
        // 九宫格是九个 quad，共 36 个顶点。
        assert_eq!(fill.len(), 36, "背景是九宫格");
        let atlas_width = huozi.texture_pixels().width() as f32;
        let margin = shape_margin();
        let corner = 8.0 + margin;
        // 每个 quad 的四个顶点依次是左上、左下、右下、右上。
        let top_left = fill[0];
        let bottom_right = fill[2];
        assert!((bottom_right.position[0] - top_left.position[0] - corner).abs() <= 0.01);
        assert!((bottom_right.position[1] - top_left.position[1] - corner).abs() <= 0.01);
        let uv_width = (bottom_right.tex_coords[0] - top_left.tex_coords[0]) * atlas_width;
        assert!((uv_width - corner * SDF_SCALE).abs() <= 0.01);
    }

    /// 着重号按字符独立成形：每个点的顶点围绕自己的锚点，不会被合成一个大点。
    #[test]
    fn emphasis_dots_are_shaped_individually() {
        let mut huozi = setup();
        let output = layout(
            &mut huozi,
            "[emphasis]甲乙丙[/emphasis]",
            &LayoutStyle::default(),
        );
        let dots: Vec<_> = decorations(&output)
            .into_iter()
            .filter(|decoration| matches!(decoration.shape, DecorationShape::Dot { .. }))
            .collect();
        assert_eq!(dots.len(), 3, "每个字一个着重号");

        for dot in &dots {
            let DecorationShape::Dot {
                center_x,
                center_y,
                diameter,
            } = dot.shape
            else {
                unreachable!()
            };
            assert!(diameter > 0.0);
            let (left, right) = span(&dot.vertices.fill);
            let middle = (left + right) / 2.0;
            assert!(
                (middle - center_x).abs() <= 1.0,
                "圆点不在自己的锚点上：中心 {middle}，锚点 {center_x}"
            );
            // 模板含四周边距，因此顶点跨度比直径大，但不应大到覆盖相邻的字。
            assert!(
                right - left < diameter + 2.0 * shape_margin() + 1.0,
                "圆点过大：跨度 {}，直径 {diameter}",
                right - left
            );
            let ys: Vec<f32> = dot
                .vertices
                .fill
                .iter()
                .map(|vertex| vertex.position[1])
                .collect();
            let middle_y = (ys.iter().cloned().fold(f32::MAX, f32::min)
                + ys.iter().cloned().fold(f32::MIN, f32::max))
                / 2.0;
            assert!((middle_y - center_y).abs() <= 1.0);
        }
    }

    /// 示亡号用九宫格边框模板绘制：中心块不绘制，四角是完整的 L 形角块。
    #[test]
    fn mourning_frame_uses_eight_nine_slice_blocks() {
        let mut huozi = setup();
        let output = layout(
            &mut huozi,
            "[mourning]甲[/mourning]",
            &LayoutStyle::default(),
        );
        let frame = decorations(&output)
            .into_iter()
            .find(|decoration| matches!(decoration.shape, DecorationShape::Frame { .. }))
            .expect("示例文本产生了示亡号边框");
        let DecorationShape::Frame {
            left,
            top,
            right,
            bottom,
            ..
        } = frame.shape
        else {
            unreachable!()
        };
        let thickness = frame.thickness.max(1.0);
        let fill = &frame.vertices.fill;
        // 九宫格去掉中心块：八个 quad，32 个顶点。
        assert_eq!(fill.len(), 32, "九宫格去掉中心块后是八块");

        let margin = frame_margin(thickness);
        let corner_x = frame_corner(thickness).min((right - left) / 2.0) + margin;
        let corner_y = frame_corner(thickness).min((bottom - top) / 2.0) + margin;
        // 第一个 quad 是左上角块：几何从 (left - margin, top - margin) 起，边长是「角块 + 边距」。
        assert!((fill[0].position[0] - (left - margin)).abs() <= 0.01);
        assert!((fill[0].position[1] - (top - margin)).abs() <= 0.01);
        assert!((fill[2].position[0] - fill[0].position[0] - corner_x).abs() <= 0.01);
        assert!((fill[2].position[1] - fill[0].position[1] - corner_y).abs() <= 0.01);
    }
}
