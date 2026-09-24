// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

//! VisDrone2019 annotation text-file parsing. No Polars dependency.

use std::path::Path;

use crate::Error;

/// VisDrone2019 object categories, indexed by `object_category`.
///
/// Converted rows do not use these indices: `label_index` follows
/// [`CLASS_CATEGORIES`] (`object_category - 1`).
pub const CATEGORIES: [&str; 12] = [
    "ignored regions",
    "pedestrian",
    "people",
    "bicycle",
    "car",
    "van",
    "truck",
    "tricycle",
    "awning-tricycle",
    "bus",
    "motor",
    "others",
];

/// The ten object classes, indexed by `label_index` (`object_category - 1`).
pub const CLASS_CATEGORIES: [&str; 10] = [
    "pedestrian",
    "people",
    "bicycle",
    "car",
    "van",
    "truck",
    "tricycle",
    "awning-tricycle",
    "bus",
    "motor",
];

/// Category name for a VisDrone `object_category` id, or `None` when out of range.
pub fn category_name(index: u8) -> Option<&'static str> {
    CATEGORIES.get(index as usize).copied()
}

/// One DET box, or the box part of one VID row. Pixel units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisDroneBox {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
    /// 1 = counts in evaluation, 0 = ignored. Always 0 for categories 0 and 11.
    pub score: u8,
    pub category: u8,
    /// 0 = none, 1 = 1..50%.
    pub truncation: u8,
    /// 0 = none, 1 = 1..50%, 2 = over 50%.
    pub occlusion: u8,
}

/// One VID row: a box on one frame of a sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisDroneVidRow {
    /// 1-based frame index, equal to the numeric frame file name.
    pub frame: u32,
    /// Track id within the sequence. 0 is a valid id.
    pub target: u32,
    pub bbox: VisDroneBox,
}

/// Which VisDrone track a split directory belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitKind {
    /// `annotations/*.txt` + `images/*.jpg`
    Det,
    /// `annotations/<seq>.txt` + `sequences/<seq>/*.jpg`
    Vid,
}

/// Split a VisDrone line into integer fields, tolerating CRLF and a trailing
/// comma. Returns `None` for a blank line.
fn split_fields(line: &str, expected: usize) -> Result<Option<Vec<u32>>, Error> {
    let trimmed = line.trim().trim_end_matches(',');
    if trimmed.is_empty() {
        return Ok(None);
    }
    let fields: Result<Vec<u32>, _> = trimmed
        .split(',')
        .map(|f| f.trim().parse::<u32>())
        .collect();
    let fields = fields.map_err(|e| {
        Error::InvalidParameters(format!("VisDrone line {line:?}: non-integer field ({e})"))
    })?;
    if fields.len() != expected {
        return Err(Error::InvalidParameters(format!(
            "VisDrone line {line:?}: expected {expected} fields, found {}",
            fields.len()
        )));
    }
    Ok(Some(fields))
}

fn bbox_from_fields(f: &[u32], line: &str) -> Result<VisDroneBox, Error> {
    let small = |v: u32, what: &str, max: u32| -> Result<u8, Error> {
        if v > max {
            return Err(Error::InvalidParameters(format!(
                "VisDrone line {line:?}: {what} {v} exceeds {max}"
            )));
        }
        Ok(v as u8)
    };
    let category = small(f[5], "object_category", 11)?;
    Ok(VisDroneBox {
        left: f[0],
        top: f[1],
        width: f[2],
        height: f[3],
        score: small(f[4], "score", 1)?,
        category,
        truncation: small(f[6], "truncation", 1)?,
        occlusion: small(f[7], "occlusion", 2)?,
    })
}

/// Parse one DET annotation line. `Ok(None)` for a blank line.
pub fn parse_det_line(line: &str) -> Result<Option<VisDroneBox>, Error> {
    match split_fields(line, 8)? {
        None => Ok(None),
        Some(f) => Ok(Some(bbox_from_fields(&f, line)?)),
    }
}

/// Parse one VID annotation line. `Ok(None)` for a blank line.
pub fn parse_vid_line(line: &str) -> Result<Option<VisDroneVidRow>, Error> {
    match split_fields(line, 10)? {
        None => Ok(None),
        Some(f) => Ok(Some(VisDroneVidRow {
            frame: f[0],
            target: f[1],
            bbox: bbox_from_fields(&f[2..], line)?,
        })),
    }
}

/// Read every box in a DET annotation file. Zero-area boxes are kept; the
/// caller decides how to treat them.
pub fn read_det_annotations(path: &Path) -> Result<Vec<VisDroneBox>, Error> {
    let text = std::fs::read_to_string(path)?;
    text.lines()
        .filter_map(|l| parse_det_line(l).transpose())
        .collect()
}

/// Read every row in a VID annotation file, in file order.
pub fn read_vid_annotations(path: &Path) -> Result<Vec<VisDroneVidRow>, Error> {
    let text = std::fs::read_to_string(path)?;
    text.lines()
        .filter_map(|l| parse_vid_line(l).transpose())
        .collect()
}

