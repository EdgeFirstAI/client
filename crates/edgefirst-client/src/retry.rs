// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Retry policies with URL-based classification for EdgeFirst Studio Client.
//!
//! # Overview
//!
//! This module implements intelligent retry logic that classifies requests into
//! two categories:
//!
//! - **StudioApi**: EdgeFirst Studio JSON-RPC API calls
//!   (`*.edgefirst.studio/api`)
//! - **FileIO**: File upload/download operations (AWS S3 pre-signed URLs,
//!   CloudFront, etc.)
//!
//! # Motivation
//!
//! Different types of operations have different failure characteristics and
//! retry requirements:
//!
//! ## Studio API Requests
//!
//! - **Low concurrency**: Sequential JSON-RPC method calls
//! - **Fast-fail desired**: Authentication failures should not retry
//! - **Predictable errors**: HTTP 401/403 indicate auth issues, not transient
//!   failures
//! - **User experience**: Users expect quick feedback on invalid credentials
//!
//! ## File I/O Operations (S3, CloudFront)
//!
//! - **High concurrency**: Parallel uploads/downloads of dataset files (100+
//!   files)
//! - **Transient failures common**: S3 rate limiting, network congestion,
//!   timeouts
//! - **Retry-safe**: Idempotent operations (pre-signed URLs, multipart uploads)
//! - **Robustness critical**: Dataset operations must complete reliably despite
//!   temporary issues
//!
//! # Classification Strategy
//!
//! URLs are classified by inspecting the host and path:
//!
//! - **StudioApi**: `https://*.edgefirst.studio/api*` (exact host match + path
//!   prefix)
//! - **FileIO**: Everything else (S3, CloudFront, or any non-API Studio path)
//!
//! # Retry Behavior
//!
//! Both scopes use the same configurable retry count (`EDGEFIRST_MAX_RETRIES`,
//! default: 3), but differ in error classification:
//!
//! ## StudioApi Error Classification
//!
//! - **Never retry**: 401 Unauthorized, 403 Forbidden (auth failures)
//! - **Always retry**: 408 Timeout, 429 Too Many Requests, 5xx Server Errors
//! - **Retry transports errors**: Connection failures, DNS errors, timeouts
//!
//! ## FileIO Error Classification
//!
//! - **Always retry**: 408 Timeout, 409 Conflict, 423 Locked, 429 Too Many
//!   Requests, 5xx Server Errors
//! - **Retry transport errors**: Connection failures, DNS errors, timeouts
//! - **No auth bypass**: All HTTP errors (including 401/403) are retried for S3
//!   URLs
//!
//! # Configuration
//!
//! - `EDGEFIRST_MAX_RETRIES`: Maximum retry attempts per request (default: 3)
//! - `EDGEFIRST_TIMEOUT`: Request timeout in seconds (default: 30)
//!
//! **For bulk file operations**, increase retry count for better resilience:
//! ```bash
//! export EDGEFIRST_MAX_RETRIES=10  # More retries for S3 operations
//! export EDGEFIRST_TIMEOUT=60      # Longer timeout for large files
//! ```
//!
//! # Examples
//!
//! ```rust
//! use edgefirst_client::{RetryScope, classify_url};
//!
//! // Studio API calls
//! assert_eq!(
//!     classify_url("https://edgefirst.studio/api"),
//!     RetryScope::StudioApi
//! );
//! assert_eq!(
//!     classify_url("https://test.edgefirst.studio/api/datasets.list"),
//!     RetryScope::StudioApi
//! );
//!
//! // File I/O operations
//! assert_eq!(
//!     classify_url("https://s3.amazonaws.com/bucket/file.bin"),
//!     RetryScope::FileIO
//! );
//! assert_eq!(
//!     classify_url("https://d123abc.cloudfront.net/dataset.zip"),
//!     RetryScope::FileIO
//! );
//! ```

use url::Url;

/// Retry scope classification for URL-based retry policies.
///
/// Determines whether a request is a Studio API call or a File I/O operation,
/// enabling different error handling strategies for each category.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetryScope {
    /// EdgeFirst Studio JSON-RPC API calls to `*.edgefirst.studio/api`.
    ///
    /// These calls should fail fast on authentication errors but retry
    /// server errors and transient failures.
    StudioApi,

    /// File upload/download operations to S3, CloudFront, or other endpoints.
    ///
    /// These operations experience high concurrency and should retry
    /// aggressively on all transient failures.
    FileIO,
}

