#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MVPUniform {
    pub width: f32,
    pub height: f32,
}
