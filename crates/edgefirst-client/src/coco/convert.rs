// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Coordinate and geometry conversion functions between COCO and EdgeFirst
//! formats.
//!
//! ## Coordinate Systems
//!
//! - **COCO**: Pixel coordinates, top-left origin for bboxes
//! - **EdgeFirst JSON**: Normalized 0-1, top-left origin for Box2d
//! - **EdgeFirst Arrow**: Normalized 0-1, center-point for box2d column

use super::types::{CocoCompressedRle, CocoRle, CocoSegmentation};
use crate::{Box2d, Error, Mask};

// =============================================================================
// Bounding Box Conversion
// =============================================================================

/// Convert COCO bbox `[x, y, w, h]` (top-left, pixels) to EdgeFirst `Box2d`
/// (top-left, normalized).
///
/// EdgeFirst `Box2d` uses `{x, y, w, h}` where `(x, y)` is the top-left corner,
/// normalized to the range `[0, 1]`.
///
/// # Arguments
/// * `bbox` - COCO bounding box `[x_min, y_min, width, height]` in pixels
/// * `image_width` - Image width in pixels
/// * `image_height` - Image height in pixels
///
/// # Returns
/// EdgeFirst `Box2d` with normalized coordinates
///
/// # Example
/// ```
/// use edgefirst_client::coco::coco_bbox_to_box2d;
///
/// let coco_bbox = [100.0, 50.0, 200.0, 150.0]; // x=100, y=50, w=200, h=150
/// let box2d = coco_bbox_to_box2d(&coco_bbox, 640, 480);
///
/// assert!((box2d.left() - 100.0 / 640.0).abs() < 1e-6);
/// assert!((box2d.top() - 50.0 / 480.0).abs() < 1e-6);
/// ```
pub fn coco_bbox_to_box2d(bbox: &[f64; 4], image_width: u32, image_height: u32) -> Box2d {
    let [x, y, w, h] = *bbox;
    let img_w = image_width as f64;
    let img_h = image_height as f64;

    Box2d::new(
        (x / img_w) as f32,
        (y / img_h) as f32,
        (w / img_w) as f32,
        (h / img_h) as f32,
    )
}

/// Convert EdgeFirst `Box2d` to COCO bbox `[x, y, w, h]` (top-left, pixels).
///
/// # Arguments
/// * `box2d` - EdgeFirst `Box2d` (normalized 0-1, top-left origin)
/// * `image_width` - Image width in pixels
/// * `image_height` - Image height in pixels
///
/// # Returns
/// COCO bbox `[x_min, y_min, width, height]` in pixels
pub fn box2d_to_coco_bbox(box2d: &Box2d, image_width: u32, image_height: u32) -> [f64; 4] {
    let img_w = image_width as f64;
    let img_h = image_height as f64;

    [
        (box2d.left() as f64) * img_w,
        (box2d.top() as f64) * img_h,
        (box2d.width() as f64) * img_w,
        (box2d.height() as f64) * img_h,
    ]
}

/// Validate that a COCO bounding box is within image bounds.
///
/// # Arguments
/// * `bbox` - COCO bounding box `[x, y, w, h]` in pixels
/// * `image_width` - Image width in pixels
/// * `image_height` - Image height in pixels
///
/// # Returns
/// `Ok(())` if valid, `Err` with description if invalid
pub fn validate_coco_bbox(
    bbox: &[f64; 4],
    image_width: u32,
    image_height: u32,
) -> Result<(), Error> {
    let [x, y, w, h] = *bbox;

    if w <= 0.0 || h <= 0.0 {
        return Err(Error::CocoError(format!(
            "Width and height must be positive: w={}, h={}",
            w, h
        )));
    }

    // Allow slight overflow for floating point precision
    let epsilon = 1.0;
    if x < -epsilon || y < -epsilon {
        return Err(Error::CocoError(format!(
            "Bbox has negative coordinates: x={}, y={}",
            x, y
        )));
    }

    if x + w > (image_width as f64) + epsilon || y + h > (image_height as f64) + epsilon {
        return Err(Error::CocoError(format!(
            "Bbox exceeds image bounds: [{}, {}, {}, {}] for {}x{} image",
            x, y, w, h, image_width, image_height
        )));
    }

    Ok(())
}

