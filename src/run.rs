use crate::state::State;
use crate::clip::Clip;
use std::time::{Duration, Instant};
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

pub async fn run(mut link: crate::Link, mut clip: Clip) {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = State::new(&window).await;
    let mut surface_configured = false;

    // Frame rate limiting
    const TARGET_FPS: u32 = 60;
    let frame_duration = Duration::from_secs_f64(1.0 / TARGET_FPS as f64);
    let mut last_frame_time = Instant::now();

    log::info!("Starting render loop with {} FPS target", TARGET_FPS);

    // Request initial redraw
    state.window().request_redraw();

    // run()
    let _ = event_loop.run(move |event, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    // UPDATED!
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => {
                            surface_configured = true;
                            state.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            if !surface_configured {
                                return;
                            }

                            let now = Instant::now();
                            let elapsed = now.duration_since(last_frame_time);

                            if elapsed >= frame_duration {
                                link.update_phase_and_beat();
                                // TODO: Somehow pass it to texture
                                let frame = clip.play_video_at_position(link.phase as f32).unwrap();

                                state.update();
                                match state.render() {
                                    Ok(_) => {}
                                    // Reconfigure the surface if it's lost or outdated
                                    Err(
                                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                    ) => state.resize(state.size),
                                    // The system is out of memory, we should probably quit
                                    Err(
                                        wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other,
                                    ) => {
                                        log::error!("OutOfMemory");
                                        control_flow.exit();
                                    }

                                    // This happens when the a frame takes too long to present
                                    Err(wgpu::SurfaceError::Timeout) => {
                                        log::warn!("Surface timeout")
                                    }
                                }
                                last_frame_time = now;
                            }

                            // Request next frame
                            state.window().request_redraw();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}
