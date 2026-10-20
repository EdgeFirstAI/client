// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

use super::{
    Ade20kToArrowOptions, ade20k_to_arrow, list_split_pairs, read_label_map, read_object_info,
};
use crate::format::read_dataset_dataframe;
use polars::prelude::*;
use std::path::{Path, PathBuf};

const OBJECT_INFO: &str = "Idx\tRatio\tTrain\tVal\tName\n\
                           1\t0.1\t1\t1\twall\n\
                           2\t0.1\t1\t1\tbuilding, edifice\n\
                           3\t0.1\t1\t1\tsky\n";

/// Write a tiny JPEG so `imagesize` can read its dimensions.
pub(crate) fn write_jpeg(path: &Path, width: u32, height: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::RgbImage::new(width, height).save(path).unwrap();
}

/// Write an 8-bit grayscale label PNG whose pixel `i` is `i % 4`.
pub(crate) fn write_label(path: &Path, width: u32, height: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let pixels = (0..width * height).map(|i| (i % 4) as u8).collect();
    image::GrayImage::from_raw(width, height, pixels)
        .unwrap()
        .save(path)
        .unwrap();
}

/// Build `ADEChallengeData2016/` with three classes, two validation images
/// and one training image.
pub(crate) fn make_ade20k(root: &Path) -> PathBuf {
    let ade = root.join("ADEChallengeData2016");
    std::fs::create_dir_all(&ade).unwrap();
    std::fs::write(ade.join("objectInfo150.txt"), OBJECT_INFO).unwrap();
    for (split, stem, w, h) in [
        ("validation", "ADE_val_00000002", 6, 4),
        ("validation", "ADE_val_00000001", 5, 3),
        ("training", "ADE_train_00000001", 4, 4),
    ] {
        write_jpeg(
            &ade.join("images").join(split).join(format!("{stem}.jpg")),
            w,
            h,
        );
        write_label(
            &ade.join("annotations")
                .join(split)
                .join(format!("{stem}.png")),
            w,
            h,
        );
    }
    ade
}

#[test]
fn object_info_parses_names_and_synonyms() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let classes = read_object_info(&ade.join("objectInfo150.txt")).unwrap();
    assert_eq!(classes.len(), 3);
    assert_eq!(classes[0].id, 1);
    assert_eq!(classes[0].name, "wall");
    assert_eq!(classes[0].synonyms, vec!["wall"]);
    assert_eq!(classes[1].id, 2);
    assert_eq!(classes[1].name, "building");
    assert_eq!(classes[1].synonyms, vec!["building", "edifice"]);
}

#[test]
fn object_info_rejects_ids_out_of_order() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("objectInfo150.txt");
    std::fs::write(&path, "Idx\tRatio\tTrain\tVal\tName\n2\t0\t0\t0\twall\n").unwrap();
    let err = read_object_info(&path).unwrap_err().to_string();
    assert!(err.contains("objectInfo150.txt"), "{err}");
}

#[test]
fn pairs_are_sorted_by_stem() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let pairs = list_split_pairs(&ade, "validation").unwrap();
    let stems: Vec<_> = pairs.iter().map(|p| p.stem.as_str()).collect();
    assert_eq!(stems, ["ADE_val_00000001", "ADE_val_00000002"]);
    assert_eq!(
        pairs[0].label,
        ade.join("annotations/validation/ADE_val_00000001.png")
    );
    assert_eq!(
        pairs[0].image,
        ade.join("images/validation/ADE_val_00000001.jpg")
    );
}

#[test]
fn image_without_label_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    std::fs::remove_file(ade.join("annotations/validation/ADE_val_00000002.png")).unwrap();
    let err = list_split_pairs(&ade, "validation")
        .unwrap_err()
        .to_string();
    assert!(err.contains("ADE_val_00000002"), "{err}");
}

#[test]
fn label_without_image_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    std::fs::remove_file(ade.join("images/validation/ADE_val_00000001.jpg")).unwrap();
    let err = list_split_pairs(&ade, "validation")
        .unwrap_err()
        .to_string();
    assert!(err.contains("ADE_val_00000001"), "{err}");
}

#[test]
fn split_directory_given_as_root_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let err = list_split_pairs(&ade.join("images"), "validation")
        .unwrap_err()
        .to_string();
    assert!(err.contains("validation"), "{err}");
}

