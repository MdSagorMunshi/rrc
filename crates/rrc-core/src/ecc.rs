use crate::error::RrcError;

/// Primitive polynomial for GF(2^8): x^8 + x^4 + x^3 + x^2 + 1 (0x11D = 285)
pub const PRIMITIVE_POLYNOMIAL: u16 = 0x11D;

/// Precomputed antilog/exponential table for GF(2^8) (doubled to 512 for fast wrap-around multiplication).
pub const EXP_TABLE: [u8; 512] = {
    let mut exp = [0u8; 512];
    let mut x: u16 = 1;
    let mut i = 0;
    while i < 255 {
        exp[i] = x as u8;
        exp[i + 255] = x as u8;
        x <<= 1;
        if (x & 0x100) != 0 {
            x ^= PRIMITIVE_POLYNOMIAL;
        }
        i += 1;
    }
    exp
};

/// Precomputed logarithm table for GF(2^8).
pub const LOG_TABLE: [u8; 256] = {
    let mut log = [0u8; 256];
    let mut i = 0;
    while i < 255 {
        log[EXP_TABLE[i] as usize] = i as u8;
        i += 1;
    }
    log
};

/// Addition in GF(2^8) (bitwise XOR).
#[inline(always)]
pub fn gf_add(a: u8, b: u8) -> u8 {
    a ^ b
}

/// Subtraction in GF(2^8) (identical to addition).
#[inline(always)]
pub fn gf_sub(a: u8, b: u8) -> u8 {
    a ^ b
}

/// Multiplication in GF(2^8) using precomputed log and exp tables.
#[inline(always)]
pub fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        0
    } else {
        EXP_TABLE[LOG_TABLE[a as usize] as usize + LOG_TABLE[b as usize] as usize]
    }
}

/// Division in GF(2^8). Returns 0 if dividend is 0; panics if divisor is 0.
#[inline(always)]
pub fn gf_div(a: u8, b: u8) -> u8 {
    if b == 0 {
        panic!("Division by zero in GF(2^8)");
    }
    if a == 0 {
        0
    } else {
        let log_a = LOG_TABLE[a as usize] as usize;
        let log_b = LOG_TABLE[b as usize] as usize;
        EXP_TABLE[(log_a + 255 - log_b) % 255]
    }
}

/// Multiplicative inverse in GF(2^8).
#[inline(always)]
pub fn gf_inv(a: u8) -> u8 {
    if a == 0 {
        panic!("Division by zero in GF(2^8)");
    }
    EXP_TABLE[255 - LOG_TABLE[a as usize] as usize]
}

/// Multiply two polynomials in GF(2^8).
pub fn poly_mul(p: &[u8], q: &[u8]) -> Vec<u8> {
    let mut r = vec![0u8; p.len() + q.len() - 1];
    for (i, &a) in p.iter().enumerate() {
        if a != 0 {
            for (j, &b) in q.iter().enumerate() {
                r[i + j] ^= gf_mul(a, b);
            }
        }
    }
    r
}

/// Compute Reed-Solomon generator polynomial for `n_sym` ECC symbols:
/// g(x) = (x - a^0)(x - a^1)...(x - a^(n_sym-1))
pub fn rs_generator_poly(n_sym: usize) -> Vec<u8> {
    let mut g = vec![1u8];
    for i in 0..n_sym {
        g = poly_mul(&g, &[1, EXP_TABLE[i]]);
    }
    g
}

/// Encode a single message block with `ecc_len` Reed-Solomon parity codewords.
/// Returns the generated `ecc_len` ECC codewords.
pub fn rs_encode_block(msg: &[u8], ecc_len: usize) -> Vec<u8> {
    if ecc_len == 0 {
        return Vec::new();
    }
    let gen = rs_generator_poly(ecc_len);
    let mut rem = vec![0u8; ecc_len];

    for &b in msg {
        let factor = gf_add(b, rem[0]);
        for j in 0..(ecc_len - 1) {
            rem[j] = gf_add(rem[j + 1], gf_mul(factor, gen[j + 1]));
        }
        rem[ecc_len - 1] = gf_mul(factor, gen[ecc_len]);
    }
    rem
}

/// Calculate syndromes for a received codeword of length `n` with `ecc_len` parity symbols.
pub fn rs_calc_syndromes(codeword: &[u8], ecc_len: usize) -> Vec<u8> {
    let n = codeword.len();
    let mut syn = vec![0u8; ecc_len];
    for i in 0..ecc_len {
        let mut val = 0u8;
        for (j, &b) in codeword.iter().enumerate() {
            let power = (i * (n - 1 - j)) % 255;
            val ^= gf_mul(b, EXP_TABLE[power]);
        }
        syn[i] = val;
    }
    syn
}

