// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

use assert_cmd::Command;
use polars::prelude::DataType;
use std::path::Path;

fn edgefirst_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("edgefirst-client"))
}

fn write_jpeg(path: &Path, width: u32, height: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::RgbImage::new(width, height).save(path).unwrap();
}

#[test]
fn det_and_vid_splits_to_arrow_and_parquet() {
    let temp = tempfile::TempDir::new().unwrap();

    let det = temp.path().join("VisDrone2019-DET-val");
    write_jpeg(&det.join("images").join("a.jpg"), 200, 100);
    std::fs::create_dir_all(det.join("annotations")).unwrap();
    std::fs::write(
        det.join("annotations").join("a.txt"),
        "10,20,40,10,1,4,0,1\r\n",
    )
    .unwrap();

    let vid = temp.path().join("VisDrone2019-VID-train");
    let seq = vid.join("sequences").join("uav0000001_00000_v");
    write_jpeg(&seq.join("0000001.jpg"), 160, 90);
    write_jpeg(&seq.join("0000002.jpg"), 160, 90);
    std::fs::create_dir_all(vid.join("annotations")).unwrap();
    std::fs::write(
        vid.join("annotations").join("uav0000001_00000_v.txt"),
        "1,7,10,10,20,20,1,1,0,0\r\n2,7,12,10,20,20,1,1,0,0\r\n",
    )
    .unwrap();

    for extension in ["arrow", "parquet"] {
        let out_dir = temp.path().join(format!("out_{extension}"));
        let output = out_dir.join(format!("out_{extension}.{extension}"));
        edgefirst_cmd()
            .args([
                "visdrone-to-arrow",
                det.to_str().unwrap(),
                vid.to_str().unwrap(),
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
            metadata.get("schema_version").map(String::as_str),
            Some("2026.04")
        );
        let groups = df.column("group").unwrap().cast(&DataType::String).unwrap();
        let groups: std::collections::BTreeSet<_> =
            groups.str().unwrap().iter().flatten().collect();
        assert_eq!(groups, std::collections::BTreeSet::from(["train", "val"]));

        assert!(
            out_dir
                .join(format!("out_{extension}"))
                .join("a.jpg")
                .is_file()
        );
        assert!(
            out_dir
                .join(format!("out_{extension}"))
                .join("uav0000001_00000_v")
                .join("uav0000001_00000_v_1.camera.jpeg")
                .is_file()
        );

        edgefirst_cmd()
            .args(["validate-snapshot", out_dir.to_str().unwrap()])
            .assert()
            .success();
    }
}

#[test]
fn group_is_required_for_unnamed_split() {
    let temp = tempfile::TempDir::new().unwrap();
    let split = temp.path().join("testdev");
    write_jpeg(&split.join("images").join("a.jpg"), 20, 10);
    std::fs::create_dir_all(split.join("annotations")).unwrap();
    std::fs::write(
        split.join("annotations").join("a.txt"),
        "1,1,5,5,1,4,0,0\r\n",
    )
    .unwrap();
    let output = temp.path().join("o").join("o.arrow");

    edgefirst_cmd()
        .args([
            "visdrone-to-arrow",
            split.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--group"));

    edgefirst_cmd()
        .args([
            "visdrone-to-arrow",
            split.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--group",
            "test-dev",
        ])
        .assert()
        .success();
}
