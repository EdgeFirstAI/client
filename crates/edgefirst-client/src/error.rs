// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use crate::Progress;
use tokio::sync::{AcquireError, watch};

/// Comprehensive error type for EdgeFirst Studio Client operations.
///
/// This enum covers all possible error conditions that can occur when using
/// the EdgeFirst Studio Client, from network issues to authentication problems
/// and data validation errors.
#[derive(Debug)]
pub enum Error {
    /// An I/O error occurred during file operations.
    IoError(std::io::Error),
    /// Configuration parsing or loading error.
    ConfigError(config::ConfigError),
    /// JSON serialization or deserialization error.
    JsonError(serde_json::Error),
    /// HTTP request error from the reqwest client.
    HttpError(reqwest::Error),
    /// Maximum number of retries exceeded for an operation.
    MaxRetriesExceeded(u32),
    /// URL parsing error.
    UrlParseError(url::ParseError),
    /// RPC error with error code and message from the server.
    RpcError(i32, String),
    /// Invalid RPC request ID format.
    InvalidRpcId(String),
    /// Environment variable error.
    EnvError(std::env::VarError),
    /// Semaphore acquisition error for concurrent operations.
    SemaphoreError(AcquireError),
    /// Async task join error.
    JoinError(tokio::task::JoinError),
    /// Error sending progress updates.
    ProgressSendError(watch::error::SendError<Progress>),
    /// Error receiving progress updates.
    ProgressRecvError(watch::error::RecvError),
    /// Path prefix stripping error.
    StripPrefixError(std::path::StripPrefixError),
    /// Integer parsing error.
    ParseIntError(std::num::ParseIntError),
    /// Server returned an invalid or unexpected response.
    InvalidResponse,
    /// Requested functionality is not yet implemented.
    NotImplemented,
    /// File part size exceeds the maximum allowed limit.
    PartTooLarge,
    /// Invalid file type provided.
    InvalidFileType(String),
    /// Invalid annotation type provided.
    InvalidAnnotationType(String),
    /// Unsupported file format.
    UnsupportedFormat(String),
    /// Required image files are missing from the dataset.
    MissingImages(String),
    /// Required annotation files are missing from the dataset.
    MissingAnnotations(String),
    /// Referenced label is missing or not found.
    MissingLabel(String),
    /// Invalid parameters provided to an operation.
    InvalidParameters(String),
    /// Attempted to use a feature that is not enabled.
    FeatureNotEnabled(String),
    /// Authentication token is empty or not provided.
    EmptyToken,
    /// Authentication token format is invalid.
    InvalidToken,
    /// Authentication token has expired.
    TokenExpired,
    /// User is not authorized to perform the requested operation.
    Unauthorized,
    /// Invalid or missing ETag header in HTTP response.
    InvalidEtag(String),
    /// Polars dataframe operation error (only with "polars" feature).
    #[cfg(feature = "polars")]
    PolarsError(polars::error::PolarsError),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<config::ConfigError> for Error {
    fn from(err: config::ConfigError) -> Self {
        Error::ConfigError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::JsonError(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::HttpError(err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::UrlParseError(err)
    }
}

impl From<std::env::VarError> for Error {
    fn from(err: std::env::VarError) -> Self {
        Error::EnvError(err)
    }
}

impl From<AcquireError> for Error {
    fn from(err: AcquireError) -> Self {
        Error::SemaphoreError(err)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Error::JoinError(err)
    }
}

impl From<watch::error::SendError<Progress>> for Error {
    fn from(err: watch::error::SendError<Progress>) -> Self {
        Error::ProgressSendError(err)
    }
}

impl From<watch::error::RecvError> for Error {
    fn from(err: watch::error::RecvError) -> Self {
        Error::ProgressRecvError(err)
    }
}

impl From<std::path::StripPrefixError> for Error {
    fn from(err: std::path::StripPrefixError) -> Self {
        Error::StripPrefixError(err)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        Error::ParseIntError(err)
    }
}

#[cfg(feature = "polars")]
impl From<polars::error::PolarsError> for Error {
    fn from(err: polars::error::PolarsError) -> Self {
        Error::PolarsError(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IoError(e) => write!(f, "I/O error: {}", e),
            Error::ConfigError(e) => write!(f, "Configuration error: {}", e),
            Error::JsonError(e) => write!(f, "JSON error: {}", e),
            Error::HttpError(e) => write!(f, "HTTP error: {}", e),
            Error::MaxRetriesExceeded(n) => write!(f, "Maximum retries ({}) exceeded", n),
            Error::UrlParseError(e) => write!(f, "URL parse error: {}", e),
            Error::RpcError(code, msg) => write!(f, "RPC error {}: {}", code, msg),
            Error::InvalidRpcId(id) => write!(f, "Invalid RPC ID: {}", id),
            Error::EnvError(e) => write!(f, "Environment variable error: {}", e),
            Error::SemaphoreError(e) => write!(f, "Semaphore error: {}", e),
            Error::JoinError(e) => write!(f, "Task join error: {}", e),
            Error::ProgressSendError(e) => write!(f, "Progress send error: {}", e),
            Error::ProgressRecvError(e) => write!(f, "Progress receive error: {}", e),
            Error::StripPrefixError(e) => write!(f, "Path prefix error: {}", e),
            Error::ParseIntError(e) => write!(f, "Integer parse error: {}", e),
            Error::InvalidResponse => write!(f, "Invalid server response"),
            Error::NotImplemented => write!(f, "Not implemented"),
            Error::PartTooLarge => write!(f, "File part size exceeds maximum limit"),
            Error::InvalidFileType(s) => write!(f, "Invalid file type: {}", s),
            Error::InvalidAnnotationType(s) => write!(f, "Invalid annotation type: {}", s),
            Error::UnsupportedFormat(s) => write!(f, "Unsupported format: {}", s),
            Error::MissingImages(s) => write!(f, "Missing images: {}", s),
            Error::MissingAnnotations(s) => write!(f, "Missing annotations: {}", s),
            Error::MissingLabel(s) => write!(f, "Missing label: {}", s),
            Error::InvalidParameters(s) => write!(f, "Invalid parameters: {}", s),
            Error::FeatureNotEnabled(s) => write!(f, "Feature not enabled: {}", s),
            Error::EmptyToken => write!(f, "Authentication token is empty"),
            Error::InvalidToken => write!(f, "Invalid authentication token"),
            Error::TokenExpired => write!(f, "Authentication token has expired"),
            Error::Unauthorized => write!(f, "Unauthorized access"),
            Error::InvalidEtag(s) => write!(f, "Invalid ETag header: {}", s),
            #[cfg(feature = "polars")]
            Error::PolarsError(e) => write!(f, "Polars error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IoError(e) => Some(e),
            Error::ConfigError(e) => Some(e),
            Error::JsonError(e) => Some(e),
            Error::HttpError(e) => Some(e),
            Error::UrlParseError(e) => Some(e),
            Error::EnvError(e) => Some(e),
            Error::JoinError(e) => Some(e),
            Error::StripPrefixError(e) => Some(e),
            Error::ParseIntError(e) => Some(e),
            #[cfg(feature = "polars")]
            Error::PolarsError(e) => Some(e),
            _ => None,
        }
    }
}
