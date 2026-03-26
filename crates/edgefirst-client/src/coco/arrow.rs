// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! COCO to EdgeFirst Arrow format conversion.
//!
//! Provides high-performance conversion between COCO JSON and EdgeFirst Arrow
//! format, supporting async operations and progress tracking.

use super::{
    convert::{
        box2d_to_coco_bbox, coco_bbox_to_box2d, coco_segmentation_to_mask_data,
        coco_segmentation_to_polygon, polygon_to_coco_polygon,
    },
    reader::CocoReader,
    types::{CocoImage, CocoIndex, CocoInfo, CocoSegmentation},
    writer::{CocoDatasetBuilder, CocoWriter},
};
use crate::{Annotation, Box2d, Error, Polygon, Progress, Sample};
use polars::prelude::*;
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::{Semaphore, mpsc::Sender};

/// Schema version written into Arrow IPC file metadata.
pub const SCHEMA_VERSION: &str = "2026.04";

/// Polygon rings for a single row: each ring is a vec of `(x, y)` coordinate pairs.
type PolygonRings = Vec<Vec<(f32, f32)>>;

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

    // Build schema-level metadata
    let mut metadata: BTreeMap<PlSmallStr, PlSmallStr> = BTreeMap::new();
    metadata.insert(
        PlSmallStr::from("schema_version"),
        PlSmallStr::from(SCHEMA_VERSION),
    );

    // Build category_metadata JSON from all categories.
    // Includes id, frequency, and any LVIS fields (synset, synonyms, def).
    // All categories are stored so that categories without annotations
    // (e.g., those only referenced in neg_category_ids) can be
    // reconstructed during Arrow→COCO export.
    if !dataset.categories.is_empty() {
        let cat_meta: HashMap<String, serde_json::Value> = dataset
            .categories
            .iter()
            .map(|c| {
                let mut entry = serde_json::Map::new();
                entry.insert("id".to_string(), serde_json::json!(c.id));
                if let Some(ref f) = c.frequency {
                    entry.insert(
                        "frequency".to_string(),
                        serde_json::Value::String(f.clone()),
                    );
                }
                if let Some(ref s) = c.synset {
                    entry.insert("synset".to_string(), serde_json::Value::String(s.clone()));
                }
                if let Some(ref syns) = c.synonyms {
                    entry.insert("synonyms".to_string(), serde_json::json!(syns));
                }
                if let Some(ref d) = c.def {
                    entry.insert(
                        "definition".to_string(),
                        serde_json::Value::String(d.clone()),
                    );
                }
                if let Some(ref sc) = c.supercategory {
                    entry.insert(
                        "supercategory".to_string(),
                        serde_json::Value::String(sc.clone()),
                    );
                }
                // Note: image_count and instance_count are intentionally not
                // stored — they are recomputable statistics that can be derived
                // from the annotations at any time.
                (c.name.clone(), serde_json::Value::Object(entry))
            })
            .collect();

        let json = serde_json::to_string(&cat_meta).unwrap_or_default();
        metadata.insert(
            PlSmallStr::from("category_metadata"),
            PlSmallStr::from(json.as_str()),
        );
    }

    // Write labels metadata: sorted list of category names by category_id.
    if !dataset.categories.is_empty() {
        let mut cats: Vec<_> = dataset.categories.iter().collect();
        cats.sort_by_key(|c| c.id);
        let labels: Vec<String> = cats.iter().map(|c| c.name.clone()).collect();
        let labels_json = serde_json::to_string(&labels).unwrap_or_default();
        metadata.insert(PlSmallStr::from("labels"), PlSmallStr::from(labels_json));
    }

    // Write Arrow file
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(output_path)?;
    let mut writer = IpcWriter::new(&mut file);
    writer.set_custom_schema_metadata(Arc::new(metadata));
    writer.finish(&mut df.clone())?;

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

    // Translate LVIS image-level fields to label_index lists
    let neg_label_indices = image.neg_category_ids.as_ref().map(|ids| {
        ids.iter()
            .filter_map(|&id| index.label_index(id).map(|idx| idx as u32))
            .collect::<Vec<u32>>()
    });
    let not_exhaustive_label_indices = image.not_exhaustive_category_ids.as_ref().map(|ids| {
        ids.iter()
            .filter_map(|&id| index.label_index(id).map(|idx| idx as u32))
            .collect::<Vec<u32>>()
    });

    let mut samples: Vec<Sample> = annotations
        .iter()
        .filter_map(|ann| {
            let label = index.label_name(ann.category_id)?;
            let label_index = index.label_index(ann.category_id);

            // Convert bbox
            let box2d = coco_bbox_to_box2d(&ann.bbox, image.width, image.height);

            // Convert segmentation based on type:
            // - Polygon → annotation.polygon (normalized coords)
            // - RLE/CompressedRle → annotation.mask (PNG-encoded MaskData)
            let (polygon, mask) = if include_masks {
                if let Some(seg) = &ann.segmentation {
                    match seg {
                        CocoSegmentation::Polygon(_) => {
                            let poly =
                                coco_segmentation_to_polygon(seg, image.width, image.height).ok();
                            (poly, None)
                        }
                        CocoSegmentation::Rle(_) | CocoSegmentation::CompressedRle(_) => {
                            let mask_data = coco_segmentation_to_mask_data(seg).ok().flatten();
                            (None, mask_data)
                        }
                    }
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            let mut annotation = Annotation::new();
            annotation.set_name(Some(sample_name.clone()));
            annotation.set_label(Some(label.to_string()));
            annotation.set_label_index(label_index);
            annotation.set_box2d(Some(box2d));
            annotation.set_polygon(polygon);
            annotation.set_mask(mask);
            annotation.set_group(group.map(String::from));
            annotation.set_iscrowd(Some(ann.iscrowd != 0));
            annotation.set_category_frequency(index.frequency(ann.category_id).map(String::from));

            // Map COCO score to appropriate geometry score field
            if let Some(score) = ann.score {
                let score_f32 = score as f32;
                if annotation.mask().is_some() {
                    annotation.set_mask_score(Some(score_f32));
                } else if annotation.polygon().is_some() {
                    annotation.set_polygon_score(Some(score_f32));
                } else {
                    annotation.set_box2d_score(Some(score_f32));
                }
            }

            let mut sample = Sample {
                image_name: Some(sample_name.clone()),
                width: Some(image.width),
                height: Some(image.height),
                group: group.map(String::from),
                annotations: vec![annotation],
                ..Default::default()
            };
            sample.neg_label_indices = neg_label_indices.clone();
            sample.not_exhaustive_label_indices = not_exhaustive_label_indices.clone();

            Some(sample)
        })
        .collect();

    // Emit sentinel for images with no annotations but with neg/exhaustive data.
    // Without this, neg_category_ids would be silently lost for images that have
    // verified-negative labels but no positive annotations.
    if samples.is_empty()
        && (image.neg_category_ids.is_some() || image.not_exhaustive_category_ids.is_some())
    {
        let mut sample = Sample {
            image_name: Some(sample_name.clone()),
            width: Some(image.width),
            height: Some(image.height),
            group: group.map(String::from),
            ..Default::default()
        };
        sample.neg_label_indices = neg_label_indices;
        sample.not_exhaustive_label_indices = not_exhaustive_label_indices;
        samples.push(sample);
    }

    samples
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
/// Reads an Arrow file and produces COCO JSON output. LVIS extension fields
/// are preserved when present in the Arrow file: `neg_category_ids`,
/// `not_exhaustive_category_ids`, category `frequency`, annotation `iscrowd`,
/// `supercategory`, and category metadata (`synset`, `synonyms`, `def`).
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

    // Read file-level metadata (must be done before consuming the reader)
    let (schema_version, category_metadata_json, labels_metadata_json) = {
        let mut meta_file = std::fs::File::open(arrow_path)?;
        let mut reader = IpcReader::new(&mut meta_file);
        let meta = reader.custom_metadata().ok().flatten();
        let sv = meta.as_ref().and_then(|m| {
            m.get(&PlSmallStr::from("schema_version"))
                .map(|s| s.to_string())
        });
        let cm = meta.as_ref().and_then(|m| {
            m.get(&PlSmallStr::from("category_metadata"))
                .map(|s| s.to_string())
        });
        let lm = meta
            .as_ref()
            .and_then(|m| m.get(&PlSmallStr::from("labels")).map(|s| s.to_string()));
        (sv, cm, lm)
    };

    // Determine format version: absent → 2025.10, present → use value
    let is_legacy = schema_version.is_none();

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
        .column("label")
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
                .unwrap_or_else(|| vec![String::new(); total_rows])
        })
        .unwrap_or_else(|| vec![String::new(); total_rows]);

    let label_indices: Vec<Option<u64>> = df
        .column("label_index")
        .ok()
        .map(|c| {
            c.u64()
                .ok()
                .map(|s| s.into_iter().collect())
                .unwrap_or_else(|| vec![None; total_rows])
        })
        .unwrap_or_else(|| vec![None; total_rows]);

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
    let box2ds = df
        .column("box2d")
        .ok()
        .map(extract_all_box2ds)
        .transpose()?
        .unwrap_or_else(|| vec![[0.0; 4]; total_rows]);

    // Extract segmentation data based on schema version
    //
    // 2025.10 (legacy): mask column is List(Float32) with NaN-separated polygon coords
    // 2026.04+:         polygon column is List(List(Float32)), mask column is Binary (PNG)
    let legacy_masks: Option<Vec<Vec<f32>>> = if is_legacy && options.include_masks {
        df.column("mask").ok().map(extract_all_masks).transpose()?
    } else {
        None
    };

    let polygons_2026: Option<Vec<Option<PolygonRings>>> = if !is_legacy && options.include_masks {
        df.column("polygon")
            .ok()
            .map(|c| extract_all_polygons(c, total_rows))
    } else {
        None
    };

    let mask_binary_2026: Option<Vec<Option<Vec<u8>>>> = if !is_legacy && options.include_masks {
        df.column("mask")
            .ok()
            .map(|c| extract_all_binary_masks(c, total_rows))
    } else {
        None
    };

    // Extract all sizes upfront if present
    let sizes = df
        .column("size")
        .ok()
        .and_then(|c| extract_all_sizes(c).ok());

    // Extract iscrowd column (optional, Boolean in 2026.04, UInt32 in older schemas)
    let iscrowds: Vec<u8> = df
        .column("iscrowd")
        .ok()
        .map(|c| {
            // Try Boolean first (2026.04 schema), then fall back to UInt32 (older schemas)
            if let Ok(bool_ca) = c.bool() {
                bool_ca
                    .into_iter()
                    .map(|v| if v.unwrap_or(false) { 1 } else { 0 })
                    .collect()
            } else {
                c.u32()
                    .ok()
                    .map(|s| s.into_iter().map(|v| v.unwrap_or(0) as u8).collect())
                    .unwrap_or_else(|| vec![0; total_rows])
            }
        })
        .unwrap_or_else(|| vec![0; total_rows]);

    // Extract category_frequency column (optional, Categorical/String)
    let category_frequencies: Vec<Option<String>> = df
        .column("category_frequency")
        .ok()
        .and_then(|c| c.cast(&DataType::String).ok())
        .map(|c| {
            c.str()
                .ok()
                .map(|s| s.into_iter().map(|v| v.map(String::from)).collect())
                .unwrap_or_else(|| vec![None; total_rows])
        })
        .unwrap_or_else(|| vec![None; total_rows]);

    // Extract neg_label_indices column (optional, List<UInt32>)
    let neg_label_indices: Vec<Option<Vec<u32>>> = df
        .column("neg_label_indices")
        .ok()
        .map(|c| extract_list_u32_column(c, total_rows))
        .unwrap_or_else(|| vec![None; total_rows]);

    // Extract not_exhaustive_label_indices column (optional, List<UInt32>)
    let not_exhaustive_label_indices: Vec<Option<Vec<u32>>> = df
        .column("not_exhaustive_label_indices")
        .ok()
        .map(|c| extract_list_u32_column(c, total_rows))
        .unwrap_or_else(|| vec![None; total_rows]);

    // Extract score columns (2026.04 schema)
    let box2d_scores: Vec<Option<f32>> = extract_f32_column(&df, "box2d_score", total_rows);
    let polygon_scores: Vec<Option<f32>> = extract_f32_column(&df, "polygon_score", total_rows);
    let mask_scores: Vec<Option<f32>> = extract_f32_column(&df, "mask_score", total_rows);

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
            let id = if let Some(Some(idx)) = label_indices.get(i) {
                builder.add_category_with_id(*idx as u32, label, None)
            } else {
                builder.add_category(label, None)
            };
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

        // Skip sentinel rows (empty label = image with neg/exhaustive data but no annotations)
        if label.is_empty() {
            continue;
        }

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

        // Build segmentation based on schema version
        let segmentation = if options.include_masks {
            if is_legacy {
                // 2025.10: mask column contains NaN-separated flat polygon coords
                legacy_masks.as_ref().and_then(|m| {
                    m.get(i).and_then(|coords| {
                        if coords.is_empty() {
                            None
                        } else {
                            let rings = unflatten_polygon_coords(coords);
                            let polygon = Polygon::new(rings);
                            let coco_poly = polygon_to_coco_polygon(&polygon, width, height);
                            if coco_poly.is_empty() {
                                None
                            } else {
                                Some(CocoSegmentation::Polygon(coco_poly))
                            }
                        }
                    })
                })
            } else {
                // 2026.04+: try mask (Binary/PNG → RLE) first, then polygon column
                let mask_seg =
                    mask_binary_2026.as_ref().and_then(|masks| {
                        masks.get(i).and_then(|opt_bytes| {
                            opt_bytes.as_ref().and_then(|png_bytes| {
                            if png_bytes.is_empty() {
                                return None;
                            }
                            let mask_data = crate::MaskData::from_png(png_bytes.clone());
                            let mw = mask_data.width();
                            let mh = mask_data.height();
                            let bit_depth = mask_data.bit_depth();
                            let decoded = mask_data.decode();

                            let binary_mask = match bit_depth {
                                1 => decoded,
                                8 => {
                                    log::warn!(
                                        "Binarizing 8-bit mask for row {} — score data is lost",
                                        i
                                    );
                                    decoded.iter().map(|&v| if v >= 128 { 1 } else { 0 }).collect()
                                }
                                16 => {
                                    log::warn!(
                                        "Binarizing 16-bit mask for row {} — score data is lost",
                                        i
                                    );
                                    // 16-bit decodes to big-endian byte pairs
                                    decoded
                                        .chunks(2)
                                        .map(|pair| {
                                            let val = if pair.len() == 2 {
                                                u16::from_be_bytes([pair[0], pair[1]])
                                            } else {
                                                0
                                            };
                                            if val >= 32768 { 1u8 } else { 0u8 }
                                        })
                                        .collect()
                                }
                                _ => decoded,
                            };

                            let rle = super::convert::encode_rle(&binary_mask, mw, mh);
                            Some(CocoSegmentation::Rle(rle))
                        })
                        })
                    });

                if mask_seg.is_some() {
                    mask_seg
                } else {
                    // Fall back to polygon column
                    polygons_2026.as_ref().and_then(|polys| {
                        polys.get(i).and_then(|opt_rings| {
                            opt_rings.as_ref().and_then(|rings| {
                                if rings.is_empty() {
                                    return None;
                                }
                                let polygon = Polygon::new(rings.clone());
                                let coco_poly = polygon_to_coco_polygon(&polygon, width, height);
                                if coco_poly.is_empty() {
                                    None
                                } else {
                                    Some(CocoSegmentation::Polygon(coco_poly))
                                }
                            })
                        })
                    })
                }
            }
        } else {
            None
        };

        // Determine the score: use first non-null from box2d_score, polygon_score, mask_score
        let score: Option<f64> = mask_scores[i]
            .or(polygon_scores[i])
            .or(box2d_scores[i])
            .map(|s| s as f64);

        if let Some(bbox) = bbox {
            let iscrowd = iscrowds[i];
            let ann_id = builder.add_annotation_with_iscrowd(
                image_id,
                category_id,
                bbox,
                segmentation,
                iscrowd,
            );

            // Set score on the annotation if present
            if let Some(score_val) = score {
                builder.set_annotation_score(ann_id, score_val);
            }
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

    // Send final progress event (may not have fired if last rows were filtered)
    if let Some(ref p) = progress
        && last_progress_update < total_rows.saturating_sub(1)
    {
        let _ = p
            .send(Progress {
                current: total_rows,
                total: total_rows,
                status: None,
            })
            .await;
    }

    // Third pass: set LVIS image-level fields (neg/not-exhaustive category IDs)
    // Since label_index == category_id, we can use the values directly.
    {
        let mut processed_images: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for i in 0..total_rows {
            if !groups_to_filter.is_empty() && !groups_to_filter.contains(&groups[i]) {
                continue;
            }
            let name = &names[i];
            if let Some(&image_id) = image_ids.get(name) {
                if !processed_images.insert(image_id) {
                    continue;
                }
                let neg = neg_label_indices[i].clone();
                let not_exhaustive = not_exhaustive_label_indices[i].clone();
                if neg.is_some() || not_exhaustive.is_some() {
                    builder.set_image_neg_categories(image_id, neg, not_exhaustive);
                }
            }
        }
    }

    // Set category frequency from the category_frequency column.
    // Build a map of category_name -> frequency from the first occurrence.
    {
        let mut freq_map: HashMap<String, String> = HashMap::new();
        for i in 0..total_rows {
            if !groups_to_filter.is_empty() && !groups_to_filter.contains(&groups[i]) {
                continue;
            }
            let label = &labels[i];
            if !label.is_empty()
                && !freq_map.contains_key(label)
                && let Some(ref freq) = category_frequencies[i]
            {
                freq_map.insert(label.clone(), freq.clone());
            }
        }
        for (name, freq) in &freq_map {
            builder.set_category_metadata(name, None, Some(freq.clone()), None, None);
        }
    }

    // Set category metadata from file-level metadata JSON
    // (id, frequency, synset, synonyms, def, supercategory).
    // Also creates categories that exist in metadata but have no annotations
    // (e.g., categories only referenced in neg_category_ids).
    // set_category_metadata only updates fields that are Some, so frequency
    // set from the column above is preserved for categories that had annotations.
    if let Some(ref json_str) = category_metadata_json
        && let Ok(meta) = serde_json::from_str::<HashMap<String, serde_json::Value>>(json_str)
    {
        for (cat_name, value) in &meta {
            let supercategory = value.get("supercategory").and_then(|v| v.as_str());

            // If this category doesn't exist yet, create it with the stored id
            if !category_ids.contains_key(cat_name.as_str()) {
                let cat_id = value.get("id").and_then(|v| v.as_u64()).map(|id| id as u32);
                let id = if let Some(cat_id) = cat_id {
                    builder.add_category_with_id(cat_id, cat_name, supercategory)
                } else {
                    builder.add_category(cat_name, supercategory)
                };
                category_ids.insert(cat_name.clone(), id);
            } else {
                // Category already exists — set supercategory if present in metadata
                if let Some(sc) = supercategory {
                    builder.set_category_supercategory(cat_name, sc);
                }
            }

            let synset = value
                .get("synset")
                .and_then(|v| v.as_str())
                .map(String::from);
            let frequency = value
                .get("frequency")
                .and_then(|v| v.as_str())
                .map(String::from);
            let synonyms = value.get("synonyms").and_then(|v| {
                v.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                })
            });
            let def = value
                .get("definition")
                .and_then(|v| v.as_str())
                .map(String::from);

            builder.set_category_metadata(cat_name, synset, frequency, synonyms, def);
        }
    }

    // Populate category names from labels metadata if categories weren't set
    // from category_metadata (e.g., older files that only have labels list).
    if category_metadata_json.is_none()
        && let Some(ref labels_json) = labels_metadata_json
        && let Ok(label_names) = serde_json::from_str::<Vec<String>>(labels_json)
    {
        for label_name in &label_names {
            if !category_ids.contains_key(label_name) {
                let id = builder.add_category(label_name, None);
                category_ids.insert(label_name.clone(), id);
            }
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

/// Extract a List<UInt32> column into a vector of optional Vec<u32>.
fn extract_list_u32_column(col: &Column, total_rows: usize) -> Vec<Option<Vec<u32>>> {
    col.list()
        .ok()
        .map(|list| {
            (0..list.len())
                .map(|i| {
                    list.get_as_series(i).and_then(|series| {
                        series
                            .u32()
                            .ok()
                            .map(|ca| ca.into_iter().flatten().collect::<Vec<u32>>())
                    })
                })
                .collect()
        })
        .unwrap_or_else(|| vec![None; total_rows])
}

/// Extract polygon rings from a `List(List(Float32))` column (2026.04 schema).
///
/// Each row is an optional list of rings; each ring is a list of flat `[x, y, x, y, ...]`
/// coordinate pairs.
fn extract_all_polygons(col: &Column, total_rows: usize) -> Vec<Option<PolygonRings>> {
    let outer_list = match col.list() {
        Ok(l) => l,
        Err(_) => return vec![None; total_rows],
    };

    let mut result = Vec::with_capacity(total_rows);
    for i in 0..outer_list.len() {
        let rings = outer_list.get_as_series(i).and_then(|ring_series| {
            let inner_list = ring_series.list().ok()?;
            let mut rings = Vec::new();
            for j in 0..inner_list.len() {
                if let Some(coords_series) = inner_list.get_as_series(j)
                    && let Ok(f32_ca) = coords_series.f32()
                {
                    let coords: Vec<f32> = f32_ca.into_iter().map(|v| v.unwrap_or(0.0)).collect();
                    // Convert flat [x, y, x, y, ...] to Vec<(f32, f32)>
                    let points: Vec<(f32, f32)> = coords
                        .chunks(2)
                        .filter(|c| c.len() == 2)
                        .map(|c| (c[0], c[1]))
                        .collect();
                    if !points.is_empty() {
                        rings.push(points);
                    }
                }
            }
            if rings.is_empty() { None } else { Some(rings) }
        });
        result.push(rings);
    }
    result
}

/// Extract binary mask data from a `Binary` column (2026.04 schema — PNG bytes).
fn extract_all_binary_masks(col: &Column, total_rows: usize) -> Vec<Option<Vec<u8>>> {
    let binary_ca = match col.binary() {
        Ok(b) => b,
        Err(_) => return vec![None; total_rows],
    };

    (0..binary_ca.len())
        .map(|i| binary_ca.get(i).map(|bytes| bytes.to_vec()))
        .collect()
}

/// Extract an optional Float32 column by name.
fn extract_f32_column(df: &DataFrame, name: &str, total_rows: usize) -> Vec<Option<f32>> {
    df.column(name)
        .ok()
        .and_then(|c| c.f32().ok())
        .map(|ca| ca.into_iter().collect())
        .unwrap_or_else(|| vec![None; total_rows])
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
                ..Default::default()
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [100.0, 100.0, 200.0, 200.0],
                area: 40000.0,
                iscrowd: 0,
                segmentation: None,
                score: None,
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
                ..Default::default()
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
                score: None,
            }],
            ..Default::default()
        };

        let index = CocoIndex::from_dataset(&dataset);

        // With masks enabled
        let samples_with_mask = convert_image_annotations(&image, &index, true, None);
        assert!(samples_with_mask[0].annotations[0].polygon().is_some());

        // With masks disabled
        let samples_no_mask = convert_image_annotations(&image, &index, false, None);
        assert!(samples_no_mask[0].annotations[0].polygon().is_none());
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
                score: None,
            }],
            categories: vec![CocoCategory {
                id: 1,
                name: "person".to_string(),
                supercategory: Some("human".to_string()),
                ..Default::default()
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

    // =========================================================================
    // Arrow IPC file metadata tests
    // =========================================================================

    #[tokio::test]
    async fn test_coco_to_arrow_schema_version_metadata() {
        let temp_dir = TempDir::new().unwrap();

        // Create minimal COCO JSON (no LVIS fields)
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
        coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();

        // Read back and verify schema_version metadata
        let mut file = std::fs::File::open(&arrow_path).unwrap();
        let mut reader = IpcReader::new(&mut file);
        let custom_meta = reader.custom_metadata().unwrap();
        assert!(custom_meta.is_some(), "custom metadata should be present");

        let meta = custom_meta.unwrap();
        assert_eq!(
            meta.get(&PlSmallStr::from("schema_version")),
            Some(&PlSmallStr::from(SCHEMA_VERSION)),
            "schema_version metadata should be '2026.04'"
        );

        // category_metadata is always present when there are categories
        assert!(
            meta.contains_key(&PlSmallStr::from("category_metadata")),
            "category_metadata should be present even without LVIS fields"
        );
    }

    #[tokio::test]
    async fn test_coco_to_arrow_category_metadata_lvis() {
        let temp_dir = TempDir::new().unwrap();

        // Create COCO JSON with LVIS category fields
        let coco_json = r#"{
            "images": [
                {"id": 1, "width": 640, "height": 480, "file_name": "test.jpg"}
            ],
            "annotations": [
                {"id": 1, "image_id": 1, "category_id": 1, "bbox": [10, 20, 100, 80], "area": 8000, "iscrowd": 0},
                {"id": 2, "image_id": 1, "category_id": 2, "bbox": [50, 60, 80, 40], "area": 3200, "iscrowd": 0}
            ],
            "categories": [
                {
                    "id": 1,
                    "name": "aerosol_can",
                    "synset": "aerosol.n.02",
                    "synonyms": ["aerosol_can", "spray_can"],
                    "def": "a dispenser that holds a substance under pressure"
                },
                {
                    "id": 2,
                    "name": "person",
                    "supercategory": "human"
                }
            ]
        }"#;

        let coco_path = temp_dir.path().join("lvis.json");
        std::fs::write(&coco_path, coco_json).unwrap();

        let arrow_path = temp_dir.path().join("lvis_output.arrow");
        let options = CocoToArrowOptions::default();
        coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();

        // Read back and verify metadata
        let mut file = std::fs::File::open(&arrow_path).unwrap();
        let mut reader = IpcReader::new(&mut file);
        let custom_meta = reader.custom_metadata().unwrap();
        assert!(custom_meta.is_some(), "custom metadata should be present");

        let meta = custom_meta.unwrap();

        // schema_version is always present
        assert_eq!(
            meta.get(&PlSmallStr::from("schema_version")),
            Some(&PlSmallStr::from(SCHEMA_VERSION)),
        );

        // category_metadata should be present (aerosol_can has LVIS fields)
        let cat_meta_str = meta
            .get(&PlSmallStr::from("category_metadata"))
            .expect("category_metadata should be present for LVIS data");

        let cat_meta: HashMap<String, serde_json::Value> =
            serde_json::from_str(cat_meta_str.as_str()).unwrap();

        // Both categories should be present (all categories are now stored)
        assert!(
            cat_meta.contains_key("aerosol_can"),
            "aerosol_can should be in category_metadata"
        );
        assert!(
            cat_meta.contains_key("person"),
            "person should also be in category_metadata"
        );

        // Verify aerosol_can entry contents
        let aerosol = cat_meta.get("aerosol_can").unwrap();
        assert_eq!(
            aerosol.get("synset").and_then(|v| v.as_str()),
            Some("aerosol.n.02")
        );
        assert_eq!(
            aerosol.get("definition").and_then(|v| v.as_str()),
            Some("a dispenser that holds a substance under pressure")
        );
        let synonyms = aerosol.get("synonyms").and_then(|v| v.as_array()).unwrap();
        assert_eq!(synonyms.len(), 2);
        assert_eq!(synonyms[0].as_str(), Some("aerosol_can"));
        assert_eq!(synonyms[1].as_str(), Some("spray_can"));
    }

    // =========================================================================
    // LVIS round-trip tests
    // =========================================================================

    #[tokio::test]
    async fn test_coco_arrow_roundtrip_lvis_supercategory() {
        let temp_dir = TempDir::new().unwrap();

        // Create COCO JSON with supercategory
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

        let coco_path = temp_dir.path().join("original.json");
        std::fs::write(&coco_path, coco_json).unwrap();

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

        // Verify supercategory is preserved
        let reader = CocoReader::new();
        let restored = reader.read_json(&restored_path).unwrap();

        assert_eq!(restored.categories.len(), 1);
        assert_eq!(restored.categories[0].name, "person");
        assert_eq!(
            restored.categories[0].supercategory,
            Some("human".to_string()),
            "supercategory should survive COCO→Arrow→COCO round-trip"
        );
    }

    #[tokio::test]
    async fn test_coco_arrow_roundtrip_neg_categories_no_annotations() {
        let temp_dir = TempDir::new().unwrap();

        // Create COCO JSON: image has neg_category_ids but NO annotations
        let coco_json = r#"{
            "images": [
                {
                    "id": 1,
                    "width": 640,
                    "height": 480,
                    "file_name": "empty.jpg",
                    "neg_category_ids": [1, 2]
                }
            ],
            "annotations": [],
            "categories": [
                {"id": 1, "name": "cat", "supercategory": "animal"},
                {"id": 2, "name": "dog", "supercategory": "animal"}
            ]
        }"#;

        let coco_path = temp_dir.path().join("original.json");
        std::fs::write(&coco_path, coco_json).unwrap();

        // Convert to Arrow
        let arrow_path = temp_dir.path().join("converted.arrow");
        let options = CocoToArrowOptions::default();
        let sample_count = coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();

        // Should have 1 sentinel sample (image with neg data but no annotations)
        assert_eq!(
            sample_count, 1,
            "sentinel row should be emitted for image with neg data"
        );

        // Convert back to COCO
        let restored_path = temp_dir.path().join("restored.json");
        let options = ArrowToCocoOptions::default();
        arrow_to_coco(&arrow_path, &restored_path, &options, None)
            .await
            .unwrap();

        // Verify neg_category_ids survived the round-trip
        let reader = CocoReader::new();
        let restored = reader.read_json(&restored_path).unwrap();

        assert_eq!(restored.images.len(), 1);
        assert_eq!(restored.annotations.len(), 0, "no annotations expected");
        assert_eq!(restored.categories.len(), 2, "both categories should exist");

        let neg = restored.images[0].neg_category_ids.as_ref();
        assert!(
            neg.is_some(),
            "neg_category_ids should survive round-trip for zero-annotation image"
        );
        let neg_ids = neg.unwrap();
        assert_eq!(neg_ids.len(), 2, "should have 2 neg categories");
        assert!(neg_ids.contains(&1), "neg_category_ids should contain 1");
        assert!(neg_ids.contains(&2), "neg_category_ids should contain 2");

        // Verify supercategory survives for annotation-free categories
        for cat in &restored.categories {
            assert_eq!(
                cat.supercategory,
                Some("animal".to_string()),
                "supercategory should survive round-trip for annotation-free category '{}'",
                cat.name
            );
        }
    }

    #[test]
    fn test_convert_image_annotations_neg_only_no_annotations() {
        let image = CocoImage {
            id: 1,
            width: 640,
            height: 480,
            file_name: "neg_only.jpg".to_string(),
            neg_category_ids: Some(vec![1, 2]),
            ..Default::default()
        };

        let dataset = CocoDataset {
            images: vec![image.clone()],
            categories: vec![
                CocoCategory {
                    id: 1,
                    name: "cat".to_string(),
                    supercategory: Some("animal".to_string()),
                    ..Default::default()
                },
                CocoCategory {
                    id: 2,
                    name: "dog".to_string(),
                    supercategory: Some("animal".to_string()),
                    ..Default::default()
                },
            ],
            annotations: vec![],
            ..Default::default()
        };

        let index = CocoIndex::from_dataset(&dataset);
        let samples = convert_image_annotations(&image, &index, true, None);

        // Should emit 1 sentinel sample (no annotations but has neg data)
        assert_eq!(
            samples.len(),
            1,
            "sentinel row should be emitted for neg-only image"
        );
        assert_eq!(samples[0].image_name, Some("neg_only".to_string()));
        assert!(
            samples[0].annotations.is_empty(),
            "sentinel should have no annotations"
        );
        assert!(
            samples[0].neg_label_indices.is_some(),
            "sentinel should preserve neg_label_indices"
        );
        assert_eq!(samples[0].neg_label_indices.as_ref().unwrap().len(), 2);
    }
}
