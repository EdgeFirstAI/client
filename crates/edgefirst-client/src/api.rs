// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use crate::{AnnotationSet, Client, Dataset, Error, Progress, Sample, client};
use chrono::{DateTime, Utc};
use log::trace;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::HashMap, fmt::Display, path::PathBuf, str::FromStr};

/// Deserializes a field that may be `null` in JSON as the type's `Default` value.
/// Unlike `#[serde(default)]` alone (which only handles absent keys), this also
/// handles explicit `null` values — common with Go's `omitempty` on slice/array fields
/// where the server may send `null` instead of `[]`.
fn deserialize_null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

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

/// Billing usage summary for the authenticated user's organization.
///
/// `org.get` only returns `latest_credit`; the spendable balance lives in the
/// `accounting.get_usage_summary` RPC. `credits` are promotional/plan credits,
/// `funds` are paid balance, and `total` is what is actually available to spend.
#[derive(Deserialize, Clone, Debug)]
pub struct UsageSummary {
    #[serde(default)]
    credits: f64,
    #[serde(default)]
    funds: f64,
    #[serde(default, rename = "total_funds_and_credits")]
    total: f64,
}

impl UsageSummary {
    pub fn credits(&self) -> f64 {
        self.credits
    }

    pub fn funds(&self) -> f64 {
        self.funds
    }

