//! EdgeFirst Dataset Format utilities.
//!
//! This module provides tools for working with the EdgeFirst Dataset Format
//! as documented in DATASET_FORMAT.md. It enables:
//!
//! - Reading and resolving file paths from Arrow annotation files
//! - Generating Arrow files from folders of images (with null annotations)
//! - Validating dataset directory structures
//! - (Future) Converting from other formats (COCO, DarkNet, YOLO, etc.)
//!
//! # EdgeFirst Dataset Format
//!
//! A dataset in EdgeFirst format consists of:
//! - An Arrow file (`{dataset_name}.arrow`) containing annotation metadata
//! - A sensor container directory (`{dataset_name}/`) with image/sensor files
//!
//! ## Supported Structures
//!
//! **Sequence-based** (frame column is not null):
//! ```text
//! dataset_name/
//! ├── dataset_name.arrow
//! └── dataset_name/
//!     └── sequence_name/
//!         ├── sequence_name_001.camera.jpeg
//!         └── sequence_name_002.camera.jpeg
//! ```
//!
//! **Image-based** (frame column is null):
//! ```text
//! dataset_name/
//! ├── dataset_name.arrow
//! └── dataset_name/
//!     ├── image1.jpg
//!     └── image2.png
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use edgefirst_client::format::{resolve_arrow_files, validate_dataset_structure};
//! use std::path::Path;
//!
//! // Resolve all files referenced by an Arrow file
//! let arrow_path = Path::new("my_dataset/my_dataset.arrow");
//! let files = resolve_arrow_files(arrow_path)?;
//! for (name, path) in &files {
//!     println!("{}: {:?}", name, path);
//! }
//!
//! // Validate the dataset structure
//! let issues = validate_dataset_structure(Path::new("my_dataset"))?;
//! if !issues.is_empty() {
//!     for issue in &issues {
//!         eprintln!("Warning: {}", issue);
//!     }
//! }
//! # Ok::<(), edgefirst_client::Error>(())
//! ```

use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::Error;

/// Image file extensions supported by EdgeFirst.
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg",
    "jpeg",
    "png",
    "camera.jpeg",
    "camera.png",
    "camera.jpg",
];

/// Resolve all file paths referenced by an Arrow annotation file.
///
/// Reads the Arrow file and extracts the `name` and `frame` columns to
/// determine which image files are referenced. Returns a map from sample
/// name to the expected relative file path within the sensor container.
///
/// # Arguments
///
/// * `arrow_path` - Path to the Arrow annotation file
///
/// # Returns
///
/// A map from sample name (e.g., "deer_001") to relative file path within
/// the sensor container (e.g., "deer/deer_001.camera.jpeg").
///
/// # Errors
///
/// Returns an error if:
/// * Arrow file cannot be read
/// * Arrow file is missing required columns
/// * Arrow file has invalid data types
///
/// # Example
///
/// ```rust,no_run
/// use edgefirst_client::format::resolve_arrow_files;
/// use std::path::Path;
///
/// let arrow_path = Path::new("dataset/dataset.arrow");
/// let files = resolve_arrow_files(arrow_path)?;
///
/// for (name, relative_path) in &files {
///     println!("Sample '{}' -> {:?}", name, relative_path);
/// }
/// # Ok::<(), edgefirst_client::Error>(())
/// ```
#[cfg(feature = "polars")]
pub fn resolve_arrow_files(arrow_path: &Path) -> Result<HashMap<String, PathBuf>, Error> {
    use polars::prelude::*;

    let mut file = File::open(arrow_path).map_err(|e| {
        Error::InvalidParameters(format!("Cannot open Arrow file {:?}: {}", arrow_path, e))
    })?;

    let df = IpcReader::new(&mut file).finish().map_err(|e| {
        Error::InvalidParameters(format!("Failed to read Arrow file {:?}: {}", arrow_path, e))
    })?;

    // Get the name column (required)
    let names = df
        .column("name")
        .map_err(|e| Error::InvalidParameters(format!("Missing 'name' column: {}", e)))?
        .str()
        .map_err(|e| Error::InvalidParameters(format!("Invalid 'name' column type: {}", e)))?;

    // Get the frame column (optional - determines sequence vs standalone)
    let frames = df.column("frame").ok();

    let mut result = HashMap::new();

    for idx in 0..df.height() {
        // Extract sample name
        let name = match names.get(idx) {
            Some(n) => n.to_string(),
            None => continue, // Skip null names
        };

        // Skip if we've already processed this sample name
        if result.contains_key(&name) {
            continue;
        }

        // Check if this is a sequence sample (frame is not null)
        let frame = frames.and_then(|col| {
            // Try as u64 first, then u32
            col.u64()
                .ok()
                .and_then(|s| s.get(idx))
                .or_else(|| col.u32().ok().and_then(|s| s.get(idx).map(|v| v as u64)))
        });

        // Build the relative path based on whether this is a sequence or standalone
        let relative_path = if let Some(frame_num) = frame {
            // Sequence: name/name_frame.camera.jpeg
            // The name column contains the sequence name
            PathBuf::from(&name).join(format!("{}_{:03}.camera.jpeg", name, frame_num))
        } else {
            // Standalone: name.jpg (or similar - we'll resolve actual extension later)
            PathBuf::from(format!("{}.camera.jpeg", name))
        };

        result.insert(name, relative_path);
    }

    Ok(result)
}

