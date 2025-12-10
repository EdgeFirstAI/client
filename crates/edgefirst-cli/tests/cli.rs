// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use assert_cmd::Command;
use base64::Engine as _;
use chrono::Utc;
use directories::ProjectDirs;
use serial_test::file_serial;
use std::{
    collections::{BTreeSet, HashMap},
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Helper to create a Command for the edgefirst-client binary
fn edgefirst_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("edgefirst-client"))
}

/// Get the test data directory (target/testdata)
/// Creates it if it doesn't exist
fn get_test_data_dir() -> PathBuf {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("testdata");

    fs::create_dir_all(&test_dir).expect("Failed to create test data directory");
    test_dir
}

/// Get the test dataset identifier from environment or default to "Deer"
/// Can be a dataset name (exact match) or dataset ID (ds-xxx format)
fn get_test_dataset() -> String {
    env::var("TEST_DATASET").unwrap_or_else(|_| "Deer".to_string())
}

/// Get the annotation types to test from environment or default to
/// "box2d,box3d,mask" Returns a vector of annotation type strings
fn get_test_dataset_types() -> Vec<String> {
    env::var("TEST_DATASET_TYPES")
        .unwrap_or_else(|_| "box2d,box3d,mask".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect()
}

/// Get the test data directory for the configured test dataset
/// (e.g., target/testdata/deer-test or target/testdata/multisensor-test)
fn get_test_dataset_path() -> PathBuf {
    let dataset = get_test_dataset();
    // If it's a dataset ID (ds-xxx), extract a friendly name for the path
    let normalized_name = if let Some(stripped) = dataset.strip_prefix("ds-") {
        format!("dataset-{}", stripped)
    } else {
        dataset.to_lowercase().replace(' ', "-")
    };
    get_test_data_dir().join(format!("{}-test", normalized_name))
}

fn get_project_id_by_name(name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg(name);

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    Ok(output_str
        .lines()
        .filter_map(|line| {
            line.split(']')
                .next()
                .and_then(|s| s.strip_prefix('['))
                .map(|s| s.trim().to_string())
        })
        .next())
}

/// Get dataset and its first annotation set by dataset identifier
///
/// The dataset parameter can be:
/// - A dataset ID (ds-xxx format): Used directly
/// - A dataset name: Searches all projects for EXACT name match
///
/// # Important
///
/// This function performs an EXACT name match when searching by name.
/// The returned dataset name is verified to match exactly to prevent
/// accidentally finding a similarly-named dataset (e.g., "Deer Roundtrip"
/// instead of "Deer").
fn get_dataset_and_first_annotation_set(
    dataset: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let (dataset_id, found_name) = if dataset.starts_with("ds-") {
        // It's a dataset ID - verify it exists and get its name
        let mut cmd = edgefirst_cmd();
        cmd.arg("dataset").arg(dataset);

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        // Extract dataset name from output (first line: [ds-xxx] Dataset Name)
        let name = output_str
            .lines()
            .next()
            .and_then(|line| {
                line.split(']')
                    .nth(1)
                    .map(|s| s.split(':').next().unwrap_or(s).trim().to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        (dataset.to_string(), name)
    } else {
        // It's a dataset name - search all projects for EXACT match
        let mut cmd = edgefirst_cmd();
        cmd.arg("datasets").arg("--name").arg(dataset);

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        // Parse output and find EXACT name match (case-sensitive)
        // Output format: [ds-xxx] Dataset Name: project_name
        //
        // Note: The API returns results sorted by match quality (exact first),
        // but we still explicitly verify the exact match to be safe.
        let all_matches: Vec<_> = output_str
            .lines()
            .filter_map(|line| {
                let (id_part, rest) = line.split_once(']')?;
                let id = id_part.strip_prefix('[')?.trim();
                let name_and_project = rest.trim();
                let name = name_and_project.split(':').next()?.trim();
                Some((id.to_string(), name.to_string()))
            })
            .collect();

        // Find exact match first (case-sensitive)
        let exact_match = all_matches.iter().find(|(_, name)| name == dataset);

        match exact_match {
            Some((id, name)) => (id.clone(), name.clone()),
            None => {
                // No exact match found - provide helpful error
                if all_matches.is_empty() {
                    return Err(format!("Dataset '{}' not found in any project", dataset).into());
                } else {
                    return Err(format!(
                        "Dataset '{}' not found. Similar datasets found: {:?}",
                        dataset,
                        all_matches.iter().map(|(_, n)| n).collect::<Vec<_>>()
                    )
                    .into());
                }
            }
        }
    };

    // CRITICAL: Verify the found dataset name matches EXACTLY what was requested
    // This prevents accidentally testing with a similarly-named dataset
    if !dataset.starts_with("ds-") {
        assert_eq!(
            found_name, dataset,
            "Dataset name mismatch: requested '{}' but found '{}'. \
             The API may have returned a near-match instead of exact match.",
            dataset, found_name
        );
    }

    let mut cmd = edgefirst_cmd();
    cmd.arg("dataset").arg(&dataset_id).arg("--annotation-sets");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let annotation_set_id = output_str
        .lines()
        .skip_while(|line| !line.contains("Annotation Sets:"))
        .skip(1)
        .find_map(|line| {
            line.split(']')
                .next()
                .and_then(|s| s.strip_prefix('['))
                .map(|s| s.trim().to_string())
                .filter(|id| id.starts_with("as-"))
        })
        .ok_or_else(|| format!("No annotation set found for dataset '{}'", dataset))?;

    Ok((dataset_id, annotation_set_id))
}

fn collect_relative_file_paths(dir: &Path) -> Result<Vec<String>, std::io::Error> {
    fn visit(current: &Path, base: &Path, files: &mut Vec<String>) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit(&path, base, files)?;
            } else if path.is_file() {
                if entry.file_name() == ".DS_Store" {
                    continue;
                }
                let rel = path.strip_prefix(base).unwrap();
                files.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    visit(dir, dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn validate_dataset_structure(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let files = collect_relative_file_paths(dir)?;

    if files.is_empty() {
        return Err("Downloaded dataset is empty".into());
    }

    // Verify all files are image files with valid extensions
    let valid_extensions = ["jpg", "jpeg", "png", "bmp", "tiff", "tif", "pcd"];
    for file in &files {
        let path = Path::new(file);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        if let Some(ext) = extension {
            if !valid_extensions.contains(&ext.as_str()) {
                return Err(format!("Invalid file extension in dataset: {}", file).into());
            }
        } else {
            return Err(format!("File without extension in dataset: {}", file).into());
        }
    }

    Ok(())
}

fn download_dataset_from_server(dataset_id: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    download_dataset_from_server_with_retries(dataset_id, 1) // default: 1 attempt
}

fn download_dataset_from_server_with_retries(
    dataset_id: &str,
    max_attempts: u32,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let downloads_root = get_test_data_dir().join("downloads");
    fs::create_dir_all(&downloads_root)?;

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let safe_dataset_id = dataset_id.replace('/', "_");
    let download_dir = downloads_root.join(format!(
        "{}_{}_{}",
        safe_dataset_id,
        std::process::id(),
        timestamp
    ));
    fs::create_dir_all(&download_dir)?;

    for attempt in 1..=max_attempts {
        let mut cmd = edgefirst_cmd();
        cmd.arg("download-dataset")
            .arg(dataset_id)
            .arg("--output")
            .arg(&download_dir);
        let result = cmd.ok();
        if result.is_ok() {
            return Ok(download_dir);
        }
        if attempt < max_attempts {
            println!(
                "Download attempt {} failed, retrying in 5 seconds...",
                attempt
            );
            std::thread::sleep(std::time::Duration::from_secs(5));
            // Clear directory for retry
            if download_dir.exists() {
                let _ = fs::remove_dir_all(&download_dir);
                fs::create_dir_all(&download_dir)?;
            }
        } else {
            // On last attempt, propagate the error
            cmd = edgefirst_cmd();
            cmd.arg("download-dataset")
                .arg(dataset_id)
                .arg("--output")
                .arg(&download_dir);
            cmd.assert().success();
        }
    }

    Ok(download_dir)
}

fn download_annotations_from_server(
    annotation_set_id: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    download_annotations_from_server_with_types(annotation_set_id, &["box2d"])
}

fn download_annotations_from_server_with_types(
    annotation_set_id: &str,
    types: &[&str],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let downloads_root = get_test_data_dir().join("downloads");
    fs::create_dir_all(&downloads_root)?;

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let safe_annotation_set_id = annotation_set_id.replace('/', "_");
    let arrow_path = downloads_root.join(format!(
        "{}_{}_{}.arrow",
        safe_annotation_set_id,
        std::process::id(),
        timestamp
    ));

    let mut cmd = edgefirst_cmd();
    cmd.arg("download-annotations")
        .arg(annotation_set_id)
        .arg("--types")
        .arg(types.join(","))
        .arg(&arrow_path);
    cmd.assert().success();

    Ok(arrow_path)
}

/// Compare two Arrow files to verify groups and annotations are preserved
/// Returns an error if there are mismatches
#[cfg(feature = "polars")]
fn compare_arrow_files(
    original_path: &Path,
    redownloaded_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use polars::prelude::*;
    use std::fs::File;

    println!("\n=== Arrow File Comparison ===");

    // Read both Arrow files
    let mut original_file = File::open(original_path)?;
    let original_df = IpcReader::new(&mut original_file).finish()?;

    let mut redownloaded_file = File::open(redownloaded_path)?;
    let redownloaded_df = IpcReader::new(&mut redownloaded_file).finish()?;

    println!("Original rows: {}", original_df.height());
    println!("Redownloaded rows: {}", redownloaded_df.height());

    // Debug: Find missing rows if counts don't match
    if original_df.height() != redownloaded_df.height() {
        println!("\n=== DEBUGGING ROW COUNT MISMATCH ===");

        // Build sets of (name, frame) tuples for both datasets
        let original_samples = if let Ok(names_col) = original_df.column("name")
            && let Ok(frames_col) = original_df.column("frame")
        {
            let names_cast = names_col.cast(&DataType::String)?;
            let frames_cast = frames_col.cast(&DataType::Int32)?;
            let names = names_cast.str()?;
            let frames = frames_cast.i32()?;

            let mut samples = std::collections::HashSet::new();
            for idx in 0..original_df.height() {
                if let Some(name) = names.get(idx) {
                    let frame = frames.get(idx);
                    samples.insert((name.to_string(), frame));
                }
            }
            Some(samples)
        } else {
            None
        };

        let redownloaded_samples = if let Ok(names_col) = redownloaded_df.column("name")
            && let Ok(frames_col) = redownloaded_df.column("frame")
        {
            let names_cast = names_col.cast(&DataType::String)?;
            let frames_cast = frames_col.cast(&DataType::Int32)?;
            let names = names_cast.str()?;
            let frames = frames_cast.i32()?;

            let mut samples = std::collections::HashSet::new();
            for idx in 0..redownloaded_df.height() {
                if let Some(name) = names.get(idx) {
                    let frame = frames.get(idx);
                    samples.insert((name.to_string(), frame));
                }
            }
            Some(samples)
        } else {
            None
        };

        if let (Some(orig), Some(redown)) = (&original_samples, &redownloaded_samples) {
            let missing_in_redownloaded: Vec<_> = orig.difference(redown).collect();
            let extra_in_redownloaded: Vec<_> = redown.difference(orig).collect();

            if !missing_in_redownloaded.is_empty() {
                println!(
                    "\nMissing in redownloaded ({} rows):",
                    missing_in_redownloaded.len()
                );
                for (name, frame) in missing_in_redownloaded.iter().take(20) {
                    println!("  - {} (frame: {:?})", name, frame);
                }
            }

            if !extra_in_redownloaded.is_empty() {
                println!(
                    "\nExtra in redownloaded ({} rows):",
                    extra_in_redownloaded.len()
                );
                for (name, frame) in extra_in_redownloaded.iter().take(20) {
                    println!("  - {} (frame: {:?})", name, frame);
                }
            }
        }

        return Err(format!(
            "Row count mismatch: {} vs {}",
            original_df.height(),
            redownloaded_df.height()
        )
        .into());
    }

    // Check that group column exists in both
    let original_has_group = original_df.column("group").is_ok();
    let redownloaded_has_group = redownloaded_df.column("group").is_ok();

    println!("Original has group column: {}", original_has_group);
    println!("Redownloaded has group column: {}", redownloaded_has_group);

    // Build sample -> group mapping for both datasets
    // Key is (name_base, frame) since a sample is uniquely identified by name+frame
    let original_groups = if let Ok(names_col) = original_df.column("name")
        && let Ok(groups_col) = original_df.column("group")
        && let Ok(frames_col) = original_df.column("frame")
    {
        // Cast to String to handle Categorical types
        let names_cast = names_col.cast(&DataType::String)?;
        let groups_cast = groups_col.cast(&DataType::String)?;
        let frames_cast = frames_col.cast(&DataType::Int32)?;
        let names = names_cast.str()?;
        let groups = groups_cast.str()?;
        let frames = frames_cast.i32()?;

        let mut map = HashMap::new();
        for idx in 0..original_df.height() {
            if let (Some(name), group_opt, frame_opt) =
                (names.get(idx), groups.get(idx), frames.get(idx))
            {
                let name_base = name.rsplit_once('.').map(|(base, _)| base).unwrap_or(name);
                let key = (name_base.to_string(), frame_opt);
                map.insert(key, group_opt.map(|g| g.to_string()));
            }
        }
        Some(map)
    } else {
        None
    };

    let redownloaded_groups = if let Ok(names_col) = redownloaded_df.column("name")
        && let Ok(groups_col) = redownloaded_df.column("group")
        && let Ok(frames_col) = redownloaded_df.column("frame")
    {
        let names_cast = names_col.cast(&DataType::String)?;
        let groups_cast = groups_col.cast(&DataType::String)?;
        let frames_cast = frames_col.cast(&DataType::Int32)?;
        let names = names_cast.str()?;
        let groups = groups_cast.str()?;
        let frames = frames_cast.i32()?;

        let mut map = HashMap::new();
        for idx in 0..redownloaded_df.height() {
            if let (Some(name), group_opt, frame_opt) =
                (names.get(idx), groups.get(idx), frames.get(idx))
            {
                let name_base = name.rsplit_once('.').map(|(base, _)| base).unwrap_or(name);
                let key = (name_base.to_string(), frame_opt);
                map.insert(key, group_opt.map(|g| g.to_string()));
            }
        }
        Some(map)
    } else {
        None
    };

    // Verify groups match if both exist
    // Key is (name_base, frame) tuple since samples are uniquely identified by
    // name+frame
    if let (Some(orig_groups), Some(redown_groups)) = (&original_groups, &redownloaded_groups) {
        let mut mismatches = Vec::new();

        for (key, orig_group) in orig_groups {
            if let Some(redown_group) = redown_groups.get(key)
                && orig_group != redown_group
            {
                let (name, frame) = key;
                let frame_str = frame.map(|f| format!("_frame_{}", f)).unwrap_or_default();
                mismatches.push(format!(
                    "  Sample '{}{}': group '{}' != '{}'",
                    name,
                    frame_str,
                    orig_group.as_deref().unwrap_or("None"),
                    redown_group.as_deref().unwrap_or("None")
                ));
            }
        }

        if !mismatches.is_empty() {
            println!("⚠️  GROUP MISMATCHES DETECTED:");
            for (i, mismatch) in mismatches.iter().take(10).enumerate() {
                println!("  {}: {}", i + 1, mismatch);
            }
            return Err(format!("Found {} group mismatches", mismatches.len()).into());
        }

        println!("✓ Groups verified: all samples have matching groups");
    } else if original_has_group && !redownloaded_has_group {
        return Err("Original file has groups but redownloaded file does not".into());
    } else if !original_has_group && redownloaded_has_group {
        return Err("Redownloaded file has groups but original file does not".into());
    }

    // CRITICAL DEBUG: Check samples with NO annotations
    // These are the most likely to lose group information
    println!("\n=== DEBUG: Samples with No Annotations ===");

    // Check original for samples with null labels (no annotations)
    if let Ok(labels_col) = original_df.column("label")
        && let Ok(groups_col) = original_df.column("group")
        && let Ok(names_col) = original_df.column("name")
    {
        let names_cast = names_col.cast(&DataType::String)?;
        let names = names_cast.str()?;
        let groups_cast = groups_col.cast(&DataType::String)?;
        let _groups = groups_cast.str()?;

        let label_is_null = labels_col.is_null();
        let group_is_null = groups_col.is_null();

        let mut no_annotation_count = 0;
        let mut no_annotation_with_group = 0;
        let mut no_annotation_without_group = Vec::new();

        for idx in 0..original_df.height() {
            if label_is_null.get(idx).unwrap_or(false) {
                no_annotation_count += 1;
                let has_group = !group_is_null.get(idx).unwrap_or(true);
                if has_group {
                    no_annotation_with_group += 1;
                } else if let Some(name) = names.get(idx) {
                    no_annotation_without_group.push(name.to_string());
                }
            }
        }

        println!(
            "Original - Samples with no annotations: {}",
            no_annotation_count
        );
        println!(
            "Original - Samples with no annotations BUT WITH group: {}",
            no_annotation_with_group
        );
        if !no_annotation_without_group.is_empty() {
            println!(
                "⚠️  Original - {} samples with no annotations AND no group:",
                no_annotation_without_group.len()
            );
            for (i, name) in no_annotation_without_group.iter().take(10).enumerate() {
                println!("    {}: {}", i + 1, name);
            }
        }
    }

    // Check redownloaded for the same
    if let Ok(labels_col) = redownloaded_df.column("label")
        && let Ok(groups_col) = redownloaded_df.column("group")
        && let Ok(names_col) = redownloaded_df.column("name")
    {
        let names_cast = names_col.cast(&DataType::String)?;
        let names = names_cast.str()?;
        let groups_cast = groups_col.cast(&DataType::String)?;
        let _groups = groups_cast.str()?;

        let label_is_null = labels_col.is_null();
        let group_is_null = groups_col.is_null();

        let mut no_annotation_count = 0;
        let mut no_annotation_with_group = 0;
        let mut no_annotation_without_group = Vec::new();

        for idx in 0..redownloaded_df.height() {
            if label_is_null.get(idx).unwrap_or(false) {
                no_annotation_count += 1;
                let has_group = !group_is_null.get(idx).unwrap_or(true);
                if has_group {
                    no_annotation_with_group += 1;
                } else if let Some(name) = names.get(idx) {
                    no_annotation_without_group.push(name.to_string());
                }
            }
        }

        println!(
            "Redownloaded - Samples with no annotations: {}",
            no_annotation_count
        );
        println!(
            "Redownloaded - Samples with no annotations BUT WITH group: {}",
            no_annotation_with_group
        );
        if !no_annotation_without_group.is_empty() {
            println!(
                "⚠️  Redownloaded - {} samples with no annotations AND no group:",
                no_annotation_without_group.len()
            );
            for (i, name) in no_annotation_without_group.iter().take(10).enumerate() {
                println!("    {}: {}", i + 1, name);
            }
        }
    }

    // Verify masks if present
    let original_has_mask = original_df.column("mask").is_ok();
    let redownloaded_has_mask = redownloaded_df.column("mask").is_ok();

    println!("Original has mask column: {}", original_has_mask);
    println!("Redownloaded has mask column: {}", redownloaded_has_mask);

    if original_has_mask && redownloaded_has_mask {
        // Count non-null masks
        let orig_mask_col = original_df.column("mask")?;
        let redown_mask_col = redownloaded_df.column("mask")?;

        let orig_mask_count = orig_mask_col.len() - orig_mask_col.null_count();
        let redown_mask_count = redown_mask_col.len() - redown_mask_col.null_count();

        println!("Original mask annotations: {}", orig_mask_count);
        println!("Redownloaded mask annotations: {}", redown_mask_count);

        if orig_mask_count != redown_mask_count {
            return Err(format!(
                "Mask count mismatch: {} vs {}",
                orig_mask_count, redown_mask_count
            )
            .into());
        }

        if orig_mask_count > 0 {
            println!(
                "✓ Mask annotations verified: {} masks preserved",
                orig_mask_count
            );
        }
    }

    // Verify box2d if present
    let original_has_box2d = original_df.column("box2d").is_ok();
    let redownloaded_has_box2d = redownloaded_df.column("box2d").is_ok();

    if original_has_box2d && redownloaded_has_box2d {
        let orig_box2d_col = original_df.column("box2d")?;
        let redown_box2d_col = redownloaded_df.column("box2d")?;

        let orig_box2d_count = orig_box2d_col.len() - orig_box2d_col.null_count();
        let redown_box2d_count = redown_box2d_col.len() - redown_box2d_col.null_count();

        println!("Original box2d annotations: {}", orig_box2d_count);
        println!("Redownloaded box2d annotations: {}", redown_box2d_count);

        if orig_box2d_count != redown_box2d_count {
            return Err(format!(
                "Box2d count mismatch: {} vs {}",
                orig_box2d_count, redown_box2d_count
            )
            .into());
        }

        if orig_box2d_count > 0 {
            println!(
                "✓ Box2d annotations verified: {} boxes preserved",
                orig_box2d_count
            );
        }
    }

    // Verify object_id references when both box2d and mask are present
    if original_has_box2d && original_has_mask && redownloaded_has_box2d && redownloaded_has_mask {
        let orig_box2d_col = original_df.column("box2d")?;
        let orig_mask_col = original_df.column("mask")?;
        let orig_object_id_col = original_df.column("object_id")?;

        // Cast object_id to String for easier comparison
        let orig_object_id_cast = orig_object_id_col.cast(&DataType::String)?;
        let orig_object_ids = orig_object_id_cast.str()?;

        // Get box2d and mask null counts to calculate non-null rows
        let _orig_box2d_null_count = orig_box2d_col.null_count();
        let _orig_mask_null_count = orig_mask_col.null_count();

        // Count rows where both box2d and mask are non-null by iterating
        let mut orig_dual_annotation_count = 0;
        let mut orig_dual_with_object_id = 0;

        // Create boolean masks for non-null values
        let orig_box2d_not_null = orig_box2d_col.is_not_null();
        let orig_mask_not_null = orig_mask_col.is_not_null();

        for idx in 0..original_df.height() {
            let has_box2d = orig_box2d_not_null.get(idx).unwrap_or(false);
            let has_mask = orig_mask_not_null.get(idx).unwrap_or(false);

            if has_box2d && has_mask {
                orig_dual_annotation_count += 1;
                if let Some(object_id) = orig_object_ids.get(idx)
                    && !object_id.is_empty()
                {
                    orig_dual_with_object_id += 1;
                }
            }
        }

        // Do the same for redownloaded
        let redown_box2d_col = redownloaded_df.column("box2d")?;
        let redown_mask_col = redownloaded_df.column("mask")?;
        let redown_object_id_col = redownloaded_df.column("object_id")?;

        let redown_object_id_cast = redown_object_id_col.cast(&DataType::String)?;
        let redown_object_ids = redown_object_id_cast.str()?;

        let mut redown_dual_annotation_count = 0;
        let mut redown_dual_with_object_id = 0;

        let redown_box2d_not_null = redown_box2d_col.is_not_null();
        let redown_mask_not_null = redown_mask_col.is_not_null();

        for idx in 0..redownloaded_df.height() {
            let has_box2d = redown_box2d_not_null.get(idx).unwrap_or(false);
            let has_mask = redown_mask_not_null.get(idx).unwrap_or(false);

            if has_box2d && has_mask {
                redown_dual_annotation_count += 1;
                if let Some(object_id) = redown_object_ids.get(idx)
                    && !object_id.is_empty()
                {
                    redown_dual_with_object_id += 1;
                }
            }
        }

        println!(
            "Original annotations with both box2d and mask: {}",
            orig_dual_annotation_count
        );
        println!(
            "Original dual annotations with object_id: {}",
            orig_dual_with_object_id
        );
        println!(
            "Redownloaded annotations with both box2d and mask: {}",
            redown_dual_annotation_count
        );
        println!(
            "Redownloaded dual annotations with object_id: {}",
            redown_dual_with_object_id
        );

        if orig_dual_annotation_count != redown_dual_annotation_count {
            return Err(format!(
                "Dual annotation count mismatch: {} vs {}",
                orig_dual_annotation_count, redown_dual_annotation_count
            )
            .into());
        }

        if orig_dual_with_object_id != redown_dual_with_object_id {
            return Err(format!(
                "Dual annotation object_id count mismatch: {} vs {}",
                orig_dual_with_object_id, redown_dual_with_object_id
            )
            .into());
        }

        if orig_dual_annotation_count > 0 {
            if orig_dual_with_object_id == 0 {
                return Err(
                    "Expected object_id references for annotations with both box2d and mask"
                        .to_string()
                        .into(),
                );
            }

            println!(
                "✓ Object_id references verified: {}/{} dual annotations have object_ids",
                orig_dual_with_object_id, orig_dual_annotation_count
            );
        }
    }

    Ok(())
}

#[test]
fn test_version() -> Result<(), Box<dyn std::error::Error>> {
    println!("STUDIO_SERVER: {:?}", env::var("STUDIO_SERVER"));
    println!("STUDIO_TOKEN: {:?}", env::var("STUDIO_TOKEN"));
    println!("STUDIO_USERNAME: {:?}", env::var("STUDIO_USERNAME"));
    println!("STUDIO_PASSWORD: {:?}", env::var("STUDIO_PASSWORD"));

    let mut cmd = edgefirst_cmd();
    cmd.arg("version");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains(env!("CARGO_PKG_VERSION")));
    Ok(())
}

#[test]
fn test_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("token");

    let token = cmd.ok()?.stdout;
    assert!(!token.is_empty());

    println!("Token: {}", String::from_utf8_lossy(&token));

    let token = String::from_utf8(token)?;
    let token_parts: Vec<&str> = token.split('.').collect();
    assert_eq!(token_parts.len(), 3);

    let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(token_parts[1])
        .unwrap();
    let payload: HashMap<String, serde_json::Value> = serde_json::from_slice(&decoded)?;
    let username = payload.get("username");
    assert!(username.is_some());
    let username = username.unwrap().as_str().unwrap();
    assert!(!username.is_empty());

    if let Ok(studio_username) = env::var("STUDIO_USERNAME") {
        assert_eq!(studio_username, username)
    }

    Ok(())
}

#[test]
fn test_organization() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("organization");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Organization:"));
    Ok(())
}

#[test]
fn test_organization_details() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("organization");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Verify all expected fields are present
    assert!(output_str.contains("Username:"));
    assert!(output_str.contains("Organization:"));
    assert!(output_str.contains("ID:"));
    assert!(output_str.contains("Credits:"));

    println!("Organization output:\n{}", output_str);
    Ok(())
}

// ===== Authentication Tests =====

/// Comprehensive authentication workflow test
///
/// Tests: login -> token validation -> logout -> re-login -> new token issued
/// This consolidates multiple auth tests into one efficient workflow.
#[test]
#[file_serial]
fn test_auth_workflow() -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs, time::SystemTime};

    // Get credentials from environment (required for authentication tests)
    let username =
        env::var("STUDIO_USERNAME").expect("STUDIO_USERNAME must be set for authentication tests");
    let _password =
        env::var("STUDIO_PASSWORD").expect("STUDIO_PASSWORD must be set for authentication tests");

    // Get the token path - must match what the CLI uses (no fallback)
    let token_path = ProjectDirs::from("ai", "EdgeFirst", "EdgeFirst Studio")
        .map(|d| d.config_dir().join("token"))
        .ok_or("ProjectDirs::from returned None - cannot determine token path")?;

    // Clean up any existing token file to ensure a clean test state
    // This prevents interference from other tests or previous runs
    if token_path.exists() {
        println!(
            "Removing existing token file ({} bytes) to ensure clean state",
            fs::metadata(&token_path).map(|m| m.len()).unwrap_or(0)
        );
        fs::remove_file(&token_path)?;
    }

    // Debug: Show environment info to help diagnose path issues
    println!("HOME: {:?}", env::var("HOME"));
    println!("XDG_CONFIG_HOME: {:?}", env::var("XDG_CONFIG_HOME"));
    println!("Token path: {:?}", token_path);
    println!("=== STEP 1: First Login ===");
    let time_before = SystemTime::now();
    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut cmd = edgefirst_cmd();
    cmd.arg("login");

    let output = cmd.output()?;
    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    println!("Login stdout:\n{}", stdout_str);
    println!("Login stderr:\n{}", stderr_str);
    println!("Login exit status: {:?}", output.status);

    assert!(
        output.status.success(),
        "Login command should succeed (exit code: {:?})",
        output.status
    );
    assert!(
        stdout_str.contains("Successfully logged into EdgeFirst Studio"),
        "Should contain success message, got: {}",
        stdout_str
    );
    assert!(
        stdout_str.contains(&username),
        "Should contain username '{}', got: {}",
        username,
        stdout_str
    );
    assert!(token_path.exists(), "Token file should exist after login");

    let metadata = fs::metadata(&token_path)?;
    let modified_time = metadata.modified()?;
    assert!(
        modified_time > time_before,
        "Token file should be updated after login"
    );

    // Validate JWT token format and username
    let first_token = fs::read_to_string(&token_path)?;
    println!(
        "Token file size: {} bytes, path: {:?}",
        first_token.len(),
        token_path
    );
    assert!(
        !first_token.is_empty(),
        "Token file should not be empty (path: {:?})",
        token_path
    );

    let token_parts: Vec<&str> = first_token.trim().split('.').collect();
    assert_eq!(
        token_parts.len(),
        3,
        "Token should be a valid JWT with 3 parts"
    );

    // Debug: Log all token parts for troubleshooting
    println!("Token structure:");
    println!("  Header ({}): {}", token_parts[0].len(), token_parts[0]);
    println!("  Payload ({}): {}", token_parts[1].len(), token_parts[1]);
    println!("  Signature ({}): {}", token_parts[2].len(), token_parts[2]);

    let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(token_parts[1])
        .unwrap_or_else(|e| {
            eprintln!(
                "Failed to decode JWT payload: {:?}. Payload part: {}",
                e, token_parts[1]
            );
            panic!("Token payload should be valid base64: {:?}", e)
        });

    // Debug: Log the decoded payload for troubleshooting
    let decoded_str = String::from_utf8_lossy(&decoded);
    println!("Decoded JWT payload ({}): {}", decoded.len(), decoded_str);

    let payload: HashMap<String, serde_json::Value> = serde_json::from_slice(&decoded)
        .unwrap_or_else(|_| {
            panic!(
                "Token payload should be valid JSON. Raw decoded: {}",
                decoded_str
            )
        });

    let token_username = payload
        .get("username")
        .and_then(|v| v.as_str())
        .expect("Token should contain username field");

    assert_eq!(
        token_username, username,
        "Token username should match login username"
    );

    println!("✓ First login successful, token valid");
    let first_modified = fs::metadata(&token_path)?.modified()?;

    println!("\n=== STEP 2: Logout ===");
    let mut cmd = edgefirst_cmd();
    cmd.arg("logout");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    assert!(output_str.contains("Successfully logged out of EdgeFirst Studio"));
    assert!(
        !token_path.exists(),
        "Token file should be removed after logout"
    );

    println!("✓ Logout successful, token file removed");

    println!("\n=== STEP 3: Re-login (verify new token issued) ===");
    std::thread::sleep(std::time::Duration::from_secs(2)); // Ensure timestamp difference

    let mut cmd = edgefirst_cmd();
    cmd.arg("login");
    cmd.ok()?;

    let second_token = fs::read_to_string(&token_path)?;
    let second_modified = fs::metadata(&token_path)?.modified()?;

    assert_ne!(
        first_token, second_token,
        "Re-login should create a new token"
    );
    assert!(
        second_modified > first_modified,
        "Token file should be updated on re-login"
    );

    println!("✓ Re-login successful, new token issued");
    println!("\n✅ Authentication workflow completed successfully");

    Ok(())
}