    pub fn total(&self) -> f64 {
        self.total
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct SamplesListResult {
    pub samples: Vec<Sample>,
    pub continue_token: Option<String>,
}

/// A single sample dimension update entry.
#[derive(Serialize, Clone, Debug)]
pub struct SampleDimensionUpdate {
    pub id: SampleID,
    pub width: u32,
    pub height: u32,
}

/// Parameters for the `samples.update_dimensions` API call.
#[derive(Serialize, Clone, Debug)]
pub struct SamplesUpdateDimensionsParams {
    pub dataset_id: DatasetID,
    pub samples: Vec<SampleDimensionUpdate>,
}

/// Result from the `samples.update_dimensions` API call.
#[derive(Deserialize, Debug)]
pub struct SamplesUpdateDimensionsResult {
    pub updated: u64,
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
    // The snapshots.restore RPC response does not include a `date` field
    // (see dve-database api/snapshots.go SnapshotAPIReturn), so accept its
    // absence rather than failing deserialization.
    #[serde(default)]
    pub date: Option<DateTime<Utc>>,
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
                    // Intermediate progress is sampled with try_send so a slow
                    // consumer never blocks the upload pipeline; the
                    // guaranteed completion event is emitted after the
                    // multipart POST returns below.
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

        let result = match client.post_multipart("val.data.upload", form).await {
            Ok(_) => Ok(()),
            Err(Error::RpcError(code, msg)) => {
                Err(client::map_rpc_error("val.data.upload", code, msg, None))
            }
            Err(e) => Err(e),
        };

        // Guarantee a terminal `current == total` event reaches the consumer
        // so completion handlers (Python callbacks, UniFFI progress bridges)
        // always observe the finished state. Use `send().await` rather than
        // `try_send` here so the event is never dropped.
        if result.is_ok()
            && let Some(tx) = progress
        {
            let _ = tx
                .send(Progress {
                    current: total,
                    total,
                    status: None,
                })
                .await;
        }
        result
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
    /// `Error::RpcError` if the server returns a JSON-RPC error envelope
    /// (decoded from the `Content-Type: application/json` body), or
    /// `Error::IoError` on file write failures. Legitimate JSON file
    /// payloads (e.g. trace JSON) are persisted normally rather than
    /// treated as an error.
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

/// Inputs for [`client::Client::start_validation_session`].
///
/// The required fields mirror what Studio's `cloud.server.start` endpoint
/// needs to create a validation session against a known training session
/// (training_session_id, model_file, val_type) and a known target
/// (dataset_id + annotation_set_id, *or* a snapshot_id).
///
/// `is_local: true` marks the resulting session as **user-managed** on
/// the server: the row is created in the database and data uploads /
/// downloads / metric updates all work normally, but no EC2 instance is
/// provisioned and no automated validator pipeline is started. That is
/// the mode our integration tests want — we get a real session to
/// exercise the upload/list/download wrappers against, and we are
/// responsible for tearing it down with
/// [`client::Client::delete_validation_sessions`] when done.
///
/// `is_kubernetes: true` analogously routes the session to a Kubernetes
/// manage type. Leave both flags `false` for the default AWS_EC2 path.
#[derive(Debug, Clone)]
pub struct StartValidationRequest {
    pub project_id: ProjectID,
    pub name: String,
    pub training_session_id: TrainingSessionID,
    pub model_file: String,
    pub val_type: String,
    pub params: HashMap<String, Parameter>,
    pub is_local: bool,
    pub is_kubernetes: bool,
    pub description: Option<String>,
    pub dataset_id: Option<DatasetID>,
    pub annotation_set_id: Option<AnnotationSetID>,
    pub snapshot_id: Option<SnapshotID>,
}

/// Result of [`client::Client::start_validation_session`].
///
/// Studio's `cloud.server.start` returns the freshly-created
/// `BackgroundTask` row. The interesting fields for downstream code are
/// the task id (which `task_info` / `tasks` / `job_stop` accept) and the
/// embedded validation-session id (the handle to the new session, the
/// thing you pass to `delete_validation_sessions` and to
/// `validation_session`).
///
/// `session_id` is `Option` because the same endpoint also returns
/// non-validation tasks (trainer, dataset import, …) and those don't
/// populate `val_session_id`. For our test fixture path the field is
/// always `Some(_)`; callers can `unwrap()` if they passed
/// `type = "validation"` semantics in the request.
#[derive(Deserialize, Debug, Clone)]
pub struct NewValidationSession {
    #[serde(rename = "id")]
    pub task_id: TaskID,
    #[serde(rename = "val_session_id", default)]
    pub session_id: Option<ValidationSessionID>,
}

impl Display for NewValidationSession {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.session_id {
            Some(id) => write!(f, "task {} session {}", self.task_id, id),
            None => write!(f, "task {} (no session)", self.task_id),
        }
    }
}

/// Request payload for [`client::Client::start_training_session`].
///
/// Launches a new training session against a single dataset using
/// group-based train/validation splits. When `train_group` / `val_group`
/// are `None`, the dataset's default split groups (`"train"` / `"val"`)
/// are used. When `tag_name` is `None`, the dataset's most recent tag is
/// used.
///
/// The hyperparameters in `params` are trainer-specific; query the
/// trainer's parameter schema with `Client::trainer_schema` (using a
/// `schema_type` from `Client::trainer_schemas`) to discover the
/// accepted parameter names, defaults, and ranges.
///
/// Set `is_local: true` for a **user-managed** session: the session row
/// is created and fully usable for artifact/metric uploads, but no cloud
/// instance is provisioned — the caller runs the training loop
/// themselves. `is_kubernetes: true` schedules onto the organization's
/// Kubernetes runner; with both flags false the server provisions a
/// cloud (AWS EC2) instance.
#[derive(Debug, Clone)]
pub struct StartTrainingRequest {
    /// Project owning the experiment and dataset.
    pub project_id: ProjectID,
    /// Name for the session's background task.
    pub name: String,
    /// Experiment (trainer) the session belongs to.
    pub experiment_id: ExperimentID,
    /// Trainer schema type (e.g. `"modelpack"`), from
    /// `Client::trainer_schemas`.
    pub trainer_type: String,
    /// Dataset to train on.
    pub dataset_id: DatasetID,
    /// Annotation set providing the ground-truth labels.
    pub annotation_set_id: AnnotationSetID,
    /// Dataset tag to train against; `None` selects the latest tag.
    pub tag_name: Option<String>,
    /// Training split group name; `None` uses the default `"train"`.
    pub train_group: Option<String>,
    /// Validation split group name; `None` uses the default `"val"`.
    pub val_group: Option<String>,
    /// Display name for the training session itself; `None` uses the
    /// task `name`.
    pub session_name: Option<String>,
    /// Optional description for the training session.
    pub session_description: Option<String>,
    /// Optional source session for transfer-learning weights.
    pub weights_session: Option<TrainingSessionID>,
    /// Trainer hyperparameters, keyed by schema parameter name.
    pub params: HashMap<String, Parameter>,
    /// Create a user-managed session (no cloud instance).
    pub is_local: bool,
    /// Schedule onto the organization's Kubernetes runner.
    pub is_kubernetes: bool,
}

/// Result of [`client::Client::start_training_session`].
///
/// Studio's `cloud.server.start` returns the freshly-created
/// `BackgroundTask` row. `task_id` can be polled via `Client::task_info`
/// to monitor the launch; `session_id` is the handle to the new training
/// session (for `Client::training_session`,
/// `Client::update_training_session`, and
/// `Client::delete_training_sessions`).
///
/// `session_id` is `Option` because the same endpoint also returns
/// non-trainer tasks and those don't populate `train_session_id`; for a
/// `type = "trainer"` launch it is always populated.
#[derive(Deserialize, Debug, Clone)]
pub struct NewTrainingSession {
    #[serde(rename = "id")]
    pub task_id: TaskID,
    #[serde(rename = "train_session_id", default)]
    pub session_id: Option<TrainingSessionID>,
}

impl Display for NewTrainingSession {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.session_id {
            Some(id) => write!(f, "task {} session {}", self.task_id, id),
            None => write!(f, "task {} (no session)", self.task_id),
        }
    }
}

/// A dataset version tag, as returned by `Client::dataset_tags`.
///
/// Tags mark dataset versions for reproducible training. The most
/// recently created tag (highest `id`) is treated as the latest.
#[derive(Deserialize, Debug, Clone)]
pub struct Tag {
    /// Tag identifier; creation-ordered, so the highest id is newest.
    pub id: u64,
    /// Tag name, referenced by training sessions as `tag_name`.
    pub name: String,
    /// The dataset this tag belongs to.
    #[serde(default)]
    pub dataset_id: u64,
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
                // Intermediate events are sampled with `try_send` so a slow
                // consumer never stalls the upload pipeline; the terminal
                // `current == total` event is emitted with an awaited send
                // after the multipart POST returns below so completion
                // handlers always fire.
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

