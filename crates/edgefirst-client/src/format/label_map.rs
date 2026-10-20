// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

//! Validation of file metadata and semantic label-map rows.

use std::path::Path;

use polars::prelude::*;

use super::{DatasetMetadata, MaskInterpretation, ValidationIssue, metadata::ERROR_PREFIX};
use crate::{Error, MaskData};

/// Checks a dataset file's metadata and, when its masks are class indices,
/// every semantic label-map row: a row with a `mask` and no label, box or
/// polygon.
pub(super) fn validate_contents(path: &Path) -> Result<Vec<ValidationIssue>, Error> {
    let metadata = match DatasetMetadata::from_map(&super::read_dataset_metadata(path)?) {
        Ok(metadata) => metadata,
        Err(Error::InvalidParameters(message)) => {
            let message = message
                .strip_prefix(ERROR_PREFIX)
                .map(str::to_owned)
                .unwrap_or(message);
            return Ok(vec![ValidationIssue::InvalidMetadata { message }]);
        }
        Err(e) => return Err(e),
    };
    if metadata.mask_interpretation != MaskInterpretation::ClassIndex {
        return Ok(Vec::new());
    }
    // Rows are decoded only for class-index files, whose label maps are checked.
    let (df, _) = super::read_dataset_dataframe(path)?;
    // Legacy files store `mask` as a polygon list; only PNG bytes are masks.
    let Some(masks) = df.column("mask").ok().and_then(|c| c.binary().ok()) else {
        return Ok(Vec::new());
    };
    let classes = metadata.labels.map_or(0, |labels| labels.len());
    let names = df.column("name")?.cast(&DataType::String)?;
    let names = names.str()?;
    let unlabelled = missing_or_empty(&df, "label");
    let no_geometry: Vec<bool> = {
        let [b2, b3, poly] = ["box2d", "box3d", "polygon"].map(|c| missing(&df, c));
        (0..df.height())
            .map(|i| b2[i] && b3[i] && poly[i])
            .collect()
    };
    let mut issues = Vec::new();
    for (i, bytes) in masks.iter().enumerate() {
        let Some(bytes) = bytes else { continue };
        if !(unlabelled[i] && no_geometry[i]) {
            continue;
        }
        if let Err(reason) = check_label_map(bytes, classes) {
            issues.push(ValidationIssue::InvalidLabelMap {
                name: names.get(i).unwrap_or_default().to_owned(),
                reason,
            });
        }
    }
    Ok(issues)
}

/// Checks one label map: an 8- or 16-bit grayscale PNG without `oFFs`, with
/// every pixel a position in `labels`. Its size is not checked: a label map
/// may be at the model's output resolution, and readers resize it.
fn check_label_map(bytes: &[u8], classes: usize) -> Result<(), String> {
    let mask = MaskData::from_png_checked(bytes.to_vec()).map_err(|e| message(&e))?;
    let depth = mask.bit_depth();
    if depth != 8 && depth != 16 {
        return Err(format!(
            "expected an 8- or 16-bit grayscale PNG, got {depth}-bit"
        ));
    }
    if mask.has_offs_chunk() {
        return Err("has an oFFs chunk; label maps cover the whole image".into());
    }
    let pixels = mask.decode().map_err(|e| message(&e))?;
    let max = if depth == 8 {
        pixels.iter().copied().map(usize::from).max()
    } else {
        pixels
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&pair| usize::from(u16::from_be_bytes(pair)))
            .max()
    };
    match max {
        Some(max) if max >= classes => Err(format!(
            "pixel value {max} is outside labels ({classes} entries)"
        )),
        _ => Ok(()),
    }
}

fn message(err: &Error) -> String {
    match err {
        Error::InvalidParameters(m) => m.clone(),
        other => other.to_string(),
    }
}

/// Per row: whether the column is absent or the value null.
fn missing(df: &DataFrame, column: &str) -> Vec<bool> {
    df.column(column)
        .map(|c| c.is_null().iter().map(|v| v.unwrap_or(true)).collect())
        .unwrap_or_else(|_| vec![true; df.height()])
}

/// Per row: whether the string column is absent, null or empty.
fn missing_or_empty(df: &DataFrame, column: &str) -> Vec<bool> {
    let Some(values) = df
        .column(column)
        .ok()
        .and_then(|c| c.cast(&DataType::String).ok())
    else {
        return vec![true; df.height()];
    };
    match values.str() {
        Ok(values) => values.iter().map(|v| v.is_none_or(str::is_empty)).collect(),
        Err(_) => missing(df, column),
    }
}

#[cfg(test)]
#[path = "label_map_tests.rs"]
mod tests;
