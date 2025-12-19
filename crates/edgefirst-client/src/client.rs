// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use crate::{
    Annotation, Error, Sample, Task,
    api::{
        AnnotationSetID, Artifact, DatasetID, Experiment, ExperimentID, LoginResult, Organization,
        Project, ProjectID, SamplesCountResult, SamplesListParams, SamplesListResult, Snapshot,
        SnapshotCreateFromDataset, SnapshotFromDatasetResult, SnapshotID, SnapshotRestore,
        SnapshotRestoreResult, Stage, TaskID, TaskInfo, TaskStages, TaskStatus, TasksListParams,
        TasksListResult, TrainingSession, TrainingSessionID, ValidationSession,
        ValidationSessionID,
    },
    dataset::{AnnotationSet, AnnotationType, Dataset, FileType, Label, NewLabel, NewLabelObject},
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

static PART_SIZE: usize = 100 * 1024 * 1024;

fn max_tasks() -> usize {
    std::env::var("MAX_TASKS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            // Default to half the number of CPUs, minimum 2, maximum 8
            // Lower max prevents timeout issues with large file uploads
            let cpus = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            (cpus / 2).clamp(2, 8)
        })
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
/// downloads, or dataset processing. It provides the current count and total
/// count to enable progress reporting in applications.
///
/// # Examples
///
/// ```rust
/// use edgefirst_client::Progress;
///
/// let progress = Progress {
///     current: 25,
///     total: 100,
/// };
/// let percentage = (progress.current as f64 / progress.total as f64) * 100.0;
/// println!(
///     "Progress: {:.1}% ({}/{})",
///     percentage, progress.current, progress.total
/// );
/// ```
#[derive(Debug, Clone)]
pub struct Progress {
    /// Current number of completed items.
    pub current: usize,
    /// Total number of items to process.
    pub total: usize,
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
            .unwrap_or(30); // Default 30s timeout for API calls

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
                    warn!("Failed to extract server from token: {}. Using default server.", e);
                    "https://edgefirst.studio".to_string()
                }
            }
        } else {
            "https://edgefirst.studio".to_string()
        };

        Ok(Client {
            http,
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
        let url = match server {
            "" | "saas" => "https://edgefirst.studio".to_string(),
            name => format!("https://{}.edgefirst.studio", name),
        };

        // Clear token from storage when changing servers to prevent
        // authentication issues with stale tokens from different instances
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
                    Ok(Client {
                        token_path: Some(token_path),
                        storage: None,
                        ..self.clone()
                    })
                }
            }
        } else {
            Ok(Client {
                token_path: Some(token_path),
                storage: None,
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
    pub async fn token(&self) -> String {
        self.token.read().await.clone()
    }

    /// Verify the token used to authenticate the client with the server.  This
    /// method is used to ensure that the token is still valid and has not
    /// expired.  If the token is invalid, the server will return an error and
    /// the client will need to login again.
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
    pub async fn username(&self) -> Result<String, Error> {
        match self.token_field("username").await? {
            serde_json::Value::String(username) => Ok(username),
            _ => Err(Error::InvalidToken),
        }
    }

    /// Returns the expiration time for the current token.
    pub async fn token_expiration(&self) -> Result<DateTime<Utc>, Error> {
        let ts = match self.token_field("exp").await? {
            serde_json::Value::Number(exp) => exp.as_i64().ok_or(Error::InvalidToken)?,
            _ => return Err(Error::InvalidToken),
        };

        match DateTime::<Utc>::from_timestamp_secs(ts) {
            Some(dt) => Ok(dt),
            None => Err(Error::InvalidToken),
        }
    }

    /// Returns the organization information for the current user.
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
    pub async fn dataset(&self, dataset_id: DatasetID) -> Result<Dataset, Error> {
        let params = HashMap::from([("dataset_id", dataset_id)]);
        self.rpc("dataset.get".to_owned(), Some(params)).await
    }

    /// Lists the labels for the specified dataset.
    pub async fn labels(&self, dataset_id: DatasetID) -> Result<Vec<Label>, Error> {
        let params = HashMap::from([("dataset_id", dataset_id)]);
        self.rpc("label.list".to_owned(), Some(params)).await
    }

    /// Add a new label to the dataset with the specified name.
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
    pub async fn delete_dataset(&self, dataset_id: DatasetID) -> Result<(), Error> {
        let params = HashMap::from([("id", dataset_id)]);
        let _: String = self.rpc("dataset.delete".to_owned(), Some(params)).await?;
        Ok(())
    }

    /// Updates the label with the specified ID to have the new name or index.
    /// Label IDs cannot be changed.  Label IDs are globally unique so the
    /// dataset_id is not required.
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

    /// Downloads dataset samples to the local filesystem.
    ///
    /// # Arguments
    ///
    /// * `dataset_id` - The unique identifier of the dataset
    /// * `groups` - Dataset groups to include (e.g., "train", "val")
    /// * `file_types` - File types to download (e.g., Image, LidarPcd)
    /// * `output` - Local directory to save downloaded files
    /// * `flatten` - If true, download all files to output root without
    ///   sequence subdirectories. When flattening, filenames are prefixed with
    ///   `{sequence_name}_{frame}_` (or `{sequence_name}_` if frame is
    ///   unavailable) unless the filename already starts with
    ///   `{sequence_name}_`, to avoid conflicts between sequences.
    /// * `progress` - Optional channel for progress updates
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
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_dataset(
        &self,
        dataset_id: DatasetID,
        groups: &[String],
        file_types: &[FileType],
        output: PathBuf,
        flatten: bool,
        progress: Option<Sender<Progress>>,
    ) -> Result<(), Error> {
        let samples = self
            .samples(dataset_id, None, &[], groups, file_types, progress.clone())
            .await?;
        fs::create_dir_all(&output).await?;

        let client = self.clone();
        let file_types = file_types.to_vec();
        let output = output.clone();

        parallel_foreach_items(samples, progress, move |sample| {
            let client = client.clone();
            let file_types = file_types.clone();
            let output = output.clone();

            async move {
                for file_type in file_types {
                    if let Some(data) = sample.download(&client, file_type.clone()).await? {
                        let (file_ext, is_image) = match file_type.clone() {
                            FileType::Image => (
                                infer::get(&data)
                                    .expect("Failed to identify image file format for sample")
                                    .extension()
                                    .to_string(),
                                true,
                            ),
                            other => (other.to_string(), false),
                        };

                        // Determine target directory based on sequence membership and flatten
                        // option
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
                                Self::build_filename(
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
                            Self::build_filename(
                                &base_name,
                                flatten,
                                sequence_dir.as_ref(),
                                sample.frame_number(),
                            )
                        };

                        let file_path = target_dir.join(&file_name);

                        let mut file = File::create(&file_path).await?;
                        file.write_all(&data).await?;
                    } else {
                        warn!(
                            "No data for sample: {}",
                            sample
                                .id()
                                .map(|id| id.to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        );
                    }
                }

                Ok(())
            }
        })
        .await
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
    pub async fn delete_annotation_set(
        &self,
        annotation_set_id: AnnotationSetID,
    ) -> Result<(), Error> {
        let params = HashMap::from([("id", annotation_set_id)]);
        let _: String = self.rpc("annset.delete".to_owned(), Some(params)).await?;
        Ok(())
    }

    /// Retrieve the annotation set with the specified ID.
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
    /// To get the annotations as a DataFrame, use the `annotations_dataframe`
    /// method instead.
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
                let _ = progress.send(Progress { current, total }).await;
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

    pub async fn samples_count(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
        annotation_types: &[AnnotationType],
        groups: &[String],
        types: &[FileType],
    ) -> Result<SamplesCountResult, Error> {
        let types = annotation_types
            .iter()
            .map(|t| t.to_string())
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

    pub async fn samples(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
        annotation_types: &[AnnotationType],
        groups: &[String],
        types: &[FileType],
        progress: Option<Sender<Progress>>,
    ) -> Result<Vec<Sample>, Error> {
        let types_vec = annotation_types
            .iter()
            .map(|t| t.to_string())
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
                let _ = progress.send(Progress { current, total }).await;
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
    ///   the `imagesize` crate. The width/height are sent to the server, but
    ///   note that the server currently doesn't return these fields when
    ///   fetching samples back.
    /// - **UUIDs are generated automatically** if not provided. If you need
    ///   deterministic UUIDs, set `sample.uuid` explicitly before calling. Note
    ///   that the server doesn't currently return UUIDs in sample queries.
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
    pub async fn populate_samples(
        &self,
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
        samples: Vec<Sample>,
        progress: Option<Sender<Progress>>,
    ) -> Result<Vec<crate::SamplesPopulateResult>, Error> {
        use crate::api::SamplesPopulateParams;

        // Track which files need to be uploaded
        let mut files_to_upload: Vec<(String, String, PathBuf, String)> = Vec::new();

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
            self.upload_sample_files(&results, files_to_upload, progress)
                .await?;
        }

        Ok(results)
    }

    fn prepare_samples_for_upload(
        &self,
        samples: Vec<Sample>,
        files_to_upload: &mut Vec<(String, String, PathBuf, String)>,
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
        files_to_upload: &mut Vec<(String, String, PathBuf, String)>,
    ) -> crate::SampleFile {
        use std::path::Path;

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
                    path.to_path_buf(),
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
        files_to_upload: Vec<(String, String, PathBuf, String)>,
        progress: Option<Sender<Progress>>,
    ) -> Result<(), Error> {
        // Build a map from (sample_uuid, basename) -> local_path
        let mut upload_map: HashMap<(String, String), PathBuf> = HashMap::new();
        for (uuid, _file_type, path, basename) in files_to_upload {
            upload_map.insert((uuid, basename), path);
        }

        let http = self.http.clone();

        // Extract the data we need for parallel upload
        let upload_tasks: Vec<_> = results
            .iter()
            .map(|result| (result.uuid.clone(), result.urls.clone()))
            .collect();

        parallel_foreach_items(upload_tasks, progress.clone(), move |(uuid, urls)| {
            let http = http.clone();
            let upload_map = upload_map.clone();

            async move {
                // Upload all files for this sample
                for url_info in &urls {
                    if let Some(local_path) =
                        upload_map.get(&(uuid.clone(), url_info.filename.clone()))
                    {
                        // Upload the file
                        upload_file_to_presigned_url(
                            http.clone(),
                            &url_info.url,
                            local_path.clone(),
                        )
                        .await?;
                    }
                }

                Ok(())
            }
        })
        .await
    }

    pub async fn download(&self, url: &str) -> Result<Vec<u8>, Error> {
        // Uses default 120s timeout from client
        let resp = self.http.get(url).send().await?;

        if !resp.status().is_success() {
            return Err(Error::HttpError(resp.error_for_status().unwrap_err()));
        }

        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Get the AnnotationGroup for the specified annotation set with the
    /// requested annotation types.  The annotation type is used to filter
    /// the annotations returned.  Images which do not have any annotations
    /// are included in the result.
    ///
    /// Get annotations as a DataFrame (2025.01 schema).
    ///
    /// **DEPRECATED**: Use [`Client::samples_dataframe()`] instead for full
    /// 2025.10 schema support including optional metadata columns.
    ///
    /// The result is a DataFrame following the EdgeFirst Dataset Format
    /// definition with 9 columns (original schema). Does not include new
    /// optional columns added in 2025.10.
    ///
    /// # Migration
    ///
    /// ```rust,no_run
    /// # use edgefirst_client::Client;
    /// # async fn example() -> Result<(), edgefirst_client::Error> {
    /// # let client = Client::new()?;
    /// # let dataset_id = 1.into();
    /// # let annotation_set_id = 1.into();
    /// # let groups = vec![];
    /// # let types = vec![];
    /// // OLD (deprecated):
    /// let df = client
    ///     .annotations_dataframe(annotation_set_id, &groups, &types, None)
    ///     .await?;
    ///
    /// // NEW (recommended):
    /// let df = client
    ///     .samples_dataframe(dataset_id, Some(annotation_set_id), &groups, &types, None)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// To get the annotations as a vector of Annotation objects, use the
    /// `annotations` method instead.
    #[deprecated(
        since = "0.8.0",
        note = "Use `samples_dataframe()` for complete 2025.10 schema support"
    )]
    #[cfg(feature = "polars")]
    pub async fn annotations_dataframe(
        &self,
        annotation_set_id: AnnotationSetID,
        groups: &[String],
        types: &[AnnotationType],
        progress: Option<Sender<Progress>>,
    ) -> Result<DataFrame, Error> {
        #[allow(deprecated)]
        use crate::dataset::annotations_dataframe;

        let annotations = self
            .annotations(annotation_set_id, groups, types, progress)
            .await?;
        #[allow(deprecated)]
        annotations_dataframe(&annotations)
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

    /// List available snapshots.  If a name is provided, only snapshots
    /// containing that name are returned.
    ///
    /// Results are sorted by match quality: exact matches first, then
    /// case-insensitive exact matches, then shorter descriptions (more
    /// specific), then alphabetically.
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
    ///     while let Some(Progress { current, total }) = rx.recv().await {
    ///         println!(
    ///             "Upload: {}/{} bytes ({:.1}%)",
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
            let _ = progress.send(Progress { current: 0, total }).await;
        }

        let params = SnapshotCreateMultipartParams {
            snapshot_name: name.to_owned(),
            keys: vec![name.to_owned()],
            file_sizes: vec![total],
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
            self.http.clone(),
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
            let _ = progress.send(Progress { current: 0, total }).await;
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
                self.http.clone(),
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
    pub async fn delete_snapshot(&self, snapshot_id: SnapshotID) -> Result<(), Error> {
        let params = HashMap::from([("snapshot_id", snapshot_id)]);
        let _: String = self
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
    ///     while let Some(Progress { current, total }) = rx.recv().await {
    ///         println!("Download: {}/{} bytes", current, total);
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

        let total = Arc::new(AtomicUsize::new(0));
        let current = Arc::new(AtomicUsize::new(0));
        let sem = Arc::new(Semaphore::new(max_tasks()));

        let tasks = items
            .iter()
            .map(|(key, url)| {
                let http = self.http.clone();
                let key = key.clone();
                let url = url.clone();
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
                    let content_length = res.content_length().unwrap_or(0) as usize;

                    if let Some(progress) = &progress {
                        let total = total.fetch_add(content_length, Ordering::SeqCst);
                        let _ = progress
                            .send(Progress {
                                current: current.load(Ordering::SeqCst),
                                total: total + content_length,
                            })
                            .await;
                    }

                    let mut file = File::create(output.join(key)).await?;
                    let mut stream = res.bytes_stream();

                    while let Some(chunk) = stream.next().await {
                        let chunk = chunk?;
                        file.write_all(&chunk).await?;
                        let len = chunk.len();

                        if let Some(progress) = &progress {
                            let total = total.load(Ordering::SeqCst);
                            let current = current.fetch_add(len, Ordering::SeqCst);

                            let _ = progress
                                .send(Progress {
                                    current: current + len,
                                    total,
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
    pub async fn training_session(
        &self,
        session_id: TrainingSessionID,
    ) -> Result<TrainingSession, Error> {
        let params = HashMap::from([("trainer_session_id", session_id)]);
        self.rpc("trainer.session.get".to_owned(), Some(params))
            .await
    }

    /// List validation sessions for the given project.
    pub async fn validation_sessions(
        &self,
        project_id: ProjectID,
    ) -> Result<Vec<ValidationSession>, Error> {
        let params = HashMap::from([("project_id", project_id)]);
        self.rpc("validate.session.list".to_owned(), Some(params))
            .await
    }

    /// Retrieve a specific validation session.
    pub async fn validation_session(
        &self,
        session_id: ValidationSessionID,
    ) -> Result<ValidationSession, Error> {
        let params = HashMap::from([("validate_session_id", session_id)]);
        self.rpc("validate.session.get".to_owned(), Some(params))
            .await
    }

    /// List the artifacts for the specified trainer session.  The artifacts
    /// are returned as a vector of strings.
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
    /// the current directory with the same filename.  A progress callback can
    /// be provided to monitor the progress of the download over a watch
    /// channel.
    pub async fn download_artifact(
        &self,
        training_session_id: TrainingSessionID,
        modelname: &str,
        filename: Option<PathBuf>,
        progress: Option<Sender<Progress>>,
    ) -> Result<(), Error> {
        let filename = filename.unwrap_or_else(|| PathBuf::from(modelname));
        let resp = self
            .http
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

        if let Some(progress) = progress {
            let total = resp.content_length().unwrap_or(0) as usize;
            let _ = progress.send(Progress { current: 0, total }).await;

            let mut file = File::create(filename).await?;
            let mut current = 0;
            let mut stream = resp.bytes_stream();

            while let Some(item) = stream.next().await {
                let chunk = item?;
                file.write_all(&chunk).await?;
                current += chunk.len();
                let _ = progress.send(Progress { current, total }).await;
            }
        } else {
            let body = resp.bytes().await?;
            fs::write(filename, body).await?;
        }

        Ok(())
    }

    /// Download the model checkpoint associated with the specified trainer
    /// session to the specified file path, if path is not provided it will be
    /// downloaded to the current directory with the same filename.  A progress
    /// callback can be provided to monitor the progress of the download over a
    /// watch channel.
    ///
    /// There is no API for listing checkpoints it is expected that trainers are
    /// aware of possible checkpoints and their names within the checkpoint
    /// folder on the server.
    pub async fn download_checkpoint(
        &self,
        training_session_id: TrainingSessionID,
        checkpoint: &str,
        filename: Option<PathBuf>,
        progress: Option<Sender<Progress>>,
    ) -> Result<(), Error> {
        let filename = filename.unwrap_or_else(|| PathBuf::from(checkpoint));
        let resp = self
            .http
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

        if let Some(progress) = progress {
            let total = resp.content_length().unwrap_or(0) as usize;
            let _ = progress.send(Progress { current: 0, total }).await;

            let mut file = File::create(filename).await?;
            let mut current = 0;
            let mut stream = resp.bytes_stream();

            while let Some(item) = stream.next().await {
                let chunk = item?;
                file.write_all(&chunk).await?;
                current += chunk.len();
                let _ = progress.send(Progress { current, total }).await;
            }
        } else {
            let body = resp.bytes().await?;
            fs::write(filename, body).await?;
        }

        Ok(())
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

    /// Retrieve the task information and status.
    pub async fn task_info(&self, task_id: TaskID) -> Result<TaskInfo, Error> {
        self.rpc(
            "task.get".to_owned(),
            Some(HashMap::from([("id", task_id)])),
        )
        .await
    }

    /// Updates the tasks status.
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

    /// Raw fetch from the Studio server is used for downloading files.
    pub async fn fetch(&self, query: &str) -> Result<Vec<u8>, Error> {
        let req = self
            .http
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
    pub async fn post_multipart(&self, method: &str, form: Form) -> Result<String, Error> {
        let req = self
            .http
            .post(format!("{}/api?method={}", self.url, method))
            .header("Accept", "application/json")
            .header("User-Agent", "EdgeFirst Client")
            .header("Authorization", format!("Bearer {}", self.token().await))
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

            let response: RpcResponse<String> = match serde_json::from_slice(&body) {
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
            let err = resp.error_for_status_ref().unwrap_err();
            Err(Error::HttpError(err))
        }
    }

    /// Send a JSON-RPC request to the server.  The method is the name of the
    /// method to call on the server.  The params are the parameters to pass to
    /// the method.  The method and params are serialized into a JSON-RPC
    /// request and sent to the server.  The response is deserialized into
    /// the specified type and returned to the caller.
    ///
    /// NOTE: This API would generally not be called directly and instead users
    /// should use the higher-level methods provided by the client.
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

    async fn rpc_without_auth<Params, RpcResult>(
        &self,
        method: String,
        params: Option<Params>,
    ) -> Result<RpcResult, Error>
    where
        Params: Serialize,
        RpcResult: DeserializeOwned,
    {
        let request = RpcRequest {
            method,
            params,
            ..Default::default()
        };

        if log_enabled!(Level::Trace) {
            trace!(
                "RPC Request: {}",
                serde_json::ser::to_string_pretty(&request)?
            );
        }

        let url = format!("{}/api", self.url);

        // Use client-level timeout (allows retry mechanism to work properly)
        // Per-request timeout overrides can prevent retries from functioning
        let res = self
            .http
            .post(&url)
            .header("Accept", "application/json")
            .header("User-Agent", "EdgeFirst Client")
            .header("Authorization", format!("Bearer {}", self.token().await))
            .json(&request)
            .send()
            .await?;

        self.process_rpc_response(res).await
    }

    async fn process_rpc_response<RpcResult>(
        &self,
        res: reqwest::Response,
    ) -> Result<RpcResult, Error>
    where
        RpcResult: DeserializeOwned,
    {
        let body = res.bytes().await?;

        if log_enabled!(Level::Trace) {
            trace!("RPC Response: {}", String::from_utf8_lossy(&body));
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
/// - Semaphore limiting concurrent tasks to `max_tasks()` (configurable via
///   `MAX_TASKS` environment variable, default: half of CPU cores, min 2, max
///   8)
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
/// * `work_fn` - Async function to execute for each item
///
/// # Examples
///
/// ```rust,ignore
/// parallel_foreach_items(samples, progress, |sample| async move {
///     // Process sample
///     sample.download(&client, file_type).await?;
///     Ok(())
/// }).await?;
/// ```
async fn parallel_foreach_items<T, F, Fut>(
    items: Vec<T>,
    progress: Option<Sender<Progress>>,
    work_fn: F,
) -> Result<(), Error>
where
    T: Send + 'static,
    F: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), Error>> + Send + 'static,
{
    let total = items.len();
    let current = Arc::new(AtomicUsize::new(0));
    let sem = Arc::new(Semaphore::new(max_tasks()));
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
    current: Arc<AtomicUsize>,
    progress: Option<Sender<Progress>>,
) -> Result<SnapshotCompleteMultipartParams, Error> {
    let filesize = path.metadata()?.len() as usize;
    let n_parts = filesize.div_ceil(PART_SIZE);
    let sem = Arc::new(Semaphore::new(max_tasks()));

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

    // Upload all parts in parallel with concurrency limiting
    let tasks = (0..n_parts)
        .map(|part| {
            let http = http.clone();
            let url = urls[part].clone();
            let etags = etags.clone();
            let path = path.to_owned();
            let sem = sem.clone();
            let progress = progress.clone();
            let current = current.clone();

            tokio::spawn(async move {
                // Acquire semaphore permit to limit concurrent uploads
                let _permit = sem.acquire().await?;

                // Upload part (retry is handled by reqwest client)
                let etag =
                    upload_part(http.clone(), url.clone(), path.clone(), part, n_parts).await?;

                // Store ETag for this part (needed to complete multipart upload)
                let mut etags = etags.lock().await;
                etags[part] = EtagPart {
                    etag,
                    part_number: part + 1,
                };

                // Update progress counter
                let current = current.fetch_add(PART_SIZE, Ordering::SeqCst);
                if let Some(progress) = &progress {
                    let _ = progress
                        .send(Progress {
                            current: current + PART_SIZE,
                            total,
                        })
                        .await;
                }

                Ok::<(), Error>(())
            })
        })
        .collect::<Vec<_>>();

    // Wait for all parts to complete
    join_all(tasks)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SnapshotCompleteMultipartParams {
        key,
        upload_id,
        etag_list: etags.lock().await.clone(),
    })
}

async fn upload_part(
    http: reqwest::Client,
    url: String,
    path: PathBuf,
    part: usize,
    n_parts: usize,
) -> Result<String, Error> {
    let filesize = path.metadata()?.len() as usize;
    let mut file = File::open(path).await?;
    file.seek(SeekFrom::Start((part * PART_SIZE) as u64))
        .await?;
    let file = file.take(PART_SIZE as u64);

    let body_length = if part + 1 == n_parts {
        filesize % PART_SIZE
    } else {
        PART_SIZE
    };

    let stream = FramedRead::new(file, BytesCodec::new());
    let body = Body::wrap_stream(stream);

    let resp = http
        .put(url.clone())
        .header(CONTENT_LENGTH, body_length)
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
async fn upload_file_to_presigned_url(
    http: reqwest::Client,
    url: &str,
    path: PathBuf,
) -> Result<(), Error> {
    // Read the entire file into memory
    let file_data = fs::read(&path).await?;
    let file_size = file_data.len();

    // Upload (retry is handled by reqwest client)
    let resp = http
        .put(url)
        .header(CONTENT_LENGTH, file_size)
        .body(file_data)
        .send()
        .await?;

    if resp.status().is_success() {
        debug!(
            "Successfully uploaded file: {:?} ({} bytes)",
            path, file_size
        );
        Ok(())
    } else {
        let status = resp.status();
        let error_text = resp.text().await.unwrap_or_default();
        Err(Error::InvalidParameters(format!(
            "Upload failed: HTTP {} - {}",
            status, error_text
        )))
    }
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
}
