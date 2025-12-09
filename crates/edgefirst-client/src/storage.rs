// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Token storage abstraction for EdgeFirst Client.
//!
//! This module provides a trait-based abstraction for token persistence,
//! allowing different storage backends to be used depending on the platform.
//!
//! # Storage Implementations
//!
//! - [`FileTokenStorage`]: Default file-based storage for desktop platforms
//! - [`MemoryTokenStorage`]: In-memory storage (no persistence)
//!
//! # Custom Storage
//!
//! Implement the [`TokenStorage`] trait to create custom storage backends,
//! such as iOS Keychain or Android EncryptedSharedPreferences.
//!
//! # Examples
//!
//! ```rust,no_run
//! use edgefirst_client::{Client, FileTokenStorage, MemoryTokenStorage};
//! use std::sync::Arc;
//!
//! # fn main() -> Result<(), edgefirst_client::Error> {
//! // Use default file storage (desktop platforms)
//! let client = Client::new()?;
//!
//! // Use memory-only storage (no persistence)
//! let client = Client::new()?.with_memory_storage();
//!
//! // Use custom file path
//! let storage = FileTokenStorage::with_path("/custom/path/token".into());
//! let client = Client::new()?.with_storage(Arc::new(storage));
//! # Ok(())
//! # }
//! ```

use directories::ProjectDirs;
use log::debug;
use std::{path::PathBuf, sync::RwLock};

/// Error type for token storage operations.
#[derive(Debug)]
pub enum StorageError {
    /// Storage is not available (e.g., cannot determine config directory).
    NotAvailable(String),
    /// Failed to read token from storage.
    ReadError(String),
    /// Failed to write token to storage.
    WriteError(String),
    /// Failed to clear token from storage.
    ClearError(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::NotAvailable(msg) => write!(f, "Token storage not available: {}", msg),
            StorageError::ReadError(msg) => write!(f, "Failed to read token: {}", msg),
            StorageError::WriteError(msg) => write!(f, "Failed to write token: {}", msg),
            StorageError::ClearError(msg) => write!(f, "Failed to clear token: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

/// Trait for persistent token storage.
///
/// Implement this trait to create custom storage backends for authentication
/// tokens. The storage must be thread-safe (`Send + Sync`).
///
/// # Platform Examples
///
/// - **Desktop**: Use [`FileTokenStorage`] to store tokens in the user's config
///   directory
/// - **iOS**: Implement using Keychain Services
/// - **Android**: Implement using EncryptedSharedPreferences
///
/// # Example Implementation
///
/// ```rust,ignore
/// use edgefirst_client::{TokenStorage, StorageError};
///
/// struct KeychainStorage {
///     service: String,
///     account: String,
/// }
///
/// impl TokenStorage for KeychainStorage {
///     fn store(&self, token: &str) -> Result<(), StorageError> {
///         // Store in Keychain
///         Ok(())
///     }
///
///     fn load(&self) -> Result<Option<String>, StorageError> {
///         // Load from Keychain
///         Ok(Some("token".to_string()))
///     }
///
///     fn clear(&self) -> Result<(), StorageError> {
///         // Remove from Keychain
///         Ok(())
///     }
/// }
/// ```
pub trait TokenStorage: Send + Sync {
    /// Store the authentication token.
    fn store(&self, token: &str) -> Result<(), StorageError>;

    /// Load the stored authentication token.
    ///
    /// Returns `Ok(None)` if no token is stored.
    fn load(&self) -> Result<Option<String>, StorageError>;

    /// Clear the stored authentication token.
    fn clear(&self) -> Result<(), StorageError>;
}

/// File-based token storage for desktop platforms.
///
/// Stores the authentication token in a file on the local filesystem. By
/// default, uses the platform-specific config directory
/// (e.g., `~/.config/EdgeFirst Studio/token` on Linux).
///
/// # Examples
///
/// ```rust,no_run
/// use edgefirst_client::FileTokenStorage;
/// use std::path::PathBuf;
///
/// // Use default path
/// let storage = FileTokenStorage::new().unwrap();
///
/// // Use custom path
/// let storage = FileTokenStorage::with_path(PathBuf::from("/custom/path/token"));
/// ```
#[derive(Debug, Clone)]
pub struct FileTokenStorage {
    path: PathBuf,
}

impl FileTokenStorage {
    /// Create a new `FileTokenStorage` using the default platform config
    /// directory.
    ///
    /// The default path is determined by the `directories` crate:
    /// - Linux: `~/.config/EdgeFirst Studio/token`
    /// - macOS: `~/Library/Application
    ///   Support/ai.EdgeFirst.EdgeFirst-Studio/token`
    /// - Windows: `C:\Users\<User>\AppData\Roaming\EdgeFirst\EdgeFirst
    ///   Studio\token`
    pub fn new() -> Result<Self, StorageError> {
        let path = ProjectDirs::from("ai", "EdgeFirst", "EdgeFirst Studio")
            .ok_or_else(|| {
                StorageError::NotAvailable("Could not determine user config directory".to_string())
            })?
            .config_dir()
            .join("token");

        debug!("FileTokenStorage using default path: {:?}", path);
        Ok(Self { path })
    }

    /// Create a new `FileTokenStorage` with a custom file path.
    pub fn with_path(path: PathBuf) -> Self {
        debug!("FileTokenStorage using custom path: {:?}", path);
        Self { path }
    }

    /// Returns the path where the token is stored.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl TokenStorage for FileTokenStorage {
    fn store(&self, token: &str) -> Result<(), StorageError> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                StorageError::WriteError(format!("Failed to create directory {:?}: {}", parent, e))
            })?;
        }

