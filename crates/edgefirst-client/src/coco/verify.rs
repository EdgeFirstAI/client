// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! COCO dataset verification utilities.
//!
//! Provides functions for comparing COCO datasets and validating annotation
//! accuracy using Hungarian matching for optimal annotation pairing.

use super::{
    decode_compressed_rle, decode_rle,
    types::{CocoAnnotation, CocoDataset, CocoSegmentation},
};
use pathfinding::{kuhn_munkres::kuhn_munkres_min, matrix::Matrix};
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

/// Result of verifying a COCO import against Studio data.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Total images in COCO dataset.
    pub coco_image_count: usize,
    /// Images found in Studio.
    pub studio_image_count: usize,
    /// Images missing from Studio.
    pub missing_images: Vec<String>,
    /// Extra images in Studio not in COCO.
    pub extra_images: Vec<String>,
    /// Total annotations in COCO dataset.
    pub coco_annotation_count: usize,
    /// Total annotations in Studio.
    pub studio_annotation_count: usize,
    /// Bounding box validation results.
    pub bbox_validation: BboxValidationResult,
    /// Segmentation mask validation results.
    pub mask_validation: MaskValidationResult,
    /// Category validation results.
    pub category_validation: CategoryValidationResult,
}

impl VerificationResult {
    /// Returns true if the verification passed all checks.
    pub fn is_valid(&self) -> bool {
        self.missing_images.is_empty()
            && self.extra_images.is_empty()
            && self.bbox_validation.is_valid()
            && self.mask_validation.is_valid()
    }

    /// Returns a summary of the verification.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "Images: {}/{} (missing: {}, extra: {})\n",
            self.studio_image_count,
            self.coco_image_count,
            self.missing_images.len(),
            self.extra_images.len()
        ));
        s.push_str(&format!(
            "Annotations: {}/{}\n",
            self.studio_annotation_count, self.coco_annotation_count
        ));
        s.push_str(&format!(
            "Bbox: {:.1}% matched, {:.4} avg IoU\n",
            self.bbox_validation.match_rate() * 100.0,
            self.bbox_validation.avg_iou()
        ));
        s.push_str(&format!(
            "Masks: {:.1}% preserved, {:.4} avg bbox IoU\n",
            self.mask_validation.preservation_rate() * 100.0,
            self.mask_validation.avg_bbox_iou()
        ));
        s
    }
}

impl fmt::Display for VerificationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "╔══════════════════════════════════════════════════════════════╗"
        )?;
        writeln!(
            f,
            "║                  COCO IMPORT VERIFICATION                    ║"
        )?;
        writeln!(
            f,
            "╠══════════════════════════════════════════════════════════════╣"
        )?;
        writeln!(
            f,
            "║ Images:      {} in COCO, {} in Studio",
            self.coco_image_count, self.studio_image_count
        )?;
        if !self.missing_images.is_empty() {
            writeln!(f, "║   Missing:   {} images", self.missing_images.len())?;
            for name in self.missing_images.iter().take(5) {
                writeln!(f, "║              - {}", name)?;
            }
            if self.missing_images.len() > 5 {
                writeln!(
                    f,
                    "║              ... and {} more",
                    self.missing_images.len() - 5
                )?;
            }
        }
        if !self.extra_images.is_empty() {
            writeln!(f, "║   Extra:     {} images", self.extra_images.len())?;
            for name in self.extra_images.iter().take(5) {
                writeln!(f, "║              - {}", name)?;
            }
            if self.extra_images.len() > 5 {
                writeln!(
                    f,
                    "║              ... and {} more",
                    self.extra_images.len() - 5
                )?;
            }
        }
        writeln!(
            f,
            "║ Annotations: {} in COCO, {} in Studio",
            self.coco_annotation_count, self.studio_annotation_count
        )?;
        writeln!(
            f,
            "╠══════════════════════════════════════════════════════════════╣"
        )?;
        write!(f, "{}", self.bbox_validation)?;
        writeln!(
            f,
            "╠══════════════════════════════════════════════════════════════╣"
        )?;
        write!(f, "{}", self.mask_validation)?;
        writeln!(
            f,
            "╠══════════════════════════════════════════════════════════════╣"
        )?;
        write!(f, "{}", self.category_validation)?;
        writeln!(
            f,
            "╠══════════════════════════════════════════════════════════════╣"
        )?;
        let status = if self.is_valid() {
            "✓ PASSED"
        } else {
            "✗ FAILED"
        };
        writeln!(f, "║ Status: {}", status)?;
        writeln!(
            f,
            "╚══════════════════════════════════════════════════════════════╝"
        )?;
        Ok(())
    }
}

