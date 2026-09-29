//! 非文字图形的 SDF 模板与顶点生成。
//!
//! 背景、下划线、删除线、着重号、示亡号、专名号和书名号共用文字 shader 的同一套 SDF 管线。本
//! 模块负责它们在库内的一生：按需生成模板位图并常驻图集、把同一个连续范围的片段定形成完整
//! 图形、再按排版单元切成段，使每段都能独立成为一个可直接绘制的元素。
//!
//! 调用方只面对最终顶点，不需要选模板、定尺寸或选阈值。

mod assemble;
mod atlas;
mod paint;
mod vertices;

pub(crate) use assemble::{
    BackgroundFragment, DecorationFragment, FragmentGroup, LineFragment, shape_backgrounds,
    shape_decorations, shape_lines,
};
pub(crate) use paint::{ShapePaint, ShapeShadow, ShapeStroke};

pub(crate) use atlas::MAX_BACKGROUND_RADIUS;

use crate::Huozi;

/// 常驻形状模板的标识。
///
/// 模板按需生成：同一份文档通常只用到少数几个半径，预先烤出全部半径会白白占掉图集。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ShapeKey {
    /// 圆角矩形，按整数半径索引。
    Rect { radius: u32 },
    /// 圆点：着重号与点线共用。
    Dot,
    /// 线带：实线、虚线与端帽共用。
    Band,
    /// 波浪周期的模板。
    Wave,
    /// 示亡号边框；`open_*` 表示该边向相邻行延续，因而不画竖边。
    Frame { open_start: bool, open_end: bool },
}

/// 一个已经常驻图集的形状模板。
///
/// UV 范围正好覆盖位图区域，图集使用 clamp 采样；一个 texel 对应一个逻辑像素。
#[derive(Debug, Clone, Copy)]
pub(crate) struct ShapeTemplate {
    /// 图集页（RGBA 通道）下标，直接用于 `Vertex::page`。
    pub(crate) page: i32,
    pub(crate) u_min: f32,
    pub(crate) u_max: f32,
    pub(crate) v_min: f32,
    pub(crate) v_max: f32,
    /// 位图的像素尺寸。
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Huozi {
    /// 取一个常驻形状模板，不存在时生成并写入图集。
    ///
    /// 模板不参与字形 LRU 淘汰，因此可以长期复用返回的 UV。
    pub(crate) fn shape_template(&mut self, key: ShapeKey) -> ShapeTemplate {
        if let Some(template) = self.shapes.get(&key) {
            return *template;
        }
        let template = atlas::generate(self, key);
        self.shapes.insert(key, template);
        template
    }
}
