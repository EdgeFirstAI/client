// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

use super::{VisDroneToArrowOptions, visdrone_to_arrow};
use crate::format::read_dataset_dataframe;
use polars::prelude::*;
use std::path::{Path, PathBuf};

/// Write a tiny JPEG so `imagesize` can read its dimensions.
pub(crate) fn write_jpeg(path: &Path, width: u32, height: u32) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    image::RgbImage::new(width, height).save(path).unwrap();
}

/// Build `VisDrone2019-DET-val/` with two images: one with three boxes
/// (including an ignored region and a trailing-comma line), one with none.
pub(crate) fn make_det_split(root: &Path) -> PathBuf {
    let split = root.join("VisDrone2019-DET-val");
    write_jpeg(&split.join("images").join("img_a.jpg"), 200, 100);
    write_jpeg(&split.join("images").join("img_b.jpg"), 100, 50);
    std::fs::create_dir_all(split.join("annotations")).unwrap();
    std::fs::write(
        split.join("annotations").join("img_a.txt"),
        "10,20,40,10,1,4,0,1\r\n0,0,200,100,0,0,0,0\r\n50,50,20,20,1,1,1,2,\r\n",
    )
    .unwrap();
    std::fs::write(split.join("annotations").join("img_b.txt"), "").unwrap();
    split
}

#[tokio::test]
async fn det_split_converts_boxes_labels_and_placeholders() {
    let dir = tempfile::TempDir::new().unwrap();
    let split = make_det_split(dir.path());
    let output = dir.path().join("out").join("out.arrow");

    let rows = visdrone_to_arrow(&[split], &output, &VisDroneToArrowOptions::default(), None)
        .await
        .unwrap();
    assert_eq!(rows, 4, "three boxes plus one placeholder");

    let (df, metadata) = read_dataset_dataframe(&output).unwrap();
    assert_eq!(
        metadata.get("schema_version").map(String::as_str),
        Some("2026.04")
    );
    let labels: Vec<String> = serde_json::from_str(metadata.get("labels").unwrap()).unwrap();
    assert_eq!(labels.len(), 12);
    assert_eq!(labels[0], "ignored regions");
    assert_eq!(labels[11], "others");

    let names = df.column("name").unwrap().str().unwrap();
    let a_rows: Vec<usize> = (0..df.height())
        .filter(|&i| names.get(i) == Some("img_a"))
        .collect();
    assert_eq!(a_rows.len(), 3);

    let label_index = df.column("label_index").unwrap().u64().unwrap();
    assert_eq!(label_index.get(a_rows[0]), Some(4));
    assert_eq!(
        label_index.get(a_rows[1]),
        Some(0),
        "ignored region kept with index 0"
    );

    let boxes = df.column("box2d").unwrap().array().unwrap();
    let first: Vec<f32> = boxes
        .get_as_series(a_rows[0])
        .unwrap()
        .f32()
        .unwrap()
        .into_no_null_iter()
        .collect();
    // left 10, top 20, w 40, h 10 on 200x100 -> cx 0.15, cy 0.25, w 0.2, h 0.1
    assert!((first[0] - 0.15).abs() < 1e-6 && (first[1] - 0.25).abs() < 1e-6);
    assert!((first[2] - 0.2).abs() < 1e-6 && (first[3] - 0.1).abs() < 1e-6);

    let truncation = df.column("truncation").unwrap().u32().unwrap();
    let occlusion = df.column("occlusion").unwrap().u32().unwrap();
    assert_eq!(truncation.get(a_rows[2]), Some(1));
    assert_eq!(occlusion.get(a_rows[2]), Some(2));

    let groups = df.column("group").unwrap().cast(&DataType::String).unwrap();
    assert_eq!(
        groups.str().unwrap().get(0),
        Some("val"),
        "group inferred from directory name"
    );
    assert!(df.column("frame").is_err(), "DET has no frames");

    let size = df.column("size").unwrap().array().unwrap();
    let b_row = (0..df.height())
        .find(|&i| names.get(i) == Some("img_b"))
        .unwrap();
    let b_size: Vec<u32> = size
        .get_as_series(b_row)
        .unwrap()
        .u32()
        .unwrap()
        .into_no_null_iter()
        .collect();
    assert_eq!(b_size, vec![100, 50]);
    assert!(df.column("label").unwrap().is_null().get(b_row).unwrap());
}

#[tokio::test]
async fn det_split_stages_images_next_to_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let split = make_det_split(dir.path());
    let output = dir.path().join("ds").join("ds.parquet");
    let options = VisDroneToArrowOptions {
        stage_images: true,
        ..Default::default()
    };
    visdrone_to_arrow(&[split], &output, &options, None)
        .await
        .unwrap();
    assert!(dir.path().join("ds").join("ds").join("img_a.jpg").is_file());
    assert!(dir.path().join("ds").join("ds").join("img_b.jpg").is_file());
    let issues = crate::format::validate_dataset_structure(&dir.path().join("ds")).unwrap();
    assert!(issues.is_empty(), "{issues:?}");
}