/// Bounding box validation results.
#[derive(Debug, Clone, Default)]
pub struct BboxValidationResult {
    /// Total annotations that were matched using Hungarian algorithm.
    pub total_matched: usize,
    /// Total annotations that could not be matched (IoU too low).
    pub total_unmatched: usize,
    /// Coordinate errors by range: [<1px, <2px, <5px, <10px, >=10px]
    pub errors_by_range: [usize; 5],
    /// Maximum coordinate error in pixels.
    pub max_error: f64,
    /// Sum of IoU values for averaging.
    pub sum_iou: f64,
}

impl BboxValidationResult {
    /// Returns the percentage of coordinates within 1 pixel error.
    pub fn within_1px_rate(&self) -> f64 {
        let total_coords = self.total_matched * 4;
        if total_coords == 0 {
            1.0
        } else {
            self.errors_by_range[0] as f64 / total_coords as f64
        }
    }

    /// Returns the percentage of coordinates within 2 pixels error.
    pub fn within_2px_rate(&self) -> f64 {
        let total_coords = self.total_matched * 4;
        if total_coords == 0 {
            1.0
        } else {
            (self.errors_by_range[0] + self.errors_by_range[1]) as f64 / total_coords as f64
        }
    }

    /// Returns the average IoU across all matched annotations.
    pub fn avg_iou(&self) -> f64 {
        if self.total_matched == 0 {
            1.0
        } else {
            self.sum_iou / self.total_matched as f64
        }
    }

    /// Returns the match rate (matched / total).
    pub fn match_rate(&self) -> f64 {
        let total = self.total_matched + self.total_unmatched;
        if total == 0 {
            1.0
        } else {
            self.total_matched as f64 / total as f64
        }
    }

    /// Returns true if bbox validation passes quality thresholds.
    pub fn is_valid(&self) -> bool {
        self.within_1px_rate() > 0.99 && self.match_rate() > 0.95 && self.avg_iou() > 0.95
    }
}

impl fmt::Display for BboxValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "║ Bounding Box Validation:")?;
        writeln!(
            f,
            "║   Matched:    {}/{} ({:.1}%)",
            self.total_matched,
            self.total_matched + self.total_unmatched,
            self.match_rate() * 100.0
        )?;
        writeln!(f, "║   Avg IoU:    {:.4}", self.avg_iou())?;
        writeln!(f, "║   Within 1px: {:.1}%", self.within_1px_rate() * 100.0)?;
        writeln!(f, "║   Within 2px: {:.1}%", self.within_2px_rate() * 100.0)?;
        writeln!(f, "║   Max error:  {:.2}px", self.max_error)?;
        Ok(())
    }
}

/// Segmentation mask validation results.
#[derive(Debug, Clone, Default)]
pub struct MaskValidationResult {
    /// Annotations with segmentation in original.
    pub original_with_seg: usize,
    /// Annotations with segmentation in restored.
    pub restored_with_seg: usize,
    /// Matched pairs where both have segmentation.
    pub matched_pairs_with_seg: usize,
    /// Polygon pairs (for vertex comparison).
    pub polygon_pairs: usize,
    /// RLE pairs converted to polygon.
    pub rle_pairs: usize,
    /// Pairs where vertex count matches exactly.
    pub vertex_count_exact_match: usize,
    /// Pairs where vertex count is within 10%.
    pub vertex_count_close_match: usize,
    /// Pairs where part count matches.
    pub part_count_match: usize,
    /// Pairs with area within 1%.
    pub area_within_1pct: usize,
    /// Pairs with area within 5%.
    pub area_within_5pct: usize,
    /// Pairs with bbox IoU >= 0.9.
    pub bbox_iou_high: usize,
    /// Pairs with bbox IoU < 0.5.
    pub bbox_iou_low: usize,
    /// Sum of area ratios.
    pub sum_area_ratio: f64,
    /// Minimum area ratio.
    pub min_area_ratio: f64,
    /// Maximum area ratio.
    pub max_area_ratio: f64,
    /// Sum of bbox IoU values.
    pub sum_bbox_iou: f64,
    /// Count of zero-area segmentations.
    pub zero_area_count: usize,
}

