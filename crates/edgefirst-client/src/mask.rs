// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Raster mask data with PNG encode/decode support.
//!
//! [`MaskData`] wraps PNG-encoded bytes for storing raster masks in Arrow
//! `Binary` columns. It supports fast header-only reads (width, height,
//! bit_depth) without decoding pixels, and can encode from raw pixels at
//! various bit depths (1-bit binary, 8-bit scores, 16-bit precision).

/// A raster mask stored as PNG-encoded bytes.
///
/// `MaskData` provides zero-copy access to PNG metadata (width, height,
/// bit_depth) by reading the IHDR chunk directly, and full encode/decode
/// for pixel data at 1-bit, 8-bit, and 16-bit depths.
///
/// # PNG layout reference
///
/// ```text
/// [0..8]   PNG signature
/// [8..12]  IHDR chunk length (always 13)
/// [12..16] IHDR chunk type ("IHDR")
/// [16..20] width  (big-endian u32)
/// [20..24] height (big-endian u32)
/// [24]     bit_depth
/// [25]     color_type
/// ...
/// ```
#[derive(Clone, Debug)]
pub struct MaskData {
    png: Vec<u8>,
}

impl MaskData {
    /// Creates a `MaskData` from raw PNG bytes.
    ///
    /// The caller is responsible for ensuring the bytes represent a valid PNG.
    pub fn from_png(png: Vec<u8>) -> Self {
        Self { png }
    }

