// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Streaming COCO JSON/ZIP readers.
//!
//! Provides memory-efficient reading of COCO annotation files from JSON files
//! or ZIP archives without requiring full extraction.

use super::types::*;
use crate::Error;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Options for COCO reading.
#[derive(Debug, Clone, Default)]
pub struct CocoReadOptions {
    /// If true, validate all annotations during reading.
    pub validate: bool,
    /// Maximum number of images to read (0 = unlimited).
    pub max_images: usize,
    /// Filter by category names (empty = all).
    pub category_filter: Vec<String>,
}

/// Streaming COCO reader for large datasets.
///
/// Supports reading from JSON files and ZIP archives.
///
/// # Example
///
/// ```rust,no_run
/// use edgefirst_client::coco::CocoReader;
///
/// let reader = CocoReader::new();
/// let dataset = reader.read_json("annotations/instances_val2017.json")?;
/// println!("Loaded {} images", dataset.images.len());
/// # Ok::<(), edgefirst_client::Error>(())
/// ```
pub struct CocoReader {
    options: CocoReadOptions,
}

impl CocoReader {
    /// Create a new COCO reader with default options.
    pub fn new() -> Self {
        Self {
            options: CocoReadOptions::default(),
        }
    }

    /// Create a new COCO reader with custom options.
    pub fn with_options(options: CocoReadOptions) -> Self {
        Self { options }
    }

    /// Read COCO dataset from a JSON file.
    ///
    /// # Arguments
    /// * `path` - Path to the COCO JSON annotation file
    ///
    /// # Returns
    /// Parsed `CocoDataset` structure
    pub fn read_json<P: AsRef<Path>>(&self, path: P) -> Result<CocoDataset, Error> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::with_capacity(64 * 1024, file);
        let dataset: CocoDataset = serde_json::from_reader(reader)?;

        if self.options.validate {
            validate_dataset(&dataset)?;
        }

