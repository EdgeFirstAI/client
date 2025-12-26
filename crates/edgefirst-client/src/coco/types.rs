// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! COCO JSON data structures for serde serialization/deserialization.
//!
//! Supports object detection and instance segmentation annotation types.
//! Keypoints, captions, and panoptic segmentation are NOT supported in this version.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level COCO dataset structure.
///
/// This is the root structure for COCO annotation files like `instances_train2017.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CocoDataset {
    /// Dataset metadata (optional but commonly present).
    #[serde(default)]
    pub info: CocoInfo,
    /// License information for the images.
    #[serde(default)]
    pub licenses: Vec<CocoLicense>,
    /// List of images in the dataset.
    pub images: Vec<CocoImage>,
    /// List of annotations (one per object instance).
    #[serde(default)]
    pub annotations: Vec<CocoAnnotation>,
    /// List of object categories/classes.
    #[serde(default)]
    pub categories: Vec<CocoCategory>,
}

/// Dataset metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CocoInfo {
    /// Year the dataset was created.
    #[serde(default)]
    pub year: Option<u32>,
    /// Version string.
    #[serde(default)]
    pub version: Option<String>,
    /// Dataset description.
    #[serde(default)]
    pub description: Option<String>,
    /// Dataset contributor.
    #[serde(default)]
    pub contributor: Option<String>,
    /// Dataset URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Date the dataset was created.
    #[serde(default)]
    pub date_created: Option<String>,
}

/// License information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoLicense {
    /// Unique license ID.
    pub id: u32,
    /// License name.
    pub name: String,
    /// License URL.
    #[serde(default)]
    pub url: Option<String>,
}

/// Image metadata.
///
/// Each image has a unique ID and associated metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoImage {
    /// Unique image ID.
    pub id: u64,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Filename (relative path within the images folder).
    pub file_name: String,
    /// License ID (references `CocoLicense.id`).
    #[serde(default)]
    pub license: Option<u32>,
    /// Flickr URL (if from Flickr).
    #[serde(default)]
    pub flickr_url: Option<String>,
    /// COCO download URL.
    #[serde(default)]
    pub coco_url: Option<String>,
    /// Date the image was captured.
    #[serde(default)]
    pub date_captured: Option<String>,
}

impl Default for CocoImage {
    fn default() -> Self {
        Self {
            id: 0,
            width: 0,
            height: 0,
            file_name: String::new(),
            license: None,
            flickr_url: None,
            coco_url: None,
            date_captured: None,
        }
    }
}

/// Category definition.
///
/// Categories define the object classes used in the dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoCategory {
    /// Unique category ID.
    pub id: u32,
    /// Category name (e.g., "person", "car").
    pub name: String,
    /// Parent category name (e.g., "human" for "person").
    #[serde(default)]
    pub supercategory: Option<String>,
}

impl Default for CocoCategory {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            supercategory: None,
        }
    }
}

/// Annotation for object detection and instance segmentation.
///
/// Each annotation represents a single object instance in an image.
///
/// Note: Keypoints, captions, and panoptic fields are NOT supported.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CocoAnnotation {
    /// Unique annotation ID.
    pub id: u64,
    /// ID of the image containing this object.
    pub image_id: u64,
    /// Category ID of this object.
    pub category_id: u32,
    /// Bounding box: `[x, y, width, height]` in pixels (top-left corner).
    pub bbox: [f64; 4],
    /// Area of the segmentation mask in pixels².
    #[serde(default)]
    pub area: f64,
    /// Whether this is a crowd annotation (0 = single instance, 1 = crowd).
    #[serde(default)]
    pub iscrowd: u8,
    /// Segmentation mask (polygon or RLE format).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segmentation: Option<CocoSegmentation>,
}

