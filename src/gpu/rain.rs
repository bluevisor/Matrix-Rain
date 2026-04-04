use rand::Rng;

pub fn char_set() -> Vec<char> {
    // Matrix Code NFI font: lowercase + digits + symbols, no uppercase, no space
    let mut set: Vec<char> = ('a'..='z').collect();
    set.extend('0'..='9');
    set.extend("=*+-<>|~^!#$%&_@".chars());
    set.retain(|&c| c != ' ');
    set
}

// Camera constants (must match app.rs / camera.rs)
const CAMERA_Z: f32 = 5.0;
const FOV_HALF_TAN: f32 = 0.57735; // tan(30°), FOV=60°
const ROW_HEIGHT: f32 = 0.5;
const LAYER_SPACING: f32 = 8.0;

/// Compute the visible row range [top, bottom] for a given layer depth.
fn layer_vis_rows(layer: usize, num_rows: usize) -> (i32, i32) {
    let dist = CAMERA_Z + layer as f32 * LAYER_SPACING;
    let half = (FOV_HALF_TAN * dist / ROW_HEIGHT).ceil() as i32;
    let center = num_rows as i32 / 2;
    (center - half, center + half)
}

const MIN_STREAM_LENGTH: usize = 5;
const MAX_STREAM_LENGTH: usize = 42;
const STREAM_SPEED_MIN: f32 = 0.5;
const STREAM_SPEED_MAX: f32 = 4.0;
const GLITCH_RATE: f64 = 0.003; // per-stream chance, applied to head chars only
const GLITCH_DURATION_MIN: u32 = 15;
const GLITCH_DURATION_MAX: u32 = 40;
const SPARKLE_RATE: f64 = 0.00003;
const FADE_LENGTH: usize = 8;

pub struct Stream {
    pub col: usize,
    pub layer: usize,
    pub y: f32,           // continuous vertical position
    pub speed: f32,
    pub length: usize,
    pub chars: Vec<usize>, // indices into char_set
    pub glitch_ttl: Vec<u32>,
    pub glitch_rate: f64,
    active: bool,
}

impl Stream {
    fn make(col: usize, layer: usize, num_rows: usize, charset_len: usize, y: f32) -> Self {
        let mut rng = rand::thread_rng();
        let max_len = MAX_STREAM_LENGTH.min(num_rows);
        let min_len = MIN_STREAM_LENGTH.min(max_len).max(3);
        let length = rng.gen_range(min_len..=max_len);
        let layer_factor = (1.0 - layer as f32 * 0.05).max(0.4);
        let speed = rng.gen_range(STREAM_SPEED_MIN..=STREAM_SPEED_MAX) * layer_factor;
        let chars: Vec<usize> = (0..length).map(|_| rng.gen_range(0..charset_len)).collect();
        let glitch_ttl = vec![0u32; length];
        Stream {
            col, layer, y, speed, length, chars, glitch_ttl,
            glitch_rate: GLITCH_RATE * rng.gen_range(0.5..2.0),
            active: true,
        }
    }

    /// Spawn from just above the visible top — for ongoing respawn.
    pub fn new(col: usize, layer: usize, num_rows: usize, charset_len: usize) -> Self {
        let mut rng = rand::thread_rng();
        let (vis_top, _) = layer_vis_rows(layer, num_rows);
        // Start fully above visible top: head enters from outside screen
        let y = rng.gen_range((vis_top - MAX_STREAM_LENGTH as i32 - 3) as f32..vis_top as f32);
        Self::make(col, layer, num_rows, charset_len, y)
    }

    /// Spawn at a random position inside the visible range — for initial fill.
    pub fn new_scattered(col: usize, layer: usize, num_rows: usize, charset_len: usize) -> Self {
        let mut rng = rand::thread_rng();
        let (vis_top, vis_bottom) = layer_vis_rows(layer, num_rows);
        let y = rng.gen_range(vis_top as f32..vis_bottom as f32);
        Self::make(col, layer, num_rows, charset_len, y)
    }

