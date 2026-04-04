use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use std::collections::HashMap;

pub struct GlyphRect {
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

pub struct GlyphAtlas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA
    pub glyphs: HashMap<char, GlyphRect>,
    pub cell_width: f32,
    pub cell_height: f32,
}

impl GlyphAtlas {
    pub fn new(chars: &[char], font_size: f32) -> Self {
        let font_data = include_bytes!("../../assets/MatrixCodeNFI.ttf");
        let font = FontRef::try_from_slice(font_data).expect("Failed to load font");
        let scale = PxScale::from(font_size);
        let scaled_font = font.as_scaled(scale);

        let cell_h = (scaled_font.ascent() - scaled_font.descent() + 1.0).ceil() as u32;
        let cell_w = (cell_h as f32 * 0.6) as u32; // monospace approximation

        // Grid layout for atlas
        let cols = 16u32;
        let rows = (chars.len() as u32 + cols - 1) / cols;
        let atlas_w = cols * cell_w;
        let atlas_h = rows * cell_h;

        // Power of 2 sizes for GPU
        let atlas_w = atlas_w.next_power_of_two().max(256);
        let atlas_h = atlas_h.next_power_of_two().max(256);

        let mut pixels = vec![0u8; (atlas_w * atlas_h * 4) as usize];
        let mut glyphs = HashMap::new();

        for (i, &ch) in chars.iter().enumerate() {
            let col = i as u32 % cols;
            let row = i as u32 / cols;
            let origin_x = col * cell_w;
            let origin_y = row * cell_h;

            // Get glyph
            let glyph_id = font.glyph_id(ch);
            let glyph = glyph_id.with_scale_and_position(
                scale,
                ab_glyph::point(0.0, scaled_font.ascent()),
            );

            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|gx, gy, coverage| {
                    let px = origin_x as i32 + bounds.min.x as i32 + gx as i32;
                    let py = origin_y as i32 + bounds.min.y as i32 + gy as i32;
                    if px >= 0 && py >= 0 && (px as u32) < atlas_w && (py as u32) < atlas_h {
                        let idx = ((py as u32 * atlas_w + px as u32) * 4) as usize;
                        let alpha = (coverage * 255.0) as u8;
                        pixels[idx] = 255;     // R
                        pixels[idx + 1] = 255; // G
                        pixels[idx + 2] = 255; // B
                        pixels[idx + 3] = alpha;
                    }
                });
            }

            glyphs.insert(ch, GlyphRect {
                u_min: origin_x as f32 / atlas_w as f32,
                v_min: origin_y as f32 / atlas_h as f32,
                u_max: (origin_x + cell_w) as f32 / atlas_w as f32,
                v_max: (origin_y + cell_h) as f32 / atlas_h as f32,
            });
        }

        // Dilate glyphs to simulate bold weight (spread alpha to neighbors)
        let mut dilated = pixels.clone();
        for y in 1..(atlas_h - 1) {
            for x in 1..(atlas_w - 1) {
                let idx = (y * atlas_w + x) as usize * 4 + 3; // alpha channel
                let mut max_alpha = pixels[idx];
                for dy in [-1i32, 0, 1] {
                    for dx in [-1i32, 0, 1] {
                        let ni = ((y as i32 + dy) as u32 * atlas_w + (x as i32 + dx) as u32) as usize * 4 + 3;
                        max_alpha = max_alpha.max(pixels[ni]);
                    }
                }
                // Blend: 60% dilated max + 40% original for softer edges
                let blended = ((max_alpha as u16 * 6 + pixels[idx] as u16 * 4) / 10) as u8;
                dilated[idx] = blended;
                if blended > 0 {
                    dilated[idx - 3] = 255; // R
                    dilated[idx - 2] = 255; // G
                    dilated[idx - 1] = 255; // B
                }
            }
        }

        GlyphAtlas {
            width: atlas_w,
            height: atlas_h,
            pixels: dilated,
            glyphs,
            cell_width: cell_w as f32,
            cell_height: cell_h as f32,
        }
    }
}
