// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use std::{collections::HashMap, fmt::Display};

use crate::{
    Client, Error,
    api::{AnnotationSetID, DatasetID, ProjectID, SampleID},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "polars")]
use polars::prelude::*;

/// File types supported in EdgeFirst Studio datasets.
///
/// Represents the different types of sensor data files that can be stored
/// and processed in a dataset. EdgeFirst Studio supports various modalities
/// including visual images and different forms of LiDAR and radar data.
///
/// # String Representations
///
/// This enum has two string representations:
/// - **Display** (`fmt::Display`): Returns the server API type name (e.g., `"lidar.depth"`)
///   used when making API requests to EdgeFirst Studio.
/// - **file_extension()**: Returns the file extension for saving (e.g., `"lidar.png"`)
///   which may differ from the API type name.
///
/// # Examples
///
/// ```rust
/// use edgefirst_client::FileType;
///
/// // Create file types from strings
/// let image_type: FileType = "image".try_into().unwrap();
/// let lidar_type: FileType = "lidar.pcd".try_into().unwrap();
///
/// // Display file types
/// println!("Processing {} files", image_type); // "Processing image files"
///
/// // Use in dataset operations - example usage
/// let file_type = FileType::Image;
/// match file_type {
///     FileType::Image => println!("Processing image files"),
///     FileType::LidarPcd => println!("Processing LiDAR point cloud files"),
///     _ => println!("Processing other sensor data"),
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum FileType {
    /// Standard image files (JPEG, PNG, etc.)
    Image,
    /// LiDAR point cloud data files (.pcd format)
    LidarPcd,
    /// LiDAR depth images (.png format)
    LidarDepth,
    /// LiDAR reflectance images (.jpg format)
    LidarReflect,
    /// Radar point cloud data files (.pcd format)
    RadarPcd,
    /// Radar cube data files (.png format)
    RadarCube,
    /// All sensor types - expands to all known file types
    All,
}

impl std::fmt::Display for FileType {
    /// Returns the server API type name for this file type.
    /// Used when making API requests to the server.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            FileType::Image => "image",
            FileType::LidarPcd => "lidar.pcd",
            FileType::LidarDepth => "lidar.depth",
            FileType::LidarReflect => "lidar.reflect",
            FileType::RadarPcd => "radar.pcd",
            FileType::RadarCube => "radar.png",
            FileType::All => "all",
        };
        write!(f, "{}", value)
    }
}

impl FileType {
    /// Returns the file extension to use when saving downloaded files.
    /// This may differ from the API type name (e.g., lidar.depth → lidar.png).
    pub fn file_extension(&self) -> &'static str {
        match self {
            FileType::Image => "jpg", // Will be overridden by infer detection
            FileType::LidarPcd => "lidar.pcd",
            FileType::LidarDepth => "lidar.png",
            FileType::LidarReflect => "lidar.jpg",
            FileType::RadarPcd => "radar.pcd",
            FileType::RadarCube => "radar.png",
            FileType::All => "",
        }
    }
}

impl TryFrom<&str> for FileType {
    type Error = crate::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "image" => Ok(FileType::Image),
            "lidar.pcd" => Ok(FileType::LidarPcd),
            // Accept CLI names (lidar.png), server names (lidar.depth), and aliases
            "lidar.png" | "lidar.depth" | "depth.png" | "depthmap" => Ok(FileType::LidarDepth),
            "lidar.jpg" | "lidar.jpeg" | "lidar.reflect" => Ok(FileType::LidarReflect),
            "radar.pcd" | "pcd" => Ok(FileType::RadarPcd),
            "radar.png" | "cube" => Ok(FileType::RadarCube),
            "all" => Ok(FileType::All),
            _ => Err(crate::Error::InvalidFileType(s.to_string())),
        }
    }
}

impl std::str::FromStr for FileType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl FileType {
    /// Returns all concrete sensor file types (excludes `All`).
    ///
    /// This is useful for expanding the `All` variant or listing available
    /// types.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgefirst_client::FileType;
    ///
    /// let all_types = FileType::all_sensor_types();
    /// assert!(all_types.contains(&FileType::Image));
    /// assert!(!all_types.contains(&FileType::All));
    /// ```
    pub fn all_sensor_types() -> Vec<FileType> {
        vec![
            FileType::Image,
            FileType::LidarPcd,
            FileType::LidarDepth,
            FileType::LidarReflect,
            FileType::RadarPcd,
            FileType::RadarCube,
        ]
    }

    /// Returns all valid type names as strings for help text.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgefirst_client::FileType;
    ///
    /// let names = FileType::type_names();
    /// assert!(names.contains(&"image"));
    /// assert!(names.contains(&"all"));
    /// ```
    pub fn type_names() -> Vec<&'static str> {
        vec![
            "image",
            "lidar.pcd",
            "lidar.png",
            "lidar.jpg",
            "radar.pcd",
            "radar.png",
            "all",
        ]
    }

    /// Expands a list of file types, replacing `All` with all concrete sensor
    /// types.
    ///
    /// If the input contains `FileType::All`, returns all sensor types.
    /// Otherwise, returns the input types unchanged.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgefirst_client::FileType;
    ///
    /// let types = vec![FileType::All];
    /// let expanded = FileType::expand_types(&types);
    /// assert_eq!(expanded.len(), 6); // All concrete sensor types
    ///
    /// let types = vec![FileType::Image, FileType::LidarPcd];
    /// let expanded = FileType::expand_types(&types);
    /// assert_eq!(expanded.len(), 2); // Unchanged
    /// ```
    pub fn expand_types(types: &[FileType]) -> Vec<FileType> {
        if types.contains(&FileType::All) {
            FileType::all_sensor_types()
        } else {
            types.to_vec()
        }
    }
}

/// Annotation types supported for labeling data in EdgeFirst Studio.
///
/// Represents the different types of annotations that can be applied to
/// sensor data for machine learning tasks. Each type corresponds to a
/// different annotation geometry and use case.
///
/// # Examples
///
/// ```rust
/// use edgefirst_client::AnnotationType;
///
/// // Create annotation types from strings (using TryFrom)
/// let box_2d: AnnotationType = "box2d".try_into().unwrap();
/// let segmentation: AnnotationType = "mask".try_into().unwrap();
///
/// // Or use From with String
/// let box_2d = AnnotationType::from("box2d".to_string());
/// let segmentation = AnnotationType::from("mask".to_string());
///
/// // Display annotation types
/// println!("Annotation type: {}", box_2d); // "Annotation type: box2d"
///
/// // Use in matching and processing
/// let annotation_type = AnnotationType::Box2d;
/// match annotation_type {
///     AnnotationType::Box2d => println!("Processing 2D bounding boxes"),
///     AnnotationType::Box3d => println!("Processing 3D bounding boxes"),
///     AnnotationType::Mask => println!("Processing segmentation masks"),
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum AnnotationType {
    /// 2D bounding boxes for object detection in images
    Box2d,
    /// 3D bounding boxes for object detection in 3D space (LiDAR, etc.)
    Box3d,
    /// Pixel-level segmentation masks for semantic/instance segmentation
    Mask,
}

impl TryFrom<&str> for AnnotationType {
    type Error = crate::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "box2d" => Ok(AnnotationType::Box2d),
            "box3d" => Ok(AnnotationType::Box3d),
            "mask" => Ok(AnnotationType::Mask),
            _ => Err(crate::Error::InvalidAnnotationType(s.to_string())),
        }
    }
}

impl From<String> for AnnotationType {
    fn from(s: String) -> Self {
        // For backward compatibility, default to Box2d if invalid
        s.as_str().try_into().unwrap_or(AnnotationType::Box2d)
    }
}

impl From<&String> for AnnotationType {
    fn from(s: &String) -> Self {
        // For backward compatibility, default to Box2d if invalid
        s.as_str().try_into().unwrap_or(AnnotationType::Box2d)
    }
}

impl AnnotationType {
    /// Returns the server API type name for this annotation type.
    ///
    /// The server uses different naming conventions than the client:
    /// - `Box2d` → `"box"` (server) vs `"box2d"` (client display)
    /// - `Box3d` → `"box3d"` (same)
    /// - `Mask` → `"seg"` (server) vs `"mask"` (client display)
    pub fn as_server_type(&self) -> &'static str {
        match self {
            AnnotationType::Box2d => "box",
            AnnotationType::Box3d => "box3d",
            AnnotationType::Mask => "seg",
        }
    }
}

impl std::fmt::Display for AnnotationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            AnnotationType::Box2d => "box2d",
            AnnotationType::Box3d => "box3d",
            AnnotationType::Mask => "mask",
        };
        write!(f, "{}", value)
    }
}

/// A dataset in EdgeFirst Studio containing sensor data and annotations.
///
/// Datasets are collections of multi-modal sensor data (images, LiDAR, radar)
/// along with their corresponding annotations (bounding boxes, segmentation
/// masks, 3D annotations). Datasets belong to projects and can be used for
/// training and validation of machine learning models.
///
/// # Features
///
/// - **Multi-modal Data**: Support for images, LiDAR point clouds, radar data
/// - **Rich Annotations**: 2D/3D bounding boxes, segmentation masks
/// - **Metadata**: Timestamps, sensor configurations, calibration data
/// - **Version Control**: Track changes and maintain data lineage
/// - **Format Conversion**: Export to popular ML frameworks
///
/// # Examples
///
/// ```no_run
/// use edgefirst_client::{Client, Dataset, DatasetID};
/// use std::str::FromStr;
///
/// # async fn example() -> Result<(), edgefirst_client::Error> {
/// # let client = Client::new()?;
/// // Get dataset information
/// let dataset_id = DatasetID::from_str("ds-abc123")?;
/// let dataset = client.dataset(dataset_id).await?;
/// println!("Dataset: {}", dataset.name());
///
/// // Access dataset metadata
/// println!("Dataset ID: {}", dataset.id());
/// println!("Description: {}", dataset.description());
/// println!("Created: {}", dataset.created());
///
/// // Work with dataset data would require additional methods
/// // that are implemented in the full API
/// # Ok(())
/// # }
/// ```
#[derive(Deserialize, Clone, Debug)]
pub struct Dataset {
    id: DatasetID,
    project_id: ProjectID,
    name: String,
    description: String,
    cloud_key: String,
    #[serde(rename = "createdAt")]
    created: DateTime<Utc>,
}

impl Display for Dataset {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.name)
    }
}

impl Dataset {
    pub fn id(&self) -> DatasetID {
        self.id
    }

    pub fn project_id(&self) -> ProjectID {
        self.project_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn cloud_key(&self) -> &str {
        &self.cloud_key
    }

    pub fn created(&self) -> &DateTime<Utc> {
        &self.created
    }

    pub async fn project(&self, client: &Client) -> Result<crate::api::Project, Error> {
        client.project(self.project_id).await
    }

    pub async fn annotation_sets(&self, client: &Client) -> Result<Vec<AnnotationSet>, Error> {
        client.annotation_sets(self.id).await
    }

    pub async fn labels(&self, client: &Client) -> Result<Vec<Label>, Error> {
        client.labels(self.id).await
    }

    pub async fn add_label(&self, client: &Client, name: &str) -> Result<(), Error> {
        client.add_label(self.id, name).await
    }

    pub async fn remove_label(&self, client: &Client, name: &str) -> Result<(), Error> {
        let labels = self.labels(client).await?;
        let label = labels
            .iter()
            .find(|l| l.name() == name)
            .ok_or_else(|| Error::MissingLabel(name.to_string()))?;
        client.remove_label(label.id()).await
    }
}

/// The AnnotationSet class represents a collection of annotations in a dataset.
/// A dataset can have multiple annotation sets, each containing annotations for
/// different tasks or purposes.
#[derive(Deserialize)]
pub struct AnnotationSet {
    id: AnnotationSetID,
    dataset_id: DatasetID,
    name: String,
    description: String,
    #[serde(rename = "date")]
    created: DateTime<Utc>,
}

impl Display for AnnotationSet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.name)
    }
}

impl AnnotationSet {
    pub fn id(&self) -> AnnotationSetID {
        self.id
    }

    pub fn dataset_id(&self) -> DatasetID {
        self.dataset_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn created(&self) -> DateTime<Utc> {
        self.created
    }

    pub async fn dataset(&self, client: &Client) -> Result<Dataset, Error> {
        client.dataset(self.dataset_id).await
    }
}

/// A sample in a dataset, typically representing a single image with metadata
/// and optional sensor data.
///
/// Each sample has a unique ID, image reference, and can include additional
/// sensor data like LiDAR, radar, or depth maps. Samples can also have
/// associated annotations.
#[derive(Serialize, Clone, Debug)]
pub struct Sample {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SampleID>,
    /// Dataset split (train, val, test) - stored in Arrow metadata, not used
    /// for directory structure.
    /// API field name discrepancy: samples.populate2 expects "group", but
    /// samples.list returns "group_name".
    #[serde(
        alias = "group_name",
        rename(serialize = "group", deserialize = "group_name"),
        skip_serializing_if = "Option::is_none"
    )]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_description: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_frame_number"
    )]
    pub frame_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Camera location and pose (GPS + IMU data).
    /// Location data is extracted from the "sensors" field during
    /// deserialization. When uploading samples, this field is serialized
    /// as "sensors" to match the samples.populate2 API format.
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename(serialize = "sensors")
    )]
    pub location: Option<Location>,
    /// Image degradation type (blur, occlusion, weather, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degradation: Option<String>,
    /// Additional sensor files (LiDAR, radar, depth maps, etc.).
    /// Deserialization is handled by custom Deserialize impl which extracts
    /// files from the "sensors" field. Serialization converts to HashMap for
    /// samples.populate2 API.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_files"
    )]
    pub files: Vec<SampleFile>,
    /// Annotations associated with this sample.
    /// Deserialization is handled by custom Deserialize impl.
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_annotations"
    )]
    pub annotations: Vec<Annotation>,
}

