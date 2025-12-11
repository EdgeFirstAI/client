// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! # EdgeFirst Studio Client Library
//!
//! The EdgeFirst Studio Client Library provides a Rust client for interacting
//! with EdgeFirst Studio, a comprehensive platform for computer vision and
//! machine learning workflows. This library enables developers to
//! programmatically manage datasets, annotations, training sessions, and other
//! Studio resources.
//!
//! ## Features
//!
//! - **Authentication**: Secure token-based authentication with automatic
//!   renewal
//! - **Dataset Management**: Upload, download, and manage datasets with various
//!   file types
//! - **Annotation Management**: Create, update, and retrieve annotations for
//!   computer vision tasks
//! - **Training & Validation**: Manage machine learning training and validation
//!   sessions
//! - **Project Organization**: Organize work into projects with hierarchical
//!   structure
//! - **Polars Integration**: Optional integration with Polars DataFrames for
//!   data analysis
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use edgefirst_client::{Client, Error};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     // Create a new client
//!     let client = Client::new()?;
//!
//!     // Authenticate with username and password
//!     let client = client.with_login("username", "password").await?;
//!
//!     // List available projects
//!     let projects = client.projects(None).await?;
//!     println!("Found {} projects", projects.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Optional Features
//!
//! - `polars`: Enables integration with Polars DataFrames for enhanced data
//!   manipulation

mod api;
mod client;
mod dataset;
mod error;
pub mod format;
mod retry;
mod storage;

pub use crate::{
    api::{
        AnnotationSetID, AppId, Artifact, DatasetID, DatasetParams, Experiment, ExperimentID,
        ImageId, Organization, OrganizationID, Parameter, PresignedUrl, Project, ProjectID,
        SampleID, SamplesCountResult, SamplesPopulateParams, SamplesPopulateResult, SequenceId,
        Snapshot, SnapshotFromDatasetResult, SnapshotID, SnapshotRestoreResult, Stage, Task,
        TaskID, TaskInfo, TrainingSession, TrainingSessionID, ValidationSession,
        ValidationSessionID,
    },
    client::{Client, Progress},
    dataset::{
        Annotation, AnnotationSet, AnnotationType, Box2d, Box3d, Dataset, FileType, GpsData,
        ImuData, Label, Location, Mask, Sample, SampleFile,
    },
    error::Error,
    retry::{RetryScope, classify_url},
    storage::{FileTokenStorage, MemoryTokenStorage, StorageError, TokenStorage},
};

#[cfg(feature = "polars")]
#[allow(deprecated)] // Re-exported for backwards compatibility
pub use crate::dataset::annotations_dataframe;

#[cfg(feature = "polars")]
pub use crate::dataset::samples_dataframe;

