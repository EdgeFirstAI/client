// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use crate::{AnnotationSet, Client, Dataset, Error, Progress, Sample, client};
use chrono::{DateTime, Utc};
use log::trace;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::HashMap, fmt::Display, path::PathBuf, str::FromStr};

/// Generic parameter value used in API requests and configuration.
///
/// This enum represents various data types that can be passed as parameters
/// to EdgeFirst Studio API calls or stored in configuration files.
///
/// # Examples
///
/// ```rust
/// use edgefirst_client::Parameter;
/// use std::collections::HashMap;
///
/// // Different parameter types
/// let int_param = Parameter::Integer(42);
/// let float_param = Parameter::Real(3.14);
/// let bool_param = Parameter::Boolean(true);
/// let string_param = Parameter::String("model_name".to_string());
///
/// // Complex nested parameters
/// let array_param = Parameter::Array(vec![
///     Parameter::Integer(1),
///     Parameter::Integer(2),
///     Parameter::Integer(3),
/// ]);
///
/// let mut config = HashMap::new();
/// config.insert("learning_rate".to_string(), Parameter::Real(0.001));
/// config.insert("epochs".to_string(), Parameter::Integer(100));
/// let object_param = Parameter::Object(config);
/// ```
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Parameter {
    /// 64-bit signed integer value.
    Integer(i64),
    /// 64-bit floating-point value.
    Real(f64),
    /// Boolean true/false value.
    Boolean(bool),
    /// UTF-8 string value.
    String(String),
    /// Array of nested parameter values.
    Array(Vec<Parameter>),
    /// Object/map with string keys and parameter values.
    Object(HashMap<String, Parameter>),
}

#[derive(Deserialize)]
pub struct LoginResult {
    pub(crate) token: String,
}

/// Generates a TypeID newtype struct with full conversion support.
///
/// Each invocation creates a `Copy + Clone + Debug + PartialEq + Eq + Hash`
/// newtype wrapping `u64`, with `Display`, `FromStr`, `TryFrom<&str>`,
/// `TryFrom<String>`, `From<u64>`, and `From<T> for u64` implementations.
///
/// The string representation uses the format `"{prefix}-{hex}"` where the
/// hex part is the lowercase hexadecimal encoding of the inner `u64` value.
macro_rules! typeid {
    ($(#[$meta:meta])* $name:ident, $prefix:literal) => {
        $(#[$meta])*
        #[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
        pub struct $name(u64);

        impl Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, concat!($prefix, "-{:x}"), self.0)
            }
        }

        impl From<u64> for $name {
            fn from(id: u64) -> Self {
                $name(id)
            }
        }

        impl From<$name> for u64 {
            fn from(val: $name) -> Self {
                val.0
            }
        }

        impl $name {
            /// Returns the raw `u64` value of this identifier.
            pub fn value(&self) -> u64 {
                self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = Error;

            fn try_from(s: &str) -> Result<Self, Self::Error> {
                $name::from_str(s)
            }
        }

        impl TryFrom<String> for $name {
            type Error = Error;

            fn try_from(s: String) -> Result<Self, Self::Error> {
                $name::from_str(&s)
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let hex_part =
                    s.strip_prefix(concat!($prefix, "-")).ok_or_else(|| {
                        Error::InvalidParameters(format!(
                            "{} must start with '{}-' prefix",
                            stringify!($name),
                            $prefix
                        ))
                    })?;
                let id = u64::from_str_radix(hex_part, 16)?;
                Ok($name(id))
            }
        }
    };
}

typeid!(
    /// Unique identifier for an organization in EdgeFirst Studio.
    ///
    /// Organizations are the top-level containers for users, projects, and
    /// resources in EdgeFirst Studio. Each organization has a unique ID that is
    /// displayed in hexadecimal format with an "org-" prefix (e.g., "org-abc123").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgefirst_client::OrganizationID;
    ///
    /// // Create from u64
    /// let org_id = OrganizationID::from(12345);
    /// println!("{}", org_id); // Displays: org-3039
    ///
    /// // Parse from string
    /// let org_id: OrganizationID = "org-abc123".try_into().unwrap();
    /// assert_eq!(org_id.value(), 0xabc123);
    /// ```
    OrganizationID,
    "org"
);

/// Organization information and metadata.
///
/// Each user belongs to an organization which contains projects, datasets,
/// and other resources. Organizations provide isolated workspaces for teams
/// and manage resource quotas and billing.
///
/// # Examples
///
/// ```no_run
/// use edgefirst_client::{Client, Organization};
///
/// # async fn example() -> Result<(), edgefirst_client::Error> {
/// # let client = Client::new()?;
/// // Access organization details
/// let org: Organization = client.organization().await?;
/// println!("Organization: {} (ID: {})", org.name(), org.id());
/// println!("Available credits: {}", org.credits());
/// # Ok(())
/// # }
/// ```
#[derive(Deserialize, Clone, Debug)]
pub struct Organization {
    id: OrganizationID,
    name: String,
    #[serde(rename = "latest_credit")]
    credits: i64,
}

impl Display for Organization {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Organization {
    pub fn id(&self) -> OrganizationID {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn credits(&self) -> i64 {
        self.credits
    }
}

typeid!(
    /// Unique identifier for a project within EdgeFirst Studio.
    ///
    /// Projects contain datasets, experiments, and models within an organization.
    /// Each project has a unique ID displayed in hexadecimal format with a "p-"
    /// prefix (e.g., "p-def456").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgefirst_client::ProjectID;
    /// use std::str::FromStr;
    ///
    /// // Create from u64
    /// let project_id = ProjectID::from(78910);
    /// println!("{}", project_id); // Displays: p-1343e
    ///
    /// // Parse from string
    /// let project_id = ProjectID::from_str("p-def456").unwrap();
    /// assert_eq!(project_id.value(), 0xdef456);
    /// ```
    ProjectID,
    "p"
);

typeid!(
    /// Unique identifier for an experiment within a project.
    ///
    /// Experiments represent individual machine learning experiments with specific
    /// configurations, datasets, and results. Each experiment has a unique ID
    /// displayed in hexadecimal format with an "exp-" prefix (e.g., "exp-123abc").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgefirst_client::ExperimentID;
    /// use std::str::FromStr;
    ///
    /// // Create from u64
    /// let exp_id = ExperimentID::from(1193046);
    /// println!("{}", exp_id); // Displays: exp-123abc
    ///
    /// // Parse from string
    /// let exp_id = ExperimentID::from_str("exp-456def").unwrap();
    /// assert_eq!(exp_id.value(), 0x456def);
    /// ```
    ExperimentID,
    "exp"
);

typeid!(
    /// Unique identifier for a training session within an experiment.
    ///
    /// Training sessions represent individual training runs with specific
    /// hyperparameters and configurations. Each training session has a unique ID
    /// displayed in hexadecimal format with a "t-" prefix (e.g., "t-789012").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgefirst_client::TrainingSessionID;
    /// use std::str::FromStr;
    ///
    /// // Create from u64
    /// let training_id = TrainingSessionID::from(7901234);
    /// println!("{}", training_id); // Displays: t-7872f2
    ///
    /// // Parse from string
    /// let training_id = TrainingSessionID::from_str("t-abc123").unwrap();
    /// assert_eq!(training_id.value(), 0xabc123);
    /// ```
    TrainingSessionID,
    "t"
);

typeid!(
    /// Unique identifier for a validation session within an experiment.
    ///
    /// Validation sessions represent model validation runs that evaluate trained
    /// models against test datasets. Each validation session has a unique ID
    /// displayed in hexadecimal format with a "v-" prefix (e.g., "v-345678").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgefirst_client::ValidationSessionID;
    ///
    /// // Create from u64
    /// let validation_id = ValidationSessionID::from(3456789);
    /// println!("{}", validation_id); // Displays: v-34c985
    ///
    /// // Parse from string
    /// let validation_id: ValidationSessionID = "v-deadbeef".try_into().unwrap();
    /// assert_eq!(validation_id.value(), 0xdeadbeef);
    /// ```
    ValidationSessionID,
    "v"
);

typeid!(
    /// Unique identifier for a snapshot in EdgeFirst Studio.
    ///
    /// Snapshots represent saved states of datasets or model checkpoints.
    /// Each snapshot has a unique ID displayed in hexadecimal format with
    /// an "ss-" prefix (e.g., "ss-f1e2d3").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgefirst_client::SnapshotID;
    /// use std::str::FromStr;
    ///
    /// let snapshot_id = SnapshotID::from_str("ss-abc123").unwrap();
    /// assert_eq!(snapshot_id.value(), 0xabc123);
    /// ```
    SnapshotID,
    "ss"
);

typeid!(
    /// Unique identifier for a task in EdgeFirst Studio.
    ///
    /// Tasks represent background operations such as training, validation,
    /// export, or dataset processing. Each task has a unique ID displayed
    /// in hexadecimal format with a "task-" prefix (e.g., "task-8e7d6c").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgefirst_client::TaskID;
    /// use std::str::FromStr;
    ///
    /// let task_id = TaskID::from_str("task-abc123").unwrap();
    /// assert_eq!(task_id.value(), 0xabc123);
    /// ```
    TaskID,
    "task"
);

typeid!(
    /// Unique identifier for a dataset within a project.
    ///
    /// Datasets contain collections of images, annotations, and other data used for
    /// machine learning experiments. Each dataset has a unique ID displayed in
    /// hexadecimal format with a "ds-" prefix (e.g., "ds-123abc").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgefirst_client::DatasetID;
    /// use std::str::FromStr;
    ///
    /// // Create from u64
    /// let dataset_id = DatasetID::from(1193046);
    /// println!("{}", dataset_id); // Displays: ds-123abc
    ///
    /// // Parse from string
    /// let dataset_id = DatasetID::from_str("ds-456def").unwrap();
    /// assert_eq!(dataset_id.value(), 0x456def);
    /// ```
    DatasetID,
    "ds"
);

typeid!(
    /// Unique identifier for an annotation set within a dataset.
    ///
    /// Annotation sets group related annotations together. Each annotation set
    /// has a unique ID displayed in hexadecimal format with an "as-" prefix
    /// (e.g., "as-3d2c1b").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgefirst_client::AnnotationSetID;
    /// use std::str::FromStr;
    ///
    /// let as_id = AnnotationSetID::from_str("as-abc123").unwrap();
    /// assert_eq!(as_id.value(), 0xabc123);
    /// ```
    AnnotationSetID,
    "as"
);

typeid!(
    /// Unique identifier for a sample within a dataset.
    ///
    /// Samples represent individual data points (images, point clouds, etc.)
    /// in a dataset. Each sample has a unique ID displayed in hexadecimal
    /// format with an "s-" prefix (e.g., "s-6c5b4a").
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgefirst_client::SampleID;
    /// use std::str::FromStr;
    ///
    /// let sample_id = SampleID::from_str("s-abc123").unwrap();
    /// assert_eq!(sample_id.value(), 0xabc123);
    /// ```
    SampleID,
    "s"
);

typeid!(
    /// Unique identifier for an application in EdgeFirst Studio.
    ///
    /// Applications represent deployed models or inference endpoints.
    /// Each application has a unique ID displayed in hexadecimal format
    /// with an "app-" prefix (e.g., "app-2e1d0c").
    AppId,
    "app"
);

typeid!(
    /// Unique identifier for an image in EdgeFirst Studio.
    ///
    /// Images are individual visual assets within a dataset sample.
    /// Each image has a unique ID displayed in hexadecimal format
    /// with an "im-" prefix (e.g., "im-4c3b2a").
    ImageId,
    "im"
);

typeid!(
    /// Unique identifier for a sequence in EdgeFirst Studio.
    ///
    /// Sequences represent temporal groupings of samples (e.g., video frames).
    /// Each sequence has a unique ID displayed in hexadecimal format
    /// with an "se-" prefix (e.g., "se-7f6e5d").
    SequenceId,
    "se"
);

/// The project class represents a project in the EdgeFirst Studio.  A project
/// contains datasets, experiments, and other resources related to a specific
/// task or workflow.
#[derive(Deserialize, Clone, Debug)]
pub struct Project {
    id: ProjectID,
    name: String,
    description: String,
}

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.id(), self.name())
    }
}

