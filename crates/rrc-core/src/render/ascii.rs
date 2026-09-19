use crate::geometry::{
    ring_count_for_version, sectors_in_ring, termination_inner_radius, termination_outer_radius,
    total_symbol_radius,
};
use crate::render::RrcStyle;
use std::f64::consts::PI;

/// Sample the polar RRC symbol at unit coordinate (x, y) relative to center.
/// Returns true for dark, false for light.
fn sample_polar(version: u8, symbol: &[Vec<bool>], px: f64, py: f64) -> bool {
    let r = (px * px + py * py).sqrt();
    let rings = ring_count_for_version(version);
    let r_term_in = termination_inner_radius(version);
    let r_term_out = termination_outer_radius(version);

    if r > r_term_out {
        // Outer quiet zone
        false
    } else if r >= r_term_in && r <= r_term_out {
        // Termination ring (dark)
        true
    } else if r >= 8.0 && r < (8.0 + rings as f64) {
        let k = (r - 8.0).floor() as usize;
        let mut theta = px.atan2(-py);
        if theta < 0.0 {
            theta += 2.0 * PI;
        }
        let n_sec = sectors_in_ring(k);
        let s = ((theta / (2.0 * PI)) * (n_sec as f64)).floor() as usize % n_sec;
        if k < symbol.len() && s < symbol[k].len() {
            symbol[k][s]
        } else {
            false
        }
    } else if r >= 7.0 && r < 8.0 {
        // Bullseye quiet ring
        false
    } else if r < 7.0 {
        // Bullseye zones (0, 2, 4, 6 are dark)
        let zone = r.floor() as usize;
        zone % 2 == 0
    } else {
        false
    }
}

/// Render an RRC symbol as a text string (ASCII block or Unicode Braille).
pub fn render_ascii(version: u8, symbol: &[Vec<bool>], style: &RrcStyle) -> String {
    let r_total = total_symbol_radius(version, style.quiet_zone as f64);

    if style.ascii_use_braille {
        // Braille mode: 2 horizontal dots, 4 vertical dots per character cell
        let grid_width = (r_total * 2.0 * 2.5).ceil() as usize;
        let grid_height = (r_total * 2.0 * 2.5).ceil() as usize;
        let char_cols = (grid_width + 1) / 2;
        let char_rows = (grid_height + 3) / 4;

        let scale_x = (r_total * 2.0) / (grid_width as f64);
        let scale_y = (r_total * 2.0) / (grid_height as f64);

        let mut out = String::with_capacity(char_rows * (char_cols + 1));

        for cy in 0..char_rows {
            for cx in 0..char_cols {
                let mut braille_bits = 0u8;

                // Braille dot offsets:
                // (0,0)->bit 0, (0,1)->bit 1, (0,2)->bit 2, (0,3)->bit 6
                // (1,0)->bit 3, (1,1)->bit 4, (1,2)->bit 5, (1,3)->bit 7
                let dot_coords = [
                    (0, 0, 0x01),
                    (0, 1, 0x02),
                    (0, 2, 0x04),
                    (1, 0, 0x08),
                    (1, 1, 0x10),
                    (1, 2, 0x20),
                    (0, 3, 0x40),
                    (1, 3, 0x80),
                ];

                for (dx, dy, bit) in dot_coords {
                    let gx = cx * 2 + dx;
                    let gy = cy * 4 + dy;
                    if gx < grid_width && gy < grid_height {
                        let px = (gx as f64 - (grid_width as f64 * 0.5)) * scale_x;
                        let py = (gy as f64 - (grid_height as f64 * 0.5)) * scale_y;
                        if sample_polar(version, symbol, px, py) {
                            braille_bits |= bit;
                        }
                    }
                }

                let ch = char::from_u32(0x2800 + (braille_bits as u32)).unwrap_or(' ');
                out.push(ch);
            }
            out.push('\n');
        }
        out
    } else {
        // Standard ASCII block mode (accounts for typical ~2:1 terminal font aspect ratio)
        let cols = (r_total * 2.0 * 2.0).ceil() as usize;
        let rows = (r_total * 2.0).ceil() as usize;
        let scale_x = (r_total * 2.0) / (cols as f64);
        let scale_y = (r_total * 2.0) / (rows as f64);

        let mut out = String::with_capacity(rows * (cols + 1));
        for r in 0..rows {
            for c in 0..cols {
                let px = (c as f64 - (cols as f64 * 0.5)) * scale_x;
                let py = (r as f64 - (rows as f64 * 0.5)) * scale_y;
                if sample_polar(version, symbol, px, py) {
                    out.push(style.ascii_dark_char);
                } else {
                    out.push(style.ascii_light_char);
                }
            }
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_render_standard() {
        let style = RrcStyle::default();
        let sym = vec![vec![true; 72], vec![false; 72], vec![true; 90]];
        let ascii = render_ascii(1, &sym, &style);
        assert!(ascii.contains('█'));
        assert!(ascii.contains(' '));
        assert!(ascii.lines().count() > 10);
    }

    #[test]
    fn test_ascii_render_braille() {
        let mut style = RrcStyle::default();
        style.ascii_use_braille = true;
        let sym = vec![vec![true; 72], vec![false; 72], vec![true; 90]];
        let braille = render_ascii(1, &sym, &style);
        assert!(braille.chars().any(|c| (0x2800..=0x28FF).contains(&(c as u32))));
    }
}
