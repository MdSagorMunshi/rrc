pub mod ascii;
pub mod jpg;
pub mod png;
pub mod svg;

use crate::error::RrcError;

/// An RGBA color representation supporting hex strings, tuples, and luminance operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };
    pub const NEON_GREEN: Self = Self { r: 0, g: 255, b: 148, a: 255 }; // #00FF94
    pub const DARK_BG: Self = Self { r: 10, g: 10, b: 10, a: 255 };     // #0A0A0A

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Parse color from hex string: "#RRGGBB" or "#RRGGBBAA" (with or without '#').
    pub fn from_hex(s: &str) -> Result<Self, RrcError> {
        let clean = s.trim().trim_start_matches('#');
        match clean.len() {
            6 => {
                let r = u8::from_str_radix(&clean[0..2], 16)
                    .map_err(|_| RrcError::RenderError("Invalid hex color".into()))?;
                let g = u8::from_str_radix(&clean[2..4], 16)
                    .map_err(|_| RrcError::RenderError("Invalid hex color".into()))?;
                let b = u8::from_str_radix(&clean[4..6], 16)
                    .map_err(|_| RrcError::RenderError("Invalid hex color".into()))?;
                Ok(Self::new(r, g, b, 255))
            }
            8 => {
                let r = u8::from_str_radix(&clean[0..2], 16)
                    .map_err(|_| RrcError::RenderError("Invalid hex color".into()))?;
                let g = u8::from_str_radix(&clean[2..4], 16)
                    .map_err(|_| RrcError::RenderError("Invalid hex color".into()))?;
                let b = u8::from_str_radix(&clean[4..6], 16)
                    .map_err(|_| RrcError::RenderError("Invalid hex color".into()))?;
                let a = u8::from_str_radix(&clean[6..8], 16)
                    .map_err(|_| RrcError::RenderError("Invalid hex color".into()))?;
                Ok(Self::new(r, g, b, a))
            }
            _ => Err(RrcError::RenderError(format!("Invalid hex color length: '{s}'"))),
        }
    }

    /// Format as standard "#RRGGBB" hex string (or "#RRGGBBAA" if alpha < 255).
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }

    /// Return relative luminance (0.0 to 1.0).
    pub fn luminance(self) -> f32 {
        (0.299 * (self.r as f32) + 0.587 * (self.g as f32) + 0.114 * (self.b as f32)) / 255.0
    }

    /// Check if color is considered dark (luminance < 0.5).
    pub fn is_dark(self) -> bool {
        self.luminance() < 0.5
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

/// Visual style of individual data sectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SectorStyle {
    /// Clean geometric wedge edges, maximum optical contrast.
    #[default]
    Sharp,
    /// Outer and inner arcs rounded for softer aesthetics.
    Rounded,
    /// Pill/stadium shape fitting inside annular wedge bounds.
    Pill,
    /// Only the inner arc is rounded.
    InnerRounded,
}

impl SectorStyle {
    pub fn from_str_loose(s: &str) -> Result<Self, RrcError> {
        match s.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "sharp" => Ok(SectorStyle::Sharp),
            "rounded" => Ok(SectorStyle::Rounded),
            "pill" => Ok(SectorStyle::Pill),
            "inner_rounded" | "innerrounded" => Ok(SectorStyle::InnerRounded),
            _ => Err(RrcError::RenderError(format!("Unknown sector style: '{s}'"))),
        }
    }
}

/// Complete configuration for styling RRC rendered outputs.
#[derive(Debug, Clone, PartialEq)]
pub struct RrcStyle {
    /// Core foreground color for dark sectors.
    pub dark_color: Color,
    /// Core foreground color for light sectors.
    pub light_color: Color,
    /// Overall background fill color.
    pub background_color: Color,

    /// Bullseye dark zone color override (defaults to `dark_color` if None).
    pub bullseye_dark: Option<Color>,
    /// Bullseye light zone color override (defaults to `light_color` if None).
    pub bullseye_light: Option<Color>,

    /// Sector geometric shape.
    pub sector_style: SectorStyle,

    /// Gap fraction between sectors within a ring (0.0 to 0.5, default: 0.02).
    pub sector_gap: f32,
    /// Gap fraction between concentric rings (0.0 to 0.5, default: 0.01).
    pub ring_gap: f32,

    /// Quiet zone width in unit multiples (default: 2.0).
    pub quiet_zone: f32,

    /// Optional SVG element ID prefix when embedding multiple RRCs in one document.
    pub svg_id_prefix: Option<String>,

    /// Pixels per unit size for raster outputs (PNG/JPEG, default: 10).
    pub render_scale: u32,
    /// JPEG compression quality (1–100, default: 90).
    pub jpeg_quality: u8,

    /// Terminal ASCII dark character (default: '█').
    pub ascii_dark_char: char,
    /// Terminal ASCII light character (default: ' ').
    pub ascii_light_char: char,
    /// Use Unicode 2x4 Braille patterns for high-density terminal display.
    pub ascii_use_braille: bool,
}

impl Default for RrcStyle {
    fn default() -> Self {
        Self {
            dark_color: Color::BLACK,
            light_color: Color::WHITE,
            background_color: Color::WHITE,
            bullseye_dark: None,
            bullseye_light: None,
            sector_style: SectorStyle::Sharp,
            sector_gap: 0.02,
            ring_gap: 0.01,
            quiet_zone: 2.0,
            svg_id_prefix: None,
            render_scale: 10,
            jpeg_quality: 90,
            ascii_dark_char: '█',
            ascii_light_char: ' ',
            ascii_use_braille: false,
        }
    }
}

impl RrcStyle {
    /// Dark terminal aesthetic with neon green accent (#00FF94 on #0A0A0A).
    pub fn dark_neon() -> Self {
        Self {
            dark_color: Color::NEON_GREEN,
            light_color: Color::DARK_BG,
            background_color: Color::DARK_BG,
            bullseye_dark: Some(Color::NEON_GREEN),
            bullseye_light: Some(Color::DARK_BG),
            sector_style: SectorStyle::Sharp,
            sector_gap: 0.02,
            ring_gap: 0.01,
            quiet_zone: 2.0,
            svg_id_prefix: None,
            render_scale: 10,
            jpeg_quality: 90,
            ascii_dark_char: '█',
            ascii_light_char: ' ',
            ascii_use_braille: true,
        }
    }

    pub fn bullseye_dark_color(&self) -> Color {
        self.bullseye_dark.unwrap_or(self.dark_color)
    }

    pub fn bullseye_light_color(&self) -> Color {
        self.bullseye_light.unwrap_or(self.light_color)
    }
}
