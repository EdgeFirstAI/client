// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Integration tests for COCO format conversion using real COCO2017 dataset.
//!
//! These tests require the COCO2017 val dataset to be available at:
//! `~/Datasets/COCO/annotations/instances_val2017.json`
//!
//! Run with: `cargo test --features polars --test coco_integration`

#[cfg(feature = "polars")]
mod integration {
    use edgefirst_client::coco::{
        arrow_to_coco, coco_to_arrow, ArrowToCocoOptions, CocoAnnotation, CocoDataset,
        CocoReader, CocoSegmentation, CocoToArrowOptions, CocoWriter,
    };
    use pathfinding::kuhn_munkres::kuhn_munkres_min;
    use pathfinding::matrix::Matrix;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};
    use tempfile::TempDir;

    /// Path to COCO2017 val annotations
    fn coco_val_path() -> Option<PathBuf> {
        let path = dirs::home_dir()?.join("Datasets/COCO/annotations/instances_val2017.json");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    // ==================== Timing Infrastructure ====================

    /// Timing statistics for a named operation
    #[derive(Debug, Clone)]
    struct TimingStats {
        name: String,
        samples: Vec<Duration>,
    }

    impl TimingStats {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                samples: Vec::new(),
            }
        }

        fn add(&mut self, duration: Duration) {
            self.samples.push(duration);
        }

        fn min(&self) -> Duration {
            self.samples.iter().copied().min().unwrap_or(Duration::ZERO)
        }

        fn max(&self) -> Duration {
            self.samples.iter().copied().max().unwrap_or(Duration::ZERO)
        }

        fn avg(&self) -> Duration {
            if self.samples.is_empty() {
                Duration::ZERO
            } else {
                self.samples.iter().sum::<Duration>() / self.samples.len() as u32
            }
        }

        fn total(&self) -> Duration {
            self.samples.iter().sum()
        }

        fn report(&self) {
            if self.samples.len() == 1 {
                println!(
                    "  {}: {:.3}s",
                    self.name,
                    self.samples[0].as_secs_f64()
                );
            } else {
                println!(
                    "  {}: min={:.3}s, max={:.3}s, avg={:.3}s, total={:.3}s ({} samples)",
                    self.name,
                    self.min().as_secs_f64(),
                    self.max().as_secs_f64(),
                    self.avg().as_secs_f64(),
                    self.total().as_secs_f64(),
                    self.samples.len()
                );
            }
        }
    }

    /// Collection of timing stats for the full round-trip test
    #[derive(Debug)]
    struct RoundTripTimings {
        coco_read: TimingStats,
        coco_to_arrow: TimingStats,
        arrow_to_coco: TimingStats,
        restored_read: TimingStats,
        bbox_validation: TimingStats,
        mask_validation: TimingStats,
    }

    impl RoundTripTimings {
        fn new() -> Self {
            Self {
                coco_read: TimingStats::new("COCO Read"),
                coco_to_arrow: TimingStats::new("COCO → Arrow"),
                arrow_to_coco: TimingStats::new("Arrow → COCO"),
                restored_read: TimingStats::new("Restored Read"),
                bbox_validation: TimingStats::new("Bbox Validation"),
                mask_validation: TimingStats::new("Mask Validation"),
            }
        }

        fn total_conversion(&self) -> Duration {
            self.coco_to_arrow.total() + self.arrow_to_coco.total()
        }

        fn total_validation(&self) -> Duration {
            self.bbox_validation.total() + self.mask_validation.total()
        }

        fn total_roundtrip(&self) -> Duration {
            self.coco_read.total()
                + self.coco_to_arrow.total()
                + self.arrow_to_coco.total()
                + self.restored_read.total()
                + self.bbox_validation.total()
                + self.mask_validation.total()
        }

        fn report(&self, annotation_count: usize) {
            println!("\n╔══════════════════════════════════════════════════════════════╗");
            println!("║                     TIMING BREAKDOWN                         ║");
            println!("╠══════════════════════════════════════════════════════════════╣");
            println!("║ Conversion:                                                  ║");
            self.coco_to_arrow.report();
            self.arrow_to_coco.report();
            println!(
                "  → Total conversion: {:.3}s",
                self.total_conversion().as_secs_f64()
            );
            println!("║                                                              ║");
            println!("║ Validation:                                                  ║");
            self.bbox_validation.report();
            self.mask_validation.report();
            println!(
                "  → Total validation: {:.3}s",
                self.total_validation().as_secs_f64()
            );
            println!("║                                                              ║");
            println!("║ I/O:                                                         ║");
            self.coco_read.report();
            self.restored_read.report();
            println!("╠══════════════════════════════════════════════════════════════╣");
            let total = self.total_roundtrip();
            let throughput = annotation_count as f64 / total.as_secs_f64();
            println!(
                "║ TOTAL ROUND-TRIP: {:.3}s ({:.0} annotations/sec)              ║",
                total.as_secs_f64(),
                throughput
            );
            println!("╚══════════════════════════════════════════════════════════════╝");
        }
    }

    // ==================== Matching Algorithms ====================

    /// Calculate Intersection over Union (IoU) for two COCO bboxes.
    /// COCO bbox format: [x, y, width, height] (top-left corner)
    fn bbox_iou(a: &[f64; 4], b: &[f64; 4]) -> f64 {
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

    /// Use Hungarian algorithm to find optimal matching between two sets of annotations.
    /// Returns pairs of (original_idx, restored_idx) for matched annotations.
    fn hungarian_match<'a>(
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

    // ==================== Segmentation Validation ====================

    /// Calculate polygon area using the Shoelace formula.
    /// Takes coordinates as flat array [x1, y1, x2, y2, ...]
    fn polygon_area(coords: &[f64]) -> f64 {
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

    /// Calculate total area of a segmentation using Shoelace formula
    fn compute_segmentation_area(seg: &CocoSegmentation) -> f64 {
        match seg {
            CocoSegmentation::Polygon(polys) => {
                polys.iter().map(|p| polygon_area(p)).sum()
            }
            _ => 0.0,
        }
    }

    /// Calculate bounding box of a polygon (min_x, min_y, max_x, max_y)
    fn polygon_bounds(coords: &[f64]) -> Option<(f64, f64, f64, f64)> {
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

    /// Calculate IoU between two polygon bounding boxes
    fn polygon_bbox_iou(seg1: &CocoSegmentation, seg2: &CocoSegmentation) -> f64 {
        let bounds1 = match seg1 {
            CocoSegmentation::Polygon(polys) => {
                polys.iter().filter_map(|p| polygon_bounds(p)).fold(None, |acc, b| {
                    match acc {
                        None => Some(b),
                        Some((min_x, min_y, max_x, max_y)) => Some((
                            min_x.min(b.0),
                            min_y.min(b.1),
                            max_x.max(b.2),
                            max_y.max(b.3),
                        )),
                    }
                })
            }
            _ => None,
        };

        let bounds2 = match seg2 {
            CocoSegmentation::Polygon(polys) => {
                polys.iter().filter_map(|p| polygon_bounds(p)).fold(None, |acc, b| {
                    match acc {
                        None => Some(b),
                        Some((min_x, min_y, max_x, max_y)) => Some((
                            min_x.min(b.0),
                            min_y.min(b.1),
                            max_x.max(b.2),
                            max_y.max(b.3),
                        )),
                    }
                })
            }
            _ => None,
        };

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

    /// Count total polygon vertices in a segmentation
    fn count_polygon_vertices(seg: &CocoSegmentation) -> usize {
        match seg {
            CocoSegmentation::Polygon(polys) => {
                polys.iter().map(|p| p.len() / 2).sum()
            }
            _ => 0,
        }
    }

    /// Count number of polygon rings/parts in a segmentation
    fn count_polygon_parts(seg: &CocoSegmentation) -> usize {
        match seg {
            CocoSegmentation::Polygon(polys) => polys.len(),
            _ => 0,
        }
    }

    /// Segmentation validation results
    #[derive(Debug, Default)]
    struct MaskValidationResult {
        /// Annotations with segmentation in original
        original_with_seg: usize,
        /// Annotations with segmentation in restored
        restored_with_seg: usize,
        /// Matched pairs with both having segmentation
        matched_pairs_with_seg: usize,
        /// Pairs where vertex count matches exactly
        vertex_count_exact_match: usize,
        /// Pairs where vertex count is within 10%
        vertex_count_close_match: usize,
        /// Pairs where polygon part count matches
        part_count_match: usize,
        /// Pairs where computed area is within 1%
        area_within_1pct: usize,
        /// Pairs where computed area is within 5%
        area_within_5pct: usize,
        /// Pairs where polygon bbox IoU >= 0.9
        bbox_iou_high: usize,
        /// Pairs where polygon bbox IoU < 0.5 (problematic)
        bbox_iou_low: usize,
        /// Sum of area ratios for averaging
        sum_area_ratio: f64,
        /// Minimum area ratio seen
        min_area_ratio: f64,
        /// Maximum area ratio seen
        max_area_ratio: f64,
        /// Sum of polygon bbox IoU for averaging
        sum_bbox_iou: f64,
        /// Count of segmentations with zero area (RLE or degenerate)
        zero_area_count: usize,
    }

    impl MaskValidationResult {
        fn new() -> Self {
            Self {
                min_area_ratio: f64::MAX,
                max_area_ratio: 0.0,
                ..Default::default()
            }
        }

        fn preservation_rate(&self) -> f64 {
            if self.original_with_seg == 0 {
                1.0
            } else {
                self.restored_with_seg as f64 / self.original_with_seg as f64
            }
        }

        fn avg_area_ratio(&self) -> f64 {
            if self.matched_pairs_with_seg == 0 {
                1.0
            } else {
                self.sum_area_ratio / self.matched_pairs_with_seg as f64
            }
        }

        fn avg_bbox_iou(&self) -> f64 {
            if self.matched_pairs_with_seg == 0 {
                1.0
            } else {
                self.sum_bbox_iou / self.matched_pairs_with_seg as f64
            }
        }

        fn report(&self) {
            println!("\n  ┌─────────────────────────────────────────────────────────┐");
            println!("  │              SEGMENTATION MASK VALIDATION                │");
            println!("  ├─────────────────────────────────────────────────────────┤");
            println!(
                "  │ Segmentation presence: {}/{} ({:.1}% preserved)",
                self.restored_with_seg,
                self.original_with_seg,
                self.preservation_rate() * 100.0
            );
            println!(
                "  │ Matched pairs with seg: {}",
                self.matched_pairs_with_seg
            );
            println!("  │                                                         │");
            println!("  │ Polygon Structure:                                      │");
            println!(
                "  │   Vertex count exact: {}/{} ({:.1}%)",
                self.vertex_count_exact_match,
                self.matched_pairs_with_seg,
                if self.matched_pairs_with_seg > 0 {
                    self.vertex_count_exact_match as f64 / self.matched_pairs_with_seg as f64 * 100.0
                } else {
                    100.0
                }
            );
            println!(
                "  │   Vertex count ±10%:  {}/{} ({:.1}%)",
                self.vertex_count_close_match,
                self.matched_pairs_with_seg,
                if self.matched_pairs_with_seg > 0 {
                    self.vertex_count_close_match as f64 / self.matched_pairs_with_seg as f64 * 100.0
                } else {
                    100.0
                }
            );
            println!(
                "  │   Part count match:   {}/{} ({:.1}%)",
                self.part_count_match,
                self.matched_pairs_with_seg,
                if self.matched_pairs_with_seg > 0 {
                    self.part_count_match as f64 / self.matched_pairs_with_seg as f64 * 100.0
                } else {
                    100.0
                }
            );
            println!("  │                                                         │");
            println!("  │ Area Preservation (Shoelace):                           │");
            println!(
                "  │   Area within ±1%:  {}/{} ({:.1}%)",
                self.area_within_1pct,
                self.matched_pairs_with_seg,
                if self.matched_pairs_with_seg > 0 {
                    self.area_within_1pct as f64 / self.matched_pairs_with_seg as f64 * 100.0
                } else {
                    100.0
                }
            );
            println!(
                "  │   Area within ±5%:  {}/{} ({:.1}%)",
                self.area_within_5pct,
                self.matched_pairs_with_seg,
                if self.matched_pairs_with_seg > 0 {
                    self.area_within_5pct as f64 / self.matched_pairs_with_seg as f64 * 100.0
                } else {
                    100.0
                }
            );
            println!(
                "  │   Area ratio: avg={:.4}, min={:.4}, max={:.4}",
                self.avg_area_ratio(),
                if self.min_area_ratio == f64::MAX { 1.0 } else { self.min_area_ratio },
                if self.max_area_ratio == 0.0 { 1.0 } else { self.max_area_ratio }
            );
            println!("  │                                                         │");
            println!("  │ Spatial Accuracy (Polygon Bbox IoU):                    │");
            println!(
                "  │   IoU >= 0.9:  {}/{} ({:.1}%)",
                self.bbox_iou_high,
                self.matched_pairs_with_seg,
                if self.matched_pairs_with_seg > 0 {
                    self.bbox_iou_high as f64 / self.matched_pairs_with_seg as f64 * 100.0
                } else {
                    100.0
                }
            );
            println!(
                "  │   IoU < 0.5:   {}/{} ({:.1}%) [problematic]",
                self.bbox_iou_low,
                self.matched_pairs_with_seg,
                if self.matched_pairs_with_seg > 0 {
                    self.bbox_iou_low as f64 / self.matched_pairs_with_seg as f64 * 100.0
                } else {
                    0.0
                }
            );
            println!(
                "  │   Average IoU: {:.4}",
                self.avg_bbox_iou()
            );
            if self.zero_area_count > 0 {
                println!(
                    "  │   Zero-area (RLE/degenerate): {}",
                    self.zero_area_count
                );
            }
            println!("  └─────────────────────────────────────────────────────────┘");
        }
    }

    /// Validate segmentation masks using Hungarian-matched annotation pairs
    fn validate_segmentation_masks(
        original: &CocoDataset,
        restored: &CocoDataset,
        timings: &mut RoundTripTimings,
    ) -> MaskValidationResult {
        let start = Instant::now();

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

        // Build annotation maps by sample name
        let original_by_name = build_annotation_map_by_name(original);
        let restored_by_name = build_annotation_map_by_name(restored);

        for (name, orig_anns) in &original_by_name {
            if let Some(rest_anns) = restored_by_name.get(name) {
                // Use Hungarian matching to pair annotations
                let matches = hungarian_match(orig_anns, rest_anns);

                for (orig_idx, rest_idx) in &matches {
                    let orig_ann = orig_anns[*orig_idx];
                    let rest_ann = rest_anns[*rest_idx];

                    // Compare segmentations
                    match (&orig_ann.segmentation, &rest_ann.segmentation) {
                        (Some(orig_seg), Some(rest_seg)) => {
                            result.matched_pairs_with_seg += 1;

                            // Compare polygon structure
                            let orig_vertices = count_polygon_vertices(orig_seg);
                            let rest_vertices = count_polygon_vertices(rest_seg);
                            let orig_parts = count_polygon_parts(orig_seg);
                            let rest_parts = count_polygon_parts(rest_seg);

                            if orig_vertices == rest_vertices {
                                result.vertex_count_exact_match += 1;
                            }

                            // Check if within 10%
                            let vertex_diff = (orig_vertices as f64 - rest_vertices as f64).abs();
                            let vertex_threshold = (orig_vertices as f64 * 0.1).max(1.0);
                            if vertex_diff <= vertex_threshold {
                                result.vertex_count_close_match += 1;
                            }

                            if orig_parts == rest_parts {
                                result.part_count_match += 1;
                            }

                            // Compare area using Shoelace formula (more reliable than stored area)
                            let orig_area = compute_segmentation_area(orig_seg);
                            let rest_area = compute_segmentation_area(rest_seg);

                            if orig_area > 0.0 && rest_area > 0.0 {
                                let area_ratio = rest_area / orig_area;
                                result.sum_area_ratio += area_ratio;
                                result.min_area_ratio = result.min_area_ratio.min(area_ratio);
                                result.max_area_ratio = result.max_area_ratio.max(area_ratio);

                                // Within 1%
                                if (area_ratio - 1.0).abs() <= 0.01 {
                                    result.area_within_1pct += 1;
                                }
                                // Within 5%
                                if (area_ratio - 1.0).abs() <= 0.05 {
                                    result.area_within_5pct += 1;
                                }
                            } else {
                                // Track zero-area segmentations (likely RLE or degenerate polygons)
                                result.zero_area_count += 1;
                            }

                            // Compare polygon bounding box IoU
                            let bbox_iou = polygon_bbox_iou(orig_seg, rest_seg);
                            result.sum_bbox_iou += bbox_iou;
                            if bbox_iou >= 0.9 {
                                result.bbox_iou_high += 1;
                            }
                            if bbox_iou < 0.5 {
                                result.bbox_iou_low += 1;
                            }
                        }
                        _ => {
                            // One or both missing segmentation - already counted above
                        }
                    }
                }
            }
        }

        timings.mask_validation.add(start.elapsed());
        result
    }

    // ==================== Bbox Validation ====================

    /// Bounding box validation results
    #[derive(Debug, Default)]
    struct BboxValidationResult {
        total_matched: usize,
        total_unmatched: usize,
        errors_by_range: [usize; 5], // <1, <2, <5, <10, >=10
        max_error: f64,
        sum_iou: f64,
    }

    impl BboxValidationResult {
        fn within_1px_rate(&self) -> f64 {
            let total_coords = self.total_matched * 4;
            if total_coords == 0 {
                0.0
            } else {
                self.errors_by_range[0] as f64 / total_coords as f64 * 100.0
            }
        }

        fn within_2px_rate(&self) -> f64 {
            let total_coords = self.total_matched * 4;
            if total_coords == 0 {
                0.0
            } else {
                (self.errors_by_range[0] + self.errors_by_range[1]) as f64 / total_coords as f64
                    * 100.0
            }
        }

        fn avg_iou(&self) -> f64 {
            if self.total_matched == 0 {
                0.0
            } else {
                self.sum_iou / self.total_matched as f64
            }
        }

        fn match_rate(&self, total: usize) -> f64 {
            if total == 0 {
                0.0
            } else {
                self.total_matched as f64 / total as f64 * 100.0
            }
        }

        fn report(&self, total_annotations: usize) {
            println!("\n  ┌─────────────────────────────────────────────────────────┐");
            println!("  │              BOUNDING BOX VALIDATION                     │");
            println!("  ├─────────────────────────────────────────────────────────┤");
            println!(
                "  │ Hungarian matching: {}/{} ({:.2}%)",
                self.total_matched,
                total_annotations,
                self.match_rate(total_annotations)
            );
            println!("  │ Unmatched: {}", self.total_unmatched);
            println!("  │ Average IoU: {:.4}", self.avg_iou());
            println!(
                "  │ Error distribution: <1px: {}, <2px: {}, <5px: {}, <10px: {}, >=10px: {}",
                self.errors_by_range[0],
                self.errors_by_range[1],
                self.errors_by_range[2],
                self.errors_by_range[3],
                self.errors_by_range[4]
            );
            println!(
                "  │ Accuracy: {:.2}% within 1px, {:.2}% within 2px",
                self.within_1px_rate(),
                self.within_2px_rate()
            );
            println!("  │ Max error: {:.4}px", self.max_error);
            println!("  └─────────────────────────────────────────────────────────┘");
        }
    }

    /// Validate bounding boxes using Hungarian matching
    fn validate_bboxes(
        original: &CocoDataset,
        restored: &CocoDataset,
        timings: &mut RoundTripTimings,
    ) -> BboxValidationResult {
        let start = Instant::now();

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

        timings.bbox_validation.add(start.elapsed());
        result
    }

    // ==================== Main Round-Trip Test ====================

    /// Test complete round-trip: COCO JSON → EdgeFirst Arrow → COCO JSON
    ///
    /// Verifies:
    /// - Annotation count preserved
    /// - Category count preserved
    /// - Category names preserved
    /// - Bounding box coordinates within epsilon
    /// - Polygon segmentation structure preserved
    #[tokio::test]
    async fn test_coco2017_val_roundtrip() {
        let coco_path = match coco_val_path() {
            Some(p) => p,
            None => {
                eprintln!(
                    "Skipping test: COCO2017 val dataset not found at ~/Datasets/COCO/annotations/instances_val2017.json"
                );
                return;
            }
        };

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let arrow_path = temp_dir.path().join("coco_val.arrow");
        let restored_path = temp_dir.path().join("coco_val_restored.json");

        let mut timings = RoundTripTimings::new();

        // Step 1: Read original COCO dataset
        let start = Instant::now();
        let reader = CocoReader::new();
        let original = reader.read_json(&coco_path).expect("Failed to read COCO JSON");
        timings.coco_read.add(start.elapsed());

        println!(
            "\n═══════════════════════════════════════════════════════════════"
        );
        println!(
            "  COCO2017 VAL ROUND-TRIP TEST"
        );
        println!(
            "═══════════════════════════════════════════════════════════════"
        );
        println!(
            "Original dataset: {} images, {} annotations, {} categories",
            original.images.len(),
            original.annotations.len(),
            original.categories.len()
        );

        // Step 2: Convert COCO → Arrow
        let to_arrow_options = CocoToArrowOptions {
            include_masks: true,
            group: Some("val".to_string()),
            ..Default::default()
        };

        let start = Instant::now();
        let arrow_count = coco_to_arrow(&coco_path, &arrow_path, &to_arrow_options, None)
            .await
            .expect("Failed to convert COCO to Arrow");
        timings.coco_to_arrow.add(start.elapsed());

        assert_eq!(
            arrow_count,
            original.annotations.len(),
            "Arrow annotation count mismatch"
        );

        // Step 3: Convert Arrow → COCO
        let to_coco_options = ArrowToCocoOptions {
            include_masks: true,
            ..Default::default()
        };

        let start = Instant::now();
        arrow_to_coco(&arrow_path, &restored_path, &to_coco_options, None)
            .await
            .expect("Failed to convert Arrow to COCO");
        timings.arrow_to_coco.add(start.elapsed());

        // Step 4: Read restored dataset
        let start = Instant::now();
        let restored = reader
            .read_json(&restored_path)
            .expect("Failed to read restored COCO JSON");
        timings.restored_read.add(start.elapsed());

        println!(
            "Restored dataset: {} images, {} annotations, {} categories",
            restored.images.len(),
            restored.annotations.len(),
            restored.categories.len()
        );

        // Step 5: Verify data integrity
        verify_annotation_count(&original, &restored);
        verify_category_preservation(&original, &restored);

        // Bbox validation with timing
        let bbox_result = validate_bboxes(&original, &restored, &mut timings);
        bbox_result.report(original.annotations.len());

        // Assert bbox accuracy
        assert!(
            bbox_result.within_1px_rate() > 99.0,
            "Too few bbox coordinates within 1px: {:.2}% (expected > 99%)",
            bbox_result.within_1px_rate()
        );
        assert!(
            bbox_result.match_rate(original.annotations.len()) > 95.0,
            "Too few annotations matched: {:.2}% (expected > 95%)",
            bbox_result.match_rate(original.annotations.len())
        );

        // Mask validation with timing
        let mask_result = validate_segmentation_masks(&original, &restored, &mut timings);
        mask_result.report();

        // Assert mask preservation
        assert!(
            mask_result.preservation_rate() > 0.95,
            "Segmentation preservation rate too low: {:.1}% (expected > 95%)",
            mask_result.preservation_rate() * 100.0
        );

        // Assert mask quality (spatial accuracy)
        // Note: Some degradation expected due to normalization round-trips
        assert!(
            mask_result.avg_bbox_iou() > 0.90,
            "Polygon bbox IoU too low: {:.4} (expected > 0.90)",
            mask_result.avg_bbox_iou()
        );

        // Most masks should have high quality (IoU >= 0.9)
        let high_iou_rate = mask_result.bbox_iou_high as f64 / mask_result.matched_pairs_with_seg as f64;
        assert!(
            high_iou_rate > 0.85,
            "Too few masks with IoU >= 0.9: {:.1}% (expected > 85%)",
            high_iou_rate * 100.0
        );

        // Print timing report
        timings.report(original.annotations.len());

        // Success criteria
        let total_time = timings.total_roundtrip();
        assert!(
            total_time.as_secs() < 120,
            "Round-trip took too long: {:.2}s (expected < 120s for debug build)",
            total_time.as_secs_f64()
        );
    }

    /// Verify annotation counts match
    fn verify_annotation_count(original: &CocoDataset, restored: &CocoDataset) {
        assert_eq!(
            original.annotations.len(),
            restored.annotations.len(),
            "Annotation count mismatch: original={}, restored={}",
            original.annotations.len(),
            restored.annotations.len()
        );
        println!(
            "✓ Annotation count preserved: {}",
            original.annotations.len()
        );
    }

    /// Verify all category names are preserved
    fn verify_category_preservation(original: &CocoDataset, restored: &CocoDataset) {
        let original_cats: std::collections::HashSet<_> =
            original.categories.iter().map(|c| &c.name).collect();
        let restored_cats: std::collections::HashSet<_> =
            restored.categories.iter().map(|c| &c.name).collect();

        for cat in &original_cats {
            assert!(
                restored_cats.contains(cat),
                "Missing category in restored: {}",
                cat
            );
        }

        println!(
            "✓ Category names preserved: {} categories",
            original_cats.len()
        );
    }

    /// Build a map of annotations by sample name for efficient lookup
    fn build_annotation_map_by_name(dataset: &CocoDataset) -> HashMap<String, Vec<&CocoAnnotation>> {
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

    // ==================== Additional Tests ====================

    /// Test with a subset for faster CI runs
    #[tokio::test]
    async fn test_coco_subset_roundtrip() {
        let coco_path = match coco_val_path() {
            Some(p) => p,
            None => {
                eprintln!("Skipping test: COCO2017 val dataset not found");
                return;
            }
        };

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Read and create a subset with first 100 images
        let reader = CocoReader::new();
        let full_dataset = reader.read_json(&coco_path).expect("Failed to read COCO");

        let subset_images: std::collections::HashSet<_> = full_dataset
            .images
            .iter()
            .take(100)
            .map(|img| img.id)
            .collect();

        let subset = CocoDataset {
            info: full_dataset.info.clone(),
            licenses: full_dataset.licenses.clone(),
            images: full_dataset
                .images
                .iter()
                .filter(|img| subset_images.contains(&img.id))
                .cloned()
                .collect(),
            categories: full_dataset.categories.clone(),
            annotations: full_dataset
                .annotations
                .iter()
                .filter(|ann| subset_images.contains(&ann.image_id))
                .cloned()
                .collect(),
        };

        // Write subset
        let subset_path = temp_dir.path().join("subset.json");
        let writer = CocoWriter::new();
        writer
            .write_json(&subset, &subset_path)
            .expect("Failed to write subset");

        println!(
            "Subset: {} images, {} annotations",
            subset.images.len(),
            subset.annotations.len()
        );

        // Round-trip test
        let arrow_path = temp_dir.path().join("subset.arrow");
        let restored_path = temp_dir.path().join("subset_restored.json");

        let options = CocoToArrowOptions::default();
        let count = coco_to_arrow(&subset_path, &arrow_path, &options, None)
            .await
            .expect("COCO → Arrow failed");

        assert_eq!(count, subset.annotations.len());

        let to_coco_options = ArrowToCocoOptions::default();
        arrow_to_coco(&arrow_path, &restored_path, &to_coco_options, None)
            .await
            .expect("Arrow → COCO failed");

        let restored = reader
            .read_json(&restored_path)
            .expect("Failed to read restored");

        assert_eq!(subset.annotations.len(), restored.annotations.len());
        println!("✓ Subset round-trip successful");
    }

    /// Benchmark: measure throughput with multiple runs
    #[tokio::test]
    async fn test_coco_conversion_performance() {
        let coco_path = match coco_val_path() {
            Some(p) => p,
            None => {
                eprintln!("Skipping test: COCO2017 val dataset not found");
                return;
            }
        };

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let arrow_path = temp_dir.path().join("perf_test.arrow");
        let restored_path = temp_dir.path().join("perf_restored.json");

        // Warm-up read
        let reader = CocoReader::new();
        let dataset = reader.read_json(&coco_path).expect("Failed to read");
        let ann_count = dataset.annotations.len();

        const RUNS: usize = 3;

        // Benchmark COCO → Arrow
        let mut to_arrow_times = TimingStats::new("COCO → Arrow");
        for _ in 0..RUNS {
            let start = Instant::now();
            let options = CocoToArrowOptions::default();
            coco_to_arrow(&coco_path, &arrow_path, &options, None)
                .await
                .expect("Conversion failed");
            to_arrow_times.add(start.elapsed());
        }

        // Benchmark Arrow → COCO
        let mut to_coco_times = TimingStats::new("Arrow → COCO");
        for _ in 0..RUNS {
            let start = Instant::now();
            let options = ArrowToCocoOptions::default();
            arrow_to_coco(&arrow_path, &restored_path, &options, None)
                .await
                .expect("Conversion failed");
            to_coco_times.add(start.elapsed());
        }

        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║               PERFORMANCE BENCHMARK ({} runs)                 ║", RUNS);
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Dataset: {} annotations                                     ║", ann_count);
        to_arrow_times.report();
        to_coco_times.report();

        let avg_roundtrip = to_arrow_times.avg() + to_coco_times.avg();
        let throughput = ann_count as f64 / avg_roundtrip.as_secs_f64();
        println!(
            "║ Round-trip avg: {:.3}s ({:.0} ann/sec)                        ║",
            avg_roundtrip.as_secs_f64(),
            throughput
        );

        // File sizes
        let arrow_size = std::fs::metadata(&arrow_path)
            .expect("Failed to get file size")
            .len();
        let coco_size = std::fs::metadata(&coco_path).unwrap().len();
        println!(
            "║ Arrow file: {:.2} MB (compression: {:.1}x)                   ║",
            arrow_size as f64 / 1024.0 / 1024.0,
            coco_size as f64 / arrow_size as f64
        );
        println!("╚══════════════════════════════════════════════════════════════╝");
    }
}
