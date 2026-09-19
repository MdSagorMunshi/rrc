/**
 * Radial Response Code (RRC) Node.js TypeScript Definitions
 * 
 * Copyright (c) 2026 Ryan Shelby <MdSagorMunshi>
 * Licensed under the Apache License, Version 2.0.
 */

export interface JsEncodeOptions {
  version?: number;
  ecc?: 'L' | 'M' | 'Q' | 'H';
  style?: 'sharp' | 'rounded' | 'pill' | 'inner_rounded';
  darkColor?: string;
  lightColor?: string;
  quietZone?: number;
  renderScale?: number;
  jpegQuality?: number;
}

export interface JsDecodeResult {
  payload: Buffer;
  text?: string;
  version: number;
  eccLevel: string;
  mode: string;
}

export interface JsVersionInfo {
  version: number;
  ringCount: number;
  totalDataBits: number;
  totalCodewords: number;
  capacityBytesM: number;
}

/**
 * Encode text or binary payload into an RRC SVG vector document.
 */
export function encodeSvg(data: string | Buffer, options?: JsEncodeOptions): string;

/**
 * Encode text or binary payload into an RRC PNG image buffer.
 */
export function encodePng(data: string | Buffer, options?: JsEncodeOptions): Buffer;

/**
 * Encode text or binary payload into an RRC JPEG image buffer.
 */
export function encodeJpeg(data: string | Buffer, options?: JsEncodeOptions): Buffer;

/**
 * Decode an RRC symbol from an uncompressed RGBA pixel buffer.
 */
export function decodeRgba(
  rgba: Buffer,
  width: number,
  height: number,
  expectedVersion?: number
): JsDecodeResult;

/**
 * Retrieve geometry and capacity specifications for an RRC version (1..=40).
 */
export function getVersionInfo(version: number): JsVersionInfo;