    pub fn update(&mut self, num_rows: usize, charset_len: usize) {
        let mut rng = rand::thread_rng();

        self.y += self.speed * 0.1;

        // Deactivate once tail has fully exited the visible bottom
        let (_, vis_bottom) = layer_vis_rows(self.layer, num_rows);
        if self.y - self.length as f32 > vis_bottom as f32 + 3.0 {
            self.active = false;
            return;
        }

        // Glitch: occasional mutation on a single head char per stream
        let glitch_window = (self.length / 4).max(3); // top quarter of stream
        for i in 0..glitch_window.min(self.chars.len()) {
            if self.glitch_ttl[i] > 0 {
                self.glitch_ttl[i] -= 1;
            } else if rng.gen::<f64>() < self.glitch_rate {
                self.chars[i] = rng.gen_range(0..charset_len);
                self.glitch_ttl[i] = rng.gen_range(GLITCH_DURATION_MIN..=GLITCH_DURATION_MAX);
                break; // only one char per stream per frame
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

/// Instance data for a single character quad sent to GPU
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CharInstance {
    pub position: [f32; 3],  // world position
    pub uv_rect: [f32; 4],   // u_min, v_min, u_max, v_max
    pub color: [f32; 4],     // RGBA with HDR brightness
}

pub struct RainSimulation {
    pub streams: Vec<Stream>,
    pub num_cols: usize,
    pub num_rows: usize,
    pub num_layers: usize,
    pub density: f64,
    pub charset_len: usize,
}

// Theme colors: from bright head to dark tail (RGB 0-1 scale)
pub struct ThemeColors {
    pub head: [f32; 3],       // white
    pub head_glow: [f32; 3],  // bright theme color
    pub bright: [f32; 3],
    pub body: [f32; 3],
    pub fade1: [f32; 3],
    pub fade2: [f32; 3],
    pub fade3: [f32; 3],
    pub dim: [f32; 3],
}

pub const GREEN_THEME: ThemeColors = ThemeColors {
    head: [1.0, 1.0, 1.0],
    head_glow: [0.5, 1.0, 0.5],
    bright: [0.0, 1.0, 0.0],
    body: [0.0, 0.8, 0.0],
    fade1: [0.0, 0.6, 0.1],
    fade2: [0.0, 0.42, 0.08],
    fade3: [0.0, 0.26, 0.06],
    dim: [0.0, 0.14, 0.04],
};

impl RainSimulation {
    pub fn new(num_cols: usize, num_rows: usize, num_layers: usize, density: f64, charset_len: usize) -> Self {
        let mut rng = rand::thread_rng();
        let mut streams = Vec::new();

        for layer in 0..num_layers {
            let layer_density = density * (1.0 - layer as f64 * 0.06);
            for col in 0..num_cols {
                if rng.gen::<f64>() < layer_density {
                    streams.push(Stream::new_scattered(col, layer, num_rows, charset_len));
                }
            }
        }

        RainSimulation {
            streams,
            num_cols,
            num_rows,
            num_layers,
            density,
            charset_len,
        }
    }

    pub fn update(&mut self) {
        let mut rng = rand::thread_rng();

        for stream in &mut self.streams {
            stream.update(self.num_rows, self.charset_len);
        }

        // Remove dead streams and respawn
        self.streams.retain(|s| s.is_active());

        // Respawn in empty column/layer slots
        for layer in 0..self.num_layers {
            let layer_density = self.density * (1.0 - layer as f64 * 0.06);
            for col in 0..self.num_cols {
                let has_stream = self.streams.iter().any(|s| s.col == col && s.layer == layer);
                if !has_stream && rng.gen::<f64>() < layer_density * 0.02 {
                    self.streams.push(Stream::new(col, layer, self.num_rows, self.charset_len));
                }
            }
        }
    }

    pub fn generate_instances(
        &self,
        chars: &[char],
        atlas: &crate::gpu::atlas::GlyphAtlas,
        theme: &ThemeColors,
        col_spacing: f32,
        row_height: f32,
        layer_spacing: f32,
        grid_offset_x: f32,
    ) -> Vec<CharInstance> {
        let mut instances = Vec::with_capacity(self.streams.len() * 20);
        let mut rng = rand::thread_rng();

        for stream in &self.streams {
            let stream_len = stream.chars.len();
            let effective_fade = FADE_LENGTH.min(1.max(stream_len.saturating_sub(2)));
            let z = -(stream.layer as f32) * layer_spacing;

            for (i, &char_idx) in stream.chars.iter().enumerate() {
                let cy = stream.y - i as f32;
                // Don't clip — let GPU frustum cull

                let ch = chars[char_idx % chars.len()];
                let uv = match atlas.glyphs.get(&ch) {
                    Some(r) => r,
                    None => continue,
                };

                let x = (stream.col as f32 - self.num_cols as f32 / 2.0) * col_spacing + grid_offset_x;
                let y = -(cy - self.num_rows as f32 / 2.0) * row_height;

                // Color based on position in stream
                let (base_color, brightness) = if i == 0 {
                    (theme.head, 3.0) // HDR bright for bloom
                } else if i == 1 && stream_len > 1 {
                    (theme.head_glow, 2.0)
                } else if i == 2 && stream_len > 2 {
                    (theme.bright, 1.5)
                } else {
                    let dist_from_end = stream_len - 1 - i;
                    if dist_from_end >= effective_fade {
                        (theme.body, 1.0)
                    } else {
                        let ratio = dist_from_end as f64 / effective_fade.max(1) as f64;
                        if ratio > 0.7 {
                            (theme.body, 0.8)
                        } else if ratio > 0.45 {
                            (theme.fade1, 0.6)
                        } else if ratio > 0.25 {
                            (theme.fade2, 0.4)
                        } else if ratio > 0.1 {
                            (theme.fade3, 0.25)
                        } else {
                            (theme.dim, 0.15)
                        }
                    }
                };

                // Sparkle
                let (color, brightness) = if i > 0 && rng.gen::<f64>() < SPARKLE_RATE {
                    (theme.head, 3.0)
                } else {
                    (base_color, brightness)
                };

                let color = color;

                instances.push(CharInstance {
                    position: [x, y, z],
                    uv_rect: [uv.u_min, uv.v_min, uv.u_max, uv.v_max],
                    color: [
                        color[0] * brightness,
                        color[1] * brightness,
                        color[2] * brightness,
                        1.0,
                    ],
                });
            }
        }

        instances
    }
}
