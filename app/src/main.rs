mod clip;
mod app;
mod state;
mod vertex;
mod link;

use link::Link;

use ffmpeg_next as ffmpeg;
use winit::{event::Event, event_loop::EventLoop, window::{Window, WindowBuilder}};

fn main() {
    env_logger::init();
    ffmpeg::init().unwrap();
    let filename = std::env::args()
        .nth(1)
        .expect("Please provide a video filename");
    println!("Opening file: {}\n", filename);

    // Video setup
    let mut clip = clip::Clip::new(&filename).unwrap();
    let _ = clip.cache_all_frames();

    let link = Link::new();
    // Main loop
    pollster::block_on(run(link, clip));
}

/// Main entry point for the application
pub async fn run(link: crate::Link, clip: clip::Clip) {
    
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    
    // Create a static reference to the window (required for State lifetime)
    let window: &'static Window = Box::leak(Box::new(window));
    
    let mut app = app::App::new(window, link, clip).await;

    let _ = event_loop.run(move |event, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == app.state.window().id() => {
                app.handle_window_event(event, control_flow);
            }
            _ => {}
        }
    });
}