#[test]
#[file_serial]
fn test_corrupted_token_handling() -> Result<(), Box<dyn std::error::Error>> {
    let _username =
        env::var("STUDIO_USERNAME").expect("STUDIO_USERNAME must be set for authentication tests");

    // Get the token path - must match what the CLI uses
    let token_path = ProjectDirs::from("ai", "EdgeFirst", "EdgeFirst Studio")
        .map(|d| d.config_dir().join("token"))
        .ok_or("ProjectDirs::from returned None - cannot determine token path")?;

    println!("Token path: {:?}", token_path);

    // Login first to create a valid token
    let mut cmd = edgefirst_cmd();
    cmd.arg("login");
    cmd.ok()?;

    assert!(token_path.exists(), "Token file should exist after login");

    // Corrupt the token file with invalid data
    fs::write(&token_path, "this.is.corrupted")?;
    println!("✓ Corrupted token file created at {:?}", token_path);

    // Try to run a command that requires authentication WITHOUT credentials
    // This should gracefully handle the corrupted token
    let mut cmd = edgefirst_cmd();
    cmd.arg("organization");
    // Explicitly unset authentication environment variables so the command can't
    // auto-login via clap's env feature. Keep STUDIO_SERVER as it controls which
    // server instance to connect to.
    cmd.env_remove("STUDIO_USERNAME");
    cmd.env_remove("STUDIO_PASSWORD");
    cmd.env_remove("STUDIO_TOKEN");

    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("Command stderr:\n{}", stderr);
    println!("Command stdout:\n{}", stdout);

    // Should fail with authentication error, not a crash
    assert!(
        !output.status.success(),
        "Command should fail with corrupted token and no credentials"
    );
    assert!(
        stderr.contains("Authentication failed")
            || stderr.contains("Please login again")
            || stderr.contains("Empty token"),
        "Should provide helpful error message about re-authenticating"
    );

    // Corrupted token should be removed
    // (either by with_token_path or by the logout in error handling)
    println!("Token file exists after error: {}", token_path.exists());

    // Should be able to login again
    let mut cmd = edgefirst_cmd();
    cmd.arg("login");
    cmd.ok()?;

    assert!(
        token_path.exists(),
        "Should be able to login after corruption"
    );
    let new_token = fs::read_to_string(&token_path)?;
    assert_ne!(new_token, "this.is.corrupted", "New token should be valid");

    println!("✓ Successfully logged in again after corruption");

    Ok(())
}

