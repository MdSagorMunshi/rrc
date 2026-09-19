# rrc-core

[![crates.io](https://img.shields.io/crates/v/rrc-core.svg)](https://crates.io/crates/rrc-core)
[![docs.rs](https://docs.rs/rrc-core/badge.svg)](https://docs.rs/rrc-core)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/MdSagorMunshi/rrc/blob/main/LICENSE)

**Radial Response Code (RRC)** — Pure Rust algorithmic engine for an original polar two-dimensional optical code symbology and computer vision scanner.

RRC is designed from first principles with concentric annular rings, 360-degree intrinsic rotation invariance, and from-scratch Galois Field $\text{GF}(2^8)$ Reed-Solomon and $\text{BCH}(16, 5)$ codecs.

## Features

- **Polar Geometry:** 40 versions (V1 to V40), expanding from 3 rings to 81 concentric data rings.
- **Divisor Snapping:** Sector counts strictly snapped to divisors of 360 degrees.
- **7-Zone Bullseye Finder:** Central $1:1:1:1:1:1:1$ radial ratio with subpixel centroid refinement.
- **Dual Mirrored Format Words:** $\text{BCH}(16, 5)$ format words at $0^\circ$ and $180^\circ$ for $100\%$ format redundancy.
- **Reed-Solomon $\text{GF}(2^8)$ Codec:** Berlekamp-Massey error locator, Chien root search, and Forney error magnitude computation.
- **8 Radial Masks:** Dynamic mask selection with penalty scoring.
- **Multi-Target Renderers:** Vector SVG (`<path>` annular wedges), PNG (with 2x2 supersampling AA), JPEG, and terminal Braille (`U+2800..U+28FF`).
- **Computer Vision Scanner:** 8-ray bullseye detection, subpixel refinement, polar unwrapping, and decoding.

## Quick Start

```rust
use rrc_core::{encode, decode, render_svg, render_png, EncodeOptions, DecodeOptions, EccLevel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let payload = b"https://github.com/MdSagorMunshi/rrc";

    // 1. Encode
    let mut opts = EncodeOptions::default();
    opts.ecc_level = EccLevel::M;
    let symbol = encode(payload, &opts)?;

    // 2. Render SVG
    let svg = render_svg(symbol.version, &symbol.matrix, &symbol.style);

    // 3. Render PNG
    let png_bytes = render_png(symbol.version, &symbol.matrix, &symbol.style)?;

    // 4. Decode
    let (width, height, rgba) = rrc_core::render_to_rgba(symbol.version, &symbol.matrix, &symbol.style);
    let result = decode(&rgba, width, height, &DecodeOptions::default())?;
    println!("Decoded: {}", String::from_utf8_lossy(&result.payload));

    Ok(())
}
```

## Documentation & Specification

- **Repository:** [https://github.com/MdSagorMunshi/rrc](https://github.com/MdSagorMunshi/rrc)
- **Specification:** [RFC 2026-RRC (SPEC.md)](https://github.com/MdSagorMunshi/rrc/blob/main/SPEC.md)
- **Integration Guide:** [INTEGRATION.md](https://github.com/MdSagorMunshi/rrc/blob/main/INTEGRATION.md)

## License

Licensed under the Apache License, Version 2.0. Author: Ryan Shelby (`MdSagorMunshi`).