#[test]
fn label_map_keeps_bytes() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let path = ade.join("annotations/validation/ADE_val_00000002.png");
    let mask = read_label_map(&path).unwrap();
    assert_eq!(mask.as_bytes(), std::fs::read(&path).unwrap());
    assert_eq!((mask.width(), mask.height(), mask.bit_depth()), (6, 4, 8));
}

#[test]
fn rgb_label_png_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("ADE_val_rgb.png");
    image::RgbImage::new(2, 2).save(&path).unwrap();
    let err = read_label_map(&path).unwrap_err().to_string();
    assert!(err.contains("ADE_val_rgb.png"), "{err}");
}

#[test]
fn one_bit_label_png_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("ADE_val_1bit.png");
    let mask = crate::MaskData::encode(&[1, 0, 0, 1], 2, 2, 1).unwrap();
    std::fs::write(&path, mask.as_bytes()).unwrap();
    let err = read_label_map(&path).unwrap_err().to_string();
    assert!(err.contains("ADE_val_1bit.png"), "{err}");
}

#[test]
fn offset_label_png_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("ADE_val_tile.png");
    let mask = crate::MaskData::encode_with_offset(&[1, 2, 3, 4], 2, 2, 8, (1, 1)).unwrap();
    std::fs::write(&path, mask.as_bytes()).unwrap();
    let err = read_label_map(&path).unwrap_err().to_string();
    assert!(err.contains("ADE_val_tile.png"), "{err}");
}

#[test]
fn label_png_with_any_offs_chunk_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("ADE_val_micrometre.png");
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 2, 2);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        // Offset (1, 1) in micrometres (unit byte 1).
        let data = [0, 0, 0, 1, 0, 0, 0, 1, 1];
        writer
            .write_chunk(png::chunk::ChunkType(*b"oFFs"), &data)
            .unwrap();
        writer.write_image_data(&[1, 2, 3, 4]).unwrap();
    }
    std::fs::write(&path, &png).unwrap();
    let err = read_label_map(&path).unwrap_err().to_string();
    assert!(err.contains("ADE_val_micrometre.png"), "{err}");
}

fn group_of(df: &DataFrame, idx: usize) -> String {
    let groups = df.column("group").unwrap().cast(&DataType::String).unwrap();
    groups.str().unwrap().get(idx).unwrap().to_owned()
}

#[tokio::test]
async fn converts_label_maps_unchanged() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let output = dir.path().join("out").join("out.arrow");
    let rows = ade20k_to_arrow(&ade, &output, &Ade20kToArrowOptions::default(), None)
        .await
        .unwrap();
    assert_eq!(rows, 3);

    let (df, _) = read_dataset_dataframe(&output).unwrap();
    assert_eq!(df.height(), 3);
    // samples_dataframe omits all-null columns, so absent means null.
    for column in ["label", "label_index", "box2d", "box3d", "polygon"] {
        if let Ok(c) = df.column(column) {
            assert_eq!(c.null_count(), 3, "{column} must be null");
        }
    }
    let names = df.column("name").unwrap().str().unwrap().clone();
    let masks = df.column("mask").unwrap().binary().unwrap().clone();
    for idx in 0..df.height() {
        let name = names.get(idx).unwrap();
        let split = if name.starts_with("ADE_val") {
            "validation"
        } else {
            "training"
        };
        assert_eq!(
            group_of(&df, idx),
            if split == "validation" {
                "val"
            } else {
                "train"
            }
        );
        let source = std::fs::read(
            ade.join("annotations")
                .join(split)
                .join(format!("{name}.png")),
        )
        .unwrap();
        assert_eq!(masks.get(idx).unwrap(), source.as_slice(), "{name}");
    }
    let sizes = df.column("size").unwrap().array().unwrap().clone();
    let first = names
        .iter()
        .position(|n| n == Some("ADE_val_00000002"))
        .unwrap();
    let size = sizes.get_as_series(first).unwrap();
    let size: Vec<u32> = size.u32().unwrap().into_no_null_iter().collect();
    assert_eq!(size, [6, 4]);
}

