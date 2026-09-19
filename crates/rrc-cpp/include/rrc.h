/**
 * Radial Response Code (RRC) — C & C++ Unified API Header
 * 
 * Copyright (c) 2026 Ryan Shelby <MdSagorMunshi>
 * Licensed under the Apache License, Version 2.0.
 */

#ifndef RRC_H
#define RRC_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Status Return Codes */
#define RRC_OK               0
#define RRC_ERR_NULL_PTR    -1
#define RRC_ERR_INVALID_ARG -2
#define RRC_ERR_ENCODE      -3
#define RRC_ERR_DECODE      -4

/* Error Correction Levels */
#define RRC_ECC_L 0
#define RRC_ECC_M 1
#define RRC_ECC_Q 2
#define RRC_ECC_H 3

/**
 * Encode data into an RRC SVG vector document.
 * 
 * @param data Pointer to input data bytes.
 * @param data_len Number of bytes in data.
 * @param ecc Error correction level (0: L, 1: M, 2: Q, 3: H).
 * @param version Explicit version (1..=40), or 0 for automatic minimum sizing.
 * @param out_svg Output pointer receiving dynamically allocated null-terminated SVG string.
 *                Caller must free this memory using `rrc_free_string()`.
 * @return RRC_OK on success, negative error code on failure.
 */
int32_t rrc_encode_svg(
    const uint8_t* data,
    size_t data_len,
    uint8_t ecc,
    uint8_t version,
    char** out_svg
);

/**
 * Encode data into an RRC PNG binary buffer.
 * 
 * @param data Pointer to input data bytes.
 * @param data_len Number of bytes in data.
 * @param ecc Error correction level (0: L, 1: M, 2: Q, 3: H).
 * @param version Explicit version (1..=40), or 0 for automatic minimum sizing.
 * @param out_png Output pointer receiving dynamically allocated PNG byte buffer.
 *                Caller must free this memory using `rrc_free_bytes()`.
 * @param out_len Output pointer receiving size of PNG buffer in bytes.
 * @return RRC_OK on success, negative error code on failure.
 */
int32_t rrc_encode_png(
    const uint8_t* data,
    size_t data_len,
    uint8_t ecc,
    uint8_t version,
    uint8_t** out_png,
    size_t* out_len
);

/**
 * Decode an RRC symbol from an RGBA image buffer.
 * 
 * @param rgba Pointer to 8-bit RGBA pixel array (width * height * 4 bytes).
 * @param width Image width in pixels.
 * @param height Image height in pixels.
 * @param expected_version Optional version hint (1..=40), or 0 to auto-detect.
 * @param out_payload Output pointer receiving dynamically allocated decoded payload bytes.
 *                    Caller must free this memory using `rrc_free_bytes()`.
 * @param out_len Output pointer receiving payload size in bytes.
 * @param out_version Output pointer receiving detected version number (can be NULL).
 * @param out_ecc Output pointer receiving detected ECC level (can be NULL).
 * @return RRC_OK on success, negative error code on failure.
 */
int32_t rrc_decode(
    const uint8_t* rgba,
    uint32_t width,
    uint32_t height,
    uint8_t expected_version,
    uint8_t** out_payload,
    size_t* out_len,
    uint8_t* out_version,
    uint8_t* out_ecc
);

/**
 * Retrieve geometry specifications for an RRC version (1..=40).
 * 
 * @param version Version number (1..=40).
 * @param out_rings Number of concentric data rings.
 * @param out_bits Total usable data bits.
 * @param out_codewords Total codewords in symbol.
 * @param out_cap_m Data byte capacity at standard ECC Level M (~15%).
 * @return RRC_OK on success, RRC_ERR_INVALID_ARG if version is invalid.
 */
int32_t rrc_version_info(
    uint8_t version,
    uint32_t* out_rings,
    uint32_t* out_bits,
    uint32_t* out_codewords,
    uint32_t* out_cap_m
);

/**
 * Free a string allocated by RRC functions.
 */
void rrc_free_string(char* ptr);

/**
 * Free a byte buffer allocated by RRC functions.
 */
void rrc_free_bytes(uint8_t* ptr, size_t len);

#ifdef __cplusplus
} // extern "C"

#include <string>
#include <vector>
#include <stdexcept>
#include <memory>

namespace rrc {

enum class EccLevel : uint8_t {
    L = RRC_ECC_L,
    M = RRC_ECC_M,
    Q = RRC_ECC_Q,
    H = RRC_ECC_H,
};

struct VersionInfo {
    uint32_t rings;
    uint32_t bits;
    uint32_t codewords;
    uint32_t capacity_bytes_m;
};

struct DecodeResult {
    std::vector<uint8_t> payload;
    uint8_t version;
    EccLevel ecc;

    std::string text() const {
        return std::string(payload.begin(), payload.end());
    }
};

class Error : public std::runtime_error {
public:
    int32_t code;
    explicit Error(int32_t c, const std::string& msg) 
        : std::runtime_error(msg), code(c) {}
};

inline std::string encode_svg(
    const std::vector<uint8_t>& data,
    EccLevel ecc = EccLevel::M,
    uint8_t version = 0
) {
    char* raw_svg = nullptr;
    int32_t res = rrc_encode_svg(data.data(), data.size(), static_cast<uint8_t>(ecc), version, &raw_svg);
    if (res != RRC_OK || !raw_svg) {
        throw Error(res, "RRC SVG encoding failed");
    }
    std::string svg_str(raw_svg);
    rrc_free_string(raw_svg);
    return svg_str;
}

inline std::string encode_svg(
    const std::string& text,
    EccLevel ecc = EccLevel::M,
    uint8_t version = 0
) {
    std::vector<uint8_t> data(text.begin(), text.end());
    return encode_svg(data, ecc, version);
}

inline std::vector<uint8_t> encode_png(
    const std::vector<uint8_t>& data,
    EccLevel ecc = EccLevel::M,
    uint8_t version = 0
) {
    uint8_t* raw_png = nullptr;
    size_t len = 0;
    int32_t res = rrc_encode_png(data.data(), data.size(), static_cast<uint8_t>(ecc), version, &raw_png, &len);
    if (res != RRC_OK || !raw_png) {
        throw Error(res, "RRC PNG encoding failed");
    }
    std::vector<uint8_t> png_bytes(raw_png, raw_png + len);
    rrc_free_bytes(raw_png, len);
    return png_bytes;
}

inline DecodeResult decode(
    const uint8_t* rgba,
    uint32_t width,
    uint32_t height,
    uint8_t expected_version = 0
) {
    uint8_t* raw_payload = nullptr;
    size_t len = 0;
    uint8_t v = 0;
    uint8_t ecc_num = 0;

    int32_t res = rrc_decode(rgba, width, height, expected_version, &raw_payload, &len, &v, &ecc_num);
    if (res != RRC_OK || !raw_payload) {
        throw Error(res, "RRC symbol decoding failed");
    }

    DecodeResult result;
    result.payload.assign(raw_payload, raw_payload + len);
    result.version = v;
    result.ecc = static_cast<EccLevel>(ecc_num);
    rrc_free_bytes(raw_payload, len);
    return result;
}

inline VersionInfo get_version_info(uint8_t version) {
    VersionInfo info{};
    int32_t res = rrc_version_info(version, &info.rings, &info.bits, &info.codewords, &info.capacity_bytes_m);
    if (res != RRC_OK) {
        throw Error(res, "Invalid version requested");
    }
    return info;
}

} // namespace rrc

#endif // __cplusplus

#endif // RRC_H
