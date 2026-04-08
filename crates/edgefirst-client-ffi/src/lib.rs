//! UniFFI bindings for EdgeFirst Client.
//!
//! This crate provides Kotlin and Swift bindings for the EdgeFirst Client
//! library using Mozilla's UniFFI framework.
//!
//! # Overview
//!
//! The FFI layer exposes the core EdgeFirst Client functionality to mobile
//! platforms:
//! - **Kotlin** bindings for Android applications
//! - **Swift** bindings for iOS/macOS applications
//!
//! # Token Storage
//!
//! Mobile platforms should implement the `TokenStorage` callback interface to
//! provide secure, platform-appropriate token persistence:
//! - Android: Use `EncryptedSharedPreferences` or Android Keystore
//! - iOS/macOS: Use Keychain Services
//!
//! # Example (Kotlin)
//!
//! ```kotlin
//! val client = Client()
//!     .withServer("test")
//!     .withLogin("username", "password")
//!
//! val projects = client.projects(null)
//! ```
//!
//! # Example (Swift)
//!
//! ```swift
//! let client = try Client()
//!     .withServer("test")
//!     .withLogin(username: "username", password: "password")
//!
//! let projects = try await client.projectsAsync(name: nil)
//! ```

uniffi::setup_scaffolding!();

use std::{collections::HashMap, sync::Arc};

use async_compat::CompatExt;
use edgefirst_client as core;

// =============================================================================
// Error Types
// =============================================================================

/// Error type for token storage operations.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum StorageError {
    /// Storage is not available (e.g., cannot determine config directory).
    #[error("Token storage not available: {message}")]
    NotAvailable { message: String },
    /// Failed to read token from storage.
    #[error("Failed to read token: {message}")]
    ReadError { message: String },
    /// Failed to write token to storage.
    #[error("Failed to write token: {message}")]
    WriteError { message: String },
    /// Failed to clear token from storage.
    #[error("Failed to clear token: {message}")]
    ClearError { message: String },
}

impl From<core::StorageError> for StorageError {
    fn from(err: core::StorageError) -> Self {
        match err {
            core::StorageError::NotAvailable(msg) => StorageError::NotAvailable { message: msg },
            core::StorageError::ReadError(msg) => StorageError::ReadError { message: msg },
            core::StorageError::WriteError(msg) => StorageError::WriteError { message: msg },
            core::StorageError::ClearError(msg) => StorageError::ClearError { message: msg },
        }
    }
}

/// Error type for client operations.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ClientError {
    /// Authentication failed or token is invalid/expired.
    #[error("Authentication error: {message}")]
    AuthenticationError { message: String },
    /// Network or HTTP error.
    #[error("Network error: {message}")]
    NetworkError { message: String },
    /// Invalid parameters provided to an operation.
    #[error("Invalid parameters: {message}")]
    InvalidParameters { message: String },
    /// Requested resource was not found.
    #[error("Not found: {message}")]
    NotFound { message: String },
    /// Token storage operation failed.
    #[error("Storage error: {message}")]
    StorageError { message: String },
    /// Internal error or unexpected condition.
    #[error("Internal error: {message}")]
    InternalError { message: String },
}

impl From<core::Error> for ClientError {
    fn from(err: core::Error) -> Self {
        match err {
            core::Error::EmptyToken | core::Error::InvalidToken | core::Error::TokenExpired => {
                ClientError::AuthenticationError {
                    message: err.to_string(),
                }
            }
            core::Error::Unauthorized => ClientError::AuthenticationError {
                message: "Unauthorized".to_string(),
            },
            core::Error::HttpError(e) => ClientError::NetworkError {
                message: e.to_string(),
            },
            core::Error::UrlParseError(e) => ClientError::InvalidParameters {
                message: e.to_string(),
            },
            core::Error::InvalidParameters(msg) => ClientError::InvalidParameters { message: msg },
            core::Error::InvalidFileType(msg) => ClientError::InvalidParameters {
                message: format!("Invalid file type: {}", msg),
            },
            core::Error::InvalidAnnotationType(msg) => ClientError::InvalidParameters {
                message: format!("Invalid annotation type: {}", msg),
            },
            core::Error::StorageError(msg) => ClientError::StorageError { message: msg },
            core::Error::RpcError(code, msg) => {
                if code == -32001 || code == -32002 {
                    ClientError::AuthenticationError { message: msg }
                } else if code == -32004 {
                    ClientError::NotFound { message: msg }
                } else {
                    ClientError::InternalError {
                        message: format!("RPC error {}: {}", code, msg),
                    }
                }
            }
            _ => ClientError::InternalError {
                message: err.to_string(),
            },
        }
    }
}

// =============================================================================
// Token Storage Callback Interface
// =============================================================================

/// Trait for persistent token storage.
///
/// Implement this interface in Kotlin/Swift to provide platform-specific
/// secure token storage (e.g., Android Keystore, iOS Keychain).
#[uniffi::export(callback_interface)]
pub trait TokenStorage: Send + Sync {
    /// Store the authentication token.
    fn store(&self, token: String) -> Result<(), StorageError>;

    /// Load the stored authentication token.
    /// Returns `None` if no token is stored.
    fn load(&self) -> Result<Option<String>, StorageError>;

    /// Clear the stored authentication token.
    fn clear(&self) -> Result<(), StorageError>;
}

/// Bridge to convert FFI TokenStorage to core TokenStorage.
#[allow(dead_code)] // Used via create_client_with_storage factory function
struct FfiTokenStorageBridge {
    inner: Arc<dyn TokenStorage>,
}

impl core::TokenStorage for FfiTokenStorageBridge {
    fn store(&self, token: &str) -> Result<(), core::StorageError> {
        self.inner.store(token.to_string()).map_err(|e| match e {
            StorageError::NotAvailable { message } => core::StorageError::NotAvailable(message),
            StorageError::ReadError { message } => core::StorageError::ReadError(message),
            StorageError::WriteError { message } => core::StorageError::WriteError(message),
            StorageError::ClearError { message } => core::StorageError::ClearError(message),
        })
    }

    fn load(&self) -> Result<Option<String>, core::StorageError> {
        self.inner.load().map_err(|e| match e {
            StorageError::NotAvailable { message } => core::StorageError::NotAvailable(message),
            StorageError::ReadError { message } => core::StorageError::ReadError(message),
            StorageError::WriteError { message } => core::StorageError::WriteError(message),
            StorageError::ClearError { message } => core::StorageError::ClearError(message),
        })
    }

    fn clear(&self) -> Result<(), core::StorageError> {
        self.inner.clear().map_err(|e| match e {
            StorageError::NotAvailable { message } => core::StorageError::NotAvailable(message),
            StorageError::ReadError { message } => core::StorageError::ReadError(message),
            StorageError::WriteError { message } => core::StorageError::WriteError(message),
            StorageError::ClearError { message } => core::StorageError::ClearError(message),
        })
    }
}

// =============================================================================
// ID Types
// =============================================================================

/// Unique identifier for an organization.
#[derive(uniffi::Record, Clone, Debug)]
pub struct OrganizationId {
    pub value: u64,
}

impl From<core::OrganizationID> for OrganizationId {
    fn from(id: core::OrganizationID) -> Self {
        Self { value: id.value() }
    }
}

impl From<OrganizationId> for core::OrganizationID {
    fn from(id: OrganizationId) -> Self {
        core::OrganizationID::from(id.value)
    }
}

/// Unique identifier for a project.
#[derive(uniffi::Record, Clone, Debug)]
pub struct ProjectId {
    pub value: u64,
}

impl From<core::ProjectID> for ProjectId {
    fn from(id: core::ProjectID) -> Self {
        Self { value: id.value() }
    }
}

impl From<ProjectId> for core::ProjectID {
    fn from(id: ProjectId) -> Self {
        core::ProjectID::from(id.value)
    }
}

/// Unique identifier for a dataset.
#[derive(uniffi::Record, Clone, Debug)]
pub struct DatasetId {
    pub value: u64,
}

impl From<core::DatasetID> for DatasetId {
    fn from(id: core::DatasetID) -> Self {
        Self { value: id.value() }
    }
}

impl From<DatasetId> for core::DatasetID {
    fn from(id: DatasetId) -> Self {
        core::DatasetID::from(id.value)
    }
}

/// Unique identifier for an experiment.
#[derive(uniffi::Record, Clone, Debug)]
pub struct ExperimentId {
    pub value: u64,
}

impl From<core::ExperimentID> for ExperimentId {
    fn from(id: core::ExperimentID) -> Self {
        Self { value: id.value() }
    }
}

impl From<ExperimentId> for core::ExperimentID {
    fn from(id: ExperimentId) -> Self {
        core::ExperimentID::from(id.value)
    }
}