// =============================================================================
// Polygon Conversion
// =============================================================================

/// Convert COCO polygon segmentation to EdgeFirst `Mask` format.
///
/// COCO polygons: `[[x1,y1,x2,y2,...], [x3,y3,...]]` (nested, pixel
/// coordinates) EdgeFirst Mask: `Vec<Vec<(f32, f32)>>` (nested, normalized 0-1)
///
/// # Arguments
/// * `polygons` - COCO polygon array (nested Vec of pixel coordinates)
/// * `image_width` - Image width in pixels
/// * `image_height` - Image height in pixels
///
/// # Returns
/// EdgeFirst `Mask` with normalized coordinates
pub fn coco_polygon_to_mask(polygons: &[Vec<f64>], image_width: u32, image_height: u32) -> Mask {
    let img_w = image_width as f64;
    let img_h = image_height as f64;

    let converted: Vec<Vec<(f32, f32)>> = polygons
        .iter()
        .filter(|poly| poly.len() >= 6) // Need at least 3 points
        .map(|polygon| {
            polygon
                .chunks(2)
                .filter_map(|chunk| {
                    if chunk.len() == 2 {
                        Some(((chunk[0] / img_w) as f32, (chunk[1] / img_h) as f32))
                    } else {
                        None
                    }
                })
                .collect()
        })
        .filter(|poly: &Vec<(f32, f32)>| poly.len() >= 3) // Still need 3+ points after conversion
        .collect();

    Mask::new(converted)
}

/// Convert EdgeFirst `Mask` format to COCO polygon segmentation.
///
/// # Arguments
/// * `mask` - EdgeFirst `Mask` with normalized coordinates
/// * `image_width` - Image width in pixels
/// * `image_height` - Image height in pixels
///
/// # Returns
/// COCO polygon array (nested Vec of pixel coordinates)
pub fn mask_to_coco_polygon(mask: &Mask, image_width: u32, image_height: u32) -> Vec<Vec<f64>> {
    let img_w = image_width as f64;
    let img_h = image_height as f64;

    mask.polygon
        .iter()
        .filter(|poly| poly.len() >= 3) // Need at least 3 points
        .map(|polygon| {
            polygon
                .iter()
                .flat_map(|(x, y)| vec![(*x as f64) * img_w, (*y as f64) * img_h])
                .collect()
        })
        .collect()
}

// =============================================================================
// RLE Decoding
// =============================================================================

/// Decode uncompressed RLE to binary mask.
///
/// **CRITICAL**: RLE uses column-major (Fortran) order, starting with
/// background.
///
/// # Arguments
/// * `rle` - COCO RLE with counts array
///
/// # Returns
/// Binary mask as `Vec<u8>` in row-major order, plus `(height, width)`
pub fn decode_rle(rle: &CocoRle) -> Result<(Vec<u8>, u32, u32), Error> {
    let [height, width] = rle.size;
    let total_pixels = (width as usize) * (height as usize);

    // Validate counts sum
    let counts_sum: u64 = rle.counts.iter().map(|&c| c as u64).sum();
    if counts_sum != total_pixels as u64 {
        return Err(Error::CocoError(format!(
            "RLE counts sum {} does not match image size {}x{} = {}",
            counts_sum, width, height, total_pixels
        )));
    }

    // Decode to column-major flat array
    let mut column_major = vec![0u8; total_pixels];
    let mut pos = 0usize;
    let mut is_foreground = false; // Starts with background

    for &count in &rle.counts {
        let count = count as usize;
        if is_foreground {
            for i in pos..(pos + count).min(column_major.len()) {
                column_major[i] = 1;
            }
        }
        pos += count;
        is_foreground = !is_foreground;
    }

    // Convert column-major to row-major
    let mut row_major = vec![0u8; total_pixels];
    for col in 0..width as usize {
        for row in 0..height as usize {
            let col_idx = col * (height as usize) + row;
            let row_idx = row * (width as usize) + col;
            if col_idx < column_major.len() && row_idx < row_major.len() {
                row_major[row_idx] = column_major[col_idx];
            }
        }
    }

    Ok((row_major, height, width))
}

