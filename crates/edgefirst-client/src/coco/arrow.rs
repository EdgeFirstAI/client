// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! COCO to EdgeFirst Arrow format conversion.
//!
//! Provides high-performance conversion between COCO JSON and EdgeFirst Arrow
//! format, supporting async operations and progress tracking.

use super::{
    convert::{
        box2d_to_coco_bbox, coco_bbox_to_box2d, coco_segmentation_to_mask, mask_to_coco_polygon,
    },
    reader::CocoReader,
    types::{CocoImage, CocoIndex, CocoInfo, CocoSegmentation},
    writer::{CocoDatasetBuilder, CocoWriter},
};
use crate::{Annotation, Box2d, Error, Mask, Progress, Sample};
use polars::prelude::*;
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::{Semaphore, mpsc::Sender};

/// Unflatten polygon coordinates from Arrow flat format.
///
/// Converts `[x1, y1, x2, y2, NaN, x3, y3, ...]` to `[[(x1,y1), (x2,y2)],
/// [(x3,y3), ...]]`
///
/// **IMPORTANT**: The separator is a SINGLE NaN, not a pair. We must process
/// elements one at a time, not in chunks of 2, to correctly handle the
/// separator.
fn unflatten_polygon_coords(coords: &[f32]) -> Vec<Vec<(f32, f32)>> {
    let mut polygons = Vec::new();
    let mut current = Vec::new();
    let mut i = 0;

    while i < coords.len() {
        if coords[i].is_nan() {
            // Single NaN separator - save current polygon and start new one
            if !current.is_empty() {
                polygons.push(std::mem::take(&mut current));
            }
            i += 1;
        } else if i + 1 < coords.len() && !coords[i + 1].is_nan() {
            // Have both x and y coordinates (neither is NaN)
            current.push((coords[i], coords[i + 1]));
            i += 2;
        } else if i + 1 < coords.len() && coords[i + 1].is_nan() {
            // x is valid but y is NaN - this shouldn't happen in well-formed data
            // but handle it gracefully: skip x, process NaN on next iteration
            i += 1;
        } else {
            // Odd trailing value - skip
            i += 1;
        }
    }

    if !current.is_empty() {
        polygons.push(current);
    }

    polygons
}

/// Options for COCO to Arrow conversion.
#[derive(Debug, Clone)]
pub struct CocoToArrowOptions {
    /// Include segmentation masks in output.
    pub include_masks: bool,
    /// Group name for all samples (e.g., "train", "val").
    pub group: Option<String>,
    /// Maximum number of parallel workers.
    pub max_workers: usize,
}

impl Default for CocoToArrowOptions {
    fn default() -> Self {
        Self {
            include_masks: true,
            group: None,
            max_workers: max_workers(),
        }
    }
}

/// Options for Arrow to COCO conversion.
#[derive(Debug, Clone)]
pub struct ArrowToCocoOptions {
    /// Filter by group names (empty = all).
    pub groups: Vec<String>,
    /// Include segmentation masks in output.
    pub include_masks: bool,
    /// COCO info section.
    pub info: Option<CocoInfo>,
}

impl Default for ArrowToCocoOptions {
    fn default() -> Self {
        Self {
            groups: vec![],
            include_masks: true,
            info: None,
        }
    }
}

/// Determine maximum number of parallel workers.
fn max_workers() -> usize {
    std::env::var("MAX_COCO_WORKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            let cpus = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            (cpus / 2).clamp(2, 8)
        })
}