impl Project {
    pub fn id(&self) -> ProjectID {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub async fn datasets(
        &self,
        client: &client::Client,
        name: Option<&str>,
    ) -> Result<Vec<Dataset>, Error> {
        client.datasets(self.id, name).await
    }

    pub async fn experiments(
        &self,
        client: &client::Client,
        name: Option<&str>,
    ) -> Result<Vec<Experiment>, Error> {
        client.experiments(self.id, name).await
    }
}

#[derive(Deserialize, Debug)]
pub struct SamplesCountResult {
    pub total: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct SamplesListParams {
    pub dataset_id: DatasetID,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_set_id: Option<AnnotationSetID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_token: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub group_names: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct SamplesListResult {
    pub samples: Vec<Sample>,
    pub continue_token: Option<String>,
}

/// Parameters for populating (importing) samples into a dataset.
///
/// Used with the `samples.populate2` API to create new samples in a dataset,
/// optionally with annotations and sensor data files.
#[derive(Serialize, Clone, Debug)]
pub struct SamplesPopulateParams {
    pub dataset_id: DatasetID,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation_set_id: Option<AnnotationSetID>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presigned_urls: Option<bool>,
    pub samples: Vec<Sample>,
}

/// Result from the `samples.populate2` API call.
///
/// The API returns an array of populated sample results, one for each sample
/// that was submitted. Each result contains the sample UUID and presigned URLs
/// for uploading the associated files.
#[derive(Deserialize, Debug, Clone)]
pub struct SamplesPopulateResult {
    /// UUID of the sample that was populated
    pub uuid: String,
    /// Presigned URLs for uploading files for this sample
    pub urls: Vec<PresignedUrl>,
}

/// A presigned URL for uploading a file to S3.
#[derive(Deserialize, Debug, Clone)]
pub struct PresignedUrl {
    /// Filename as specified in the sample
    pub filename: String,
    /// S3 key path
    pub key: String,
    /// Presigned URL for uploading (PUT request)
    pub url: String,
}

// ============================================================================
// Annotation API Types
// ============================================================================

/// Annotation data for the server-side `annotation.add_bulk` API.
///
/// This struct represents annotations in the format expected by the server,
/// which differs from our client-side `Annotation` struct. Key differences:
/// - Uses `image_id` (server) vs `sample_id` (client)
/// - Uses `type` string ("box", "seg") vs `AnnotationType` enum
/// - Coordinates are stored as separate `x`, `y`, `w`, `h` fields
/// - Polygon is stored as a JSON string
#[derive(Serialize, Clone, Debug)]
pub struct ServerAnnotation {
    /// Label ID (resolved from label name before sending)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_id: Option<u64>,
    /// Label index (alternative to label_id)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_index: Option<u64>,
    /// Label name (alternative to label_id)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_name: Option<String>,
    /// Annotation type: "box" for bounding box, "seg" for segmentation
    #[serde(rename = "type")]
    pub annotation_type: String,
    /// Bounding box X coordinate (normalized 0-1, center)
    pub x: f64,
    /// Bounding box Y coordinate (normalized 0-1, center)
    pub y: f64,
    /// Bounding box width (normalized 0-1)
    pub w: f64,
    /// Bounding box height (normalized 0-1)
    pub h: f64,
    /// Confidence score (0-1)
    pub score: f64,
    /// Polygon data as JSON string (for segmentation)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub polygon: String,
    /// Image/sample ID in the database
    pub image_id: u64,
    /// Annotation set ID
    pub annotation_set_id: u64,
    /// Object tracking reference (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_reference: Option<String>,
}

/// Parameters for the `annotation.add_bulk` API.
#[derive(Serialize, Debug)]
pub struct AnnotationAddBulkParams {
    pub annotation_set_id: u64,
    pub annotations: Vec<ServerAnnotation>,
}

/// Parameters for the `annotation.bulk.del` API.
#[derive(Serialize, Debug)]
pub struct AnnotationBulkDeleteParams {
    pub annotation_set_id: u64,
    pub annotation_types: Vec<String>,
    /// Image IDs to delete annotations from (required if delete_all is false)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub image_ids: Vec<u64>,
    /// Delete all annotations of the specified types in the annotation set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_all: Option<bool>,
}

#[derive(Deserialize)]
pub struct Snapshot {
    id: SnapshotID,
    description: String,
    status: String,
    path: String,
    #[serde(rename = "date")]
    created: DateTime<Utc>,
}

impl Display for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.description)
    }
}

impl Snapshot {
    pub fn id(&self) -> SnapshotID {
        self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn created(&self) -> &DateTime<Utc> {
        &self.created
    }
}

#[derive(Serialize, Debug)]
pub struct SnapshotRestore {
    pub project_id: ProjectID,
    pub snapshot_id: SnapshotID,
    pub fps: u64,
    #[serde(rename = "enabled_topics", skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    #[serde(rename = "label_names", skip_serializing_if = "Vec::is_empty")]
    pub autolabel: Vec<String>,
    #[serde(rename = "depth_gen")]
    pub autodepth: bool,
    pub agtg_pipeline: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset_description: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct SnapshotRestoreResult {
    pub id: SnapshotID,
    pub description: String,
    pub dataset_name: String,
    pub dataset_id: DatasetID,
    pub annotation_set_id: AnnotationSetID,
    #[serde(default)]
    pub task_id: Option<TaskID>,
    pub date: DateTime<Utc>,
}

/// Parameters for creating a snapshot from an existing dataset on the server.
///
/// This is used with the `snapshots.create` RPC to trigger server-side snapshot
/// generation from dataset data (images + annotations).
#[derive(Serialize, Debug)]
pub struct SnapshotCreateFromDataset {
    /// Name/description for the snapshot
    pub description: String,
    /// Dataset ID to create snapshot from
    pub dataset_id: DatasetID,
    /// Annotation set ID to use for snapshot creation
    pub annotation_set_id: AnnotationSetID,
}

/// Result of creating a snapshot from an existing dataset.
///
/// Contains the snapshot ID and task ID for monitoring progress.
#[derive(Deserialize, Debug)]
pub struct SnapshotFromDatasetResult {
    /// The created snapshot ID
    #[serde(alias = "snapshot_id")]
    pub id: SnapshotID,
    /// Task ID for monitoring snapshot creation progress
    #[serde(default)]
    pub task_id: Option<TaskID>,
}

#[derive(Deserialize)]
pub struct Experiment {
    id: ExperimentID,
    project_id: ProjectID,
    name: String,
    description: String,
}

impl Display for Experiment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.name)
    }
}

impl Experiment {
    pub fn id(&self) -> ExperimentID {
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

    pub async fn project(&self, client: &client::Client) -> Result<Project, Error> {
        client.project(self.project_id).await
    }

    pub async fn training_sessions(
        &self,
        client: &client::Client,
        name: Option<&str>,
    ) -> Result<Vec<TrainingSession>, Error> {
        client.training_sessions(self.id, name).await
    }
}

#[derive(Serialize, Debug)]
pub struct PublishMetrics {
    #[serde(rename = "trainer_session_id", skip_serializing_if = "Option::is_none")]
    pub trainer_session_id: Option<TrainingSessionID>,
    #[serde(
        rename = "validate_session_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub validate_session_id: Option<ValidationSessionID>,
    pub metrics: HashMap<String, Parameter>,
}

#[derive(Deserialize)]
struct TrainingSessionParams {
    model_params: HashMap<String, Parameter>,
    dataset_params: DatasetParams,
}

#[derive(Deserialize)]
pub struct TrainingSession {
    id: TrainingSessionID,
    #[serde(rename = "trainer_id")]
    experiment_id: ExperimentID,
    model: String,
    name: String,
    description: String,
    params: TrainingSessionParams,
    #[serde(rename = "docker_task")]
    task: Task,
}

impl Display for TrainingSession {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}", self.id, self.name())
    }
}

