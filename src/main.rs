use std::env;
use std::process;
use std::time::Duration;

use ffmpeg_next as ffmpeg; // High-level FFmpeg bindings
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::TextureAccess;

fn main() {
    // Initialize FFmpeg
    if let Err(e) = ffmpeg::init() {
        eprintln!("Failed to initialize FFmpeg: {}", e);
        process::exit(1);
    }

    // Get video filename from the command line
    let filename = env::args().nth(1).expect("Please provide a video filename");
    println!("Opening file: {}", filename);

    // Open the input video file
    let mut ictx = ffmpeg::format::input(&filename).expect("Failed to open input file");

    // Find the best video stream
    let input = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .expect("Could not find a video stream");
    let stream_index = input.index();

    // Create a decoder for the video stream
    let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())
        .expect("Failed to get codec context");
    let mut decoder = context_decoder.decoder().video().expect("Failed to get decoder");

    let width = decoder.width();
    let height = decoder.height();
    println!("Video resolution: {}x{}", width, height);

    // Initialize SDL2 for window and rendering
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // Create SDL2 window and canvas
    let window = video_subsystem
        .window("Simple Video Player", width, height)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    // Create a texture for YUV frames
    let mut texture = texture_creator
        .create_texture(PixelFormatEnum::IYUV, TextureAccess::Streaming, width, height)
        .unwrap();

    // Get SDL2 event pump to handle input and window events
    let mut event_pump = sdl_context.event_pump().unwrap();

    // Create a scaler to convert from input format to YUV420P (what SDL2 understands)
    let mut scaler = ffmpeg::software::scaling::Context::get(
        decoder.format(),
        width,
        height,
        ffmpeg::format::Pixel::YUV420P,
        width,
        height,
        ffmpeg::software::scaling::Flags::BILINEAR,
    )
    .unwrap();

    let mut frame_index = 0;

    // Main decoding loop
    'outer: for (stream, packet) in ictx.packets() {
        // Poll SDL2 events (e.g., quit, escape key)
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    println!("Quitting.");
                    break 'outer;
                },
                _ => {}
            }
        }

        // Only process packets from the video stream
        if stream.index() == stream_index {
            decoder.send_packet(&packet).unwrap();

            let mut decoded = ffmpeg::util::frame::Video::empty();

            // Try to receive all frames for this packet
            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut yuv_frame = ffmpeg::util::frame::Video::empty();

                // Convert to YUV420P
                scaler.run(&decoded, &mut yuv_frame).unwrap();

                // Upload YUV data to the SDL2 texture
                texture
                    .update_yuv(
                        None,
                        yuv_frame.data(0), yuv_frame.stride(0) as usize,
                        yuv_frame.data(1), yuv_frame.stride(1) as usize,
                        yuv_frame.data(2), yuv_frame.stride(2) as usize,
                    )
                    .unwrap();

                // Render the frame
                canvas.clear();
                canvas.copy(&texture, None, None).unwrap();
                canvas.present();

                println!("Displayed frame {}", frame_index);
                frame_index += 1;

                // Delay for roughly 30 FPS
                std::thread::sleep(Duration::from_millis(30));
            }
        }
    }

    // Flush remaining frames from the decoder
    decoder.send_eof().unwrap();
    let mut decoded = ffmpeg::util::frame::Video::empty();
    while decoder.receive_frame(&mut decoded).is_ok() {
        let mut yuv_frame = ffmpeg::util::frame::Video::empty();
        scaler.run(&decoded, &mut yuv_frame).unwrap();
        texture
            .update_yuv(
                None,
                yuv_frame.data(0), yuv_frame.stride(0) as usize,
                yuv_frame.data(1), yuv_frame.stride(1) as usize,
                yuv_frame.data(2), yuv_frame.stride(2) as usize,
            )
            .unwrap();
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
        std::thread::sleep(Duration::from_millis(30));
    }

    println!("Done playing.");
}
