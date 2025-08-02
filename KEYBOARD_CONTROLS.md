# Keyboard Controls

This document describes all keyboard controls available in the Voop Video Player.

## Control Architecture

The video player uses a two-tier keyboard handling system:

1. **App-level controls** (`app.rs`) - Handle video navigation and app-wide functionality
2. **State-level controls** (`state.rs`) - Handle window management and fullscreen

## Key Bindings

### 🎬 Video Navigation (App-level)
| Key | Action | Description |
|-----|--------|-------------|
| `←` Left Arrow | Seek Backwards | Jump backwards by 5% of video length |
| `→` Right Arrow | Seek Forwards | Jump forwards by 5% of video length |
| `Home` | Jump to Start | Seek to beginning of video (0%) |
| `End` | Jump to End | Seek to end of video (100%) |
| `R` | Reset Playback | Return to automatic link-based timing |

### 🖥️ Window & Fullscreen (State-level)
| Key | Action | Description |
|-----|--------|-------------|
| `F11` | Toggle Fullscreen | Enter/exit fullscreen mode |
| `F` | Alt Fullscreen | Alternative fullscreen toggle (recommended for macOS) |
| `Space` | Quick Fullscreen | Simple fullscreen toggle |
| `Escape` | Exit Fullscreen | Exit fullscreen mode only (when in fullscreen) |

## Behavior Details

### Video Navigation
- **Relative Seeking**: Left/Right arrows seek relative to the current position
- **Manual Override**: Using navigation keys switches from automatic link-based playback to manual control
- **Position Clamping**: All seek operations are clamped between 0.0 and 1.0 (0% to 100%)
- **Reset Functionality**: Press `R` to return to automatic playback mode

### Fullscreen Management
- **Cross-platform**: Works on macOS, Linux, and Windows
- **Borderless Mode**: Uses borderless fullscreen for fast Alt+Tab switching
- **State Tracking**: Prevents unnecessary system calls by tracking fullscreen state

## macOS Compatibility

### F11 Key Issue
On macOS, F11 is bound to "Show Desktop" by default, which moves windows instead of toggling fullscreen.

**Solutions:**
1. **Use alternative keys**: `F` or `Space` (recommended)
2. **Disable system binding**: System Preferences > Keyboard > Shortcuts > Mission Control > Show Desktop F11
3. **Check logs**: The app logs all key presses for debugging

## Event Flow

```
Keyboard Press
    ↓
App-level Check (handle_window_event)
    ├─ Arrow Keys → Video Navigation
    ├─ Home/End → Absolute Seeking  
    ├─ R Key → Reset to Auto
    └─ Other Keys ↓
State-level Check (input)
    ├─ F11/F/Space → Fullscreen Toggle
    ├─ Escape → Exit Fullscreen
    └─ Unhandled → Ignored
```

## Implementation Notes

### Key Detection
- Uses `PhysicalKey::Code` for consistent cross-platform behavior
- Only responds to `ElementState::Pressed` events (ignores key releases)
- App-level keys return early to prevent passing to State-level

### Position Management
- `manual_position: Option<f32>` tracks manual seeking state
- `None` = automatic playback, `Some(pos)` = manual position
- Position is used in `get_current_position()` method

### Logging
All key presses are logged for debugging:
```
[INFO] Key pressed: Code(ArrowLeft)
[INFO] Left arrow pressed - seeking backwards
[INFO] Seeking from 0.450 to 0.400 (delta: -0.050)
```

## Usage Examples

### Basic Navigation
```bash
# Start the player
cargo run -- video.mp4

# Navigate video
← ← ←     # Seek backwards (3 × 5% = 15%)
→         # Seek forwards (5%)
Home      # Jump to start
End       # Jump to end
R         # Return to auto-playback
```

### Fullscreen Usage
```bash
# Enter fullscreen
F         # Recommended for macOS
Space     # Alternative
F11       # Standard (may not work on macOS)

# Exit fullscreen
F         # Toggle back
Escape    # Exit only
```

## Extending Functionality

To add new keyboard controls:

1. **App-level controls** - Add to `handle_window_event()` in `app.rs`
2. **State-level controls** - Add to `input()` method in `state.rs`
3. **Remember to update documentation** - Add entries to this file

### Example: Adding Page Up/Down
```rust
// In app.rs handle_window_event()
WindowEvent::KeyboardInput {
    event: KeyEvent {
        physical_key: PhysicalKey::Code(KeyCode::PageUp),
        state: winit::event::ElementState::Pressed,
        ..
    },
    ..
} => {
    self.seek_relative(-0.10); // Seek backwards 10%
    return;
}
```
