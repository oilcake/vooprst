mod clip;
use rusty_link::{AblLink, SessionState};
use std::io::{self, Write};

fn main() {
    let filename = std::env::args().nth(1).expect("Please provide a video filename");
    println!("Opening file: {}\n", filename);

    // Link
    let link = AblLink::new(120.0);
    link.enable(true);
    let mut state = SessionState::new();
    let quantum = 1.0;

    // Video setup
    let mut clip = clip::Clip::new(&filename).unwrap();

    // Main loop
    loop {

        link.capture_app_session_state(&mut state);
        let now = link.clock_micros();

        let beat = state.beat_at_time(now, quantum);
        let phase = state.phase_at_time(now, quantum);

        if let Ok(_frame) = clip.play_video_at_position(phase as f32) {
            println!("\rBeat: {:.2}, Phase: {:.2}", beat, phase);
        }

        print!("\rBeat: {:.2}, Phase: {:.2}", beat, phase);
        let _ = io::stdout().flush();

        std::thread::sleep(std::time::Duration::from_millis(15));
    }
}

