# Matrix Rain

A Matrix digital rain effect with two renderers: a GPU-accelerated version with cinematic depth-of-field, and a classic terminal version.

By John Zheng (with Gemini & Claude)

## GPU Version (Rust/wgpu)

Real-time 3D rain with bloom, bokeh depth-of-field, and tone mapping.

### Features

- Instanced glyph rendering with texture atlas
- Multi-pass Gaussian depth-of-field blur (f/2.8 bokeh)
- Bloom with bright extraction and separable blur
- Reinhard tone mapping and vignette
- Katakana + digit character set
- Multiple depth layers with parallax

### Requirements

- Rust toolchain
- macOS, Linux, or Windows with Vulkan/Metal/DX12 support

### Build & Run

```bash
cargo run --bin matrix-rain-gpu --release
```

Or download a pre-built binary from [Releases](https://github.com/bluevisor/Matrix-Rain_Terminal/releases).

### Controls

| Key | Action |
|-----|--------|
| `+` / `-` | Zoom in / out |
| `ESC` / `q` | Quit |

## Terminal Version (Python)

Classic terminal rain using `curses` with 256-color gradients.

### Features

- Smooth falling character streams with multi-shade gradient tails
- Glitch mutations and white sparkle flashes
- Randomized stream speeds, lengths, and burst spawns
- In-app options menu (press `ESC`) to configure color theme, speed, and density
- Handles terminal resizing on the fly

### Requirements

- Python 3
- A terminal with color support (256-color recommended)

### Usage

```bash
python3 python/matrix_rain.py
```

### Controls

| Key | Action |
|-----|--------|
| `ESC` | Open / close options menu |
| `q` / `Q` | Quit |
| `↑` `↓` | Navigate menu |
| `←` `→` | Cycle menu values |
| `Enter` | Select menu item |
