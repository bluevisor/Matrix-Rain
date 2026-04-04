// ╔════════════════════════════════════════════╗
// ║   __  __   _  _____ ___ _____  __          ║
// ║  |  \/  | /_\|_   _| _ \_ _\ \/ /          ║
// ║  | |\/| |/ _ \ | | |   /| | >  <           ║
// ║  |_|  |_/_/ \_\|_| |_|_\___/_/\_\          ║
// ║   ___    _   ___ _  _                      ║
// ║  | _ \  /_\ |_ _| \| |                     ║
// ║  |   / / _ \ | || .` |                     ║
// ║  |_|_\/_/ \_\___|_|\_|        v0.1.0       ║
// ║────────────────────────────────────────────║
// ║  Terminal digital rain effect in Rust.     ║
// ║  Katakana streams with gradient fading,    ║
// ║  glitch mutations, and color themes.       ║
// ║                                            ║
// ║  Author: John Zheng                        ║
// ║  Built with: Gemini & Claude               ║
// ╚════════════════════════════════════════════╝

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor, ResetColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::Rng;
use std::io::{self, Write};
use std::time::{Duration, Instant};

// --- Configuration ---
const MIN_STREAM_LENGTH: usize = 5;
const MAX_STREAM_LENGTH: usize = 42;
const STREAM_SPEED_MIN: u32 = 1;
const STREAM_SPEED_MAX: u32 = 8;
const FADE_LENGTH: usize = 8;
const GLITCH_RATE: f64 = 0.005;
const GLITCH_DURATION_MIN: u32 = 15;
const GLITCH_DURATION_MAX: u32 = 40;
const SPARKLE_RATE: f64 = 0.0004;
const SPAWN_BURST_CHANCE: f64 = 0.0001;
const SPAWN_BURST_SIZE: (usize, usize) = (3, 15);

// Character sets
fn katakana_chars() -> Vec<char> {
    (0xFF61..0xFF9Fu32).filter_map(char::from_u32).collect()
}

fn digit_chars() -> Vec<char> {
    "0123456789".chars().collect()
}

fn symbol_chars() -> Vec<char> {
    "=*+-<>|~^".chars().collect()
}

fn char_set() -> Vec<char> {
    let mut set = katakana_chars();
    set.extend(digit_chars());
    set.extend(symbol_chars());
    set
}

// Color themes: (head_glow, bright, med_bright, medium, dim, very_dim, near_black, faintest)
// Each color is (r, g, b) in 0-255 scale
type ThemeColors = [(u8, u8, u8); 8];

const THEMES: &[(&str, ThemeColors)] = &[
    ("Green", [
        (128, 255, 128), (0, 255, 0), (0, 204, 0), (0, 153, 25),
        (0, 107, 20), (0, 66, 15), (0, 36, 10), (0, 20, 5),
    ]),
    ("Amber", [
        (255, 230, 128), (255, 191, 0), (204, 148, 0), (153, 107, 0),
        (107, 71, 0), (66, 43, 0), (36, 23, 0), (20, 13, 0),
    ]),
    ("Cyan", [
        (128, 255, 255), (0, 255, 255), (0, 204, 204), (0, 153, 166),
        (0, 107, 115), (0, 66, 71), (0, 36, 38), (0, 20, 22),
    ]),
    ("Red", [
        (255, 128, 128), (255, 0, 0), (204, 0, 0), (166, 0, 20),
        (115, 0, 15), (71, 0, 10), (38, 0, 5), (22, 0, 3),
    ]),
    ("Blue", [
        (153, 179, 255), (51, 102, 255), (38, 77, 204), (25, 64, 153),
        (15, 43, 107), (8, 25, 66), (4, 13, 36), (2, 6, 20),
    ]),
    ("Purple", [
        (217, 128, 255), (179, 0, 255), (140, 0, 204), (107, 0, 153),
        (74, 0, 107), (46, 0, 66), (24, 0, 36), (14, 0, 20),
    ]),
    ("Pink", [
        (255, 153, 204), (255, 51, 153), (204, 38, 120), (153, 25, 92),
        (107, 18, 64), (66, 10, 38), (36, 5, 20), (20, 3, 11),
    ]),
    ("White", [
        (255, 255, 255), (255, 255, 255), (204, 204, 204), (153, 153, 153),
        (107, 107, 107), (66, 66, 66), (36, 36, 36), (20, 20, 20),
    ]),
];

