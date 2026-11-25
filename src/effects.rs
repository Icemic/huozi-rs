//! 仅改变绘制外观的字形效果，不依赖 Godot、GPU 后端或调用方时钟。
//!
//! 调用方保留 shaping 后的布局，为同一 cluster 的全部字形提供相同的来源范围。
//! 效果求值不分配内存，只改变当前外观，不修改字形进宽、交互矩形或图集数据。
//! 多个效果按调用顺序叠加位移和 RGB 颜色。

use std::ops::Range;

use serde::{Deserialize, Serialize};

/// 所有 Huozi 调用方均可使用的固定视觉效果。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    Cluster,
    Wave,
    Shake,
    /// 目标 RGBA 颜色；alpha 控制混色强度，不改变字形透明度。
    Glow {
        color: [f32; 4],
    },
    Glitch,
}

/// 调用方提供的单个 shaping cluster 的显示进度与动画状态。
#[derive(Debug, Clone)]
pub struct EffectContext {
    /// Unicode scalar 半开范围；起点决定动画相位。
    pub source: Range<usize>,
    pub elapsed_seconds: f64,
    /// `None` 显示全文；`Some(n)` 只显示前 `n` 个 scalar 覆盖的完整 cluster。
    pub visible_characters: Option<usize>,
    /// 停止运动与颜色脉动，保留静态 glow 颜色。
    pub reduce_motion: bool,
}

/// 当前绘制外观；效果保留 alpha，且不会重新显示已隐藏的字形。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphAppearance {
    pub offset: [f32; 2],
    pub color: [f32; 4],
    pub visible: bool,
}

impl Effect {
    /// 解析效果名称，不接受调用方专用的标签前缀。
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "cluster" => Some(Self::Cluster),
            "wave" => Some(Self::Wave),
            "shake" => Some(Self::Shake),
            "glow" => Some(Self::Glow {
                color: [1.0, 0.86, 0.53, 1.0],
            }),
            "glitch" => Some(Self::Glitch),
            _ => None,
        }
    }

    /// 将一个效果应用于当前外观，不改变布局。
    /// 尚未完整显示的 cluster 会直接隐藏，不修改其绘制属性。
    pub fn apply(self, context: &EffectContext, glyph: &mut GlyphAppearance) {
        if context
            .visible_characters
            .is_some_and(|visible| context.source.end > visible)
        {
            glyph.visible = false;
            return;
        }

        let phase = context.source.start as f64 * 1.7;
        let clock = context.elapsed_seconds;
        match self {
            Self::Cluster => {}
            Self::Glow { color } => {
                let emphasis = if context.reduce_motion {
                    0.9
                } else {
                    0.9 + 0.1 * (clock * 2.0 + phase).sin()
                };
                glyph.blend_rgb(
                    [color[0], color[1], color[2]],
                    (emphasis * 0.4) as f32 * color[3],
                );
            }
            _ if context.reduce_motion => {}
            Self::Wave => {
                glyph.offset[1] += ((clock * 3.0 + phase).sin() * 2.0).round() as f32;
            }
            Self::Shake => {
                let tick = (clock * 12.0).floor();
                glyph.offset[0] += (tick * 7.0 + phase).sin().round() as f32;
                glyph.offset[1] += (tick * 11.0 + phase).sin().round() as f32;
            }
            Self::Glitch => {
                let tick = (clock * 8.0).floor();
                if (tick * 13.0 + phase).sin() > 0.94 {
                    let direction = (tick + phase).sin();
                    if direction != 0.0 {
                        glyph.offset[0] += (2.0 * direction.signum()) as f32;
                    }
                    glyph.blend_rgb([0.55, 0.95, 1.0], 0.6);
                }
            }
        }
    }
}

impl GlyphAppearance {
    fn blend_rgb(&mut self, target: [f32; 3], weight: f32) {
        for (channel, target) in self.color[..3].iter_mut().zip(target) {
            *channel += (target - *channel) * weight;
        }
    }
}
