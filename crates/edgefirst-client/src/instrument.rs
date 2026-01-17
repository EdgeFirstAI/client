// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Au-Zone Technologies. All Rights Reserved.

//! Conditional instrumentation support for profiling.
//!
//! This module provides tracing integration when the `profiling` feature is enabled.
//! When disabled, instrumentation compiles to zero-cost no-ops.
//!
//! # Features
//!
//! - `profiling` - Base feature enabling tracing spans (no backend)
//! - `tracy` - Tracy real-time profiler backend (includes `profiling`)
//! - `trace-file` - Chrome/Perfetto JSON trace file output (includes `profiling`)
//!
//! # Usage
//!
//! For method-level instrumentation, use the `#[cfg_attr]` pattern:
//!
//! ```rust,ignore
//! #[cfg_attr(feature = "profiling", tracing::instrument(skip(self)))]
//! pub async fn my_method(&self) -> Result<T, Error> {
//!     // ...
//! }
//! ```
//!
//! For manual span creation within functions:
//!
//! ```rust,ignore
//! #[cfg(feature = "profiling")]
//! let _span = tracing::info_span!("operation_name", field = value).entered();
//!
//! // ... code to profile
//!
//! #[cfg(feature = "profiling")]
//! drop(_span);  // Optional: explicitly end span early
//! ```
//!
//! # Trace Output
//!
//! When built with `trace-file` feature, traces can be written to JSON files
//! compatible with [Perfetto UI](https://ui.perfetto.dev/) and convertible to
//! CTF format using [ctf2ctf](https://github.com/KDABLabs/ctf2ctf).

#[cfg(feature = "profiling")]
pub use tracing::{
    debug, debug_span, error, error_span, info, info_span, instrument, trace, trace_span, warn,
    warn_span, Instrument, Level, Span,
};

/// Conditional span creation macro - compiles to no-op when profiling is disabled.
///
/// # Example
///
/// ```rust,ignore
/// use edgefirst_client::span;
///
/// fn process_data() {
///     let _span = span!("process_data", items = 100);
///     // ... processing
/// }
/// ```
#[cfg(feature = "profiling")]
#[macro_export]
macro_rules! span {
    ($name:expr) => {
        tracing::info_span!($name)
    };
    ($name:expr, $($field:tt)*) => {
        tracing::info_span!($name, $($field)*)
    };
}

#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! span {
    ($name:expr) => {
        ()
    };
    ($name:expr, $($field:tt)*) => {
        ()
    };
}

/// Conditional event recording macro - compiles to no-op when profiling is disabled.
///
/// Use this for recording events within spans without creating new spans.
///
/// # Example
///
/// ```rust,ignore
/// use edgefirst_client::trace_event;
///
/// fn download_file(url: &str) {
///     trace_event!("starting download", url = %url);
///     // ... download
///     trace_event!("download complete", bytes = 1024);
/// }
/// ```
#[cfg(feature = "profiling")]
#[macro_export]
macro_rules! trace_event {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*)
    };
}

#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! trace_event {
    ($($arg:tt)*) => {};
}
