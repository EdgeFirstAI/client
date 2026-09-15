// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

//! VisDrone2019 to EdgeFirst Arrow IPC or Parquet conversion.

use super::reader::{
    CATEGORIES, SplitKind, VisDroneBox, category_name, detect_split_kind,
    infer_group_from_dir_name, read_det_annotations,
};
use crate::{
    Annotation, Box2d, Error, Progress, Sample,
    coco::{SCHEMA_VERSION, write_dataset},
    format::stage_files,
};
use polars::prelude::PlSmallStr;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{Semaphore, mpsc::Sender};

/// Options for VisDrone to Arrow conversion. Construct with
/// `..Default::default()` so new fields can be added in minor releases.
#[derive(Debug, Clone)]
pub struct VisDroneToArrowOptions {
    /// Group for every sample. When `None` the group is inferred from each
    /// split directory name (`-train`, `-val`, `-test-dev`, `-test-challenge`)
    /// and conversion fails for a directory whose name carries no split.
    pub group: Option<String>,
    /// Stage the split's images into the sibling container of the output.
    pub stage_images: bool,
    /// Symlink staged images instead of copying (Unix only; copies elsewhere).
    pub link_images: bool,
    /// Parallel JPEG header reads.
    pub max_workers: usize,
}

impl Default for VisDroneToArrowOptions {
    fn default() -> Self {
        Self {
            group: None,
            stage_images: false,
            link_images: false,
            max_workers: max_workers(),
        }
    }
}

fn max_workers() -> usize {
    std::env::var("MAX_VISDRONE_WORKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            let cpus = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            (cpus / 2).clamp(2, 8)
        })
}

/// A source image and its destination relative to the staged container.
struct StagedFile {
    src: PathBuf,
    dest: PathBuf,
}

/// Read `[width, height]` from a JPEG or PNG header.
fn image_size(path: &Path) -> Result<(u32, u32), Error> {
    let size = imagesize::size(path).map_err(|e| {
        Error::InvalidParameters(format!("Cannot read image size of {}: {e}", path.display()))
    })?;
    Ok((size.width as u32, size.height as u32))
}

/// Normalize a VisDrone pixel box into an EdgeFirst annotation. Label,
/// label_index, truncation and occlusion are set; name/frame/object_id are
/// the caller's responsibility.
fn box_to_annotation(bbox: &VisDroneBox, width: u32, height: u32) -> Annotation {
    let (w, h) = (width as f32, height as f32);
    let mut ann = Annotation::new();
    ann.set_label(category_name(bbox.category).map(String::from));
    ann.set_label_index(Some(bbox.category as u64));
    ann.set_box2d(Some(Box2d::new(
        bbox.left as f32 / w,
        bbox.top as f32 / h,
        bbox.width as f32 / w,
        bbox.height as f32 / h,
    )));
    ann.set_truncation(Some(bbox.truncation));
    ann.set_occlusion(Some(bbox.occlusion));
    ann
}

/// Warn once per split when the `score == 0 <=> category in {0, 11}`
/// invariant does not hold, since the score is not stored.
fn check_score_invariant(split: &Path, boxes: impl Iterator<Item = (u8, u8)>) {
    let violations = boxes
        .filter(|(score, category)| (*score == 0) != matches!(category, 0 | 11))
        .count();
    if violations > 0 {
        log::warn!(
            "{}: {violations} boxes violate the VisDrone score/category rule; the score flag is derived from the label and those rows will be treated by label",
            split.display()
        );
    }
}

