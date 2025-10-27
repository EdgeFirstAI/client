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
        write!(f, "{} {}", self.uid(), self.name)
    }
}

impl Dataset {
    pub fn id(&self) -> DatasetID {
        self.id
    }

    pub fn uid(&self) -> String {
        self.id.to_string()
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
        write!(f, "{} {}", self.uid(), self.name)
    }
}

impl AnnotationSet {
    pub fn id(&self) -> AnnotationSetID {
        self.id
    }

    pub fn uid(&self) -> String {
        self.id.to_string()
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
    #[serde(alias = "group_name", skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Additional sensor files (LiDAR, radar, depth maps, etc.).
    /// When deserializing from samples.list: Vec<SampleFile>
    /// When serializing for samples.populate: HashMap<String, String>
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
            FilesFormat::Map(map) => map
                .into_iter()
                .map(|(file_type, filename)| SampleFile::with_filename(file_type, filename))
                .collect(),
        })
        .unwrap_or_default())
}

// Custom serializer for annotations field - converts Vec<Annotation> to
// format expected by server: {"bbox": [...], "box3d": [...], "mask": [...]}
fn serialize_annotations<S>(annotations: &Vec<Annotation>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap;

    // Group annotations by type
    let mut bbox_annotations = Vec::new();
    let mut box3d_annotations = Vec::new();
    let mut mask_annotations = Vec::new();

    for ann in annotations {
        if ann.box2d().is_some() {
            bbox_annotations.push(ann);
        } else if ann.box3d().is_some() {
            box3d_annotations.push(ann);
        } else if ann.mask().is_some() {
            mask_annotations.push(ann);
        }
    }

    let mut map = serializer.serialize_map(Some(3))?;

    if !bbox_annotations.is_empty() {
        map.serialize_entry("bbox", &bbox_annotations)?;
    }
    if !box3d_annotations.is_empty() {
        map.serialize_entry("box3d", &box3d_annotations)?;
    }
    if !mask_annotations.is_empty() {
        map.serialize_entry("mask", &mask_annotations)?;
    }

    map.end()
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
            AnnotationsFormat::Map(map) => {
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
        })
        .unwrap_or_default())
}

impl Display for Sample {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.uid().unwrap_or_else(|| "unknown".to_string()),
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
            files: vec![],
            annotations: vec![],
        }
    }

    pub fn id(&self) -> Option<SampleID> {
        self.id
    }

    pub fn uid(&self) -> Option<String> {
        self.id.map(|id| id.to_string())
    }

    pub fn name(&self) -> Option<String> {
        self.image_name.as_ref().map(|image_name| {
            let name = image_name
                .rsplit_once('.')
                .map_or_else(|| image_name.clone(), |(name, _)| name.to_string());
            name.rsplit_once(".camera")
                .map_or_else(|| name.clone(), |(name, _)| name.to_string())
        })
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

    pub async fn download(
        &self,
        client: &Client,
        file_type: FileType,
    ) -> Result<Option<Vec<u8>>, Error> {
        let url = match file_type {
            FileType::Image => self.image_url.as_ref(),
            file => self
                .files
                .iter()
                .find(|f| f.r#type == file.to_string())
                .and_then(|f| f.url.as_ref()),
        };

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

/// IMU orientation data (roll, pitch, yaw in degrees).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImuData {
    pub roll: f64,
    pub pitch: f64,
    pub yaw: f64,
}

#[allow(dead_code)]
pub trait TypeName {
    fn type_name() -> String;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Deserialize, Clone, Debug)]
#[serde(from = "AnnotationHelper")]
pub struct Annotation {
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_id: Option<SampleID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(rename = "object_reference", skip_serializing_if = "Option::is_none")]
    object_id: Option<String>,
    #[serde(rename = "label_name", skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box2d: Option<Box2d>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box3d: Option<Box3d>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mask: Option<Mask>,
}

