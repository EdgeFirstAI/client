//! UniFFI binding generator for EdgeFirst Client.
//!
//! This binary generates Kotlin and Swift bindings from the UniFFI definitions
//! in the `edgefirst-client-ffi` crate.
//!
//! # Usage
//!
//! Generate Kotlin bindings:
//! ```bash
//! cargo run -p uniffi-bindgen -- generate \
//!     --library target/release/libedgefirst_client_ffi.so \
//!     --language kotlin \
//!     --out-dir bindings/kotlin
//! ```
//!
//! Generate Swift bindings:
//! ```bash
//! cargo run -p uniffi-bindgen -- generate \
//!     --library target/release/libedgefirst_client_ffi.dylib \
//!     --language swift \
//!     --out-dir bindings/swift
//! ```

fn main() {
    uniffi::uniffi_bindgen_main()
}
