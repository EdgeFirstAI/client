// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Streaming COCO JSON/ZIP writers.
//!
//! Provides efficient writing of COCO annotation files to JSON or ZIP archives.

use super::types::{
    CocoAnnotation, CocoCategory, CocoDataset, CocoImage, CocoInfo, CocoSegmentation,
};
use crate::Error;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
use zip::{CompressionMethod, write::SimpleFileOptions};

/// Options for COCO writing.
#[derive(Debug, Clone)]
pub struct CocoWriteOptions {
    /// Compress output (for ZIP).
    pub compress: bool,
    /// Pretty-print JSON with indentation.
    pub pretty: bool,
}

impl Default for CocoWriteOptions {
    fn default() -> Self {
        Self {
            compress: true,
            pretty: false,
        }
    }
}

/// COCO writer for generating JSON and ZIP files.
///
/// # Example
///
/// ```rust,no_run
/// use edgefirst_client::coco::{CocoDataset, CocoWriter};
///
/// let writer = CocoWriter::new();
/// let dataset = CocoDataset::default();
/// writer.write_json(&dataset, "annotations.json")?;
/// # Ok::<(), edgefirst_client::Error>(())
/// ```
pub struct CocoWriter {
    options: CocoWriteOptions,
}

impl CocoWriter {
    /// Create a new COCO writer with default options.
    pub fn new() -> Self {
        Self {
            options: CocoWriteOptions::default(),
        }
    }

    /// Create a new COCO writer with custom options.
    pub fn with_options(options: CocoWriteOptions) -> Self {
        Self { options }
    }

    /// Write COCO dataset to a JSON file.
    ///
    /// # Arguments
    /// * `dataset` - The COCO dataset to write
    /// * `path` - Output file path
    pub fn write_json<P: AsRef<Path>>(&self, dataset: &CocoDataset, path: P) -> Result<(), Error> {
        // Ensure parent directory exists
        if let Some(parent) = path.as_ref().parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(path.as_ref())?;
        let writer = BufWriter::with_capacity(64 * 1024, file);

        if self.options.pretty {
            serde_json::to_writer_pretty(writer, dataset)?;
        } else {
            serde_json::to_writer(writer, dataset)?;
        }

        Ok(())
    }

    /// Write COCO dataset to a ZIP file with images.
    ///
    /// Creates a ZIP archive with:
    /// - `annotations/instances.json` - The COCO annotations
    /// - Images at their original relative paths
    ///
    /// # Arguments
    /// * `dataset` - The COCO dataset to write
    /// * `images` - Iterator of `(filename, image_data)` pairs
    /// * `path` - Output ZIP file path
    pub fn write_zip<P: AsRef<Path>>(
        &self,
        dataset: &CocoDataset,
        images: impl Iterator<Item = (String, Vec<u8>)>,
        path: P,
    ) -> Result<(), Error> {
        // Ensure parent directory exists
        if let Some(parent) = path.as_ref().parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(path.as_ref())?;
        let mut zip = zip::ZipWriter::new(file);

        let options = if self.options.compress {
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
        } else {
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
        };

        // Write annotations
        zip.start_file("annotations/instances.json", options)?;
        let json = if self.options.pretty {
            serde_json::to_string_pretty(dataset)?
        } else {
            serde_json::to_string(dataset)?
        };
        zip.write_all(json.as_bytes())?;

        // Write images
        for (filename, data) in images {
            zip.start_file(&filename, options)?;
            zip.write_all(&data)?;
        }

        zip.finish()?;
        Ok(())
    }

    /// Write COCO dataset to a ZIP file with images from a source directory.
    ///
    /// # Arguments
    /// * `dataset` - The COCO dataset to write
    /// * `images_dir` - Directory containing source images
    /// * `path` - Output ZIP file path
    pub fn write_zip_from_dir<P: AsRef<Path>>(
        &self,
        dataset: &CocoDataset,
        images_dir: P,
        path: P,
    ) -> Result<(), Error> {
        let images_dir = images_dir.as_ref();

        // Collect image data
        let images = dataset.images.iter().filter_map(|img| {
            let img_path = images_dir.join(&img.file_name);
            std::fs::read(&img_path)
                .ok()
                .map(|data| (format!("images/{}", img.file_name), data))
        });

        self.write_zip(dataset, images, path)
    }

