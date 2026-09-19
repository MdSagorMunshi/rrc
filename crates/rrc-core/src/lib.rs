//! # RRC (Radial Response Code) Core Library
//!
//! RRC is a completely original, open-source, two-dimensional optical code format and scanner system
//! featuring a polar (concentric-ring) geometry, rotation-invariant bullseye detection,
//! from-scratch Reed-Solomon error correction over GF(2^8), multi-pattern masking, and rich visual styling.
//!
//! ## Key Architectural Features
//! - **Pure Radial Geometry:** Concentric data rings with sectors dividing 360° evenly, indexed by (ring, sector).
//! - **Concentric Bullseye Finder:** 7 alternating dark/light zones at the center, providing orientation-free detection.
//! - **From-Scratch Reed-Solomon:** Custom implementation of GF(2^8) arithmetic, Berlekamp-Massey, Chien search, and Forney algorithm.
//! - **Visual Styling Engine:** Direct support for SVG vector, PNG raster, JPEG, and high-density Unicode Braille / ASCII output.

pub mod bitstream;
pub mod decoder;
pub mod detect;
pub mod ecc;
pub mod encoder;
pub mod error;
pub mod geometry;
pub mod mask;
pub mod render;

// Re-exports of primary APIs
pub use bitstream::Mode;
pub use decoder::{decode, DecodeOptions, DecodeResult};
pub use encoder::{encode, EncodeOptions, RrcSymbol};
pub use error::RrcError;
pub use geometry::{
    find_fitting_version, version_info, EccLevel, VersionInfo, BASE_SECTORS, BULLSEYE_RADIUS,
    BULLSEYE_ZONES, DATA_START_RADIUS, QUIET_RING_WIDTH, RING_SCALE_FACTOR, RING_THICKNESS,
    TERMINATION_RING_WIDTH, VERSION_TABLE,
};
pub use render::ascii::render_ascii;
pub use render::jpg::render_jpeg;
pub use render::png::{render_png, render_to_rgba};
pub use render::svg::render_svg;
pub use render::{Color, RrcStyle, SectorStyle};
