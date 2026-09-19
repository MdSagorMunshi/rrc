use crate::error::RrcError;
use crate::geometry::EccLevel;

/// Mode indicator constants (4 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Numeric = 0b0001,
    Alphanumeric = 0b0010,
    Byte = 0b0100,
    Extended = 0b1000,
}

impl Mode {
    pub fn from_bits(bits: u8) -> Result<Self, RrcError> {
        match bits {
            0b0001 => Ok(Mode::Numeric),
            0b0010 => Ok(Mode::Alphanumeric),
            0b0100 => Ok(Mode::Byte),
            0b1000 => Ok(Mode::Extended),
            _ => Err(RrcError::InvalidMode(bits)),
        }
    }

    pub fn to_bits(self) -> u8 {
        self as u8
    }

    pub fn length_bits(self, version: u8) -> usize {
        match self {
            Mode::Numeric => {
                if version <= 9 {
                    10
                } else if version <= 26 {
                    12
                } else {
                    14
                }
            }
            Mode::Alphanumeric => {
                if version <= 9 {
                    9
                } else if version <= 26 {
                    11
                } else {
                    13
                }
            }
            Mode::Byte => {
                if version <= 9 {
                    8
                } else {
                    16
                }
            }
            Mode::Extended => 16,
        }
    }
}

// BCH polynomial for 16-bit format word: x^10 + x^8 + x^5 + x^4 + x^2 + x + 1 (0x537)
const BCH_POLY: u32 = 0x537;

/// Encode 5-bit format data (2 bits ECC, 3 bits mask) into 16-bit BCH format word.
pub fn encode_format_word(ecc_level: EccLevel, mask_index: u8) -> u16 {
    let data = ((ecc_level.to_bits() as u32) << 3) | ((mask_index & 0b111) as u32);
    let mut val = data << 11;
    for i in 0..5 {
        if (val & (1 << (15 - i))) != 0 {
            val ^= BCH_POLY << (5 - i);
        }
    }
    ((data << 11) | val) as u16
}

/// Precomputed 32-entry codebook of all valid 16-bit format words.
pub static FORMAT_CODEBOOK: [u16; 32] = {
    let mut book = [0u16; 32];
    let mut d = 0;
    while d < 32 {
        let mut val = (d as u32) << 11;
        let mut i = 0;
        while i < 5 {
            if (val & (1 << (15 - i))) != 0 {
                val ^= BCH_POLY << (5 - i);
            }
            i += 1;
        }
        book[d] = (((d as u32) << 11) | val) as u16;
        d += 1;
    }
    book
};

/// Decode 16-bit format word using minimum Hamming distance (corrects up to 3 bit errors).
pub fn decode_format_word(received: u16) -> Result<(EccLevel, u8), RrcError> {
    let mut best_idx = 0;
    let mut min_dist = 16;

    for (idx, &valid) in FORMAT_CODEBOOK.iter().enumerate() {
        let dist = (received ^ valid).count_ones() as usize;
        if dist < min_dist {
            min_dist = dist;
            best_idx = idx;
            if min_dist == 0 {
                break;
            }
        }
    }

    if min_dist <= 3 {
        let ecc_bits = ((best_idx >> 3) & 0b11) as u8;
        let mask = (best_idx & 0b111) as u8;
        Ok((EccLevel::from_bits(ecc_bits), mask))
    } else {
        Err(RrcError::FormatWordCorrupted)
    }
}

/// Alphanumeric character encoding lookup table.
const ALPHANUMERIC_CHARS: &[u8; 45] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

fn alpha_char_value(c: u8) -> Result<u16, RrcError> {
    for (i, &ch) in ALPHANUMERIC_CHARS.iter().enumerate() {
        if ch == c {
            return Ok(i as u16);
        }
    }
    Err(RrcError::InvalidCharacter(c as char))
}

fn alpha_value_char(val: u16) -> u8 {
    ALPHANUMERIC_CHARS[(val as usize) % 45]
}

/// Automatically detect the best encoding mode for given data.
pub fn detect_mode(data: &[u8]) -> Mode {
    if data.is_empty() {
        return Mode::Byte;
    }
    // Check if numeric
    if data.iter().all(|&b| b.is_ascii_digit()) {
        return Mode::Numeric;
    }
    // Check if alphanumeric
    if data.iter().all(|&b| alpha_char_value(b).is_ok()) {
        return Mode::Alphanumeric;
    }
    Mode::Byte
}

/// A bit buffer for appending arbitrary-length bit sequences.
#[derive(Debug, Clone, Default)]
pub struct BitBuffer {
    bits: Vec<bool>,
}

