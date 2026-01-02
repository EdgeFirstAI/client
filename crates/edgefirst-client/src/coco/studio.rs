// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! COCO import/export for EdgeFirst Studio.
//!
//! Provides high-level workflows for importing COCO datasets into Studio
//! and exporting Studio datasets to COCO format.
//!
//! **Note:** COCO datasets must be extracted before import. ZIP archives
//! are not supported directly - extract images to `train2017/`, `val2017/`,
//! etc. subdirectories first.

use super::{
    convert::*,
    reader::{CocoReadOptions, CocoReader, read_coco_directory},
    types::*,
    writer::{CocoDatasetBuilder, CocoWriteOptions, CocoWriter},
};
use crate::{
    Annotation, AnnotationSetID, Client, DatasetID, Error, FileType, Progress, Sample, SampleFile,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use tokio::sync::mpsc::Sender;

/// Result of a COCO import operation.
#[derive(Debug, Clone)]
pub struct CocoImportResult {
    /// Total number of images in the COCO dataset.
    pub total_images: usize,
    /// Number of images that were already imported (skipped).
    pub skipped: usize,
    /// Number of images newly imported.
    pub imported: usize,
}

/// Options for importing COCO to Studio.
#[derive(Debug, Clone)]
pub struct CocoImportOptions {
    /// Include segmentation masks.
    pub include_masks: bool,
    /// Include images (upload them to Studio).
    pub include_images: bool,
    /// Group name for all samples (e.g., "train", "val").
    pub group: Option<String>,
    /// Batch size for API calls.
    pub batch_size: usize,
    /// Maximum concurrent uploads (default: 64).
    pub concurrency: usize,
    /// Resume import by skipping already-imported samples.
    /// When true (default), checks existing samples and skips duplicates.
    pub resume: bool,
}

impl Default for CocoImportOptions {
    fn default() -> Self {
        Self {
            include_masks: true,
            include_images: true,
            group: None,
            batch_size: 100,
            concurrency: 64,
            resume: true,
        }
    }
}

/// Options for exporting Studio to COCO.
#[derive(Debug, Clone)]
pub struct CocoExportOptions {
    /// Filter by group names (empty = all).
    pub groups: Vec<String>,
    /// Include segmentation masks in output.
    pub include_masks: bool,
    /// Include images in output (download and add to ZIP).
    pub include_images: bool,
    /// Output as ZIP archive (if false, output JSON only).
    pub output_zip: bool,
    /// Pretty-print JSON.
    pub pretty_json: bool,
    /// COCO info section.
    pub info: Option<CocoInfo>,
}

impl Default for CocoExportOptions {
    fn default() -> Self {
        Self {
            groups: vec![],
            include_masks: true,
            include_images: false,
            output_zip: false,
            pretty_json: false,
            info: None,
        }
    }
}

/// Import COCO dataset into EdgeFirst Studio.
///
/// Reads COCO annotations and images from an extracted directory,
/// converts to EdgeFirst format, and uploads to Studio using the bulk API.
///
/// # Arguments
/// * `client` - Authenticated Studio client
/// * `coco_path` - Path to COCO annotation JSON file (images must be extracted
///   in sibling directories like `train2017/`, `val2017/`)
/// * `dataset_id` - Target dataset in Studio
/// * `annotation_set_id` - Target annotation set
/// * `options` - Import options
/// * `progress` - Optional progress channel
///
/// # Returns
/// Import result with counts of total, skipped, and imported samples
///
/// # Errors
/// Returns an error if:
/// - No annotation file is found
/// - Images are not extracted (ZIP archives not supported)
/// - Upload to Studio fails
///
/// # Resume Behavior
/// When `options.resume` is true (default), the function checks which samples
/// already exist in the target dataset and skips them. This allows resuming
/// interrupted imports without re-uploading data.
///
/// # Example
/// ```bash
/// # First extract COCO dataset:
/// cd ~/Datasets/COCO
/// unzip annotations_trainval2017.zip
/// unzip val2017.zip
///
/// # Then import:
/// edgefirst import-coco annotations/instances_val2017.json DS_ID AS_ID --group val
///
/// # If interrupted, simply run again - it will resume from where it left off
/// ```
pub async fn import_coco_to_studio(
    client: &Client,
    coco_path: impl AsRef<Path>,
    dataset_id: DatasetID,
    annotation_set_id: AnnotationSetID,
    options: &CocoImportOptions,
    progress: Option<Sender<Progress>>,
) -> Result<CocoImportResult, Error> {
    let coco_path = coco_path.as_ref();

    // Read annotations - when given a directory, read ALL annotation files
    let (dataset, images_dir) = if coco_path.is_dir() {
        // Read all annotation files and merge into one dataset
        let datasets = read_coco_directory(coco_path, &CocoReadOptions::default())?;
        log::info!("Found {} annotation files in directory", datasets.len());

        // Merge all datasets, preserving group info by prefixing file_name
        let mut merged = CocoDataset::default();
        for (mut ds, group) in datasets {
            log::info!(
                "  - {} group: {} images, {} annotations",
                group,
                ds.images.len(),
                ds.annotations.len()
            );
            // Prefix file_name with group folder so infer_group_from_folder can extract it
            // e.g., "000000123.jpg" -> "train2017/000000123.jpg"
            for image in &mut ds.images {
                if !image.file_name.contains('/') {
                    image.file_name = format!("{}2017/{}", group, image.file_name);
                }
            }
            merge_coco_datasets(&mut merged, ds);
        }
        (merged, coco_path.to_path_buf())
    } else if coco_path.extension().is_some_and(|e| e == "json") {
        // JSON file directly
        let reader = CocoReader::new();
        let dataset = reader.read_json(coco_path)?;
        let parent = coco_path
            .parent()
            .and_then(|p| p.parent()) // Go up from annotations/ to COCO root
            .unwrap_or(Path::new("."));
        (dataset, parent.to_path_buf())
    } else {
        return Err(Error::InvalidParameters(
            "COCO import requires a JSON annotation file or directory. \
             ZIP archives must be extracted first."
                .to_string(),
        ));
    };

    // User-provided group acts as a filter (only import images from matching
    // folders)
    let group_filter = options.group.as_deref();

    let total_images = dataset.images.len();
    if total_images == 0 {
        return Err(Error::MissingAnnotations(
            "No images found in COCO dataset".to_string(),
        ));
    }

    // Validate that images are extracted
    if options.include_images {
        validate_images_extracted(&dataset, &images_dir)?;
    }

    // Check for existing samples if resume is enabled
    // Query ALL samples (no group filter) to properly detect duplicates
    let existing_names: HashSet<String> = if options.resume {
        log::info!("Checking for existing samples in dataset {}...", dataset_id);
        let names = client.sample_names(dataset_id.clone(), &[], None).await?;
        log::info!("Found {} existing samples in dataset", names.len());
        if !names.is_empty() {
            // Log a few sample names for debugging
            let samples: Vec<_> = names.iter().take(3).collect();
            log::debug!("Sample names from server: {:?}", samples);
        }
        names
    } else {
        HashSet::new()
    };

    // Filter images:
    // 1. Skip if already imported (resume mode)
    // 2. Skip if doesn't match group filter (when --group is specified)
    let images_to_import: Vec<_> = dataset
        .images
        .iter()
        .filter(|img| {
            // Check group filter first
            if let Some(filter) = group_filter {
                let inferred = super::reader::infer_group_from_folder(&img.file_name);
                if inferred.as_deref() != Some(filter) {
                    return false;
                }
            }

            // Check if already imported
            let sample_name = Path::new(&img.file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
                .unwrap_or_else(|| img.file_name.clone());
            !existing_names.contains(&sample_name)
        })
        .collect();

    // Count images filtered by group vs already imported
    let filtered_by_group = if group_filter.is_some() {
        dataset
            .images
            .iter()
            .filter(|img| {
                let inferred = super::reader::infer_group_from_folder(&img.file_name);
                inferred.as_deref() != group_filter
            })
            .count()
    } else {
        0
    };
    let skipped = total_images - filtered_by_group - images_to_import.len();
    let to_import = images_to_import.len();

    // Log filtering info
    if filtered_by_group > 0 {
        log::info!(
            "Group filter '{}': {} images excluded, {} matching",
            group_filter.unwrap_or(""),
            filtered_by_group,
            total_images - filtered_by_group
        );
    }

    // If nothing to import, return early
    if to_import == 0 {
        if skipped > 0 {
            log::info!(
                "All {} matching images already imported, nothing to do",
                skipped
            );
        } else {
            log::info!("No images to import");
        }
        return Ok(CocoImportResult {
            total_images,
            skipped,
            imported: 0,
        });
    }

    if skipped > 0 {
        log::info!(
            "Resuming import: {} of {} images already imported, {} remaining",
            skipped,
            total_images,
            to_import
        );
    }

    let index = CocoIndex::from_dataset(&dataset);

    if let Some(ref p) = progress {
        let _ = p
            .send(Progress {
                current: 0,
                total: to_import,
            })
            .await;
    }

    // Convert and upload in batches with high concurrency
    let mut imported = 0;

    for batch in images_to_import.chunks(options.batch_size) {
        let mut samples = Vec::with_capacity(batch.len());

        for image in batch {
            // Always infer group from image folder path
            let image_group = super::reader::infer_group_from_folder(&image.file_name);

            let sample = convert_coco_image_to_sample(
                image,
                &index,
                &images_dir,
                options.include_masks,
                options.include_images,
                image_group.as_deref(),
            )?;
            samples.push(sample);
        }

        // Upload batch with high concurrency
        client
            .populate_samples_with_concurrency(
                dataset_id.clone(),
                Some(annotation_set_id.clone()),
                samples,
                None,
                Some(options.concurrency),
            )
            .await?;

        imported += batch.len();

        // Update progress
        if let Some(ref p) = progress {
            let _ = p
                .send(Progress {
                    current: imported,
                    total: to_import,
                })
                .await;
        }
    }

    Ok(CocoImportResult {
        total_images,
        skipped,
        imported,
    })
}

/// Validate that images are extracted and accessible.
fn validate_images_extracted(dataset: &CocoDataset, images_dir: &Path) -> Result<(), Error> {
    // Sample a few images to verify they exist
    let sample_size = std::cmp::min(5, dataset.images.len());
    let mut missing = Vec::new();

    for image in dataset.images.iter().take(sample_size) {
        if find_image_file(images_dir, &image.file_name).is_none() {
            missing.push(image.file_name.clone());
        }
    }

    if !missing.is_empty() {
        let examples: Vec<_> = missing.iter().take(3).cloned().collect();
        return Err(Error::MissingImages(format!(
            "Images must be extracted before import.\n\
             Cannot find: {}\n\n\
             Searched in: {}\n\
             Expected subdirectories: train2017/, val2017/, images/\n\n\
             Please extract your COCO image archives first:\n\
             $ cd {} && unzip train2017.zip && unzip val2017.zip",
            examples.join(", "),
            images_dir.display(),
            images_dir.display()
        )));
    }

    Ok(())
}

/// Find an image file in standard COCO directory locations.
fn find_image_file(base_dir: &Path, file_name: &str) -> Option<PathBuf> {
    let candidates = [
        base_dir.join(file_name),
        base_dir.join("images").join(file_name),
        base_dir.join("train2017").join(file_name),
        base_dir.join("val2017").join(file_name),
        base_dir.join("test2017").join(file_name),
        base_dir.join("train2014").join(file_name),
        base_dir.join("val2014").join(file_name),
    ];
    candidates.into_iter().find(|p| p.exists())
}

/// Infer group name from COCO annotation filename.
///
/// Examples:
/// - `instances_train2017.json` -> Some("train")
/// - `instances_val2017.json` -> Some("val")
/// - `instances_test2017.json` -> Some("test")
/// - `custom.json` -> None
fn infer_group_from_filename(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;

    // Look for patterns like "instances_train2017" or "instances_val2014"
    if let Some(rest) = stem.strip_prefix("instances_") {
        // Remove trailing year digits: "train2017" -> "train"
        let group = rest.trim_end_matches(char::is_numeric);
        if !group.is_empty() {
            return Some(group.to_string());
        }
    }

    // Also handle patterns like "train_instances" or just "train"
    for prefix in ["train", "val", "test", "validation"] {
        if stem.starts_with(prefix) {
            return Some(prefix.to_string());
        }
    }

    None
}

/// Merge two COCO datasets, avoiding duplicates.
///
/// Images and categories are deduplicated by ID. Annotations are always
/// appended (assuming globally unique IDs across annotation files).
fn merge_coco_datasets(target: &mut CocoDataset, source: CocoDataset) {
    // Merge images (deduplicate by id)
    let existing_image_ids: HashSet<_> = target.images.iter().map(|i| i.id).collect();
    for image in source.images {
        if !existing_image_ids.contains(&image.id) {
            target.images.push(image);
        }
    }

    // Merge categories (deduplicate by id)
    let existing_cat_ids: HashSet<_> = target.categories.iter().map(|c| c.id).collect();
    for cat in source.categories {
        if !existing_cat_ids.contains(&cat.id) {
            target.categories.push(cat);
        }
    }

    // Merge annotations (always append - IDs should be globally unique)
    target.annotations.extend(source.annotations);

    // Merge licenses (deduplicate by id)
    let existing_license_ids: HashSet<_> = target.licenses.iter().map(|l| l.id).collect();
    for license in source.licenses {
        if !existing_license_ids.contains(&license.id) {
            target.licenses.push(license);
        }
    }

    // Take info from source if target has none
    if target.info.description.is_none() && source.info.description.is_some() {
        target.info = source.info;
    }
}

/// Convert a COCO image with annotations to an EdgeFirst Sample.
fn convert_coco_image_to_sample(
    image: &CocoImage,
    index: &CocoIndex,
    images_dir: &Path,
    include_masks: bool,
    include_images: bool,
    group: Option<&str>,
) -> Result<Sample, Error> {
    let sample_name = Path::new(&image.file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(String::from)
        .unwrap_or_else(|| image.file_name.clone());

    // Create annotations
    let annotations = index
        .annotations_for_image(image.id)
        .iter()
        .filter_map(|ann| {
            let label = index.label_name(ann.category_id)?;
            let label_index = index.label_index(ann.category_id);

            let box2d = coco_bbox_to_box2d(&ann.bbox, image.width, image.height);

            let mask = if include_masks {
                ann.segmentation
                    .as_ref()
                    .and_then(|seg| coco_segmentation_to_mask(seg, image.width, image.height).ok())
            } else {
                None
            };

            {
                let mut ann = Annotation::new();
                ann.set_name(Some(sample_name.clone()));
                ann.set_label(Some(label.to_string()));
                ann.set_label_index(label_index);
                ann.set_box2d(Some(box2d));
                ann.set_mask(mask);
                ann.set_group(group.map(String::from));
                Some(ann)
            }
        })
        .collect();

    // Create sample files
    let mut files = Vec::new();
    if include_images {
        if let Some(image_path) = find_image_file(images_dir, &image.file_name) {
            files.push(SampleFile::with_filename(
                FileType::Image.to_string(),
                image_path.to_string_lossy().to_string(),
            ));
        }
    }

    Ok(Sample {
        image_name: Some(sample_name),
        width: Some(image.width),
        height: Some(image.height),
        group: group.map(String::from),
        files,
        annotations,
        ..Default::default()
    })
}

/// Export Studio dataset to COCO format.
///
/// Downloads samples and annotations from Studio and converts to COCO format.
///
/// # Arguments
/// * `client` - Authenticated Studio client
/// * `dataset_id` - Source dataset in Studio
/// * `annotation_set_id` - Source annotation set
/// * `output_path` - Output file path (JSON or ZIP)
/// * `options` - Export options
/// * `progress` - Optional progress channel
///
/// # Returns
/// Number of annotations exported
pub async fn export_studio_to_coco(
    client: &Client,
    dataset_id: DatasetID,
    annotation_set_id: AnnotationSetID,
    output_path: impl AsRef<Path>,
    options: &CocoExportOptions,
    progress: Option<Sender<Progress>>,
) -> Result<usize, Error> {
    let output_path = output_path.as_ref();

    // Fetch samples from Studio with annotations
    let groups: Vec<String> = options.groups.clone();
    let annotation_types = [crate::AnnotationType::Box2d, crate::AnnotationType::Mask];

    // Fetch all samples
    let all_samples = client
        .samples(
            dataset_id.clone(),
            Some(annotation_set_id.clone()),
            &annotation_types,
            &groups,
            &[],
            progress.clone(),
        )
        .await?;

    // Convert to COCO format
    let mut builder = CocoDatasetBuilder::new();

    if let Some(info) = &options.info {
        builder = builder.info(info.clone());
    }

    for sample in &all_samples {
        let image_name = sample.image_name.as_deref().unwrap_or("unknown");
        let width = sample.width.unwrap_or(0);
        let height = sample.height.unwrap_or(0);

        // Use the image_name directly if it has an extension, otherwise add .jpg
        let file_name = if image_name.contains('.') {
            image_name.to_string()
        } else {
            format!("{}.jpg", image_name)
        };
        let image_id = builder.add_image(&file_name, width, height);

        for ann in &sample.annotations {
            if let Some(box2d) = ann.box2d() {
                let label = ann.label().map(|s| s.as_str()).unwrap_or("unknown");
                let category_id = builder.add_category(label, None);

                let bbox = box2d_to_coco_bbox(&box2d, width, height);

                let segmentation = if options.include_masks {
                    ann.mask().map(|mask| {
                        let coco_poly = mask_to_coco_polygon(&mask, width, height);
                        CocoSegmentation::Polygon(coco_poly)
                    })
                } else {
                    None
                };

                builder.add_annotation(image_id, category_id, bbox, segmentation);
            }
        }
    }

    let dataset = builder.build();
    let annotation_count = dataset.annotations.len();

    // Write output
    let writer = CocoWriter::with_options(CocoWriteOptions {
        compress: true,
        pretty: options.pretty_json,
    });

    if options.output_zip {
        // Download images and create ZIP
        let images = if options.include_images {
            download_images(client, &all_samples, progress.clone()).await?
        } else {
            vec![]
        };

        writer.write_zip(&dataset, images.into_iter(), output_path)?;
    } else {
        writer.write_json(&dataset, output_path)?;
    }

    Ok(annotation_count)
}

/// Download images for samples from their presigned URLs.
///
/// Returns a vector of (archive_path, image_data) pairs suitable for ZIP
/// creation.
async fn download_images(
    client: &Client,
    samples: &[Sample],
    progress: Option<Sender<Progress>>,
) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let mut result = Vec::with_capacity(samples.len());
    let total = samples.len();

    for (i, sample) in samples.iter().enumerate() {
        // Find image file URL
        let image_url = sample.files.iter().find_map(|f| {
            if f.file_type() == "image" {
                f.url()
            } else {
                None
            }
        });

        if let Some(url) = image_url {
            // Download the image
            match client.download(url).await {
                Ok(data) => {
                    // Build archive path from sample name
                    let name = sample.image_name.as_deref().unwrap_or("unknown");
                    let filename = if name.contains('.') {
                        format!("images/{}", name)
                    } else {
                        format!("images/{}.jpg", name)
                    };
                    result.push((filename, data));
                }
                Err(e) => {
                    // Log warning but continue with other images
                    log::warn!(
                        "Failed to download image for sample {:?}: {}",
                        sample.image_name,
                        e
                    );
                }
            }
        }

        // Update progress
        if let Some(ref p) = progress {
            let _ = p
                .send(Progress {
                    current: i + 1,
                    total,
                })
                .await;
        }
    }

    Ok(result)
}

/// Options for verifying a COCO import.
#[derive(Debug, Clone)]
pub struct CocoVerifyOptions {
    /// Include segmentation mask verification.
    pub verify_masks: bool,
    /// Group to verify (None = all groups).
    pub group: Option<String>,
}

impl Default for CocoVerifyOptions {
    fn default() -> Self {
        Self {
            verify_masks: true,
            group: None,
        }
    }
}

/// Result of a COCO annotation update operation.
#[derive(Debug, Clone)]
pub struct CocoUpdateResult {
    /// Total number of images in the COCO dataset.
    pub total_images: usize,
    /// Number of samples that were updated with new annotations.
    pub updated: usize,
    /// Number of COCO images not found in Studio (not updated).
    pub not_found: usize,
}

/// Options for updating annotations on existing samples.
#[derive(Debug, Clone)]
pub struct CocoUpdateOptions {
    /// Include segmentation masks in the update.
    pub include_masks: bool,
    /// Group name filter (None = match any group).
    pub group: Option<String>,
    /// Batch size for API calls.
    pub batch_size: usize,
    /// Maximum concurrent operations.
    pub concurrency: usize,
}

impl Default for CocoUpdateOptions {
    fn default() -> Self {
        Self {
            include_masks: true,
            group: None,
            batch_size: 100,
            concurrency: 64,
        }
    }
}

/// Update annotations on existing samples without re-uploading images.
///
/// This function reads COCO annotations and updates the annotations on samples
/// that already exist in Studio. It's useful for:
/// - Adding masks to samples that were imported without them
/// - Syncing updated annotations to Studio
///
/// **Note:** This does NOT upload images. Samples must already exist in Studio.
///
/// # Arguments
/// * `client` - Authenticated Studio client
/// * `coco_path` - Path to COCO annotation JSON file
/// * `dataset_id` - Target dataset in Studio
/// * `annotation_set_id` - Target annotation set
/// * `options` - Update options
/// * `progress` - Optional progress channel
///
/// # Returns
/// Update result with counts of updated and not-found samples.
pub async fn update_coco_annotations(
    client: &Client,
    coco_path: impl AsRef<Path>,
    dataset_id: DatasetID,
    annotation_set_id: AnnotationSetID,
    options: &CocoUpdateOptions,
    progress: Option<Sender<Progress>>,
) -> Result<CocoUpdateResult, Error> {
    use crate::{SampleID, api::ServerAnnotation};
    use std::collections::HashMap;

    let coco_path = coco_path.as_ref();

    // Read COCO annotations - when given a directory, read ALL annotation files
    let dataset = if coco_path.is_dir() {
        // Read all annotation files and merge into one dataset
        let datasets = read_coco_directory(coco_path, &CocoReadOptions::default())?;
        log::info!("Found {} annotation files in directory", datasets.len());

        // Merge all datasets, preserving group info by prefixing file_name
        let mut merged = CocoDataset::default();
        for (mut ds, group) in datasets {
            log::info!(
                "  - {} group: {} images, {} annotations",
                group,
                ds.images.len(),
                ds.annotations.len()
            );
            // Prefix file_name with group folder so infer_group_from_folder can extract it
            for image in &mut ds.images {
                if !image.file_name.contains('/') {
                    image.file_name = format!("{}2017/{}", group, image.file_name);
                }
            }
            merge_coco_datasets(&mut merged, ds);
        }
        merged
    } else if coco_path.extension().is_some_and(|e| e == "json") {
        let reader = CocoReader::new();
        reader.read_json(coco_path)?
    } else {
        return Err(Error::InvalidParameters(
            "COCO update requires a JSON annotation file or directory.".to_string(),
        ));
    };

    let total_images = dataset.images.len();

    if total_images == 0 {
        return Err(Error::MissingAnnotations(
            "No images found in COCO dataset".to_string(),
        ));
    }

    log::info!(
        "COCO dataset: {} images, {} annotations, {} categories",
        total_images,
        dataset.annotations.len(),
        dataset.categories.len()
    );

    // Query ALL existing samples from Studio to get their IDs and dimensions
    // (no group filter - we want to update annotations regardless of group)
    log::info!("Fetching existing samples from Studio...");
    let existing_samples = client
        .samples(
            dataset_id.clone(),
            Some(annotation_set_id.clone()),
            &[],
            &[], // No group filter - get all samples
            &[],
            progress.clone(), // Show progress while fetching
        )
        .await?;

    // Build a map of sample name -> (sample_id, width, height, group)
    let mut sample_info: HashMap<String, (SampleID, u32, u32, Option<String>)> = HashMap::new();
    for sample in &existing_samples {
        if let (Some(name), Some(id), Some(w), Some(h)) =
            (sample.name(), sample.id(), sample.width, sample.height)
        {
            sample_info.insert(name, (id, w, h, sample.group.clone()));
        }
    }

    log::info!(
        "Found {} existing samples in Studio with IDs and dimensions",
        sample_info.len()
    );

    // Build COCO index for efficient annotation lookup
    let coco_index = CocoIndex::from_dataset(&dataset);

    // Get existing labels and create any missing ones from COCO categories
    let existing_labels = client.labels(dataset_id.clone()).await?;
    let existing_label_names: std::collections::HashSet<String> = existing_labels
        .iter()
        .map(|l| l.name().to_string())
        .collect();

    // Find COCO categories that don't exist as labels in Studio
    let mut missing_labels: Vec<String> = Vec::new();
    for category in dataset.categories.iter() {
        if !existing_label_names.contains(&category.name) {
            missing_labels.push(category.name.clone());
        }
    }

    // Create missing labels
    if !missing_labels.is_empty() {
        log::info!(
            "Creating {} missing labels in Studio...",
            missing_labels.len()
        );
        for label_name in &missing_labels {
            client.add_label(dataset_id.clone(), label_name).await?;
        }
    }

    // Re-query labels to get their IDs after creation
    let labels = client.labels(dataset_id.clone()).await?;
    let label_map: HashMap<String, u64> = labels
        .iter()
        .map(|l| (l.name().to_string(), l.id()))
        .collect();

    log::info!(
        "Label map has {} entries for {} COCO categories",
        label_map.len(),
        dataset.categories.len()
    );

    // Collect sample IDs to update and annotations to add
    let mut sample_ids_to_update: Vec<SampleID> = Vec::new();
    let mut server_annotations: Vec<ServerAnnotation> = Vec::new();
    // Samples that need group updates: (sample_id, group_name)
    let mut samples_needing_group_update: Vec<(SampleID, String)> = Vec::new();
    let mut not_found = 0;
    let mut missing_label_count = 0;

    let annotation_set_id_u64: u64 = annotation_set_id.into();

    for coco_image in &dataset.images {
        let sample_name = std::path::Path::new(&coco_image.file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(String::from)
            .unwrap_or_else(|| coco_image.file_name.clone());

        // Extract expected group from file_name (e.g., "train2017/000000123.jpg" ->
        // "train")
        let expected_group = super::reader::infer_group_from_folder(&coco_image.file_name);

        // Check if sample exists in Studio
        if let Some((sample_id, width, height, current_group)) = sample_info.get(&sample_name) {
            let (sample_id, width, height) = (*sample_id, *width, *height);
            sample_ids_to_update.push(sample_id);

            // Check if group needs updating
            if let Some(ref expected) = expected_group {
                if expected_group != *current_group {
                    samples_needing_group_update.push((sample_id, expected.clone()));
                }
            }

            // Get annotations for this image
            let annotations = coco_index.annotations_for_image(coco_image.id);
            let image_id: u64 = sample_id.into();

            for coco_ann in annotations {
                // Get category name and label_id
                let category_name = coco_index
                    .categories
                    .get(&coco_ann.category_id)
                    .map(|c| c.name.as_str())
                    .unwrap_or("unknown");

                let label_id = label_map.get(category_name).copied();
                if label_id.is_none() {
                    missing_label_count += 1;
                }

                // Convert bounding box to server format (x, y, w, h normalized center-based)
                let box2d = coco_bbox_to_box2d(&coco_ann.bbox, width, height);

                // Convert mask to polygon string if enabled
                let polygon = if options.include_masks {
                    coco_ann
                        .segmentation
                        .as_ref()
                        .and_then(|seg| coco_segmentation_to_mask(seg, width, height).ok())
                        .map(|mask| mask_to_polygon_string(&mask))
                        .unwrap_or_default()
                } else {
                    String::new()
                };

                // Determine annotation type
                let annotation_type = if !polygon.is_empty() {
                    "seg".to_string()
                } else {
                    "box".to_string()
                };

                server_annotations.push(ServerAnnotation {
                    label_id,
                    label_index: None,
                    label_name: Some(category_name.to_string()),
                    annotation_type,
                    x: box2d.left() as f64,
                    y: box2d.top() as f64,
                    w: box2d.width() as f64,
                    h: box2d.height() as f64,
                    score: 1.0,
                    polygon,
                    image_id,
                    annotation_set_id: annotation_set_id_u64,
                    object_reference: None,
                });
            }
        } else {
            not_found += 1;
            log::debug!("Sample not found in Studio: {}", sample_name);
        }
    }

    let to_update = sample_ids_to_update.len();
    log::info!(
        "Updating {} samples ({} not found in Studio), {} annotations",
        to_update,
        not_found,
        server_annotations.len()
    );

    if missing_label_count > 0 {
        log::warn!(
            "{} annotations have missing label_id (category not found in label map)",
            missing_label_count
        );
    }

    if to_update == 0 {
        return Ok(CocoUpdateResult {
            total_images,
            updated: 0,
            not_found,
        });
    }

    // Send initial progress
    if let Some(ref tx) = progress {
        let _ = tx
            .send(Progress {
                current: 0,
                total: to_update,
            })
            .await;
    }

    // Step 1: Delete existing annotations for these samples
    log::info!(
        "Deleting existing annotations for {} samples...",
        sample_ids_to_update.len()
    );
    let annotation_types = if options.include_masks {
        vec!["box".to_string(), "seg".to_string()]
    } else {
        vec!["box".to_string()]
    };

    // Delete in batches to avoid overwhelming the server
    for batch in sample_ids_to_update.chunks(options.batch_size) {
        client
            .delete_annotations_bulk(annotation_set_id.clone(), &annotation_types, batch)
            .await?;
    }

    // Send progress after delete
    if let Some(ref tx) = progress {
        let _ = tx
            .send(Progress {
                current: to_update / 2,
                total: to_update,
            })
            .await;
    }

    // Step 2: Add new annotations in batches
    log::info!("Adding {} new annotations...", server_annotations.len());
    let mut added = 0;
    for batch in server_annotations.chunks(options.batch_size) {
        client
            .add_annotations_bulk(annotation_set_id, batch.to_vec())
            .await?;
        added += batch.len();
        log::debug!("Added {} annotations so far", added);
    }

    // Final progress update
    if let Some(ref tx) = progress {
        let _ = tx
            .send(Progress {
                current: to_update,
                total: to_update,
            })
            .await;
    }

    // Step 3: Update sample groups if needed using image.set_group_id API
    let groups_updated = if !samples_needing_group_update.is_empty() {
        log::info!(
            "Updating groups for {} samples...",
            samples_needing_group_update.len()
        );

        // Collect unique group names and get/create their IDs
        let unique_groups: HashSet<String> = samples_needing_group_update
            .iter()
            .map(|(_, group)| group.clone())
            .collect();

        let mut group_id_map: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        for group_name in unique_groups {
            match client
                .get_or_create_group(dataset_id.clone(), &group_name)
                .await
            {
                Ok(group_id) => {
                    group_id_map.insert(group_name, group_id);
                }
                Err(e) => {
                    log::warn!("Failed to get/create group '{}': {}", group_name, e);
                }
            }
        }

        // Update each sample's group
        let mut updated_count = 0;
        let mut failed_count = 0;
        for (sample_id, group_name) in &samples_needing_group_update {
            if let Some(&group_id) = group_id_map.get(group_name) {
                match client.set_sample_group_id(*sample_id, group_id).await {
                    Ok(_) => {
                        updated_count += 1;
                        if updated_count % 1000 == 0 {
                            log::debug!("Updated groups for {} samples so far", updated_count);
                        }
                    }
                    Err(e) => {
                        failed_count += 1;
                        if failed_count <= 5 {
                            log::warn!("Failed to update group for sample {:?}: {}", sample_id, e);
                        }
                    }
                }
            }
        }
        if failed_count > 5 {
            log::warn!("... and {} more group update failures", failed_count - 5);
        }
        log::info!(
            "Updated groups for {} samples ({} failed)",
            updated_count,
            failed_count
        );
        updated_count
    } else {
        0
    };

    log::info!(
        "Update complete: {} samples updated, {} not found, {} annotations added, {} groups updated",
        to_update,
        not_found,
        added,
        groups_updated
    );

    Ok(CocoUpdateResult {
        total_images,
        updated: to_update,
        not_found,
    })
}

/// Convert a Mask to a polygon string for the server API.
///
/// The server expects a 3D array format: `[[[x1,y1],[x2,y2],...], ...]`
/// where each point is an `[x, y]` pair. This matches how the server
/// parses polygons in `annotations_handler.go`:
/// ```go
/// var polygons [][][]float64
/// json.Unmarshal([]byte(ann.Polygon), &polygons)
/// ```
///
/// **Note:** This function filters out NaN and Infinity values which would
/// serialize as `null` in JSON and cause parsing failures on the server.
fn mask_to_polygon_string(mask: &crate::Mask) -> String {
    // Convert Vec<Vec<(f32, f32)>> to Vec<Vec<[f32; 2]>> for proper JSON
    // serialization Filter out any NaN or Infinity values which would become
    // "null" in JSON
    let polygons: Vec<Vec<[f32; 2]>> = mask
        .polygon
        .iter()
        .map(|ring| {
            ring.iter()
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .map(|&(x, y)| [x, y])
                .collect()
        })
        .filter(|ring: &Vec<[f32; 2]>| ring.len() >= 3) // Need at least 3 points for a valid polygon
        .collect();

    serde_json::to_string(&polygons).unwrap_or_default()
}

/// Compute COCO bounding box from mask polygon.
///
/// When the server doesn't return bounding box coordinates for segmentation
/// annotations, we compute them from the mask polygon bounds.
fn compute_bbox_from_mask(mask: &crate::Mask, width: u32, height: u32) -> Option<[f64; 4]> {
    if mask.polygon.is_empty() {
        return None;
    }

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for ring in &mask.polygon {
        for &(x, y) in ring {
            if x.is_finite() && y.is_finite() {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if min_x == f32::MAX || min_y == f32::MAX {
        return None;
    }

    // Convert normalized coordinates to COCO pixel coordinates [x, y, w, h]
    let x = (min_x * width as f32) as f64;
    let y = (min_y * height as f32) as f64;
    let w = ((max_x - min_x) * width as f32) as f64;
    let h = ((max_y - min_y) * height as f32) as f64;

    if w > 0.0 && h > 0.0 {
        Some([x, y, w, h])
    } else {
        None
    }
}

/// Verify a COCO dataset import against Studio data.
///
/// Compares the local COCO dataset against what's stored in Studio to verify:
/// - All images are present (no missing, no extras)
/// - All annotations are correct (using Hungarian matching)
/// - Bounding boxes match within tolerance
/// - Segmentation masks match (if enabled)
///
/// This does NOT download images - it only compares metadata and annotations.
///
/// # Arguments
/// * `client` - Authenticated Studio client
/// * `coco_path` - Path to local COCO annotation JSON file
/// * `dataset_id` - Dataset in Studio to verify against
/// * `annotation_set_id` - Annotation set in Studio to verify against
/// * `options` - Verification options
/// * `progress` - Optional progress channel
///
/// # Returns
/// Verification result with detailed comparison metrics.
pub async fn verify_coco_import(
    client: &Client,
    coco_path: impl AsRef<Path>,
    dataset_id: DatasetID,
    annotation_set_id: AnnotationSetID,
    options: &CocoVerifyOptions,
    progress: Option<Sender<Progress>>,
) -> Result<super::verify::VerificationResult, Error> {
    use super::{verify::*, writer::CocoDatasetBuilder};

    let coco_path = coco_path.as_ref();

    // Read local COCO dataset
    log::info!("Reading local COCO dataset from {:?}", coco_path);
    let (coco_dataset, inferred_group) = if coco_path.is_dir() {
        // Read all annotation files and merge into one dataset
        let datasets = read_coco_directory(coco_path, &CocoReadOptions::default())?;
        log::info!("Found {} annotation files in directory", datasets.len());

        let mut merged = CocoDataset::default();
        for (ds, group) in datasets {
            log::info!(
                "  - {} group: {} images, {} annotations",
                group,
                ds.images.len(),
                ds.annotations.len()
            );
            merge_coco_datasets(&mut merged, ds);
        }
        // When verifying entire directory, don't filter by group
        (merged, None)
    } else if coco_path.extension().is_some_and(|e| e == "json") {
        let reader = CocoReader::new();
        let dataset = reader.read_json(coco_path)?;
        let group = infer_group_from_filename(coco_path);
        (dataset, group)
    } else {
        return Err(Error::InvalidParameters(
            "COCO verification requires a JSON annotation file or directory.".to_string(),
        ));
    };

    // Determine group filter (only when verifying a single JSON file)
    let effective_group = options.group.clone().or(inferred_group);
    let groups: Vec<String> = effective_group
        .as_ref()
        .map(|g| vec![g.clone()])
        .unwrap_or_default();

    log::info!(
        "Local COCO: {} images, {} annotations",
        coco_dataset.images.len(),
        coco_dataset.annotations.len()
    );

    // Fetch samples from Studio with annotations
    log::info!("Fetching samples from Studio dataset {}...", dataset_id);
    let annotation_types = [crate::AnnotationType::Box2d, crate::AnnotationType::Mask];

    let studio_samples = client
        .samples(
            dataset_id.clone(),
            Some(annotation_set_id.clone()),
            &annotation_types,
            &groups,
            &[],
            progress.clone(),
        )
        .await?;

    let total_annotations: usize = studio_samples.iter().map(|s| s.annotations.len()).sum();
    log::info!(
        "Studio: {} samples, {} total annotations",
        studio_samples.len(),
        total_annotations
    );

    // Convert Studio samples to COCO format for comparison
    let mut builder = CocoDatasetBuilder::new();

    for sample in &studio_samples {
        let image_name = sample.image_name.as_deref().unwrap_or("unknown");
        let width = sample.width.unwrap_or(0);
        let height = sample.height.unwrap_or(0);

        // Use the image_name directly if it has an extension, otherwise add .jpg
        let file_name = if image_name.contains('.') {
            image_name.to_string()
        } else {
            format!("{}.jpg", image_name)
        };
        let image_id = builder.add_image(&file_name, width, height);

        for ann in &sample.annotations {
            // Get bbox from box2d if present, otherwise compute from mask
            let bbox = if let Some(box2d) = ann.box2d() {
                Some(box2d_to_coco_bbox(box2d, width, height))
            } else if let Some(mask) = ann.mask() {
                // Compute bbox from mask polygon bounds
                compute_bbox_from_mask(mask, width, height)
            } else {
                None
            };

            if let Some(bbox) = bbox {
                let label = ann.label().map(|s| s.as_str()).unwrap_or("unknown");
                let category_id = builder.add_category(label, None);

                let segmentation = if options.verify_masks {
                    ann.mask().map(|mask| {
                        let coco_poly = mask_to_coco_polygon(&mask, width, height);
                        CocoSegmentation::Polygon(coco_poly)
                    })
                } else {
                    None
                };

                builder.add_annotation(image_id, category_id, bbox, segmentation);
            }
        }
    }

    let studio_dataset = builder.build();

    // Build sample name sets for comparison
    let coco_names: HashSet<String> = coco_dataset
        .images
        .iter()
        .map(|img| {
            Path::new(&img.file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
                .unwrap_or_else(|| img.file_name.clone())
        })
        .collect();

    let studio_names: HashSet<String> = studio_samples.iter().filter_map(|s| s.name()).collect();

    let missing_images: Vec<String> = coco_names.difference(&studio_names).cloned().collect();
    let extra_images: Vec<String> = studio_names.difference(&coco_names).cloned().collect();

    // Validate bounding boxes
    log::info!("Validating bounding boxes...");
    let bbox_validation = validate_bboxes(&coco_dataset, &studio_dataset);

    // Validate masks if enabled
    log::info!("Validating segmentation masks...");
    let mask_validation = if options.verify_masks {
        validate_masks(&coco_dataset, &studio_dataset)
    } else {
        MaskValidationResult::new()
    };

    // Validate categories
    let category_validation = validate_categories(&coco_dataset, &studio_dataset);

    Ok(VerificationResult {
        coco_image_count: coco_dataset.images.len(),
        studio_image_count: studio_samples.len(),
        missing_images,
        extra_images,
        coco_annotation_count: coco_dataset.annotations.len(),
        studio_annotation_count: studio_dataset.annotations.len(),
        bbox_validation,
        mask_validation,
        category_validation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coco_import_options_default() {
        let options = CocoImportOptions::default();
        assert!(options.include_masks);
        assert!(options.include_images);
        assert!(options.group.is_none());
        assert_eq!(options.batch_size, 100);
        assert_eq!(options.concurrency, 64);
        assert!(options.resume);
    }

    #[test]
    fn test_coco_export_options_default() {
        let options = CocoExportOptions::default();
        assert!(options.groups.is_empty());
        assert!(options.include_masks);
        assert!(!options.include_images);
        assert!(!options.output_zip);
    }

    #[test]
    fn test_find_image_file() {
        // Test with non-existent directory - should return None
        let result = find_image_file(Path::new("/nonexistent"), "test.jpg");
        assert!(result.is_none());
    }

    #[test]
    fn test_convert_coco_image_to_sample() {
        let image = CocoImage {
            id: 1,
            width: 640,
            height: 480,
            file_name: "test.jpg".to_string(),
            ..Default::default()
        };

        let dataset = CocoDataset {
            images: vec![image.clone()],
            categories: vec![CocoCategory {
                id: 1,
                name: "person".to_string(),
                supercategory: None,
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [100.0, 50.0, 200.0, 150.0],
                area: 30000.0,
                iscrowd: 0,
                segmentation: None,
            }],
            ..Default::default()
        };

        let index = CocoIndex::from_dataset(&dataset);

        let sample = convert_coco_image_to_sample(
            &image,
            &index,
            Path::new("/tmp"),
            true,
            false, // Don't try to include images
            Some("train"),
        )
        .unwrap();

        assert_eq!(sample.image_name, Some("test".to_string()));
        assert_eq!(sample.width, Some(640));
        assert_eq!(sample.height, Some(480));
        assert_eq!(sample.group, Some("train".to_string()));
        assert_eq!(sample.annotations.len(), 1);
        assert_eq!(sample.annotations[0].label(), Some(&"person".to_string()));
    }

    #[test]
    fn test_mask_to_polygon_string() {
        // Create a simple triangle mask
        let mask = crate::Mask::new(vec![vec![(0.1, 0.2), (0.3, 0.4), (0.5, 0.6)]]);

        let result = mask_to_polygon_string(&mask);

        // Server expects 3D array format: [[[x1,y1],[x2,y2],...]]
        // NOT COCO format: [[x1,y1,x2,y2,...]]
        assert_eq!(result, "[[[0.1,0.2],[0.3,0.4],[0.5,0.6]]]");
    }

    #[test]
    fn test_mask_to_polygon_string_multiple_rings() {
        // Create a mask with two polygons (e.g., disjoint regions)
        // Each polygon needs at least 3 points to be valid
        let mask = crate::Mask::new(vec![
            vec![(0.1, 0.1), (0.2, 0.1), (0.15, 0.2)], // Triangle 1
            vec![(0.5, 0.5), (0.6, 0.5), (0.55, 0.6)], // Triangle 2
        ]);

        let result = mask_to_polygon_string(&mask);

        // Should produce two separate polygon rings
        assert_eq!(
            result,
            "[[[0.1,0.1],[0.2,0.1],[0.15,0.2]],[[0.5,0.5],[0.6,0.5],[0.55,0.6]]]"
        );
    }

    #[test]
    fn test_mask_to_polygon_string_filters_nan_values() {
        // Test that NaN values are filtered out
        let mask = crate::Mask::new(vec![vec![
            (0.1, 0.2),
            (f32::NAN, 0.4), // NaN value - should be filtered
            (0.3, 0.4),
            (0.5, 0.6),
        ]]);

        let result = mask_to_polygon_string(&mask);

        // NaN values should be filtered out, not serialized as "null"
        assert!(
            !result.contains("null"),
            "NaN values should be filtered out, got: {}",
            result
        );
        // Should have 3 valid points remaining
        assert_eq!(result, "[[[0.1,0.2],[0.3,0.4],[0.5,0.6]]]");
    }

    #[test]
    fn test_mask_to_polygon_string_filters_infinity() {
        // Test that Infinity values are filtered out
        let mask = crate::Mask::new(vec![vec![
            (0.1, 0.2),
            (f32::INFINITY, 0.4), // Infinity - should be filtered
            (0.3, 0.4),
            (0.5, 0.6),
        ]]);

        let result = mask_to_polygon_string(&mask);

        assert!(
            !result.contains("null"),
            "Infinity values should be filtered out"
        );
        assert_eq!(result, "[[[0.1,0.2],[0.3,0.4],[0.5,0.6]]]");
    }

    #[test]
    fn test_mask_to_polygon_string_too_few_points_after_filter() {
        // If filtering leaves fewer than 3 points, the ring should be dropped
        let mask = crate::Mask::new(vec![vec![
            (0.1, 0.2),
            (f32::NAN, 0.4),      // filtered
            (f32::NAN, f32::NAN), // filtered
        ]]);

        let result = mask_to_polygon_string(&mask);

        // Only 1 point remains, so the ring should be dropped
        assert_eq!(result, "[]");
    }
}