/// Berlekamp-Massey algorithm to find the error locator polynomial Lambda(x).
pub fn rs_berlekamp_massey(syndromes: &[u8]) -> Vec<u8> {
    let mut c = vec![1u8];
    let mut b = vec![1u8];
    let mut l = 0usize;
    let mut m = 1usize;
    let mut b_scale = 1u8;

    for n in 0..syndromes.len() {
        let mut d = syndromes[n];
        for i in 1..=l {
            if i < c.len() {
                d ^= gf_mul(c[i], syndromes[n - i]);
            }
        }

        if d == 0 {
            m += 1;
        } else {
            let t = c.clone();
            let scale = gf_div(d, b_scale);
            let needed = m + b.len();
            if c.len() < needed {
                c.resize(needed, 0);
            }
            for (i, &coef) in b.iter().enumerate() {
                c[m + i] ^= gf_mul(scale, coef);
            }

            if 2 * l <= n {
                l = n + 1 - l;
                b = t;
                b_scale = d;
                m = 1;
            } else {
                m += 1;
            }
        }
    }
    c
}

/// Chien search: evaluate roots of Lambda(x) to determine error locations.
pub fn rs_chien_search(locator: &[u8], n: usize) -> Result<Vec<usize>, RrcError> {
    let mut err_pos = Vec::new();
    for j in 0..n {
        let power = (255 - ((n - 1 - j) % 255)) % 255;
        let z = EXP_TABLE[power];
        let mut val = 0u8;
        let mut term = 1u8;
        for &coef in locator {
            val ^= gf_mul(coef, term);
            term = gf_mul(term, z);
        }
        if val == 0 {
            err_pos.push(j);
        }
    }

    // Locator degree (ignoring trailing zeros) must match number of found roots
    let deg = locator.iter().rposition(|&c| c != 0).unwrap_or(0);
    if err_pos.len() != deg {
        return Err(RrcError::EccUncorrectable);
    }
    Ok(err_pos)
}

/// Forney's algorithm: calculate error magnitudes for identified error positions.
pub fn rs_forney(syndromes: &[u8], locator: &[u8], err_pos: &[usize], n: usize) -> Vec<(usize, u8)> {
    let full_omega = poly_mul(syndromes, locator);
    let omega = if full_omega.len() > syndromes.len() {
        &full_omega[..syndromes.len()]
    } else {
        &full_omega[..]
    };

    let mut loc_prime = vec![0u8; locator.len()];
    for i in (1..locator.len()).step_by(2) {
        loc_prime[i - 1] = locator[i];
    }

    let mut corrections = Vec::with_capacity(err_pos.len());
    for &j in err_pos {
        let x_k = EXP_TABLE[(n - 1 - j) % 255];
        let z = EXP_TABLE[(255 - ((n - 1 - j) % 255)) % 255];

        let mut omega_val = 0u8;
        let mut term = 1u8;
        for &c in omega {
            omega_val ^= gf_mul(c, term);
            term = gf_mul(term, z);
        }

        let mut loc_prime_val = 0u8;
        let mut term_p = 1u8;
        for &c in &loc_prime {
            loc_prime_val ^= gf_mul(c, term_p);
            term_p = gf_mul(term_p, z);
        }

        if loc_prime_val == 0 {
            continue;
        }

        let mag = gf_mul(x_k, gf_div(omega_val, loc_prime_val));
        corrections.push((j, mag));
    }
    corrections
}

/// Decode a single Reed-Solomon codeword block.
/// Corrects up to `ecc_len / 2` errors. Returns the corrected data codewords.
pub fn rs_decode_block(codeword: &[u8], ecc_len: usize) -> Result<Vec<u8>, RrcError> {
    if codeword.len() < ecc_len {
        return Err(RrcError::EccUncorrectable);
    }
    let data_len = codeword.len() - ecc_len;
    let syn = rs_calc_syndromes(codeword, ecc_len);

    if syn.iter().all(|&s| s == 0) {
        return Ok(codeword[..data_len].to_vec());
    }

    let locator = rs_berlekamp_massey(&syn);
    let positions = rs_chien_search(&locator, codeword.len())?;
    if positions.len() > ecc_len / 2 {
        return Err(RrcError::EccUncorrectable);
    }

    let corrections = rs_forney(&syn, &locator, &positions, codeword.len());
    let mut corrected = codeword.to_vec();
    for (pos, mag) in corrections {
        corrected[pos] ^= mag;
    }

    let syn_check = rs_calc_syndromes(&corrected, ecc_len);
    if syn_check.iter().any(|&s| s != 0) {
        return Err(RrcError::EccUncorrectable);
    }

    Ok(corrected[..data_len].to_vec())
}