/// Classifies a URL to determine which retry policy to apply.
///
/// This function performs URL-based classification to differentiate between
/// EdgeFirst Studio API calls and File I/O operations (S3, CloudFront, etc.).
///
/// # Classification Algorithm
///
/// 1. Parse URL using proper URL parser (handles ports, query params,
///    fragments)
/// 2. Check protocol: Only HTTP/HTTPS are classified as StudioApi (all others →
///    FileIO)
/// 3. Check host: Must be `edgefirst.studio` or `*.edgefirst.studio`
/// 4. Check path: Must start with `/api` (exact match or `/api/...`)
/// 5. If all conditions met → `StudioApi`, otherwise → `FileIO`
///
/// # Edge Cases Handled
///
/// - **Port numbers**: `https://test.edgefirst.studio:8080/api` → StudioApi
/// - **Trailing slashes**: `https://edgefirst.studio/api/` → StudioApi
/// - **Query parameters**: `https://edgefirst.studio/api?foo=bar` → StudioApi
/// - **Subdomains**: `https://ocean.edgefirst.studio/api` → StudioApi
/// - **Similar domains**: `https://edgefirst.studio.com/api` → FileIO (not
///   exact match)
/// - **Path injection**: `https://evil.com/edgefirst.studio/api` → FileIO (host
///   mismatch)
/// - **Non-API paths**: `https://edgefirst.studio/download` → FileIO
///
/// # Security
///
/// The function uses proper URL parsing to prevent domain spoofing attacks.
/// Only the URL host is checked, not the path, preventing injection via
/// `https://attacker.com/edgefirst.studio/api`.
///
/// # Examples
///
/// ```rust
/// use edgefirst_client::{RetryScope, classify_url};
///
/// // Studio API URLs
/// assert_eq!(
///     classify_url("https://edgefirst.studio/api"),
///     RetryScope::StudioApi
/// );
/// assert_eq!(
///     classify_url("https://test.edgefirst.studio/api/datasets"),
///     RetryScope::StudioApi
/// );
/// assert_eq!(
///     classify_url("https://test.edgefirst.studio:443/api?token=abc"),
///     RetryScope::StudioApi
/// );
///
/// // File I/O URLs (S3, CloudFront, etc.)
/// assert_eq!(
///     classify_url("https://s3.amazonaws.com/bucket/file.bin"),
///     RetryScope::FileIO
/// );
/// assert_eq!(
///     classify_url("https://d123abc.cloudfront.net/dataset.zip"),
///     RetryScope::FileIO
/// );
/// assert_eq!(
///     classify_url("https://edgefirst.studio/download_model"),
///     RetryScope::FileIO // Non-API path
/// );
/// ```
pub fn classify_url(url: &str) -> RetryScope {
    // Try to parse as proper URL
    if let Ok(parsed) = Url::parse(url) {
        // Only match HTTP/HTTPS protocols
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return RetryScope::FileIO;
        }

        if let Some(host) = parsed.host_str() {
            let host_matches = host == "edgefirst.studio" || host.ends_with(".edgefirst.studio");

            // Path must be exactly "/api" or start with "/api/" (not "/apis" etc.)
            let path = parsed.path();
            let path_is_api = path == "/api" || path.starts_with("/api/");

            if host_matches && path_is_api {
                return RetryScope::StudioApi;
            }
        }
    }

    RetryScope::FileIO
}

