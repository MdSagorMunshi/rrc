uniffi::include_scaffolding!("rrc");

use rrc_core::{
    decode as core_decode, encode as core_encode, render_png, render_svg,
    version_info as core_version_info, DecodeOptions, EccLevel, EncodeOptions,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RrcKotlinError {
    #[error("Encoding error: {0}")]
    EncodeError(String),
    #[error("Decoding error: {0}")]
    DecodeError(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

#[derive(Debug, Clone)]
pub struct DecodeResult {
    pub payload: Vec<u8>,
    pub text: Option<String>,
    pub version: u8,
    pub ecc_level: String,
    pub mode: String,
}

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: u8,
    pub ring_count: u32,
    pub total_data_bits: u32,
    pub total_codewords: u32,
    pub capacity_bytes_m: u32,
}

fn parse_ecc(ecc_str: &str) -> Result<EccLevel, RrcKotlinError> {
    match ecc_str.trim().to_uppercase().as_str() {
        "L" => Ok(EccLevel::L),
        "M" => Ok(EccLevel::M),
        "Q" => Ok(EccLevel::Q),
        "H" => Ok(EccLevel::H),
        _ => Err(RrcKotlinError::InvalidParameter(format!(
            "Invalid ECC level '{ecc_str}', expected L, M, Q, or H"
        ))),
    }
}

pub fn encode_svg(
    data: Vec<u8>,
    ecc_level: String,
    version: Option<u8>,
) -> Result<String, RrcKotlinError> {
    let mut opts = EncodeOptions::default();
    opts.ecc_level = parse_ecc(&ecc_level)?;
    opts.version = version;

    let sym = core_encode(&data, &opts)
        .map_err(|e| RrcKotlinError::EncodeError(e.to_string()))?;
    Ok(render_svg(sym.version, &sym.matrix, &sym.style))
}

pub fn encode_png(
    data: Vec<u8>,
    ecc_level: String,
    version: Option<u8>,
) -> Result<Vec<u8>, RrcKotlinError> {
    let mut opts = EncodeOptions::default();
    opts.ecc_level = parse_ecc(&ecc_level)?;
    opts.version = version;

    let sym = core_encode(&data, &opts)
        .map_err(|e| RrcKotlinError::EncodeError(e.to_string()))?;
    render_png(sym.version, &sym.matrix, &sym.style)
        .map_err(|e| RrcKotlinError::EncodeError(e.to_string()))
}

pub fn decode_rgba(
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    expected_version: Option<u8>,
) -> Result<DecodeResult, RrcKotlinError> {
    let dec_opts = DecodeOptions {
        verbose: false,
        expected_version,
    };

    let res = core_decode(&rgba, width, height, &dec_opts)
        .map_err(|e| RrcKotlinError::DecodeError(e.to_string()))?;

    Ok(DecodeResult {
        payload: res.payload,
        text: res.text,
        version: res.version,
        ecc_level: format!("{:?}", res.ecc_level),
        mode: format!("{:?}", res.mode),
    })
}

pub fn get_version_info(version: u8) -> Result<VersionInfo, RrcKotlinError> {
    if !(1..=40).contains(&version) {
        return Err(RrcKotlinError::InvalidParameter(
            "Version must be between 1 and 40".into(),
        ));
    }

    let info = core_version_info(version)
        .map_err(|e| RrcKotlinError::InvalidParameter(e.to_string()))?;

    Ok(VersionInfo {
        version,
        ring_count: info.ring_count as u32,
        total_data_bits: info.total_data_bits as u32,
        total_codewords: info.total_codewords as u32,
        capacity_bytes_m: info.capacity_bytes_m as u32,
    })
}