impl BitBuffer {
    pub fn new() -> Self {
        Self { bits: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bits: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.bits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    pub fn push_bit(&mut self, bit: bool) {
        self.bits.push(bit);
    }

    pub fn push_bits(&mut self, value: u64, bit_count: usize) {
        for i in (0..bit_count).rev() {
            self.bits.push(((value >> i) & 1) != 0);
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity((self.bits.len() + 7) / 8);
        for chunk in self.bits.chunks(8) {
            let mut b = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit {
                    b |= 1 << (7 - i);
                }
            }
            bytes.push(b);
        }
        bytes
    }

    pub fn as_slice(&self) -> &[bool] {
        &self.bits
    }
}

/// A bit reader for sequential reading of arbitrary-length bit sequences.
pub struct BitReader<'a> {
    bits: &'a [bool],
    cursor: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(bits: &'a [bool]) -> Self {
        Self { bits, cursor: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.bits.len().saturating_sub(self.cursor)
    }

    pub fn read_bit(&mut self) -> Result<bool, RrcError> {
        if self.cursor >= self.bits.len() {
            return Err(RrcError::TruncatedBitstream);
        }
        let b = self.bits[self.cursor];
        self.cursor += 1;
        Ok(b)
    }

    pub fn read_bits(&mut self, bit_count: usize) -> Result<u64, RrcError> {
        if self.remaining() < bit_count {
            return Err(RrcError::TruncatedBitstream);
        }
        let mut val = 0u64;
        for _ in 0..bit_count {
            val = (val << 1) | (self.read_bit()? as u64);
        }
        Ok(val)
    }
}

/// Assemble the complete data bitstream before Reed-Solomon encoding.
pub fn assemble_data_codewords(
    version: u8,
    mode: Mode,
    data: &[u8],
    data_codewords_target: usize,
) -> Result<Vec<u8>, RrcError> {
    let mut buf = BitBuffer::with_capacity(data_codewords_target * 8);

    // 1. Version (6 bits)
    buf.push_bits(version as u64, 6);

    // 2. Mode indicator (4 bits)
    buf.push_bits(mode.to_bits() as u64, 4);

    // 3. Length field
    let len_bits = mode.length_bits(version);
    let count = match mode {
        Mode::Numeric | Mode::Alphanumeric | Mode::Byte | Mode::Extended => data.len(),
    };
    if count >= (1 << len_bits) {
        return Err(RrcError::DataTooLarge {
            needed: count,
            capacity: (1 << len_bits) - 1,
        });
    }
    buf.push_bits(count as u64, len_bits);

    // 4. Payload encoding
    match mode {
        Mode::Numeric => {
            let mut i = 0;
            while i < data.len() {
                let rem = data.len() - i;
                if rem >= 3 {
                    let d1 = (data[i] - b'0') as u64;
                    let d2 = (data[i + 1] - b'0') as u64;
                    let d3 = (data[i + 2] - b'0') as u64;
                    let val = d1 * 100 + d2 * 10 + d3;
                    buf.push_bits(val, 10);
                    i += 3;
                } else if rem == 2 {
                    let d1 = (data[i] - b'0') as u64;
                    let d2 = (data[i + 1] - b'0') as u64;
                    let val = d1 * 10 + d2;
                    buf.push_bits(val, 7);
                    i += 2;
                } else {
                    let d1 = (data[i] - b'0') as u64;
                    buf.push_bits(d1, 4);
                    i += 1;
                }
            }
        }
        Mode::Alphanumeric => {
            let mut i = 0;
            while i < data.len() {
                let rem = data.len() - i;
                if rem >= 2 {
                    let c1 = alpha_char_value(data[i])? as u64;
                    let c2 = alpha_char_value(data[i + 1])? as u64;
                    let val = c1 * 45 + c2;
                    buf.push_bits(val, 11);
                    i += 2;
                } else {
                    let c1 = alpha_char_value(data[i])? as u64;
                    buf.push_bits(c1, 6);
                    i += 1;
                }
            }
        }
        Mode::Byte | Mode::Extended => {
            for &b in data {
                buf.push_bits(b as u64, 8);
            }
        }
    }

    let target_bits = data_codewords_target * 8;
    if buf.len() > target_bits {
        return Err(RrcError::DataTooLarge {
            needed: buf.len(),
            capacity: target_bits,
        });
    }

    // 5. Terminator (up to 4 zero bits)
    let term_bits = 4.min(target_bits - buf.len());
    buf.push_bits(0, term_bits);

    // 6. Byte alignment
    let pad_to_byte = (8 - (buf.len() % 8)) % 8;
    buf.push_bits(0, pad_to_byte);

    // 7. Pad bytes (alternating 0xEC, 0x11)
    let mut bytes = buf.to_bytes();
    let pad_pattern = [0xECu8, 0x11u8];
    let mut pad_idx = 0;
    while bytes.len() < data_codewords_target {
        bytes.push(pad_pattern[pad_idx % 2]);
        pad_idx += 1;
    }

    Ok(bytes)
}

/// Disassemble and decode data payload from error-corrected data bytes.
pub fn disassemble_data_codewords(bytes: &[u8]) -> Result<(u8, Mode, Vec<u8>), RrcError> {
    // Convert bytes to bits
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for &b in bytes {
        for i in (0..8).rev() {
            bits.push(((b >> i) & 1) != 0);
        }
    }

    let mut reader = BitReader::new(&bits);

    // 1. Version (6 bits)
    let version = reader.read_bits(6)? as u8;
    if !(1..=40).contains(&version) {
        return Err(RrcError::InvalidVersion(version));
    }

    // 2. Mode (4 bits)
    let mode_bits = reader.read_bits(4)? as u8;
    let mode = Mode::from_bits(mode_bits)?;

    // 3. Length
    let len_bits = mode.length_bits(version);
    let count = reader.read_bits(len_bits)? as usize;

    // 4. Payload
    let mut payload = Vec::with_capacity(count);
    match mode {
        Mode::Numeric => {
            let mut remaining = count;
            while remaining > 0 {
                if remaining >= 3 {
                    let val = reader.read_bits(10)?;
                    let d1 = (val / 100) as u8 + b'0';
                    let d2 = ((val % 100) / 10) as u8 + b'0';
                    let d3 = (val % 10) as u8 + b'0';
                    payload.extend_from_slice(&[d1, d2, d3]);
                    remaining -= 3;
                } else if remaining == 2 {
                    let val = reader.read_bits(7)?;
                    let d1 = (val / 10) as u8 + b'0';
                    let d2 = (val % 10) as u8 + b'0';
                    payload.extend_from_slice(&[d1, d2]);
                    remaining -= 2;
                } else {
                    let val = reader.read_bits(4)?;
                    payload.push(val as u8 + b'0');
                    remaining -= 1;
                }
            }
        }
        Mode::Alphanumeric => {
            let mut remaining = count;
            while remaining > 0 {
                if remaining >= 2 {
                    let val = reader.read_bits(11)? as u16;
                    let c1 = val / 45;
                    let c2 = val % 45;
                    payload.push(alpha_value_char(c1));
                    payload.push(alpha_value_char(c2));
                    remaining -= 2;
                } else {
                    let val = reader.read_bits(6)? as u16;
                    payload.push(alpha_value_char(val));
                    remaining -= 1;
                }
            }
        }
        Mode::Byte | Mode::Extended => {
            for _ in 0..count {
                payload.push(reader.read_bits(8)? as u8);
            }
        }
    }

    Ok((version, mode, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_word_bch_correction() {
        for ecc in [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H] {
            for mask in 0..8 {
                let word = encode_format_word(ecc, mask);

                // Exact match
                let (dec_ecc, dec_mask) = decode_format_word(word).unwrap();
                assert_eq!(dec_ecc, ecc);
                assert_eq!(dec_mask, mask);

                // Inject 1, 2, and 3 bit errors
                for bit1 in 0..16 {
                    let corrupted_1 = word ^ (1 << bit1);
                    let (res_ecc, res_mask) = decode_format_word(corrupted_1).unwrap();
                    assert_eq!(res_ecc, ecc);
                    assert_eq!(res_mask, mask);

                    for bit2 in (bit1 + 1)..16 {
                        let corrupted_2 = word ^ (1 << bit1) ^ (1 << bit2);
                        let (res2_ecc, res2_mask) = decode_format_word(corrupted_2).unwrap();
                        assert_eq!(res2_ecc, ecc);
                        assert_eq!(res2_mask, mask);

                        for bit3 in (bit2 + 1)..16 {
                            let corrupted_3 = word ^ (1 << bit1) ^ (1 << bit2) ^ (1 << bit3);
                            let (res3_ecc, res3_mask) = decode_format_word(corrupted_3).unwrap();
                            assert_eq!(res3_ecc, ecc);
                            assert_eq!(res3_mask, mask);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_numeric_mode_roundtrip() {
        let text = b"1234567890123";
        let target = 20;
        let encoded = assemble_data_codewords(1, Mode::Numeric, text, target).unwrap();
        assert_eq!(encoded.len(), target);

        let (ver, mode, decoded) = disassemble_data_codewords(&encoded).unwrap();
        assert_eq!(ver, 1);
        assert_eq!(mode, Mode::Numeric);
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_alphanumeric_mode_roundtrip() {
        let text = b"RRC-VERSION: 2026";
        let target = 20;
        let encoded = assemble_data_codewords(1, Mode::Alphanumeric, text, target).unwrap();
        assert_eq!(encoded.len(), target);

        let (ver, mode, decoded) = disassemble_data_codewords(&encoded).unwrap();
        assert_eq!(ver, 1);
        assert_eq!(mode, Mode::Alphanumeric);
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_byte_mode_roundtrip() {
        let text = b"https://github.com/MdSagorMunshi/rrc";
        let target = 50;
        let encoded = assemble_data_codewords(2, Mode::Byte, text, target).unwrap();
        assert_eq!(encoded.len(), target);

        let (ver, mode, decoded) = disassemble_data_codewords(&encoded).unwrap();
        assert_eq!(ver, 2);
        assert_eq!(mode, Mode::Byte);
        assert_eq!(decoded, text);
    }
}