/// Decode a single Reed-Solomon codeword block with optional known erasure positions.
/// Corrects up to `2e + v <= ecc_len` where `e` is unknown errors and `v` is known erasures.
pub fn rs_decode_block_erasures(
    codeword: &[u8],
    ecc_len: usize,
    erasures: &[usize],
) -> Result<Vec<u8>, RrcError> {
    if codeword.len() < ecc_len || erasures.len() > ecc_len {
        return Err(RrcError::EccUncorrectable);
    }
    let data_len = codeword.len() - ecc_len;
    let n = codeword.len();
    let syn = rs_calc_syndromes(codeword, ecc_len);

    if syn.iter().all(|&s| s == 0) {
        return Ok(codeword[..data_len].to_vec());
    }

    if erasures.is_empty() {
        return rs_decode_block(codeword, ecc_len);
    }

    // Build erasure locator polynomial Gamma(x) = prod (1 - X_k * x)
    let mut gamma = vec![1u8];
    for &j in erasures {
        let x_k = EXP_TABLE[(n - 1 - j) % 255];
        gamma = poly_mul(&gamma, &[1, x_k]);
    }

    let locator = if erasures.len() == ecc_len {
        gamma
    } else {
        let s_star = poly_mul(&syn, &gamma);
        let s_mod = if s_star.len() > ecc_len {
            &s_star[..ecc_len]
        } else {
            &s_star[..]
        };
        let lambda = rs_berlekamp_massey(s_mod);
        poly_mul(&lambda, &gamma)
    };

    let all_positions = rs_chien_search(&locator, n)?;
    if all_positions.len() > ecc_len {
        return Err(RrcError::EccUncorrectable);
    }

    let corrections = rs_forney(&syn, &locator, &all_positions, n);
    let mut corrected = codeword.to_vec();
    for (pos, mag) in corrections {
        corrected[pos] ^= mag;
    }

    let syn_check = rs_calc_syndromes(&corrected, ecc_len);
    if syn_check.iter().any(|&s| s != 0) {
        return Err(RrcError::EccUncorrectable);
    }

    Ok(corrected[..data_len].to_vec())
}

/// Decode full message across one or more interleaved Reed-Solomon blocks with erasures.
pub fn rs_decode_erasures(
    received: &[u8],
    total_codewords: usize,
    ecc_codewords: usize,
    erasures: &[usize],
) -> Result<Vec<u8>, RrcError> {
    if received.len() != total_codewords {
        return Err(RrcError::TruncatedBitstream);
    }
    let data_codewords = total_codewords - ecc_codewords;
    let num_blocks = (total_codewords + 254) / 255;

    if num_blocks <= 1 {
        return rs_decode_block_erasures(received, ecc_codewords, erasures);
    }

    // If multi-block, partition erasures into respective blocks
    let base_data = data_codewords / num_blocks;
    let extra_data = data_codewords % num_blocks;
    let base_ecc = ecc_codewords / num_blocks;
    let extra_ecc = ecc_codewords % num_blocks;

    let mut data_lens = vec![base_data; num_blocks];
    for (i, len) in data_lens.iter_mut().enumerate() {
        if i >= num_blocks - extra_data {
            *len += 1;
        }
    }

    let mut ecc_lens = vec![base_ecc; num_blocks];
    for (i, len) in ecc_lens.iter_mut().enumerate() {
        if i >= num_blocks - extra_ecc {
            *len += 1;
        }
    }

    // De-interleave data
    let mut data_blocks = vec![Vec::new(); num_blocks];
    let mut idx = 0;
    let max_d = *data_lens.iter().max().unwrap_or(&0);
    for col in 0..max_d {
        for (b, len) in data_lens.iter().enumerate() {
            if col < *len {
                data_blocks[b].push(received[idx]);
                idx += 1;
            }
        }
    }

    // De-interleave ECC
    let mut ecc_blocks = vec![Vec::new(); num_blocks];
    let max_e = *ecc_lens.iter().max().unwrap_or(&0);
    for col in 0..max_e {
        for (b, len) in ecc_lens.iter().enumerate() {
            if col < *len {
                ecc_blocks[b].push(received[idx]);
                idx += 1;
            }
        }
    }

    // Decode each block
    let mut out_data = Vec::with_capacity(data_codewords);
    for b in 0..num_blocks {
        let mut block_codeword = data_blocks[b].clone();
        block_codeword.extend_from_slice(&ecc_blocks[b]);
        let corrected = rs_decode_block(&block_codeword, ecc_lens[b])?;
        out_data.extend_from_slice(&corrected);
    }

    Ok(out_data)
}

