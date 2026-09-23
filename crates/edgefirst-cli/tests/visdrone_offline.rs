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
            Some(edgefirst_client::coco::SCHEMA_VERSION)
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

#[test]
fn keep_ignored_flags_rows_without_labels() {
    let temp = tempfile::TempDir::new().unwrap();
    let split = temp.path().join("VisDrone2019-DET-val");
    write_jpeg(&split.join("images").join("a.jpg"), 200, 100);
    std::fs::create_dir_all(split.join("annotations")).unwrap();
    std::fs::write(
        split.join("annotations").join("a.txt"),
        "10,20,40,10,1,4,0,1\r\n0,0,200,100,0,0,0,0\r\n50,50,20,20,0,11,0,0\r\n",
    )
    .unwrap();

    let default_output = temp.path().join("default").join("default.arrow");
    edgefirst_cmd()
        .args([
            "visdrone-to-arrow",
            split.to_str().unwrap(),
            "-o",
            default_output.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Converted 1 rows"));
    let (df, _) = edgefirst_client::format::read_dataset_dataframe(&default_output).unwrap();
    assert!(df.column("ignore").is_err());
    assert!(df.column("exclude").is_err());
    let index = df
        .column("label_index")
        .unwrap()
        .cast(&DataType::UInt64)
        .unwrap();
    assert_eq!(
        index.u64().unwrap().get(0),
        Some(3),
        "car is category 4, index 3"
    );

    let output = temp.path().join("out").join("out.arrow");
    edgefirst_cmd()
        .args([
            "visdrone-to-arrow",
            split.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--keep-ignored",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Converted 3 rows"));

    let (df, _) = edgefirst_client::format::read_dataset_dataframe(&output).unwrap();
    let flags = |name: &str| -> Vec<Option<bool>> {
        df.column(name).unwrap().bool().unwrap().iter().collect()
    };
    let label = df.column("label").unwrap().cast(&DataType::String).unwrap();
    let label: Vec<Option<&str>> = label.str().unwrap().iter().collect();
    let index = df
        .column("label_index")
        .unwrap()
        .cast(&DataType::UInt64)
        .unwrap();
    let index: Vec<Option<u64>> = index.u64().unwrap().iter().collect();

    let rows: Vec<_> = flags("ignore")
        .into_iter()
        .zip(flags("exclude"))
        .zip(label)
        .zip(index)
        .map(|(((i, e), l), x)| (i, e, l, x))
        .collect();
    assert_eq!(rows.len(), 3);
    assert!(rows.contains(&(None, None, Some("car"), Some(3))));
    assert!(
        rows.contains(&(Some(true), None, None, None)),
        "ignored region: {rows:?}"
    );
    assert!(
        rows.contains(&(None, Some(true), None, None)),
        "others: {rows:?}"
    );
}