/// Creates a retry policy with URL-based classification.
///
/// This function builds a reqwest retry policy that inspects each request URL
/// and applies different error classification rules based on whether it's a
/// Studio API call or a File I/O operation.
///
/// # Retry Configuration
///
/// - **Max retries**: Configurable via `EDGEFIRST_MAX_RETRIES` (default: 3)
/// - **Timeout**: Configurable via `EDGEFIRST_TIMEOUT` (default: 30 seconds)
///
/// # Error Classification by Scope
///
/// ## StudioApi (*.edgefirst.studio/api)
///
/// Optimized for fast-fail on authentication errors:
///
/// | HTTP Status | Action | Rationale |
/// |-------------|--------|-----------|
/// | 401, 403 | Never retry | Authentication failure - user action required |
/// | 408, 429 | Retry | Timeout, rate limiting - transient |
/// | 5xx | Retry | Server error - may recover |
/// | Connection errors | Retry | Network issues - transient |
///
/// ## FileIO (S3, CloudFront, etc.)
///
/// Optimized for robustness under high concurrency:
///
/// | HTTP Status | Action | Rationale |
/// |-------------|--------|-----------|
/// | 408, 429 | Retry | Timeout, rate limiting - common with S3 |
/// | 409, 423 | Retry | Conflict, locked - S3 eventual consistency |
/// | 5xx | Retry | Server error - S3 transient issues |
/// | Connection errors | Retry | Network issues - common in parallel uploads |
///
/// # Usage Recommendations
///
/// **For dataset downloads/uploads** (many concurrent S3 operations):
/// ```bash
/// export EDGEFIRST_MAX_RETRIES=10  # More retries for robustness
/// export EDGEFIRST_TIMEOUT=60      # Longer timeout for large files
/// ```
///
/// **For testing** (fast failure detection):
/// ```bash
/// export EDGEFIRST_MAX_RETRIES=1   # Minimal retries
/// export EDGEFIRST_TIMEOUT=10      # Quick timeout
/// ```
///
/// # Implementation Notes
///
/// Due to reqwest retry API limitations, both StudioApi and FileIO use the
/// same `max_retries_per_request` value. The differentiation is in error
/// classification only (which errors trigger retries), not retry count.
///
/// For operations requiring different retry counts, use separate Client
/// instances with different `EDGEFIRST_MAX_RETRIES` configuration.
pub fn create_retry_policy() -> reqwest::retry::Builder {
    let max_retries = std::env::var("EDGEFIRST_MAX_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3); // Reduced from 5 to 3 for faster failures

    // Use wildcard host scope since we do URL inspection in classify_fn
    reqwest::retry::for_host("*")
        .max_retries_per_request(max_retries)
        .classify_fn(|req_rep| {
            let url = req_rep.uri().to_string();

            match classify_url(&url) {
                RetryScope::StudioApi => {
                    // Studio API: Never retry auth failures, retry server errors
                    match req_rep.status() {
                        Some(status) => match status.as_u16() {
                            401 | 403 => req_rep.success(), // Auth failures - don't retry
                            429 | 408 | 500..=599 => req_rep.retryable(),
                            _ => req_rep.success(),
                        },
                        // No status code means connection error, timeout, or other transport
                        // failure These are safe to retry for API calls
                        None if req_rep.error().is_some() => req_rep.retryable(),
                        None => req_rep.success(),
                    }
                }
                RetryScope::FileIO => {
                    // File I/O: Retry all transient errors
                    match req_rep.status() {
                        Some(status) => match status.as_u16() {
                            429 | 408 | 500..=599 | 409 | 423 => req_rep.retryable(),
                            _ => req_rep.success(),
                        },
                        None if req_rep.error().is_some() => req_rep.retryable(),
                        None => req_rep.success(),
                    }
                }
            }
        })
}

pub fn log_retry_configuration() {
    let max_retries = std::env::var("EDGEFIRST_MAX_RETRIES").unwrap_or_else(|_| "3".to_string());
    let timeout = std::env::var("EDGEFIRST_TIMEOUT").unwrap_or_else(|_| "30".to_string());
    log::debug!(
        "Retry configuration - max_retries={}, timeout={}s",
        max_retries,
        timeout
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_url_studio_api() {
        // Base production URL
        assert_eq!(
            classify_url("https://edgefirst.studio/api"),
            RetryScope::StudioApi
        );

        // Server-specific instances
        assert_eq!(
            classify_url("https://test.edgefirst.studio/api"),
            RetryScope::StudioApi
        );
        assert_eq!(
            classify_url("https://stage.edgefirst.studio/api"),
            RetryScope::StudioApi
        );
        assert_eq!(
            classify_url("https://saas.edgefirst.studio/api"),
            RetryScope::StudioApi
        );
        assert_eq!(
            classify_url("https://ocean.edgefirst.studio/api"),
            RetryScope::StudioApi
        );

        // API endpoints with paths
        assert_eq!(
            classify_url("https://test.edgefirst.studio/api/datasets"),
            RetryScope::StudioApi
        );
        assert_eq!(
            classify_url("https://stage.edgefirst.studio/api/auth.login"),
            RetryScope::StudioApi
        );
    }

    #[test]
    fn test_classify_url_file_io() {
        // S3 URLs for file operations
        assert_eq!(
            classify_url("https://s3.amazonaws.com/bucket/file.bin"),
            RetryScope::FileIO
        );

        // CloudFront URLs for file distribution
        assert_eq!(
            classify_url("https://d123abc.cloudfront.net/file.bin"),
            RetryScope::FileIO
        );

        // Non-API paths on edgefirst.studio domain
        assert_eq!(
            classify_url("https://edgefirst.studio/docs"),
            RetryScope::FileIO
        );
        assert_eq!(
            classify_url("https://test.edgefirst.studio/download_model"),
            RetryScope::FileIO
        );
        assert_eq!(
            classify_url("https://stage.edgefirst.studio/download_checkpoint"),
            RetryScope::FileIO
        );

        // Generic download URLs
        assert_eq!(
            classify_url("https://example.com/download"),
            RetryScope::FileIO
        );
    }
}
