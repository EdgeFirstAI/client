// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use crate::{AnnotationSet, Client, Dataset, Error, Sample, client};
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

/// Unique identifier for an organization in EdgeFirst Studio.
///
/// Organizations are the top-level containers for users, projects, and resources
/// in EdgeFirst Studio. Each organization has a unique ID that is displayed
/// in hexadecimal format with an "org-" prefix (e.g., "org-abc123").
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
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrganizationID(u64);

impl Display for OrganizationID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "org-{:x}", self.0)
    }
}

impl From<u64> for OrganizationID {
    fn from(id: u64) -> Self {
        OrganizationID(id)
    }
}

impl From<OrganizationID> for u64 {
    fn from(val: OrganizationID) -> Self {
        val.0
    }
}

impl OrganizationID {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for OrganizationID {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let hex_part = s.strip_prefix("org-").ok_or_else(|| {
            Error::InvalidParameters("Organization ID must start with 'org-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(OrganizationID(id))
    }
}

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
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProjectID(u64);

impl Display for ProjectID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "p-{:x}", self.0)
    }
}

impl From<u64> for ProjectID {
    fn from(id: u64) -> Self {
        ProjectID(id)
    }
}

impl From<ProjectID> for u64 {
    fn from(val: ProjectID) -> Self {
        val.0
    }
}

impl ProjectID {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for ProjectID {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        ProjectID::from_str(s)
    }
}

impl TryFrom<String> for ProjectID {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        ProjectID::from_str(&s)
    }
}

impl FromStr for ProjectID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex_part = s.strip_prefix("p-").ok_or_else(|| {
            Error::InvalidParameters("Project ID must start with 'p-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(ProjectID(id))
    }
}

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
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ExperimentID(u64);

impl Display for ExperimentID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "exp-{:x}", self.0)
    }
}

impl From<u64> for ExperimentID {
    fn from(id: u64) -> Self {
        ExperimentID(id)
    }
}

impl From<ExperimentID> for u64 {
    fn from(val: ExperimentID) -> Self {
        val.0
    }
}

impl ExperimentID {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for ExperimentID {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        ExperimentID::from_str(s)
    }
}

impl TryFrom<String> for ExperimentID {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        ExperimentID::from_str(&s)
    }
}

impl FromStr for ExperimentID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex_part = s.strip_prefix("exp-").ok_or_else(|| {
            Error::InvalidParameters("Experiment ID must start with 'exp-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(ExperimentID(id))
    }
}

/// Unique identifier for a training session within an experiment.
///
/// Training sessions represent individual training runs with specific hyperparameters
/// and configurations. Each training session has a unique ID displayed in hexadecimal 
/// format with a "t-" prefix (e.g., "t-789012").
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
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TrainingSessionID(u64);

impl Display for TrainingSessionID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "t-{:x}", self.0)
    }
}

impl From<u64> for TrainingSessionID {
    fn from(id: u64) -> Self {
        TrainingSessionID(id)
    }
}

impl From<TrainingSessionID> for u64 {
    fn from(val: TrainingSessionID) -> Self {
        val.0
    }
}

impl TrainingSessionID {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for TrainingSessionID {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        TrainingSessionID::from_str(s)
    }
}

impl TryFrom<String> for TrainingSessionID {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        TrainingSessionID::from_str(&s)
    }
}

impl FromStr for TrainingSessionID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex_part = s.strip_prefix("t-").ok_or_else(|| {
            Error::InvalidParameters("Training Session ID must start with 't-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(TrainingSessionID(id))
    }
}

/// Unique identifier for a validation session within an experiment.
///
/// Validation sessions represent model validation runs that evaluate trained models
/// against test datasets. Each validation session has a unique ID displayed in 
/// hexadecimal format with a "v-" prefix (e.g., "v-345678").
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
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ValidationSessionID(u64);

impl Display for ValidationSessionID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "v-{:x}", self.0)
    }
}

impl From<u64> for ValidationSessionID {
    fn from(id: u64) -> Self {
        ValidationSessionID(id)
    }
}

impl From<ValidationSessionID> for u64 {
    fn from(val: ValidationSessionID) -> Self {
        val.0
    }
}

impl ValidationSessionID {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for ValidationSessionID {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let hex_part = s.strip_prefix("v-").ok_or_else(|| {
            Error::InvalidParameters(
                "Validation Session ID must start with 'v-' prefix".to_string(),
            )
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(ValidationSessionID(id))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SnapshotID(u64);

impl Display for SnapshotID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ss-{:x}", self.0)
    }
}

impl From<u64> for SnapshotID {
    fn from(id: u64) -> Self {
        SnapshotID(id)
    }
}

impl From<SnapshotID> for u64 {
    fn from(val: SnapshotID) -> Self {
        val.0
    }
}

impl SnapshotID {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for SnapshotID {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let hex_part = s.strip_prefix("ss-").ok_or_else(|| {
            Error::InvalidParameters("Snapshot ID must start with 'ss-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(SnapshotID(id))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskID(u64);

impl Display for TaskID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "task-{:x}", self.0)
    }
}

impl From<u64> for TaskID {
    fn from(id: u64) -> Self {
        TaskID(id)
    }
}

impl From<TaskID> for u64 {
    fn from(val: TaskID) -> Self {
        val.0
    }
}

impl TaskID {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for TaskID {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        TaskID::from_str(s)
    }
}

impl TryFrom<String> for TaskID {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        TaskID::from_str(&s)
    }
}

impl FromStr for TaskID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex_part = s.strip_prefix("task-").ok_or_else(|| {
            Error::InvalidParameters("Task ID must start with 'task-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(TaskID(id))
    }
}

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
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct DatasetID(u64);

impl Display for DatasetID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ds-{:x}", self.0)
    }
}

impl From<u64> for DatasetID {
    fn from(id: u64) -> Self {
        DatasetID(id)
    }
}

impl From<DatasetID> for u64 {
    fn from(val: DatasetID) -> Self {
        val.0
    }
}

impl DatasetID {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for DatasetID {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        DatasetID::from_str(s)
    }
}

impl TryFrom<String> for DatasetID {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        DatasetID::from_str(&s)
    }
}

impl FromStr for DatasetID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex_part = s.strip_prefix("ds-").ok_or_else(|| {
            Error::InvalidParameters("Dataset ID must start with 'ds-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(DatasetID(id))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnnotationSetID(u64);

impl Display for AnnotationSetID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "as-{:x}", self.0)
    }
}

impl From<u64> for AnnotationSetID {
    fn from(id: u64) -> Self {
        AnnotationSetID(id)
    }
}

impl From<AnnotationSetID> for u64 {
    fn from(val: AnnotationSetID) -> Self {
        val.0
    }
}

impl AnnotationSetID {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for AnnotationSetID {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        AnnotationSetID::from_str(s)
    }
}

impl TryFrom<String> for AnnotationSetID {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        AnnotationSetID::from_str(&s)
    }
}

