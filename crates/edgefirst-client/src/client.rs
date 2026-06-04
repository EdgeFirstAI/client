// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use crate::{
    Annotation, Error, Sample, Task,
    api::{
        AnnotationSetID, Artifact, DatasetID, Experiment, ExperimentID, LoginResult,
        NewValidationSession, Organization, Project, ProjectID, SampleID, SamplesCountResult,
        SamplesListParams, SamplesListResult, Snapshot, SnapshotCreateFromDataset,
        SnapshotFromDatasetResult, SnapshotID, SnapshotRestore, SnapshotRestoreResult, Stage,
        StartValidationRequest, TaskID, TaskInfo, TaskStages, TaskStatus, TasksListParams,
        TasksListResult, TrainingSession, TrainingSessionID, ValidationSession,
        ValidationSessionID,
    },
    dataset::{
        AnnotationSet, AnnotationType, Dataset, FileType, Group, Label, NewLabel, NewLabelObject,
    },
    retry::{create_retry_policy, log_retry_configuration},
    storage::{FileTokenStorage, MemoryTokenStorage, TokenStorage},
};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use futures::{StreamExt as _, future::join_all};
use log::{Level, debug, error, log_enabled, trace, warn};
use reqwest::{Body, header::CONTENT_LENGTH, multipart::Form};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::create_dir_all,
    io::{SeekFrom, Write as _},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
    vec,
};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt as _, AsyncSeekExt as _, AsyncWriteExt as _},
    sync::{RwLock, Semaphore, mpsc::Sender},
};
use tokio_util::codec::{BytesCodec, FramedRead};
use walkdir::WalkDir;

#[cfg(feature = "polars")]
use polars::prelude::*;

/// Maps a JSON-RPC error code to a typed `Error` variant when the code is
/// well-known; otherwise returns `Error::RpcError(code, message)` unchanged.
///
/// Scoped to the new DE-2565 methods. Existing methods continue to return
/// `Error::RpcError` directly.
///
/// Server error codes (from `api.go` via `jrpc.Fail`):
/// - `1`   – generic server error
/// - `3`   – validation / bad request
/// - `10`  – internal server error
/// - `101` – resource not found (e.g. "Cannot find task...", "not found in DB")
/// - `401` – unauthenticated
/// - `403` – forbidden
/// - `413` – payload too large
pub(crate) fn map_rpc_error(
    method: &str,
    code: i32,
    message: String,
    task_id: Option<crate::api::TaskID>,
) -> Error {
    // Server emits "Cannot find task...", "not found in DB", and other phrasings
    // for code 101. Code 101 with a task_id is task-not-found by contract
    // (see api.go), so we return the typed variant unconditionally when the
    // caller supplied a task_id — message phrasing is treated as informational
    // and is preserved by the RPC layer for diagnostic logging upstream.
    if code == 101
        && let Some(id) = task_id
    {
        return Error::TaskNotFound(id);
    }
    match code {
        401 | 403 => Error::PermissionDenied(method.to_string()),
        413 => Error::PayloadTooLarge {
            method: method.to_string(),
            size_hint: None,
        },
        _ => Error::RpcError(code, message),
    }
}

/// Returns true if `val` is structurally a JSON-RPC 2.0 *error* envelope.
///
/// A real envelope must:
/// 1. Be a JSON object,
/// 2. Carry a `"jsonrpc"` member (the protocol-version sentinel — JSON-RPC
///    2.0 §5 mandates this on every response object),
/// 3. Carry an `"error"` object that includes a numeric `"code"` field.
///
/// This is intentionally stricter than a "looks for a top-level `error`
/// key" check so that legitimate JSON file payloads (validation traces,
/// metrics dumps, diagnostics) which happen to include a free-form `error`
/// field are *not* misclassified as RPC failures.
///
/// Extracted so it can be unit-tested without a live server.
pub(crate) fn is_jsonrpc_error_envelope(val: &serde_json::Value) -> bool {
    let Some(obj) = val.as_object() else {
        return false;
    };
    // Protocol-version sentinel — only JSON-RPC envelopes carry this.
    if !obj.contains_key("jsonrpc") {
        return false;
    }
    let Some(err) = obj.get("error").and_then(|e| e.as_object()) else {
        return false;
    };
    err.get("code")
        .map(|c| c.is_i64() || c.is_u64())
        .unwrap_or(false)
}

/// Validates that `group` and `name` are both non-empty strings for chart
/// operations (`add_chart`, `get_chart`). Extracted so it can be unit-tested
/// without a live server.
pub(crate) fn validate_chart_args(group: &str, name: &str) -> Result<(), Error> {
    if group.is_empty() || name.is_empty() {
        return Err(Error::InvalidParameters(
            "chart: group and name must be non-empty".into(),
        ));
    }
    Ok(())
}

static PART_SIZE: usize = 100 * 1024 * 1024;

/// Source for file content during upload - either a local path or raw bytes.
#[derive(Clone)]
enum FileSource {
    /// File content from a local filesystem path.
    Path(PathBuf),
    /// File content as raw bytes (e.g., from a ZIP archive).
    Bytes(Vec<u8>),
}

fn max_tasks() -> usize {
    std::env::var("MAX_TASKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            // Default to half the number of CPUs, minimum 2, maximum 8
            let cpus = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            (cpus / 2).clamp(2, 8)
        })
}

/// Maximum concurrent upload tasks for multipart S3 uploads.
///
/// Higher concurrency improves upload throughput by saturating available
/// bandwidth. Can be overridden via `MAX_UPLOAD_TASKS` environment variable.
fn max_upload_tasks() -> usize {
    std::env::var("MAX_UPLOAD_TASKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8) // Default to 8 concurrent part uploads
}

/// Filters items by name and sorts by match quality.
///
/// Match quality priority (best to worst):
/// 1. Exact match (case-sensitive)
/// 2. Exact match (case-insensitive)
/// 3. Substring match (shorter names first, then alphabetically)
///
/// This ensures that searching for "Deer" returns "Deer" before
/// "Deer Roundtrip 20251129" or "Reindeer".
fn filter_and_sort_by_name<T, F>(items: Vec<T>, filter: &str, get_name: F) -> Vec<T>
where
    F: Fn(&T) -> &str,
{
    let filter_lower = filter.to_lowercase();
    let mut filtered: Vec<T> = items
        .into_iter()
        .filter(|item| get_name(item).to_lowercase().contains(&filter_lower))
        .collect();

    filtered.sort_by(|a, b| {
        let name_a = get_name(a);
        let name_b = get_name(b);

        // Priority 1: Exact match (case-sensitive)
        let exact_a = name_a == filter;
        let exact_b = name_b == filter;
        if exact_a != exact_b {
            return exact_b.cmp(&exact_a); // true (exact) comes first
        }

        // Priority 2: Exact match (case-insensitive)
        let exact_ci_a = name_a.to_lowercase() == filter_lower;
        let exact_ci_b = name_b.to_lowercase() == filter_lower;
        if exact_ci_a != exact_ci_b {
            return exact_ci_b.cmp(&exact_ci_a);
        }

        // Priority 3: Shorter names first (more specific matches)
        let len_cmp = name_a.len().cmp(&name_b.len());
        if len_cmp != std::cmp::Ordering::Equal {
            return len_cmp;
        }

        // Priority 4: Alphabetical order for stability
        name_a.cmp(name_b)
    });

    filtered
}

/// Whether `host` refers to a loopback (machine-local) endpoint.
///
/// Used by [`Client::with_url`] to decide whether a plain-`http://` URL is
/// safe to accept. Loopback traffic never leaves the machine, so the
/// usual concern about leaking the Studio bearer token in plaintext does
/// not apply — that's how wiremock and local dev servers connect.
fn is_loopback_host(host: Option<&url::Host<&str>>) -> bool {
    match host {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        // RFC 6761 reserves "localhost" (and `*.localhost`) as a loopback
        // name. Compare case-insensitively because URL hosts are matched
        // that way and developers do type capitalized variants.
        Some(url::Host::Domain(d)) => {
            d.eq_ignore_ascii_case("localhost") || d.to_ascii_lowercase().ends_with(".localhost")
        }
        None => false,
    }
}

fn sanitize_path_component(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "unnamed".to_string();
    }

    let component = Path::new(trimmed)
        .file_name()
        .unwrap_or_else(|| OsStr::new(trimmed));

    let sanitized: String = component
        .to_string_lossy()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();

    if sanitized.is_empty() {
        "unnamed".to_string()
    } else {
        sanitized
    }
}

/// Progress information for long-running operations.
///
/// This struct tracks the current progress of operations like file uploads,
/// downloads, or dataset processing. It provides the current count, total
/// count, and an optional status string to enable progress reporting in
/// applications.
///
/// # Multi-Stage Progress
///
/// The `status` field enables multi-stage progress tracking. When an operation
/// has multiple phases, the status field changes to indicate the current phase.
/// Applications should detect status changes to reset their progress display.
///
/// # Operation Progress Details
///
/// | Operation | Status | Unit | Notes |
/// |-----------|--------|------|-------|
/// | [`download_dataset`] | `None` then `"Downloading"` | samples | Two phases: fetch metadata, then download files |
/// | [`populate_samples`] | `None` | samples | Each sample may contain multiple files |
/// | [`samples`] | `None` | samples | Paginated API fetch |
/// | [`sample_names`] | `None` | samples | Paginated API fetch, names only |
/// | [`annotations`] | `None` | samples | Samples processed for annotations |
/// | [`download_artifact`] | `None` | bytes | Single file byte-level progress |
/// | [`download_checkpoint`] | `None` | bytes | Single file byte-level progress |
/// | [`download_snapshot`] | `None` | bytes | Combined byte progress across all files |
///
/// [`download_dataset`]: Client::download_dataset
/// [`populate_samples`]: Client::populate_samples
/// [`samples`]: Client::samples
/// [`sample_names`]: Client::sample_names
/// [`annotations`]: Client::annotations
/// [`download_artifact`]: Client::download_artifact
/// [`download_checkpoint`]: Client::download_checkpoint
/// [`download_snapshot`]: Client::download_snapshot
///
/// # Examples
///
/// Basic progress display:
///
/// ```rust
/// use edgefirst_client::Progress;
///
/// let progress = Progress {
///     current: 25,
///     total: 100,
///     status: Some("Downloading".to_string()),
/// };
/// let percentage = (progress.current as f64 / progress.total as f64) * 100.0;
/// println!(
///     "{}: {:.1}% ({}/{})",
///     progress.status.as_deref().unwrap_or("Progress"),
///     percentage,
///     progress.current,
///     progress.total
/// );
/// ```
///
/// Multi-stage progress handling (e.g., for `download_dataset`):
///
/// ```rust,ignore
/// let mut last_status: Option<String> = None;
///
/// while let Some(progress) = rx.recv().await {
///     // Detect stage change and reset progress bar
///     if progress.status != last_status {
///         if let Some(ref status) = progress.status {
///             println!("\n{}", status);
///         }
///         last_status = progress.status.clone();
///     }
///
///     let pct = (progress.current as f64 / progress.total as f64) * 100.0;
///     print!("\r{:.1}% ({}/{})", pct, progress.current, progress.total);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Progress {
    /// Current number of completed items or bytes.
    pub current: usize,
    /// Total number of items or bytes to process.
    pub total: usize,
    /// Optional status describing the current operation phase.
    ///
    /// When this value changes from `None` to `Some(...)` or between different
    /// values, it indicates a new phase has started. Applications should reset
    /// their progress display when the status changes.
    ///
    /// Currently only [`Client::download_dataset`] uses status changes:
    /// - Phase 1: `None` while fetching sample metadata
    /// - Phase 2: `"Downloading"` while downloading files
    ///
    /// All other operations use `None` throughout.
    pub status: Option<String>,
}

#[derive(Serialize)]
struct RpcRequest<Params> {
    id: u64,
    jsonrpc: String,
    method: String,
    params: Option<Params>,
}

impl<T> Default for RpcRequest<T> {
    fn default() -> Self {
        RpcRequest {
            id: 0,
            jsonrpc: "2.0".to_string(),
            method: "".to_string(),
            params: None,
        }
    }
}

#[derive(Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

