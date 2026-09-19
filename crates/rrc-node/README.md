# rrcjs (Radial Response Code)

High-performance JavaScript, TypeScript & Node.js bindings for **Radial Response Code (RRC)** — an original polar 2D optical code format with 360-degree rotation invariance, concentric annular data rings, and Reed-Solomon error correction over $\text{GF}(2^8)$.

Powered by Rust and N-API (`napi-rs`).

## Installation

```bash
npm install rrcjs
```

## Quick Start (TypeScript / ESM)

```typescript
import { encodeSvg, encodePng, decodeRgba, getVersionInfo } from 'rrcjs';
import * as fs from 'fs';

// Query version capacity specs
const v1 = getVersionInfo(1);
console.log(`V1 rings: ${v1.ringCount}, data bytes: ${v1.capacityBytesM}`);

// 1. Generate Vector SVG
const svg = encodeSvg('https://github.com/MdSagorMunshi/rrc', {
  ecc: 'H',
  style: 'rounded',
  darkColor: '#00FF94',
  lightColor: '#0A0A0A',
});
fs.writeFileSync('badge.svg', svg);

// 2. Generate Raster PNG Buffer
const png = encodePng('https://github.com/MdSagorMunshi/rrc', {
  ecc: 'M',
  renderScale: 12,
});
fs.writeFileSync('badge.png', png);

// 3. Decode an RGBA Buffer
// const result = decodeRgba(rawRgbaBuffer, width, height);
// console.log(`Decoded text: ${result.text}`);
```

## Documentation & Specification

- **GitHub Repository:** [https://github.com/MdSagorMunshi/rrc](https://github.com/MdSagorMunshi/rrc)
- **Specification:** [RFC 2026-RRC (SPEC.md)](https://github.com/MdSagorMunshi/rrc/blob/main/SPEC.md)
- **Full Integration Guide:** [INTEGRATION.md](https://github.com/MdSagorMunshi/rrc/blob/main/INTEGRATION.md)

## License

Apache-2.0. Author: Ryan Shelby (`MdSagorMunshi`).
