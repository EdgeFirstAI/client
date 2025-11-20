// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use assert_cmd::Command;
use base64::Engine as _;
use chrono::Utc;
use directories::ProjectDirs;
use serial_test::serial;
use std::{
    collections::{BTreeSet, HashMap},
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

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
    let normalized_name = if dataset.starts_with("ds-") {
        format!("dataset-{}", &dataset[3..])
    } else {
        dataset.to_lowercase().replace(' ', "-")
    };
    get_test_data_dir().join(format!("{}-test", normalized_name))
}

fn get_project_id_by_name(name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
/// - A dataset name: Searches all projects for exact name match
fn get_dataset_and_first_annotation_set(
    dataset: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let dataset_id = if dataset.starts_with("ds-") {
        // It's a dataset ID - verify it exists
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
        cmd.arg("dataset").arg(dataset);

        let result = cmd.ok();
        if result.is_err() {
            return Err(format!("Dataset ID '{}' not found", dataset).into());
        }
        dataset.to_string()
    } else {
        // It's a dataset name - search all projects for exact match
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
        cmd.arg("datasets").arg("--name").arg(dataset);

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        // Parse output and find exact name match
        // Output format: [ds-xxx] Dataset Name: project_name
        output_str
            .lines()
            .filter_map(|line| {
                let (id_part, rest) = line.split_once(']')?;
                let id = id_part.strip_prefix('[')?.trim();
                let name_and_project = rest.trim();
                let name = name_and_project.split(':').next()?.trim();
                Some((id.to_string(), name.to_string()))
            })
            .find(|(_, name)| name == dataset)
            .map(|(id, _)| id)
            .ok_or_else(|| format!("Dataset '{}' not found in any project", dataset))?
    };

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("download-dataset")
        .arg(dataset_id)
        .arg("--output")
        .arg(&download_dir);
    cmd.assert().success();

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

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    } else if original_has_group || redownloaded_has_group {
        println!("⚠️  Warning: One file has groups but the other doesn't");
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

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("version");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains(env!("CARGO_PKG_VERSION")));
    Ok(())
}

#[test]
fn test_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("organization");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Organization:"));
    Ok(())
}

#[test]
fn test_organization_details() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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

#[test]
#[serial]
fn test_login() -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs, path::PathBuf, time::SystemTime};

    // Get credentials from environment (required for authentication tests)
    let username =
        env::var("STUDIO_USERNAME").expect("STUDIO_USERNAME must be set for authentication tests");
    let _password =
        env::var("STUDIO_PASSWORD").expect("STUDIO_PASSWORD must be set for authentication tests");

    // Determine token path (same logic as in client.rs)
    let token_path = ProjectDirs::from("ai", "EdgeFirst", "EdgeFirst Studio")
        .map(|d| d.config_dir().join("token"))
        .unwrap_or_else(|| PathBuf::from(".edgefirst_token"));

    // Record timestamp before login
    let time_before = SystemTime::now();
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Run login command with environment variables
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("login");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Verify success message
    assert!(output_str.contains("Successfully logged into EdgeFirst Studio"));
    assert!(output_str.contains(&username));
    println!("Login output:\n{}", output_str);

    // Verify token file was created/updated
    assert!(token_path.exists(), "Token file should exist after login");

    // Verify token file was modified after we started
    let metadata = fs::metadata(&token_path)?;
    let modified_time = metadata.modified()?;
    assert!(
        modified_time > time_before,
        "Token file should be updated after login"
    );

    // Read and validate the token
    let token_content = fs::read_to_string(&token_path)?;
    assert!(!token_content.is_empty(), "Token file should not be empty");

    // Validate JWT token format and username
    let token_parts: Vec<&str> = token_content.trim().split('.').collect();
    assert_eq!(
        token_parts.len(),
        3,
        "Token should be a valid JWT with 3 parts"
    );

    let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(token_parts[1])
        .expect("Token payload should be valid base64");
    let payload: HashMap<String, serde_json::Value> =
        serde_json::from_slice(&decoded).expect("Token payload should be valid JSON");

    let token_username = payload
        .get("username")
        .and_then(|v| v.as_str())
        .expect("Token should contain username field");

    assert_eq!(
        token_username, username,
        "Token username should match login username"
    );

    println!("✓ Token file created at: {:?}", token_path);
    println!("✓ Token contains correct username: {}", token_username);

    Ok(())
}

#[test]
#[serial]
fn test_logout() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    // First, ensure we're logged in by running login
    let _username =
        env::var("STUDIO_USERNAME").expect("STUDIO_USERNAME must be set for authentication tests");
    let _password =
        env::var("STUDIO_PASSWORD").expect("STUDIO_PASSWORD must be set for authentication tests");

    let token_path = ProjectDirs::from("ai", "EdgeFirst", "EdgeFirst Studio")
        .map(|d| d.config_dir().join("token"))
        .unwrap_or_else(|| PathBuf::from(".edgefirst_token"));

    // Login first to ensure token exists
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("login");
    cmd.ok()?;

    // Verify token file exists before logout
    assert!(token_path.exists(), "Token file should exist before logout");

    // Run logout command
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("logout");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    // Verify success message
    assert!(output_str.contains("Successfully logged out of EdgeFirst Studio"));
    println!("Logout output:\n{}", output_str);

    // Verify token file was removed
    assert!(
        !token_path.exists(),
        "Token file should be removed after logout"
    );

    println!("✓ Token file removed: {:?}", token_path);

    // Re-login for other tests (cleanup)
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("login");
    cmd.ok()?;

    Ok(())
}