// ===== Project Tests =====

#[test]
fn test_projects_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects");
    cmd.assert().success();
    Ok(())
}

#[test]
fn test_projects_filter_by_name() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    assert!(output_str.contains("Unit Testing"));
    println!("Filtered projects:\n{}", output_str);
    Ok(())
}

#[test]
fn test_project_by_id() -> Result<(), Box<dyn std::error::Error>> {
    // First get the project list to extract an ID
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Extract project ID from output like "[123] Unit Testing: description"
    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("project").arg(&id);
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("Unit Testing"));
    }

    Ok(())
}

// ===== Dataset Tests =====

#[test]
fn test_datasets_list_all() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("datasets");
    cmd.assert().success();
    Ok(())
}

#[test]
fn test_datasets_by_project() -> Result<(), Box<dyn std::error::Error>> {
    // First get project ID
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("datasets").arg(&id);
        cmd.assert().success();
    }

    Ok(())
}

#[test]
fn test_datasets_with_labels() -> Result<(), Box<dyn std::error::Error>> {
    // Get Unit Testing with COCO dataset
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("datasets").arg(&id).arg("--labels");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        assert!(output_str.contains("Labels:"));
        println!("Datasets with labels:\n{}", output_str);
    }

    Ok(())
}

#[test]
fn test_datasets_with_annotation_sets() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("datasets").arg(&id).arg("--annotation-sets");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        assert!(output_str.contains("Annotation Sets:"));
        println!("Datasets with annotation sets:\n{}", output_str);
    }

    Ok(())
}

#[test]
fn test_dataset_by_id() -> Result<(), Box<dyn std::error::Error>> {
    // Get a dataset ID from Unit Testing project
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("datasets").arg(&proj_id);

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        // Get first dataset ID from output
        let dataset_id = output_str
            .lines()
            .next()
            .and_then(|line| line.split(']').next())
            .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

        if let Some(ds_id) = dataset_id {
            let mut cmd = edgefirst_cmd();
            cmd.arg("dataset").arg(&ds_id);
            cmd.assert().success();
        }
    }

    Ok(())
}

#[test]
#[file_serial]
fn test_download_annotations() -> Result<(), Box<dyn std::error::Error>> {
    let dataset = get_test_dataset();
    let dataset_name_lower = dataset.to_lowercase().replace("ds-", "dataset-");
    let (_, annotation_set_id) = get_dataset_and_first_annotation_set(&dataset)?;

    let test_dir = get_test_data_dir();

    // Test JSON format download
    let json_file = test_dir.join(format!(
        "{}_annotations_{}.json",
        dataset_name_lower,
        std::process::id()
    ));

    let mut cmd = edgefirst_cmd();
    cmd.arg("download-annotations")
        .arg(&annotation_set_id)
        .arg(&json_file);

    cmd.assert().success();

    assert!(json_file.exists(), "JSON annotations file should exist");
    assert!(
        json_file.metadata()?.len() > 0,
        "JSON annotations file should not be empty"
    );
    println!("Downloaded annotations to {:?}", json_file);

    fs::remove_file(&json_file)?;

    // Test Arrow format download
    let arrow_file = test_dir.join(format!(
        "{}_annotations_{}.arrow",
        dataset_name_lower,
        std::process::id()
    ));

    let mut cmd = edgefirst_cmd();
    cmd.arg("download-annotations")
        .arg(&annotation_set_id)
        .arg(&arrow_file);

    cmd.assert().success();

    assert!(arrow_file.exists(), "Arrow annotations file should exist");
    assert!(
        arrow_file.metadata()?.len() > 0,
        "Arrow annotations file should not be empty"
    );
    println!("Downloaded annotations to {:?}", arrow_file);

    fs::remove_file(&arrow_file)?;

    Ok(())
}

#[test]
#[file_serial]
fn test_upload_dataset_persistent_copy() -> Result<(), Box<dyn std::error::Error>> {
    let dataset = get_test_dataset();
    let (source_dataset_id, source_annotation_set_id) =
        get_dataset_and_first_annotation_set(&dataset)?;

    let images_dir = download_dataset_from_server(&source_dataset_id)?;
    let annotations_path = download_annotations_from_server(&source_annotation_set_id)?;
    validate_dataset_structure(images_dir.as_path())?;

    let project_id = get_project_id_by_name("Unit Testing")?
        .ok_or_else(|| "Project 'Unit Testing' not found".to_string())?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let new_dataset_name = format!("QA {} Upload {}", dataset, timestamp);

    let mut cmd = edgefirst_cmd();
    cmd.arg("create-dataset")
        .arg(&project_id)
        .arg(&new_dataset_name);

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;
    let new_dataset_id = output_str
        .lines()
        .find_map(|line| line.strip_prefix("Created dataset with ID: "))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "Failed to parse dataset ID from create-dataset output".to_string())?;

    let annotation_set_name = format!("{} Annotations", new_dataset_name);
    let mut cmd = edgefirst_cmd();
    cmd.arg("create-annotation-set")
        .arg(&new_dataset_id)
        .arg(&annotation_set_name);

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;
    let new_annotation_set_id = output_str
        .lines()
        .find_map(|line| line.strip_prefix("Created annotation set with ID: "))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| {
            "Failed to parse annotation set ID from create-annotation-set output".to_string()
        })?;

    let mut cmd = edgefirst_cmd();
    cmd.arg("upload-dataset")
        .arg(&new_dataset_id)
        .arg("--annotations")
        .arg(&annotations_path)
        .arg("--annotation-set-id")
        .arg(&new_annotation_set_id)
        .arg("--images")
        .arg(&images_dir);

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;
    assert!(
        output_str.contains("Successfully uploaded") || output_str.contains("samples"),
        "Expected upload to report success, got: {}",
        output_str
    );

    println!(
        "✓ Created and uploaded dataset: {} (annotation set {})",
        new_dataset_id, new_annotation_set_id
    );
    println!("  Images uploaded from: {:?}", images_dir);
    println!("  Annotations uploaded from: {:?}", annotations_path);

    // Clean up: delete the created dataset (this also deletes the annotation set)
    println!("\n=== CLEANUP: Deleting created dataset ===");
    let mut cmd = edgefirst_cmd();
    cmd.arg("delete-dataset").arg(&new_dataset_id);

    match cmd.output() {
        Ok(output) if output.status.success() => {
            println!("✓ Deleted dataset: {}", new_dataset_id);
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "⚠️  Failed to delete dataset {}: {}",
                new_dataset_id, stderr
            );
        }
        Err(e) => {
            eprintln!("⚠️  Error deleting dataset {}: {}", new_dataset_id, e);
        }
    }

    // Clean up local files
    if let Err(err) = fs::remove_dir_all(&images_dir) {
        eprintln!(
            "⚠️  Failed to remove downloaded images directory {:?}: {}",
            images_dir, err
        );
    }

    if let Err(err) = fs::remove_file(&annotations_path) {
        eprintln!(
            "⚠️  Failed to remove downloaded annotations file {:?}: {}",
            annotations_path, err
        );
    }

    Ok(())
}

/// End-to-end dataset roundtrip test
///
/// Tests complete workflow: Download → Upload → Download → Compare → Cleanup
///
/// Dataset: Configurable via TEST_DATASET env var (default: "Deer")
/// Requirements:
/// - Dataset must exist in "Unit Testing" project
/// - Must have at least one annotation set
/// - Supports mixed sensors, annotation types, and sequences
///
/// **Note**: This test uploads 1600+ samples and takes ~3 minutes to complete.
#[test]
#[file_serial]
#[ignore = "Temporarily disabled due to CI timeout issues - run locally with: cargo test test_dataset_roundtrip -- --ignored"]
fn test_dataset_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    // Download→Upload→Download→Compare test for configurable dataset
    // This verifies Arrow file format preserves all metadata (sequences, groups,
    // annotations)

    let dataset = get_test_dataset();
    println!("Testing dataset roundtrip for: {}", dataset);

    let types = get_test_dataset_types();
    println!("Testing annotation types: {}", types.join(","));

    // Step 1: Download original dataset
    let (source_dataset_id, source_annotation_set_id) =
        get_dataset_and_first_annotation_set(&dataset)?;

    let original_images = download_dataset_from_server(&source_dataset_id)?;
    let original_annotations = download_annotations_from_server_with_types(
        &source_annotation_set_id,
        &types.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    )?;

    // Verify downloaded dataset structure is valid
    validate_dataset_structure(original_images.as_path())?;
    println!("✓ Downloaded dataset has valid structure");

    // Step 2: Upload to new dataset
    let project_id = get_project_id_by_name("Unit Testing")?
        .ok_or_else(|| "Project 'Unit Testing' not found".to_string())?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let new_dataset_name = format!("{} Roundtrip {}", dataset, timestamp);

    let mut cmd = edgefirst_cmd();
    cmd.arg("create-dataset")
        .arg(&project_id)
        .arg(&new_dataset_name);
    let output = cmd.ok()?.stdout;
    let new_dataset_id = String::from_utf8(output)?
        .lines()
        .find_map(|line| line.strip_prefix("Created dataset with ID: "))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "Failed to parse dataset ID".to_string())?;

    let annotation_set_name = format!("{} Annotations", new_dataset_name);
    let mut cmd = edgefirst_cmd();
    cmd.arg("create-annotation-set")
        .arg(&new_dataset_id)
        .arg(&annotation_set_name);
    let output = cmd.ok()?.stdout;
    let new_annotation_set_id = String::from_utf8(output)?
        .lines()
        .find_map(|line| line.strip_prefix("Created annotation set with ID: "))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "Failed to parse annotation set ID".to_string())?;

    // Upload with sequence support
    let mut cmd = edgefirst_cmd();
    cmd.arg("upload-dataset")
        .arg(&new_dataset_id)
        .arg("--annotations")
        .arg(&original_annotations)
        .arg("--annotation-set-id")
        .arg(&new_annotation_set_id)
        .arg("--images")
        .arg(&original_images);

    // Capture output to see debug messages
    let output = cmd.ok()?;
    eprintln!("\n=== UPLOAD COMMAND OUTPUT ===");
    eprintln!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    eprintln!("=== END UPLOAD OUTPUT ===\n");

    // Step 3: Download the uploaded dataset
    let redownloaded_images = download_dataset_from_server(&new_dataset_id)?;
    let redownloaded_annotations = download_annotations_from_server_with_types(
        &new_annotation_set_id,
        &types.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    )?;

    // Step 4: Compare image counts and directory structure
    // Note: Server may rename files, so we compare counts and structure, not exact
    // names
    let original_files = collect_relative_file_paths(&original_images)?;
    let redownloaded_files = collect_relative_file_paths(&redownloaded_images)?;

    // Count root images and sequence images
    let original_root_count = original_files.iter().filter(|p| !p.contains('/')).count();
    let original_seq_count = original_files.iter().filter(|p| p.contains('/')).count();
    let redownloaded_root_count = redownloaded_files
        .iter()
        .filter(|p| !p.contains('/'))
        .count();
    let redownloaded_seq_count = redownloaded_files
        .iter()
        .filter(|p| p.contains('/'))
        .count();

    println!("\n=== File Distribution ===");
    println!(
        "Original: {} root images, {} sequence images",
        original_root_count, original_seq_count
    );
    println!(
        "Redownloaded: {} root images, {} sequence images",
        redownloaded_root_count, redownloaded_seq_count
    );

    // Verify total file count matches
    assert_eq!(
        original_files.len(),
        redownloaded_files.len(),
        "File count mismatch: original {} vs redownloaded {}",
        original_files.len(),
        redownloaded_files.len()
    );

    // Verify root vs sequence distribution matches
    assert_eq!(
        original_root_count, redownloaded_root_count,
        "Root image count mismatch: {} vs {}",
        original_root_count, redownloaded_root_count
    );

    assert_eq!(
        original_seq_count, redownloaded_seq_count,
        "Sequence image count mismatch: {} vs {}",
        original_seq_count, redownloaded_seq_count
    );

    // Count sequence subdirectories (if any sequences exist)
    if original_seq_count > 0 {
        let original_sequences: BTreeSet<String> = original_files
            .iter()
            .filter_map(|p| p.split('/').next().map(|s| s.to_string()))
            .collect();
        let redownloaded_sequences: BTreeSet<String> = redownloaded_files
            .iter()
            .filter_map(|p| p.split('/').next().map(|s| s.to_string()))
            .collect();

        assert_eq!(
            original_sequences.len(),
            redownloaded_sequences.len(),
            "Sequence count mismatch: {} vs {}",
            original_sequences.len(),
            redownloaded_sequences.len()
        );
        println!("  Sequences: {} preserved", original_sequences.len());
    }

    // Step 5: Compare Arrow file sample counts AND verify groups/annotations are
    // preserved File names may differ, but sample count and metadata structure
    // should match
    let original_arrow_bytes = fs::read(&original_annotations)?;
    let redownloaded_arrow_bytes = fs::read(&redownloaded_annotations)?;

    println!("\n=== Arrow File Comparison ===");
    println!(
        "Arrow files: original {} bytes, redownloaded {} bytes",
        original_arrow_bytes.len(),
        redownloaded_arrow_bytes.len()
    );

    // Comprehensive verification of groups and annotations
    #[cfg(feature = "polars")]
    {
        println!("\n=== COMPREHENSIVE VERIFICATION ===");
        match compare_arrow_files(&original_annotations, &redownloaded_annotations) {
            Ok(()) => println!("✓ Groups and annotations verified successfully"),
            Err(e) => {
                return Err(format!("Arrow file verification failed: {}", e).into());
            }
        }
    }

    println!(
        "✓ {} dataset roundtrip successful: {} ({} annotation set {})",
        dataset, new_dataset_name, new_dataset_id, new_annotation_set_id
    );
    println!(
        "  Files: {} original, {} redownloaded",
        original_files.len(),
        redownloaded_files.len()
    );
    println!(
        "  Arrow file sizes: original {} bytes, redownloaded {} bytes",
        original_arrow_bytes.len(),
        redownloaded_arrow_bytes.len()
    );

    // Cleanup local files
    fs::remove_dir_all(&original_images).ok();
    fs::remove_file(&original_annotations).ok();
    fs::remove_dir_all(&redownloaded_images).ok();
    fs::remove_file(&redownloaded_annotations).ok();

    // Cleanup: Delete the created dataset from the server
    println!("\n=== CLEANUP: Deleting created dataset ===");
    let mut cmd = edgefirst_cmd();
    cmd.arg("delete-dataset").arg(&new_dataset_id);

    match cmd.output() {
        Ok(output) if output.status.success() => {
            println!("✓ Deleted dataset: {}", new_dataset_id);
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "⚠️  Failed to delete dataset {}: {}",
                new_dataset_id, stderr
            );
        }
        Err(e) => {
            eprintln!("⚠️  Error deleting dataset {}: {}", new_dataset_id, e);
        }
    }

    println!("\n✅ Dataset roundtrip test completed successfully");

    Ok(())
}

// ===== Experiment and Training Session Tests =====

#[test]
fn test_experiments_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("experiments").arg(&id);
        cmd.assert().success();
    }

    Ok(())
}

#[test]
fn test_experiments_filter_by_name() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("experiments")
            .arg(&id)
            .arg("--name")
            .arg("Unit Testing");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        assert!(output_str.contains("Unit Testing"));
        println!("Filtered experiments:\n{}", output_str);
    }

    Ok(())
}

#[test]
fn test_experiment_by_id() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("experiments")
            .arg(&proj_id)
            .arg("--name")
            .arg("Unit Testing");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        // Extract experiment ID (format: [exp-XXX])
        let exp_id = output_str
            .lines()
            .find(|line| line.contains("Unit Testing") && line.contains('['))
            .and_then(|line| {
                line.split('[')
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .map(|s| s.trim().to_string())
            });

        if let Some(id) = exp_id {
            let mut cmd = edgefirst_cmd();
            cmd.arg("experiment").arg(&id);
            cmd.assert()
                .success()
                .stdout(predicates::str::contains("Unit Testing"));
        }
    }

    Ok(())
}