        Ok(self.apply_filters(dataset))
    }

    /// Read COCO annotations from a ZIP file.
    ///
    /// Looks for annotation JSON files in standard COCO locations:
    /// - `annotations/instances_*.json`
    /// - `annotations/*.json`
    /// - Root level `*.json` files
    ///
    /// # Arguments
    /// * `path` - Path to the ZIP archive containing annotations
    ///
    /// # Returns
    /// Merged `CocoDataset` from all annotation files found
    pub fn read_annotations_zip<P: AsRef<Path>>(&self, path: P) -> Result<CocoDataset, Error> {
        let file = File::open(path.as_ref())?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut merged = CocoDataset::default();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();

            // Only process JSON files containing annotations
            if name.ends_with(".json") && name.contains("instances") {
                let mut contents = String::new();
                entry.read_to_string(&mut contents)?;

                let dataset: CocoDataset = serde_json::from_str(&contents)?;
                merge_datasets(&mut merged, dataset);
            }
        }

        if self.options.validate {
            validate_dataset(&merged)?;
        }

        Ok(self.apply_filters(merged))
    }

    /// List image files in a COCO ZIP or folder.
    ///
    /// # Arguments
    /// * `path` - Path to COCO images folder or ZIP archive
    ///
    /// # Returns
    /// Vector of `(relative_path, absolute_path)` for each image
    pub fn list_images<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<(String, std::path::PathBuf)>, Error> {
        let path = path.as_ref();
        let mut images = Vec::new();

        if path.is_dir() {
            // Walk directory
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let filename = entry.file_name().to_string_lossy().to_lowercase();
                if filename.ends_with(".jpg")
                    || filename.ends_with(".jpeg")
                    || filename.ends_with(".png")
                {
                    let rel_path = entry
                        .path()
                        .strip_prefix(path)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .to_string();
                    images.push((rel_path, entry.path().to_path_buf()));
                }
            }
        } else if path.extension().is_some_and(|e| e == "zip") {
            // List from ZIP
            let file = File::open(path)?;
            let mut archive = zip::ZipArchive::new(file)?;

            for i in 0..archive.len() {
                let entry = archive.by_index(i)?;
                let name = entry.name().to_string();
                let name_lower = name.to_lowercase();

                if !entry.is_dir()
                    && (name_lower.ends_with(".jpg")
                        || name_lower.ends_with(".jpeg")
                        || name_lower.ends_with(".png"))
                {
                    images.push((name.clone(), path.join(&name)));
                }
            }
        }

        Ok(images)
    }

    /// Read a single image from a ZIP archive.
    ///
    /// # Arguments
    /// * `zip_path` - Path to the ZIP archive
    /// * `image_name` - Name of the image file within the archive
    ///
    /// # Returns
    /// Raw image bytes
    pub fn read_image_from_zip<P: AsRef<Path>>(
        &self,
        zip_path: P,
        image_name: &str,
    ) -> Result<Vec<u8>, Error> {
        let file = File::open(zip_path.as_ref())?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut entry = archive.by_name(image_name)?;
        let mut buffer = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    /// Apply filters from options to the dataset.
    fn apply_filters(&self, mut dataset: CocoDataset) -> CocoDataset {
        // Apply max_images filter
        if self.options.max_images > 0 && dataset.images.len() > self.options.max_images {
            let image_ids: HashSet<_> = dataset
                .images
                .iter()
                .take(self.options.max_images)
                .map(|i| i.id)
                .collect();

            dataset.images.truncate(self.options.max_images);
            dataset
                .annotations
                .retain(|a| image_ids.contains(&a.image_id));
        }

        // Apply category filter
        if !self.options.category_filter.is_empty() {
            let category_ids: HashSet<_> = dataset
                .categories
                .iter()
                .filter(|c| self.options.category_filter.contains(&c.name))
                .map(|c| c.id)
                .collect();

            dataset
                .categories
                .retain(|c| self.options.category_filter.contains(&c.name));
            dataset
                .annotations
                .retain(|a| category_ids.contains(&a.category_id));
        }

        dataset
    }
}