/// Unique identifier for a training session.
#[derive(uniffi::Record, Clone, Debug)]
pub struct TrainingSessionId {
    pub value: u64,
}

impl From<core::TrainingSessionID> for TrainingSessionId {
    fn from(id: core::TrainingSessionID) -> Self {
        Self { value: id.value() }
    }
}

impl From<TrainingSessionId> for core::TrainingSessionID {
    fn from(id: TrainingSessionId) -> Self {
        core::TrainingSessionID::from(id.value)
    }
}

/// Unique identifier for a validation session.
#[derive(uniffi::Record, Clone, Debug)]
pub struct ValidationSessionId {
    pub value: u64,
}

impl From<core::ValidationSessionID> for ValidationSessionId {
    fn from(id: core::ValidationSessionID) -> Self {
        Self { value: id.value() }
    }
}

impl From<ValidationSessionId> for core::ValidationSessionID {
    fn from(id: ValidationSessionId) -> Self {
        core::ValidationSessionID::from(id.value)
    }
}

/// Unique identifier for a snapshot.
#[derive(uniffi::Record, Clone, Debug)]
pub struct SnapshotId {
    pub value: u64,
}

impl From<core::SnapshotID> for SnapshotId {
    fn from(id: core::SnapshotID) -> Self {
        Self { value: id.value() }
    }
}

impl From<SnapshotId> for core::SnapshotID {
    fn from(id: SnapshotId) -> Self {
        core::SnapshotID::from(id.value)
    }
}

/// Unique identifier for a task.
#[derive(uniffi::Record, Clone, Debug)]
pub struct TaskId {
    pub value: u64,
}

impl From<core::TaskID> for TaskId {
    fn from(id: core::TaskID) -> Self {
        Self { value: id.value() }
    }
}

impl From<TaskId> for core::TaskID {
    fn from(id: TaskId) -> Self {
        core::TaskID::from(id.value)
    }
}

/// Unique identifier for an annotation set.
#[derive(uniffi::Record, Clone, Debug)]
pub struct AnnotationSetId {
    pub value: u64,
}

impl From<core::AnnotationSetID> for AnnotationSetId {
    fn from(id: core::AnnotationSetID) -> Self {
        Self { value: id.value() }
    }
}

impl From<AnnotationSetId> for core::AnnotationSetID {
    fn from(id: AnnotationSetId) -> Self {
        core::AnnotationSetID::from(id.value)
    }
}

/// Unique identifier for a sample.
#[derive(uniffi::Record, Clone, Debug)]
pub struct SampleId {
    pub value: u64,
}

impl From<core::SampleID> for SampleId {
    fn from(id: core::SampleID) -> Self {
        Self { value: id.value() }
    }
}

impl From<SampleId> for core::SampleID {
    fn from(id: SampleId) -> Self {
        core::SampleID::from(id.value)
    }
}

/// Unique identifier for an image.
#[derive(uniffi::Record, Clone, Debug)]
pub struct ImageId {
    pub value: u64,
}

impl From<core::ImageId> for ImageId {
    fn from(id: core::ImageId) -> Self {
        Self { value: id.value() }
    }
}

impl From<ImageId> for core::ImageId {
    fn from(id: ImageId) -> Self {
        core::ImageId::from(id.value)
    }
}

/// Unique identifier for an application.
#[derive(uniffi::Record, Clone, Debug)]
pub struct AppId {
    pub value: u64,
}

impl From<core::AppId> for AppId {
    fn from(id: core::AppId) -> Self {
        Self { value: id.value() }
    }
}

impl From<AppId> for core::AppId {
    fn from(id: AppId) -> Self {
        core::AppId::from(id.value)
    }
}

/// Unique identifier for a sequence.
#[derive(uniffi::Record, Clone, Debug)]
pub struct SequenceId {
    pub value: u64,
}

impl From<core::SequenceId> for SequenceId {
    fn from(id: core::SequenceId) -> Self {
        Self { value: id.value() }
    }
}

impl From<SequenceId> for core::SequenceId {
    fn from(id: SequenceId) -> Self {
        core::SequenceId::from(id.value)
    }
}

// =============================================================================
// ID String Parse/Format Functions
// =============================================================================

/// Generates a pair of UniFFI-exported free functions for parsing an ID type
/// from its string representation (e.g. `"p-42"`) and formatting it back.
macro_rules! ffi_id_string_functions {
    ($parse_fn:ident, $format_fn:ident, $ffi_type:ident, $core_type:ty) => {
        #[uniffi::export]
        fn $parse_fn(s: String) -> Result<$ffi_type, ClientError> {
            let id: $core_type =
                s.parse()
                    .map_err(|e: core::Error| ClientError::InvalidParameters {
                        message: e.to_string(),
                    })?;
            Ok($ffi_type::from(id))
        }

        #[uniffi::export]
        fn $format_fn(id: $ffi_type) -> String {
            let core_id = <$core_type>::from(id);
            core_id.to_string()
        }
    };
}

ffi_id_string_functions!(
    parse_organization_id,
    format_organization_id,
    OrganizationId,
    core::OrganizationID
);
ffi_id_string_functions!(
    parse_project_id,
    format_project_id,
    ProjectId,
    core::ProjectID
);
ffi_id_string_functions!(
    parse_experiment_id,
    format_experiment_id,
    ExperimentId,
    core::ExperimentID
);
ffi_id_string_functions!(
    parse_training_session_id,
    format_training_session_id,
    TrainingSessionId,
    core::TrainingSessionID
);
ffi_id_string_functions!(
    parse_validation_session_id,
    format_validation_session_id,
    ValidationSessionId,
    core::ValidationSessionID
);
ffi_id_string_functions!(
    parse_snapshot_id,
    format_snapshot_id,
    SnapshotId,
    core::SnapshotID
);
ffi_id_string_functions!(parse_task_id, format_task_id, TaskId, core::TaskID);
ffi_id_string_functions!(
    parse_dataset_id,
    format_dataset_id,
    DatasetId,
    core::DatasetID
);
ffi_id_string_functions!(
    parse_annotation_set_id,
    format_annotation_set_id,
    AnnotationSetId,
    core::AnnotationSetID
);
ffi_id_string_functions!(parse_sample_id, format_sample_id, SampleId, core::SampleID);
ffi_id_string_functions!(parse_app_id, format_app_id, AppId, core::AppId);
ffi_id_string_functions!(parse_image_id, format_image_id, ImageId, core::ImageId);
ffi_id_string_functions!(
    parse_sequence_id,
    format_sequence_id,
    SequenceId,
    core::SequenceId
);

// =============================================================================
// Enum Types
// =============================================================================

/// File types supported in EdgeFirst Studio datasets.
#[derive(uniffi::Enum, Clone, Debug)]
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
    /// All sensor types (expands to all of the above)
    All,
}

impl From<core::FileType> for FileType {
    fn from(ft: core::FileType) -> Self {
        match ft {
            core::FileType::Image => FileType::Image,
            core::FileType::LidarPcd => FileType::LidarPcd,
            core::FileType::LidarDepth => FileType::LidarDepth,
            core::FileType::LidarReflect => FileType::LidarReflect,
            core::FileType::RadarPcd => FileType::RadarPcd,
            core::FileType::RadarCube => FileType::RadarCube,
            core::FileType::All => FileType::All,
        }
    }
}

impl From<FileType> for core::FileType {
    fn from(ft: FileType) -> Self {
        match ft {
            FileType::Image => core::FileType::Image,
            FileType::LidarPcd => core::FileType::LidarPcd,
            FileType::LidarDepth => core::FileType::LidarDepth,
            FileType::LidarReflect => core::FileType::LidarReflect,
            FileType::RadarPcd => core::FileType::RadarPcd,
            FileType::RadarCube => core::FileType::RadarCube,
            FileType::All => core::FileType::All,
        }
    }
}

/// Annotation types supported for labeling data.
#[derive(uniffi::Enum, Clone, Debug)]
pub enum AnnotationType {
    /// 2D bounding boxes for object detection in images
    Box2d,
    /// 3D bounding boxes for object detection in 3D space
    Box3d,
    /// Vector polygon contours for instance segmentation
    Polygon,
    /// Raster pixel masks for semantic/instance segmentation
    Mask,
}

impl From<core::AnnotationType> for AnnotationType {
    fn from(at: core::AnnotationType) -> Self {
        match at {
            core::AnnotationType::Box2d => AnnotationType::Box2d,
            core::AnnotationType::Box3d => AnnotationType::Box3d,
            core::AnnotationType::Polygon => AnnotationType::Polygon,
            core::AnnotationType::Mask => AnnotationType::Mask,
        }
    }
}

