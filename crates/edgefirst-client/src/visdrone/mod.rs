// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

//! # VisDrone2019 Dataset Format Support
//!
//! Converts the VisDrone2019-DET (still images) and VisDrone2019-VID (video
//! sequences) annotation layouts into the EdgeFirst Dataset Format.
//!
//! ## Source layout
//!
//! ```text
//! VisDrone2019-DET-val/            VisDrone2019-VID-val/
//! ├── annotations/<name>.txt       ├── annotations/<seq>.txt
//! └── images/<name>.jpg            └── sequences/<seq>/0000001.jpg
//! ```
//!
//! DET lines: `left,top,width,height,score,category,truncation,occlusion`.
//! VID lines prepend `frame_index,target_id`. All values are pixel integers.
//!
//! ## Mapping
//!
//! - `category` 1..=10 becomes `label` (see [`CLASS_CATEGORIES`]) with
//!   `label_index = category - 1`, matching the Ultralytics VisDrone mapping.
//! - Categories 0 (`ignored regions`) and 11 (`others`) are dropped by
//!   default. With `keep_ignored` they are kept without a label, flagged
//!   `ignore` and `exclude` respectively.
//! - The file metadata `labels` lists [`CLASS_CATEGORIES`].
//! - An image or frame whose rows are all dropped is still emitted as a
//!   sample with no annotations.
//! - `score` is not stored: it is 0 exactly when the category is 0 or 11.
//! - `truncation` and `occlusion` become the `truncation` and `occlusion`
//!   columns.
//! - VID sequences become `name` = sequence, `frame` = frame index, and
//!   `object_id` = `<seq>/<target_id>`.
//!
//! ## Example
//!
//! ```rust,no_run
//! use edgefirst_client::visdrone::{VisDroneToArrowOptions, visdrone_to_arrow};
//!
//! # async fn example() -> Result<(), edgefirst_client::Error> {
//! let inputs = [std::path::PathBuf::from("VisDrone2019-DET-val")];
//! let rows = visdrone_to_arrow(&inputs, "visdrone/visdrone.arrow", &VisDroneToArrowOptions::default(), None).await?;
//! println!("wrote {rows} rows");
//! # Ok(())
//! # }
//! ```

mod reader;

#[cfg(feature = "polars")]
mod arrow;

pub use reader::{
    CATEGORIES, CLASS_CATEGORIES, SplitKind, VisDroneBox, VisDroneVidRow, category_name,
    detect_split_kind, infer_group_from_dir_name, parse_det_line, parse_vid_line,
    read_det_annotations, read_vid_annotations,
};

#[cfg(feature = "polars")]
pub use arrow::{VisDroneToArrowOptions, visdrone_to_arrow};

#[cfg(all(test, feature = "polars"))]
mod tests;
