# Fullscreen Functionality

This document describes the fullscreen functionality implemented for the Voop Video Player.

## Features

The video player now supports fullscreen mode with the following features:

### Keyboard Controls
- **F11 Key**: Toggle fullscreen mode on/off
- **F Key**: Alternative fullscreen toggle (recommended for macOS)
- **Space Key**: Simple fullscreen toggle
- **Escape Key**: Exit fullscreen mode (only when in fullscreen)

### macOS Note
On macOS, F11 is bound to "Show Desktop" by default, which will move the window instead of toggling fullscreen. Use these alternatives:
1. **F key** - Simple alternative (recommended)
2. **Space key** - Easy to reach
3. Disable macOS F11 binding in System Preferences > Keyboard > Shortcuts > Mission Control
4. The app logs all key presses to help debug which keys are being received

### Fullscreen Types
The implementation uses **Borderless Fullscreen** mode, which:
- Takes up the entire screen
- Hides window decorations (title bar, border)
- Allows for faster switching between applications
- Works well on macOS and other platforms

## Implementation Details

### Key Components

1. **State Management** (`state.rs`):
   - `is_fullscreen: bool` - Tracks current fullscreen state
   - `toggle_fullscreen()` - Toggles between fullscreen and windowed mode
   - `enter_fullscreen()` - Explicitly enters fullscreen mode
   - `exit_fullscreen()` - Explicitly exits fullscreen mode

2. **Input Handling**:
   - F11 key handling for toggle functionality
   - Escape key handling for exiting fullscreen
   - Input events are processed before other window events

3. **Window Configuration**:
   - Default window size: 1280x720
   - Minimum window size: 640x360
   - Window title: "Voop Video Player"

### Usage

When running the video player:

```bash
cargo run -- your_video_file.mp4
```

- Press **F11** to enter fullscreen mode
- Press **F11** again or **Escape** to exit fullscreen mode
- The video will automatically resize to fit the screen while maintaining aspect ratio

### Platform Compatibility

This implementation is designed to work across platforms:
- **macOS**: Uses native fullscreen APIs
- **Linux**: Works with X11 and Wayland
- **Windows**: Uses Windows fullscreen APIs

The `winit` library handles platform-specific differences automatically.

### Technical Notes

- The fullscreen state is tracked internally to prevent unnecessary API calls
- Logging is included for debugging fullscreen transitions
- The implementation uses `Fullscreen::Borderless(None)` which automatically selects the current monitor
- Surface reconfiguration happens automatically during fullscreen transitions through the existing resize handling