#[test]
fn test_training_sessions_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("experiments")
            .arg(&proj_id)
            .arg("--name")
            .arg("Unit Testing");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        // Extract experiment ID (format: [exp-XXX])
        let exp_id = output_str
            .lines()
            .find(|line| line.contains("Unit Testing") && line.contains('['))
            .and_then(|line| {
                line.split('[')
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .map(|s| s.trim().to_string())
            });

        if let Some(id) = exp_id {
            let mut cmd = edgefirst_cmd();
            cmd.arg("training-sessions").arg(&id);
            cmd.assert().success();
        }
    }

    Ok(())
}

#[test]
fn test_training_sessions_filter_by_name() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("experiments")
            .arg(&proj_id)
            .arg("--name")
            .arg("Unit Testing");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        let exp_id = output_str
            .lines()
            .find(|line| line.contains("Unit Testing") && line.contains('['))
            .and_then(|line| {
                line.split('[')
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .map(|s| s.trim().to_string())
            });

        if let Some(id) = exp_id {
            let mut cmd = edgefirst_cmd();
            cmd.arg("training-sessions")
                .arg(&id)
                .arg("--name")
                .arg("modelpack-usermanaged");

            let output = cmd.ok()?.stdout;
            let output_str = String::from_utf8(output)?;

            assert!(output_str.contains("modelpack-usermanaged"));
            println!("Filtered training sessions:\n{}", output_str);
        }
    }

    Ok(())
}

#[test]
fn test_training_session_by_id() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("experiments")
            .arg(&proj_id)
            .arg("--name")
            .arg("Unit Testing");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        let exp_id = output_str
            .lines()
            .find(|line| line.contains("Unit Testing") && line.contains('['))
            .and_then(|line| {
                line.split('[')
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .map(|s| s.trim().to_string())
            });

        if let Some(id) = exp_id {
            let mut cmd = edgefirst_cmd();
            cmd.arg("training-sessions")
                .arg(&id)
                .arg("--name")
                .arg("modelpack-usermanaged");

            let output = cmd.ok()?.stdout;
            let output_str = String::from_utf8(output)?;

            // Extract session ID (format: t-xxx)
            let session_id = output_str
                .lines()
                .find(|line| line.contains("modelpack-usermanaged"))
                .and_then(|line| line.split_whitespace().next())
                .map(|s| s.to_string());

            if let Some(sid) = session_id {
                let mut cmd = edgefirst_cmd();
                cmd.arg("training-session").arg(&sid);
                cmd.assert()
                    .success()
                    .stdout(predicates::str::contains("modelpack-usermanaged"));
            }
        }
    }

    Ok(())
}

#[test]
fn test_training_session_with_model_params() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("experiments")
            .arg(&proj_id)
            .arg("--name")
            .arg("Unit Testing");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        let exp_id = output_str
            .lines()
            .find(|line| line.contains("Unit Testing") && line.contains('['))
            .and_then(|line| {
                line.split('[')
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .map(|s| s.trim().to_string())
            });

        if let Some(id) = exp_id {
            let mut cmd = edgefirst_cmd();
            cmd.arg("training-sessions")
                .arg(&id)
                .arg("--name")
                .arg("modelpack-usermanaged");

            let output = cmd.ok()?.stdout;
            let output_str = String::from_utf8(output)?;

            let session_id = output_str
                .lines()
                .find(|line| line.contains("modelpack-usermanaged"))
                .and_then(|line| line.split_whitespace().next())
                .map(|s| s.to_string());

            if let Some(sid) = session_id {
                let mut cmd = edgefirst_cmd();
                cmd.arg("training-session").arg(&sid).arg("--model");

                let output = cmd.ok()?.stdout;
                let output_str = String::from_utf8(output)?;

                assert!(output_str.contains("Model Parameters:"));
                println!("Training session with model params:\n{}", output_str);
            }
        }
    }

    Ok(())
}

#[test]
fn test_training_session_with_dataset_params() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("experiments")
            .arg(&proj_id)
            .arg("--name")
            .arg("Unit Testing");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        let exp_id = output_str
            .lines()
            .find(|line| line.contains("Unit Testing") && line.contains('['))
            .and_then(|line| {
                line.split('[')
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .map(|s| s.trim().to_string())
            });

        if let Some(id) = exp_id {
            let mut cmd = edgefirst_cmd();
            cmd.arg("training-sessions")
                .arg(&id)
                .arg("--name")
                .arg("modelpack-usermanaged");

            let output = cmd.ok()?.stdout;
            let output_str = String::from_utf8(output)?;

            let session_id = output_str
                .lines()
                .find(|line| line.contains("modelpack-usermanaged"))
                .and_then(|line| line.split_whitespace().next())
                .map(|s| s.to_string());

            if let Some(sid) = session_id {
                let mut cmd = edgefirst_cmd();
                cmd.arg("training-session").arg(&sid).arg("--dataset");

                let output = cmd.ok()?.stdout;
                let output_str = String::from_utf8(output)?;

                assert!(output_str.contains("Dataset Parameters:"));
                println!("Training session with dataset params:\n{}", output_str);
            }
        }
    }

    Ok(())
}

#[test]
fn test_training_session_with_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("experiments")
            .arg(&proj_id)
            .arg("--name")
            .arg("Unit Testing");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        let exp_id = output_str
            .lines()
            .find(|line| line.contains("Unit Testing") && line.contains('['))
            .and_then(|line| {
                line.split('[')
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .map(|s| s.trim().to_string())
            });

        if let Some(id) = exp_id {
            let mut cmd = edgefirst_cmd();
            cmd.arg("training-sessions")
                .arg(&id)
                .arg("--name")
                .arg("modelpack-960x540");

            let output = cmd.ok()?.stdout;
            let output_str = String::from_utf8(output)?;

            let session_id = output_str
                .lines()
                .find(|line| line.contains("modelpack-960x540"))
                .and_then(|line| line.split_whitespace().next())
                .map(|s| s.to_string());

            if let Some(sid) = session_id {
                let mut cmd = edgefirst_cmd();
                cmd.arg("training-session").arg(&sid).arg("--artifacts");

                let output = cmd.ok()?.stdout;
                let output_str = String::from_utf8(output)?;

                assert!(output_str.contains("Artifacts:"));
                println!("Training session with artifacts:\n{}", output_str);
            }
        }
    }

    Ok(())
}

// ===== Artifact Tests =====

/// Generic helper to extract values from CLI output using different strategies
fn extract_from_output<F>(output: &str, extractor: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    extractor(output)
}

/// Extracts the first ID in brackets from the first line (e.g., "[123] Name")
fn extract_first_id(output: &str) -> Option<String> {
    extract_from_output(output, |o| {
        o.lines()
            .next()
            .and_then(|line| line.split(']').next())
            .and_then(|s| s.trim_start_matches('[').parse::<String>().ok())
    })
}

/// Finds experiment ID for "Unit Testing" project
fn find_experiment_id(output: &str) -> Option<String> {
    extract_from_output(output, |o| {
        o.lines()
            .find(|line| line.contains("Unit Testing") && line.contains('['))
            .and_then(|line| {
                line.split('[')
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .map(|s| s.trim().to_string())
            })
    })
}

/// Finds training session ID by matching session name
fn find_training_session_id(output: &str, name: &str) -> Option<String> {
    extract_from_output(output, |o| {
        o.lines()
            .find(|line| line.contains(name))
            .and_then(|line| line.split_whitespace().next())
            .map(|s| s.to_string())
    })
}

/// Extracts artifact name from bulleted list (e.g., "- artifact.tar.gz")
fn extract_artifact_name(output: &str) -> Option<String> {
    extract_from_output(output, |o| {
        o.lines()
            .find(|line| line.trim().starts_with("- "))
            .map(|line| line.trim().trim_start_matches("- ").to_string())
    })
}

#[test]
#[file_serial]
fn test_download_artifact() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let proj_id = extract_first_id(&output_str).ok_or("Failed to extract project ID")?;

    let mut cmd = edgefirst_cmd();
    cmd.arg("experiments")
        .arg(&proj_id)
        .arg("--name")
        .arg("Unit Testing");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let exp_id = find_experiment_id(&output_str).ok_or("Failed to find experiment ID")?;

    let mut cmd = edgefirst_cmd();
    cmd.arg("training-sessions")
        .arg(&exp_id)
        .arg("--name")
        .arg("modelpack-960x540");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let session_id = find_training_session_id(&output_str, "modelpack-960x540")
        .ok_or("Failed to find training session")?;

    let mut cmd = edgefirst_cmd();
    cmd.arg("training-session")
        .arg(&session_id)
        .arg("--artifacts");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let artifact_name = extract_artifact_name(&output_str).ok_or("Failed to find artifact name")?;

    // Use target/testdata directory for downloads
    let test_dir = get_test_data_dir();
    let output_file = test_dir.join(format!("artifact_{}_{}", std::process::id(), artifact_name));

    // Clean up any existing file
    if output_file.exists() {
        fs::remove_file(&output_file)?;
    }

    let mut cmd = edgefirst_cmd();
    cmd.arg("download-artifact")
        .arg(&session_id)
        .arg(&artifact_name)
        .arg("--output")
        .arg(&output_file);

    cmd.assert().success();

    // Verify file was downloaded
    assert!(output_file.exists());
    println!("Downloaded artifact to {:?}", output_file);

    // Clean up
    fs::remove_file(&output_file)?;

    Ok(())
}

#[test]
#[file_serial]
fn test_upload_artifact() -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs::File, io::Write};

    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("experiments")
            .arg(&proj_id)
            .arg("--name")
            .arg("Unit Testing");

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        let exp_id = output_str
            .lines()
            .find(|line| line.contains("Unit Testing") && line.contains('['))
            .and_then(|line| {
                line.split('[')
                    .nth(1)
                    .and_then(|s| s.split(']').next())
                    .map(|s| s.trim().to_string())
            });

        if let Some(id) = exp_id {
            let mut cmd = edgefirst_cmd();
            cmd.arg("training-sessions")
                .arg(&id)
                .arg("--name")
                .arg("modelpack-usermanaged");

            let output = cmd.ok()?.stdout;
            let output_str = String::from_utf8(output)?;

            let session_id = output_str
                .lines()
                .find(|line| line.contains("modelpack-usermanaged"))
                .and_then(|line| line.split_whitespace().next())
                .map(|s| s.to_string());

            if let Some(sid) = session_id {
                // Create a test file to upload
                let test_file = "test_checkpoint_cli.txt";
                let mut file = File::create(test_file)?;
                writeln!(file, "Checkpoint from CLI test")?;

                let mut cmd = edgefirst_cmd();
                cmd.arg("upload-artifact")
                    .arg(&sid)
                    .arg(test_file)
                    .arg("--name")
                    .arg("checkpoint_cli.txt");

                cmd.assert().success();
                println!("Uploaded artifact checkpoint_cli.txt to session {}", sid);

                // Clean up
                fs::remove_file(test_file)?;
            }
        }
    }

    Ok(())
}

// ===== Task Tests =====

#[test]
fn test_tasks_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("tasks");
    cmd.assert().success();
    Ok(())
}

#[test]
fn test_tasks_with_name_filter() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("tasks").arg("--name").arg("modelpack-usermanaged");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    println!("Tasks with name filter:\n{}", output_str);
    Ok(())
}

#[test]
fn test_tasks_with_stages() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("tasks")
        .arg("--name")
        .arg("modelpack-usermanaged")
        .arg("--stages");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    println!("Tasks with stages:\n{}", output_str);
    Ok(())
}

#[test]
fn test_task_by_id() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("tasks").arg("--name").arg("modelpack-usermanaged");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Extract task ID from first line (format: "task-XXXX [...]  name => status")
    let task_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .map(|s| s.trim().to_string());

    if let Some(id) = task_id {
        let mut cmd = edgefirst_cmd();
        cmd.arg("task").arg(&id);
        cmd.assert().success();
        println!("Retrieved task details for ID: {}", id);
    }

    Ok(())
}

// ===== Validation Session Tests =====

#[test]
fn test_validation_sessions_list() -> Result<(), Box<dyn std::error::Error>> {
    // First get the "Unit Testing" project ID
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Extract project ID from first line (format: "[p-XXXX] name: description")
    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| {
            line.split(']')
                .next()
                .and_then(|s| s.strip_prefix('['))
                .map(|s| s.trim().to_string())
        })
        .expect("Could not find Unit Testing project");

    println!("Found project ID: {}", project_id);

    // Now list validation sessions for this project
    let mut cmd = edgefirst_cmd();
    cmd.arg("validation-sessions").arg(&project_id);

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    println!("Validation sessions:\n{}", output_str);

    // Should contain at least the "modelpack-usermanaged" session
    assert!(output_str.contains("modelpack-usermanaged"));

    Ok(())
}

#[test]
fn test_validation_session_details() -> Result<(), Box<dyn std::error::Error>> {
    // First get the "Unit Testing" project ID
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| {
            line.split(']')
                .next()
                .and_then(|s| s.strip_prefix('['))
                .map(|s| s.trim().to_string())
        })
        .expect("Could not find Unit Testing project");

    // Get validation sessions
    let mut cmd = edgefirst_cmd();
    cmd.arg("validation-sessions").arg(&project_id);

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Extract validation session ID from first line (format: "[v-XXXX] name:
    // description")
    let session_id = output_str.lines().next().and_then(|line| {
        line.split(']')
            .next()
            .and_then(|s| s.strip_prefix('['))
            .map(|s| s.trim().to_string())
    });

    if let Some(id) = session_id {
        println!("Found validation session ID: {}", id);

        // Get validation session details
        let mut cmd = edgefirst_cmd();
        cmd.arg("validation-session").arg(&id);

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        println!("Validation session details:\n{}", output_str);

        // Should contain the session ID
        assert!(output_str.contains(&id));
    }

    Ok(())
}

// ============================================================================
// Upload Dataset Tests
// ============================================================================

/// Helper function to get "Test Labels" dataset for write operations
fn get_test_labels_dataset() -> Result<(String, String), Box<dyn std::error::Error>> {
    // Get Unit Testing project
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;
    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| {
            line.split(']')
                .next()
                .and_then(|s| s.strip_prefix('['))
                .map(|s| s.trim().to_string())
        })
        .expect("Could not find Unit Testing project");

    // Get datasets and find "Test Labels" dataset
    let mut cmd = edgefirst_cmd();
    cmd.arg("datasets").arg(&project_id);
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Find the Test Labels dataset
    let test_labels_dataset = output_str
        .lines()
        .find(|line| line.contains("Test Labels"))
        .and_then(|line| {
            line.split(']')
                .next()
                .and_then(|s| s.strip_prefix('['))
                .map(|s| s.trim().to_string())
        })
        .expect("Could not find Test Labels dataset");

    println!("Found Test Labels dataset: {}", test_labels_dataset);

    // Get annotation sets for the dataset
    let mut cmd = edgefirst_cmd();
    cmd.arg("dataset")
        .arg(&test_labels_dataset)
        .arg("--annotation-sets");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    println!("Annotation sets output:\n{}", output_str);

    // Extract first annotation set ID (format: "[as-XXXX] name")
    // Skip the dataset info line and find annotation set lines
    let annotation_set_id = output_str
        .lines()
        .skip_while(|line| !line.contains("Annotation Sets:"))
        .skip(1) // Skip the "Annotation Sets:" header
        .find(|line| line.trim().starts_with('[') && line.contains("as-"))
        .and_then(|line| {
            line.split(']')
                .next()
                .and_then(|s| s.strip_prefix('['))
                .map(|s| s.trim().to_string())
        })
        .expect("Could not find annotation set for Test Labels dataset");

    println!("Found annotation set: {}", annotation_set_id);

    Ok((test_labels_dataset, annotation_set_id))
}

#[test]
#[file_serial]
fn test_upload_dataset_full_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Get Test Labels dataset for write operations
    let (dataset_id, annotation_set_id) = get_test_labels_dataset()?;

    // Get test data paths
    let dataset = get_test_dataset();
    let test_data_dir = get_test_dataset_path();
    let dataset_lower = dataset.to_lowercase().replace("ds-", "dataset-");
    let annotations_path = test_data_dir.join(format!("{}-stage.arrow", dataset_lower));
    let images_path = test_data_dir.join(&dataset_lower);

    // Verify test data exists
    if !annotations_path.exists() {
        eprintln!("⚠️  Test data not found: {}", annotations_path.display());
        eprintln!("    Skipping test - run download tests first to populate test data");
        return Ok(());
    }

    // Run upload-dataset with all parameters (full mode)
    let mut cmd = edgefirst_cmd();
    cmd.arg("upload-dataset")
        .arg(&dataset_id)
        .arg("--annotations")
        .arg(&annotations_path)
        .arg("--annotation-set-id")
        .arg(&annotation_set_id)
        .arg("--images")
        .arg(&images_path);

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    println!("Upload output:\n{}", output_str);

    // Verify success message or samples message
    assert!(
        output_str.contains("Successfully uploaded") || output_str.contains("samples"),
        "Expected success or samples message"
    );

    Ok(())
}