impl From<AnnotationType> for core::AnnotationType {
    fn from(at: AnnotationType) -> Self {
        match at {
            AnnotationType::Box2d => core::AnnotationType::Box2d,
            AnnotationType::Box3d => core::AnnotationType::Box3d,
            AnnotationType::Polygon => core::AnnotationType::Polygon,
            AnnotationType::Mask => core::AnnotationType::Mask,
        }
    }
}

/// Generic parameter value used in API requests and configuration.
#[derive(uniffi::Enum, Clone, Debug)]
pub enum Parameter {
    /// 64-bit signed integer value.
    Integer { value: i64 },
    /// 64-bit floating-point value.
    Real { value: f64 },
    /// Boolean true/false value.
    Boolean { value: bool },
    /// UTF-8 string value.
    String { value: String },
    /// Array of nested parameter values.
    Array { values: Vec<Parameter> },
    /// Object/map with string keys and parameter values.
    Object { entries: HashMap<String, Parameter> },
}

impl From<core::Parameter> for Parameter {
    fn from(p: core::Parameter) -> Self {
        match p {
            core::Parameter::Integer(v) => Parameter::Integer { value: v },
            core::Parameter::Real(v) => Parameter::Real { value: v },
            core::Parameter::Boolean(v) => Parameter::Boolean { value: v },
            core::Parameter::String(v) => Parameter::String { value: v },
            core::Parameter::Array(arr) => Parameter::Array {
                values: arr.into_iter().map(Parameter::from).collect(),
            },
            core::Parameter::Object(map) => Parameter::Object {
                entries: map
                    .into_iter()
                    .map(|(k, v)| (k, Parameter::from(v)))
                    .collect(),
            },
        }
    }
}

impl From<Parameter> for core::Parameter {
    fn from(p: Parameter) -> Self {
        match p {
            Parameter::Integer { value } => core::Parameter::Integer(value),
            Parameter::Real { value } => core::Parameter::Real(value),
            Parameter::Boolean { value } => core::Parameter::Boolean(value),
            Parameter::String { value } => core::Parameter::String(value),
            Parameter::Array { values } => {
                core::Parameter::Array(values.into_iter().map(core::Parameter::from).collect())
            }
            Parameter::Object { entries } => core::Parameter::Object(
                entries
                    .into_iter()
                    .map(|(k, v)| (k, core::Parameter::from(v)))
                    .collect(),
            ),
        }
    }
}

// =============================================================================
// Data Record Types
// =============================================================================

/// Organization information and metadata.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub credits: i64,
}

impl From<core::Organization> for Organization {
    fn from(org: core::Organization) -> Self {
        Self {
            id: org.id().into(),
            name: org.name().to_string(),
            credits: org.credits(),
        }
    }
}

/// A project in EdgeFirst Studio.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: String,
}

impl From<core::Project> for Project {
    fn from(p: core::Project) -> Self {
        Self {
            id: p.id().into(),
            name: p.name().to_string(),
            description: p.description().to_string(),
        }
    }
}

/// A dataset in EdgeFirst Studio.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Dataset {
    pub id: DatasetId,
    pub project_id: ProjectId,
    pub name: String,
    pub description: String,
    pub created: String,
}

impl From<core::Dataset> for Dataset {
    fn from(d: core::Dataset) -> Self {
        Self {
            id: d.id().into(),
            project_id: d.project_id().into(),
            name: d.name().to_string(),
            description: d.description().to_string(),
            created: d.created().to_rfc3339(),
        }
    }
}

/// An annotation set in a dataset.
#[derive(uniffi::Record, Clone, Debug)]
pub struct AnnotationSet {
    pub id: AnnotationSetId,
    pub dataset_id: DatasetId,
    pub name: String,
    pub description: String,
    pub created: String,
}

impl From<core::AnnotationSet> for AnnotationSet {
    fn from(a: core::AnnotationSet) -> Self {
        Self {
            id: a.id().into(),
            dataset_id: a.dataset_id().into(),
            name: a.name().to_string(),
            description: a.description().to_string(),
            created: a.created().to_rfc3339(),
        }
    }
}

/// A label for annotations.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Label {
    pub id: u64,
    pub name: String,
}

impl From<core::Label> for Label {
    fn from(l: core::Label) -> Self {
        Self {
            id: l.id(),
            name: l.name().to_string(),
        }
    }
}

/// 2D bounding box annotation.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Box2d {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl From<core::Box2d> for Box2d {
    fn from(b: core::Box2d) -> Self {
        Self {
            left: b.left(),
            top: b.top(),
            width: b.width(),
            height: b.height(),
        }
    }
}

impl From<Box2d> for core::Box2d {
    fn from(b: Box2d) -> Self {
        core::Box2d::new(b.left, b.top, b.width, b.height)
    }
}

/// 3D bounding box annotation.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Box3d {
    pub cx: f32,
    pub cy: f32,
    pub cz: f32,
    pub width: f32,
    pub height: f32,
    pub length: f32,
}

impl From<core::Box3d> for Box3d {
    fn from(b: core::Box3d) -> Self {
        Self {
            cx: b.cx(),
            cy: b.cy(),
            cz: b.cz(),
            width: b.width(),
            height: b.height(),
            length: b.length(),
        }
    }
}

impl From<Box3d> for core::Box3d {
    fn from(b: Box3d) -> Self {
        core::Box3d::new(b.cx, b.cy, b.cz, b.width, b.height, b.length)
    }
}

/// GPS location data.
#[derive(uniffi::Record, Clone, Debug)]
pub struct GpsData {
    pub lat: f64,
    pub lon: f64,
}

impl From<core::GpsData> for GpsData {
    fn from(g: core::GpsData) -> Self {
        Self {
            lat: g.lat,
            lon: g.lon,
        }
    }
}

/// IMU orientation data (roll, pitch, yaw in degrees).
#[derive(uniffi::Record, Clone, Debug)]
pub struct ImuData {
    pub roll: f64,
    pub pitch: f64,
    pub yaw: f64,
}

impl From<core::ImuData> for ImuData {
    fn from(i: core::ImuData) -> Self {
        Self {
            roll: i.roll,
            pitch: i.pitch,
            yaw: i.yaw,
        }
    }
}

/// An experiment in EdgeFirst Studio.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Experiment {
    pub id: ExperimentId,
    pub project_id: ProjectId,
    pub name: String,
    pub description: String,
}

impl From<core::Experiment> for Experiment {
    fn from(e: core::Experiment) -> Self {
        Self {
            id: e.id().into(),
            project_id: e.project_id().into(),
            name: e.name().to_string(),
            description: e.description().to_string(),
        }
    }
}

/// A training session in an experiment.
#[derive(uniffi::Record, Clone, Debug)]
pub struct TrainingSession {
    pub id: TrainingSessionId,
    pub experiment_id: ExperimentId,
    pub name: String,
    pub description: String,
    pub model: String,
}

impl From<core::TrainingSession> for TrainingSession {
    fn from(t: core::TrainingSession) -> Self {
        Self {
            id: t.id().into(),
            experiment_id: t.experiment_id().into(),
            name: t.name().to_string(),
            description: t.description().to_string(),
            model: t.model().to_string(),
        }
    }
}

/// A validation session in an experiment.
#[derive(uniffi::Record, Clone, Debug)]
pub struct ValidationSession {
    pub id: ValidationSessionId,
    pub experiment_id: ExperimentId,
    pub training_session_id: TrainingSessionId,
    pub dataset_id: DatasetId,
    pub annotation_set_id: AnnotationSetId,
    pub description: String,
}

impl From<core::ValidationSession> for ValidationSession {
    fn from(v: core::ValidationSession) -> Self {
        Self {
            id: v.id().into(),
            experiment_id: v.experiment_id().into(),
            training_session_id: v.training_session_id().into(),
            dataset_id: v.dataset_id().into(),
            annotation_set_id: v.annotation_set_id().into(),
            description: v.description().to_string(),
        }
    }
}

/// A task in EdgeFirst Studio.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub workflow: String,
    pub status: String,
    pub manager: Option<String>,
    pub instance: String,
    pub created: String,
}

impl From<core::Task> for Task {
    fn from(t: core::Task) -> Self {
        Self {
            id: t.id().into(),
            name: t.name().to_string(),
            workflow: t.workflow().to_string(),
            status: t.status().to_string(),
            manager: t.manager().map(|s| s.to_string()),
            instance: t.instance().to_string(),
            created: t.created().to_rfc3339(),
        }
    }
}

/// Detailed task information.
#[derive(uniffi::Record, Clone, Debug)]
pub struct TaskInfo {
    pub id: TaskId,
    pub project_id: Option<ProjectId>,
    pub description: String,
    pub workflow: String,
    pub status: Option<String>,
    pub created: String,
    pub completed: String,
}