impl TrainingSession {
    pub fn id(&self) -> TrainingSessionID {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn experiment_id(&self) -> ExperimentID {
        self.experiment_id
    }

    pub fn task(&self) -> Task {
        self.task.clone()
    }

    pub fn model_params(&self) -> &HashMap<String, Parameter> {
        &self.params.model_params
    }

    pub fn dataset_params(&self) -> &DatasetParams {
        &self.params.dataset_params
    }

    pub fn train_group(&self) -> &str {
        &self.params.dataset_params.train_group
    }

    pub fn val_group(&self) -> &str {
        &self.params.dataset_params.val_group
    }

    pub async fn experiment(&self, client: &client::Client) -> Result<Experiment, Error> {
        client.experiment(self.experiment_id).await
    }

    pub async fn dataset(&self, client: &client::Client) -> Result<Dataset, Error> {
        client.dataset(self.params.dataset_params.dataset_id).await
    }

    pub async fn annotation_set(&self, client: &client::Client) -> Result<AnnotationSet, Error> {
        client
            .annotation_set(self.params.dataset_params.annotation_set_id)
            .await
    }

    pub async fn artifacts(&self, client: &client::Client) -> Result<Vec<Artifact>, Error> {
        client.artifacts(self.id).await
    }

    pub async fn metrics(
        &self,
        client: &client::Client,
    ) -> Result<HashMap<String, Parameter>, Error> {
        #[derive(Deserialize)]
        #[serde(untagged, deny_unknown_fields, expecting = "map, empty map or string")]
        enum Response {
            Empty {},
            Map(HashMap<String, Parameter>),
            String(String),
        }

        let params = HashMap::from([("trainer_session_id", self.id().value())]);
        let resp: Response = client
            .rpc("trainer.session.metrics".to_owned(), Some(params))
            .await?;

        Ok(match resp {
            Response::String(metrics) => serde_json::from_str(&metrics)?,
            Response::Map(metrics) => metrics,
            Response::Empty {} => HashMap::new(),
        })
    }

    pub async fn set_metrics(
        &self,
        client: &client::Client,
        metrics: HashMap<String, Parameter>,
    ) -> Result<(), Error> {
        let metrics = PublishMetrics {
            trainer_session_id: Some(self.id()),
            validate_session_id: None,
            metrics,
        };

        let _: String = client
            .rpc("trainer.session.metrics".to_owned(), Some(metrics))
            .await?;

        Ok(())
    }

    /// Downloads an artifact from the training session.
    pub async fn download_artifact(
        &self,
        client: &client::Client,
        filename: &str,
    ) -> Result<Vec<u8>, Error> {
        client
            .fetch(&format!(
                "download_model?training_session_id={}&file={}",
                self.id().value(),
                filename
            ))
            .await
    }

    /// Uploads an artifact to the training session.  The filename will
    /// be used as the name of the file in the training session while path is
    /// the local path to the file to upload.
    pub async fn upload_artifact(
        &self,
        client: &client::Client,
        filename: &str,
        path: PathBuf,
    ) -> Result<(), Error> {
        self.upload(client, &[(format!("artifacts/{}", filename), path)])
            .await
    }

    /// Downloads a checkpoint file from the training session.
    pub async fn download_checkpoint(
        &self,
        client: &client::Client,
        filename: &str,
    ) -> Result<Vec<u8>, Error> {
        client
            .fetch(&format!(
                "download_checkpoint?folder=checkpoints&training_session_id={}&file={}",
                self.id().value(),
                filename
            ))
            .await
    }

    /// Uploads a checkpoint file to the training session.  The filename will
    /// be used as the name of the file in the training session while path is
    /// the local path to the file to upload.
    pub async fn upload_checkpoint(
        &self,
        client: &client::Client,
        filename: &str,
        path: PathBuf,
    ) -> Result<(), Error> {
        self.upload(client, &[(format!("checkpoints/{}", filename), path)])
            .await
    }

    /// Downloads a file from the training session.  Should only be used for
    /// text files, binary files must be downloaded using download_artifact or
    /// download_checkpoint.
    pub async fn download(&self, client: &client::Client, filename: &str) -> Result<String, Error> {
        #[derive(Serialize)]
        struct DownloadRequest {
            session_id: TrainingSessionID,
            file_path: String,
        }

        let params = DownloadRequest {
            session_id: self.id(),
            file_path: filename.to_string(),
        };

        client
            .rpc("trainer.download.file".to_owned(), Some(params))
            .await
    }

    pub async fn upload(
        &self,
        client: &client::Client,
        files: &[(String, PathBuf)],
    ) -> Result<(), Error> {
        let mut parts = Form::new().part(
            "params",
            Part::text(format!("{{ \"session_id\": {} }}", self.id().value())),
        );

        for (name, path) in files {
            let file_part = Part::file(path).await?.file_name(name.to_owned());
            parts = parts.part("file", file_part);
        }

        let result = client.post_multipart("trainer.upload.files", parts).await?;
        trace!("TrainingSession::upload: {:?}", result);
        Ok(())
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ValidationSession {
    id: ValidationSessionID,
    description: String,
    dataset_id: DatasetID,
    experiment_id: ExperimentID,
    training_session_id: TrainingSessionID,
    #[serde(rename = "gt_annotation_set_id")]
    annotation_set_id: AnnotationSetID,
    #[serde(deserialize_with = "validation_session_params")]
    params: HashMap<String, Parameter>,
    #[serde(rename = "docker_task")]
    task: Task,
}

fn validation_session_params<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Parameter>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct ModelParams {
        validation: Option<HashMap<String, Parameter>>,
    }

    #[derive(Deserialize)]
    struct ValidateParams {
        model: String,
    }

    #[derive(Deserialize)]
    struct Params {
        model_params: ModelParams,
        validate_params: ValidateParams,
    }

    let params = Params::deserialize(deserializer)?;
    let params = match params.model_params.validation {
        Some(mut map) => {
            map.insert(
                "model".to_string(),
                Parameter::String(params.validate_params.model),
            );
            map
        }
        None => HashMap::from([(
            "model".to_string(),
            Parameter::String(params.validate_params.model),
        )]),
    };

    Ok(params)
}

impl ValidationSession {
    pub fn id(&self) -> ValidationSessionID {
        self.id
    }

    pub fn name(&self) -> &str {
        self.task.name()
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn dataset_id(&self) -> DatasetID {
        self.dataset_id
    }

    pub fn experiment_id(&self) -> ExperimentID {
        self.experiment_id
    }

    pub fn training_session_id(&self) -> TrainingSessionID {
        self.training_session_id
    }

    pub fn annotation_set_id(&self) -> AnnotationSetID {
        self.annotation_set_id
    }

    pub fn params(&self) -> &HashMap<String, Parameter> {
        &self.params
    }

    pub fn task(&self) -> &Task {
        &self.task
    }

    pub async fn metrics(
        &self,
        client: &client::Client,
    ) -> Result<HashMap<String, Parameter>, Error> {
        #[derive(Deserialize)]
        #[serde(untagged, deny_unknown_fields, expecting = "map, empty map or string")]
        enum Response {
            Empty {},
            Map(HashMap<String, Parameter>),
            String(String),
        }

        let params = HashMap::from([("validate_session_id", self.id().value())]);
        let resp: Response = client
            .rpc("validate.session.metrics".to_owned(), Some(params))
            .await?;

        Ok(match resp {
            Response::String(metrics) => serde_json::from_str(&metrics)?,
            Response::Map(metrics) => metrics,
            Response::Empty {} => HashMap::new(),
        })
    }

    pub async fn set_metrics(
        &self,
        client: &client::Client,
        metrics: HashMap<String, Parameter>,
    ) -> Result<(), Error> {
        let metrics = PublishMetrics {
            trainer_session_id: None,
            validate_session_id: Some(self.id()),
            metrics,
        };

        let _: String = client
            .rpc("validate.session.metrics".to_owned(), Some(metrics))
            .await?;

        Ok(())
    }

    /// Uploads files to this validation session's data folder.
    ///
    /// **Breaking change**: this method replaces the former `upload`.
    /// It targets the new `val.data.upload` endpoint (which supports an optional
    /// `folder` argument and uses session-scoped permissions). The semantics
    /// differ from the old endpoint — the old `upload` cannot be silently
    /// repointed because the wire shapes differ (singular session_id, folder
    /// argument, different return shape).
    ///
    /// # Arguments
    /// * `client`   - The authenticated client instance.
    /// * `files`    - List of `(filename, path)` pairs to upload.
    /// * `folder`   - Optional logical subdirectory under the session data root.
    /// * `progress` - Optional progress channel. Emits `Progress { current,
    ///   total, status: None }` events as bytes are streamed to the server.
    ///   `total` equals the sum of all file sizes in bytes; `current` tracks
    ///   aggregate bytes sent across all files using a shared atomic counter.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns `Error::PermissionDenied` if the server rejects the request, or
    /// `Error::RpcError` for other server-side failures.
    pub async fn upload_data(
        &self,
        client: &client::Client,
        files: &[(String, std::path::PathBuf)],
        folder: Option<&str>,
        progress: Option<tokio::sync::mpsc::Sender<Progress>>,
    ) -> Result<(), Error> {
        use futures::StreamExt;
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        use tokio_util::io::ReaderStream;

        // Pre-compute total size across all files.
        let mut total: usize = 0;
        let mut file_meta = Vec::with_capacity(files.len());
        for (name, path) in files {
            let f = tokio::fs::File::open(path).await?;
            let len = f.metadata().await?.len() as usize;
            total += len;
            file_meta.push((name.clone(), f, len));
        }

        // Shared atomic counter so all file parts bump the same sent counter.
        let sent = Arc::new(AtomicUsize::new(0));

        let mut form = Form::new().text("session_id", self.id().value().to_string());
        if let Some(folder) = folder.filter(|s| !s.is_empty()) {
            form = form.text("folder", folder.to_owned());
        }

        for (name, file, len) in file_meta {
            let reader_stream = ReaderStream::new(file);
            let sent_clone = sent.clone();
            let progress_clone = progress.clone();
            let progress_stream = reader_stream.inspect(move |chunk_result| {
                if let Ok(chunk) = chunk_result {
                    let current =
                        sent_clone.fetch_add(chunk.len(), Ordering::Relaxed) + chunk.len();
                    if let Some(tx) = &progress_clone {
                        let _ = tx.try_send(Progress {
                            current,
                            total,
                            status: None,
                        });
                    }
                }
            });
            let body = reqwest::Body::wrap_stream(progress_stream);
            let part = Part::stream_with_length(body, len as u64).file_name(name);
            form = form.part("file", part);
        }

        match client.post_multipart("val.data.upload", form).await {
            Ok(_) => Ok(()),
            Err(Error::RpcError(code, msg)) => {
                Err(client::map_rpc_error("val.data.upload", code, msg, None))
            }
            Err(e) => Err(e),
        }
    }

    /// Streams a file from this validation session's data folder to `output_path`.
    ///
    /// # Arguments
    /// * `client`      - The authenticated client instance.
    /// * `filename`    - Name of the file to download (relative to the session data root).
    /// * `output_path` - Local path to write the downloaded file.
    /// * `progress`    - Optional progress channel; events carry bytes received
    ///   and `Content-Length` total (0 if server omits it).
    ///
    /// # Returns
    /// `Ok(())` when the file has been written and flushed.
    ///
    /// # Errors
    /// Returns `Error::PermissionDenied` if authorization fails,
    /// `Error::InvalidResponse` if the server returns JSON instead of binary, or
    /// `Error::IoError` on file write failures.
    pub async fn download_data(
        &self,
        client: &client::Client,
        filename: &str,
        output_path: &std::path::Path,
        progress: Option<tokio::sync::mpsc::Sender<Progress>>,
    ) -> Result<(), Error> {
        let req = client::ValDataDownloadRequest {
            session_id: self.id().value(),
            filename: filename.to_owned(),
        };
        match client
            .rpc_download("val.data.download", &req, output_path, progress)
            .await
        {
            Ok(()) => Ok(()),
            Err(Error::RpcError(code, msg)) => {
                Err(client::map_rpc_error("val.data.download", code, msg, None))
            }
            Err(e) => Err(e),
        }
    }

    /// Lists files attached to this validation session's data folder.
    ///
    /// The server returns a flat list of relative file paths
    /// (slash-separated, e.g. `"folder/file.txt"`), sorted lexicographically.
    ///
    /// # Arguments
    /// * `client` - The authenticated client instance.
    ///
    /// # Returns
    /// A flat `Vec<String>` of relative file paths within the session data folder.
    ///
    /// # Errors
    /// Returns `Error::PermissionDenied` if authorization fails, or
    /// `Error::RpcError` for other server-side failures.
    pub async fn data_list(&self, client: &client::Client) -> Result<Vec<String>, Error> {
        let req = client::ValDataListRequest {
            session_id: self.id().value(),
        };
        match client.rpc("val.data.list".to_owned(), Some(&req)).await {
            Ok(r) => Ok(r),
            Err(Error::RpcError(code, msg)) => {
                Err(client::map_rpc_error("val.data.list", code, msg, None))
            }
            Err(e) => Err(e),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct DatasetParams {
    dataset_id: DatasetID,
    annotation_set_id: AnnotationSetID,
    #[serde(rename = "train_group_name")]
    train_group: String,
    #[serde(rename = "val_group_name")]
    val_group: String,
}

impl DatasetParams {
    pub fn dataset_id(&self) -> DatasetID {
        self.dataset_id
    }

    pub fn annotation_set_id(&self) -> AnnotationSetID {
        self.annotation_set_id
    }

    pub fn train_group(&self) -> &str {
        &self.train_group
    }

    pub fn val_group(&self) -> &str {
        &self.val_group
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct TasksListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<String>>,
    #[serde(rename = "manage_types", skip_serializing_if = "Option::is_none")]
    pub manager: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Vec<String>>,
}

/// List of data and chart artefacts attached to a task.
///
/// Returned by `TaskInfo::data_list` and `TaskInfo::list_charts`. The `data`
/// map encodes the folder layout: keys are folder names, values are filenames
/// within that folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDataList {
    pub server: String,
    #[serde(rename = "organization_uid")]
    pub organization_uid: String,
    #[serde(default)]
    pub traces: Vec<String>,
    #[serde(default)]
    pub data: std::collections::HashMap<String, Vec<String>>,
}

/// A job (app run) entry returned by `Client::jobs`.
///
/// Wraps the server's batch-job representation. The `task_id` field links
/// back to the underlying task that can be polled via `Client::task_info`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// App code (e.g. `"edgefirst-validator:2.9.5"`).
    #[serde(default)]
    pub code: String,
    /// Display title from the app definition.
    #[serde(default)]
    pub title: String,
    /// User-supplied job label provided at `job_run` time.
    #[serde(default)]
    pub job_name: String,
    /// Cloud-batch job identifier (e.g. AWS Batch job ID). Opaque string.
    #[serde(default)]
    pub job_id: String,
    /// Cloud-batch state (e.g. `"RUNNING"`, `"SUCCEEDED"`, `"FAILED"`).
    #[serde(default)]
    pub state: String,
    /// Job launch timestamp. Optional in case the server omits it for some states.
    #[serde(default)]
    pub launch: Option<DateTime<Utc>>,
    /// The Studio task id linked to this job. Use with `Client::task_info`.
    ///
    /// The server emits this as Go `int64`; negative values are clamped to 0
    /// when converting to `TaskID` via the `task_id()` accessor.
    pub task_id: i64,
}

impl Job {
    /// Returns the `TaskID` corresponding to this job, for chaining with
    /// `Client::task_info`.
    ///
    /// Saturates at 0 for safety: the server should never emit a negative
    /// task_id, but the Go `int64` type makes it representable.
    pub fn task_id(&self) -> TaskID {
        TaskID::from(self.task_id.max(0) as u64)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TasksListResult {
    pub tasks: Vec<Task>,
    pub continue_token: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Task {
    id: TaskID,
    name: String,
    #[serde(rename = "type")]
    workflow: String,
    status: String,
    #[serde(rename = "manage_type")]
    manager: Option<String>,
    #[serde(rename = "instance_type")]
    instance: String,
    #[serde(rename = "date")]
    created: DateTime<Utc>,
}

impl Task {
    pub fn id(&self) -> TaskID {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn workflow(&self) -> &str {
        &self.workflow
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn manager(&self) -> Option<&str> {
        self.manager.as_deref()
    }

    pub fn instance(&self) -> &str {
        &self.instance
    }

    pub fn created(&self) -> &DateTime<Utc> {
        &self.created
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} [{:?} {}] {}",
            self.id,
            self.manager(),
            self.workflow(),
            self.name()
        )
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TaskInfo {
    id: TaskID,
    project_id: Option<ProjectID>,
    #[serde(rename = "task_description", alias = "description", default)]
    description: String,
    #[serde(rename = "type")]
    workflow: String,
    status: Option<String>,
    #[serde(default)]
    progress: TaskProgress,
    #[serde(
        rename = "created_date",
        alias = "created",
        default = "default_datetime_utc"
    )]
    created: DateTime<Utc>,
    #[serde(
        rename = "end_date",
        alias = "completed",
        default = "default_datetime_utc"
    )]
    completed: DateTime<Utc>,
}

fn default_datetime_utc() -> DateTime<Utc> {
    DateTime::UNIX_EPOCH
}

impl Display for TaskInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}: {}", self.id, self.workflow(), self.description())
    }
}

impl TaskInfo {
    pub fn id(&self) -> TaskID {
        self.id
    }

    pub fn project_id(&self) -> Option<ProjectID> {
        self.project_id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn workflow(&self) -> &str {
        &self.workflow
    }

    pub fn status(&self) -> &Option<String> {
        &self.status
    }

    pub async fn set_status(&mut self, client: &Client, status: &str) -> Result<(), Error> {
        let t = client.task_status(self.id(), status).await?;
        self.status = Some(t.status);
        Ok(())
    }

    pub fn stages(&self) -> HashMap<String, Stage> {
        match &self.progress.stages {
            Some(stages) => stages.clone(),
            None => HashMap::new(),
        }
    }

    pub async fn update_stage(
        &mut self,
        client: &Client,
        stage: &str,
        status: &str,
        message: &str,
        percentage: u8,
    ) -> Result<(), Error> {
        client
            .update_stage(self.id(), stage, status, message, percentage)
            .await?;
        let t = client.task_info(self.id()).await?;
        self.progress.stages = Some(t.progress.stages.unwrap_or_default());
        Ok(())
    }

    pub async fn set_stages(
        &mut self,
        client: &Client,
        stages: &[(&str, &str)],
    ) -> Result<(), Error> {
        client.set_stages(self.id(), stages).await?;
        let t = client.task_info(self.id()).await?;
        self.progress.stages = Some(t.progress.stages.unwrap_or_default());
        Ok(())
    }

    /// Lists the data artefacts (non-chart files) attached to this task.
    ///
    /// The returned `TaskDataList::data` map is keyed by folder name.
    /// Trace files are also surfaced separately in `traces`.
    ///
    /// # Arguments
    /// * `client` - The authenticated client instance.
    ///
    /// # Returns
    /// A `TaskDataList` where `data` maps folder names to lists of filenames.
    ///
    /// # Errors
    /// Returns `Error::TaskNotFound` if the task does not exist,
    /// `Error::PermissionDenied` if authorization fails, or
    /// `Error::RpcError` for other server-side failures.
    pub async fn data_list(&self, client: &client::Client) -> Result<TaskDataList, Error> {
        let req = client::TaskDataListRequest {
            task_id: self.id().value(),
        };
        match client.rpc("task.data.list".to_owned(), Some(&req)).await {
            Ok(r) => Ok(r),
            Err(Error::RpcError(code, msg)) => Err(client::map_rpc_error(
                "task.data.list",
                code,
                msg,
                Some(self.id()),
            )),
            Err(e) => Err(e),
        }
    }

    /// Uploads a data file to this task.
    ///
    /// # Arguments
    /// * `client`   - The authenticated client instance.
    /// * `path`     - Local file path to upload. The filename is derived from
    ///   the path's last component.
    /// * `folder`   - Optional logical subdirectory under the task data root.
    ///   Empty-string is normalised to `None`.
    /// * `progress` - Optional progress channel. Emits `Progress { current,
    ///   total, status: None }` events as bytes are streamed to the server.
    ///   `total` equals the file size in bytes; `current` tracks bytes sent.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns `Error::InvalidParameters` if the path has no valid filename,
    /// `Error::TaskNotFound` if the task does not exist,
    /// `Error::PermissionDenied` if authorization fails, or
    /// `Error::RpcError` for other server-side failures.
    pub async fn upload_data(
        &self,
        client: &client::Client,
        path: &std::path::Path,
        folder: Option<&str>,
        progress: Option<tokio::sync::mpsc::Sender<Progress>>,
    ) -> Result<(), Error> {
        use futures::StreamExt;
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        use tokio_util::io::ReaderStream;

        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::InvalidParameters("path must have a UTF-8 filename".into()))?
            .to_owned();

        let file = tokio::fs::File::open(path).await?;
        let total = file.metadata().await?.len() as usize;
        let sent = Arc::new(AtomicUsize::new(0));

        let reader_stream = ReaderStream::new(file);
        let sent_clone = sent.clone();
        let progress_clone = progress.clone();
        let progress_stream = reader_stream.inspect(move |chunk_result| {
            if let Ok(chunk) = chunk_result {
                let current = sent_clone.fetch_add(chunk.len(), Ordering::Relaxed) + chunk.len();
                if let Some(tx) = &progress_clone {
                    let _ = tx.try_send(Progress {
                        current,
                        total,
                        status: None,
                    });
                }
            }
        });

        let body = reqwest::Body::wrap_stream(progress_stream);
        let file_part = Part::stream_with_length(body, total as u64).file_name(file_name);

        let mut form = Form::new().text("task_id", self.id().value().to_string());
        if let Some(folder) = folder.filter(|s| !s.is_empty()) {
            form = form.text("folder", folder.to_owned());
        }
        form = form.part("file", file_part);

        match client.post_multipart("task.data.upload", form).await {
            Ok(_) => Ok(()),
            Err(Error::RpcError(code, msg)) => Err(client::map_rpc_error(
                "task.data.upload",
                code,
                msg,
                Some(self.id()),
            )),
            Err(e) => Err(e),
        }
    }

    /// Streams a data file from this task to `output_path`.
    ///
    /// `folder` is the logical subdirectory under the task data root;
    /// pass `None` (or `Some("")`) to download from the root.
    ///
    /// Progress is reported via the optional `progress` channel; values
    /// match the server-reported `Content-Length` when available.
    ///
    /// # Arguments
    /// * `client`      - The authenticated client instance.
    /// * `file`        - Filename to download.
    /// * `folder`      - Optional logical subdirectory under the task data root;
    ///   `None` or `Some("")` targets the root.
    /// * `output_path` - Local path to write the downloaded file.
    /// * `progress`    - Optional progress channel; events carry bytes received
    ///   and `Content-Length` total (0 if server omits it).
    ///
    /// # Returns
    /// `Ok(())` when the file has been written and flushed.
    ///
    /// # Errors
    /// Returns `Error::TaskNotFound` if the task does not exist,
    /// `Error::PermissionDenied` if authorization fails,
    /// `Error::InvalidResponse` if the server returns JSON instead of binary, or
    /// `Error::IoError` on file write failures.
    pub async fn download_data(
        &self,
        client: &client::Client,
        file: &str,
        folder: Option<&str>,
        output_path: &std::path::Path,
        progress: Option<tokio::sync::mpsc::Sender<Progress>>,
    ) -> Result<(), Error> {
        let folder = folder.unwrap_or("").to_owned();
        let req = client::TaskDataDownloadRequest {
            task_id: self.id().value(),
            folder,
            file: file.to_owned(),
        };
        match client
            .rpc_download("task.data.download", &req, output_path, progress)
            .await
        {
            Ok(()) => Ok(()),
            Err(Error::RpcError(code, msg)) => Err(client::map_rpc_error(
                "task.data.download",
                code,
                msg,
                Some(self.id()),
            )),
            Err(e) => Err(e),
        }
    }

    /// Adds (or overwrites) a chart under `(group, name)` for this task.
    ///
    /// `data` is the chart body — arbitrary JSON via the `Parameter` enum.
    /// `params` are optional chart-rendering parameters.
    ///
    /// The server's `task.chart.add` is upsert semantics: a chart with the
    /// same `(group, name)` is overwritten.
    ///
    /// Returns `()` — the server does not return a chart id. Charts are
    /// identified by `(group, name)` and the same key overwrites on subsequent
    /// calls.
    ///
    /// # Arguments
    /// * `client` - The authenticated client instance.
    /// * `group`  - Chart group name (non-empty).
    /// * `name`   - Chart name within the group (non-empty).
    /// * `data`   - Chart body as a `Parameter` (arbitrary JSON).
    /// * `params` - Optional chart-rendering parameters as a `Parameter`.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns `Error::InvalidParameters` if `group` or `name` is empty,
    /// `Error::TaskNotFound` if the task does not exist,
    /// `Error::PermissionDenied` if authorization fails, or
    /// `Error::RpcError` for other server-side failures.
    pub async fn add_chart(
        &self,
        client: &client::Client,
        group: &str,
        name: &str,
        data: Parameter,
        params: Option<Parameter>,
    ) -> Result<(), Error> {
        client::validate_chart_args(group, name)?;
        let req = client::TaskChartAddRequest {
            task_id: self.id().value(),
            group_name: group.to_owned(),
            chart_name: name.to_owned(),
            params,
            data,
        };
        let _resp: serde_json::Value =
            match client.rpc("task.chart.add".to_owned(), Some(&req)).await {
                Ok(r) => r,
                Err(Error::RpcError(code, msg)) => {
                    return Err(client::map_rpc_error(
                        "task.chart.add",
                        code,
                        msg,
                        Some(self.id()),
                    ));
                }
                Err(e) => return Err(e),
            };
        Ok(())
    }

    /// Lists charts attached to this task, optionally filtered to a single group.
    ///
    /// Returns the same `TaskDataList` shape as `data_list`, where the `data`
    /// map encodes `group -> [chart_filenames]`.
    ///
    /// # Arguments
    /// * `client` - The authenticated client instance.
    /// * `group`  - Optional group name to filter results; `None` returns all groups.
    ///
    /// # Returns
    /// A `TaskDataList` where `data` maps group names to lists of chart filenames.
    ///
    /// # Errors
    /// Returns `Error::TaskNotFound` if the task does not exist,
    /// `Error::PermissionDenied` if authorization fails, or
    /// `Error::RpcError` for other server-side failures.
    pub async fn list_charts(
        &self,
        client: &client::Client,
        group: Option<&str>,
    ) -> Result<TaskDataList, Error> {
        let req = client::TaskChartListRequest {
            task_id: self.id().value(),
            group_name: group.unwrap_or("").to_owned(),
        };
        match client.rpc("task.chart.list".to_owned(), Some(&req)).await {
            Ok(r) => Ok(r),
            Err(Error::RpcError(code, msg)) => Err(client::map_rpc_error(
                "task.chart.list",
                code,
                msg,
                Some(self.id()),
            )),
            Err(e) => Err(e),
        }
    }

    /// Fetches the raw chart body for `(group, name)` on this task.
    ///
    /// The returned `Parameter` is the deserialized chart JSON; the caller
    /// is responsible for interpreting the shape (line, bar, scatter, etc.).
    ///
    /// # Arguments
    /// * `client` - The authenticated client instance.
    /// * `group`  - Chart group name (non-empty).
    /// * `name`   - Chart name within the group (non-empty).
    ///
    /// # Returns
    /// The chart body deserialized as a `Parameter`.
    ///
    /// # Errors
    /// Returns `Error::InvalidParameters` if `group` or `name` is empty,
    /// `Error::TaskNotFound` if the task does not exist,
    /// `Error::PermissionDenied` if authorization fails, or
    /// `Error::RpcError` for other server-side failures.
    pub async fn get_chart(
        &self,
        client: &client::Client,
        group: &str,
        name: &str,
    ) -> Result<Parameter, Error> {
        client::validate_chart_args(group, name)?;
        let req = client::TaskChartGetRequest {
            task_id: self.id().value(),
            group_name: group.to_owned(),
            chart_name: name.to_owned(),
        };
        match client.rpc("task.chart.get".to_owned(), Some(&req)).await {
            Ok(r) => Ok(r),
            Err(Error::RpcError(code, msg)) => Err(client::map_rpc_error(
                "task.chart.get",
                code,
                msg,
                Some(self.id()),
            )),
            Err(e) => Err(e),
        }
    }

    pub fn created(&self) -> &DateTime<Utc> {
        &self.created
    }

    pub fn completed(&self) -> &DateTime<Utc> {
        &self.completed
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct TaskProgress {
    stages: Option<HashMap<String, Stage>>,
}

#[derive(Serialize, Debug, Clone)]
pub struct TaskStatus {
    #[serde(rename = "docker_task_id")]
    pub task_id: TaskID,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stage {
    #[serde(rename = "docker_task_id", skip_serializing_if = "Option::is_none")]
    task_id: Option<TaskID>,
    stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    percentage: u8,
}

impl Stage {
    pub fn new(
        task_id: Option<TaskID>,
        stage: String,
        status: Option<String>,
        message: Option<String>,
        percentage: u8,
    ) -> Self {
        Stage {
            task_id,
            stage,
            status,
            description: None,
            message,
            percentage,
        }
    }

    pub fn task_id(&self) -> &Option<TaskID> {
        &self.task_id
    }

    pub fn stage(&self) -> &str {
        &self.stage
    }

    pub fn status(&self) -> &Option<String> {
        &self.status
    }

    pub fn description(&self) -> &Option<String> {
        &self.description
    }

    pub fn message(&self) -> &Option<String> {
        &self.message
    }

    pub fn percentage(&self) -> u8 {
        self.percentage
    }
}

#[derive(Serialize, Debug)]
pub struct TaskStages {
    #[serde(rename = "docker_task_id")]
    pub task_id: TaskID,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<HashMap<String, String>>,
}

#[derive(Deserialize, Debug)]
pub struct Artifact {
    name: String,
    #[serde(rename = "modelType")]
    model_type: String,
}

impl Artifact {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn model_type(&self) -> &str {
        &self.model_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== OrganizationID Tests ==========
    #[test]
    fn test_organization_id_from_u64() {
        let id = OrganizationID::from(12345);
        assert_eq!(id.value(), 12345);
    }

    #[test]
    fn test_organization_id_display() {
        let id = OrganizationID::from(0xabc123);
        assert_eq!(format!("{}", id), "org-abc123");
    }

    #[test]
    fn test_organization_id_try_from_str_valid() {
        let id = OrganizationID::try_from("org-abc123").unwrap();
        assert_eq!(id.value(), 0xabc123);
    }

    #[test]
    fn test_organization_id_try_from_str_invalid_prefix() {
        let result = OrganizationID::try_from("invalid-abc123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 'org-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_organization_id_try_from_str_invalid_hex() {
        let result = OrganizationID::try_from("org-xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_organization_id_try_from_str_empty() {
        let result = OrganizationID::try_from("org-");
        assert!(result.is_err());
    }

    #[test]
    fn test_organization_id_into_u64() {
        let id = OrganizationID::from(54321);
        let value: u64 = id.into();
        assert_eq!(value, 54321);
    }

    // ========== ProjectID Tests ==========
    #[test]
    fn test_project_id_from_u64() {
        let id = ProjectID::from(78910);
        assert_eq!(id.value(), 78910);
    }

    #[test]
    fn test_project_id_display() {
        let id = ProjectID::from(0xdef456);
        assert_eq!(format!("{}", id), "p-def456");
    }

    #[test]
    fn test_project_id_from_str_valid() {
        let id = ProjectID::from_str("p-def456").unwrap();
        assert_eq!(id.value(), 0xdef456);
    }

    #[test]
    fn test_project_id_try_from_str_valid() {
        let id = ProjectID::try_from("p-123abc").unwrap();
        assert_eq!(id.value(), 0x123abc);
    }

    #[test]
    fn test_project_id_try_from_string_valid() {
        let id = ProjectID::try_from("p-456def".to_string()).unwrap();
        assert_eq!(id.value(), 0x456def);
    }

    #[test]
    fn test_project_id_from_str_invalid_prefix() {
        let result = ProjectID::from_str("proj-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 'p-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_project_id_from_str_invalid_hex() {
        let result = ProjectID::from_str("p-notahex");
        assert!(result.is_err());
    }

    #[test]
    fn test_project_id_into_u64() {
        let id = ProjectID::from(99999);
        let value: u64 = id.into();
        assert_eq!(value, 99999);
    }

    // ========== ExperimentID Tests ==========
    #[test]
    fn test_experiment_id_from_u64() {
        let id = ExperimentID::from(1193046);
        assert_eq!(id.value(), 1193046);
    }

    #[test]
    fn test_experiment_id_display() {
        let id = ExperimentID::from(0x123abc);
        assert_eq!(format!("{}", id), "exp-123abc");
    }

    #[test]
    fn test_experiment_id_from_str_valid() {
        let id = ExperimentID::from_str("exp-456def").unwrap();
        assert_eq!(id.value(), 0x456def);
    }

    #[test]
    fn test_experiment_id_try_from_str_valid() {
        let id = ExperimentID::try_from("exp-789abc").unwrap();
        assert_eq!(id.value(), 0x789abc);
    }

    #[test]
    fn test_experiment_id_try_from_string_valid() {
        let id = ExperimentID::try_from("exp-fedcba".to_string()).unwrap();
        assert_eq!(id.value(), 0xfedcba);
    }

    #[test]
    fn test_experiment_id_from_str_invalid_prefix() {
        let result = ExperimentID::from_str("experiment-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 'exp-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_experiment_id_from_str_invalid_hex() {
        let result = ExperimentID::from_str("exp-zzz");
        assert!(result.is_err());
    }

    #[test]
    fn test_experiment_id_into_u64() {
        let id = ExperimentID::from(777777);
        let value: u64 = id.into();
        assert_eq!(value, 777777);
    }

    // ========== TrainingSessionID Tests ==========
    #[test]
    fn test_training_session_id_from_u64() {
        let id = TrainingSessionID::from(7901234);
        assert_eq!(id.value(), 7901234);
    }

    #[test]
    fn test_training_session_id_display() {
        let id = TrainingSessionID::from(0xabc123);
        assert_eq!(format!("{}", id), "t-abc123");
    }

    #[test]
    fn test_training_session_id_from_str_valid() {
        let id = TrainingSessionID::from_str("t-abc123").unwrap();
        assert_eq!(id.value(), 0xabc123);
    }

    #[test]
    fn test_training_session_id_try_from_str_valid() {
        let id = TrainingSessionID::try_from("t-deadbeef").unwrap();
        assert_eq!(id.value(), 0xdeadbeef);
    }

    #[test]
    fn test_training_session_id_try_from_string_valid() {
        let id = TrainingSessionID::try_from("t-cafebabe".to_string()).unwrap();
        assert_eq!(id.value(), 0xcafebabe);
    }

    #[test]
    fn test_training_session_id_from_str_invalid_prefix() {
        let result = TrainingSessionID::from_str("training-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 't-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_training_session_id_from_str_invalid_hex() {
        let result = TrainingSessionID::from_str("t-qqq");
        assert!(result.is_err());
    }

    #[test]
    fn test_training_session_id_into_u64() {
        let id = TrainingSessionID::from(123456);
        let value: u64 = id.into();
        assert_eq!(value, 123456);
    }

    // ========== ValidationSessionID Tests ==========
    #[test]
    fn test_validation_session_id_from_u64() {
        let id = ValidationSessionID::from(3456789);
        assert_eq!(id.value(), 3456789);
    }

    #[test]
    fn test_validation_session_id_display() {
        let id = ValidationSessionID::from(0x34c985);
        assert_eq!(format!("{}", id), "v-34c985");
    }

    #[test]
    fn test_validation_session_id_try_from_str_valid() {
        let id = ValidationSessionID::try_from("v-deadbeef").unwrap();
        assert_eq!(id.value(), 0xdeadbeef);
    }

    #[test]
    fn test_validation_session_id_try_from_string_valid() {
        let id = ValidationSessionID::try_from("v-12345678".to_string()).unwrap();
        assert_eq!(id.value(), 0x12345678);
    }

    #[test]
    fn test_validation_session_id_try_from_str_invalid_prefix() {
        let result = ValidationSessionID::try_from("validation-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 'v-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_validation_session_id_try_from_str_invalid_hex() {
        let result = ValidationSessionID::try_from("v-xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_session_id_into_u64() {
        let id = ValidationSessionID::from(987654);
        let value: u64 = id.into();
        assert_eq!(value, 987654);
    }

    // ========== SnapshotID Tests ==========
    #[test]
    fn test_snapshot_id_from_u64() {
        let id = SnapshotID::from(111222);
        assert_eq!(id.value(), 111222);
    }

    #[test]
    fn test_snapshot_id_display() {
        let id = SnapshotID::from(0xaabbcc);
        assert_eq!(format!("{}", id), "ss-aabbcc");
    }

    #[test]
    fn test_snapshot_id_try_from_str_valid() {
        let id = SnapshotID::try_from("ss-aabbcc").unwrap();
        assert_eq!(id.value(), 0xaabbcc);
    }

    #[test]
    fn test_snapshot_id_try_from_str_invalid_prefix() {
        let result = SnapshotID::try_from("snapshot-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 'ss-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_snapshot_id_try_from_str_invalid_hex() {
        let result = SnapshotID::try_from("ss-ggg");
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_id_into_u64() {
        let id = SnapshotID::from(333444);
        let value: u64 = id.into();
        assert_eq!(value, 333444);
    }

    // ========== TaskID Tests ==========
    #[test]
    fn test_task_id_from_u64() {
        let id = TaskID::from(555666);
        assert_eq!(id.value(), 555666);
    }

    #[test]
    fn test_task_id_display() {
        let id = TaskID::from(0x123456);
        assert_eq!(format!("{}", id), "task-123456");
    }

    #[test]
    fn test_task_id_from_str_valid() {
        let id = TaskID::from_str("task-123456").unwrap();
        assert_eq!(id.value(), 0x123456);
    }

    #[test]
    fn test_task_id_try_from_str_valid() {
        let id = TaskID::try_from("task-abcdef").unwrap();
        assert_eq!(id.value(), 0xabcdef);
    }

    #[test]
    fn test_task_id_try_from_string_valid() {
        let id = TaskID::try_from("task-fedcba".to_string()).unwrap();
        assert_eq!(id.value(), 0xfedcba);
    }

    #[test]
    fn test_task_id_from_str_invalid_prefix() {
        let result = TaskID::from_str("t-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 'task-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_task_id_from_str_invalid_hex() {
        let result = TaskID::from_str("task-zzz");
        assert!(result.is_err());
    }

    #[test]
    fn test_task_id_into_u64() {
        let id = TaskID::from(777888);
        let value: u64 = id.into();
        assert_eq!(value, 777888);
    }

    // ========== DatasetID Tests ==========
    #[test]
    fn test_dataset_id_from_u64() {
        let id = DatasetID::from(1193046);
        assert_eq!(id.value(), 1193046);
    }

    #[test]
    fn test_dataset_id_display() {
        let id = DatasetID::from(0x123abc);
        assert_eq!(format!("{}", id), "ds-123abc");
    }

    #[test]
    fn test_dataset_id_from_str_valid() {
        let id = DatasetID::from_str("ds-456def").unwrap();
        assert_eq!(id.value(), 0x456def);
    }

    #[test]
    fn test_dataset_id_try_from_str_valid() {
        let id = DatasetID::try_from("ds-789abc").unwrap();
        assert_eq!(id.value(), 0x789abc);
    }

    #[test]
    fn test_dataset_id_try_from_string_valid() {
        let id = DatasetID::try_from("ds-fedcba".to_string()).unwrap();
        assert_eq!(id.value(), 0xfedcba);
    }

    #[test]
    fn test_dataset_id_from_str_invalid_prefix() {
        let result = DatasetID::from_str("dataset-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 'ds-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_dataset_id_from_str_invalid_hex() {
        let result = DatasetID::from_str("ds-zzz");
        assert!(result.is_err());
    }

    #[test]
    fn test_dataset_id_into_u64() {
        let id = DatasetID::from(111111);
        let value: u64 = id.into();
        assert_eq!(value, 111111);
    }

    // ========== AnnotationSetID Tests ==========
    #[test]
    fn test_annotation_set_id_from_u64() {
        let id = AnnotationSetID::from(222333);
        assert_eq!(id.value(), 222333);
    }

    #[test]
    fn test_annotation_set_id_display() {
        let id = AnnotationSetID::from(0xabcdef);
        assert_eq!(format!("{}", id), "as-abcdef");
    }

    #[test]
    fn test_annotation_set_id_from_str_valid() {
        let id = AnnotationSetID::from_str("as-abcdef").unwrap();
        assert_eq!(id.value(), 0xabcdef);
    }

    #[test]
    fn test_annotation_set_id_try_from_str_valid() {
        let id = AnnotationSetID::try_from("as-123456").unwrap();
        assert_eq!(id.value(), 0x123456);
    }

    #[test]
    fn test_annotation_set_id_try_from_string_valid() {
        let id = AnnotationSetID::try_from("as-fedcba".to_string()).unwrap();
        assert_eq!(id.value(), 0xfedcba);
    }

    #[test]
    fn test_annotation_set_id_from_str_invalid_prefix() {
        let result = AnnotationSetID::from_str("annotation-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 'as-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_annotation_set_id_from_str_invalid_hex() {
        let result = AnnotationSetID::from_str("as-zzz");
        assert!(result.is_err());
    }

    #[test]
    fn test_annotation_set_id_into_u64() {
        let id = AnnotationSetID::from(444555);
        let value: u64 = id.into();
        assert_eq!(value, 444555);
    }

    // ========== SampleID Tests ==========
    #[test]
    fn test_sample_id_from_u64() {
        let id = SampleID::from(666777);
        assert_eq!(id.value(), 666777);
    }

    #[test]
    fn test_sample_id_display() {
        let id = SampleID::from(0x987654);
        assert_eq!(format!("{}", id), "s-987654");
    }

    #[test]
    fn test_sample_id_try_from_str_valid() {
        let id = SampleID::try_from("s-987654").unwrap();
        assert_eq!(id.value(), 0x987654);
    }

    #[test]
    fn test_sample_id_try_from_str_invalid_prefix() {
        let result = SampleID::try_from("sample-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 's-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_sample_id_try_from_str_invalid_hex() {
        let result = SampleID::try_from("s-zzz");
        assert!(result.is_err());
    }

    #[test]
    fn test_sample_id_into_u64() {
        let id = SampleID::from(888999);
        let value: u64 = id.into();
        assert_eq!(value, 888999);
    }

    // ========== AppId Tests ==========
    #[test]
    fn test_app_id_from_u64() {
        let id = AppId::from(123123);
        assert_eq!(id.value(), 123123);
    }

    #[test]
    fn test_app_id_display() {
        let id = AppId::from(0x456789);
        assert_eq!(format!("{}", id), "app-456789");
    }

    #[test]
    fn test_app_id_try_from_str_valid() {
        let id = AppId::try_from("app-456789").unwrap();
        assert_eq!(id.value(), 0x456789);
    }

    #[test]
    fn test_app_id_try_from_str_invalid_prefix() {
        let result = AppId::try_from("application-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 'app-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_app_id_try_from_str_invalid_hex() {
        let result = AppId::try_from("app-zzz");
        assert!(result.is_err());
    }

    #[test]
    fn test_app_id_into_u64() {
        let id = AppId::from(321321);
        let value: u64 = id.into();
        assert_eq!(value, 321321);
    }

    // ========== ImageId Tests ==========
    #[test]
    fn test_image_id_from_u64() {
        let id = ImageId::from(789789);
        assert_eq!(id.value(), 789789);
    }

    #[test]
    fn test_image_id_display() {
        let id = ImageId::from(0xabcd1234);
        assert_eq!(format!("{}", id), "im-abcd1234");
    }

    #[test]
    fn test_image_id_try_from_str_valid() {
        let id = ImageId::try_from("im-abcd1234").unwrap();
        assert_eq!(id.value(), 0xabcd1234);
    }

    #[test]
    fn test_image_id_try_from_str_invalid_prefix() {
        let result = ImageId::try_from("image-123");
        assert!(result.is_err());
        match result {
            Err(Error::InvalidParameters(msg)) => {
                assert!(msg.contains("must start with 'im-'"));
            }
            _ => panic!("Expected InvalidParameters error"),
        }
    }

    #[test]
    fn test_image_id_try_from_str_invalid_hex() {
        let result = ImageId::try_from("im-zzz");
        assert!(result.is_err());
    }

    #[test]
    fn test_image_id_into_u64() {
        let id = ImageId::from(987987);
        let value: u64 = id.into();
        assert_eq!(value, 987987);
    }

    // ========== ID Type Hash and Equality Tests ==========
    #[test]
    fn test_id_types_equality() {
        let id1 = ProjectID::from(12345);
        let id2 = ProjectID::from(12345);
        let id3 = ProjectID::from(54321);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_id_types_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(DatasetID::from(100));
        set.insert(DatasetID::from(200));
        set.insert(DatasetID::from(100)); // duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&DatasetID::from(100)));
        assert!(set.contains(&DatasetID::from(200)));
    }

    #[test]
    fn test_id_types_copy_clone() {
        let id1 = ExperimentID::from(999);
        let id2 = id1; // Copy
        let id3 = id1; // Also Copy (no need for clone())

        assert_eq!(id1, id2);
        assert_eq!(id1, id3);
    }

    // ========== Edge Cases ==========
    #[test]
    fn test_id_zero_value() {
        let id = ProjectID::from(0);
        assert_eq!(format!("{}", id), "p-0");
        assert_eq!(id.value(), 0);
    }

    #[test]
    fn test_id_max_value() {
        let id = ProjectID::from(u64::MAX);
        assert_eq!(format!("{}", id), "p-ffffffffffffffff");
        assert_eq!(id.value(), u64::MAX);
    }

    #[test]
    fn test_id_round_trip_conversion() {
        let original = 0xdeadbeef_u64;
        let id = TrainingSessionID::from(original);
        let back: u64 = id.into();
        assert_eq!(original, back);
    }

    #[test]
    fn test_id_case_insensitive_hex() {
        // Hexadecimal parsing should handle both upper and lowercase
        let id1 = DatasetID::from_str("ds-ABCDEF").unwrap();
        let id2 = DatasetID::from_str("ds-abcdef").unwrap();
        assert_eq!(id1.value(), id2.value());
    }

    #[test]
    fn test_id_with_leading_zeros() {
        let id = ProjectID::from_str("p-00001234").unwrap();
        assert_eq!(id.value(), 0x1234);
    }

    // ========== Parameter Tests ==========
    #[test]
    fn test_parameter_integer() {
        let param = Parameter::Integer(42);
        match param {
            Parameter::Integer(val) => assert_eq!(val, 42),
            _ => panic!("Expected Integer variant"),
        }
    }

    #[test]
    fn test_parameter_real() {
        let param = Parameter::Real(2.5);
        match param {
            Parameter::Real(val) => assert_eq!(val, 2.5),
            _ => panic!("Expected Real variant"),
        }
    }

    #[test]
    fn test_parameter_boolean() {
        let param = Parameter::Boolean(true);
        match param {
            Parameter::Boolean(val) => assert!(val),
            _ => panic!("Expected Boolean variant"),
        }
    }

    #[test]
    fn test_parameter_string() {
        let param = Parameter::String("test".to_string());
        match param {
            Parameter::String(val) => assert_eq!(val, "test"),
            _ => panic!("Expected String variant"),
        }
    }

    #[test]
    fn test_parameter_array() {
        let param = Parameter::Array(vec![
            Parameter::Integer(1),
            Parameter::Integer(2),
            Parameter::Integer(3),
        ]);
        match param {
            Parameter::Array(arr) => assert_eq!(arr.len(), 3),
            _ => panic!("Expected Array variant"),
        }
    }

    #[test]
    fn test_parameter_object() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), Parameter::Integer(100));
        let param = Parameter::Object(map);
        match param {
            Parameter::Object(obj) => {
                assert_eq!(obj.len(), 1);
                assert!(obj.contains_key("key"));
            }
            _ => panic!("Expected Object variant"),
        }
    }

    #[test]
    fn test_parameter_clone() {
        let param1 = Parameter::Integer(42);
        let param2 = param1.clone();
        assert_eq!(param1, param2);
    }

    #[test]
    fn test_parameter_nested() {
        let inner_array = Parameter::Array(vec![Parameter::Integer(1), Parameter::Integer(2)]);
        let outer_array = Parameter::Array(vec![inner_array.clone(), inner_array]);

        match outer_array {
            Parameter::Array(arr) => {
                assert_eq!(arr.len(), 2);
            }
            _ => panic!("Expected Array variant"),
        }
    }

    // ========== Comprehensive TypeID Conversion Tests (macro-driven) ==========

    macro_rules! test_typeid_conversions {
        ($test_name:ident, $type:ty, $prefix:literal, $wrong_prefix:literal) => {
            #[test]
            fn $test_name() {
                // 1. From<u64> round-trip
                let id = <$type>::from(0xabc123);
                assert_eq!(id.value(), 0xabc123);

                // 2. Display format
                assert_eq!(format!("{}", id), concat!($prefix, "-abc123"));

                // 3. FromStr valid
                let id: $type = concat!($prefix, "-abc123").parse().unwrap();
                assert_eq!(id.value(), 0xabc123);

                // 4. FromStr wrong prefix
                assert!(concat!($wrong_prefix, "-abc").parse::<$type>().is_err());

                // 5. FromStr missing prefix
                assert!("abc123".parse::<$type>().is_err());

                // 6. FromStr invalid hex
                assert!(concat!($prefix, "-xyz").parse::<$type>().is_err());

                // 7. TryFrom<&str>
                let id = <$type>::try_from(concat!($prefix, "-abc123")).unwrap();
                assert_eq!(id.value(), 0xabc123);

                // 8. TryFrom<String>
                let id = <$type>::try_from(concat!($prefix, "-abc123").to_string()).unwrap();
                assert_eq!(id.value(), 0xabc123);

                // 9. Serde round-trip
                let id = <$type>::from(0xabc123);
                let json = serde_json::to_string(&id).unwrap();
                let parsed: $type = serde_json::from_str(&json).unwrap();
                assert_eq!(id, parsed);

                // 10. From<T> for u64
                let id = <$type>::from(0xabc123);
                let val: u64 = id.into();
                assert_eq!(val, 0xabc123);
            }
        };
    }

    test_typeid_conversions!(test_organization_id_conversions, OrganizationID, "org", "p");
    test_typeid_conversions!(test_project_id_conversions, ProjectID, "p", "org");
    test_typeid_conversions!(test_experiment_id_conversions, ExperimentID, "exp", "p");
    test_typeid_conversions!(
        test_training_session_id_conversions,
        TrainingSessionID,
        "t",
        "v"
    );
    test_typeid_conversions!(
        test_validation_session_id_conversions,
        ValidationSessionID,
        "v",
        "t"
    );
    test_typeid_conversions!(test_snapshot_id_conversions, SnapshotID, "ss", "ds");
    test_typeid_conversions!(test_task_id_conversions, TaskID, "task", "t");
    test_typeid_conversions!(test_dataset_id_conversions, DatasetID, "ds", "ss");
    test_typeid_conversions!(
        test_annotation_set_id_conversions,
        AnnotationSetID,
        "as",
        "ds"
    );
    test_typeid_conversions!(test_sample_id_conversions, SampleID, "s", "p");
    test_typeid_conversions!(test_app_id_conversions, AppId, "app", "p");
    test_typeid_conversions!(test_image_id_conversions, ImageId, "im", "se");
    test_typeid_conversions!(test_sequence_id_conversions, SequenceId, "se", "im");
}

#[cfg(test)]
mod tests_task_data_list {
    use super::*;

    #[test]
    fn task_data_list_deserializes_from_server_shape() {
        let json = r#"{
            "server": "test.edgefirst.studio",
            "organization_uid": "org-abc123",
            "traces": ["trace/imx95.json"],
            "data": {
                "predictions": ["predictions.parquet"],
                "trace": ["imx95.json"]
            }
        }"#;
        let parsed: TaskDataList = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.server, "test.edgefirst.studio");
        assert_eq!(parsed.organization_uid, "org-abc123");
        assert_eq!(parsed.traces, vec!["trace/imx95.json"]);
        assert_eq!(
            parsed.data.get("predictions").unwrap(),
            &vec!["predictions.parquet".to_string()]
        );
    }
}

#[cfg(test)]
mod tests_upload_data {
    // Documents the empty-folder collapse rule used by upload_data:
    // folder=Some("") must behave as None to avoid sending an empty form
    // field that the server might interpret incorrectly.
    #[test]
    fn folder_empty_string_is_normalised() {
        let folder: Option<&str> = Some("");
        assert!(folder.filter(|s| !s.is_empty()).is_none());

        let folder_real: Option<&str> = Some("predictions");
        assert!(folder_real.filter(|s| !s.is_empty()).is_some());
    }
}

#[cfg(test)]
mod tests_job_struct {
    use super::*;

    #[test]
    fn job_deserializes_with_all_fields() {
        let json = r#"{
            "code": "edgefirst-validator:2.9.5",
            "title": "EdgeFirst Validator",
            "job_name": "smoke-test",
            "job_id": "aws-batch-abc",
            "state": "RUNNING",
            "launch": "2026-05-14T15:00:00Z",
            "task_id": 6789
        }"#;
        let job: Job = serde_json::from_str(json).unwrap();
        assert_eq!(job.code, "edgefirst-validator:2.9.5");
        assert_eq!(job.title, "EdgeFirst Validator");
        assert_eq!(job.job_name, "smoke-test");
        assert_eq!(job.job_id, "aws-batch-abc");
        assert_eq!(job.state, "RUNNING");
        assert!(job.launch.is_some());
        assert_eq!(job.task_id, 6789);
    }

    #[test]
    fn job_tolerates_missing_optional_fields() {
        // The server occasionally omits everything except task_id (e.g. for
        // jobs that never reached the batch system). #[serde(default)] should
        // fill in empty strings / None.
        let json = r#"{ "task_id": 42 }"#;
        let job: Job = serde_json::from_str(json).unwrap();
        assert_eq!(job.task_id, 42);
        assert!(job.code.is_empty());
        assert!(job.title.is_empty());
        assert!(job.job_name.is_empty());
        assert!(job.job_id.is_empty());
        assert!(job.state.is_empty());
        assert!(job.launch.is_none());
    }

    #[test]
    fn job_task_id_accessor_saturates_negative_to_zero() {
        // Go emits int64; negative values are nonsense but the wire type
        // makes them representable. The accessor must clamp at 0 rather
        // than wrapping into a huge u64 (which would point at a different
        // task).
        let job = Job {
            code: String::new(),
            title: String::new(),
            job_name: String::new(),
            job_id: String::new(),
            state: String::new(),
            launch: None,
            task_id: -1,
        };
        assert_eq!(job.task_id().value(), 0);
    }

    #[test]
    fn job_task_id_accessor_passes_through_positive_values() {
        let job = Job {
            code: String::new(),
            title: String::new(),
            job_name: String::new(),
            job_id: String::new(),
            state: String::new(),
            launch: None,
            task_id: 12345,
        };
        assert_eq!(job.task_id().value(), 12345);
    }

    #[test]
    fn job_ignores_unknown_fields() {
        // The server BK_BATCH wrapper carries a number of fields we don't
        // care about (docker_task, aws_region, etc.). Deserialization must
        // not break when these are present.
        let json = r#"{
            "code": "x",
            "task_id": 1,
            "docker_task": { "image": "x" },
            "aws_region": "us-east-1",
            "tags": ["a", "b"]
        }"#;
        let job: Job = serde_json::from_str(json).unwrap();
        assert_eq!(job.task_id, 1);
    }
}

#[cfg(test)]
mod tests_task_info_schema_tolerance {
    use super::*;

    // TaskID derives a transparent numeric Serialize/Deserialize on the wire
    // (the hex prefix is the Display form, not the JSON form), so the test
    // fixtures encode `id` as a number.

    #[test]
    fn task_info_accepts_task_description_field() {
        // New server: emits `task_description`.
        let json = r#"{
            "id": 6699,
            "type": "edgefirst-validator:2.9.5",
            "task_description": "Profiler run for IMX95",
            "status": "running"
        }"#;
        let info: TaskInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.description(), "Profiler run for IMX95");
    }

    #[test]
    fn task_info_accepts_legacy_description_field() {
        // Older server / fixtures: emit `description` (aliased).
        let json = r#"{
            "id": 6699,
            "type": "edgefirst-validator:2.9.5",
            "description": "Legacy description"
        }"#;
        let info: TaskInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.description(), "Legacy description");
    }

    #[test]
    fn task_info_tolerates_missing_description() {
        // Neither field present → empty string (default).
        let json = r#"{
            "id": 6699,
            "type": "x"
        }"#;
        let info: TaskInfo = serde_json::from_str(json).unwrap();
        assert!(info.description().is_empty());
    }

    #[test]
    fn task_info_tolerates_missing_dates_via_default() {
        // Server may omit `created_date` / `end_date` for early-stage tasks.
        let json = r#"{
            "id": 6699,
            "type": "x"
        }"#;
        let info: TaskInfo = serde_json::from_str(json).unwrap();
        // Defaults to UNIX_EPOCH per `default_datetime_utc()`.
        assert_eq!(info.id().value(), 6699);
    }

    #[test]
    fn task_info_status_accessor_returns_option() {
        let json = r#"{
            "id": 1,
            "type": "x"
        }"#;
        let info: TaskInfo = serde_json::from_str(json).unwrap();
        assert!(info.status().is_none());
    }

    #[test]
    fn task_info_stages_returns_empty_map_when_unset() {
        let json = r#"{
            "id": 1,
            "type": "x"
        }"#;
        let info: TaskInfo = serde_json::from_str(json).unwrap();
        let stages = info.stages();
        assert!(stages.is_empty());
    }
}

#[cfg(test)]
mod tests_stage_struct {
    use super::*;

    #[test]
    fn stage_new_sets_only_supplied_fields() {
        let stage = Stage::new(
            None,
            "download".into(),
            Some("running".into()),
            Some("fetching".into()),
            42,
        );
        assert!(stage.task_id().is_none());
        assert_eq!(stage.stage(), "download");
        assert_eq!(stage.status().as_deref(), Some("running"));
        assert_eq!(stage.message().as_deref(), Some("fetching"));
        assert_eq!(stage.percentage(), 42);
        // `new` does not populate `description`.
        assert!(stage.description().is_none());
    }

    #[test]
    fn stage_serializes_without_optional_none_fields() {
        // skip_serializing_if=Option::is_none must omit None status/message.
        let stage = Stage::new(None, "init".into(), None, None, 0);
        let json = serde_json::to_value(&stage).unwrap();
        assert!(json.get("status").is_none(), "got: {json}");
        assert!(json.get("message").is_none(), "got: {json}");
        assert!(json.get("docker_task_id").is_none(), "got: {json}");
        // Required field is present.
        assert_eq!(json["stage"], "init");
        assert_eq!(json["percentage"], 0);
    }

    #[test]
    fn stage_serializes_task_id_when_present() {
        let task_id = TaskID::from(0xdeadu64);
        let stage = Stage::new(Some(task_id), "x".into(), None, None, 0);
        let json = serde_json::to_value(&stage).unwrap();
        // Stage carries the task_id under the `docker_task_id` legacy key on
        // the wire.
        assert!(json.get("docker_task_id").is_some());
    }

    #[test]
    fn stage_round_trips_through_json() {
        let stage = Stage::new(
            None,
            "train".into(),
            Some("done".into()),
            Some("epoch 100".into()),
            100,
        );
        let s = serde_json::to_string(&stage).unwrap();
        let back: Stage = serde_json::from_str(&s).unwrap();
        assert_eq!(back.stage(), "train");
        assert_eq!(back.status().as_deref(), Some("done"));
        assert_eq!(back.message().as_deref(), Some("epoch 100"));
        assert_eq!(back.percentage(), 100);
    }
}

#[cfg(test)]
mod tests_task_data_list_extra {
    use super::*;

    #[test]
    fn task_data_list_with_empty_data_map() {
        let json = r#"{
            "server": "studio",
            "organization_uid": "org-1",
            "traces": [],
            "data": {}
        }"#;
        let parsed: TaskDataList = serde_json::from_str(json).unwrap();
        assert!(parsed.traces.is_empty());
        assert!(parsed.data.is_empty());
    }

    #[test]
    fn task_data_list_multiple_folders() {
        let json = r#"{
            "server": "studio",
            "organization_uid": "org-1",
            "traces": ["t1", "t2"],
            "data": {
                "predictions": ["a.parquet", "b.parquet"],
                "metrics": ["loss.json"]
            }
        }"#;
        let parsed: TaskDataList = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.traces.len(), 2);
        assert_eq!(parsed.data.len(), 2);
        assert_eq!(parsed.data["predictions"].len(), 2);
    }
}

#[cfg(test)]
mod tests_artifact_struct {
    use super::*;

    #[test]
    fn artifact_accessors_return_strs() {
        // Artifact uses serde(rename) for modelType → model_type. Make sure
        // the JSON shape coming off the wire round-trips through accessors.
        let json = r#"{ "name": "best.onnx", "modelType": "yolo" }"#;
        let a: Artifact = serde_json::from_str(json).unwrap();
        assert_eq!(a.name(), "best.onnx");
        assert_eq!(a.model_type(), "yolo");
    }
}

#[cfg(test)]
mod tests_task_status_serialize {
    use super::*;

    #[test]
    fn task_status_uses_docker_task_id_wire_field() {
        let s = TaskStatus {
            task_id: TaskID::from(0x1a2bu64),
            status: "training".into(),
        };
        let json = serde_json::to_value(&s).unwrap();
        // Server takes legacy field name.
        assert!(json.get("docker_task_id").is_some(), "got: {json}");
        assert_eq!(json["status"], "training");
    }
}

#[cfg(test)]
mod tests_task_stages_serialize {
    use super::*;

    #[test]
    fn task_stages_omits_empty_vec() {
        let stages = TaskStages {
            task_id: TaskID::from(1u64),
            stages: Vec::new(),
        };
        let json = serde_json::to_value(&stages).unwrap();
        // `skip_serializing_if = "Vec::is_empty"` means the field is absent.
        assert!(json.get("stages").is_none(), "got: {json}");
    }

    #[test]
    fn task_stages_serializes_non_empty_vec() {
        let stages = TaskStages {
            task_id: TaskID::from(1u64),
            stages: vec![std::collections::HashMap::from([(
                "stage".to_string(),
                "download".to_string(),
            )])],
        };
        let json = serde_json::to_value(&stages).unwrap();
        assert_eq!(json["stages"][0]["stage"], "download");
    }
}
