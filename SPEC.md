# Radial Response Code (RRC) Symbology Specification
**Standard Specification Document — RFC 2026-RRC**  
**Version:** 1.0.0  
**Date:** September 2026  
**Author:** Ryan Shelby (`MdSagorMunshi`)  
**License:** Apache-2.0  
**Repository:** [https://github.com/MdSagorMunshi/rrc](https://github.com/MdSagorMunshi/rrc)

---

## Abstract
Radial Response Code (RRC) is an original, two-dimensional optical code format designed from first principles using a polar (radial/concentric-ring) coordinate geometry. RRC is fundamentally distinct from matrix barcodes such as QR Code, Data Matrix, or Aztec. By distributing data modules across concentric annular wedges centered on a multi-tier bullseye finder, RRC provides intrinsic 360-degree rotation invariance, deterministic angular orientation recovery via redundant format sectors, and robust Reed-Solomon error correction over Galois Field $\text{GF}(2^8)$.

---

## Table of Contents
1. [Scope & Conformance](#1-scope--conformance)
2. [Terms & Definitions](#2-terms--definitions)
3. [Mathematical Foundations & Symbol Geometry](#3-mathematical-foundations--symbol-geometry)
4. [Bullseye Finder & Alignment Pattern](#4-bullseye-finder--alignment-pattern)
5. [Format Word & Angular Calibration](#5-format-word--angular-calibration)
6. [Data Encoding & Modes](#6-data-encoding--modes)
7. [Error Correction Codewords](#7-error-correction-codewords)
8. [Radial Mask Patterns & Penalty Evaluation](#8-radial-mask-patterns--penalty-evaluation)
9. [Visual Layout & Rendering Specifications](#9-visual-layout--rendering-specifications)
10. [Scanner Detection & Decoding Pipeline](#10-scanner-detection--decoding-pipeline)
11. [Complete Version Table (V1 – V40)](#11-complete-version-table-v1--v40)
12. [Reference Implementation Directory](#12-reference-implementation-directory)

---

## 1. Scope & Conformance

### 1.1 Requirements Language
The key words "**MUST**", "**MUST NOT**", "**REQUIRED**", "**SHALL**", "**SHALL NOT**", "**SHOULD**", "**SHOULD NOT**", "**RECOMMENDED**", "**MAY**", and "**OPTIONAL**" in this document are to be interpreted as described in [RFC 2119](https://www.ietf.org/rfc/rfc2119.txt).

### 1.2 Conformance Levels
- **Conforming Encoder:** Any software or hardware system that accepts structured input data and generates valid RRC visual symbols complying with all geometric, bitstream, error correction, and masking rules defined in Chapters 3 through 9.
- **Conforming Decoder:** Any image processing and decoding system that correctly detects, calibrates, samples, error-corrects, and recovers data payloads from valid RRC symbols conforming to this specification across any arbitrary orientation angle $\theta \in [0, 2\pi)$.

---

## 2. Terms & Definitions

- **Annular Wedge:** The geometric planar region bounded between two concentric circles of radii $r_{\text{inner}}$ and $r_{\text{outer}}$ and two radial angles $\theta_{\text{start}}$ and $\theta_{\text{end}}$.
- **Bullseye Finder:** The 7-zone concentric alternating dark/light circle pattern located at the exact coordinate origin $(0, 0)$ of the symbol.
- **Concentric Data Ring:** An annular track centered at the symbol origin containing a discrete integer number of data sectors. Rings are indexed $k \in [0, K-1]$ from innermost to outermost.
- **Divisors of 360:** The set of valid sector counts for data rings: $\{24, 30, 36, 40, 45, 60, 72, 90, 120, 180, 360\}$. Every ring's sector count strictly divides 360 degrees.
- **Format Word:** A 16-bit self-protecting $\text{BCH}(16, 5)$ codeword encoding the symbol's Error Correction Level (2 bits) and Mask Function Index (3 bits), mirrored at $0^\circ$ and $180^\circ$ in Ring 0.
- **Quiet Ring:** An unencoded annular band of light modules at radius $r \in [7.0, 8.0]$ units that isolates the bullseye finder from the surrounding data rings.
- **Termination Ring:** A solid boundary ring of width 0.5 units located immediately outside the outermost data ring to prevent visual bleed into the outer quiet zone.
- **Unit (Module Width):** The fundamental dimensionless length unit $u$. The bullseye finder outer radius is exactly $7.0u$.

---

## 3. Mathematical Foundations & Symbol Geometry

### 3.1 Polar Coordinate System
Every point on the RRC symbol plane is parameterized by polar coordinates $(r, \theta)$ relative to the bullseye center $(c_x, c_y)$:

$$r = \sqrt{(x - c_x)^2 + (y - c_y)^2}$$
$$\theta = \text{atan2}(x - c_x, -(y - c_y)) \pmod{2\pi}$$

Where $\theta = 0$ corresponds to the upward vertical ray ($12\text{ o'clock}$ position), and $\theta$ increases clockwise.

### 3.2 Ring Dimensions
Let $K$ be the total number of concentric data rings for Version $V \in [1, 40]$:

$$K = 3 + (V - 1) \cdot 2$$

Each data ring $k \in [0, K-1]$ occupies an annular width $w_{\text{ring}} = 1.0u$:

$$r_{\text{inner}}(k) = R_{\text{quiet}} + k \cdot w_{\text{ring}} = 8.0 + k$$
$$r_{\text{outer}}(k) = r_{\text{inner}}(k) + 1.0 = 9.0 + k$$
$$r_{\text{mid}}(k) = r_{\text{inner}}(k) + 0.5$$

### 3.3 Sector Count Snapping
The nominal number of sectors in ring $k$ is given by:

$$S_{\text{raw}}(k) = S_0 + \text{round}(k \cdot \text{scale})$$

Where $S_0 = 72$ and $\text{scale} = 0.125 \times 72 = 9$. To enforce polar periodicity and avoid fractional sector misalignments, $S_{\text{raw}}(k)$ MUST be snapped to the nearest value in the valid divisor set $\mathcal{D}_{360}$:

$$\mathcal{D}_{360} = \{24, 30, 36, 40, 45, 60, 72, 90, 120, 180, 360\}$$
$$S_k = \arg\min_{d \in \mathcal{D}_{360}} |d - S_{\text{raw}}(k)|$$

The angular span of each sector in ring $k$ is:

$$\Delta \theta_k = \frac{2\pi}{S_k}$$

The physical arc length along the centerline is:

$$L_k = r_{\text{mid}}(k) \cdot \Delta \theta_k$$

### 3.4 Symbol Outer Bounds
The total radius of the symbol $R_{\text{total}}$ including finder, quiet ring, data rings, termination ring, and quiet zone is:

$$R_{\text{total}} = 8.0 + K \cdot 1.0 + 0.5 + Q$$

Where $Q$ is the outer quiet zone width ($Q \ge 2.0u$).

---

## 4. Bullseye Finder & Alignment Pattern

### 4.1 Zone Proportions
The central finder pattern consists of 7 alternating concentric zones with a precise $1:1:1:1:1:1:1$ radial width ratio:

| Zone | Radius Interval $[r_a, r_b)$ | State | Visual Appearance |
| :---: | :---: | :---: | :---: |
| 0 | $[0.0u, 1.0u)$ | Dark | Solid Central Disk |
| 1 | $[1.0u, 2.0u)$ | Light | Annular Ring 1 |
| 2 | $[2.0u, 3.0u)$ | Dark | Annular Ring 2 |
| 3 | $[3.0u, 4.0u)$ | Light | Annular Ring 3 |
| 4 | $[4.0u, 5.0u)$ | Dark | Annular Ring 4 |
| 5 | $[5.0u, 6.0u)$ | Light | Annular Ring 5 |
| 6 | $[6.0u, 7.0u)$ | Dark | Outer Finder Ring |
| 7 | $[7.0u, 8.0u)$ | Light | **Concentric Quiet Ring** |

```
               [ Bullseye Cross-Section ]
  Dark   Light   Dark   Light   Dark   Light   Dark (Center)
 | 1u  |  1u   |  1u  |  1u   |  1u  |  1u   |     2u      | ...
```

### 4.2 Multi-Ray Bullseye Detection
A decoder MUST evaluate 8 radial rays ($0^\circ, 45^\circ, 90^\circ, 135^\circ, 180^\circ, 225^\circ, 270^\circ, 315^\circ$) intersecting candidate centers. The run-length ratio along any line passing through $(c_x, c_y)$ MUST match $1:1:1:1:1:1:1$ within an acceptance tolerance of $\pm 40\%$.

### 4.3 Subpixel Centroid Refinement
Upon locating candidate pixels, the exact subpixel center $(\bar{x}, \bar{y})$ SHALL be calculated using intensity-weighted moments over the central disk:

$$\bar{x} = \frac{\sum_{(x, y) \in \mathcal{C}} x \cdot (255 - I(x, y))}{\sum_{(x, y) \in \mathcal{C}} (255 - I(x, y))}$$
$$\bar{y} = \frac{\sum_{(x, y) \in \mathcal{C}} y \cdot (255 - I(x, y))}{\sum_{(x, y) \in \mathcal{C}} (255 - I(x, y))}$$

Where $I(x, y)$ is the grayscale pixel intensity ($0 = \text{black}, 255 = \text{white}$).

---

## 5. Format Word & Angular Calibration

### 5.1 Ring 0 Sector Allocation
Ring 0 contains exactly 72 sectors ($5.0^\circ$ per sector). Its layout is strictly designated as follows:

- **Sector 0 ($0.0^\circ \to 5.0^\circ$):** Primary Reference Gap (**MUST** be dark).
- **Sectors 1..16 ($5.0^\circ \to 85.0^\circ$):** Primary Format Word (16 bits, MSB first).
- **Sectors 17..35 ($85.0^\circ \to 180.0^\circ$):** Ring 0 Data Sectors (19 data bits).
- **Sector 36 ($180.0^\circ \to 185.0^\circ$):** Mirrored Reference Gap (**MUST** be dark).
- **Sectors 37..52 ($185.0^\circ \to 265.0^\circ$):** Mirrored Format Word (16 bits, exact duplicate).
- **Sectors 53..71 ($265.0^\circ \to 360.0^\circ$):** Ring 0 Data Sectors (19 data bits).

Total usable data bits in Ring 0: $19 + 19 = 38$ bits.

### 5.2 Format Word Structure
The Format Word is a 16-bit sequence constructed from 5 information bits:
- **Bits 4..3 (2 bits):** Error Correction Level
  - `00`: Level L (~7% recovery)
  - `01`: Level M (~15% recovery, default)
  - `10`: Level Q (~25% recovery)
  - `11`: Level H (~30% recovery)
- **Bits 2..0 (3 bits):** Radial Mask Function Index (`000` to `111`).

### 5.3 BCH(16, 5) Error Correction
The 5 information bits are protected by 11 error correction parity bits using a cyclic Bose-Chaudhuri-Hocquenghem $\text{BCH}(16, 5)$ code capable of correcting up to 3 bit errors.
The generator polynomial $g_{\text{BCH}}(x)$ is:

$$g_{\text{BCH}}(x) = x^{11} + x^{10} + x^9 + x^8 + x^7 + x^6 + x^5 + x^3 + x + 1 \quad (0\text{x}17E5)$$

Parity bits are computed via polynomial division:

$$p(x) = (d(x) \cdot x^{11}) \pmod{g_{\text{BCH}}(x)}$$
$$\text{raw\_word} = (d \ll 11) \oplus p$$

To prevent all-zero sequences, the 16-bit word is XORed with the alternating mask:

$$\text{format\_word} = \text{raw\_word} \oplus 0\text{x}5555$$

---

## 6. Data Encoding & Modes

### 6.1 Encoding Modes
RRC supports 4 distinct data compaction modes:

| Mode Name | Mode Indicator | Character Set | Compaction Ratio |
| :---: | :---: | :---: | :---: |
| **Numeric** | `0001` | Digits `0`–`9` | 3 digits $\to$ 10 bits |
| **Alphanumeric** | `0010` | 45 characters: `0-9`, `A-Z`, ` `, `$`, `%`, `*`, `+`, `-`, `.`, `/`, `:` | 2 chars $\to$ 11 bits |
| **Byte** | `0100` | ISO/IEC 8859-1 / Raw 8-bit Octets | 1 byte $\to$ 8 bits |
| **Extended** | `0111` | UTF-8 String / Arbitrary Binary Streams | 1 byte $\to$ 8 bits |

### 6.2 Character Count Indicator Bit Lengths
The length of the character count indicator depends on the active mode and version range:

| Mode | Versions 1–9 | Versions 10–26 | Versions 27–40 |
| :---: | :---: | :---: | :---: |
| Numeric | 10 bits | 12 bits | 14 bits |
| Alphanumeric | 9 bits | 11 bits | 13 bits |
| Byte | 8 bits | 16 bits | 16 bits |
| Extended | 16 bits | 16 bits | 24 bits |

### 6.3 Padding & Bitstream Termination
1. **Terminator:** After the final data payload bit, a 4-bit terminator `0000` SHALL be appended (truncated if remaining symbol capacity is $< 4$ bits).
2. **Byte Alignment:** Zero bits (`0`) SHALL be appended until the total bitstream length is a multiple of 8.
3. **Pad Bytes:** If the byte count is less than the required data codeword capacity $N_{\text{data}}$, the bitstream SHALL be padded by alternating pad bytes $0\text{xEC}$ and $0\text{x11}$:
   $$0\text{xEC} = 11101100_2, \quad 0\text{x11} = 00010001_2$$

---

## 7. Error Correction Codewords

### 7.1 Galois Field $\text{GF}(2^8)$
All Reed-Solomon calculations are performed in the finite field $\text{GF}(2^8)$ defined by the irreducible field polynomial:

$$p(x) = x^8 + x^4 + x^3 + x^2 + 1 \quad (0\text{x}11D = 285)$$

With primitive root generator $\alpha = 0\text{x}02$.

### 7.2 Generator Polynomial
The Reed-Solomon generator polynomial for a block with $2t$ parity codewords is:

$$g_{\text{RS}}(x) = \prod_{i=0}^{2t-1} (x - \alpha^i)$$

### 7.3 Multi-Block Partitioning and Interleaving
When the total codeword count $N$ exceeds single-block limits ($> 30$ codewords), codewords MUST be partitioned into $B$ interleaved blocks. Let:
- $N_{\text{block}} = \lfloor N / B \rfloor$
- $R = N \pmod B$
- First $B - R$ blocks have length $N_{\text{block}}$
- Last $R$ blocks have length $N_{\text{block}} + 1$

Blocks are encoded independently, then data codewords are interleaved cyclically into the output bitstream, followed by interleaved error correction parity codewords.

---

## 8. Radial Mask Patterns & Penalty Evaluation

### 8.1 Radial Mask Functions
To balance dark/light module distribution and eliminate deceptive false alignment signatures, the symbol bitstream MUST be XORed with one of 8 radial mask functions $M_m(k, s)$:

| Index | Formula $M_m(k, s)$ | Geometric Effect |
| :---: | :---: | :---: |
| 0 | $(k + s) \pmod 2 == 0$ | Alternating checkerboard along rings |
| 1 | $k \pmod 2 == 0$ | Concentric alternating ring bands |
| 2 | $s \pmod 3 == 0$ | Radial sector spokes (every 3rd sector) |
| 3 | $(k + s) \pmod 3 == 0$ | Stepped spiral bands |
| 4 | $(\lfloor k / 2 \rfloor + \lfloor s / 3 \rfloor) \pmod 2 == 0$ | Polar macro-blocks |
| 5 | $((k \cdot s) \pmod 2 + (k \cdot s) \pmod 3) == 0$ | Logarithmic spiral moiré dispersion |
| 6 | $((k + s) \pmod 2 + (k \cdot s) \pmod 3) \pmod 2 == 0$ | Non-linear polar diffusion |
| 7 | $((k \cdot s) \pmod 3 + (k + s) \pmod 2) \pmod 2 == 0$ | Pseudo-random radial hashing |

### 8.2 Penalty Evaluation Function
The encoder MUST evaluate all 8 masks and select the mask that minimizes total penalty $P_{\text{total}}$:

$$P_{\text{total}} = P_1 + P_2 + P_3$$

1. **Angular Adjacency Runs ($P_1$):** For every contiguous run of identical modules $\ge 5$ along an annular ring:
   $$P_1 = \sum (3 + (\text{run\_len} - 5))$$
2. **Radial Concentric Stripes ($P_2$):** For identical modules aligned radially across adjacent rings at matching normalized angles:
   $$P_2 = \sum (\text{radial\_run} \ge 4) \times 5$$
3. **Symbol Balance Deviation ($P_3$):** If the percentage of dark modules $D\%$ deviates from 50%:
   $$P_3 = 10 \times \left\lfloor \frac{|D - 50|}{5} \right\rfloor$$

---

## 9. Visual Layout & Rendering Specifications

### 9.1 Annular Wedge Path Construction
Every dark module in ring $k$ spanning $[\theta_1, \theta_2]$ is rendered as a closed vector path:

```
         (x2, y2) ─────────── (x3, y3)   <- Outer Arc (r_outer)
            │                    │
            │     Annular        │
            │      Wedge         │
            │                    │
         (x1, y1) ─────────── (x4, y4)   <- Inner Arc (r_inner)
```

$$\text{Inner Arc Start: } (c_x + r_{\text{in}}\sin\theta_1, c_y - r_{\text{in}}\cos\theta_1)$$
$$\text{Outer Arc Start: } (c_x + r_{\text{out}}\sin\theta_1, c_y - r_{\text{out}}\cos\theta_1)$$
$$\text{Outer Arc End: } (c_x + r_{\text{out}}\sin\theta_2, c_y - r_{\text{out}}\cos\theta_2)$$
$$\text{Inner Arc End: } (c_x + r_{\text{in}}\sin\theta_2, c_y - r_{\text{in}}\cos\theta_2)$$

### 9.2 Sector Styles
An encoder **MAY** support the following sector styles:
- `Sharp`: Exact annular wedge with sharp radial vertices.
- `Rounded`: Filleted corners along inner and outer arcs.
- `Pill`: Stadium-shaped capsule centered inside the wedge cell.
- `InnerRounded`: Outer arc sharp, inner arc filleted.

---

## 10. Scanner Detection & Decoding Pipeline

```
  [ Input Image ]
        │
        ▼
  [ 1. Bullseye Detection (8-Ray Scan, 1:1:1:1:1:1:1 Ratio) ]
        │
        ▼
  [ 2. Subpixel Centroid Refinement (Intensity Moments) ]
        │
        ▼
  [ 3. Polar Scan & Angular Calibration (Locate Ring 0 Ref Gaps at 0° and 180°) ]
        │
        ▼
  [ 4. BCH(16, 5) Format Word Decoding (Recover ECC Level & Mask Index) ]
        │
        ▼
  [ 5. Sector Sampling & Adaptive Thresholding ]
        │
        ▼
  [ 6. Unmasking & Reed-Solomon Error Correction over GF(2^8) ]
        │
        ▼
  [ 7. Bitstream Disassembly & Payload Extraction ]
```

Decoders SHALL search candidate orientations at angular increments of $1.0^\circ$, confirm the primary reference gap at $\theta$, sample the 16-bit BCH word, and verify the mirrored word at $\theta + 180^\circ$.

---

## 11. Complete Version Table (V1 – V40)

| Ver | Rings | Total Codewords | Data Bytes (L) | Data Bytes (M) | Data Bytes (Q) | Data Bytes (H) | Numeric (M) | Alpha (M) |
|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **1** | 3 | 24 | 22 | 20 | 18 | 16 | 40 | 24 |
| **2** | 5 | 51 | 47 | 43 | 38 | 36 | 96 | 58 |
| **3** | 7 | 87 | 81 | 74 | 65 | 61 | 170 | 102 |
| **4** | 9 | 132 | 123 | 112 | 99 | 92 | 261 | 158 |
| **5** | 11 | 186 | 173 | 158 | 140 | 130 | 372 | 224 |
| **6** | 13 | 249 | 232 | 212 | 187 | 174 | 501 | 303 |
| **7** | 15 | 321 | 299 | 273 | 241 | 225 | 648 | 391 |
| **8** | 17 | 402 | 374 | 342 | 302 | 281 | 813 | 491 |
| **9** | 19 | 492 | 458 | 418 | 369 | 344 | 996 | 601 |
| **10** | 21 | 591 | 550 | 502 | 443 | 414 | 1197 | 723 |
| **11** | 23 | 699 | 650 | 594 | 524 | 489 | 1418 | 856 |
| **12** | 25 | 816 | 759 | 694 | 612 | 571 | 1658 | 1001 |
| **13** | 27 | 942 | 876 | 801 | 706 | 659 | 1915 | 1157 |
| **14** | 29 | 1077 | 1002 | 915 | 808 | 754 | 2188 | 1322 |
| **15** | 31 | 1221 | 1136 | 1038 | 916 | 855 | 2484 | 1500 |
| **16** | 33 | 1374 | 1278 | 1168 | 1030 | 962 | 2796 | 1689 |
| **17** | 35 | 1536 | 1428 | 1306 | 1152 | 1075 | 3127 | 1889 |
| **18** | 37 | 1707 | 1588 | 1451 | 1280 | 1195 | 3475 | 2099 |
| **19** | 39 | 1887 | 1755 | 1604 | 1415 | 1321 | 3842 | 2321 |
| **20** | 41 | 2076 | 1931 | 1765 | 1557 | 1453 | 4228 | 2554 |
| **25** | 51 | 3168 | 2946 | 2693 | 2376 | 2218 | 6456 | 3900 |
| **30** | 61 | 4488 | 4174 | 3815 | 3366 | 3142 | 9148 | 5527 |
| **35** | 71 | 6036 | 5613 | 5131 | 4527 | 4225 | 12307 | 7436 |
| **40** | 81 | 7812 | 7265 | 6640 | 5859 | 5468 | 15928 | 9623 |

---

## 12. Reference Implementation Directory

This repository contains the complete Apache-2.0 reference implementation:
- `crates/rrc-core`: Pure Rust algorithmic engine (zero external barcode/RS dependencies).
- `crates/rrc-cli`: Cross-platform CLI utility with terminal Unicode Braille rendering.
- `crates/rrc-cpp`: C ABI export and C++17 RAII wrapper header.
- `crates/rrc-node`: Node.js NAPI bindings with TypeScript definitions.
- `crates/rrc-kotlin`: Android and Kotlin bindings via UniFFI.
- `spec/diagrams/symbol_layout.svg`: High-resolution annotated vector layout diagram.

---
*End of Radial Response Code (RRC) Specification.*
