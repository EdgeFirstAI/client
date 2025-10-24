// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use assert_cmd::Command;
use base64::Engine as _;
use directories::ProjectDirs;
use serial_test::serial;
use std::{collections::HashMap, env, path::PathBuf};

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

    std::fs::create_dir_all(&test_dir).expect("Failed to create test data directory");
    test_dir
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
fn test_download_dataset() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // Get the Unit Testing project
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
        // Get datasets and find "Deer" dataset
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
        cmd.arg("datasets").arg(&proj_id);

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        // Find the Deer dataset by name
        let deer_dataset = output_str
            .lines()
            .find(|line| line.contains("Deer"))
            .and_then(|line| line.split(']').next())
            .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

        if let Some(ds_id) = deer_dataset {
            // Use target/testdata directory for the download
            let test_dir = get_test_data_dir();
            let dataset_dir = test_dir.join(format!("deer_dataset_{}", std::process::id()));
            fs::create_dir_all(&dataset_dir)?;

            // Download the dataset to test directory using --output flag
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
            cmd.arg("download-dataset")
                .arg(&ds_id)
                .arg("--output")
                .arg(&dataset_dir);

            let result = cmd.assert().try_success();

            if result.is_ok() {
                // Verify the download created files
                assert!(dataset_dir.exists(), "Download directory should exist");

                // Check if any files were downloaded
                let entries: Vec<_> = fs::read_dir(&dataset_dir)?.filter_map(|e| e.ok()).collect();

                assert!(!entries.is_empty(), "Dataset download should create files");
                println!("Downloaded {} files to {:?}", entries.len(), dataset_dir);
            }

            // Clean up
            fs::remove_dir_all(&dataset_dir)?;
        }
    }

    Ok(())
}

#[test]
fn test_download_annotations() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // Get the Unit Testing project
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
        // Get datasets and find "Deer" dataset
        let mut cmd = Command::cargo_bin("edgefirst-client")?;
        cmd.arg("datasets").arg(&proj_id);

        let output = cmd.ok()?.stdout;
        let output_str = String::from_utf8(output)?;

        // Find the Deer dataset by name
        let deer_dataset = output_str
            .lines()
            .find(|line| line.contains("Deer"))
            .and_then(|line| line.split(']').next())
            .and_then(|s| s.trim_start_matches('[').parse::<String>().ok());

        if let Some(ds_id) = deer_dataset {
            // Get dataset details with annotation sets
            let mut cmd = Command::cargo_bin("edgefirst-client")?;
            cmd.arg("dataset").arg(&ds_id).arg("--annotation-sets");

            let output = cmd.ok()?.stdout;
            let output_str = String::from_utf8(output)?;

            // Find the first annotation set ID (starts with "as-")
            let annotation_set = output_str
                .lines()
                .find(|line| line.trim().starts_with("as-"))
                .and_then(|line| line.split_whitespace().next())
                .map(|s| s.to_string());

            if let Some(as_id) = annotation_set {
                // Use target/testdata directory for downloads
                let test_dir = get_test_data_dir();

                // Test JSON format download
                let json_file =
                    test_dir.join(format!("deer_annotations_{}.json", std::process::id()));

                let mut cmd = Command::cargo_bin("edgefirst-client")?;
                cmd.arg("download-annotations").arg(&as_id).arg(&json_file);

                cmd.assert().success();

                // Verify the JSON file was created
                assert!(json_file.exists(), "JSON annotations file should exist");
                assert!(
                    json_file.metadata()?.len() > 0,
                    "JSON annotations file should not be empty"
                );
                println!("Downloaded annotations to {:?}", json_file);

                // Clean up JSON file
                fs::remove_file(&json_file)?;

                // Test Arrow format download
                let arrow_file =
                    test_dir.join(format!("deer_annotations_{}.arrow", std::process::id()));

                let mut cmd = Command::cargo_bin("edgefirst-client")?;
                cmd.arg("download-annotations").arg(&as_id).arg(&arrow_file);

                cmd.assert().success();

                // Verify the Arrow file was created
                assert!(arrow_file.exists(), "Arrow annotations file should exist");
                assert!(
                    arrow_file.metadata()?.len() > 0,
                    "Arrow annotations file should not be empty"
                );
                println!("Downloaded annotations to {:?}", arrow_file);

                // Clean up Arrow file
                fs::remove_file(&arrow_file)?;
            }
        }
    }

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

#[test]
fn test_download_artifact() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

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

                // Extract first artifact name
                let artifact_name = output_str
                    .lines()
                    .find(|line| line.trim().starts_with("- "))
                    .map(|line| line.trim().trim_start_matches("- ").to_string());

                if let Some(name) = artifact_name {
                    // Use target/testdata directory for downloads
                    let test_dir = get_test_data_dir();
                    let output_file =
                        test_dir.join(format!("artifact_{}_{}", std::process::id(), name));

                    // Clean up any existing file
                    if output_file.exists() {
                        fs::remove_file(&output_file)?;
                    }

                    let mut cmd = Command::cargo_bin("edgefirst-client")?;
                    cmd.arg("download-artifact")
                        .arg(&sid)
                        .arg(&name)
                        .arg("--output")
                        .arg(&output_file);

                    cmd.assert().success();

                    // Verify file was downloaded
                    assert!(output_file.exists());
                    println!("Downloaded artifact to {:?}", output_file);

                    // Clean up
                    fs::remove_file(&output_file)?;
                }
            }
        }
    }

    Ok(())
}

#[test]
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
                std::fs::remove_file(test_file)?;
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

/// Helper to get path to test data
fn get_deer_test_data_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("testdata")
        .join("deer-test")
}

#[test]
#[serial]
fn test_upload_dataset_full_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Get Test Labels dataset for write operations
    let (dataset_id, annotation_set_id) = get_test_labels_dataset()?;

    // Get test data paths
    let test_data_dir = get_deer_test_data_path();
    let annotations_path = test_data_dir.join("deer-stage.arrow");
    let images_path = test_data_dir.join("deer");

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
    let test_data_dir = get_deer_test_data_path();
    // Use deer-stage.arrow which matches the downloaded images better
    let annotations_path = test_data_dir.join("deer-stage.arrow");

    // Verify test data exists
    if !annotations_path.exists() {
        eprintln!("⚠️  Test data not found");
        eprintln!("    Skipping test - run download tests first to populate test data");
        return Ok(());
    }

    // Test auto-discovery: For deer-stage.arrow, try to find folder/zip
    // Since we have deer/ (not deer-stage/), auto-discovery should fail gracefully
    // Run upload-dataset WITHOUT --images parameter
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
    let test_data_dir = get_deer_test_data_path();
    let images_path = test_data_dir.join("deer");

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
    let test_data_dir = get_deer_test_data_path();
    let annotations_path = test_data_dir.join("deer-stage.arrow");
    let images_path = test_data_dir.join("deer");

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

    // Get test data paths (Deer dataset has 1646 images, which will trigger
    // batching)
    let test_data_dir = get_deer_test_data_path();
    let annotations_path = test_data_dir.join("deer-stage.arrow");
    let images_path = test_data_dir.join("deer");

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