        let result = match client.post_multipart("task.data.upload", form).await {
            Ok(_) => Ok(()),
            Err(Error::RpcError(code, msg)) => Err(client::map_rpc_error(
                "task.data.upload",
                code,
                msg,
                Some(self.id()),
            )),
            Err(e) => Err(e),
        };

        // Guaranteed completion event: send the terminal progress update
        // with `send().await` so consumers always see `current == total`
        // even if they were slow to drain intermediate samples.
        if result.is_ok()
            && let Some(tx) = progress
        {
            let _ = tx
                .send(Progress {
                    current: total,
                    total,
                    status: None,
                })
                .await;
        }
        result
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
    /// `Error::RpcError` if the server returns a JSON-RPC error envelope
    /// (decoded from the `Content-Type: application/json` body), or
    /// `Error::IoError` on file write failures. Legitimate JSON file
    /// payloads (e.g. trace JSON, chart bodies) are persisted normally
    /// rather than treated as an error.
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

// ──────────────────────────────────────────────────────────────────────────────
// Dataset Versioning Types
// ──────────────────────────────────────────────────────────────────────────────

/// A named version tag that captures a complete dataset state snapshot at a
/// specific serial number. Tags are immutable once created and enable
/// reproducible training and validation by referencing an exact dataset state.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct VersionTag {
    id: u64,
    dataset_id: u64,
    name: String,
    serial: u64,
    #[serde(default)]
    description: String,
    created_by: String,
    created_at: DateTime<Utc>,
    #[serde(default)]
    image_count: u64,
    #[serde(default)]
    annotation_counts: HashMap<String, u64>,
    #[serde(default)]
    sensor_counts: HashMap<String, u64>,
    #[serde(default)]
    label_count: u64,
    #[serde(default)]
    annotation_set_count: u64,
    #[serde(default)]
    snapshot_id: Option<u64>,
}

impl VersionTag {
    /// Returns the tag's unique identifier.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the dataset ID this tag belongs to.
    pub fn dataset_id(&self) -> u64 {
        self.dataset_id
    }

    /// Returns the tag name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the changelog serial number this tag references.
    pub fn serial(&self) -> u64 {
        self.serial
    }

    /// Returns the tag description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the username that created this tag.
    pub fn created_by(&self) -> &str {
        &self.created_by
    }