/// Convert one DET split directory. Returns one `Sample` per box plus a
/// placeholder `Sample` per image without boxes, and the image staging list.
async fn det_split_samples(
    split: &Path,
    group: Option<&str>,
    max_workers: usize,
    progress: &Option<Sender<Progress>>,
    counter: &Arc<std::sync::atomic::AtomicUsize>,
    total: usize,
) -> Result<(Vec<Sample>, Vec<StagedFile>), Error> {
    let images_dir = split.join("images");
    let ann_dir = split.join("annotations");

    let mut image_paths: Vec<PathBuf> = std::fs::read_dir(&images_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| {
                e.eq_ignore_ascii_case("jpg")
                    || e.eq_ignore_ascii_case("jpeg")
                    || e.eq_ignore_ascii_case("png")
            })
        })
        .collect();
    image_paths.sort();

    let sem = Arc::new(Semaphore::new(max_workers));
    let mut tasks = Vec::with_capacity(image_paths.len());
    for path in image_paths {
        let sem = sem.clone();
        let ann_dir = ann_dir.clone();
        let group = group.map(String::from);
        let split = split.to_path_buf();
        let progress = progress.clone();
        let counter = counter.clone();
        tasks.push(tokio::task::spawn(async move {
            let _permit = sem.acquire().await.map_err(Error::SemaphoreError)?;
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| {
                    Error::InvalidParameters(format!("Bad image name {}", path.display()))
                })?
                .to_string();
            let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
            let ann_path = ann_dir.join(format!("{stem}.txt"));
            let boxes = if ann_path.is_file() {
                tokio::task::spawn_blocking(move || read_det_annotations(&ann_path)).await??
            } else {
                log::warn!("{}: no annotation file for {file_name}", split.display());
                Vec::new()
            };
            let (width, height) = tokio::task::spawn_blocking({
                let path = path.clone();
                move || image_size(&path)
            })
            .await??;

            check_score_invariant(&split, boxes.iter().map(|b| (b.score, b.category)));

            let mut samples: Vec<Sample> = boxes
                .iter()
                .map(|bbox| {
                    let mut ann = box_to_annotation(bbox, width, height);
                    ann.set_name(Some(stem.clone()));
                    ann.set_group(group.clone());
                    Sample {
                        image_name: Some(file_name.clone()),
                        width: Some(width),
                        height: Some(height),
                        group: group.clone(),
                        annotations: vec![ann],
                        ..Default::default()
                    }
                })
                .collect();
            if samples.is_empty() {
                samples.push(Sample {
                    image_name: Some(file_name.clone()),
                    width: Some(width),
                    height: Some(height),
                    group: group.clone(),
                    ..Default::default()
                });
            }

            let done = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            if let Some(p) = &progress {
                let _ = p
                    .send(Progress {
                        current: done,
                        total,
                        status: None,
                    })
                    .await;
            }
            let staged = StagedFile {
                src: path,
                dest: PathBuf::from(file_name),
            };
            Ok::<_, Error>((samples, staged))
        }));
    }

    let mut all = Vec::new();
    let mut staged = Vec::new();
    for task in tasks {
        let (samples, file) = task.await??;
        all.extend(samples);
        staged.push(file);
    }
    Ok((all, staged))
}

async fn vid_split_samples(
    split: &Path,
    _group: Option<&str>,
    _max_workers: usize,
    _progress: &Option<Sender<Progress>>,
    _counter: &Arc<std::sync::atomic::AtomicUsize>,
    _total: usize,
) -> Result<(Vec<Sample>, Vec<StagedFile>), Error> {
    Err(Error::UnsupportedFormat(format!(
        "VID split not yet supported: {}",
        split.display()
    )))
}

/// File-level metadata: schema version, ordered labels, and per-label ids.
fn build_metadata() -> BTreeMap<PlSmallStr, PlSmallStr> {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        PlSmallStr::from("schema_version"),
        PlSmallStr::from(SCHEMA_VERSION),
    );
    let labels = serde_json::to_string(&CATEGORIES).unwrap_or_default();
    metadata.insert(
        PlSmallStr::from("labels"),
        PlSmallStr::from(labels.as_str()),
    );
    let cat_meta: serde_json::Map<String, serde_json::Value> = CATEGORIES
        .iter()
        .enumerate()
        .map(|(id, name)| (name.to_string(), serde_json::json!({ "id": id })))
        .collect();
    let cat_meta = serde_json::to_string(&cat_meta).unwrap_or_default();
    metadata.insert(
        PlSmallStr::from("category_metadata"),
        PlSmallStr::from(cat_meta.as_str()),
    );
    metadata
}