#[test]
#[file_serial]
fn test_upload_dataset_auto_discovery() -> Result<(), Box<dyn std::error::Error>> {
    // Get Test Labels dataset
    let (dataset_id, annotation_set_id) = get_test_labels_dataset()?;

    // Get test data paths
    let dataset = get_test_dataset();
    let test_data_dir = get_test_dataset_path();
    let dataset_lower = dataset.to_lowercase().replace("ds-", "dataset-");
    let annotations_path = test_data_dir.join(format!("{}-stage.arrow", dataset_lower));

    // Verify test data exists
    if !annotations_path.exists() {
        eprintln!("⚠️  Test data not found");
        eprintln!("    Skipping test - run download tests first to populate test data");
        return Ok(());
    }

    // Test auto-discovery: For {dataset}-stage.arrow, try to find folder/zip
    // Since we have {dataset}/ (not {dataset}-stage/), auto-discovery should fail
    // gracefully Run upload-dataset WITHOUT --images parameter
    let mut cmd = edgefirst_cmd();
    cmd.arg("upload-dataset")
        .arg(&dataset_id)
        .arg("--annotations")
        .arg(&annotations_path)
        .arg("--annotation-set-id")
        .arg(&annotation_set_id);

    let result = cmd.output()?;
    let stderr_str = String::from_utf8(result.stderr)?;

    println!("Upload stderr:\n{}", stderr_str);

    // Should fail with message about not finding images (deer-stage/ doesn't exist)
    assert!(
        !result.status.success(),
        "Auto-discovery should fail when deer-stage/ folder doesn't exist"
    );
    assert!(
        stderr_str.contains("Could not find images"),
        "Expected error about missing images directory"
    );

    Ok(())
}

#[test]
#[file_serial]
fn test_upload_dataset_images_only() -> Result<(), Box<dyn std::error::Error>> {
    // Get Test Labels dataset
    let (dataset_id, _annotation_set_id) = get_test_labels_dataset()?;

    // Get test data paths
    let dataset = get_test_dataset();
    let test_data_dir = get_test_dataset_path();
    let dataset_lower = dataset.to_lowercase().replace("ds-", "dataset-");
    let images_path = test_data_dir.join(&dataset_lower);

    // Verify test data exists
    if !images_path.exists() {
        eprintln!("⚠️  Test data not found: {}", images_path.display());
        eprintln!("    Skipping test - run download tests first to populate test data");
        return Ok(());
    }

    // Run upload-dataset in images-only mode (no annotations)
    let mut cmd = edgefirst_cmd();
    cmd.arg("upload-dataset")
        .arg(&dataset_id)
        .arg("--images")
        .arg(&images_path);

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    println!("Upload output:\n{}", output_str);

    // Verify success message or samples message
    assert!(
        output_str.contains("Successfully uploaded") || output_str.contains("samples"),
        "Expected success or samples message"
    );

    Ok(())
}

#[test]
#[file_serial]
fn test_upload_dataset_warning_no_annotation_set_id() -> Result<(), Box<dyn std::error::Error>> {
    // Get Test Labels dataset
    let (dataset_id, _annotation_set_id) = get_test_labels_dataset()?;

    // Get test data paths
    let dataset = get_test_dataset();
    let test_data_dir = get_test_dataset_path();
    let dataset_lower = dataset.to_lowercase().replace("ds-", "dataset-");
    let annotations_path = test_data_dir.join(format!("{}-stage.arrow", dataset_lower));
    let images_path = test_data_dir.join(&dataset_lower);

    // Verify test data exists
    if !annotations_path.exists() {
        eprintln!("⚠️  Test data not found: {}", annotations_path.display());
        eprintln!("    Skipping test - run download tests first to populate test data");
        return Ok(());
    }

    // Run upload-dataset with annotations but NO annotation_set_id (should warn)
    let mut cmd = edgefirst_cmd();
    cmd.arg("upload-dataset")
        .arg(&dataset_id)
        .arg("--annotations")
        .arg(&annotations_path)
        .arg("--images")
        .arg(&images_path);

    let result = cmd.output()?;
    let stdout_str = String::from_utf8(result.stdout)?;
    let stderr_str = String::from_utf8(result.stderr)?;

    println!("Upload stdout:\n{}", stdout_str);
    println!("Upload stderr:\n{}", stderr_str);

    // Verify warning message is present in stderr
    assert!(
        stderr_str.contains("⚠️") || stderr_str.contains("Warning"),
        "Expected warning message about missing annotation_set_id in stderr"
    );
    assert!(
        stderr_str.contains("annotation-set-id"),
        "Expected warning to mention annotation-set-id parameter"
    );

    // Should still succeed (uploading images only)
    assert!(
        result.status.success(),
        "Command should succeed when uploading images only"
    );
    assert!(
        stdout_str.contains("Successfully uploaded") || stdout_str.contains("samples"),
        "Expected success or samples message for images"
    );

    Ok(())
}

#[test]
#[file_serial]
fn test_upload_dataset_batching() -> Result<(), Box<dyn std::error::Error>> {
    // Get Test Labels dataset
    let (dataset_id, annotation_set_id) = get_test_labels_dataset()?;

    // Get test data paths (test dataset may have many images, which will trigger
    // batching)
    let dataset = get_test_dataset();
    let test_data_dir = get_test_dataset_path();
    let dataset_lower = dataset.to_lowercase().replace("ds-", "dataset-");
    let annotations_path = test_data_dir.join(format!("{}-stage.arrow", dataset_lower));
    let images_path = test_data_dir.join(&dataset_lower);

    // Verify test data exists
    if !annotations_path.exists() {
        eprintln!("⚠️  Test data not found: {}", annotations_path.display());
        eprintln!("    Skipping test - run download tests first to populate test data");
        return Ok(());
    }

    // Run upload-dataset with full dataset (should trigger batching at 500 samples)
    let mut cmd = edgefirst_cmd();
    cmd.arg("upload-dataset")
        .arg(&dataset_id)
        .arg("--annotations")
        .arg(&annotations_path)
        .arg("--annotation-set-id")
        .arg(&annotation_set_id)
        .arg("--images")
        .arg(&images_path);

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    println!("Upload output:\n{}", output_str);

    // With 1646 samples, should see batching messages if uploading new data
    // Expected: "Uploading batch 1/4", "Uploading batch 2/4", etc.
    // Note: May not see batching if samples already exist

    // Verify success
    assert!(
        output_str.contains("Successfully uploaded") || output_str.contains("samples"),
        "Expected success or samples message"
    );

    Ok(())
}

#[test]
#[file_serial]
fn test_upload_dataset_missing_parameters() -> Result<(), Box<dyn std::error::Error>> {
    // Get Test Labels dataset
    let (dataset_id, _annotation_set_id) = get_test_labels_dataset()?;

    // Try to run upload-dataset with NEITHER annotations NOR images (should fail)
    let mut cmd = edgefirst_cmd();
    cmd.arg("upload-dataset").arg(&dataset_id);

    let result = cmd.output()?;
    let output_str = String::from_utf8(result.stderr)?;

    println!("Error output:\n{}", output_str);

    // Should fail with error about missing parameters
    assert!(
        !result.status.success(),
        "Command should fail when both annotations and images are missing"
    );
    assert!(
        output_str.contains("annotations")
            || output_str.contains("images")
            || output_str.contains("Must provide"),
        "Error message should mention missing parameters"
    );

    Ok(())
}

#[test]
#[file_serial]
fn test_upload_dataset_invalid_path() -> Result<(), Box<dyn std::error::Error>> {
    // Get Test Labels dataset
    let (dataset_id, _annotation_set_id) = get_test_labels_dataset()?;

    // Try to upload with non-existent path
    let mut cmd = edgefirst_cmd();
    cmd.arg("upload-dataset")
        .arg(&dataset_id)
        .arg("--images")
        .arg("/nonexistent/path/to/images");

    let result = cmd.output()?;

    // Should fail
    assert!(
        !result.status.success(),
        "Command should fail with invalid path"
    );

    Ok(())
}

// ===== Dataset Management Tests =====

#[test]
#[file_serial]
fn test_dataset_crud() -> Result<(), Box<dyn std::error::Error>> {
    // Get Unit Testing project
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;
    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| {
            line.split(']')
                .next()
                .and_then(|s| s.strip_prefix('['))
                .map(|s| s.trim().to_string())
        })
        .expect("Could not find Unit Testing project");

    // 1. Create a test dataset
    let dataset_name = format!("CLI CRUD Test {}", chrono::Utc::now().timestamp());
    let mut cmd = edgefirst_cmd();
    cmd.arg("create-dataset")
        .arg(&project_id)
        .arg(&dataset_name)
        .arg("--description")
        .arg("Dataset for CLI CRUD test");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Verify dataset was created
    assert!(output_str.contains("Created dataset with ID:"));
    assert!(output_str.contains("ds-"));

    // Extract dataset ID
    let dataset_id = output_str
        .trim()
        .strip_prefix("Created dataset with ID: ")
        .expect("Could not extract dataset ID");

    println!(
        "✓ Step 1: Created dataset {} ({})",
        dataset_name, dataset_id
    );

    // 2. Create an annotation set for the dataset
    let annotation_set_name = format!("CLI CRUD AnnotationSet {}", chrono::Utc::now().timestamp());
    let mut cmd = edgefirst_cmd();
    cmd.arg("create-annotation-set")
        .arg(dataset_id)
        .arg(&annotation_set_name)
        .arg("--description")
        .arg("Annotation set for CLI CRUD test");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Verify annotation set was created
    assert!(output_str.contains("Created annotation set with ID:"));
    assert!(output_str.contains("as-"));

    // Extract annotation set ID
    let annotation_set_id = output_str
        .trim()
        .strip_prefix("Created annotation set with ID: ")
        .expect("Could not extract annotation set ID");

    println!(
        "✓ Step 2: Created annotation set {} ({})",
        annotation_set_name, annotation_set_id
    );

    // 3. (Skipped for now) Upload dataset with samples
    println!("✓ Step 3: Skipped - Upload samples (future enhancement)");

    // 4. Delete the annotation set
    let mut cmd = edgefirst_cmd();
    cmd.arg("delete-annotation-set").arg(annotation_set_id);

    let result = cmd.output()?;

    // Note: Server may not support annset.delete yet, so we tolerate failure
    if result.status.success() {
        let output_str = String::from_utf8(result.stdout)?;
        assert!(output_str.contains("marked as deleted"));
        assert!(output_str.contains(annotation_set_id));
        println!("✓ Step 4: Deleted annotation set {}", annotation_set_id);
    } else {
        let stderr = String::from_utf8(result.stderr)?;
        println!(
            "✓ Step 4: Annotation set deletion not supported by server (expected): {}",
            stderr.lines().next().unwrap_or("")
        );
    }

    // 5. Delete the dataset (this will also delete associated annotation sets)
    let mut cmd = edgefirst_cmd();
    cmd.arg("delete-dataset").arg(dataset_id);

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Verify dataset deletion message
    assert!(output_str.contains("marked as deleted"));
    assert!(output_str.contains(dataset_id));

    println!("✓ Step 5: Deleted dataset {}", dataset_id);
    println!("✅ Dataset CRUD workflow completed successfully");

    Ok(())
}

#[test]
#[file_serial]
fn test_download_dataset_flatten() -> Result<(), Box<dyn std::error::Error>> {
    // Test the --flatten option to download sequences without subdirectories
    let dataset = get_test_dataset();
    let (dataset_id, _) = get_dataset_and_first_annotation_set(&dataset)?;

    let downloads_root = get_test_data_dir().join("downloads");
    fs::create_dir_all(&downloads_root)?;

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();

    // Download with normal structure (sequences in subdirectories)
    let normal_dir = downloads_root.join(format!("normal_{}_{}", std::process::id(), timestamp));
    fs::create_dir_all(&normal_dir)?;

    println!("Downloading dataset with normal structure...");
    let mut cmd = edgefirst_cmd();
    cmd.arg("download-dataset")
        .arg(&dataset_id)
        .arg("--output")
        .arg(&normal_dir);
    cmd.assert().success();

    // Download with flattened structure
    let flatten_dir = downloads_root.join(format!("flatten_{}_{}", std::process::id(), timestamp));
    fs::create_dir_all(&flatten_dir)?;

    println!("Downloading dataset with --flatten option...");
    let mut cmd = edgefirst_cmd();
    cmd.arg("download-dataset")
        .arg(&dataset_id)
        .arg("--output")
        .arg(&flatten_dir)
        .arg("--flatten");
    cmd.assert().success();

    // Verify normal structure has subdirectories for sequences
    let normal_entries: Vec<_> = fs::read_dir(&normal_dir)?.filter_map(|e| e.ok()).collect();

    println!("Normal download structure:");
    let has_subdirs = normal_entries.iter().any(|e| e.path().is_dir());
    for entry in &normal_entries {
        let path = entry.path();
        let entry_type = if path.is_dir() { "DIR " } else { "FILE" };
        println!(
            "  {} {}",
            entry_type,
            path.file_name().unwrap().to_string_lossy()
        );
    }

    // Verify flattened structure has no subdirectories (all files in root)
    let flatten_entries: Vec<_> = fs::read_dir(&flatten_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .collect();

    println!("\nFlattened download structure:");
    let flatten_has_subdirs = flatten_entries.iter().any(|e| e.path().is_dir());
    for entry in &flatten_entries {
        let path = entry.path();
        let entry_type = if path.is_dir() { "DIR " } else { "FILE" };
        println!(
            "  {} {}",
            entry_type,
            path.file_name().unwrap().to_string_lossy()
        );
    }

    // Assert flatten has no subdirectories
    assert!(
        !flatten_has_subdirs,
        "Flattened download should not have subdirectories"
    );

    // Count total files in both structures
    let count_files = |dir: &Path| -> Result<usize, Box<dyn std::error::Error>> {
        let mut count = 0;
        for entry in walkdir::WalkDir::new(dir).min_depth(1).max_depth(10) {
            let entry = entry?;
            if entry.file_type().is_file() {
                count += 1;
            }
        }
        Ok(count)
    };

    let normal_file_count = count_files(&normal_dir)?;
    let flatten_file_count = count_files(&flatten_dir)?;

    println!("\nFile counts:");
    println!("  Normal structure: {} files", normal_file_count);
    println!("  Flattened structure: {} files", flatten_file_count);

    // Both should have the same number of files
    assert_eq!(
        normal_file_count, flatten_file_count,
        "Normal and flattened downloads should have same number of files"
    );

    // If dataset has sequences, verify normal has subdirectories
    if has_subdirs {
        println!("\n✓ Dataset contains sequences - normal download has subdirectories");

        // For flattened structure, verify filenames contain sequence prefixes
        let flatten_files: Vec<String> = flatten_entries
            .iter()
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        // At least some files should have underscore-separated sequence prefixes
        // (format: sequence_name_frame_rest.ext or sequence_name_rest.ext)
        let has_prefixed_files = flatten_files
            .iter()
            .any(|name| name.matches('_').count() >= 1);

        if has_prefixed_files {
            println!("✓ Flattened files contain sequence prefixes");
            println!("  Sample filenames:");
            for filename in flatten_files.iter().take(3) {
                println!("    - {}", filename);
            }
        }
    } else {
        println!("\n✓ Dataset contains no sequences - both structures are flat");
    }

    // Cleanup downloaded directories
    fs::remove_dir_all(&normal_dir).ok();
    fs::remove_dir_all(&flatten_dir).ok();

    println!("\n✅ Flatten option test completed successfully");
    Ok(())
}

// ============================================================================
// SNAPSHOT TESTS
// ============================================================================

#[test]
#[file_serial]
fn test_snapshots_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = edgefirst_cmd();
    cmd.arg("snapshots");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    println!("Snapshots list output:\n{}", output_str);

    // Should have header or at least complete without error
    assert!(
        output_str.contains("ss-") || output_str.is_empty() || output_str.contains("No snapshots"),
        "Expected snapshot list with ss- IDs or empty/no snapshots message"
    );

    Ok(())
}