    /// Returns when this tag was created.
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns the number of images at tag time.
    pub fn image_count(&self) -> u64 {
        self.image_count
    }

    /// Returns annotation counts by type (e.g., `{"box": 150000, "seg": 20000}`).
    pub fn annotation_counts(&self) -> &HashMap<String, u64> {
        &self.annotation_counts
    }

    /// Returns sensor data counts by type (e.g., `{"lidar": 25000}`).
    pub fn sensor_counts(&self) -> &HashMap<String, u64> {
        &self.sensor_counts
    }

    /// Returns the number of labels at tag time.
    pub fn label_count(&self) -> u64 {
        self.label_count
    }

    /// Returns the number of annotation sets at tag time.
    pub fn annotation_set_count(&self) -> u64 {
        self.annotation_set_count
    }

    /// Returns the optional snapshot export ID.
    pub fn snapshot_id(&self) -> Option<u64> {
        self.snapshot_id
    }
}

impl Display for VersionTag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} (serial {})", self.name, self.serial)
    }
}

/// A single entry in the dataset changelog, recording one modification.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChangelogEntry {
    id: u64,
    dataset_id: u64,
    serial: u64,
    entity_type: String,
    operation: String,
    #[serde(default)]
    entity_id: Option<u64>,
    #[serde(default)]
    change_data: serde_json::Value,
    username: String,
    organization_id: u64,
    created_at: DateTime<Utc>,
    #[serde(default)]
    message: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    s3_version_ids: Vec<serde_json::Value>,
}

impl ChangelogEntry {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn dataset_id(&self) -> u64 {
        self.dataset_id
    }

    /// Returns the monotonic serial number for this change.
    pub fn serial(&self) -> u64 {
        self.serial
    }

    /// Returns the entity type (image, annotation, label, annotation_set, sensor_data, dataset).
    pub fn entity_type(&self) -> &str {
        &self.entity_type
    }

    /// Returns the operation (create, update, delete, bulk_create, bulk_delete, baseline, restore).
    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn entity_id(&self) -> Option<u64> {
        self.entity_id
    }

    /// Returns the change details as a JSON value.
    pub fn change_data(&self) -> &serde_json::Value {
        &self.change_data
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn organization_id(&self) -> u64 {
        self.organization_id
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn s3_version_ids(&self) -> &[serde_json::Value] {
        &self.s3_version_ids
    }
}

/// Paginated response from the `version.changelog` endpoint.
#[derive(Deserialize, Debug, Clone)]
pub struct ChangelogResponse {
    pub entries: Vec<ChangelogEntry>,
    pub count: u64,
    #[serde(default)]
    pub continue_token: String,
    #[serde(default)]
    pub from_serial: Option<u64>,
    #[serde(default)]
    pub to_serial: Option<u64>,
}

/// Cached metrics summary for a dataset's current state.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DatasetSummary {
    dataset_id: u64,
    current_serial: u64,
    #[serde(default)]
    image_count: u64,
    #[serde(default)]
    annotation_counts: HashMap<String, u64>,
    #[serde(default)]
    sensor_counts: HashMap<String, u64>,
    #[serde(default)]
    label_count: u64,
    #[serde(default)]
    annotation_set_count: u64,
    last_updated: DateTime<Utc>,
}

impl DatasetSummary {
    pub fn dataset_id(&self) -> u64 {
        self.dataset_id
    }

    pub fn current_serial(&self) -> u64 {
        self.current_serial
    }

    pub fn image_count(&self) -> u64 {
        self.image_count
    }

    pub fn annotation_counts(&self) -> &HashMap<String, u64> {
        &self.annotation_counts
    }

    pub fn sensor_counts(&self) -> &HashMap<String, u64> {
        &self.sensor_counts
    }

    pub fn label_count(&self) -> u64 {
        self.label_count
    }

    pub fn annotation_set_count(&self) -> u64 {
        self.annotation_set_count
    }