/// Convert COCO annotations to EdgeFirst Arrow format.
///
/// This is a high-performance async conversion that uses parallel workers
/// for parsing and transforming annotations.
///
/// # Arguments
/// * `coco_path` - Path to COCO annotation JSON file or ZIP archive
/// * `output_path` - Output Arrow file path
/// * `options` - Conversion options
/// * `progress` - Optional progress channel
///
/// # Returns
/// Number of samples converted
pub async fn coco_to_arrow<P: AsRef<Path>>(
    coco_path: P,
    output_path: P,
    options: &CocoToArrowOptions,
    progress: Option<Sender<Progress>>,
) -> Result<usize, Error> {
    let coco_path = coco_path.as_ref();
    let output_path = output_path.as_ref();

    // Read COCO dataset
    let reader = CocoReader::new();
    let dataset = if coco_path.extension().is_some_and(|e| e == "zip") {
        reader.read_annotations_zip(coco_path)?
    } else {
        reader.read_json(coco_path)?
    };

    // Build index for efficient lookups
    let index = Arc::new(CocoIndex::from_dataset(&dataset));
    let total_images = dataset.images.len();

    // Send initial progress
    if let Some(ref p) = progress {
        let _ = p
            .send(Progress {
                current: 0,
                total: total_images,
                status: None,
            })
            .await;
    }

    // Process images in parallel
    let sem = Arc::new(Semaphore::new(options.max_workers));
    let current = Arc::new(AtomicUsize::new(0));
    let include_masks = options.include_masks;
    let group = options.group.clone();

    let mut tasks = Vec::with_capacity(total_images);

    for image in dataset.images {
        let sem = sem.clone();
        let index = index.clone();
        let current = current.clone();
        let progress = progress.clone();
        let total = total_images;
        let group = group.clone();

        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.map_err(Error::SemaphoreError)?;

            // Convert this image's annotations to EdgeFirst samples
            let samples =
                convert_image_annotations(&image, &index, include_masks, group.as_deref());

            // Update progress
            let c = current.fetch_add(1, Ordering::SeqCst) + 1;
            if let Some(ref p) = progress {
                let _ = p
                    .send(Progress {
                        current: c,
                        total,
                        status: None,
                    })
                    .await;
            }

            Ok::<Vec<Sample>, Error>(samples)
        });

        tasks.push(task);
    }

    // Collect all samples
    let mut all_samples = Vec::with_capacity(total_images);
    for task in tasks {
        let samples = task.await??;
        all_samples.extend(samples);
    }

    // Convert to DataFrame
    let df = crate::samples_dataframe(&all_samples)?;

    // Write Arrow file
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(output_path)?;
    IpcWriter::new(&mut file).finish(&mut df.clone())?;

    Ok(all_samples.len())
}

/// Convert a single image's annotations to EdgeFirst samples.
fn convert_image_annotations(
    image: &CocoImage,
    index: &CocoIndex,
    include_masks: bool,
    group: Option<&str>,
) -> Vec<Sample> {
    let annotations = index.annotations_for_image(image.id);
    let sample_name = sample_name_from_filename(&image.file_name);

    annotations
        .iter()
        .filter_map(|ann| {
            let label = index.label_name(ann.category_id)?;
            let label_index = index.label_index(ann.category_id);

            // Convert bbox
            let box2d = coco_bbox_to_box2d(&ann.bbox, image.width, image.height);

            // Convert mask if present and requested
            let mask = if include_masks {
                ann.segmentation
                    .as_ref()
                    .and_then(|seg| coco_segmentation_to_mask(seg, image.width, image.height).ok())
            } else {
                None
            };

            let mut annotation = Annotation::new();
            annotation.set_name(Some(sample_name.clone()));
            annotation.set_label(Some(label.to_string()));
            annotation.set_label_index(label_index);
            annotation.set_box2d(Some(box2d));
            annotation.set_mask(mask);
            annotation.set_group(group.map(String::from));

            let sample = Sample {
                image_name: Some(sample_name.clone()),
                width: Some(image.width),
                height: Some(image.height),
                group: group.map(String::from),
                annotations: vec![annotation],
                ..Default::default()
            };

            Some(sample)
        })
        .collect()
}

/// Extract sample name from image filename.
fn sample_name_from_filename(filename: &str) -> String {
    Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(String::from)
        .unwrap_or_else(|| filename.to_string())
}

