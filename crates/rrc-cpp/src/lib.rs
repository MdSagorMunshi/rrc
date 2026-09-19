use rrc_core::{
    decode, encode, render_png, render_svg, version_info, DecodeOptions, EccLevel, EncodeOptions,
};
use std::ffi::CString;
use std::os::raw::c_char;
use std::slice;

pub const RRC_OK: i32 = 0;
pub const RRC_ERR_NULL_PTR: i32 = -1;
pub const RRC_ERR_INVALID_ARG: i32 = -2;
pub const RRC_ERR_ENCODE: i32 = -3;
pub const RRC_ERR_DECODE: i32 = -4;

fn parse_ecc(ecc: u8) -> Option<EccLevel> {
    match ecc {
        0 => Some(EccLevel::L),
        1 => Some(EccLevel::M),
        2 => Some(EccLevel::Q),
        3 => Some(EccLevel::H),
        _ => None,
    }
}

/// Encode data into an RRC SVG string.
/// The caller is responsible for freeing `*out_svg` with `rrc_free_string`.
#[no_mangle]
pub unsafe extern "C" fn rrc_encode_svg(
    data: *const u8,
    data_len: usize,
    ecc: u8,
    version: u8,
    out_svg: *mut *mut c_char,
) -> i32 {
    if data.is_null() || out_svg.is_null() {
        return RRC_ERR_NULL_PTR;
    }

    let ecc_level = match parse_ecc(ecc) {
        Some(e) => e,
        None => return RRC_ERR_INVALID_ARG,
    };

    let slice = slice::from_raw_parts(data, data_len);
    let mut opts = EncodeOptions::default();
    opts.ecc_level = ecc_level;
    opts.version = if version > 0 { Some(version) } else { None };

    match encode(slice, &opts) {
        Ok(sym) => {
            let svg_string = render_svg(sym.version, &sym.matrix, &sym.style);
            match CString::new(svg_string) {
                Ok(c_str) => {
                    *out_svg = c_str.into_raw();
                    RRC_OK
                }
                Err(_) => RRC_ERR_ENCODE,
            }
        }
        Err(_) => RRC_ERR_ENCODE,
    }
}

/// Encode data into an RRC PNG binary buffer.
/// The caller is responsible for freeing `*out_png` with `rrc_free_bytes`.
#[no_mangle]
pub unsafe extern "C" fn rrc_encode_png(
    data: *const u8,
    data_len: usize,
    ecc: u8,
    version: u8,
    out_png: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if data.is_null() || out_png.is_null() || out_len.is_null() {
        return RRC_ERR_NULL_PTR;
    }

    let ecc_level = match parse_ecc(ecc) {
        Some(e) => e,
        None => return RRC_ERR_INVALID_ARG,
    };

    let slice = slice::from_raw_parts(data, data_len);
    let mut opts = EncodeOptions::default();
    opts.ecc_level = ecc_level;
    opts.version = if version > 0 { Some(version) } else { None };

    match encode(slice, &opts) {
        Ok(sym) => match render_png(sym.version, &sym.matrix, &sym.style) {
            Ok(bytes) => {
                let len = bytes.len();
                let mut boxed = bytes.into_boxed_slice();
                *out_png = boxed.as_mut_ptr();
                *out_len = len;
                std::mem::forget(boxed);
                RRC_OK
            }
            Err(_) => RRC_ERR_ENCODE,
        },
        Err(_) => RRC_ERR_ENCODE,
    }
}

/// Decode an RRC symbol from an RGBA image buffer.
/// The caller is responsible for freeing `*out_payload` with `rrc_free_bytes`.
#[no_mangle]
pub unsafe extern "C" fn rrc_decode(
    rgba: *const u8,
    width: u32,
    height: u32,
    expected_version: u8,
    out_payload: *mut *mut u8,
    out_len: *mut usize,
    out_version: *mut u8,
    out_ecc: *mut u8,
) -> i32 {
    if rgba.is_null() || out_payload.is_null() || out_len.is_null() {
        return RRC_ERR_NULL_PTR;
    }

    let expected_len = (width as usize) * (height as usize) * 4;
    let slice = slice::from_raw_parts(rgba, expected_len);

    let opts = DecodeOptions {
        verbose: false,
        expected_version: if expected_version > 0 {
            Some(expected_version)
        } else {
            None
        },
    };

    match decode(slice, width, height, &opts) {
        Ok(res) => {
            let len = res.payload.len();
            let mut boxed = res.payload.into_boxed_slice();
            *out_payload = boxed.as_mut_ptr();
            *out_len = len;
            std::mem::forget(boxed);

            if !out_version.is_null() {
                *out_version = res.version;
            }
            if !out_ecc.is_null() {
                *out_ecc = match res.ecc_level {
                    EccLevel::L => 0,
                    EccLevel::M => 1,
                    EccLevel::Q => 2,
                    EccLevel::H => 3,
                };
            }
            RRC_OK
        }
        Err(_) => RRC_ERR_DECODE,
    }
}

/// Query specifications for an RRC version (1..=40).
#[no_mangle]
pub unsafe extern "C" fn rrc_version_info(
    version: u8,
    out_rings: *mut u32,
    out_bits: *mut u32,
    out_codewords: *mut u32,
    out_cap_m: *mut u32,
) -> i32 {
    match version_info(version) {
        Ok(info) => {
            if !out_rings.is_null() {
                *out_rings = info.ring_count as u32;
            }
            if !out_bits.is_null() {
                *out_bits = info.total_data_bits as u32;
            }
            if !out_codewords.is_null() {
                *out_codewords = info.total_codewords as u32;
            }
            if !out_cap_m.is_null() {
                *out_cap_m = info.capacity_bytes_m as u32;
            }
            RRC_OK
        }
        Err(_) => RRC_ERR_INVALID_ARG,
    }
}

/// Free a C string returned by RRC functions.
#[no_mangle]
pub unsafe extern "C" fn rrc_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Free a byte buffer returned by RRC functions.
#[no_mangle]
pub unsafe extern "C" fn rrc_free_bytes(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}
