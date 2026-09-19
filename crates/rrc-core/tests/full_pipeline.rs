use rrc_core::{
    decode, encode, render_svg, DecodeOptions, EccLevel, EncodeOptions, Mode,
    SectorStyle,
};
use std::f64::consts::PI;

/// Rotate an RGBA image buffer by `angle_rad` radians around its center.
fn rotate_rgba(image: &[u8], width: u32, height: u32, angle_rad: f64) -> Vec<u8> {
    let mut rotated = vec![255u8; (width * height * 4) as usize];
    let cx = width as f64 * 0.5;
    let cy = height as f64 * 0.5;
    let cos_a = (-angle_rad).cos();
    let sin_a = (-angle_rad).sin();

    for y in 0..height {
        for x in 0..width {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;

            let src_x = cx + dx * cos_a - dy * sin_a;
            let src_y = cy + dx * sin_a + dy * cos_a;

            if src_x >= 0.0 && src_x < (width - 1) as f64 && src_y >= 0.0 && src_y < (height - 1) as f64 {
                let x0 = src_x.floor() as usize;
                let y0 = src_y.floor() as usize;
                let idx = (y0 * (width as usize) + x0) * 4;
                let out_idx = ((y * width + x) * 4) as usize;
                rotated[out_idx..out_idx + 4].copy_from_slice(&image[idx..idx + 4]);
            }
        }
    }
    rotated
}

/// Blank out a contiguous arc of the symbol outside the bullseye and quiet ring.
fn blank_out_arc(image: &mut [u8], width: u32, height: u32, arc_fraction: f64) {
    let cx = width as f64 * 0.5;
    let cy = height as f64 * 0.5;
    let max_r = cx.min(cy);
    // Data rings start at 8.0 units out of 14.0 units total radius in V1
    let data_start_r = max_r * (8.0 / 14.0);

    let arc_start = 1.0; // Start arc at ~57 degrees to leave sector 0 ref gap intact
    let arc_span = arc_fraction * 2.0 * PI;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let r = (dx * dx + dy * dy).sqrt();
            if r >= data_start_r && r <= max_r {
                let mut theta = dx.atan2(-dy);
                if theta < 0.0 {
                    theta += 2.0 * PI;
                }
                if theta >= arc_start && theta <= (arc_start + arc_span) {
                    let idx = ((y * width + x) * 4) as usize;
                    image[idx] = 255;
                    image[idx + 1] = 255;
                    image[idx + 2] = 255;
                    image[idx + 3] = 255;
                }
            }
        }
    }
}

#[test]
fn test_rotation_invariance_all_angles() {
    let payload = b"RRC-ROTATION-2026";
    let mut opts = EncodeOptions::default();
    opts.version = Some(1);
    let sym = encode(payload, &opts).expect("Encode should succeed");
    let (width, height, rgba) = rrc_core::render_to_rgba(sym.version, &sym.matrix, &sym.style);

    // Test 45, 90, 135, 180, 225, 270, 315 degrees
    let angles_deg = [45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0];

    for &deg in &angles_deg {
        let rad = deg * PI / 180.0;
        let rotated = rotate_rgba(&rgba, width, height, rad);
        let dec = decode(&rotated, width, height, &DecodeOptions::default())
            .unwrap_or_else(|e| panic!("Failed to decode at rotation {deg} deg: {e}"));
        assert_eq!(dec.payload, payload, "Payload mismatch at {deg} deg");
    }
}

#[test]
fn test_roundtrip_all_modes() {
    let test_cases: Vec<(Mode, &[u8])> = vec![
        (Mode::Numeric, b"987654321012345"),
        (Mode::Alphanumeric, b"RADIAL RESPONSE CODE 2026"),
        (Mode::Byte, b"https://rrc.spec/v1?token=42"),
        (Mode::Extended, &[0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03]),
    ];

    for (mode, payload) in test_cases {
        let mut opts = EncodeOptions::default();
        opts.mode = Some(mode);
        let sym = encode(payload, &opts).expect("Encode should succeed");
        let (width, height, rgba) = rrc_core::render_to_rgba(sym.version, &sym.matrix, &sym.style);
        let dec = decode(&rgba, width, height, &DecodeOptions::default()).expect("Decode should succeed");
        assert_eq!(dec.payload, payload);
        assert_eq!(dec.mode, mode);
    }
}

#[test]
fn test_roundtrip_multiple_versions() {
    let versions = [1, 2, 3, 5];
    for &v in &versions {
        let payload = format!("RRC V{v}").into_bytes();
        let mut opts = EncodeOptions::default();
        opts.version = Some(v);
        let sym = encode(&payload, &opts).expect("Encode should succeed");
        let (width, height, rgba) = rrc_core::render_to_rgba(sym.version, &sym.matrix, &sym.style);
        let dec = decode(&rgba, width, height, &DecodeOptions { expected_version: Some(v), verbose: false })
            .unwrap_or_else(|e| panic!("Decode failed for V{v}: {e}"));
        assert_eq!(dec.payload, payload);
    }
}

#[test]
fn test_damage_tolerance_ecc_level_h() {
    let payload = b"DAMAGE TEST";
    let mut opts = EncodeOptions::default();
    opts.version = Some(1);
    opts.ecc_level = EccLevel::H;
    let sym = encode(payload, &opts).expect("Encode should succeed");
    let (width, height, mut rgba) = rrc_core::render_to_rgba(sym.version, &sym.matrix, &sym.style);

    // Blank out 10% of the symbol arc (~2.4 codewords corrupted, well within ECC H capacity of 4 errors)
    blank_out_arc(&mut rgba, width, height, 0.10);

    let dec = decode(&rgba, width, height, &DecodeOptions { verbose: true, expected_version: Some(1) })
        .expect("ECC Level H should recover damaged symbol");
    assert_eq!(dec.payload, payload);
}

#[test]
fn test_all_sector_styles_svg_validity() {
    let payload = b"STYLE TEST";
    let styles = [
        SectorStyle::Sharp,
        SectorStyle::Rounded,
        SectorStyle::Pill,
        SectorStyle::InnerRounded,
    ];

    for s in styles {
        let mut opts = EncodeOptions::default();
        opts.style.sector_style = s;
        let sym = encode(payload, &opts).expect("Encode should succeed");
        let svg = render_svg(sym.version, &sym.matrix, &sym.style);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains(r#"id="rrc-bullseye""#));
        assert!(svg.contains(r#"id="rrc-termination""#));
        assert!(svg.ends_with("</svg>\n"));
    }
}