/// Information about a resolved sample file.
#[derive(Debug, Clone)]
pub struct ResolvedFile {
    /// Sample name from the Arrow file
    pub name: String,
    /// Frame number (None for standalone images)
    pub frame: Option<u64>,
    /// Actual file path on disk (if found)
    pub path: Option<PathBuf>,
    /// Expected relative path within sensor container
    pub expected_path: PathBuf,
}

/// Resolve Arrow file references against actual files in a sensor container.
///
/// This function reads an Arrow file, extracts sample references, and attempts
/// to match them against actual files in the sensor container directory.
///
/// # Arguments
///
/// * `arrow_path` - Path to the Arrow annotation file
/// * `sensor_container` - Path to the sensor container directory
///
/// # Returns
///
/// A list of resolved files with match information.
///
/// # Example
///
/// ```rust,no_run
/// use edgefirst_client::format::resolve_files_with_container;
/// use std::path::Path;
///
/// let resolved = resolve_files_with_container(
///     Path::new("dataset/dataset.arrow"),
///     Path::new("dataset/dataset"),
/// )?;
///
/// for file in &resolved {
///     match &file.path {
///         Some(p) => println!("Found: {} -> {:?}", file.name, p),
///         None => println!("Missing: {} (expected {:?})", file.name, file.expected_path),
///     }
/// }
/// # Ok::<(), edgefirst_client::Error>(())
/// ```
#[cfg(feature = "polars")]
pub fn resolve_files_with_container(
    arrow_path: &Path,
    sensor_container: &Path,
) -> Result<Vec<ResolvedFile>, Error> {
    use polars::prelude::*;

    let mut file = File::open(arrow_path).map_err(|e| {
        Error::InvalidParameters(format!("Cannot open Arrow file {:?}: {}", arrow_path, e))
    })?;

    let df = IpcReader::new(&mut file).finish().map_err(|e| {
        Error::InvalidParameters(format!("Failed to read Arrow file {:?}: {}", arrow_path, e))
    })?;

    // Build an index of all files in the sensor container
    let file_index = build_file_index(sensor_container)?;

    // Get the name column (required)
    let names = df
        .column("name")
        .map_err(|e| Error::InvalidParameters(format!("Missing 'name' column: {}", e)))?
        .str()
        .map_err(|e| Error::InvalidParameters(format!("Invalid 'name' column type: {}", e)))?;

    // Get the frame column (optional)
    let frames = df.column("frame").ok();

    let mut result = Vec::new();
    let mut seen_samples: HashMap<String, bool> = HashMap::new();

    for idx in 0..df.height() {
        let name = match names.get(idx) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Create unique key for deduplication (name + frame)
        let frame = frames.and_then(|col| {
            col.u64()
                .ok()
                .and_then(|s| s.get(idx))
                .or_else(|| col.u32().ok().and_then(|s| s.get(idx).map(|v| v as u64)))
        });

        let sample_key = match frame {
            Some(f) => format!("{}_{}", name, f),
            None => name.clone(),
        };

        // Skip duplicates
        if seen_samples.contains_key(&sample_key) {
            continue;
        }
        seen_samples.insert(sample_key.clone(), true);

        // Build expected path and try to find actual file
        let expected_path = if let Some(frame_num) = frame {
            PathBuf::from(&name).join(format!("{}_{:03}.camera.jpeg", name, frame_num))
        } else {
            PathBuf::from(format!("{}.camera.jpeg", name))
        };

        // Try to find the actual file using flexible matching
        let actual_path = find_matching_file(&file_index, &name, frame);

        result.push(ResolvedFile {
            name,
            frame,
            path: actual_path,
            expected_path,
        });
    }

    Ok(result)
}

