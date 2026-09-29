//! 非文字图形的颜色与描边、阴影参数。
//!
//! 这些参数只在生成顶点时使用：颜色、描边宽度、阴影偏移与模糊都会换算进顶点携带的字段，最终
//! 对外只有顶点。

/// 非文字绘制结果的颜色与描边、阴影参数。
///
/// 颜色在布局时按 `ColorSpace` 转换为线性或 sRGB 数值。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShapePaint {
    pub(crate) fill: [f32; 4],
    pub(crate) stroke: Option<ShapeStroke>,
    pub(crate) shadow: Option<ShapeShadow>,
}

/// 非文字图形的描边参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapeStroke {
    pub(crate) color: [f32; 4],
    pub(crate) width: f32,
}

/// 非文字图形的阴影参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapeShadow {
    pub(crate) color: [f32; 4],
    pub(crate) offset_x: f32,
    pub(crate) offset_y: f32,
    pub(crate) blur_radius: f32,
    pub(crate) spread_radius: f32,
}
