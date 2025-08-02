use crate::clip::Clip;
use crate::state::State;
use std::path::PathBuf;
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
    files: Vec<PathBuf>,
    current_file_index: usize,
    last_mouse_activity: Instant,
    cursor_hidden: bool,
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
    pub async fn new(window: &'static Window, link: crate::Link, clip: Clip, files: Vec<PathBuf>, current_file_index: usize) -> Self {
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
            files,
            current_file_index,
            last_mouse_activity: Instant::now(),
            cursor_hidden: false,
        }
    }

    /// Handle left arrow press - load previous file
    fn on_left_arrow(&mut self) {
        if self.current_file_index > 0 {
            self.load_file(self.current_file_index - 1);
        } else {
            log::info!("Already at first file");
        }
    }

    /// Handle right arrow press - load next file
    fn on_right_arrow(&mut self) {
        if self.current_file_index < self.files.len() - 1 {
            self.load_file(self.current_file_index + 1);
        } else {
            log::info!("Already at last file");
        }
    }

    /// Load a file by index
    fn load_file(&mut self, index: usize) {
        if index >= self.files.len() {
            log::error!("File index {} out of bounds (max: {})", index, self.files.len() - 1);
            return;
        }

        let file_path = &self.files[index];
        log::info!("Loading file {}/{}: {}", index + 1, self.files.len(), file_path.display());

        match Clip::new(file_path.to_str().unwrap()) {
            Ok(mut new_clip) => {
                if let Err(e) = new_clip.cache_all_frames() {
                    log::error!("Failed to cache frames for {}: {}", file_path.display(), e);
                    return;
                }
                
                self.clip = new_clip;
                self.current_file_index = index;
                self.texture_initialized = false; // Reset texture for new video dimensions
                
                // Update window title
                let filename = file_path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Unknown");
                self.state.window().set_title(&format!("Voop Video Player - {}", filename));
                
                log::info!("Successfully loaded file: {}", filename);
            }
            Err(e) => {
                log::error!("Failed to load file {}: {}", file_path.display(), e);
            }
        }
    }

    /// Handle window events
    pub fn handle_window_event(&mut self, event: &WindowEvent, elwt: &EventLoopWindowTarget<()>) {
        // Handle mouse activity for cursor hiding
        match event {
            WindowEvent::CursorMoved { .. } => {
                self.on_mouse_activity();
            }
            WindowEvent::MouseInput { .. } => {
                self.on_mouse_activity();
            }
            WindowEvent::MouseWheel { .. } => {
                self.on_mouse_activity();
            }
            _ => {}
        }

        // First check for app-level keys before passing to state
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::ArrowLeft),
                    state: winit::event::ElementState::Pressed,
                    ..
                },
                ..
            } => {
                self.on_left_arrow();
                return; // Don't pass to state
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::ArrowRight),
                    state: winit::event::ElementState::Pressed,
                    ..
                },
                ..
            } => {
                self.on_right_arrow();
                return; // Don't pass to state
            }
            _ => {}
        }

        // Then handle other events through state or directly
        if !self.state.input(event) {
            match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(physical_size) => {
                    self.handle_resize(*physical_size);
                }
                WindowEvent::RedrawRequested => {
                    self.handle_redraw_request(elwt);
                }
                WindowEvent::Occluded(occluded) => {
                    log::info!("Window occluded: {}", occluded);
                }
                _ => {}
            }
        }
    }

    /// Handle mouse activity - show cursor and reset timer
    fn on_mouse_activity(&mut self) {
        self.last_mouse_activity = Instant::now();
        if self.cursor_hidden {
            self.show_cursor();
        }
    }

    /// Show the cursor
    fn show_cursor(&mut self) {
        self.state.window().set_cursor_visible(true);
        self.cursor_hidden = false;
        log::debug!("Cursor shown");
    }

    /// Hide the cursor
    fn hide_cursor(&mut self) {
        self.state.window().set_cursor_visible(false);
        self.cursor_hidden = true;
        log::debug!("Cursor hidden");
    }

    /// Check if cursor should be hidden based on inactivity
    fn update_cursor_visibility(&mut self) {
        const CURSOR_HIDE_TIMEOUT: Duration = Duration::from_secs(1);
        
        if !self.cursor_hidden && self.last_mouse_activity.elapsed() >= CURSOR_HIDE_TIMEOUT {
            self.hide_cursor();
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
            // Update cursor visibility based on mouse inactivity
            self.update_cursor_visibility();
            
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
