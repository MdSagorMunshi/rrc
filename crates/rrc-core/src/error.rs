use thiserror::Error;

/// Error type returned by RRC encoding, decoding, and rendering operations.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RrcError {
    /// The input data exceeds the maximum capacity of the chosen or largest version.
    #[error("Data too large for RRC capacity: needed {needed} bits, available {capacity} bits")]
    DataTooLarge { needed: usize, capacity: usize },

    /// The specified version is invalid (must be between 1 and 40).
    #[error("Invalid RRC version: {0} (supported versions: 1–40)")]
    InvalidVersion(u8),

    /// The specified ECC level is invalid.
    #[error("Invalid ECC level: {0}")]
    InvalidEccLevel(String),

    /// The input string contains characters not supported by the selected mode.
    #[error("Invalid character '{0}' for the selected encoding mode")]
    InvalidCharacter(char),

    /// Unrecognized mode indicator in the bitstream.
    #[error("Unknown or unsupported mode indicator: {0:#06b}")]
    InvalidMode(u8),

    /// The bitstream length is invalid or incomplete.
    #[error("Truncated or malformed bitstream")]
    TruncatedBitstream,

    /// Format word could not be decoded or was corrupted beyond BCH recovery.
    #[error("Format word corrupted and unrecoverable")]
    FormatWordCorrupted,

    /// Reed-Solomon error correction failed because errors exceeded recovery capacity.
    #[error("Reed-Solomon decoding failed: uncorrectable errors detected in codeword block")]
    EccUncorrectable,

    /// No RRC bullseye finder pattern was detected in the image.
    #[error("RRC finder pattern (bullseye) not found in image")]
    NotFound,

    /// The input image buffer is too small or dimensions are invalid.
    #[error("Input image is too small or invalid dimensions ({width}x{height})")]
    ImageTooSmall { width: u32, height: u32 },

    /// General decoding or sampling failure.
    #[error("RRC decoding failure: {0}")]
    DecodingFailed(String),

    /// An error occurred during image rasterization or encoding.
    #[error("Rendering error: {0}")]
    RenderError(String),
}