    /// Split a dataset by group and write each group to its own directory.
    ///
    /// Creates a directory structure like:
    /// ```text
    /// output_dir/
    /// ├── train/
    /// │   ├── annotations/instances_train.json
    /// │   └── images/
    /// │       └── *.jpg
    /// └── val/
    ///     ├── annotations/instances_val.json
    ///     └── images/
    ///         └── *.jpg
    /// ```
    ///
    /// # Arguments
    /// * `dataset` - The COCO dataset to split
    /// * `group_assignments` - Parallel array of group names for each image
    /// * `images_source` - Optional source directory containing images to copy
    /// * `output_dir` - Output root directory
    ///
    /// # Returns
    /// HashMap of group name → number of images written
    pub fn write_split_by_group<P: AsRef<Path>>(
        &self,
        dataset: &CocoDataset,
        group_assignments: &[String],
        images_source: Option<&Path>,
        output_dir: P,
    ) -> Result<std::collections::HashMap<String, usize>, Error> {
        use std::collections::{HashMap, HashSet};

        let output_dir = output_dir.as_ref();

        // Validate input
        if dataset.images.len() != group_assignments.len() {
            return Err(Error::CocoError(format!(
                "Image count ({}) does not match group assignment count ({})",
                dataset.images.len(),
                group_assignments.len()
            )));
        }

        // Build groups
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, group) in group_assignments.iter().enumerate() {
            groups.entry(group.clone()).or_default().push(idx);
        }

        let mut result = HashMap::new();

        for (group_name, image_indices) in &groups {
            // Create subdirectory structure
            let group_dir = output_dir.join(group_name);
            let annotations_dir = group_dir.join("annotations");
            let images_dir = group_dir.join("images");

            std::fs::create_dir_all(&annotations_dir)?;
            std::fs::create_dir_all(&images_dir)?;

            // Build subset dataset for this group
            let image_ids: HashSet<u64> = image_indices
                .iter()
                .map(|&idx| dataset.images[idx].id)
                .collect();

            let subset = CocoDataset {
                info: dataset.info.clone(),
                licenses: dataset.licenses.clone(),
                images: image_indices
                    .iter()
                    .map(|&idx| dataset.images[idx].clone())
                    .collect(),
                annotations: dataset
                    .annotations
                    .iter()
                    .filter(|ann| image_ids.contains(&ann.image_id))
                    .cloned()
                    .collect(),
                categories: dataset.categories.clone(),
            };

            // Write annotations JSON
            let ann_file = annotations_dir.join(format!("instances_{}.json", group_name));
            self.write_json(&subset, &ann_file)?;

            // Copy images if source provided
            if let Some(source) = images_source {
                for &idx in image_indices {
                    let image = &dataset.images[idx];
                    let src_path = source.join(&image.file_name);
                    let dst_path = images_dir.join(&image.file_name);

                    if src_path.exists() {
                        std::fs::copy(&src_path, &dst_path)?;
                    }
                }
            }

            result.insert(group_name.clone(), image_indices.len());
        }

        Ok(result)
    }

    /// Split a dataset by group and write each group to its own ZIP archive.
    ///
    /// Creates ZIP archives like:
    /// - `output_dir/train.zip` containing train split
    /// - `output_dir/val.zip` containing val split
    ///
    /// # Arguments
    /// * `dataset` - The COCO dataset to split
    /// * `group_assignments` - Parallel array of group names for each image
    /// * `images_source` - Optional source directory containing images to
    ///   include
    /// * `output_dir` - Output directory for ZIP files
    ///
    /// # Returns
    /// HashMap of group name → number of images written
    pub fn write_split_by_group_zip<P: AsRef<Path>>(
        &self,
        dataset: &CocoDataset,
        group_assignments: &[String],
        images_source: Option<&Path>,
        output_dir: P,
    ) -> Result<std::collections::HashMap<String, usize>, Error> {
        use std::collections::{HashMap, HashSet};

        let output_dir = output_dir.as_ref();
        std::fs::create_dir_all(output_dir)?;

        // Validate input
        if dataset.images.len() != group_assignments.len() {
            return Err(Error::CocoError(format!(
                "Image count ({}) does not match group assignment count ({})",
                dataset.images.len(),
                group_assignments.len()
            )));
        }

        // Build groups
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, group) in group_assignments.iter().enumerate() {
            groups.entry(group.clone()).or_default().push(idx);
        }

        let mut result = HashMap::new();

        for (group_name, image_indices) in &groups {
            // Build subset dataset for this group
            let image_ids: HashSet<u64> = image_indices
                .iter()
                .map(|&idx| dataset.images[idx].id)
                .collect();

            let subset = CocoDataset {
                info: dataset.info.clone(),
                licenses: dataset.licenses.clone(),
                images: image_indices
                    .iter()
                    .map(|&idx| dataset.images[idx].clone())
                    .collect(),
                annotations: dataset
                    .annotations
                    .iter()
                    .filter(|ann| image_ids.contains(&ann.image_id))
                    .cloned()
                    .collect(),
                categories: dataset.categories.clone(),
            };

            // Collect images if source provided
            let images: Vec<(String, Vec<u8>)> = if let Some(source) = images_source {
                image_indices
                    .iter()
                    .filter_map(|&idx| {
                        let image = &dataset.images[idx];
                        let src_path = source.join(&image.file_name);
                        std::fs::read(&src_path)
                            .ok()
                            .map(|data| (format!("images/{}", image.file_name), data))
                    })
                    .collect()
            } else {
                vec![]
            };

            // Write ZIP
            let zip_path = output_dir.join(format!("{}.zip", group_name));
            self.write_zip(&subset, images.into_iter(), &zip_path)?;

            result.insert(group_name.clone(), image_indices.len());
        }

        Ok(result)
    }
}