/// Decode LEB128 encoded string to counts array.
///
/// Based on pycocotools encoding.
fn decode_leb128(s: &str) -> Result<Vec<u32>, Error> {
    let bytes = s.as_bytes();
    let mut counts = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let mut value: i64 = 0;
        let mut shift = 0;
        let mut more = true;

        while more && i < bytes.len() {
            let byte = bytes[i] as i64;
            i += 1;

            // Decode based on character ranges (pycocotools encoding)
            let decoded = if (48..96).contains(&byte) {
                byte - 48 // '0'-'_'
            } else if byte >= 96 {
                byte - 96 + 48 // 'a' and above
            } else {
                return Err(Error::CocoError(format!(
                    "Invalid LEB128 character: {}",
                    byte as u8 as char
                )));
            };

            value |= (decoded & 0x1F) << shift;
            more = decoded >= 32;
            shift += 5;
        }

        // Sign extend if needed
        if shift < 32 && (value & (1 << (shift - 1))) != 0 {
            value |= (-1i64) << shift;
        }

        counts.push(value);
    }

    // Convert from diff encoding to absolute counts
    let mut result = Vec::with_capacity(counts.len());
    let mut prev: i64 = 0;
    for diff in counts {
        prev += diff;
        result.push(prev.max(0) as u32);
    }

    Ok(result)
}

/// Decode compressed RLE (LEB128) to binary mask.
pub fn decode_compressed_rle(compressed: &CocoCompressedRle) -> Result<(Vec<u8>, u32, u32), Error> {
    let counts = decode_leb128(&compressed.counts)?;

    let rle = CocoRle {
        counts,
        size: compressed.size,
    };

    decode_rle(&rle)
}

// =============================================================================
// Contour Extraction
// =============================================================================

/// Convert binary mask to polygon contours.
///
/// Uses a simple boundary tracing algorithm to extract outer contours from
/// a binary segmentation mask.
///
/// # Arguments
/// * `mask` - Binary mask (0 = background, 1 = foreground) in row-major order
/// * `width` - Image width
/// * `height` - Image height
///
/// # Returns
/// Vector of contours, each contour is a vector of `(x, y)` pixel coordinates
pub fn mask_to_contours(mask: &[u8], width: u32, height: u32) -> Vec<Vec<(f64, f64)>> {
    let mut contours = Vec::new();
    let mut visited = vec![false; mask.len()];

    let w = width as usize;
    let h = height as usize;

    for start_y in 0..h {
        for start_x in 0..w {
            let idx = start_y * w + start_x;
            if mask[idx] == 1 && !visited[idx] {
                // Check if this is a boundary pixel (has at least one neighbor that's 0 or
                // edge)
                let is_boundary = start_x == 0
                    || start_x == w - 1
                    || start_y == 0
                    || start_y == h - 1
                    || (start_x > 0 && mask[idx - 1] == 0)
                    || (start_x < w - 1 && mask[idx + 1] == 0)
                    || (start_y > 0 && mask[idx - w] == 0)
                    || (start_y < h - 1 && mask[idx + w] == 0);

                if is_boundary
                    && let Some(contour) = trace_contour(mask, w, h, start_x, start_y, &mut visited)
                    && contour.len() >= 3
                {
                    contours.push(contour);
                }
            }
        }
    }

    contours
}

/// Trace a contour starting from the given point using 8-connectivity.
fn trace_contour(
    mask: &[u8],
    width: usize,
    height: usize,
    start_x: usize,
    start_y: usize,
    visited: &mut [bool],
) -> Option<Vec<(f64, f64)>> {
    let mut contour = Vec::new();
    let mut x = start_x;
    let mut y = start_y;

    // Direction vectors for 8-connectivity: E, SE, S, SW, W, NW, N, NE
    let dx: [i32; 8] = [1, 1, 0, -1, -1, -1, 0, 1];
    let dy: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];

    let mut dir = 0usize; // Start going east
    let max_steps = width * height;
    let mut steps = 0;

    loop {
        let idx = y * width + x;
        if !visited[idx] {
            contour.push((x as f64, y as f64));
            visited[idx] = true;
        }

        // Find next boundary pixel
        let mut found = false;
        for i in 0..8 {
            let new_dir = (dir + i) % 8;
            let nx = x as i32 + dx[new_dir];
            let ny = y as i32 + dy[new_dir];

            if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                let nidx = (ny as usize) * width + (nx as usize);
                if mask[nidx] == 1 {
                    x = nx as usize;
                    y = ny as usize;
                    dir = (new_dir + 5) % 8; // Turn around and search from there
                    found = true;
                    break;
                }
            }
        }

        if !found || (x == start_x && y == start_y && contour.len() > 2) {
            break;
        }

        steps += 1;
        if steps > max_steps {
            break; // Safety limit
        }
    }

    if contour.len() >= 3 {
        Some(contour)
    } else {
        None
    }
}