/// Build an index of all files in a directory for fast lookup.
fn build_file_index(root: &Path) -> Result<HashMap<String, PathBuf>, Error> {
    let mut index = HashMap::new();

    if !root.exists() {
        return Ok(index);
    }

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path().to_path_buf();
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // Index by full filename
            index.insert(filename.to_lowercase(), path.clone());

            // Also index by stem (without extension) for flexible matching
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                // Handle double extensions like .camera.jpeg
                let clean_stem = stem.strip_suffix(".camera").unwrap_or(stem).to_lowercase();
                index.entry(clean_stem).or_insert_with(|| path.clone());
            }
        }
    }

    Ok(index)
}

/// Find a matching file in the index using flexible matching.
fn find_matching_file(
    index: &HashMap<String, PathBuf>,
    name: &str,
    frame: Option<u64>,
) -> Option<PathBuf> {
    let search_key = match frame {
        Some(f) => format!("{}_{:03}", name, f).to_lowercase(),
        None => name.to_lowercase(),
    };

    // Try exact filename match first
    for ext in IMAGE_EXTENSIONS {
        let key = format!("{}.{}", search_key, ext);
        if let Some(path) = index.get(&key) {
            return Some(path.clone());
        }
    }

    // Try stem match
    if let Some(path) = index.get(&search_key) {
        return Some(path.clone());
    }

    None
}

/// Validation issue found in dataset structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationIssue {
    /// Arrow file is missing
    MissingArrowFile { expected: PathBuf },
    /// Sensor container directory is missing
    MissingSensorContainer { expected: PathBuf },
    /// A referenced file is missing
    MissingFile { name: String, expected: PathBuf },
    /// An unreferenced file was found in the container
    UnreferencedFile { path: PathBuf },
    /// Invalid directory structure
    InvalidStructure { message: String },
}

impl std::fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationIssue::MissingArrowFile { expected } => {
                write!(f, "Missing Arrow file: {:?}", expected)
            }
            ValidationIssue::MissingSensorContainer { expected } => {
                write!(f, "Missing sensor container directory: {:?}", expected)
            }
            ValidationIssue::MissingFile { name, expected } => {
                write!(f, "Missing file for sample '{}': {:?}", name, expected)
            }
            ValidationIssue::UnreferencedFile { path } => {
                write!(f, "Unreferenced file in container: {:?}", path)
            }
            ValidationIssue::InvalidStructure { message } => {
                write!(f, "Invalid structure: {}", message)
            }
        }
    }
}

