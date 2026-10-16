// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

use crate::{Error, MaskData};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Source split directory and the EdgeFirst group it becomes.
pub const SPLITS: [(&str, &str); 2] = [("training", "train"), ("validation", "val")];

/// One class from `objectInfo150.txt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ade20kClass {
    /// ADE20K class id, which is also the class's pixel value.
    pub id: u32,
    /// First listed name, used as the EdgeFirst label.
    pub name: String,
    /// Every listed name, in source order (the first equals `name`).
    pub synonyms: Vec<String>,
}

/// An image and its label PNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ade20kPair {
    /// File name without extension, shared by the image and its label.
    pub stem: String,
    pub image: PathBuf,
    pub label: PathBuf,
}

/// Reads `objectInfo150.txt`: a tab-separated header line, then one line per
/// class with the id in the first column and comma-separated names in the
/// fifth. Ids must run 1, 2, … in order, because the id is the pixel value.
pub fn read_object_info(path: &Path) -> Result<Vec<Ade20kClass>, Error> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Error::InvalidParameters(format!("Cannot read {}: {e}", path.display())))?;
    let invalid = |line: usize, why: &str| {
        Error::InvalidParameters(format!("{} line {line}: {why}", path.display()))
    };
    let mut classes = Vec::new();
    for (n, line) in text.lines().enumerate().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 5 {
            return Err(invalid(n + 1, "expected 5 tab-separated columns"));
        }
        let id: u32 = fields[0]
            .trim()
            .parse()
            .map_err(|_| invalid(n + 1, "class id is not an integer"))?;
        if id as usize != classes.len() + 1 {
            return Err(invalid(
                n + 1,
                &format!("class id {id}, expected {}", classes.len() + 1),
            ));
        }
        let synonyms: Vec<String> = fields[4]
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        let name = synonyms
            .first()
            .cloned()
            .ok_or_else(|| invalid(n + 1, "class has no name"))?;
        classes.push(Ade20kClass { id, name, synonyms });
    }
    if classes.is_empty() {
        return Err(Error::InvalidParameters(format!(
            "{} lists no classes",
            path.display()
        )));
    }
    Ok(classes)
}

/// Lists `root/images/<split_dir>/*.jpg` and `root/annotations/<split_dir>/*.png`
/// as pairs sorted by stem. Every image needs a label and every label an
/// image.
pub fn list_split_pairs(root: &Path, split_dir: &str) -> Result<Vec<Ade20kPair>, Error> {
    let images = files_by_stem(&root.join("images").join(split_dir), "jpg")?;
    let mut labels = files_by_stem(&root.join("annotations").join(split_dir), "png")?;
    let mut pairs = Vec::with_capacity(images.len());
    for (stem, image) in images {
        let label = labels.remove(&stem).ok_or_else(|| {
            Error::InvalidParameters(format!("ADE20K image {stem} has no label PNG"))
        })?;
        pairs.push(Ade20kPair { stem, image, label });
    }
    if let Some(stem) = labels.into_keys().next() {
        return Err(Error::InvalidParameters(format!(
            "ADE20K label {stem}.png has no image"
        )));
    }
    Ok(pairs)
}

fn files_by_stem(dir: &Path, extension: &str) -> Result<BTreeMap<String, PathBuf>, Error> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        Error::InvalidParameters(format!(
            "Cannot read ADE20K directory {}: {e}",
            dir.display()
        ))
    })?;
    let mut files = BTreeMap::new();
    for entry in entries {
        let path = entry?.path();
        let matches = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(extension));
        if let (true, Some(stem)) = (matches, path.file_stem().and_then(|s| s.to_str())) {
            files.insert(stem.to_owned(), path.clone());
        }
    }
    Ok(files)
}

/// Reads a label PNG unchanged. It must be 8-bit grayscale with no `oFFs`
/// chunk, so every pixel is a class id on the image's own grid.
pub fn read_label_map(path: &Path) -> Result<MaskData, Error> {
    let invalid =
        |why: String| Error::InvalidParameters(format!("ADE20K label {}: {why}", path.display()));
    let bytes = std::fs::read(path).map_err(|e| invalid(e.to_string()))?;
    let mask = MaskData::from_png_checked(bytes).map_err(|e| invalid(e.to_string()))?;
    if mask.bit_depth() != 8 {
        return Err(invalid(format!(
            "expected an 8-bit PNG, got {}-bit",
            mask.bit_depth()
        )));
    }
    if mask.offset().is_some() {
        return Err(invalid(
            "has an oFFs offset; label maps cover the whole image".into(),
        ));
    }
    Ok(mask)
}
