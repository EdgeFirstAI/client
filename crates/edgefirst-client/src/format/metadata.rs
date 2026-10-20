// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

//! Typed, validated file-level metadata of an EdgeFirst dataset file.

use std::{collections::BTreeMap, fmt, str::FromStr};

use crate::Error;

const SCHEMA_VERSION: &str = "schema_version";
const LABELS: &str = "labels";
const IGNORE_INDEX: &str = "ignore_index";
const MASK_INTERPRETATION: &str = "mask_interpretation";
const CATEGORY_METADATA: &str = "category_metadata";

/// Meaning of the pixel values of the raster masks in a dataset file, from
/// the `mask_interpretation` metadata key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MaskInterpretation {
    /// 0/1 instance masks. The default when the key is absent.
    #[default]
    Binary,
    /// Argmax class indices: each pixel value is a position in `labels`.
    ClassIndex,
    /// Quantized confidence scores.
    Confidence,
    /// Quantized sigmoid outputs.
    Sigmoid,
    /// Quantized logits.
    Logits,
}

impl MaskInterpretation {
    /// The metadata value, e.g. `"class_index"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Binary => "binary",
            Self::ClassIndex => "class_index",
            Self::Confidence => "confidence",
            Self::Sigmoid => "sigmoid",
            Self::Logits => "logits",
        }
    }
}

impl fmt::Display for MaskInterpretation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MaskInterpretation {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Ok(match s {
            "binary" => Self::Binary,
            "class_index" => Self::ClassIndex,
            "confidence" => Self::Confidence,
            "sigmoid" => Self::Sigmoid,
            "logits" => Self::Logits,
            other => {
                return Err(invalid(format!(
                    "{MASK_INTERPRETATION} '{other}' is not one of binary, class_index, confidence, sigmoid, logits"
                )));
            }
        })
    }
}

/// File-level metadata of an EdgeFirst dataset file, parsed and validated.
///
/// Keys this type does not interpret are kept unchanged in `extra`, so
/// `from_map` followed by `to_map` loses nothing except a redundant
/// `mask_interpretation = "binary"`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DatasetMetadata {
    /// `schema_version`, e.g. `"2026.10"`. Absent in legacy files.
    pub schema_version: Option<String>,
    /// Ordered class names; for class-index masks, pixel value `i` is
    /// `labels[i]`.
    pub labels: Option<Vec<String>>,
    /// Position in `labels` whose pixels are don't-care.
    pub ignore_index: Option<usize>,
    /// Meaning of raster mask pixel values.
    pub mask_interpretation: MaskInterpretation,
    /// Per-label reference data keyed by label name.
    pub category_metadata: Option<serde_json::Map<String, serde_json::Value>>,
    /// Every other key, unchanged.
    pub extra: BTreeMap<String, String>,
}

impl DatasetMetadata {
    /// Parses and validates a raw metadata map.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidParameters`] naming the key and value when
    /// `labels` is not a JSON array of strings, `ignore_index` is not a
    /// position within `labels`, `mask_interpretation` is unknown or is
    /// `class_index` without `labels`, or `category_metadata` is not a JSON
    /// object.
    pub fn from_map(map: &BTreeMap<String, String>) -> Result<Self, Error> {
        let mut extra = map.clone();
        let schema_version = extra.remove(SCHEMA_VERSION);

        let labels = extra
            .remove(LABELS)
            .map(|raw| {
                serde_json::from_str::<Vec<String>>(&raw).map_err(|_| {
                    invalid(format!(
                        "{LABELS} must be a JSON array of strings, got {raw}"
                    ))
                })
            })
            .transpose()?;

        let ignore_index = extra
            .remove(IGNORE_INDEX)
            .map(|raw| parse_ignore_index(&raw, labels.as_deref()))
            .transpose()?;

        let mask_interpretation = extra
            .remove(MASK_INTERPRETATION)
            .map(|raw| raw.parse())
            .transpose()?
            .unwrap_or_default();
        if mask_interpretation == MaskInterpretation::ClassIndex && labels.is_none() {
            return Err(invalid(format!(
                "{MASK_INTERPRETATION} class_index needs {LABELS} to name the class of each pixel value"
            )));
        }

        let category_metadata = extra
            .remove(CATEGORY_METADATA)
            .map(|raw| match serde_json::from_str(&raw) {
                Ok(serde_json::Value::Object(object)) => Ok(object),
                _ => Err(invalid(format!(
                    "{CATEGORY_METADATA} must be a JSON object, got {raw}"
                ))),
            })
            .transpose()?;

        Ok(Self {
            schema_version,
            labels,
            ignore_index,
            mask_interpretation,
            category_metadata,
            extra,
        })
    }

    /// Reads and validates the metadata of a `.arrow`, `.ipc` or `.parquet`
    /// dataset file without decoding its rows.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, or the
    /// [`from_map`](Self::from_map) error prefixed with the file path.
    #[cfg(feature = "polars")]
    pub fn read(path: &std::path::Path) -> Result<Self, Error> {
        Self::from_file_map(path, &super::read_dataset_metadata(path)?)
    }

    /// [`from_map`](Self::from_map) for metadata already read from `path`,
    /// with the path in the error message.
    pub(crate) fn from_file_map(
        path: &std::path::Path,
        map: &BTreeMap<String, String>,
    ) -> Result<Self, Error> {
        Self::from_map(map).map_err(|e| match e {
            Error::InvalidParameters(m) => {
                Error::InvalidParameters(format!("{}: {m}", path.display()))
            }
            other => other,
        })
    }

    /// Serialises back to a raw metadata map. `mask_interpretation` is
    /// written only when it is not the default `binary`.
    pub fn to_map(&self) -> BTreeMap<String, String> {
        let mut map = self.extra.clone();
        if let Some(version) = &self.schema_version {
            map.insert(SCHEMA_VERSION.into(), version.clone());
        }
        if let Some(labels) = &self.labels {
            map.insert(LABELS.into(), json(labels));
        }
        if let Some(index) = self.ignore_index {
            map.insert(IGNORE_INDEX.into(), index.to_string());
        }
        if self.mask_interpretation != MaskInterpretation::Binary {
            map.insert(
                MASK_INTERPRETATION.into(),
                self.mask_interpretation.as_str().into(),
            );
        }
        if let Some(categories) = &self.category_metadata {
            map.insert(CATEGORY_METADATA.into(), json(categories));
        }
        map
    }
}

fn parse_ignore_index(raw: &str, labels: Option<&[String]>) -> Result<usize, Error> {
    let index = raw
        .bytes()
        .all(|b| b.is_ascii_digit())
        .then(|| raw.parse::<usize>().ok())
        .flatten()
        .ok_or_else(|| {
            invalid(format!(
                "{IGNORE_INDEX} must be a non-negative integer, got '{raw}'"
            ))
        })?;
    let labels = labels.ok_or_else(|| {
        invalid(format!(
            "{IGNORE_INDEX} {index} is a position in {LABELS}, but the file has no {LABELS}"
        ))
    })?;
    if index >= labels.len() {
        return Err(invalid(format!(
            "{IGNORE_INDEX} {index} is outside {LABELS}, which has {} entries",
            labels.len()
        )));
    }
    Ok(index)
}

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("labels and category metadata serialise to JSON")
}

/// Start of every [`DatasetMetadata::from_map`] error message.
pub(super) const ERROR_PREFIX: &str = "Invalid dataset metadata: ";

fn invalid(message: String) -> Error {
    Error::InvalidParameters(format!("{ERROR_PREFIX}{message}"))
}

#[cfg(test)]
#[path = "metadata_tests.rs"]
mod tests;
