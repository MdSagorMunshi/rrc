use crate::error::RrcError;

/// Number of concentric zones in the center bullseye finder pattern.
pub const BULLSEYE_ZONES: usize = 7;

/// Outer radius of the center bullseye finder pattern in units.
pub const BULLSEYE_RADIUS: f64 = 7.0;

/// Radial thickness of the quiet zone ring separating bullseye from Ring 0.
pub const QUIET_RING_WIDTH: f64 = 1.0;

/// Inner radius of data Ring 0 in units.
pub const DATA_START_RADIUS: f64 = 8.0;

/// Radial thickness of a single data ring in units.
pub const RING_THICKNESS: f64 = 1.0;

/// Radial thickness of the outer termination ring.
pub const TERMINATION_RING_WIDTH: f64 = 1.0;

/// Minimum width of the outer quiet zone outside the termination ring.
pub const OUTER_QUIET_ZONE_MIN: f64 = 2.0;

/// Base sector count for data Ring 0.
pub const BASE_SECTORS: usize = 72;

/// Scale factor for sector count growth per ring index.
pub const RING_SCALE_FACTOR: f64 = 0.125;

/// Allowed sector counts per ring (divisors of 360).
pub const DIVISORS_360: [usize; 11] = [24, 30, 36, 40, 45, 60, 72, 90, 120, 180, 360];

/// Error Correction Level for an RRC symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EccLevel {
    /// Low: ~7% codeword recovery capability.
    L = 0,
    /// Medium: ~15% codeword recovery capability (default).
    M = 1,
    /// Quality: ~25% codeword recovery capability.
    Q = 2,
    /// High: ~30% codeword recovery capability.
    H = 3,
}

impl EccLevel {
    /// Convert 2-bit format indicator to EccLevel.
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => EccLevel::L,
            0b01 => EccLevel::M,
            0b10 => EccLevel::Q,
            _ => EccLevel::H,
        }
    }

    /// Convert EccLevel to 2-bit format indicator.
    pub fn to_bits(self) -> u8 {
        self as u8
    }

    /// Approximate fraction of total codewords allocated to ECC.
    pub fn ecc_ratio(self) -> f64 {
        match self {
            EccLevel::L => 0.07,
            EccLevel::M => 0.15,
            EccLevel::Q => 0.25,
            EccLevel::H => 0.30,
        }
    }

    /// Parse from case-insensitive string ("L", "M", "Q", "H").
    pub fn from_str_loose(s: &str) -> Result<Self, RrcError> {
        match s.trim().to_ascii_uppercase().as_str() {
            "L" => Ok(EccLevel::L),
            "M" => Ok(EccLevel::M),
            "Q" => Ok(EccLevel::Q),
            "H" => Ok(EccLevel::H),
            _ => Err(RrcError::InvalidEccLevel(s.to_string())),
        }
    }

    /// Single character representation.
    pub fn as_char(self) -> char {
        match self {
            EccLevel::L => 'L',
            EccLevel::M => 'M',
            EccLevel::Q => 'Q',
            EccLevel::H => 'H',
        }
    }
}

/// Metadata and capacities for a specific RRC version (1 to 40).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VersionInfo {
    /// Version number (1–40).
    pub version: u8,
    /// Total number of data rings.
    pub ring_count: usize,
    /// Innermost ring sector count.
    pub base_sectors: usize,
    /// Sector scaling factor.
    pub ring_scale_factor: f64,
    /// Total usable data sectors (excluding format word and reference gaps).
    pub total_data_bits: usize,
    /// Total bytes (data bits / 8).
    pub total_codewords: usize,
    /// ECC codewords at level L.
    pub ecc_codewords_l: usize,
    /// ECC codewords at level M.
    pub ecc_codewords_m: usize,
    /// ECC codewords at level Q.
    pub ecc_codewords_q: usize,
    /// ECC codewords at level H.
    pub ecc_codewords_h: usize,
    /// Usable payload bytes at level M.
    pub capacity_bytes_m: usize,
    /// Usable numeric characters at level M.
    pub capacity_numeric_m: usize,
    /// Usable alphanumeric characters at level M.
    pub capacity_alpha_m: usize,
}

impl VersionInfo {
    /// Return the number of ECC codewords for this version at the specified level.
    pub fn ecc_codewords(&self, level: EccLevel) -> usize {
        match level {
            EccLevel::L => self.ecc_codewords_l,
            EccLevel::M => self.ecc_codewords_m,
            EccLevel::Q => self.ecc_codewords_q,
            EccLevel::H => self.ecc_codewords_h,
        }
    }

