use crate::clip::Clip;
use crate::state::State;
use std::time::{Duration, Instant};
use winit::{
    event::*,
    event_loop::EventLoopWindowTarget,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

/// App manages the application state and coordinates between different components
pub struct App {
    link: crate::Link,
    clip: Clip,
    pub state: State<'static>,
    surface_configured: bool,
    texture_initialized: bool,
    frame_limiter: FrameLimiter,
}

/// Helper struct for frame rate limiting
struct FrameLimiter {
    target_fps: u32,
    frame_duration: Duration,
    last_frame_time: Instant,
}

impl FrameLimiter {
    fn new(target_fps: u32) -> Self {
        Self {
            target_fps,
            frame_duration: Duration::from_secs_f64(1.0 / target_fps as f64),
            last_frame_time: Instant::now(),
        }
    }

    fn should_render(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);
        
        if elapsed >= self.frame_duration {
            self.last_frame_time = now;
            true
        } else {
            false
        }
    }
}

impl App {
    /// Create a new App instance with the given components
    pub async fn new(window: &'static Window, link: crate::Link, clip: Clip) -> Self {
        let state = State::new(window).await;
        let frame_limiter = FrameLimiter::new(60); // 60 FPS target
        
        log::info!("Starting render loop with {} FPS target", frame_limiter.target_fps);
        
        // Request initial redraw
        state.window().request_redraw();
        
        Self {
            link,
            clip,
            state,
            surface_configured: false,
            texture_initialized: false,
            frame_limiter,
        }
    }

    /// Handle window events
    pub fn handle_window_event(&mut self, event: &WindowEvent, elwt: &EventLoopWindowTarget<()>) {
        if !self.state.input(event) {
            match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(physical_size) => {
                    self.handle_resize(*physical_size);
                }
                WindowEvent::RedrawRequested => {
                    self.handle_redraw_request(elwt);
                }
                _ => {}
            }
        }
    }

    /// Handle window resize events
    fn handle_resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.surface_configured = true;
        self.state.resize(new_size);
    }

    /// Handle redraw requests and perform rendering
    fn handle_redraw_request(&mut self, elwt: &EventLoopWindowTarget<()>) {
        if !self.surface_configured {
            return;
        }

        if self.frame_limiter.should_render() {
            // Update link timing
            self.link.update_phase_and_beat();
            
            // Get current video frame
            let frame = self.clip.play_video_at_position(self.link.phase as f32);

            // Initialize texture on first frame
            if !self.texture_initialized {
                self.initialize_texture(&frame);
            }

            // Update rendering state with new frame
            self.state.update_texture_with_frame(&frame);
            self.state.update();
            
            // Render frame and handle errors
            if let Err(error) = self.state.render() {
                self.handle_render_error(error, elwt);
            }
        }

        // Request next frame
        self.state.window().request_redraw();
    }

    /// Initialize texture with video frame dimensions
    fn initialize_texture(&mut self, frame: &ffmpeg_next::util::frame::Video) {
        self.state.recreate_texture(
            frame.width() as u32,
            frame.height() as u32,
        );
        self.texture_initialized = true;
    }

    /// Handle rendering errors
    fn handle_render_error(&mut self, error: wgpu::SurfaceError, elwt: &EventLoopWindowTarget<()>) {
        match error {
            // Reconfigure the surface if it's lost or outdated
            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                self.state.resize(self.state.size);
            }
            // The system is out of memory, we should quit
            wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other => {
                log::error!("Render error: {:?}", error);
                elwt.exit();
            }
            // This happens when a frame takes too long to present
            wgpu::SurfaceError::Timeout => {
                log::warn!("Surface timeout");
            }
        }
    }
}
