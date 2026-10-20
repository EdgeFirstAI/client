// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

use std::path::{Path, PathBuf};

use crate::{
    Annotation, MaskData, Sample,
    format::{ValidationIssue, validate_dataset_structure},
};

const TWO_CLASSES: [(&str, &str); 2] = [
    ("labels", r#"["unlabeled","wall"]"#),
    ("mask_interpretation", "class_index"),
];

fn label_map_sample(name: &str, mask: MaskData, size: (u32, u32)) -> Sample {
    let mut row = Annotation::new();
    row.set_name(Some(name.into()));
    row.set_mask(Some(mask));
    Sample {
        image_name: Some(format!("{name}.jpg")),
        width: Some(size.0),
        height: Some(size.1),
        annotations: vec![row],
        ..Default::default()
    }
}

/// Writes `samples` as dataset `ds` under `root` with the given metadata.
fn write_snapshot(root: &Path, samples: &[Sample], meta: &[(&str, &str)]) -> PathBuf {
    let dir = root.join("ds");
    std::fs::create_dir_all(dir.join("ds")).unwrap();
    let mut df = crate::samples_dataframe(samples).unwrap();
    let meta = meta
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect();
    crate::coco::write_dataset(&mut df, &dir.join("ds.arrow"), meta).unwrap();
    dir
}

/// Label-map and metadata issues only; missing image files are expected.
fn content_issues(dir: &Path) -> Vec<ValidationIssue> {
    validate_dataset_structure(dir)
        .unwrap()
        .into_iter()
        .filter(|i| {
            matches!(
                i,
                ValidationIssue::InvalidLabelMap { .. } | ValidationIssue::InvalidMetadata { .. }
            )
        })
        .collect()
}

fn single_reason(issues: &[ValidationIssue]) -> (&str, &str) {
    match issues {
        [ValidationIssue::InvalidLabelMap { name, reason }] => (name, reason),
        other => panic!("expected one InvalidLabelMap, got {other:?}"),
    }
}

#[test]
fn valid_eight_bit_label_map_passes() {
    let dir = tempfile::TempDir::new().unwrap();
    let mask = MaskData::encode(&[0, 1, 1, 0, 1, 1], 3, 2, 8).unwrap();
    let snapshot = write_snapshot(
        dir.path(),
        &[label_map_sample("a", mask, (3, 2))],
        &TWO_CLASSES,
    );
    assert!(content_issues(&snapshot).is_empty());
}

#[test]
fn valid_sixteen_bit_label_map_passes() {
    let dir = tempfile::TempDir::new().unwrap();
    let labels: Vec<String> = (0..300).map(|i| format!("c{i}")).collect();
    let labels = serde_json::to_string(&labels).unwrap();
    let pixels: Vec<u16> = (0..12).map(|i| 290 + i as u16 - 2).collect();
    let mask = MaskData::encode_16bit(&pixels, 4, 3).unwrap();
    let snapshot = write_snapshot(
        dir.path(),
        &[label_map_sample("a", mask, (4, 3))],
        &[
            ("labels", labels.as_str()),
            ("mask_interpretation", "class_index"),
        ],
    );
    assert!(content_issues(&snapshot).is_empty());
}

#[test]
fn pixel_outside_labels_is_reported() {
    let dir = tempfile::TempDir::new().unwrap();
    let mask = MaskData::encode(&[0, 1, 2, 0], 2, 2, 8).unwrap();
    let snapshot = write_snapshot(
        dir.path(),
        &[label_map_sample("a", mask, (2, 2))],
        &TWO_CLASSES,
    );
    let issues = content_issues(&snapshot);
    let (name, reason) = single_reason(&issues);
    assert_eq!(name, "a");
    assert!(
        reason.contains('2') && reason.contains("labels"),
        "{reason}"
    );
}

#[test]
fn sixteen_bit_pixel_outside_labels_is_reported() {
    let dir = tempfile::TempDir::new().unwrap();
    let mask = MaskData::encode_16bit(&[0, 1, 300, 1], 2, 2).unwrap();
    let snapshot = write_snapshot(
        dir.path(),
        &[label_map_sample("a", mask, (2, 2))],
        &TWO_CLASSES,
    );
    let issues = content_issues(&snapshot);
    assert!(single_reason(&issues).1.contains("300"), "{issues:?}");
}

#[test]
fn one_bit_label_map_is_reported() {
    let dir = tempfile::TempDir::new().unwrap();
    let mask = MaskData::encode(&[0, 1, 1, 0], 2, 2, 1).unwrap();
    let snapshot = write_snapshot(
        dir.path(),
        &[label_map_sample("a", mask, (2, 2))],
        &TWO_CLASSES,
    );
    let issues = content_issues(&snapshot);
    assert!(single_reason(&issues).1.contains("1-bit"), "{issues:?}");
}

#[test]
fn offset_label_map_is_reported() {
    let dir = tempfile::TempDir::new().unwrap();
    let mask = MaskData::encode_with_offset(&[0, 1, 1, 0], 2, 2, 8, (1, 1)).unwrap();
    let snapshot = write_snapshot(
        dir.path(),
        &[label_map_sample("a", mask, (4, 4))],
        &TWO_CLASSES,
    );
    let issues = content_issues(&snapshot);
    assert!(single_reason(&issues).1.contains("oFFs"), "{issues:?}");
}

#[test]
fn model_output_sized_label_map_passes() {
    let dir = tempfile::TempDir::new().unwrap();
    let mask = MaskData::encode(&[0, 1, 1, 0], 2, 2, 8).unwrap();
    let snapshot = write_snapshot(
        dir.path(),
        &[label_map_sample("a", mask, (8, 8))],
        &TWO_CLASSES,
    );
    assert!(content_issues(&snapshot).is_empty());
}

#[test]
fn invalid_metadata_is_reported() {
    let dir = tempfile::TempDir::new().unwrap();
    let mask = MaskData::encode(&[0, 1, 1, 0], 2, 2, 8).unwrap();
    let snapshot = write_snapshot(
        dir.path(),
        &[label_map_sample("a", mask, (2, 2))],
        &[("labels", r#"["a"]"#), ("ignore_index", "5")],
    );
    match content_issues(&snapshot).as_slice() {
        [ValidationIssue::InvalidMetadata { message }] => {
            assert!(message.contains("ignore_index"), "{message}")
        }
        other => panic!("expected InvalidMetadata, got {other:?}"),
    }
}

#[test]
fn mask_only_rows_are_not_label_maps_without_class_index() {
    let dir = tempfile::TempDir::new().unwrap();
    let mask = MaskData::encode(&[0, 1, 1, 0], 2, 2, 1).unwrap();
    let snapshot = write_snapshot(dir.path(), &[label_map_sample("a", mask, (2, 2))], &[]);
    assert!(content_issues(&snapshot).is_empty());
}

#[test]
fn labelled_masks_are_not_label_maps() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut instance = Annotation::new();
    instance.set_name(Some("a".into()));
    instance.set_label(Some("wall".into()));
    instance.set_mask(Some(MaskData::encode(&[0, 1, 1, 0], 2, 2, 1).unwrap()));
    let label_map = label_map_sample("a", MaskData::encode(&[1; 4], 2, 2, 8).unwrap(), (2, 2));
    let sample = Sample {
        annotations: vec![instance, label_map.annotations[0].clone()],
        ..label_map
    };
    let snapshot = write_snapshot(dir.path(), &[sample], &TWO_CLASSES);
    assert!(content_issues(&snapshot).is_empty());
}

#[test]
fn issues_display_the_row_and_reason() {
    let issue = ValidationIssue::InvalidLabelMap {
        name: "ADE_val_00000001".into(),
        reason: "pixel value 151 is outside labels (151 entries)".into(),
    };
    assert_eq!(
        issue.to_string(),
        "Invalid label map for sample 'ADE_val_00000001': pixel value 151 is outside labels (151 entries)"
    );
    let issue = ValidationIssue::InvalidMetadata {
        message: "ignore_index 5 is outside labels".into(),
    };
    assert_eq!(
        issue.to_string(),
        "Invalid dataset metadata: ignore_index 5 is outside labels"
    );
}

#[test]
fn content_issues_are_critical() {
    let critical = [
        ValidationIssue::InvalidMetadata {
            message: String::new(),
        },
        ValidationIssue::InvalidLabelMap {
            name: String::new(),
            reason: String::new(),
        },
        ValidationIssue::MissingArrowFile {
            expected: PathBuf::new(),
        },
        ValidationIssue::MissingSensorContainer {
            expected: PathBuf::new(),
        },
    ];
    assert!(critical.iter().all(ValidationIssue::is_critical));
    let minor = [
        ValidationIssue::MissingFile {
            name: String::new(),
            expected: PathBuf::new(),
        },
        ValidationIssue::UnreferencedFile {
            path: PathBuf::new(),
        },
    ];
    assert!(!minor.iter().any(ValidationIssue::is_critical));
}