impl From<core::TaskInfo> for TaskInfo {
    fn from(t: core::TaskInfo) -> Self {
        Self {
            id: t.id().into(),
            project_id: t.project_id().map(|id| id.into()),
            description: t.description().to_string(),
            workflow: t.workflow().to_string(),
            status: t.status().clone(),
            created: t.created().to_rfc3339(),
            completed: t.completed().to_rfc3339(),
        }
    }
}

/// A stage in a task's progress.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Stage {
    pub stage: String,
    pub status: Option<String>,
    pub message: Option<String>,
    pub percentage: u8,
}

impl From<core::Stage> for Stage {
    fn from(s: core::Stage) -> Self {
        Self {
            stage: s.stage().to_string(),
            status: s.status().clone(),
            message: s.message().clone(),
            percentage: s.percentage(),
        }
    }
}

/// A model artifact from a training session.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Artifact {
    pub name: String,
    pub model_type: String,
}

impl From<core::Artifact> for Artifact {
    fn from(a: core::Artifact) -> Self {
        Self {
            name: a.name().to_string(),
            model_type: a.model_type().to_string(),
        }
    }
}

/// A snapshot in EdgeFirst Studio.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub description: String,
    pub status: String,
    pub path: String,
    pub created: String,
}

impl From<core::Snapshot> for Snapshot {
    fn from(s: core::Snapshot) -> Self {
        Self {
            id: s.id().into(),
            description: s.description().to_string(),
            status: s.status().to_string(),
            path: s.path().to_string(),
            created: s.created().to_rfc3339(),
        }
    }
}

/// A dataset version tag (immutable point-in-time marker).
#[derive(uniffi::Record, Clone, Debug)]
pub struct VersionTag {
    pub id: u64,
    pub dataset_id: u64,
    pub name: String,
    pub serial: u64,
    pub description: String,
    pub created_by: String,
    pub created_at: String,
    pub image_count: u64,
    pub annotation_counts: HashMap<String, u64>,
    pub sensor_counts: HashMap<String, u64>,
    pub label_count: u64,
    pub annotation_set_count: u64,
    pub snapshot_id: Option<u64>,
}

impl From<core::VersionTag> for VersionTag {
    fn from(t: core::VersionTag) -> Self {
        Self {
            id: t.id(),
            dataset_id: t.dataset_id(),
            name: t.name().to_string(),
            serial: t.serial(),
            description: t.description().to_string(),
            created_by: t.created_by().to_string(),
            created_at: t.created_at().to_rfc3339(),
            image_count: t.image_count(),
            annotation_counts: t.annotation_counts().clone(),
            sensor_counts: t.sensor_counts().clone(),
            label_count: t.label_count(),
            annotation_set_count: t.annotation_set_count(),
            snapshot_id: t.snapshot_id(),
        }
    }
}

/// A single entry in the dataset changelog.
#[derive(uniffi::Record, Clone, Debug)]
pub struct ChangelogEntry {
    pub id: u64,
    pub dataset_id: u64,
    pub serial: u64,
    pub entity_type: String,
    pub operation: String,
    pub entity_id: Option<u64>,
    pub change_data: String,
    pub username: String,
    pub organization_id: u64,
    pub created_at: String,
    pub message: String,
    pub s3_version_ids: Vec<String>,
}

impl From<core::ChangelogEntry> for ChangelogEntry {
    fn from(e: core::ChangelogEntry) -> Self {
        Self {
            id: e.id(),
            dataset_id: e.dataset_id(),
            serial: e.serial(),
            entity_type: e.entity_type().to_string(),
            operation: e.operation().to_string(),
            entity_id: e.entity_id(),
            change_data: serde_json::to_string(e.change_data()).unwrap_or_default(),
            username: e.username().to_string(),
            organization_id: e.organization_id(),
            created_at: e.created_at().to_rfc3339(),
            message: e.message().to_string(),
            s3_version_ids: e
                .s3_version_ids()
                .iter()
                .map(|v| serde_json::to_string(v).unwrap_or_default())
                .collect(),
        }
    }
}

/// Paginated changelog response.
#[derive(uniffi::Record, Clone, Debug)]
pub struct ChangelogResponse {
    pub entries: Vec<ChangelogEntry>,
    pub count: u64,
    pub continue_token: String,
    pub from_serial: Option<u64>,
    pub to_serial: Option<u64>,
}

impl From<core::ChangelogResponse> for ChangelogResponse {
    fn from(r: core::ChangelogResponse) -> Self {
        Self {
            entries: r.entries.into_iter().map(ChangelogEntry::from).collect(),
            count: r.count,
            continue_token: r.continue_token,
            from_serial: r.from_serial,
            to_serial: r.to_serial,
        }
    }
}

/// Cached metrics summary for a dataset's current state.
#[derive(uniffi::Record, Clone, Debug)]
pub struct DatasetSummary {
    pub dataset_id: u64,
    pub current_serial: u64,
    pub image_count: u64,
    pub annotation_counts: HashMap<String, u64>,
    pub sensor_counts: HashMap<String, u64>,
    pub label_count: u64,
    pub annotation_set_count: u64,
    pub last_updated: String,
}

impl From<core::DatasetSummary> for DatasetSummary {
    fn from(s: core::DatasetSummary) -> Self {
        Self {
            dataset_id: s.dataset_id(),
            current_serial: s.current_serial(),
            image_count: s.image_count(),
            annotation_counts: s.annotation_counts().clone(),
            sensor_counts: s.sensor_counts().clone(),
            label_count: s.label_count(),
            annotation_set_count: s.annotation_set_count(),
            last_updated: s.last_updated().to_rfc3339(),
        }
    }
}

/// Current version information for a dataset.
#[derive(uniffi::Record, Clone, Debug)]
pub struct VersionCurrentResponse {
    pub dataset_id: u64,
    pub current_serial: u64,
    pub latest_tag: Option<VersionTag>,
    pub tags: Vec<VersionTag>,
    pub summary: Option<DatasetSummary>,
}

impl From<core::VersionCurrentResponse> for VersionCurrentResponse {
    fn from(r: core::VersionCurrentResponse) -> Self {
        Self {
            dataset_id: r.dataset_id,
            current_serial: r.current_serial,
            latest_tag: r.latest_tag.map(VersionTag::from),
            tags: r.tags.into_iter().map(VersionTag::from).collect(),
            summary: r.summary.map(DatasetSummary::from),
        }
    }
}

/// Source tag information in a restore result.
#[derive(uniffi::Record, Clone, Debug)]
pub struct RestoredFrom {
    pub tag: String,
    pub serial: u64,
}

impl From<core::RestoredFrom> for RestoredFrom {
    fn from(r: core::RestoredFrom) -> Self {
        Self {
            tag: r.tag,
            serial: r.serial,
        }
    }
}

/// Counts of entities restored.
#[derive(uniffi::Record, Clone, Debug)]
pub struct RestoredCounts {
    pub images: u64,
    pub labels: u64,
    pub annotation_sets: u64,
}

impl From<core::RestoredCounts> for RestoredCounts {
    fn from(r: core::RestoredCounts) -> Self {
        Self {
            images: r.images,
            labels: r.labels,
            annotation_sets: r.annotation_sets,
        }
    }
}

/// Result from restoring a dataset to a version tag.
#[derive(uniffi::Record, Clone, Debug)]
pub struct RestoreResult {
    pub success: bool,
    pub new_serial: u64,
    pub restored_from: RestoredFrom,
    pub restored_counts: RestoredCounts,
    pub message: String,
}

impl From<core::RestoreResult> for RestoreResult {
    fn from(r: core::RestoreResult) -> Self {
        Self {
            success: r.success,
            new_serial: r.new_serial,
            restored_from: RestoredFrom::from(r.restored_from),
            restored_counts: RestoredCounts::from(r.restored_counts),
            message: r.message,
        }
    }
}

/// A 2D point (x, y coordinates).
#[derive(uniffi::Record, Clone, Debug)]
pub struct Point2d {
    pub x: f32,
    pub y: f32,
}

/// A polygon ring as a list of 2D points.
#[derive(uniffi::Record, Clone, Debug)]
pub struct PolygonRing {
    pub points: Vec<Point2d>,
}

/// Segmentation mask as a list of polygon rings.
///
/// Each ring is a closed polygon defined by a sequence of (x, y) coordinates.
/// Multiple rings allow for complex shapes with holes.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Polygon {
    pub rings: Vec<PolygonRing>,
}

impl From<core::Polygon> for Polygon {
    fn from(p: core::Polygon) -> Self {
        Self {
            rings: p
                .rings
                .into_iter()
                .map(|ring| PolygonRing {
                    points: ring.into_iter().map(|(x, y)| Point2d { x, y }).collect(),
                })
                .collect(),
        }
    }
}