#[derive(Deserialize)]
struct RpcResponse<RpcResult> {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    jsonrpc: String,
    error: Option<RpcError>,
    result: Option<RpcResult>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct EmptyResult {}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct SnapshotCreateParams {
    snapshot_name: String,
    keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SnapshotCreateResult {
    snapshot_id: SnapshotID,
    urls: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SnapshotCreateMultipartParams {
    snapshot_name: String,
    keys: Vec<String>,
    file_sizes: Vec<usize>,
    /// Optional snapshot type (e.g., "ziparrow" for EdgeFirst Dataset Format)
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    snapshot_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SnapshotCreateMultipartResultField {
    Id(u64),
    Part(SnapshotPart),
}

#[derive(Debug, Serialize)]
struct SnapshotCompleteMultipartParams {
    key: String,
    upload_id: String,
    etag_list: Vec<EtagPart>,
}

#[derive(Debug, Clone, Serialize)]
struct EtagPart {
    #[serde(rename = "ETag")]
    etag: String,
    #[serde(rename = "PartNumber")]
    part_number: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct SnapshotPart {
    key: Option<String>,
    upload_id: String,
    urls: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SnapshotStatusParams {
    snapshot_id: SnapshotID,
    status: String,
}

#[derive(Deserialize, Debug)]
struct SnapshotStatusResult {
    #[allow(dead_code)]
    pub id: SnapshotID,
    #[allow(dead_code)]
    pub uid: String,
    #[allow(dead_code)]
    pub description: String,
    #[allow(dead_code)]
    pub date: String,
    #[allow(dead_code)]
    pub status: String,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct ImageListParams {
    images_filter: ImagesFilter,
    image_files_filter: HashMap<String, String>,
    only_ids: bool,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct ImagesFilter {
    dataset_id: DatasetID,
}

/// Main client for interacting with EdgeFirst Studio Server.
///
/// The EdgeFirst Client handles the connection to the EdgeFirst Studio Server
/// and manages authentication, RPC calls, and data operations. It provides
/// methods for managing projects, datasets, experiments, training sessions,
/// and various utility functions for data processing.
///
/// The client supports multiple authentication methods and can work with both
/// SaaS and self-hosted EdgeFirst Studio instances.
///
/// # Features
///
/// - **Authentication**: Token-based authentication with automatic persistence
/// - **Dataset Management**: Upload, download, and manipulate datasets
/// - **Project Operations**: Create and manage projects and experiments
/// - **Training & Validation**: Submit and monitor ML training jobs
/// - **Data Integration**: Convert between EdgeFirst datasets and popular
///   formats
/// - **Progress Tracking**: Real-time progress updates for long-running
///   operations
///
/// # Examples
///
/// ```no_run
/// use edgefirst_client::{Client, DatasetID};
/// use std::str::FromStr;
///
/// # async fn example() -> Result<(), edgefirst_client::Error> {
/// // Create a new client and authenticate
/// let mut client = Client::new()?;
/// let client = client
///     .with_login("your-email@example.com", "password")
///     .await?;
///
/// // Or use an existing token
/// let base_client = Client::new()?;
/// let client = base_client.with_token("your-token-here")?;
///
/// // Get organization and projects
/// let org = client.organization().await?;
/// let projects = client.projects(None).await?;
///
/// // Work with datasets
/// let dataset_id = DatasetID::from_str("ds-abc123")?;
/// let dataset = client.dataset(dataset_id).await?;
/// # Ok(())
/// # }
/// ```
/// Client is Clone but cannot derive Debug due to dyn TokenStorage
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    /// HTTP client for long-running bulk transfers (uploads/downloads, no total-request
    /// timeout). An idle read timeout is still configured on the underlying client, and
    /// some operations (such as uploads) may apply additional per-request timeouts.
    bulk_http: reqwest::Client,
    url: String,
    token: Arc<RwLock<String>>,
    /// Token storage backend. When set, tokens are automatically persisted.
    storage: Option<Arc<dyn TokenStorage>>,
    /// Legacy token path field for backwards compatibility with
    /// with_token_path(). Deprecated: Use with_storage() instead.
    token_path: Option<PathBuf>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("url", &self.url)
            .field("has_storage", &self.storage.is_some())
            .field("token_path", &self.token_path)
            .finish()
    }
}

/// Private context struct for pagination operations
struct FetchContext<'a> {
    dataset_id: DatasetID,
    annotation_set_id: Option<AnnotationSetID>,
    groups: &'a [String],
    types: Vec<String>,
    labels: &'a HashMap<String, u64>,
}

#[derive(Debug, Serialize)]
struct JobsListRequest {}

#[derive(Debug, Serialize)]
struct JobRunRequest {
    name: String,
    job_name: String,
    env: std::collections::HashMap<String, String>,
    data: std::collections::HashMap<String, crate::api::Parameter>,
}

#[derive(Debug, Serialize)]
struct JobStopRequest {
    task_id: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskDataListRequest {
    pub(crate) task_id: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskDataDownloadRequest {
    pub(crate) task_id: u64,
    pub(crate) folder: String,
    pub(crate) file: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskChartAddRequest {
    pub(crate) task_id: u64,
    pub(crate) group_name: String,
    pub(crate) chart_name: String,
    pub(crate) params: Option<crate::api::Parameter>,
    pub(crate) data: crate::api::Parameter,
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskChartListRequest {
    pub(crate) task_id: u64,
    pub(crate) group_name: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TaskChartGetRequest {
    pub(crate) task_id: u64,
    pub(crate) group_name: String,
    pub(crate) chart_name: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ValDataDownloadRequest {
    pub(crate) session_id: u64,
    pub(crate) filename: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ValDataListRequest {
    pub(crate) session_id: u64,
}

/// Streams the body of a successful `reqwest` response to a file on disk,
/// emitting optional progress events.
///
/// Both `download_artifact` and `rpc_download` share this logic. The caller is
/// responsible for creating any required parent directories before calling this
/// function.
///
/// # Arguments
/// * `resp`     - A successful (HTTP 2xx) `reqwest::Response` whose body will
///   be streamed to `path`.
/// * `path`     - Destination file path (created or truncated).
/// * `progress` - Optional channel; events carry bytes received and
///   `Content-Length` total (0 if the server omits it).
///
/// # Errors
/// Returns `Error::IoError` on file I/O failures or propagates stream errors.
async fn stream_response_to_file(
    resp: reqwest::Response,
    path: &std::path::Path,
    progress: Option<tokio::sync::mpsc::Sender<Progress>>,
) -> Result<(), Error> {
    use tokio::io::AsyncWriteExt as _;
    let total = resp.content_length().unwrap_or(0) as usize;
    let mut stream = resp.bytes_stream();
    let mut file = tokio::fs::File::create(path).await?;
    let mut current = 0usize;

    if let Some(ref tx) = progress {
        let _ = tx
            .send(Progress {
                current: 0,
                total,
                status: None,
            })
            .await;
    }

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        current += chunk.len();
        if let Some(ref tx) = progress {
            let _ = tx
                .send(Progress {
                    current,
                    total,
                    status: None,
                })
                .await;
        }
    }

    // Flush tokio's internal write buffer to the OS before returning.
    // tokio::fs::File buffers writes internally; without this, the buffer
    // may not reach the filesystem before the caller reads the file.
    file.flush().await?;
    Ok(())
}

impl Client {
    /// Create a new unauthenticated client with the default saas server.
    ///
    /// By default, the client uses [`FileTokenStorage`] for token persistence.
    /// Use [`with_storage`][Self::with_storage],
    /// [`with_memory_storage`][Self::with_memory_storage],
    /// or [`with_no_storage`][Self::with_no_storage] to configure storage
    /// behavior.
    ///
    /// To connect to a different server, use [`with_server`][Self::with_server]
    /// or [`with_token`][Self::with_token] (tokens include the server
    /// instance).
    ///
    /// This client is created without a token and will need to authenticate
    /// before using methods that require authentication.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use edgefirst_client::Client;
    ///
    /// # fn main() -> Result<(), edgefirst_client::Error> {
    /// // Create client with default file storage
    /// let client = Client::new()?;
    ///
    /// // Create client without token persistence
    /// let client = Client::new()?.with_memory_storage();
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Result<Self, Error> {
        log_retry_configuration();

        // Get timeout from environment or use default
        let timeout_secs = std::env::var("EDGEFIRST_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30); // Default 30s total deadline for API calls

        // Per-chunk idle timeout for bulk transfers: fires only when no bytes
        // arrive for this duration. Resets after every received chunk, so a
        // healthy multi-GB transfer will never be interrupted.
        let read_timeout_secs = std::env::var("EDGEFIRST_READ_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120); // Default 120s idle timeout for bulk transfers

        // Create single HTTP client with URL-based retry policy
        //
        // The retry policy classifies requests into two categories:
        // - StudioApi (*.edgefirst.studio/api): Fast-fail on auth errors, retry server
        //   errors
        // - FileIO (S3, CloudFront, etc.): Retry all transient errors for robustness
        //
        // This allows the same client to handle both API calls and file operations
        // with appropriate retry behavior for each. See retry.rs for details.
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(timeout_secs))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .retry(create_retry_policy())
            .build()?;

        // Separate HTTP client for bulk transfers (uploads and downloads).
        // No total-request timeout (EDGEFIRST_TIMEOUT does not apply here).
        // Uses read_timeout instead: resets after every received chunk, so a
        // healthy large transfer is never interrupted, but a truly stalled
        // connection (no bytes for EDGEFIRST_READ_TIMEOUT seconds) is aborted.
        let bulk_http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .read_timeout(Duration::from_secs(read_timeout_secs))
            .pool_idle_timeout(Duration::from_secs(90))
            // Bulk file transfers fan out to many concurrent presigned-URL
            // uploads — up to `EDGEFIRST_UPLOAD_BATCHES` pipelined batches ×
            // `max_tasks()` uploads each. Keep enough idle connections warm to
            // reuse across that fan-out instead of churning new TLS handshakes.
            .pool_max_idle_per_host(64)
            .retry(create_retry_policy())
            .build()?;

        // Default to file storage, loading any existing token
        let storage: Arc<dyn TokenStorage> = match FileTokenStorage::new() {
            Ok(file_storage) => Arc::new(file_storage),
            Err(e) => {
                warn!(
                    "Could not initialize file token storage: {}. Using memory storage.",
                    e
                );
                Arc::new(MemoryTokenStorage::new())
            }
        };

        // Try to load existing token from storage
        let token = match storage.load() {
            Ok(Some(t)) => t,
            Ok(None) => String::new(),
            Err(e) => {
                warn!(
                    "Failed to load token from storage: {}. Starting with empty token.",
                    e
                );
                String::new()
            }
        };

        // Extract server from token if available
        let url = if !token.is_empty() {
            match Self::extract_server_from_token(&token) {
                Ok(server) => format!("https://{}.edgefirst.studio", server),
                Err(e) => {
                    warn!(
                        "Failed to extract server from token: {}. Using default server.",
                        e
                    );
                    "https://edgefirst.studio".to_string()
                }
            }
        } else {
            "https://edgefirst.studio".to_string()
        };

        Ok(Client {
            http,
            bulk_http,
            url,
            token: Arc::new(tokio::sync::RwLock::new(token)),
            storage: Some(storage),
            token_path: None,
        })
    }

    /// Returns a new client connected to the specified server instance.
    ///
    /// The server parameter is an instance name that maps to a URL:
    /// - `""` or `"saas"` → `https://edgefirst.studio` (default production
    ///   server)
    /// - `"test"` → `https://test.edgefirst.studio`
    /// - `"stage"` → `https://stage.edgefirst.studio`
    /// - `"dev"` → `https://dev.edgefirst.studio`
    /// - `"{name}"` → `https://{name}.edgefirst.studio`
    ///
    /// # Server Selection Priority
    ///
    /// When using the CLI or Python API, server selection follows this
    /// priority:
    ///
    /// 1. **Token's server** (highest priority) - JWT tokens encode the server
    ///    they were issued for. If you have a valid token, its server is used.
    /// 2. **`with_server()` / `--server`** - Used when logging in or when no
    ///    token is available. If a token exists with a different server, a
    ///    warning is emitted and the token's server takes priority.
    /// 3. **Default `"saas"`** - If no token and no server specified, the
    ///    production server (`https://edgefirst.studio`) is used.
    ///
    /// # Important Notes
    ///
    /// - If a token is already set in the client, calling this method will
    ///   **drop the token** as tokens are specific to the server instance.
    /// - Use [`parse_token_server`][Self::parse_token_server] to check a
    ///   token's server before calling this method.
    /// - For login operations, call `with_server()` first, then authenticate.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use edgefirst_client::Client;
    ///
    /// # fn main() -> Result<(), edgefirst_client::Error> {
    /// let client = Client::new()?.with_server("test")?;
    /// assert_eq!(client.url(), "https://test.edgefirst.studio");
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_server(&self, server: &str) -> Result<Self, Error> {
        // Resolve the target URL. Full URLs (self-hosted Studio,
        // wiremock) are validated through `with_url` so the HTTPS rules
        // there apply uniformly. Short names map to the SaaS pattern.
        // We extract only the URL string and rebuild the Client below,
        // because `with_url` preserves the in-memory token (the contract
        // for self-hosted deployments) whereas `with_server` deliberately
        // clears it (a different server means a stale token).
        let url = if server.starts_with("http://") || server.starts_with("https://") {
            self.with_url(server)?.url().to_string()
        } else {
            match server {
                "" | "saas" => "https://edgefirst.studio".to_string(),
                name => format!("https://{}.edgefirst.studio", name),
            }
        };

        // Clear token from storage when changing servers to prevent
        // authentication issues with stale tokens from different
        // instances. This runs whether the caller passed a short name
        // or a full URL — both reach a new server.
        if let Some(ref storage) = self.storage
            && let Err(e) = storage.clear()
        {
            warn!(
                "Failed to clear token from storage when changing servers: {}",
                e
            );
        }

        Ok(Client {
            url,
            token: Arc::new(tokio::sync::RwLock::new(String::new())),
            ..self.clone()
        })
    }

    /// Returns a new client pointed at an explicit URL.
    ///
    /// Used for self-hosted Studio deployments (e.g.
    /// `https://studio.example.com`) and for offline integration tests
    /// against a mock HTTP server (e.g. `http://127.0.0.1:8080`). The
    /// token is preserved so callers can chain
    /// `Client::new()?.with_url(...)?.with_token(...)`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UrlParseError`] for syntactically invalid URLs and
    /// [`Error::InsecureUrl`] for plain `http://` URLs that resolve to a
    /// non-loopback host: the Studio bearer token rides in the
    /// `Authorization` header, and plain HTTP would leak it in the clear.
    /// Loopback URLs (`127.0.0.1`, `::1`, `localhost`, `*.localhost`) are
    /// permitted because traffic never leaves the machine — wiremock and
    /// local dev servers go through that path.
    pub fn with_url(&self, url: &str) -> Result<Self, Error> {
        // Reject malformed inputs early so test failures point at the test
        // rather than a downstream reqwest send.
        let parsed = url::Url::parse(url)?;
        let scheme = parsed.scheme();
        if scheme == "http" {
            if !is_loopback_host(parsed.host().as_ref()) {
                return Err(Error::InsecureUrl(url.to_string()));
            }
        } else if scheme != "https" {
            return Err(Error::InsecureUrl(url.to_string()));
        }
        Ok(Client {
            url: url.trim_end_matches('/').to_string(),
            ..self.clone()
        })
    }

    /// Returns a new client with the specified token storage backend.
    ///
    /// Use this to configure custom token storage, such as platform-specific
    /// secure storage (iOS Keychain, Android EncryptedSharedPreferences).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use edgefirst_client::{Client, FileTokenStorage};
    /// use std::{path::PathBuf, sync::Arc};
    ///
    /// # fn main() -> Result<(), edgefirst_client::Error> {
    /// // Use a custom file path for token storage
    /// let storage = FileTokenStorage::with_path(PathBuf::from("/custom/path/token"));
    /// let client = Client::new()?.with_storage(Arc::new(storage));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_storage(self, storage: Arc<dyn TokenStorage>) -> Self {
        // Try to load existing token from the new storage
        let token = match storage.load() {
            Ok(Some(t)) => t,
            Ok(None) => String::new(),
            Err(e) => {
                warn!(
                    "Failed to load token from storage: {}. Starting with empty token.",
                    e
                );
                String::new()
            }
        };

        Client {
            token: Arc::new(tokio::sync::RwLock::new(token)),
            storage: Some(storage),
            token_path: None,
            ..self
        }
    }

    /// Returns a new client with in-memory token storage (no persistence).
    ///
    /// Tokens are stored in memory only and lost when the application exits.
    /// This is useful for testing or when you want to manage token persistence
    /// externally.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use edgefirst_client::Client;
    ///
    /// # fn main() -> Result<(), edgefirst_client::Error> {
    /// let client = Client::new()?.with_memory_storage();
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_memory_storage(self) -> Self {
        Client {
            token: Arc::new(tokio::sync::RwLock::new(String::new())),
            storage: Some(Arc::new(MemoryTokenStorage::new())),
            token_path: None,
            ..self
        }
    }

    /// Returns a new client with no token storage.
    ///
    /// Tokens are not persisted. Use this when you want to manage tokens
    /// entirely manually.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use edgefirst_client::Client;
    ///
    /// # fn main() -> Result<(), edgefirst_client::Error> {
    /// let client = Client::new()?.with_no_storage();
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_no_storage(self) -> Self {
        Client {
            storage: None,
            token_path: None,
            ..self
        }
    }

    /// Returns a new client authenticated with the provided username and
    /// password.
    ///
    /// The token is automatically persisted to storage (if configured).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use edgefirst_client::Client;
    ///
    /// # async fn example() -> Result<(), edgefirst_client::Error> {
    /// let client = Client::new()?
    ///     .with_server("test")?
    ///     .with_login("user@example.com", "password")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, password)))]
    pub async fn with_login(&self, username: &str, password: &str) -> Result<Self, Error> {
        let params = HashMap::from([("username", username), ("password", password)]);
        let login: LoginResult = self
            .rpc_without_auth("auth.login".to_owned(), Some(params))
            .await?;

        // Validate that the server returned a non-empty token
        if login.token.is_empty() {
            return Err(Error::EmptyToken);
        }

        // Persist token to storage if configured
        if let Some(ref storage) = self.storage
            && let Err(e) = storage.store(&login.token)
        {
            warn!("Failed to persist token to storage: {}", e);
        }

        Ok(Client {
            token: Arc::new(tokio::sync::RwLock::new(login.token)),
            ..self.clone()
        })
    }

    /// Returns a new client which will load and save the token to the specified
    /// path.
    ///
    /// **Deprecated**: Use [`with_storage`][Self::with_storage] with
    /// [`FileTokenStorage`] instead for more flexible token management.
    ///
    /// This method is maintained for backwards compatibility with existing
    /// code. It disables the default storage and uses file-based storage at
    /// the specified path.
    pub fn with_token_path(&self, token_path: Option<&Path>) -> Result<Self, Error> {
        let token_path = match token_path {
            Some(path) => path.to_path_buf(),
            None => ProjectDirs::from("ai", "EdgeFirst", "EdgeFirst Studio")
                .ok_or_else(|| {
                    Error::IoError(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Could not determine user config directory",
                    ))
                })?
                .config_dir()
                .join("token"),
        };

        debug!("Using token path (legacy): {:?}", token_path);

        let token = match token_path.exists() {
            true => std::fs::read_to_string(&token_path)?,
            false => "".to_string(),
        };

        if !token.is_empty() {
            match self.with_token(&token) {
                Ok(client) => Ok(Client {
                    token_path: Some(token_path),
                    storage: None, // Disable new storage when using legacy token_path
                    ..client
                }),
                Err(e) => {
                    // Token is corrupted or invalid - remove it and continue with no token
                    warn!(
                        "Invalid or corrupted token file at {:?}: {:?}. Removing token file.",
                        token_path, e
                    );
                    if let Err(remove_err) = std::fs::remove_file(&token_path) {
                        warn!("Failed to remove corrupted token file: {:?}", remove_err);
                    }
                    // Clear any token from default storage to ensure we don't use it
                    Ok(Client {
                        token_path: Some(token_path),
                        storage: None,
                        token: Arc::new(RwLock::new("".to_string())),
                        ..self.clone()
                    })
                }
            }
        } else {
            // No token in the legacy file - clear any token from default storage
            Ok(Client {
                token_path: Some(token_path),
                storage: None,
                token: Arc::new(RwLock::new("".to_string())),
                ..self.clone()
            })
        }
    }

    /// Returns a new client authenticated with the provided token.
    ///
    /// The token is automatically persisted to storage (if configured).
    /// The server URL is extracted from the token payload.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use edgefirst_client::Client;
    ///
    /// # fn main() -> Result<(), edgefirst_client::Error> {
    /// let client = Client::new()?.with_token("your-jwt-token")?;
    /// # Ok(())
    /// # }
    /// ```
    /// Extract server name from JWT token payload.
    ///
    /// Helper method to parse the JWT token and extract the "server" field
    /// from the payload. Returns the server name (e.g., "test", "stage", "")
    /// or an error if the token is invalid.
    fn extract_server_from_token(token: &str) -> Result<String, Error> {
        let token_parts: Vec<&str> = token.split('.').collect();
        if token_parts.len() != 3 {
            return Err(Error::InvalidToken);
        }

        let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
            .decode(token_parts[1])
            .map_err(|_| Error::InvalidToken)?;
        let payload: HashMap<String, serde_json::Value> = serde_json::from_slice(&decoded)?;
        let server = match payload.get("server") {
            Some(value) => value.as_str().ok_or(Error::InvalidToken)?.to_string(),
            None => return Err(Error::InvalidToken),
        };

        Ok(server)
    }

    pub fn with_token(&self, token: &str) -> Result<Self, Error> {
        if token.is_empty() {
            return Ok(self.clone());
        }

        let server = Self::extract_server_from_token(token)?;

        // Persist token to storage if configured
        if let Some(ref storage) = self.storage
            && let Err(e) = storage.store(token)
        {
            warn!("Failed to persist token to storage: {}", e);
        }

        Ok(Client {
            url: format!("https://{}.edgefirst.studio", server),
            token: Arc::new(tokio::sync::RwLock::new(token.to_string())),
            ..self.clone()
        })
    }

    /// Persist the current token to storage.
    ///
    /// This is automatically called when using [`with_login`][Self::with_login]
    /// or [`with_token`][Self::with_token], so you typically don't need to call
    /// this directly.
    ///
    /// If using the legacy `token_path` configuration, saves to the file path.
    /// If using the new storage abstraction, saves to the configured storage.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn save_token(&self) -> Result<(), Error> {
        let token = self.token.read().await;

        // Try new storage first
        if let Some(ref storage) = self.storage {
            storage.store(&token)?;
            debug!("Token saved to storage");
            return Ok(());
        }

        // Fall back to legacy token_path behavior
        let path = self.token_path.clone().unwrap_or_else(|| {
            ProjectDirs::from("ai", "EdgeFirst", "EdgeFirst Studio")
                .map(|dirs| dirs.config_dir().join("token"))
                .unwrap_or_else(|| PathBuf::from(".token"))
        });

        create_dir_all(path.parent().ok_or_else(|| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Token path has no parent directory",
            ))
        })?)?;
        let mut file = std::fs::File::create(&path)?;
        file.write_all(token.as_bytes())?;

        debug!("Saved token to {:?}", path);

        Ok(())
    }

    /// Return the version of the EdgeFirst Studio server for the current
    /// client connection.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn version(&self) -> Result<String, Error> {
        let version: HashMap<String, String> = self
            .rpc_without_auth::<(), HashMap<String, String>>("version".to_owned(), None)
            .await?;
        let version = version.get("version").ok_or(Error::InvalidResponse)?;
        Ok(version.to_owned())
    }

    /// Clear the token used to authenticate the client with the server.
    ///
    /// Clears the token from memory and from storage (if configured).
    /// If using the legacy `token_path` configuration, removes the token file.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn logout(&self) -> Result<(), Error> {
        {
            let mut token = self.token.write().await;
            *token = "".to_string();
        }

        // Clear from new storage if configured
        if let Some(ref storage) = self.storage
            && let Err(e) = storage.clear()
        {
            warn!("Failed to clear token from storage: {}", e);
        }

        // Also clear legacy token_path if configured
        if let Some(path) = &self.token_path
            && path.exists()
        {
            fs::remove_file(path).await?;
        }

        Ok(())
    }

    /// Return the token used to authenticate the client with the server.  When
    /// logging into the server using a username and password, the token is
    /// returned by the server and stored in the client for future interactions.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn token(&self) -> String {
        self.token.read().await.clone()
    }

    /// Verify the token used to authenticate the client with the server.  This
    /// method is used to ensure that the token is still valid and has not
    /// expired.  If the token is invalid, the server will return an error and
    /// the client will need to login again.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn verify_token(&self) -> Result<(), Error> {
        self.rpc::<(), LoginResult>("auth.verify_token".to_owned(), None)
            .await?;
        Ok::<(), Error>(())
    }

    /// Renew the token used to authenticate the client with the server.
    ///
    /// Refreshes the token before it expires. If the token has already expired,
    /// the server will return an error and you will need to login again.
    ///
    /// The new token is automatically persisted to storage (if configured).
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn renew_token(&self) -> Result<(), Error> {
        let params = HashMap::from([("username".to_string(), self.username().await?)]);
        let result: LoginResult = self
            .rpc_without_auth("auth.refresh".to_owned(), Some(params))
            .await?;

        {
            let mut token = self.token.write().await;
            *token = result.token.clone();
        }

        // Persist to new storage if configured
        if let Some(ref storage) = self.storage
            && let Err(e) = storage.store(&result.token)
        {
            warn!("Failed to persist renewed token to storage: {}", e);
        }

        // Also persist to legacy token_path if configured
        if self.token_path.is_some() {
            self.save_token().await?;
        }

        Ok(())
    }

    async fn token_field(&self, field: &str) -> Result<serde_json::Value, Error> {
        let token = self.token.read().await;
        if token.is_empty() {
            return Err(Error::EmptyToken);
        }

        let token_parts: Vec<&str> = token.split('.').collect();
        if token_parts.len() != 3 {
            return Err(Error::InvalidToken);
        }

        let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
            .decode(token_parts[1])
            .map_err(|_| Error::InvalidToken)?;
        let payload: HashMap<String, serde_json::Value> = serde_json::from_slice(&decoded)?;
        match payload.get(field) {
            Some(value) => Ok(value.to_owned()),
            None => Err(Error::InvalidToken),
        }
    }

    /// Returns the URL of the EdgeFirst Studio server for the current client.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the server name for the current client.
    ///
    /// This extracts the server name from the client's URL:
    /// - `https://edgefirst.studio` → `"saas"`
    /// - `https://test.edgefirst.studio` → `"test"`
    /// - `https://{name}.edgefirst.studio` → `"{name}"`
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use edgefirst_client::Client;
    ///
    /// # fn main() -> Result<(), edgefirst_client::Error> {
    /// let client = Client::new()?.with_server("test")?;
    /// assert_eq!(client.server(), "test");
    ///
    /// let client = Client::new()?; // default
    /// assert_eq!(client.server(), "saas");
    /// # Ok(())
    /// # }
    /// ```
    pub fn server(&self) -> &str {
        if self.url == "https://edgefirst.studio" {
            "saas"
        } else if let Some(name) = self.url.strip_prefix("https://") {
            name.strip_suffix(".edgefirst.studio").unwrap_or("saas")
        } else {
            "saas"
        }
    }

    /// Returns the username associated with the current token.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn username(&self) -> Result<String, Error> {
        match self.token_field("username").await? {
            serde_json::Value::String(username) => Ok(username),
            _ => Err(Error::InvalidToken),
        }
    }

    /// Returns the expiration time for the current token.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn token_expiration(&self) -> Result<DateTime<Utc>, Error> {
        let ts = match self.token_field("exp").await? {
            serde_json::Value::Number(exp) => exp.as_i64().ok_or(Error::InvalidToken)?,
            _ => return Err(Error::InvalidToken),
        };

        match DateTime::<Utc>::from_timestamp(ts, 0) {
            Some(dt) => Ok(dt),
            None => Err(Error::InvalidToken),
        }
    }

