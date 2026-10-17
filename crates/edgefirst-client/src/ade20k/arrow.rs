// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

//! ADE20K to EdgeFirst Arrow IPC or Parquet conversion.

use super::reader::{
    Ade20kClass, Ade20kPair, SPLITS, list_split_pairs, read_label_map, read_object_info,
};
use crate::{
    Annotation, Error, Progress, Sample,
    coco::{SCHEMA_VERSION, write_dataset},
    format::stage_files,
};
use polars::prelude::PlSmallStr;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::{Semaphore, mpsc::Sender};

/// Name of the don't-care class at `labels` position 0.
const UNLABELED: &str = "unlabeled";

/// Options for ADE20K to Arrow conversion. Construct with
/// `..Default::default()` so new fields can be added in minor releases.
#[derive(Debug, Clone)]
pub struct Ade20kToArrowOptions {
    /// Convert only this group (`"train"` or `"val"`); `None` converts both.
    pub group: Option<String>,
    /// Stage the images into the sibling container of the output.
    pub stage_images: bool,
    /// Symlink staged images instead of copying (Unix only; copies elsewhere).
    pub link_images: bool,
    /// Parallel image and label reads. Must be at least 1.
    pub max_workers: usize,
}

impl Default for Ade20kToArrowOptions {
    fn default() -> Self {
        Self {
            group: None,
            stage_images: false,
            link_images: false,
            max_workers: max_workers(),
        }
    }
}

/// Default worker count: `MAX_ADE20K_WORKERS` when set to a positive
/// integer, otherwise half the CPUs clamped to 2..=8.
fn max_workers() -> usize {
    std::env::var("MAX_ADE20K_WORKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&v: &usize| v > 0)
        .unwrap_or_else(|| {
            let cpus = std::thread::available_parallelism()
                .map(std::num::NonZeroUsize::get)
                .unwrap_or(4);
            (cpus / 2).clamp(2, 8)
        })
}

/// Builds the label-map sample for one image, checking that the label PNG
/// has the image's size.
fn pair_to_sample(pair: &Ade20kPair, group: &str) -> Result<Sample, Error> {
    let size = imagesize::size(&pair.image).map_err(|e| {
        Error::InvalidParameters(format!(
            "Cannot read image size of {}: {e}",
            pair.image.display()
        ))
    })?;
    let (width, height) = (size.width as u32, size.height as u32);
    let mask = read_label_map(&pair.label)?;
    if (mask.width(), mask.height()) != (width, height) {
        return Err(Error::InvalidParameters(format!(
            "ADE20K {}: label is {}x{} but the image is {width}x{height}",
            pair.stem,
            mask.width(),
            mask.height()
        )));
    }
    let file_name = pair
        .image
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            Error::InvalidParameters(format!("Bad image name {}", pair.image.display()))
        })?
        .to_owned();
    let mut ann = Annotation::new();
    ann.set_name(Some(pair.stem.clone()));
    ann.set_group(Some(group.to_owned()));
    ann.set_mask(Some(mask));
    Ok(Sample {
        image_name: Some(file_name),
        width: Some(width),
        height: Some(height),
        group: Some(group.to_owned()),
        annotations: vec![ann],
        ..Default::default()
    })
}

/// File-level metadata: `labels` puts `unlabeled` at position 0 so a pixel
/// value is its class's position, which `ignore_index` marks as don't-care.
fn build_metadata(classes: &[Ade20kClass]) -> BTreeMap<PlSmallStr, PlSmallStr> {
    let mut labels = vec![UNLABELED];
    labels.extend(classes.iter().map(|c| c.name.as_str()));
    let categories: serde_json::Map<String, serde_json::Value> = classes
        .iter()
        .map(|c| {
            let mut entry = serde_json::json!({ "id": c.id });
            if c.synonyms.len() > 1 {
                entry["synonyms"] = serde_json::json!(c.synonyms);
            }
            (c.name.clone(), entry)
        })
        .collect();
    let json = |v: serde_json::Value| PlSmallStr::from(v.to_string().as_str());
    BTreeMap::from([
        ("schema_version".into(), SCHEMA_VERSION.into()),
        ("labels".into(), json(serde_json::json!(labels))),
        ("ignore_index".into(), "0".into()),
        ("mask_interpretation".into(), "class_index".into()),
        (
            "category_metadata".into(),
            json(serde_json::Value::Object(categories)),
        ),
    ])
}

/// Convert ADEChallengeData2016 into one EdgeFirst dataset of semantic
/// label maps.
///
/// `root` is the `ADEChallengeData2016` directory. Each image becomes one row
/// whose `mask` is its label PNG, unchanged. The output extension selects
/// Arrow IPC or Parquet. Returns the number of rows written.
pub async fn ade20k_to_arrow<P: AsRef<Path>>(
    root: &Path,
    output_path: P,
    options: &Ade20kToArrowOptions,
    progress: Option<Sender<Progress>>,
) -> Result<usize, Error> {
    let output_path = output_path.as_ref();
    if options.max_workers == 0 {
        return Err(Error::InvalidParameters(
            "max_workers must be at least 1".into(),
        ));
    }
    let splits: Vec<(&str, &str)> = match options.group.as_deref() {
        None => SPLITS.to_vec(),
        Some(g) => vec![
            SPLITS
                .into_iter()
                .find(|(_, group)| *group == g)
                .ok_or_else(|| {
                    Error::InvalidParameters(format!(
                        "Unknown ADE20K group '{g}'; use train or val"
                    ))
                })?,
        ],
    };

    let classes = read_object_info(&root.join("objectInfo150.txt"))?;
    let mut work: Vec<(Ade20kPair, &str)> = Vec::new();
    for (split_dir, group) in splits {
        let pairs = list_split_pairs(root, split_dir)?;
        work.extend(pairs.into_iter().map(|p| (p, group)));
    }

    let total = work.len();
    if let Some(p) = &progress {
        let _ = p
            .send(Progress {
                current: 0,
                total,
                status: None,
            })
            .await;
    }

    let sem = Arc::new(Semaphore::new(options.max_workers));
    let counter = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::with_capacity(total);
    for (pair, group) in &work {
        let (sem, counter, progress) = (sem.clone(), counter.clone(), progress.clone());
        let (pair, group) = (pair.clone(), group.to_string());
        tasks.push(tokio::task::spawn(async move {
            let _permit = sem.acquire().await.map_err(Error::SemaphoreError)?;
            let sample =
                tokio::task::spawn_blocking(move || pair_to_sample(&pair, &group)).await??;
            let done = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if let Some(p) = &progress {
                let _ = p
                    .send(Progress {
                        current: done,
                        total,
                        status: None,
                    })
                    .await;
            }
            Ok::<_, Error>(sample)
        }));
    }
    let mut samples = Vec::with_capacity(total);
    for task in tasks {
        samples.push(task.await??);
    }

    let mut df = crate::samples_dataframe(&samples)?;
    write_dataset(&mut df, output_path, build_metadata(&classes))?;

    if options.stage_images {
        let files: Vec<(PathBuf, PathBuf)> = work
            .iter()
            .zip(&samples)
            .map(|((pair, _), sample)| {
                let name = sample.image_name.clone().unwrap_or_default();
                (pair.image.clone(), PathBuf::from(name))
            })
            .collect();
        let report = stage_files(output_path, &files, options.link_images);
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

    Ok(samples.len())
}
