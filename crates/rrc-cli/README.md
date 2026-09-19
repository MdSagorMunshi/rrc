# rrc-cli

[![crates.io](https://img.shields.io/crates/v/rrc-cli.svg)](https://crates.io/crates/rrc-cli)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/MdSagorMunshi/rrc/blob/main/LICENSE)

**Radial Response Code (RRC)** — Reference command-line tool for encoding, decoding, and inspecting RRC polar 2D optical codes.

## Installation

```bash
cargo install rrc-cli
```

## Commands & Usage

### Encode to File or Terminal
```bash
# Generate high-resolution SVG with dark neon aesthetic
rrc encode "https://github.com/MdSagorMunshi/rrc" \
  -o rrc_symbol.svg \
  --ecc H \
  --dark-color "#00FF94" \
  --light-color "#0A0A0A"

# Generate PNG raster image
rrc encode "RADIAL RESPONSE CODE" -o code.png --scale 12

# Display directly in terminal using Unicode Braille
rrc encode "https://github.com/MdSagorMunshi/rrc" --braille
```

### Decode an Image
```bash
rrc decode code.png
```

### Inspect Version Specifications
```bash
# Inspect specific version (1..=40)
rrc info 1

# View full capacity table (V1 to V40)
rrc info
```

## Documentation & Specification

- **Repository:** [https://github.com/MdSagorMunshi/rrc](https://github.com/MdSagorMunshi/rrc)
- **Specification:** [RFC 2026-RRC (SPEC.md)](https://github.com/MdSagorMunshi/rrc/blob/main/SPEC.md)

## License

Licensed under the Apache License, Version 2.0. Author: Ryan Shelby (`MdSagorMunshi`).
