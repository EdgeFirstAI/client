// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Raster mask data with PNG encode/decode support.
//!
//! [`MaskData`] wraps PNG-encoded bytes for storing raster masks in Arrow
//! `Binary` columns. It supports fast header-only reads (width, height,
//! bit_depth, offset) without decoding pixels, and can encode from raw pixels
//! at various bit depths (1-bit binary, 8-bit scores, 16-bit precision).

/// PNG magic bytes (first 8 bytes of every valid PNG file).
const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Minimum length of a valid PNG file with IHDR chunk.
/// 8 (signature) + 4 (length) + 4 (type "IHDR") + 13 (IHDR data) + 4 (IHDR CRC) = 33
const MIN_PNG_LEN: usize = 33;

/// Largest mask, in pixels, that is decoded or rasterized (100 megapixels).
pub(crate) const MAX_PIXELS: u64 = 100_000_000;

/// PNG `oFFs` chunk type: image position as x/y (i32 BE) plus a unit byte.
const OFFS_CHUNK: [u8; 4] = *b"oFFs";

/// `oFFs` unit specifier for pixels; the only other defined unit is 1 (micrometres).
const OFFS_UNIT_PIXEL: u8 = 0;

/// A raster mask stored as PNG-encoded bytes.
///
/// `MaskData` provides zero-copy access to PNG metadata (width, height,
/// bit_depth) by reading the IHDR chunk directly, and full encode/decode
/// for pixel data at 1-bit, 8-bit, and 16-bit depths.
///
/// An instance mask may cover only a tile of the image, in which case the
/// PNG carries an `oFFs` chunk (pixel units) with the tile's top-left
/// position on the image; see [`offset`](Self::offset).
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
    /// For validated construction, use [`from_png_checked`](Self::from_png_checked).
    pub fn from_png(png: Vec<u8>) -> Self {
        Self { png }
    }

    /// Creates a `MaskData` from raw PNG bytes with validation.
    ///
    /// Validates that the bytes represent a valid grayscale PNG:
    /// - Length >= 33 bytes (signature + IHDR chunk including CRC)
    /// - PNG magic bytes at offset 0
    /// - Color type byte (offset 25) is 0 (grayscale)
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not a valid grayscale PNG.
    pub fn from_png_checked(png: Vec<u8>) -> Result<Self, crate::Error> {
        if png.len() < MIN_PNG_LEN {
            return Err(crate::Error::InvalidParameters(format!(
                "PNG data too short: {} bytes, minimum {} required",
                png.len(),
                MIN_PNG_LEN
            )));
        }
        if png[..8] != PNG_SIGNATURE {
            return Err(crate::Error::InvalidParameters(
                "invalid PNG signature: not a PNG file".to_string(),
            ));
        }
        let color_type = png[25];
        if color_type != 0 {
            return Err(crate::Error::InvalidParameters(format!(
                "PNG color type must be 0 (grayscale), got {}",
                color_type
            )));
        }

        // Try to parse PNG header to catch truncated/malformed data
        let decoder = png::Decoder::new(std::io::Cursor::new(&png));
        if decoder.read_info().is_err() {
            return Err(crate::Error::InvalidParameters(
                "PNG data is malformed or truncated".to_string(),
            ));
        }

        Ok(Self { png })
    }

    /// Returns `true` if the underlying bytes contain a valid PNG signature
    /// and are long enough to have an IHDR chunk.
    pub fn is_valid(&self) -> bool {
        self.png.len() >= MIN_PNG_LEN && self.png[..8] == PNG_SIGNATURE
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
    /// Returns 0 if the PNG data is too short or invalid.
    pub fn width(&self) -> u32 {
        self.png
            .get(16..20)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_be_bytes)
            .unwrap_or(0)
    }

    /// Returns the image height by reading the PNG IHDR chunk (bytes 20..24).
    ///
    /// Returns 0 if the PNG data is too short or invalid.
    pub fn height(&self) -> u32 {
        self.png
            .get(20..24)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_be_bytes)
            .unwrap_or(0)
    }

    /// Returns the bit depth by reading the PNG IHDR chunk (byte 24).
    ///
    /// Returns 0 if the PNG data is too short or invalid.
    pub fn bit_depth(&self) -> u8 {
        self.png.get(24).copied().unwrap_or(0)
    }

    /// Returns the mask's top-left position on the image from the PNG `oFFs`
    /// chunk, walking chunk headers up to the first `IDAT` without decoding.
    ///
    /// Returns `None` if there is no `oFFs` chunk before the image data, if
    /// its unit is not pixels or its length is not 9 bytes, or if the PNG
    /// data is too short or invalid.
    /// A mask without an offset is image-sized and placed at the origin.
    pub fn offset(&self) -> Option<(i32, i32)> {
        if !self.is_valid() {
            return None;
        }
        let mut pos = PNG_SIGNATURE.len();
        while let Some(header) = self.png.get(pos..pos.checked_add(8)?) {
            let len = u32::from_be_bytes(header[..4].try_into().ok()?) as usize;
            let chunk_type: [u8; 4] = header[4..8].try_into().ok()?;
            let data_start = pos + 8;
            match &chunk_type {
                b"IDAT" | b"IEND" => return None,
                t if *t == OFFS_CHUNK => {
                    let data = self.png.get(data_start..data_start.checked_add(len)?)?;
                    if len != 9 || data[8] != OFFS_UNIT_PIXEL {
                        return None;
                    }
                    let x = i32::from_be_bytes(data[0..4].try_into().ok()?);
                    let y = i32::from_be_bytes(data[4..8].try_into().ok()?);
                    return Some((x, y));
                }
                _ => {}
            }
            // Chunk data is followed by a 4-byte CRC.
            pos = data_start.checked_add(len)?.checked_add(4)?;
        }
        None
    }

    /// Encodes raw 8-bit grayscale pixels into a PNG.
    ///
    /// For `bit_depth == 1`, pixel values must be `0` or `1` and will be packed
    /// into 1-bit-per-pixel PNG rows (MSB first, 8 pixels per byte, with
    /// zero-padding on the last byte if `width` is not a multiple of 8).
    ///
    /// For `bit_depth == 8`, pixels are encoded directly as 8-bit grayscale.
    ///
    /// # Errors
    ///
    /// Returns an error if `bit_depth` is not 1 or 8, or if `pixels.len()`
    /// does not equal `width * height`.
    pub fn encode(
        pixels: &[u8],
        width: u32,
        height: u32,
        bit_depth: u8,
    ) -> Result<Self, crate::Error> {
        Self::encode_impl(pixels, width, height, bit_depth, None)
    }

    /// Encodes raw 8-bit grayscale pixels into a PNG positioned at `offset`
    /// on the image.
    ///
    /// Identical to [`encode`](Self::encode) but also writes an `oFFs` chunk
    /// (pixel units) so readers place the mask's top-left at `(x, y)` on the
    /// image, treating pixels outside it as background.
    ///
    /// # Errors
    ///
    /// Returns an error if `bit_depth` is not 1 or 8, or if `pixels.len()`
    /// does not equal `width * height`.
    ///
    /// # Examples
    ///
    /// ```
    /// use edgefirst_client::MaskData;
    ///
    /// let mask = MaskData::encode_with_offset(&[1, 0, 0, 1], 2, 2, 1, (120, 48)).unwrap();
    /// assert_eq!(mask.offset(), Some((120, 48)));
    /// assert_eq!((mask.width(), mask.height()), (2, 2));
    /// ```
    pub fn encode_with_offset(
        pixels: &[u8],
        width: u32,
        height: u32,
        bit_depth: u8,
        offset: (i32, i32),
    ) -> Result<Self, crate::Error> {
        Self::encode_impl(pixels, width, height, bit_depth, Some(offset))
    }

    fn encode_impl(
        pixels: &[u8],
        width: u32,
        height: u32,
        bit_depth: u8,
        offset: Option<(i32, i32)>,
    ) -> Result<Self, crate::Error> {
        if bit_depth != 1 && bit_depth != 8 {
            return Err(crate::Error::InvalidParameters(format!(
                "bit_depth must be 1 or 8, got {}",
                bit_depth
            )));
        }
        let expected = (width as usize) * (height as usize);
        if pixels.len() != expected {
            return Err(crate::Error::InvalidParameters(format!(
                "pixel count mismatch: expected {}, got {}",
                expected,
                pixels.len()
            )));
        }

        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, width, height);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(match bit_depth {
                1 => png::BitDepth::One,
                8 => png::BitDepth::Eight,
                _ => unreachable!(),
            });

            let mut writer = encoder.write_header().map_err(|e| {
                crate::Error::InvalidParameters(format!("PNG header write failed: {}", e))
            })?;

            if let Some((x, y)) = offset {
                let mut data = [0u8; 9];
                data[0..4].copy_from_slice(&x.to_be_bytes());
                data[4..8].copy_from_slice(&y.to_be_bytes());
                data[8] = OFFS_UNIT_PIXEL;
                writer
                    .write_chunk(png::chunk::ChunkType(OFFS_CHUNK), &data)
                    .map_err(|e| {
                        crate::Error::InvalidParameters(format!(
                            "PNG oFFs chunk write failed: {}",
                            e
                        ))
                    })?;
            }

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
                    writer.write_image_data(&packed).map_err(|e| {
                        crate::Error::InvalidParameters(format!(
                            "PNG image data write failed: {}",
                            e
                        ))
                    })?;
                }
                8 => {
                    writer.write_image_data(pixels).map_err(|e| {
                        crate::Error::InvalidParameters(format!(
                            "PNG image data write failed: {}",
                            e
                        ))
                    })?;
                }
                _ => unreachable!(),
            }
        }
        Ok(Self { png: buf })
    }

    /// Encodes raw 16-bit grayscale pixels into a PNG.
    ///
    /// Each `u16` value is written as two bytes in big-endian order, matching
    /// the PNG 16-bit grayscale format.
    ///
    /// # Errors
    ///
    /// Returns an error if `pixels.len()` does not equal `width * height`.
    pub fn encode_16bit(pixels: &[u16], width: u32, height: u32) -> Result<Self, crate::Error> {
        let expected = (width as usize) * (height as usize);
        if pixels.len() != expected {
            return Err(crate::Error::InvalidParameters(format!(
                "pixel count mismatch: expected {}, got {}",
                expected,
                pixels.len()
            )));
        }

        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, width, height);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Sixteen);

            let mut writer = encoder.write_header().map_err(|e| {
                crate::Error::InvalidParameters(format!("PNG header write failed: {}", e))
            })?;

            let raw: Vec<u8> = pixels.iter().flat_map(|&v| v.to_be_bytes()).collect();
            writer.write_image_data(&raw).map_err(|e| {
                crate::Error::InvalidParameters(format!("PNG image data write failed: {}", e))
            })?;
        }
        Ok(Self { png: buf })
    }

    /// Decodes the PNG image to raw pixel bytes.
    ///
    /// For 1-bit PNGs, each pixel is unpacked to a single byte (`0` or `1`).
    /// For 8-bit PNGs, pixel bytes are returned directly.
    /// For 16-bit PNGs, each pixel yields two bytes in big-endian order.
    ///
    /// # Errors
    ///
    /// Returns an error if the PNG data is malformed or cannot be decoded.
    pub fn decode(&self) -> Result<Vec<u8>, crate::Error> {
        let decoder = png::Decoder::new(std::io::Cursor::new(self.png.as_slice()));
        let mut reader = decoder
            .read_info()
            .map_err(|e| crate::Error::InvalidParameters(format!("PNG info read failed: {}", e)))?;

        // Guard against decompression bombs
        let info = reader.info();
        let total_pixels = info.width as u64 * info.height as u64;
        if total_pixels > MAX_PIXELS {
            return Err(crate::Error::InvalidParameters(format!(
                "PNG dimensions {}x{} exceed maximum of {} pixels",
                info.width, info.height, MAX_PIXELS
            )));
        }

        let buffer_size = reader.output_buffer_size().ok_or_else(|| {
            crate::Error::InvalidParameters("PNG output buffer size unavailable".to_string())
        })?;
        let mut raw = vec![0u8; buffer_size];
        let info = reader.next_frame(&mut raw).map_err(|e| {
            crate::Error::InvalidParameters(format!("PNG frame read failed: {}", e))
        })?;
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
            Ok(unpacked)
        } else {
            Ok(raw)
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
        let mask = MaskData::encode(&pixels, 3, 3, 8).unwrap();

        assert_eq!(mask.width(), 3);
        assert_eq!(mask.height(), 3);
        assert_eq!(mask.bit_depth(), 8);

        let decoded = mask.decode().unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn test_encode_decode_1bit() {
        // 8x2 image, byte-aligned width
        let pixels: Vec<u8> = vec![
            1, 0, 1, 0, 1, 0, 1, 0, // row 0
            0, 1, 0, 1, 0, 1, 0, 1, // row 1
        ];
        let mask = MaskData::encode(&pixels, 8, 2, 1).unwrap();

        assert_eq!(mask.width(), 8);
        assert_eq!(mask.height(), 2);
        assert_eq!(mask.bit_depth(), 1);

        let decoded = mask.decode().unwrap();
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn test_encode_decode_16bit() {
        // 2x2 image with u16 values
        let pixels: Vec<u16> = vec![0, 256, 65535, 1024];
        let mask = MaskData::encode_16bit(&pixels, 2, 2).unwrap();

        assert_eq!(mask.width(), 2);
        assert_eq!(mask.height(), 2);
        assert_eq!(mask.bit_depth(), 16);

        let decoded = mask.decode().unwrap();
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
        let mask = MaskData::encode(&pixels, width, height, 8).unwrap();

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
        let original = MaskData::encode(&pixels, 3, 2, 8).unwrap();

        let bytes = original.into_bytes();
        let reconstructed = MaskData::from_png(bytes);

        assert_eq!(reconstructed.width(), 3);
        assert_eq!(reconstructed.height(), 2);
        assert_eq!(reconstructed.bit_depth(), 8);
        assert_eq!(reconstructed.decode().unwrap(), pixels);
    }

    #[test]
    fn test_1bit_non_aligned_width() {
        // 5x3 image: width not a multiple of 8
        let pixels: Vec<u8> = vec![
            1, 0, 1, 1, 0, // row 0
            0, 1, 0, 0, 1, // row 1
            1, 1, 1, 0, 0, // row 2
        ];
        let mask = MaskData::encode(&pixels, 5, 3, 1).unwrap();

        assert_eq!(mask.width(), 5);
        assert_eq!(mask.height(), 3);
        assert_eq!(mask.bit_depth(), 1);

        let decoded = mask.decode().unwrap();
        assert_eq!(decoded, pixels);
    }

    // =========================================================================
    // from_png_checked validation tests
    // =========================================================================

    #[test]
    fn test_from_png_empty_bytes() {
        let result = MaskData::from_png_checked(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_png_truncated() {
        // Just the magic bytes, no IHDR
        let result = MaskData::from_png_checked(PNG_SIGNATURE.to_vec());
        assert!(result.is_err());
    }

    #[test]
    fn test_from_png_garbage() {
        let result = MaskData::from_png_checked(vec![0u8; 64]);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_png_wrong_color_type() {
        // Build a valid-length buffer with correct signature but wrong color type
        let mut fake_png = vec![0u8; MIN_PNG_LEN];
        fake_png[..8].copy_from_slice(&PNG_SIGNATURE);
        fake_png[25] = 2; // RGB instead of grayscale
        let result = MaskData::from_png_checked(fake_png);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_png_checked_valid() {
        let pixels: Vec<u8> = vec![0, 128, 255, 64];
        let mask = MaskData::encode(&pixels, 2, 2, 8).unwrap();
        let bytes = mask.into_bytes();
        let result = MaskData::from_png_checked(bytes);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_valid() {
        let pixels: Vec<u8> = vec![0, 128, 255, 64];
        let mask = MaskData::encode(&pixels, 2, 2, 8).unwrap();
        assert!(mask.is_valid());

        let invalid = MaskData::from_png(vec![1, 2, 3]);
        assert!(!invalid.is_valid());
    }

    // =========================================================================
    // Header reads on invalid data return 0 instead of panicking
    // =========================================================================

    #[test]
    fn test_width_height_bit_depth_short_data() {
        let mask = MaskData::from_png(vec![]);
        assert_eq!(mask.width(), 0);
        assert_eq!(mask.height(), 0);
        assert_eq!(mask.bit_depth(), 0);

        let mask2 = MaskData::from_png(vec![0; 10]);
        assert_eq!(mask2.width(), 0);
        assert_eq!(mask2.height(), 0);
        assert_eq!(mask2.bit_depth(), 0);
    }

    #[test]
    fn test_decode_invalid_data_returns_error() {
        let mask = MaskData::from_png(vec![1, 2, 3]);
        assert!(mask.decode().is_err());
    }

    #[test]
    fn test_encode_invalid_bit_depth() {
        let result = MaskData::encode(&[0; 4], 2, 2, 4);
        assert!(result.is_err());
    }

    // =========================================================================
    // oFFs offset tests
    // =========================================================================

    /// Encodes a 1x1 1-bit PNG with a raw `oFFs` chunk payload.
    fn png_with_offs(data: &[u8]) -> MaskData {
        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, 1, 1);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::One);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_chunk(png::chunk::ChunkType(OFFS_CHUNK), data)
                .unwrap();
            writer.write_image_data(&[0x80]).unwrap();
        }
        MaskData::from_png(buf)
    }

    #[test]
    fn test_offset_absent() {
        let mask = MaskData::encode(&[1, 0, 0, 1], 2, 2, 1).unwrap();
        assert_eq!(mask.offset(), None);
    }

    #[test]
    fn test_offset_present() {
        let mut data = Vec::new();
        data.extend_from_slice(&37i32.to_be_bytes());
        data.extend_from_slice(&(-5i32).to_be_bytes());
        data.push(OFFS_UNIT_PIXEL);
        assert_eq!(png_with_offs(&data).offset(), Some((37, -5)));
    }

    #[test]
    fn test_offset_non_pixel_unit_ignored() {
        let mut data = Vec::new();
        data.extend_from_slice(&37i32.to_be_bytes());
        data.extend_from_slice(&5i32.to_be_bytes());
        data.push(1);
        assert_eq!(png_with_offs(&data).offset(), None);
    }

    #[test]
    fn test_offset_invalid_data() {
        assert_eq!(MaskData::from_png(vec![]).offset(), None);
        assert_eq!(MaskData::from_png(vec![1, 2, 3]).offset(), None);

        // Valid prefix truncated mid-chunk must not panic.
        let mask = MaskData::encode_with_offset(&[1], 1, 1, 1, (3, 4)).unwrap();
        let bytes = mask.as_bytes();
        for len in 0..bytes.len() {
            let _ = MaskData::from_png(bytes[..len].to_vec()).offset();
        }
    }

    #[test]
    fn test_encode_with_offset_roundtrip() {
        let pixels: Vec<u8> = vec![
            1, 0, 1, 1, 0, // row 0
            0, 1, 0, 0, 1, // row 1
            1, 1, 1, 0, 0, // row 2
        ];
        for bit_depth in [1u8, 8] {
            let mask = MaskData::encode_with_offset(&pixels, 5, 3, bit_depth, (-2, 1024)).unwrap();
            assert_eq!(mask.width(), 5);
            assert_eq!(mask.height(), 3);
            assert_eq!(mask.bit_depth(), bit_depth);
            assert_eq!(mask.offset(), Some((-2, 1024)));
            assert_eq!(mask.decode().unwrap(), pixels);
            assert!(MaskData::from_png_checked(mask.into_bytes()).is_ok());
        }
    }

    #[test]
    fn test_encode_with_offset_invalid_bit_depth() {
        assert!(MaskData::encode_with_offset(&[0; 4], 2, 2, 4, (0, 0)).is_err());
    }

    #[test]
    fn test_encode_pixel_count_mismatch() {
        let result = MaskData::encode(&[0; 3], 2, 2, 8);
        assert!(result.is_err());
    }
}
