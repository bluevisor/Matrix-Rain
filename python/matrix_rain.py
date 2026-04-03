#!/usr/bin/env python3
# ╔════════════════════════════════════════════╗
# ║   __  __   _  _____ ___ _____  __          ║
# ║  |  \/  | /_\|_   _| _ \_ _\ \/ /          ║
# ║  | |\/| |/ _ \ | | |   /| | >  <           ║
# ║  |_|  |_/_/ \_\|_| |_|_\___/_/\_\          ║
# ║   ___    _   ___ _  _                      ║
# ║  | _ \  /_\ |_ _| \| |                     ║
# ║  |   / / _ \ | || .` |                     ║
# ║  |_|_\/_/ \_\___|_|\_|        v0.1.0       ║
# ║────────────────────────────────────────────║
# ║  Terminal digital rain effect in Python.   ║
# ║  Katakana streams with gradient fading,    ║
# ║  glitch mutations, and color themes.       ║
# ║                                            ║
# ║  Author: John Zheng                        ║
# ║  Built with: Gemini & Claude               ║
# ╚════════════════════════════════════════════╝

import curses
import random
import sys

# --- Configuration ---
MIN_STREAM_LENGTH = 5
MAX_STREAM_LENGTH = 42
STREAM_SPEED_MIN = 1
STREAM_SPEED_MAX = 8
FRAME_DELAY = 0.028          # ~35 FPS
STREAM_DENSITY = 0.55        # Chance for a column to have an active stream
FADE_LENGTH = 8              # Gradient tail length
GLITCH_RATE = 0.005          # Chance a body char mutates per frame
GLITCH_DURATION_MIN = 15     # Minimum frames a glitched char persists
GLITCH_DURATION_MAX = 40     # Maximum frames a glitched char persists
SPARKLE_RATE = 0.0004        # Chance any drawn char briefly flashes white
SPAWN_BURST_CHANCE = 0.0001  # Chance per frame of a burst of new streams
SPAWN_BURST_SIZE = (3, 15)   # Range of streams added in a burst

# Character sets
KATAKANA = [chr(i) for i in range(0xFF61, 0xFF9F)]
DIGITS = [str(i) for i in range(10)]
SYMBOLS = list("=*+-<>|~^")
LATIN = list("abcdefghjkmnoprstuvwxz")
CHAR_SET = KATAKANA + DIGITS + SYMBOLS
CHAR_SET_WITH_LATIN = KATAKANA + DIGITS + SYMBOLS + LATIN

# Color pair indices
PAIR_BRIGHT = 1
PAIR_WHITE = 2
PAIR_DIM = 3
PAIR_BODY = 4
PAIR_FADE1 = 5
PAIR_FADE2 = 6
PAIR_FADE3 = 7
PAIR_MENU_BORDER = 8
PAIR_MENU_TEXT = 9
PAIR_MENU_HIGHLIGHT = 10
PAIR_HEAD_GLOW = 12
PAIR_MENU_VALUE = 11

# Color themes: name -> (head_glow, bright, med_bright, medium, dim, very_dim, near_black, faintest)
# Each tuple is (R, G, B) in curses 0-1000 scale
# head_glow sits between white and bright — a pastel/washed version of the theme
COLOR_THEMES = {
    "Green": [
        (500, 1000, 500), (0, 1000, 0), (0, 800, 0), (0, 600, 100),
        (0, 420, 80), (0, 260, 60), (0, 140, 40), (0, 80, 20),
    ],
    "Amber": [
        (1000, 900, 500), (1000, 750, 0), (800, 580, 0), (600, 420, 0),
        (420, 280, 0), (260, 170, 0), (140, 90, 0), (80, 50, 0),
    ],
    "Cyan": [
        (500, 1000, 1000), (0, 1000, 1000), (0, 800, 800), (0, 600, 650),
        (0, 420, 450), (0, 260, 280), (0, 140, 150), (0, 80, 85),
    ],
    "Red": [
        (1000, 500, 500), (1000, 0, 0), (800, 0, 0), (650, 0, 80),
        (450, 0, 60), (280, 0, 40), (150, 0, 20), (85, 0, 10),
    ],
    "Blue": [
        (600, 700, 1000), (200, 400, 1000), (150, 300, 800), (100, 250, 600),
        (60, 170, 420), (30, 100, 260), (15, 50, 140), (8, 25, 80),
    ],
    "Purple": [
        (850, 500, 1000), (700, 0, 1000), (550, 0, 800), (420, 0, 600),
        (290, 0, 420), (180, 0, 260), (95, 0, 140), (55, 0, 80),
    ],
    "Pink": [
        (1000, 600, 800), (1000, 200, 600), (800, 150, 470), (600, 100, 360),
        (420, 70, 250), (260, 40, 150), (140, 20, 80), (80, 10, 45),
    ],
    "White": [
        (1000, 1000, 1000), (1000, 1000, 1000), (800, 800, 800), (600, 600, 600),
        (420, 420, 420), (260, 260, 260), (140, 140, 140), (80, 80, 80),
    ],
}