impl MaskValidationResult {
    /// Create a new result with initialized min/max values.
    pub fn new() -> Self {
        Self {
            min_area_ratio: f64::MAX,
            max_area_ratio: 0.0,
            ..Default::default()
        }
    }

    /// Returns the segmentation preservation rate.
    pub fn preservation_rate(&self) -> f64 {
        if self.original_with_seg == 0 {
            1.0
        } else {
            self.restored_with_seg as f64 / self.original_with_seg as f64
        }
    }

    /// Returns the average area ratio.
    pub fn avg_area_ratio(&self) -> f64 {
        let valid_count = self
            .matched_pairs_with_seg
            .saturating_sub(self.zero_area_count);
        if valid_count == 0 {
            1.0
        } else {
            self.sum_area_ratio / valid_count as f64
        }
    }

    /// Returns the average bbox IoU.
    pub fn avg_bbox_iou(&self) -> f64 {
        if self.matched_pairs_with_seg == 0 {
            1.0
        } else {
            self.sum_bbox_iou / self.matched_pairs_with_seg as f64
        }
    }

    /// Returns true if mask validation passes quality thresholds.
    pub fn is_valid(&self) -> bool {
        self.preservation_rate() > 0.95 && self.avg_bbox_iou() > 0.90
    }
}

impl fmt::Display for MaskValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "║ Segmentation Mask Validation:")?;
        writeln!(
            f,
            "║   Preserved:   {}/{} ({:.1}%)",
            self.restored_with_seg,
            self.original_with_seg,
            self.preservation_rate() * 100.0
        )?;
        writeln!(
            f,
            "║   Matched:     {} ({} polygon, {} RLE→polygon)",
            self.matched_pairs_with_seg, self.polygon_pairs, self.rle_pairs
        )?;
        writeln!(f, "║   Avg bbox IoU: {:.4}", self.avg_bbox_iou())?;
        writeln!(
            f,
            "║   High IoU (>=0.9): {}/{} ({:.1}%)",
            self.bbox_iou_high,
            self.matched_pairs_with_seg,
            if self.matched_pairs_with_seg > 0 {
                self.bbox_iou_high as f64 / self.matched_pairs_with_seg as f64 * 100.0
            } else {
                100.0
            }
        )?;
        if self.polygon_pairs > 0 {
            writeln!(
                f,
                "║   Vertex exact: {}/{} ({:.1}%)",
                self.vertex_count_exact_match,
                self.polygon_pairs,
                self.vertex_count_exact_match as f64 / self.polygon_pairs as f64 * 100.0
            )?;
        }
        Ok(())
    }
}

/// Category validation results.
#[derive(Debug, Clone, Default)]
pub struct CategoryValidationResult {
    /// Categories in COCO dataset.
    pub coco_categories: HashSet<String>,
    /// Categories in Studio.
    pub studio_categories: HashSet<String>,
    /// Categories missing from Studio.
    pub missing_categories: Vec<String>,
    /// Extra categories in Studio.
    pub extra_categories: Vec<String>,
}

impl CategoryValidationResult {
    /// Returns true if all categories are present.
    pub fn is_valid(&self) -> bool {
        self.missing_categories.is_empty()
    }
}

