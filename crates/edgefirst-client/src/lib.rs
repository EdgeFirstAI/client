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

pub use crate::{
    api::{
        AnnotationSetID, AppId, Artifact, DatasetID, DatasetParams, Experiment, ExperimentID,
        ImageId, Organization, OrganizationID, Parameter, PresignedUrl, Project, ProjectID,
        SampleID, SamplesPopulateParams, SamplesPopulateResult, SequenceId, SnapshotID, Stage,
        Task, TaskID, TaskInfo, TrainingSession, TrainingSessionID, ValidationSession,
        ValidationSessionID,
    },
    client::{Client, Progress},
    dataset::{
        Annotation, AnnotationSet, AnnotationType, Box2d, Box3d, Dataset, FileType, GpsData,
        ImuData, Label, Location, Mask, Sample, SampleFile,
    },
    error::Error,
};

#[cfg(feature = "polars")]
pub use crate::dataset::annotations_dataframe;

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
    async fn test_tasks() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        let project = project
            .first()
            .expect("'Unit Testing' project should exist");
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
        let tasks = tasks
            .into_iter()
            .filter(|t| t.project_id() == Some(project.id()))
            .collect::<Vec<_>>();
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
}
