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
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            FileType::Image => "image",
            FileType::LidarPcd => "lidar.pcd",
            FileType::LidarDepth => "lidar.png",
            FileType::LidarReflect => "lidar.jpg",
            FileType::RadarPcd => "radar.pcd",
            FileType::RadarCube => "radar.png",
        };
        write!(f, "{}", value)
    }
}

impl TryFrom<&str> for FileType {
    type Error = crate::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "image" => Ok(FileType::Image),
            "lidar.pcd" => Ok(FileType::LidarPcd),
            "lidar.png" => Ok(FileType::LidarDepth),
            "lidar.jpg" => Ok(FileType::LidarReflect),
            "radar.pcd" => Ok(FileType::RadarPcd),
            "radar.png" => Ok(FileType::RadarCube),
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
#[derive(Serialize, Deserialize, Clone, Debug)]
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
    /// Serialized as "sensors" for API compatibility with populate endpoint.
    #[serde(rename = "sensors", skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    /// Image degradation type (blur, occlusion, weather, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degradation: Option<String>,
    /// Additional sensor files (LiDAR, radar, depth maps, etc.).
    /// When deserializing from samples.list: Vec<SampleFile>
    /// When serializing for samples.populate2: HashMap<String, String>
    /// (file_type -> filename)
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_files",
        deserialize_with = "deserialize_files"
    )]
    pub files: Vec<SampleFile>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_annotations",
        deserialize_with = "deserialize_annotations"
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

// Custom deserializer for files field - converts HashMap or Vec to
// Vec<SampleFile>
fn deserialize_files<'de, D>(deserializer: D) -> Result<Vec<SampleFile>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum FilesFormat {
        Vec(Vec<SampleFile>),
        Map(HashMap<String, String>),
    }

    let value = Option::<FilesFormat>::deserialize(deserializer)?;
    Ok(value
        .map(|v| match v {
            FilesFormat::Vec(files) => files,
            FilesFormat::Map(map) => convert_files_map_to_vec(map),
        })
        .unwrap_or_default())
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

    pub async fn download(
        &self,
        client: &Client,
        file_type: FileType,
    ) -> Result<Option<Vec<u8>>, Error> {
        let url = resolve_file_url(&file_type, self.image_url.as_deref(), &self.files);

        Ok(match url {
            Some(url) => Some(client.download(url).await?),
            None => None,
        })
    }
}

/// A file associated with a sample (e.g., LiDAR point cloud, radar data).
///
/// For samples retrieved from the server, this contains the file type and URL.
/// For samples being populated to the server, this can be a type and filename.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SampleFile {
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<String>,
}

impl SampleFile {
    /// Creates a new sample file with type and URL (for downloaded samples).
    pub fn with_url(file_type: String, url: String) -> Self {
        Self {
            r#type: file_type,
            url: Some(url),
            filename: None,
        }
    }

