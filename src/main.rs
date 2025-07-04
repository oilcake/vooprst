mod clip;
mod run;
mod state;
mod vertex;

use std::io::{self, Write};

use ffmpeg_next as ffmpeg;
use rusty_link::{AblLink, SessionState};

struct Link {
    link: AblLink,
    quantum: f64,
    beat: f64,
    phase: f64,
    state: SessionState,
}
impl Link {
    fn new() -> Link {
        // Link
        let link = AblLink::new(120.0);
        link.enable(true);
        let state = SessionState::new();
        let quantum = 1.0;
        Link {
            link,
            quantum,
            beat: 0.0,
            phase: 0.0,
            state,
        }
    }
    fn update_phase_and_beat(&mut self) {
        self.link.capture_app_session_state(&mut self.state);
        let now = self.link.clock_micros();

        self.beat = self.state.beat_at_time(now, self.quantum);
        self.phase = self.state.phase_at_time(now, self.quantum);
        print!("\rBeat: {:.2}, Phase: {:.2}", self.beat, self.phase);
        let _ = io::stdout().flush();
    }
}
fn main() {
    ffmpeg::init().unwrap();
    let filename = std::env::args()
        .nth(1)
        .expect("Please provide a video filename");
    println!("Opening file: {}\n", filename);

    // Video setup
    let clip = clip::Clip::new(&filename).unwrap();

    let link = Link::new();
    // Main loop
    pollster::block_on(run::run(link, clip));
}
