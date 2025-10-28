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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // Tests for wrapped error types - follow the pattern:
    // 1. Create inner error
    // 2. Capture inner error string
    // 3. Wrap to custom Error type
    // 4. Capture wrapped error string
    // 5. Verify inner string is substring of wrapped string

    #[test]
    fn test_io_error_wrapping() {
        // 1. Create inner error
        let inner_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        // 2. Capture inner error string
        let inner_str = inner_err.to_string();
        // 3. Wrap to custom Error type
        let wrapped_err: Error = inner_err.into();
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify inner string is substring of wrapped string
        assert!(
            wrapped_str.contains(&inner_str),
            "Wrapped error '{}' should contain inner error '{}'",
            wrapped_str,
            inner_str
        );
        assert!(wrapped_str.starts_with("I/O error: "));
    }

    #[test]
    fn test_config_error_wrapping() {
        // 1. Create inner error - Force a config error by trying to deserialize empty
        //    config to a required struct
        #[derive(Debug, serde::Deserialize)]
        #[allow(dead_code)]
        struct RequiredField {
            required: String,
        }

        let inner_err = config::Config::builder()
            .build()
            .unwrap()
            .try_deserialize::<RequiredField>()
            .unwrap_err();
        // 2. Capture inner error string
        let inner_str = inner_err.to_string();
        // 3. Wrap to custom Error type
        let wrapped_err: Error = inner_err.into();
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify inner string is substring of wrapped string
        assert!(
            wrapped_str.contains(&inner_str),
            "Wrapped error '{}' should contain inner error '{}'",
            wrapped_str,
            inner_str
        );
        assert!(wrapped_str.starts_with("Configuration error: "));
    }

    #[test]
    fn test_json_error_wrapping() {
        // 1. Create inner error - invalid JSON
        let inner_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        // 2. Capture inner error string
        let inner_str = inner_err.to_string();
        // 3. Wrap to custom Error type
        let wrapped_err: Error = inner_err.into();
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify inner string is substring of wrapped string
        assert!(
            wrapped_str.contains(&inner_str),
            "Wrapped error '{}' should contain inner error '{}'",
            wrapped_str,
            inner_str
        );
        assert!(wrapped_str.starts_with("JSON error: "));
    }

    #[test]
    fn test_url_parse_error_wrapping() {
        // 1. Create inner error - invalid URL
        let inner_err = url::Url::parse("not a valid url").unwrap_err();
        // 2. Capture inner error string
        let inner_str = inner_err.to_string();
        // 3. Wrap to custom Error type
        let wrapped_err: Error = inner_err.into();
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify inner string is substring of wrapped string
        assert!(
            wrapped_str.contains(&inner_str),
            "Wrapped error '{}' should contain inner error '{}'",
            wrapped_str,
            inner_str
        );
        assert!(wrapped_str.starts_with("URL parse error: "));
    }

    #[test]
    fn test_env_error_wrapping() {
        // 1. Create inner error - missing environment variable
        let inner_err = std::env::var("NONEXISTENT_VAR_12345").unwrap_err();
        // 2. Capture inner error string
        let inner_str = inner_err.to_string();
        // 3. Wrap to custom Error type
        let wrapped_err: Error = inner_err.into();
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify inner string is substring of wrapped string
        assert!(
            wrapped_str.contains(&inner_str),
            "Wrapped error '{}' should contain inner error '{}'",
            wrapped_str,
            inner_str
        );
        assert!(wrapped_str.starts_with("Environment variable error: "));
    }

    #[test]
    fn test_strip_prefix_error_wrapping() {
        // 1. Create inner error - strip non-existent prefix
        let path = Path::new("/foo/bar");
        let prefix = Path::new("/baz");
        let inner_err = path.strip_prefix(prefix).unwrap_err();
        // 2. Capture inner error string
        let inner_str = inner_err.to_string();
        // 3. Wrap to custom Error type
        let wrapped_err: Error = inner_err.into();
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify inner string is substring of wrapped string
        assert!(
            wrapped_str.contains(&inner_str),
            "Wrapped error '{}' should contain inner error '{}'",
            wrapped_str,
            inner_str
        );
        assert!(wrapped_str.starts_with("Path prefix error: "));
    }

    #[test]
    fn test_parse_int_error_wrapping() {
        // 1. Create inner error - invalid integer string
        let inner_err = "not a number".parse::<i32>().unwrap_err();
        // 2. Capture inner error string
        let inner_str = inner_err.to_string();
        // 3. Wrap to custom Error type
        let wrapped_err: Error = inner_err.into();
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify inner string is substring of wrapped string
        assert!(
            wrapped_str.contains(&inner_str),
            "Wrapped error '{}' should contain inner error '{}'",
            wrapped_str,
            inner_str
        );
        assert!(wrapped_str.starts_with("Integer parse error: "));
    }

    #[cfg(feature = "polars")]
    #[test]
    fn test_polars_error_wrapping() {
        // 1. Create inner error - duplicate column names cause an error
        use polars::prelude::*;
        let inner_err = DataFrame::new(vec![
            Series::new("a".into(), &[1, 2, 3]).into(),
            Series::new("a".into(), &[4, 5, 6]).into(),
        ])
        .unwrap_err();
        // 2. Capture inner error string
        let inner_str = inner_err.to_string();
        // 3. Wrap to custom Error type
        let wrapped_err: Error = inner_err.into();
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify inner string is substring of wrapped string
        assert!(
            wrapped_str.contains(&inner_str),
            "Wrapped error '{}' should contain inner error '{}'",
            wrapped_str,
            inner_str
        );
        assert!(wrapped_str.starts_with("Polars error: "));
    }

    // Tests for wrapped primitive types - follow the pattern:
    // 1. Create random primitive value
    // 2. Capture the primitive as string
    // 3. Wrap to custom Error type
    // 4. Capture wrapped error string
    // 5. Verify primitive string is substring of wrapped string

    #[test]
    fn test_max_retries_exceeded() {
        // 1. Create primitive value
        let retry_count = 42u32;
        // 2. Capture primitive as string
        let primitive_str = retry_count.to_string();
        // 3. Wrap to custom Error type
        let wrapped_err = Error::MaxRetriesExceeded(retry_count);
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(&primitive_str),
            "Wrapped error '{}' should contain retry count '{}'",
            wrapped_str,
            primitive_str
        );
        assert!(wrapped_str.starts_with("Maximum retries"));
    }

    #[test]
    fn test_rpc_error() {
        // 1. Create primitive values
        let error_code = -32600;
        let error_msg = "Invalid Request";
        // 2. Capture primitives as strings
        let code_str = error_code.to_string();
        // 3. Wrap to custom Error type
        let wrapped_err = Error::RpcError(error_code, error_msg.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive strings are substrings of wrapped string
        assert!(
            wrapped_str.contains(&code_str),
            "Wrapped error '{}' should contain error code '{}'",
            wrapped_str,
            code_str
        );
        assert!(
            wrapped_str.contains(error_msg),
            "Wrapped error '{}' should contain error message '{}'",
            wrapped_str,
            error_msg
        );
        assert!(wrapped_str.starts_with("RPC error"));
    }

    #[test]
    fn test_invalid_rpc_id() {
        // 1. Create primitive value
        let invalid_id = "not-a-valid-id-123";
        // 2. Capture primitive as string (already a string)
        // 3. Wrap to custom Error type
        let wrapped_err = Error::InvalidRpcId(invalid_id.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(invalid_id),
            "Wrapped error '{}' should contain invalid ID '{}'",
            wrapped_str,
            invalid_id
        );
        assert!(wrapped_str.starts_with("Invalid RPC ID: "));
    }

    #[test]
    fn test_invalid_file_type() {
        // 1. Create primitive value
        let file_type = "unknown_format";
        // 2. Capture primitive as string (already a string)
        // 3. Wrap to custom Error type
        let wrapped_err = Error::InvalidFileType(file_type.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(file_type),
            "Wrapped error '{}' should contain file type '{}'",
            wrapped_str,
            file_type
        );
        assert!(wrapped_str.starts_with("Invalid file type: "));
    }

    #[test]
    fn test_invalid_annotation_type() {
        // 1. Create primitive value
        let annotation_type = "unsupported_annotation";
        // 2. Capture primitive as string (already a string)
        // 3. Wrap to custom Error type
        let wrapped_err = Error::InvalidAnnotationType(annotation_type.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(annotation_type),
            "Wrapped error '{}' should contain annotation type '{}'",
            wrapped_str,
            annotation_type
        );
        assert!(wrapped_str.starts_with("Invalid annotation type: "));
    }

    #[test]
    fn test_unsupported_format() {
        // 1. Create primitive value
        let format = "xyz_format";
        // 2. Capture primitive as string (already a string)
        // 3. Wrap to custom Error type
        let wrapped_err = Error::UnsupportedFormat(format.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(format),
            "Wrapped error '{}' should contain format '{}'",
            wrapped_str,
            format
        );
        assert!(wrapped_str.starts_with("Unsupported format: "));
    }

    #[test]
    fn test_missing_images() {
        // 1. Create primitive value
        let details = "image001.jpg, image002.jpg";
        // 2. Capture primitive as string (already a string)
        // 3. Wrap to custom Error type
        let wrapped_err = Error::MissingImages(details.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(details),
            "Wrapped error '{}' should contain details '{}'",
            wrapped_str,
            details
        );
        assert!(wrapped_str.starts_with("Missing images: "));
    }

    #[test]
    fn test_missing_annotations() {
        // 1. Create primitive value
        let details = "annotations.json";
        // 2. Capture primitive as string (already a string)
        // 3. Wrap to custom Error type
        let wrapped_err = Error::MissingAnnotations(details.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(details),
            "Wrapped error '{}' should contain details '{}'",
            wrapped_str,
            details
        );
        assert!(wrapped_str.starts_with("Missing annotations: "));
    }

    #[test]
    fn test_missing_label() {
        // 1. Create primitive value
        let label = "person";
        // 2. Capture primitive as string (already a string)
        // 3. Wrap to custom Error type
        let wrapped_err = Error::MissingLabel(label.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(label),
            "Wrapped error '{}' should contain label '{}'",
            wrapped_str,
            label
        );
        assert!(wrapped_str.starts_with("Missing label: "));
    }

    #[test]
    fn test_invalid_parameters() {
        // 1. Create primitive value
        let params = "batch_size must be positive";
        // 2. Capture primitive as string (already a string)
        // 3. Wrap to custom Error type
        let wrapped_err = Error::InvalidParameters(params.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(params),
            "Wrapped error '{}' should contain params '{}'",
            wrapped_str,
            params
        );
        assert!(wrapped_str.starts_with("Invalid parameters: "));
    }

    #[test]
    fn test_feature_not_enabled() {
        // 1. Create primitive value
        let feature = "polars";
        // 2. Capture primitive as string (already a string)
        // 3. Wrap to custom Error type
        let wrapped_err = Error::FeatureNotEnabled(feature.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(feature),
            "Wrapped error '{}' should contain feature '{}'",
            wrapped_str,
            feature
        );
        assert!(wrapped_str.starts_with("Feature not enabled: "));
    }

    #[test]
    fn test_invalid_etag() {
        // 1. Create primitive value
        let etag = "malformed-etag-value";
        // 2. Capture primitive as string (already a string)
        // 3. Wrap to custom Error type
        let wrapped_err = Error::InvalidEtag(etag.to_string());
        // 4. Capture wrapped error string
        let wrapped_str = wrapped_err.to_string();
        // 5. Verify primitive string is substring of wrapped string
        assert!(
            wrapped_str.contains(etag),
            "Wrapped error '{}' should contain etag '{}'",
            wrapped_str,
            etag
        );
        assert!(wrapped_str.starts_with("Invalid ETag header: "));
    }

    // Tests for simple errors without wrapped content
    // Just verify they can be created and displayed

    #[test]
    fn test_invalid_response() {
        let err = Error::InvalidResponse;
        let err_str = err.to_string();
        assert_eq!(err_str, "Invalid server response");
    }

    #[test]
    fn test_not_implemented() {
        let err = Error::NotImplemented;
        let err_str = err.to_string();
        assert_eq!(err_str, "Not implemented");
    }

    #[test]
    fn test_part_too_large() {
        let err = Error::PartTooLarge;
        let err_str = err.to_string();
        assert_eq!(err_str, "File part size exceeds maximum limit");
    }

    #[test]
    fn test_empty_token() {
        let err = Error::EmptyToken;
        let err_str = err.to_string();
        assert_eq!(err_str, "Authentication token is empty");
    }

    #[test]
    fn test_invalid_token() {
        let err = Error::InvalidToken;
        let err_str = err.to_string();
        assert_eq!(err_str, "Invalid authentication token");
    }

    #[test]
    fn test_token_expired() {
        let err = Error::TokenExpired;
        let err_str = err.to_string();
        assert_eq!(err_str, "Authentication token has expired");
    }

    #[test]
    fn test_unauthorized() {
        let err = Error::Unauthorized;
        let err_str = err.to_string();
        assert_eq!(err_str, "Unauthorized access");
    }
}