#[tokio::test]
async fn group_override_and_unknown_dir_name() {
    let dir = tempfile::TempDir::new().unwrap();
    let split = make_det_split(dir.path());
    let renamed = dir.path().join("testdev");
    std::fs::rename(&split, &renamed).unwrap();
    let output = dir.path().join("o").join("o.arrow");

    let err = visdrone_to_arrow(
        std::slice::from_ref(&renamed),
        &output,
        &VisDroneToArrowOptions::default(),
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, crate::Error::InvalidParameters(_)), "{err:?}");

    let options = VisDroneToArrowOptions {
        group: Some("test-dev".into()),
        ..Default::default()
    };
    visdrone_to_arrow(&[renamed], &output, &options, None)
        .await
        .unwrap();
    let (df, _) = read_dataset_dataframe(&output).unwrap();
    let groups = df.column("group").unwrap().cast(&DataType::String).unwrap();
    assert_eq!(groups.str().unwrap().get(0), Some("test-dev"));
}

/// Build `VisDrone2019-VID-train/` with one three-frame sequence. Frame 3
/// has no rows. Track 7 changes category between frames 1 and 2. Track 0
/// is a real object. The ignored region carries a track id.
pub(crate) fn make_vid_split(root: &Path) -> PathBuf {
    let split = root.join("VisDrone2019-VID-train");
    let seq = split.join("sequences").join("uav0000001_00000_v");
    for frame in 1..=3 {
        write_jpeg(&seq.join(format!("{frame:07}.jpg")), 160, 90);
    }
    std::fs::create_dir_all(split.join("annotations")).unwrap();
    std::fs::write(
        split.join("annotations").join("uav0000001_00000_v.txt"),
        "1,7,10,10,20,20,1,1,0,0\r\n1,0,50,50,10,10,1,4,0,1\r\n2,7,12,10,20,20,1,2,0,0\r\n2,9,0,0,160,90,0,0,0,0\r\n",
    )
    .unwrap();
    split
}

#[tokio::test]
async fn vid_split_converts_frames_tracks_and_placeholders() {
    let dir = tempfile::TempDir::new().unwrap();
    let split = make_vid_split(dir.path());
    let output = dir.path().join("vid").join("vid.arrow");
    let options = VisDroneToArrowOptions {
        stage_images: true,
        ..Default::default()
    };

    let rows = visdrone_to_arrow(&[split], &output, &options, None)
        .await
        .unwrap();
    assert_eq!(rows, 5, "four boxes plus one placeholder for frame 3");

    let (df, _) = read_dataset_dataframe(&output).unwrap();
    let names = df.column("name").unwrap().str().unwrap();
    assert!((0..df.height()).all(|i| names.get(i) == Some("uav0000001_00000_v")));

    let frames = df.column("frame").unwrap().u32().unwrap();
    let object_ids = df.column("object_id").unwrap().str().unwrap();
    let labels = df.column("label").unwrap().cast(&DataType::String).unwrap();
    let labels = labels.str().unwrap();

    let mut seen: Vec<(u32, Option<&str>, Option<&str>)> = (0..df.height())
        .map(|i| (frames.get(i).unwrap(), object_ids.get(i), labels.get(i)))
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![
            (1, Some("uav0000001_00000_v/0"), Some("car")),
            (1, Some("uav0000001_00000_v/7"), Some("pedestrian")),
            (2, Some("uav0000001_00000_v/7"), Some("people")),
            (2, Some("uav0000001_00000_v/9"), Some("ignored regions")),
            (3, None, None),
        ]
    );

    let groups = df.column("group").unwrap().cast(&DataType::String).unwrap();
    assert_eq!(groups.str().unwrap().get(0), Some("train"));

    let container = dir
        .path()
        .join("vid")
        .join("vid")
        .join("uav0000001_00000_v");
    for frame in 1..=3 {
        assert!(
            container
                .join(format!("uav0000001_00000_v_{frame}.camera.jpeg"))
                .is_file(),
            "frame {frame}"
        );
    }
    let issues = crate::format::validate_dataset_structure(&dir.path().join("vid")).unwrap();
    assert!(issues.is_empty(), "{issues:?}");
}

#[tokio::test]
async fn det_and_vid_splits_combine_into_one_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let det = make_det_split(dir.path());
    let vid = make_vid_split(dir.path());
    let output = dir.path().join("mix").join("mix.parquet");
    let rows = visdrone_to_arrow(
        &[det, vid],
        &output,
        &VisDroneToArrowOptions::default(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(rows, 9);
    let (df, _) = read_dataset_dataframe(&output).unwrap();
    let groups = df.column("group").unwrap().cast(&DataType::String).unwrap();
    let set: std::collections::BTreeSet<_> = groups.str().unwrap().iter().flatten().collect();
    assert_eq!(set, std::collections::BTreeSet::from(["train", "val"]));
    assert_eq!(
        df.column("frame").unwrap().null_count(),
        4,
        "DET rows have null frames"
    );
}
