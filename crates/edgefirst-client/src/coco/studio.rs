// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! COCO import/export for EdgeFirst Studio.
//!
//! Provides high-level workflows for importing COCO datasets into Studio
//! and exporting Studio datasets to COCO format.

use super::convert::*;
use super::reader::CocoReader;
use super::types::*;
use super::writer::{CocoDatasetBuilder, CocoWriter, CocoWriteOptions};
use crate::{
    Annotation, AnnotationSetID, Client, DatasetID, Error, FileType, Progress, Sample, SampleFile,
};
use std::path::Path;
use tokio::sync::mpsc::Sender;

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
}

impl Default for CocoImportOptions {
    fn default() -> Self {
        Self {
            include_masks: true,
            include_images: true,
            group: None,
            batch_size: 100,
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
/// Reads COCO annotations and images, converts to EdgeFirst format,
/// and uploads to Studio using the bulk API.
///
/// # Arguments
/// * `client` - Authenticated Studio client
/// * `coco_path` - Path to COCO folder or ZIP (containing annotations and images)
/// * `dataset_id` - Target dataset in Studio
/// * `annotation_set_id` - Target annotation set
/// * `options` - Import options
/// * `progress` - Optional progress channel
///
/// # Returns
/// Number of samples imported
pub async fn import_coco_to_studio(
    client: &Client,
    coco_path: impl AsRef<Path>,
    dataset_id: DatasetID,
    annotation_set_id: AnnotationSetID,
    options: &CocoImportOptions,
    progress: Option<Sender<Progress>>,
) -> Result<usize, Error> {
    let coco_path = coco_path.as_ref();

    // Detect if it's a directory or ZIP
    let (dataset, images_source) = if coco_path.is_dir() {
        // Look for annotation file
        let ann_path = find_annotation_file(coco_path)?;
        let reader = CocoReader::new();
        let dataset = reader.read_json(&ann_path)?;
        (dataset, ImageSource::Directory(coco_path.to_path_buf()))
    } else if coco_path.extension().is_some_and(|e| e == "zip") {
        let reader = CocoReader::new();
        let dataset = reader.read_annotations_zip(coco_path)?;
        (dataset, ImageSource::Zip(coco_path.to_path_buf()))
    } else {
        // Assume it's a JSON file directly
        let reader = CocoReader::new();
        let dataset = reader.read_json(coco_path)?;
        let parent = coco_path.parent().unwrap_or(Path::new("."));
        (dataset, ImageSource::Directory(parent.to_path_buf()))
    };

    let index = CocoIndex::from_dataset(&dataset);
    let total_images = dataset.images.len();

    if let Some(ref p) = progress {
        let _ = p.send(Progress { current: 0, total: total_images }).await;
    }

    // Convert and upload in batches
    let mut imported = 0;

    for (batch_idx, batch) in dataset.images.chunks(options.batch_size).enumerate() {
        let mut samples = Vec::with_capacity(batch.len());

        for image in batch {
            let sample = convert_coco_image_to_sample(
                image,
                &index,
                &images_source,
                options.include_masks,
                options.include_images,
                options.group.as_deref(),
            )?;
            samples.push(sample);
        }

        // Upload batch
        client
            .populate_samples(
                dataset_id.clone(),
                Some(annotation_set_id.clone()),
                samples,
                progress.clone(),
            )
            .await?;

        imported += batch.len();

        // Update progress
        if let Some(ref p) = progress {
            let current = (batch_idx + 1) * options.batch_size;
            let _ = p.send(Progress { current: current.min(total_images), total: total_images }).await;
        }
    }

    Ok(imported)
}

/// Image source for import.
enum ImageSource {
    Directory(std::path::PathBuf),
    Zip(std::path::PathBuf),
}

/// Find the annotation file in a COCO directory.
fn find_annotation_file(dir: &Path) -> Result<std::path::PathBuf, Error> {
    // Common COCO annotation paths
    let candidates = [
        dir.join("annotations").join("instances_train2017.json"),
        dir.join("annotations").join("instances_val2017.json"),
        dir.join("annotations/instances.json"),
        dir.join("instances.json"),
    ];

    for path in &candidates {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    // Look for any instances*.json file
    for entry in walkdir::WalkDir::new(dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let name = entry.file_name().to_string_lossy();
        if name.starts_with("instances") && name.ends_with(".json") {
            return Ok(entry.path().to_path_buf());
        }
    }

    Err(Error::MissingAnnotations(format!(
        "No COCO annotation file found in {}",
        dir.display()
    )))
}

/// Convert a COCO image with annotations to an EdgeFirst Sample.
fn convert_coco_image_to_sample(
    image: &CocoImage,
    index: &CocoIndex,
    images_source: &ImageSource,
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
                ann.segmentation.as_ref().and_then(|seg| {
                    coco_segmentation_to_mask(seg, image.width, image.height).ok()
                })
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
        let image_url = match images_source {
            ImageSource::Directory(dir) => {
                // Look for image in common locations
                let candidates = [
                    dir.join(&image.file_name),
                    dir.join("images").join(&image.file_name),
                    dir.join("train2017").join(&image.file_name),
                    dir.join("val2017").join(&image.file_name),
                ];

                candidates
                    .iter()
                    .find(|p| p.exists())
                    .map(|p| format!("file://{}", p.display()))
            }
            ImageSource::Zip(zip_path) => {
                // Reference image in ZIP
                Some(format!(
                    "zip://{}#{}",
                    zip_path.display(),
                    image.file_name
                ))
            }
        };

        if let Some(url) = image_url {
            files.push(SampleFile::with_url(FileType::Image.to_string(), url));
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

    // Fetch samples from Studio
    let groups: Vec<String> = options.groups.clone();
    let file_types: Vec<FileType> = vec![FileType::Image];
    let annotation_types = vec![crate::AnnotationType::Box2d];

    // Fetch all samples
    let all_samples = client
        .samples(
            dataset_id.clone(),
            Some(annotation_set_id.clone()),
            &annotation_types,
            &groups,
            &file_types,
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

        let image_id = builder.add_image(
            &format!("{}.jpg", image_name),
            width,
            height,
        );

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
/// Returns a vector of (archive_path, image_data) pairs suitable for ZIP creation.
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
                    log::warn!("Failed to download image for sample {:?}: {}", sample.image_name, e);
                }
            }
        }

        // Update progress
        if let Some(ref p) = progress {
            let _ = p.send(Progress { current: i + 1, total }).await;
        }
    }

    Ok(result)
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
        let source = ImageSource::Directory(std::path::PathBuf::from("/tmp"));

        let sample = convert_coco_image_to_sample(
            &image,
            &index,
            &source,
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
}