/// Validate the structure of a dataset directory.
///
/// Checks that the directory follows the EdgeFirst Dataset Format:
/// - Arrow file exists at expected location
/// - Sensor container directory exists
/// - All files referenced in Arrow file exist in container
/// - Reports any unreferenced files
///
/// # Arguments
///
/// * `dataset_dir` - Path to the snapshot root directory
///
/// # Returns
///
/// A list of validation issues (empty if valid).
///
/// # Example
///
/// ```rust,no_run
/// use edgefirst_client::format::validate_dataset_structure;
/// use std::path::Path;
///
/// let issues = validate_dataset_structure(Path::new("my_dataset"))?;
/// if issues.is_empty() {
///     println!("Dataset structure is valid!");
/// } else {
///     for issue in &issues {
///         eprintln!("Issue: {}", issue);
///     }
/// }
/// # Ok::<(), edgefirst_client::Error>(())
/// ```
#[cfg(feature = "polars")]
pub fn validate_dataset_structure(dataset_dir: &Path) -> Result<Vec<ValidationIssue>, Error> {
    let mut issues = Vec::new();

    // Get the dataset name from the directory name
    let dataset_name = dataset_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::InvalidParameters("Invalid dataset directory path".to_owned()))?;

    // Check for Arrow file
    let arrow_path = dataset_dir.join(format!("{}.arrow", dataset_name));
    if !arrow_path.exists() {
        issues.push(ValidationIssue::MissingArrowFile {
            expected: arrow_path.clone(),
        });
        // Can't continue validation without Arrow file
        return Ok(issues);
    }

    // Check for sensor container
    let container_path = dataset_dir.join(dataset_name);
    if !container_path.exists() {
        issues.push(ValidationIssue::MissingSensorContainer {
            expected: container_path.clone(),
        });
        // Can't continue validation without container
        return Ok(issues);
    }

    // Resolve files and check for missing ones
    let resolved = resolve_files_with_container(&arrow_path, &container_path)?;

    // Track which files were referenced
    let mut referenced_files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    for file in &resolved {
        match &file.path {
            Some(path) => {
                referenced_files.insert(path.clone());
            }
            None => {
                issues.push(ValidationIssue::MissingFile {
                    name: file.name.clone(),
                    expected: file.expected_path.clone(),
                });
            }
        }
    }

    // Find unreferenced files in container
    for entry in WalkDir::new(&container_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path().to_path_buf();

        // Check if this file is an image file
        let is_image = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                matches!(
                    e.to_lowercase().as_str(),
                    "jpg" | "jpeg" | "png" | "pcd" | "bin"
                )
            })
            .unwrap_or(false);

        if is_image && !referenced_files.contains(&path) {
            issues.push(ValidationIssue::UnreferencedFile { path });
        }
    }

    Ok(issues)
}

/// Generate an Arrow file from a folder of images.
///
/// Scans the folder for image files and creates an Arrow annotation file
/// with null annotations (for unannotated datasets). This is useful for
/// importing existing image collections into EdgeFirst.
///
/// # Arguments
///
/// * `folder` - Path to the folder containing images
/// * `output` - Path where the Arrow file should be written
/// * `detect_sequences` - If true, attempt to detect sequences from naming
///   patterns
///
/// # Returns
///
/// The number of samples (images) included in the Arrow file.
///
/// # Sequence Detection
///
/// When `detect_sequences` is true, the function looks for patterns like:
/// - `{name}_{number}.{ext}` → sequence with frame number
/// - `{sequence}/{name}_{number}.{ext}` → sequence in subdirectory
///
/// # Example
///
/// ```rust,no_run
/// use edgefirst_client::format::generate_arrow_from_folder;
/// use std::path::Path;
///
/// // Generate Arrow file from images
/// let count = generate_arrow_from_folder(
///     Path::new("my_images"),
///     Path::new("my_dataset/my_dataset.arrow"),
///     true, // detect sequences
/// )?;
/// println!("Created Arrow file with {} samples", count);
/// # Ok::<(), edgefirst_client::Error>(())
/// ```
#[cfg(feature = "polars")]
pub fn generate_arrow_from_folder(
    folder: &Path,
    output: &Path,
    detect_sequences: bool,
) -> Result<usize, Error> {
    use polars::prelude::*;
    use std::io::BufWriter;

    // Collect all image files
    let image_files: Vec<PathBuf> = WalkDir::new(folder)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_lowercase().as_str(),
                        "jpg" | "jpeg" | "png" | "pcd" | "bin"
                    )
                })
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    if image_files.is_empty() {
        return Err(Error::InvalidParameters(
            "No image files found in folder".to_owned(),
        ));
    }

    // Parse each image file to extract name and frame
    let mut names: Vec<String> = Vec::new();
    let mut frames: Vec<Option<u64>> = Vec::new();

    for path in &image_files {
        let (name, frame) = parse_image_filename(path, folder, detect_sequences);
        names.push(name);
        frames.push(frame);
    }

    // Build the DataFrame with the 2025.10 schema
    let name_series = Series::new("name".into(), &names);
    let frame_series = Series::new("frame".into(), &frames);

    // Create null columns for annotations
    let null_strings: Vec<Option<&str>> = vec![None; names.len()];
    let null_u64s: Vec<Option<u64>> = vec![None; names.len()];

    let object_id_series = Series::new("object_id".into(), &null_strings);
    let label_series = Series::new("label".into(), &null_strings);
    let label_index_series = Series::new("label_index".into(), &null_u64s);
    let group_series = Series::new("group".into(), &null_strings);

    // Null geometry columns - use Option<Series> like annotations_dataframe does
    let null_series_vec: Vec<Option<Series>> = vec![None; names.len()];

    let mask_series = Series::new("mask".into(), null_series_vec.clone())
        .cast(&DataType::List(Box::new(DataType::Float32)))?;

    let box2d_series = Series::new("box2d".into(), null_series_vec.clone())
        .cast(&DataType::Array(Box::new(DataType::Float32), 4))?;

    let box3d_series = Series::new("box3d".into(), null_series_vec)
        .cast(&DataType::Array(Box::new(DataType::Float32), 6))?;

    let mut df = DataFrame::new(vec![
        name_series.into(),
        frame_series.into(),
        object_id_series.into(),
        label_series.into(),
        label_index_series.into(),
        group_series.into(),
        mask_series.into(),
        box2d_series.into(),
        box3d_series.into(),
    ])?;

    // Create output directory if needed
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write the Arrow file
    let file = File::create(output)?;
    let writer = BufWriter::new(file);
    IpcWriter::new(writer)
        .finish(&mut df)
        .map_err(|e| Error::InvalidParameters(format!("Failed to write Arrow file: {}", e)))?;

    Ok(image_files.len())
}