    /// Returns the organization information for the current user.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn organization(&self) -> Result<Organization, Error> {
        self.rpc::<(), Organization>("org.get".to_owned(), None)
            .await
    }

    /// Returns a list of projects available to the user.  The projects are
    /// returned as a vector of Project objects.  If a name filter is
    /// provided, only projects matching the filter are returned.
    ///
    /// Results are sorted by match quality: exact matches first, then
    /// case-insensitive exact matches, then shorter names (more specific),
    /// then alphabetically.
    ///
    /// Projects are the top-level organizational unit in EdgeFirst Studio.
    /// Projects contain datasets, trainers, and trainer sessions.  Projects
    /// are used to group related datasets and trainers together.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn projects(&self, name: Option<&str>) -> Result<Vec<Project>, Error> {
        let projects = self
            .rpc::<(), Vec<Project>>("project.list".to_owned(), None)
            .await?;
        if let Some(name) = name {
            Ok(filter_and_sort_by_name(projects, name, |p| p.name()))
        } else {
            Ok(projects)
        }
    }

    /// Return the project with the specified project ID.  If the project does
    /// not exist, an error is returned.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(project_id = %project_id)))]
    pub async fn project(&self, project_id: ProjectID) -> Result<Project, Error> {
        let params = HashMap::from([("project_id", project_id)]);
        self.rpc("project.get".to_owned(), Some(params)).await
    }

    /// Returns a list of datasets available to the user.  The datasets are
    /// returned as a vector of Dataset objects.  If a name filter is
    /// provided, only datasets matching the filter are returned.
    ///
    /// Results are sorted by match quality: exact matches first, then
    /// case-insensitive exact matches, then shorter names (more specific),
    /// then alphabetically. This ensures "Deer" returns before "Deer
    /// Roundtrip".
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn datasets(
        &self,
        project_id: ProjectID,
        name: Option<&str>,
    ) -> Result<Vec<Dataset>, Error> {
        let params = HashMap::from([("project_id", project_id)]);
        let datasets: Vec<Dataset> = self.rpc("dataset.list".to_owned(), Some(params)).await?;
        if let Some(name) = name {
            Ok(filter_and_sort_by_name(datasets, name, |d| d.name()))
        } else {
            Ok(datasets)
        }
    }

    /// Return the dataset with the specified dataset ID.  If the dataset does
    /// not exist, an error is returned.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(dataset_id = %dataset_id)))]
    pub async fn dataset(&self, dataset_id: DatasetID) -> Result<Dataset, Error> {
        let params = HashMap::from([("dataset_id", dataset_id)]);
        self.rpc("dataset.get".to_owned(), Some(params)).await
    }

    /// Lists the labels for the specified dataset.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(dataset_id = %dataset_id)))]
    pub async fn labels(&self, dataset_id: DatasetID) -> Result<Vec<Label>, Error> {
        let params = HashMap::from([("dataset_id", dataset_id)]);
        self.rpc("label.list".to_owned(), Some(params)).await
    }

    /// Add a new label to the dataset with the specified name.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(dataset_id = %dataset_id)))]
    pub async fn add_label(&self, dataset_id: DatasetID, name: &str) -> Result<(), Error> {
        let new_label = NewLabel {
            dataset_id,
            labels: vec![NewLabelObject {
                name: name.to_owned(),
            }],
        };
        let _: String = self.rpc("label.add2".to_owned(), Some(new_label)).await?;
        Ok(())
    }

    /// Removes the label with the specified ID from the dataset.  Label IDs are
    /// globally unique so the dataset_id is not required.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn remove_label(&self, label_id: u64) -> Result<(), Error> {
        let params = HashMap::from([("label_id", label_id)]);
        let _: String = self.rpc("label.del".to_owned(), Some(params)).await?;
        Ok(())
    }

    /// Creates a new dataset in the specified project.
    ///
    /// # Arguments
    ///
    /// * `project_id` - The ID of the project to create the dataset in
    /// * `name` - The name of the new dataset
    /// * `description` - Optional description for the dataset
    ///
    /// # Returns
    ///
    /// Returns the dataset ID of the newly created dataset.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn create_dataset(
        &self,
        project_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<DatasetID, Error> {
        let mut params = HashMap::new();
        params.insert("project_id", project_id);
        params.insert("name", name);
        if let Some(desc) = description {
            params.insert("description", desc);
        }

        #[derive(Deserialize)]
        struct CreateDatasetResult {
            id: DatasetID,
        }

        let result: CreateDatasetResult =
            self.rpc("dataset.create".to_owned(), Some(params)).await?;
        Ok(result.id)
    }

    /// Deletes a dataset by marking it as deleted.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The ID of the dataset to delete
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the dataset was successfully marked as deleted.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(dataset_id = %dataset_id)))]
    pub async fn delete_dataset(&self, dataset_id: DatasetID) -> Result<(), Error> {
        let params = HashMap::from([("id", dataset_id)]);
        let _: serde_json::Value = self.rpc("dataset.delete".to_owned(), Some(params)).await?;
        Ok(())
    }

    /// Updates the label with the specified ID to have the new name or index.
    /// Label IDs cannot be changed.  Label IDs are globally unique so the
    /// dataset_id is not required.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, label)))]
    pub async fn update_label(&self, label: &Label) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Params {
            dataset_id: DatasetID,
            label_id: u64,
            label_name: String,
            label_index: u64,
        }

        let _: String = self
            .rpc(
                "label.update".to_owned(),
                Some(Params {
                    dataset_id: label.dataset_id(),
                    label_id: label.id(),
                    label_name: label.name().to_owned(),
                    label_index: label.index(),
                }),
            )
            .await?;
        Ok(())
    }

    /// Lists the groups for the specified dataset.
    ///
    /// Groups are used to organize samples into logical subsets such as
    /// "train", "val", "test", etc. Each sample can belong to at most one
    /// group at a time.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The ID of the dataset to list groups for
    ///
    /// # Returns
    ///
    /// Returns a vector of [`Group`] objects for the dataset. Returns an
    /// empty vector if no groups have been created yet.
    ///
    /// # Errors
    ///
    /// Returns an error if the dataset does not exist or cannot be accessed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use edgefirst_client::{Client, DatasetID};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new()?.with_token_path(None)?;
    /// let dataset_id: DatasetID = "ds-123".try_into()?;
    ///
    /// let groups = client.groups(dataset_id).await?;
    /// for group in groups {
    ///     println!("{}: {}", group.id, group.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(dataset_id = %dataset_id)))]
    pub async fn groups(&self, dataset_id: DatasetID) -> Result<Vec<Group>, Error> {
        let params = HashMap::from([("dataset_id", dataset_id)]);
        self.rpc("groups.list".to_owned(), Some(params)).await
    }

    /// Gets an existing group by name or creates a new one.
    ///
    /// This is a convenience method that first checks if a group with the
    /// specified name exists, and creates it if not. This is useful when
    /// you need to ensure a group exists before assigning samples to it.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The ID of the dataset
    /// * `name` - The name of the group (e.g., "train", "val", "test")
    ///
    /// # Returns
    ///
    /// Returns the group ID (either existing or newly created).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The dataset does not exist or cannot be accessed
    /// - The group creation fails
    ///
    /// # Concurrency
    ///
    /// This method handles concurrent creation attempts gracefully. If another
    /// process creates the group between the existence check and creation,
    /// this method will return the existing group's ID.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use edgefirst_client::{Client, DatasetID};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new()?.with_token_path(None)?;
    /// let dataset_id: DatasetID = "ds-123".try_into()?;
    ///
    /// // Get or create a "train" group
    /// let train_group_id = client
    ///     .get_or_create_group(dataset_id.clone(), "train")
    ///     .await?;
    /// println!("Train group ID: {}", train_group_id);
    ///
    /// // Calling again returns the same ID
    /// let same_id = client.get_or_create_group(dataset_id, "train").await?;
    /// assert_eq!(train_group_id, same_id);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(dataset_id = %dataset_id)))]
    pub async fn get_or_create_group(
        &self,
        dataset_id: DatasetID,
        name: &str,
    ) -> Result<u64, Error> {
        // First check if the group already exists
        let groups = self.groups(dataset_id).await?;
        if let Some(group) = groups.iter().find(|g| g.name == name) {
            return Ok(group.id);
        }

        // Create the group
        #[derive(Serialize)]
        struct CreateGroupParams {
            dataset_id: DatasetID,
            group_names: Vec<String>,
            group_splits: Vec<i64>,
        }

        let params = CreateGroupParams {
            dataset_id,
            group_names: vec![name.to_string()],
            group_splits: vec![0], // No automatic splitting
        };

        let created_groups: Vec<Group> = self.rpc("groups.create".to_owned(), Some(params)).await?;
        if let Some(group) = created_groups.into_iter().find(|g| g.name == name) {
            Ok(group.id)
        } else {
            // Group might have been created by concurrent call, try fetching again
            let groups = self.groups(dataset_id).await?;
            groups
                .iter()
                .find(|g| g.name == name)
                .map(|g| g.id)
                .ok_or_else(|| {
                    Error::RpcError(0, format!("Failed to create or find group '{}'", name))
                })
        }
    }

    /// Sets the group for a sample.
    ///
    /// Assigns a sample to a specific group. Each sample can belong to at most
    /// one group at a time. Setting a new group replaces any existing group
    /// assignment.
    ///
    /// # Arguments
    ///
    /// * `sample_id` - The ID of the sample (image) to update
    /// * `group_id` - The ID of the group to assign. Use
    ///   [`get_or_create_group`] to obtain a group ID from a name.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The sample does not exist
    /// - The group does not exist
    /// - Insufficient permissions to modify the sample
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use edgefirst_client::{Client, DatasetID, SampleID};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new()?.with_token_path(None)?;
    /// let dataset_id: DatasetID = "ds-123".try_into()?;
    /// let sample_id: SampleID = 12345.into();
    ///
    /// // Get or create the "val" group
    /// let val_group_id = client.get_or_create_group(dataset_id, "val").await?;
    ///
    /// // Assign the sample to the "val" group
    /// client.set_sample_group_id(sample_id, val_group_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`get_or_create_group`]: Self::get_or_create_group
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn set_sample_group_id(
        &self,
        sample_id: SampleID,
        group_id: u64,
    ) -> Result<(), Error> {
        #[derive(Serialize)]
        struct SetGroupParams {
            image_id: SampleID,
            group_id: u64,
        }

        let params = SetGroupParams {
            image_id: sample_id,
            group_id,
        };
        let _: String = self
            .rpc("image.set_group_id".to_owned(), Some(params))
            .await?;
        Ok(())
    }

    /// Downloads dataset samples to the local filesystem.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The unique identifier of the dataset
    /// * `groups` - Dataset groups to include (e.g., "train", "val")
    /// * `file_types` - File types to download. Supported types:
    ///   - `FileType::Image` - Standard image files (JPEG, PNG, etc.)
    ///   - `FileType::LidarPcd` - LiDAR point cloud data (.pcd format)
    ///   - `FileType::LidarDepth` - LiDAR depth images (.png format)
    ///   - `FileType::LidarReflect` - LiDAR reflectance images (.jpg format)
    ///   - `FileType::RadarPcd` - Radar point cloud data (.pcd format)
    ///   - `FileType::RadarCube` - Radar cube data (.png format)
    ///   - `FileType::All` - All sensor types (expands to all of the above)
    /// * `output` - Local directory to save downloaded files
    /// * `flatten` - If true, download all files to output root without
    ///   sequence subdirectories. When flattening, filenames are prefixed with
    ///   `{sequence_name}_{frame}_` (or `{sequence_name}_` if frame is
    ///   unavailable) unless the filename already starts with
    ///   `{sequence_name}_`, to avoid conflicts between sequences.
    /// * `progress` - Optional channel for progress updates
    ///
    /// # Progress
    ///
    /// This operation has two phases with distinct progress reporting:
    ///
    /// 1. **Fetching metadata** (`status: None`): Retrieves sample information
    ///    from the server. Progress counts samples fetched.
    /// 2. **Downloading files** (`status: "Downloading"`): Downloads actual
    ///    files to disk. Progress counts samples completed (each sample may
    ///    have multiple files for different sensor types).
    ///
    /// Applications should detect the status change from `None` to
    /// `"Downloading"` to reset their progress bar for the second phase.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success or an error if download fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use edgefirst_client::{Client, DatasetID, FileType};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new()?.with_token_path(None)?;
    /// let dataset_id: DatasetID = "ds-123".try_into()?;
    ///
    /// // Download with sequence subdirectories (default)
    /// client
    ///     .download_dataset(
    ///         dataset_id,
    ///         &[],
    ///         &[FileType::Image],
    ///         "./data".into(),
    ///         false,
    ///         None,
    ///     )
    ///     .await?;
    ///
    /// // Download flattened (all files in one directory)
    /// client
    ///     .download_dataset(
    ///         dataset_id,
    ///         &[],
    ///         &[FileType::Image],
    ///         "./data".into(),
    ///         true,
    ///         None,
    ///     )
    ///     .await?;
    ///
    /// // Download all sensor types
    /// client
    ///     .download_dataset(
    ///         dataset_id,
    ///         &[],
    ///         &FileType::expand_types(&[FileType::All]),
    ///         "./data".into(),
    ///         false,
    ///         None,
    ///     )
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, groups, file_types, progress), fields(dataset_id = %dataset_id, output = %output.display())))]
    pub async fn download_dataset(
        &self,
        dataset_id: DatasetID,
        groups: &[String],
        file_types: &[FileType],
        output: PathBuf,
        flatten: bool,
        progress: Option<Sender<Progress>>,
    ) -> Result<(), Error> {
        // Phase 1: Fetch sample metadata (pass progress directly, no wrapper)
        let samples = self
            .samples(dataset_id, None, &[], groups, file_types, progress.clone())
            .await?;
        fs::create_dir_all(&output).await?;

        // Phase 2: Download actual files using direct semaphore pattern
        let total = samples.len();
        let current = Arc::new(AtomicUsize::new(0));
        let sem = Arc::new(Semaphore::new(max_tasks()));

        // Send initial progress for download phase
        if let Some(ref progress) = progress {
            let _ = progress
                .send(Progress {
                    current: 0,
                    total,
                    status: Some("Downloading".to_string()),
                })
                .await;
        }

        let tasks = samples
            .into_iter()
            .map(|sample| {
                let client = self.clone();
                let file_types = file_types.to_vec();
                let output = output.clone();
                let progress = progress.clone();
                let current = current.clone();
                let sem = sem.clone();

                tokio::spawn(async move {
                    let _permit = sem.acquire().await.map_err(|_| {
                        Error::IoError(std::io::Error::other("Semaphore closed unexpectedly"))
                    })?;

                    for file_type in &file_types {
                        if let Some(data) = sample.download(&client, file_type.clone()).await? {
                            let (file_ext, is_image) = match file_type {
                                FileType::Image => (
                                    infer::get(&data)
                                        .expect("Failed to identify image file format for sample")
                                        .extension()
                                        .to_string(),
                                    true,
                                ),
                                other => (other.file_extension().to_string(), false),
                            };

                            // Determine target directory based on sequence membership and
                            // flatten option
                            // - flatten=false + sequence_name: dataset/sequence_name/
                            // - flatten=false + no sequence: dataset/ (root level)
                            // - flatten=true: dataset/ (all files in output root)
                            // NOTE: group (train/val/test) is NOT used for directory structure
                            let sequence_dir = sample
                                .sequence_name()
                                .map(|name| sanitize_path_component(name));

                            let target_dir = if flatten {
                                output.clone()
                            } else {
                                sequence_dir
                                    .as_ref()
                                    .map(|seq| output.join(seq))
                                    .unwrap_or_else(|| output.clone())
                            };
                            fs::create_dir_all(&target_dir).await?;

                            let sanitized_sample_name = sample
                                .name()
                                .map(|name| sanitize_path_component(&name))
                                .unwrap_or_else(|| "unknown".to_string());

                            let image_name = sample.image_name().map(sanitize_path_component);

                            // Construct filename with smart prefixing for flatten mode
                            // When flatten=true and sample belongs to a sequence:
                            //   - Check if filename already starts with "{sequence_name}_"
                            //   - If not, prepend "{sequence_name}_{frame}_" to avoid conflicts
                            //   - If yes, use filename as-is (already uniquely named)
                            let file_name = if is_image {
                                if let Some(img_name) = image_name {
                                    Client::build_filename(
                                        &img_name,
                                        flatten,
                                        sequence_dir.as_ref(),
                                        sample.frame_number(),
                                    )
                                } else {
                                    format!("{}.{}", sanitized_sample_name, file_ext)
                                }
                            } else {
                                let base_name = format!("{}.{}", sanitized_sample_name, file_ext);
                                Client::build_filename(
                                    &base_name,
                                    flatten,
                                    sequence_dir.as_ref(),
                                    sample.frame_number(),
                                )
                            };

                            let file_path = target_dir.join(&file_name);

                            let mut file = File::create(&file_path).await?;
                            file.write_all(&data).await?;
                        }
                    }

                    // Update progress after sample completes
                    if let Some(progress) = &progress {
                        let completed = current.fetch_add(1, Ordering::SeqCst) + 1;
                        let _ = progress
                            .send(Progress {
                                current: completed,
                                total,
                                status: Some("Downloading".to_string()),
                            })
                            .await;
                    }

                    Ok::<(), Error>(())
                })
            })
            .collect::<Vec<_>>();

        join_all(tasks)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    /// Builds a filename with smart prefixing for flatten mode.
    ///
    /// When flattening sequences into a single directory, this function ensures
    /// unique filenames by checking if the sequence prefix already exists and
    /// adding it if necessary.
    ///
    /// # Logic
    ///
    /// - If `flatten=false`: returns `base_name` unchanged
    /// - If `flatten=true` and no sequence: returns `base_name` unchanged
    /// - If `flatten=true` and in sequence:
    ///   - Already prefixed with `{sequence_name}_`: returns `base_name`
    ///     unchanged
    ///   - Not prefixed: returns `{sequence_name}_{frame}_{base_name}` or
    ///     `{sequence_name}_{base_name}`
    fn build_filename(
        base_name: &str,
        flatten: bool,
        sequence_name: Option<&String>,
        frame_number: Option<u32>,
    ) -> String {
        if !flatten || sequence_name.is_none() {
            return base_name.to_string();
        }

        let seq_name = sequence_name.unwrap();
        let prefix = format!("{}_", seq_name);

        // Check if already prefixed with sequence name
        if base_name.starts_with(&prefix) {
            base_name.to_string()
        } else {
            // Add sequence (and optionally frame) prefix
            match frame_number {
                Some(frame) => format!("{}{}_{}", prefix, frame, base_name),
                None => format!("{}{}", prefix, base_name),
            }
        }
    }

    /// List available annotation sets for the specified dataset.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(dataset_id = %dataset_id)))]
    pub async fn annotation_sets(
        &self,
        dataset_id: DatasetID,
    ) -> Result<Vec<AnnotationSet>, Error> {
        let params = HashMap::from([("dataset_id", dataset_id)]);
        self.rpc("annset.list".to_owned(), Some(params)).await
    }

    /// Create a new annotation set for the specified dataset.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The ID of the dataset to create the annotation set in
    /// * `name` - The name of the new annotation set
    /// * `description` - Optional description for the annotation set
    ///
    /// # Returns
    ///
    /// Returns the annotation set ID of the newly created annotation set.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn create_annotation_set(
        &self,
        dataset_id: DatasetID,
        name: &str,
        description: Option<&str>,
    ) -> Result<AnnotationSetID, Error> {
        #[derive(Serialize)]
        struct Params<'a> {
            dataset_id: DatasetID,
            name: &'a str,
            operator: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<&'a str>,
        }

        #[derive(Deserialize)]
        struct CreateAnnotationSetResult {
            id: AnnotationSetID,
        }

        let username = self.username().await?;
        let result: CreateAnnotationSetResult = self
            .rpc(
                "annset.add".to_owned(),
                Some(Params {
                    dataset_id,
                    name,
                    operator: &username,
                    description,
                }),
            )
            .await?;
        Ok(result.id)
    }

    /// Deletes an annotation set by marking it as deleted.
    ///
    /// # Arguments
    ///
    /// * `annotation_set_id` - The ID of the annotation set to delete
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the annotation set was successfully marked as
    /// deleted.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(annotation_set_id = %annotation_set_id)))]
    pub async fn delete_annotation_set(
        &self,
        annotation_set_id: AnnotationSetID,
    ) -> Result<(), Error> {
        let params = HashMap::from([("id", annotation_set_id)]);
        let _: serde_json::Value = self.rpc("annset.delete".to_owned(), Some(params)).await?;
        Ok(())
    }

    /// Retrieve the annotation set with the specified ID.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(annotation_set_id = %annotation_set_id)))]
    pub async fn annotation_set(
        &self,
        annotation_set_id: AnnotationSetID,
    ) -> Result<AnnotationSet, Error> {
        let params = HashMap::from([("annotation_set_id", annotation_set_id)]);
        self.rpc("annset.get".to_owned(), Some(params)).await
    }

    /// Get the annotations for the specified annotation set with the
    /// requested annotation types.  The annotation types are used to filter
    /// the annotations returned.  The groups parameter is used to filter for
    /// dataset groups (train, val, test).  Images which do not have any
    /// annotations are also included in the result as long as they are in the
    /// requested groups (when specified).
    ///
    /// The result is a vector of Annotations objects which contain the
    /// full dataset along with the annotations for the specified types.
    ///
    /// # Progress
    ///
    /// Reports progress with `status: None` as samples are fetched and
    /// processed for their annotations. Progress unit is samples processed
    /// (not individual annotations).
    ///
    /// To get the annotations as a DataFrame, use the `samples_dataframe`
    /// method instead.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(annotation_set_id = %annotation_set_id)))]
    pub async fn annotations(
        &self,
        annotation_set_id: AnnotationSetID,
        groups: &[String],
        annotation_types: &[AnnotationType],
        progress: Option<Sender<Progress>>,
    ) -> Result<Vec<Annotation>, Error> {
        let dataset_id = self.annotation_set(annotation_set_id).await?.dataset_id();
        let labels = self
            .labels(dataset_id)
            .await?
            .into_iter()
            .map(|label| (label.name().to_string(), label.index()))
            .collect::<HashMap<_, _>>();
        let total = self
            .samples_count(
                dataset_id,
                Some(annotation_set_id),
                annotation_types,
                groups,
                &[],
            )
            .await?
            .total as usize;

        if total == 0 {
            return Ok(vec![]);
        }

        let context = FetchContext {
            dataset_id,
            annotation_set_id: Some(annotation_set_id),
            groups,
            types: annotation_types.iter().map(|t| t.to_string()).collect(),
            labels: &labels,
        };

        self.fetch_annotations_paginated(context, total, progress)
            .await
    }

    async fn fetch_annotations_paginated(
        &self,
        context: FetchContext<'_>,
        total: usize,
        progress: Option<Sender<Progress>>,
    ) -> Result<Vec<Annotation>, Error> {
        let mut annotations = vec![];
        let mut continue_token: Option<String> = None;
        let mut current = 0;

        loop {
            let params = SamplesListParams {
                dataset_id: context.dataset_id,
                annotation_set_id: context.annotation_set_id,
                types: context.types.clone(),
                group_names: context.groups.to_vec(),
                continue_token,
            };

            let result: SamplesListResult =
                self.rpc("samples.list".to_owned(), Some(params)).await?;
            current += result.samples.len();
            continue_token = result.continue_token;

            if result.samples.is_empty() {
                break;
            }

            self.process_sample_annotations(&result.samples, context.labels, &mut annotations);

            if let Some(progress) = &progress {
                let _ = progress
                    .send(Progress {
                        current,
                        total,
                        status: None,
                    })
                    .await;
            }

            match &continue_token {
                Some(token) if !token.is_empty() => continue,
                _ => break,
            }
        }

        drop(progress);
        Ok(annotations)
    }

    fn process_sample_annotations(
        &self,
        samples: &[Sample],
        labels: &HashMap<String, u64>,
        annotations: &mut Vec<Annotation>,
    ) {
        for sample in samples {
            if sample.annotations().is_empty() {
                let mut annotation = Annotation::new();
                annotation.set_sample_id(sample.id());
                annotation.set_name(sample.name());
                annotation.set_sequence_name(sample.sequence_name().cloned());
                annotation.set_frame_number(sample.frame_number());
                annotation.set_group(sample.group().cloned());
                annotations.push(annotation);
                continue;
            }

            for annotation in sample.annotations() {
                let mut annotation = annotation.clone();
                annotation.set_sample_id(sample.id());
                annotation.set_name(sample.name());
                annotation.set_sequence_name(sample.sequence_name().cloned());
                annotation.set_frame_number(sample.frame_number());
                annotation.set_group(sample.group().cloned());
                Self::set_label_index_from_map(&mut annotation, labels);
                annotations.push(annotation);
            }
        }
    }

    /// Delete annotations in bulk from specified samples.
    ///
    /// This method calls the `annotation.bulk.del` API to efficiently remove
    /// annotations from multiple samples at once. Useful for clearing
    /// annotations before re-importing updated data.
    ///
    /// # Arguments
    /// * `annotation_set_id` - The annotation set containing the annotations
    /// * `annotation_types` - Types to delete: "box" for bounding boxes, "seg"
    ///   for masks
    /// * `sample_ids` - Sample IDs (image IDs) to delete annotations from
    ///
    /// # Example
    /// ```no_run
    /// # use edgefirst_client::{Client, AnnotationSetID, SampleID};
    /// # async fn example() -> Result<(), edgefirst_client::Error> {
    /// # let client = Client::new()?.with_login("user", "pass").await?;
    /// let annotation_set_id = AnnotationSetID::from(123);
    /// let sample_ids = vec![SampleID::from(1), SampleID::from(2)];
    ///
    /// client
    ///     .delete_annotations_bulk(
    ///         annotation_set_id,
    ///         &["box".to_string(), "seg".to_string()],
    ///         &sample_ids,
    ///     )
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, annotation_types, sample_ids), fields(annotation_set_id = %annotation_set_id)))]
    pub async fn delete_annotations_bulk(
        &self,
        annotation_set_id: AnnotationSetID,
        annotation_types: &[String],
        sample_ids: &[SampleID],
    ) -> Result<(), Error> {
        use crate::api::AnnotationBulkDeleteParams;

        let params = AnnotationBulkDeleteParams {
            annotation_set_id: annotation_set_id.into(),
            annotation_types: annotation_types.to_vec(),
            image_ids: sample_ids.iter().map(|id| (*id).into()).collect(),
            delete_all: None,
        };

        let _: String = self
            .rpc("annotation.bulk.del".to_owned(), Some(params))
            .await?;
        Ok(())
    }

    /// Add annotations in bulk.
    ///
    /// This method calls the `annotation.add_bulk` API to efficiently add
    /// multiple annotations at once. The annotations must be in server format
    /// with image_id references.
    ///
    /// # Arguments
    /// * `annotation_set_id` - The annotation set to add annotations to
    /// * `annotations` - Vector of server-format annotations to add
    ///
    /// # Returns
    /// Vector of created annotation records from the server.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, annotations), fields(annotation_count = annotations.len())))]
    pub async fn add_annotations_bulk(
        &self,
        annotation_set_id: AnnotationSetID,
        annotations: Vec<crate::api::ServerAnnotation>,
    ) -> Result<Vec<serde_json::Value>, Error> {
        use crate::api::AnnotationAddBulkParams;

        let params = AnnotationAddBulkParams {
            annotation_set_id: annotation_set_id.into(),
            annotations,
        };

        self.rpc("annotation.add_bulk".to_owned(), Some(params))
            .await
    }

    /// Helper to parse frame number from image_name when sequence_name is
    /// present. This ensures frame_number is always derived from the image
    /// filename, not from the server's frame_number field (which may be
    /// inconsistent).
    ///
    /// Returns Some(frame_number) if sequence_name is present and frame can be
    /// parsed, otherwise None.
    fn parse_frame_from_image_name(
        image_name: Option<&String>,
        sequence_name: Option<&String>,
    ) -> Option<u32> {
        use std::path::Path;

        let sequence = sequence_name?;
        let name = image_name?;

        // Extract stem (remove extension)
        let stem = Path::new(name).file_stem().and_then(|s| s.to_str())?;

        // Parse frame from format: "sequence_XXX" where XXX is the frame number
        stem.strip_prefix(sequence)
            .and_then(|suffix| suffix.strip_prefix('_'))
            .and_then(|frame_str| frame_str.parse::<u32>().ok())
    }

    /// Helper to set label index from a label map
    fn set_label_index_from_map(annotation: &mut Annotation, labels: &HashMap<String, u64>) {
        if let Some(label) = annotation.label() {
            annotation.set_label_index(Some(labels[label.as_str()]));
        }
    }

    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, annotation_types, groups, types), fields(dataset_id = %dataset_id, annotation_set_id = ?annotation_set_id)))]
    pub async fn samples_count(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
        annotation_types: &[AnnotationType],
        groups: &[String],
        types: &[FileType],
    ) -> Result<SamplesCountResult, Error> {
        // Use server type names for API calls (e.g., "box" instead of "box2d")
        let types = annotation_types
            .iter()
            .map(|t| t.as_server_type().to_string())
            .chain(types.iter().map(|t| t.to_string()))
            .collect::<Vec<_>>();

        let params = SamplesListParams {
            dataset_id,
            annotation_set_id,
            group_names: groups.to_vec(),
            types,
            continue_token: None,
        };

        self.rpc("samples.count".to_owned(), Some(params)).await
    }

    /// Fetches samples from a dataset with optional annotation and file type
    /// filters.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The dataset to fetch samples from
    /// * `annotation_set_id` - Optional annotation set to include annotations
    ///   from
    /// * `annotation_types` - Filter by annotation types (box2d, box3d, mask)
    /// * `groups` - Filter by sample groups (e.g., "train", "val", "test")
    /// * `types` - File types to include metadata for
    /// * `progress` - Optional channel for progress updates
    ///
    /// # Progress
    ///
    /// Reports progress with `status: None` as samples are fetched from the
    /// server in paginated batches. Progress unit is samples fetched.
    ///
    /// # Returns
    ///
    /// Vector of [`Sample`] objects with metadata and optionally annotations.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, annotation_types, groups, types, progress), fields(dataset_id = %dataset_id, annotation_set_id = ?annotation_set_id)))]
    pub async fn samples(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
        annotation_types: &[AnnotationType],
        groups: &[String],
        types: &[FileType],
        progress: Option<Sender<Progress>>,
    ) -> Result<Vec<Sample>, Error> {
        // Use server type names for API calls (e.g., "box" instead of "box2d")
        let types_vec = annotation_types
            .iter()
            .map(|t| t.as_server_type().to_string())
            .chain(types.iter().map(|t| t.to_string()))
            .collect::<Vec<_>>();
        let labels = self
            .labels(dataset_id)
            .await?
            .into_iter()
            .map(|label| (label.name().to_string(), label.index()))
            .collect::<HashMap<_, _>>();
        let total = self
            .samples_count(dataset_id, annotation_set_id, annotation_types, groups, &[])
            .await?
            .total as usize;

        if total == 0 {
            return Ok(vec![]);
        }

        let context = FetchContext {
            dataset_id,
            annotation_set_id,
            groups,
            types: types_vec,
            labels: &labels,
        };

        self.fetch_samples_paginated(context, total, progress).await
    }

    /// Get all sample names in a dataset.
    ///
    /// This is an efficient method for checking which samples already exist,
    /// useful for resuming interrupted imports. It only retrieves sample names
    /// without loading full annotation data.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The dataset to query
    /// * `groups` - Optional group filter (empty = all groups)
    /// * `progress` - Optional progress channel
    ///
    /// # Progress
    ///
    /// Reports progress with `status: None` as sample names are fetched from
    /// the server in paginated batches. Progress unit is samples fetched.
    ///
    /// # Returns
    ///
    /// A HashSet of sample names (image_name field) that exist in the dataset.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(dataset_id = %dataset_id)))]
    pub async fn sample_names(
        &self,
        dataset_id: DatasetID,
        groups: &[String],
        progress: Option<Sender<Progress>>,
    ) -> Result<std::collections::HashSet<String>, Error> {
        use std::collections::HashSet;

        let total = self
            .samples_count(dataset_id, None, &[], groups, &[])
            .await?
            .total as usize;

        if total == 0 {
            return Ok(HashSet::new());
        }

        let mut names = HashSet::with_capacity(total);
        let mut continue_token: Option<String> = None;
        let mut current = 0;

        loop {
            let params = SamplesListParams {
                dataset_id,
                annotation_set_id: None,
                types: vec![], // No type filter - we just want names
                group_names: groups.to_vec(),
                continue_token: continue_token.clone(),
            };

            let result: SamplesListResult =
                self.rpc("samples.list".to_owned(), Some(params)).await?;
            current += result.samples.len();
            continue_token = result.continue_token;

            if result.samples.is_empty() {
                break;
            }

            // Extract sample names (normalized without extension)
            for sample in result.samples {
                if let Some(name) = sample.name() {
                    names.insert(name);
                }
            }

            if let Some(ref p) = progress {
                let _ = p
                    .send(Progress {
                        current,
                        total,
                        status: None,
                    })
                    .await;
            }

            match &continue_token {
                Some(token) if !token.is_empty() => continue,
                _ => break,
            }
        }

        Ok(names)
    }

    async fn fetch_samples_paginated(
        &self,
        context: FetchContext<'_>,
        total: usize,
        progress: Option<Sender<Progress>>,
    ) -> Result<Vec<Sample>, Error> {
        let mut samples = vec![];
        let mut continue_token: Option<String> = None;
        let mut current = 0;

        loop {
            let params = SamplesListParams {
                dataset_id: context.dataset_id,
                annotation_set_id: context.annotation_set_id,
                types: context.types.clone(),
                group_names: context.groups.to_vec(),
                continue_token: continue_token.clone(),
            };

            let result: SamplesListResult =
                self.rpc("samples.list".to_owned(), Some(params)).await?;
            current += result.samples.len();
            continue_token = result.continue_token;

            if result.samples.is_empty() {
                break;
            }

            samples.append(
                &mut result
                    .samples
                    .into_iter()
                    .map(|s| {
                        // Use server's frame_number if valid (>= 0 after deserialization)
                        // Otherwise parse from image_name as fallback
                        // This ensures we respect explicit frame_number from uploads
                        // while still handling legacy data that only has filename encoding
                        let frame_number = s.frame_number.or_else(|| {
                            Self::parse_frame_from_image_name(
                                s.image_name.as_ref(),
                                s.sequence_name.as_ref(),
                            )
                        });

                        let mut anns = s.annotations().to_vec();
                        for ann in &mut anns {
                            // Set annotation fields from parent sample
                            ann.set_name(s.name());
                            ann.set_group(s.group().cloned());
                            ann.set_sequence_name(s.sequence_name().cloned());
                            ann.set_frame_number(frame_number);
                            Self::set_label_index_from_map(ann, context.labels);
                        }
                        s.with_annotations(anns).with_frame_number(frame_number)
                    })
                    .collect::<Vec<_>>(),
            );

            if let Some(progress) = &progress {
                let _ = progress
                    .send(Progress {
                        current,
                        total,
                        status: None,
                    })
                    .await;
            }

            match &continue_token {
                Some(token) if !token.is_empty() => continue,
                _ => break,
            }
        }

        drop(progress);
        Ok(samples)
    }

    /// Populates (imports) samples into a dataset using the `samples.populate2`
    /// API.
    ///
    /// This method creates new samples in the specified dataset, optionally
    /// with annotations and sensor data files. For each sample, the `files`
    /// field is checked for local file paths. If a filename is a valid path
    /// to an existing file, the file will be automatically uploaded to S3
    /// using presigned URLs returned by the server. The filename in the
    /// request is replaced with the basename (path removed) before sending
    /// to the server.
    ///
    /// # Important Notes
    ///
    /// - **`annotation_set_id` is REQUIRED** when importing samples with
    ///   annotations. Without it, the server will accept the request but will
    ///   not save the annotation data. Use [`Client::annotation_sets`] to query
    ///   available annotation sets for a dataset, or create a new one via the
    ///   Studio UI.
    /// - **Box2d coordinates must be normalized** (0.0-1.0 range) for bounding
    ///   boxes. Divide pixel coordinates by image width/height before creating
    ///   [`Box2d`](crate::Box2d) annotations.
    /// - **Files are uploaded automatically** when the filename is a valid
    ///   local path. The method will replace the full path with just the
    ///   basename before sending to the server.
    /// - **Image dimensions are extracted automatically** for image files using
    ///   the `imagesize` crate. The width/height are sent to the server and
    ///   stored in the `image_files` table. These dimensions are returned by
    ///   `samples.list` and used in [`samples_dataframe`](crate::samples_dataframe)
    ///   to populate the `size` column.
    /// - **UUIDs are generated automatically** if not provided. If you need
    ///   deterministic UUIDs, set `sample.uuid` explicitly before calling.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The ID of the dataset to populate
    /// * `annotation_set_id` - **Required** if samples contain annotations,
    ///   otherwise they will be ignored. Query with
    ///   [`Client::annotation_sets`].
    /// * `samples` - Vector of samples to import with metadata and file
    ///   references. For files, use the full local path - it will be uploaded
    ///   automatically. UUIDs and image dimensions will be
    ///   auto-generated/extracted if not provided.
    /// * `progress` - Optional channel for progress updates
    ///
    /// # Progress
    ///
    /// Reports progress with `status: None` as each sample's files are
    /// uploaded. Progress unit is samples (not individual files). Each
    /// sample may contain multiple files (image, lidar, radar, etc.) which
    /// are all uploaded before the sample is counted as complete.
    ///
    /// # Returns
    ///
    /// Returns the API result with sample UUIDs and upload status.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use edgefirst_client::{Annotation, Box2d, Client, DatasetID, Sample, SampleFile};
    ///
    /// # async fn example() -> Result<(), edgefirst_client::Error> {
    /// # let client = Client::new()?.with_login("user", "pass").await?;
    /// # let dataset_id = DatasetID::from(1);
    /// // Query available annotation sets for the dataset
    /// let annotation_sets = client.annotation_sets(dataset_id).await?;
    /// let annotation_set_id = annotation_sets
    ///     .first()
    ///     .ok_or_else(|| {
    ///         edgefirst_client::Error::InvalidParameters("No annotation sets found".to_string())
    ///     })?
    ///     .id();
    ///
    /// // Create sample with annotation (UUID will be auto-generated)
    /// let mut sample = Sample::new();
    /// sample.width = Some(1920);
    /// sample.height = Some(1080);
    /// sample.group = Some("train".to_string());
    ///
    /// // Add file - use full path to local file, it will be uploaded automatically
    /// sample.files = vec![SampleFile::with_filename(
    ///     "image".to_string(),
    ///     "/path/to/image.jpg".to_string(),
    /// )];
    ///
    /// // Add bounding box annotation with NORMALIZED coordinates (0.0-1.0)
    /// let mut annotation = Annotation::new();
    /// annotation.set_label(Some("person".to_string()));
    /// // Normalize pixel coordinates by dividing by image dimensions
    /// let bbox = Box2d::new(0.5, 0.5, 0.25, 0.25); // (x, y, w, h) normalized
    /// annotation.set_box2d(Some(bbox));
    /// sample.annotations = vec![annotation];
    ///
    /// // Populate with annotation_set_id (REQUIRED for annotations)
    /// let result = client
    ///     .populate_samples(dataset_id, Some(annotation_set_id), vec![sample], None)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, samples, progress), fields(sample_count = samples.len())))]
    pub async fn populate_samples(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
        samples: Vec<Sample>,
        progress: Option<Sender<Progress>>,
    ) -> Result<Vec<crate::SamplesPopulateResult>, Error> {
        self.populate_samples_with_concurrency(
            dataset_id,
            annotation_set_id,
            samples,
            progress,
            None,
        )
        .await
    }

    /// Populate samples with custom upload concurrency.
    ///
    /// Same as [`populate_samples`](Self::populate_samples) but allows
    /// specifying the maximum number of concurrent file uploads. Use this
    /// for bulk imports where higher concurrency can significantly reduce
    /// upload time.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, samples, progress), fields(sample_count = samples.len())))]
    pub async fn populate_samples_with_concurrency(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
        samples: Vec<Sample>,
        progress: Option<Sender<Progress>>,
        concurrency: Option<usize>,
    ) -> Result<Vec<crate::SamplesPopulateResult>, Error> {
        use crate::api::SamplesPopulateParams;

        // Track which files need to be uploaded
        let mut files_to_upload: Vec<(String, String, FileSource, String)> = Vec::new();

        // Process samples to detect local files and generate UUIDs
        let samples = self.prepare_samples_for_upload(samples, &mut files_to_upload)?;

        let has_files_to_upload = !files_to_upload.is_empty();

        // Call populate API with presigned_urls=true if we have files to upload
        let params = SamplesPopulateParams {
            dataset_id,
            annotation_set_id,
            presigned_urls: Some(has_files_to_upload),
            samples,
        };

        let results: Vec<crate::SamplesPopulateResult> = self
            .rpc("samples.populate2".to_owned(), Some(params))
            .await?;

        // Upload files if we have any
        if has_files_to_upload {
            self.upload_sample_files(&results, files_to_upload, progress, concurrency)
                .await?;
        }

        Ok(results)
    }

    fn prepare_samples_for_upload(
        &self,
        samples: Vec<Sample>,
        files_to_upload: &mut Vec<(String, String, FileSource, String)>,
    ) -> Result<Vec<Sample>, Error> {
        Ok(samples
            .into_iter()
            .map(|mut sample| {
                // Generate UUID if not provided
                if sample.uuid.is_none() {
                    sample.uuid = Some(uuid::Uuid::new_v4().to_string());
                }

                let sample_uuid = sample.uuid.clone().expect("UUID just set above");

                // Process files: detect local paths and queue for upload
                let files_copy = sample.files.clone();
                let updated_files: Vec<crate::SampleFile> = files_copy
                    .iter()
                    .map(|file| {
                        self.process_sample_file(file, &sample_uuid, &mut sample, files_to_upload)
                    })
                    .collect();

                sample.files = updated_files;
                sample
            })
            .collect())
    }

    fn process_sample_file(
        &self,
        file: &crate::SampleFile,
        sample_uuid: &str,
        sample: &mut Sample,
        files_to_upload: &mut Vec<(String, String, FileSource, String)>,
    ) -> crate::SampleFile {
        use std::path::Path;

        // Handle files with raw bytes (e.g., from ZIP archives)
        if let Some(bytes) = file.bytes()
            && let Some(filename) = file.filename()
        {
            // For image files with bytes, try to extract dimensions if not already set
            if file.file_type() == "image"
                && (sample.width.is_none() || sample.height.is_none())
                && let Ok(size) = imagesize::blob_size(bytes)
            {
                sample.width = Some(size.width as u32);
                sample.height = Some(size.height as u32);
            }

            // Store the bytes for later upload
            files_to_upload.push((
                sample_uuid.to_string(),
                file.file_type().to_string(),
                FileSource::Bytes(bytes.to_vec()),
                filename.to_string(),
            ));

            // Return SampleFile with just the filename
            return crate::SampleFile::with_filename(
                file.file_type().to_string(),
                filename.to_string(),
            );
        }

        // Handle files with local paths
        if let Some(filename) = file.filename() {
            let path = Path::new(filename);

            // Check if this is a valid local file path
            if path.exists()
                && path.is_file()
                && let Some(basename) = path.file_name().and_then(|s| s.to_str())
            {
                // For image files, try to extract dimensions if not already set
                if file.file_type() == "image"
                    && (sample.width.is_none() || sample.height.is_none())
                    && let Ok(size) = imagesize::size(path)
                {
                    sample.width = Some(size.width as u32);
                    sample.height = Some(size.height as u32);
                }

                // Store the full path for later upload
                files_to_upload.push((
                    sample_uuid.to_string(),
                    file.file_type().to_string(),
                    FileSource::Path(path.to_path_buf()),
                    basename.to_string(),
                ));

                // Return SampleFile with just the basename
                return crate::SampleFile::with_filename(
                    file.file_type().to_string(),
                    basename.to_string(),
                );
            }
        }
        // Return the file unchanged if not a local path
        file.clone()
    }

    async fn upload_sample_files(
        &self,
        results: &[crate::SamplesPopulateResult],
        files_to_upload: Vec<(String, String, FileSource, String)>,
        progress: Option<Sender<Progress>>,
        concurrency: Option<usize>,
    ) -> Result<(), Error> {
        // Build a map from (sample_uuid, basename) -> file source
        let mut upload_map: HashMap<(String, String), FileSource> = HashMap::new();
        for (uuid, _file_type, source, basename) in files_to_upload {
            upload_map.insert((uuid, basename), source);
        }

        let http = self.bulk_http.clone();

        // Extract the data we need for parallel upload
        let upload_tasks: Vec<_> = results
            .iter()
            .map(|result| (result.uuid.clone(), result.urls.clone()))
            .collect();

        parallel_foreach_items(
            upload_tasks,
            progress.clone(),
            concurrency,
            move |(uuid, urls)| {
                let http = http.clone();
                let upload_map = upload_map.clone();

                async move {
                    // Upload all files for this sample
                    for url_info in &urls {
                        if let Some(source) =
                            upload_map.get(&(uuid.clone(), url_info.filename.clone()))
                        {
                            match source {
                                FileSource::Path(path) => {
                                    upload_file_to_presigned_url(
                                        http.clone(),
                                        &url_info.url,
                                        path.clone(),
                                    )
                                    .await?;
                                }
                                FileSource::Bytes(bytes) => {
                                    upload_bytes_to_presigned_url(
                                        http.clone(),
                                        &url_info.url,
                                        bytes.clone(),
                                        &url_info.filename,
                                    )
                                    .await?;
                                }
                            }
                        }
                    }

                    Ok(())
                }
            },
        )
        .await
    }

    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn download(&self, url: &str) -> Result<Vec<u8>, Error> {
        // Validate URL is absolute (has scheme) to avoid RelativeUrlWithoutBase error
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(Error::InvalidParameters(format!(
                "Invalid URL (must be absolute): {}",
                url
            )));
        }

        let resp = self.bulk_http.get(url).send().await?;

        if !resp.status().is_success() {
            return Err(Error::HttpError(resp.error_for_status().unwrap_err()));
        }

        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Get samples as a DataFrame with complete 2025.10 schema.
    ///
    /// This is the recommended method for obtaining dataset annotations in
    /// DataFrame format. It includes all sample metadata (size, location,
    /// pose, degradation) as optional columns.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - Dataset identifier
    /// * `annotation_set_id` - Optional annotation set filter
    /// * `groups` - Dataset groups to include (train, val, test)
    /// * `types` - Annotation types to filter (bbox, box3d, mask)
    /// * `progress` - Optional progress callback
    ///
    /// # Progress
    ///
    /// Reports progress with `status: None` as samples are fetched from the
    /// server in paginated batches. Progress unit is samples fetched. This
    /// method delegates to [`samples()`](Self::samples) and shares its
    /// progress behavior.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use edgefirst_client::Client;
    ///
    /// # async fn example() -> Result<(), edgefirst_client::Error> {
    /// # let client = Client::new()?;
    /// # let dataset_id = 1.into();
    /// # let annotation_set_id = 1.into();
    /// let df = client
    ///     .samples_dataframe(
    ///         dataset_id,
    ///         Some(annotation_set_id),
    ///         &["train".to_string()],
    ///         &[],
    ///         None,
    ///     )
    ///     .await?;
    /// println!("DataFrame shape: {:?}", df.shape());
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "polars")]
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(dataset_id = %dataset_id)))]
    pub async fn samples_dataframe(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
        groups: &[String],
        types: &[AnnotationType],
        progress: Option<Sender<Progress>>,
    ) -> Result<DataFrame, Error> {
        use crate::dataset::samples_dataframe;

        let samples = self
            .samples(dataset_id, annotation_set_id, types, groups, &[], progress)
            .await?;
        samples_dataframe(&samples)
    }

    /// Update image dimensions for existing samples in a dataset.
    ///
    /// This is useful for backfilling width/height data on samples that were
    /// uploaded before dimension extraction was added, or where dimensions
    /// could not be determined at upload time.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The dataset containing the samples
    /// * `updates` - List of dimension updates (sample ID, width, height)
    ///
    /// # Returns
    ///
    /// The number of samples that were successfully updated.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, updates), fields(dataset_id = %dataset_id, count = updates.len())))]
    pub async fn update_sample_dimensions(
        &self,
        dataset_id: DatasetID,
        updates: Vec<crate::SampleDimensionUpdate>,
    ) -> Result<u64, Error> {
        use crate::api::SamplesUpdateDimensionsParams;

        if updates.is_empty() {
            return Ok(0);
        }

        // Batch in groups of 500 to stay within server limits
        let mut total_updated = 0u64;
        for chunk in updates.chunks(500) {
            let params = SamplesUpdateDimensionsParams {
                dataset_id,
                samples: chunk.to_vec(),
            };
            let result: crate::SamplesUpdateDimensionsResult = self
                .rpc("samples.update_dimensions".to_owned(), Some(params))
                .await?;
            total_updated += result.updated;
        }
        Ok(total_updated)
    }

    /// Backfill missing image dimensions for a dataset.
    ///
    /// Downloads image data for samples that are missing width/height,
    /// extracts the dimensions using the `imagesize` crate, and updates
    /// the server with the computed values.
    ///
    /// This is a one-time repair operation for datasets that were uploaded
    /// before the client added automatic dimension extraction.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The dataset to backfill
    /// * `progress` - Optional progress channel
    ///
    /// # Returns
    ///
    /// The number of samples whose dimensions were updated.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, progress), fields(dataset_id = %dataset_id)))]
    pub async fn backfill_sample_dimensions(
        &self,
        dataset_id: DatasetID,
        progress: Option<Sender<Progress>>,
    ) -> Result<u64, Error> {
        // Fetch all samples; listing progress is not forwarded to the caller
        // since it would interleave with the dimension-computing phase.
        let samples = self.samples(dataset_id, None, &[], &[], &[], None).await?;

        // Filter to samples missing dimensions
        let missing: Vec<&Sample> = samples
            .iter()
            .filter(|s| s.width.is_none() || s.height.is_none())
            .collect();

        if missing.is_empty() {
            return Ok(0);
        }

        let total = missing.len();
        let mut updates: Vec<crate::SampleDimensionUpdate> = Vec::with_capacity(total);

        for (i, sample) in missing.into_iter().enumerate() {
            let current = i + 1;

            let Some(id) = sample.id() else {
                Self::send_progress(&progress, current, total).await;
                continue;
            };

            let Some(url) = sample.image_url() else {
                #[cfg(feature = "profiling")]
                tracing::warn!(sample_id = %id, "skipping sample: no image URL");
                Self::send_progress(&progress, current, total).await;
                continue;
            };

            // Download image data to determine dimensions
            let resp = self.bulk_http.get(url).send().await;
            let Ok(resp) = resp else {
                #[cfg(feature = "profiling")]
                tracing::warn!(sample_id = %id, "skipping sample: download failed");
                Self::send_progress(&progress, current, total).await;
                continue;
            };

            // Skip non-success responses (e.g. 404, 500) rather than parsing error pages
            if !resp.status().is_success() {
                #[cfg(feature = "profiling")]
                tracing::warn!(sample_id = %id, status = %resp.status(), "skipping sample: non-success HTTP status");
                Self::send_progress(&progress, current, total).await;
                continue;
            }

            let Ok(bytes) = resp.bytes().await else {
                #[cfg(feature = "profiling")]
                tracing::warn!(sample_id = %id, "skipping sample: failed to read response body");
                Self::send_progress(&progress, current, total).await;
                continue;
            };

            // Extract dimensions from the downloaded image
            let Ok(size) = imagesize::blob_size(&bytes) else {
                #[cfg(feature = "profiling")]
                tracing::warn!(sample_id = %id, "skipping sample: could not determine dimensions");
                Self::send_progress(&progress, current, total).await;
                continue;
            };

            let (Ok(width), Ok(height)) = (u32::try_from(size.width), u32::try_from(size.height))
            else {
                #[cfg(feature = "profiling")]
                tracing::warn!(sample_id = %id, width = size.width, height = size.height, "skipping sample: dimensions overflow u32");
                Self::send_progress(&progress, current, total).await;
                continue;
            };

            updates.push(crate::SampleDimensionUpdate { id, width, height });
            Self::send_progress(&progress, current, total).await;
        }

        // Send updates to server
        self.update_sample_dimensions(dataset_id, updates).await
    }

    /// Emit a progress event if a progress channel is provided.
    async fn send_progress(progress: &Option<Sender<Progress>>, current: usize, total: usize) {
        if let Some(tx) = progress {
            let _ = tx
                .send(Progress {
                    current,
                    total,
                    status: Some("Computing dimensions".to_string()),
                })
                .await;
        }
    }

    /// List available snapshots.  If a name is provided, only snapshots
    /// containing that name are returned.
    ///
    /// Results are sorted by match quality: exact matches first, then
    /// case-insensitive exact matches, then shorter descriptions (more
    /// specific), then alphabetically.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn snapshots(&self, name: Option<&str>) -> Result<Vec<Snapshot>, Error> {
        let snapshots: Vec<Snapshot> = self
            .rpc::<(), Vec<Snapshot>>("snapshots.list".to_owned(), None)
            .await?;
        if let Some(name) = name {
            Ok(filter_and_sort_by_name(snapshots, name, |s| {
                s.description()
            }))
        } else {
            Ok(snapshots)
        }
    }

    /// Get the snapshot with the specified id.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(snapshot_id = %snapshot_id)))]
    pub async fn snapshot(&self, snapshot_id: SnapshotID) -> Result<Snapshot, Error> {
        let params = HashMap::from([("snapshot_id", snapshot_id)]);
        self.rpc("snapshots.get".to_owned(), Some(params)).await
    }

    /// Create a new snapshot from an MCAP file or EdgeFirst Dataset directory.
    ///
    /// Snapshots are frozen datasets in EdgeFirst Dataset Format (Zip/Arrow
    /// pairs) that serve two primary purposes:
    ///
    /// 1. **MCAP uploads**: Upload MCAP files containing sensor data (images,
    ///    point clouds, IMU, GPS) to EdgeFirst Studio. Snapshots can then be
    ///    restored with AGTG (Automatic Ground Truth Generation) and optional
    ///    auto-depth processing.
    ///
    /// 2. **Dataset exchange**: Export datasets for backup, sharing, or
    ///    migration between EdgeFirst Studio instances using the create →
    ///    download → upload → restore workflow.
    ///
    /// Large files are automatically chunked into 100MB parts and uploaded
    /// concurrently using S3 multipart upload with presigned URLs. Each chunk
    /// is streamed without loading into memory, maintaining constant memory
    /// usage.
    ///
    /// **Concurrency tuning**: Set `MAX_TASKS` to control concurrent
    /// uploads (default: half of CPU cores, min 2, max 8). Lower values work
    /// better for large files to avoid timeout issues. Higher values (16-32)
    /// are better for many small files.
    ///
    /// # Arguments
    ///
    /// * `path` - Local file path to MCAP file or directory containing
    ///   EdgeFirst Dataset Format files (Zip/Arrow pairs)
    /// * `progress` - Optional channel to receive upload progress updates
    ///
    /// # Progress
    ///
    /// Reports progress with `status: None` as file data is uploaded. Progress
    /// unit is bytes uploaded. For single files, total is the file size. For
    /// directories, total is the combined size of all files.
    ///
    /// # Returns
    ///
    /// Returns a `Snapshot` object with ID, description, status, path, and
    /// creation timestamp on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Path doesn't exist or contains invalid UTF-8
    /// * File format is invalid (not MCAP or EdgeFirst Dataset Format)
    /// * Upload fails or network error occurs
    /// * Server rejects the snapshot
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use edgefirst_client::{Client, Progress};
    /// # use tokio::sync::mpsc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new()?.with_token_path(None)?;
    ///
    /// // Upload MCAP file with progress tracking
    /// let (tx, mut rx) = mpsc::channel(1);
    /// tokio::spawn(async move {
    ///     while let Some(Progress {
    ///         current,
    ///         total,
    ///         status,
    ///     }) = rx.recv().await
    ///     {
    ///         println!(
    ///             "{}: {}/{} bytes ({:.1}%)",
    ///             status.as_deref().unwrap_or("Upload"),
    ///             current,
    ///             total,
    ///             (current as f64 / total as f64) * 100.0
    ///         );
    ///     }
    /// });
    /// let snapshot = client.create_snapshot("data.mcap", Some(tx)).await?;
    /// println!("Created snapshot: {:?}", snapshot.id());
    ///
    /// // Upload dataset directory (no progress)
    /// let snapshot = client.create_snapshot("./dataset_export/", None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// * [`restore_snapshot`](Self::restore_snapshot) - Restore snapshot to
    ///   dataset
    /// * [`download_snapshot`](Self::download_snapshot) - Download snapshot
    ///   data
    /// * [`delete_snapshot`](Self::delete_snapshot) - Delete snapshot
    /// * [AGTG Documentation](https://doc.edgefirst.ai/latest/datasets/tutorials/annotations/automatic/)
    /// * [Snapshots Guide](https://doc.edgefirst.ai/latest/studio/snapshots/)
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, progress)))]
    pub async fn create_snapshot(
        &self,
        path: &str,
        progress: Option<Sender<Progress>>,
    ) -> Result<Snapshot, Error> {
        let path = Path::new(path);

        if path.is_dir() {
            let path_str = path.to_str().ok_or_else(|| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Path contains invalid UTF-8",
                ))
            })?;
            return self.create_snapshot_folder(path_str, progress).await;
        }

        let name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid filename",
            ))
        })?;
        let total = path.metadata()?.len() as usize;
        let current = Arc::new(AtomicUsize::new(0));

        if let Some(progress) = &progress {
            let _ = progress
                .send(Progress {
                    current: 0,
                    total,
                    status: None,
                })
                .await;
        }

        let params = SnapshotCreateMultipartParams {
            snapshot_name: name.to_owned(),
            keys: vec![name.to_owned()],
            file_sizes: vec![total],
            snapshot_type: None,
        };
        let multipart: HashMap<String, SnapshotCreateMultipartResultField> = self
            .rpc(
                "snapshots.create_upload_url_multipart".to_owned(),
                Some(params),
            )
            .await?;

        let snapshot_id = match multipart.get("snapshot_id") {
            Some(SnapshotCreateMultipartResultField::Id(id)) => SnapshotID::from(*id),
            _ => return Err(Error::InvalidResponse),
        };

        let snapshot = self.snapshot(snapshot_id).await?;
        let part_prefix = snapshot
            .path()
            .split("::/")
            .last()
            .ok_or(Error::InvalidResponse)?
            .to_owned();
        let part_key = format!("{}/{}", part_prefix, name);
        let mut part = match multipart.get(&part_key) {
            Some(SnapshotCreateMultipartResultField::Part(part)) => part,
            _ => return Err(Error::InvalidResponse),
        }
        .clone();
        part.key = Some(part_key);

        let params = upload_multipart(
            self.bulk_http.clone(),
            part.clone(),
            path.to_path_buf(),
            total,
            current,
            progress.clone(),
        )
        .await?;

        let complete: String = self
            .rpc(
                "snapshots.complete_multipart_upload".to_owned(),
                Some(params),
            )
            .await?;
        debug!("Snapshot Multipart Complete: {:?}", complete);

        let params: SnapshotStatusParams = SnapshotStatusParams {
            snapshot_id,
            status: "available".to_owned(),
        };
        let _: SnapshotStatusResult = self
            .rpc("snapshots.update".to_owned(), Some(params))
            .await?;

        if let Some(progress) = progress {
            drop(progress);
        }

        self.snapshot(snapshot_id).await
    }

    async fn create_snapshot_folder(
        &self,
        path: &str,
        progress: Option<Sender<Progress>>,
    ) -> Result<Snapshot, Error> {
        let path = Path::new(path);
        let name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid directory name",
            ))
        })?;

        let files = WalkDir::new(path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| entry.path().strip_prefix(path).ok().map(|p| p.to_owned()))
            .collect::<Vec<_>>();

        let total: usize = files
            .iter()
            .filter_map(|file| path.join(file).metadata().ok())
            .map(|metadata| metadata.len() as usize)
            .sum();
        let current = Arc::new(AtomicUsize::new(0));

        if let Some(progress) = &progress {
            let _ = progress
                .send(Progress {
                    current: 0,
                    total,
                    status: None,
                })
                .await;
        }

        let keys = files
            .iter()
            .filter_map(|key| key.to_str().map(|s| s.to_owned()))
            .collect::<Vec<_>>();
        let file_sizes = files
            .iter()
            .filter_map(|key| path.join(key).metadata().ok())
            .map(|metadata| metadata.len() as usize)
            .collect::<Vec<_>>();

        let params = SnapshotCreateMultipartParams {
            snapshot_name: name.to_owned(),
            keys,
            file_sizes,
            snapshot_type: None,
        };

        let multipart: HashMap<String, SnapshotCreateMultipartResultField> = self
            .rpc(
                "snapshots.create_upload_url_multipart".to_owned(),
                Some(params),
            )
            .await?;

        let snapshot_id = match multipart.get("snapshot_id") {
            Some(SnapshotCreateMultipartResultField::Id(id)) => SnapshotID::from(*id),
            _ => return Err(Error::InvalidResponse),
        };

        let snapshot = self.snapshot(snapshot_id).await?;
        let part_prefix = snapshot
            .path()
            .split("::/")
            .last()
            .ok_or(Error::InvalidResponse)?
            .to_owned();

        for file in files {
            let file_str = file.to_str().ok_or_else(|| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "File path contains invalid UTF-8",
                ))
            })?;
            let part_key = format!("{}/{}", part_prefix, file_str);
            let mut part = match multipart.get(&part_key) {
                Some(SnapshotCreateMultipartResultField::Part(part)) => part,
                _ => return Err(Error::InvalidResponse),
            }
            .clone();
            part.key = Some(part_key);

            let params = upload_multipart(
                self.bulk_http.clone(),
                part.clone(),
                path.join(file),
                total,
                current.clone(),
                progress.clone(),
            )
            .await?;

            let complete: String = self
                .rpc(
                    "snapshots.complete_multipart_upload".to_owned(),
                    Some(params),
                )
                .await?;
            debug!("Snapshot Part Complete: {:?}", complete);
        }

        let params = SnapshotStatusParams {
            snapshot_id,
            status: "available".to_owned(),
        };
        let _: SnapshotStatusResult = self
            .rpc("snapshots.update".to_owned(), Some(params))
            .await?;

        if let Some(progress) = progress {
            drop(progress);
        }

        self.snapshot(snapshot_id).await
    }

    /// Create a snapshot from EdgeFirst Dataset Format files (.arrow + .zip).
    ///
    /// Uploads a paired Arrow manifest and ZIP archive as a single snapshot.
    /// This format is the native EdgeFirst Dataset Format used for efficient
    /// dataset storage and transfer.
    ///
    /// # Arguments
    ///
    /// * `arrow_path` - Path to the Arrow manifest file (.arrow)
    /// * `zip_path` - Path to the ZIP archive containing images (.zip)
    /// * `description` - Optional description for the snapshot
    /// * `progress` - Optional progress channel for upload tracking
    ///
    /// # File Requirements
    ///
    /// - Arrow file must have `.arrow` extension
    /// - ZIP file must have `.zip` extension
    /// - Both files must exist and be readable
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use edgefirst_client::Client;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new()?.with_token_path(None)?;
    ///
    /// let snapshot = client
    ///     .create_snapshot_edgefirst_format(
    ///         "dataset.arrow",
    ///         "dataset.zip",
    ///         Some("My Dataset Snapshot"),
    ///         None,
    ///     )
    ///     .await?;
    /// println!("Created snapshot: {}", snapshot.id());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// * [`create_snapshot`](Self::create_snapshot) - Upload single file or
    ///   folder
    /// * [`restore_snapshot`](Self::restore_snapshot) - Restore snapshot to
    ///   dataset
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, progress)))]
    pub async fn create_snapshot_edgefirst_format(
        &self,
        arrow_path: &str,
        zip_path: &str,
        description: Option<&str>,
        progress: Option<Sender<Progress>>,
    ) -> Result<Snapshot, Error> {
        let arrow_path = Path::new(arrow_path);
        let zip_path = Path::new(zip_path);

        // Validate files exist
        if !arrow_path.exists() {
            return Err(Error::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Arrow file not found: {}", arrow_path.display()),
            )));
        }
        if !zip_path.exists() {
            return Err(Error::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("ZIP file not found: {}", zip_path.display()),
            )));
        }

        // Get file names
        let arrow_name = arrow_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid Arrow filename",
                ))
            })?;
        let zip_name = zip_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid ZIP filename",
                ))
            })?;

        // Generate snapshot name from arrow file (without extension)
        let snapshot_name = description
            .map(|s| s.to_string())
            .or_else(|| {
                arrow_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "edgefirst_dataset".to_string());

        // Calculate file sizes
        let arrow_size = arrow_path.metadata()?.len() as usize;
        let zip_size = zip_path.metadata()?.len() as usize;
        let total = arrow_size + zip_size;
        let current = Arc::new(AtomicUsize::new(0));

        if let Some(progress) = &progress {
            let _ = progress
                .send(Progress {
                    current: 0,
                    total,
                    status: None,
                })
                .await;
        }

        // Create multipart upload request with "ziparrow" type
        let params = SnapshotCreateMultipartParams {
            snapshot_name,
            keys: vec![arrow_name.to_owned(), zip_name.to_owned()],
            file_sizes: vec![arrow_size, zip_size],
            snapshot_type: Some("ziparrow".to_string()),
        };

        let multipart: HashMap<String, SnapshotCreateMultipartResultField> = self
            .rpc(
                "snapshots.create_upload_url_multipart".to_owned(),
                Some(params),
            )
            .await?;

        let snapshot_id = match multipart.get("snapshot_id") {
            Some(SnapshotCreateMultipartResultField::Id(id)) => SnapshotID::from(*id),
            _ => return Err(Error::InvalidResponse),
        };

        let snapshot = self.snapshot(snapshot_id).await?;
        let part_prefix = snapshot
            .path()
            .split("::/")
            .last()
            .ok_or(Error::InvalidResponse)?
            .to_owned();

        // Upload Arrow file
        let arrow_key = format!("{}/{}", part_prefix, arrow_name);
        let mut arrow_part = match multipart.get(&arrow_key) {
            Some(SnapshotCreateMultipartResultField::Part(part)) => part.clone(),
            _ => return Err(Error::InvalidResponse),
        };
        arrow_part.key = Some(arrow_key);

        let params = upload_multipart(
            self.bulk_http.clone(),
            arrow_part,
            arrow_path.to_path_buf(),
            total,
            current.clone(),
            progress.clone(),
        )
        .await?;

        let _: String = self
            .rpc(
                "snapshots.complete_multipart_upload".to_owned(),
                Some(params),
            )
            .await?;
        debug!("Arrow file upload complete");

        // Upload ZIP file
        let zip_key = format!("{}/{}", part_prefix, zip_name);
        let mut zip_part = match multipart.get(&zip_key) {
            Some(SnapshotCreateMultipartResultField::Part(part)) => part.clone(),
            _ => return Err(Error::InvalidResponse),
        };
        zip_part.key = Some(zip_key);

        let params = upload_multipart(
            self.bulk_http.clone(),
            zip_part,
            zip_path.to_path_buf(),
            total,
            current.clone(),
            progress.clone(),
        )
        .await?;

        let _: String = self
            .rpc(
                "snapshots.complete_multipart_upload".to_owned(),
                Some(params),
            )
            .await?;
        debug!("ZIP file upload complete");

        // Mark snapshot as available
        let params = SnapshotStatusParams {
            snapshot_id,
            status: "available".to_owned(),
        };
        let _: SnapshotStatusResult = self
            .rpc("snapshots.update".to_owned(), Some(params))
            .await?;

        if let Some(progress) = progress {
            drop(progress);
        }

        self.snapshot(snapshot_id).await
    }

    /// Delete a snapshot from EdgeFirst Studio.
    ///
    /// Permanently removes a snapshot and its associated data. This operation
    /// cannot be undone.
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - The snapshot ID to delete
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Snapshot doesn't exist
    /// * User lacks permission to delete the snapshot
    /// * Server error occurs
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use edgefirst_client::{Client, SnapshotID};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new()?.with_token_path(None)?;
    /// let snapshot_id = SnapshotID::from(123);
    /// client.delete_snapshot(snapshot_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// * [`create_snapshot`](Self::create_snapshot) - Upload snapshot
    /// * [`snapshots`](Self::snapshots) - List all snapshots
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(snapshot_id = %snapshot_id)))]
    pub async fn delete_snapshot(&self, snapshot_id: SnapshotID) -> Result<(), Error> {
        let params = HashMap::from([("snapshot_id", snapshot_id)]);
        let _: serde_json::Value = self
            .rpc("snapshots.delete".to_owned(), Some(params))
            .await?;
        Ok(())
    }

    /// Create a snapshot from an existing dataset on the server.
    ///
    /// Triggers server-side snapshot generation which exports the dataset's
    /// images and annotations into a downloadable EdgeFirst Dataset Format
    /// snapshot.
    ///
    /// This is the inverse of [`restore_snapshot`](Self::restore_snapshot) -
    /// while restore creates a dataset from a snapshot, this method creates a
    /// snapshot from a dataset.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The dataset ID to create snapshot from
    /// * `description` - Description for the created snapshot
    ///
    /// # Returns
    ///
    /// Returns a `SnapshotCreateResult` containing the snapshot ID and task ID
    /// for monitoring progress.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Dataset doesn't exist
    /// * User lacks permission to access the dataset
    /// * Server rejects the request
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use edgefirst_client::{Client, DatasetID};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new()?.with_token_path(None)?;
    /// let dataset_id = DatasetID::from(123);
    ///
    /// // Create snapshot from dataset (all annotation sets)
    /// let result = client
    ///     .create_snapshot_from_dataset(dataset_id, "My Dataset Backup", None)
    ///     .await?;
    /// println!("Created snapshot: {:?}", result.id);
    ///
    /// // Monitor progress via task ID
    /// if let Some(task_id) = result.task_id {
    ///     println!("Task: {}", task_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// * [`create_snapshot`](Self::create_snapshot) - Upload local files as
    ///   snapshot
    /// * [`restore_snapshot`](Self::restore_snapshot) - Restore snapshot to
    ///   dataset
    /// * [`download_snapshot`](Self::download_snapshot) - Download snapshot
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(dataset_id = %dataset_id)))]
    pub async fn create_snapshot_from_dataset(
        &self,
        dataset_id: DatasetID,
        description: &str,
        annotation_set_id: Option<AnnotationSetID>,
    ) -> Result<SnapshotFromDatasetResult, Error> {
        // Resolve annotation_set_id: use provided value or fetch default
        let annotation_set_id = match annotation_set_id {
            Some(id) => id,
            None => {
                // Fetch annotation sets and find default ("annotations") or use first
                let sets = self.annotation_sets(dataset_id).await?;
                if sets.is_empty() {
                    return Err(Error::InvalidParameters(
                        "No annotation sets available for dataset".to_owned(),
                    ));
                }
                // Look for "annotations" set (default), otherwise use first
                sets.iter()
                    .find(|s| s.name() == "annotations")
                    .unwrap_or(&sets[0])
                    .id()
            }
        };
        let params = SnapshotCreateFromDataset {
            description: description.to_owned(),
            dataset_id,
            annotation_set_id,
        };
        self.rpc("snapshots.create".to_owned(), Some(params)).await
    }

    /// Download a snapshot from EdgeFirst Studio to local storage.
    ///
    /// Downloads all files in a snapshot (single MCAP file or directory of
    /// EdgeFirst Dataset Format files) to the specified output path. Files are
    /// downloaded concurrently with progress tracking.
    ///
    /// **Concurrency tuning**: Set `MAX_TASKS` to control concurrent
    /// downloads (default: half of CPU cores, min 2, max 8).
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - The snapshot ID to download
    /// * `output` - Local directory path to save downloaded files
    /// * `progress` - Optional channel to receive download progress updates
    ///
    /// # Progress
    ///
    /// Reports progress with `status: None` as file data is received. Progress
    /// unit is bytes downloaded across all files combined. The total
    /// accumulates as file sizes become known (from HTTP Content-Length
    /// headers), so both `current` and `total` may increase during
    /// download.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Snapshot doesn't exist
    /// * Output directory cannot be created
    /// * Download fails or network error occurs
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use edgefirst_client::{Client, SnapshotID, Progress};
    /// # use tokio::sync::mpsc;
    /// # use std::path::PathBuf;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new()?.with_token_path(None)?;
    /// let snapshot_id = SnapshotID::from(123);
    ///
    /// // Download with progress tracking
    /// let (tx, mut rx) = mpsc::channel(1);
    /// tokio::spawn(async move {
    ///     while let Some(Progress {
    ///         current,
    ///         total,
    ///         status,
    ///     }) = rx.recv().await
    ///     {
    ///         println!(
    ///             "{}: {}/{} bytes",
    ///             status.as_deref().unwrap_or("Download"),
    ///             current,
    ///             total
    ///         );
    ///     }
    /// });
    /// client
    ///     .download_snapshot(snapshot_id, PathBuf::from("./output"), Some(tx))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// * [`create_snapshot`](Self::create_snapshot) - Upload snapshot
    /// * [`restore_snapshot`](Self::restore_snapshot) - Restore snapshot to
    ///   dataset
    /// * [`delete_snapshot`](Self::delete_snapshot) - Delete snapshot
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, progress), fields(snapshot_id = %snapshot_id, output = %output.display())))]
    pub async fn download_snapshot(
        &self,
        snapshot_id: SnapshotID,
        output: PathBuf,
        progress: Option<Sender<Progress>>,
    ) -> Result<(), Error> {
        fs::create_dir_all(&output).await?;

        let params = HashMap::from([("snapshot_id", snapshot_id)]);
        let items: HashMap<String, String> = self
            .rpc("snapshots.create_download_url".to_owned(), Some(params))
            .await?;

        // Single-phase: each task holds its semaphore permit for the full
        // lifetime of the request (GET → headers → stream → disk). This bounds
        // the number of simultaneously-open connections to max_tasks() and
        // avoids accumulating all responses in memory before streaming.
        //
        // total is updated atomically as each response's Content-Length header
        // arrives, so progress tracking is accurate without a separate phase.
        let http = self.bulk_http.clone();
        let current = Arc::new(AtomicUsize::new(0));
        let total = Arc::new(AtomicUsize::new(0));
        let sem = Arc::new(Semaphore::new(max_tasks()));

        let tasks = items
            .into_iter()
            .map(|(key, url)| {
                let http = http.clone();
                let output = output.clone();
                let progress = progress.clone();
                let current = current.clone();
                let total = total.clone();
                let sem = sem.clone();

                tokio::spawn(async move {
                    let _permit = sem.acquire().await.map_err(|_| {
                        Error::IoError(std::io::Error::other("Semaphore closed unexpectedly"))
                    })?;

                    let res = http.get(url).send().await?;
                    let res = res.error_for_status()?;

                    // Contribute this file's size to the running total so the
                    // caller's progress bar knows the overall scope.
                    if let Some(len) = res.content_length() {
                        total.fetch_add(len as usize, Ordering::SeqCst);
                    }

                    let mut file = File::create(output.join(key)).await?;
                    let mut stream = res.bytes_stream();

                    while let Some(chunk) = stream.next().await {
                        let chunk = chunk?;
                        file.write_all(&chunk).await?;
                        let len = chunk.len();

                        if let Some(progress) = &progress {
                            let cur = current.fetch_add(len, Ordering::SeqCst) + len;
                            let tot = total.load(Ordering::SeqCst);
                            let _ = progress
                                .send(Progress {
                                    current: cur,
                                    total: tot,
                                    status: None,
                                })
                                .await;
                        }
                    }

                    Ok::<(), Error>(())
                })
            })
            .collect::<Vec<_>>();

        join_all(tasks)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    /// Restore a snapshot to a dataset in EdgeFirst Studio with optional AGTG.
    ///
    /// Restores a snapshot (MCAP file or EdgeFirst Dataset) into a dataset in
    /// the specified project. For MCAP files, supports:
    ///
    /// * **AGTG (Automatic Ground Truth Generation)**: Automatically annotate
    ///   detected objects with 2D masks/boxes and 3D boxes (if radar/LiDAR
    ///   present)
    /// * **Auto-depth**: Generate depthmaps (Maivin/Raivin cameras only)
    /// * **Topic filtering**: Select specific MCAP topics to restore
    ///
    /// For EdgeFirst Dataset snapshots, this simply imports the pre-existing
    /// dataset structure.
    ///
    /// # Arguments
    ///
    /// * `project_id` - Target project ID
    /// * `snapshot_id` - Snapshot ID to restore
    /// * `topics` - MCAP topics to include (empty = all topics)
    /// * `autolabel` - Object labels for AGTG (empty = no auto-annotation)
    /// * `autodepth` - Generate depthmaps (Maivin/Raivin only)
    /// * `dataset_name` - Optional custom dataset name
    /// * `dataset_description` - Optional dataset description
    ///
    /// # Returns
    ///
    /// Returns a `SnapshotRestoreResult` with the new dataset ID and status.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Snapshot or project doesn't exist
    /// * Snapshot format is invalid
    /// * Server rejects restoration parameters
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use edgefirst_client::{Client, ProjectID, SnapshotID};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new()?.with_token_path(None)?;
    /// let project_id = ProjectID::from(1);
    /// let snapshot_id = SnapshotID::from(123);
    ///
    /// // Restore MCAP with AGTG for "person" and "car" detection
    /// let result = client
    ///     .restore_snapshot(
    ///         project_id,
    ///         snapshot_id,
    ///         &[],                                        // All topics
    ///         &["person".to_string(), "car".to_string()], // AGTG labels
    ///         true,                                       // Auto-depth
    ///         Some("Highway Dataset"),
    ///         Some("Collected on I-95"),
    ///     )
    ///     .await?;
    /// println!("Restored to dataset: {:?}", result.dataset_id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// * [`create_snapshot`](Self::create_snapshot) - Upload snapshot
    /// * [`download_snapshot`](Self::download_snapshot) - Download snapshot
    /// * [AGTG Documentation](https://doc.edgefirst.ai/latest/datasets/tutorials/annotations/automatic/)
    #[allow(clippy::too_many_arguments)]
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn restore_snapshot(
        &self,
        project_id: ProjectID,
        snapshot_id: SnapshotID,
        topics: &[String],
        autolabel: &[String],
        autodepth: bool,
        dataset_name: Option<&str>,
        dataset_description: Option<&str>,
    ) -> Result<SnapshotRestoreResult, Error> {
        let params = SnapshotRestore {
            project_id,
            snapshot_id,
            fps: 1,
            autodepth,
            agtg_pipeline: !autolabel.is_empty(),
            autolabel: autolabel.to_vec(),
            topics: topics.to_vec(),
            dataset_name: dataset_name.map(|s| s.to_owned()),
            dataset_description: dataset_description.map(|s| s.to_owned()),
        };
        self.rpc("snapshots.restore".to_owned(), Some(params)).await
    }

    /// Returns a list of experiments available to the user.  The experiments
    /// are returned as a vector of Experiment objects.  If name is provided
    /// then only experiments containing this string are returned.
    ///
    /// Results are sorted by match quality: exact matches first, then
    /// case-insensitive exact matches, then shorter names (more specific),
    /// then alphabetically.
    ///
    /// Experiments provide a method of organizing training and validation
    /// sessions together and are akin to an Experiment in MLFlow terminology.  
    /// Each experiment can have multiple trainer sessions associated with it,
    /// these would be akin to runs in MLFlow terminology.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn experiments(
        &self,
        project_id: ProjectID,
        name: Option<&str>,
    ) -> Result<Vec<Experiment>, Error> {
        let params = HashMap::from([("project_id", project_id)]);
        let experiments: Vec<Experiment> =
            self.rpc("trainer.list2".to_owned(), Some(params)).await?;
        if let Some(name) = name {
            Ok(filter_and_sort_by_name(experiments, name, |e| e.name()))
        } else {
            Ok(experiments)
        }
    }

    /// Return the experiment with the specified experiment ID.  If the
    /// experiment does not exist, an error is returned.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn experiment(&self, experiment_id: ExperimentID) -> Result<Experiment, Error> {
        let params = HashMap::from([("trainer_id", experiment_id)]);
        self.rpc("trainer.get".to_owned(), Some(params)).await
    }

    /// Returns a list of trainer sessions available to the user.  The trainer
    /// sessions are returned as a vector of TrainingSession objects.  If name
    /// is provided then only trainer sessions containing this string are
    /// returned.
    ///
    /// Results are sorted by match quality: exact matches first, then
    /// case-insensitive exact matches, then shorter names (more specific),
    /// then alphabetically.
    ///
    /// Trainer sessions are akin to runs in MLFlow terminology.  These
    /// represent an actual training session which will produce metrics and
    /// model artifacts.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn training_sessions(
        &self,
        experiment_id: ExperimentID,
        name: Option<&str>,
    ) -> Result<Vec<TrainingSession>, Error> {
        let params = HashMap::from([("trainer_id", experiment_id)]);
        let sessions: Vec<TrainingSession> = self
            .rpc("trainer.session.list".to_owned(), Some(params))
            .await?;
        if let Some(name) = name {
            Ok(filter_and_sort_by_name(sessions, name, |s| s.name()))
        } else {
            Ok(sessions)
        }
    }

    /// Return the trainer session with the specified trainer session ID.  If
    /// the trainer session does not exist, an error is returned.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn training_session(
        &self,
        session_id: TrainingSessionID,
    ) -> Result<TrainingSession, Error> {
        let params = HashMap::from([("trainer_session_id", session_id)]);
        self.rpc("trainer.session.get".to_owned(), Some(params))
            .await
    }

    /// List validation sessions for the given project.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn validation_sessions(
        &self,
        project_id: ProjectID,
    ) -> Result<Vec<ValidationSession>, Error> {
        let params = HashMap::from([("project_id", project_id)]);
        self.rpc("validate.session.list".to_owned(), Some(params))
            .await
    }

    /// Retrieve a specific validation session.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn validation_session(
        &self,
        session_id: ValidationSessionID,
    ) -> Result<ValidationSession, Error> {
        let params = HashMap::from([("validate_session_id", session_id)]);
        self.rpc("validate.session.get".to_owned(), Some(params))
            .await
    }

    /// Create a new validation session via Studio's `cloud.server.start`.
    ///
    /// Pass `is_local: true` in the [`StartValidationRequest`] to create
    /// a **user-managed** session: the database row is created and the
    /// session is fully usable for data uploads / downloads / metrics,
    /// but no EC2 instance is provisioned and no automated validator
    /// pipeline is started. That is the mode our integration tests use
    /// — they create a session, exercise the wrapper APIs against it,
    /// then call [`Client::delete_validation_sessions`] in teardown so
    /// no stray sessions accumulate on the test account.
    ///
    /// Returns a [`NewValidationSession`] carrying the backing task id
    /// and the freshly-minted validation session id.
    ///
    /// # Errors
    ///
    /// Surfaces any RPC error from `cloud.server.start`. Common cases:
    /// `RpcError(101, …)` if a required entity is missing (project,
    /// training session, dataset, …); `PermissionDenied` if the caller
    /// can't write to the target project.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, req)))]
    pub async fn start_validation_session(
        &self,
        req: StartValidationRequest,
    ) -> Result<NewValidationSession, Error> {
        // Build the params shape the server expects. `cloud.server.start`
        // is intentionally generic — different server types pull
        // different fields out of `params` — so we serialize manually to
        // match the JS frontend's call site verbatim (see
        // `dve-frontend/src/components/ValidationPage/StartValidatorModal.vue`).
        let mut body = serde_json::Map::new();
        body.insert(
            "type".into(),
            serde_json::Value::String("validation".into()),
        );
        body.insert("name".into(), serde_json::Value::String(req.name));
        body.insert("project_id".into(), serde_json::to_value(req.project_id)?);
        body.insert(
            "training_session_id".into(),
            serde_json::to_value(req.training_session_id)?,
        );
        body.insert(
            "model_file".into(),
            serde_json::Value::String(req.model_file),
        );
        body.insert("val_type".into(), serde_json::Value::String(req.val_type));
        body.insert("is_local".into(), serde_json::Value::Bool(req.is_local));
        body.insert(
            "is_kubernetes".into(),
            serde_json::Value::Bool(req.is_kubernetes),
        );

        // `validate.session` reads its config from `params.params` (one
        // extra envelope level). The outer `params` wrapper is required
        // even when the inner map is empty.
        let inner = serde_json::to_value(req.params)?;
        let mut outer = serde_json::Map::new();
        outer.insert("params".into(), inner);
        body.insert("params".into(), serde_json::Value::Object(outer));

        if let Some(d) = req.description {
            body.insert("description".into(), serde_json::Value::String(d));
        }
        if let Some(id) = req.dataset_id {
            body.insert("dataset_id".into(), serde_json::to_value(id)?);
        }
        if let Some(id) = req.annotation_set_id {
            body.insert("annotation_set_id".into(), serde_json::to_value(id)?);
        }
        if let Some(id) = req.snapshot_id {
            body.insert("snapshot_id".into(), serde_json::to_value(id)?);
        }

        self.rpc("cloud.server.start".to_owned(), Some(body)).await
    }

    /// Delete one or more validation sessions via
    /// `validate.session.delete`.
    ///
    /// Used by integration tests to tear down sessions they created
    /// with [`Client::start_validation_session`]; idempotent against
    /// already-deleted ids on the server side (the RPC accepts the
    /// list, deletes what it can, and surfaces an error only if none
    /// of the ids were resolvable).
    ///
    /// # Errors
    ///
    /// Surfaces any RPC error from `validate.session.delete`. A
    /// `PermissionDenied` indicates the caller lacks
    /// `TrainerWrite` on at least one of the listed sessions.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn delete_validation_sessions(
        &self,
        session_ids: &[ValidationSessionID],
    ) -> Result<(), Error> {
        let mut body = serde_json::Map::new();
        body.insert("session_ids".into(), serde_json::to_value(session_ids)?);
        let _: serde_json::Value = self
            .rpc("validate.session.delete".to_owned(), Some(body))
            .await?;
        Ok(())
    }

    /// List the artifacts for the specified trainer session.  The artifacts
    /// are returned as a vector of strings.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn artifacts(
        &self,
        training_session_id: TrainingSessionID,
    ) -> Result<Vec<Artifact>, Error> {
        let params = HashMap::from([("training_session_id", training_session_id)]);
        self.rpc("trainer.get_artifacts".to_owned(), Some(params))
            .await
    }

    /// Download the model artifact for the specified trainer session to the
    /// specified file path, if path is not provided it will be downloaded to
    /// the current directory with the same filename.
    ///
    /// # Progress
    ///
    /// Reports progress with `status: None` as file data is received. Progress
    /// unit is bytes downloaded. Total is determined from the HTTP
    /// Content-Length header (may be 0 if server doesn't provide it).
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, progress), fields(training_session_id = %training_session_id)))]
    pub async fn download_artifact(
        &self,
        training_session_id: TrainingSessionID,
        modelname: &str,
        filename: Option<PathBuf>,
        progress: Option<Sender<Progress>>,
    ) -> Result<(), Error> {
        let filename = filename.unwrap_or_else(|| PathBuf::from(modelname));
        let resp = self
            .bulk_http
            .get(format!(
                "{}/download_model?training_session_id={}&file={}",
                self.url,
                training_session_id.value(),
                modelname
            ))
            .header("Authorization", format!("Bearer {}", self.token().await))
            .send()
            .await?;
        if !resp.status().is_success() {
            let err = resp.error_for_status_ref().unwrap_err();
            return Err(Error::HttpError(err));
        }

        if let Some(parent) = filename.parent() {
            fs::create_dir_all(parent).await?;
        }

        stream_response_to_file(resp, &filename, progress).await
    }

    /// Download the model checkpoint associated with the specified trainer
    /// session to the specified file path, if path is not provided it will be
    /// downloaded to the current directory with the same filename.
    ///
    /// There is no API for listing checkpoints it is expected that trainers are
    /// aware of possible checkpoints and their names within the checkpoint
    /// folder on the server.
    ///
    /// # Progress
    ///
    /// Reports progress with `status: None` as file data is received. Progress
    /// unit is bytes downloaded. Total is determined from the HTTP
    /// Content-Length header (may be 0 if server doesn't provide it).
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, progress), fields(training_session_id = %training_session_id)))]
    pub async fn download_checkpoint(
        &self,
        training_session_id: TrainingSessionID,
        checkpoint: &str,
        filename: Option<PathBuf>,
        progress: Option<Sender<Progress>>,
    ) -> Result<(), Error> {
        let filename = filename.unwrap_or_else(|| PathBuf::from(checkpoint));
        let resp = self
            .bulk_http
            .get(format!(
                "{}/download_checkpoint?folder=checkpoints&training_session_id={}&file={}",
                self.url,
                training_session_id.value(),
                checkpoint
            ))
            .header("Authorization", format!("Bearer {}", self.token().await))
            .send()
            .await?;
        if !resp.status().is_success() {
            let err = resp.error_for_status_ref().unwrap_err();
            return Err(Error::HttpError(err));
        }

        if let Some(parent) = filename.parent() {
            fs::create_dir_all(parent).await?;
        }

        stream_response_to_file(resp, &filename, progress).await
    }

    /// Return a list of tasks for the current user.
    ///
    /// # Arguments
    ///
    /// * `name` - Optional filter for task name (client-side substring match)
    /// * `workflow` - Optional filter for workflow/task type. If provided,
    ///   filters server-side by exact match. Valid values include: "trainer",
    ///   "validation", "snapshot-create", "snapshot-restore", "copyds",
    ///   "upload", "auto-ann", "auto-seg", "aigt", "import", "export",
    ///   "convertor", "twostage"
    /// * `status` - Optional filter for task status (e.g., "running",
    ///   "complete", "error")
    /// * `manager` - Optional filter for task manager type (e.g., "aws",
    ///   "user", "kubernetes")
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn tasks(
        &self,
        name: Option<&str>,
        workflow: Option<&str>,
        status: Option<&str>,
        manager: Option<&str>,
    ) -> Result<Vec<Task>, Error> {
        let mut params = TasksListParams {
            continue_token: None,
            types: workflow.map(|w| vec![w.to_owned()]),
            status: status.map(|s| vec![s.to_owned()]),
            manager: manager.map(|m| vec![m.to_owned()]),
        };
        let mut tasks = Vec::new();

        loop {
            let result = self
                .rpc::<_, TasksListResult>("task.list".to_owned(), Some(&params))
                .await?;
            tasks.extend(result.tasks);

            if result.continue_token.is_none() || result.continue_token == Some("".into()) {
                params.continue_token = None;
            } else {
                params.continue_token = result.continue_token;
            }

            if params.continue_token.is_none() {
                break;
            }
        }

        if let Some(name) = name {
            tasks = filter_and_sort_by_name(tasks, name, |t| t.name());
        }

        Ok(tasks)
    }

    /// Submits a job (app run) to the server and returns the resulting `Job`
    /// record (which carries the linked task id alongside the cloud-batch
    /// metadata).
    ///
    /// # Arguments
    /// * `app_name` - The name of the registered app to run (e.g., `"edgefirst-validator"`).
    /// * `job_name` - A user-defined label for this run.
    /// * `env` - Environment variables passed to the job (string-string map).
    /// * `data` - Job input payload (e.g., session ids, parameters).
    ///
    /// # Returns
    /// The full `Job` record returned by the server (wraps the BK_BATCH object),
    /// including AWS Batch job ID, state, and the linked `task_id`. Callers that
    /// only need the task ID can call `.task_id()` on the returned `Job`.
    pub async fn job_run(
        &self,
        app_name: &str,
        job_name: &str,
        env: std::collections::HashMap<String, String>,
        data: std::collections::HashMap<String, crate::api::Parameter>,
    ) -> Result<crate::api::Job, Error> {
        let req = JobRunRequest {
            name: app_name.to_owned(),
            job_name: job_name.to_owned(),
            env,
            data,
        };
        let resp: crate::api::Job = match self.rpc("job.run".to_owned(), Some(&req)).await {
            Ok(r) => r,
            Err(Error::RpcError(code, msg)) => {
                return Err(map_rpc_error("job.run", code, msg, None));
            }
            Err(e) => return Err(e),
        };
        Ok(resp)
    }

    /// Requests a running job task be stopped.
    ///
    /// Returns `Ok(())` if the stop request was accepted by the server. The
    /// task may still take time to fully terminate; poll `task_info` if you
    /// need to wait for shutdown.
    pub async fn job_stop(&self, task_id: crate::api::TaskID) -> Result<(), Error> {
        let req = JobStopRequest {
            task_id: task_id.value(),
        };
        // We don't care about the response body; deserialize as serde_json::Value.
        let _resp: serde_json::Value = match self.rpc("job.stop".to_owned(), Some(&req)).await {
            Ok(r) => r,
            Err(Error::RpcError(code, msg)) => {
                return Err(map_rpc_error("job.stop", code, msg, Some(task_id)));
            }
            Err(e) => return Err(e),
        };
        Ok(())
    }

    /// Lists job (app-run) entries visible to the authenticated user.
    ///
    /// The server returns AWS Batch-wrapper entries (not bare `Task` objects),
    /// surfacing cloud-batch state (`RUNNING`/`SUCCEEDED`/...) and the linked
    /// `task_id`. Use `Job::task_id()` + `Client::task_info` to fetch the
    /// underlying task details.
    ///
    /// The server does not support server-side filters, so the optional
    /// `name` argument is applied client-side as a substring match against
    /// each job's `job_name`.
    pub async fn jobs(&self, name: Option<&str>) -> Result<Vec<crate::api::Job>, Error> {
        let req = JobsListRequest {};
        let mut jobs: Vec<crate::api::Job> = match self.rpc("job.list".to_owned(), Some(&req)).await
        {
            Ok(r) => r,
            Err(Error::RpcError(code, msg)) => {
                return Err(map_rpc_error("job.list", code, msg, None));
            }
            Err(e) => return Err(e),
        };
        if let Some(name) = name {
            let needle = name.to_lowercase();
            jobs.retain(|j| j.job_name.to_lowercase().contains(&needle));
            jobs.sort_by(|a, b| a.job_name.cmp(&b.job_name));
        }
        Ok(jobs)
    }

    /// Retrieve the task information and status.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self), fields(task_id = %task_id)))]
    pub async fn task_info(&self, task_id: TaskID) -> Result<TaskInfo, Error> {
        self.rpc(
            "task.get".to_owned(),
            Some(HashMap::from([("id", task_id)])),
        )
        .await
    }

    /// Updates the tasks status.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn task_status(&self, task_id: TaskID, status: &str) -> Result<Task, Error> {
        let status = TaskStatus {
            task_id,
            status: status.to_owned(),
        };
        self.rpc("docker.update.status".to_owned(), Some(status))
            .await
    }

    /// Defines the stages for the task.  The stages are defined as a mapping
    /// from stage names to their descriptions.  Once stages are defined their
    /// status can be updated using the update_stage method.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, stages)))]
    pub async fn set_stages(&self, task_id: TaskID, stages: &[(&str, &str)]) -> Result<(), Error> {
        let stages: Vec<HashMap<String, String>> = stages
            .iter()
            .map(|(key, value)| {
                let mut stage_map = HashMap::new();
                stage_map.insert(key.to_string(), value.to_string());
                stage_map
            })
            .collect();
        let params = TaskStages { task_id, stages };
        let _: Task = self.rpc("status.stages".to_owned(), Some(params)).await?;
        Ok(())
    }

    /// Updates the progress of the task for the provided stage and status
    /// information.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn update_stage(
        &self,
        task_id: TaskID,
        stage: &str,
        status: &str,
        message: &str,
        percentage: u8,
    ) -> Result<(), Error> {
        let stage = Stage::new(
            Some(task_id),
            stage.to_owned(),
            Some(status.to_owned()),
            Some(message.to_owned()),
            percentage,
        );
        let _: Task = self.rpc("status.update".to_owned(), Some(stage)).await?;
        Ok(())
    }

    /// Authenticated fetch from the Studio server using the bulk HTTP client
    /// (no total-request timeout; idle read timeout per chunk).
    ///
    /// **Buffers the entire response body into memory.** Suitable for small to
    /// medium payloads. For very large binary downloads (multi-GB artifacts or
    /// checkpoints), prefer a streaming approach that writes directly to disk.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
    pub async fn fetch(&self, query: &str) -> Result<Vec<u8>, Error> {
        let req = self
            .bulk_http
            .get(format!("{}/{}", self.url, query))
            .header("User-Agent", "EdgeFirst Client")
            .header("Authorization", format!("Bearer {}", self.token().await));
        let resp = req.send().await?;

        if resp.status().is_success() {
            let body = resp.bytes().await?;

            if log_enabled!(Level::Trace) {
                trace!("Fetch Response: {}", String::from_utf8_lossy(&body));
            }

            Ok(body.to_vec())
        } else {
            let err = resp.error_for_status_ref().unwrap_err();
            Err(Error::HttpError(err))
        }
    }

    /// Sends a multipart post request to the server.  This is used by the
    /// upload and download APIs which do not use JSON-RPC but instead transfer
    /// files using multipart/form-data.
    ///
    /// The result field is deserialized as `serde_json::Value` rather than
    /// `String` because different server endpoints return different shapes —
    /// `val.data.upload` returns a plain string while `task.data.upload`
    /// returns an object `{"message":…,"path":…,"size":…}`.  All current
    /// callers discard the return value so this is backwards-compatible.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, form)))]
    pub async fn post_multipart(
        &self,
        method: &str,
        form: Form,
    ) -> Result<serde_json::Value, Error> {
        let upload_timeout_secs = std::env::var("EDGEFIRST_UPLOAD_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(600u64);

        let req = self
            .http
            .post(format!("{}/api?method={}", self.url, method))
            .header("Accept", "application/json")
            .header("User-Agent", "EdgeFirst Client")
            .header("Authorization", format!("Bearer {}", self.token().await))
            .timeout(Duration::from_secs(upload_timeout_secs))
            .multipart(form);
        let resp = req.send().await?;

        if resp.status().is_success() {
            let body = resp.bytes().await?;

            if log_enabled!(Level::Trace) {
                trace!(
                    "POST Multipart Response: {}",
                    String::from_utf8_lossy(&body)
                );
            }

            let response: RpcResponse<serde_json::Value> = match serde_json::from_slice(&body) {
                Ok(response) => response,
                Err(err) => {
                    error!("Invalid JSON Response: {}", String::from_utf8_lossy(&body));
                    return Err(err.into());
                }
            };

            if let Some(error) = response.error {
                Err(Error::RpcError(error.code, error.message))
            } else if let Some(result) = response.result {
                Ok(result)
            } else {
                Err(Error::InvalidResponse)
            }
        } else {
            // HTTP-level failure on the multipart upload. Map 413 to the
            // typed `PayloadTooLarge` variant so callers see the same error
            // type from both single-file rpc_download paths and multipart
            // upload paths; everything else falls through to HttpError.
            let status = resp.status();
            if status.as_u16() == 413 {
                return Err(Error::PayloadTooLarge {
                    method: method.to_string(),
                    size_hint: None,
                });
            }
            let err = resp.error_for_status_ref().unwrap_err();
            Err(Error::HttpError(err))
        }
    }

    /// Internal helper: POST a JSON-RPC request and stream the binary response
    /// to `output_path`. The response is assumed to be raw binary (not a JSON
    /// envelope). Use for endpoints that return file contents directly.
    ///
    /// On HTTP non-success, the response body is read as text and surfaced
    /// via `Error::RpcError(status_code, body)`.
    pub(crate) async fn rpc_download<P: Serialize>(
        &self,
        method: &str,
        params: &P,
        output_path: &std::path::Path,
        progress: Option<tokio::sync::mpsc::Sender<Progress>>,
    ) -> Result<(), Error> {
        let envelope = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": method,
            "params": params,
        });

        let url = format!("{}/api", self.url);
        let resp = self
            .bulk_http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token().await))
            .json(&envelope)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            if status.as_u16() == 413 {
                return Err(Error::PayloadTooLarge {
                    method: method.to_string(),
                    size_hint: None,
                });
            }
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::RpcError(status.as_u16() as i32, body));
        }

        // HTTP 200 with Content-Type: application/json can mean two things:
        //   (a) a JSON-RPC error envelope when the server failed mid-way
        //       (e.g. {"jsonrpc":"2.0","error":{"code":N,"message":"..."}}),
        //   (b) a legitimate JSON file payload — validation traces, chart
        //       bodies, metrics, etc., are typically served with this MIME.
        //
        // Disambiguate structurally: a JSON-RPC 2.0 envelope is required to
        // carry a `jsonrpc` member, and an *error* envelope further requires
        // an `error.code` integer (per RFC 8259 + JSON-RPC 2.0 §5). Only
        // decode the body as an error if both markers are present. This is
        // strict enough to leave legitimate JSON artifacts that happen to
        // contain a free-form `error` field (metrics, diagnostics, log
        // dumps) untouched, while still catching every real server
        // failure.
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        if content_type.contains("application/json") {
            let body = resp.bytes().await?;
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body)
                && is_jsonrpc_error_envelope(&val)
                && let Some(err_obj) = val.get("error")
            {
                let code = err_obj.get("code").and_then(|c| c.as_i64()).unwrap_or(-1) as i32;
                let message = err_obj
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                return Err(Error::RpcError(code, message));
            }
            // Not an error envelope — body is a JSON file. Write it to disk
            // and emit a single completion progress event so callers (e.g.,
            // Python download_data progress callbacks) see the download
            // finish.
            //
            // `Path::parent` returns `Some("")` for a bare filename like
            // "metrics.json"; `create_dir_all("")` errors out with
            // `NotFound`, so only create the parent when it actually names
            // a directory.
            if let Some(parent) = output_path.parent()
                && !parent.as_os_str().is_empty()
            {
                tokio::fs::create_dir_all(parent).await?;
            }
            let mut file = tokio::fs::File::create(output_path).await?;
            file.write_all(&body).await?;
            file.flush().await?;
            if let Some(tx) = progress {
                let total = body.len();
                // Use the awaited send for the final event so completion
                // handlers are never silently dropped.
                let _ = tx
                    .send(Progress {
                        current: total,
                        total,
                        status: None,
                    })
                    .await;
            }
            return Ok(());
        }

        // Same empty-parent guard for the streaming download path: passing
        // a bare filename like "metrics.json" must write to the current
        // directory rather than failing on `create_dir_all("")`.
        if let Some(parent) = output_path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await?;
        }

        stream_response_to_file(resp, output_path, progress).await
    }

    /// Send a JSON-RPC request to the server.  The method is the name of the
    /// method to call on the server.  The params are the parameters to pass to
    /// the method.  The method and params are serialized into a JSON-RPC
    /// request and sent to the server.  The response is deserialized into
    /// the specified type and returned to the caller.
    ///
    /// NOTE: This API would generally not be called directly and instead users
    /// should use the higher-level methods provided by the client.
    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, params), fields(method = %method)))]
    pub async fn rpc<Params, RpcResult>(
        &self,
        method: String,
        params: Option<Params>,
    ) -> Result<RpcResult, Error>
    where
        Params: Serialize,
        RpcResult: DeserializeOwned,
    {
        let auth_expires = self.token_expiration().await?;
        if auth_expires <= Utc::now() + Duration::from_secs(3600) {
            self.renew_token().await?;
        }

        self.rpc_without_auth(method, params).await
    }

    #[cfg_attr(feature = "profiling", tracing::instrument(skip(self, params), fields(method = %method, request = tracing::field::Empty, response = tracing::field::Empty)))]
    async fn rpc_without_auth<Params, RpcResult>(
        &self,
        method: String,
        params: Option<Params>,
    ) -> Result<RpcResult, Error>
    where
        Params: Serialize,
        RpcResult: DeserializeOwned,
    {
        let max_retries = std::env::var("EDGEFIRST_MAX_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5usize);

        let url = format!("{}/api", self.url);

        // Serialize request body once before retry loop to avoid Clone bound on Params
        let request = RpcRequest {
            method: method.clone(),
            params,
            ..Default::default()
        };

        // Log request for debugging (log crate) and profiling (tracing crate)
        let request_json = if method == "auth.login" {
            // Redact auth.login params (contains password)
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": &method,
                "params": "[REDACTED - contains credentials]",
                "id": request.id
            })
            .to_string()
        } else {
            serde_json::to_string(&request)?
        };

        if log_enabled!(Level::Trace) {
            trace!("RPC Request: {}", request_json);
        }

        // Record request on current span for Perfetto when profiling is enabled
        #[cfg(feature = "profiling")]
        tracing::Span::current().record("request", &request_json);

        let request_body = serde_json::to_vec(&request)?;
        let mut last_error: Option<Error> = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                // Exponential backoff with jitter: base delay * 2^attempt, capped at 30s
                // Jitter: randomize between 100%-150% of base delay to avoid thundering herd
                // while ensuring we never retry faster than the base delay
                let base_delay_secs = (1u64 << (attempt - 1).min(5)).min(30);
                let jitter_factor = 1.0 + (rand::random::<f64>() * 0.5); // 1.0 to 1.5
                let delay_ms = (base_delay_secs as f64 * 1000.0 * jitter_factor) as u64;
                let delay = Duration::from_millis(delay_ms);
                warn!(
                    "Retry {}/{} for RPC '{}' after {:?}",
                    attempt, max_retries, method, delay
                );
                tokio::time::sleep(delay).await;
            }

            let result = self
                .http
                .post(&url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("User-Agent", "EdgeFirst Client")
                .header("Authorization", format!("Bearer {}", self.token().await))
                .body(request_body.clone())
                .send()
                .await;

            match result {
                Ok(res) => {
                    let status = res.status();
                    let status_code = status.as_u16();

                    // Check for retryable HTTP status codes before processing response
                    if matches!(status_code, 408 | 429 | 500 | 502 | 503 | 504)
                        && attempt < max_retries
                    {
                        warn!(
                            "RPC '{}' failed with HTTP {} (retrying)",
                            method, status_code
                        );
                        last_error = Some(Error::HttpError(res.error_for_status().unwrap_err()));
                        continue;
                    }

                    // Process the response
                    match self.process_rpc_response(res).await {
                        Ok(result) => {
                            if attempt > 0 {
                                debug!("RPC '{}' succeeded on retry {}", method, attempt);
                            }
                            return Ok(result);
                        }
                        Err(e) => {
                            // Don't retry client errors (4xx except 408, 429)
                            if attempt > 0 {
                                error!("RPC '{}' failed after {} retries: {}", method, attempt, e);
                            }
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    // Transport error (timeout, connection failure, etc.)
                    let is_timeout = e.is_timeout();
                    let is_connect = e.is_connect();

                    if (is_timeout || is_connect) && attempt < max_retries {
                        warn!(
                            "RPC '{}' transport error (retrying): {}",
                            method,
                            if is_timeout {
                                "timeout"
                            } else {
                                "connection failed"
                            }
                        );
                        last_error = Some(Error::HttpError(e));
                        continue;
                    }

                    if attempt > 0 {
                        error!("RPC '{}' failed after {} retries: {}", method, attempt, e);
                    }
                    return Err(Error::HttpError(e));
                }
            }
        }

        // Should not reach here
        Err(last_error.unwrap_or_else(|| {
            Error::InvalidParameters(format!(
                "RPC '{}' failed after {} retries",
                method, max_retries
            ))
        }))
    }

    async fn process_rpc_response<RpcResult>(
        &self,
        res: reqwest::Response,
    ) -> Result<RpcResult, Error>
    where
        RpcResult: DeserializeOwned,
    {
        let body = res.bytes().await?;
        let response_str = String::from_utf8_lossy(&body);

        if log_enabled!(Level::Trace) {
            trace!("RPC Response: {}", response_str);
        }

        // Record response on current span for Perfetto when profiling is enabled
        // Truncate large responses to avoid bloating trace files
        #[cfg(feature = "profiling")]
        {
            const MAX_RESPONSE_LEN: usize = 4096;
            let truncated = if response_str.len() > MAX_RESPONSE_LEN {
                // Use floor_char_boundary to avoid panicking on multi-byte UTF-8 chars
                let safe_end = response_str.floor_char_boundary(MAX_RESPONSE_LEN);
                format!(
                    "{}...[truncated {} bytes]",
                    &response_str[..safe_end],
                    response_str.len() - safe_end
                )
            } else {
                response_str.to_string()
            };
            tracing::Span::current().record("response", &truncated);
        }

        let response: RpcResponse<RpcResult> = match serde_json::from_slice(&body) {
            Ok(response) => response,
            Err(err) => {
                error!("Invalid JSON Response: {}", String::from_utf8_lossy(&body));
                return Err(err.into());
            }
        };

        // FIXME: Studio Server always returns 999 as the id.
        // if request.id.to_string() != response.id {
        //     return Err(Error::InvalidRpcId(response.id));
        // }

        if let Some(error) = response.error {
            Err(Error::RpcError(error.code, error.message))
        } else if let Some(result) = response.result {
            Ok(result)
        } else {
            Err(Error::InvalidResponse)
        }
    }
}