const SPEED_PRESETS: &[(&str, u64)] = &[
    ("Slow", 56),
    ("Normal", 28),
    ("Fast", 14),
    ("Ludicrous", 7),
];

const DENSITY_PRESETS: &[(&str, f64)] = &[
    ("Sparse", 0.20),
    ("Normal", 0.55),
    ("Dense", 0.92),
    ("Downpour", 1.84),
];

fn theme_color(theme_idx: usize, shade: usize) -> Color {
    let (r, g, b) = THEMES[theme_idx].1[shade];
    Color::Rgb { r, g, b }
}

struct RainStream {
    col: u16,
    height: u16,
    max_len: usize,
    min_len: usize,
    length: usize,
    y: i32,
    speed: u32,
    chars: Vec<char>,
    glitch_ttl: Vec<u32>,
    counter: u32,
    glitch_rate: f64,
    charset: Vec<char>,
}

impl RainStream {
    fn new(col: u16, height: u16, charset: Vec<char>) -> Self {
        let max_len = MAX_STREAM_LENGTH.min(height as usize - 1);
        let min_len = MIN_STREAM_LENGTH.min(max_len).max(3);
        let max_len = max_len.max(min_len);
        let mut s = RainStream {
            col,
            height,
            max_len,
            min_len,
            length: 0,
            y: 0,
            speed: 0,
            chars: Vec::new(),
            glitch_ttl: Vec::new(),
            counter: 0,
            glitch_rate: 0.0,
            charset,
        };
        s.reset();
        s
    }

    fn rand_char(&self) -> char {
        let mut rng = rand::thread_rng();
        self.charset[rng.gen_range(0..self.charset.len())]
    }

    fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        self.length = if self.min_len < self.max_len {
            rng.gen_range(self.min_len..=self.max_len)
        } else {
            self.min_len
        };
        self.y = rng.gen_range(-(self.length as i32) - 5..=-1);
        self.speed = rng.gen_range(STREAM_SPEED_MIN..=STREAM_SPEED_MAX);
        let char_count = rng.gen_range(1..=1.max(self.length / 2));
        self.chars = (0..char_count).map(|_| self.rand_char()).collect();
        self.glitch_ttl = vec![0; self.chars.len()];
        self.counter = 0;
        self.glitch_rate = GLITCH_RATE * rng.gen_range(0.5..2.0);
    }

    fn update(&mut self) {
        let mut rng = rand::thread_rng();
        self.counter += 1;
        if self.counter >= self.speed {
            self.y += 1;
            self.counter = 0;

            if self.y >= self.height as i32 + self.length as i32 + 2 {
                self.reset();
                return;
            }

            let new_char = self.rand_char();
            self.chars.insert(0, new_char);
            self.glitch_ttl.insert(0, 0);
            if self.chars.len() > self.length {
                self.chars.pop();
                self.glitch_ttl.pop();
            }
        }

        // Glitch mutations
        for i in 0..self.chars.len() {
            if self.glitch_ttl[i] > 0 {
                self.glitch_ttl[i] -= 1;
                if self.glitch_ttl[i] == 0 {
                    self.chars[i] = self.rand_char();
                }
            } else if rng.gen::<f64>() < self.glitch_rate {
                self.chars[i] = self.rand_char();
                self.glitch_ttl[i] = rng.gen_range(GLITCH_DURATION_MIN..=GLITCH_DURATION_MAX);
            }
        }
    }

    fn draw(
        &self,
        stdout: &mut io::Stdout,
        theme_idx: usize,
        drawn: &mut Vec<(u16, u16)>,
        exclude: Option<(u16, u16, u16, u16)>, // (y1, x1, y2, x2) region to skip
    ) {
        let mut rng = rand::thread_rng();
        let stream_len = self.chars.len();
        let effective_fade = FADE_LENGTH.min(1.max(stream_len.saturating_sub(2)));

        for (i, &ch) in self.chars.iter().enumerate() {
            let cy = self.y - i as i32;
            if cy < 0 || cy >= self.height as i32 {
                continue;
            }
            let row = cy as u16;

            // Skip cells inside the menu region
            if let Some((y1, x1, y2, x2)) = exclude {
                if row >= y1 && row <= y2 && self.col >= x1 && self.col <= x2 {
                    continue;
                }
            }

            let (color, bold) = if i == 0 {
                (Color::White, true)
            } else if i == 1 && stream_len > 1 {
                (theme_color(theme_idx, 0), true) // head_glow
            } else if i == 2 && stream_len > 2 {
                (theme_color(theme_idx, 1), true) // bright
            } else {
                let dist_from_end = stream_len - 1 - i;
                if dist_from_end >= effective_fade {
                    (theme_color(theme_idx, 2), true) // body bold
                } else {
                    let ratio = dist_from_end as f64 / effective_fade.max(1) as f64;
                    if ratio > 0.7 {
                        (theme_color(theme_idx, 2), false) // body
                    } else if ratio > 0.45 {
                        (theme_color(theme_idx, 3), false) // fade1
                    } else if ratio > 0.25 {
                        (theme_color(theme_idx, 4), false) // fade2
                    } else if ratio > 0.1 {
                        (theme_color(theme_idx, 5), false) // fade3
                    } else {
                        (theme_color(theme_idx, 6), false) // dim
                    }
                }
            };

            // Sparkle
            let (color, bold) = if i > 0 && rng.gen::<f64>() < SPARKLE_RATE {
                (Color::White, true)
            } else {
                (color, bold)
            };

            let _ = execute!(
                stdout,
                MoveTo(self.col, row),
                SetForegroundColor(color),
                SetAttribute(if bold { Attribute::Bold } else { Attribute::NormalIntensity }),
                Print(ch)
            );
            drawn.push((row, self.col));
        }
    }
}