// Custom deserializer for frame_number - converts -1 to None
// Server returns -1 for non-sequence samples, but clients should see None
fn deserialize_frame_number<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let value = Option::<i32>::deserialize(deserializer)?;
    Ok(value.and_then(|v| if v < 0 { None } else { Some(v as u32) }))
}

/// Check if a string is a valid downloadable URL (http/https).
/// Used to distinguish between pre-signed URLs and inline base64/JSON data.
fn is_valid_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

// Custom serializer for files field - converts Vec<SampleFile> to
// HashMap<String, String>
fn serialize_files<S>(files: &[SampleFile], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    let map: HashMap<String, String> = files
        .iter()
        .filter_map(|f| {
            f.filename()
                .map(|filename| (f.file_type().to_string(), filename.to_string()))
        })
        .collect();
    map.serialize(serializer)
}

// Custom serializer for annotations field - serializes to a flat
// Vec<Annotation> to match the updated samples.populate2 contract (annotations
// array)
fn serialize_annotations<S>(annotations: &Vec<Annotation>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serde::Serialize::serialize(annotations, serializer)
}

// Custom deserializer for annotations field - converts server format back to
// Vec<Annotation>
fn deserialize_annotations<'de, D>(deserializer: D) -> Result<Vec<Annotation>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AnnotationsFormat {
        Vec(Vec<Annotation>),
        Map(HashMap<String, Vec<Annotation>>),
    }

    let value = Option::<AnnotationsFormat>::deserialize(deserializer)?;
    Ok(value
        .map(|v| match v {
            AnnotationsFormat::Vec(annotations) => annotations,
            AnnotationsFormat::Map(map) => convert_annotations_map_to_vec(map),
        })
        .unwrap_or_default())
}

/// Intermediate struct for deserializing sensors data that may contain both
/// file references (URLs/data) and location data (GPS/IMU).
#[derive(Debug, Default)]
struct SensorsData {
    files: Vec<SampleFile>,
    location: Option<Location>,
}

/// Deserialize sensors field into both files and location data.
fn deserialize_sensors_data(value: Option<serde_json::Value>) -> SensorsData {
    use serde_json::Value;

    /// Create a SampleFile from a string value, distinguishing URL vs inline
    /// data.
    fn create_sample_file(file_type: String, value: String) -> SampleFile {
        if is_valid_url(&value) {
            SampleFile::with_url(file_type, value)
        } else {
            SampleFile::with_data(file_type, value)
        }
    }

    /// Create a SampleFile from any JSON value, converting non-strings to JSON.
    fn create_sample_file_from_value(file_type: String, value: Value) -> Option<SampleFile> {
        match value {
            Value::String(s) => Some(create_sample_file(file_type, s)),
            Value::Object(_) | Value::Array(_) => {
                // Inline JSON data (legacy format) - serialize to string
                serde_json::to_string(&value)
                    .ok()
                    .map(|data| SampleFile::with_data(file_type, data))
            }
            _ => None,
        }
    }

    /// Try to extract Location from a JSON object containing gps/imu keys.
    fn extract_location(map: &serde_json::Map<String, Value>) -> Option<Location> {
        let gps = map
            .get("gps")
            .and_then(|v| serde_json::from_value::<GpsData>(v.clone()).ok());
        let imu = map
            .get("imu")
            .and_then(|v| serde_json::from_value::<ImuData>(v.clone()).ok());

        if gps.is_some() || imu.is_some() {
            Some(Location { gps, imu })
        } else {
            None
        }
    }

    let mut result = SensorsData::default();

    match value {
        None => result,
        Some(Value::Array(arr)) => {
            // Array of single-key objects: [{"radar.png": "url"}, {"gps": {...}}, ...]
            for item in arr {
                if let Value::Object(map) = item {
                    // Check if this looks like a SampleFile object (has "type" key)
                    if map.contains_key("type") {
                        // Try to parse as SampleFile
                        if let Ok(file) =
                            serde_json::from_value::<SampleFile>(Value::Object(map.clone()))
                        {
                            result.files.push(file);
                        }
                    } else {
                        // Check for location data (gps/imu)
                        if let Some(loc) = extract_location(&map) {
                            // Merge with existing location
                            if let Some(ref mut existing) = result.location {
                                if loc.gps.is_some() {
                                    existing.gps = loc.gps;
                                }
                                if loc.imu.is_some() {
                                    existing.imu = loc.imu;
                                }
                            } else {
                                result.location = Some(loc);
                            }
                        } else {
                            // Single-key object: {file_type: url_or_data}
                            for (file_type, value) in map {
                                if let Some(file) = create_sample_file_from_value(file_type, value)
                                {
                                    result.files.push(file);
                                }
                            }
                        }
                    }
                }
            }
            result
        }
        Some(Value::Object(map)) => {
            // Check if this contains location data (gps or imu keys with object values)
            if let Some(loc) = extract_location(&map) {
                result.location = Some(loc);
            }

            // Also extract any file references (non-location keys)
            for (key, value) in map {
                if key != "gps"
                    && key != "imu"
                    && let Some(file) = create_sample_file_from_value(key, value)
                {
                    result.files.push(file);
                }
            }
            result
        }
        Some(_) => result,
    }
}

/// Raw sample structure for deserialization.
/// This mirrors Sample but deserializes sensors into a combined struct
/// that captures both files and location data.
#[derive(Deserialize)]
struct SampleRaw {
    #[serde(default)]
    id: Option<SampleID>,
    #[serde(alias = "group_name")]
    group: Option<String>,
    sequence_name: Option<String>,
    sequence_uuid: Option<String>,
    sequence_description: Option<String>,
    #[serde(default, deserialize_with = "deserialize_frame_number")]
    frame_number: Option<u32>,
    uuid: Option<String>,
    image_name: Option<String>,
    image_url: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    date: Option<DateTime<Utc>>,
    source: Option<String>,
    degradation: Option<String>,
    /// Raw sensors JSON - will be processed into files + location
    #[serde(default, alias = "sensors")]
    sensors: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "deserialize_annotations")]
    annotations: Vec<Annotation>,
}

impl From<SampleRaw> for Sample {
    fn from(raw: SampleRaw) -> Self {
        let sensors_data = deserialize_sensors_data(raw.sensors);

        Sample {
            id: raw.id,
            group: raw.group,
            sequence_name: raw.sequence_name,
            sequence_uuid: raw.sequence_uuid,
            sequence_description: raw.sequence_description,
            frame_number: raw.frame_number,
            uuid: raw.uuid,
            image_name: raw.image_name,
            image_url: raw.image_url,
            width: raw.width,
            height: raw.height,
            date: raw.date,
            source: raw.source,
            location: sensors_data.location,
            degradation: raw.degradation,
            files: sensors_data.files,
            annotations: raw.annotations,
        }
    }
}

impl<'de> serde::Deserialize<'de> for Sample {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = SampleRaw::deserialize(deserializer)?;
        Ok(Sample::from(raw))
    }
}

impl Display for Sample {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            self.image_name().unwrap_or("unknown")
        )
    }
}

impl Default for Sample {
    fn default() -> Self {
        Self::new()
    }
}

impl Sample {
    /// Creates a new empty sample.
    pub fn new() -> Self {
        Self {
            id: None,
            group: None,
            sequence_name: None,
            sequence_uuid: None,
            sequence_description: None,
            frame_number: None,
            uuid: None,
            image_name: None,
            image_url: None,
            width: None,
            height: None,
            date: None,
            source: None,
            location: None,
            degradation: None,
            files: vec![],
            annotations: vec![],
        }
    }

    pub fn id(&self) -> Option<SampleID> {
        self.id
    }

    pub fn name(&self) -> Option<String> {
        self.image_name.as_ref().map(|n| extract_sample_name(n))
    }

    pub fn group(&self) -> Option<&String> {
        self.group.as_ref()
    }

    pub fn sequence_name(&self) -> Option<&String> {
        self.sequence_name.as_ref()
    }

    pub fn sequence_uuid(&self) -> Option<&String> {
        self.sequence_uuid.as_ref()
    }

    pub fn sequence_description(&self) -> Option<&String> {
        self.sequence_description.as_ref()
    }

    pub fn frame_number(&self) -> Option<u32> {
        self.frame_number
    }

    pub fn uuid(&self) -> Option<&String> {
        self.uuid.as_ref()
    }

    pub fn image_name(&self) -> Option<&str> {
        self.image_name.as_deref()
    }

    pub fn image_url(&self) -> Option<&str> {
        self.image_url.as_deref()
    }

    pub fn width(&self) -> Option<u32> {
        self.width
    }

    pub fn height(&self) -> Option<u32> {
        self.height
    }

    pub fn date(&self) -> Option<DateTime<Utc>> {
        self.date
    }

    pub fn source(&self) -> Option<&String> {
        self.source.as_ref()
    }

    pub fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }

    pub fn files(&self) -> &[SampleFile] {
        &self.files
    }

    pub fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }

    pub fn with_annotations(mut self, annotations: Vec<Annotation>) -> Self {
        self.annotations = annotations;
        self
    }

    pub fn with_frame_number(mut self, frame_number: Option<u32>) -> Self {
        self.frame_number = frame_number;
        self
    }

    /// Downloads a file of the specified type for this sample.
    ///
    /// Supports both newer datasets (pre-signed URLs) and legacy datasets
    /// (inline base64-encoded data):
    /// 1. First tries to download from URL if available
    /// 2. Falls back to decoding inline base64 data for legacy datasets
    pub async fn download(
        &self,
        client: &Client,
        file_type: FileType,
    ) -> Result<Option<Vec<u8>>, Error> {
        use base64::{Engine, engine::general_purpose::STANDARD};

        // Handle image type separately (uses image_url field)
        if file_type == FileType::Image {
            if let Some(url) = self.image_url.as_deref()
                && is_valid_url(url) {
                    return Ok(Some(client.download(url).await?));
                }
            return Ok(None);
        }

        // Find the matching file for this type
        let file = resolve_file(&file_type, &self.files);

        match file {
            Some(f) => {
                // Prefer URL (newer datasets)
                if let Some(url) = f.url() {
                    return Ok(Some(client.download(url).await?));
                }

                // Fall back to inline data (legacy datasets)
                if let Some(data) = f.data() {
                    // Legacy data can be in several formats:
                    // 1. Base64-encoded JSON: "eyJyYWRhci5wY2QiOi..." -> {"radar.pcd": "content"}
                    // 2. Direct JSON wrapper: {"radar.pcd": "content"}
                    // 3. Raw content (PCD text, etc.)

                    // Try base64 decode first
                    let decoded = if let Ok(bytes) = STANDARD.decode(data) {
                        // Check if decoded bytes are UTF-8 JSON
                        if let Ok(text) = String::from_utf8(bytes.clone()) {
                            if text.starts_with('{') {
                                // It's JSON - use the text for further processing
                                text
                            } else {
                                // Non-JSON binary data - return as-is
                                return Ok(Some(bytes));
                            }
                        } else {
                            // Binary data - return as-is
                            return Ok(Some(bytes));
                        }
                    } else {
                        // Not base64 - use original data
                        data.to_string()
                    };

                    // Try to unwrap JSON wrapper: {"type_name": "content"}
                    let content = if decoded.starts_with('{') {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&decoded) {
                            if let Some(obj) = json.as_object() {
                                obj.values()
                                    .next()
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or(decoded)
                            } else {
                                decoded
                            }
                        } else {
                            decoded
                        }
                    } else {
                        decoded
                    };

                    return Ok(Some(content.as_bytes().to_vec()));
                }

                Ok(None)
            }
            None => Ok(None),
        }
    }
}

/// A file associated with a sample (e.g., LiDAR point cloud, radar data).
///
/// For samples retrieved from the server, this contains the file type and URL.
/// For samples being populated to the server, this can be a type and filename.
///
/// Legacy datasets may have inline base64-encoded data instead of URLs.
/// The `data` field stores this inline content for fallback when no URL exists.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SampleFile {
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
    /// Inline base64-encoded data for legacy datasets without pre-signed URLs.
    #[serde(skip_serializing_if = "Option::is_none", skip_deserializing)]
    data: Option<String>,
    /// Raw bytes for direct upload (e.g., from ZIP archives).
    /// This field is not serialized - it's only used during the upload process.
    #[serde(skip)]
    bytes: Option<Vec<u8>>,
}

impl SampleFile {
    /// Creates a new sample file with type and URL (for newer datasets).
    pub fn with_url(file_type: String, url: String) -> Self {
        Self {
            r#type: file_type,
            url: Some(url),
            filename: None,
            data: None,
            bytes: None,
        }
    }

    /// Creates a new sample file with type and filename (for populate API).
    pub fn with_filename(file_type: String, filename: String) -> Self {
        Self {
            r#type: file_type,
            url: None,
            filename: Some(filename),
            data: None,
            bytes: None,
        }
    }

    /// Creates a new sample file with inline data (for legacy datasets).
    pub fn with_data(file_type: String, data: String) -> Self {
        Self {
            r#type: file_type,
            url: None,
            filename: None,
            data: Some(data),
            bytes: None,
        }
    }

