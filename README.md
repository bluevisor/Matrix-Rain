# Matrix Rain - Terminal Edition

A Matrix digital rain effect for the terminal, written in Python with `curses`.

By John Zheng (with Gemini & Claude)

## Features

- Smooth falling character streams with multi-shade gradient tails
- Katakana, digits, and symbol character sets
- Glitch mutations and white sparkle flashes
- Randomized stream speeds, lengths, and burst spawns
- In-app options menu (press `ESC`) to configure:
  - **Color Theme** — Green, Amber, Cyan, Red, Blue, Purple, Pink, White
  - **Speed** — Slow, Normal, Fast, Ludicrous
  - **Density** — Sparse, Normal, Dense, Downpour
- 256-color gradient support with automatic fallback for basic terminals
- Handles terminal resizing on the fly

## Requirements

- Python 3
- A terminal with color support (256-color recommended)

## Usage

```bash
python3 matrix_rain.py
```

## Controls

| Key | Action |
|-----|--------|
| `ESC` | Open / close options menu |
| `q` / `Q` | Quit |
| `↑` `↓` | Navigate menu |
| `←` `→` | Cycle menu values |
| `Enter` | Select menu item |
