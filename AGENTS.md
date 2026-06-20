# Voop — Ableton Link Video Looper

Early-stage video installation/looping tool for touring bands and experimenting artists.
Video player synced to Ableton Link tempo grid.

## Stack

- **Language:** Rust (edition 2021)
- **GPU:** wgpu (Vulkan/Metal/DX12)
- **Video decode:** ffmpeg-next 6.x
- **Windowing:** winit 0.29
- **Sync:** rusty_link (Ableton Link bindings)
- **Channels:** crossbeam-channel (bounded frame queue, unbounded commands)

## Workspace crates

| Crate       | Purpose                                                                   |
| ----------- | ------------------------------------------------------------------------- |
| `app`       | Binary entrypoint, winit event loop, keyboard dispatch                    |
| `clip`      | FFmpeg video decode + whole-file frame cache (`Clip`, `Decoder`)          |
| `render`    | wgpu pipeline: YUV420P → RGBA16Float compute shader, fullscreen quad draw |
| `transport` | Ableton Link session: beat/phase tracking                                 |

## Architecture

```
[main thread]                    [decoder thread]
  App ──command channel──→       Decoder
    │                              │
    │                            Clip (all frames in RAM)
    │                              │
  State ←──frame channel──        Link.phase → frame position
    │
  wgpu surface / render
```

- Decoder loop: `Link::update_phase_and_beat()` → `Clip::play_video_at_position(phase)` → push frame to bounded channel
- App loop: pull latest frame from channel → YUV→RGBA compute shader → fullscreen quad render
- File switching: `ArrowLeft`/`ArrowRight` send `DecoderCommand` via unbounded channel
- `Clip` decodes all frames into `Vec<Video>` on load (naive, RAM-heavy)

## Current limitations

- Only YUV420P pixel format supported
- Entire video cached in RAM (no streaming decode)
- Single clip at a time, no layering/blending
- No UI beyond fullscreen quad
- No clip launch quantization to Link grid (phase directly maps to position)

## Build & run

```bash
just run                          # uses samples/ dir
PATH_TO_VIDEO=/my/videos just run # custom video folder
```

Requires ffmpeg dev libraries installed on system.

## Key files

| File                              | Role                                                      |
| --------------------------------- | --------------------------------------------------------- |
| `app/src/main.rs`                 | Entrypoint, video-file discovery, thread spawn            |
| `app/src/app.rs`                  | Event loop, keyboard dispatch, frame limiter, cursor hide |
| `clip/src/clip.rs`                | FFmpeg open, decode, full-frame cache                     |
| `clip/src/decoder.rs`             | Decoder loop, Link sync, command handling                 |
| `render/src/state.rs`             | wgpu init, YUV upload, compute dispatch, render pass      |
| `render/src/compute_converter.rs` | YUV→RGBA compute shader pipeline                          |
| `render/src/yuv.rs`               | YUV420P data layout helpers                               |
| `transport/src/link.rs`           | Ableton Link wrapper (beat/phase)                         |