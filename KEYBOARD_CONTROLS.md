# Keyboard Controls

This document describes all keyboard controls available in the Voop Video Player.

## Control Architecture

The video player uses a two-tier keyboard handling system:

1. **App-level controls** (`app.rs`) - Handle video navigation and app-wide functionality
2. **State-level controls** (`state.rs`) - Handle window management and fullscreen

## Key Bindings

### 🎬 Video Navigation (App-level)
| Key             | Action         | Description                                |
| --------------- | -------------- | -------------------------------------------|
| `←` Left Arrow  | Previous clip  | Obviously plays previous clip from the set |
| `→` Right Arrow | Next Clip      | Quite the same with next                   |

### 🖥️ Window & Fullscreen (State-level)
| Key      | Action            | Description                                           |
| -------- | ----------------- | ----------------------------------------------------- |
| `F`      | Alt Fullscreen    | Alternative fullscreen toggle (recommended for macOS) |
| `Escape` | Exit Fullscreen   | Exit fullscreen mode only (when in fullscreen)        |


## Event Flow

```
Keyboard Press
    ↓
App-level Check (handle_window_event)
    ├─ Arrow Keys → Video Navigation
    └─ Other Keys ↓
State-level Check (input)
    ├─ F → Fullscreen Toggle
    ├─ Escape → Exit Fullscreen
    └─ Unhandled → Ignored
```

## Implementation Notes

### Key Detection
- Uses `PhysicalKey::Code` for consistent cross-platform behavior
- Only responds to `ElementState::Pressed` events (ignores key releases)
- App-level keys return early to prevent passing to State-level