/// Process items in parallel with semaphore concurrency control and progress
/// tracking.
///
/// This helper eliminates boilerplate for parallel item processing with:
/// - Semaphore limiting concurrent tasks (configurable via `concurrency` param
///   or `MAX_TASKS` env var, default: half of CPU cores clamped to 2-8)
/// - Atomic progress counter with automatic item-level updates
/// - Progress updates sent after each item completes (not byte-level streaming)
/// - Proper error propagation from spawned tasks
///
/// Note: This is optimized for discrete items with post-completion progress
/// updates. For byte-level streaming progress or custom retry logic, use
/// specialized implementations.
///
/// # Arguments
///
/// * `items` - Collection of items to process in parallel
/// * `progress` - Optional progress channel for tracking completion
/// * `concurrency` - Optional max concurrent tasks (defaults to `max_tasks()`)
/// * `work_fn` - Async function to execute for each item
///
/// # Examples
///
/// ```rust,ignore
/// // Use default concurrency
/// parallel_foreach_items(samples, progress, None, |sample| async move {
///     sample.download(&client, file_type).await?;
///     Ok(())
/// }).await?;
/// ```
async fn parallel_foreach_items<T, F, Fut>(
    items: Vec<T>,
    progress: Option<Sender<Progress>>,
    concurrency: Option<usize>,
    work_fn: F,
) -> Result<(), Error>
where
    T: Send + 'static,
    F: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), Error>> + Send + 'static,
{
    let total = items.len();
    let current = Arc::new(AtomicUsize::new(0));
    let sem = Arc::new(Semaphore::new(concurrency.unwrap_or_else(max_tasks)));
    let work_fn = Arc::new(work_fn);

    let tasks = items
        .into_iter()
        .map(|item| {
            let sem = sem.clone();
            let current = current.clone();
            let progress = progress.clone();
            let work_fn = work_fn.clone();

            tokio::spawn(async move {
                let _permit = sem.acquire().await.map_err(|_| {
                    Error::IoError(std::io::Error::other("Semaphore closed unexpectedly"))
                })?;

                // Execute the actual work
                work_fn(item).await?;

                // Update progress
                if let Some(progress) = &progress {
                    let current = current.fetch_add(1, Ordering::SeqCst);
                    let _ = progress
                        .send(Progress {
                            current: current + 1,
                            total,
                            status: None,
                        })
                        .await;
                }

                Ok::<(), Error>(())
            })
        })
        .collect::<Vec<_>>();

    join_all(tasks)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(progress) = progress {
        drop(progress);
    }

    Ok(())
}