/// Convert EdgeFirst Arrow format to COCO annotations.
///
/// Reads an Arrow file and produces COCO JSON output.
///
/// # Arguments
/// * `arrow_path` - Path to EdgeFirst Arrow file
/// * `output_path` - Output COCO JSON file path
/// * `options` - Conversion options
/// * `progress` - Optional progress channel
///
/// # Returns
/// Number of annotations converted
pub async fn arrow_to_coco<P: AsRef<Path>>(
    arrow_path: P,
    output_path: P,
    options: &ArrowToCocoOptions,
    progress: Option<Sender<Progress>>,
) -> Result<usize, Error> {
    let arrow_path = arrow_path.as_ref();
    let output_path = output_path.as_ref();

    // Read Arrow file
    let mut file = std::fs::File::open(arrow_path)?;
    let df = IpcReader::new(&mut file).finish()?;

    // Get group column for filtering
    let groups_to_filter: std::collections::HashSet<_> = options.groups.iter().cloned().collect();

    let total_rows = df.height();

    if let Some(ref p) = progress {
        let _ = p
            .send(Progress {
                current: 0,
                total: total_rows,
                status: None,
            })
            .await;
    }

    // Extract columns - all at once for O(n) instead of O(n²) per-row access
    let names: Vec<String> = df
        .column("name")?
        .str()?
        .into_iter()
        .map(|s| s.unwrap_or_default().to_string())
        .collect();

    let labels: Vec<String> = df
        .column("label")?
        .cast(&DataType::String)?
        .str()?
        .into_iter()
        .map(|s| s.unwrap_or_default().to_string())
        .collect();

    // Get group column for filtering
    let groups: Vec<String> = df
        .column("group")
        .ok()
        .and_then(|c| c.cast(&DataType::String).ok())
        .map(|c| {
            c.str()
                .ok()
                .map(|s| {
                    s.into_iter()
                        .map(|v| v.unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default()
        })
        .unwrap_or_else(|| vec!["".to_string(); total_rows]);

    // Extract all box2d values upfront (O(n) instead of O(n²))
    let box2ds = extract_all_box2ds(df.column("box2d")?)?;

    // Extract all masks upfront if present
    let masks = if options.include_masks {
        df.column("mask").ok().map(extract_all_masks).transpose()?
    } else {
        None
    };

    // Extract all sizes upfront if present
    let sizes = df
        .column("size")
        .ok()
        .and_then(|c| extract_all_sizes(c).ok());

    // Build COCO dataset
    let mut builder = CocoDatasetBuilder::new();

    if let Some(info) = &options.info {
        builder = builder.info(info.clone());
    }

    // Track unique images and categories
    let mut image_dimensions: HashMap<String, (u32, u32)> = HashMap::new();
    let mut image_ids: HashMap<String, u64> = HashMap::new();
    let mut category_ids: HashMap<String, u32> = HashMap::new();

    // First pass: collect unique images and categories
    for i in 0..total_rows {
        // Skip if group filtering is active and this row doesn't match
        if !groups_to_filter.is_empty() && !groups_to_filter.contains(&groups[i]) {
            continue;
        }

        let name = &names[i];
        let label = &labels[i];

        // Get or estimate image dimensions
        if !image_ids.contains_key(name) {
            let (width, height) = sizes
                .as_ref()
                .and_then(|s| s.get(i).copied())
                .unwrap_or((0, 0));

            let id = builder.add_image(name, width, height);
            image_ids.insert(name.clone(), id);
            image_dimensions.insert(name.clone(), (width, height));
        }

        if !label.is_empty() && !category_ids.contains_key(label) {
            let id = builder.add_category(label, None);
            category_ids.insert(label.clone(), id);
        }
    }

    // Second pass: create annotations
    let mut last_progress_update = 0;
    for i in 0..total_rows {
        // Skip if group filtering is active and this row doesn't match
        if !groups_to_filter.is_empty() && !groups_to_filter.contains(&groups[i]) {
            continue;
        }

        let name = &names[i];
        let label = &labels[i];

        let image_id = *image_ids.get(name).unwrap_or(&0);
        let category_id = *category_ids.get(label).unwrap_or(&0);
        let (width, height) = *image_dimensions.get(name).unwrap_or(&(1, 1));

        // Convert box2d from Arrow center-normalized [cx, cy, w, h] to COCO format
        // Arrow stores center-point, Box2d expects top-left
        let bbox = box2ds.get(i).map(|box2d| {
            let cx = box2d[0];
            let cy = box2d[1];
            let w = box2d[2];
            let h = box2d[3];
            // Convert from center-point to top-left format
            let left = cx - w / 2.0;
            let top = cy - h / 2.0;
            let ef_box2d = Box2d::new(left, top, w, h);
            box2d_to_coco_bbox(&ef_box2d, width, height)
        });

        // Convert mask if present
        let segmentation = if options.include_masks {
            masks.as_ref().and_then(|m| {
                m.get(i).and_then(|coords| {
                    if coords.is_empty() {
                        None
                    } else {
                        let polygons = unflatten_polygon_coords(coords);
                        let mask = Mask::new(polygons);
                        let coco_poly = mask_to_coco_polygon(&mask, width, height);
                        if coco_poly.is_empty() {
                            None
                        } else {
                            Some(CocoSegmentation::Polygon(coco_poly))
                        }
                    }
                })
            })
        } else {
            None
        };

        if let Some(bbox) = bbox {
            builder.add_annotation(image_id, category_id, bbox, segmentation);
        }

        // Update progress every 1000 rows to reduce overhead
        if let Some(ref p) = progress
            && (i - last_progress_update >= 1000 || i == total_rows - 1)
        {
            let _ = p
                .send(Progress {
                    current: i + 1,
                    total: total_rows,
                    status: None,
                })
                .await;
            last_progress_update = i;
        }
    }

    let dataset = builder.build();
    let annotation_count = dataset.annotations.len();

    // Write output
    let writer = CocoWriter::new();
    writer.write_json(&dataset, output_path)?;

    Ok(annotation_count)
}

/// Extract all box2d values from a column at once (O(n) instead of O(n²)).
fn extract_all_box2ds(col: &Column) -> Result<Vec<[f32; 4]>, Error> {
    let arr = col.array()?;
    let mut result = Vec::with_capacity(arr.len());

    for inner in arr.amortized_iter() {
        let values = if let Some(inner) = inner {
            let series = inner.as_ref();
            let vals: Vec<f32> = series
                .f32()
                .map_err(|e| Error::CocoError(format!("box2d cast error: {}", e)))?
                .into_iter()
                .map(|v| v.unwrap_or(0.0))
                .collect();

            if vals.len() == 4 {
                [vals[0], vals[1], vals[2], vals[3]]
            } else {
                [0.0, 0.0, 0.0, 0.0]
            }
        } else {
            [0.0, 0.0, 0.0, 0.0]
        };
        result.push(values);
    }

    Ok(result)
}

/// Extract all mask coordinates from a column at once (O(n) instead of O(n²)).
fn extract_all_masks(col: &Column) -> Result<Vec<Vec<f32>>, Error> {
    let list = col.list()?;
    let mut result = Vec::with_capacity(list.len());

    for i in 0..list.len() {
        let coords = match list.get_as_series(i) {
            Some(series) => series
                .f32()
                .map_err(|e| Error::CocoError(format!("mask cast error: {}", e)))?
                .into_iter()
                .map(|v| v.unwrap_or(f32::NAN))
                .collect(),
            None => vec![],
        };
        result.push(coords);
    }

    Ok(result)
}

/// Extract all image sizes from a column at once.
fn extract_all_sizes(col: &Column) -> Result<Vec<(u32, u32)>, Error> {
    let arr = col.array()?;
    let mut result = Vec::with_capacity(arr.len());

    for inner in arr.amortized_iter() {
        let size = if let Some(inner) = inner {
            let series = inner.as_ref();
            let values: Vec<u32> = series
                .u32()
                .map_err(|e| Error::CocoError(format!("size cast error: {}", e)))?
                .into_iter()
                .map(|v| v.unwrap_or(0))
                .collect();

            if values.len() >= 2 {
                (values[0], values[1])
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };
        result.push(size);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coco::{CocoAnnotation, CocoCategory, CocoDataset};
    use tempfile::TempDir;

    // =========================================================================
    // unflatten_polygon_coords tests
    // =========================================================================

    #[test]
    fn test_unflatten_polygon_coords_empty() {
        let coords: Vec<f32> = vec![];
        let result = unflatten_polygon_coords(&coords);
        assert!(result.is_empty());
    }

    #[test]
    fn test_unflatten_polygon_coords_single_polygon() {
        // Simple rectangle: 4 points
        let coords = vec![0.1, 0.2, 0.3, 0.2, 0.3, 0.4, 0.1, 0.4];
        let result = unflatten_polygon_coords(&coords);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 4);
        assert_eq!(result[0][0], (0.1, 0.2));
        assert_eq!(result[0][3], (0.1, 0.4));
    }

    #[test]
    fn test_unflatten_polygon_coords_multiple_polygons() {
        // Two triangles separated by NaN
        let coords = vec![
            0.1,
            0.1,
            0.2,
            0.1,
            0.15,
            0.2,      // First triangle
            f32::NAN, // Separator
            0.5,
            0.5,
            0.6,
            0.5,
            0.55,
            0.6, // Second triangle
        ];
        let result = unflatten_polygon_coords(&coords);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 3);
        assert_eq!(result[1].len(), 3);
        assert_eq!(result[0][0], (0.1, 0.1));
        assert_eq!(result[1][0], (0.5, 0.5));
    }

    #[test]
    fn test_unflatten_polygon_coords_leading_nan() {
        // NaN at the start should be handled gracefully
        let coords = vec![f32::NAN, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        let result = unflatten_polygon_coords(&coords);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 3);
    }

    #[test]
    fn test_unflatten_polygon_coords_trailing_nan() {
        // NaN at the end
        let coords = vec![0.1, 0.2, 0.3, 0.4, f32::NAN];
        let result = unflatten_polygon_coords(&coords);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 2);
    }

    #[test]
    fn test_unflatten_polygon_coords_consecutive_nans() {
        // Multiple NaNs in a row
        let coords = vec![0.1, 0.2, f32::NAN, f32::NAN, 0.3, 0.4];
        let result = unflatten_polygon_coords(&coords);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[1].len(), 1);
    }

    #[test]
    fn test_unflatten_polygon_coords_odd_values() {
        // Odd number of coordinates (trailing x without y)
        let coords = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let result = unflatten_polygon_coords(&coords);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 2); // Only complete pairs
    }

    // =========================================================================
    // convert_image_annotations tests
    // =========================================================================

    #[test]
    fn test_convert_image_annotations_basic() {
        let image = CocoImage {
            id: 1,
            width: 640,
            height: 480,
            file_name: "test_image.jpg".to_string(),
            ..Default::default()
        };

        let dataset = CocoDataset {
            images: vec![image.clone()],
            categories: vec![CocoCategory {
                id: 1,
                name: "cat".to_string(),
                supercategory: Some("animal".to_string()),
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [100.0, 100.0, 200.0, 200.0],
                area: 40000.0,
                iscrowd: 0,
                segmentation: None,
            }],
            ..Default::default()
        };

        let index = CocoIndex::from_dataset(&dataset);
        let samples = convert_image_annotations(&image, &index, true, Some("train"));

        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].image_name, Some("test_image".to_string()));
        assert_eq!(samples[0].group, Some("train".to_string()));
        assert_eq!(samples[0].annotations.len(), 1);
        assert_eq!(samples[0].annotations[0].label(), Some(&"cat".to_string()));
    }

    #[test]
    fn test_convert_image_annotations_with_mask() {
        let image = CocoImage {
            id: 1,
            width: 100,
            height: 100,
            file_name: "masked.jpg".to_string(),
            ..Default::default()
        };

        let dataset = CocoDataset {
            images: vec![image.clone()],
            categories: vec![CocoCategory {
                id: 1,
                name: "object".to_string(),
                supercategory: None,
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [10.0, 10.0, 50.0, 50.0],
                area: 2500.0,
                iscrowd: 0,
                segmentation: Some(CocoSegmentation::Polygon(vec![vec![
                    10.0, 10.0, 60.0, 10.0, 60.0, 60.0, 10.0, 60.0,
                ]])),
            }],
            ..Default::default()
        };

        let index = CocoIndex::from_dataset(&dataset);

        // With masks enabled
        let samples_with_mask = convert_image_annotations(&image, &index, true, None);
        assert!(samples_with_mask[0].annotations[0].mask().is_some());

        // With masks disabled
        let samples_no_mask = convert_image_annotations(&image, &index, false, None);
        assert!(samples_no_mask[0].annotations[0].mask().is_none());
    }

    #[test]
    fn test_convert_image_annotations_no_annotations() {
        let image = CocoImage {
            id: 1,
            width: 640,
            height: 480,
            file_name: "empty.jpg".to_string(),
            ..Default::default()
        };

        let dataset = CocoDataset {
            images: vec![image.clone()],
            categories: vec![],
            annotations: vec![],
            ..Default::default()
        };

        let index = CocoIndex::from_dataset(&dataset);
        let samples = convert_image_annotations(&image, &index, true, None);

        assert!(samples.is_empty());
    }

    // =========================================================================
    // sample_name_from_filename tests
    // =========================================================================

    #[test]
    fn test_sample_name_from_filename() {
        assert_eq!(
            sample_name_from_filename("000000397133.jpg"),
            "000000397133"
        );
        assert_eq!(sample_name_from_filename("train2017/image.jpg"), "image");
        assert_eq!(sample_name_from_filename("test"), "test");
    }

    #[test]
    fn test_sample_name_from_filename_nested_path() {
        assert_eq!(
            sample_name_from_filename("a/b/c/deep_image.png"),
            "deep_image"
        );
    }

    #[test]
    fn test_sample_name_from_filename_no_extension() {
        assert_eq!(sample_name_from_filename("no_extension"), "no_extension");
    }

    // =========================================================================
    // Options tests
    // =========================================================================

    #[test]
    fn test_coco_to_arrow_options_default() {
        let options = CocoToArrowOptions::default();
        assert!(options.include_masks);
        assert!(options.group.is_none());
        assert!(options.max_workers >= 2);
    }

    #[test]
    fn test_arrow_to_coco_options_default() {
        let options = ArrowToCocoOptions::default();
        assert!(options.groups.is_empty());
        assert!(options.include_masks);
        assert!(options.info.is_none());
    }

    #[test]
    fn test_max_workers() {
        let workers = max_workers();
        assert!(workers >= 2);
        assert!(workers <= 8);
    }

    #[tokio::test]
    async fn test_coco_to_arrow_minimal() {
        let temp_dir = TempDir::new().unwrap();

        // Create minimal COCO JSON
        let coco_json = r#"{
            "images": [
                {"id": 1, "width": 640, "height": 480, "file_name": "test.jpg"}
            ],
            "annotations": [
                {"id": 1, "image_id": 1, "category_id": 1, "bbox": [10, 20, 100, 80], "area": 8000, "iscrowd": 0}
            ],
            "categories": [
                {"id": 1, "name": "person", "supercategory": "human"}
            ]
        }"#;

        let coco_path = temp_dir.path().join("test.json");
        std::fs::write(&coco_path, coco_json).unwrap();

        let arrow_path = temp_dir.path().join("output.arrow");

        let options = CocoToArrowOptions::default();
        let count = coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();

        assert_eq!(count, 1);
        assert!(arrow_path.exists());

        // Verify Arrow contents
        let mut file = std::fs::File::open(&arrow_path).unwrap();
        let df = IpcReader::new(&mut file).finish().unwrap();
        assert_eq!(df.height(), 1);
    }

    #[tokio::test]
    async fn test_arrow_to_coco_roundtrip() {
        let temp_dir = TempDir::new().unwrap();

        // Create COCO JSON
        let original = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 640,
                height: 480,
                file_name: "test.jpg".to_string(),
                ..Default::default()
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [100.0, 50.0, 200.0, 150.0],
                area: 30000.0,
                iscrowd: 0,
                segmentation: Some(CocoSegmentation::Polygon(vec![vec![
                    100.0, 50.0, 300.0, 50.0, 300.0, 200.0, 100.0, 200.0,
                ]])),
            }],
            categories: vec![CocoCategory {
                id: 1,
                name: "person".to_string(),
                supercategory: Some("human".to_string()),
            }],
            ..Default::default()
        };

        // Write original COCO
        let coco_path = temp_dir.path().join("original.json");
        let writer = CocoWriter::new();
        writer.write_json(&original, &coco_path).unwrap();

        // Convert to Arrow
        let arrow_path = temp_dir.path().join("converted.arrow");
        let options = CocoToArrowOptions::default();
        coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();

        // Convert back to COCO
        let restored_path = temp_dir.path().join("restored.json");
        let options = ArrowToCocoOptions::default();
        arrow_to_coco(&arrow_path, &restored_path, &options, None)
            .await
            .unwrap();

        // Verify restored data
        let reader = CocoReader::new();
        let restored = reader.read_json(&restored_path).unwrap();

        assert_eq!(restored.images.len(), 1);
        assert_eq!(restored.annotations.len(), 1);
        assert_eq!(restored.categories.len(), 1);

        // Check category name preserved
        assert_eq!(restored.categories[0].name, "person");
    }
}
