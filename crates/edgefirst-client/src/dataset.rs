// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use std::fmt::Display;

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
/// let image_type = FileType::from("image");
/// let lidar_type = FileType::from("lidar.pcd");
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

impl From<&str> for FileType {
    fn from(s: &str) -> Self {
        match s {
            "image" => FileType::Image,
            "lidar.pcd" => FileType::LidarPcd,
            "lidar.png" => FileType::LidarDepth,
            "lidar.jpg" => FileType::LidarReflect,
            "radar.pcd" => FileType::RadarPcd,
            "radar.png" => FileType::RadarCube,
            _ => panic!("Invalid file type"),
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
/// // Create annotation types from strings
/// let box_2d = AnnotationType::from("box2d");
/// let segmentation = AnnotationType::from("mask");
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

impl From<&str> for AnnotationType {
    fn from(s: &str) -> Self {
        match s {
            "box2d" => AnnotationType::Box2d,
            "box3d" => AnnotationType::Box3d,
            "mask" => AnnotationType::Mask,
            _ => panic!("Invalid annotation type"),
        }
    }
}

impl From<String> for AnnotationType {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl From<&String> for AnnotationType {
    fn from(s: &String) -> Self {
        s.as_str().into()
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
/// along with their corresponding annotations (bounding boxes, segmentation masks,
/// 3D annotations). Datasets belong to projects and can be used for training
/// and validation of machine learning models.
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Sample {
    id: SampleID,
    #[serde(alias = "group_name", skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    sequence_name: Option<String>,
    image_name: String,
    image_url: String,
    #[serde(rename = "sensors")]
    files: Option<Vec<SampleFile>>,
    annotations: Option<Vec<Annotation>>,
}

impl Display for Sample {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.uid(), self.image_name())
    }
}

impl Sample {
    pub fn id(&self) -> SampleID {
        self.id
    }

    pub fn uid(&self) -> String {
        self.id.to_string()
    }

    pub fn name(&self) -> String {
        let name = self
            .image_name
            .rsplit_once('.')
            .map_or_else(|| self.image_name.clone(), |(name, _)| name.to_string());
        name.rsplit_once(".camera")
            .map_or_else(|| name.clone(), |(name, _)| name.to_string())
    }

    pub fn group(&self) -> Option<&String> {
        self.group.as_ref()
    }

    pub fn sequence_name(&self) -> Option<&String> {
        self.sequence_name.as_ref()
    }

    pub fn image_name(&self) -> &str {
        &self.image_name
    }

    pub fn image_url(&self) -> &str {
        &self.image_url
    }

    pub fn files(&self) -> &[SampleFile] {
        match &self.files {
            Some(files) => files,
            None => &[],
        }
    }

    pub fn annotations(&self) -> &[Annotation] {
        match &self.annotations {
            Some(annotations) => annotations,
            None => &[],
        }
    }

    pub fn with_annotations(mut self, annotations: Vec<Annotation>) -> Self {
        self.annotations = Some(annotations);
        self
    }

    pub async fn download(
        &self,
        client: &Client,
        file_type: FileType,
    ) -> Result<Option<Vec<u8>>, Error> {
        let url = match file_type {
            FileType::Image => Some(&self.image_url),
            file => self
                .files
                .as_ref()
                .and_then(|files| files.iter().find(|f| f.r#type == file.to_string()))
                .map(|f| &f.url),
        };

        Ok(match url {
            Some(url) => Some(client.download(url).await?),
            None => None,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SampleFile {
    r#type: String,
    url: String,
}

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Annotation {
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_id: Option<SampleID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    object_id: Option<String>,
    #[serde(alias = "label_name", skip_serializing_if = "Option::is_none")]
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

    pub fn label(&self) -> Option<&String> {
        self.label.as_ref()
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

    pub fn box3d(&self) -> Option<&Box3d> {
        self.box3d.as_ref()
    }

    pub fn mask(&self) -> Option<&Mask> {
        self.mask.as_ref()
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
pub struct Group {
    pub id: u64, // Groups seem to use raw u64, not a specific ID type
    pub name: String,
}

#[cfg(feature = "polars")]
pub fn annotations_dataframe(annotations: &[Annotation]) -> DataFrame {
    use itertools::Itertools;
    use log::warn;
    use std::path::Path;

    let (names, frames, objects, labels, label_indices, groups, masks, boxes2d, boxes3d) =
        annotations
            .iter()
            .map(|ann| {
                let name = match &ann.name {
                    Some(name) => name,
                    None => {
                        warn!("annotation missing image name, skipping");
                        return (
                            String::new(),
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                        );
                    }
                };

                let name = Path::new(name).file_stem().unwrap().to_str().unwrap();

                let (name, frame) = match &ann.sequence_name {
                    Some(sequence) => match name.strip_prefix(sequence) {
                        Some(frame) => (
                            sequence.to_string(),
                            Some(frame.trim_start_matches('_').to_string()),
                        ),
                        None => {
                            warn!(
                                "image_name {} does not match sequence_name {}",
                                name, sequence
                            );
                            return (
                                String::new(),
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                                None,
                            );
                        }
                    },
                    None => (name.to_string(), None),
                };

                let masks = match &ann.mask {
                    Some(seg) => {
                        use polars::series::Series;

                        let mut list = Vec::new();
                        for polygon in &seg.polygon {
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

                        Some(Series::new("mask".into(), list))
                    }
                    None => Option::<Series>::None,
                };

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

                (
                    name,
                    frame,
                    ann.object_id.clone(),
                    ann.label.clone(),
                    ann.label_index,
                    ann.group.clone(),
                    masks,
                    box2d,
                    box3d,
                )
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
        ))
        .unwrap()
        .into();
    let label_indices = Series::new("label_index".into(), label_indices).into();
    let groups = Series::new("group".into(), groups)
        .cast(&DataType::Categorical(
            Categories::new("groups".into(), "groups".into(), CategoricalPhysical::U8),
            Arc::new(CategoricalMapping::new(u8::MAX as usize)),
        ))
        .unwrap()
        .into();
    let masks = Series::new("mask".into(), masks)
        .cast(&DataType::List(Box::new(DataType::Float32)))
        .unwrap()
        .into();
    let boxes2d = Series::new("box2d".into(), boxes2d)
        .cast(&DataType::Array(Box::new(DataType::Float32), 4))
        .unwrap()
        .into();
    let boxes3d = Series::new("box3d".into(), boxes3d)
        .cast(&DataType::Array(Box::new(DataType::Float32), 6))
        .unwrap()
        .into();

    DataFrame::new(vec![
        names,
        frames,
        objects,
        labels,
        label_indices,
        groups,
        masks,
        boxes2d,
        boxes3d,
    ])
    .unwrap()
}