/// Upload a file to S3 using multipart upload with presigned URLs.
///
/// Splits a file into chunks (100MB each) and uploads them in parallel using
/// S3 multipart upload protocol. Returns completion parameters with ETags for
/// finalizing the upload.
///
/// This function handles:
/// - Splitting files into parts based on PART_SIZE (100MB)
/// - Parallel upload with concurrency limiting via `max_tasks()` (configurable
///   with `MAX_TASKS`, default: half of CPU cores, min 2, max 8)
/// - Retry logic (handled by reqwest client)
/// - Progress tracking across all parts
///
/// # Arguments
///
/// * `http` - HTTP client for making requests
/// * `part` - Snapshot part info with presigned URLs for each chunk
/// * `path` - Local file path to upload
/// * `total` - Total bytes across all files for progress calculation
/// * `current` - Atomic counter tracking bytes uploaded across all operations
/// * `progress` - Optional channel for sending progress updates
///
/// # Returns
///
/// Parameters needed to complete the multipart upload (key, upload_id, ETags)
async fn upload_multipart(
    http: reqwest::Client,
    part: SnapshotPart,
    path: PathBuf,
    total: usize,
    confirmed_bytes: Arc<AtomicUsize>,
    progress: Option<Sender<Progress>>,
) -> Result<SnapshotCompleteMultipartParams, Error> {
    let filesize = path.metadata()?.len() as usize;
    let n_parts = filesize.div_ceil(PART_SIZE);
    let sem = Arc::new(Semaphore::new(max_upload_tasks()));

    let key = part.key.ok_or(Error::InvalidResponse)?;
    let upload_id = part.upload_id;

    let urls = part.urls.clone();

    // Pre-allocate ETag slots for all parts
    let etags = Arc::new(tokio::sync::Mutex::new(vec![
        EtagPart {
            etag: "".to_owned(),
            part_number: 0,
        };
        n_parts
    ]));

    // Per-part byte counters for streaming progress (reset on retry)
    let part_bytes: Arc<Vec<AtomicUsize>> = Arc::new(
        (0..n_parts)
            .map(|_| AtomicUsize::new(0))
            .collect::<Vec<_>>(),
    );

    // Upload all parts in parallel with concurrency limiting
    let tasks = (0..n_parts)
        .map(|part_idx| {
            let http = http.clone();
            let url = urls[part_idx].clone();
            let etags = etags.clone();
            let path = path.to_owned();
            let sem = sem.clone();
            let progress = progress.clone();
            let confirmed_bytes = confirmed_bytes.clone();
            let part_bytes = part_bytes.clone();

            // Calculate this part's size
            let part_size = if part_idx + 1 == n_parts && !filesize.is_multiple_of(PART_SIZE) {
                filesize % PART_SIZE
            } else {
                PART_SIZE
            };

            tokio::spawn(async move {
                // Acquire semaphore permit to limit concurrent uploads
                let _permit = sem.acquire().await.map_err(|_| {
                    Error::IoError(std::io::Error::other("Semaphore closed unexpectedly"))
                })?;

                // Upload part with streaming progress and retry logic
                let etag = upload_part_with_progress(
                    http,
                    url,
                    path,
                    part_idx,
                    n_parts,
                    part_size,
                    total,
                    confirmed_bytes.clone(),
                    part_bytes.clone(),
                    progress.clone(),
                )
                .await?;

                // Store ETag for this part (needed to complete multipart upload)
                let mut etags_guard = etags.lock().await;
                etags_guard[part_idx] = EtagPart {
                    etag,
                    part_number: part_idx + 1,
                };

                // Part completed successfully - add to confirmed bytes
                confirmed_bytes.fetch_add(part_size, Ordering::SeqCst);
                // Reset part counter since it's now confirmed
                part_bytes[part_idx].store(0, Ordering::SeqCst);

                // Send final progress update for this part
                if let Some(progress) = &progress {
                    let current = confirmed_bytes.load(Ordering::SeqCst)
                        + part_bytes
                            .iter()
                            .map(|p| p.load(Ordering::SeqCst))
                            .sum::<usize>();
                    let _ = progress
                        .send(Progress {
                            current,
                            total,
                            status: None,
                        })
                        .await;
                }

                Ok::<(), Error>(())
            })
        })
        .collect::<Vec<_>>();

    // Wait for all parts to complete (double collect to handle both JoinError and
    // inner Error)
    join_all(tasks)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SnapshotCompleteMultipartParams {
        key,
        upload_id,
        etag_list: etags.lock().await.clone(),
    })
}

