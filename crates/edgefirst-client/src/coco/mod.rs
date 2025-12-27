// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! # COCO Dataset Format Support
//!
//! This module provides comprehensive support for the COCO (Common Objects in Context)
//! dataset format, enabling bidirectional conversion between COCO and EdgeFirst formats.
//!
//! ## Supported Workflows
//!
//! 1. **COCO → EdgeFirst Arrow**: Convert COCO JSON/ZIP to Arrow-based EdgeFirst format
//! 2. **EdgeFirst Arrow → COCO**: Convert Arrow format back to COCO JSON
//! 3. **COCO → Studio**: Import COCO directly into EdgeFirst Studio via API
//! 4. **Studio → COCO**: Export Studio dataset to COCO format
//!
//! ## Scope
//!
//! Phase 1 supports:
//! - Bounding boxes (box2d)
//! - Polygon segmentation (mask)
//! - RLE segmentation (decoded to polygons)
//!
//! Not yet supported: keypoints, captions, panoptic segmentation.
//!
//! ## Example
//!
//! ```rust,no_run
//! use edgefirst_client::coco::{CocoReader, coco_to_arrow, CocoToArrowOptions};
//!
//! # async fn example() -> Result<(), edgefirst_client::Error> {
//! // Read COCO annotations
//! let reader = CocoReader::new();
//! let dataset = reader.read_json("annotations/instances_val2017.json")?;
//! println!("Found {} images and {} annotations",
//!          dataset.images.len(), dataset.annotations.len());
//!
//! // Convert to EdgeFirst Arrow format
//! let options = CocoToArrowOptions::default();
//! let count = coco_to_arrow(
//!     "annotations/instances_val2017.json",
//!     "dataset.arrow",
//!     &options,
//!     None,
//! ).await?;
//! println!("Converted {} samples", count);
//! # Ok(())
//! # }
//! ```

mod convert;
mod reader;
mod types;
mod writer;

#[cfg(feature = "polars")]
mod arrow;

#[cfg(feature = "polars")]
pub mod studio;

// Re-export types
pub use types::{
    CocoAnnotation, CocoCategory, CocoCompressedRle, CocoDataset, CocoImage, CocoIndex, CocoInfo,
    CocoLicense, CocoRle, CocoSegmentation,
};

// Re-export readers/writers
pub use reader::{infer_group_from_filename, read_coco_directory, CocoReadOptions, CocoReader};
pub use writer::{CocoWriteOptions, CocoWriter};

// Re-export conversion functions
pub use convert::{
    box2d_to_coco_bbox, calculate_coco_area, coco_bbox_to_box2d, coco_polygon_to_mask,
    coco_rle_to_mask, coco_segmentation_to_mask, decode_compressed_rle, decode_rle,
    mask_to_coco_polygon, mask_to_contours, validate_coco_bbox,
};

// Re-export Arrow conversions (feature-gated)
#[cfg(feature = "polars")]
pub use arrow::{arrow_to_coco, coco_to_arrow, ArrowToCocoOptions, CocoToArrowOptions};

// Re-export Studio integration (feature-gated)
#[cfg(feature = "polars")]
pub use studio::{
    export_studio_to_coco, import_coco_to_studio, CocoExportOptions, CocoImportOptions,
};

#[cfg(test)]
mod tests;