/// Segmentation format: polygon array or RLE.
///
/// COCO supports two segmentation formats:
/// - **Polygon**: For single instances (`iscrowd=0`), uses nested coordinate arrays
/// - **RLE**: For crowds (`iscrowd=1`), uses run-length encoding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CocoSegmentation {
    /// Polygon format: `[[x1,y1,x2,y2,...], [x3,y3,...]]`
    ///
    /// Multiple polygons represent disjoint regions of the same object.
    Polygon(Vec<Vec<f64>>),
    /// Uncompressed RLE format with counts array.
    Rle(CocoRle),
    /// Compressed RLE format with LEB128-encoded counts string.
    CompressedRle(CocoCompressedRle),
}

/// Uncompressed RLE (Run-Length Encoding) segmentation.
///
/// The counts array alternates between background and foreground pixel runs,
/// starting with background. The encoding is **column-major** (Fortran order).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoRle {
    /// Run-length counts: `[bg_run, fg_run, bg_run, fg_run, ...]`
    pub counts: Vec<u32>,
    /// Image size as `[height, width]` (NOT `[width, height]`!)
    pub size: [u32; 2],
}

/// Compressed RLE segmentation (LEB128 encoded).
///
/// Used by pycocotools for more compact storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoCompressedRle {
    /// LEB128-encoded counts string.
    pub counts: String,
    /// Image size as `[height, width]`.
    pub size: [u32; 2],
}

/// Lookup tables for efficient COCO data access.
///
/// Builds indexes from a `CocoDataset` for O(1) lookups.
#[derive(Debug, Clone)]
pub struct CocoIndex {
    /// `image_id` → `CocoImage`
    pub images: HashMap<u64, CocoImage>,
    /// `category_id` → `CocoCategory`
    pub categories: HashMap<u32, CocoCategory>,
    /// `category_id` → `label_index` (0-based, alphabetical order by name)
    pub label_indices: HashMap<u32, u64>,
    /// `image_id` → `Vec<CocoAnnotation>`
    pub annotations_by_image: HashMap<u64, Vec<CocoAnnotation>>,
}

impl CocoIndex {
    /// Build lookup index from a `CocoDataset`.
    ///
    /// Creates efficient lookup tables for accessing images, categories,
    /// and annotations by their IDs.
    pub fn from_dataset(dataset: &CocoDataset) -> Self {
        let images: HashMap<_, _> = dataset
            .images
            .iter()
            .map(|img| (img.id, img.clone()))
            .collect();

        let categories: HashMap<_, _> = dataset
            .categories
            .iter()
            .map(|cat| (cat.id, cat.clone()))
            .collect();

        // Build alphabetically-sorted label indices for consistent ordering
        let mut category_names: Vec<_> = dataset
            .categories
            .iter()
            .map(|c| (c.id, c.name.clone()))
            .collect();
        category_names.sort_by(|a, b| a.1.cmp(&b.1));
        let label_indices: HashMap<_, _> = category_names
            .iter()
            .enumerate()
            .map(|(idx, (cat_id, _))| (*cat_id, idx as u64))
            .collect();

        let mut annotations_by_image: HashMap<u64, Vec<CocoAnnotation>> = HashMap::new();
        for ann in &dataset.annotations {
            annotations_by_image
                .entry(ann.image_id)
                .or_default()
                .push(ann.clone());
        }

        Self {
            images,
            categories,
            label_indices,
            annotations_by_image,
        }
    }

    /// Get the label name for a category ID.
    pub fn label_name(&self, category_id: u32) -> Option<&str> {
        self.categories.get(&category_id).map(|c| c.name.as_str())
    }

    /// Get the label index for a category ID.
    pub fn label_index(&self, category_id: u32) -> Option<u64> {
        self.label_indices.get(&category_id).copied()
    }