    /// Creates a new sample file with type and filename (for populate API).
    pub fn with_filename(file_type: String, filename: String) -> Self {
        Self {
            r#type: file_type,
            url: None,
            filename: Some(filename),
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
        use serde::Deserialize;

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum MaskFormat {
            Polygon { polygon: Vec<Vec<(f32, f32)>> },
            Direct(Vec<Vec<(f32, f32)>>),
        }

        match MaskFormat::deserialize(deserializer)? {
            MaskFormat::Polygon { polygon } => Ok(Self { polygon }),
            MaskFormat::Direct(polygon) => Ok(Self { polygon }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct Group {
    pub id: u64, // Groups seem to use raw u64, not a specific ID type
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

/// Resolve file URL for a given file type from sample data.
///
/// Pure function that extracts the URL resolution logic from
/// `Sample::download()`. Returns `Some(url)` if the file exists, `None`
/// otherwise.
///
/// # Examples
/// - Image: Uses `image_url` field
/// - Other files: Searches `files` array by type matching
///
/// # Arguments
/// * `file_type` - The type of file to resolve (e.g., "image", "lidar.pcd")
/// * `image_url` - The sample's image URL (for FileType::Image)
/// * `files` - The sample's file list (for other file types)
fn resolve_file_url<'a>(
    file_type: &FileType,
    image_url: Option<&'a str>,
    files: &'a [SampleFile],
) -> Option<&'a str> {
    match file_type {
        FileType::Image => image_url,
        file => files
            .iter()
            .find(|f| f.r#type == file.to_string())
            .and_then(|f| f.url.as_deref()),
    }
}

// ============================================================================
// DESERIALIZATION FORMAT CONVERSION HELPERS
// ============================================================================

/// Convert files HashMap format to Vec<SampleFile>.
///
/// Pure function that handles the conversion from the server's populate API
/// format (HashMap<String, String>) to the internal Vec<SampleFile>
/// representation.
///
/// # Arguments
/// * `map` - HashMap where key is file type (e.g., "lidar.pcd") and value is
///   filename
fn convert_files_map_to_vec(map: HashMap<String, String>) -> Vec<SampleFile> {
    map.into_iter()
        .map(|(file_type, filename)| SampleFile::with_filename(file_type, filename))
        .collect()
}

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
        let cases = vec![
            (FileType::Image, "image"),
            (FileType::LidarPcd, "lidar.pcd"),
            (FileType::LidarDepth, "lidar.png"),
            (FileType::LidarReflect, "lidar.jpg"),
            (FileType::RadarPcd, "radar.pcd"),
            (FileType::RadarCube, "radar.png"),
        ];

        // Test: Display → to_string()
        for (file_type, expected_str) in &cases {
            assert_eq!(file_type.to_string(), *expected_str);
        }

        // Test: try_from() string parsing
        for (file_type, type_str) in &cases {
            assert_eq!(FileType::try_from(*type_str).unwrap(), *file_type);
        }

        // Test: FromStr trait
        for (file_type, type_str) in &cases {
            assert_eq!(FileType::from_str(type_str).unwrap(), *file_type);
        }

        // Test: Invalid input
        assert!(FileType::try_from("invalid").is_err());

        // Test: Round-trip (Display → try_from)
        for (file_type, _) in &cases {
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

    // ==== File URL Resolution Tests ====
    #[test]
    fn test_resolve_file_url_image_type() {
        let image_url = Some("https://example.com/image.jpg");
        let files = vec![];
        let result = resolve_file_url(&FileType::Image, image_url, &files);
        assert_eq!(result, Some("https://example.com/image.jpg"));
    }

    #[test]
    fn test_resolve_file_url_lidar_pcd() {
        let image_url = Some("https://example.com/image.jpg");
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
        let result = resolve_file_url(&FileType::LidarPcd, image_url, &files);
        assert_eq!(result, Some("https://example.com/file.pcd"));
    }

    #[test]
    fn test_resolve_file_url_not_found() {
        let image_url = Some("https://example.com/image.jpg");
        let files = vec![SampleFile::with_url(
            "lidar.pcd".to_string(),
            "https://example.com/file.pcd".to_string(),
        )];
        // Requesting radar.pcd which doesn't exist in files
        let result = resolve_file_url(&FileType::RadarPcd, image_url, &files);
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_file_url_no_image_url() {
        let image_url = None;
        let files = vec![];
        let result = resolve_file_url(&FileType::Image, image_url, &files);
        assert_eq!(result, None);
    }

    // ==== Format Conversion Tests ====
    #[test]
    fn test_convert_files_map_to_vec_single_file() {
        let mut map = HashMap::new();
        map.insert("lidar.pcd".to_string(), "scan001.pcd".to_string());

        let files = convert_files_map_to_vec(map);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_type(), "lidar.pcd");
        assert_eq!(files[0].filename(), Some("scan001.pcd"));
    }

    #[test]
    fn test_convert_files_map_to_vec_multiple_files() {
        let mut map = HashMap::new();
        map.insert("lidar.pcd".to_string(), "scan.pcd".to_string());
        map.insert("radar.pcd".to_string(), "radar.pcd".to_string());

        let files = convert_files_map_to_vec(map);
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_convert_files_map_to_vec_empty() {
        let map = HashMap::new();
        let files = convert_files_map_to_vec(map);
        assert_eq!(files.len(), 0);
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
}