THEME_NAMES = list(COLOR_THEMES.keys())

# Speed presets: name -> frame_delay
SPEED_PRESETS = [
    ("Slow", 0.056),
    ("Normal", 0.028),
    ("Fast", 0.014),
    ("Ludicrous", 0.007),
]

# Density presets
DENSITY_PRESETS = [
    ("Sparse", 0.20),
    ("Normal", 0.55),
    ("Dense", 0.92),
    ("Downpour", 1.84),
]

_bg = curses.COLOR_BLACK  # module-level, set during init
_use_256 = False


def _apply_theme(theme_name):
    """Apply a color theme by updating color definitions."""
    global _bg
    colors = COLOR_THEMES[theme_name]
    if _use_256:
        # colors[0]=head_glow, [1]=bright, [2]=med_bright, [3]=medium,
        # [4]=dim, [5]=very_dim, [6]=near_black, [7]=faintest
        for i, (r, g, b) in enumerate(colors):
            curses.init_color(20 + i, r, g, b)
        curses.init_pair(PAIR_HEAD_GLOW, 20, _bg)  # color 20 = head_glow
        curses.init_pair(PAIR_BRIGHT, 21, _bg)      # color 21 = bright
        curses.init_pair(PAIR_BODY, 22, _bg)         # color 22 = med_bright
        curses.init_pair(PAIR_FADE1, 23, _bg)        # color 23 = medium
        curses.init_pair(PAIR_FADE2, 24, _bg)        # color 24 = dim
        curses.init_pair(PAIR_DIM, 25, _bg)          # color 25 = very_dim
        curses.init_pair(PAIR_FADE3, 26, _bg)        # color 26 = near_black


def _setup_colors():
    """Initialize color pairs with multiple shades for gradient."""
    global _bg, _use_256
    curses.start_color()
    if not curses.has_colors():
        return False

    try:
        curses.use_default_colors()
        _bg = -1
    except curses.error:
        _bg = curses.COLOR_BLACK

    if curses.can_change_color() and curses.COLORS >= 256:
        _use_256 = True
        _apply_theme("Green")
        curses.init_pair(PAIR_WHITE, curses.COLOR_WHITE, _bg)
        # Menu colors
        curses.init_color(30, 0, 700, 0)
        curses.init_pair(PAIR_MENU_BORDER, 30, _bg)
        curses.init_pair(PAIR_MENU_TEXT, curses.COLOR_WHITE, _bg)
        curses.init_color(31, 0, 0, 0)
        curses.init_color(32, 0, 900, 0)
        curses.init_pair(PAIR_MENU_HIGHLIGHT, 31, 32)
        curses.init_pair(PAIR_MENU_VALUE, 20, _bg)
    else:
        _use_256 = False
        curses.init_pair(PAIR_HEAD_GLOW, curses.COLOR_WHITE, _bg)
        curses.init_pair(PAIR_BRIGHT, curses.COLOR_GREEN, _bg)
        curses.init_pair(PAIR_WHITE, curses.COLOR_WHITE, _bg)
        curses.init_pair(PAIR_DIM, curses.COLOR_GREEN, _bg)
        curses.init_pair(PAIR_BODY, curses.COLOR_GREEN, _bg)
        curses.init_pair(PAIR_FADE1, curses.COLOR_GREEN, _bg)
        curses.init_pair(PAIR_FADE2, curses.COLOR_GREEN, _bg)
        curses.init_pair(PAIR_FADE3, curses.COLOR_GREEN, _bg)
        curses.init_pair(PAIR_MENU_BORDER, curses.COLOR_GREEN, _bg)
        curses.init_pair(PAIR_MENU_TEXT, curses.COLOR_WHITE, _bg)
        curses.init_pair(PAIR_MENU_HIGHLIGHT, curses.COLOR_BLACK, curses.COLOR_GREEN)
        curses.init_pair(PAIR_MENU_VALUE, curses.COLOR_GREEN, _bg)

    return True