        std::fs::write(&self.path, token).map_err(|e| {
            StorageError::WriteError(format!("Failed to write token to {:?}: {}", self.path, e))
        })?;

        debug!("Token stored to {:?}", self.path);
        Ok(())
    }

    fn load(&self) -> Result<Option<String>, StorageError> {
        if !self.path.exists() {
            debug!("No token file found at {:?}", self.path);
            return Ok(None);
        }

        let token = std::fs::read_to_string(&self.path).map_err(|e| {
            StorageError::ReadError(format!("Failed to read token from {:?}: {}", self.path, e))
        })?;

        if token.is_empty() {
            debug!("Token file at {:?} is empty", self.path);
            return Ok(None);
        }

        debug!("Token loaded from {:?}", self.path);
        Ok(Some(token))
    }

    fn clear(&self) -> Result<(), StorageError> {
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|e| {
                StorageError::ClearError(format!(
                    "Failed to remove token file {:?}: {}",
                    self.path, e
                ))
            })?;
            debug!("Token file removed from {:?}", self.path);
        }
        Ok(())
    }
}

/// In-memory token storage (no persistence).
///
/// Stores the authentication token in memory only. The token is lost when the
/// application exits. This is useful for:
///
/// - Testing
/// - Mobile platforms that use custom secure storage
/// - Applications that don't need token persistence
///
/// # Examples
///
/// ```rust
/// use edgefirst_client::{MemoryTokenStorage, TokenStorage};
///
/// let storage = MemoryTokenStorage::new();
/// storage.store("my-token").unwrap();
/// assert_eq!(storage.load().unwrap(), Some("my-token".to_string()));
/// storage.clear().unwrap();
/// assert_eq!(storage.load().unwrap(), None);
/// ```
#[derive(Debug, Default)]
pub struct MemoryTokenStorage {
    token: RwLock<Option<String>>,
}

impl MemoryTokenStorage {
    /// Create a new `MemoryTokenStorage`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl TokenStorage for MemoryTokenStorage {
    fn store(&self, token: &str) -> Result<(), StorageError> {
        let mut guard = self.token.write().map_err(|e| {
            StorageError::WriteError(format!("Failed to acquire write lock: {}", e))
        })?;
        *guard = Some(token.to_string());
        Ok(())
    }

