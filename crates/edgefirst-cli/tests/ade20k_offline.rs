// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

use assert_cmd::Command;
use std::path::{Path, PathBuf};

fn edgefirst_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("edgefirst-client"))
}

fn write_jpeg(path: &Path, width: u32, height: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::RgbImage::new(width, height).save(path).unwrap();
}

fn write_label(path: &Path, width: u32, height: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let pixels: Vec<u8> = (0..width * height).map(|i| (i % 4) as u8).collect();
    let png = edgefirst_client::MaskData::encode(&pixels, width, height, 8).unwrap();
    std::fs::write(path, png.as_bytes()).unwrap();
}

fn make_ade20k(root: &Path) -> PathBuf {
    let ade = root.join("ADEChallengeData2016");
    std::fs::create_dir_all(&ade).unwrap();
    std::fs::write(
        ade.join("objectInfo150.txt"),
        "Idx\tRatio\tTrain\tVal\tName\n1\t0.1\t1\t1\twall\n2\t0.1\t1\t1\tbuilding, edifice\n3\t0.1\t1\t1\tsky\n",
    )
    .unwrap();
    for (split, stem, w, h) in [
        ("validation", "ADE_val_00000001", 5, 3),
        ("validation", "ADE_val_00000002", 6, 4),
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
fn ade20k_to_arrow_and_parquet() {
    let temp = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(temp.path());

    for extension in ["arrow", "parquet"] {
        let out_dir = temp.path().join(format!("out_{extension}"));
        let output = out_dir.join(format!("out_{extension}.{extension}"));
        edgefirst_cmd()
            .args([
                "ade20k-to-arrow",
                ade.to_str().unwrap(),
                "-o",
                output.to_str().unwrap(),
                "--images",
            ])
            .assert()
            .success()
            .stdout(predicates::str::contains("Converted 3 rows"));

        let (df, metadata) = edgefirst_client::format::read_dataset_dataframe(&output).unwrap();
        assert_eq!(df.height(), 3);
        assert_eq!(
            metadata.get("mask_interpretation").map(String::as_str),
            Some("class_index")
        );
        assert!(
            out_dir
                .join(format!("out_{extension}"))
                .join("ADE_val_00000001.jpg")
                .is_file()
        );

        edgefirst_cmd()
            .args(["validate-snapshot", out_dir.to_str().unwrap()])
            .assert()
            .success();
    }
}

#[test]
fn group_selects_validation() {
    let temp = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(temp.path());
    let output = temp.path().join("val").join("val.arrow");
    edgefirst_cmd()
        .args([
            "ade20k-to-arrow",
            ade.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--group",
            "val",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Converted 2 rows"));
}

#[test]
fn unknown_group_fails() {
    let temp = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(temp.path());
    let output = temp.path().join("t").join("t.arrow");
    edgefirst_cmd()
        .args([
            "ade20k-to-arrow",
            ade.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--group",
            "test",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Unknown ADE20K group"));
}

#[test]
fn validate_snapshot_rejects_label_values_outside_labels() {
    let temp = tempfile::TempDir::new().unwrap();
    let ade = make_ade20k(temp.path());
    let bad = edgefirst_client::MaskData::encode(&[9; 15], 5, 3, 8).unwrap();
    std::fs::write(
        ade.join("annotations/validation/ADE_val_00000001.png"),
        bad.as_bytes(),
    )
    .unwrap();
    let out_dir = temp.path().join("out");
    let output = out_dir.join("out.arrow");
    edgefirst_cmd()
        .args([
            "ade20k-to-arrow",
            ade.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--images",
        ])
        .assert()
        .success();

    edgefirst_cmd()
        .args(["validate-snapshot", out_dir.to_str().unwrap()])
        .assert()
        .failure()
        .stdout(predicates::str::contains(
            "Invalid label map for sample 'ADE_val_00000001': pixel value 9 is outside labels (4 entries)",
        ));
}
