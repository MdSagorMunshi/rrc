use crate::error::RrcError;
use crate::render::png::render_to_rgba;
use crate::render::RrcStyle;
use image::{codecs::jpeg::JpegEncoder, ExtendedColorType, ImageEncoder};
use std::io::Cursor;

/// Render an RRC symbol to JPEG binary buffer with quality control.
pub fn render_jpeg(
    version: u8,
    symbol: &[Vec<bool>],
    style: &RrcStyle,
    quality: Option<u8>,
) -> Result<Vec<u8>, RrcError> {
    let q = quality.unwrap_or(style.jpeg_quality).clamp(1, 100);
    let (width, height, rgba) = render_to_rgba(version, symbol, style);

    // Convert RGBA to RGB for JPEG
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for chunk in rgba.chunks(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let a = chunk[3] as u32;

        if a == 255 {
            rgb.extend_from_slice(&[r, g, b]);
        } else {
            // Alpha composite against background
            let bg = style.background_color;
            let cr = ((r as u32 * a + bg.r as u32 * (255 - a)) / 255) as u8;
            let cg = ((g as u32 * a + bg.g as u32 * (255 - a)) / 255) as u8;
            let cb = ((b as u32 * a + bg.b as u32 * (255 - a)) / 255) as u8;
            rgb.extend_from_slice(&[cr, cg, cb]);
        }
    }

    let mut out = Cursor::new(Vec::new());
    let encoder = JpegEncoder::new_with_quality(&mut out, q);
    encoder
        .write_image(&rgb, width, height, ExtendedColorType::Rgb8)
        .map_err(|e| RrcError::RenderError(format!("JPEG encoding error: {e}")))?;

    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jpeg_render_generates_valid_jpeg() {
        let mut style = RrcStyle::default();
        style.render_scale = 4;
        let sym = vec![vec![true; 72], vec![false; 72], vec![true; 90]];
        let jpeg = render_jpeg(1, &sym, &style, Some(85)).expect("JPEG render should succeed");
        assert!(jpeg.len() > 100);
        // Check JPEG SOI marker: FF D8
        assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
    }
}
