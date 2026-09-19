use crate::error::RrcError;
use crate::geometry::{
    ring_count_for_version, sectors_in_ring, termination_inner_radius, termination_outer_radius,
    total_symbol_radius,
};
use crate::render::RrcStyle;
use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use std::f64::consts::PI;
use std::io::Cursor;

/// Render an RRC symbol to a raw RGBA pixel buffer (width, height, bytes).
pub fn render_to_rgba(version: u8, symbol: &[Vec<bool>], style: &RrcStyle) -> (u32, u32, Vec<u8>) {
    let r_total = total_symbol_radius(version, style.quiet_zone as f64);
    let scale = style.render_scale.max(1) as f64;
    let size = (r_total * 2.0 * scale).ceil() as u32;
    let size = if size % 2 == 0 { size } else { size + 1 };
    let center = (size as f64) * 0.5;

    let rings = ring_count_for_version(version);
    let r_term_in = termination_inner_radius(version);
    let r_term_out = termination_outer_radius(version);

    let b_dark = style.bullseye_dark_color();
    let b_light = style.bullseye_light_color();
    let dark = style.dark_color;
    let light = style.light_color;
    let bg = style.background_color;

    let half_ring_gap = (style.ring_gap as f64 * 0.5).clamp(0.0, 0.4);
    let half_sector_gap = (style.sector_gap as f64 * 0.5).clamp(0.0, 0.4);

    let mut buffer = vec![0u8; (size * size * 4) as usize];

    // Precompute sector counts for all rings
    let ring_sectors_count: Vec<usize> = (0..rings).map(|k| sectors_in_ring(k)).collect();

    // 2x2 supersampling offsets
    let subpixel_offsets = [
        (-0.25, -0.25),
        (0.25, -0.25),
        (-0.25, 0.25),
        (0.25, 0.25),
    ];

    for y in 0..size {
        for x in 0..size {
            let mut r_accum = 0u32;
            let mut g_accum = 0u32;
            let mut b_accum = 0u32;
            let mut a_accum = 0u32;

            for &(ox, oy) in &subpixel_offsets {
                let px = (x as f64 + 0.5 + ox - center) / scale;
                let py = (y as f64 + 0.5 + oy - center) / scale;

                let r = (px * px + py * py).sqrt();

                let sample_color = if r > r_total {
                    bg
                } else if r > r_term_out {
                    bg
                } else if r >= r_term_in && r <= r_term_out {
                    dark
                } else if r >= 8.0 && r < (8.0 + rings as f64) {
                    let k = (r - 8.0).floor() as usize;
                    let frac_r = (r - 8.0) - (k as f64);

                    if frac_r < half_ring_gap || frac_r > (1.0 - half_ring_gap) {
                        bg
                    } else {
                        // Angle from 12 o'clock, clockwise [0, 2*PI)
                        let mut theta = px.atan2(-py);
                        if theta < 0.0 {
                            theta += 2.0 * PI;
                        }

                        let n_sec = ring_sectors_count[k];
                        let sec_pos = (theta / (2.0 * PI)) * (n_sec as f64);
                        let s = sec_pos.floor() as usize % n_sec;
                        let frac_theta = sec_pos - sec_pos.floor();

                        if frac_theta < half_sector_gap || frac_theta > (1.0 - half_sector_gap) {
                            bg
                        } else if k < symbol.len() && s < symbol[k].len() {
                            if symbol[k][s] {
                                dark
                            } else {
                                light
                            }
                        } else {
                            bg
                        }
                    }
                } else if r >= 7.0 && r < 8.0 {
                    // Bullseye quiet ring
                    b_light
                } else if r < 7.0 {
                    // Bullseye zones (0..6)
                    let zone = r.floor() as usize;
                    if zone % 2 == 0 {
                        b_dark
                    } else {
                        b_light
                    }
                } else {
                    bg
                };

                r_accum += sample_color.r as u32;
                g_accum += sample_color.g as u32;
                b_accum += sample_color.b as u32;
                a_accum += sample_color.a as u32;
            }

            let idx = ((y * size + x) * 4) as usize;
            buffer[idx] = (r_accum / 4) as u8;
            buffer[idx + 1] = (g_accum / 4) as u8;
            buffer[idx + 2] = (b_accum / 4) as u8;
            buffer[idx + 3] = (a_accum / 4) as u8;
        }
    }

    (size, size, buffer)
}

/// Render an RRC symbol to PNG binary buffer.
pub fn render_png(version: u8, symbol: &[Vec<bool>], style: &RrcStyle) -> Result<Vec<u8>, RrcError> {
    let (width, height, rgba) = render_to_rgba(version, symbol, style);
    let mut out = Cursor::new(Vec::new());
    let encoder = PngEncoder::new(&mut out);
    encoder
        .write_image(&rgba, width, height, ExtendedColorType::Rgba8)
        .map_err(|e| RrcError::RenderError(format!("PNG encoding error: {e}")))?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_png_render_generates_valid_png() {
        let mut style = RrcStyle::default();
        style.render_scale = 4;
        let sym = vec![vec![true; 72], vec![false; 72], vec![true; 90]];
        let png = render_png(1, &sym, &style).expect("PNG render should succeed");
        assert!(png.len() > 100);
        // Check PNG signature: 89 50 4E 47 0D 0A 1A 0A
        assert_eq!(&png[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }
}