    /// Creates a new sample file with raw bytes for direct upload.
    ///
    /// This is useful for uploading files from ZIP archives without extracting
    /// to disk first. The bytes are uploaded directly to the presigned URL.
    ///
    /// # Arguments
    /// * `file_type` - The type of file (e.g., "image", "lidar.pcd")
    /// * `filename` - The filename to use for the upload
    /// * `bytes` - The raw file bytes
    pub fn with_bytes(file_type: String, filename: String, bytes: Vec<u8>) -> Self {
        Self {
            r#type: file_type,
            url: None,
            filename: Some(filename),
            data: None,
            bytes: Some(bytes),
        }
    }

    pub fn file_type(&self) -> &str {
        &self.r#type
    }

    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Returns inline base64-encoded data (for legacy datasets).
    pub fn data(&self) -> Option<&str> {
        self.data.as_deref()
    }

    /// Returns raw bytes for direct upload (from ZIP archives, etc.).
    pub fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
}

/// Location and pose information for a sample.
///
/// Contains GPS coordinates and IMU orientation data describing where and how
/// the camera was positioned when capturing the sample.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Location {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gps: Option<GpsData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imu: Option<ImuData>,
}

/// GPS location data (latitude and longitude).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GpsData {
    pub lat: f64,
    pub lon: f64,
}

impl GpsData {
    /// Validate GPS coordinates are within valid ranges.
    ///
    /// Checks if latitude and longitude values are within valid geographic
    /// ranges. Helps catch data corruption or API issues early.
    ///
    /// # Returns
    /// `Ok(())` if valid, `Err(String)` with descriptive error message
    /// otherwise
    ///
    /// # Valid Ranges
    /// - Latitude: -90.0 to +90.0 degrees
    /// - Longitude: -180.0 to +180.0 degrees
    ///
    /// # Examples
    /// ```
    /// use edgefirst_client::GpsData;
    ///
    /// let gps = GpsData {
    ///     lat: 37.7749,
    ///     lon: -122.4194,
    /// };
    /// assert!(gps.validate().is_ok());
    ///
    /// let bad_gps = GpsData {
    ///     lat: 100.0,
    ///     lon: 0.0,
    /// };
    /// assert!(bad_gps.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        validate_gps_coordinates(self.lat, self.lon)
    }
}

/// IMU orientation data (roll, pitch, yaw in degrees).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImuData {
    pub roll: f64,
    pub pitch: f64,
    pub yaw: f64,
}

impl ImuData {
    /// Validate IMU orientation angles are within valid ranges.
    ///
    /// Checks if roll, pitch, and yaw values are finite and within reasonable
    /// ranges. Helps catch data corruption or sensor errors early.
    ///
    /// # Returns
    /// `Ok(())` if valid, `Err(String)` with descriptive error message
    /// otherwise
    ///
    /// # Valid Ranges
    /// - Roll: -180.0 to +180.0 degrees
    /// - Pitch: -90.0 to +90.0 degrees (typical gimbal lock range)
    /// - Yaw: -180.0 to +180.0 degrees (or 0 to 360, normalized)
    ///
    /// # Examples
    /// ```
    /// use edgefirst_client::ImuData;
    ///
    /// let imu = ImuData {
    ///     roll: 10.0,
    ///     pitch: 5.0,
    ///     yaw: 90.0,
    /// };
    /// assert!(imu.validate().is_ok());
    ///
    /// let bad_imu = ImuData {
    ///     roll: 200.0,
    ///     pitch: 0.0,
    ///     yaw: 0.0,
    /// };
    /// assert!(bad_imu.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        validate_imu_orientation(self.roll, self.pitch, self.yaw)
    }
}

#[allow(dead_code)]
pub trait TypeName {
    fn type_name() -> String;
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Box3d {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    h: f32,
    l: f32,
}

impl TypeName for Box3d {
    fn type_name() -> String {
        "box3d".to_owned()
    }
}

impl Box3d {
    pub fn new(cx: f32, cy: f32, cz: f32, width: f32, height: f32, length: f32) -> Self {
        Self {
            x: cx,
            y: cy,
            z: cz,
            w: width,
            h: height,
            l: length,
        }
    }

    pub fn width(&self) -> f32 {
        self.w
    }

    pub fn height(&self) -> f32 {
        self.h
    }

    pub fn length(&self) -> f32 {
        self.l
    }

    pub fn cx(&self) -> f32 {
        self.x
    }

    pub fn cy(&self) -> f32 {
        self.y
    }

    pub fn cz(&self) -> f32 {
        self.z
    }

    pub fn left(&self) -> f32 {
        self.x - self.w / 2.0
    }

    pub fn top(&self) -> f32 {
        self.y - self.h / 2.0
    }

