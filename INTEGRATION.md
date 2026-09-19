# 🛠️ RRC Integration & Implementation Guide

Comprehensive developer guide for integrating, embedding, and utilizing **Radial Response Code (RRC)** across any programming language, platform, or hardware environment.

---

## 📑 Table of Contents

1. [Platform Support Matrix](#1-platform-support-matrix)
2. [Rust (`rrc-core`)](#2-rust-rrc-core)
3. [Node.js & TypeScript (`@rrc/node`)](#3-nodejs--typescript-rrcnode)
4. [C & C++17 (`rrc-cpp`)](#4-c--c17-rrccpp)
5. [Android & Kotlin (`rrc-kotlin`)](#5-android--kotlin-rrckotlin)
6. [Python (via C ABI & ctypes)](#6-python-via-c-abi--ctypes)
7. [Web & WebAssembly Integration](#7-web--webassembly-integration)
8. [Production Best Practices & Tuning](#8-production-best-practices--tuning)

---

## 1. Platform Support Matrix

| Target Ecosystem | Module / Binding | Delivery Mechanism | Primary Use Cases |
| :--- | :--- | :--- | :--- |
| **Rust** | `crates/rrc-core` | Native Crate | Core embedded systems, backend microservices, CLI tools |
| **Node.js / TS** | `crates/rrc-node` | NAPI-RS binary addon | Web APIs, serverless lambdas, Next.js / Express microservices |
| **C & C++** | `crates/rrc-cpp` | `librrc_cpp.so` / `.a` + `rrc.h` | OpenCV video pipelines, game engines (Unreal, Godot), Qt apps |
| **Android / Kotlin**| `crates/rrc-kotlin` | UniFFI + JNA (`.aar` / `.so`) | Mobile camera scanner apps, ticket validation, point-of-sale |
| **Python** | C FFI / `ctypes` | `ctypes` binding to `librrc_cpp` | Data science pipelines, OpenCV image processing, rapid scripts |
| **CLI / Shell** | `crates/rrc-cli` | Standalone binary | DevOps scripts, terminal Braille previews, batch asset generation |

---

## 2. Rust (`rrc-core`)

The reference algorithmic engine is implemented in pure, safe Rust with zero barcode or third-party error correction dependencies.

### 2.1 Installation
Add `rrc-core` to your `Cargo.toml`:

```toml
[dependencies]
rrc-core = { path = "path/to/rrc/crates/rrc-core" }
# Or via git:
# rrc-core = { git = "https://github.com/MdSagorMunshi/rrc.git", package = "rrc-core" }
image = { version = "0.25", features = ["png", "jpeg"] }
```

---

### 2.2 Complete Encode Example

```rust
use rrc_core::{
    encode, render_svg, render_png, render_jpeg, render_ascii,
    Color, EccLevel, EncodeOptions, RrcStyle, SectorStyle,
};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let payload = b"https://github.com/MdSagorMunshi/rrc";

    // 1. Configure custom visual style
    let mut style = RrcStyle::dark_neon(); // Preset: #00FF94 on #0A0A0A
    style.sector_style = SectorStyle::Rounded; // Sharp, Rounded, Pill, InnerRounded
    style.quiet_zone = 2.5;                    // Multiplier for outer border margin
    style.render_scale = 12;                   // Pixels per unit for PNG/JPEG

    // 2. Configure encoder options
    let mut options = EncodeOptions::default();
    options.ecc_level = EccLevel::H; // High: ~30% error recovery capacity
    options.style = style.clone();
    // options.version = Some(2);    // Explicit version, or None for auto-fit

    // 3. Generate the RRC symbol matrix
    let symbol = encode(payload, &options)?;
    println!(
        "Encoded Version {} with {} rings, Mask #{}",
        symbol.version,
        symbol.matrix.len(),
        symbol.mask_index
    );

    // 4. Render to vector SVG
    let svg_xml = render_svg(symbol.version, &symbol.matrix, &symbol.style);
    fs::write("badge.svg", svg_xml)?;

    // 5. Render to raster PNG
    let png_bytes = render_png(symbol.version, &symbol.matrix, &symbol.style)?;
    fs::write("badge.png", png_bytes)?;

    // 6. Render to raster JPEG (Quality: 95)
    let jpeg_bytes = render_jpeg(symbol.version, &symbol.matrix, &symbol.style, Some(95))?;
    fs::write("badge.jpg", jpeg_bytes)?;

    // 7. Render high-density Unicode Braille to terminal
    let mut term_style = style;
    term_style.ascii_use_braille = true;
    println!("\n{}", render_ascii(symbol.version, &symbol.matrix, &term_style));

    Ok(())
}
```

---

### 2.3 Complete Decode Example

```rust
use rrc_core::{decode, DecodeOptions};
use image::GenericImageView;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load an image file into memory (PNG, JPEG, WebP, BMP, TIFF)
    let img = image::open("badge.png")?.to_rgba8();
    let (width, height) = img.dimensions();
    let raw_rgba = img.into_raw();

    // 2. Configure decoder options
    let options = DecodeOptions {
        verbose: false,         // Set to true for internal scanning diagnostics
        expected_version: None, // Auto-detect version 1..=40
    };

    // 3. Decode the symbol
    let result = decode(&raw_rgba, width, height, &options)?;

    println!("✓ Successfully Decoded RRC Symbol!");
    println!("  Version:       {}", result.version);
    println!("  ECC Level:     {:?}", result.ecc_level);
    println!("  Encoding Mode: {:?}", result.mode);
    println!("  Payload bytes: {}", result.payload.len());

    if let Some(text) = result.text {
        println!("  Text Content:  {}", text);
    } else {
        println!("  Binary Payload: {:02X?}", result.payload);
    }

    Ok(())
}
```

---

## 3. Node.js & TypeScript (`@rrc/node`)

The Node.js binding is powered by N-API via `napi-rs`, offering direct C-speed execution without V8 serialization overhead.

### 3.1 Building and Linking the Native Module
From the repository root:
```bash
cd crates/rrc-node
npm install
npm run build
```

Link into your local Node.js application:
```bash
npm link
# In your consuming project:
npm link @rrc/node
```

---

### 3.2 TypeScript Usage Example

```typescript
import {
  encodeSvg,
  encodePng,
  encodeJpeg,
  decodeRgba,
  getVersionInfo,
  JsEncodeOptions,
} from '@rrc/node';
import * as fs from 'fs';

// Query version capacity specs
const v1Info = getVersionInfo(1);
console.log(`V1 rings: ${v1Info.ringCount}, data bytes: ${v1Info.capacityBytesM}`);

// Custom styling options
const options: JsEncodeOptions = {
  ecc: 'H',
  style: 'rounded',
  darkColor: '#00FF94',
  lightColor: '#0A0A0A',
  quietZone: 2.0,
  renderScale: 10,
};

// 1. Encode text to vector SVG string
const svgContent: string = encodeSvg('https://github.com/MdSagorMunshi/rrc', options);
fs.writeFileSync('output.svg', svgContent);

// 2. Encode to PNG buffer
const pngBuffer: Buffer = encodePng('https://github.com/MdSagorMunshi/rrc', options);
fs.writeFileSync('output.png', pngBuffer);
```

---

### 3.3 Production Microservice Recipe (Express.js)

Dynamic on-the-fly badge generation service:

```typescript
import express, { Request, Response } from 'express';
import { encodeSvg, encodePng } from '@rrc/node';

const app = express();
const port = 3000;

// GET /rrc/svg?data=hello&ecc=M&color=%2300FF94
app.get('/rrc/svg', (req: Request, res: Response) => {
  const data = (req.query.data as string) || 'https://github.com/MdSagorMunshi/rrc';
  const ecc = (req.query.ecc as any) || 'M';
  const color = (req.query.color as string) || '#000000';

  try {
    const svg = encodeSvg(data, {
      ecc,
      darkColor: color,
      lightColor: '#FFFFFF',
      style: 'sharp',
    });
    res.setHeader('Content-Type', 'image/svg+xml');
    res.setHeader('Cache-Control', 'public, max-age=86400');
    res.send(svg);
  } catch (err: any) {
    res.status(400).json({ error: err.message });
  }
});

// GET /rrc/png?data=hello
app.get('/rrc/png', (req: Request, res: Response) => {
  const data = (req.query.data as string) || 'https://github.com/MdSagorMunshi/rrc';
  try {
    const png = encodePng(data, { ecc: 'M', renderScale: 10 });
    res.setHeader('Content-Type', 'image/png');
    res.send(png);
  } catch (err: any) {
    res.status(400).json({ error: err.message });
  }
});

app.listen(port, () => {
  console.log(`RRC Service listening at http://localhost:${port}`);
});
```

---

### 3.4 Decoding Images in Node.js (with Sharp)

```typescript
import sharp from 'sharp';
import { decodeRgba } from '@rrc/node';

async function scanImage(filePath: string) {
  // Convert any image format to raw 32-bit RGBA pixel buffer
  const image = sharp(filePath);
  const { width, height } = await image.metadata();

  if (!width || !height) {
    throw new Error('Could not read image dimensions');
  }

  const rawRgbaBuffer = await image.raw().ensureAlpha().toBuffer();

  // Decode the RRC symbol
  const result = decodeRgba(rawRgbaBuffer, width, height);

  console.log('Decoded Payload:', result.text ?? result.payload.toString('hex'));
  console.log(`Version: V${result.version}, Mode: ${result.mode}, ECC: ${result.eccLevel}`);
}

scanImage('output.png').catch(console.error);
```

---

## 4. C & C++17 (`rrc-cpp`)

The C/C++ interface provides a single unified header [`include/rrc.h`](crates/rrc-cpp/include/rrc.h) supporting both standard C99 ABI calls and modern C++17 RAII abstractions.

### 4.1 Build Native Static & Shared Libraries
```bash
cargo build --release -p rrc-cpp
```
This produces:
- `target/release/librrc_cpp.a` (Static archive)
- `target/release/librrc_cpp.so` (Linux shared object) or `.dylib` (macOS) / `.dll` (Windows)

---

### 4.2 CMake Integration

Add to your project's `CMakeLists.txt`:

```cmake
cmake_minimum_required(VERSION 3.15)
project(MyRrcApp CXX)

set(CMAKE_CXX_STANDARD 17)

# Point to RRC header and built library
set(RRC_ROOT "/path/to/RRC")
include_directories(${RRC_ROOT}/crates/rrc-cpp/include)

add_library(rrc_lib STATIC IMPORTED)
set_target_properties(rrc_lib PROPERTIES
    IMPORTED_LOCATION "${RRC_ROOT}/target/release/librrc_cpp.a"
)

add_executable(my_app main.cpp)
target_link_libraries(my_app PRIVATE rrc_lib pthread dl m)
```

---

### 4.3 Modern C++17 Example

```cpp
#include "rrc.h"
#include <iostream>
#include <fstream>

int main() {
    try {
        std::string text = "https://github.com/MdSagorMunshi/rrc";

        // 1. Query version information
        auto info = rrc::get_version_info(1);
        std::cout << "RRC Version 1: " << info.rings << " rings, capacity: " 
                  << info.capacity_bytes_m << " bytes\n";

        // 2. Generate vector SVG string
        std::string svg = rrc::encode_svg(text, rrc::EccLevel::H);
        std::ofstream svg_file("symbol.svg");
        svg_file << svg;
        svg_file.close();
        std::cout << "Saved symbol.svg (" << svg.length() << " bytes)\n";

        // 3. Generate PNG binary buffer
        std::vector<uint8_t> data(text.begin(), text.end());
        std::vector<uint8_t> png = rrc::encode_png(data, rrc::EccLevel::M);
        std::ofstream png_file("symbol.png", std::ios::binary);
        png_file.write(reinterpret_cast<const char*>(png.data()), png.size());
        png_file.close();
        std::cout << "Saved symbol.png (" << png.size() << " bytes)\n";

    } catch (const rrc::Error& e) {
        std::cerr << "RRC Error [" << e.code << "]: " << e.what() << "\n";
        return 1;
    }
    return 0;
}
```

---

### 4.4 Real-Time OpenCV Video Scanner (C++)

Real-time scanning directly from a webcam stream:

```cpp
#include "rrc.h"
#include <opencv2/opencv.hpp>
#include <iostream>

int main() {
    cv::VideoCapture cap(0);
    if (!cap.isOpened()) {
        std::cerr << "Error: Cannot open camera feed\n";
        return -1;
    }

    cv::Mat frame, rgba;
    std::cout << "Scanning for RRC codes. Press ESC to exit.\n";

    while (true) {
        cap >> frame;
        if (frame.empty()) break;

        // Convert OpenCV BGR to 32-bit RGBA
        cv::cvtColor(frame, rgba, cv::COLOR_BGR2RGBA);

        try {
            // Attempt to decode the current frame
            auto result = rrc::decode(rgba.data, rgba.cols, rgba.rows);

            std::cout << ">> Found RRC V" << (int)result.version << ": "
                      << result.text() << "\n";

            // Visual feedback on screen
            cv::putText(frame, "RRC DETECTED: " + result.text(),
                        cv::Point(30, 50), cv::FONT_HERSHEY_SIMPLEX, 0.8,
                        cv::Scalar(0, 255, 100), 2);

        } catch (const rrc::Error&) {
            // No symbol detected in current frame, continue scanning
        }

        cv::imshow("RRC Real-Time Scanner", frame);
        if (cv::waitKey(1) == 27) break; // ESC
    }

    return 0;
}
```

---

### 4.5 Pure C99 ABI Example

For embedded platforms or C projects without C++ runtime:

```c
#include "rrc.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    const char* text = "RRC C99 EMBEDDED";
    char* svg_output = NULL;

    int32_t status = rrc_encode_svg(
        (const uint8_t*)text,
        strlen(text),
        RRC_ECC_M,      // Error correction level
        0,              // Version (0 = auto)
        &svg_output
    );

    if (status == RRC_OK && svg_output != NULL) {
        printf("SVG generated successfully:\n%.100s...\n", svg_output);
        
        // Critical: Free memory allocated by Rust
        rrc_free_string(svg_output);
    } else {
        fprintf(stderr, "Encoding failed with error code: %d\n", status);
        return 1;
    }

    return 0;
}
```

---

## 5. Android & Kotlin (`rrc-kotlin`)

Android integration is handled through UniFFI bindings and JNA (`com.sun.jna`), wrapping the native Rust shared library compiled for Android NDK targets.

### 5.1 Gradle Configuration
In your Android app's `app/build.gradle.kts`:

```kotlin
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

dependencies {
    implementation("net.java.dev.jna:jna:5.14.0@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.0")
}
```

Compile the native `.so` files for Android ABIs using `cargo-ndk`:
```bash
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o app/src/main/jniLibs build --release -p rrc-kotlin
```

---

### 5.2 Encoding to Android `Bitmap`

```kotlin
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import com.github.mdsagormunshi.rrc.*

class RrcGenerator {
    fun generateRrcBitmap(payload: String, eccLevel: String = "M"): Bitmap {
        // Generate PNG byte array via UniFFI
        val pngBytes = encodePng(payload.toByteArray(Charsets.UTF_8), eccLevel, null)

        // Decode PNG bytes into Android native Bitmap
        return BitmapFactory.decodeByteArray(pngBytes, 0, pngBytes.size)
    }
}
```

---

### 5.3 Android CameraX Real-Time Scanning Analyzer

```kotlin
import android.annotation.SuppressLint
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import com.github.mdsagormunshi.rrc.*
import java.nio.ByteBuffer

class RrcCameraAnalyzer(
    private val onSymbolDetected: (String, Int) -> Unit
) : ImageAnalysis.Analyzer {

    @SuppressLint("UnsafeOptInUsageError")
    override fun analyze(imageProxy: ImageProxy) {
        val image = imageProxy.image
        if (image == null) {
            imageProxy.close()
            return
        }

        val width = imageProxy.width
        val height = imageProxy.height

        // Convert camera image to 32-bit RGBA byte array
        val rgbaBytes = convertYUV420ToRGBA(imageProxy)

        try {
            // Run RRC scanner
            val result = decodeRgba(rgbaBytes, width.toLong(), height.toLong(), null)
            val text = result.text ?: String(result.payload, Charsets.UTF_8)
            onSymbolDetected(text, result.version.toInt())
        } catch (e: Exception) {
            // Scanner found no symbol in this frame
        } finally {
            imageProxy.close()
        }
    }

    private fun convertYUV420ToRGBA(imageProxy: ImageProxy): ByteArray {
        // Implementation converts YUV planes to contiguous RGBA byte array
        val buffer = ByteArray(imageProxy.width * imageProxy.height * 4)
        // ... (standard Android YUV to RGBA conversion)
        return buffer
    }
}
```

---

## 6. Python (via C ABI & ctypes)

You can call RRC directly in Python without compiling a custom extension module by using `ctypes` to link with `librrc_cpp.so`.

### 6.1 Pure Python Wrapper (`rrc.py`)

```python
import ctypes
import os
from typing import Optional, Tuple
from PIL import Image
import numpy as np

# Load the compiled shared library
lib_path = os.path.abspath("target/release/librrc_cpp.so")
rrc = ctypes.CDLL(lib_path)

# Configure C function prototypes
rrc.rrc_encode_svg.argtypes = [
    ctypes.POINTER(ctypes.c_uint8),
    ctypes.c_size_t,
    ctypes.c_uint8,
    ctypes.c_uint8,
    ctypes.POINTER(ctypes.c_char_p),
]
rrc.rrc_encode_svg.restype = ctypes.c_int32

rrc.rrc_encode_png.argtypes = [
    ctypes.POINTER(ctypes.c_uint8),
    ctypes.c_size_t,
    ctypes.c_uint8,
    ctypes.c_uint8,
    ctypes.POINTER(ctypes.POINTER(ctypes.c_uint8)),
    ctypes.POINTER(ctypes.c_size_t),
]
rrc.rrc_encode_png.restype = ctypes.c_int32

rrc.rrc_decode.argtypes = [
    ctypes.POINTER(ctypes.c_uint8),
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_uint8,
    ctypes.POINTER(ctypes.POINTER(ctypes.c_uint8)),
    ctypes.POINTER(ctypes.c_size_t),
    ctypes.POINTER(ctypes.c_uint8),
    ctypes.POINTER(ctypes.c_uint8),
]
rrc.rrc_decode.restype = ctypes.c_int32

rrc.rrc_free_string.argtypes = [ctypes.c_char_p]
rrc.rrc_free_bytes.argtypes = [ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t]

ECC_LEVELS = {"L": 0, "M": 1, "Q": 2, "H": 3}

def encode_svg(data: str, ecc: str = "M", version: int = 0) -> str:
    raw_data = data.encode("utf-8")
    data_arr = (ctypes.c_uint8 * len(raw_data))(*raw_data)
    out_svg = ctypes.c_char_p()

    status = rrc.rrc_encode_svg(data_arr, len(raw_data), ECC_LEVELS[ecc], version, ctypes.byref(out_svg))
    if status != 0:
        raise RuntimeError(f"RRC SVG encoding failed with code {status}")

    result = out_svg.value.decode("utf-8")
    rrc.rrc_free_string(out_svg)
    return result

def decode_image(image_path: str) -> Tuple[str, int]:
    # Open image and convert to 32-bit RGBA numpy array
    img = Image.open(image_path).convert("RGBA")
    width, height = img.size
    rgba_bytes = np.array(img, dtype=np.uint8).tobytes()

    buf = (ctypes.c_uint8 * len(rgba_bytes)).from_buffer_copy(rgba_bytes)
    out_payload = ctypes.POINTER(ctypes.c_uint8)()
    out_len = ctypes.c_size_t()
    out_version = ctypes.c_uint8()
    out_ecc = ctypes.c_uint8()

    status = rrc.rrc_decode(
        buf, width, height, 0,
        ctypes.byref(out_payload), ctypes.byref(out_len),
        ctypes.byref(out_version), ctypes.byref(out_ecc)
    )
    if status != 0:
        raise RuntimeError(f"Decode failed with status {status}")

    payload = bytes(out_payload[:out_len.value]).decode("utf-8", errors="replace")
    version = out_version.value
    rrc.rrc_free_bytes(out_payload, out_len.value)
    return payload, version

# Usage:
if __name__ == "__main__":
    svg = encode_svg("https://github.com/MdSagorMunshi/rrc", ecc="H")
    with open("python_rrc.svg", "w") as f:
        f.write(svg)
    print("Saved python_rrc.svg")
```

---

## 7. Web & WebAssembly Integration

### 7.1 Direct SVG DOM Injection
Because RRC's SVG renderer produces clean vector paths without external fonts, you can inject it directly into any modern frontend framework (React, Vue, Svelte):

```html
<div id="rrc-container" class="neon-glow"></div>

<script type="module">
  // Example fetching dynamic SVG from your Node.js endpoint
  async function loadBadge() {
    const res = await fetch('/rrc/svg?data=' + encodeURIComponent(window.location.href));
    const svgText = await res.text();
    document.getElementById('rrc-container').innerHTML = svgText;
  }
  loadBadge();
</script>

<style>
  .neon-glow svg {
    filter: drop-shadow(0 0 12px rgba(0, 255, 148, 0.4));
    transition: transform 0.3s ease;
  }
  .neon-glow svg:hover {
    transform: scale(1.05) rotate(5deg);
  }
</style>
```

---

## 8. Production Best Practices & Tuning

### 8.1 Resolution Guidelines for Decoders
For reliable camera scanning:
- **Versions 1–5 (3–11 rings):** Minimum symbol bounding box of **$180 \times 180$ pixels**.
- **Versions 6–15 (13–31 rings):** Minimum symbol bounding box of **$320 \times 320$ pixels**.
- **Versions 16–40 (33–81 rings):** Minimum symbol bounding box of **$560 \times 560$ pixels**.

### 8.2 Lighting & Contrast
- RRC's bullseye detector uses intensity moments. Always maintain a minimum **$3:1$ contrast ratio** between dark modules and light background.
- If using inverted dark themes (e.g. Neon Green on Dark Charcoal), the detector automatically identifies contrast polarity using the quiet ring.

### 8.3 Thread Safety
- All core encoding and decoding functions in `rrc-core` and the FFI wrappers are **100% thread-safe and re-entrant** (`Send + Sync`).
- In video processing pipelines, run image decoding on a dedicated worker pool (e.g. `rayon` in Rust, Web Workers in Node.js, `Dispatchers.Default` in Kotlin) to maintain consistent 60 FPS UI rendering.

---
*For formal mathematical equations, Galois field arithmetic proofs, and complete version specifications, see [SPEC.md](SPEC.md).*
