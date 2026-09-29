//! 形状模板的位图生成。
//!
//! 模板在高于显示尺寸的分辨率下栅格化（[`SDF_SCALE`] 个 texel 对应 1 逻辑像素），沿用与文字完全
//! 相同的 SDF 编码与阈值语义。若按 1 texel = 1 逻辑像素栅格化，1 像素宽的描边只落在约一个 texel
//! 上：沿直线边正好压在 texel 行上，到弧线处则落在 texel 之间，圆弧上的描边会被采样成断续的直边。
//!
//! 模板位图为形状盒四周各留一圈透明边距，供描边外扩、阴影偏移与模糊使用；`calculate_sdf` 会在其
//! 外侧再加一圈 `BUFFER` texel。

use crate::Huozi;
use crate::constant::BUFFER;
use crate::sdf::calculate_sdf;

use super::{ShapeKey, ShapeTemplate};

/// 模板的 SDF 过采样倍率：1 逻辑像素对应多少模板 texel。
pub(crate) const SDF_SCALE: f32 = 4.0;

/// 形状盒之外额外保留的逻辑像素边距。
const TEMPLATE_MARGIN: f32 = 8.0;

/// 圆角矩形模板覆盖的最大半径。
///
/// 模板按整数半径各生成一份，绘制时取同半径的那一份：圆角半径是连续量，取最近模板会把多个半径
/// 折叠到同一形状上，圆角会跳变。
pub(crate) const MAX_BACKGROUND_RADIUS: u32 = 32;

/// 圆角矩形模板的形状盒最小边长；半径更大时自动放大。
const RECT_SHAPE_SIZE: f32 = 32.0;

/// 圆点模板的直径与线带模板的厚度、长度；绘制时按目标尺寸等比缩放。
pub(crate) const DOT_TEMPLATE_DIAMETER: f32 = 8.0;
pub(crate) const BAND_TEMPLATE_THICKNESS: f32 = 2.0;
const BAND_TEMPLATE_LENGTH: f32 = 32.0;

/// 波浪模板的一个周期及其度量。
///
/// 振幅、厚度相对周期的比例取自 CLREQ 参考实现的最终参数，因此按目标波周期等比缩放后厚度与振幅
/// 自动跟随。
pub(crate) const WAVE_TEMPLATE_PERIOD: f32 = 16.0;
pub(crate) const WAVE_TEMPLATE_AMPLITUDE: f32 = 2.4;
pub(crate) const WAVE_TEMPLATE_THICKNESS: f32 = 2.5;
pub(crate) const WAVE_TEMPLATE_HEIGHT: f32 = 16.0;

/// 示亡号边框模板的基准线宽、角块深度与空腔宽度。
///
/// 模板形状盒是一个矩形框，可分别选择是否有左、右竖边；绘制时整体按目标线宽等比缩放。
pub(crate) const FRAME_TEMPLATE_THICKNESS: f32 = 2.0;
/// 角块在空腔一侧额外覆盖的深度。
///
/// 描边从形状边界向外扩，在空腔那一侧就是向空腔内扩；这段区域必须由 1:1 的角块提供，否则空腔边界
/// 上的描边会被几何边缘切掉，框只剩外侧描边。
pub(crate) const FRAME_TEMPLATE_NOTCH: f32 = TEMPLATE_MARGIN;
/// 空腔中可沿两个方向拉伸的中间段宽度。
const FRAME_TEMPLATE_GAP: f32 = FRAME_TEMPLATE_THICKNESS * 2.0;
const FRAME_TEMPLATE_SPAN: f32 =
    FRAME_TEMPLATE_THICKNESS * 2.0 + FRAME_TEMPLATE_NOTCH * 2.0 + FRAME_TEMPLATE_GAP;

/// 生成一个模板的位图并写入图集。
pub(crate) fn generate(huozi: &mut Huozi, key: ShapeKey) -> ShapeTemplate {
    match key {
        ShapeKey::Rect { radius } => {
            let radius = radius.min(MAX_BACKGROUND_RADIUS) as f32;
            let size = RECT_SHAPE_SIZE.max(radius * 2.0 + 8.0);
            rasterize(huozi, size, size, |x, y| {
                rounded_rect_distance(x, y, size, size, radius)
            })
        }
        ShapeKey::Dot => {
            let diameter = DOT_TEMPLATE_DIAMETER;
            let half = diameter / 2.0;
            rasterize(huozi, diameter, diameter, |x, y| {
                ((x - half).powi(2) + (y - half).powi(2)).sqrt() - half
            })
        }
        // 必须是有限矩形：若距离函数只随 y 变化（无限长的带），模板两端边距里仍会算作“在带内”，
        // 几何外扩多少就会实画出多少。
        ShapeKey::Band => rasterize(
            huozi,
            BAND_TEMPLATE_LENGTH,
            BAND_TEMPLATE_THICKNESS,
            |x, y| rounded_rect_distance(x, y, BAND_TEMPLATE_LENGTH, BAND_TEMPLATE_THICKNESS, 0.0),
        ),
        ShapeKey::Wave => rasterize(huozi, WAVE_TEMPLATE_PERIOD, WAVE_TEMPLATE_HEIGHT, |x, y| {
            wave_distance(
                x,
                y,
                WAVE_TEMPLATE_PERIOD,
                WAVE_TEMPLATE_AMPLITUDE,
                WAVE_TEMPLATE_THICKNESS,
                WAVE_TEMPLATE_HEIGHT,
            )
        }),
        ShapeKey::Frame {
            open_start,
            open_end,
        } => rasterize(huozi, FRAME_TEMPLATE_SPAN, FRAME_TEMPLATE_SPAN, |x, y| {
            frame_distance(x, y, !open_start, !open_end)
        }),
    }
}