    pub fn front(&self) -> f32 {
        self.z - self.l / 2.0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Box2d {
    h: f32,
    w: f32,
    x: f32,
    y: f32,
}

impl TypeName for Box2d {
    fn type_name() -> String {
        "box2d".to_owned()
    }
}

impl Box2d {
    pub fn new(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self {
            x: left,
            y: top,
            w: width,
            h: height,
        }
    }

    pub fn width(&self) -> f32 {
        self.w
    }

    pub fn height(&self) -> f32 {
        self.h
    }

    pub fn left(&self) -> f32 {
        self.x
    }

    pub fn top(&self) -> f32 {
        self.y
    }

    pub fn cx(&self) -> f32 {
        self.x + self.w / 2.0
    }

    pub fn cy(&self) -> f32 {
        self.y + self.h / 2.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub polygon: Vec<Vec<(f32, f32)>>,
}

impl TypeName for Mask {
    fn type_name() -> String {
        "mask".to_owned()
    }
}

impl Mask {
    pub fn new(polygon: Vec<Vec<(f32, f32)>>) -> Self {
        Self { polygon }
    }
}

impl serde::Serialize for Mask {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(&self.polygon, serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Mask {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // First, deserialize to a raw JSON value to handle various formats
        let value = serde_json::Value::deserialize(deserializer)?;

        // Try to extract polygon data from various formats
        let polygon_value = if let Some(obj) = value.as_object() {
            // Format: {"polygon": [...]}
            obj.get("polygon")
                .cloned()
                .unwrap_or(serde_json::Value::Null)
        } else {
            // Format: [[...]] (direct array)
            value
        };

        // Parse the polygon array, filtering out null/invalid values
        let polygon = parse_polygon_value(&polygon_value);

        Ok(Self { polygon })
    }
}

/// Parse polygon value from JSON, handling malformed data gracefully.
///
/// Handles multiple formats:
/// - `[[[x,y],[x,y],...]]` - 3D array with point pairs (correct format)
/// - `[[x,y,x,y,...]]` - 2D array with flat coords (COCO format, legacy)
/// - `[[null,null,...]]` - corrupted data (returns empty)
/// - `null` - missing data (returns empty)
fn parse_polygon_value(value: &serde_json::Value) -> Vec<Vec<(f32, f32)>> {
    let Some(outer_array) = value.as_array() else {
        return vec![];
    };

    let mut result = Vec::new();

    for ring in outer_array {
        let Some(ring_array) = ring.as_array() else {
            continue;
        };

        // Check if this is a 3D array (point pairs) or 2D array (flat coords)
        let is_3d = ring_array
            .first()
            .map(|first| first.is_array())
            .unwrap_or(false);

        let points: Vec<(f32, f32)> = if is_3d {
            // 3D format: [[x1,y1], [x2,y2], ...]
            ring_array
                .iter()
                .filter_map(|point| {
                    let arr = point.as_array()?;
                    if arr.len() >= 2 {
                        let x = arr[0].as_f64()? as f32;
                        let y = arr[1].as_f64()? as f32;
                        if x.is_finite() && y.is_finite() {
                            Some((x, y))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            // 2D format (flat): [x1, y1, x2, y2, ...]
            ring_array
                .chunks(2)
                .filter_map(|chunk| {
                    if chunk.len() >= 2 {
                        let x = chunk[0].as_f64()? as f32;
                        let y = chunk[1].as_f64()? as f32;
                        if x.is_finite() && y.is_finite() {
                            Some((x, y))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        };

        // Only add rings with at least 3 valid points
        if points.len() >= 3 {
            result.push(points);
        }
    }

    result
}

/// Helper struct for deserializing annotations from the server.
///
/// The server sends bounding box coordinates as flat fields (x, y, w, h) at the
/// annotation level, but we want to store them as a nested Box2d struct.
#[derive(Deserialize)]
struct AnnotationRaw {
    #[serde(default)]
    sample_id: Option<SampleID>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    sequence_name: Option<String>,
    #[serde(default)]
    frame_number: Option<u32>,
    #[serde(rename = "group_name", default)]
    group: Option<String>,
    #[serde(rename = "object_reference", alias = "object_id", default)]
    object_id: Option<String>,
    #[serde(default)]
    label_name: Option<String>,
    #[serde(default)]
    label_index: Option<u64>,
    // Nested box2d format (if server sends it this way)
    #[serde(default)]
    box2d: Option<Box2d>,
    #[serde(default)]
    box3d: Option<Box3d>,
    #[serde(default)]
    mask: Option<Mask>,
    // Flat box2d fields from server (x, y, w, h at annotation level)
    #[serde(default)]
    x: Option<f64>,
    #[serde(default)]
    y: Option<f64>,
    #[serde(default)]
    w: Option<f64>,
    #[serde(default)]
    h: Option<f64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Annotation {
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_id: Option<SampleID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frame_number: Option<u32>,
    /// Dataset split (train, val, test) - matches `Sample.group`.
    /// JSON field name: "group_name" (Studio API uses this name for both upload
    /// and download).
    #[serde(rename = "group_name", skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    /// Object tracking identifier across frames.
    /// JSON field name: "object_reference" for upload (populate), "object_id"
    /// for download (list).
    #[serde(
        rename = "object_reference",
        alias = "object_id",
        skip_serializing_if = "Option::is_none"
    )]
    object_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box2d: Option<Box2d>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box3d: Option<Box3d>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mask: Option<Mask>,
}

impl<'de> serde::Deserialize<'de> for Annotation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize to AnnotationRaw first to handle server format differences
        let raw: AnnotationRaw = serde::Deserialize::deserialize(deserializer)?;

        // Prefer nested box2d if present, otherwise construct from flat x/y/w/h
        let box2d = raw.box2d.or_else(|| match (raw.x, raw.y, raw.w, raw.h) {
            (Some(x), Some(y), Some(w), Some(h)) if w > 0.0 && h > 0.0 => {
                Some(Box2d::new(x as f32, y as f32, w as f32, h as f32))
            }
            _ => None,
        });

        Ok(Annotation {
            sample_id: raw.sample_id,
            name: raw.name,
            sequence_name: raw.sequence_name,
            frame_number: raw.frame_number,
            group: raw.group,
            object_id: raw.object_id,
            label_name: raw.label_name,
            label_index: raw.label_index,
            box2d,
            box3d: raw.box3d,
            mask: raw.mask,
        })
    }
}

impl Default for Annotation {
    fn default() -> Self {
        Self::new()
    }
}

impl Annotation {
    pub fn new() -> Self {
        Self {
            sample_id: None,
            name: None,
            sequence_name: None,
            frame_number: None,
            group: None,
            object_id: None,
            label_name: None,
            label_index: None,
            box2d: None,
            box3d: None,
            mask: None,
        }
    }

    pub fn set_sample_id(&mut self, sample_id: Option<SampleID>) {
        self.sample_id = sample_id;
    }

    pub fn sample_id(&self) -> Option<SampleID> {
        self.sample_id
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    pub fn set_sequence_name(&mut self, sequence_name: Option<String>) {
        self.sequence_name = sequence_name;
    }

    pub fn sequence_name(&self) -> Option<&String> {
        self.sequence_name.as_ref()
    }

    pub fn set_frame_number(&mut self, frame_number: Option<u32>) {
        self.frame_number = frame_number;
    }

    pub fn frame_number(&self) -> Option<u32> {
        self.frame_number
    }

    pub fn set_group(&mut self, group: Option<String>) {
        self.group = group;
    }

    pub fn group(&self) -> Option<&String> {
        self.group.as_ref()
    }

    pub fn object_id(&self) -> Option<&String> {
        self.object_id.as_ref()
    }

    pub fn set_object_id(&mut self, object_id: Option<String>) {
        self.object_id = object_id;
    }

    #[deprecated(note = "renamed to object_id")]
    pub fn object_reference(&self) -> Option<&String> {
        self.object_id()
    }

    #[deprecated(note = "renamed to set_object_id")]
    pub fn set_object_reference(&mut self, object_reference: Option<String>) {
        self.set_object_id(object_reference);
    }

    pub fn label(&self) -> Option<&String> {
        self.label_name.as_ref()
    }

    pub fn set_label(&mut self, label_name: Option<String>) {
        self.label_name = label_name;
    }

    pub fn label_index(&self) -> Option<u64> {
        self.label_index
    }

    pub fn set_label_index(&mut self, label_index: Option<u64>) {
        self.label_index = label_index;
    }

    pub fn box2d(&self) -> Option<&Box2d> {
        self.box2d.as_ref()
    }

    pub fn set_box2d(&mut self, box2d: Option<Box2d>) {
        self.box2d = box2d;
    }

    pub fn box3d(&self) -> Option<&Box3d> {
        self.box3d.as_ref()
    }

    pub fn set_box3d(&mut self, box3d: Option<Box3d>) {
        self.box3d = box3d;
    }

    pub fn mask(&self) -> Option<&Mask> {
        self.mask.as_ref()
    }

    pub fn set_mask(&mut self, mask: Option<Mask>) {
        self.mask = mask;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Label {
    id: u64,
    dataset_id: DatasetID,
    index: u64,
    name: String,
}

impl Label {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn dataset_id(&self) -> DatasetID {
        self.dataset_id
    }

    pub fn index(&self) -> u64 {
        self.index
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn remove(&self, client: &Client) -> Result<(), Error> {
        client.remove_label(self.id()).await
    }

    pub async fn set_name(&mut self, client: &Client, name: &str) -> Result<(), Error> {
        self.name = name.to_string();
        client.update_label(self).await
    }

    pub async fn set_index(&mut self, client: &Client, index: u64) -> Result<(), Error> {
        self.index = index;
        client.update_label(self).await
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct NewLabelObject {
    pub name: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct NewLabel {
    pub dataset_id: DatasetID,
    pub labels: Vec<NewLabelObject>,
}

/// A dataset group for organizing samples into logical subsets.
///
/// Groups are used to partition samples within a dataset for different purposes
/// such as training, validation, and testing. Each sample can belong to at most
/// one group at a time.
///
/// # Common Group Names
///
/// - `"train"` - Training data for model fitting
/// - `"val"` - Validation data for hyperparameter tuning
/// - `"test"` - Test data for final evaluation
///
/// # Examples
///
/// ```rust,no_run
/// use edgefirst_client::{Client, DatasetID};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new()?.with_token_path(None)?;
/// let dataset_id: DatasetID = "ds-123".try_into()?;
///
/// // List all groups in the dataset
/// let groups = client.groups(dataset_id).await?;
/// for group in groups {
///     println!("Group [{}]: {}", group.id, group.name);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Group {
    /// The unique numeric identifier for this group.
    ///
    /// Group IDs are assigned by the server and are unique within an
    /// organization.
    pub id: u64,

    /// The human-readable name of the group.
    ///
    /// Common names include "train", "val", "test", but any string is valid.
    pub name: String,
}

#[cfg(feature = "polars")]
fn extract_annotation_name(ann: &Annotation) -> Option<(String, Option<u32>)> {
    use std::path::Path;

    let name = ann.name.as_ref()?;
    let name = Path::new(name).file_stem()?.to_str()?;

    // For sequences, return base name and frame number
    // For non-sequences, return name and None
    match &ann.sequence_name {
        Some(sequence) => Some((sequence.clone(), ann.frame_number)),
        None => Some((name.to_string(), None)),
    }
}

#[cfg(feature = "polars")]
fn convert_mask_to_series(mask: &Mask) -> Series {
    use polars::series::Series;

    let list = flatten_polygon_coordinates(&mask.polygon);
    Series::new("mask".into(), list)
}

/// Create a DataFrame from a slice of annotations (2025.01 schema).
///
/// **DEPRECATED**: Use [`samples_dataframe()`] instead for full 2025.10 schema
/// support including optional metadata columns (size, location, pose,
/// degradation).
///
/// This function generates a DataFrame with the original 9-column schema.
/// It remains functional for backward compatibility but does not include
/// new optional columns added in version 2025.10.
///
/// # Schema (2025.01)
///
/// - `name`: Sample name (String)
/// - `frame`: Frame number (UInt64)
/// - `object_id`: Object tracking ID (String)
/// - `label`: Object label (Categorical)
/// - `label_index`: Label index (UInt64)
/// - `group`: Dataset group (Categorical)
/// - `mask`: Segmentation mask (List<Float32>)
/// - `box2d`: 2D bounding box [cx, cy, w, h] (Array<Float32, 4>)
/// - `box3d`: 3D bounding box [x, y, z, w, h, l] (Array<Float32, 6>)
///
/// # Migration
///
/// ```rust,no_run
/// use edgefirst_client::{Client, samples_dataframe};
///
/// # async fn example() -> Result<(), edgefirst_client::Error> {
/// # let client = Client::new()?;
/// # let dataset_id = 1.into();
/// # let annotation_set_id = 1.into();
/// # let groups = vec![];
/// # let types = vec![];
/// // OLD (deprecated):
/// let annotations = client
///     .annotations(annotation_set_id, &groups, &types, None)
///     .await?;
/// let df = edgefirst_client::annotations_dataframe(&annotations)?;
///
/// // NEW (recommended):
/// let samples = client
///     .samples(
///         dataset_id,
///         Some(annotation_set_id),
///         &types,
///         &groups,
///         &[],
///         None,
///     )
///     .await?;
/// let df = samples_dataframe(&samples)?;
/// # Ok(())
/// # }
/// ```
#[deprecated(
    since = "0.8.0",
    note = "Use `samples_dataframe()` for complete 2025.10 schema support"
)]
#[cfg(feature = "polars")]
pub fn annotations_dataframe(annotations: &[Annotation]) -> Result<DataFrame, Error> {
    use itertools::Itertools;

    let (names, frames, objects, labels, label_indices, groups, masks, boxes2d, boxes3d) =
        annotations
            .iter()
            .filter_map(|ann| {
                let (name, frame) = extract_annotation_name(ann)?;

                let masks = ann.mask.as_ref().map(convert_mask_to_series);

                let box2d = ann.box2d.as_ref().map(|box2d| {
                    Series::new(
                        "box2d".into(),
                        [box2d.cx(), box2d.cy(), box2d.width(), box2d.height()],
                    )
                });

                let box3d = ann.box3d.as_ref().map(|box3d| {
                    Series::new(
                        "box3d".into(),
                        [box3d.x, box3d.y, box3d.z, box3d.w, box3d.h, box3d.l],
                    )
                });

                Some((
                    name,
                    frame,
                    ann.object_id().cloned(),
                    ann.label_name.clone(),
                    ann.label_index,
                    ann.group.clone(),
                    masks,
                    box2d,
                    box3d,
                ))
            })
            .multiunzip::<(
                Vec<_>, // names
                Vec<_>, // frames
                Vec<_>, // objects
                Vec<_>, // labels
                Vec<_>, // label_indices
                Vec<_>, // groups
                Vec<_>, // masks
                Vec<_>, // boxes2d
                Vec<_>, // boxes3d
            )>();
    let names = Series::new("name".into(), names).into();
    let frames = Series::new("frame".into(), frames).into();
    let objects = Series::new("object_id".into(), objects).into();
    let labels = Series::new("label".into(), labels)
        .cast(&DataType::Categorical(
            Categories::new("labels".into(), "labels".into(), CategoricalPhysical::U8),
            Arc::new(CategoricalMapping::new(u8::MAX as usize)),
        ))?
        .into();
    let label_indices = Series::new("label_index".into(), label_indices).into();
    let groups = Series::new("group".into(), groups)
        .cast(&DataType::Categorical(
            Categories::new("groups".into(), "groups".into(), CategoricalPhysical::U8),
            Arc::new(CategoricalMapping::new(u8::MAX as usize)),
        ))?
        .into();
    let masks = Series::new("mask".into(), masks)
        .cast(&DataType::List(Box::new(DataType::Float32)))?
        .into();
    let boxes2d = Series::new("box2d".into(), boxes2d)
        .cast(&DataType::Array(Box::new(DataType::Float32), 4))?
        .into();
    let boxes3d = Series::new("box3d".into(), boxes3d)
        .cast(&DataType::Array(Box::new(DataType::Float32), 6))?
        .into();

    Ok(DataFrame::new(vec![
        names,
        frames,
        objects,
        labels,
        label_indices,
        groups,
        masks,
        boxes2d,
        boxes3d,
    ])?)
}

/// Create a DataFrame from a slice of samples with complete 2025.10 schema.
///
/// This function generates a DataFrame with all 13 columns including optional
/// sample metadata (size, location, pose, degradation). Each annotation in
/// each sample becomes one row in the DataFrame.
///
/// # Schema (2025.10)
///
/// - `name`: Sample name (String)
/// - `frame`: Frame number (UInt64)
/// - `object_id`: Object tracking ID (String)
/// - `label`: Object label (Categorical)
/// - `label_index`: Label index (UInt64)
/// - `group`: Dataset group (Categorical)
/// - `mask`: Segmentation mask (List<Float32>)
/// - `box2d`: 2D bounding box [cx, cy, w, h] (Array<Float32, 4>)
/// - `box3d`: 3D bounding box [x, y, z, w, h, l] (Array<Float32, 6>)
/// - `size`: Image size [width, height] (Array<UInt32, 2>) - OPTIONAL
/// - `location`: GPS [lat, lon] (Array<Float32, 2>) - OPTIONAL
/// - `pose`: IMU [yaw, pitch, roll] (Array<Float32, 3>) - OPTIONAL
/// - `degradation`: Image degradation (String) - OPTIONAL
///
/// # Example
///
/// ```rust,no_run
/// use edgefirst_client::{Client, samples_dataframe};
///
/// # async fn example() -> Result<(), edgefirst_client::Error> {
/// # let client = Client::new()?;
/// # let dataset_id = 1.into();
/// # let annotation_set_id = 1.into();
/// let samples = client
///     .samples(dataset_id, Some(annotation_set_id), &[], &[], &[], None)
///     .await?;
/// let df = samples_dataframe(&samples)?;
/// println!("DataFrame shape: {:?}", df.shape());
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "polars")]
pub fn samples_dataframe(samples: &[Sample]) -> Result<DataFrame, Error> {
    // Flatten samples into annotation rows with sample metadata
    let rows: Vec<_> = samples
        .iter()
        .flat_map(|sample| {
            // Extract sample metadata once per sample
            let size = match (sample.width, sample.height) {
                (Some(w), Some(h)) => Some(vec![w, h]),
                _ => None,
            };

            let location = sample.location.as_ref().and_then(|loc| {
                loc.gps
                    .as_ref()
                    .map(|gps| vec![gps.lat as f32, gps.lon as f32])
            });

            let pose = sample.location.as_ref().and_then(|loc| {
                loc.imu
                    .as_ref()
                    .map(|imu| vec![imu.yaw as f32, imu.pitch as f32, imu.roll as f32])
            });

            let degradation = sample.degradation.clone();

            // If no annotations, create one row for the sample (null annotations)
            if sample.annotations.is_empty() {
                let (name, frame) = match extract_annotation_name_from_sample(sample) {
                    Some(nf) => nf,
                    None => return vec![],
                };

                return vec![(
                    name,
                    frame,
                    None,                 // object_id placeholder for now
                    None,                 // label
                    None,                 // label_index
                    sample.group.clone(), // group
                    None,                 // mask
                    None,                 // box2d
                    None,                 // box3d
                    size.clone(),
                    location.clone(),
                    pose.clone(),
                    degradation.clone(),
                )];
            }

            // Create one row per annotation
            sample
                .annotations
                .iter()
                .filter_map(|ann| {
                    let (name, frame) = extract_annotation_name(ann)?;

                    let mask = ann.mask.as_ref().map(convert_mask_to_series);

                    let box2d = ann.box2d.as_ref().map(|box2d| {
                        Series::new(
                            "box2d".into(),
                            [box2d.cx(), box2d.cy(), box2d.width(), box2d.height()],
                        )
                    });

                    let box3d = ann.box3d.as_ref().map(|box3d| {
                        Series::new(
                            "box3d".into(),
                            [box3d.x, box3d.y, box3d.z, box3d.w, box3d.h, box3d.l],
                        )
                    });

                    Some((
                        name,
                        frame,
                        ann.object_id().cloned(),
                        ann.label_name.clone(),
                        ann.label_index,
                        sample.group.clone(), // Group is on Sample, not Annotation
                        mask,
                        box2d,
                        box3d,
                        size.clone(),
                        location.clone(),
                        pose.clone(),
                        degradation.clone(),
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect();

    // Manually unzip into separate vectors
    let mut names = Vec::new();
    let mut frames = Vec::new();
    let mut objects = Vec::new();
    let mut labels = Vec::new();
    let mut label_indices = Vec::new();
    let mut groups = Vec::new();
    let mut masks = Vec::new();
    let mut boxes2d = Vec::new();
    let mut boxes3d = Vec::new();
    let mut sizes = Vec::new();
    let mut locations = Vec::new();
    let mut poses = Vec::new();
    let mut degradations = Vec::new();

    for (
        name,
        frame,
        object,
        label,
        label_index,
        group,
        mask,
        box2d,
        box3d,
        size,
        location,
        pose,
        degradation,
    ) in rows
    {
        names.push(name);
        frames.push(frame);
        objects.push(object);
        labels.push(label);
        label_indices.push(label_index);
        groups.push(group);
        masks.push(mask);
        boxes2d.push(box2d);
        boxes3d.push(box3d);
        sizes.push(size);
        locations.push(location);
        poses.push(pose);
        degradations.push(degradation);
    }

    // Build DataFrame columns
    let names = Series::new("name".into(), names).into();
    let frames = Series::new("frame".into(), frames).into();
    let objects = Series::new("object_id".into(), objects).into();

    // Column name: "label" (NOT "label_name")
    let labels = Series::new("label".into(), labels)
        .cast(&DataType::Categorical(
            Categories::new("labels".into(), "labels".into(), CategoricalPhysical::U8),
            Arc::new(CategoricalMapping::new(u8::MAX as usize)),
        ))?
        .into();

    let label_indices = Series::new("label_index".into(), label_indices).into();

    // Column name: "group" (NOT "group_name")
    let groups = Series::new("group".into(), groups)
        .cast(&DataType::Categorical(
            Categories::new("groups".into(), "groups".into(), CategoricalPhysical::U8),
            Arc::new(CategoricalMapping::new(u8::MAX as usize)),
        ))?
        .into();

    let masks = Series::new("mask".into(), masks)
        .cast(&DataType::List(Box::new(DataType::Float32)))?
        .into();
    let boxes2d = Series::new("box2d".into(), boxes2d)
        .cast(&DataType::Array(Box::new(DataType::Float32), 4))?
        .into();
    let boxes3d = Series::new("box3d".into(), boxes3d)
        .cast(&DataType::Array(Box::new(DataType::Float32), 6))?
        .into();

    // NEW: Optional columns (2025.10)
    // Convert Vec<Option<Vec<T>>> to Vec<Option<Series>> for array columns
    let size_series: Vec<Option<Series>> = sizes
        .into_iter()
        .map(|opt_vec| opt_vec.map(|vec| Series::new("size".into(), vec)))
        .collect();
    let sizes = Series::new("size".into(), size_series)
        .cast(&DataType::Array(Box::new(DataType::UInt32), 2))?
        .into();

    let location_series: Vec<Option<Series>> = locations
        .into_iter()
        .map(|opt_vec| opt_vec.map(|vec| Series::new("location".into(), vec)))
        .collect();
    let locations = Series::new("location".into(), location_series)
        .cast(&DataType::Array(Box::new(DataType::Float32), 2))?
        .into();

    let pose_series: Vec<Option<Series>> = poses
        .into_iter()
        .map(|opt_vec| opt_vec.map(|vec| Series::new("pose".into(), vec)))
        .collect();
    let poses = Series::new("pose".into(), pose_series)
        .cast(&DataType::Array(Box::new(DataType::Float32), 3))?
        .into();

    let degradations = Series::new("degradation".into(), degradations).into();

    Ok(DataFrame::new(vec![
        names,
        frames,
        objects,
        labels,
        label_indices,
        groups,
        masks,
        boxes2d,
        boxes3d,
        sizes,
        locations,
        poses,
        degradations,
    ])?)
}

// Helper: Extract name/frame from Sample (for samples with no annotations)
#[cfg(feature = "polars")]
fn extract_annotation_name_from_sample(sample: &Sample) -> Option<(String, Option<u32>)> {
    use std::path::Path;

    let name = sample.image_name.as_ref()?;
    let name = Path::new(name).file_stem()?.to_str()?;

    // For sequences, return base name and frame number
    // For non-sequences, return name and None
    match &sample.sequence_name {
        Some(sequence) => Some((sequence.clone(), sample.frame_number)),
        None => Some((name.to_string(), None)),
    }
}

// ============================================================================
// PURE FUNCTIONS FOR TESTABLE CORE LOGIC
// ============================================================================

/// Extract sample name from image filename by:
/// 1. Removing file extension (everything after last dot)
/// 2. Removing .camera suffix if present
///
/// # Examples
/// - "scene_001.camera.jpg" → "scene_001"
/// - "image.jpg" → "image"
/// - ".jpg" → ".jpg" (preserves filenames starting with dot)
fn extract_sample_name(image_name: &str) -> String {
    // Step 1: Remove file extension (but preserve filenames starting with dot)
    let name = image_name
        .rsplit_once('.')
        .and_then(|(name, _)| {
            // Only remove extension if the name part is non-empty (handles ".jpg" case)
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .unwrap_or_else(|| image_name.to_string());

    // Step 2: Remove .camera suffix if present
    name.rsplit_once(".camera")
        .and_then(|(name, _)| {
            // Only remove .camera if the name part is non-empty
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .unwrap_or_else(|| name.clone())
}

/// Resolve a file for a given file type from sample data.
///
/// Returns the matching `SampleFile` if found, which may contain either
/// a URL (newer datasets) or inline data (legacy datasets).
///
/// # Arguments
/// * `file_type` - The type of file to resolve (e.g., LidarPcd, RadarPcd)
/// * `files` - The sample's file list
fn resolve_file<'a>(file_type: &FileType, files: &'a [SampleFile]) -> Option<&'a SampleFile> {
    match file_type {
        FileType::Image => None, // Image uses image_url field, not files
        FileType::All => None,   // All should be expanded before calling this
        file => {
            // Get all possible names for this file type (primary + aliases)
            let type_names = file_type_names(file);
            files
                .iter()
                .find(|f| type_names.contains(&f.r#type.as_str()))
        }
    }
}

/// Returns all possible server-side names for a file type.
/// The server uses specific naming conventions in the STUDIO_DB_TYPE_MAP.
fn file_type_names(file_type: &FileType) -> Vec<&'static str> {
    match file_type {
        FileType::Image => vec!["image"],
        FileType::LidarPcd => vec!["lidar.pcd"],
        FileType::LidarDepth => vec!["lidar.depth", "depth.png", "depthmap"],
        FileType::LidarReflect => vec!["lidar.reflect"],
        FileType::RadarPcd => vec!["radar.pcd", "pcd"],
        FileType::RadarCube => vec!["radar.png", "cube"],
        FileType::All => vec![],
    }
}

// ============================================================================
// DESERIALIZATION FORMAT CONVERSION HELPERS
// ============================================================================

/// Convert annotations grouped format to flat Vec<Annotation>.
///
/// Pure function that handles the conversion from the server's legacy format
/// (HashMap<String, Vec<Annotation>>) to the flat Vec<Annotation>
/// representation.
///
/// # Arguments
/// * `map` - HashMap where keys are annotation types ("bbox", "box3d", "mask")
fn convert_annotations_map_to_vec(map: HashMap<String, Vec<Annotation>>) -> Vec<Annotation> {
    let mut all_annotations = Vec::new();
    if let Some(bbox_anns) = map.get("bbox") {
        all_annotations.extend(bbox_anns.clone());
    }
    if let Some(box3d_anns) = map.get("box3d") {
        all_annotations.extend(box3d_anns.clone());
    }
    if let Some(mask_anns) = map.get("mask") {
        all_annotations.extend(mask_anns.clone());
    }
    all_annotations
}

// ============================================================================
// GPS/IMU VALIDATION HELPERS
// ============================================================================

/// Validate GPS coordinates are within valid ranges.
///
/// Pure function that checks if latitude and longitude values are within valid
/// geographic ranges. Helps catch data corruption or API issues early.
///
/// # Arguments
/// * `lat` - Latitude in degrees
/// * `lon` - Longitude in degrees
///
/// # Returns
/// `Ok(())` if valid, `Err(String)` with descriptive error message otherwise
///
/// # Valid Ranges
/// - Latitude: -90.0 to +90.0 degrees
/// - Longitude: -180.0 to +180.0 degrees
fn validate_gps_coordinates(lat: f64, lon: f64) -> Result<(), String> {
    if !lat.is_finite() {
        return Err(format!("GPS latitude is not finite: {}", lat));
    }
    if !lon.is_finite() {
        return Err(format!("GPS longitude is not finite: {}", lon));
    }
    if !(-90.0..=90.0).contains(&lat) {
        return Err(format!("GPS latitude out of range [-90, 90]: {}", lat));
    }
    if !(-180.0..=180.0).contains(&lon) {
        return Err(format!("GPS longitude out of range [-180, 180]: {}", lon));
    }
    Ok(())
}

/// Validate IMU orientation angles are within valid ranges.
///
/// Pure function that checks if roll, pitch, and yaw values are finite and
/// within reasonable ranges. Helps catch data corruption or sensor errors
/// early.
///
/// # Arguments
/// * `roll` - Roll angle in degrees
/// * `pitch` - Pitch angle in degrees
/// * `yaw` - Yaw angle in degrees
///
/// # Returns
/// `Ok(())` if valid, `Err(String)` with descriptive error message otherwise
///
/// # Valid Ranges
/// - Roll: -180.0 to +180.0 degrees
/// - Pitch: -90.0 to +90.0 degrees (typical gimbal lock range)
/// - Yaw: -180.0 to +180.0 degrees (or 0 to 360, normalized)
fn validate_imu_orientation(roll: f64, pitch: f64, yaw: f64) -> Result<(), String> {
    if !roll.is_finite() {
        return Err(format!("IMU roll is not finite: {}", roll));
    }
    if !pitch.is_finite() {
        return Err(format!("IMU pitch is not finite: {}", pitch));
    }
    if !yaw.is_finite() {
        return Err(format!("IMU yaw is not finite: {}", yaw));
    }
    if !(-180.0..=180.0).contains(&roll) {
        return Err(format!("IMU roll out of range [-180, 180]: {}", roll));
    }
    if !(-90.0..=90.0).contains(&pitch) {
        return Err(format!("IMU pitch out of range [-90, 90]: {}", pitch));
    }
    if !(-180.0..=180.0).contains(&yaw) {
        return Err(format!("IMU yaw out of range [-180, 180]: {}", yaw));
    }
    Ok(())
}

// ============================================================================
// MASK POLYGON CONVERSION HELPERS
// ============================================================================

/// Flatten polygon coordinates into a flat vector of f32 values for Polars
/// Series.
///
/// Converts nested polygon structure into a flat list of coordinates with
/// NaN separators between polygons:
/// - Input: [[(x1, y1), (x2, y2)], [(x3, y3)]]
/// - Output: [x1, y1, x2, y2, NaN, x3, y3]
#[cfg(feature = "polars")]
fn flatten_polygon_coordinates(polygons: &[Vec<(f32, f32)>]) -> Vec<f32> {
    let mut list = Vec::new();

    for polygon in polygons {
        for &(x, y) in polygon {
            list.push(x);
            list.push(y);
        }
        // Separate polygons with NaN
        if !polygons.is_empty() {
            list.push(f32::NAN);
        }
    }

    // Remove the last NaN if it exists (trailing separator not needed)
    if !list.is_empty() && list[list.len() - 1].is_nan() {
        list.pop();
    }

    list
}

/// Unflatten coordinates with NaN separators back to nested polygon
/// structure.
///
/// Converts flat list of coordinates with NaN separators back to nested
/// polygon structure (inverse of flatten_polygon_coordinates):
/// - Input: [x1, y1, x2, y2, NaN, x3, y3]
/// - Output: [[(x1, y1), (x2, y2)], [(x3, y3)]]
///
/// This function is used when parsing Arrow files to reconstruct the nested
/// polygon format required by the EdgeFirst Studio API.
///
/// # Examples
///
/// ```rust
/// use edgefirst_client::unflatten_polygon_coordinates;
///
/// let coords = vec![1.0, 2.0, 3.0, 4.0, f32::NAN, 5.0, 6.0];
/// let polygons = unflatten_polygon_coordinates(&coords);
///
/// assert_eq!(polygons.len(), 2);
/// assert_eq!(polygons[0], vec![(1.0, 2.0), (3.0, 4.0)]);
/// assert_eq!(polygons[1], vec![(5.0, 6.0)]);
/// ```
#[cfg(feature = "polars")]
pub fn unflatten_polygon_coordinates(coords: &[f32]) -> Vec<Vec<(f32, f32)>> {
    let mut polygons = Vec::new();
    let mut current_polygon = Vec::new();
    let mut i = 0;

    while i < coords.len() {
        if coords[i].is_nan() {
            // NaN separator - save current polygon and start new one
            if !current_polygon.is_empty() {
                polygons.push(current_polygon.clone());
                current_polygon.clear();
            }
            i += 1;
        } else if i + 1 < coords.len() {
            // Have both x and y coordinates
            current_polygon.push((coords[i], coords[i + 1]));
            i += 2;
        } else {
            // Odd number of coordinates (malformed data) - skip last value
            i += 1;
        }
    }

    // Save the last polygon if not empty
    if !current_polygon.is_empty() {
        polygons.push(current_polygon);
    }

    polygons
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // ============================================================================
    // TEST HELPER FUNCTIONS (Pure Logic for Testing)
    // ============================================================================

    /// Flatten legacy grouped annotation format to a single vector.
    ///
    /// Converts HashMap<String, Vec<Annotation>> (with bbox/box3d/mask keys)
    /// into a flat Vec<Annotation> in deterministic order.
    fn flatten_annotation_map(
        map: std::collections::HashMap<String, Vec<Annotation>>,
    ) -> Vec<Annotation> {
        let mut all_annotations = Vec::new();

        // Process in fixed order for deterministic results
        for key in ["bbox", "box3d", "mask"] {
            if let Some(mut anns) = map.get(key).cloned() {
                all_annotations.append(&mut anns);
            }
        }

        all_annotations
    }

    /// Get the JSON field name for the Annotation group field (for tests).
    fn annotation_group_field_name() -> &'static str {
        "group_name"
    }

    /// Get the JSON field name for the Annotation object_id field (for tests).
    fn annotation_object_id_field_name() -> &'static str {
        "object_reference"
    }

    /// Get the accepted alias for the Annotation object_id field (for tests).
    fn annotation_object_id_alias() -> &'static str {
        "object_id"
    }

    /// Validate that annotation field names match expected values in JSON (for
    /// tests).
    fn validate_annotation_field_names(
        json_str: &str,
        expected_group: bool,
        expected_object_ref: bool,
    ) -> Result<(), String> {
        if expected_group && !json_str.contains("\"group_name\"") {
            return Err("Missing expected field: group_name".to_string());
        }
        if expected_object_ref && !json_str.contains("\"object_reference\"") {
            return Err("Missing expected field: object_reference".to_string());
        }
        Ok(())
    }

    // ==== FileType Conversion Tests ====
    #[test]
    fn test_file_type_conversions() {
        // to_string() returns server API type names
        let api_cases = vec![
            (FileType::Image, "image"),
            (FileType::LidarPcd, "lidar.pcd"),
            (FileType::LidarDepth, "lidar.depth"),
            (FileType::LidarReflect, "lidar.reflect"),
            (FileType::RadarPcd, "radar.pcd"),
            (FileType::RadarCube, "radar.png"),
        ];

        // file_extension() returns file extensions for saving
        let ext_cases = vec![
            (FileType::Image, "jpg"),
            (FileType::LidarPcd, "lidar.pcd"),
            (FileType::LidarDepth, "lidar.png"),
            (FileType::LidarReflect, "lidar.jpg"),
            (FileType::RadarPcd, "radar.pcd"),
            (FileType::RadarCube, "radar.png"),
        ];

        // Test: Display → to_string() returns server API names
        for (file_type, expected_str) in &api_cases {
            assert_eq!(file_type.to_string(), *expected_str);
        }

        // Test: file_extension() returns correct extensions
        for (file_type, expected_ext) in &ext_cases {
            assert_eq!(file_type.file_extension(), *expected_ext);
        }

        // Test: try_from() string parsing (accepts multiple aliases)
        assert_eq!(
            FileType::try_from("lidar.depth").unwrap(),
            FileType::LidarDepth
        );
        assert_eq!(
            FileType::try_from("lidar.png").unwrap(),
            FileType::LidarDepth
        );
        assert_eq!(
            FileType::try_from("depth.png").unwrap(),
            FileType::LidarDepth
        );
        assert_eq!(
            FileType::try_from("lidar.reflect").unwrap(),
            FileType::LidarReflect
        );
        assert_eq!(
            FileType::try_from("lidar.jpg").unwrap(),
            FileType::LidarReflect
        );
        assert_eq!(
            FileType::try_from("lidar.jpeg").unwrap(),
            FileType::LidarReflect
        );

        // Test: Invalid input
        assert!(FileType::try_from("invalid").is_err());

        // Test: Round-trip (Display → try_from)
        for (file_type, _) in &api_cases {
            let s = file_type.to_string();
            let parsed = FileType::try_from(s.as_str()).unwrap();
            assert_eq!(parsed, *file_type);
        }
    }

    // ==== AnnotationType Conversion Tests ====
    #[test]
    fn test_annotation_type_conversions() {
        let cases = vec![
            (AnnotationType::Box2d, "box2d"),
            (AnnotationType::Box3d, "box3d"),
            (AnnotationType::Mask, "mask"),
        ];

        // Test: Display → to_string()
        for (ann_type, expected_str) in &cases {
            assert_eq!(ann_type.to_string(), *expected_str);
        }

        // Test: try_from() string parsing
        for (ann_type, type_str) in &cases {
            assert_eq!(AnnotationType::try_from(*type_str).unwrap(), *ann_type);
        }

        // Test: From<String> (backward compatibility)
        assert_eq!(
            AnnotationType::from("box2d".to_string()),
            AnnotationType::Box2d
        );
        assert_eq!(
            AnnotationType::from("box3d".to_string()),
            AnnotationType::Box3d
        );
        assert_eq!(
            AnnotationType::from("mask".to_string()),
            AnnotationType::Mask
        );

        // Invalid defaults to Box2d for backward compatibility
        assert_eq!(
            AnnotationType::from("invalid".to_string()),
            AnnotationType::Box2d
        );

        // Test: Invalid input
        assert!(AnnotationType::try_from("invalid").is_err());

        // Test: Round-trip (Display → try_from)
        for (ann_type, _) in &cases {
            let s = ann_type.to_string();
            let parsed = AnnotationType::try_from(s.as_str()).unwrap();
            assert_eq!(parsed, *ann_type);
        }
    }

    // ==== Pure Function: extract_sample_name Tests ====
    #[test]
    fn test_extract_sample_name_with_extension_and_camera() {
        assert_eq!(extract_sample_name("scene_001.camera.jpg"), "scene_001");
    }

    #[test]
    fn test_extract_sample_name_multiple_dots() {
        assert_eq!(extract_sample_name("image.v2.camera.png"), "image.v2");
    }

    #[test]
    fn test_extract_sample_name_extension_only() {
        assert_eq!(extract_sample_name("test.jpg"), "test");
    }

    #[test]
    fn test_extract_sample_name_no_extension() {
        assert_eq!(extract_sample_name("test"), "test");
    }

    #[test]
    fn test_extract_sample_name_edge_case_dot_prefix() {
        assert_eq!(extract_sample_name(".jpg"), ".jpg");
    }

    // ==== File Resolution Tests ====
    #[test]
    fn test_resolve_file_image_type_returns_none() {
        // Image type uses image_url field, not files array
        let files = vec![];
        let result = resolve_file(&FileType::Image, &files);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_file_lidar_pcd() {
        let files = vec![
            SampleFile::with_url(
                "lidar.pcd".to_string(),
                "https://example.com/file.pcd".to_string(),
            ),
            SampleFile::with_url(
                "radar.pcd".to_string(),
                "https://example.com/radar.pcd".to_string(),
            ),
        ];
        let result = resolve_file(&FileType::LidarPcd, &files);
        assert!(result.is_some());
        assert_eq!(result.unwrap().url(), Some("https://example.com/file.pcd"));
    }

    #[test]
    fn test_resolve_file_not_found() {
        let files = vec![SampleFile::with_url(
            "lidar.pcd".to_string(),
            "https://example.com/file.pcd".to_string(),
        )];
        // Requesting radar.pcd which doesn't exist in files
        let result = resolve_file(&FileType::RadarPcd, &files);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_file_lidar_depth() {
        // Server returns "lidar.depth" for LiDAR depth data
        let files = vec![SampleFile::with_url(
            "lidar.depth".to_string(),
            "https://example.com/depth.png".to_string(),
        )];
        let result = resolve_file(&FileType::LidarDepth, &files);
        assert!(result.is_some());
        assert_eq!(result.unwrap().url(), Some("https://example.com/depth.png"));
    }

    #[test]
    fn test_resolve_file_lidar_reflect() {
        // Server returns "lidar.reflect" for LiDAR reflectance data
        let files = vec![SampleFile::with_url(
            "lidar.reflect".to_string(),
            "https://example.com/reflect.png".to_string(),
        )];
        let result = resolve_file(&FileType::LidarReflect, &files);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().url(),
            Some("https://example.com/reflect.png")
        );
    }

    #[test]
    fn test_resolve_file_radar_cube() {
        // Server returns "radar.png" or "cube" for radar cube data
        let files = vec![SampleFile::with_url(
            "radar.png".to_string(),
            "https://example.com/radar.png".to_string(),
        )];
        let result = resolve_file(&FileType::RadarCube, &files);
        assert!(result.is_some());
        assert_eq!(result.unwrap().url(), Some("https://example.com/radar.png"));
    }

    #[test]
    fn test_resolve_file_with_inline_data() {
        // Legacy datasets may have inline data instead of URLs
        let files = vec![SampleFile::with_data(
            "radar.pcd".to_string(),
            "SGVsbG8gV29ybGQ=".to_string(), // base64 "Hello World"
        )];
        let result = resolve_file(&FileType::RadarPcd, &files);
        assert!(result.is_some());
        let file = result.unwrap();
        assert!(file.url().is_none());
        assert_eq!(file.data(), Some("SGVsbG8gV29ybGQ="));
    }

    #[test]
    fn test_convert_annotations_map_to_vec_with_bbox() {
        let mut map = HashMap::new();
        let bbox_ann = Annotation::new();
        map.insert("bbox".to_string(), vec![bbox_ann.clone()]);

        let annotations = convert_annotations_map_to_vec(map);
        assert_eq!(annotations.len(), 1);
    }

    #[test]
    fn test_convert_annotations_map_to_vec_all_types() {
        let mut map = HashMap::new();
        map.insert("bbox".to_string(), vec![Annotation::new()]);
        map.insert("box3d".to_string(), vec![Annotation::new()]);
        map.insert("mask".to_string(), vec![Annotation::new()]);

        let annotations = convert_annotations_map_to_vec(map);
        assert_eq!(annotations.len(), 3);
    }

    #[test]
    fn test_convert_annotations_map_to_vec_empty() {
        let map = HashMap::new();
        let annotations = convert_annotations_map_to_vec(map);
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn test_convert_annotations_map_to_vec_unknown_type_ignored() {
        let mut map = HashMap::new();
        map.insert("unknown".to_string(), vec![Annotation::new()]);

        let annotations = convert_annotations_map_to_vec(map);
        // Unknown types are ignored
        assert_eq!(annotations.len(), 0);
    }

    // ==== Annotation Field Mapping Tests ====
    #[test]
    fn test_annotation_group_field_name() {
        assert_eq!(annotation_group_field_name(), "group_name");
    }

    #[test]
    fn test_annotation_object_id_field_name() {
        assert_eq!(annotation_object_id_field_name(), "object_reference");
    }

    #[test]
    fn test_annotation_object_id_alias() {
        assert_eq!(annotation_object_id_alias(), "object_id");
    }

    #[test]
    fn test_validate_annotation_field_names_success() {
        let json = r#"{"group_name":"train","object_reference":"obj1"}"#;
        assert!(validate_annotation_field_names(json, true, true).is_ok());
    }

    #[test]
    fn test_validate_annotation_field_names_missing_group() {
        let json = r#"{"object_reference":"obj1"}"#;
        let result = validate_annotation_field_names(json, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("group_name"));
    }

    #[test]
    fn test_validate_annotation_field_names_missing_object_ref() {
        let json = r#"{"group_name":"train"}"#;
        let result = validate_annotation_field_names(json, false, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("object_reference"));
    }

    #[test]
    fn test_annotation_serialization_field_names() {
        // Test that Annotation serializes with correct field names
        let mut ann = Annotation::new();
        ann.set_group(Some("train".to_string()));
        ann.set_object_id(Some("obj1".to_string()));

        let json = serde_json::to_string(&ann).unwrap();
        // Verify JSON contains correct field names
        assert!(validate_annotation_field_names(&json, true, true).is_ok());
    }

    // ==== GPS/IMU Validation Tests ====
    #[test]
    fn test_validate_gps_coordinates_valid() {
        assert!(validate_gps_coordinates(37.7749, -122.4194).is_ok()); // San Francisco
        assert!(validate_gps_coordinates(0.0, 0.0).is_ok()); // Null Island
        assert!(validate_gps_coordinates(90.0, 180.0).is_ok()); // Edge cases
        assert!(validate_gps_coordinates(-90.0, -180.0).is_ok()); // Edge cases
    }

    #[test]
    fn test_validate_gps_coordinates_invalid_latitude() {
        let result = validate_gps_coordinates(91.0, 0.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("latitude out of range"));

        let result = validate_gps_coordinates(-91.0, 0.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("latitude out of range"));
    }

    #[test]
    fn test_validate_gps_coordinates_invalid_longitude() {
        let result = validate_gps_coordinates(0.0, 181.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("longitude out of range"));

        let result = validate_gps_coordinates(0.0, -181.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("longitude out of range"));
    }

    #[test]
    fn test_validate_gps_coordinates_non_finite() {
        let result = validate_gps_coordinates(f64::NAN, 0.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not finite"));

        let result = validate_gps_coordinates(0.0, f64::INFINITY);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not finite"));
    }

    #[test]
    fn test_validate_imu_orientation_valid() {
        assert!(validate_imu_orientation(0.0, 0.0, 0.0).is_ok());
        assert!(validate_imu_orientation(45.0, 30.0, 90.0).is_ok());
        assert!(validate_imu_orientation(180.0, 90.0, -180.0).is_ok()); // Edge cases
        assert!(validate_imu_orientation(-180.0, -90.0, 180.0).is_ok()); // Edge cases
    }

    #[test]
    fn test_validate_imu_orientation_invalid_roll() {
        let result = validate_imu_orientation(181.0, 0.0, 0.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("roll out of range"));

        let result = validate_imu_orientation(-181.0, 0.0, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_imu_orientation_invalid_pitch() {
        let result = validate_imu_orientation(0.0, 91.0, 0.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("pitch out of range"));

        let result = validate_imu_orientation(0.0, -91.0, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_imu_orientation_non_finite() {
        let result = validate_imu_orientation(f64::NAN, 0.0, 0.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not finite"));

        let result = validate_imu_orientation(0.0, f64::INFINITY, 0.0);
        assert!(result.is_err());

        let result = validate_imu_orientation(0.0, 0.0, f64::NEG_INFINITY);
        assert!(result.is_err());
    }

    // ==== Polygon Flattening Tests ====
    #[test]
    #[cfg(feature = "polars")]
    fn test_flatten_polygon_coordinates_single_polygon() {
        let polygons = vec![vec![(1.0, 2.0), (3.0, 4.0)]];
        let result = flatten_polygon_coordinates(&polygons);

        // Should have x1, y1, x2, y2 (no trailing NaN)
        assert_eq!(result.len(), 4);
        assert_eq!(&result[..4], &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    #[cfg(feature = "polars")]
    fn test_flatten_polygon_coordinates_multiple_polygons() {
        let polygons = vec![vec![(1.0, 2.0), (3.0, 4.0)], vec![(5.0, 6.0), (7.0, 8.0)]];
        let result = flatten_polygon_coordinates(&polygons);

        // x1, y1, x2, y2, NaN, x3, y3, x4, y4 (no trailing NaN)
        assert_eq!(result.len(), 9);
        assert_eq!(&result[..4], &[1.0, 2.0, 3.0, 4.0]);
        assert!(result[4].is_nan()); // NaN separator
        assert_eq!(&result[5..9], &[5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    #[cfg(feature = "polars")]
    fn test_flatten_polygon_coordinates_empty() {
        let polygons: Vec<Vec<(f32, f32)>> = vec![];
        let result = flatten_polygon_coordinates(&polygons);

        assert_eq!(result.len(), 0);
    }

    // ==== Polygon Unflattening Tests ====
    #[test]
    #[cfg(feature = "polars")]
    fn test_unflatten_polygon_coordinates_single_polygon() {
        let coords = vec![1.0, 2.0, 3.0, 4.0];
        let result = unflatten_polygon_coordinates(&coords);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[0][0], (1.0, 2.0));
        assert_eq!(result[0][1], (3.0, 4.0));
    }

    #[test]
    #[cfg(feature = "polars")]
    fn test_unflatten_polygon_coordinates_multiple_polygons() {
        let coords = vec![1.0, 2.0, 3.0, 4.0, f32::NAN, 5.0, 6.0, 7.0, 8.0];
        let result = unflatten_polygon_coordinates(&coords);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[0][0], (1.0, 2.0));
        assert_eq!(result[0][1], (3.0, 4.0));
        assert_eq!(result[1].len(), 2);
        assert_eq!(result[1][0], (5.0, 6.0));
        assert_eq!(result[1][1], (7.0, 8.0));
    }

    #[test]
    #[cfg(feature = "polars")]
    fn test_unflatten_polygon_coordinates_roundtrip() {
        // Test that flatten -> unflatten produces the same result
        let original = vec![vec![(1.0, 2.0), (3.0, 4.0)], vec![(5.0, 6.0), (7.0, 8.0)]];
        let flattened = flatten_polygon_coordinates(&original);
        let result = unflatten_polygon_coordinates(&flattened);

        assert_eq!(result, original);
    }

    // ==== Annotation Format Flattening Tests ====
    #[test]
    fn test_flatten_annotation_map_all_types() {
        use std::collections::HashMap;

        let mut map = HashMap::new();

        // Create test annotations
        let mut bbox_ann = Annotation::new();
        bbox_ann.set_label(Some("bbox_label".to_string()));

        let mut box3d_ann = Annotation::new();
        box3d_ann.set_label(Some("box3d_label".to_string()));

        let mut mask_ann = Annotation::new();
        mask_ann.set_label(Some("mask_label".to_string()));

        map.insert("bbox".to_string(), vec![bbox_ann.clone()]);
        map.insert("box3d".to_string(), vec![box3d_ann.clone()]);
        map.insert("mask".to_string(), vec![mask_ann.clone()]);

        let result = flatten_annotation_map(map);

        assert_eq!(result.len(), 3);
        // Check ordering: bbox, box3d, mask
        assert_eq!(result[0].label(), Some(&"bbox_label".to_string()));
        assert_eq!(result[1].label(), Some(&"box3d_label".to_string()));
        assert_eq!(result[2].label(), Some(&"mask_label".to_string()));
    }

    #[test]
    fn test_flatten_annotation_map_single_type() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        let mut bbox_ann = Annotation::new();
        bbox_ann.set_label(Some("test".to_string()));
        map.insert("bbox".to_string(), vec![bbox_ann]);

        let result = flatten_annotation_map(map);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].label(), Some(&"test".to_string()));
    }

    #[test]
    fn test_flatten_annotation_map_empty() {
        use std::collections::HashMap;

        let map = HashMap::new();
        let result = flatten_annotation_map(map);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_flatten_annotation_map_deterministic_order() {
        use std::collections::HashMap;

        let mut map = HashMap::new();

        let mut bbox_ann = Annotation::new();
        bbox_ann.set_label(Some("bbox".to_string()));

        let mut box3d_ann = Annotation::new();
        box3d_ann.set_label(Some("box3d".to_string()));

        let mut mask_ann = Annotation::new();
        mask_ann.set_label(Some("mask".to_string()));

        // Insert in reverse order to test deterministic ordering
        map.insert("mask".to_string(), vec![mask_ann]);
        map.insert("box3d".to_string(), vec![box3d_ann]);
        map.insert("bbox".to_string(), vec![bbox_ann]);

        let result = flatten_annotation_map(map);

        // Should be bbox, box3d, mask regardless of insertion order
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].label(), Some(&"bbox".to_string()));
        assert_eq!(result[1].label(), Some(&"box3d".to_string()));
        assert_eq!(result[2].label(), Some(&"mask".to_string()));
    }

    // ==== Box2d Tests ====
    #[test]
    fn test_box2d_construction_and_accessors() {
        // Test case 1: Basic construction with positive coordinates
        let bbox = Box2d::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(
            (bbox.left(), bbox.top(), bbox.width(), bbox.height()),
            (10.0, 20.0, 100.0, 50.0)
        );

        // Test case 2: Center calculations
        assert_eq!((bbox.cx(), bbox.cy()), (60.0, 45.0)); // 10+50, 20+25

        // Test case 3: Zero origin
        let bbox = Box2d::new(0.0, 0.0, 640.0, 480.0);
        assert_eq!(
            (bbox.left(), bbox.top(), bbox.width(), bbox.height()),
            (0.0, 0.0, 640.0, 480.0)
        );
        assert_eq!((bbox.cx(), bbox.cy()), (320.0, 240.0));
    }

    #[test]
    fn test_box2d_center_calculation() {
        let bbox = Box2d::new(10.0, 20.0, 100.0, 50.0);

        // Center = position + size/2
        assert_eq!(bbox.cx(), 60.0); // 10 + 100/2
        assert_eq!(bbox.cy(), 45.0); // 20 + 50/2
    }

    #[test]
    fn test_box2d_zero_dimensions() {
        let bbox = Box2d::new(10.0, 20.0, 0.0, 0.0);

        // When width/height are zero, center = position
        assert_eq!(bbox.cx(), 10.0);
        assert_eq!(bbox.cy(), 20.0);
    }

    #[test]
    fn test_box2d_negative_dimensions() {
        let bbox = Box2d::new(100.0, 100.0, -50.0, -50.0);

        // Negative dimensions create inverted boxes (valid edge case)
        assert_eq!(bbox.width(), -50.0);
        assert_eq!(bbox.height(), -50.0);
        assert_eq!(bbox.cx(), 75.0); // 100 + (-50)/2
        assert_eq!(bbox.cy(), 75.0); // 100 + (-50)/2
    }

    // ==== Box3d Tests ====
    #[test]
    fn test_box3d_construction_and_accessors() {
        // Test case 1: Basic 3D construction
        let bbox = Box3d::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        assert_eq!((bbox.cx(), bbox.cy(), bbox.cz()), (1.0, 2.0, 3.0));
        assert_eq!(
            (bbox.width(), bbox.height(), bbox.length()),
            (4.0, 5.0, 6.0)
        );

        // Test case 2: Corners calculation with offset center
        let bbox = Box3d::new(10.0, 20.0, 30.0, 4.0, 6.0, 8.0);
        assert_eq!((bbox.left(), bbox.top(), bbox.front()), (8.0, 17.0, 26.0)); // 10-2, 20-3, 30-4

        // Test case 3: Center at origin with negative corners
        let bbox = Box3d::new(0.0, 0.0, 0.0, 2.0, 3.0, 4.0);
        assert_eq!((bbox.cx(), bbox.cy(), bbox.cz()), (0.0, 0.0, 0.0));
        assert_eq!(
            (bbox.width(), bbox.height(), bbox.length()),
            (2.0, 3.0, 4.0)
        );
        assert_eq!((bbox.left(), bbox.top(), bbox.front()), (-1.0, -1.5, -2.0));
    }

    #[test]
    fn test_box3d_center_calculation() {
        let bbox = Box3d::new(10.0, 20.0, 30.0, 100.0, 50.0, 40.0);

        // Center values as specified in constructor
        assert_eq!(bbox.cx(), 10.0);
        assert_eq!(bbox.cy(), 20.0);
        assert_eq!(bbox.cz(), 30.0);
    }

    #[test]
    fn test_box3d_zero_dimensions() {
        let bbox = Box3d::new(5.0, 10.0, 15.0, 0.0, 0.0, 0.0);

        // When all dimensions are zero, corners = center
        assert_eq!(bbox.cx(), 5.0);
        assert_eq!(bbox.cy(), 10.0);
        assert_eq!(bbox.cz(), 15.0);
        assert_eq!((bbox.left(), bbox.top(), bbox.front()), (5.0, 10.0, 15.0));
    }

    #[test]
    fn test_box3d_negative_dimensions() {
        let bbox = Box3d::new(100.0, 100.0, 100.0, -50.0, -50.0, -50.0);

        // Negative dimensions create inverted boxes
        assert_eq!(bbox.width(), -50.0);
        assert_eq!(bbox.height(), -50.0);
        assert_eq!(bbox.length(), -50.0);
        assert_eq!(
            (bbox.left(), bbox.top(), bbox.front()),
            (125.0, 125.0, 125.0)
        );
    }

    // ==== Mask Tests ====
    #[test]
    fn test_mask_creation_and_deserialization() {
        // Test case 1: Direct construction
        let polygon = vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]];
        let mask = Mask::new(polygon.clone());
        assert_eq!(mask.polygon, polygon);

        // Test case 2: Deserialization from legacy format
        let legacy = serde_json::json!({
            "mask": {
                "polygon": [[
                    [0.0_f32, 0.0_f32],
                    [1.0_f32, 0.0_f32],
                    [1.0_f32, 1.0_f32]
                ]]
            }
        });

        #[derive(serde::Deserialize)]
        struct Wrapper {
            mask: Mask,
        }

        let parsed: Wrapper = serde_json::from_value(legacy).unwrap();
        assert_eq!(parsed.mask.polygon.len(), 1);
        assert_eq!(parsed.mask.polygon[0].len(), 3);
    }

    // ==== Sample Tests ====
    #[test]
    fn test_sample_construction_and_accessors() {
        // Test case 1: New sample is empty
        let sample = Sample::new();
        assert_eq!(sample.id(), None);
        assert_eq!(sample.image_name(), None);
        assert_eq!(sample.width(), None);
        assert_eq!(sample.height(), None);

        // Test case 2: Sample with populated fields
        let mut sample = Sample::new();
        sample.image_name = Some("test.jpg".to_string());
        sample.width = Some(1920);
        sample.height = Some(1080);
        sample.group = Some("group1".to_string());

        assert_eq!(sample.image_name(), Some("test.jpg"));
        assert_eq!(sample.width(), Some(1920));
        assert_eq!(sample.height(), Some(1080));
        assert_eq!(sample.group(), Some(&"group1".to_string()));
    }

    #[test]
    fn test_sample_name_extraction_from_image_name() {
        let mut sample = Sample::new();

        // Test case 1: Basic image name with extension
        sample.image_name = Some("test_image.jpg".to_string());
        assert_eq!(sample.name(), Some("test_image".to_string()));

        // Test case 2: Image name with .camera suffix
        sample.image_name = Some("test_image.camera.jpg".to_string());
        assert_eq!(sample.name(), Some("test_image".to_string()));

        // Test case 3: Image name without extension
        sample.image_name = Some("test_image".to_string());
        assert_eq!(sample.name(), Some("test_image".to_string()));
    }

    // ==== Annotation Tests ====
    #[test]
    fn test_annotation_construction_and_setters() {
        // Test case 1: New annotation is empty
        let ann = Annotation::new();
        assert_eq!(ann.sample_id(), None);
        assert_eq!(ann.label(), None);
        assert_eq!(ann.box2d(), None);
        assert_eq!(ann.box3d(), None);
        assert_eq!(ann.mask(), None);

        // Test case 2: Setting annotation fields
        let mut ann = Annotation::new();
        ann.set_label(Some("car".to_string()));
        assert_eq!(ann.label(), Some(&"car".to_string()));

        ann.set_label_index(Some(42));
        assert_eq!(ann.label_index(), Some(42));

        // Test case 3: Setting bounding box
        let bbox = Box2d::new(10.0, 20.0, 100.0, 50.0);
        ann.set_box2d(Some(bbox.clone()));
        assert!(ann.box2d().is_some());
        assert_eq!(ann.box2d().unwrap().left(), 10.0);
    }

    // ==== SampleFile Tests ====
    #[test]
    fn test_sample_file_with_url_and_filename() {
        // Test case 1: SampleFile with URL
        let file = SampleFile::with_url(
            "lidar.pcd".to_string(),
            "https://example.com/file.pcd".to_string(),
        );
        assert_eq!(file.file_type(), "lidar.pcd");
        assert_eq!(file.url(), Some("https://example.com/file.pcd"));
        assert_eq!(file.filename(), None);

        // Test case 2: SampleFile with local filename
        let file = SampleFile::with_filename("image".to_string(), "test.jpg".to_string());
        assert_eq!(file.file_type(), "image");
        assert_eq!(file.filename(), Some("test.jpg"));
        assert_eq!(file.url(), None);
    }

    // ==== Sample GPS/IMU Deserialization Tests ====
    #[test]
    fn test_sample_deserializes_gps_imu_from_sensors() {
        use serde_json::json;

        // Test: GPS and IMU data in sensors array is extracted to location field
        let sample_json = json!({
            "id": 123,
            "image_name": "test.jpg",
            "sensors": [
                {"gps": {"lat": 37.7749, "lon": -122.4194}},
                {"imu": {"roll": 1.5, "pitch": 2.5, "yaw": 3.5}},
                {"radar.pcd": "https://example.com/radar.pcd"}
            ]
        });

        let sample: Sample = serde_json::from_value(sample_json).unwrap();

        // Verify location was extracted
        assert!(sample.location.is_some());
        let location = sample.location.as_ref().unwrap();

        // Verify GPS data
        assert!(location.gps.is_some());
        let gps = location.gps.as_ref().unwrap();
        assert!((gps.lat - 37.7749).abs() < 0.0001);
        assert!((gps.lon - (-122.4194)).abs() < 0.0001);

        // Verify IMU data
        assert!(location.imu.is_some());
        let imu = location.imu.as_ref().unwrap();
        assert!((imu.roll - 1.5).abs() < 0.0001);
        assert!((imu.pitch - 2.5).abs() < 0.0001);
        assert!((imu.yaw - 3.5).abs() < 0.0001);

        // Verify files were also extracted (non-GPS/IMU entries)
        assert_eq!(sample.files.len(), 1);
        assert_eq!(sample.files[0].file_type(), "radar.pcd");
        assert_eq!(sample.files[0].url(), Some("https://example.com/radar.pcd"));
    }

    #[test]
    fn test_sample_deserializes_gps_only() {
        use serde_json::json;

        // Test: Only GPS data in sensors
        let sample_json = json!({
            "id": 456,
            "sensors": [
                {"gps": {"lat": 40.7128, "lon": -74.0060}}
            ]
        });

        let sample: Sample = serde_json::from_value(sample_json).unwrap();

        assert!(sample.location.is_some());
        let location = sample.location.as_ref().unwrap();

        assert!(location.gps.is_some());
        assert!(location.imu.is_none());

        let gps = location.gps.as_ref().unwrap();
        assert!((gps.lat - 40.7128).abs() < 0.0001);
        assert!((gps.lon - (-74.0060)).abs() < 0.0001);
    }

    #[test]
    fn test_sample_deserializes_without_location() {
        use serde_json::json;

        // Test: Sample with only file sensors (no GPS/IMU)
        let sample_json = json!({
            "id": 789,
            "sensors": [
                {"radar.pcd": "https://example.com/radar.pcd"},
                {"lidar.pcd": "https://example.com/lidar.pcd"}
            ]
        });

        let sample: Sample = serde_json::from_value(sample_json).unwrap();

        // No location data
        assert!(sample.location.is_none());

        // Both files extracted
        assert_eq!(sample.files.len(), 2);
    }

    // ==== Label Tests ====
    #[test]
    fn test_label_deserialization_and_accessors() {
        use serde_json::json;

        // Test case 1: Label deserialization and accessors
        let label_json = json!({
            "id": 123,
            "dataset_id": 456,
            "index": 5,
            "name": "car"
        });

        let label: Label = serde_json::from_value(label_json).unwrap();
        assert_eq!(label.id(), 123);
        assert_eq!(label.index(), 5);
        assert_eq!(label.name(), "car");
        assert_eq!(label.to_string(), "car");
        assert_eq!(format!("{}", label), "car");

        // Test case 2: Different label
        let label_json = json!({
            "id": 1,
            "dataset_id": 100,
            "index": 0,
            "name": "person"
        });

        let label: Label = serde_json::from_value(label_json).unwrap();
        assert_eq!(format!("{}", label), "person");
    }

    // ==== Annotation Serialization Tests ====
    #[test]
    fn test_annotation_serialization_with_mask_and_box() {
        let polygon = vec![vec![
            (0.0_f32, 0.0_f32),
            (1.0_f32, 0.0_f32),
            (1.0_f32, 1.0_f32),
        ]];

        let mut annotation = Annotation::new();
        annotation.set_label(Some("test".to_string()));
        annotation.set_box2d(Some(Box2d::new(10.0, 20.0, 30.0, 40.0)));
        annotation.set_mask(Some(Mask::new(polygon)));

        let mut sample = Sample::new();
        sample.annotations.push(annotation);

        let json = serde_json::to_value(&sample).unwrap();
        let annotations = json
            .get("annotations")
            .and_then(|value| value.as_array())
            .expect("annotations serialized as array");
        assert_eq!(annotations.len(), 1);

        let annotation_json = annotations[0].as_object().expect("annotation object");
        assert!(annotation_json.contains_key("box2d"));
        assert!(annotation_json.contains_key("mask"));
        assert!(!annotation_json.contains_key("x"));
        assert!(
            annotation_json
                .get("mask")
                .and_then(|value| value.as_array())
                .is_some()
        );
    }

    #[test]
    fn test_frame_number_negative_one_deserializes_as_none() {
        // Server returns frame_number: -1 for non-sequence samples
        // This should deserialize as None for the client
        let json = r#"{
            "uuid": "test-uuid",
            "frame_number": -1
        }"#;

        let sample: Sample = serde_json::from_str(json).unwrap();
        assert_eq!(sample.frame_number, None);
    }

    #[test]
    fn test_frame_number_positive_value_deserializes_correctly() {
        // Valid frame numbers should deserialize normally
        let json = r#"{
            "uuid": "test-uuid",
            "frame_number": 5
        }"#;

        let sample: Sample = serde_json::from_str(json).unwrap();
        assert_eq!(sample.frame_number, Some(5));
    }

    #[test]
    fn test_frame_number_null_deserializes_as_none() {
        // Explicit null should also be None
        let json = r#"{
            "uuid": "test-uuid",
            "frame_number": null
        }"#;

        let sample: Sample = serde_json::from_str(json).unwrap();
        assert_eq!(sample.frame_number, None);
    }

    #[test]
    fn test_frame_number_missing_deserializes_as_none() {
        // Missing field should be None
        let json = r#"{
            "uuid": "test-uuid"
        }"#;

        let sample: Sample = serde_json::from_str(json).unwrap();
        assert_eq!(sample.frame_number, None);
    }

    // =========================================================================
    // samples_dataframe tests - CRITICAL: Verify group preservation
    // =========================================================================

    #[cfg(feature = "polars")]
    #[test]
    fn test_samples_dataframe_preserves_group_for_samples_without_annotations() {
        use polars::prelude::*;

        // Create sample WITH annotations
        let mut sample_with_ann = Sample::new();
        sample_with_ann.image_name = Some("annotated.jpg".to_string());
        sample_with_ann.group = Some("train".to_string());
        let mut annotation = Annotation::new();
        annotation.set_label(Some("car".to_string()));
        annotation.set_box2d(Some(Box2d::new(0.1, 0.2, 0.3, 0.4)));
        annotation.set_name(Some("annotated".to_string()));
        sample_with_ann.annotations = vec![annotation];

        // Create sample WITHOUT annotations (this is the critical case)
        let mut sample_no_ann = Sample::new();
        sample_no_ann.image_name = Some("unannotated.jpg".to_string());
        sample_no_ann.group = Some("val".to_string()); // Should be preserved!
        sample_no_ann.annotations = vec![]; // Empty annotations

        let samples = vec![sample_with_ann, sample_no_ann];

        // Convert to DataFrame
        let df = samples_dataframe(&samples).expect("Failed to create DataFrame");

        // Verify we have 2 rows (one per sample)
        assert_eq!(df.height(), 2, "Expected 2 rows (one per sample)");

        // Get the group column
        let groups_col = df.column("group").expect("group column should exist");
        let groups_cast = groups_col.cast(&DataType::String).expect("cast to string");
        let groups = groups_cast.str().expect("as str");

        // Find the row for "unannotated" and verify it has group "val"
        let names_col = df.column("name").expect("name column should exist");
        let names_cast = names_col.cast(&DataType::String).expect("cast to string");
        let names = names_cast.str().expect("as str");

        let mut found_unannotated = false;
        for idx in 0..df.height() {
            if let Some(name) = names.get(idx)
                && name == "unannotated"
            {
                found_unannotated = true;
                let group = groups.get(idx);
                assert_eq!(
                    group,
                    Some("val"),
                    "CRITICAL: Sample 'unannotated' without annotations must have group 'val'"
                );
            }
        }

        assert!(
            found_unannotated,
            "Did not find 'unannotated' sample in DataFrame - \
             this means samples without annotations are not being included"
        );
    }

    #[cfg(feature = "polars")]
    #[test]
    fn test_samples_dataframe_includes_all_samples_even_without_annotations() {
        // Verify that samples without annotations still appear in the DataFrame
        // with null annotation fields but WITH their group field populated

        let mut sample1 = Sample::new();
        sample1.image_name = Some("with_ann.jpg".to_string());
        sample1.group = Some("train".to_string());
        let mut ann = Annotation::new();
        ann.set_label(Some("person".to_string()));
        ann.set_box2d(Some(Box2d::new(0.0, 0.0, 0.5, 0.5)));
        ann.set_name(Some("with_ann".to_string()));
        sample1.annotations = vec![ann];

        let mut sample2 = Sample::new();
        sample2.image_name = Some("no_ann_train.jpg".to_string());
        sample2.group = Some("train".to_string());
        sample2.annotations = vec![];

        let mut sample3 = Sample::new();
        sample3.image_name = Some("no_ann_val.jpg".to_string());
        sample3.group = Some("val".to_string());
        sample3.annotations = vec![];

        let samples = vec![sample1, sample2, sample3];

        let df = samples_dataframe(&samples).expect("Failed to create DataFrame");

        // We should have exactly 3 rows - one per sample
        assert_eq!(
            df.height(),
            3,
            "Expected 3 rows (samples without annotations should create one row each)"
        );

        // Check that all groups are present
        let groups_col = df.column("group").expect("group column");
        let groups_cast = groups_col.cast(&polars::prelude::DataType::String).unwrap();
        let groups = groups_cast.str().unwrap();

        let mut train_count = 0;
        let mut val_count = 0;

        for idx in 0..df.height() {
            match groups.get(idx) {
                Some("train") => train_count += 1,
                Some("val") => val_count += 1,
                other => panic!(
                    "Unexpected group value at row {}: {:?}. \
                     All samples should have their group preserved.",
                    idx, other
                ),
            }
        }

        assert_eq!(train_count, 2, "Expected 2 samples in 'train' group");
        assert_eq!(val_count, 1, "Expected 1 sample in 'val' group");
    }

    #[cfg(feature = "polars")]
    #[test]
    fn test_samples_dataframe_group_is_not_null_for_samples_with_group() {
        // CRITICAL: Even when a sample has no annotations, if it has a group,
        // that group must NOT be null in the DataFrame

        let mut sample = Sample::new();
        sample.image_name = Some("test.jpg".to_string());
        sample.group = Some("test_group".to_string());
        sample.annotations = vec![];

        let df = samples_dataframe(&[sample]).expect("Failed to create DataFrame");

        let groups_col = df.column("group").expect("group column");

        // The group column should have NO nulls because our sample has a group
        assert_eq!(
            groups_col.null_count(),
            0,
            "Sample with group='test_group' but no annotations has NULL group in DataFrame. \
             This is a bug in samples_dataframe - group must be preserved!"
        );
    }

    #[cfg(feature = "polars")]
    #[test]
    fn test_samples_dataframe_group_consistent_across_all_rows_for_same_image() {
        use polars::prelude::*;

        // Test that when a sample has multiple annotations, ALL rows have
        // the same group value (not just the first one)

        let mut sample = Sample::new();
        sample.image_name = Some("multi_ann.jpg".to_string());
        sample.group = Some("train".to_string());

        // Add multiple annotations
        let mut ann1 = Annotation::new();
        ann1.set_label(Some("car".to_string()));
        ann1.set_box2d(Some(Box2d::new(0.1, 0.2, 0.3, 0.4)));
        ann1.set_name(Some("multi_ann".to_string()));

        let mut ann2 = Annotation::new();
        ann2.set_label(Some("truck".to_string()));
        ann2.set_box2d(Some(Box2d::new(0.5, 0.6, 0.2, 0.2)));
        ann2.set_name(Some("multi_ann".to_string()));

        let mut ann3 = Annotation::new();
        ann3.set_label(Some("bus".to_string()));
        ann3.set_box2d(Some(Box2d::new(0.7, 0.8, 0.1, 0.1)));
        ann3.set_name(Some("multi_ann".to_string()));

        sample.annotations = vec![ann1, ann2, ann3];

        let df = samples_dataframe(&[sample]).expect("Failed to create DataFrame");

        // Should have 3 rows (one per annotation)
        assert_eq!(df.height(), 3, "Expected 3 rows (one per annotation)");

        // ALL rows should have the group "train" (not just the first one)
        let groups_col = df.column("group").expect("group column");
        let groups_cast = groups_col.cast(&DataType::String).expect("cast to string");
        let groups = groups_cast.str().expect("as str");

        // No nulls allowed
        assert_eq!(groups_col.null_count(), 0, "No rows should have null group");

        // All rows should have the same group
        for idx in 0..df.height() {
            let group = groups.get(idx);
            assert_eq!(
                group,
                Some("train"),
                "Row {} should have group 'train', got {:?}. \
                 All rows for the same image must have identical group values.",
                idx,
                group
            );
        }
    }
}