#[cfg(feature = "polars")]
pub use crate::dataset::unflatten_polygon_coordinates;

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        env,
        fs::{File, read_to_string},
        io::Write,
        path::PathBuf,
    };

    /// Get the test data directory (target/testdata)
    /// Creates it if it doesn't exist
    fn get_test_data_dir() -> PathBuf {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("CARGO_MANIFEST_DIR should have parent")
            .parent()
            .expect("workspace root should exist")
            .join("target")
            .join("testdata");

        std::fs::create_dir_all(&test_dir).expect("Failed to create test data directory");
        test_dir
    }

    #[ctor::ctor]
    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    async fn get_client() -> Result<Client, Error> {
        let client = Client::new()?.with_token_path(None)?;

        let client = match env::var("STUDIO_TOKEN") {
            Ok(token) => client.with_token(&token)?,
            Err(_) => client,
        };

        let client = match env::var("STUDIO_SERVER") {
            Ok(server) => client.with_server(&server)?,
            Err(_) => client,
        };

        let client = match (env::var("STUDIO_USERNAME"), env::var("STUDIO_PASSWORD")) {
            (Ok(username), Ok(password)) => client.with_login(&username, &password).await?,
            _ => client,
        };

        client.verify_token().await?;

        Ok(client)
    }

    /// Helper: Get training session for "Unit Testing" project
    async fn get_training_session_for_artifacts() -> Result<TrainingSession, Error> {
        let client = get_client().await?;
        let project = client
            .projects(Some("Unit Testing"))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::InvalidParameters("Unit Testing project not found".into()))?;
        let experiment = client
            .experiments(project.id(), Some("Unit Testing"))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::InvalidParameters("Unit Testing experiment not found".into()))?;
        let session = client
            .training_sessions(experiment.id(), Some("modelpack-960x540"))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| {
                Error::InvalidParameters("modelpack-960x540 session not found".into())
            })?;
        Ok(session)
    }

    /// Helper: Get training session for "modelpack-usermanaged"
    async fn get_training_session_for_checkpoints() -> Result<TrainingSession, Error> {
        let client = get_client().await?;
        let project = client
            .projects(Some("Unit Testing"))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::InvalidParameters("Unit Testing project not found".into()))?;
        let experiment = client
            .experiments(project.id(), Some("Unit Testing"))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::InvalidParameters("Unit Testing experiment not found".into()))?;
        let session = client
            .training_sessions(experiment.id(), Some("modelpack-usermanaged"))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| {
                Error::InvalidParameters("modelpack-usermanaged session not found".into())
            })?;
        Ok(session)
    }

    #[tokio::test]
    async fn test_training_session() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        assert!(!project.is_empty());
        let project = project
            .first()
            .expect("'Unit Testing' project should exist");
        let experiment = client
            .experiments(project.id(), Some("Unit Testing"))
            .await?;
        let experiment = experiment
            .first()
            .expect("'Unit Testing' experiment should exist");

        let sessions = client
            .training_sessions(experiment.id(), Some("modelpack-usermanaged"))
            .await?;
        assert_ne!(sessions.len(), 0);
        let session = sessions
            .first()
            .expect("Training sessions should exist for experiment");

        let metrics = HashMap::from([
            ("epochs".to_string(), Parameter::Integer(10)),
            ("loss".to_string(), Parameter::Real(0.05)),
            (
                "model".to_string(),
                Parameter::String("modelpack".to_string()),
            ),
        ]);

        session.set_metrics(&client, metrics).await?;
        let updated_metrics = session.metrics(&client).await?;
        assert_eq!(updated_metrics.len(), 3);
        assert_eq!(updated_metrics.get("epochs"), Some(&Parameter::Integer(10)));
        assert_eq!(updated_metrics.get("loss"), Some(&Parameter::Real(0.05)));
        assert_eq!(
            updated_metrics.get("model"),
            Some(&Parameter::String("modelpack".to_string()))
        );

        println!("Updated Metrics: {:?}", updated_metrics);

        let mut labels = tempfile::NamedTempFile::new()?;
        write!(labels, "background")?;
        labels.flush()?;

        session
            .upload(
                &client,
                &[(
                    "artifacts/labels.txt".to_string(),
                    labels.path().to_path_buf(),
                )],
            )
            .await?;

        let labels = session.download(&client, "artifacts/labels.txt").await?;
        assert_eq!(labels, "background");

        Ok(())
    }

    #[tokio::test]
    async fn test_validate() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        assert!(!project.is_empty());
        let project = project
            .first()
            .expect("'Unit Testing' project should exist");

        let sessions = client.validation_sessions(project.id()).await?;
        for session in &sessions {
            let s = client.validation_session(session.id()).await?;
            assert_eq!(s.id(), session.id());
            assert_eq!(s.description(), session.description());
        }

        let session = sessions
            .into_iter()
            .find(|s| s.name() == "modelpack-usermanaged")
            .ok_or_else(|| {
                Error::InvalidParameters(format!(
                    "Validation session 'modelpack-usermanaged' not found in project '{}'",
                    project.name()
                ))
            })?;

        let metrics = HashMap::from([("accuracy".to_string(), Parameter::Real(0.95))]);
        session.set_metrics(&client, metrics).await?;

        let metrics = session.metrics(&client).await?;
        assert_eq!(metrics.get("accuracy"), Some(&Parameter::Real(0.95)));

        Ok(())
    }

    #[tokio::test]
    async fn test_download_artifact_success() -> Result<(), Error> {
        let trainer = get_training_session_for_artifacts().await?;
        let client = get_client().await?;
        let artifacts = client.artifacts(trainer.id()).await?;
        assert!(!artifacts.is_empty());

        let test_dir = get_test_data_dir();
        let artifact = &artifacts[0];
        let output_path = test_dir.join(artifact.name());

        client
            .download_artifact(
                trainer.id(),
                artifact.name(),
                Some(output_path.clone()),
                None,
            )
            .await?;

        assert!(output_path.exists());
        if output_path.exists() {
            std::fs::remove_file(&output_path)?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_download_artifact_not_found() -> Result<(), Error> {
        let trainer = get_training_session_for_artifacts().await?;
        let client = get_client().await?;
        let test_dir = get_test_data_dir();
        let fake_path = test_dir.join("nonexistent_artifact.txt");

        let result = client
            .download_artifact(
                trainer.id(),
                "nonexistent_artifact.txt",
                Some(fake_path.clone()),
                None,
            )
            .await;

        assert!(result.is_err());
        assert!(!fake_path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_artifacts() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        assert!(!project.is_empty());
        let project = project
            .first()
            .expect("'Unit Testing' project should exist");
        let experiment = client
            .experiments(project.id(), Some("Unit Testing"))
            .await?;
        let experiment = experiment
            .first()
            .expect("'Unit Testing' experiment should exist");
        let trainer = client
            .training_sessions(experiment.id(), Some("modelpack-960x540"))
            .await?;
        let trainer = trainer
            .first()
            .expect("'modelpack-960x540' training session should exist");
        let artifacts = client.artifacts(trainer.id()).await?;
        assert!(!artifacts.is_empty());

        let test_dir = get_test_data_dir();

        for artifact in artifacts {
            let output_path = test_dir.join(artifact.name());
            client
                .download_artifact(
                    trainer.id(),
                    artifact.name(),
                    Some(output_path.clone()),
                    None,
                )
                .await?;

            // Clean up downloaded file
            if output_path.exists() {
                std::fs::remove_file(&output_path)?;
            }
        }

        let fake_path = test_dir.join("fakefile.txt");
        let res = client
            .download_artifact(trainer.id(), "fakefile.txt", Some(fake_path.clone()), None)
            .await;
        assert!(res.is_err());
        assert!(!fake_path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_download_checkpoint_success() -> Result<(), Error> {
        let trainer = get_training_session_for_checkpoints().await?;
        let client = get_client().await?;
        let test_dir = get_test_data_dir();

        // Create temporary test file
        let checkpoint_path = test_dir.join("test_checkpoint.txt");
        {
            let mut f = File::create(&checkpoint_path)?;
            f.write_all(b"Test Checkpoint Content")?;
        }

        // Upload the checkpoint
        trainer
            .upload(
                &client,
                &[(
                    "checkpoints/test_checkpoint.txt".to_string(),
                    checkpoint_path.clone(),
                )],
            )
            .await?;

        // Download and verify
        let download_path = test_dir.join("downloaded_checkpoint.txt");
        client
            .download_checkpoint(
                trainer.id(),
                "test_checkpoint.txt",
                Some(download_path.clone()),
                None,
            )
            .await?;

        let content = read_to_string(&download_path)?;
        assert_eq!(content, "Test Checkpoint Content");

        // Cleanup
        if checkpoint_path.exists() {
            std::fs::remove_file(&checkpoint_path)?;
        }
        if download_path.exists() {
            std::fs::remove_file(&download_path)?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_download_checkpoint_not_found() -> Result<(), Error> {
        let trainer = get_training_session_for_checkpoints().await?;
        let client = get_client().await?;
        let test_dir = get_test_data_dir();
        let fake_path = test_dir.join("nonexistent_checkpoint.txt");

        let result = client
            .download_checkpoint(
                trainer.id(),
                "nonexistent_checkpoint.txt",
                Some(fake_path.clone()),
                None,
            )
            .await;

        assert!(result.is_err());
        assert!(!fake_path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoints() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        assert!(!project.is_empty());
        let project = project
            .first()
            .expect("'Unit Testing' project should exist");
        let experiment = client
            .experiments(project.id(), Some("Unit Testing"))
            .await?;
        let experiment = experiment.first().ok_or_else(|| {
            Error::InvalidParameters(format!(
                "Experiment 'Unit Testing' not found in project '{}'",
                project.name()
            ))
        })?;
        let trainer = client
            .training_sessions(experiment.id(), Some("modelpack-usermanaged"))
            .await?;
        let trainer = trainer
            .first()
            .expect("'modelpack-usermanaged' training session should exist");

        let test_dir = get_test_data_dir();
        let checkpoint_path = test_dir.join("checkpoint.txt");
        let checkpoint2_path = test_dir.join("checkpoint2.txt");

        {
            let mut chkpt = File::create(&checkpoint_path)?;
            chkpt.write_all(b"Test Checkpoint")?;
        }

        trainer
            .upload(
                &client,
                &[(
                    "checkpoints/checkpoint.txt".to_string(),
                    checkpoint_path.clone(),
                )],
            )
            .await?;

        client
            .download_checkpoint(
                trainer.id(),
                "checkpoint.txt",
                Some(checkpoint2_path.clone()),
                None,
            )
            .await?;

        let chkpt = read_to_string(&checkpoint2_path)?;
        assert_eq!(chkpt, "Test Checkpoint");

        let fake_path = test_dir.join("fakefile.txt");
        let res = client
            .download_checkpoint(trainer.id(), "fakefile.txt", Some(fake_path.clone()), None)
            .await;
        assert!(res.is_err());
        assert!(!fake_path.exists());

        // Clean up
        if checkpoint_path.exists() {
            std::fs::remove_file(&checkpoint_path)?;
        }
        if checkpoint2_path.exists() {
            std::fs::remove_file(&checkpoint2_path)?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_task_retrieval() -> Result<(), Error> {
        let client = get_client().await?;

        // Test: Get all tasks
        let tasks = client.tasks(None, None, None, None).await?;
        assert!(!tasks.is_empty());

        // Test: Get task info for first task
        let task_id = tasks[0].id();
        let task_info = client.task_info(task_id).await?;
        assert_eq!(task_info.id(), task_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_task_filtering_by_name() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        let project = project
            .first()
            .expect("'Unit Testing' project should exist");

        // Test: Get tasks by name
        let tasks = client
            .tasks(Some("modelpack-usermanaged"), None, None, None)
            .await?;

        if !tasks.is_empty() {
            // Get detailed info for each task
            let task_infos = tasks
                .into_iter()
                .map(|t| client.task_info(t.id()))
                .collect::<Vec<_>>();
            let task_infos = futures::future::try_join_all(task_infos).await?;

            // Filter by project
            let filtered = task_infos
                .into_iter()
                .filter(|t| t.project_id() == Some(project.id()))
                .collect::<Vec<_>>();

            if !filtered.is_empty() {
                assert_eq!(filtered[0].project_id(), Some(project.id()));
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_task_status_and_stages() -> Result<(), Error> {
        let client = get_client().await?;

        // Get first available task
        let tasks = client.tasks(None, None, None, None).await?;
        if tasks.is_empty() {
            return Ok(());
        }

        let task_id = tasks[0].id();

        // Test: Get task status
        let status = client.task_status(task_id, "training").await?;
        assert_eq!(status.id(), task_id);
        assert_eq!(status.status(), "training");

        // Test: Set stages
        let stages = [
            ("download", "Downloading Dataset"),
            ("train", "Training Model"),
            ("export", "Exporting Model"),
        ];
        client.set_stages(task_id, &stages).await?;

        // Test: Update stage
        client
            .update_stage(task_id, "download", "running", "Downloading dataset", 50)
            .await?;

        // Verify task with updated stages
        let updated_task = client.task_info(task_id).await?;
        assert_eq!(updated_task.id(), task_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_tasks() -> Result<(), Error> {
        let client = get_client().await?;
        let tasks = client.tasks(None, None, None, None).await?;

        for task in tasks {
            let task_info = client.task_info(task.id()).await?;
            println!("{} - {}", task, task_info);
        }

        let tasks = client
            .tasks(Some("modelpack-usermanaged"), None, None, None)
            .await?;
        let tasks = tasks
            .into_iter()
            .map(|t| client.task_info(t.id()))
            .collect::<Vec<_>>();
        let tasks = futures::future::try_join_all(tasks).await?;
        assert_ne!(tasks.len(), 0);
        let task = &tasks[0];

        let t = client.task_status(task.id(), "training").await?;
        assert_eq!(t.id(), task.id());
        assert_eq!(t.status(), "training");

        let stages = [
            ("download", "Downloading Dataset"),
            ("train", "Training Model"),
            ("export", "Exporting Model"),
        ];
        client.set_stages(task.id(), &stages).await?;

        client
            .update_stage(task.id(), "download", "running", "Downloading dataset", 50)
            .await?;

        let task = client.task_info(task.id()).await?;
        println!("task progress: {:?}", task.stages());

        Ok(())
    }

    // ============================================================================
    // Retry URL Classification Tests
    // ============================================================================

    mod retry_url_classification {
        use super::*;

        #[test]
        fn test_studio_api_base_url() {
            // Base production URL
            assert_eq!(
                classify_url("https://edgefirst.studio/api"),
                RetryScope::StudioApi
            );
        }

        #[test]
        fn test_studio_api_with_trailing_slash() {
            // Trailing slash should be handled correctly
            assert_eq!(
                classify_url("https://edgefirst.studio/api/"),
                RetryScope::StudioApi
            );
        }

        #[test]
        fn test_studio_api_with_path() {
            // API endpoints with additional path segments
            assert_eq!(
                classify_url("https://edgefirst.studio/api/datasets"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://edgefirst.studio/api/auth.login"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://edgefirst.studio/api/trainer/session"),
                RetryScope::StudioApi
            );
        }

        #[test]
        fn test_studio_api_with_query_params() {
            // Query parameters should not affect classification
            assert_eq!(
                classify_url("https://edgefirst.studio/api?foo=bar"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://edgefirst.studio/api/datasets?page=1&limit=10"),
                RetryScope::StudioApi
            );
        }

        #[test]
        fn test_studio_api_subdomains() {
            // Server-specific instances (test, stage, saas, ocean, etc.)
            assert_eq!(
                classify_url("https://test.edgefirst.studio/api"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://stage.edgefirst.studio/api"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://saas.edgefirst.studio/api"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://ocean.edgefirst.studio/api"),
                RetryScope::StudioApi
            );
        }

        #[test]
        fn test_studio_api_with_standard_port() {
            // Standard HTTPS port (443) should be handled
            assert_eq!(
                classify_url("https://edgefirst.studio:443/api"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://test.edgefirst.studio:443/api"),
                RetryScope::StudioApi
            );
        }

        #[test]
        fn test_studio_api_with_custom_port() {
            // Custom ports should be handled correctly
            assert_eq!(
                classify_url("https://test.edgefirst.studio:8080/api"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://edgefirst.studio:8443/api"),
                RetryScope::StudioApi
            );
        }

        #[test]
        fn test_studio_api_http_protocol() {
            // HTTP (not HTTPS) should still be recognized
            assert_eq!(
                classify_url("http://edgefirst.studio/api"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("http://test.edgefirst.studio/api"),
                RetryScope::StudioApi
            );
        }

        #[test]
        fn test_file_io_s3_urls() {
            // S3 URLs for file operations
            assert_eq!(
                classify_url("https://s3.amazonaws.com/bucket/file.bin"),
                RetryScope::FileIO
            );
            assert_eq!(
                classify_url("https://s3.us-west-2.amazonaws.com/mybucket/data.zip"),
                RetryScope::FileIO
            );
        }

        #[test]
        fn test_file_io_cloudfront_urls() {
            // CloudFront URLs for file distribution
            assert_eq!(
                classify_url("https://d123abc.cloudfront.net/file.bin"),
                RetryScope::FileIO
            );
            assert_eq!(
                classify_url("https://d456def.cloudfront.net/path/to/file.tar.gz"),
                RetryScope::FileIO
            );
        }

        #[test]
        fn test_file_io_non_api_studio_paths() {
            // Non-API paths on edgefirst.studio domain
            assert_eq!(
                classify_url("https://edgefirst.studio/docs"),
                RetryScope::FileIO
            );
            assert_eq!(
                classify_url("https://edgefirst.studio/download_model"),
                RetryScope::FileIO
            );
            assert_eq!(
                classify_url("https://test.edgefirst.studio/download_model"),
                RetryScope::FileIO
            );
            assert_eq!(
                classify_url("https://stage.edgefirst.studio/download_checkpoint"),
                RetryScope::FileIO
            );
        }

        #[test]
        fn test_file_io_generic_urls() {
            // Generic download URLs
            assert_eq!(
                classify_url("https://example.com/download"),
                RetryScope::FileIO
            );
            assert_eq!(
                classify_url("https://cdn.example.com/files/data.json"),
                RetryScope::FileIO
            );
        }

        #[test]
        fn test_security_malicious_url_substring() {
            // Security: URL with edgefirst.studio in path should NOT match
            assert_eq!(
                classify_url("https://evil.com/test.edgefirst.studio/api"),
                RetryScope::FileIO
            );
            assert_eq!(
                classify_url("https://attacker.com/edgefirst.studio/api/fake"),
                RetryScope::FileIO
            );
        }

        #[test]
        fn test_edge_case_similar_domains() {
            // Similar but different domains should be FileIO
            assert_eq!(
                classify_url("https://edgefirst.studio.com/api"),
                RetryScope::FileIO
            );
            assert_eq!(
                classify_url("https://notedgefirst.studio/api"),
                RetryScope::FileIO
            );
            assert_eq!(
                classify_url("https://edgefirststudio.com/api"),
                RetryScope::FileIO
            );
        }

        #[test]
        fn test_edge_case_invalid_urls() {
            // Invalid URLs should default to FileIO
            assert_eq!(classify_url("not a url"), RetryScope::FileIO);
            assert_eq!(classify_url(""), RetryScope::FileIO);
            assert_eq!(
                classify_url("ftp://edgefirst.studio/api"),
                RetryScope::FileIO
            );
        }

        #[test]
        fn test_edge_case_url_normalization() {
            // URL normalization edge cases
            assert_eq!(
                classify_url("https://EDGEFIRST.STUDIO/api"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://test.EDGEFIRST.studio/api"),
                RetryScope::StudioApi
            );
        }

        #[test]
        fn test_comprehensive_subdomain_coverage() {
            // Ensure all known server instances are recognized
            let subdomains = vec![
                "test", "stage", "saas", "ocean", "prod", "dev", "qa", "demo",
            ];

            for subdomain in subdomains {
                let url = format!("https://{}.edgefirst.studio/api", subdomain);
                assert_eq!(
                    classify_url(&url),
                    RetryScope::StudioApi,
                    "Failed for subdomain: {}",
                    subdomain
                );
            }
        }

        #[test]
        fn test_api_path_variations() {
            // Various API path patterns
            assert_eq!(
                classify_url("https://edgefirst.studio/api"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://edgefirst.studio/api/"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://edgefirst.studio/api/v1"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://edgefirst.studio/api/v2/datasets"),
                RetryScope::StudioApi
            );

            // Non-/api paths should be FileIO
            assert_eq!(
                classify_url("https://edgefirst.studio/apis"),
                RetryScope::FileIO
            );
            assert_eq!(
                classify_url("https://edgefirst.studio/v1/api"),
                RetryScope::FileIO
            );
        }

        #[test]
        fn test_port_range_coverage() {
            // Test various port numbers
            let ports = vec![80, 443, 8080, 8443, 3000, 5000, 9000];

            for port in ports {
                let url = format!("https://test.edgefirst.studio:{}/api", port);
                assert_eq!(
                    classify_url(&url),
                    RetryScope::StudioApi,
                    "Failed for port: {}",
                    port
                );
            }
        }

        #[test]
        fn test_complex_query_strings() {
            // Complex query parameters with special characters
            assert_eq!(
                classify_url("https://edgefirst.studio/api?token=abc123&redirect=/dashboard"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://test.edgefirst.studio/api?q=search%20term&page=1"),
                RetryScope::StudioApi
            );
        }

        #[test]
        fn test_url_with_fragment() {
            // URLs with fragments (#) - fragments are not sent to server
            assert_eq!(
                classify_url("https://edgefirst.studio/api#section"),
                RetryScope::StudioApi
            );
            assert_eq!(
                classify_url("https://test.edgefirst.studio/api/datasets#results"),
                RetryScope::StudioApi
            );
        }
    }
}