/// Detect whether `dir` is a DET split (`images/`) or a VID split
/// (`sequences/`). Both need an `annotations/` directory.
pub fn detect_split_kind(dir: &Path) -> Result<SplitKind, Error> {
    if !dir.join("annotations").is_dir() {
        return Err(Error::InvalidParameters(format!(
            "{} is not a VisDrone split: missing annotations/ directory",
            dir.display()
        )));
    }
    if dir.join("images").is_dir() {
        Ok(SplitKind::Det)
    } else if dir.join("sequences").is_dir() {
        Ok(SplitKind::Vid)
    } else {
        Err(Error::InvalidParameters(format!(
            "{} is not a VisDrone split: expected images/ (DET) or sequences/ (VID)",
            dir.display()
        )))
    }
}

/// Infer the dataset group from a VisDrone split directory name such as
/// `VisDrone2019-DET-train` or `VisDrone2019-VID-test-dev`.
pub fn infer_group_from_dir_name(dir: &Path) -> Option<String> {
    let name = dir.file_name()?.to_str()?;
    for split in ["test-challenge", "test-dev", "train", "val"] {
        if name.ends_with(&format!("-{split}")) {
            return Some(split.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn category_table_matches_visdrone_ids() {
        assert_eq!(category_name(0), Some("ignored regions"));
        assert_eq!(category_name(1), Some("pedestrian"));
        assert_eq!(category_name(8), Some("awning-tricycle"));
        assert_eq!(category_name(11), Some("others"));
        assert_eq!(category_name(12), None);
    }

    #[test]
    fn det_line_parses_eight_fields() {
        let b = parse_det_line("871,572,54,92,1,4,0,1").unwrap().unwrap();
        assert_eq!((b.left, b.top, b.width, b.height), (871, 572, 54, 92));
        assert_eq!(
            (b.score, b.category, b.truncation, b.occlusion),
            (1, 4, 0, 1)
        );
    }

    #[test]
    fn det_line_tolerates_crlf_and_trailing_comma() {
        let b = parse_det_line("718,1063,23,14,1,1,1,0,\r")
            .unwrap()
            .unwrap();
        assert_eq!(b.category, 1);
        assert_eq!(b.truncation, 1);
    }

    #[test]
    fn det_line_blank_is_none() {
        assert!(parse_det_line("").unwrap().is_none());
        assert!(parse_det_line("  \r\n").unwrap().is_none());
    }

    #[test]
    fn det_line_rejects_wrong_field_count() {
        let err = parse_det_line("1,2,3").unwrap_err();
        assert!(matches!(err, Error::InvalidParameters(_)), "{err:?}");
    }

    #[test]
    fn det_line_rejects_unknown_category() {
        assert!(parse_det_line("1,2,3,4,1,12,0,0").is_err());
    }

    #[test]
    fn vid_line_parses_ten_fields() {
        let r = parse_vid_line("102,0,38,666,71,88,1,1,1,0")
            .unwrap()
            .unwrap();
        assert_eq!(r.frame, 102);
        assert_eq!(r.target, 0);
        assert_eq!(r.bbox.left, 38);
        assert_eq!(r.bbox.category, 1);
    }

    #[test]
    fn det_file_reader_keeps_zero_area_boxes() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "1,1,0,5,1,4,0,0\r\n2,2,10,10,0,0,0,0\r\n").unwrap();
        let boxes = read_det_annotations(&path).unwrap();
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0].width, 0);
        assert_eq!(boxes[1].category, 0);
    }

    #[test]
    fn split_kind_detection() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("annotations")).unwrap();
        assert!(detect_split_kind(dir.path()).is_err());
        std::fs::create_dir_all(dir.path().join("images")).unwrap();
        assert_eq!(detect_split_kind(dir.path()).unwrap(), SplitKind::Det);
        std::fs::remove_dir(dir.path().join("images")).unwrap();
        std::fs::create_dir_all(dir.path().join("sequences")).unwrap();
        assert_eq!(detect_split_kind(dir.path()).unwrap(), SplitKind::Vid);
    }

    #[test]
    fn group_inference_from_directory_name() {
        assert_eq!(
            infer_group_from_dir_name(Path::new("VisDrone2019-DET-train")).as_deref(),
            Some("train")
        );
        assert_eq!(
            infer_group_from_dir_name(Path::new("/x/VisDrone2019-VID-val")).as_deref(),
            Some("val")
        );
        assert_eq!(
            infer_group_from_dir_name(Path::new("VisDrone2019-DET-test-dev")).as_deref(),
            Some("test-dev")
        );
        assert_eq!(
            infer_group_from_dir_name(Path::new("VisDrone2019-DET-test-challenge")).as_deref(),
            Some("test-challenge")
        );
        assert_eq!(infer_group_from_dir_name(Path::new("testdev")), None);
    }
}