    /// Get annotations for an image.
    pub fn annotations_for_image(&self, image_id: u64) -> &[CocoAnnotation] {
        self.annotations_by_image
            .get(&image_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coco_dataset_default() {
        let dataset = CocoDataset::default();
        assert!(dataset.images.is_empty());
        assert!(dataset.annotations.is_empty());
        assert!(dataset.categories.is_empty());
    }

    #[test]
    fn test_coco_index_from_dataset() {
        let dataset = CocoDataset {
            images: vec![
                CocoImage {
                    id: 1,
                    width: 640,
                    height: 480,
                    file_name: "image1.jpg".to_string(),
                    ..Default::default()
                },
                CocoImage {
                    id: 2,
                    width: 800,
                    height: 600,
                    file_name: "image2.jpg".to_string(),
                    ..Default::default()
                },
            ],
            categories: vec![
                CocoCategory {
                    id: 1,
                    name: "person".to_string(),
                    supercategory: Some("human".to_string()),
                },
                CocoCategory {
                    id: 2,
                    name: "car".to_string(),
                    supercategory: Some("vehicle".to_string()),
                },
            ],
            annotations: vec![
                CocoAnnotation {
                    id: 100,
                    image_id: 1,
                    category_id: 1,
                    bbox: [10.0, 20.0, 100.0, 200.0],
                    area: 20000.0,
                    iscrowd: 0,
                    segmentation: None,
                },
                CocoAnnotation {
                    id: 101,
                    image_id: 1,
                    category_id: 2,
                    bbox: [50.0, 60.0, 150.0, 100.0],
                    area: 15000.0,
                    iscrowd: 0,
                    segmentation: None,
                },
            ],
            ..Default::default()
        };

        let index = CocoIndex::from_dataset(&dataset);

        // Check images lookup
        assert_eq!(index.images.len(), 2);
        assert_eq!(index.images.get(&1).unwrap().file_name, "image1.jpg");

        // Check categories lookup
        assert_eq!(index.categories.len(), 2);
        assert_eq!(index.label_name(1), Some("person"));
        assert_eq!(index.label_name(2), Some("car"));

        // Check alphabetical label indices (car=0, person=1)
        assert_eq!(index.label_index(2), Some(0)); // car
        assert_eq!(index.label_index(1), Some(1)); // person

        // Check annotations by image
        let anns = index.annotations_for_image(1);
        assert_eq!(anns.len(), 2);

        let anns = index.annotations_for_image(2);
        assert!(anns.is_empty());
    }

    #[test]
    fn test_coco_segmentation_polygon_deserialize() {
        let json = r#"[[100.0, 200.0, 150.0, 250.0, 100.0, 250.0]]"#;
        let seg: CocoSegmentation = serde_json::from_str(json).unwrap();

        match seg {
            CocoSegmentation::Polygon(polys) => {
                assert_eq!(polys.len(), 1);
                assert_eq!(polys[0].len(), 6);
            }
            _ => panic!("Expected polygon segmentation"),
        }
    }

    #[test]
    fn test_coco_segmentation_rle_deserialize() {
        let json = r#"{"counts": [10, 20, 30, 40], "size": [100, 200]}"#;
        let seg: CocoSegmentation = serde_json::from_str(json).unwrap();

        match seg {
            CocoSegmentation::Rle(rle) => {
                assert_eq!(rle.counts, vec![10, 20, 30, 40]);
                assert_eq!(rle.size, [100, 200]);
            }
            _ => panic!("Expected RLE segmentation"),
        }
    }

    #[test]
    fn test_coco_annotation_roundtrip() {
        let ann = CocoAnnotation {
            id: 12345,
            image_id: 67890,
            category_id: 1,
            bbox: [100.5, 200.5, 50.0, 80.0],
            area: 4000.0,
            iscrowd: 0,
            segmentation: Some(CocoSegmentation::Polygon(vec![vec![
                100.0, 200.0, 150.0, 200.0, 150.0, 280.0, 100.0, 280.0,
            ]])),
        };

        let json = serde_json::to_string(&ann).unwrap();
        let restored: CocoAnnotation = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, ann.id);
        assert_eq!(restored.image_id, ann.image_id);
        assert_eq!(restored.category_id, ann.category_id);
        assert_eq!(restored.bbox, ann.bbox);
    }
}