class RainStream:
    """A single falling stream of characters with gradient fading."""

    def __init__(self, col, height, width):
        self.col = col
        self.height = height
        self.width = width
        self.max_len = min(MAX_STREAM_LENGTH, height - 1)
        self.min_len = min(MIN_STREAM_LENGTH, self.max_len)
        if self.min_len < 3:
            self.min_len = 3
        if self.max_len < self.min_len:
            self.max_len = self.min_len
        self._reset()

    def _rand_char(self):
        return random.choice(CHAR_SET)

    def _reset(self):
        self.length = random.randint(self.min_len, self.max_len) if self.min_len < self.max_len else self.min_len
        self.y = random.randint(-self.length - 5, -1)
        self.speed = random.randint(STREAM_SPEED_MIN, STREAM_SPEED_MAX)
        self.chars = [self._rand_char() for _ in range(random.randint(1, max(1, self.length // 2)))]
        # Parallel list: frames remaining for each char's glitch lock (0 = normal)
        self.glitch_ttl = [0] * len(self.chars)
        # Original chars saved so glitched ones can revert when TTL expires
        self.original_chars = list(self.chars)
        self.counter = 0
        self.glitch_rate = GLITCH_RATE * random.uniform(0.5, 2.0)

    def update(self, current_height):
        self.height = current_height
        self.max_len = min(MAX_STREAM_LENGTH, self.height - 1)

        self.counter += 1
        if self.counter >= self.speed:
            self.y += 1
            self.counter = 0

            if self.y >= self.height + self.length + 2:
                self._reset()
                return

            new_char = self._rand_char()
            self.chars.insert(0, new_char)
            self.original_chars.insert(0, new_char)
            self.glitch_ttl.insert(0, 0)
            if len(self.chars) > self.length:
                self.chars.pop()
                self.original_chars.pop()
                self.glitch_ttl.pop()

        # Glitch: rare mutations that persist for many frames
        for i in range(2, len(self.chars)):
            if self.glitch_ttl[i] > 0:
                # Glitch still active — count down
                self.glitch_ttl[i] -= 1
                if self.glitch_ttl[i] == 0:
                    # Revert to a fresh random char (simulates "settling")
                    self.chars[i] = self._rand_char()
                    self.original_chars[i] = self.chars[i]
            else:
                # No active glitch — roll for a new one
                if random.random() < self.glitch_rate:
                    self.original_chars[i] = self.chars[i]
                    self.chars[i] = self._rand_char()
                    self.glitch_ttl[i] = random.randint(GLITCH_DURATION_MIN, GLITCH_DURATION_MAX)

    def draw(self, screen):
        """Draw the stream with a multi-shade gradient tail. Returns set of (y, x) drawn."""
        drawn = set()
        stream_len = len(self.chars)
        effective_fade = min(FADE_LENGTH, max(1, stream_len - 2))

        for i, char in enumerate(self.chars):
            cy = self.y - i
            if not (0 <= cy < self.height):
                continue

            try:
                if i == 0:
                    attr = curses.color_pair(PAIR_WHITE) | curses.A_BOLD
                elif i == 1 and stream_len > 1:
                    attr = curses.color_pair(PAIR_HEAD_GLOW) | curses.A_BOLD
                elif i == 2 and stream_len > 2:
                    attr = curses.color_pair(PAIR_BRIGHT) | curses.A_BOLD
                else:
                    dist_from_end = stream_len - 1 - i
                    if dist_from_end >= effective_fade:
                        attr = curses.color_pair(PAIR_BODY) | curses.A_BOLD
                    else:
                        ratio = dist_from_end / max(1, effective_fade)
                        if ratio > 0.7:
                            attr = curses.color_pair(PAIR_BODY)
                        elif ratio > 0.45:
                            attr = curses.color_pair(PAIR_FADE1)
                        elif ratio > 0.25:
                            attr = curses.color_pair(PAIR_FADE2)
                        elif ratio > 0.1:
                            attr = curses.color_pair(PAIR_FADE3) | curses.A_DIM
                        else:
                            attr = curses.color_pair(PAIR_DIM) | curses.A_DIM

                if i > 0 and random.random() < SPARKLE_RATE:
                    attr = curses.color_pair(PAIR_WHITE) | curses.A_BOLD

                screen.addstr(cy, self.col, char, attr)
                drawn.add((cy, self.col))

            except curses.error:
                pass

        return drawn


def _draw_menu(screen, height, width, menu_sel, theme_idx, speed_idx, density_idx):
    """Draw the options menu overlay with btop-style rounded borders."""
    menu_items = [
        ("Color Theme", THEME_NAMES[theme_idx]),
        ("Speed", SPEED_PRESETS[speed_idx][0]),
        ("Density", DENSITY_PRESETS[density_idx][0]),
        ("Resume", ""),
        ("Quit", ""),
    ]

    hint = "\u2191\u2193 navigate  \u2190\u2192 change  \u23ce select  esc close"
    inner_w = max(36, len(hint) + 4)
    box_w = inner_w + 2  # +2 for border chars
    box_h = len(menu_items) + 8  # title, divider, blank, items, blank, hint, border*2
    start_y = max(0, (height - box_h) // 2)
    start_x = max(0, (width - box_w) // 2)

    if start_y + box_h > height or start_x + box_w > width:
        return

    border = curses.color_pair(PAIR_MENU_BORDER) | curses.A_BOLD
    border_dim = curses.color_pair(PAIR_MENU_BORDER)
    text = curses.color_pair(PAIR_MENU_TEXT)
    highlight = curses.color_pair(PAIR_MENU_HIGHLIGHT) | curses.A_BOLD
    hint_attr = curses.color_pair(PAIR_MENU_BORDER)

    # Unicode box-drawing (btop style: rounded corners, thin lines)
    TL, TR, BL, BR = "\u256d", "\u256e", "\u2570", "\u256f"
    H, V = "\u2500", "\u2502"
    LT, RT = "\u251c", "\u2524"  # tee connectors for divider

    try:
        # Clear the box area to black for a clean backdrop
        for row in range(box_h):
            try:
                screen.addstr(start_y + row, start_x, " " * box_w)
            except curses.error:
                pass

        # Top border: ╭──────────────────╮
        screen.addstr(start_y, start_x, TL + H * inner_w + TR, border)

        # Side borders for all inner rows
        for row in range(1, box_h - 1):
            screen.addstr(start_y + row, start_x, V, border)
            screen.addstr(start_y + row, start_x + box_w - 1, V, border)

        # Bottom border: ╰──────────────────╯
        screen.addstr(start_y + box_h - 1, start_x, BL + H * inner_w + BR, border)

        # Title row (centered, bold white)
        title = " OPTIONS "
        tx = start_x + 1 + (inner_w - len(title)) // 2
        screen.addstr(start_y + 1, start_x + 1, " " * inner_w, text)
        screen.addstr(start_y + 1, tx, title, text | curses.A_BOLD)

        # Divider with tee connectors: ├──────────────────┤
        screen.addstr(start_y + 2, start_x, LT + H * inner_w + RT, border)

        # Blank row after divider
        screen.addstr(start_y + 3, start_x + 1, " " * inner_w, text)

        # Menu items
        for i, (label, value) in enumerate(menu_items):
            row_y = start_y + 4 + i
            if i == menu_sel:
                row_str = f"  \u25b8 {label}"
                if value:
                    val_text = f"\u25c2 {value} \u25b8"
                    padding = inner_w - len(row_str) - 2 - len(val_text)
                    row_str += ":" + " " * max(1, padding) + val_text
                row_str = row_str.ljust(inner_w)
                screen.addstr(row_y, start_x + 1, row_str, highlight)
            else:
                row_str = f"    {label}"
                if value:
                    row_str += f":  {value}"
                row_str = row_str.ljust(inner_w)
                screen.addstr(row_y, start_x + 1, row_str, text)

        # Blank row before hint
        blank_y = start_y + 4 + len(menu_items)
        screen.addstr(blank_y, start_x + 1, " " * inner_w, text)

        # Footer hint (centered, dim border color)
        hy = start_y + box_h - 2
        screen.addstr(hy, start_x + 1, " " * inner_w, text)
        hx = start_x + 1 + (inner_w - len(hint)) // 2
        screen.addstr(hy, hx, hint, hint_attr)

    except curses.error:
        pass


def main(screen):
    """Main loop for the Matrix rain effect."""
    curses.curs_set(0)
    screen.nodelay(True)
    screen.timeout(int(FRAME_DELAY * 1000))

    if not _setup_colors():
        raise RuntimeError("Terminal does not support colors.")

    height, width = screen.getmaxyx()

    # Current settings
    theme_idx = 0
    speed_idx = 1       # "Normal"
    density_idx = 1     # "Normal"
    frame_delay = FRAME_DELAY
    density = STREAM_DENSITY
    menu_open = False
    menu_sel = 0

    def make_streams(w, h):
        if h <= 2 or w <= 0:
            return []
        return [RainStream(col, h, w) for col in range(w) if random.random() < density]

    streams = make_streams(width, height)
    frame = 0
    prev_positions = set()

    while True:
        try:
            key = screen.getch()

            if menu_open:
                is_enter = key in (ord('\n'), 10, 13)

                if key == 27 or (is_enter and menu_sel != 4):
                    # ESC or Enter on any non-Quit item = resume
                    menu_open = False
                    screen.erase()
                    prev_positions = set()
                    screen.timeout(int(frame_delay * 1000))
                    continue
                elif key == curses.KEY_UP:
                    menu_sel = (menu_sel - 1) % 5
                elif key == curses.KEY_DOWN:
                    menu_sel = (menu_sel + 1) % 5
                elif is_enter and menu_sel == 4:  # Quit
                    return
                elif key in (curses.KEY_RIGHT, curses.KEY_LEFT):
                    direction = -1 if key == curses.KEY_LEFT else 1
                    if menu_sel == 0:  # Color Theme
                        theme_idx = (theme_idx + direction) % len(THEME_NAMES)
                        _apply_theme(THEME_NAMES[theme_idx])
                    elif menu_sel == 1:  # Speed
                        speed_idx = (speed_idx + direction) % len(SPEED_PRESETS)
                        frame_delay = SPEED_PRESETS[speed_idx][1]
                        screen.timeout(int(frame_delay * 1000))
                    elif menu_sel == 2:  # Density
                        density_idx = (density_idx + direction) % len(DENSITY_PRESETS)
                        density = DENSITY_PRESETS[density_idx][1]
                        streams = make_streams(width, height)
                        screen.erase()
                        prev_positions = set()

                # Redraw rain behind menu (at slower rate)
                cur_positions = set()
                for stream in streams:
                    stream.update(height)
                    cur_positions.update(stream.draw(screen))
                stale = prev_positions - cur_positions
                for y, x in stale:
                    try:
                        screen.addch(y, x, ' ')
                    except curses.error:
                        pass
                prev_positions = cur_positions

                _draw_menu(screen, height, width, menu_sel, theme_idx, speed_idx, density_idx)
                screen.refresh()
                continue

            # --- Normal rain mode ---
            if key in (ord('q'), ord('Q')):
                break
            elif key == 27:  # ESC opens menu
                menu_open = True
                menu_sel = 0
                screen.timeout(50)  # Faster polling for menu responsiveness
                continue
            elif key == curses.KEY_RESIZE:
                new_h, new_w = screen.getmaxyx()
                if new_h != height or new_w != width:
                    height, width = new_h, new_w
                    screen.erase()
                    screen.refresh()
                    prev_positions = set()
                    if height <= 2 or width <= 0:
                        streams = []
                    else:
                        streams = make_streams(width, height)
                continue

            # Update all streams
            cur_positions = set()
            for stream in streams:
                stream.update(height)
                cur_positions.update(stream.draw(screen))

            # Clear only cells that had content last frame but not this frame
            stale = prev_positions - cur_positions
            for y, x in stale:
                try:
                    screen.addch(y, x, ' ')
                except curses.error:
                    pass

            prev_positions = cur_positions

            # Occasional burst: spawn new streams in empty columns
            if random.random() < SPAWN_BURST_CHANCE and width > 0 and height > 2:
                used_cols = {s.col for s in streams}
                free_cols = [c for c in range(width) if c not in used_cols]
                if free_cols:
                    burst = random.randint(*SPAWN_BURST_SIZE)
                    for _ in range(min(burst, len(free_cols))):
                        col = random.choice(free_cols)
                        free_cols.remove(col)
                        streams.append(RainStream(col, height, width))

            # Cleanup: remove streams whose columns are now offscreen after resize
            if streams:
                streams = [s for s in streams if s.col < width]

            screen.refresh()
            frame += 1

        except curses.error:
            curses.endwin()
            print(f"\nCurses error. Terminal may have been resized too quickly.")
            print("Please restart the script.")
            sys.exit(1)
        except KeyboardInterrupt:
            break


if __name__ == "__main__":
    try:
        curses.wrapper(main)
    except Exception as e:
        try:
            curses.endwin()
        except Exception:
            pass
        print(f"\nError: {e}")
    finally:
        print(random.choice([
            "The Matrix has you.",
            "Follow the white rabbit.",
            "There is no spoon.",
            "Wake up, Neo...",
            "Welcome to the real world.",
            "I know kung fu.",
            "Ignorance is bliss.",
            "Free your mind.",
            "Not like this... not like this...",
            "Everything that has a beginning has an end.",
            "The answer is out there, Neo.",
            "You take the red pill, you stay in Wonderland.",
            "You are the one.",
            "Dodge this.",
            "Guns. Lots of guns.",
            "What is the Matrix?",
            "Never send a human to do a machine's job.",
            "You've been living in a dream world, Neo.",
            "Choice. The problem is choice.",
        ]))