impl From<Polygon> for core::Polygon {
    fn from(p: Polygon) -> Self {
        core::Polygon::new(
            p.rings
                .into_iter()
                .map(|ring| ring.points.into_iter().map(|p| (p.x, p.y)).collect())
                .collect(),
        )
    }
}

/// A file associated with a sample (e.g., LiDAR point cloud, radar data).
#[derive(uniffi::Record, Clone, Debug)]
pub struct SampleFile {
    /// File type identifier (e.g., "lidar_pcd", "radar_cube").
    pub file_type: String,
    /// URL to download the file (present for retrieved samples).
    pub url: Option<String>,
    /// Local filename (used when populating samples).
    pub filename: Option<String>,
}

impl From<core::SampleFile> for SampleFile {
    fn from(f: core::SampleFile) -> Self {
        Self {
            file_type: f.file_type().to_string(),
            url: f.url().map(|s| s.to_string()),
            filename: f.filename().map(|s| s.to_string()),
        }
    }
}

impl From<SampleFile> for core::SampleFile {
    fn from(f: SampleFile) -> Self {
        if let Some(url) = f.url {
            core::SampleFile::with_url(f.file_type, url)
        } else if let Some(filename) = f.filename {
            core::SampleFile::with_filename(f.file_type, filename)
        } else {
            // Default to empty filename
            core::SampleFile::with_filename(f.file_type, String::new())
        }
    }
}

/// Location and pose information for a sample.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Location {
    /// GPS coordinates (latitude, longitude).
    pub gps: Option<GpsData>,
    /// IMU orientation (roll, pitch, yaw).
    pub imu: Option<ImuData>,
}

impl From<core::Location> for Location {
    fn from(l: core::Location) -> Self {
        Self {
            gps: l.gps.map(GpsData::from),
            imu: l.imu.map(ImuData::from),
        }
    }
}

impl From<Location> for core::Location {
    fn from(l: Location) -> Self {
        core::Location {
            gps: l.gps.map(|g| core::GpsData {
                lat: g.lat,
                lon: g.lon,
            }),
            imu: l.imu.map(|i| core::ImuData {
                roll: i.roll,
                pitch: i.pitch,
                yaw: i.yaw,
            }),
        }
    }
}

/// An annotation on a sample (bounding box, mask, etc.).
#[derive(uniffi::Record, Clone, Debug)]
pub struct Annotation {
    /// Sample this annotation belongs to.
    pub sample_id: Option<SampleId>,
    /// Image/sample name.
    pub name: Option<String>,
    /// Sequence this annotation belongs to.
    pub sequence_name: Option<String>,
    /// Frame number within the sequence.
    pub frame_number: Option<u32>,
    /// Dataset split (train, val, test).
    pub group: Option<String>,
    /// Object tracking identifier across frames.
    pub object_id: Option<String>,
    /// Label/class name.
    pub label_name: Option<String>,
    /// Label/class index.
    pub label_index: Option<u64>,
    /// Whether this annotation marks a crowd region (COCO `iscrowd`).
    pub iscrowd: Option<bool>,
    /// 2D bounding box.
    pub box2d: Option<Box2d>,
    /// 3D bounding box.
    pub box3d: Option<Box3d>,
    /// Polygon contours.
    pub polygon: Option<Polygon>,
    /// Raster mask as raw PNG bytes.
    pub mask: Option<Vec<u8>>,
    /// Confidence score for the 2D bounding box prediction.
    pub box2d_score: Option<f32>,
    /// Confidence score for the 3D bounding box prediction.
    pub box3d_score: Option<f32>,
    /// Confidence score for the polygon prediction.
    pub polygon_score: Option<f32>,
    /// Confidence score for the mask prediction.
    pub mask_score: Option<f32>,
}

impl From<core::Annotation> for Annotation {
    fn from(a: core::Annotation) -> Self {
        Self {
            sample_id: a.sample_id().map(SampleId::from),
            name: a.name().cloned(),
            sequence_name: a.sequence_name().cloned(),
            frame_number: a.frame_number(),
            group: a.group().cloned(),
            object_id: a.object_id().cloned(),
            label_name: a.label().cloned(),
            label_index: a.label_index(),
            iscrowd: a.iscrowd(),
            box2d: a.box2d().map(|b| Box2d::from(b.clone())),
            box3d: a.box3d().map(|b| Box3d::from(b.clone())),
            polygon: a.polygon().map(|p| Polygon::from(p.clone())),
            mask: a.mask().map(|m| m.as_bytes().to_vec()),
            box2d_score: a.box2d_score(),
            box3d_score: a.box3d_score(),
            polygon_score: a.polygon_score(),
            mask_score: a.mask_score(),
        }
    }
}

impl TryFrom<Annotation> for core::Annotation {
    type Error = ClientError;

    fn try_from(a: Annotation) -> Result<Self, Self::Error> {
        let mut ann = core::Annotation::new();
        ann.set_sample_id(a.sample_id.map(core::SampleID::from));
        ann.set_name(a.name);
        if let Some(seq) = a.sequence_name {
            ann.set_sequence_name(Some(seq));
        }
        if let Some(frame) = a.frame_number {
            ann.set_frame_number(Some(frame));
        }
        if let Some(group) = a.group {
            ann.set_group(Some(group));
        }
        ann.set_object_id(a.object_id);
        ann.set_label(a.label_name);
        if let Some(idx) = a.label_index {
            ann.set_label_index(Some(idx));
        }
        ann.set_iscrowd(a.iscrowd);
        ann.set_box2d(a.box2d.map(core::Box2d::from));
        ann.set_box3d(a.box3d.map(core::Box3d::from));
        ann.set_polygon(a.polygon.map(core::Polygon::from));
        if let Some(bytes) = a.mask {
            let mask = core::MaskData::from_png_checked(bytes).map_err(|e| {
                ClientError::InvalidParameters {
                    message: format!("Invalid PNG mask data: {e}"),
                }
            })?;
            ann.set_mask(Some(mask));
        }
        ann.set_box2d_score(a.box2d_score);
        ann.set_box3d_score(a.box3d_score);
        ann.set_polygon_score(a.polygon_score);
        ann.set_mask_score(a.mask_score);
        Ok(ann)
    }
}

/// Validate an FFI annotation, returning an error if mask data is invalid.
///
/// Swift/Kotlin callers should use this function to validate annotations
/// with mask data before passing them to API methods.
#[uniffi::export]
pub fn validate_annotation(annotation: &Annotation) -> Result<(), ClientError> {
    if let Some(ref bytes) = annotation.mask {
        core::MaskData::from_png_checked(bytes.clone()).map_err(|e| {
            ClientError::InvalidParameters {
                message: format!("Invalid PNG mask data: {e}"),
            }
        })?;
    }
    Ok(())
}

/// Pipeline timing measurements for a sample, in nanoseconds.
///
/// Each field records the wall-clock duration of one pipeline stage.
#[derive(uniffi::Record, Clone, Debug)]
pub struct Timing {
    /// Duration of the data-loading stage (nanoseconds).
    pub load: Option<i64>,
    /// Duration of the preprocessing stage (nanoseconds).
    pub preprocess: Option<i64>,
    /// Duration of the inference stage (nanoseconds).
    pub inference: Option<i64>,
    /// Duration of the decoding / postprocessing stage (nanoseconds).
    pub decode: Option<i64>,
}

impl From<core::Timing> for Timing {
    fn from(t: core::Timing) -> Self {
        Self {
            load: t.load,
            preprocess: t.preprocess,
            inference: t.inference,
            decode: t.decode,
        }
    }
}

impl From<Timing> for core::Timing {
    fn from(t: Timing) -> Self {
        core::Timing {
            load: t.load,
            preprocess: t.preprocess,
            inference: t.inference,
            decode: t.decode,
        }
    }
}

/// A sample in a dataset (image with metadata and annotations).
#[derive(uniffi::Record, Clone, Debug)]
pub struct Sample {
    /// Unique sample identifier.
    pub id: Option<SampleId>,
    /// Dataset split (train, val, test).
    pub group: Option<String>,
    /// Sequence name for video/temporal data.
    pub sequence_name: Option<String>,
    /// Sequence UUID.
    pub sequence_uuid: Option<String>,
    /// Sequence description.
    pub sequence_description: Option<String>,
    /// Frame number within the sequence.
    pub frame_number: Option<u32>,
    /// Sample UUID.
    pub uuid: Option<String>,
    /// Primary image filename.
    pub image_name: Option<String>,
    /// URL to download the primary image.
    pub image_url: Option<String>,
    /// Image width in pixels.
    pub width: Option<u32>,
    /// Image height in pixels.
    pub height: Option<u32>,
    /// Capture date/time (ISO 8601 format).
    pub date: Option<String>,
    /// Data source identifier.
    pub source: Option<String>,
    /// Camera location and pose.
    pub location: Option<Location>,
    /// Image degradation type (blur, occlusion, weather, etc.).
    pub degradation: Option<String>,
    /// Additional sensor files (LiDAR, radar, etc.).
    pub files: Vec<SampleFile>,
    /// Annotations on this sample.
    pub annotations: Vec<Annotation>,
    /// Pipeline timing measurements (nanoseconds per stage).
    pub timing: Option<Timing>,
}