impl FromStr for AnnotationSetID {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex_part = s.strip_prefix("as-").ok_or_else(|| {
            Error::InvalidParameters("Annotation Set ID must start with 'as-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(AnnotationSetID(id))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SampleID(u64);

impl Display for SampleID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "s-{:x}", self.0)
    }
}

impl From<u64> for SampleID {
    fn from(id: u64) -> Self {
        SampleID(id)
    }
}

impl From<SampleID> for u64 {
    fn from(val: SampleID) -> Self {
        val.0
    }
}

impl SampleID {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for SampleID {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let hex_part = s.strip_prefix("s-").ok_or_else(|| {
            Error::InvalidParameters("Sample ID must start with 's-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(SampleID(id))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AppId(u64);

impl Display for AppId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "app-{:x}", self.0)
    }
}

impl From<u64> for AppId {
    fn from(id: u64) -> Self {
        AppId(id)
    }
}

impl From<AppId> for u64 {
    fn from(val: AppId) -> Self {
        val.0
    }
}

impl AppId {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for AppId {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let hex_part = s.strip_prefix("app-").ok_or_else(|| {
            Error::InvalidParameters("App ID must start with 'app-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(AppId(id))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageId(u64);

impl Display for ImageId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "im-{:x}", self.0)
    }
}

impl From<u64> for ImageId {
    fn from(id: u64) -> Self {
        ImageId(id)
    }
}

impl From<ImageId> for u64 {
    fn from(val: ImageId) -> Self {
        val.0
    }
}

impl ImageId {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for ImageId {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let hex_part = s.strip_prefix("im-").ok_or_else(|| {
            Error::InvalidParameters("Image ID must start with 'im-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(ImageId(id))
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SequenceId(u64);

impl Display for SequenceId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "se-{:x}", self.0)
    }
}

impl From<u64> for SequenceId {
    fn from(id: u64) -> Self {
        SequenceId(id)
    }
}

impl From<SequenceId> for u64 {
    fn from(val: SequenceId) -> Self {
        val.0
    }
}

impl SequenceId {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl TryFrom<&str> for SequenceId {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let hex_part = s.strip_prefix("se-").ok_or_else(|| {
            Error::InvalidParameters("Sequence ID must start with 'se-' prefix".to_string())
        })?;
        let id = u64::from_str_radix(hex_part, 16)?;
        Ok(SequenceId(id))
    }
}

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
    #[serde(rename = "enabled_topics")]
    pub topics: Vec<String>,
    #[serde(rename = "label_names")]
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
    #[serde(rename = "docker_task_id")]
    pub task_id: TaskID,
    pub date: DateTime<Utc>,
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
        write!(f, "{} {}", self.uid(), self.name)
    }
}

impl Experiment {
    pub fn id(&self) -> ExperimentID {
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
        write!(f, "{} {}", self.uid(), self.name())
    }
}

impl TrainingSession {
    pub fn id(&self) -> TrainingSessionID {
        self.id
    }

    pub fn uid(&self) -> String {
        self.id.to_string()
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

    pub fn uid(&self) -> String {
        self.id.to_string()
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

        let result = client
            .post_multipart("validate.upload.files", parts)
            .await?;
        trace!("ValidationSession::upload: {:?}", result);
        Ok(())
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
    #[serde(rename = "manage_types", skip_serializing_if = "Option::is_none")]
    pub manager: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Vec<String>>,
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

    pub fn uid(&self) -> String {
        self.id.to_string()
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
            self.uid(),
            self.manager(),
            self.workflow(),
            self.name()
        )
    }
}

#[derive(Deserialize, Debug)]
pub struct TaskInfo {
    id: TaskID,
    project_id: Option<ProjectID>,
    #[serde(rename = "task_description")]
    description: String,
    #[serde(rename = "type")]
    workflow: String,
    status: Option<String>,
    progress: TaskProgress,
    #[serde(rename = "created_date")]
    created: DateTime<Utc>,
    #[serde(rename = "end_date")]
    completed: DateTime<Utc>,
}

impl Display for TaskInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {}: {}",
            self.uid(),
            self.workflow(),
            self.description()
        )
    }
}

impl TaskInfo {
    pub fn id(&self) -> TaskID {
        self.id
    }

    pub fn uid(&self) -> String {
        self.id.to_string()
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

    pub fn created(&self) -> &DateTime<Utc> {
        &self.created
    }

    pub fn completed(&self) -> &DateTime<Utc> {
        &self.completed
    }
}

#[derive(Deserialize, Debug)]
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