/// Convert RLE segmentation to EdgeFirst `Mask`.
///
/// Decodes the RLE, extracts contours, and normalizes to `[0, 1]` range.
pub fn coco_rle_to_mask(rle: &CocoRle, image_width: u32, image_height: u32) -> Result<Mask, Error> {
    let (binary_mask, height, width) = decode_rle(rle)?;
    let contours = mask_to_contours(&binary_mask, width, height);

    // Normalize contours to 0-1 range
    let normalized: Vec<Vec<(f32, f32)>> = contours
        .iter()
        .map(|contour| {
            contour
                .iter()
                .map(|(x, y)| {
                    (
                        (*x / image_width as f64) as f32,
                        (*y / image_height as f64) as f32,
                    )
                })
                .collect()
        })
        .collect();

    Ok(Mask::new(normalized))
}

/// Convert any COCO segmentation to EdgeFirst `Mask`.
///
/// Handles all segmentation types: polygon, RLE, and compressed RLE.
pub fn coco_segmentation_to_mask(
    segmentation: &CocoSegmentation,
    image_width: u32,
    image_height: u32,
) -> Result<Mask, Error> {
    match segmentation {
        CocoSegmentation::Polygon(polygons) => {
            Ok(coco_polygon_to_mask(polygons, image_width, image_height))
        }
        CocoSegmentation::Rle(rle) => coco_rle_to_mask(rle, image_width, image_height),
        CocoSegmentation::CompressedRle(compressed) => {
            let counts = decode_leb128(&compressed.counts)?;
            let rle = CocoRle {
                counts,
                size: compressed.size,
            };
            coco_rle_to_mask(&rle, image_width, image_height)
        }
    }
}

// =============================================================================
// Area Calculation
// =============================================================================

/// Calculate area from COCO segmentation (in pixels²).
pub fn calculate_coco_area(segmentation: &CocoSegmentation) -> Result<f64, Error> {
    match segmentation {
        CocoSegmentation::Polygon(polygons) => {
            // Use shoelace formula for polygon area
            let mut total_area = 0.0;
            for polygon in polygons {
                total_area += shoelace_area(polygon);
            }
            Ok(total_area)
        }
        CocoSegmentation::Rle(rle) => {
            let (mask, _, _) = decode_rle(rle)?;
            let area = mask.iter().filter(|&&v| v == 1).count() as f64;
            Ok(area)
        }
        CocoSegmentation::CompressedRle(compressed) => {
            let (mask, _, _) = decode_compressed_rle(compressed)?;
            let area = mask.iter().filter(|&&v| v == 1).count() as f64;
            Ok(area)
        }
    }
}