impl From<core::Sample> for Sample {
    fn from(s: core::Sample) -> Self {
        let timing = s.timing.clone().map(Timing::from);
        Self {
            id: s.id().map(SampleId::from),
            group: s.group().cloned(),
            sequence_name: s.sequence_name().cloned(),
            sequence_uuid: s.sequence_uuid().cloned(),
            sequence_description: s.sequence_description().cloned(),
            frame_number: s.frame_number(),
            uuid: s.uuid().cloned(),
            image_name: s.image_name().map(|s| s.to_string()),
            image_url: s.image_url().map(|s| s.to_string()),
            width: s.width(),
            height: s.height(),
            date: s.date().map(|d| d.to_rfc3339()),
            source: s.source().cloned(),
            location: s.location().map(|l| Location::from(l.clone())),
            degradation: s.degradation.clone(),
            files: s.files().iter().cloned().map(SampleFile::from).collect(),
            annotations: s
                .annotations()
                .iter()
                .cloned()
                .map(Annotation::from)
                .collect(),
            timing,
        }
    }
}

// =============================================================================
// Factory Functions
// =============================================================================

/// Create a new client with custom token storage.
///
/// Use this to provide platform-specific secure storage implementations.
///
/// # Example (Kotlin)
///
/// ```kotlin
/// class SecureStorage : TokenStorage {
///     override fun store(token: String) { /* ... */ }
///     override fun load(): String? { /* ... */ }
///     override fun clear() { /* ... */ }
/// }
///
/// val client = createClientWithStorage(SecureStorage())
/// ```
#[uniffi::export]
pub fn create_client_with_storage(
    storage: Box<dyn TokenStorage>,
) -> Result<Arc<Client>, ClientError> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| ClientError::InternalError {
        message: e.to_string(),
    })?;
    let bridge: Arc<dyn core::TokenStorage> = Arc::new(FfiTokenStorageBridge {
        inner: Arc::from(storage),
    });
    let inner = core::Client::new()?.with_storage(bridge);
    Ok(Arc::new(Client { inner, runtime }))
}

// =============================================================================
// Client Object
// =============================================================================

/// Main client for interacting with EdgeFirst Studio.
#[derive(uniffi::Object)]
pub struct Client {
    inner: core::Client,
    runtime: tokio::runtime::Runtime,
}

#[uniffi::export]
impl Client {
    /// Create a new client with default file token storage.
    #[uniffi::constructor]
    pub fn new() -> Result<Arc<Self>, ClientError> {
        let runtime = tokio::runtime::Runtime::new().map_err(|e| ClientError::InternalError {
            message: e.to_string(),
        })?;
        let inner = core::Client::new()?;
        Ok(Arc::new(Self { inner, runtime }))
    }

    /// Create a new client with in-memory token storage (no persistence).
    #[uniffi::constructor]
    pub fn with_memory_storage() -> Result<Arc<Self>, ClientError> {
        let runtime = tokio::runtime::Runtime::new().map_err(|e| ClientError::InternalError {
            message: e.to_string(),
        })?;
        let inner = core::Client::new()?.with_memory_storage();
        Ok(Arc::new(Self { inner, runtime }))
    }

    /// Returns a new client connected to the specified server instance.
    ///
    /// Server names: "" or "saas" → production, "test", "stage", "dev", or
    /// custom.
    pub fn with_server(self: Arc<Self>, name: String) -> Result<Arc<Self>, ClientError> {
        let inner = self.inner.with_server(&name)?;
        Ok(Arc::new(Self {
            inner,
            runtime: tokio::runtime::Runtime::new().map_err(|e| ClientError::InternalError {
                message: e.to_string(),
            })?,
        }))
    }

    /// Returns a new client with the specified authentication token.
    pub fn with_token(self: Arc<Self>, token: String) -> Result<Arc<Self>, ClientError> {
        let inner = self.inner.with_token(&token)?;
        Ok(Arc::new(Self {
            inner,
            runtime: tokio::runtime::Runtime::new().map_err(|e| ClientError::InternalError {
                message: e.to_string(),
            })?,
        }))
    }

    /// Authenticate with username and password (blocking).
    pub fn with_login(
        self: Arc<Self>,
        username: String,
        password: String,
    ) -> Result<Arc<Self>, ClientError> {
        let inner = self
            .runtime
            .block_on(self.inner.with_login(&username, &password))?;
        Ok(Arc::new(Self {
            inner,
            runtime: tokio::runtime::Runtime::new().map_err(|e| ClientError::InternalError {
                message: e.to_string(),
            })?,
        }))
    }

    /// Clear authentication token and log out.
    pub fn logout(&self) -> Result<(), ClientError> {
        self.runtime.block_on(self.inner.logout())?;
        Ok(())
    }

    /// Verify that the current token is valid.
    pub fn verify_token(&self) -> Result<(), ClientError> {
        self.runtime.block_on(self.inner.verify_token())?;
        Ok(())
    }

    /// Get the current server URL.
    pub fn url(&self) -> String {
        self.inner.url().to_string()
    }

    // =========================================================================
    // Organization & Projects
    // =========================================================================

    /// Get the current user's organization.
    pub fn organization(&self) -> Result<Organization, ClientError> {
        let org = self.runtime.block_on(self.inner.organization())?;
        Ok(org.into())
    }

    /// List projects, optionally filtered by name.
    pub fn projects(&self, name: Option<String>) -> Result<Vec<Project>, ClientError> {
        let projects = self
            .runtime
            .block_on(self.inner.projects(name.as_deref()))?;
        Ok(projects.into_iter().map(Project::from).collect())
    }

    /// Get a project by ID.
    pub fn project(&self, id: ProjectId) -> Result<Project, ClientError> {
        let project = self.runtime.block_on(self.inner.project(id.into()))?;
        Ok(project.into())
    }

    // =========================================================================
    // Datasets
    // =========================================================================

    /// List datasets in a project, optionally filtered by name.
    pub fn datasets(
        &self,
        project_id: ProjectId,
        name: Option<String>,
    ) -> Result<Vec<Dataset>, ClientError> {
        let datasets = self
            .runtime
            .block_on(self.inner.datasets(project_id.into(), name.as_deref()))?;
        Ok(datasets.into_iter().map(Dataset::from).collect())
    }

    /// Get a dataset by ID.
    pub fn dataset(&self, id: DatasetId) -> Result<Dataset, ClientError> {
        let dataset = self.runtime.block_on(self.inner.dataset(id.into()))?;
        Ok(dataset.into())
    }

    /// Get annotation sets for a dataset, optionally at a specific version.
    pub fn annotation_sets(
        &self,
        dataset_id: DatasetId,
        version: Option<String>,
    ) -> Result<Vec<AnnotationSet>, ClientError> {
        let sets = self.runtime.block_on(
            self.inner
                .annotation_sets(dataset_id.into(), version.as_deref()),
        )?;
        Ok(sets.into_iter().map(AnnotationSet::from).collect())
    }

    /// Get labels for a dataset, optionally at a specific version.
    pub fn labels(
        &self,
        dataset_id: DatasetId,
        version: Option<String>,
    ) -> Result<Vec<Label>, ClientError> {
        let labels = self
            .runtime
            .block_on(self.inner.labels(dataset_id.into(), version.as_deref()))?;
        Ok(labels.into_iter().map(Label::from).collect())
    }

    // =========================================================================
    // Experiments
    // =========================================================================

    /// List experiments in a project, optionally filtered by name.
    pub fn experiments(
        &self,
        project_id: ProjectId,
        name: Option<String>,
    ) -> Result<Vec<Experiment>, ClientError> {
        let experiments = self
            .runtime
            .block_on(self.inner.experiments(project_id.into(), name.as_deref()))?;
        Ok(experiments.into_iter().map(Experiment::from).collect())
    }

    /// Get an experiment by ID.
    pub fn experiment(&self, id: ExperimentId) -> Result<Experiment, ClientError> {
        let experiment = self.runtime.block_on(self.inner.experiment(id.into()))?;
        Ok(experiment.into())
    }