    /// Return usable data codewords (bytes) for this version at the specified level.
    pub fn data_codewords(&self, level: EccLevel) -> usize {
        self.total_codewords.saturating_sub(self.ecc_codewords(level))
    }
}

/// Compute the exact sector count for data ring `k`, snapped to the nearest divisor of 360.
pub fn sectors_in_ring(k: usize) -> usize {
    let raw = BASE_SECTORS as f64 * (1.0 + (k as f64) * RING_SCALE_FACTOR);
    let mut best = DIVISORS_360[0];
    let mut min_diff = (best as f64 - raw).abs();
    for &d in &DIVISORS_360[1..] {
        let diff = (d as f64 - raw).abs();
        if diff < min_diff {
            min_diff = diff;
            best = d;
        }
    }
    best.min(360)
}

/// Check if a given sector in a ring is a reference sector gap (Sector 0 at 0°, or mirrored Sector 36 in Ring 0).
pub fn is_reference_gap(ring: usize, sector: usize) -> bool {
    if sector == 0 {
        return true;
    }
    // Mirrored reference gap in Ring 0 at 180°
    if ring == 0 && sector == 36 {
        return true;
    }
    false
}

/// Check if a sector in Ring 0 contains Format Word 1 (primary) or Format Word 2 (mirrored).
/// Returns Some((bit_index, is_mirrored)) if format word sector, None otherwise.
pub fn format_word_sector_info(ring: usize, sector: usize) -> Option<(usize, bool)> {
    if ring != 0 {
        return None;
    }
    if (1..=16).contains(&sector) {
        Some((sector - 1, false))
    } else if (37..=52).contains(&sector) {
        Some((sector - 37, true))
    } else {
        None
    }
}

/// Check if a given sector carries payload/ECC data.
pub fn is_data_sector(ring: usize, sector: usize) -> bool {
    if is_reference_gap(ring, sector) {
        return false;
    }
    if format_word_sector_info(ring, sector).is_some() {
        return false;
    }
    true
}

/// Number of usable data sectors in ring `k`.
pub fn data_sectors_in_ring(k: usize) -> usize {
    let total = sectors_in_ring(k);
    if k == 0 {
        // Total 72 - 2 reference gaps (0, 36) - 32 format word bits (1..16, 37..52) = 38
        total - 2 - 32
    } else {
        // Total - 1 reference gap (0)
        total - 1
    }
}

/// Inner radius of ring `k` in units.
pub fn inner_radius(k: usize) -> f64 {
    DATA_START_RADIUS + (k as f64) * RING_THICKNESS
}

/// Outer radius of ring `k` in units.
pub fn outer_radius(k: usize) -> f64 {
    inner_radius(k) + RING_THICKNESS
}

/// Midpoint radius of ring `k` in units.
pub fn mid_radius(k: usize) -> f64 {
    inner_radius(k) + RING_THICKNESS * 0.5
}

/// Inner radius of the termination ring for a given version.
pub fn termination_inner_radius(version: u8) -> f64 {
    let rings = ring_count_for_version(version);
    inner_radius(rings)
}

/// Outer radius of the termination ring for a given version.
pub fn termination_outer_radius(version: u8) -> f64 {
    termination_inner_radius(version) + TERMINATION_RING_WIDTH
}

/// Total radius of the symbol including the outer quiet zone.
pub fn total_symbol_radius(version: u8, quiet_zone_units: f64) -> f64 {
    termination_outer_radius(version) + quiet_zone_units.max(OUTER_QUIET_ZONE_MIN)
}

/// Total data ring count for a version (V1 = 3 rings, V40 = 81 rings).
pub fn ring_count_for_version(version: u8) -> usize {
    3 + (version.saturating_sub(1) as usize) * 2
}

/// Total usable data bits for a version.
pub fn total_data_bits_for_version(version: u8) -> usize {
    let rings = ring_count_for_version(version);
    let mut sum = 0;
    for k in 0..rings {
        sum += data_sectors_in_ring(k);
    }
    sum
}

use std::sync::LazyLock;

