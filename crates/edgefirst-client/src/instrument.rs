// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Au-Zone Technologies. All Rights Reserved.

//! Conditional instrumentation support for Tracy profiling.
//!
//! This module provides tracing integration when the `tracy` feature is enabled.
//! When disabled, instrumentation compiles to zero-cost no-ops.
//!
//! # Usage
//!
//! For method-level instrumentation, use the `#[cfg_attr]` pattern:
//!
//! ```rust,ignore
//! #[cfg_attr(feature = "tracy", tracing::instrument(skip(self)))]
//! pub async fn my_method(&self) -> Result<T, Error> {
//!     // ...
//! }
//! ```
//!
//! For manual span creation within functions, use the macros from this module:
//!
//! ```rust,ignore
//! use crate::instrument::*;
//!
//! fn my_function() {
//!     let _span = info_span!("operation_name", field = value);
//!     // span ends when dropped
//! }
//! ```

#[cfg(feature = "tracy")]
pub use tracing::{
    debug, debug_span, error, error_span, info, info_span, instrument, trace, trace_span, warn,
    warn_span, Instrument, Level, Span,
};