impl fmt::Display for CategoryValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "║ Categories:    {} in COCO, {} in Studio",
            self.coco_categories.len(),
            self.studio_categories.len()
        )?;
        if !self.missing_categories.is_empty() {
            writeln!(f, "║   Missing:     {:?}", self.missing_categories)?;
        }
        if !self.extra_categories.is_empty() {
            writeln!(f, "║   Extra:       {:?}", self.extra_categories)?;
        }
        Ok(())
    }
}

/// Calculate Intersection over Union (IoU) for two COCO bboxes.
/// COCO bbox format: [x, y, width, height] (top-left corner)
pub fn bbox_iou(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let a_x1 = a[0];
    let a_y1 = a[1];
    let a_x2 = a[0] + a[2];
    let a_y2 = a[1] + a[3];

    let b_x1 = b[0];
    let b_y1 = b[1];
    let b_x2 = b[0] + b[2];
    let b_y2 = b[1] + b[3];

    // Intersection
    let inter_x1 = a_x1.max(b_x1);
    let inter_y1 = a_y1.max(b_y1);
    let inter_x2 = a_x2.min(b_x2);
    let inter_y2 = a_y2.min(b_y2);

    let inter_w = (inter_x2 - inter_x1).max(0.0);
    let inter_h = (inter_y2 - inter_y1).max(0.0);
    let inter_area = inter_w * inter_h;

    // Union
    let a_area = a[2] * a[3];
    let b_area = b[2] * b[3];
    let union_area = a_area + b_area - inter_area;

    if union_area > 0.0 {
        inter_area / union_area
    } else {
        0.0
    }
}