/// Returns the menu bounding box as (y1, x1, y2, x2) or None if it can't fit.
fn menu_bounds(height: u16, width: u16) -> Option<(u16, u16, u16, u16)> {
    let hint = "↑↓ navigate  ←→ change  ⏎ select  esc close";
    let inner_w = 36usize.max(hint.chars().count() + 4);
    let box_w = inner_w + 2;
    let box_h = 5 + 8; // 5 menu items + 8 chrome
    let start_y = (height as usize).saturating_sub(box_h) / 2;
    let start_x = (width as usize).saturating_sub(box_w) / 2;
    if start_y + box_h > height as usize || start_x + box_w > width as usize {
        return None;
    }
    Some((start_y as u16, start_x as u16, (start_y + box_h - 1) as u16, (start_x + box_w - 1) as u16))
}

fn draw_menu(
    stdout: &mut io::Stdout,
    height: u16,
    width: u16,
    menu_sel: usize,
    theme_idx: usize,
    speed_idx: usize,
    density_idx: usize,
) {
    let menu_items: Vec<(&str, String)> = vec![
        ("Color Theme", THEMES[theme_idx].0.to_string()),
        ("Speed", SPEED_PRESETS[speed_idx].0.to_string()),
        ("Density", DENSITY_PRESETS[density_idx].0.to_string()),
        ("Resume", String::new()),
        ("Quit", String::new()),
    ];

    let hint = "↑↓ navigate  ←→ change  ⏎ select  esc close";
    let inner_w = 36usize.max(hint.chars().count() + 4);
    let box_w = inner_w + 2;
    let box_h = menu_items.len() + 8;

    let start_y = (height as usize).saturating_sub(box_h) / 2;
    let start_x = (width as usize).saturating_sub(box_w) / 2;

    if start_y + box_h > height as usize || start_x + box_w > width as usize {
        return;
    }

    let border_color = theme_color(theme_idx, 3);

    // Clear box area
    for row in 0..box_h {
        let _ = execute!(
            stdout,
            MoveTo(start_x as u16, (start_y + row) as u16),
            SetForegroundColor(Color::White),
            Print(" ".repeat(box_w))
        );
    }

    let h_line = "─".repeat(inner_w);

    // Top border
    let _ = execute!(
        stdout,
        MoveTo(start_x as u16, start_y as u16),
        SetForegroundColor(border_color),
        SetAttribute(Attribute::Bold),
        Print(format!("╭{}╮", h_line))
    );

    // Side borders
    for row in 1..box_h - 1 {
        let _ = execute!(
            stdout,
            MoveTo(start_x as u16, (start_y + row) as u16),
            SetForegroundColor(border_color),
            Print("│")
        );
        let _ = execute!(
            stdout,
            MoveTo((start_x + box_w - 1) as u16, (start_y + row) as u16),
            SetForegroundColor(border_color),
            Print("│")
        );
    }

    // Bottom border
    let _ = execute!(
        stdout,
        MoveTo(start_x as u16, (start_y + box_h - 1) as u16),
        SetForegroundColor(border_color),
        SetAttribute(Attribute::Bold),
        Print(format!("╰{}╯", h_line))
    );

    // Title
    let title = " OPTIONS ";
    let tx = start_x + 1 + (inner_w - title.len()) / 2;
    let _ = execute!(
        stdout,
        MoveTo(tx as u16, (start_y + 1) as u16),
        SetForegroundColor(Color::White),
        SetAttribute(Attribute::Bold),
        Print(title)
    );

    // Divider
    let _ = execute!(
        stdout,
        MoveTo(start_x as u16, (start_y + 2) as u16),
        SetForegroundColor(border_color),
        Print(format!("├{}┤", h_line))
    );

    // Menu items
    for (i, (label, value)) in menu_items.iter().enumerate() {
        let row_y = (start_y + 4 + i) as u16;
        if i == menu_sel {
            let row_str = if !value.is_empty() {
                let val_text = format!("◂ {} ▸", value);
                let prefix = format!("  ▸ {}:", label);
                let padding = inner_w.saturating_sub(prefix.chars().count() + val_text.chars().count());
                format!("{}{}{}", prefix, " ".repeat(padding), val_text)
            } else {
                format!("  ▸ {}", label)
            };
            let padded: String = format!("{:<width$}", row_str, width = inner_w);
            let _ = execute!(
                stdout,
                MoveTo((start_x + 1) as u16, row_y),
                SetForegroundColor(Color::Black),
                SetAttribute(Attribute::Bold),
                // Simulate highlight with reverse
                SetAttribute(Attribute::Reverse),
                SetForegroundColor(theme_color(theme_idx, 1)),
                Print(&padded),
                SetAttribute(Attribute::NoReverse)
            );
        } else {
            let row_str = if !value.is_empty() {
                format!("    {}:  {}", label, value)
            } else {
                format!("    {}", label)
            };
            let padded = format!("{:<width$}", row_str, width = inner_w);
            let _ = execute!(
                stdout,
                MoveTo((start_x + 1) as u16, row_y),
                SetForegroundColor(Color::White),
                SetAttribute(Attribute::Reset),
                Print(&padded)
            );
        }
    }

    // Hint
    let hy = (start_y + box_h - 2) as u16;
    let hx = start_x + 1 + (inner_w.saturating_sub(hint.chars().count())) / 2;
    let _ = execute!(
        stdout,
        MoveTo(hx as u16, hy),
        SetForegroundColor(border_color),
        SetAttribute(Attribute::Reset),
        Print(hint)
    );
}

