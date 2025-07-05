mod clip;
mod run;
mod state;
mod vertex;
mod link;

use link::Link;

use ffmpeg_next as ffmpeg;

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