/// Use Hungarian algorithm to find optimal matching between two sets of
/// annotations based on bounding box IoU.
/// Returns pairs of (original_idx, restored_idx) for matched annotations.
pub fn hungarian_match<'a>(
    orig_anns: &[&'a CocoAnnotation],
    rest_anns: &[&'a CocoAnnotation],
) -> Vec<(usize, usize)> {
    if orig_anns.is_empty() || rest_anns.is_empty() {
        return vec![];
    }

    let n = orig_anns.len();
    let m = rest_anns.len();

    // Make the matrix square by padding with high-cost dummy entries
    let size = n.max(m);

    // Build cost matrix: cost = (1 - IoU) * scale
    // We use i64 for kuhn_munkres, scale by 10000 for precision
    let scale = 10000i64;
    let max_cost = scale; // Cost for non-matching (IoU = 0)

    let mut weights = Vec::with_capacity(size * size);
    for i in 0..size {
        for j in 0..size {
            let cost = if i < n && j < m {
                let iou = bbox_iou(&orig_anns[i].bbox, &rest_anns[j].bbox);
                ((1.0 - iou) * scale as f64) as i64
            } else {
                max_cost // Dummy entry
            };
            weights.push(cost);
        }
    }

    let matrix = Matrix::from_vec(size, size, weights).expect("Failed to create matrix");
    let (_, assignments) = kuhn_munkres_min(&matrix);

    // Filter to only real matches (not dummy) with reasonable IoU
    let min_iou_threshold = 0.3; // Only accept matches with IoU > 0.3
    assignments
        .iter()
        .enumerate()
        .filter_map(|(i, &j)| {
            if i < n && j < m {
                let iou = bbox_iou(&orig_anns[i].bbox, &rest_anns[j].bbox);
                if iou >= min_iou_threshold {
                    Some((i, j))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Calculate polygon area using the Shoelace formula.
/// Takes coordinates as flat array [x1, y1, x2, y2, ...]
pub fn polygon_area(coords: &[f64]) -> f64 {
    let n = coords.len() / 2;
    if n < 3 {
        return 0.0;
    }

    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let x_i = coords[i * 2];
        let y_i = coords[i * 2 + 1];
        let x_j = coords[j * 2];
        let y_j = coords[j * 2 + 1];
        area += x_i * y_j - x_j * y_i;
    }
    (area / 2.0).abs()
}

/// Calculate total area of a segmentation.
pub fn compute_segmentation_area(seg: &CocoSegmentation) -> f64 {
    match seg {
        CocoSegmentation::Polygon(polys) => polys.iter().map(|p| polygon_area(p)).sum(),
        CocoSegmentation::Rle(rle) => {
            if let Ok((mask, _, _)) = decode_rle(rle) {
                mask.iter().filter(|&&v| v == 1).count() as f64
            } else {
                0.0
            }
        }
        CocoSegmentation::CompressedRle(compressed) => {
            if let Ok((mask, _, _)) = decode_compressed_rle(compressed) {
                mask.iter().filter(|&&v| v == 1).count() as f64
            } else {
                0.0
            }
        }
    }
}

/// Calculate bounding box of a polygon (min_x, min_y, max_x, max_y)
pub fn polygon_bounds(coords: &[f64]) -> Option<(f64, f64, f64, f64)> {
    if coords.len() < 4 {
        return None;
    }
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for chunk in coords.chunks(2) {
        if chunk.len() == 2 {
            min_x = min_x.min(chunk[0]);
            max_x = max_x.max(chunk[0]);
            min_y = min_y.min(chunk[1]);
            max_y = max_y.max(chunk[1]);
        }
    }
    Some((min_x, min_y, max_x, max_y))
}

/// Get bounding box for any segmentation type.
pub fn segmentation_bounds(seg: &CocoSegmentation) -> Option<(f64, f64, f64, f64)> {
    match seg {
        CocoSegmentation::Polygon(polys) => {
            polys
                .iter()
                .filter_map(|p| polygon_bounds(p))
                .fold(None, |acc, b| match acc {
                    None => Some(b),
                    Some((min_x, min_y, max_x, max_y)) => Some((
                        min_x.min(b.0),
                        min_y.min(b.1),
                        max_x.max(b.2),
                        max_y.max(b.3),
                    )),
                })
        }
        CocoSegmentation::Rle(rle) => {
            let (mask, height, width) = decode_rle(rle).ok()?;
            rle_mask_bounds(&mask, height, width)
        }
        CocoSegmentation::CompressedRle(compressed) => {
            let (mask, height, width) = decode_compressed_rle(compressed).ok()?;
            rle_mask_bounds(&mask, height, width)
        }
    }
}

/// Find bounds of a binary mask.
fn rle_mask_bounds(mask: &[u8], height: u32, width: u32) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found_any = false;

    for y in 0..height {
        for x in 0..width {
            let idx = (y as usize) * (width as usize) + (x as usize);
            if mask.get(idx) == Some(&1) {
                found_any = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }

    if found_any {
        Some((min_x as f64, min_y as f64, max_x as f64, max_y as f64))
    } else {
        None
    }
}

/// Calculate IoU between two segmentation bounding boxes.
pub fn segmentation_bbox_iou(seg1: &CocoSegmentation, seg2: &CocoSegmentation) -> f64 {
    let bounds1 = segmentation_bounds(seg1);
    let bounds2 = segmentation_bounds(seg2);

    match (bounds1, bounds2) {
        (Some((a_x1, a_y1, a_x2, a_y2)), Some((b_x1, b_y1, b_x2, b_y2))) => {
            let inter_x1 = a_x1.max(b_x1);
            let inter_y1 = a_y1.max(b_y1);
            let inter_x2 = a_x2.min(b_x2);
            let inter_y2 = a_y2.min(b_y2);

            let inter_w = (inter_x2 - inter_x1).max(0.0);
            let inter_h = (inter_y2 - inter_y1).max(0.0);
            let inter_area = inter_w * inter_h;

            let a_area = (a_x2 - a_x1) * (a_y2 - a_y1);
            let b_area = (b_x2 - b_x1) * (b_y2 - b_y1);
            let union_area = a_area + b_area - inter_area;

            if union_area > 0.0 {
                inter_area / union_area
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

/// Count total polygon vertices in a segmentation.
pub fn count_polygon_vertices(seg: &CocoSegmentation) -> usize {
    match seg {
        CocoSegmentation::Polygon(polys) => polys.iter().map(|p| p.len() / 2).sum(),
        _ => 0,
    }
}

/// Count number of polygon parts in a segmentation.
pub fn count_polygon_parts(seg: &CocoSegmentation) -> usize {
    match seg {
        CocoSegmentation::Polygon(polys) => polys.len(),
        _ => 0,
    }
}

/// Build a map of annotations by sample name for efficient lookup.
pub fn build_annotation_map_by_name(
    dataset: &CocoDataset,
) -> HashMap<String, Vec<&CocoAnnotation>> {
    let image_names: HashMap<u64, String> = dataset
        .images
        .iter()
        .map(|img| {
            let name = std::path::Path::new(&img.file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&img.file_name)
                .to_string();
            (img.id, name)
        })
        .collect();

    let mut map: HashMap<String, Vec<_>> = HashMap::new();
    for ann in &dataset.annotations {
        if let Some(name) = image_names.get(&ann.image_id) {
            map.entry(name.clone()).or_default().push(ann);
        }
    }
    map
}

/// Validate bounding boxes between two datasets using Hungarian matching.
pub fn validate_bboxes(original: &CocoDataset, restored: &CocoDataset) -> BboxValidationResult {
    let mut result = BboxValidationResult::default();

    let original_by_name = build_annotation_map_by_name(original);
    let restored_by_name = build_annotation_map_by_name(restored);

    for (name, orig_anns) in &original_by_name {
        if let Some(rest_anns) = restored_by_name.get(name) {
            let matches = hungarian_match(orig_anns, rest_anns);

            for (orig_idx, rest_idx) in &matches {
                let orig_ann = orig_anns[*orig_idx];
                let rest_ann = rest_anns[*rest_idx];

                // Track IoU
                let iou = bbox_iou(&orig_ann.bbox, &rest_ann.bbox);
                result.sum_iou += iou;

                // Measure coordinate errors
                for i in 0..4 {
                    let error = (orig_ann.bbox[i] - rest_ann.bbox[i]).abs();
                    result.max_error = result.max_error.max(error);

                    if error < 1.0 {
                        result.errors_by_range[0] += 1;
                    } else if error < 2.0 {
                        result.errors_by_range[1] += 1;
                    } else if error < 5.0 {
                        result.errors_by_range[2] += 1;
                    } else if error < 10.0 {
                        result.errors_by_range[3] += 1;
                    } else {
                        result.errors_by_range[4] += 1;
                    }
                }
                result.total_matched += 1;
            }

            result.total_unmatched += orig_anns.len() - matches.len();
        } else {
            result.total_unmatched += orig_anns.len();
        }
    }

    result
}

/// Validate segmentation masks between two datasets using Hungarian matching.
pub fn validate_masks(original: &CocoDataset, restored: &CocoDataset) -> MaskValidationResult {
    let mut result = MaskValidationResult::new();

    // Count segmentations in original and restored
    result.original_with_seg = original
        .annotations
        .iter()
        .filter(|a| a.segmentation.is_some())
        .count();
    result.restored_with_seg = restored
        .annotations
        .iter()
        .filter(|a| a.segmentation.is_some())
        .count();

    let original_by_name = build_annotation_map_by_name(original);
    let restored_by_name = build_annotation_map_by_name(restored);

    for (name, orig_anns) in &original_by_name {
        if let Some(rest_anns) = restored_by_name.get(name) {
            let matches = hungarian_match(orig_anns, rest_anns);

            for (orig_idx, rest_idx) in &matches {
                let orig_ann = orig_anns[*orig_idx];
                let rest_ann = rest_anns[*rest_idx];

                match (&orig_ann.segmentation, &rest_ann.segmentation) {
                    (Some(orig_seg), Some(rest_seg)) => {
                        result.matched_pairs_with_seg += 1;

                        let is_rle = matches!(
                            orig_seg,
                            CocoSegmentation::Rle(_) | CocoSegmentation::CompressedRle(_)
                        );

                        if is_rle {
                            result.rle_pairs += 1;
                        } else {
                            result.polygon_pairs += 1;

                            let orig_vertices = count_polygon_vertices(orig_seg);
                            let rest_vertices = count_polygon_vertices(rest_seg);
                            let orig_parts = count_polygon_parts(orig_seg);
                            let rest_parts = count_polygon_parts(rest_seg);

                            if orig_vertices == rest_vertices {
                                result.vertex_count_exact_match += 1;
                            }

                            let vertex_diff = (orig_vertices as f64 - rest_vertices as f64).abs();
                            let vertex_threshold = (orig_vertices as f64 * 0.1).max(1.0);
                            if vertex_diff <= vertex_threshold {
                                result.vertex_count_close_match += 1;
                            }

                            if orig_parts == rest_parts {
                                result.part_count_match += 1;
                            }
                        }

                        // Compare area
                        let orig_area = compute_segmentation_area(orig_seg);
                        let rest_area = compute_segmentation_area(rest_seg);

                        if orig_area > 0.0 && rest_area > 0.0 {
                            let area_ratio = rest_area / orig_area;
                            result.sum_area_ratio += area_ratio;
                            result.min_area_ratio = result.min_area_ratio.min(area_ratio);
                            result.max_area_ratio = result.max_area_ratio.max(area_ratio);

                            if (area_ratio - 1.0).abs() <= 0.01 {
                                result.area_within_1pct += 1;
                            }
                            if (area_ratio - 1.0).abs() <= 0.05 {
                                result.area_within_5pct += 1;
                            }
                        } else {
                            result.zero_area_count += 1;
                        }

                        // Compare bounding box IoU
                        let seg_iou = segmentation_bbox_iou(orig_seg, rest_seg);
                        result.sum_bbox_iou += seg_iou;
                        if seg_iou >= 0.9 {
                            result.bbox_iou_high += 1;
                        }
                        if seg_iou < 0.5 {
                            result.bbox_iou_low += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    result
}

/// Validate categories between two datasets.
pub fn validate_categories(
    original: &CocoDataset,
    restored: &CocoDataset,
) -> CategoryValidationResult {
    let coco_cats: HashSet<String> = original.categories.iter().map(|c| c.name.clone()).collect();
    let studio_cats: HashSet<String> = restored.categories.iter().map(|c| c.name.clone()).collect();

    let missing: Vec<String> = coco_cats.difference(&studio_cats).cloned().collect();
    let extra: Vec<String> = studio_cats.difference(&coco_cats).cloned().collect();

    CategoryValidationResult {
        coco_categories: coco_cats,
        studio_categories: studio_cats,
        missing_categories: missing,
        extra_categories: extra,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbox_iou_perfect_overlap() {
        let a = [0.0, 0.0, 100.0, 100.0];
        let b = [0.0, 0.0, 100.0, 100.0];
        assert!((bbox_iou(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_bbox_iou_no_overlap() {
        let a = [0.0, 0.0, 100.0, 100.0];
        let b = [200.0, 200.0, 100.0, 100.0];
        assert!(bbox_iou(&a, &b) < 1e-6);
    }

    #[test]
    fn test_bbox_iou_partial_overlap() {
        let a = [0.0, 0.0, 100.0, 100.0];
        let b = [50.0, 50.0, 100.0, 100.0];
        // Intersection: 50x50 = 2500, Union: 10000 + 10000 - 2500 = 17500
        let expected = 2500.0 / 17500.0;
        assert!((bbox_iou(&a, &b) - expected).abs() < 1e-6);
    }

    #[test]
    fn test_polygon_area_square() {
        // 10x10 square
        let coords = [0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0];
        assert!((polygon_area(&coords) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_polygon_area_triangle() {
        // Triangle with base 10 and height 10
        let coords = [0.0, 0.0, 10.0, 0.0, 5.0, 10.0];
        assert!((polygon_area(&coords) - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_bbox_validation_result_rates() {
        let mut result = BboxValidationResult::default();
        result.total_matched = 100;
        result.total_unmatched = 10;
        result.errors_by_range[0] = 350; // 350/400 = 87.5%
        result.errors_by_range[1] = 40;
        result.sum_iou = 95.0;

        assert!((result.match_rate() - 0.909).abs() < 0.01);
        assert!((result.avg_iou() - 0.95).abs() < 0.01);
    }
}