impl Default for CocoWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a COCO dataset.
///
/// Provides a convenient API for incrementally building a COCO dataset.
#[derive(Debug, Default)]
pub struct CocoDatasetBuilder {
    dataset: CocoDataset,
    next_image_id: u64,
    next_annotation_id: u64,
    next_category_id: u32,
}

impl CocoDatasetBuilder {
    /// Create a new dataset builder.
    pub fn new() -> Self {
        Self {
            dataset: CocoDataset::default(),
            next_image_id: 1,
            next_annotation_id: 1,
            next_category_id: 1,
        }
    }

    /// Set dataset info.
    pub fn info(mut self, info: CocoInfo) -> Self {
        self.dataset.info = info;
        self
    }

    /// Add a category, returning its ID.
    pub fn add_category(&mut self, name: &str, supercategory: Option<&str>) -> u32 {
        // Check if category already exists
        for cat in &self.dataset.categories {
            if cat.name == name {
                return cat.id;
            }
        }

        let id = self.next_category_id;
        self.next_category_id += 1;

        self.dataset.categories.push(CocoCategory {
            id,
            name: name.to_string(),
            supercategory: supercategory.map(String::from),
        });

        id
    }

    /// Add an image, returning its ID.
    pub fn add_image(&mut self, file_name: &str, width: u32, height: u32) -> u64 {
        let id = self.next_image_id;
        self.next_image_id += 1;

        self.dataset.images.push(CocoImage {
            id,
            width,
            height,
            file_name: file_name.to_string(),
            ..Default::default()
        });

        id
    }

    /// Add an annotation, returning its ID.
    pub fn add_annotation(
        &mut self,
        image_id: u64,
        category_id: u32,
        bbox: [f64; 4],
        segmentation: Option<CocoSegmentation>,
    ) -> u64 {
        let id = self.next_annotation_id;
        self.next_annotation_id += 1;

        let area = bbox[2] * bbox[3]; // Default area from bbox

        self.dataset.annotations.push(CocoAnnotation {
            id,
            image_id,
            category_id,
            bbox,
            area,
            iscrowd: 0,
            segmentation,
        });

        id
    }

    /// Build the final dataset.
    pub fn build(self) -> CocoDataset {
        self.dataset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_writer_default() {
        let writer = CocoWriter::new();
        assert!(writer.options.compress);
        assert!(!writer.options.pretty);
    }

    #[test]
    fn test_write_json() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.json");

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

        let writer = CocoWriter::new();
        writer.write_json(&dataset, &output_path).unwrap();

        // Verify file was created
        assert!(output_path.exists());

        // Read it back and verify
        let contents = std::fs::read_to_string(&output_path).unwrap();
        let restored: CocoDataset = serde_json::from_str(&contents).unwrap();
        assert_eq!(restored.images.len(), 1);
        assert_eq!(restored.annotations.len(), 1);
    }

    #[test]
    fn test_write_json_pretty() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_pretty.json");

        let dataset = CocoDataset::default();

        let writer = CocoWriter::with_options(CocoWriteOptions {
            pretty: true,
            compress: false,
        });
        writer.write_json(&dataset, &output_path).unwrap();

