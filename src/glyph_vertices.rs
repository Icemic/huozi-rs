use tiqian::core::geometry::Rect;
use tiqian::core::layout_queries::RichTextCornerRadii;
use tiqian::core::text_model::{DecorationKind, RichTextLinePattern};

use crate::layout::Vertex;

/// 一个可以直接绘制的布局元素。
///
/// 每个元素自带顶点，因此 `RichTextLayoutOutput::glyphs` 的可见前缀就是逐字显示状态。
///
/// 非文字图形的同一个连续范围在布局时已经合并定形，再按排版单元切成段：各段拼起来是完整的图形，
/// 段与段之间没有重复的描边或阴影，逐字推进时图形跟着长。
///
/// [`RichTextLayoutOutput::glyphs`]: crate::layout::RichTextLayoutOutput
#[derive(Debug, Clone)]
pub enum UnitVertices {
    Text(TextVertices),
    Background(BackgroundVertices),
    Line(LineVertices),
    Decoration(DecorationVertices),
    InlineObject(InlineObjectVertices),
}

/// 一组按绘制层组织的图形顶点。
///
/// 每层是一串四边形：每 4 个连续顶点构成一个 quad，顺序为左上、左下、右下、右上，按
/// `[0, 1, 2, 0, 2, 3]` 展开即可。层为空表示该层没有内容。
#[derive(Debug, Clone, Default)]
pub struct ShapeVertices {
    pub shadow: Vec<Vertex>,
    pub stroke: Vec<Vertex>,
    pub fill: Vec<Vertex>,
}

/// 正文、拼音注音或注音符号的 SDF 四边形。建议按 shadow、stroke、fill 的顺序绘制。
#[derive(Debug, Clone)]
pub struct TextVertices {
    /// 阴影层顶点。
    pub shadow: Option<[Vertex; 4]>,
    /// 描边层顶点。
    pub stroke: Option<[Vertex; 4]>,
    /// 填充层顶点。
    pub fill: [Vertex; 4],
    /// 文字在文本流方向上的位置。
    pub col: u32,
    /// 文字在垂直于文本流方向上的位置。
    pub row: u32,
    /// 占位矩形左上角 x。
    pub x: u32,
    /// 占位矩形左上角 y。
    pub y: u32,
    /// 占位矩形宽度。
    pub width: u32,
    /// 占位矩形高度。
    pub height: u32,
    /// 字形相对图集基准字号的缩放比。
    pub scale_ratio: f32,
    /// 这段文字在富文本中的角色。
    pub role: TextRole,
}

/// 文字在富文本中的角色。三者共用同一个 SDF 图集与文字 shader。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRole {
    Body,
    Ruby,
    Bopomofo,
}

/// 一个排版单元对某个连续背景范围的贡献。
///
/// 顶点覆盖该单元对应的横向范围；同一个连续范围的各段拼起来是完整的圆角矩形。
#[derive(Debug, Clone)]
pub struct BackgroundVertices {
    /// cluster 对应的文档坐标矩形。
    pub rect: Rect,
    /// Tiqian 已解析的四角半径。
    pub corner_radii: RichTextCornerRadii,
    /// 按绘制层组织的顶点。
    pub vertices: ShapeVertices,
    pub row: u32,
    pub col: u32,
}

/// 一个排版单元对某条下划线或删除线范围的贡献。
#[derive(Debug, Clone)]
pub struct LineVertices {
    /// 当前片段覆盖的横向区间。
    pub left: f32,
    pub right: f32,
    /// Tiqian 决定的中心线纵坐标。
    pub line_y: f32,
    /// 线宽。
    pub thickness: f32,
    /// 实线、虚线或点线及其原始长度参数。
    pub pattern: RichTextLinePattern,
    /// 按绘制层组织的顶点。
    pub vertices: ShapeVertices,
    pub row: u32,
    pub col: u32,
}

/// 一个排版单元对某条 CLREQ 装饰范围或某个着重号字符的贡献。
#[derive(Debug, Clone)]
pub struct DecorationVertices {
    pub kind: DecorationKind,
    pub shape: DecorationShape,
    /// 线宽；着重号没有线宽，取 0。
    pub thickness: f32,
    /// 波浪参数；只有书名号提供。
    pub wave: Option<DecorationWave>,
    /// 按绘制层组织的顶点。
    pub vertices: ShapeVertices,
    pub row: u32,
    pub col: u32,
}

/// 装饰片段的几何形状。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecorationShape {
    /// 矩形几何：示亡号、专名号与书名号。`top == bottom` 表示这是一条中心线。
    Frame {
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
        /// 完整源范围向相邻行延续；该边不属于本条范围，渲染方不绘制对应边框。
        open_start: bool,
        open_end: bool,
    },
    /// 圆点几何：着重号。
    Dot {
        center_x: f32,
        center_y: f32,
        diameter: f32,
    },
}

/// 书名号波浪线的周期与振幅，单位为逻辑像素。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecorationWave {
    pub wavelength: f32,
    pub amplitude: f32,
}

/// 一个行内对象的绘制结果。
///
/// Huozi 不保存对象资源、GPU handle 或回调；`id` 是调用方资源表的键。它没有顶点：对象内容由
/// 调用方按自己的资源绘制，这里只给出布局位置。
#[derive(Debug, Clone)]
pub struct InlineObjectVertices {
    /// 文档坐标矩形。
    pub rect: Rect,
    /// 调用方声明的资源键；缺失或为空时没有对应的交互输出。
    pub id: Option<String>,
    /// 替代文本与语义文本。
    pub alt: String,
    pub row: u32,
    pub col: u32,
}
