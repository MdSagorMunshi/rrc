use crate::geometry::is_data_sector;

/// Evaluate whether the sector at `(ring, sector)` should be inverted under mask `mask_index` (0–7).
pub fn mask_fn(mask_index: u8, ring: usize, sector: usize) -> bool {
    match mask_index & 0b111 {
        0 => (ring + sector) % 2 == 0,
        1 => ring % 2 == 0,
        2 => sector % 3 == 0,
        3 => (ring + sector) % 3 == 0,
        4 => (ring / 2 + sector / 3) % 2 == 0,
        5 => ((ring * sector) % 2 + (ring * sector) % 3) == 0,
        6 => (((ring * sector) % 2 + (ring * sector) % 3) % 2) == 0,
        7 => (((ring + sector) % 2 + (ring * sector) % 3) % 2) == 0,
        _ => unreachable!(),
    }
}

/// Apply mask `mask_index` to symbol data sectors.
/// Note: Format word sectors and reference gaps are never masked!
pub fn apply_mask(symbol: &mut [Vec<bool>], mask_index: u8) {
    for (ring_idx, ring) in symbol.iter_mut().enumerate() {
        for (sector_idx, cell) in ring.iter_mut().enumerate() {
            if is_data_sector(ring_idx, sector_idx) {
                if mask_fn(mask_index, ring_idx, sector_idx) {
                    *cell = !*cell;
                }
            }
        }
    }
}

/// Evaluate penalty score for a symbol layout.
/// Lower scores represent better visual distribution (fewer patterns, better dark/light balance).
pub fn calculate_penalty(symbol: &[Vec<bool>]) -> u64 {
    let mut total_penalty = 0u64;
    let mut total_dark = 0usize;
    let mut total_data_sectors = 0usize;

    for (ring_idx, ring) in symbol.iter().enumerate() {
        let n_sectors = ring.len();
        if n_sectors == 0 {
            continue;
        }

        // Count dark and data sectors
        for (sector_idx, &is_dark) in ring.iter().enumerate() {
            if is_data_sector(ring_idx, sector_idx) {
                total_data_sectors += 1;
                if is_dark {
                    total_dark += 1;
                }
            }
        }

        // 1. Run penalty: runs of 5+ identical consecutive data sectors
        let mut cur_color = ring[0];
        let mut run_len = 1;
        let deg_per_sector = 360.0 / (n_sectors as f64);

        for i in 1..n_sectors {
            if ring[i] == cur_color {
                run_len += 1;
            } else {
                if run_len >= 5 {
                    total_penalty += 10 + (run_len as u64 - 5) * 3;
                }
                // 2. Large contiguous arc penalty (> 45 degrees)
                let arc_deg = (run_len as f64) * deg_per_sector;
                if arc_deg > 45.0 {
                    total_penalty += ((arc_deg - 45.0) * 2.0) as u64;
                }
                cur_color = ring[i];
                run_len = 1;
            }
        }
        if run_len >= 5 {
            total_penalty += 10 + (run_len as u64 - 5) * 3;
        }
        let arc_deg = (run_len as f64) * deg_per_sector;
        if arc_deg > 45.0 {
            total_penalty += ((arc_deg - 45.0) * 2.0) as u64;
        }
    }

    // 3. Dark/Light balance penalty (ratio deviating from 50%)
    if total_data_sectors > 0 {
        let ratio = (total_dark as f64) / (total_data_sectors as f64);
        let dev = (ratio - 0.5).abs();
        let balance_penalty = (dev * 1000.0) as u64;
        total_penalty += balance_penalty;
    }

    total_penalty
}

/// Select the optimal mask (0–7) with the lowest penalty score.
pub fn select_best_mask(unmasked_symbol: &[Vec<bool>]) -> (u8, Vec<Vec<bool>>) {
    let mut best_mask = 0u8;
    let mut min_penalty = u64::MAX;
    let mut best_symbol = unmasked_symbol.to_vec();

    for mask in 0..8 {
        let mut candidate = unmasked_symbol.to_vec();
        apply_mask(&mut candidate, mask);
        let penalty = calculate_penalty(&candidate);
        if penalty < min_penalty {
            min_penalty = penalty;
            best_mask = mask;
            best_symbol = candidate;
        }
    }

    (best_mask, best_symbol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{ring_count_for_version, sectors_in_ring};

    #[test]
    fn test_mask_idempotency() {
        let rings = ring_count_for_version(1);
        let mut original = Vec::new();
        for k in 0..rings {
            let s = sectors_in_ring(k);
            original.push(vec![false; s]);
        }

        for mask in 0..8 {
            let mut sym = original.clone();
            apply_mask(&mut sym, mask);
            // Apply again to invert back
            apply_mask(&mut sym, mask);
            assert_eq!(sym, original, "Mask {mask} double-application must yield identity");
        }
    }

    #[test]
    fn test_penalty_prefers_balanced_symbol() {
        let rings = ring_count_for_version(1);
        let mut all_dark = Vec::new();
        let mut checkered = Vec::new();

        for k in 0..rings {
            let s = sectors_in_ring(k);
            all_dark.push(vec![true; s]);
            checkered.push((0..s).map(|i| (k + i) % 2 == 0).collect());
        }

        let penalty_all_dark = calculate_penalty(&all_dark);
        let penalty_checkered = calculate_penalty(&checkered);
        assert!(penalty_checkered < penalty_all_dark);
    }
}