#[test]
#[serial]
fn test_sleep_30_seconds() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;

    let start = Instant::now();

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("sleep").arg("30");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let elapsed = start.elapsed();

    println!("Sleep output:\n{}", output_str);
    println!("Elapsed time: {:?}", elapsed);

    assert!(output_str.contains("Sleeping for 30 seconds"));
    assert!(output_str.contains("Sleep complete"));
    assert!(
        elapsed.as_secs() >= 30,
        "Sleep should take at least 30 seconds"
    );

    Ok(())
}

#[test]
#[serial]
fn test_logout_after_sleep() -> Result<(), Box<dyn std::error::Error>> {
    use std::{path::PathBuf, time::Instant};

    let _username =
        env::var("STUDIO_USERNAME").expect("STUDIO_USERNAME must be set for authentication tests");

    let token_path = ProjectDirs::from("ai", "EdgeFirst", "EdgeFirst Studio")
        .map(|d| d.config_dir().join("token"))
        .unwrap_or_else(|| PathBuf::from(".edgefirst_token"));

    // Login first
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("login");
    cmd.ok()?;

    assert!(token_path.exists(), "Token file should exist before logout");

    let start = Instant::now();

    // Run logout command
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("logout");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let elapsed = start.elapsed();

    println!("Logout output:\n{}", output_str);
    println!("Logout elapsed time: {:?}", elapsed);

    // If serial is working, this should complete quickly
    // If tests run in parallel and sleep blocks logout, this will be slow
    assert!(
        elapsed.as_secs() < 5,
        "Logout should complete in under 5 seconds, took {:?}",
        elapsed
    );

    assert!(output_str.contains("Successfully logged out of EdgeFirst Studio"));
    assert!(
        !token_path.exists(),
        "Token file should be removed after logout"
    );

    // Re-login for other tests
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("login");
    cmd.ok()?;

    Ok(())
}

#[test]
#[serial]
fn test_login_creates_new_token() -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs, path::PathBuf};

    let _username =
        env::var("STUDIO_USERNAME").expect("STUDIO_USERNAME must be set for authentication tests");

    let token_path = ProjectDirs::from("ai", "EdgeFirst", "EdgeFirst Studio")
        .map(|d| d.config_dir().join("token"))
        .unwrap_or_else(|| PathBuf::from(".edgefirst_token"));

    // First login
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("login");
    cmd.ok()?;

    let first_token = fs::read_to_string(&token_path)?;
    let first_modified = fs::metadata(&token_path)?.modified()?;

    // Wait a bit to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Second login
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("login");
    cmd.ok()?;

    let second_token = fs::read_to_string(&token_path)?;
    let second_modified = fs::metadata(&token_path)?.modified()?;

    // Tokens should be different (new token issued)
    assert_ne!(
        first_token, second_token,
        "Login should create a new token each time"
    );

    // File should be newer
    assert!(
        second_modified > first_modified,
        "Token file should be updated on re-login"
    );

    println!("✓ First token and second token are different");
    println!("✓ Token file timestamp updated");

    Ok(())
}

#[test]
#[serial]
fn test_corrupted_token_handling() -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs, path::PathBuf};

    let _username =
        env::var("STUDIO_USERNAME").expect("STUDIO_USERNAME must be set for authentication tests");

    let token_path = ProjectDirs::from("ai", "EdgeFirst", "EdgeFirst Studio")
        .map(|d| d.config_dir().join("token"))
        .unwrap_or_else(|| PathBuf::from(".edgefirst_token"));

    // Login first to create a valid token
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("login");
    cmd.ok()?;

    assert!(token_path.exists(), "Token file should exist after login");

    // Corrupt the token file with invalid data
    fs::write(&token_path, "this.is.corrupted")?;
    println!("✓ Corrupted token file created");

    // Try to run a command that requires authentication WITHOUT credentials
    // This should gracefully handle the corrupted token
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("organization");
    // Clear environment variables so the command can't auto-login
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects");
    cmd.assert().success();
    Ok(())
}

#[test]
fn test_projects_filter_by_name() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("datasets");
    cmd.assert().success();
    Ok(())
}

#[test]
fn test_datasets_by_project() -> Result<(), Box<dyn std::error::Error>> {
    // First get project ID
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
        cmd.arg("datasets").arg(&id);
        cmd.assert().success();
    }

    Ok(())
}

