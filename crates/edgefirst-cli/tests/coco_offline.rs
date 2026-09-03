// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use assert_cmd::Command;
use polars::prelude::DataType;

fn edgefirst_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("edgefirst-client"))
}

#[test]
fn directory_to_arrow_and_parquet_preserves_groups() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let coco_root = temp_dir.path().join("coco");
    let annotations = coco_root.join("annotations");
    std::fs::create_dir_all(&annotations).unwrap();

    let split_json = |file_name: &str| {
        format!(
            r#"{{
                "images": [{{"id": 1, "width": 640, "height": 480, "file_name": "{file_name}"}}],
                "annotations": [{{"id": 1, "image_id": 1, "category_id": 1, "bbox": [10, 20, 100, 80], "area": 8000, "iscrowd": 0}}],
                "categories": [{{"id": 1, "name": "person"}}]
            }}"#
        )
    };
    std::fs::write(
        annotations.join("instances_train2017.json"),
        split_json("train.jpg"),
    )
    .unwrap();
    std::fs::write(
        annotations.join("instances_val2017.json"),
        split_json("val.jpg"),
    )
    .unwrap();

    for extension in ["arrow", "parquet"] {
        let output = temp_dir.path().join(format!("combined.{extension}"));
        edgefirst_cmd()
            .args([
                "coco-to-arrow",
                coco_root.to_str().unwrap(),
                "-o",
                output.to_str().unwrap(),
            ])
            .assert()
            .success()
            .stdout(predicates::str::contains("Converted 2"));

        let (dataframe, _) = edgefirst_client::format::read_dataset_dataframe(&output).unwrap();
        let groups = dataframe
            .column("group")
            .unwrap()
            .cast(&DataType::String)
            .unwrap();
        let groups: std::collections::BTreeSet<_> =
            groups.str().unwrap().iter().flatten().collect();
        assert_eq!(groups, std::collections::BTreeSet::from(["train", "val"]));
    }
}
