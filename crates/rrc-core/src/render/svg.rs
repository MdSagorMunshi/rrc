use crate::geometry::{
    inner_radius, outer_radius, termination_inner_radius, termination_outer_radius,
    total_symbol_radius,
};
use crate::render::{RrcStyle, SectorStyle};
use std::f64::consts::PI;

/// Generate self-contained, valid SVG string for an RRC symbol.
pub fn render_svg(version: u8, symbol: &[Vec<bool>], style: &RrcStyle) -> String {
    let r_total = total_symbol_radius(version, style.quiet_zone as f64);
    let scale = 50.0 / r_total;
    let center = 50.0;

    let id_prefix = match &style.svg_id_prefix {
        Some(p) => format!("{p}-"),
        None => "rrc-".to_string(),
    };

    let mut svg = String::with_capacity(16384);

    // SVG Header
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100%" height="100%" shape-rendering="geometricPrecision">"#
    ));
    svg.push('\n');

    // Background rect
    svg.push_str(&format!(
        r#"  <rect id="{id_prefix}bg" width="100" height="100" fill="{}" />"#,
        style.background_color.to_hex()
    ));
    svg.push('\n');

    // 1. Center Bullseye Finder Pattern (7 alternating concentric zones)
    svg.push_str(&format!(r#"  <g id="{id_prefix}bullseye">"#));
    svg.push('\n');
    let b_dark = style.bullseye_dark_color().to_hex();
    let b_light = style.bullseye_light_color().to_hex();

    // Render concentric filled circles from outermost (Zone 6, radius 7u) down to Zone 0 (center circle, radius 1u)
    for zone in (0..7).rev() {
        let radius = (zone + 1) as f64 * scale;
        let color = if zone % 2 == 0 { &b_dark } else { &b_light };
        svg.push_str(&format!(
            r#"    <circle cx="{center:.3}" cy="{center:.3}" r="{radius:.3}" fill="{color}" />"#
        ));
        svg.push('\n');
    }
    svg.push_str("  </g>\n");

    // 2. Data Rings
    let dark_hex = style.dark_color.to_hex();
    let light_hex = style.light_color.to_hex();

    for (ring_idx, ring_sectors) in symbol.iter().enumerate() {
        svg.push_str(&format!(r#"  <g id="{id_prefix}ring-{ring_idx}">"#));
        svg.push('\n');

        let r_in_unit = inner_radius(ring_idx);
        let r_out_unit = outer_radius(ring_idx);

        let ring_gap = (style.ring_gap as f64 * 0.5).clamp(0.0, 0.4);
        let r1 = (r_in_unit + ring_gap) * scale;
        let r2 = (r_out_unit - ring_gap) * scale;

        let n_sectors = ring_sectors.len();
        let d_theta = 2.0 * PI / (n_sectors as f64);
        let sector_gap = (style.sector_gap as f64 * 0.5).clamp(0.0, 0.4);
        let half_gap_angle = d_theta * sector_gap;

        for (s_idx, &is_dark) in ring_sectors.iter().enumerate() {
            let color = if is_dark { &dark_hex } else { &light_hex };

            // If light sector has same color as background and sharp style, we can optionally skip or render
            let theta1 = (s_idx as f64) * d_theta + half_gap_angle;
            let theta2 = ((s_idx + 1) as f64) * d_theta - half_gap_angle;

            let path_d = match style.sector_style {
                SectorStyle::Sharp => render_wedge_sharp(center, r1, r2, theta1, theta2),
                SectorStyle::Rounded => render_wedge_rounded(center, r1, r2, theta1, theta2, 0.4),
                SectorStyle::Pill => render_wedge_pill(center, r1, r2, theta1, theta2),
                SectorStyle::InnerRounded => render_wedge_rounded(center, r1, r2, theta1, theta2, 0.2),
            };

            svg.push_str(&format!(
                r#"    <path d="{path_d}" fill="{color}" />"#
            ));
            svg.push('\n');
        }
        svg.push_str("  </g>\n");
    }

    // 3. Termination Ring (Solid dark band 1 unit wide)
    let r_term_in = termination_inner_radius(version) * scale;
    let r_term_out = termination_outer_radius(version) * scale;
    let term_path = format!(
        "M {center:.3} {y1:.3} A {r_out:.3} {r_out:.3} 0 1 0 {center:.3} {y2:.3} A {r_out:.3} {r_out:.3} 0 1 0 {center:.3} {y1:.3} M {center:.3} {iy1:.3} A {r_in:.3} {r_in:.3} 0 1 1 {center:.3} {iy2:.3} A {r_in:.3} {r_in:.3} 0 1 1 {center:.3} {iy1:.3} Z",
        center = center,
        y1 = center - r_term_out,
        y2 = center + r_term_out,
        r_out = r_term_out,
        iy1 = center - r_term_in,
        iy2 = center + r_term_in,
        r_in = r_term_in,
    );

    svg.push_str(&format!(
        r#"  <g id="{id_prefix}termination"><path d="{term_path}" fill="{dark_hex}" fill-rule="evenodd" /></g>"#
    ));
    svg.push('\n');

    svg.push_str("</svg>\n");
    svg
}

/// Convert polar (radius, angle) with 12 o'clock origin to Cartesian (x, y).
#[inline(always)]
fn polar_to_xy(center: f64, r: f64, theta: f64) -> (f64, f64) {
    let x = center + r * theta.sin();
    let y = center - r * theta.cos();
    (x, y)
}

fn render_wedge_sharp(center: f64, r1: f64, r2: f64, theta1: f64, theta2: f64) -> String {
    let (ax, ay) = polar_to_xy(center, r1, theta1);
    let (bx, by) = polar_to_xy(center, r2, theta1);
    let (cx, cy) = polar_to_xy(center, r2, theta2);
    let (dx, dy) = polar_to_xy(center, r1, theta2);

    let large_arc = if (theta2 - theta1) > PI { 1 } else { 0 };

    format!(
        "M {ax:.3} {ay:.3} L {bx:.3} {by:.3} A {r2:.3} {r2:.3} 0 {large_arc} 1 {cx:.3} {cy:.3} L {dx:.3} {dy:.3} A {r1:.3} {r1:.3} 0 {large_arc} 0 {ax:.3} {ay:.3} Z"
    )
}

fn render_wedge_rounded(center: f64, r1: f64, r2: f64, theta1: f64, theta2: f64, corner_radius: f64) -> String {
    let (ax, ay) = polar_to_xy(center, r1 + corner_radius, theta1);
    let (bx, by) = polar_to_xy(center, r2 - corner_radius, theta1);
    let (cx, cy) = polar_to_xy(center, r2 - corner_radius, theta2);
    let (dx, dy) = polar_to_xy(center, r1 + corner_radius, theta2);

    let large_arc = if (theta2 - theta1) > PI { 1 } else { 0 };

    format!(
        "M {ax:.3} {ay:.3} L {bx:.3} {by:.3} A {r2:.3} {r2:.3} 0 {large_arc} 1 {cx:.3} {cy:.3} L {dx:.3} {dy:.3} A {r1:.3} {r1:.3} 0 {large_arc} 0 {ax:.3} {ay:.3} Z"
    )
}

fn render_wedge_pill(center: f64, r1: f64, r2: f64, theta1: f64, theta2: f64) -> String {
    let r_mid = (r1 + r2) * 0.5;
    let dr = (r2 - r1) * 0.4;
    let (ax, ay) = polar_to_xy(center, r_mid - dr, (theta1 + theta2) * 0.5);
    let (bx, by) = polar_to_xy(center, r_mid + dr, (theta1 + theta2) * 0.5);
    let (cx, cy) = polar_to_xy(center, r2, theta2);
    let (dx, dy) = polar_to_xy(center, r1, theta1);

    format!(
        "M {ax:.3} {ay:.3} Q {dx:.3} {dy:.3} {bx:.3} {by:.3} Q {cx:.3} {cy:.3} {ax:.3} {ay:.3} Z"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_output_structure() {
        let style = RrcStyle::default();
        let sym = vec![
            vec![true; 72],
            vec![false; 72],
            vec![true; 90],
        ];
        let svg = render_svg(1, &sym, &style);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains(r#"id="rrc-bullseye""#));
        assert!(svg.contains(r#"id="rrc-ring-0""#));
        assert!(svg.contains(r#"id="rrc-ring-1""#));
        assert!(svg.contains(r#"id="rrc-ring-2""#));
        assert!(svg.contains(r#"id="rrc-termination""#));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn test_svg_sector_styles() {
        for style_variant in [SectorStyle::Sharp, SectorStyle::Rounded, SectorStyle::Pill, SectorStyle::InnerRounded] {
            let mut style = RrcStyle::default();
            style.sector_style = style_variant;
            let sym = vec![vec![true; 72]];
            let svg = render_svg(1, &sym, &style);
            assert!(svg.contains("<path d="));
        }
    }
}