    pub fn last_updated(&self) -> DateTime<Utc> {
        self.last_updated
    }
}

/// Response from `version.current` with serial, tags, and summary.
#[derive(Deserialize, Debug, Clone)]
pub struct VersionCurrentResponse {
    pub dataset_id: u64,
    pub current_serial: u64,
    #[serde(default)]
    pub latest_tag: Option<VersionTag>,
    #[serde(default)]
    pub tags: Vec<VersionTag>,
    #[serde(default)]
    pub summary: Option<DatasetSummary>,
}

/// Source tag information in a restore result.
#[derive(Deserialize, Debug, Clone)]
pub struct RestoredFrom {
    pub tag: String,
    pub serial: u64,
}

/// Counts of entities restored.
#[derive(Deserialize, Debug, Clone)]
pub struct RestoredCounts {
    pub images: u64,
    pub labels: u64,
    pub annotation_sets: u64,
}

/// Result from `version.tag.restore`.
#[derive(Deserialize, Debug, Clone)]
pub struct RestoreResult {
    pub success: bool,
    pub new_serial: u64,
    pub restored_from: RestoredFrom,
    pub restored_counts: RestoredCounts,
    pub message: String,
}

// RPC parameter structs for versioning endpoints

#[derive(Serialize)]
pub(crate) struct VersionTagCreateParams {
    pub dataset_id: DatasetID,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct VersionTagNameParams {
    pub dataset_id: DatasetID,
    pub name: String,
}

#[derive(Serialize)]
pub(crate) struct VersionChangelogParams {
    pub dataset_id: DatasetID,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_token: Option<String>,
}

/// Count result from `version.changelog.count`.
#[derive(Deserialize, Debug)]
pub(crate) struct ChangelogCountResult {
    pub count: u64,
}

/// Catalog entry describing an available trainer type.
///
/// Returned by `Client::trainer_schemas`. The `schema_type` value is
/// what gets passed to `Client::trainer_schema` to fetch the full
/// parameter schema, and to `StartTrainingRequest::trainer_type` when
/// launching a training session.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainerSchemaInfo {
    /// Internal trainer name (e.g. `"modelpack"`).
    pub name: String,
    /// Human-readable label shown in the Studio UI.
    #[serde(default)]
    pub label: String,
    /// Schema type identifier used for schema lookup and launch.
    #[serde(default)]
    pub schema_type: String,
}

/// The kind of input a [`SchemaField`] describes.
///
/// Mirrors the field types rendered by the Studio UI's dynamic schema
/// forms. Unrecognized types deserialize as [`SchemaFieldType::Unknown`]
/// so newer servers never break older clients.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaFieldType {
    /// Container of nested fields (see [`SchemaField::children`]).
    Group,
    /// Numeric slider with `min`/`max`/`step` bounds.
    Slider,
    /// Selection from [`SchemaField::options`].
    Select,
    /// Boolean toggle, optionally revealing nested `children`.
    Bool,
    /// Integer input.
    Int,
    /// Floating-point input.
    Float,
    /// Text input.
    Text,
    /// Date input.
    Date,
    /// Studio project reference.
    Project,
    /// Studio dataset reference.
    Dataset,
    /// Studio training-session reference.
    Trainer,
    /// File upload.
    Upload,
    /// Server-side metadata entry (machine image, entrypoint); not a
    /// user-facing parameter.
    Info,
    /// Any type this client version does not recognize.
    #[serde(other)]
    Unknown,
}

/// Deserialize an optional string leniently: schema authors sometimes
/// use bare numbers or booleans for display fields (e.g. an option
/// labelled `1`), which are coerced to their string representation.
fn lenient_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.map(|v| match v {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    }))
}

/// One selectable option of a `select` [`SchemaField`].
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchemaOption {
    /// Option value; may be any JSON scalar (string, number, …).
    #[serde(default)]
    pub name: Option<Parameter>,
    /// Human-readable label; non-string labels (e.g. a bare number)
    /// are coerced to strings.
    #[serde(default, deserialize_with = "lenient_string")]
    pub label: Option<String>,
    /// Nested fields revealed when this option is selected.
    #[serde(default)]
    pub children: Vec<SchemaField>,
}