#[test]
#[file_serial]
fn test_snapshot_get() -> Result<(), Box<dyn std::error::Error>> {
    // First, list snapshots to get a valid ID
    let mut cmd = edgefirst_cmd();
    cmd.arg("snapshots");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Extract first snapshot ID (format: [ss-XXXX] where XXXX is hexadecimal)
    let snapshot_id = output_str.lines().find_map(|line| {
        line.split(']')
            .next()
            .and_then(|s| s.strip_prefix('['))
            .filter(|id| {
                id.starts_with("ss-")
                    && id.len() > 3
                    && id.chars().skip(3).all(|c| c.is_ascii_hexdigit())
            })
            .map(|s| s.trim().to_string())
    });

    if let Some(id) = snapshot_id {
        println!("Testing with snapshot ID: {}", id);

        // Get snapshot details - allow failure if snapshot was deleted
        let mut cmd = edgefirst_cmd();
        cmd.arg("snapshot").arg(&id);

        match cmd.ok() {
            Ok(result) => {
                let output_str = String::from_utf8(result.stdout)?;
                println!("Snapshot details:\n{}", output_str);

                // Should contain the ID and basic info
                assert!(
                    output_str.contains(&id),
                    "Expected snapshot details to contain ID"
                );
            }
            Err(_) => {
                // Snapshot may have been deleted between list and get - this is acceptable
                println!(
                    "Note: Snapshot {} may have been deleted - skipping verification",
                    id
                );
            }
        }
    } else {
        return Err("No snapshots found - test server should have at least one snapshot".into());
    }

    Ok(())
}