    // =========================================================================
    // Training Sessions
    // =========================================================================

    /// List training sessions in an experiment, optionally filtered by name.
    pub fn training_sessions(
        &self,
        experiment_id: ExperimentId,
        name: Option<String>,
    ) -> Result<Vec<TrainingSession>, ClientError> {
        let sessions = self.runtime.block_on(
            self.inner
                .training_sessions(experiment_id.into(), name.as_deref()),
        )?;
        Ok(sessions.into_iter().map(TrainingSession::from).collect())
    }

    /// Get a training session by ID.
    pub fn training_session(&self, id: TrainingSessionId) -> Result<TrainingSession, ClientError> {
        let session = self
            .runtime
            .block_on(self.inner.training_session(id.into()))?;
        Ok(session.into())
    }

    /// Get artifacts for a training session.
    pub fn artifacts(
        &self,
        training_session_id: TrainingSessionId,
    ) -> Result<Vec<Artifact>, ClientError> {
        let artifacts = self
            .runtime
            .block_on(self.inner.artifacts(training_session_id.into()))?;
        Ok(artifacts.into_iter().map(Artifact::from).collect())
    }

    // =========================================================================
    // Validation Sessions
    // =========================================================================

    /// List validation sessions for a project.
    pub fn validation_sessions(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<ValidationSession>, ClientError> {
        let sessions = self
            .runtime
            .block_on(self.inner.validation_sessions(project_id.into()))?;
        Ok(sessions.into_iter().map(ValidationSession::from).collect())
    }

    // =========================================================================
    // Snapshots
    // =========================================================================

    /// List snapshots, optionally filtered by name.
    pub fn snapshots(&self, name: Option<String>) -> Result<Vec<Snapshot>, ClientError> {
        let snapshots = self
            .runtime
            .block_on(self.inner.snapshots(name.as_deref()))?;
        Ok(snapshots.into_iter().map(Snapshot::from).collect())
    }

    /// Get a snapshot by ID.
    pub fn snapshot(&self, id: SnapshotId) -> Result<Snapshot, ClientError> {
        let snapshot = self.runtime.block_on(self.inner.snapshot(id.into()))?;
        Ok(snapshot.into())
    }

    // =========================================================================
    // Tasks
    // =========================================================================

    /// Get task information by ID.
    pub fn task_info(&self, id: TaskId) -> Result<TaskInfo, ClientError> {
        let info = self.runtime.block_on(self.inner.task_info(id.into()))?;
        Ok(info.into())
    }

    // =========================================================================
    // Dataset Versioning
    // =========================================================================

    /// Create a new version tag for a dataset.
    pub fn version_tag_create(
        &self,
        dataset_id: DatasetId,
        name: String,
        description: Option<String>,
    ) -> Result<VersionTag, ClientError> {
        let tag = self.runtime.block_on(self.inner.version_tag_create(
            dataset_id.into(),
            &name,
            description.as_deref(),
        ))?;
        Ok(tag.into())
    }

    /// Get a specific version tag by name.
    pub fn version_tag_get(
        &self,
        dataset_id: DatasetId,
        name: String,
    ) -> Result<VersionTag, ClientError> {
        let tag = self
            .runtime
            .block_on(self.inner.version_tag_get(dataset_id.into(), &name))?;
        Ok(tag.into())
    }

    /// List all version tags for a dataset.
    pub fn version_tag_list(&self, dataset_id: DatasetId) -> Result<Vec<VersionTag>, ClientError> {
        let tags = self
            .runtime
            .block_on(self.inner.version_tag_list(dataset_id.into()))?;
        Ok(tags.into_iter().map(VersionTag::from).collect())
    }

    /// Delete a version tag from a dataset.
    pub fn version_tag_delete(
        &self,
        dataset_id: DatasetId,
        name: String,
    ) -> Result<String, ClientError> {
        let result = self
            .runtime
            .block_on(self.inner.version_tag_delete(dataset_id.into(), &name))?;
        Ok(result)
    }

    /// Restore a dataset to the state at a specific version tag.
    pub fn version_tag_restore(
        &self,
        dataset_id: DatasetId,
        name: String,
    ) -> Result<RestoreResult, ClientError> {
        let result = self
            .runtime
            .block_on(self.inner.version_tag_restore(dataset_id.into(), &name))?;
        Ok(result.into())
    }

    /// Get the changelog for a dataset between two versions.
    pub fn version_changelog(
        &self,
        dataset_id: DatasetId,
        from_version: Option<String>,
        to_version: Option<String>,
        entity_types: Option<Vec<String>>,
        limit: Option<u64>,
        continue_token: Option<String>,
    ) -> Result<ChangelogResponse, ClientError> {
        let result = self.runtime.block_on(self.inner.version_changelog(
            dataset_id.into(),
            from_version.as_deref(),
            to_version.as_deref(),
            entity_types.as_deref(),
            limit,
            continue_token.as_deref(),
        ))?;
        Ok(result.into())
    }

    /// Get the count of changelog entries between two versions.
    pub fn version_changelog_count(
        &self,
        dataset_id: DatasetId,
        from_version: Option<String>,
        to_version: Option<String>,
        entity_types: Option<Vec<String>>,
    ) -> Result<u64, ClientError> {
        let count = self.runtime.block_on(self.inner.version_changelog_count(
            dataset_id.into(),
            from_version.as_deref(),
            to_version.as_deref(),
            entity_types.as_deref(),
        ))?;
        Ok(count)
    }

    /// Get the current version information for a dataset.
    pub fn version_current(
        &self,
        dataset_id: DatasetId,
    ) -> Result<VersionCurrentResponse, ClientError> {
        let result = self
            .runtime
            .block_on(self.inner.version_current(dataset_id.into()))?;
        Ok(result.into())
    }

    /// Get the version summary for a dataset.
    pub fn version_summary(&self, dataset_id: DatasetId) -> Result<DatasetSummary, ClientError> {
        let summary = self
            .runtime
            .block_on(self.inner.version_summary(dataset_id.into()))?;
        Ok(summary.into())
    }

    /// Recalculate the version summary for a dataset.
    pub fn version_summary_recalculate(
        &self,
        dataset_id: DatasetId,
    ) -> Result<DatasetSummary, ClientError> {
        let summary = self
            .runtime
            .block_on(self.inner.version_summary_recalculate(dataset_id.into()))?;
        Ok(summary.into())
    }
}

// =============================================================================
// Async Methods (for Swift async/await and Kotlin coroutines)
// =============================================================================

#[uniffi::export]
impl Client {
    /// Authenticate with username and password (async).
    ///
    /// Uses `async-compat` to enter Tokio context for reqwest compatibility
    /// while allowing UniFFI to drive the future from Swift/Kotlin.
    pub async fn with_login_async(
        self: Arc<Self>,
        username: String,
        password: String,
    ) -> Result<Arc<Self>, ClientError> {
        async {
            let inner = self.inner.with_login(&username, &password).await?;
            Ok(Arc::new(Self {
                inner,
                runtime: tokio::runtime::Runtime::new().map_err(|e| {
                    ClientError::InternalError {
                        message: e.to_string(),
                    }
                })?,
            }))
        }
        .compat()
        .await
    }

    /// Get the current user's organization (async).
    pub async fn organization_async(&self) -> Result<Organization, ClientError> {
        async {
            let org = self.inner.organization().await?;
            Ok(org.into())
        }
        .compat()
        .await
    }

    /// List projects, optionally filtered by name (async).
    pub async fn projects_async(&self, name: Option<String>) -> Result<Vec<Project>, ClientError> {
        async {
            let projects = self.inner.projects(name.as_deref()).await?;
            Ok(projects.into_iter().map(Project::from).collect())
        }
        .compat()
        .await
    }

    /// Get a project by ID (async).
    pub async fn project_async(&self, id: ProjectId) -> Result<Project, ClientError> {
        async {
            let project = self.inner.project(id.into()).await?;
            Ok(project.into())
        }
        .compat()
        .await
    }

    /// List datasets in a project (async).
    pub async fn datasets_async(
        &self,
        project_id: ProjectId,
        name: Option<String>,
    ) -> Result<Vec<Dataset>, ClientError> {
        async {
            let datasets = self
                .inner
                .datasets(project_id.into(), name.as_deref())
                .await?;
            Ok(datasets.into_iter().map(Dataset::from).collect())
        }
        .compat()
        .await
    }

    /// Get a dataset by ID (async).
    pub async fn dataset_async(&self, id: DatasetId) -> Result<Dataset, ClientError> {
        async {
            let dataset = self.inner.dataset(id.into()).await?;
            Ok(dataset.into())
        }
        .compat()
        .await
    }