    /// Returns a reference to the underlying PNG bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.png
    }

    /// Consumes the `MaskData` and returns the underlying PNG bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.png
    }

    /// Returns the image width by reading the PNG IHDR chunk (bytes 16..20).
    ///
    /// # Panics
    ///
    /// Panics if the PNG data is too short to contain a valid IHDR chunk.
    pub fn width(&self) -> u32 {
        u32::from_be_bytes(
            self.png[16..20]
                .try_into()
                .expect("PNG IHDR should contain width at bytes 16..20"),
        )
    }

    /// Returns the image height by reading the PNG IHDR chunk (bytes 20..24).
    ///
    /// # Panics
    ///
    /// Panics if the PNG data is too short to contain a valid IHDR chunk.
    pub fn height(&self) -> u32 {
        u32::from_be_bytes(
            self.png[20..24]
                .try_into()
                .expect("PNG IHDR should contain height at bytes 20..24"),
        )
    }

    /// Returns the bit depth by reading the PNG IHDR chunk (byte 24).
    ///
    /// # Panics
    ///
    /// Panics if the PNG data is too short to contain a valid IHDR chunk.
    pub fn bit_depth(&self) -> u8 {
        self.png[24]
    }

    /// Encodes raw 8-bit grayscale pixels into a PNG.
    ///
    /// For `bit_depth == 1`, pixel values must be `0` or `1` and will be packed
    /// into 1-bit-per-pixel PNG rows (MSB first, 8 pixels per byte, with
    /// zero-padding on the last byte if `width` is not a multiple of 8).
    ///
    /// For `bit_depth == 8`, pixels are encoded directly as 8-bit grayscale.
    ///
    /// # Panics
    ///
    /// Panics if `bit_depth` is not 1 or 8, or if `pixels.len()` does not
    /// equal `width * height`.
    pub fn encode(pixels: &[u8], width: u32, height: u32, bit_depth: u8) -> Self {
        assert!(
            bit_depth == 1 || bit_depth == 8,
            "bit_depth must be 1 or 8, got {bit_depth}"
        );
        let expected = (width as usize) * (height as usize);
        assert_eq!(
            pixels.len(),
            expected,
            "pixel count mismatch: expected {expected}, got {}",
            pixels.len()
        );

        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, width, height);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(match bit_depth {
                1 => png::BitDepth::One,
                8 => png::BitDepth::Eight,
                _ => unreachable!(),
            });

            let mut writer = encoder.write_header().expect("PNG header write failed");

            match bit_depth {
                1 => {
                    let bytes_per_row = (width as usize).div_ceil(8);
                    let mut packed = vec![0u8; bytes_per_row * height as usize];
                    for y in 0..height as usize {
                        for x in 0..width as usize {
                            if pixels[y * width as usize + x] != 0 {
                                packed[y * bytes_per_row + x / 8] |= 0x80 >> (x % 8);
                            }
                        }
                    }
                    writer
                        .write_image_data(&packed)
                        .expect("PNG image data write failed");
                }
                8 => {
                    writer
                        .write_image_data(pixels)
                        .expect("PNG image data write failed");
                }
                _ => unreachable!(),
            }
        }
        Self { png: buf }
    }

    /// Encodes raw 16-bit grayscale pixels into a PNG.
    ///
    /// Each `u16` value is written as two bytes in big-endian order, matching
    /// the PNG 16-bit grayscale format.
    ///
    /// # Panics
    ///
    /// Panics if `pixels.len()` does not equal `width * height`.
    pub fn encode_16bit(pixels: &[u16], width: u32, height: u32) -> Self {
        let expected = (width as usize) * (height as usize);
        assert_eq!(
            pixels.len(),
            expected,
            "pixel count mismatch: expected {expected}, got {}",
            pixels.len()
        );

        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, width, height);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Sixteen);

            let mut writer = encoder.write_header().expect("PNG header write failed");

            let raw: Vec<u8> = pixels.iter().flat_map(|&v| v.to_be_bytes()).collect();
            writer
                .write_image_data(&raw)
                .expect("PNG image data write failed");
        }
        Self { png: buf }
    }

    /// Decodes the PNG image to raw pixel bytes.
    ///
    /// For 1-bit PNGs, each pixel is unpacked to a single byte (`0` or `1`).
    /// For 8-bit PNGs, pixel bytes are returned directly.
    /// For 16-bit PNGs, each pixel yields two bytes in big-endian order.
    pub fn decode(&self) -> Vec<u8> {
        let decoder = png::Decoder::new(self.png.as_slice());
        let mut reader = decoder.read_info().expect("PNG info read failed");
        let mut raw = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut raw).expect("PNG frame read failed");
        raw.truncate(info.buffer_size());

        if info.bit_depth == png::BitDepth::One {
            let width = info.width as usize;
            let height = info.height as usize;
            let bytes_per_row = width.div_ceil(8);
            let mut unpacked = Vec::with_capacity(width * height);
            for y in 0..height {
                for x in 0..width {
                    let byte = raw[y * bytes_per_row + x / 8];
                    let bit = (byte >> (7 - (x % 8))) & 1;
                    unpacked.push(bit);
                }
            }
            unpacked
        } else {
            raw
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_8bit() {
        // 3x3 image with varied grayscale values
        let pixels: Vec<u8> = vec![0, 64, 128, 192, 255, 1, 100, 200, 50];
        let mask = MaskData::encode(&pixels, 3, 3, 8);

        assert_eq!(mask.width(), 3);
        assert_eq!(mask.height(), 3);
        assert_eq!(mask.bit_depth(), 8);

        let decoded = mask.decode();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn test_encode_decode_1bit() {
        // 8x2 image, byte-aligned width
        let pixels: Vec<u8> = vec![
            1, 0, 1, 0, 1, 0, 1, 0, // row 0
            0, 1, 0, 1, 0, 1, 0, 1, // row 1
        ];
        let mask = MaskData::encode(&pixels, 8, 2, 1);

        assert_eq!(mask.width(), 8);
        assert_eq!(mask.height(), 2);
        assert_eq!(mask.bit_depth(), 1);

        let decoded = mask.decode();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn test_encode_decode_16bit() {
        // 2x2 image with u16 values
        let pixels: Vec<u16> = vec![0, 256, 65535, 1024];
        let mask = MaskData::encode_16bit(&pixels, 2, 2);

        assert_eq!(mask.width(), 2);
        assert_eq!(mask.height(), 2);
        assert_eq!(mask.bit_depth(), 16);

        let decoded = mask.decode();
        // 16-bit PNG decodes to big-endian byte pairs
        let expected: Vec<u8> = pixels.iter().flat_map(|&v| v.to_be_bytes()).collect();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_header_read_without_decode() {
        // 640x480 all-zeros: verify header reads work and PNG compresses well
        let width = 640u32;
        let height = 480u32;
        let pixels = vec![0u8; (width * height) as usize];
        let mask = MaskData::encode(&pixels, width, height, 8);

        assert_eq!(mask.width(), width);
        assert_eq!(mask.height(), height);
        assert_eq!(mask.bit_depth(), 8);

        // PNG compression of all-zeros should be much smaller than raw pixels
        let raw_size = (width * height) as usize;
        assert!(
            mask.as_bytes().len() < raw_size,
            "PNG ({} bytes) should be smaller than raw ({} bytes)",
            mask.as_bytes().len(),
            raw_size,
        );
    }

    #[test]
    fn test_from_png_bytes() {
        // Encode, extract bytes, reconstruct, verify roundtrip
        let pixels: Vec<u8> = vec![10, 20, 30, 40, 50, 60];
        let original = MaskData::encode(&pixels, 3, 2, 8);

        let bytes = original.into_bytes();
        let reconstructed = MaskData::from_png(bytes);

        assert_eq!(reconstructed.width(), 3);
        assert_eq!(reconstructed.height(), 2);
        assert_eq!(reconstructed.bit_depth(), 8);
        assert_eq!(reconstructed.decode(), pixels);
    }

    #[test]
    fn test_1bit_non_aligned_width() {
        // 5x3 image: width not a multiple of 8
        let pixels: Vec<u8> = vec![
            1, 0, 1, 1, 0, // row 0
            0, 1, 0, 0, 1, // row 1
            1, 1, 1, 0, 0, // row 2
        ];
        let mask = MaskData::encode(&pixels, 5, 3, 1);

        assert_eq!(mask.width(), 5);
        assert_eq!(mask.height(), 3);
        assert_eq!(mask.bit_depth(), 1);

        let decoded = mask.decode();
        assert_eq!(decoded, pixels);
    }
}