#[test]
fn test_datasets_with_labels() -> Result<(), Box<dyn std::error::Error>> {
    // Get Sample Project with COCO dataset
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Sample Project");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Sample Project");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
            cmd.arg("dataset").arg(&ds_id);
            cmd.assert().success();
        }
    }

    Ok(())
}

#[test]
#[serial]
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

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
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

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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

    let dataset_lower = dataset.to_lowercase().replace("ds-", "dataset-");
    let cache_file = get_test_data_dir().join(format!("{}_upload_latest.txt", dataset_lower));
    fs::write(
        &cache_file,
        format!(
            "dataset_id={}\nannotation_set_id={}\ncreated_at={}\nsource_dataset={}\n",
            new_dataset_id, new_annotation_set_id, timestamp, source_dataset_id
        ),
    )?;

    println!(
        "Persistent QA dataset created: {} (annotation set {})",
        new_dataset_id, new_annotation_set_id
    );
    println!("Images uploaded from: {:?}", images_dir);
    println!("Annotations uploaded from: {:?}", annotations_path);
    println!("Recorded IDs in {:?}", cache_file);

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
/// Tests complete workflow: Download → Upload → Download → Compare
///
/// Dataset: Configurable via TEST_DATASET_NAME env var (default: "Deer")
/// Requirements:
/// - Dataset must exist in "Unit Testing" project
/// - Must have at least one annotation set
/// - Supports mixed sensors, annotation types, and sequences
///
/// **Note**: This test uploads 1600+ samples and requires extended timeout.
/// Run with: `EDGEFIRST_TIMEOUT=120 cargo test --package edgefirst-cli
/// test_dataset_roundtrip -- --ignored --nocapture`
#[test]
#[serial]
#[ignore = "Requires EDGEFIRST_TIMEOUT=120 due to large dataset upload (1600+ samples). Run with: cargo test test_dataset_roundtrip -- --ignored"]
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

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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

    // Cleanup
    fs::remove_dir_all(&original_images).ok();
    fs::remove_file(&original_annotations).ok();
    fs::remove_dir_all(&redownloaded_images).ok();
    fs::remove_file(&redownloaded_annotations).ok();

    Ok(())
}

// ===== Experiment and Training Session Tests =====

#[test]
fn test_experiments_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
        cmd.arg("experiments").arg(&id);
        cmd.assert().success();
    }

    Ok(())
}

#[test]
fn test_experiments_filter_by_name() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
            cmd.arg("training-sessions").arg(&id);
            cmd.assert().success();
        }
    }

    Ok(())
}

#[test]
fn test_training_sessions_filter_by_name() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
                let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
                let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
                let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
                let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
fn test_download_artifact() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let proj_id = extract_first_id(&output_str).ok_or("Failed to extract project ID")?;

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("experiments")
        .arg(&proj_id)
        .arg("--name")
        .arg("Unit Testing");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let exp_id = find_experiment_id(&output_str).ok_or("Failed to find experiment ID")?;

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("training-sessions")
        .arg(&exp_id)
        .arg("--name")
        .arg("modelpack-960x540");
    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let session_id = find_training_session_id(&output_str, "modelpack-960x540")
        .ok_or("Failed to find training session")?;

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
fn test_upload_artifact() -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs::File, io::Write};

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("projects").arg("--name").arg("Unit Testing");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    let project_id = output_str
        .lines()
        .next()
        .and_then(|line| line.split(']').next())
        .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

    if let Some(proj_id) = project_id {
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
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

                let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("tasks");
    cmd.assert().success();
    Ok(())
}

#[test]
fn test_tasks_with_name_filter() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("tasks").arg("--name").arg("modelpack-usermanaged");

    let output = cmd.ok()?.stdout;
    let output_str = String::from_utf8(output)?;

    println!("Tasks with name filter:\n{}", output_str);
    Ok(())
}

#[test]
fn test_tasks_with_stages() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
fn test_upload_dataset_missing_parameters() -> Result<(), Box<dyn std::error::Error>> {
    // Get Test Labels dataset
    let (dataset_id, _annotation_set_id) = get_test_labels_dataset()?;

    // Try to run upload-dataset with NEITHER annotations NOR images (should fail)
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
fn test_upload_dataset_invalid_path() -> Result<(), Box<dyn std::error::Error>> {
    // Get Test Labels dataset
    let (dataset_id, _annotation_set_id) = get_test_labels_dataset()?;

    // Try to upload with non-existent path
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
fn test_dataset_crud() -> Result<(), Box<dyn std::error::Error>> {
    // Get Unit Testing project
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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
#[serial]
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
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("download-dataset")
        .arg(&dataset_id)
        .arg("--output")
        .arg(&normal_dir);
    cmd.assert().success();

    // Download with flattened structure
    let flatten_dir = downloads_root.join(format!("flatten_{}_{}", std::process::id(), timestamp));
    fs::create_dir_all(&flatten_dir)?;

    println!("Downloading dataset with --flatten option...");
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
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

    println!("\n✅ Flatten option test completed successfully");
    Ok(())
}