/// A single field descriptor from a trainer or validator parameter
/// schema.
///
/// Schemas describe the hyperparameters a trainer/validator accepts —
/// the same descriptors the Studio UI renders as dynamic forms. Use them
/// to discover parameter names, defaults, and valid ranges before
/// launching a session with `Client::start_training_session`.
///
/// Deserialization is tolerant: unknown JSON keys are ignored and most
/// fields are optional, so schema evolution on the server does not break
/// this client.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchemaField {
    /// Parameter name — the key to use in the launch `params` map.
    #[serde(default, deserialize_with = "lenient_string")]
    pub name: Option<String>,
    /// Human-readable label; non-string labels are coerced to strings.
    #[serde(default, deserialize_with = "lenient_string")]
    pub label: Option<String>,
    /// Longer description of the parameter.
    #[serde(default, deserialize_with = "lenient_string")]
    pub description: Option<String>,
    /// Whether a value is required to launch.
    #[serde(default)]
    pub required: bool,
    /// Default value applied when the parameter is omitted.
    #[serde(default)]
    pub default: Option<Parameter>,
    /// The kind of input this field describes.
    #[serde(rename = "type", default)]
    pub field_type: Option<SchemaFieldType>,
    /// Minimum value (numeric fields).
    #[serde(default)]
    pub min: Option<f64>,
    /// Maximum value (numeric fields).
    #[serde(default)]
    pub max: Option<f64>,
    /// Step size (numeric fields).
    #[serde(default)]
    pub step: Option<f64>,
    /// Selectable options (`select` fields).
    #[serde(default)]
    pub options: Vec<SchemaOption>,
    /// Nested fields (`group` fields, or `bool` fields that reveal
    /// sub-parameters when enabled).
    #[serde(default)]
    pub children: Vec<SchemaField>,
    /// Render the select as a dropdown.
    #[serde(default)]
    pub is_dropdown: bool,
    /// Allow selecting multiple options.
    #[serde(default)]
    pub multi_select: bool,
    /// Render the text input as multi-line.
    #[serde(default)]
    pub is_multi_line: bool,
    /// Mask the text input (passwords).
    #[serde(default)]
    pub hidden: bool,
    /// Restrict text input to numeric characters.
    #[serde(default)]
    pub numeric_only: bool,
    /// Dataset fields: enable dataset tag selection.
    #[serde(default)]
    pub enable_tags_selection: bool,
    /// Dataset fields: enable annotation set selection.
    #[serde(default)]
    pub enable_annotation_set_selection: bool,
    /// Slider fields: number of slider handles (1 = value, 2 = range).
    #[serde(default)]
    pub values: Option<Vec<Parameter>>,
}