    fn load(&self) -> Result<Option<String>, StorageError> {
        let guard = self
            .token
            .read()
            .map_err(|e| StorageError::ReadError(format!("Failed to acquire read lock: {}", e)))?;
        Ok(guard.clone())
    }

    fn clear(&self) -> Result<(), StorageError> {
        let mut guard = self.token.write().map_err(|e| {
            StorageError::ClearError(format!("Failed to acquire write lock: {}", e))
        })?;
        *guard = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn test_memory_storage_store_load_clear() {
        let storage = MemoryTokenStorage::new();

        // Initially empty
        assert_eq!(storage.load().unwrap(), None);

        // Store token
        storage.store("test-token").unwrap();
        assert_eq!(storage.load().unwrap(), Some("test-token".to_string()));

        // Clear token
        storage.clear().unwrap();
        assert_eq!(storage.load().unwrap(), None);
    }

    #[test]
    fn test_memory_storage_overwrite() {
        let storage = MemoryTokenStorage::new();

        storage.store("token-1").unwrap();
        assert_eq!(storage.load().unwrap(), Some("token-1".to_string()));

        storage.store("token-2").unwrap();
        assert_eq!(storage.load().unwrap(), Some("token-2".to_string()));
    }

    #[test]
    fn test_memory_storage_thread_safety() {
        let storage = Arc::new(MemoryTokenStorage::new());
        let storage_clone = Arc::clone(&storage);

        let handle = std::thread::spawn(move || {
            storage_clone.store("thread-token").unwrap();
        });

        handle.join().unwrap();
        assert_eq!(storage.load().unwrap(), Some("thread-token".to_string()));
    }

    #[test]
    fn test_file_storage_store_load_clear() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("token");
        let storage = FileTokenStorage::with_path(token_path.clone());

        // Initially empty (file doesn't exist)
        assert_eq!(storage.load().unwrap(), None);

        // Store token
        storage.store("file-test-token").unwrap();
        assert!(token_path.exists());
        assert_eq!(storage.load().unwrap(), Some("file-test-token".to_string()));

        // Clear token
        storage.clear().unwrap();
        assert!(!token_path.exists());
        assert_eq!(storage.load().unwrap(), None);
    }

    #[test]
    fn test_file_storage_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("nested").join("dirs").join("token");
        let storage = FileTokenStorage::with_path(token_path.clone());

        storage.store("nested-token").unwrap();
        assert!(token_path.exists());
        assert_eq!(storage.load().unwrap(), Some("nested-token".to_string()));
    }

    #[test]
    fn test_file_storage_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("token");
        let storage = FileTokenStorage::with_path(token_path);

        storage.store("token-1").unwrap();
        assert_eq!(storage.load().unwrap(), Some("token-1".to_string()));

        storage.store("token-2").unwrap();
        assert_eq!(storage.load().unwrap(), Some("token-2".to_string()));
    }

    #[test]
    fn test_file_storage_clear_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("nonexistent_token");
        let storage = FileTokenStorage::with_path(token_path);

        // Should not error when clearing nonexistent file
        assert!(storage.clear().is_ok());
    }

    #[test]
    fn test_file_storage_path() {
        let path = PathBuf::from("/custom/path/token");
        let storage = FileTokenStorage::with_path(path.clone());
        assert_eq!(storage.path(), &path);
    }

    #[test]
    fn test_storage_error_display() {
        let err = StorageError::NotAvailable("test".to_string());
        assert!(err.to_string().contains("test"));
        assert!(err.to_string().contains("not available"));

        let err = StorageError::ReadError("read failed".to_string());
        assert!(err.to_string().contains("read failed"));

        let err = StorageError::WriteError("write failed".to_string());
        assert!(err.to_string().contains("write failed"));

        let err = StorageError::ClearError("clear failed".to_string());
        assert!(err.to_string().contains("clear failed"));
    }
}