const MATRIX_QUOTES: &[&str] = &[
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
];

fn main() {
    let result = run();
    // Print a random quote on exit
    let mut rng = rand::thread_rng();
    let quote = MATRIX_QUOTES[rng.gen_range(0..MATRIX_QUOTES.len())];
    println!("{}", quote);

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let result = rain_loop(&mut stdout);

    execute!(stdout, Show, ResetColor, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

fn rain_loop(stdout: &mut io::Stdout) -> Result<(), Box<dyn std::error::Error>> {
    let charset = char_set();
    let mut rng = rand::thread_rng();

    let (mut width, mut height) = terminal::size()?;

    let mut theme_idx: usize = 0;
    let mut speed_idx: usize = 1;
    let mut density_idx: usize = 1;
    let mut frame_delay = Duration::from_millis(SPEED_PRESETS[speed_idx].1);
    let mut density = DENSITY_PRESETS[density_idx].1;
    let mut menu_open = false;
    let mut menu_sel: usize = 0;

    let make_streams = |w: u16, h: u16, density: f64, charset: &Vec<char>| -> Vec<RainStream> {
        if h <= 2 || w == 0 {
            return Vec::new();
        }
        let mut rng = rand::thread_rng();
        (0..w)
            .filter(|_| rng.gen::<f64>() < density)
            .map(|col| RainStream::new(col, h, charset.clone()))
            .collect()
    };

    let mut streams = make_streams(width, height, density, &charset);
    let mut prev_positions: Vec<(u16, u16)> = Vec::new();

    // Clear screen
    execute!(stdout, Clear(ClearType::All))?;

    loop {
        let _frame_start = Instant::now();
        let poll_dur = if menu_open {
            Duration::from_millis(50)
        } else {
            frame_delay
        };

        if event::poll(poll_dur)? {
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                if menu_open {
                    match code {
                        KeyCode::Esc => {
                            menu_open = false;
                            execute!(stdout, Clear(ClearType::All))?;
                            prev_positions.clear();
                            continue;
                        }
                        KeyCode::Enter => {
                            if menu_sel == 3 {
                                // Resume
                                menu_open = false;
                                execute!(stdout, Clear(ClearType::All))?;
                                prev_positions.clear();
                                continue;
                            } else if menu_sel == 4 {
                                // Quit
                                return Ok(());
                            }
                        }
                        KeyCode::Up => {
                            menu_sel = (menu_sel + 4) % 5;
                        }
                        KeyCode::Down => {
                            menu_sel = (menu_sel + 1) % 5;
                        }
                        KeyCode::Left | KeyCode::Right => {
                            let dir: isize = if code == KeyCode::Left { -1 } else { 1 };
                            match menu_sel {
                                0 => {
                                    theme_idx = ((theme_idx as isize + dir).rem_euclid(THEMES.len() as isize)) as usize;
                                }
                                1 => {
                                    speed_idx = ((speed_idx as isize + dir).rem_euclid(SPEED_PRESETS.len() as isize)) as usize;
                                    frame_delay = Duration::from_millis(SPEED_PRESETS[speed_idx].1);
                                }
                                2 => {
                                    density_idx = ((density_idx as isize + dir).rem_euclid(DENSITY_PRESETS.len() as isize)) as usize;
                                    density = DENSITY_PRESETS[density_idx].1;
                                    streams = make_streams(width, height, density, &charset);
                                    execute!(stdout, Clear(ClearType::All))?;
                                    prev_positions.clear();
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                } else {
                    match code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
                        KeyCode::Esc => {
                            menu_open = true;
                            menu_sel = 0;
                            continue;
                        }
                        _ => {}
                    }
                }
            } else if let Event::Resize(w, h) = event::read()? {
                if w != width || h != height {
                    width = w;
                    height = h;
                    execute!(stdout, Clear(ClearType::All))?;
                    prev_positions.clear();
                    streams = make_streams(width, height, density, &charset);
                }
                continue;
            }
        }

        // Check for resize
        let (w, h) = terminal::size()?;
        if w != width || h != height {
            width = w;
            height = h;
            execute!(stdout, Clear(ClearType::All))?;
            prev_positions.clear();
            streams = make_streams(width, height, density, &charset);
        }

        // Update and draw streams
        let exclude = if menu_open { menu_bounds(height, width) } else { None };
        let mut cur_positions: Vec<(u16, u16)> = Vec::new();
        for stream in &mut streams {
            stream.height = height;
            stream.max_len = MAX_STREAM_LENGTH.min(height as usize - 1);
            stream.update();
            stream.draw(stdout, theme_idx, &mut cur_positions, exclude);
        }

        // Clear stale positions (skip menu region)
        for &(row, col) in &prev_positions {
            if let Some((y1, x1, y2, x2)) = exclude {
                if row >= y1 && row <= y2 && col >= x1 && col <= x2 {
                    continue;
                }
            }
            if !cur_positions.contains(&(row, col)) {
                let _ = execute!(
                    stdout,
                    MoveTo(col, row),
                    ResetColor,
                    SetAttribute(Attribute::Reset),
                    Print(' ')
                );
            }
        }

        prev_positions = cur_positions;

        // Spawn burst
        if rng.gen::<f64>() < SPAWN_BURST_CHANCE && width > 0 && height > 2 {
            let used_cols: std::collections::HashSet<u16> = streams.iter().map(|s| s.col).collect();
            let mut free_cols: Vec<u16> = (0..width).filter(|c| !used_cols.contains(c)).collect();
            if !free_cols.is_empty() {
                let burst = rng.gen_range(SPAWN_BURST_SIZE.0..=SPAWN_BURST_SIZE.1);
                for _ in 0..burst.min(free_cols.len()) {
                    let idx = rng.gen_range(0..free_cols.len());
                    let col = free_cols.swap_remove(idx);
                    streams.push(RainStream::new(col, height, charset.clone()));
                }
            }
        }

        // Remove offscreen streams
        streams.retain(|s| s.col < width);

        if menu_open {
            draw_menu(stdout, height, width, menu_sel, theme_idx, speed_idx, density_idx);
        }

        stdout.flush()?;
    }
}
