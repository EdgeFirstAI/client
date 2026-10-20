// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

use std::collections::BTreeMap;

use super::{DatasetMetadata, MaskInterpretation};

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn label_map_keys() -> BTreeMap<String, String> {
    map(&[
        ("schema_version", "2026.10"),
        ("labels", r#"["unlabeled","wall","building"]"#),
        ("ignore_index", "0"),
        ("mask_interpretation", "class_index"),
        ("category_metadata", r#"{"wall":{"id":1}}"#),
        ("source", "ADE20K"),
    ])
}

fn rejection(pairs: &[(&str, &str)]) -> String {
    DatasetMetadata::from_map(&map(pairs))
        .unwrap_err()
        .to_string()
}

#[test]
fn absent_keys_take_defaults() {
    let meta = DatasetMetadata::from_map(&BTreeMap::new()).unwrap();
    assert_eq!(meta, DatasetMetadata::default());
    assert_eq!(meta.mask_interpretation, MaskInterpretation::Binary);
    assert!(meta.to_map().is_empty());
}

#[test]
fn parses_label_map_keys() {
    let meta = DatasetMetadata::from_map(&label_map_keys()).unwrap();
    assert_eq!(meta.schema_version.as_deref(), Some("2026.10"));
    assert_eq!(
        meta.labels.as_deref(),
        Some(&["unlabeled".to_string(), "wall".into(), "building".into()][..])
    );
    assert_eq!(meta.ignore_index, Some(0));
    assert_eq!(meta.mask_interpretation, MaskInterpretation::ClassIndex);
    assert_eq!(
        meta.category_metadata.as_ref().unwrap()["wall"]["id"],
        serde_json::json!(1)
    );
    assert_eq!(meta.extra, map(&[("source", "ADE20K")]));
}

#[test]
fn round_trips_through_map() {
    let source = label_map_keys();
    let meta = DatasetMetadata::from_map(&source).unwrap();
    assert_eq!(meta.to_map(), source);
    assert_eq!(DatasetMetadata::from_map(&meta.to_map()).unwrap(), meta);
}

#[test]
fn explicit_binary_is_omitted_on_write() {
    let meta = DatasetMetadata::from_map(&map(&[("mask_interpretation", "binary")])).unwrap();
    assert_eq!(meta.mask_interpretation, MaskInterpretation::Binary);
    assert!(!meta.to_map().contains_key("mask_interpretation"));
}

#[test]
fn every_interpretation_round_trips_as_text() {
    for value in ["binary", "class_index", "confidence", "sigmoid", "logits"] {
        let parsed: MaskInterpretation = value.parse().unwrap();
        assert_eq!(parsed.as_str(), value);
        assert_eq!(parsed.to_string(), value);
    }
}

#[test]
fn rejects_labels_that_are_not_a_string_array() {
    for bad in ["wall", "[1, 2]", r#"{"a": 1}"#] {
        let err = rejection(&[("labels", bad)]);
        assert!(err.contains("labels"), "{err}");
    }
}

#[test]
fn rejects_ignore_index_that_is_not_an_integer() {
    for bad in ["-1", "zero", "1.5", ""] {
        let err = rejection(&[("labels", r#"["a","b"]"#), ("ignore_index", bad)]);
        assert!(err.contains("ignore_index"), "{err}");
        assert!(err.contains(bad), "{err}");
    }
}

#[test]
fn rejects_ignore_index_without_labels() {
    let err = rejection(&[("ignore_index", "0")]);
    assert!(
        err.contains("ignore_index") && err.contains("labels"),
        "{err}"
    );
}

#[test]
fn rejects_ignore_index_outside_labels() {
    let err = rejection(&[("labels", r#"["a","b"]"#), ("ignore_index", "2")]);
    assert!(err.contains("ignore_index") && err.contains('2'), "{err}");
}

#[test]
fn rejects_unknown_mask_interpretation() {
    let err = rejection(&[("mask_interpretation", "probability")]);
    assert!(
        err.contains("mask_interpretation") && err.contains("probability"),
        "{err}"
    );
}

#[test]
fn rejects_class_index_without_labels() {
    let err = rejection(&[("mask_interpretation", "class_index")]);
    assert!(
        err.contains("class_index") && err.contains("labels"),
        "{err}"
    );
}

#[test]
fn rejects_category_metadata_that_is_not_an_object() {
    let err = rejection(&[("category_metadata", "[1]")]);
    assert!(err.contains("category_metadata"), "{err}");
}

#[cfg(feature = "polars")]
#[test]
fn reads_arrow_and_parquet_files() {
    use polars::prelude::*;
    use std::sync::Arc;

    let dir = tempfile::TempDir::new().unwrap();
    let source = label_map_keys();
    let frame =
        || DataFrame::new_infer_height(vec![Series::new("name".into(), &["a"]).into()]).unwrap();

    let arrow = dir.path().join("meta.arrow");
    let mut writer = IpcWriter::new(std::fs::File::create(&arrow).unwrap());
    writer.set_custom_schema_metadata(Arc::new(
        source
            .iter()
            .map(|(k, v)| (PlSmallStr::from(k.as_str()), PlSmallStr::from(v.as_str())))
            .collect(),
    ));
    writer.finish(&mut frame()).unwrap();

    let parquet = dir.path().join("meta.parquet");
    ParquetWriter::new(std::fs::File::create(&parquet).unwrap())
        .with_key_value_metadata(Some(KeyValueMetadata::from_static(
            source.clone().into_iter().collect(),
        )))
        .finish(&mut frame())
        .unwrap();

    let expected = DatasetMetadata::from_map(&source).unwrap();
    assert_eq!(DatasetMetadata::read(&arrow).unwrap(), expected);
    let from_parquet = DatasetMetadata::read(&parquet).unwrap();
    assert_eq!(from_parquet.labels, expected.labels);
    assert_eq!(from_parquet.ignore_index, Some(0));
    assert_eq!(
        from_parquet.mask_interpretation,
        MaskInterpretation::ClassIndex
    );
}

#[cfg(feature = "polars")]
#[test]
fn read_reports_invalid_metadata_with_the_path() {
    use polars::prelude::*;
    use std::sync::Arc;

    let dir = tempfile::TempDir::new().unwrap();
    let arrow = dir.path().join("bad.arrow");
    let mut writer = IpcWriter::new(std::fs::File::create(&arrow).unwrap());
    writer.set_custom_schema_metadata(Arc::new(
        [(
            PlSmallStr::from("mask_interpretation"),
            PlSmallStr::from("probability"),
        )]
        .into_iter()
        .collect(),
    ));
    let mut df =
        DataFrame::new_infer_height(vec![Series::new("name".into(), &["a"]).into()]).unwrap();
    writer.finish(&mut df).unwrap();

    let err = DatasetMetadata::read(&arrow).unwrap_err().to_string();
    assert!(
        err.contains("bad.arrow") && err.contains("probability"),
        "{err}"
    );
}

#[cfg(feature = "polars")]
#[test]
fn read_rejects_an_unreadable_arrow_file() {
    let dir = tempfile::TempDir::new().unwrap();
    for (name, bytes) in [
        ("empty.arrow", &b""[..]),
        ("garbage.arrow", b"not an arrow file"),
    ] {
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        let err = DatasetMetadata::read(&path).unwrap_err().to_string();
        assert!(err.contains(name), "{err}");
        assert!(
            super::super::read_dataset_metadata(&path).is_err(),
            "{name}"
        );
    }
}
