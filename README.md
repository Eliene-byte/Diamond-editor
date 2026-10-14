# DaVinci Clone

A lightweight, professional video editor clone of DaVinci Resolve written in Rust. Target: **45MB binary**, **60% feature parity** with Edit page focus.

## Features

### Edit Page (Primary Focus - 60% Coverage)
- **Timeline Editing**: Multi-track video/audio, ripple/roll/slip/slide edits
- **Professional Trim**: Frame-accurate trimming with keyboard shortcuts
- **Transitions**: Cross dissolve, wipes, slides, pushes
- **Effects Stack**: Transform, crop, opacity, blend modes, speed, color correction
- **Audio Mixing**: Volume, pan, EQ, dynamics, sends/buses
- **Markers**: Chapter, note, todo, sync points
- **Proxy Workflow**: Auto proxy generation, smart proxy rendering
- **Waveforms**: Real-time audio waveform display

### Color Page
- **Node Graph**: Serial, parallel, layer, outside nodes
- **Primary Grading**: Lift/Gamma/Gain, Offset/Contrast/Pivot, Temp/Tint
- **Log Wheels**: Shadows/Midtones/Highlights with range control
- **Curves**: Custom RGB + Luma curves
- **Hue vs Hue/Sat/Lum**: 6-vector secondary grading
- **Qualifiers**: HSL qualification with softness
- **Windows**: Power windows with tracking
- **OpenFX**: Plugin support
- **Scopes**: Waveform, Parade, Vectorscope, Histogram (GPU)

### Fusion Page
- **Node-based Compositing**: 3D workspace, particles, 3D camera
- **Keying**: Delta keyer, chroma key, luma key
- **Tracking**: Planar tracker, camera tracker
- **Rotoscoping**: B-spline, bezier shapes
- **3D**: Camera projection, 3D text, displacement

### Fairlight Page
- **Mixer**: Full channel strips with EQ, dynamics, inserts, sends
- **Automation**: Read/Write/Touch/Latch modes
- **Metering**: True peak, K-weighted, EBU R128
- **ADR**: Automatic dialogue replacement tools
- **Sound Library**: Built-in effects

### Deliver Page
- **Render Queue**: Multiple simultaneous renders
- **Presets**: YouTube, Vimeo, ProRes, DNxHR, H.264/HEVC
- **Custom Codecs**: FFmpeg-based encoding
- **Burn-ins**: Timecode, watermark, filename

## Architecture

```
src/
├── core/           # Types, time, color, math, IDs, config, errors
├── media/          # Scanner, decoder, proxy, thumbnail, waveform, cache, pool
├── timeline/       # Tracks, clips, edit ops, playhead, selection, undo, markers, transitions
├── effects/        # Video, audio, transitions, generators
├── gpu/            # Renderer, pipelines, textures, buffers, color, scopes, compute, playback
├── audio/          # DSP, engine, mixer
├── ui/             # Widgets, layout, theme, pages, viewer, timeline, media pool, scopes, mixer, inspector, node graph
└── main.rs         # Entry point
```

## Requirements

- **Rust**: 1.75+ (MSRV)
- **GPU**: Vulkan 1.2+ / Metal / DirectX 12
- **FFmpeg**: 6.x (for codec support)
- **RAM**: 8GB minimum, 16GB+ recommended
- **Storage**: SSD required for media cache

## Building

```bash
# Minimal build (~45MB)
cargo build --release --features "gpu-vulkan,codec-ffmpeg,audio-cpal"

# With profiling
cargo build --profile release-with-debug --features "gpu-vulkan,codec-ffmpeg,audio-cpal,profiling"

# Linux
sudo apt install libvulkan-dev libasound2-dev libudev-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev pkg-config cmake clang libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libswresample-dev libavfilter-dev libsdl2-dev

# macOS
brew install vulkan-sdk cmake pkg-config ffmpeg sdl2

# Windows
choco install vulkan-sdk cmake pkg-config ffmpeg sdl2
```

## Running

```bash
# Run the editor
cargo run --release --features "gpu-vulkan,codec-ffmpeg,audio-cpal"

# Run with custom config
DAVINCI_CLONE_CONFIG=/path/to/config.toml cargo run --release
```

## Configuration

Config file at `~/.config/DaVinciClone/config.toml`:

```toml
[general]
auto_save_interval_sec = 300
max_undo_levels = 100

[ui]
theme = "Dark"
font_size = 13.0
accent_color = "#FF6B00"

[media]
auto_create_proxies = false
proxy_resolution = "Half"

[timeline]
default_video_tracks = 4
default_audio_tracks = 8
snap_enabled = true

[gpu]
backend = "Auto"
vsync = true
```

## Performance Targets

| Metric | Target |
|--------|--------|
| Binary Size | < 45 MB |
| Startup Time | < 2s |
| Timeline Scrub | 60fps @ 4K |
| Playback | 24fps @ 4K (no effects) |
| Color Grade | 30fps @ 4K (10 nodes) |
| Export | Real-time @ 1080p |

## Keyboard Shortcuts (DaVinci Style)

| Action | Shortcut |
|--------|----------|
| Play/Pause | Space |
| Step Frame | ← / → |
| Step 10 Frames | ↑ / ↓ |
| Set In | I |
| Set Out | O |
| Ripple Delete | Shift+Delete |
| Blade | B |
| Select | A |
| Trim | T |
| Zoom | Z |
| Hand | H |
| Undo | Ctrl+Z |
| Redo | Ctrl+Shift+Z |

## License

MIT License - See [LICENSE](LICENSE) for details.

## Contributing

1. Fork the repository
2. Create feature branch
3. Make changes with tests
4. Run `cargo fmt && cargo clippy`
5. Submit PR

## Roadmap

- [ ] **v0.1**: Edit page complete (current)
- [ ] **v0.2**: Color page + GPU scopes
- [ ] **v0.3**: Fusion page basic nodes
- [ ] **v0.4**: Fairlight mixer + automation
- [ ] **v0.5**: Deliver page + render queue
- [ ] **v1.0**: Full 60% parity, plugin API
- [ ] **v2.0**: Collaboration, cloud render, AI tools

## Acknowledgments

- [wgpu](https://github.com/gfx-rs/wgpu) - Cross-platform GPU
- [FFmpeg](https://ffmpeg.org/) - Media decoding/encoding
- [cpal](https://github.com/RustAudio/cpal) - Audio I/O
- [parking_lot](https://github.com/Amanieu/parking_lot) - Fast synchronization
- [slotmap](https://github.com/orium/slotmap) - Entity management
- [glam](https://github.com/bitshifter/glam-rs) - Math library