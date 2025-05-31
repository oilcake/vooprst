mod clip;
mod display;
use rusty_link::{AblLink, SessionState};
use std::io::{self, Write};
use sdl2::{event::Event, keyboard::Keycode};

fn main() {
    let filename = std::env::args().nth(1).expect("Please provide a video filename");
    println!("Opening file: {}\n", filename);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    // Link
    let link = AblLink::new(120.0);
    link.enable(true);
    let mut state = SessionState::new();
    let quantum = 1.0;

    // Video setup
    let mut clip = clip::Clip::new(&filename).unwrap();
    let mut display = display::SdlVideoDisplay::new(&video_subsystem, clip.size.width(), clip.size.height());

    // Main loop
    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                _ => {}
            }
        }

        link.capture_app_session_state(&mut state);
        let now = link.clock_micros();

        let beat = state.beat_at_time(now, quantum);
        let phase = state.phase_at_time(now, quantum);

        if let Ok(frame) = clip.play_video_at_position(phase as f32) {
            display.display_frame(&frame);
        }

        print!("\rBeat: {:.2}, Phase: {:.2}", beat, phase);
        let _ = io::stdout().flush();

        std::thread::sleep(std::time::Duration::from_millis(15));
    }
}