/// A validator parameter schema, as returned by
/// `Client::validator_schemas`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ValidatorSchema {
    /// Schema type identifier (matched against a model's trainer type).
    #[serde(rename = "type", default)]
    pub schema_type: String,
    /// Internal validator name.
    #[serde(default)]
    pub name: String,
    /// The parameter field descriptors.
    #[serde(default)]
    pub schema: Vec<SchemaField>,
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

    // ========== UsageSummary Tests ==========
    #[test]
    fn test_usage_summary_deserialize_and_accessors() {
        let usage: UsageSummary = serde_json::from_str(
            r#"{"credits": 12.5, "funds": 49092.92, "total_funds_and_credits": 49105.42}"#,
        )
        .unwrap();
        assert_eq!(usage.credits(), 12.5);
        assert_eq!(usage.funds(), 49092.92);
        assert_eq!(usage.total(), 49105.42);
    }

    #[test]
    fn test_usage_summary_defaults_for_missing_fields() {
        // All fields are #[serde(default)] and `total` is renamed from
        // `total_funds_and_credits`, so an empty object yields zeros and an
        // unrenamed `total` key is ignored.
        let usage: UsageSummary = serde_json::from_str("{}").unwrap();
        assert_eq!(usage.credits(), 0.0);
        assert_eq!(usage.funds(), 0.0);
        assert_eq!(usage.total(), 0.0);
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

    // ========== Versioning Type Deserialization Tests ==========

    #[test]
    fn test_version_tag_deserialize_full() {
        let json = r#"{
            "id": 456, "dataset_id": 1715004, "name": "training-v1.0",
            "serial": 42, "description": "Ready for production",
            "created_by": "user@example.com", "created_at": "2025-01-15T10:30:00Z",
            "image_count": 50000, "annotation_counts": {"box": 150000, "seg": 20000},
            "sensor_counts": {"lidar": 25000}, "label_count": 15,
            "annotation_set_count": 3, "snapshot_id": 789
        }"#;
        let tag: VersionTag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name(), "training-v1.0");
        assert_eq!(tag.serial(), 42);
        assert_eq!(tag.image_count(), 50000);
        assert_eq!(tag.annotation_counts().get("box"), Some(&150000));
        assert_eq!(tag.snapshot_id(), Some(789));
    }

    #[test]
    fn test_version_tag_deserialize_omitempty() {
        // snapshot_id absent (Go omitempty) must deserialize as None
        let json = r#"{
            "id": 1, "dataset_id": 2, "name": "v1.0", "serial": 5,
            "description": "", "created_by": "user",
            "created_at": "2025-01-01T00:00:00Z"
        }"#;
        let tag: VersionTag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.snapshot_id(), None);
        assert_eq!(tag.image_count(), 0);
        assert!(tag.annotation_counts().is_empty());
    }

    #[test]
    fn test_changelog_entry_deserialize_omitempty() {
        // entity_id and s3_version_ids absent (Go omitempty)
        let json = r#"{
            "id": 1, "dataset_id": 2, "serial": 3, "entity_type": "image",
            "operation": "bulk_create", "change_data": {"count": 5},
            "username": "user", "organization_id": 1,
            "created_at": "2025-01-01T00:00:00Z", "message": ""
        }"#;
        let entry: ChangelogEntry = serde_json::from_str(json).unwrap();
        assert!(entry.entity_id().is_none());
        assert!(entry.s3_version_ids().is_empty());
        assert_eq!(entry.entity_type(), "image");
        assert_eq!(entry.operation(), "bulk_create");
    }

    #[test]
    fn test_changelog_response_deserialize() {
        let json = r#"{
            "entries": [], "count": 0, "continue_token": ""
        }"#;
        let resp: ChangelogResponse = serde_json::from_str(json).unwrap();
        assert!(resp.entries.is_empty());
        assert_eq!(resp.count, 0);
        assert!(resp.continue_token.is_empty());
        assert!(resp.from_serial.is_none());
    }

    #[test]
    fn test_version_current_no_latest_tag() {
        // latest_tag absent (Go omitempty) must deserialize as None
        let json = r#"{
            "dataset_id": 100, "current_serial": 5, "tags": []
        }"#;
        let resp: VersionCurrentResponse = serde_json::from_str(json).unwrap();
        assert!(resp.latest_tag.is_none());
        assert!(resp.tags.is_empty());
        assert_eq!(resp.current_serial, 5);
    }

    #[test]
    fn test_version_current_with_latest_tag() {
        let json = r#"{
            "dataset_id": 100, "current_serial": 42,
            "latest_tag": {
                "id": 1, "dataset_id": 100, "name": "v1.0", "serial": 42,
                "description": "test", "created_by": "user",
                "created_at": "2025-01-01T00:00:00Z",
                "image_count": 10, "label_count": 2, "annotation_set_count": 1
            },
            "tags": []
        }"#;
        let resp: VersionCurrentResponse = serde_json::from_str(json).unwrap();
        assert!(resp.latest_tag.is_some());
        assert_eq!(resp.latest_tag.unwrap().name(), "v1.0");
    }

    #[test]
    fn test_dataset_summary_deserialize() {
        let json = r#"{
            "dataset_id": 100, "current_serial": 10,
            "image_count": 5000, "annotation_counts": {"box": 10000},
            "sensor_counts": {}, "label_count": 8,
            "annotation_set_count": 2, "last_updated": "2025-06-01T12:00:00Z"
        }"#;
        let summary: DatasetSummary = serde_json::from_str(json).unwrap();
        assert_eq!(summary.image_count(), 5000);
        assert_eq!(summary.label_count(), 8);
        assert_eq!(summary.annotation_counts().get("box"), Some(&10000));
    }

    #[test]
    fn test_restore_result_deserialize() {
        let json = r#"{
            "success": true, "new_serial": 45,
            "restored_from": {"tag": "v1.0", "serial": 42},
            "restored_counts": {"images": 5000, "labels": 15, "annotation_sets": 3},
            "message": "Dataset restored to tag v1.0"
        }"#;
        let result: RestoreResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.new_serial, 45);
        assert_eq!(result.restored_from.tag, "v1.0");
        assert_eq!(result.restored_from.serial, 42);
        assert_eq!(result.restored_counts.images, 5000);
    }
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