/// Count the images in a split so progress totals are known up front.
fn count_images(split: &Path, kind: SplitKind) -> usize {
    match kind {
        SplitKind::Det => std::fs::read_dir(split.join("images"))
            .map(|d| d.count())
            .unwrap_or(0),
        SplitKind::Vid => std::fs::read_dir(split.join("sequences"))
            .map(|d| {
                d.filter_map(Result::ok)
                    .filter(|e| e.path().is_dir())
                    .map(|e| std::fs::read_dir(e.path()).map(|f| f.count()).unwrap_or(0))
                    .sum()
            })
            .unwrap_or(0),
    }
}

/// Convert one or more VisDrone2019 split directories into one EdgeFirst
/// dataset.
///
/// Each input is a DET split (`annotations/` + `images/`) or a VID split
/// (`annotations/` + `sequences/`). Both kinds may be mixed in one call.
/// The output extension selects Arrow IPC or Parquet. Returns the number of
/// rows written, including placeholder rows for images without boxes.
pub async fn visdrone_to_arrow<P: AsRef<Path>>(
    inputs: &[PathBuf],
    output_path: P,
    options: &VisDroneToArrowOptions,
    progress: Option<Sender<Progress>>,
) -> Result<usize, Error> {
    let output_path = output_path.as_ref();
    if inputs.is_empty() {
        return Err(Error::InvalidParameters(
            "No VisDrone split directories given".into(),
        ));
    }

    let mut plan: Vec<(PathBuf, SplitKind, String)> = Vec::with_capacity(inputs.len());
    for input in inputs {
        let kind = detect_split_kind(input)?;
        let group = match &options.group {
            Some(g) => g.clone(),
            None => infer_group_from_dir_name(input).ok_or_else(|| {
                Error::InvalidParameters(format!(
                    "Cannot infer the split from '{}'; rename it to end in -train, -val, -test-dev or -test-challenge, or pass --group",
                    input.display()
                ))
            })?,
        };
        plan.push((input.clone(), kind, group));
    }

    let total: usize = plan.iter().map(|(p, k, _)| count_images(p, *k)).sum();
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    if let Some(p) = &progress {
        let _ = p
            .send(Progress {
                current: 0,
                total,
                status: None,
            })
            .await;
    }

    let mut all_samples = Vec::new();
    let mut all_staged = Vec::new();
    for (split, kind, group) in &plan {
        let (samples, staged) = match kind {
            SplitKind::Det => {
                det_split_samples(
                    split,
                    Some(group),
                    options.max_workers,
                    &progress,
                    &counter,
                    total,
                )
                .await?
            }
            SplitKind::Vid => {
                vid_split_samples(
                    split,
                    Some(group),
                    options.max_workers,
                    &progress,
                    &counter,
                    total,
                )
                .await?
            }
        };
        all_samples.extend(samples);
        all_staged.extend(staged);
    }

    let mut df = crate::samples_dataframe(&all_samples)?;
    write_dataset(&mut df, output_path, build_metadata())?;

    if options.stage_images {
        let pairs: Vec<(PathBuf, PathBuf)> =
            all_staged.into_iter().map(|f| (f.src, f.dest)).collect();
        let report = stage_files(output_path, &pairs, options.link_images);
        log::info!("Staged {} images", report.staged);
        if !report.missing.is_empty() || !report.failed.is_empty() || report.collisions > 0 {
            log::warn!(
                "Image staging: {} missing, {} failed, {} collisions (first missing: {:?})",
                report.missing.len(),
                report.failed.len(),
                report.collisions,
                report.missing.first()
            );
        }
    }

    Ok(all_samples.len())
}