fn build_version_table() -> [VersionInfo; 40] {
    let mut table = [VersionInfo {
        version: 1,
        ring_count: 3,
        base_sectors: BASE_SECTORS,
        ring_scale_factor: RING_SCALE_FACTOR,
        total_data_bits: 0,
        total_codewords: 0,
        ecc_codewords_l: 0,
        ecc_codewords_m: 0,
        ecc_codewords_q: 0,
        ecc_codewords_h: 0,
        capacity_bytes_m: 0,
        capacity_numeric_m: 0,
        capacity_alpha_m: 0,
    }; 40];

    for v in 1..=40 {
        let rings = 3 + (v - 1) * 2;
        let mut bits = 0;
        for k in 0..rings {
            bits += data_sectors_in_ring(k);
        }
        let total_cw = bits / 8;
        let ecc_l = ((total_cw as f64 * 0.07).round() as usize).max(2);
        let ecc_m = ((total_cw as f64 * 0.15).round() as usize).max(4);
        let ecc_q = ((total_cw as f64 * 0.25).round() as usize).max(6);
        let ecc_h = ((total_cw as f64 * 0.30).round() as usize).max(8);
        let cap_m = total_cw.saturating_sub(ecc_m);
        let cap_num = if cap_m > 3 { ((cap_m - 3) as f64 * 2.4) as usize } else { 0 };
        let cap_alpha = if cap_m > 3 { ((cap_m - 3) as f64 * 1.45) as usize } else { 0 };

        table[v - 1] = VersionInfo {
            version: v as u8,
            ring_count: rings,
            base_sectors: BASE_SECTORS,
            ring_scale_factor: RING_SCALE_FACTOR,
            total_data_bits: bits,
            total_codewords: total_cw,
            ecc_codewords_l: ecc_l,
            ecc_codewords_m: ecc_m,
            ecc_codewords_q: ecc_q,
            ecc_codewords_h: ecc_h,
            capacity_bytes_m: cap_m,
            capacity_numeric_m: cap_num,
            capacity_alpha_m: cap_alpha,
        };
    }
    table
}

/// Precomputed 40-version lookup table.
pub static VERSION_TABLE: LazyLock<[VersionInfo; 40]> = LazyLock::new(build_version_table);

/// Retrieve `VersionInfo` for a version number (1..=40).
pub fn version_info(version: u8) -> Result<&'static VersionInfo, RrcError> {
    if (1..=40).contains(&version) {
        Ok(&VERSION_TABLE[(version - 1) as usize])
    } else {
        Err(RrcError::InvalidVersion(version))
    }
}

/// Find the minimum version capable of storing `data_bytes` at the given ECC level.
pub fn find_fitting_version(data_bytes: usize, ecc_level: EccLevel) -> Result<u8, RrcError> {
    for info in VERSION_TABLE.iter() {
        if info.data_codewords(ecc_level) >= data_bytes {
            return Ok(info.version);
        }
    }
    Err(RrcError::DataTooLarge {
        needed: data_bytes * 8,
        capacity: VERSION_TABLE[39].data_codewords(ecc_level) * 8,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_sectors_divide_360() {
        for k in 0..100 {
            let s = sectors_in_ring(k);
            assert_eq!(360 % s, 0, "Ring {k} sector count {s} must divide 360 evenly");
        }
    }

    #[test]
    fn test_version_table_monotonic() {
        for v in 1..40 {
            let cur = version_info(v).unwrap();
            let next = version_info(v + 1).unwrap();
            assert!(next.total_data_bits > cur.total_data_bits);
            assert!(next.total_codewords > cur.total_codewords);
            assert!(next.capacity_bytes_m > cur.capacity_bytes_m);
        }
    }

    #[test]
    fn test_version_1_and_40_bounds() {
        let v1 = version_info(1).unwrap();
        assert_eq!(v1.ring_count, 3);
        assert!(v1.capacity_bytes_m >= 18);

        let v40 = version_info(40).unwrap();
        assert_eq!(v40.ring_count, 81);
        assert!(v40.capacity_bytes_m >= 2500);
    }

    #[test]
    fn test_ring_0_geometry() {
        assert_eq!(sectors_in_ring(0), 72);
        assert!(is_reference_gap(0, 0));
        assert!(is_reference_gap(0, 36));
        assert_eq!(format_word_sector_info(0, 1), Some((0, false)));
        assert_eq!(format_word_sector_info(0, 16), Some((15, false)));
        assert_eq!(format_word_sector_info(0, 37), Some((0, true)));
        assert_eq!(format_word_sector_info(0, 52), Some((15, true)));
        assert!(is_data_sector(0, 17));
        assert!(is_data_sector(0, 35));
        assert!(is_data_sector(0, 53));
        assert!(is_data_sector(0, 71));
        assert_eq!(data_sectors_in_ring(0), 38);
    }
}