    /// Get annotation sets for a dataset, optionally at a specific version (async).
    pub async fn annotation_sets_async(
        &self,
        dataset_id: DatasetId,
        version: Option<String>,
    ) -> Result<Vec<AnnotationSet>, ClientError> {
        async {
            let sets = self
                .inner
                .annotation_sets(dataset_id.into(), version.as_deref())
                .await?;
            Ok(sets.into_iter().map(AnnotationSet::from).collect())
        }
        .compat()
        .await
    }

    /// Get labels for a dataset, optionally at a specific version (async).
    pub async fn labels_async(
        &self,
        dataset_id: DatasetId,
        version: Option<String>,
    ) -> Result<Vec<Label>, ClientError> {
        async {
            let labels = self
                .inner
                .labels(dataset_id.into(), version.as_deref())
                .await?;
            Ok(labels.into_iter().map(Label::from).collect())
        }
        .compat()
        .await
    }

    /// List experiments in a project (async).
    pub async fn experiments_async(
        &self,
        project_id: ProjectId,
        name: Option<String>,
    ) -> Result<Vec<Experiment>, ClientError> {
        async {
            let experiments = self
                .inner
                .experiments(project_id.into(), name.as_deref())
                .await?;
            Ok(experiments.into_iter().map(Experiment::from).collect())
        }
        .compat()
        .await
    }

    /// Get an experiment by ID (async).
    pub async fn experiment_async(&self, id: ExperimentId) -> Result<Experiment, ClientError> {
        async {
            let experiment = self.inner.experiment(id.into()).await?;
            Ok(experiment.into())
        }
        .compat()
        .await
    }

    /// List training sessions in an experiment (async).
    pub async fn training_sessions_async(
        &self,
        experiment_id: ExperimentId,
        name: Option<String>,
    ) -> Result<Vec<TrainingSession>, ClientError> {
        async {
            let sessions = self
                .inner
                .training_sessions(experiment_id.into(), name.as_deref())
                .await?;
            Ok(sessions.into_iter().map(TrainingSession::from).collect())
        }
        .compat()
        .await
    }

    /// Get a training session by ID (async).
    pub async fn training_session_async(
        &self,
        id: TrainingSessionId,
    ) -> Result<TrainingSession, ClientError> {
        async {
            let session = self.inner.training_session(id.into()).await?;
            Ok(session.into())
        }
        .compat()
        .await
    }

    /// Get artifacts for a training session (async).
    pub async fn artifacts_async(
        &self,
        training_session_id: TrainingSessionId,
    ) -> Result<Vec<Artifact>, ClientError> {
        async {
            let artifacts = self.inner.artifacts(training_session_id.into()).await?;
            Ok(artifacts.into_iter().map(Artifact::from).collect())
        }
        .compat()
        .await
    }

    /// List validation sessions for a project (async).
    pub async fn validation_sessions_async(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<ValidationSession>, ClientError> {
        async {
            let sessions = self.inner.validation_sessions(project_id.into()).await?;
            Ok(sessions.into_iter().map(ValidationSession::from).collect())
        }
        .compat()
        .await
    }

    /// List snapshots, optionally filtered by name (async).
    pub async fn snapshots_async(
        &self,
        name: Option<String>,
    ) -> Result<Vec<Snapshot>, ClientError> {
        async {
            let snapshots = self.inner.snapshots(name.as_deref()).await?;
            Ok(snapshots.into_iter().map(Snapshot::from).collect())
        }
        .compat()
        .await
    }

    /// Get a snapshot by ID (async).
    pub async fn snapshot_async(&self, id: SnapshotId) -> Result<Snapshot, ClientError> {
        async {
            let snapshot = self.inner.snapshot(id.into()).await?;
            Ok(snapshot.into())
        }
        .compat()
        .await
    }

    /// Get task information by ID (async).
    pub async fn task_info_async(&self, id: TaskId) -> Result<TaskInfo, ClientError> {
        async {
            let info = self.inner.task_info(id.into()).await?;
            Ok(info.into())
        }
        .compat()
        .await
    }

    /// Verify that the current token is valid (async).
    pub async fn verify_token_async(&self) -> Result<(), ClientError> {
        async {
            self.inner.verify_token().await?;
            Ok(())
        }
        .compat()
        .await
    }

    /// Clear authentication token and log out (async).
    pub async fn logout_async(&self) -> Result<(), ClientError> {
        async {
            self.inner.logout().await?;
            Ok(())
        }
        .compat()
        .await
    }

    // =========================================================================
    // Dataset Versioning (async)
    // =========================================================================

    /// Create a new version tag for a dataset (async).
    pub async fn version_tag_create_async(
        &self,
        dataset_id: DatasetId,
        name: String,
        description: Option<String>,
    ) -> Result<VersionTag, ClientError> {
        async {
            let tag = self
                .inner
                .version_tag_create(dataset_id.into(), &name, description.as_deref())
                .await?;
            Ok(tag.into())
        }
        .compat()
        .await
    }

    /// Get a specific version tag by name (async).
    pub async fn version_tag_get_async(
        &self,
        dataset_id: DatasetId,
        name: String,
    ) -> Result<VersionTag, ClientError> {
        async {
            let tag = self.inner.version_tag_get(dataset_id.into(), &name).await?;
            Ok(tag.into())
        }
        .compat()
        .await
    }

    /// List all version tags for a dataset (async).
    pub async fn version_tag_list_async(
        &self,
        dataset_id: DatasetId,
    ) -> Result<Vec<VersionTag>, ClientError> {
        async {
            let tags = self.inner.version_tag_list(dataset_id.into()).await?;
            Ok(tags.into_iter().map(VersionTag::from).collect())
        }
        .compat()
        .await
    }

    /// Delete a version tag from a dataset (async).
    pub async fn version_tag_delete_async(
        &self,
        dataset_id: DatasetId,
        name: String,
    ) -> Result<String, ClientError> {
        async {
            let result = self
                .inner
                .version_tag_delete(dataset_id.into(), &name)
                .await?;
            Ok(result)
        }
        .compat()
        .await
    }

    /// Restore a dataset to the state at a specific version tag (async).
    pub async fn version_tag_restore_async(
        &self,
        dataset_id: DatasetId,
        name: String,
    ) -> Result<RestoreResult, ClientError> {
        async {
            let result = self
                .inner
                .version_tag_restore(dataset_id.into(), &name)
                .await?;
            Ok(result.into())
        }
        .compat()
        .await
    }

    /// Get the changelog for a dataset between two versions (async).
    pub async fn version_changelog_async(
        &self,
        dataset_id: DatasetId,
        from_version: Option<String>,
        to_version: Option<String>,
        entity_types: Option<Vec<String>>,
        limit: Option<u64>,
        continue_token: Option<String>,
    ) -> Result<ChangelogResponse, ClientError> {
        async {
            let result = self
                .inner
                .version_changelog(
                    dataset_id.into(),
                    from_version.as_deref(),
                    to_version.as_deref(),
                    entity_types.as_deref(),
                    limit,
                    continue_token.as_deref(),
                )
                .await?;
            Ok(result.into())
        }
        .compat()
        .await
    }

    /// Get the count of changelog entries between two versions (async).
    pub async fn version_changelog_count_async(
        &self,
        dataset_id: DatasetId,
        from_version: Option<String>,
        to_version: Option<String>,
        entity_types: Option<Vec<String>>,
    ) -> Result<u64, ClientError> {
        async {
            let count = self
                .inner
                .version_changelog_count(
                    dataset_id.into(),
                    from_version.as_deref(),
                    to_version.as_deref(),
                    entity_types.as_deref(),
                )
                .await?;
            Ok(count)
        }
        .compat()
        .await
    }

    /// Get the current version information for a dataset (async).
    pub async fn version_current_async(
        &self,
        dataset_id: DatasetId,
    ) -> Result<VersionCurrentResponse, ClientError> {
        async {
            let result = self.inner.version_current(dataset_id.into()).await?;
            Ok(result.into())
        }
        .compat()
        .await
    }

    /// Get the version summary for a dataset (async).
    pub async fn version_summary_async(
        &self,
        dataset_id: DatasetId,
    ) -> Result<DatasetSummary, ClientError> {
        async {
            let summary = self.inner.version_summary(dataset_id.into()).await?;
            Ok(summary.into())
        }
        .compat()
        .await
    }

    /// Recalculate the version summary for a dataset (async).
    pub async fn version_summary_recalculate_async(
        &self,
        dataset_id: DatasetId,
    ) -> Result<DatasetSummary, ClientError> {
        async {
            let summary = self
                .inner
                .version_summary_recalculate(dataset_id.into())
                .await?;
            Ok(summary.into())
        }
        .compat()
        .await
    }
}
