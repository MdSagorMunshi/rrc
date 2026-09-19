use crate::error::RrcError;
use crate::geometry::{inner_radius, outer_radius};
use std::f64::consts::PI;

/// Represents a detected center bullseye location and estimated scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BullseyeLocation {
    pub center_x: f64,
    pub center_y: f64,
    pub unit_size: f64,
    pub score: f64,
}

/// Convert RGBA or RGB image buffer to 8-bit grayscale.
pub fn to_grayscale(image_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, RrcError> {
    let total_pixels = (width * height) as usize;
    if total_pixels == 0 {
        return Err(RrcError::ImageTooSmall { width, height });
    }

    let bpp = image_data.len() / total_pixels;
    let mut gray = Vec::with_capacity(total_pixels);

    match bpp {
        4 => {
            for chunk in image_data.chunks_exact(4) {
                let a = chunk[3] as f32 / 255.0;
                let r = (chunk[0] as f32) * a + 255.0 * (1.0 - a);
                let g = (chunk[1] as f32) * a + 255.0 * (1.0 - a);
                let b = (chunk[2] as f32) * a + 255.0 * (1.0 - a);
                let y = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
                gray.push(y);
            }
        }
        3 => {
            for chunk in image_data.chunks_exact(3) {
                let y = (0.299 * (chunk[0] as f32) + 0.587 * (chunk[1] as f32) + 0.114 * (chunk[2] as f32)) as u8;
                gray.push(y);
            }
        }
        1 => {
            gray.extend_from_slice(image_data);
        }
        _ => return Err(RrcError::DecodingFailed(format!("Unsupported bytes per pixel: {bpp}"))),
    }

    Ok(gray)
}

/// Sample grayscale value with bilinear interpolation.
pub fn sample_bilinear(gray: &[u8], width: u32, height: u32, x: f64, y: f64) -> f64 {
    if x < 0.0 || y < 0.0 || x >= (width - 1) as f64 || y >= (height - 1) as f64 {
        let cx = x.clamp(0.0, (width.saturating_sub(1)) as f64) as usize;
        let cy = y.clamp(0.0, (height.saturating_sub(1)) as f64) as usize;
        return gray[cy * (width as usize) + cx] as f64;
    }

    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let fx = x - (x0 as f64);
    let fy = y - (y0 as f64);

    let w = width as usize;
    let p00 = gray[y0 * w + x0] as f64;
    let p10 = gray[y0 * w + x1] as f64;
    let p01 = gray[y1 * w + x0] as f64;
    let p11 = gray[y1 * w + x1] as f64;

    let top = p00 * (1.0 - fx) + p10 * fx;
    let bot = p01 * (1.0 - fx) + p11 * fx;
    top * (1.0 - fy) + bot * fy
}

/// Locate the center bullseye finder pattern using 8-directional radial contrast scans with multi-threshold candidate evaluation.
pub fn find_bullseye(gray: &[u8], width: u32, height: u32) -> Result<BullseyeLocation, RrcError> {
    let w = width as usize;
    let h = height as usize;
    if w < 24 || h < 24 {
        return Err(RrcError::ImageTooSmall { width, height });
    }

    let min_b = *gray.iter().min().unwrap_or(&0) as f64;
    let max_b = *gray.iter().max().unwrap_or(&255) as f64;
    if (max_b - min_b) < 15.0 {
        return Err(RrcError::NotFound);
    }

    let mut sum = 0u64;
    for &b in gray {
        sum += b as u64;
    }
    let mean_thresh = (sum / (gray.len() as u64)) as f64;
    let mid_thresh = (min_b + max_b) * 0.5;

    let mut thresholds = vec![mid_thresh];
    if (mean_thresh - mid_thresh).abs() > 10.0 {
        thresholds.push(mean_thresh);
    }
    thresholds.push(min_b * 0.35 + max_b * 0.65);
    thresholds.push(min_b * 0.65 + max_b * 0.35);

    let mut best_overall: Option<BullseyeLocation> = None;
    for threshold in thresholds {
        if let Ok(loc) = find_bullseye_at_threshold(gray, width, height, threshold) {
            if best_overall.as_ref().map_or(true, |b| loc.score > b.score) {
                best_overall = Some(loc);
                if loc.score >= 5.0 {
                    break;
                }
            }
        }
    }

    best_overall.ok_or(RrcError::NotFound)
}

fn find_bullseye_at_threshold(gray: &[u8], width: u32, height: u32, threshold: f64) -> Result<BullseyeLocation, RrcError> {
    let w = width as usize;
    let h = height as usize;

    // 8 ray directions (normalized vectors)
    let inv_sqrt2 = std::f64::consts::FRAC_1_SQRT_2;
    let directions = [
        (0.0, -1.0),            // N (12 o'clock)
        (inv_sqrt2, -inv_sqrt2), // NE
        (1.0, 0.0),             // E
        (inv_sqrt2, inv_sqrt2),  // SE
        (0.0, 1.0),             // S
        (-inv_sqrt2, inv_sqrt2), // SW
        (-1.0, 0.0),            // W
        (-inv_sqrt2, -inv_sqrt2),// NW
    ];

    let mut best_score = 0.0f64;
    let mut best_center = (w as f64 * 0.5, h as f64 * 0.5);
    let mut best_unit_size = 1.0f64;

    // Subsampled search for candidates
    let step = 2.max((w.min(h)) / 120);

    for y in (step..h - step).step_by(step) {
        for x in (step..w - step).step_by(step) {
            let center_val = gray[y * w + x] as f64;
            // Bullseye center must be dark (below threshold)
            if center_val >= threshold {
                continue;
            }

            let max_r = (w.min(h) as f64) * 0.5;

            // Test 8 radial scan lines
            let mut ray_unit_estimates = Vec::with_capacity(8);
            let mut valid_rays = 0;

            for &(dx, dy) in &directions {
                let mut transitions = Vec::with_capacity(7);
                let mut cur_is_dark = true;
                let mut step_r = 1.0f64;

                while step_r < max_r {
                    let sx = (x as f64) + dx * step_r;
                    let sy = (y as f64) + dy * step_r;
                    if sx < 0.0 || sy < 0.0 || sx >= (w as f64) || sy >= (h as f64) {
                        break;
                    }
                    let val = sample_bilinear(gray, width, height, sx, sy);
                    let is_dark = val < threshold;
                    if is_dark != cur_is_dark {
                        transitions.push(step_r);
                        cur_is_dark = is_dark;
                        if transitions.len() == 7 {
                            break;
                        }
                    }
                    step_r += 0.5;
                }

                // Bullseye has 7 concentric zones, so 6 or 7 transitions
                if transitions.len() >= 6 {
                    valid_rays += 1;
                    // First transition is outer edge of zone 0 (radius 1u)
                    // 7th transition is outer edge of zone 6 (radius 7u)
                    let u_est = transitions[0];
                    if transitions.len() >= 7 {
                        let u_est_avg = transitions[6] / 7.0;
                        ray_unit_estimates.push(u_est_avg);
                    } else {
                        ray_unit_estimates.push(u_est);
                    }
                }
            }

            if valid_rays >= 6 && !ray_unit_estimates.is_empty() {
                let mean_u: f64 = ray_unit_estimates.iter().sum::<f64>() / (ray_unit_estimates.len() as f64);
                let var: f64 = ray_unit_estimates.iter().map(|&u| (u - mean_u).powi(2)).sum::<f64>()
                    / (ray_unit_estimates.len() as f64);
                let std_dev = var.sqrt();

                // Lower variance across rays indicates higher circular symmetry
                let score = (valid_rays as f64) / (1.0 + std_dev / mean_u.max(1.0));
                if score > best_score {
                    best_score = score;
                    best_center = (x as f64, y as f64);
                    best_unit_size = mean_u;
                }
            }
        }
    }

    if best_score < 3.0 {
        return Err(RrcError::NotFound);
    }

    // Subpixel centroid refinement within innermost dark zone
    let (bx, by) = best_center;
    let r_core = (best_unit_size * 0.9).max(1.0);
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_w = 0.0;

    let min_x = ((bx - r_core).floor() as usize).max(0);
    let max_x = ((bx + r_core).ceil() as usize).min(w - 1);
    let min_y = ((by - r_core).floor() as usize).max(0);
    let max_y = ((by + r_core).ceil() as usize).min(h - 1);

    for cy in min_y..=max_y {
        for cx in min_x..=max_x {
            let dx = cx as f64 - bx;
            let dy = cy as f64 - by;
            if dx * dx + dy * dy <= r_core * r_core {
                let val = gray[cy * w + cx] as f64;
                if val < threshold {
                    let weight = (threshold - val).max(0.1);
                    sum_x += cx as f64 * weight;
                    sum_y += cy as f64 * weight;
                    sum_w += weight;
                }
            }
        }
    }

    let refined_center = if sum_w > 0.0 {
        (sum_x / sum_w, sum_y / sum_w)
    } else {
        best_center
    };

    Ok(BullseyeLocation {
        center_x: refined_center.0,
        center_y: refined_center.1,
        unit_size: best_unit_size.max(0.5),
        score: best_score,
    })
}

/// Unwrap polar coordinates from Cartesian image centered at (center_x, center_y).
/// Output is a 2D array [row: radius, col: angle].
pub fn unwrap_polar(
    gray: &[u8],
    width: u32,
    height: u32,
    center_x: f64,
    center_y: f64,
    unit_size: f64,
    r_in_units: f64,
    r_out_units: f64,
    n_angles: usize,
    n_radii: usize,
) -> Vec<f64> {
    let mut unwrapped = vec![0.0f64; n_radii * n_angles];
    let dr = (r_out_units - r_in_units) / (n_radii as f64);
    let dtheta = (2.0 * PI) / (n_angles as f64);

    for r_idx in 0..n_radii {
        let r_unit = r_in_units + (r_idx as f64 + 0.5) * dr;
        let r_px = r_unit * unit_size;

        for a_idx in 0..n_angles {
            let theta = (a_idx as f64) * dtheta;
            // 12 o'clock is theta = 0, clockwise
            let x = center_x + r_px * theta.sin();
            let y = center_y - r_px * theta.cos();
            let val = sample_bilinear(gray, width, height, x, y);
            unwrapped[r_idx * n_angles + a_idx] = val;
        }
    }

    unwrapped
}

/// Calibrate angular rotation by finding the radial dark alignment of reference sector gaps.
pub fn find_angular_rotation(
    polar_data: &[f64],
    n_angles: usize,
    n_radii: usize,
    _threshold: f64,
) -> f64 {
    let mut best_angle_idx = 0;
    let mut min_intensity = f64::MAX;

    // Scan all angle columns
    for a in 0..n_angles {
        let mut col_sum = 0.0;
        for r in 0..n_radii {
            col_sum += polar_data[r * n_angles + a];
        }
        if col_sum < min_intensity {
            min_intensity = col_sum;
            best_angle_idx = a;
        }
    }

    (best_angle_idx as f64 / n_angles as f64) * 2.0 * PI
}

/// Sample sectors of a single data ring, detecting whether each sector is dark and whether it is an erasure.
/// Returns (bits, erasures).
pub fn sample_ring_sectors_detect(
    gray: &[u8],
    width: u32,
    height: u32,
    center_x: f64,
    center_y: f64,
    unit_size: f64,
    ring_idx: usize,
    n_sectors: usize,
    rotation_offset: f64,
) -> (Vec<bool>, Vec<bool>) {
    let r_mid = (inner_radius(ring_idx) + 0.5) * unit_size;
    let r_inner = inner_radius(ring_idx) * unit_size;
    let r_outer = outer_radius(ring_idx) * unit_size;

    let d_theta = (2.0 * PI) / (n_sectors as f64);
    let mut intensities = Vec::with_capacity(n_sectors);

    for s in 0..n_sectors {
        let theta_base = (s as f64) * d_theta + rotation_offset;
        let mut sum = 0.0;
        let sub_points = [
            (r_mid, theta_base + d_theta * 0.3),
            (r_mid, theta_base + d_theta * 0.5),
            (r_mid, theta_base + d_theta * 0.7),
            (r_inner * 0.7 + r_outer * 0.3, theta_base + d_theta * 0.5),
            (r_inner * 0.3 + r_outer * 0.7, theta_base + d_theta * 0.5),
        ];

        for &(r, theta) in &sub_points {
            let x = center_x + r * theta.sin();
            let y = center_y - r * theta.cos();
            let val = sample_bilinear(gray, width, height, x, y);
            sum += val;
        }

        let avg = sum / (sub_points.len() as f64);
        intensities.push(avg);
    }

    let min_val = intensities.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = intensities.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let threshold = if (max_val - min_val) > 40.0 {
        (min_val + max_val) * 0.5
    } else {
        intensities.iter().sum::<f64>() / (n_sectors as f64)
    };

    let bits = intensities.iter().map(|&val| val < threshold).collect();
    let erasures = vec![false; n_sectors];
    (bits, erasures)
}

/// Sample sectors of a single data ring.
/// Returns boolean vector (true = dark, false = light).
pub fn sample_ring_sectors(
    gray: &[u8],
    width: u32,
    height: u32,
    center_x: f64,
    center_y: f64,
    unit_size: f64,
    ring_idx: usize,
    n_sectors: usize,
    rotation_offset: f64,
) -> Vec<bool> {
    sample_ring_sectors_detect(
        gray,
        width,
        height,
        center_x,
        center_y,
        unit_size,
        ring_idx,
        n_sectors,
        rotation_offset,
    )
    .0
}