impl Default for CocoReader {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate a COCO dataset for consistency.
fn validate_dataset(dataset: &CocoDataset) -> Result<(), Error> {
    let image_ids: HashSet<_> = dataset.images.iter().map(|i| i.id).collect();
    let category_ids: HashSet<_> = dataset.categories.iter().map(|c| c.id).collect();

    for ann in &dataset.annotations {
        if !image_ids.contains(&ann.image_id) {
            return Err(Error::CocoError(format!(
                "Annotation {} references non-existent image_id {}",
                ann.id, ann.image_id
            )));
        }

        if !category_ids.contains(&ann.category_id) {
            return Err(Error::CocoError(format!(
                "Annotation {} references non-existent category_id {}",
                ann.id, ann.category_id
            )));
        }

        // Validate bbox
        if ann.bbox[2] <= 0.0 || ann.bbox[3] <= 0.0 {
            return Err(Error::CocoError(format!(
                "Annotation {} has invalid bbox dimensions",
                ann.id
            )));
        }
    }

    Ok(())
}

/// Merge a source dataset into a target dataset.
fn merge_datasets(target: &mut CocoDataset, source: CocoDataset) {
    // Take info if not set
    if target.info.description.is_none() {
        target.info = source.info;
    }

    // Merge images (deduplicate by id)
    let existing_ids: HashSet<_> = target.images.iter().map(|i| i.id).collect();
    for image in source.images {
        if !existing_ids.contains(&image.id) {
            target.images.push(image);
        }
    }

    // Merge categories (deduplicate by id)
    let existing_cats: HashSet<_> = target.categories.iter().map(|c| c.id).collect();
    for cat in source.categories {
        if !existing_cats.contains(&cat.id) {
            target.categories.push(cat);
        }
    }

    // Merge annotations (always append - IDs should be globally unique)
    target.annotations.extend(source.annotations);

    // Merge licenses
    let existing_licenses: HashSet<_> = target.licenses.iter().map(|l| l.id).collect();
    for lic in source.licenses {
        if !existing_licenses.contains(&lic.id) {
            target.licenses.push(lic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_default() {
        let reader = CocoReader::new();
        assert!(!reader.options.validate);
        assert_eq!(reader.options.max_images, 0);
        assert!(reader.options.category_filter.is_empty());
    }

    #[test]
    fn test_reader_with_options() {
        let options = CocoReadOptions {
            validate: true,
            max_images: 100,
            category_filter: vec!["person".to_string()],
        };
        let reader = CocoReader::with_options(options.clone());
        assert!(reader.options.validate);
        assert_eq!(reader.options.max_images, 100);
    }

    #[test]
    fn test_validate_dataset_valid() {
        let dataset = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 640,
                height: 480,
                file_name: "test.jpg".to_string(),
                ..Default::default()
            }],
            categories: vec![CocoCategory {
                id: 1,
                name: "person".to_string(),
                supercategory: None,
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [10.0, 20.0, 100.0, 80.0],
                area: 8000.0,
                iscrowd: 0,
                segmentation: None,
            }],
            ..Default::default()
        };

        assert!(validate_dataset(&dataset).is_ok());
    }

    #[test]
    fn test_validate_dataset_missing_image() {
        let dataset = CocoDataset {
            images: vec![],
            categories: vec![CocoCategory {
                id: 1,
                name: "person".to_string(),
                supercategory: None,
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 999, // Non-existent
                category_id: 1,
                bbox: [10.0, 20.0, 100.0, 80.0],
                ..Default::default()
            }],
            ..Default::default()
        };

        assert!(validate_dataset(&dataset).is_err());
    }

    #[test]
    fn test_merge_datasets() {
        let mut target = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 640,
                height: 480,
                file_name: "img1.jpg".to_string(),
                ..Default::default()
            }],
            categories: vec![CocoCategory {
                id: 1,
                name: "person".to_string(),
                supercategory: None,
            }],
            annotations: vec![],
            ..Default::default()
        };

        let source = CocoDataset {
            images: vec![
                CocoImage {
                    id: 1, // Duplicate - should not be added
                    width: 640,
                    height: 480,
                    file_name: "img1.jpg".to_string(),
                    ..Default::default()
                },
                CocoImage {
                    id: 2, // New - should be added
                    width: 800,
                    height: 600,
                    file_name: "img2.jpg".to_string(),
                    ..Default::default()
                },
            ],
            categories: vec![CocoCategory {
                id: 2,
                name: "car".to_string(),
                supercategory: None,
            }],
            annotations: vec![],
            ..Default::default()
        };

        merge_datasets(&mut target, source);

        assert_eq!(target.images.len(), 2);
        assert_eq!(target.categories.len(), 2);
    }

    #[test]
    fn test_apply_max_images_filter() {
        let reader = CocoReader::with_options(CocoReadOptions {
            max_images: 2,
            ..Default::default()
        });

        let dataset = CocoDataset {
            images: vec![
                CocoImage {
                    id: 1,
                    ..Default::default()
                },
                CocoImage {
                    id: 2,
                    ..Default::default()
                },
                CocoImage {
                    id: 3,
                    ..Default::default()
                },
            ],
            annotations: vec![
                CocoAnnotation {
                    id: 1,
                    image_id: 1,
                    ..Default::default()
                },
                CocoAnnotation {
                    id: 2,
                    image_id: 2,
                    ..Default::default()
                },
                CocoAnnotation {
                    id: 3,
                    image_id: 3,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let filtered = reader.apply_filters(dataset);
        assert_eq!(filtered.images.len(), 2);
        assert_eq!(filtered.annotations.len(), 2);
    }
}
