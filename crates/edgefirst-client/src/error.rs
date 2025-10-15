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
    /// Invalid annotation type provided.
    InvalidAnnotationType(String),
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
