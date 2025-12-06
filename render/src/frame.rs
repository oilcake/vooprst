pub(crate) struct Frame {
    pub universal_texture: Option<wgpu::Texture>,
    pub video_width: u32,
    pub video_height: u32,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            universal_texture: None,
            video_width: 0,
            video_height: 0,
        }
    }
}
