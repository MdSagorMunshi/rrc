use crate::bitstream::{decode_format_word, disassemble_data_codewords, Mode};
use crate::detect::{find_bullseye, sample_ring_sectors, sample_ring_sectors_detect, to_grayscale};
use crate::ecc::{rs_decode, rs_decode_erasures};
use crate::error::RrcError;
use crate::geometry::{
    is_data_sector, ring_count_for_version, sectors_in_ring, version_info, EccLevel,
};
use crate::mask::mask_fn;
use std::f64::consts::PI;

/// Options configuring the RRC decoding process.
#[derive(Debug, Clone, Default)]
pub struct DecodeOptions {
    /// Provide verbose diagnostic logging.
    pub verbose: bool,
    /// Hint/constrain expected RRC version.
    pub expected_version: Option<u8>,
}

/// The result of successfully decoding an RRC symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeResult {
    /// Raw payload bytes.
    pub payload: Vec<u8>,
    /// UTF-8 decoded string, if valid UTF-8.
    pub text: Option<String>,
    /// Detected RRC version (1–40).
    pub version: u8,
    /// Detected Error Correction Level.
    pub ecc_level: EccLevel,
    /// Detected Encoding Mode.
    pub mode: Mode,
}

/// Decode an RRC symbol from an image buffer (RGBA, RGB, or Grayscale).
pub fn decode(
    image_data: &[u8],
    width: u32,
    height: u32,
    options: &DecodeOptions,
) -> Result<DecodeResult, RrcError> {
    let gray = to_grayscale(image_data, width, height)?;

    // 1. Detect bullseye center and unit size
    let bullseye = find_bullseye(&gray, width, height)?;
    let cx = bullseye.center_x;
    let cy = bullseye.center_y;
    let u = bullseye.unit_size;

    // 2. Locate angular rotation angle and Format Word
    // Ring 0 has 72 sectors (5.0 degrees per sector)
    // Scan candidate rotation angles from 0 to 360 degrees in 1.0-degree steps
    let angle_step = 2.0 * PI / 360.0;
    let fine_step = 2.0 * PI / 1440.0;

    let mut candidate_alignments = Vec::new();

    for step_i in 0..360 {
        let rot = (step_i as f64) * angle_step;
        let ring0 = sample_ring_sectors(&gray, width, height, cx, cy, u, 0, 72, rot);

        // Primary reference gap at sector 0 and format word 1
        if ring0[0] {
            let mut word1 = 0u16;
            for i in 0..16 {
                if ring0[i + 1] {
                    word1 |= 1 << (15 - i);
                }
            }
            if let Ok((ecc, mask)) = decode_format_word(word1) {
                candidate_alignments.push((rot, ecc, mask));
            }
        }

        // Mirrored reference gap at sector 36 and format word 2 (at 180 degrees)
        if ring0[36] {
            let mut word2 = 0u16;
            for i in 0..16 {
                if ring0[i + 37] {
                    word2 |= 1 << (15 - i);
                }
            }
            if let Ok((ecc, mask)) = decode_format_word(word2) {
                candidate_alignments.push((rot + PI, ecc, mask));
            }
        }
    }

    if candidate_alignments.is_empty() {
        return Err(RrcError::FormatWordCorrupted);
    }

    // Cluster adjacent candidates within 5 degrees that have the same format word
    let mut clustered_candidates: Vec<(f64, EccLevel, u8)> = Vec::new();
    for &(rot, ecc, mask) in &candidate_alignments {
        let norm_rot = rot.rem_euclid(2.0 * PI);
        let mut found = false;
        for c in &mut clustered_candidates {
            let diff = (c.0 - norm_rot).abs();
            let ang_dist = diff.min(2.0 * PI - diff);
            if c.1 == ecc && c.2 == mask && ang_dist < (6.0 * PI / 180.0) {
                found = true;
                break;
            }
        }
        if !found {
            clustered_candidates.push((norm_rot, ecc, mask));
        }
    }

    if options.verbose {
        eprintln!("Detected bullseye: cx={cx:.2}, cy={cy:.2}, u={u:.2}");
        eprintln!("Found {} unique candidate alignment(s)", clustered_candidates.len());
    }

    for &(coarse_rot, ecc_level, mask_index) in &clustered_candidates {
        let mut min_valid_rot = None;
        let mut max_valid_rot = None;

        for offset_step in -10..=10 {
            let rot = coarse_rot + (offset_step as f64) * fine_step;
            let ring0 = sample_ring_sectors(&gray, width, height, cx, cy, u, 0, 72, rot);
            let mut word = 0u16;
            for i in 0..16 {
                if ring0[i + 1] {
                    word |= 1 << (15 - i);
                }
            }
            if let Ok((e, m)) = decode_format_word(word) {
                if e == ecc_level && m == mask_index {
                    if min_valid_rot.is_none() {
                        min_valid_rot = Some(rot);
                    }
                    max_valid_rot = Some(rot);
                }
            }
        }

        let rotation = match (min_valid_rot, max_valid_rot) {
            (Some(min_r), Some(max_r)) => (min_r + max_r) * 0.5,
            _ => coarse_rot,
        };

        for &test_rot in &[rotation, rotation + PI] {
            let version_range: Vec<u8> = match options.expected_version {
                Some(v) => vec![v],
                None => (1..=40).collect(),
            };

            for &v in &version_range {
                let info = match version_info(v) {
                    Ok(i) => i,
                    Err(_) => continue,
                };

                let rings = ring_count_for_version(v);
                let mut bitstream = Vec::with_capacity(info.total_data_bits);
                let mut erased_bits = Vec::with_capacity(info.total_data_bits);

                // Read all rings and sectors
                for k in 0..rings {
                    let n_sec = sectors_in_ring(k);
                    let (ring_cells, ring_erased) =
                        sample_ring_sectors_detect(&gray, width, height, cx, cy, u, k, n_sec, test_rot);

                    for s in 0..n_sec {
                        if is_data_sector(k, s) {
                            let raw_bit = ring_cells[s];
                            let unmasked_bit = if mask_fn(mask_index, k, s) {
                                !raw_bit
                            } else {
                                raw_bit
                            };
                            bitstream.push(unmasked_bit);
                            erased_bits.push(ring_erased[s]);
                        }
                    }
                }

                // Convert bitstream to codewords
                let target_codewords = info.total_codewords;
                let ecc_cw_count = info.ecc_codewords(ecc_level);
                if bitstream.len() < target_codewords * 8 {
                    continue;
                }

                let mut codewords = Vec::with_capacity(target_codewords);
                for chunk in bitstream[..target_codewords * 8].chunks(8) {
                    let mut byte_val = 0u8;
                    for (i, &bit) in chunk.iter().enumerate() {
                        if bit {
                            byte_val |= 1 << (7 - i);
                        }
                    }
                    codewords.push(byte_val);
                }

                // Identify erasure codeword indices
                let mut erasures = Vec::new();
                for (b_idx, chunk) in erased_bits[..target_codewords * 8].chunks(8).enumerate() {
                    if chunk.iter().any(|&e| e) {
                        erasures.push(b_idx);
                    }
                }

                if options.verbose && erasures.len() > 0 {
                    eprintln!("v={v} candidate test_rot={test_rot:.4}: erasures={}/{}", erasures.len(), ecc_cw_count);
                }

                // 4. Run Reed-Solomon error correction with erasures
                let decoded_bytes_res = if !erasures.is_empty() && erasures.len() <= ecc_cw_count {
                    rs_decode_erasures(&codewords, target_codewords, ecc_cw_count, &erasures)
                        .or_else(|_| rs_decode(&codewords, target_codewords, ecc_cw_count))
                } else {
                    rs_decode(&codewords, target_codewords, ecc_cw_count)
                };

                if let Ok(data_bytes) = decoded_bytes_res {
                    // 5. Disassemble and parse payload
                    if let Ok((parsed_version, mode, payload)) = disassemble_data_codewords(&data_bytes) {
                        if parsed_version == v {
                            let text = String::from_utf8(payload.clone()).ok();
                            return Ok(DecodeResult {
                                payload,
                                text,
                                version: v,
                                ecc_level,
                                mode,
                            });
                        }
                    }
                }
            }
        }
    }

    Err(RrcError::DecodingFailed("Could not decode symbol data or ECC failed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::{encode, EncodeOptions};

    #[test]
    fn test_encode_and_decode_roundtrip_v1() {
        let payload = b"HELLO RRC";
        let sym = encode(payload, &EncodeOptions::default()).expect("Encode should succeed");
        let (width, height, rgba) = crate::render::png::render_to_rgba(sym.version, &sym.matrix, &sym.style);

        let decoded = decode(&rgba, width, height, &DecodeOptions::default()).expect("Decode should succeed");
        assert_eq!(decoded.payload, payload);
        assert_eq!(decoded.text.as_deref(), Some("HELLO RRC"));
        assert_eq!(decoded.version, sym.version);
    }
}
