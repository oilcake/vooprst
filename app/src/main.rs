mod clip;
mod app;
mod state;
mod vertex;

use transport::link::Link;

use ffmpeg_next as ffmpeg;
use std::path::PathBuf;
use winit::{event::Event, event_loop::EventLoop, window::{Window, WindowBuilder}};

fn main() {
    env_logger::init();
    ffmpeg::init().unwrap();
    let path_arg = std::env::args()
        .nth(1)
        .expect("Please provide a video file or folder path");
    
    let path = PathBuf::from(&path_arg);
    
    // Determine if it's a file or directory
    let (files, current_index) = if path.is_dir() {
        // Load all video files from directory
        let mut video_files: Vec<PathBuf> = std::fs::read_dir(&path)
            .expect("Failed to read directory")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_file() {
                    let ext = path.extension()?.to_str()?;
                    if matches!(ext.to_lowercase().as_str(), "mp4" | "avi" | "mov" | "mkv" | "webm") {
                        Some(path)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        video_files.sort();
        
        if video_files.is_empty() {
            panic!("No video files found in directory: {}", path_arg);
        }
        
        println!("Found {} video files in directory", video_files.len());
        (video_files, 0)
    } else if path.is_file() {
        // Single file
        println!("Loading single file: {}", path_arg);
        (vec![path], 0)
    } else {
        panic!("Path does not exist: {}", path_arg);
    };
    
    // Load first file
    let first_file = &files[current_index];
    println!("Opening file: {}\n", first_file.display());
    
    let mut clip = clip::Clip::new(first_file.to_str().unwrap()).unwrap();
    let _ = clip.cache_all_frames();

    let link = Link::new();
    // Main loop
    pollster::block_on(run(link, clip, files, current_index));
}

/// Main entry point for the application
pub async fn run(link: crate::Link, clip: clip::Clip, files: Vec<PathBuf>, current_index: usize) {
    
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Voop Video Player")
        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
        .with_min_inner_size(winit::dpi::LogicalSize::new(640, 360))
        .build(&event_loop)
        .unwrap();
    
    // Create a static reference to the window (required for State lifetime)
    let window: &'static Window = Box::leak(Box::new(window));
    
    let mut app = app::App::new(window, link, clip, files, current_index).await;

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