/// Calculate polygon area using the shoelace formula.
fn shoelace_area(polygon: &[f64]) -> f64 {
    if polygon.len() < 6 {
        return 0.0;
    }

    let n = polygon.len() / 2;
    let mut area = 0.0;

    for i in 0..n {
        let j = (i + 1) % n;
        let x1 = polygon[i * 2];
        let y1 = polygon[i * 2 + 1];
        let x2 = polygon[j * 2];
        let y2 = polygon[j * 2 + 1];
        area += x1 * y2 - x2 * y1;
    }

    (area / 2.0).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Bounding Box Tests
    // =========================================================================

    #[test]
    fn test_coco_bbox_to_box2d() {
        let bbox = [100.0, 50.0, 200.0, 150.0];
        let box2d = coco_bbox_to_box2d(&bbox, 640, 480);

        assert!((box2d.left() - 100.0 / 640.0).abs() < 1e-6);
        assert!((box2d.top() - 50.0 / 480.0).abs() < 1e-6);
        assert!((box2d.width() - 200.0 / 640.0).abs() < 1e-6);
        assert!((box2d.height() - 150.0 / 480.0).abs() < 1e-6);
    }

    #[test]
    fn test_box2d_to_coco_bbox() {
        let box2d = Box2d::new(0.15625, 0.104167, 0.3125, 0.3125);
        let bbox = box2d_to_coco_bbox(&box2d, 640, 480);

        assert!((bbox[0] - 100.0).abs() < 1.0);
        assert!((bbox[1] - 50.0).abs() < 1.0);
        assert!((bbox[2] - 200.0).abs() < 1.0);
        assert!((bbox[3] - 150.0).abs() < 1.0);
    }

    #[test]
    fn test_bbox_roundtrip() {
        let original = [123.5, 456.7, 89.1, 234.5];
        let image_w = 1920;
        let image_h = 1080;

        let box2d = coco_bbox_to_box2d(&original, image_w, image_h);
        let restored = box2d_to_coco_bbox(&box2d, image_w, image_h);

        for i in 0..4 {
            assert!(
                (original[i] - restored[i]).abs() < 1.0,
                "Mismatch at index {}: {} vs {}",
                i,
                original[i],
                restored[i]
            );
        }
    }

    #[test]
    fn test_validate_coco_bbox_valid() {
        assert!(validate_coco_bbox(&[10.0, 20.0, 100.0, 80.0], 640, 480).is_ok());
        assert!(validate_coco_bbox(&[0.0, 0.0, 640.0, 480.0], 640, 480).is_ok());
    }

    #[test]
    fn test_validate_coco_bbox_invalid() {
        // Negative dimensions
        assert!(validate_coco_bbox(&[10.0, 20.0, -100.0, 80.0], 640, 480).is_err());
        // Zero dimensions
        assert!(validate_coco_bbox(&[10.0, 20.0, 0.0, 80.0], 640, 480).is_err());
        // Out of bounds
        assert!(validate_coco_bbox(&[600.0, 400.0, 100.0, 100.0], 640, 480).is_err());
    }

    // =========================================================================
    // Polygon Tests
    // =========================================================================

    #[test]
    fn test_coco_polygon_to_mask() {
        let polygons = vec![vec![100.0, 100.0, 200.0, 100.0, 200.0, 200.0, 100.0, 200.0]];
        let mask = coco_polygon_to_mask(&polygons, 400, 400);

        assert_eq!(mask.polygon.len(), 1);
        assert_eq!(mask.polygon[0].len(), 4);

        // Check normalized coordinates
        assert!((mask.polygon[0][0].0 - 0.25).abs() < 1e-6);
        assert!((mask.polygon[0][0].1 - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_mask_to_coco_polygon() {
        let mask = Mask::new(vec![vec![
            (0.25, 0.25),
            (0.5, 0.25),
            (0.5, 0.5),
            (0.25, 0.5),
        ]]);

        let polygons = mask_to_coco_polygon(&mask, 400, 400);

        assert_eq!(polygons.len(), 1);
        assert_eq!(polygons[0].len(), 8); // 4 points * 2 coords

        assert!((polygons[0][0] - 100.0).abs() < 1e-6);
        assert!((polygons[0][1] - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_polygon_roundtrip() {
        let original = vec![vec![
            50.0, 60.0, 150.0, 60.0, 180.0, 120.0, 150.0, 180.0, 50.0, 180.0, 20.0, 120.0,
        ]];

        let image_w = 300;
        let image_h = 300;

        let mask = coco_polygon_to_mask(&original, image_w, image_h);
        let restored = mask_to_coco_polygon(&mask, image_w, image_h);

        assert_eq!(original.len(), restored.len());
        assert_eq!(original[0].len(), restored[0].len());

        for i in 0..original[0].len() {
            assert!(
                (original[0][i] - restored[0][i]).abs() < 1.0,
                "Mismatch at index {}: {} vs {}",
                i,
                original[0][i],
                restored[0][i]
            );
        }
    }

    #[test]
    fn test_polygon_multiple_regions() {
        let polygons = vec![
            vec![10.0, 10.0, 50.0, 10.0, 50.0, 50.0, 10.0, 50.0],
            vec![60.0, 60.0, 90.0, 60.0, 90.0, 90.0, 60.0, 90.0],
        ];

        let mask = coco_polygon_to_mask(&polygons, 100, 100);

        assert_eq!(mask.polygon.len(), 2);
        assert_eq!(mask.polygon[0].len(), 4);
        assert_eq!(mask.polygon[1].len(), 4);
    }

    #[test]
    fn test_polygon_filters_too_small() {
        let polygons = vec![
            vec![10.0, 10.0],                         // Only 1 point - should be filtered
            vec![10.0, 10.0, 50.0, 50.0],             // Only 2 points - should be filtered
            vec![10.0, 10.0, 50.0, 10.0, 50.0, 50.0], // 3 points - should be kept
        ];

        let mask = coco_polygon_to_mask(&polygons, 100, 100);

        assert_eq!(mask.polygon.len(), 1);
    }

    // =========================================================================
    // RLE Tests
    // =========================================================================

    #[test]
    fn test_decode_rle_simple() {
        // 2x3 image with pattern:
        // 0 1
        // 1 1
        // 0 0
        // Column-major: [0,1,0], [1,1,0] → counts: [1,1,1, 0,2,1] simplified to
        // [1,2,1,2]
        let rle = CocoRle {
            counts: vec![1, 2, 1, 2], /* bg=1, fg=2, bg=1, fg=2 (wait, that's 6 not 6... let me
                                       * recalc) */
            size: [3, 2], // height=3, width=2
        };

        // Total pixels = 6
        // counts = [1, 2, 1, 2] sums to 6 ✓
        // Column 0: bg=1 (pixel 0), fg=2 (pixels 1,2)
        // Column 1: bg=1 (pixel 3), fg=2 (pixels 4,5)

        let result = decode_rle(&rle);
        assert!(result.is_ok());

        let (mask, height, width) = result.unwrap();
        assert_eq!(height, 3);
        assert_eq!(width, 2);
        assert_eq!(mask.len(), 6);

        // Row-major layout:
        // Row 0: mask[0]=col0_row0, mask[1]=col1_row0
        // Row 1: mask[2]=col0_row1, mask[3]=col1_row1
        // Row 2: mask[4]=col0_row2, mask[5]=col1_row2
    }

    #[test]
    fn test_decode_rle_all_background() {
        let rle = CocoRle {
            counts: vec![100], // All background
            size: [10, 10],
        };

        let (mask, _, _) = decode_rle(&rle).unwrap();
        assert!(mask.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_decode_rle_all_foreground() {
        let rle = CocoRle {
            counts: vec![0, 100], // No background, all foreground
            size: [10, 10],
        };

        let (mask, _, _) = decode_rle(&rle).unwrap();
        assert!(mask.iter().all(|&v| v == 1));
    }

    #[test]
    fn test_decode_rle_invalid_counts() {
        let rle = CocoRle {
            counts: vec![50], // Only 50 pixels, but image is 100
            size: [10, 10],
        };

        let result = decode_rle(&rle);
        assert!(result.is_err());
    }

    // =========================================================================
    // Area Calculation Tests
    // =========================================================================

    #[test]
    fn test_shoelace_area_square() {
        // 100x100 square
        let polygon = vec![0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let area = shoelace_area(&polygon);
        assert!((area - 10000.0).abs() < 1e-6);
    }

    #[test]
    fn test_shoelace_area_triangle() {
        // Triangle with vertices at (0,0), (100,0), (50,100)
        // Area = 0.5 * base * height = 0.5 * 100 * 100 = 5000
        let polygon = vec![0.0, 0.0, 100.0, 0.0, 50.0, 100.0];
        let area = shoelace_area(&polygon);
        assert!((area - 5000.0).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_coco_area_polygon() {
        let seg =
            CocoSegmentation::Polygon(vec![vec![0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0]]);
        let area = calculate_coco_area(&seg).unwrap();
        assert!((area - 10000.0).abs() < 1e-6);
    }
}