        let contents = std::fs::read_to_string(&output_path).unwrap();
        assert!(contents.contains('\n')); // Pretty-printed should have newlines
    }

    #[test]
    fn test_dataset_builder() {
        let mut builder = CocoDatasetBuilder::new();

        // Add categories
        let person_id = builder.add_category("person", Some("human"));
        let car_id = builder.add_category("car", Some("vehicle"));

        assert_eq!(person_id, 1);
        assert_eq!(car_id, 2);

        // Adding same category returns existing ID
        let person_id2 = builder.add_category("person", None);
        assert_eq!(person_id2, 1);

        // Add images
        let img1 = builder.add_image("image1.jpg", 640, 480);
        let img2 = builder.add_image("image2.jpg", 800, 600);

        assert_eq!(img1, 1);
        assert_eq!(img2, 2);

        // Add annotations
        let ann1 = builder.add_annotation(img1, person_id, [10.0, 20.0, 100.0, 80.0], None);
        let ann2 = builder.add_annotation(img1, car_id, [50.0, 60.0, 150.0, 100.0], None);

        assert_eq!(ann1, 1);
        assert_eq!(ann2, 2);

        // Build final dataset
        let dataset = builder.build();

        assert_eq!(dataset.categories.len(), 2);
        assert_eq!(dataset.images.len(), 2);
        assert_eq!(dataset.annotations.len(), 2);
    }

    #[test]
    fn test_write_zip() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.zip");

        let dataset = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 100,
                height: 100,
                file_name: "test.jpg".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        // Create a fake image
        let images = vec![("images/test.jpg".to_string(), vec![0xFF, 0xD8, 0xFF])];

        let writer = CocoWriter::new();
        writer
            .write_zip(&dataset, images.into_iter(), &output_path)
            .unwrap();

        // Verify ZIP was created
        assert!(output_path.exists());

        // Verify contents
        let file = std::fs::File::open(&output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        // Should contain annotations and image
        assert!(archive.by_name("annotations/instances.json").is_ok());
        assert!(archive.by_name("images/test.jpg").is_ok());
    }

    #[test]
    fn test_write_split_by_group() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("split_output");

        let dataset = CocoDataset {
            images: vec![
                CocoImage {
                    id: 1,
                    width: 640,
                    height: 480,
                    file_name: "train1.jpg".to_string(),
                    ..Default::default()
                },
                CocoImage {
                    id: 2,
                    width: 640,
                    height: 480,
                    file_name: "train2.jpg".to_string(),
                    ..Default::default()
                },
                CocoImage {
                    id: 3,
                    width: 800,
                    height: 600,
                    file_name: "val1.jpg".to_string(),
                    ..Default::default()
                },
            ],
            categories: vec![CocoCategory {
                id: 1,
                name: "person".to_string(),
                supercategory: None,
            }],
            annotations: vec![
                CocoAnnotation {
                    id: 1,
                    image_id: 1,
                    category_id: 1,
                    bbox: [10.0, 20.0, 100.0, 80.0],
                    ..Default::default()
                },
                CocoAnnotation {
                    id: 2,
                    image_id: 2,
                    category_id: 1,
                    bbox: [20.0, 30.0, 100.0, 80.0],
                    ..Default::default()
                },
                CocoAnnotation {
                    id: 3,
                    image_id: 3,
                    category_id: 1,
                    bbox: [30.0, 40.0, 100.0, 80.0],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let groups = vec!["train".to_string(), "train".to_string(), "val".to_string()];

        let writer = CocoWriter::new();
        let result = writer
            .write_split_by_group(&dataset, &groups, None, &output_dir)
            .unwrap();

        // Verify counts
        assert_eq!(result.get("train"), Some(&2));
        assert_eq!(result.get("val"), Some(&1));

        // Verify directory structure
        assert!(
            output_dir
                .join("train/annotations/instances_train.json")
                .exists()
        );
        assert!(
            output_dir
                .join("val/annotations/instances_val.json")
                .exists()
        );

        // Verify train JSON content
        let train_json =
            std::fs::read_to_string(output_dir.join("train/annotations/instances_train.json"))
                .unwrap();
        let train_data: CocoDataset = serde_json::from_str(&train_json).unwrap();
        assert_eq!(train_data.images.len(), 2);
        assert_eq!(train_data.annotations.len(), 2);

        // Verify val JSON content
        let val_json =
            std::fs::read_to_string(output_dir.join("val/annotations/instances_val.json")).unwrap();
        let val_data: CocoDataset = serde_json::from_str(&val_json).unwrap();
        assert_eq!(val_data.images.len(), 1);
        assert_eq!(val_data.annotations.len(), 1);
    }

    #[test]
    fn test_write_split_by_group_mismatch() {
        let dataset = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                ..Default::default()
            }],
            ..Default::default()
        };

        // Wrong number of group assignments
        let groups = vec!["train".to_string(), "val".to_string()];

        let writer = CocoWriter::new();
        let result =
            writer.write_split_by_group(&dataset, &groups, None, std::path::Path::new("/tmp/test"));

        assert!(result.is_err());
    }
}