/// Upload a single part with streaming progress tracking and retry logic.
///
/// Progress is reported continuously as bytes are sent. On retry, the part's
/// progress counter is reset to avoid over-reporting.
#[allow(clippy::too_many_arguments)]
async fn upload_part_with_progress(
    http: reqwest::Client,
    url: String,
    path: PathBuf,
    part_idx: usize,
    n_parts: usize,
    part_size: usize,
    total: usize,
    confirmed_bytes: Arc<AtomicUsize>,
    part_bytes: Arc<Vec<AtomicUsize>>,
    progress: Option<Sender<Progress>>,
) -> Result<String, Error> {
    let max_retries = std::env::var("EDGEFIRST_MAX_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5usize);

    // Per-part total upload timeout. Covers the send phase (request body) where
    // read_timeout does not apply. Each part is at most PART_SIZE (100MB), so
    // this bounds how long a stalled upload can block before retrying.
    let upload_timeout_secs = std::env::var("EDGEFIRST_UPLOAD_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600u64); // 600s = 100MB at ~170 KB/s minimum

    let mut last_error: Option<Error> = None;

    for attempt in 0..=max_retries {
        if attempt > 0 {
            // Reset this part's progress counter before retry
            part_bytes[part_idx].store(0, Ordering::SeqCst);

            // Exponential backoff: 1s, 2s, 4s, 8s, ...
            let delay = Duration::from_secs(1 << (attempt - 1).min(4));
            warn!(
                "Retry {}/{} for part {} after {:?}",
                attempt, max_retries, part_idx, delay
            );
            tokio::time::sleep(delay).await;
        }

        match upload_part_streaming(
            http.clone(),
            url.clone(),
            path.clone(),
            part_idx,
            n_parts,
            part_size,
            total,
            upload_timeout_secs,
            confirmed_bytes.clone(),
            part_bytes.clone(),
            progress.clone(),
        )
        .await
        {
            Ok(etag) => return Ok(etag),
            Err(e) => {
                // Check if error is retryable
                let is_retryable = matches!(
                    &e,
                    Error::HttpError(re) if re.is_timeout() || re.is_connect() ||
                        re.status().map(|s: reqwest::StatusCode| s.as_u16()).unwrap_or(0) >= 500
                );

                if is_retryable && attempt < max_retries {
                    last_error = Some(e);
                    continue;
                }

                return Err(e);
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| Error::IoError(std::io::Error::other("Upload failed after retries"))))
}

/// Perform the actual upload with streaming progress.
#[allow(clippy::too_many_arguments)]
async fn upload_part_streaming(
    http: reqwest::Client,
    url: String,
    path: PathBuf,
    part_idx: usize,
    n_parts: usize,
    _part_size: usize,
    total: usize,
    upload_timeout_secs: u64,
    confirmed_bytes: Arc<AtomicUsize>,
    part_bytes: Arc<Vec<AtomicUsize>>,
    progress: Option<Sender<Progress>>,
) -> Result<String, Error> {
    let filesize = path.metadata()?.len() as usize;
    let mut file = File::open(&path).await?;
    file.seek(SeekFrom::Start((part_idx * PART_SIZE) as u64))
        .await?;
    let file = file.take(PART_SIZE as u64);

    let body_length = if part_idx + 1 == n_parts && !filesize.is_multiple_of(PART_SIZE) {
        filesize % PART_SIZE
    } else {
        PART_SIZE
    };

    // Create stream with progress tracking
    let stream = FramedRead::new(file, BytesCodec::new());

    // Wrap stream to track bytes sent and report progress
    let progress_stream = stream.map(move |result| {
        if let Ok(ref bytes) = result {
            let bytes_len = bytes.len();
            part_bytes[part_idx].fetch_add(bytes_len, Ordering::SeqCst);

            // Send progress update (fire-and-forget via try_send to avoid blocking)
            if let Some(ref progress) = progress {
                let current = confirmed_bytes.load(Ordering::SeqCst)
                    + part_bytes
                        .iter()
                        .map(|p| p.load(Ordering::SeqCst))
                        .sum::<usize>();
                // Best-effort progress reporting: use try_send to avoid blocking.
                // If the channel is full or closed, we intentionally skip this update
                // to avoid stalling the upload; subsequent updates will still be delivered.
                let _ = progress.try_send(Progress {
                    current,
                    total,
                    status: None,
                });
            }
        }
        result.map(|b| b.freeze())
    });

    let body = Body::wrap_stream(progress_stream);

    let resp = http
        .put(url)
        .header(CONTENT_LENGTH, body_length)
        .timeout(Duration::from_secs(upload_timeout_secs))
        .body(body)
        .send()
        .await?
        .error_for_status()?;

    let etag = resp
        .headers()
        .get("etag")
        .ok_or_else(|| Error::InvalidEtag("Missing ETag header".to_string()))?
        .to_str()
        .map_err(|_| Error::InvalidEtag("Invalid ETag encoding".to_string()))?
        .to_owned();

    // Studio Server requires etag without the quotes.
    let etag = etag
        .strip_prefix("\"")
        .ok_or_else(|| Error::InvalidEtag("Missing opening quote".to_string()))?;
    let etag = etag
        .strip_suffix("\"")
        .ok_or_else(|| Error::InvalidEtag("Missing closing quote".to_string()))?;

    Ok(etag.to_owned())
}

/// Upload a complete file to a presigned S3 URL using HTTP PUT.
///
/// This is used for populate_samples to upload files to S3 after
/// receiving presigned URLs from the server.
///
/// Includes explicit retry logic with exponential backoff for transient
/// failures.
/// Classify a reqwest transport error (one where no HTTP response was received)
/// as a transient failure worth retrying.
///
/// Presigned-URL uploads buffer the body in memory and a PUT to the same object
/// key is idempotent, so replaying any transport-level failure is safe. Besides
/// timeouts and connect failures this covers request/body send errors such as
/// hyper's `IncompleteMessage` (a peer closing a keep-alive connection mid-send)
/// — transients that pipelined, high-concurrency uploads provoke far more often
/// than serial ones, and which the previous `is_timeout() || is_connect()` gate
/// missed (aborting the whole upload on a single blip).
fn is_retryable_upload_error(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect() || e.is_request() || e.is_body()
}

async fn upload_file_to_presigned_url(
    http: reqwest::Client,
    url: &str,
    path: PathBuf,
) -> Result<(), Error> {
    let max_retries = std::env::var("EDGEFIRST_MAX_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5usize);

    let upload_timeout_secs = std::env::var("EDGEFIRST_UPLOAD_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600u64);

    // Read the entire file into memory once
    let file_data = fs::read(&path).await?;
    let file_size = file_data.len();
    let filename = path.file_name().unwrap_or_default().to_string_lossy();

    let mut last_error: Option<Error> = None;

    for attempt in 0..=max_retries {
        if attempt > 0 {
            // Exponential backoff: 1s, 2s, 4s, 8s, ...
            let delay = Duration::from_secs(1 << (attempt - 1).min(4));
            warn!(
                "Retry {}/{} for upload '{}' after {:?}",
                attempt, max_retries, filename, delay
            );
            tokio::time::sleep(delay).await;
        }

        // Attempt upload
        let result = http
            .put(url)
            .header(CONTENT_LENGTH, file_size)
            .timeout(Duration::from_secs(upload_timeout_secs))
            .body(file_data.clone())
            .send()
            .await;

        match result {
            Ok(resp) => {
                if resp.status().is_success() {
                    if attempt > 0 {
                        debug!(
                            "Upload '{}' succeeded on retry {} ({} bytes)",
                            filename, attempt, file_size
                        );
                    } else {
                        debug!(
                            "Successfully uploaded file: {} ({} bytes)",
                            filename, file_size
                        );
                    }
                    return Ok(());
                }

                let status = resp.status();
                let status_code = status.as_u16();

                // Check if error is retryable
                let is_retryable =
                    matches!(status_code, 408 | 429 | 500 | 502 | 503 | 504 | 409 | 423);

                if is_retryable && attempt < max_retries {
                    let error_text = resp.text().await.unwrap_or_default();
                    warn!(
                        "Upload '{}' failed with HTTP {} (retryable): {}",
                        filename, status_code, error_text
                    );
                    last_error = Some(Error::InvalidParameters(format!(
                        "Upload failed: HTTP {} - {}",
                        status, error_text
                    )));
                    continue;
                }

                // Non-retryable error or max retries exceeded
                let error_text = resp.text().await.unwrap_or_default();
                if attempt > 0 {
                    error!(
                        "Upload '{}' failed after {} retries: HTTP {} - {}",
                        filename, attempt, status, error_text
                    );
                }
                return Err(Error::InvalidParameters(format!(
                    "Upload failed: HTTP {} - {}",
                    status, error_text
                )));
            }
            Err(e) => {
                // Transport error: no HTTP response was received. The body is
                // buffered in memory and the PUT is idempotent, so any transient
                // transport failure is safe to replay (see
                // `is_retryable_upload_error`).
                if is_retryable_upload_error(&e) && attempt < max_retries {
                    warn!("Upload '{}' transport error (retrying): {}", filename, e);
                    last_error = Some(Error::HttpError(e));
                    continue;
                }

                // Non-retryable or max retries exceeded
                if attempt > 0 {
                    error!(
                        "Upload '{}' failed after {} retries: {}",
                        filename, attempt, e
                    );
                }
                return Err(Error::HttpError(e));
            }
        }
    }

    // Should not reach here, but return last error if we do
    Err(last_error.unwrap_or_else(|| {
        Error::InvalidParameters(format!("Upload failed after {} retries", max_retries))
    }))
}

/// Upload bytes directly to a presigned S3 URL using HTTP PUT.
///
/// This is used for populate_samples to upload file content from memory
/// (e.g., from ZIP archives) without writing to disk first.
///
/// Includes explicit retry logic with exponential backoff for transient
/// failures.
async fn upload_bytes_to_presigned_url(
    http: reqwest::Client,
    url: &str,
    file_data: Vec<u8>,
    filename: &str,
) -> Result<(), Error> {
    let max_retries = std::env::var("EDGEFIRST_MAX_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5usize);

    let upload_timeout_secs = std::env::var("EDGEFIRST_UPLOAD_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600u64);

    let file_size = file_data.len();
    let mut last_error: Option<Error> = None;

    for attempt in 0..=max_retries {
        if attempt > 0 {
            // Exponential backoff: 1s, 2s, 4s, 8s, ...
            let delay = Duration::from_secs(1 << (attempt - 1).min(4));
            warn!(
                "Retry {}/{} for upload '{}' after {:?}",
                attempt, max_retries, filename, delay
            );
            tokio::time::sleep(delay).await;
        }

        // Attempt upload
        let result = http
            .put(url)
            .header(CONTENT_LENGTH, file_size)
            .timeout(Duration::from_secs(upload_timeout_secs))
            .body(file_data.clone())
            .send()
            .await;

        match result {
            Ok(resp) => {
                if resp.status().is_success() {
                    if attempt > 0 {
                        debug!(
                            "Upload '{}' succeeded on retry {} ({} bytes)",
                            filename, attempt, file_size
                        );
                    } else {
                        debug!(
                            "Successfully uploaded file: {} ({} bytes)",
                            filename, file_size
                        );
                    }
                    return Ok(());
                }

                let status = resp.status();
                let status_code = status.as_u16();

                // Check if error is retryable
                let is_retryable =
                    matches!(status_code, 408 | 429 | 500 | 502 | 503 | 504 | 409 | 423);

                if is_retryable && attempt < max_retries {
                    let error_text = resp.text().await.unwrap_or_default();
                    warn!(
                        "Upload '{}' failed with HTTP {} (retryable): {}",
                        filename, status_code, error_text
                    );
                    last_error = Some(Error::InvalidParameters(format!(
                        "Upload failed: HTTP {} - {}",
                        status, error_text
                    )));
                    continue;
                }

                // Non-retryable error or max retries exceeded
                let error_text = resp.text().await.unwrap_or_default();
                if attempt > 0 {
                    error!(
                        "Upload '{}' failed after {} retries: HTTP {} - {}",
                        filename, attempt, status, error_text
                    );
                }
                return Err(Error::InvalidParameters(format!(
                    "Upload failed: HTTP {} - {}",
                    status, error_text
                )));
            }
            Err(e) => {
                // Transport error: no HTTP response was received. The body is
                // buffered in memory and the PUT is idempotent, so any transient
                // transport failure is safe to replay (see
                // `is_retryable_upload_error`).
                if is_retryable_upload_error(&e) && attempt < max_retries {
                    warn!("Upload '{}' transport error (retrying): {}", filename, e);
                    last_error = Some(Error::HttpError(e));
                    continue;
                }

                // Non-retryable or max retries exceeded
                if attempt > 0 {
                    error!(
                        "Upload '{}' failed after {} retries: {}",
                        filename, attempt, e
                    );
                }
                return Err(Error::HttpError(e));
            }
        }
    }

    // Should not reach here, but return last error if we do
    Err(last_error.unwrap_or_else(|| {
        Error::InvalidParameters(format!("Upload failed after {} retries", max_retries))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_and_sort_by_name_exact_match_first() {
        // Test that exact matches come first
        let items = vec![
            "Deer Roundtrip 123".to_string(),
            "Deer".to_string(),
            "Reindeer".to_string(),
            "DEER".to_string(),
        ];
        let result = filter_and_sort_by_name(items, "Deer", |s| s.as_str());
        assert_eq!(result[0], "Deer"); // Exact match first
        assert_eq!(result[1], "DEER"); // Case-insensitive exact match second
    }

    #[test]
    fn test_filter_and_sort_by_name_shorter_names_preferred() {
        // Test that shorter names (more specific) come before longer ones
        let items = vec![
            "Test Dataset ABC".to_string(),
            "Test".to_string(),
            "Test Dataset".to_string(),
        ];
        let result = filter_and_sort_by_name(items, "Test", |s| s.as_str());
        assert_eq!(result[0], "Test"); // Exact match first
        assert_eq!(result[1], "Test Dataset"); // Shorter substring match
        assert_eq!(result[2], "Test Dataset ABC"); // Longer substring match
    }

    #[test]
    fn test_filter_and_sort_by_name_case_insensitive_filter() {
        // Test that filtering is case-insensitive
        let items = vec![
            "UPPERCASE".to_string(),
            "lowercase".to_string(),
            "MixedCase".to_string(),
        ];
        let result = filter_and_sort_by_name(items, "case", |s| s.as_str());
        assert_eq!(result.len(), 3); // All items should match
    }

    #[test]
    fn test_filter_and_sort_by_name_no_matches() {
        // Test that empty result is returned when no matches
        let items = vec!["Apple".to_string(), "Banana".to_string()];
        let result = filter_and_sort_by_name(items, "Cherry", |s| s.as_str());
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_and_sort_by_name_alphabetical_tiebreaker() {
        // Test alphabetical ordering for same-length names
        let items = vec![
            "TestC".to_string(),
            "TestA".to_string(),
            "TestB".to_string(),
        ];
        let result = filter_and_sort_by_name(items, "Test", |s| s.as_str());
        assert_eq!(result, vec!["TestA", "TestB", "TestC"]);
    }

    #[test]
    fn test_build_filename_no_flatten() {
        // When flatten=false, should return base_name unchanged
        let result = Client::build_filename("image.jpg", false, Some(&"seq".to_string()), Some(42));
        assert_eq!(result, "image.jpg");

        let result = Client::build_filename("test.png", false, None, None);
        assert_eq!(result, "test.png");
    }

    #[test]
    fn test_build_filename_flatten_no_sequence() {
        // When flatten=true but no sequence, should return base_name unchanged
        let result = Client::build_filename("standalone.jpg", true, None, None);
        assert_eq!(result, "standalone.jpg");
    }

    #[test]
    fn test_build_filename_flatten_with_sequence_not_prefixed() {
        // When flatten=true, in sequence, filename not prefixed → add prefix
        let result = Client::build_filename(
            "image.camera.jpeg",
            true,
            Some(&"deer_sequence".to_string()),
            Some(42),
        );
        assert_eq!(result, "deer_sequence_42_image.camera.jpeg");
    }

    #[test]
    fn test_build_filename_flatten_with_sequence_no_frame() {
        // When flatten=true, in sequence, no frame number → prefix with sequence only
        let result =
            Client::build_filename("image.jpg", true, Some(&"sequence_A".to_string()), None);
        assert_eq!(result, "sequence_A_image.jpg");
    }

    #[test]
    fn test_build_filename_flatten_already_prefixed() {
        // When flatten=true, filename already starts with sequence_ → return unchanged
        let result = Client::build_filename(
            "deer_sequence_042.camera.jpeg",
            true,
            Some(&"deer_sequence".to_string()),
            Some(42),
        );
        assert_eq!(result, "deer_sequence_042.camera.jpeg");
    }

    #[test]
    fn test_build_filename_flatten_already_prefixed_different_frame() {
        // Edge case: filename has sequence prefix but we're adding different frame
        // Should still respect existing prefix
        let result = Client::build_filename(
            "sequence_A_001.jpg",
            true,
            Some(&"sequence_A".to_string()),
            Some(2),
        );
        assert_eq!(result, "sequence_A_001.jpg");
    }

    #[test]
    fn test_build_filename_flatten_partial_match() {
        // Edge case: filename contains sequence name but not as prefix
        let result = Client::build_filename(
            "test_sequence_A_image.jpg",
            true,
            Some(&"sequence_A".to_string()),
            Some(5),
        );
        // Should add prefix because it doesn't START with "sequence_A_"
        assert_eq!(result, "sequence_A_5_test_sequence_A_image.jpg");
    }

    #[test]
    fn test_build_filename_flatten_preserves_extension() {
        // Verify that file extensions are preserved correctly
        let extensions = vec![
            "jpeg",
            "jpg",
            "png",
            "camera.jpeg",
            "lidar.pcd",
            "depth.png",
        ];

        for ext in extensions {
            let filename = format!("image.{}", ext);
            let result = Client::build_filename(&filename, true, Some(&"seq".to_string()), Some(1));
            assert!(
                result.ends_with(&format!(".{}", ext)),
                "Extension .{} not preserved in {}",
                ext,
                result
            );
        }
    }

    #[test]
    fn test_build_filename_flatten_sanitization_compatibility() {
        // Test with sanitized path components (no special chars)
        let result = Client::build_filename(
            "sample_001.jpg",
            true,
            Some(&"seq_name_with_underscores".to_string()),
            Some(10),
        );
        assert_eq!(result, "seq_name_with_underscores_10_sample_001.jpg");
    }

    // =========================================================================
    // Additional filter_and_sort_by_name tests for exact match determinism
    // =========================================================================

    #[test]
    fn test_filter_and_sort_by_name_exact_match_is_deterministic() {
        // Test that searching for "Deer" always returns "Deer" first, not
        // "Deer Roundtrip 20251129" or similar
        let items = vec![
            "Deer Roundtrip 20251129".to_string(),
            "White-Tailed Deer".to_string(),
            "Deer".to_string(),
            "Deer Snapshot Test".to_string(),
            "Reindeer Dataset".to_string(),
        ];

        let result = filter_and_sort_by_name(items, "Deer", |s| s.as_str());

        // CRITICAL: First result must be exact match "Deer"
        assert_eq!(
            result.first().map(|s| s.as_str()),
            Some("Deer"),
            "Expected exact match 'Deer' first, got: {:?}",
            result.first()
        );

        // Verify all items containing "Deer" are present (case-insensitive)
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_filter_and_sort_by_name_exact_match_with_different_cases() {
        // Verify case-sensitive exact match takes priority over case-insensitive
        let items = vec![
            "DEER".to_string(),
            "deer".to_string(),
            "Deer".to_string(),
            "Deer Test".to_string(),
        ];

        let result = filter_and_sort_by_name(items, "Deer", |s| s.as_str());

        // Priority 1: Case-sensitive exact match "Deer" first
        assert_eq!(result[0], "Deer");
        // Priority 2: Case-insensitive exact matches next
        assert!(result[1] == "DEER" || result[1] == "deer");
        assert!(result[2] == "DEER" || result[2] == "deer");
    }

    #[test]
    fn test_filter_and_sort_by_name_snapshot_realistic_scenario() {
        // Realistic scenario: User searches for snapshot "Deer" and multiple
        // snapshots exist with similar names
        let items = vec![
            "Unit Testing - Deer Dataset Backup".to_string(),
            "Deer".to_string(),
            "Deer Snapshot 2025-01-15".to_string(),
            "Original Deer".to_string(),
        ];

        let result = filter_and_sort_by_name(items, "Deer", |s| s.as_str());

        // MUST return exact match first for deterministic test behavior
        assert_eq!(
            result[0], "Deer",
            "Searching for 'Deer' should return exact 'Deer' first"
        );
    }

    #[test]
    fn test_filter_and_sort_by_name_dataset_realistic_scenario() {
        // Realistic scenario: User searches for dataset "Deer" but multiple
        // datasets have "Deer" in their name
        let items = vec![
            "Deer Roundtrip".to_string(),
            "Deer".to_string(),
            "deer".to_string(),
            "White-Tailed Deer".to_string(),
            "Deer-V2".to_string(),
        ];

        let result = filter_and_sort_by_name(items, "Deer", |s| s.as_str());

        // Exact case-sensitive match must be first
        assert_eq!(result[0], "Deer");
        // Case-insensitive exact match should be second
        assert_eq!(result[1], "deer");
        // Shorter names should come before longer names
        assert!(
            result.iter().position(|s| s == "Deer-V2").unwrap()
                < result.iter().position(|s| s == "Deer Roundtrip").unwrap()
        );
    }

    #[test]
    fn test_filter_and_sort_by_name_first_result_is_always_best_match() {
        // CRITICAL: The first result should ALWAYS be the best match
        // This is essential for deterministic test behavior
        let scenarios = vec![
            // (items, filter, expected_first)
            (vec!["Deer Dataset", "Deer", "deer"], "Deer", "Deer"),
            (vec!["test", "TEST", "Test Data"], "test", "test"),
            (vec!["ABC", "ABCD", "abc"], "ABC", "ABC"),
        ];

        for (items, filter, expected_first) in scenarios {
            let items: Vec<String> = items.iter().map(|s| s.to_string()).collect();
            let result = filter_and_sort_by_name(items, filter, |s| s.as_str());

            assert_eq!(
                result.first().map(|s| s.as_str()),
                Some(expected_first),
                "For filter '{}', expected first result '{}', got: {:?}",
                filter,
                expected_first,
                result.first()
            );
        }
    }

    #[test]
    fn test_with_server_clears_storage() {
        use crate::storage::MemoryTokenStorage;

        // Create client with memory storage and a token
        let storage = Arc::new(MemoryTokenStorage::new());
        storage.store("test-token").unwrap();

        let client = Client::new().unwrap().with_storage(storage.clone());

        // Verify token is loaded
        assert_eq!(storage.load().unwrap(), Some("test-token".to_string()));

        // Change server - should clear storage
        let _new_client = client.with_server("test").unwrap();

        // Verify storage was cleared
        assert_eq!(storage.load().unwrap(), None);
    }

    #[test]
    fn test_with_server_clears_storage_even_for_full_url() {
        // Regression: `with_server` used to short-circuit to `with_url`
        // when given a full URL, which preserved the bearer token. The
        // contract for `with_server` is that switching servers means
        // the token from the old server is no longer trusted.
        use crate::storage::MemoryTokenStorage;

        let storage = Arc::new(MemoryTokenStorage::new());
        storage.store("token-from-old-server").unwrap();
        let client = Client::new().unwrap().with_storage(storage.clone());
        assert_eq!(
            storage.load().unwrap(),
            Some("token-from-old-server".to_string())
        );

        // Switch to a self-hosted Studio (full URL). Storage must be
        // cleared, and the new client must have a blank in-memory token.
        let new_client = client
            .with_server("https://studio.example.com")
            .expect("https full URL through with_server");
        assert_eq!(storage.load().unwrap(), None);
        assert_eq!(new_client.url(), "https://studio.example.com");

        // The new client should not carry the old token in memory either.
        let in_mem = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { new_client.token.read().await.clone() });
        assert!(in_mem.is_empty(), "expected blank token, got {in_mem:?}");
    }

    #[test]
    fn test_with_server_rejects_insecure_full_url() {
        // `with_server` validates full URLs through `with_url`, so the
        // HTTPS rule applies uniformly. Plain http to a public host
        // must be rejected — the bearer token would otherwise leak in
        // plaintext when the caller next authenticates.
        let client = Client::new().unwrap();
        let err = client.with_server("http://studio.example.com").unwrap_err();
        assert!(matches!(err, Error::InsecureUrl(_)));
    }

    // ===== with_url HTTPS enforcement =====
    //
    // The bearer token rides in the Authorization header, so plain
    // http:// to a public host would leak it in the clear. The function
    // must reject those URLs, but still let wiremock / local-dev URLs
    // through (loopback addresses, "localhost", "*.localhost").

    #[test]
    fn with_url_accepts_https_public_host() {
        let client = Client::new().unwrap();
        let out = client
            .with_url("https://studio.example.com")
            .expect("https public host must be accepted");
        assert_eq!(out.url(), "https://studio.example.com");
    }

    #[test]
    fn with_url_accepts_http_loopback_ipv4() {
        let client = Client::new().unwrap();
        let out = client
            .with_url("http://127.0.0.1:8080")
            .expect("http://127.0.0.1 must be accepted (loopback)");
        assert_eq!(out.url(), "http://127.0.0.1:8080");
    }

    #[test]
    fn with_url_accepts_http_loopback_ipv6() {
        let client = Client::new().unwrap();
        let out = client
            .with_url("http://[::1]:8080")
            .expect("http://[::1] must be accepted (loopback)");
        assert!(out.url().starts_with("http://[::1]"));
    }

    #[test]
    fn with_url_accepts_http_localhost() {
        let client = Client::new().unwrap();
        client
            .with_url("http://localhost:8080")
            .expect("http://localhost must be accepted");
        client
            .with_url("http://LOCALHOST")
            .expect("http://LOCALHOST must be accepted (case-insensitive)");
        client
            .with_url("http://wiremock.localhost")
            .expect("http://*.localhost must be accepted");
    }

    #[test]
    fn with_url_rejects_http_public_host() {
        let client = Client::new().unwrap();
        let err = client.with_url("http://studio.example.com").unwrap_err();
        match err {
            Error::InsecureUrl(u) => assert_eq!(u, "http://studio.example.com"),
            other => panic!("expected InsecureUrl, got {other:?}"),
        }
    }

    #[test]
    fn with_url_rejects_http_public_ip() {
        let client = Client::new().unwrap();
        // 8.8.8.8 is not loopback; must be rejected.
        let err = client.with_url("http://8.8.8.8").unwrap_err();
        assert!(matches!(err, Error::InsecureUrl(_)));
    }

    #[test]
    fn with_url_rejects_non_http_scheme() {
        let client = Client::new().unwrap();
        // file:// would otherwise parse, but it's not a transport we
        // can use for RPC and we don't want to silently accept it.
        let err = client.with_url("file:///etc/passwd").unwrap_err();
        assert!(matches!(err, Error::InsecureUrl(_)));
    }
}

#[cfg(test)]
mod tests_map_rpc_error {
    use super::*;
    use crate::api::TaskID;

    #[test]
    fn maps_not_found_with_task_id_to_typed_variant() {
        // Server code 101 + "not found" message + task_id present → TaskNotFound
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let err = map_rpc_error(
            "task.data.list",
            101,
            "task not found".to_string(),
            Some(task_id),
        );
        assert!(matches!(err, Error::TaskNotFound(_)));
    }

    #[test]
    fn maps_cannot_find_phrasing_to_typed_variant() {
        // The DVE server emits "Cannot find task..." — the original "not found"
        // substring match missed this and the caller saw a generic RpcError.
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let err = map_rpc_error(
            "task.data.list",
            101,
            "Cannot find task with id 6789".to_string(),
            Some(task_id),
        );
        assert!(
            matches!(err, Error::TaskNotFound(_)),
            "'Cannot find task' should map to TaskNotFound, got {err:?}"
        );
    }

    #[test]
    fn maps_does_not_exist_phrasing_to_typed_variant() {
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let err = map_rpc_error(
            "task.chart.get",
            101,
            "task does not exist".to_string(),
            Some(task_id),
        );
        assert!(matches!(err, Error::TaskNotFound(_)));
    }

    #[test]
    fn maps_code_101_with_unknown_phrasing_when_task_id_supplied() {
        // Server contract for code 101 is "resource not found"; even if the
        // phrasing is novel, the typed variant should be returned so callers
        // can write a stable `match`.
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let err = map_rpc_error(
            "task.data.list",
            101,
            "completely novel server message".to_string(),
            Some(task_id),
        );
        assert!(
            matches!(err, Error::TaskNotFound(_)),
            "code 101 + task_id should always map to TaskNotFound, got {err:?}"
        );
    }

    #[test]
    fn maps_permission_codes_to_typed_variant() {
        for code in [401, 403] {
            let err = map_rpc_error("task.chart.add", code, "denied".to_string(), None);
            assert!(
                matches!(err, Error::PermissionDenied(_)),
                "code {} did not map",
                code
            );
        }
    }

    #[test]
    fn permission_denied_records_method_for_diagnostics() {
        let err = map_rpc_error("task.data.upload", 403, "forbidden".to_string(), None);
        match err {
            Error::PermissionDenied(method) => assert_eq!(method, "task.data.upload"),
            other => panic!("expected PermissionDenied, got {:?}", other),
        }
    }

    #[test]
    fn maps_payload_too_large_to_typed_variant() {
        let err = map_rpc_error("val.data.upload", 413, "request too large".into(), None);
        match err {
            Error::PayloadTooLarge { method, size_hint } => {
                assert_eq!(method, "val.data.upload");
                assert!(size_hint.is_none());
            }
            other => panic!("expected PayloadTooLarge, got {:?}", other),
        }
    }

    #[test]
    fn falls_through_to_generic_rpc_error_for_unknown_codes() {
        let err = map_rpc_error("task.data.list", -99999, "weird".to_string(), None);
        match err {
            Error::RpcError(code, msg) => {
                assert_eq!(code, -99999);
                assert_eq!(msg, "weird");
            }
            other => panic!("expected RpcError, got {:?}", other),
        }
    }

    #[test]
    fn not_found_without_task_id_falls_through() {
        // Code 101 without task_id → generic RpcError (no task to name)
        let err = map_rpc_error("task.data.list", 101, "not found".to_string(), None);
        assert!(matches!(err, Error::RpcError(101, _)));
    }

    #[test]
    fn code_101_with_task_id_always_maps_even_with_unrelated_message() {
        // Previously the test asserted fall-through for non-"not found"
        // messages, but the contract for code 101 is "resource not found"
        // (see api.go), so when a task_id is present the typed variant is
        // returned unconditionally to give callers a stable error type.
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let err = map_rpc_error(
            "task.data.list",
            101,
            "permission denied".to_string(),
            Some(task_id),
        );
        assert!(matches!(err, Error::TaskNotFound(_)));
    }
}

#[cfg(test)]
mod tests_jobs {
    use super::*;

    #[test]
    fn jobs_list_request_serializes_to_empty_object() {
        let req = JobsListRequest {};
        assert_eq!(serde_json::to_value(&req).unwrap(), serde_json::json!({}));
    }

    #[test]
    fn job_deserializes_from_bk_batch_shape() {
        let json = r#"{
            "code": "edgefirst-validator:2.9.5",
            "title": "EdgeFirst Validator",
            "job_name": "smoke-test",
            "job_id": "aws-batch-abc",
            "state": "RUNNING",
            "launch": "2026-05-14T15:00:00Z",
            "task_id": 6789,
            "docker_task": {},
            "extra_field": "ignored"
        }"#;
        let job: crate::api::Job = serde_json::from_str(json).unwrap();
        assert_eq!(job.code, "edgefirst-validator:2.9.5");
        assert_eq!(job.state, "RUNNING");
        assert_eq!(job.task_id, 6789);
        assert_eq!(job.task_id().value(), 6789);
    }
}

#[cfg(test)]
mod tests_job_run {
    use super::*;
    use crate::api::Parameter;
    use std::collections::HashMap;

    #[test]
    fn job_run_request_serializes_with_expected_fields() {
        let req = JobRunRequest {
            name: "edgefirst-validator".into(),
            job_name: "post-profile-run".into(),
            env: HashMap::from([("LOG_LEVEL".into(), "info".into())]),
            data: HashMap::from([("validation_session_id".into(), Parameter::Integer(2707))]),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["name"], "edgefirst-validator");
        assert_eq!(json["job_name"], "post-profile-run");
        assert_eq!(json["env"]["LOG_LEVEL"], "info");
        assert_eq!(json["data"]["validation_session_id"], 2707);
    }

    #[test]
    fn job_run_response_deserializes_as_job() {
        // job.run now returns the full BK_BATCH record; deserialize as Job.
        let json = r#"{
            "code": "edgefirst-validator:2.9.5",
            "title": "EdgeFirst Validator",
            "job_name": "post-profile-run",
            "job_id": "aws-batch-job-xxx",
            "state": "SUBMITTED",
            "task_id": 6789
        }"#;
        let job: crate::api::Job = serde_json::from_str(json).unwrap();
        assert_eq!(job.task_id, 6789);
        assert_eq!(job.job_id, "aws-batch-job-xxx");
        assert_eq!(job.state, "SUBMITTED");
    }
}

#[cfg(test)]
mod tests_job_stop {
    use super::*;
    use crate::api::TaskID;

    #[test]
    fn job_stop_request_serializes_with_task_id() {
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let req = JobStopRequest {
            task_id: task_id.value(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["task_id"], task_id.value());
    }
}

#[cfg(test)]
mod tests_task_data_list_request {
    use super::*;
    use crate::api::TaskID;

    #[test]
    fn task_data_list_request_serializes_with_task_id() {
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let req = TaskDataListRequest {
            task_id: task_id.value(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["task_id"], task_id.value());
    }
}

#[cfg(test)]
mod tests_task_data_download {
    use super::*;
    use crate::api::TaskID;

    #[test]
    fn task_data_download_request_serializes_with_all_fields() {
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let req = TaskDataDownloadRequest {
            task_id: task_id.value(),
            folder: "predictions".into(),
            file: "predictions.parquet".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["task_id"], task_id.value());
        assert_eq!(json["folder"], "predictions");
        assert_eq!(json["file"], "predictions.parquet");
    }
}

#[cfg(test)]
mod tests_task_chart_add {
    use super::*;
    use crate::api::{Parameter, TaskID};

    #[test]
    fn task_chart_add_request_serializes_with_correct_fields() {
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let data = Parameter::Object(std::collections::HashMap::from([(
            "type".into(),
            Parameter::String("line".into()),
        )]));
        let req = TaskChartAddRequest {
            task_id: task_id.value(),
            group_name: "metrics".into(),
            chart_name: "loss".into(),
            params: None,
            data,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["task_id"], task_id.value());
        assert_eq!(json["group_name"], "metrics");
        assert_eq!(json["chart_name"], "loss");
        assert_eq!(json["data"]["type"], "line");
        assert!(json["params"].is_null());
    }
}

#[cfg(test)]
mod tests_task_chart_list {
    use super::*;
    use crate::api::TaskID;

    #[test]
    fn task_chart_list_request_omits_empty_group_name() {
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let req = TaskChartListRequest {
            task_id: task_id.value(),
            group_name: String::new(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["task_id"], task_id.value());
        assert_eq!(json["group_name"], "");
    }
}

#[cfg(test)]
mod tests_task_chart_get {
    use super::*;
    use crate::api::TaskID;

    #[test]
    fn task_chart_get_request_serializes_with_all_fields() {
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let req = TaskChartGetRequest {
            task_id: task_id.value(),
            group_name: "metrics".into(),
            chart_name: "loss".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["task_id"], task_id.value());
        assert_eq!(json["group_name"], "metrics");
        assert_eq!(json["chart_name"], "loss");
    }
}

#[cfg(test)]
mod tests_val_data_download {
    use super::*;

    #[test]
    fn val_data_download_request_serializes() {
        let req = ValDataDownloadRequest {
            session_id: 2707,
            filename: "trace/imx95.json".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["session_id"], 2707);
        assert_eq!(json["filename"], "trace/imx95.json");
    }
}

#[cfg(test)]
mod tests_val_data_list {
    use super::*;

    #[test]
    fn val_data_list_request_serializes() {
        let req = ValDataListRequest { session_id: 2707 };
        assert_eq!(
            serde_json::to_value(&req).unwrap(),
            serde_json::json!({"session_id": 2707})
        );
    }
}

#[cfg(test)]
mod tests_jsonrpc_envelope_detection {
    use super::*;

    #[test]
    fn detects_real_envelope() {
        let v = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 0,
            "error": { "code": 101, "message": "Cannot find task" },
        });
        assert!(is_jsonrpc_error_envelope(&v));
    }

    #[test]
    fn rejects_plain_json_artifact_with_error_field() {
        // A diagnostics file with a free-form `error` object — must not be
        // misread as an RPC envelope just because the key collides.
        let v = serde_json::json!({
            "metric": "loss",
            "value": 0.42,
            "error": { "code": "ENV_NOT_FOUND", "message": "missing var" },
        });
        assert!(
            !is_jsonrpc_error_envelope(&v),
            "missing jsonrpc sentinel should mean 'not an envelope'"
        );
    }

    #[test]
    fn rejects_envelope_missing_jsonrpc_sentinel() {
        // Bare `error` block without the protocol-version marker.
        let v = serde_json::json!({
            "id": 0,
            "error": { "code": 101, "message": "x" },
        });
        assert!(!is_jsonrpc_error_envelope(&v));
    }

    #[test]
    fn rejects_envelope_with_non_object_error_field() {
        // A diagnostics file shaped like JSON-RPC accidentally but using
        // a string for `error`.
        let v = serde_json::json!({
            "jsonrpc": "2.0",
            "error": "something went wrong",
        });
        assert!(!is_jsonrpc_error_envelope(&v));
    }

    #[test]
    fn rejects_envelope_without_error_code() {
        // Real envelopes always carry an integer error.code; missing one
        // is suspicious enough to refuse the envelope classification.
        let v = serde_json::json!({
            "jsonrpc": "2.0",
            "error": { "message": "no code" },
        });
        assert!(!is_jsonrpc_error_envelope(&v));
    }

    #[test]
    fn rejects_envelope_with_non_numeric_error_code() {
        let v = serde_json::json!({
            "jsonrpc": "2.0",
            "error": { "code": "ENOENT", "message": "x" },
        });
        assert!(!is_jsonrpc_error_envelope(&v));
    }

    #[test]
    fn rejects_non_object_root() {
        // A JSON file whose root is an array — common for metrics dumps —
        // must not be misread.
        let v = serde_json::json!([1, 2, 3]);
        assert!(!is_jsonrpc_error_envelope(&v));
    }

    #[test]
    fn accepts_unsigned_error_code() {
        // The server's code is technically i32 but JSON has no signed/
        // unsigned distinction — accept both shapes.
        let v = serde_json::json!({
            "jsonrpc": "2.0",
            "error": { "code": 101u32, "message": "x" },
        });
        assert!(is_jsonrpc_error_envelope(&v));
    }
}

#[cfg(test)]
mod tests_validate_chart_args {
    use super::*;

    #[test]
    fn rejects_empty_group() {
        let err = validate_chart_args("", "name").unwrap_err();
        assert!(matches!(err, Error::InvalidParameters(_)));
    }

    #[test]
    fn rejects_empty_name() {
        let err = validate_chart_args("group", "").unwrap_err();
        assert!(matches!(err, Error::InvalidParameters(_)));
    }

    #[test]
    fn rejects_both_empty() {
        let err = validate_chart_args("", "").unwrap_err();
        assert!(matches!(err, Error::InvalidParameters(_)));
    }

    #[test]
    fn accepts_valid_args() {
        assert!(validate_chart_args("group", "name").is_ok());
    }

    #[test]
    fn accepts_unicode_args() {
        // Unicode names are allowed; only emptiness is rejected.
        assert!(validate_chart_args("metrics-集合", "损失").is_ok());
    }
}

// ---------------------------------------------------------------------------
// Additional offline tests for request shapes + helpers added in DE-2565.
//
// These focus on the wire-shape and helper logic that does not require a
// live Studio server — they significantly boost coverage of client.rs.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_job_run_request_shape {
    use super::*;
    use crate::api::Parameter;
    use std::collections::HashMap;

    #[test]
    fn empty_env_and_data_serialize_as_empty_objects() {
        let req = JobRunRequest {
            name: "edgefirst-validator".into(),
            job_name: "smoke".into(),
            env: HashMap::new(),
            data: HashMap::new(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["name"], "edgefirst-validator");
        assert_eq!(json["env"], serde_json::json!({}));
        assert_eq!(json["data"], serde_json::json!({}));
    }

    #[test]
    fn data_passes_through_parameter_object_payloads() {
        // Confirms the Parameter wrapper survives JSON serialization round-trip
        // for the kind of structured chart payload that exercises Parameter
        // variants (Real, Integer, String, Array, Object, Boolean).
        let req = JobRunRequest {
            name: "edgefirst-validator".into(),
            job_name: "feat".into(),
            env: HashMap::new(),
            data: HashMap::from([
                ("flag".into(), Parameter::Boolean(true)),
                ("epochs".into(), Parameter::Integer(50)),
                ("lr".into(), Parameter::Real(1e-3)),
                ("name".into(), Parameter::String("hello".into())),
            ]),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["data"]["flag"], true);
        assert_eq!(json["data"]["epochs"], 50);
        assert!(json["data"]["lr"].as_f64().unwrap() > 0.0);
        assert_eq!(json["data"]["name"], "hello");
    }
}

#[cfg(test)]
mod tests_task_data_chart_request_shape {
    use super::*;
    use crate::api::{Parameter, TaskID};

    #[test]
    fn chart_add_request_with_params_serializes_object() {
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let params = Parameter::Object(std::collections::HashMap::from([(
            "y_axis".into(),
            Parameter::String("log".into()),
        )]));
        let data = Parameter::Object(std::collections::HashMap::from([(
            "type".into(),
            Parameter::String("line".into()),
        )]));
        let req = TaskChartAddRequest {
            task_id: task_id.value(),
            group_name: "metrics".into(),
            chart_name: "loss".into(),
            params: Some(params),
            data,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["params"]["y_axis"], "log");
    }

    #[test]
    fn task_data_list_request_round_trips() {
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let req = TaskDataListRequest {
            task_id: task_id.value(),
        };
        let json = serde_json::to_string(&req).unwrap();
        // Field order is stable for a single-field struct, so an exact match
        // is meaningful here.
        assert_eq!(json, format!("{{\"task_id\":{}}}", task_id.value()));
    }

    #[test]
    fn task_data_download_request_treats_folder_and_file_independently() {
        let task_id = TaskID::try_from("task-1a2b").unwrap();
        let req = TaskDataDownloadRequest {
            task_id: task_id.value(),
            folder: "validation/run-01".into(),
            file: "metrics.json".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        // Server takes folder + file separately (not a single combined path)
        // so callers don't have to escape slashes themselves.
        assert_eq!(json["folder"], "validation/run-01");
        assert_eq!(json["file"], "metrics.json");
    }
}

#[cfg(test)]
mod tests_val_data_request_shape {
    use super::*;

    #[test]
    fn val_data_list_round_trips() {
        let req = ValDataListRequest { session_id: 2707 };
        let s = serde_json::to_string(&req).unwrap();
        let back: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(back["session_id"], 2707);
    }

    #[test]
    fn val_data_download_round_trips_with_nested_path() {
        let req = ValDataDownloadRequest {
            session_id: 2707,
            filename: "subfolder/imx95.json".into(),
        };
        let s = serde_json::to_string(&req).unwrap();
        let back: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(back["session_id"], 2707);
        assert_eq!(back["filename"], "subfolder/imx95.json");
    }
}

#[cfg(test)]
mod tests_progress_struct {
    use super::*;

    #[test]
    fn progress_can_be_constructed_with_zero_total() {
        // Servers sometimes omit Content-Length; progress events should still
        // be representable. This guards the public field-level API.
        let p = Progress {
            current: 0,
            total: 0,
            status: None,
        };
        assert_eq!(p.current, 0);
        assert_eq!(p.total, 0);
        assert!(p.status.is_none());
    }

    #[test]
    fn progress_tracks_current_independently_of_total() {
        let p = Progress {
            current: 123,
            total: 456,
            status: Some("Downloading".into()),
        };
        assert_eq!(p.current, 123);
        assert_eq!(p.total, 456);
        assert_eq!(p.status.as_deref(), Some("Downloading"));
    }

    #[test]
    fn progress_can_be_cloned() {
        // Progress is consumed by progress sinks which may need to retain a
        // copy independently of the channel — derive(Clone) must hold.
        let p = Progress {
            current: 10,
            total: 20,
            status: Some("phase".into()),
        };
        let q = p.clone();
        assert_eq!(q.current, p.current);
        assert_eq!(q.total, p.total);
        assert_eq!(q.status, p.status);
    }
}

#[cfg(test)]
mod tests_bare_filename_parent {
    // Documents the empty-parent guard added for `rpc_download` so that
    // callers passing a bare filename like "metrics.json" download to the
    // current directory instead of erroring on `create_dir_all("")`.
    use std::path::Path;

    #[test]
    fn bare_filename_parent_is_empty_path() {
        // This is the invariant our guard depends on. If a future Rust
        // release ever changed `Path::parent` for bare filenames, the guard
        // would need revisiting.
        let p = Path::new("metrics.json");
        let parent = p.parent().expect("bare filename always has Some parent");
        assert!(
            parent.as_os_str().is_empty(),
            "Path::parent for bare filename should be empty, got: {parent:?}"
        );
    }

    #[test]
    fn path_with_directory_has_non_empty_parent() {
        // The companion case: when the path includes a directory, the
        // parent is non-empty and `create_dir_all` should be invoked.
        let p = Path::new("dir/metrics.json");
        let parent = p.parent().expect("path-with-dir always has Some parent");
        assert!(!parent.as_os_str().is_empty());
        assert_eq!(parent, Path::new("dir"));
    }
}