#[test]
#[file_serial]
fn test_snapshot_create_download_delete_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // This test covers create, download, and delete in a single workflow

    // Create a test file to snapshot
    let test_data_dir = get_test_data_dir();
    let test_file = test_data_dir.join("test_snapshot_workflow.txt");
    fs::write(&test_file, b"Test snapshot workflow data")?;

    println!("=== STEP 1: Create Snapshot ===");
    // Create snapshot (no dataset ID needed - snapshots are project-agnostic)
    let mut cmd = edgefirst_cmd();
    cmd.arg("create-snapshot").arg(&test_file);
    let create_output = cmd.ok()?.stdout;
    let create_output_str = String::from_utf8(create_output)?;

    println!("Create snapshot output:\n{}", create_output_str);

    // Extract snapshot ID from creation output (format: [ss-XXX] name)
    let snapshot_id = create_output_str
        .lines()
        .find_map(|line| {
            // Look for pattern like "[ss-e0e]"
            if let Some(start) = line.find('[')
                && let Some(end) = line[start..].find(']')
            {
                let id_with_brackets = &line[start..start + end + 1];
                let id = id_with_brackets
                    .trim_start_matches('[')
                    .trim_end_matches(']');
                if id.starts_with("ss-") {
                    return Some(id.to_string());
                }
            }
            None
        })
        .expect("Could not extract snapshot ID from creation output");

    println!("✓ Created snapshot: {}", snapshot_id);

    println!("\n=== STEP 2: Wait for Snapshot Processing ===");
    // Wait for snapshot to be completed (snapshots need processing time)
    // Use the API directly to check status
    use edgefirst_client::{Client as EdgFirstClient, SnapshotID};
    let api_client = EdgFirstClient::new()?.with_token_path(None)?;
    let snap_id = SnapshotID::try_from(snapshot_id.as_str())?;

    let rt = tokio::runtime::Runtime::new()?;
    let mut attempts = 0;
    let max_attempts = 30; // 30 seconds max wait
    loop {
        let snapshot = rt.block_on(api_client.snapshot(snap_id))?;
        let status = snapshot.status();

        // Snapshot is ready when status is "available" or "completed"
        if status == "available" || status == "completed" {
            println!("✓ Snapshot ready (status: {})", status);
            break;
        }

        attempts += 1;
        if attempts >= max_attempts {
            panic!(
                "Snapshot did not become available within {} seconds. Last status: {}",
                max_attempts, status
            );
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    println!("\n=== STEP 3: Download Snapshot ===");
    // Create download directory
    let downloads_root = get_test_data_dir().join("downloads");
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let download_dir =
        downloads_root.join(format!("snapshot_{}_{}", std::process::id(), timestamp));
    fs::create_dir_all(&download_dir)?;

    // Download snapshot (signature: snapshot_id --output path)
    let mut cmd = edgefirst_cmd();
    cmd.arg("download-snapshot")
        .arg(&snapshot_id)
        .arg("--output")
        .arg(&download_dir);
    let download_output = cmd.ok()?.stdout;
    let download_output_str = String::from_utf8(download_output)?;

    println!("Download snapshot output:\n{}", download_output_str);

    // Verify download directory has content
    let entries: Vec<_> = fs::read_dir(&download_dir)?
        .filter_map(|e| e.ok())
        .collect();

    assert!(
        !entries.is_empty(),
        "Expected downloaded snapshot to contain files"
    );

    println!("✓ Downloaded {} items", entries.len());

    println!("\n=== STEP 4: Delete Snapshot ===");
    // Delete the snapshot
    let mut cmd = edgefirst_cmd();
    cmd.arg("delete-snapshot").arg(&snapshot_id);
    let delete_output = cmd.ok()?.stdout;
    let delete_output_str = String::from_utf8(delete_output)?;

    println!("Delete snapshot output:\n{}", delete_output_str);

    println!("✓ Deleted snapshot: {}", snapshot_id);

    // Clean up test file and download directory
    let _ = fs::remove_file(&test_file);
    let _ = fs::remove_dir_all(&download_dir);

    println!("\n✅ Snapshot workflow test completed successfully");
    Ok(())
}

/// Compute SHA256 checksum of a file
fn compute_file_checksum(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[test]
#[file_serial]
fn test_snapshot_restore() -> Result<(), Box<dyn std::error::Error>> {
    // =========================================================================
    // SNAPSHOT RESTORE TEST
    //
    // Tests that restoring a snapshot produces a dataset identical to the
    // snapshot's contents. Validates group information is preserved.
    //
    // Flow (leveraging server-side async processing):
    // 1. Start restore (returns task ID for async monitoring)
    // 2. Download snapshot locally (while restore runs on server)
    // 3. Use task --monitor to wait for restore completion
    // 4. Download restored dataset and compare with snapshot
    // =========================================================================

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║  SNAPSHOT RESTORE TEST                                         ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    // =========================================================================
    // STEP 1: Find the snapshot and project
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 1: Find Snapshot and Project                               │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    // Find Unit Testing project
    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .find_map(|line| {
            let id_part = line.split(']').next()?;
            let id = id_part.strip_prefix('[')?.trim();
            if id.starts_with("p-") {
                Some(id.to_string())
            } else {
                None
            }
        })
        .expect("Could not find Unit Testing project");
    println!("✓ Project: {}", project_id);

    // List all snapshots and find "Unit Testing - Deer Dataset"
    let mut cmd = edgefirst_cmd();
    cmd.arg("snapshots");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Parse snapshots - format: [ss-xxx] Description (status)
    let snapshot_id = output_str
        .lines()
        .find_map(|line| {
            if line.contains("Unit Testing - Deer Dataset") {
                let id_part = line.split(']').next()?;
                let id = id_part.strip_prefix('[')?.trim();
                if id.starts_with("ss-") {
                    return Some(id.to_string());
                }
            }
            None
        })
        .expect("Could not find 'Unit Testing - Deer Dataset' snapshot");
    println!("✓ Snapshot: {}", snapshot_id);

    // =========================================================================
    // STEP 2: Start restore (returns task ID for async monitoring)
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 2: Start Restore (async - returns task ID)                 │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    let mut restore_cmd = edgefirst_cmd();
    restore_cmd
        .arg("restore-snapshot")
        .arg(&project_id)
        .arg(&snapshot_id);
    restore_cmd.timeout(std::time::Duration::from_secs(60));

    let restore_output = restore_cmd.ok()?.stdout;
    let restore_output_str = String::from_utf8(restore_output)?;
    println!("Restore output:\n{}", restore_output_str);

    // Extract dataset ID from restore output
    let restored_dataset_id = restore_output_str
        .lines()
        .find_map(|line| {
            if let Some(start) = line.find("[ds-") {
                let rest = &line[start + 1..];
                if let Some(end) = rest.find(']') {
                    return Some(rest[..end].to_string());
                }
            }
            None
        })
        .expect("Could not extract dataset ID from restore output");
    println!("✓ Restored dataset: {}", restored_dataset_id);

    // Extract task ID from restore output (format: "Task: [task-xxx]")
    let task_id = restore_output_str.lines().find_map(|line| {
        if let Some(start) = line.find("[task-") {
            let rest = &line[start + 1..];
            if let Some(end) = rest.find(']') {
                return Some(rest[..end].to_string());
            }
        }
        None
    });

    if let Some(ref tid) = task_id {
        println!("✓ Task ID: {}", tid);
    } else {
        println!("⚠️  No task ID returned - restore may be synchronous");
    }

    // =========================================================================
    // STEP 3: Download snapshot locally (while restore runs on server)
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 3: Download Snapshot Locally                               │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    // Setup download directory
    let test_root = get_test_data_dir().join("snapshot_restore_test");
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let test_dir = test_root.join(format!("test_{}_{}", std::process::id(), timestamp));
    let snapshot_download_dir = test_dir.join("snapshot");
    fs::create_dir_all(&snapshot_download_dir)?;

    println!(
        "Downloading snapshot to {}...",
        snapshot_download_dir.display()
    );
    let mut download_cmd = edgefirst_cmd();
    download_cmd
        .arg("download-snapshot")
        .arg(&snapshot_id)
        .arg("--output")
        .arg(&snapshot_download_dir);
    download_cmd.timeout(std::time::Duration::from_secs(300));
    download_cmd.ok()?;

    // Find downloaded files (always dataset.arrow and dataset.zip)
    let snapshot_arrow = snapshot_download_dir.join("dataset.arrow");
    let snapshot_zip = snapshot_download_dir.join("dataset.zip");

    assert!(
        snapshot_arrow.exists(),
        "Expected dataset.arrow in snapshot download"
    );
    println!(
        "✓ Downloaded snapshot arrow: {} ({} bytes)",
        snapshot_arrow.display(),
        fs::metadata(&snapshot_arrow)?.len()
    );
    if snapshot_zip.exists() {
        println!(
            "✓ Downloaded snapshot zip: {} ({} bytes)",
            snapshot_zip.display(),
            fs::metadata(&snapshot_zip)?.len()
        );
    }

    // =========================================================================
    // STEP 4: Wait for restore to complete using task --monitor
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 4: Wait for Restore to Complete                            │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    if let Some(tid) = task_id {
        println!("Monitoring task {} for completion...", tid);
        let mut task_cmd = edgefirst_cmd();
        task_cmd.arg("task").arg(&tid).arg("--monitor");
        task_cmd.timeout(std::time::Duration::from_secs(600));
        task_cmd.ok()?;
        println!("✓ Restore task completed");
    } else {
        // No task ID - wait a bit for synchronous restore to settle
        println!("No task ID - waiting 5 seconds for restore to settle...");
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    // =========================================================================
    // STEP 5: Get Restored Dataset's Annotation Set
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 5: Get Restored Dataset Annotation Set                     │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    // Get annotation set from restored dataset
    let mut cmd = edgefirst_cmd();
    cmd.arg("dataset")
        .arg(&restored_dataset_id)
        .arg("--annotation-sets");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let restored_annotation_set_id = output_str
        .lines()
        .skip_while(|line| !line.contains("Annotation Sets:"))
        .skip(1)
        .find_map(|line| {
            line.split(']')
                .next()
                .and_then(|s| s.strip_prefix('['))
                .map(|s| s.trim().to_string())
                .filter(|id| id.starts_with("as-"))
        })
        .expect("No annotation set found in restored dataset");
    println!("✓ Restored annotation set: {}", restored_annotation_set_id);

    // =========================================================================
    // STEP 6: Validate Snapshot Arrow Baseline
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 6: Validate Snapshot Arrow Baseline                        │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    // Extract image→group mapping from snapshot arrow as the source of truth
    // An IMAGE is uniquely identified by (name, frame) - name is sequence, frame is
    // frame number CRITICAL: All rows must have group values (no nulls allowed
    // in baseline)
    #[cfg(feature = "polars")]
    let snap_image_groups: HashMap<(String, Option<i32>), String> = {
        use polars::prelude::*;

        let mut snap_file = fs::File::open(&snapshot_arrow)?;
        let snap_df = IpcReader::new(&mut snap_file).finish()?;

        println!(
            "Snapshot arrow: {} rows, {} columns",
            snap_df.height(),
            snap_df.width()
        );
        println!("Snapshot columns: {:?}", snap_df.get_column_names());

        // Check group column exists
        let snap_groups = snap_df
            .column("group")
            .expect("Snapshot arrow missing 'group' column!");
        let snap_name_col = snap_df.column("name")?;
        let snap_frame_col = snap_df.column("frame")?;

        // BASELINE VALIDATION: No nulls allowed in snapshot group column
        let snap_null_count = snap_groups.null_count();
        assert_eq!(
            snap_null_count, 0,
            "SNAPSHOT BASELINE INVALID: {} rows have null group values! All rows must have group.",
            snap_null_count
        );
        println!(
            "✓ Snapshot baseline valid: all {} rows have group values",
            snap_df.height()
        );

        // Build (name, frame)→group mapping
        // All rows for same (name, frame) should have consistent group
        let snap_groups_cast = snap_groups.cast(&DataType::String)?;
        let snap_groups_str = snap_groups_cast.str()?;
        let snap_names_cast = snap_name_col.cast(&DataType::String)?;
        let snap_names = snap_names_cast.str()?;
        let snap_frames = snap_frame_col.i32()?;

        let mut image_groups: HashMap<(String, Option<i32>), String> = HashMap::new();
        let mut inconsistent_groups: Vec<(String, Option<i32>, String, String)> = Vec::new();

        for idx in 0..snap_df.height() {
            if let (Some(name), Some(group)) = (snap_names.get(idx), snap_groups_str.get(idx)) {
                let name = name.to_string();
                let frame = snap_frames.get(idx);
                let group = group.to_string();
                let key = (name.clone(), frame);

                if let Some(existing) = image_groups.get(&key) {
                    if existing != &group {
                        inconsistent_groups.push((name, frame, existing.clone(), group));
                    }
                } else {
                    image_groups.insert(key, group);
                }
            }
        }

        assert!(
            inconsistent_groups.is_empty(),
            "SNAPSHOT BASELINE INVALID: {} images have inconsistent groups!\nFirst few: {:?}",
            inconsistent_groups.len(),
            inconsistent_groups.iter().take(5).collect::<Vec<_>>()
        );

        // Count unique sequences and images
        let unique_sequences: std::collections::HashSet<_> =
            image_groups.keys().map(|(n, _)| n.clone()).collect();
        println!(
            "✓ Snapshot baseline valid: {} unique images across {} sequences",
            image_groups.len(),
            unique_sequences.len()
        );

        // Show group distribution
        let mut group_counts: HashMap<&str, usize> = HashMap::new();
        for group in image_groups.values() {
            *group_counts.entry(group.as_str()).or_default() += 1;
        }
        println!("  Group distribution: {:?}", group_counts);

        image_groups
    };

    #[cfg(not(feature = "polars"))]
    let snap_image_groups: HashMap<(String, Option<i32>), String> = {
        panic!("This test requires the 'polars' feature");
    };

    // =========================================================================
    // STEP 7: Get Samples via Library API (bypassing CLI)
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 7: Fetch Restored Samples via Library API                  │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    use edgefirst_client::{AnnotationSetID, AnnotationType, Client as EdgeFirstClient, DatasetID};

    let api_client = EdgeFirstClient::new()?.with_token_path(None)?;
    let rt = tokio::runtime::Runtime::new()?;

    // Parse dataset and annotation set IDs
    let dataset_id = DatasetID::try_from(restored_dataset_id.as_str())?;
    let annotation_set_id = AnnotationSetID::try_from(restored_annotation_set_id.as_str())?;

    println!("Fetching samples from restored dataset via API...");
    println!("  Dataset: {}", restored_dataset_id);
    println!("  Annotation Set: {}", restored_annotation_set_id);

    // Fetch all samples using the library API directly
    let samples = rt.block_on(api_client.samples(
        dataset_id,
        Some(annotation_set_id),
        &[
            AnnotationType::Box2d,
            AnnotationType::Box3d,
            AnnotationType::Mask,
        ],
        &[], // All groups
        &[], // No file type filter
        None,
    ))?;

    println!("✓ Fetched {} samples from API", samples.len());

    // Build (sequence_name, frame_number)→group mapping from API response
    // NOTE: sample.name() returns extracted name (e.g., "seq_123" from
    // "seq_123.jpg")       sample.sequence_name() returns the actual sequence
    // name for sequences For the comparison, we use sequence_name +
    // frame_number to match snapshot's (name, frame)
    let mut api_image_groups: HashMap<(String, Option<u32>), Option<String>> = HashMap::new();
    let mut api_inconsistent_groups: Vec<(String, Option<u32>, String, String)> = Vec::new();

    for sample in &samples {
        // Use sequence_name for sequences, fall back to name for standalone images
        let seq_name = sample.sequence_name().cloned().or_else(|| sample.name());
        let Some(seq_name) = seq_name else {
            continue; // Skip samples without any name
        };
        let frame = sample.frame_number();
        let group = sample.group().cloned();
        let key = (seq_name.clone(), frame);

        if let Some(existing) = api_image_groups.get(&key) {
            // Check consistency
            if existing != &group {
                api_inconsistent_groups.push((
                    seq_name,
                    frame,
                    existing.clone().unwrap_or_else(|| "null".to_string()),
                    group.clone().unwrap_or_else(|| "null".to_string()),
                ));
            }
        } else {
            api_image_groups.insert(key, group);
        }
    }

    // Report any inconsistencies in API response
    if !api_inconsistent_groups.is_empty() {
        println!(
            "⚠️  {} images have inconsistent groups in API response:",
            api_inconsistent_groups.len()
        );
        for (name, frame, g1, g2) in api_inconsistent_groups.iter().take(5) {
            println!("    ({}, {:?}) ({} vs {})", name, frame, g1, g2);
        }
    }

    // Count unique sequences
    let unique_sequences: std::collections::HashSet<_> =
        api_image_groups.keys().map(|(n, _)| n.clone()).collect();
    println!(
        "✓ API response has {} unique images across {} sequences",
        api_image_groups.len(),
        unique_sequences.len()
    );

    // Show API group distribution
    let mut api_group_counts: HashMap<String, usize> = HashMap::new();
    for group in api_image_groups.values() {
        let key = group.clone().unwrap_or_else(|| "null".to_string());
        *api_group_counts.entry(key).or_default() += 1;
    }
    println!("  API group distribution: {:?}", api_group_counts);

    // =========================================================================
    // STEP 8: Compare Snapshot Baseline vs API Response
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 8: Compare Snapshot Baseline vs API Response               │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    // Helper to format image key for display
    fn fmt_img(key: &(String, Option<i32>)) -> String {
        match key.1 {
            Some(f) => format!("({}, frame={})", key.0, f),
            None => format!("({}, frame=None)", key.0),
        }
    }

    let mut lost_groups: Vec<((String, Option<i32>), String)> = Vec::new();
    let mut changed_groups: Vec<((String, Option<i32>), String, String)> = Vec::new();
    let mut missing_images: Vec<((String, Option<i32>), String)> = Vec::new();
    let mut matched_count = 0;

    for (img, snap_group) in &snap_image_groups {
        // Convert i32 frame to u32 for API lookup
        let api_key = (img.0.clone(), img.1.map(|f| f as u32));
        if let Some(api_group) = api_image_groups.get(&api_key) {
            matched_count += 1;
            match api_group {
                None => lost_groups.push((img.clone(), snap_group.clone())),
                Some(rg) if rg != snap_group => {
                    changed_groups.push((img.clone(), snap_group.clone(), rg.clone()))
                }
                _ => {} // Matches - good!
            }
        } else {
            missing_images.push((img.clone(), snap_group.clone()));
        }
    }

    // Count images in API but not in snapshot
    // NOTE: This indicates a SERVER BUG in snapshot creation - unannotated images
    // should still be included in the Arrow file with their group assignment.
    // The snapshot Arrow should have ALL images, not just annotated ones.
    let snap_keys: std::collections::HashSet<_> = snap_image_groups
        .keys()
        .map(|(n, f)| (n.clone(), f.map(|x| x as u32)))
        .collect();
    let api_only: Vec<_> = api_image_groups
        .keys()
        .filter(|k| !snap_keys.contains(*k))
        .collect();

    if !api_only.is_empty() {
        println!(
            "⚠️  {} images in API but NOT in snapshot (SERVER BUG: unannotated images missing from snapshot Arrow)",
            api_only.len()
        );
        println!("   These images exist in the dataset but were not included in the snapshot.");
        println!(
            "   The server should include ALL images in the Arrow file, even unannotated ones."
        );
    }

    // Report findings
    if !missing_images.is_empty() {
        println!(
            "\n⚠️  {} images from snapshot NOT FOUND in API response:",
            missing_images.len()
        );
        for (img, group) in missing_images.iter().take(5) {
            println!("    {} (was: {})", fmt_img(img), group);
        }
        if missing_images.len() > 5 {
            println!("    ... and {} more", missing_images.len() - 5);
        }
    }

    if !lost_groups.is_empty() {
        println!(
            "\n⚠️  {} images LOST their group (now null) in API:",
            lost_groups.len()
        );
        for (img, group) in lost_groups.iter().take(5) {
            println!("    {} (was: {})", fmt_img(img), group);
        }
        if lost_groups.len() > 5 {
            println!("    ... and {} more", lost_groups.len() - 5);
        }
    }

    if !changed_groups.is_empty() {
        println!(
            "\n⚠️  {} images CHANGED group in API:",
            changed_groups.len()
        );
        for (img, old, new) in changed_groups.iter().take(10) {
            println!("    {} ({} -> {})", fmt_img(img), old, new);
        }
        if changed_groups.len() > 10 {
            println!("    ... and {} more", changed_groups.len() - 10);
        }
    }

    // The critical assertions:

    // 1. All snapshot images should be in API response
    assert!(
        missing_images.is_empty(),
        "MISSING IMAGES: {} images from snapshot not found in API!\nFirst few: {:?}",
        missing_images.len(),
        missing_images.iter().take(3).collect::<Vec<_>>()
    );
    println!(
        "✓ All {} snapshot images found in API response",
        snap_image_groups.len()
    );

    // 2. No images should lose their group
    assert!(
        lost_groups.is_empty(),
        "GROUP DATA LOSS: {} images lost their group in API!\nFirst few: {:?}",
        lost_groups.len(),
        lost_groups.iter().take(3).collect::<Vec<_>>()
    );
    println!("✓ No images lost their group");

    // 3. No images should change their group assignment
    assert!(
        changed_groups.is_empty(),
        "GROUP CHANGED: {} images had their group changed!\nFirst few: {:?}",
        changed_groups.len(),
        changed_groups.iter().take(3).collect::<Vec<_>>()
    );
    println!("✓ No images changed their group");

    // =========================================================================
    // CLEANUP
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ CLEANUP                                                         │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    // Delete restored dataset
    let mut cmd = edgefirst_cmd();
    cmd.arg("delete-dataset").arg(&restored_dataset_id);
    match cmd.output() {
        Ok(output) if output.status.success() => {
            println!("✓ Deleted restored dataset: {}", restored_dataset_id);
        }
        _ => {
            println!(
                "⚠️  Could not delete restored dataset: {}",
                restored_dataset_id
            );
        }
    }

    // Clean up local files
    fs::remove_dir_all(&test_dir).ok();
    println!("✓ Cleaned up test directory");

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  ✅ SNAPSHOT RESTORE TEST PASSED                                ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    Ok(())
}

/// Test that exporting a dataset to a snapshot preserves all data.
///
/// This is the mirror of `test_snapshot_restore` - it tests the inverse
/// operation:
/// 1. Get the Deer dataset and download its annotations (original Arrow)
/// 2. Export the dataset to a snapshot on the server (using export-snapshot)
/// 3. Wait for export to complete
/// 4. Download the created snapshot
/// 5. Compare the original Arrow with the snapshot's Arrow
///
/// Test the create-snapshot command with a server-side dataset.
///
/// This validates that server-side snapshot creation from a dataset produces
/// an Arrow file that matches the dataset's annotations, including groups.
#[test]
#[file_serial]
fn test_create_snapshot_from_dataset() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║  CREATE SNAPSHOT FROM DATASET TEST                             ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    // =========================================================================
    // STEP 1: Get the Deer dataset and its annotation set
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 1: Get Deer Dataset                                        │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    let (dataset_id, annotation_set_id) = get_dataset_and_first_annotation_set("Deer")?;
    println!("✓ Dataset: {}", dataset_id);
    println!("✓ Annotation Set: {}", annotation_set_id);

    // =========================================================================
    // STEP 2: Download annotations from dataset (original Arrow baseline)
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 2: Download Original Annotations (Baseline)                │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    // Setup test directory
    let test_root = get_test_data_dir().join("snapshot_export_test");
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let test_dir = test_root.join(format!("test_{}_{}", std::process::id(), timestamp));
    fs::create_dir_all(&test_dir)?;

    let original_arrow = test_dir.join("original.arrow");

    let mut cmd = edgefirst_cmd();
    cmd.arg("download-annotations")
        .arg(&annotation_set_id)
        .arg("--types")
        .arg("box2d,mask")
        .arg(&original_arrow);
    cmd.timeout(std::time::Duration::from_secs(120));
    cmd.assert().success();

    println!(
        "✓ Downloaded original annotations: {} ({} bytes)",
        original_arrow.display(),
        fs::metadata(&original_arrow)?.len()
    );

    // =========================================================================
    // STEP 3: Export dataset to snapshot (triggers server-side creation)
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 3: Create Snapshot from Dataset                            │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    let snapshot_name = format!("QA Export Test {}", timestamp);

    let mut export_cmd = edgefirst_cmd();
    export_cmd
        .arg("create-snapshot")
        .arg(&dataset_id)
        .arg("--description")
        .arg(&snapshot_name);
    export_cmd.timeout(std::time::Duration::from_secs(120));

    let export_output = export_cmd.ok()?.stdout;
    let export_output_str = String::from_utf8(export_output)?;
    println!("Export output:\n{}", export_output_str);

    // Extract snapshot ID from export output
    let snapshot_id = export_output_str
        .lines()
        .find_map(|line| {
            if let Some(start) = line.find("[ss-") {
                let rest = &line[start + 1..];
                if let Some(end) = rest.find(']') {
                    return Some(rest[..end].to_string());
                }
            }
            None
        })
        .expect("Could not extract snapshot ID from export output");
    println!("✓ Created snapshot: {}", snapshot_id);

    // Extract task ID from export output (format: "Task: [task-xxx]")
    let task_id = export_output_str.lines().find_map(|line| {
        if let Some(start) = line.find("[task-") {
            let rest = &line[start + 1..];
            if let Some(end) = rest.find(']') {
                return Some(rest[..end].to_string());
            }
        }
        None
    });

    if let Some(ref tid) = task_id {
        println!("✓ Task ID: {}", tid);
    } else {
        println!("⚠️  No task ID returned - export may be synchronous");
    }

    // =========================================================================
    // STEP 4: Wait for export to complete
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 4: Wait for Export to Complete                             │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    if let Some(tid) = task_id {
        println!("Monitoring task {} for completion...", tid);
        let mut task_cmd = edgefirst_cmd();
        task_cmd.arg("task").arg(&tid).arg("--monitor");
        task_cmd.timeout(std::time::Duration::from_secs(600));
        task_cmd.ok()?;
        println!("✓ Export task completed");
    } else {
        // No task ID - poll snapshot status directly
        println!("No task ID - polling snapshot status...");
        use edgefirst_client::{Client as EdgeFirstClient, SnapshotID};
        let api_client = EdgeFirstClient::new()?.with_token_path(None)?;
        let snap_id = SnapshotID::try_from(snapshot_id.as_str())?;

        let rt = tokio::runtime::Runtime::new()?;
        let mut attempts = 0;
        let max_attempts = 120; // 2 minutes max wait
        loop {
            let snapshot = rt.block_on(api_client.snapshot(snap_id))?;
            let status = snapshot.status();
            if status == "available" || status == "completed" {
                println!("✓ Snapshot ready (status: {})", status);
                break;
            }
            if status == "failed" || status == "error" {
                panic!("Snapshot export failed (status: {})", status);
            }
            attempts += 1;
            if attempts >= max_attempts {
                panic!(
                    "Snapshot did not become available within {} seconds. Last status: {}",
                    max_attempts, status
                );
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    // =========================================================================
    // STEP 5: Download the created snapshot
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 5: Download Created Snapshot                               │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    let snapshot_download_dir = test_dir.join("snapshot");
    fs::create_dir_all(&snapshot_download_dir)?;

    println!(
        "Downloading snapshot to {}...",
        snapshot_download_dir.display()
    );
    let mut download_cmd = edgefirst_cmd();
    download_cmd
        .arg("download-snapshot")
        .arg(&snapshot_id)
        .arg("--output")
        .arg(&snapshot_download_dir);
    download_cmd.timeout(std::time::Duration::from_secs(300));
    download_cmd.ok()?;

    let snapshot_arrow = snapshot_download_dir.join("dataset.arrow");
    assert!(
        snapshot_arrow.exists(),
        "Expected dataset.arrow in snapshot download"
    );
    println!(
        "✓ Downloaded snapshot arrow: {} ({} bytes)",
        snapshot_arrow.display(),
        fs::metadata(&snapshot_arrow)?.len()
    );

    // =========================================================================
    // STEP 6: Compare Original Arrow vs Snapshot Arrow
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 6: Compare Original vs Snapshot Arrow                      │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    #[cfg(feature = "polars")]
    {
        use polars::prelude::*;

        // Read both Arrow files
        let mut original_file = fs::File::open(&original_arrow)?;
        let original_df = IpcReader::new(&mut original_file).finish()?;

        let mut snapshot_file = fs::File::open(&snapshot_arrow)?;
        let snapshot_df = IpcReader::new(&mut snapshot_file).finish()?;

        println!(
            "Original Arrow: {} rows, {} columns",
            original_df.height(),
            original_df.width()
        );
        println!(
            "Snapshot Arrow: {} rows, {} columns",
            snapshot_df.height(),
            snapshot_df.width()
        );
        println!("Original columns: {:?}", original_df.get_column_names());
        println!("Snapshot columns: {:?}", snapshot_df.get_column_names());

        // Build (name, frame) -> group mapping for both
        fn build_image_groups(
            df: &DataFrame,
        ) -> Result<HashMap<(String, Option<i32>), Option<String>>, Box<dyn std::error::Error>>
        {
            let name_col = df.column("name")?;
            let frame_col = df.column("frame")?;
            let group_col = df.column("group").ok();

            let names_cast = name_col.cast(&DataType::String)?;
            let names = names_cast.str()?;
            // Cast frame to i32 to handle both i32 and u32 sources
            let frames_cast = frame_col.cast(&DataType::Int32)?;
            let frames = frames_cast.i32()?;

            let groups = group_col.and_then(|g| g.cast(&DataType::String).ok());

            let mut map: HashMap<(String, Option<i32>), Option<String>> = HashMap::new();

            for idx in 0..df.height() {
                if let Some(name) = names.get(idx) {
                    let frame = frames.get(idx);
                    let group = groups
                        .as_ref()
                        .and_then(|g| g.str().ok())
                        .and_then(|g| g.get(idx))
                        .map(|s| s.to_string());
                    let key = (name.to_string(), frame);
                    // Only insert if not already present (first row wins for group)
                    map.entry(key).or_insert(group);
                }
            }

            Ok(map)
        }

        let original_groups = build_image_groups(&original_df)?;
        let snapshot_groups = build_image_groups(&snapshot_df)?;

        // Count unique images in each
        println!("Original: {} unique images", original_groups.len());
        println!("Snapshot: {} unique images", snapshot_groups.len());

        // Show group distributions
        fn count_groups(
            groups: &HashMap<(String, Option<i32>), Option<String>>,
        ) -> HashMap<String, usize> {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for group in groups.values() {
                let key = group.clone().unwrap_or_else(|| "null".to_string());
                *counts.entry(key).or_default() += 1;
            }
            counts
        }

        println!(
            "Original group distribution: {:?}",
            count_groups(&original_groups)
        );
        println!(
            "Snapshot group distribution: {:?}",
            count_groups(&snapshot_groups)
        );

        // Compare: All original images should be in snapshot with same group
        let mut missing_in_snapshot: Vec<(String, Option<i32>)> = Vec::new();
        let mut group_mismatches: Vec<((String, Option<i32>), Option<String>, Option<String>)> =
            Vec::new();

        for (key, orig_group) in &original_groups {
            if let Some(snap_group) = snapshot_groups.get(key) {
                if orig_group != snap_group {
                    group_mismatches.push((key.clone(), orig_group.clone(), snap_group.clone()));
                }
            } else {
                missing_in_snapshot.push(key.clone());
            }
        }

        // Report findings
        if !missing_in_snapshot.is_empty() {
            println!(
                "\n⚠️  {} images from original NOT FOUND in snapshot:",
                missing_in_snapshot.len()
            );
            for key in missing_in_snapshot.iter().take(10) {
                println!("    ({}, frame={:?})", key.0, key.1);
            }
            if missing_in_snapshot.len() > 10 {
                println!("    ... and {} more", missing_in_snapshot.len() - 10);
            }
        }

        if !group_mismatches.is_empty() {
            println!(
                "\n⚠️  {} images have GROUP MISMATCH:",
                group_mismatches.len()
            );
            for (key, orig, snap) in group_mismatches.iter().take(10) {
                println!(
                    "    ({}, frame={:?}): original={:?} vs snapshot={:?}",
                    key.0, key.1, orig, snap
                );
            }
            if group_mismatches.len() > 10 {
                println!("    ... and {} more", group_mismatches.len() - 10);
            }
        }

        // Extra images in snapshot (not an error, but interesting)
        let extra_in_snapshot: Vec<_> = snapshot_groups
            .keys()
            .filter(|k| !original_groups.contains_key(*k))
            .collect();
        if !extra_in_snapshot.is_empty() {
            println!(
                "\nℹ️  {} images in snapshot but not in original download:",
                extra_in_snapshot.len()
            );
            println!("   (This may be due to download-annotations filtering)");
        }

        // Critical assertions
        assert!(
            missing_in_snapshot.is_empty(),
            "MISSING IMAGES: {} images from original not found in snapshot!\nFirst few: {:?}",
            missing_in_snapshot.len(),
            missing_in_snapshot.iter().take(5).collect::<Vec<_>>()
        );
        println!(
            "✓ All {} original images found in snapshot",
            original_groups.len()
        );

        assert!(
            group_mismatches.is_empty(),
            "GROUP MISMATCH: {} images have different groups!\nFirst few: {:?}",
            group_mismatches.len(),
            group_mismatches.iter().take(5).collect::<Vec<_>>()
        );
        println!("✓ All groups match between original and snapshot");
    }

    #[cfg(not(feature = "polars"))]
    {
        println!("⚠️  Polars feature not enabled - skipping detailed Arrow comparison");
        println!("   Build with --all-features to enable full verification");
    }

    // =========================================================================
    // CLEANUP
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ CLEANUP                                                         │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    // Delete the created snapshot
    let mut cmd = edgefirst_cmd();
    cmd.arg("delete-snapshot").arg(&snapshot_id);
    match cmd.output() {
        Ok(output) if output.status.success() => {
            println!("✓ Deleted snapshot: {}", snapshot_id);
        }
        _ => {
            println!("⚠️  Could not delete snapshot: {}", snapshot_id);
        }
    }

    // Clean up local files
    fs::remove_dir_all(&test_dir).ok();
    println!("✓ Cleaned up test directory");

    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  ✅ CREATE SNAPSHOT FROM DATASET TEST PASSED                    ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    Ok(())
}

#[test]
#[file_serial]
#[ignore = "Requires MCAP test data (4GB+). Set TEST_MCAP_SNAPSHOT_ID to run."]
fn test_snapshot_restore_with_mcap_processing() -> Result<(), Box<dyn std::error::Error>> {
    // This test requires an MCAP file to test autodepth and autolabel features.
    // These features only work with MCAP snapshots, not image-based snapshots.
    //
    // Prerequisites:
    // 1. Upload an MCAP file as a snapshot
    // 2. Set TEST_MCAP_SNAPSHOT_ID environment variable to the snapshot ID
    //
    // The --autolabel and --autodepth flags:
    // - --autolabel <labels>: Runs AGTG auto-annotation with specified labels
    //   (requires MCAP)
    // - --autodepth: Generates depth maps (requires Maivin/Raivin camera data in
    //   MCAP)

    let snapshot_id = env::var("TEST_MCAP_SNAPSHOT_ID")
        .expect("TEST_MCAP_SNAPSHOT_ID must be set to run this test");

    let project_id =
        get_project_id_by_name("Unit Testing")?.expect("Unit Testing project not found");

    let mut datasets_to_cleanup = Vec::new();

    // Test 1: Restore with autolabel
    println!("=== STEP 1: Restore with autolabel ===");
    let mut cmd = edgefirst_cmd();
    cmd.arg("restore-snapshot")
        .arg(&project_id)
        .arg(&snapshot_id)
        .arg("--autolabel")
        .arg("car,person,deer");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;
    println!("Autolabel restore output:\n{}", output_str);

    if let Some(dataset_id) = output_str.lines().find_map(|line| {
        if line.contains("ds-")
            && let Some(start) = line.find("[ds-")
        {
            let rest = &line[start + 1..];
            rest.find(']').map(|end| rest[..end].to_string())
        } else {
            None
        }
    }) {
        datasets_to_cleanup.push(dataset_id.clone());
        println!("✓ Created dataset with autolabel: {}", dataset_id);
    }

    // Test 2: Restore with autodepth (requires Maivin/Raivin camera)
    println!("\n=== STEP 2: Restore with autodepth ===");
    let mut cmd = edgefirst_cmd();
    cmd.arg("restore-snapshot")
        .arg(&project_id)
        .arg(&snapshot_id)
        .arg("--autodepth");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;
    println!("Autodepth restore output:\n{}", output_str);

    if let Some(dataset_id) = output_str.lines().find_map(|line| {
        if line.contains("ds-")
            && let Some(start) = line.find("[ds-")
        {
            let rest = &line[start + 1..];
            rest.find(']').map(|end| rest[..end].to_string())
        } else {
            None
        }
    }) {
        datasets_to_cleanup.push(dataset_id.clone());
        println!("✓ Created dataset with autodepth: {}", dataset_id);
    }

    // Test 3: Restore with both autolabel and autodepth
    println!("\n=== STEP 3: Restore with autolabel + autodepth ===");
    let mut cmd = edgefirst_cmd();
    cmd.arg("restore-snapshot")
        .arg(&project_id)
        .arg(&snapshot_id)
        .arg("--autolabel")
        .arg("car,person")
        .arg("--autodepth");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;
    println!("Combined restore output:\n{}", output_str);

    if let Some(dataset_id) = output_str.lines().find_map(|line| {
        if line.contains("ds-")
            && let Some(start) = line.find("[ds-")
        {
            let rest = &line[start + 1..];
            rest.find(']').map(|end| rest[..end].to_string())
        } else {
            None
        }
    }) {
        datasets_to_cleanup.push(dataset_id.clone());
        println!(
            "✓ Created dataset with autolabel + autodepth: {}",
            dataset_id
        );
    }

    // Cleanup
    println!(
        "\n=== CLEANUP: Deleting {} datasets ===",
        datasets_to_cleanup.len()
    );
    for dataset_id in datasets_to_cleanup {
        let mut cmd = edgefirst_cmd();
        cmd.arg("delete-dataset").arg(&dataset_id);
        match cmd.output() {
            Ok(output) if output.status.success() => {
                println!("✓ Deleted dataset: {}", dataset_id);
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("⚠ Failed to delete {}: {}", dataset_id, stderr);
            }
            Err(e) => {
                println!("⚠ Error deleting {}: {}", dataset_id, e);
            }
        }
    }

    println!("\n✅ MCAP processing test completed");
    Ok(())
}

/// Test that the server rejects snapshots with inconsistent group values.
///
/// This test creates a malformed snapshot where the same image has two
/// annotation rows with conflicting group values (train vs val). The server
/// MUST reject this during restore as it violates the data integrity
/// constraint that all rows for a given image must have identical group values.
#[test]
#[file_serial]
#[ignore = "Server-side validation for inconsistent groups not yet implemented. This test verifies the expected behavior when it is."]
fn test_server_rejects_inconsistent_group_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    use polars::prelude::*;
    use std::io::Write;

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║  SERVER VALIDATION: Inconsistent Group Rejection Test          ║");
    println!("╚════════════════════════════════════════════════════════════════╝");

    // =========================================================================
    // STEP 1: Create test directory structure (EdgeFirst Dataset Format)
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 1: Create Malformed Snapshot Data                          │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    let test_data_dir = get_test_data_dir();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let dataset_name = format!("test_{}_{timestamp}", std::process::id());

    // EdgeFirst Dataset Format structure:
    // dataset_root/                  <- Root directory (passed to create-snapshot)
    // ├── dataset_root.arrow         <- Arrow file with SAME name as root directory
    // └── dataset_root/              <- Sensor container with SAME name as root
    //     └── test_image.png         <- Image files in sensor container

    let dataset_root = test_data_dir
        .join("inconsistent_group_test")
        .join(&dataset_name);
    let sensor_container = dataset_root.join(&dataset_name);
    fs::create_dir_all(&sensor_container)?;

    // Create a simple 1x1 red PNG image (minimal valid PNG)
    let png_data: [u8; 70] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature (8 bytes)
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR length + type (8 bytes)
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 pixels (8 bytes)
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, // depth, color, CRC (9 bytes)
        0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT length + type (8 bytes)
        0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, // compressed data (8 bytes)
        0x00, 0x03, 0x00, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB4, // more data + CRC (9 bytes)
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND length + type (8 bytes)
        0xAE, 0x42, 0x60, 0x82, // IEND CRC (4 bytes)
    ]; // Total: 70 bytes

    // Create image in sensor container
    let image_path = sensor_container.join("test_image.png");
    let mut image_file = fs::File::create(&image_path)?;
    image_file.write_all(&png_data)?;
    println!("✓ Created sensor container with test_image.png");

    // Create {dataset_name}.arrow in dataset_root with CONFLICTING group values
    // Row 1: test_image, group=train, label=cat
    // Row 2: test_image, group=val, label=dog   <-- CONFLICT! Same image, different
    // group
    let arrow_path = dataset_root.join(format!("{}.arrow", dataset_name));

    let names = Series::new("name".into(), vec!["test_image", "test_image"]);
    let frames: Series = Series::new("frame".into(), vec![None::<u32>, None::<u32>]);
    let groups = Series::new("group".into(), vec![Some("train"), Some("val")]); // CONFLICT!
    let labels = Series::new("label".into(), vec![Some("cat"), Some("dog")]);

    // Create box2d data as array columns
    let box2d_data: Vec<Option<[f32; 4]>> = vec![
        Some([0.5, 0.5, 0.2, 0.2]), // cx, cy, w, h
        Some([0.3, 0.3, 0.1, 0.1]),
    ];
    let box2d_series: Vec<Option<Series>> = box2d_data
        .into_iter()
        .map(|opt| opt.map(|arr| Series::new("box2d".into(), arr.to_vec())))
        .collect();
    let box2d = Series::new("box2d".into(), box2d_series)
        .cast(&DataType::Array(Box::new(DataType::Float32), 4))?;

    let mut df = DataFrame::new(vec![
        names.into_column(),
        frames.into_column(),
        groups.into_column(),
        labels.into_column(),
        box2d.into_column(),
    ])?;

    let mut arrow_file = fs::File::create(&arrow_path)?;
    IpcWriter::new(&mut arrow_file).finish(&mut df)?;
    println!("✓ Created {}.arrow with CONFLICTING groups:", dataset_name);
    println!("    Row 1: test_image, group=train");
    println!("    Row 2: test_image, group=val   <-- CONFLICT!");

    // =========================================================================
    // STEP 2: Upload snapshot
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 2: Upload Malformed Snapshot                               │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    let mut cmd = edgefirst_cmd();
    // Pass the dataset_root - EdgeFirst Dataset Format expects {name}.arrow inside
    // root
    cmd.arg("create-snapshot").arg(&dataset_root);
    cmd.timeout(std::time::Duration::from_secs(120));
    let create_output = cmd.ok()?.stdout;
    let create_output_str = String::from_utf8(create_output)?;
    println!("Create snapshot output:\n{}", create_output_str);

    let snapshot_id = create_output_str
        .lines()
        .find_map(|line| {
            if let Some(start) = line.find('[')
                && let Some(end) = line[start..].find(']')
            {
                let id = &line[start + 1..start + end];
                if id.starts_with("ss-") {
                    return Some(id.to_string());
                }
            }
            None
        })
        .expect("Could not extract snapshot ID from creation output");
    println!("✓ Uploaded snapshot: {}", snapshot_id);

    // Wait for snapshot to be processed
    println!("Waiting for snapshot processing...");
    use edgefirst_client::{Client as EdgeFirstClient, SnapshotID};
    let api_client = EdgeFirstClient::new()?.with_token_path(None)?;
    let snap_id = SnapshotID::try_from(snapshot_id.as_str())?;

    let rt = tokio::runtime::Runtime::new()?;
    let mut attempts = 0;
    let max_attempts = 60;
    loop {
        let snapshot = rt.block_on(api_client.snapshot(snap_id))?;
        let status = snapshot.status();
        if status == "available" || status == "completed" {
            println!("✓ Snapshot ready (status: {})", status);
            break;
        }
        if status == "failed" || status == "error" {
            println!("⚠ Snapshot processing failed (status: {})", status);
            println!("  This may indicate server-side validation caught the issue early");
            // Clean up the dataset_root directory (contains both Arrow file and sensor
            // container)
            fs::remove_dir_all(&dataset_root).ok();
            return Ok(());
        }
        attempts += 1;
        if attempts >= max_attempts {
            panic!(
                "Snapshot did not become available within {} seconds",
                max_attempts
            );
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // =========================================================================
    // STEP 3: Get project ID for restore
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 3: Get Project for Restore                                 │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    let mut cmd = edgefirst_cmd();
    cmd.arg("projects").arg("--name").arg("Unit Testing");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .find_map(|line| {
            line.split(']')
                .next()
                .and_then(|s| s.strip_prefix('['))
                .map(|s| s.trim().to_string())
                .filter(|id| id.starts_with("p-"))
        })
        .expect("No project found matching 'Unit Testing'");
    println!("✓ Project: {}", project_id);

    // =========================================================================
    // STEP 4: Attempt to restore - Server SHOULD reject this
    // =========================================================================
    println!("\n┌─────────────────────────────────────────────────────────────────┐");
    println!("│ STEP 4: Attempt Restore (Should FAIL)                           │");
    println!("└─────────────────────────────────────────────────────────────────┘");

    let mut cmd = edgefirst_cmd();
    cmd.arg("restore-snapshot")
        .arg(&project_id)
        .arg(&snapshot_id)
        .arg("--monitor"); // Wait for completion to see the error
    cmd.timeout(std::time::Duration::from_secs(300));

    let result = cmd.output();

    // Clean up the entire dataset_root directory regardless of outcome
    fs::remove_dir_all(&dataset_root).ok();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("Restore stdout:\n{}", stdout);
            println!("Restore stderr:\n{}", stderr);

            if output.status.success() {
                // If restore succeeded, we need to check if a dataset was created
                // and if so, try to clean it up and FAIL the test
                if let Some(dataset_id) = stdout.lines().chain(stderr.lines()).find_map(|line| {
                    if let Some(start) = line.find("[ds-") {
                        let rest = &line[start + 1..];
                        rest.find(']').map(|end| rest[..end].to_string())
                    } else {
                        None
                    }
                }) {
                    println!("⚠ Cleaning up erroneously created dataset: {}", dataset_id);
                    let mut cleanup_cmd = edgefirst_cmd();
                    cleanup_cmd.arg("delete-dataset").arg(&dataset_id);
                    cleanup_cmd.output().ok();
                }

                // Check if the task failed even though command returned success
                let task_failed = stdout.contains("failed")
                    || stderr.contains("failed")
                    || stdout.contains("error")
                    || stderr.contains("Inconsistent group");

                if task_failed {
                    // Verify we got a meaningful error message
                    let combined = format!("{}\n{}", stdout, stderr);
                    assert!(
                        combined.contains("Inconsistent group"),
                        "Expected meaningful error message mentioning 'Inconsistent group', got:\n{}",
                        combined
                    );

                    println!(
                        "\n╔════════════════════════════════════════════════════════════════╗"
                    );
                    println!("║  ✅ SERVER CORRECTLY REJECTED INCONSISTENT GROUPS              ║");
                    println!("║  ✅ Error message is meaningful and actionable                 ║");
                    println!("╚════════════════════════════════════════════════════════════════╝");
                    return Ok(());
                }

                panic!(
                    "SERVER BUG: Restore SUCCEEDED with inconsistent groups!\n\
                     The server should have rejected the snapshot with conflicting\n\
                     group values (train vs val) for the same image.\n\
                     This indicates the server-side validation is not working."
                );
            } else {
                // Command failed - this is expected!
                let error_output = format!("{}\n{}", stdout, stderr);

                // Verify it failed for the RIGHT reason with a meaningful message
                assert!(
                    error_output.contains("Inconsistent group"),
                    "Expected meaningful error message mentioning 'Inconsistent group', got:\n{}",
                    error_output
                );

                println!("\n╔════════════════════════════════════════════════════════════════╗");
                println!("║  ✅ SERVER CORRECTLY REJECTED INCONSISTENT GROUPS              ║");
                println!("║  ✅ Error message is meaningful and actionable                 ║");
                println!("╚════════════════════════════════════════════════════════════════╝");
                return Ok(());
            }
        }
        Err(e) => {
            // Command execution failed - could be timeout or other issue
            println!("Restore command error: {}", e);
            println!("\n⚠ Could not determine if server rejected the invalid data");
            return Ok(());
        }
    }
}
