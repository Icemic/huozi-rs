pub const GRID_SIZE: f32 = 128.;
pub const FONT_SIZE: f32 = 96.;
pub const BUFFER: u32 = 16;
pub const RADIUS: f32 = 24.;
pub const CUTOFF: f32 = 0.25;
/// 填充阈值的额外下调量，单位为归一化 SDF 值；正值使笔画外扩。
pub const FILL_THRESHOLD_BIAS: f32 = 0.014643;
/// SDF 边缘额外平滑半宽，单位为逻辑像素；设为 0 可关闭额外平滑。
pub const EDGE_SMOOTHING_HALF_WIDTH: f32 = 0.30;
pub const TEXTURE_SIZE: u32 = 2048;
// 112 is just a magic number, may should be replaced by more reasonable algorithm
pub const ASCENT: f32 = 112.;