/// Decode full message across one or more interleaved Reed-Solomon blocks without erasure hints.
pub fn rs_decode(received: &[u8], total_codewords: usize, ecc_codewords: usize) -> Result<Vec<u8>, RrcError> {
    rs_decode_erasures(received, total_codewords, ecc_codewords, &[])
}
pub fn rs_encode(data: &[u8], total_codewords: usize, ecc_codewords: usize) -> Vec<u8> {
    let data_codewords = total_codewords - ecc_codewords;
    assert_eq!(data.len(), data_codewords, "Data length must match data_codewords");

    let num_blocks = (total_codewords + 254) / 255;
    if num_blocks <= 1 {
        let ecc = rs_encode_block(data, ecc_codewords);
        let mut out = data.to_vec();
        out.extend_from_slice(&ecc);
        return out;
    }

    // Multi-block partition and interleaving
    let base_data = data_codewords / num_blocks;
    let extra_data = data_codewords % num_blocks;
    let base_ecc = ecc_codewords / num_blocks;
    let extra_ecc = ecc_codewords % num_blocks;

    let mut data_blocks = Vec::with_capacity(num_blocks);
    let mut ecc_blocks = Vec::with_capacity(num_blocks);
    let mut offset = 0;

    for i in 0..num_blocks {
        let d_len = base_data + if i >= num_blocks - extra_data { 1 } else { 0 };
        let e_len = base_ecc + if i >= num_blocks - extra_ecc { 1 } else { 0 };
        let block_data = &data[offset..offset + d_len];
        offset += d_len;
        let block_ecc = rs_encode_block(block_data, e_len);
        data_blocks.push(block_data.to_vec());
        ecc_blocks.push(block_ecc);
    }

    // Interleave data codewords
    let mut out = Vec::with_capacity(total_codewords);
    let max_d = data_blocks.iter().map(|b| b.len()).max().unwrap_or(0);
    for col in 0..max_d {
        for block in &data_blocks {
            if col < block.len() {
                out.push(block[col]);
            }
        }
    }

    // Interleave ECC codewords
    let max_e = ecc_blocks.iter().map(|b| b.len()).max().unwrap_or(0);
    for col in 0..max_e {
        for block in &ecc_blocks {
            if col < block.len() {
                out.push(block[col]);
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gf_arithmetic() {
        assert_eq!(gf_add(0x57, 0x83), 0xD4);
        assert_eq!(gf_mul(0, 50), 0);
        assert_eq!(gf_mul(1, 250), 250);
        assert_eq!(gf_mul(2, 1), 2);
        assert_eq!(gf_div(100, 100), 1);
        for a in 1..=255 {
            let inv = gf_inv(a);
            assert_eq!(gf_mul(a, inv), 1, "Inverse check for {a}");
        }
    }

    #[test]
    fn test_rs_single_block_correction() {
        let msg = b"Radial Response Code 2026";
        let ecc_len = 10; // can correct up to 5 byte errors
        let ecc = rs_encode_block(msg, ecc_len);

        let mut codeword = msg.to_vec();
        codeword.extend_from_slice(&ecc);

        // Inject 5 errors
        codeword[0] ^= 0x42;
        codeword[5] ^= 0x99;
        codeword[12] ^= 0x11;
        codeword[20] ^= 0xFF;
        codeword[30] ^= 0x7E;

        let recovered = rs_decode_block(&codeword, ecc_len).expect("Decode should recover 5 errors");
        assert_eq!(recovered, msg);
    }

    #[test]
    fn test_rs_interleaved_multi_block() {
        let mut msg = Vec::new();
        for i in 0..400 {
            msg.push((i * 7 % 256) as u8);
        }
        let ecc_len = 80;
        let total = msg.len() + ecc_len;
        let encoded = rs_encode(&msg, total, ecc_len);

        let mut corrupted = encoded.clone();
        // Corrupt several bytes across blocks
        for i in (0..total).step_by(30) {
            corrupted[i] ^= 0xAA;
        }

        let recovered = rs_decode(&corrupted, total, ecc_len).expect("Multi-block decode should succeed");
        assert_eq!(recovered, msg);
    }

    #[test]
    fn test_rs_uncorrectable_fails_gracefully() {
        let msg = b"Testing ECC limits";
        let ecc_len = 4; // can correct at most 2 errors
        let ecc = rs_encode_block(msg, ecc_len);

        let mut codeword = msg.to_vec();
        codeword.extend_from_slice(&ecc);

        // Corrupt 3 positions (exceeds t/2)
        codeword[1] ^= 0x01;
        codeword[2] ^= 0x02;
        codeword[3] ^= 0x03;

        let res = rs_decode_block(&codeword, ecc_len);
        assert!(res.is_err(), "Should detect uncorrectable errors");
    }
}