/// Parse an image filename to extract sample name and frame number.
fn parse_image_filename(path: &Path, root: &Path, detect_sequences: bool) -> (String, Option<u64>) {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Remove .camera suffix if present
    let clean_stem = stem.strip_suffix(".camera").unwrap_or(stem);

    if !detect_sequences {
        return (clean_stem.to_string(), None);
    }

    // Try to detect sequence pattern: name_frame
    // Look for trailing number separated by underscore
    if let Some(idx) = clean_stem.rfind('_') {
        let (name_part, frame_part) = clean_stem.split_at(idx);
        let frame_str = &frame_part[1..]; // Skip the underscore

        if let Ok(frame) = frame_str.parse::<u64>() {
            // Check if this might be in a sequence directory
            let relative = path.strip_prefix(root).unwrap_or(path);
            if relative.components().count() > 1 {
                // In a subdirectory - this is likely a sequence
                return (name_part.to_string(), Some(frame));
            }

            // Also detect if multiple files share the same prefix
            // (This is a heuristic - files in root with _N pattern are likely sequences)
            return (name_part.to_string(), Some(frame));
        }
    }

    // No sequence detected
    (clean_stem.to_string(), None)
}

/// Get the expected sensor container path for a dataset directory.
///
/// # Arguments
///
/// * `dataset_dir` - Path to the snapshot root directory
///
/// # Returns
///
/// The expected path to the sensor container directory.
pub fn get_sensor_container_path(dataset_dir: &Path) -> Option<PathBuf> {
    let dataset_name = dataset_dir.file_name()?.to_str()?;
    Some(dataset_dir.join(dataset_name))
}