// Helper struct for deserialization that matches the nested format
#[derive(Deserialize)]
struct AnnotationHelper {
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_id: Option<SampleID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(rename = "object_reference", skip_serializing_if = "Option::is_none")]
    object_id: Option<String>,
    #[serde(rename = "label_name", skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box2d: Option<Box2d>,
    #[serde(skip_serializing_if = "Option::is_none")]
    box3d: Option<Box3d>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mask: Option<Mask>,
}

impl From<AnnotationHelper> for Annotation {
    fn from(helper: AnnotationHelper) -> Self {
        Self {
            sample_id: helper.sample_id,
            name: helper.name,
            sequence_name: helper.sequence_name,
            group: helper.group,
            object_id: helper.object_id,
            label: helper.label,
            label_index: helper.label_index,
            box2d: helper.box2d,
            box3d: helper.box3d,
            mask: helper.mask,
        }
    }
}

// Custom serializer that flattens box2d/box3d fields
impl Serialize for Annotation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;

        if let Some(ref sample_id) = self.sample_id {
            map.serialize_entry("sample_id", sample_id)?;
        }
        if let Some(ref name) = self.name {
            map.serialize_entry("name", name)?;
        }
        if let Some(ref sequence_name) = self.sequence_name {
            map.serialize_entry("sequence_name", sequence_name)?;
        }
        if let Some(ref group) = self.group {
            map.serialize_entry("group", group)?;
        }
        if let Some(ref object_id) = self.object_id {
            map.serialize_entry("object_reference", object_id)?;
        }
        if let Some(ref label) = self.label {
            map.serialize_entry("label_name", label)?;
        }
        if let Some(label_index) = self.label_index {
            map.serialize_entry("label_index", &label_index)?;
        }

        // Flatten box2d fields
        if let Some(ref box2d) = self.box2d {
            map.serialize_entry("x", &box2d.x)?;
            map.serialize_entry("y", &box2d.y)?;
            map.serialize_entry("w", &box2d.w)?;
            map.serialize_entry("h", &box2d.h)?;
        }

        // Flatten box3d fields
        if let Some(ref box3d) = self.box3d {
            map.serialize_entry("x", &box3d.x)?;
            map.serialize_entry("y", &box3d.y)?;
            map.serialize_entry("z", &box3d.z)?;
            map.serialize_entry("w", &box3d.w)?;
            map.serialize_entry("h", &box3d.h)?;
            map.serialize_entry("l", &box3d.l)?;
        }

        if let Some(ref mask) = self.mask {
            map.serialize_entry("mask", mask)?;
        }

        map.end()
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
            group: None,
            object_id: None,
            label: None,
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

    pub fn label(&self) -> Option<&String> {
        self.label.as_ref()
    }

    pub fn set_label(&mut self, label: Option<String>) {
        self.label = label;
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
fn extract_annotation_name(ann: &Annotation) -> Option<(String, Option<String>)> {
    use log::warn;
    use std::path::Path;

    let name = ann.name.as_ref()?;

    let name = Path::new(name).file_stem()?.to_str()?;

    match &ann.sequence_name {
        Some(sequence) => match name.strip_prefix(sequence) {
            Some(frame) => Some((
                sequence.clone(),
                Some(frame.trim_start_matches('_').to_owned()),
            )),
            None => {
                warn!(
                    "image_name {} does not match sequence_name {}",
                    name, sequence
                );
                None
            }
        },
        None => Some((name.to_string(), None)),
    }
}

#[cfg(feature = "polars")]
fn convert_mask_to_series(mask: &Mask) -> Series {
    use polars::series::Series;

    let mut list = Vec::new();
    for polygon in &mask.polygon {
        for &(x, y) in polygon {
            list.push(x);
            list.push(y);
        }
        // Separate polygons with NaN
        list.push(f32::NAN);
    }

    // Remove the last NaN if it exists
    let list = if !list.is_empty() {
        list[..list.len() - 1].to_vec()
    } else {
        vec![]
    };

    Series::new("mask".into(), list)
}

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
                    ann.object_id.clone(),
                    ann.label.clone(),
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
