use napi::bindgen_prelude::*;
use napi_derive::napi;
use rrc_core::{
    decode as core_decode, encode as core_encode, render_jpeg, render_png, render_svg,
    version_info as core_version_info, Color, DecodeOptions, EccLevel, EncodeOptions,
    SectorStyle,
};

#[napi(object)]
pub struct JsEncodeOptions {
    pub version: Option<u32>,
    pub ecc: Option<String>,
    pub style: Option<String>,
    pub dark_color: Option<String>,
    pub light_color: Option<String>,
    pub quiet_zone: Option<f64>,
    pub render_scale: Option<u32>,
    pub jpeg_quality: Option<u32>,
}

#[napi(object)]
pub struct JsDecodeResult {
    pub payload: Buffer,
    pub text: Option<String>,
    pub version: u32,
    pub ecc_level: String,
    pub mode: String,
}

#[napi(object)]
pub struct JsVersionInfo {
    pub version: u32,
    pub ring_count: u32,
    pub total_data_bits: u32,
    pub total_codewords: u32,
    pub capacity_bytes_m: u32,
}

fn parse_ecc(s: &str) -> Result<EccLevel> {
    match s.trim().to_uppercase().as_str() {
        "L" => Ok(EccLevel::L),
        "M" => Ok(EccLevel::M),
        "Q" => Ok(EccLevel::Q),
        "H" => Ok(EccLevel::H),
        _ => Err(Error::from_reason(format!("Invalid ECC level: '{s}'. Expected L, M, Q, or H"))),
    }
}

fn parse_style(s: &str) -> Result<SectorStyle> {
    match s.trim().to_lowercase().as_str() {
        "sharp" => Ok(SectorStyle::Sharp),
        "rounded" => Ok(SectorStyle::Rounded),
        "pill" => Ok(SectorStyle::Pill),
        "inner_rounded" | "innerrounded" => Ok(SectorStyle::InnerRounded),
        _ => Err(Error::from_reason(format!("Invalid sector style: '{s}'"))),
    }
}

fn build_encode_options(opts: Option<JsEncodeOptions>) -> Result<EncodeOptions> {
    let mut options = EncodeOptions::default();
    if let Some(o) = opts {
        if let Some(v) = o.version {
            if !(1..=40).contains(&v) {
                return Err(Error::from_reason("Version must be between 1 and 40"));
            }
            options.version = Some(v as u8);
        }
        if let Some(ref ecc_str) = o.ecc {
            options.ecc_level = parse_ecc(ecc_str)?;
        }
        if let Some(ref style_str) = o.style {
            options.style.sector_style = parse_style(style_str)?;
        }
        if let Some(ref dc) = o.dark_color {
            options.style.dark_color = Color::from_hex(dc)
                .map_err(|e| Error::from_reason(e.to_string()))?;
        }
        if let Some(ref lc) = o.light_color {
            options.style.light_color = Color::from_hex(lc)
                .map_err(|e| Error::from_reason(e.to_string()))?;
        }
        if let Some(qz) = o.quiet_zone {
            options.style.quiet_zone = qz as f32;
        }
        if let Some(scale) = o.render_scale {
            options.style.render_scale = scale;
        }
        if let Some(q) = o.jpeg_quality {
            options.style.jpeg_quality = q.clamp(1, 100) as u8;
        }
    }
    Ok(options)
}

/// Encode data into an RRC SVG string.
#[napi]
pub fn encode_svg(data: Either<String, Buffer>, options: Option<JsEncodeOptions>) -> Result<String> {
    let bytes: Vec<u8> = match data {
        Either::A(s) => s.into_bytes(),
        Either::B(b) => b.to_vec(),
    };
    let opts = build_encode_options(options)?;
    let sym = core_encode(&bytes, &opts).map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(render_svg(sym.version, &sym.matrix, &sym.style))
}

/// Encode data into an RRC PNG buffer.
#[napi]
pub fn encode_png(data: Either<String, Buffer>, options: Option<JsEncodeOptions>) -> Result<Buffer> {
    let bytes: Vec<u8> = match data {
        Either::A(s) => s.into_bytes(),
        Either::B(b) => b.to_vec(),
    };
    let opts = build_encode_options(options)?;
    let sym = core_encode(&bytes, &opts).map_err(|e| Error::from_reason(e.to_string()))?;
    let png_bytes = render_png(sym.version, &sym.matrix, &sym.style)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(Buffer::from(png_bytes))
}

/// Encode data into an RRC JPEG buffer.
#[napi]
pub fn encode_jpeg(data: Either<String, Buffer>, options: Option<JsEncodeOptions>) -> Result<Buffer> {
    let bytes: Vec<u8> = match data {
        Either::A(s) => s.into_bytes(),
        Either::B(b) => b.to_vec(),
    };
    let opts = build_encode_options(options)?;
    let sym = core_encode(&bytes, &opts).map_err(|e| Error::from_reason(e.to_string()))?;
    let jpg_bytes = render_jpeg(sym.version, &sym.matrix, &sym.style, None)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(Buffer::from(jpg_bytes))
}

/// Decode an RRC symbol from an RGBA buffer.
#[napi]
pub fn decode_rgba(rgba: Buffer, width: u32, height: u32, expected_version: Option<u32>) -> Result<JsDecodeResult> {
    let dec_opts = DecodeOptions {
        verbose: false,
        expected_version: expected_version.map(|v| v as u8),
    };

    let res = core_decode(&rgba, width, height, &dec_opts)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(JsDecodeResult {
        payload: Buffer::from(res.payload),
        text: res.text,
        version: res.version as u32,
        ecc_level: format!("{:?}", res.ecc_level),
        mode: format!("{:?}", res.mode),
    })
}

/// Get specifications and capacity metrics for a given RRC version.
#[napi]
pub fn get_version_info(version: u32) -> Result<JsVersionInfo> {
    if !(1..=40).contains(&version) {
        return Err(Error::from_reason("Version must be between 1 and 40"));
    }
    let info = core_version_info(version as u8).map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(JsVersionInfo {
        version,
        ring_count: info.ring_count as u32,
        total_data_bits: info.total_data_bits as u32,
        total_codewords: info.total_codewords as u32,
        capacity_bytes_m: info.capacity_bytes_m as u32,
    })
}