/// Get the expected Arrow file path for a dataset directory.
///
/// # Arguments
///
/// * `dataset_dir` - Path to the snapshot root directory
///
/// # Returns
///
/// The expected path to the Arrow annotation file.
pub fn get_arrow_path(dataset_dir: &Path) -> Option<PathBuf> {
    let dataset_name = dataset_dir.file_name()?.to_str()?;
    Some(dataset_dir.join(format!("{}.arrow", dataset_name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Create a test image file (minimal JPEG).
    fn create_test_image(path: &Path) {
        // Minimal valid JPEG (smallest possible)
        let jpeg_data: &[u8] = &[
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06,
            0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D,
            0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D,
            0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28,
            0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
            0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01,
            0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10,
            0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05, 0x05, 0x04, 0x04, 0x00, 0x00,
            0x01, 0x7D, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06,
            0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42,
            0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16,
            0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37,
            0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55,
            0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73,
            0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
            0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5,
            0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA,
            0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6,
            0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA,
            0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xDA, 0x00, 0x08,
            0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0xFB, 0xD5, 0xDB, 0x20, 0xA8, 0xF1, 0x4D, 0x9E,
            0xBA, 0x79, 0xC5, 0x14, 0x51, 0x40, 0xFF, 0xD9,
        ];

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = File::create(path).unwrap();
        file.write_all(jpeg_data).unwrap();
    }

    #[test]
    fn test_get_arrow_path() {
        let dir = Path::new("/data/my_dataset");
        let arrow = get_arrow_path(dir).unwrap();
        assert_eq!(arrow, PathBuf::from("/data/my_dataset/my_dataset.arrow"));
    }

    #[test]
    fn test_get_sensor_container_path() {
        let dir = Path::new("/data/my_dataset");
        let container = get_sensor_container_path(dir).unwrap();
        assert_eq!(container, PathBuf::from("/data/my_dataset/my_dataset"));
    }

    #[test]
    fn test_parse_image_filename_standalone() {
        let root = Path::new("/data");
        let path = Path::new("/data/image.jpg");

        let (name, frame) = parse_image_filename(path, root, true);
        assert_eq!(name, "image");
        assert_eq!(frame, None);
    }

    #[test]
    fn test_parse_image_filename_camera_extension() {
        let root = Path::new("/data");
        let path = Path::new("/data/sample.camera.jpeg");

        let (name, frame) = parse_image_filename(path, root, true);
        assert_eq!(name, "sample");
        assert_eq!(frame, None);
    }

    #[test]
    fn test_parse_image_filename_sequence() {
        let root = Path::new("/data");
        let path = Path::new("/data/seq/seq_001.camera.jpeg");

        let (name, frame) = parse_image_filename(path, root, true);
        assert_eq!(name, "seq");
        assert_eq!(frame, Some(1));
    }

    #[test]
    fn test_parse_image_filename_no_sequence_detection() {
        let root = Path::new("/data");
        let path = Path::new("/data/seq/seq_001.camera.jpeg");

        let (name, frame) = parse_image_filename(path, root, false);
        assert_eq!(name, "seq_001");
        assert_eq!(frame, None);
    }

    #[test]
    fn test_build_file_index() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        create_test_image(&root.join("image1.jpg"));
        create_test_image(&root.join("sub/image2.camera.jpeg"));

        let index = build_file_index(root).unwrap();

        // Check that files are indexed
        assert!(index.contains_key("image1.jpg"));
        assert!(index.contains_key("image2.camera.jpeg"));

        // Check stem indexing
        assert!(index.contains_key("image1"));
        assert!(index.contains_key("image2"));
    }

    #[test]
    fn test_find_matching_file() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        create_test_image(&root.join("sample.camera.jpeg"));
        create_test_image(&root.join("seq/seq_001.camera.jpeg"));

        let index = build_file_index(root).unwrap();

        // Find standalone file
        let found = find_matching_file(&index, "sample", None);
        assert!(found.is_some());

        // Find sequence file
        let found = find_matching_file(&index, "seq", Some(1));
        assert!(found.is_some());

        // Missing file
        let found = find_matching_file(&index, "nonexistent", None);
        assert!(found.is_none());
    }

    #[cfg(feature = "polars")]
    #[test]
    fn test_generate_arrow_from_folder() {
        use polars::prelude::*;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test images
        let images_dir = root.join("images");
        create_test_image(&images_dir.join("photo1.jpg"));
        create_test_image(&images_dir.join("photo2.png"));
        create_test_image(&images_dir.join("seq/seq_001.camera.jpeg"));
        create_test_image(&images_dir.join("seq/seq_002.camera.jpeg"));

        // Generate Arrow file
        let arrow_path = root.join("output.arrow");
        let count = generate_arrow_from_folder(&images_dir, &arrow_path, true).unwrap();

        assert_eq!(count, 4);
        assert!(arrow_path.exists());

        // Verify Arrow file content
        let mut file = File::open(&arrow_path).unwrap();
        let df = IpcReader::new(&mut file).finish().unwrap();

        assert_eq!(df.height(), 4);
        assert!(df.column("name").is_ok());
        assert!(df.column("frame").is_ok());
        assert!(df.column("label").is_ok());
    }

    #[cfg(feature = "polars")]
    #[test]
    fn test_resolve_arrow_files() {
        use polars::prelude::*;
        use std::io::BufWriter;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a simple Arrow file
        let names = Series::new("name".into(), &["sample1", "sample2", "seq"]);
        let frames: Vec<Option<u64>> = vec![None, None, Some(1)];
        let frame_series = Series::new("frame".into(), &frames);

        let mut df = DataFrame::new(vec![names.into(), frame_series.into()]).unwrap();

        let arrow_path = root.join("test.arrow");
        let file = File::create(&arrow_path).unwrap();
        let writer = BufWriter::new(file);
        IpcWriter::new(writer).finish(&mut df).unwrap();

        // Test resolution
        let resolved = resolve_arrow_files(&arrow_path).unwrap();

        assert_eq!(resolved.len(), 3);
        assert!(resolved.contains_key("sample1"));
        assert!(resolved.contains_key("sample2"));
        assert!(resolved.contains_key("seq"));
    }

    #[cfg(feature = "polars")]
    #[test]
    fn test_validate_dataset_structure_valid() {
        use polars::prelude::*;
        use std::io::BufWriter;

        let temp_dir = TempDir::new().unwrap();
        let dataset_dir = temp_dir.path().join("my_dataset");
        std::fs::create_dir_all(&dataset_dir).unwrap();

        // Create Arrow file
        let names = Series::new("name".into(), &["image1"]);
        let frames: Vec<Option<u64>> = vec![None];
        let frame_series = Series::new("frame".into(), &frames);

        let mut df = DataFrame::new(vec![names.into(), frame_series.into()]).unwrap();

        let arrow_path = dataset_dir.join("my_dataset.arrow");
        let file = File::create(&arrow_path).unwrap();
        let writer = BufWriter::new(file);
        IpcWriter::new(writer).finish(&mut df).unwrap();

        // Create sensor container with matching file
        let container = dataset_dir.join("my_dataset");
        create_test_image(&container.join("image1.camera.jpeg"));

        // Validate
        let issues = validate_dataset_structure(&dataset_dir).unwrap();

        // Should have no missing file issues
        let missing_files: Vec<_> = issues
            .iter()
            .filter(|i| matches!(i, ValidationIssue::MissingFile { .. }))
            .collect();
        assert!(
            missing_files.is_empty(),
            "Unexpected missing files: {:?}",
            missing_files
        );
    }

    #[cfg(feature = "polars")]
    #[test]
    fn test_validate_dataset_structure_missing_arrow() {
        let temp_dir = TempDir::new().unwrap();
        let dataset_dir = temp_dir.path().join("my_dataset");
        std::fs::create_dir_all(&dataset_dir).unwrap();

        let issues = validate_dataset_structure(&dataset_dir).unwrap();

        assert_eq!(issues.len(), 1);
        assert!(matches!(
            &issues[0],
            ValidationIssue::MissingArrowFile { .. }
        ));
    }

    #[test]
    fn test_image_extensions() {
        assert!(IMAGE_EXTENSIONS.contains(&"jpg"));
        assert!(IMAGE_EXTENSIONS.contains(&"jpeg"));
        assert!(IMAGE_EXTENSIONS.contains(&"png"));
        assert!(IMAGE_EXTENSIONS.contains(&"camera.jpeg"));
    }

    #[test]
    fn test_validation_issue_display() {
        let issue = ValidationIssue::MissingFile {
            name: "test".to_string(),
            expected: PathBuf::from("test.jpg"),
        };
        let display = format!("{}", issue);
        assert!(display.contains("test"));
        assert!(display.contains("test.jpg"));
    }
}