/// 按解析式距离场栅格化一张模板并写入图集。
///
/// `distance` 收到形状盒内的逻辑坐标（形状盒左上角为原点），返回带符号距离。
fn rasterize(
    huozi: &mut Huozi,
    shape_width: f32,
    shape_height: f32,
    distance: impl Fn(f32, f32) -> f32,
) -> ShapeTemplate {
    let width = texels(shape_width + TEMPLATE_MARGIN * 2.0);
    let height = texels(shape_height + TEMPLATE_MARGIN * 2.0);
    let mut alpha = vec![0_u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let px = (x as f32 + 0.5) / SDF_SCALE - TEMPLATE_MARGIN;
            let py = (y as f32 + 0.5) / SDF_SCALE - TEMPLATE_MARGIN;
            // 覆盖率按半个 texel 过渡，与字形栅格化一致。
            let signed = distance(px, py) * SDF_SCALE;
            alpha[(y * width + x) as usize] = ((0.5 - signed).clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    let (bitmap, width, height) = calculate_sdf(&alpha, width, height);
    let slot = huozi.write_atlas_bitmap(&bitmap, width, height);
    let texture_size = huozi.texture_pixels().width() as f32;
    ShapeTemplate {
        page: slot.page,
        u_min: slot.offset_x as f32 / texture_size,
        u_max: (slot.offset_x + width as i32) as f32 / texture_size,
        v_min: slot.offset_y as f32 / texture_size,
        v_max: (slot.offset_y + height as i32) as f32 / texture_size,
        width,
        height,
    }
}

/// 把逻辑像素尺寸换算成模板 texel 数。
fn texels(logical: f32) -> u32 {
    (logical * SDF_SCALE).round().max(1.0) as u32
}

/// 模板位图中形状盒到图像边缘的 texel 数：SDF 的缓冲圈加上形状边距。
fn shape_margin_texels() -> f32 {
    BUFFER as f32 + TEMPLATE_MARGIN * SDF_SCALE
}

/// 模板位图中形状盒到图像边缘的逻辑像素距离。
pub(crate) fn shape_margin() -> f32 {
    shape_margin_texels() / SDF_SCALE
}

/// 边框角块的形状尺寸：线宽加上空腔侧需要 1:1 覆盖的深度。
pub(crate) fn frame_corner(thickness: f32) -> f32 {
    thickness + FRAME_TEMPLATE_NOTCH * thickness / FRAME_TEMPLATE_THICKNESS
}

/// 边框几何相对形状盒在目标尺寸下的外扩量；模板按线宽等比缩放。
///
/// 测试用它从顶点几何反推形状盒。
#[cfg(test)]
pub(crate) fn frame_margin(thickness: f32) -> f32 {
    shape_margin() * thickness / FRAME_TEMPLATE_THICKNESS
}

/// 模板位图内形状盒沿 u 方向的 UV 范围（排除两侧边距）。
pub(crate) fn shape_u(template: ShapeTemplate) -> [f32; 2] {
    let [texel_u, _] = texel_uv(template);
    let margin = shape_margin_texels() * texel_u;
    [template.u_min + margin, template.u_max - margin]
}

/// 一个模板 texel 在 UV 空间中的跨度。
///
/// 模板位图只占图集的一个网格块，因此分母必须是模板自身的位图尺寸。
pub(crate) fn texel_uv(template: ShapeTemplate) -> [f32; 2] {
    [
        (template.u_max - template.u_min) / template.width as f32,
        (template.v_max - template.v_min) / template.height as f32,
    ]
}

/// 点到圆角矩形的带符号距离；内部为负。
fn rounded_rect_distance(px: f32, py: f32, width: f32, height: f32, radius: f32) -> f32 {
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    let radius = radius.min(half_width).min(half_height);
    let dx = (px - half_width).abs() - (half_width - radius);
    let dy = (py - half_height).abs() - (half_height - radius);
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    outside + dx.max(dy).min(0.0) - radius
}

/// 点到轴对齐矩形的带符号距离；内部为负。矩形以 `left`、`top` 为左上角。
fn box_distance(px: f32, py: f32, left: f32, top: f32, right: f32, bottom: f32) -> f32 {
    let dx = (left - px).max(px - right);
    let dy = (top - py).max(py - bottom);
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    outside + dx.max(dy).min(0.0)
}

/// 点到示亡号边框形状的带符号距离；框由上边、下边与可选的左右竖边并集而成。
fn frame_distance(px: f32, py: f32, left_edge: bool, right_edge: bool) -> f32 {
    let span = FRAME_TEMPLATE_SPAN;
    let thickness = FRAME_TEMPLATE_THICKNESS;
    let mut distance = box_distance(px, py, 0.0, 0.0, span, thickness);
    distance = distance.min(box_distance(px, py, 0.0, span - thickness, span, span));
    if left_edge {
        distance = distance.min(box_distance(
            px,
            py,
            0.0,
            thickness,
            thickness,
            span - thickness,
        ));
    }
    if right_edge {
        distance = distance.min(box_distance(
            px,
            py,
            span - thickness,
            thickness,
            span,
            span - thickness,
        ));
    }
    distance
}

/// 点到一段正弦波的带符号距离；用于生成可首尾相接的波浪模板。
fn wave_distance(
    px: f32,
    py: f32,
    period: f32,
    amplitude: f32,
    thickness: f32,
    height: f32,
) -> f32 {
    const SAMPLES: usize = 32;
    let center_y = height / 2.0;
    let start = -period;
    let end = 2.0 * period;
    let mut best = f32::INFINITY;
    let mut previous = (
        start,
        center_y + amplitude * (std::f32::consts::TAU * start / period).sin(),
    );
    for index in 1..=SAMPLES {
        let x = start + (end - start) * index as f32 / SAMPLES as f32;
        let y = center_y + amplitude * (std::f32::consts::TAU * x / period).sin();
        best = best.min(point_segment_distance(px, py, previous, (x, y)));
        previous = (x, y);
    }
    best - thickness / 2.0
}

fn point_segment_distance(px: f32, py: f32, start: (f32, f32), end: (f32, f32)) -> f32 {
    let (sx, sy) = start;
    let (ex, ey) = end;
    let dx = ex - sx;
    let dy = ey - sy;
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared <= f32::EPSILON {
        0.0
    } else {
        (((px - sx) * dx + (py - sy) * dy) / length_squared).clamp(0.0, 1.0)
    };
    let cx = sx + t * dx;
    let cy = sy + t * dy;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FontSource;

    const TEST_FONT: &[u8] = include_bytes!("../../resources/fonts/SourceHanSansSC-VF.otf");

    fn setup() -> Huozi {
        Huozi::new(vec![FontSource::new(TEST_FONT.to_vec())]).unwrap()
    }

    /// 读出模板位图的单通道像素。
    fn template_pixels(huozi: &Huozi, template: ShapeTemplate) -> Vec<u8> {
        let atlas = huozi.texture_pixels();
        let stride = atlas.width() as usize;
        let x0 = (template.u_min * atlas.width() as f32).round() as usize;
        let y0 = (template.v_min * atlas.height() as f32).round() as usize;
        let mut pixels = Vec::with_capacity((template.width * template.height) as usize);
        for y in 0..template.height as usize {
            for x in 0..template.width as usize {
                pixels.push(
                    atlas.pixels()[((y0 + y) * stride + x0 + x) * 4 + template.page as usize],
                );
            }
        }
        pixels
    }

    /// 不同半径各自有模板，且位图确实不同。
    #[test]
    fn each_radius_gets_its_own_template() {
        let mut huozi = setup();
        let templates = [0_u32, 4, 5, 8, 12, 20, MAX_BACKGROUND_RADIUS]
            .map(|radius| huozi.shape_template(ShapeKey::Rect { radius }));

        for (index, left) in templates.iter().enumerate() {
            for right in &templates[index + 1..] {
                assert_ne!(
                    template_pixels(&huozi, *left),
                    template_pixels(&huozi, *right),
                    "两个半径复用了同一份模板位图"
                );
            }
        }
    }

    /// 模板必须在高于显示尺寸的 SDF 分辨率下栅格化，否则 1 像素宽的描边只落在约一个 texel 上，
    /// 弧线会被采样成断续的直边。
    #[test]
    fn shape_templates_oversample_the_sdf() {
        let mut huozi = setup();
        assert!(SDF_SCALE >= 2.0, "SDF 过采样倍率过低：{SDF_SCALE}");
        let template = huozi.shape_template(ShapeKey::Rect { radius: 8 });
        let atlas_width = huozi.texture_pixels().width() as f32;

        let margin = shape_margin();
        let shape_texels =
            (template.u_max - template.u_min) * atlas_width - margin * SDF_SCALE * 2.0;
        let expected = template.width as f32 - margin * SDF_SCALE * 2.0;
        assert!(
            (shape_texels - expected).abs() <= 0.5,
            "形状区 texel 数 = {shape_texels}，期望 {expected}"
        );
    }
}
