use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pos: [f32; 2],
    uv:  [f32; 2],
}
impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as _,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub const VERTICES: &[Vertex] = &[
    Vertex { pos: [-1.0, -1.0], uv: [0.0, 1.0] },
    Vertex { pos: [ 1.0, -1.0], uv: [1.0, 1.0] },
    Vertex { pos: [ 1.0,  1.0], uv: [1.0, 0.0] },
    Vertex { pos: [-1.0,  1.0], uv: [0.0, 0.0] },
];
pub const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

