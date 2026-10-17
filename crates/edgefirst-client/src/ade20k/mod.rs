// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

//! # ADE20K (SceneParse150) Dataset Format Support
//!
//! Converts the ADEChallengeData2016 release of ADE20K into the EdgeFirst
//! Dataset Format as semantic label maps.
//!
//! ## Source layout
//!
//! ```text
//! ADEChallengeData2016/
//! ├── objectInfo150.txt
//! ├── images/{training,validation}/ADE_*.jpg
//! └── annotations/{training,validation}/ADE_*.png
//! ```
//!
//! Each annotation is an 8-bit grayscale PNG the size of its image. Pixel
//! value `i` in 1..=150 is the class on line `i` of `objectInfo150.txt`;
//! 0 is unlabelled.
//!
//! ## Mapping
//!
//! - One row per image, whose `mask` is the annotation PNG, byte for byte.
//!   `label` and all geometry columns are null.
//! - `training` becomes group `train` and `validation` becomes group `val`.
//! - The file metadata `labels` is `"unlabeled"` followed by the first name
//!   of each class, so a pixel value is its position in `labels`;
//!   `ignore_index` is `"0"` and `mask_interpretation` is `"class_index"`.
//! - `category_metadata` keeps each class's ADE20K id and, when it has more
//!   than one, its names as synonyms.
//!
//! ## Example
//!
//! ```rust,no_run
//! use edgefirst_client::ade20k::{Ade20kToArrowOptions, ade20k_to_arrow};
//!
//! # async fn example() -> Result<(), edgefirst_client::Error> {
//! let root = std::path::Path::new("ADEChallengeData2016");
//! let rows = ade20k_to_arrow(root, "ade20k/ade20k.arrow", &Ade20kToArrowOptions::default(), None).await?;
//! println!("wrote {rows} rows");
//! # Ok(())
//! # }
//! ```

mod reader;

#[cfg(feature = "polars")]
mod arrow;

pub use reader::{
    Ade20kClass, Ade20kPair, SPLITS, list_split_pairs, read_label_map, read_object_info,
};

#[cfg(feature = "polars")]
pub use arrow::{Ade20kToArrowOptions, ade20k_to_arrow};

#[cfg(all(test, feature = "polars"))]
mod tests;