#[tokio::test]
async fn writes_class_index_metadata() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let output = dir.path().join("out").join("out.arrow");
    ade20k_to_arrow(&ade, &output, &Ade20kToArrowOptions::default(), None)
        .await
        .unwrap();
    let (_, metadata) = read_dataset_dataframe(&output).unwrap();
    let get = |k: &str| metadata.get(k).map(String::as_str);
    assert_eq!(get("schema_version"), Some(crate::coco::SCHEMA_VERSION));
    let labels: Vec<String> = serde_json::from_str(get("labels").unwrap()).unwrap();
    assert_eq!(labels, ["unlabeled", "wall", "building", "sky"]);
    assert_eq!(get("ignore_index"), Some("0"));
    assert_eq!(get("mask_interpretation"), Some("class_index"));
    let categories: serde_json::Value =
        serde_json::from_str(get("category_metadata").unwrap()).unwrap();
    assert_eq!(
        categories["building"],
        serde_json::json!({"id": 2, "synonyms": ["building", "edifice"]})
    );
    assert_eq!(categories["wall"], serde_json::json!({"id": 1}));
    assert!(categories.get("unlabeled").is_none());

    let typed = crate::format::DatasetMetadata::read(&output).unwrap();
    assert_eq!(typed.ignore_index, Some(0));
    assert_eq!(
        typed.mask_interpretation,
        crate::format::MaskInterpretation::ClassIndex
    );
    assert_eq!(typed.labels.unwrap().len(), 4);
    assert!(typed.extra.is_empty(), "{:?}", typed.extra);
}

#[tokio::test]
async fn group_selects_one_split() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let output = dir.path().join("out").join("out.arrow");
    let options = Ade20kToArrowOptions {
        group: Some("val".into()),
        ..Default::default()
    };
    assert_eq!(
        ade20k_to_arrow(&ade, &output, &options, None)
            .await
            .unwrap(),
        2
    );
    let (df, _) = read_dataset_dataframe(&output).unwrap();
    assert!((0..df.height()).all(|i| group_of(&df, i) == "val"));
}

#[tokio::test]
async fn unknown_group_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let output = dir.path().join("out").join("out.arrow");
    let options = Ade20kToArrowOptions {
        group: Some("test".into()),
        ..Default::default()
    };
    let err = ade20k_to_arrow(&ade, &output, &options, None)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("test"), "{err}");
    assert!(!output.exists());
}

#[tokio::test]
async fn root_without_object_info_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let output = dir.path().join("out").join("out.arrow");
    let err = ade20k_to_arrow(&ade.join("images"), &output, &Default::default(), None)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("objectInfo150.txt"), "{err}");
}

#[tokio::test]
async fn label_size_mismatch_is_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    write_jpeg(&ade.join("images/training/ADE_train_00000001.jpg"), 5, 4);
    let output = dir.path().join("out").join("out.arrow");
    let err = ade20k_to_arrow(&ade, &output, &Default::default(), None)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("ADE_train_00000001"), "{err}");
}

#[tokio::test]
async fn parquet_matches_arrow() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let arrow = dir.path().join("a").join("a.arrow");
    let parquet = dir.path().join("p").join("p.parquet");
    ade20k_to_arrow(&ade, &arrow, &Default::default(), None)
        .await
        .unwrap();
    ade20k_to_arrow(&ade, &parquet, &Default::default(), None)
        .await
        .unwrap();
    let (a, am) = read_dataset_dataframe(&arrow).unwrap();
    let (p, pm) = read_dataset_dataframe(&parquet).unwrap();
    assert!(a.equals_missing(&p));
    // Parquet adds its own ARROW:schema entry.
    let pm: std::collections::BTreeMap<_, _> = pm
        .into_iter()
        .filter(|(k, _)| !k.starts_with("ARROW:"))
        .collect();
    assert_eq!(am, pm);
}

#[tokio::test]
async fn stages_images_next_to_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let output = dir.path().join("ds").join("ds.arrow");
    let options = Ade20kToArrowOptions {
        stage_images: true,
        ..Default::default()
    };
    ade20k_to_arrow(&ade, &output, &options, None)
        .await
        .unwrap();
    let staged = dir.path().join("ds").join("ds");
    assert!(staged.join("ADE_val_00000001.jpg").is_file());
    assert!(staged.join("ADE_train_00000001.jpg").is_file());
    let issues = crate::format::validate_dataset_structure(&dir.path().join("ds")).unwrap();
    assert!(issues.is_empty(), "{issues:?}");
}

#[tokio::test]
async fn arrow_to_coco_skips_label_maps() {
    let dir = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(dir.path());
    let output = dir.path().join("out").join("out.arrow");
    ade20k_to_arrow(&ade, &output, &Default::default(), None)
        .await
        .unwrap();
    let coco = dir.path().join("coco.json");
    crate::coco::arrow_to_coco(&output, &coco, &Default::default(), None)
        .await
        .unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&coco).unwrap()).unwrap();
    assert_eq!(json["images"].as_array().unwrap().len(), 3);
    assert!(json["annotations"].as_array().unwrap().is_empty());
}
