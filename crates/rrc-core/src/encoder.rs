use crate::bitstream::{
    assemble_data_codewords, detect_mode, encode_format_word, Mode,
};
use crate::ecc::rs_encode;
use crate::error::RrcError;
use crate::geometry::{
    find_fitting_version, format_word_sector_info, is_reference_gap, ring_count_for_version,
    sectors_in_ring, version_info, EccLevel,
};
use crate::mask::select_best_mask;
use crate::render::ascii::render_ascii;
use crate::render::jpg::render_jpeg;
use crate::render::png::render_png;
use crate::render::svg::render_svg;
use crate::render::RrcStyle;

/// Options configuring the RRC encoding process.
#[derive(Debug, Clone)]
pub struct EncodeOptions {
    /// Force a specific RRC version (1–40); if None, minimum required version is auto-selected.
    pub version: Option<u8>,
    /// Error correction level (default: Medium).
    pub ecc_level: EccLevel,
    /// Encoding mode (default: auto-detected from data).
    pub mode: Option<Mode>,
    /// Visual styling parameters for renderers.
    pub style: RrcStyle,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            version: None,
            ecc_level: EccLevel::M,
            mode: None,
            style: RrcStyle::default(),
        }
    }
}

/// An encoded, masked RRC symbol ready for visual rendering or transmission.
#[derive(Debug, Clone)]
pub struct RrcSymbol {
    pub version: u8,
    pub ecc_level: EccLevel,
    pub mode: Mode,
    pub mask_index: u8,
    pub payload: Vec<u8>,
    pub matrix: Vec<Vec<bool>>,
    pub style: RrcStyle,
}

impl RrcSymbol {
    /// Render symbol to an SVG string.
    pub fn to_svg(&self) -> String {
        render_svg(self.version, &self.matrix, &self.style)
    }

    /// Render symbol to PNG binary image buffer.
    pub fn to_png(&self) -> Result<Vec<u8>, RrcError> {
        render_png(self.version, &self.matrix, &self.style)
    }

    /// Render symbol to JPEG binary image buffer.
    pub fn to_jpeg(&self, quality: Option<u8>) -> Result<Vec<u8>, RrcError> {
        render_jpeg(self.version, &self.matrix, &self.style, quality)
    }

    /// Render symbol as ASCII/Braille terminal text.
    pub fn to_ascii(&self) -> String {
        render_ascii(self.version, &self.matrix, &self.style)
    }
}

/// Encode arbitrary data into an RRC symbol.
pub fn encode(data: &[u8], options: &EncodeOptions) -> Result<RrcSymbol, RrcError> {
    let mode = options.mode.unwrap_or_else(|| detect_mode(data));

    // Determine version
    let version = match options.version {
        Some(v) => {
            if !(1..=40).contains(&v) {
                return Err(RrcError::InvalidVersion(v));
            }
            let info = version_info(v)?;
            let needed_bytes = match mode {
                Mode::Numeric => (data.len() * 10 + 2) / 24 + 3,
                Mode::Alphanumeric => (data.len() * 11 + 1) / 16 + 3,
                Mode::Byte | Mode::Extended => data.len() + 3,
            };
            if info.data_codewords(options.ecc_level) < needed_bytes {
                return Err(RrcError::DataTooLarge {
                    needed: needed_bytes * 8,
                    capacity: info.data_codewords(options.ecc_level) * 8,
                });
            }
            v
        }
        None => {
            // Estimate needed bytes based on mode
            let needed_bytes = match mode {
                Mode::Numeric => (data.len() * 10 + 2) / 24 + 3,
                Mode::Alphanumeric => (data.len() * 11 + 1) / 16 + 3,
                Mode::Byte | Mode::Extended => data.len() + 3,
            };
            find_fitting_version(needed_bytes, options.ecc_level)?
        }
    };

    let info = version_info(version)?;
    let data_cw_count = info.data_codewords(options.ecc_level);
    let total_cw_count = info.total_codewords;
    let ecc_cw_count = info.ecc_codewords(options.ecc_level);

    // 1. Assemble data codewords
    let data_bytes = assemble_data_codewords(version, mode, data, data_cw_count)?;

    // 2. Compute Reed-Solomon ECC and assemble full codewords
    let full_codewords = rs_encode(&data_bytes, total_cw_count, ecc_cw_count);

    // 3. Convert full codewords to bit array
    let mut bitstream = Vec::with_capacity(full_codewords.len() * 8);
    for &b in &full_codewords {
        for i in (0..8).rev() {
            bitstream.push(((b >> i) & 1) != 0);
        }
    }

    // 4. Map bits into symbol rings and sectors
    let rings = ring_count_for_version(version);
    let mut unmasked_symbol = Vec::with_capacity(rings);
    let mut bit_idx = 0;

    for k in 0..rings {
        let n_sectors = sectors_in_ring(k);
        let mut ring_cells = vec![false; n_sectors];

        for s in 0..n_sectors {
            if is_reference_gap(k, s) {
                // Reference gap is always dark
                ring_cells[s] = true;
            } else if format_word_sector_info(k, s).is_some() {
                // Format word will be written after mask selection
                ring_cells[s] = false;
            } else {
                // Data sector
                if bit_idx < bitstream.len() {
                    ring_cells[s] = bitstream[bit_idx];
                    bit_idx += 1;
                } else {
                    // Remaining padding bits if any
                    ring_cells[s] = false;
                }
            }
        }
        unmasked_symbol.push(ring_cells);
    }

    // 5. Select best mask
    let (best_mask, mut final_matrix) = select_best_mask(&unmasked_symbol);

    // 6. Encode and write Format Word (both primary and mirrored in Ring 0)
    let format_word = encode_format_word(options.ecc_level, best_mask);
    for bit in 0..16 {
        let bit_val = ((format_word >> (15 - bit)) & 1) != 0;
        // Primary format word: sectors 1..16
        final_matrix[0][bit + 1] = bit_val;
        // Mirrored format word: sectors 37..52
        final_matrix[0][bit + 37] = bit_val;
    }

    // Ensure reference gaps are intact
    final_matrix[0][0] = true;
    final_matrix[0][36] = true;
    for k in 1..rings {
        final_matrix[k][0] = true;
    }

    Ok(RrcSymbol {
        version,
        ecc_level: options.ecc_level,
        mode,
        mask_index: best_mask,
        payload: data.to_vec(),
        matrix: final_matrix,
        style: options.style.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_short_text() {
        let data = b"https://github.com/MdSagorMunshi/rrc";
        let sym = encode(data, &EncodeOptions::default()).expect("Encoding should succeed");
        assert!(sym.version >= 1);
        assert_eq!(sym.ecc_level, EccLevel::M);
        assert_eq!(sym.matrix.len(), ring_count_for_version(sym.version));
    }

    #[test]
    fn test_encode_with_options() {
        let mut opts = EncodeOptions::default();
        opts.version = Some(2);
        opts.ecc_level = EccLevel::H;
        let sym = encode(b"RRC Test", &opts).expect("Encoding should succeed");
        assert_eq!(sym.version, 2);
        assert_eq!(sym.ecc_level, EccLevel::H);
    }
}
