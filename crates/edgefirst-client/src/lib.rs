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
        ImageId, Organization, OrganizationID, Parameter, Project, ProjectID, SampleID, SequenceId,
        SnapshotID, Stage, Task, TaskID, TaskInfo, TrainingSession, TrainingSessionID,
        ValidationSession, ValidationSessionID,
    },
    client::{Client, Progress},
    dataset::{
        Annotation, AnnotationSet, AnnotationType, Box2d, Box3d, Dataset, FileType, Label, Mask,
        Sample,
    },
    error::Error,
};

#[cfg(feature = "polars")]
pub use crate::dataset::annotations_dataframe;

#[cfg(test)]
mod tests {
    use super::*;
    use polars::frame::UniqueKeepStrategy;
    use std::{
        collections::HashMap,
        env,
        fs::{File, read_to_string},
        io::Write,
        path::Path,
    };
    use tokio::time::{Duration, sleep};

    #[ctor::ctor]
    fn init() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    #[tokio::test]
    async fn test_version() -> Result<(), Error> {
        let client = match env::var("STUDIO_SERVER") {
            Ok(server) => Client::new()?.with_server(&server)?,
            Err(_) => Client::new()?,
        };
        let result = client.version().await?;
        println!("EdgeFirst Studio Version: {}", result);
        Ok(())
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
    async fn test_token() -> Result<(), Error> {
        let client = get_client().await?;
        let token = client.token().await;
        assert!(!token.is_empty());
        println!("Token: {}", token);

        let exp = client.token_expiration().await?;
        println!("Token Expiration: {}", exp);

        let username = client.username().await?;
        assert!(!username.is_empty());
        println!("Username: {}", username);

        // Wait for 2 seconds to ensure token renewal updates the time
        sleep(Duration::from_secs(2)).await;

        client.renew_token().await?;
        let new_token = client.token().await;
        assert!(!new_token.is_empty());
        assert_ne!(token, new_token);
        println!("New Token Expiration: {}", client.token_expiration().await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_organization() -> Result<(), Error> {
        let client = get_client().await?;
        let org = client.organization().await?;
        println!(
            "Organization: {}\nID: {}\nCredits: {}",
            org.name(),
            org.id(),
            org.credits()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_projects() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        assert!(!project.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_datasets() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        assert!(!project.is_empty());
        let project = project.first().unwrap();
        let datasets = client.datasets(project.id(), None).await?;

        for dataset in datasets {
            let dataset_id = dataset.id();
            let result = client.dataset(dataset_id).await?;
            assert_eq!(result.id(), dataset_id);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_labels() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        assert!(!project.is_empty());
        let project = project.first().unwrap();
        let datasets = client.datasets(project.id(), Some("Test Labels")).await?;
        let dataset = datasets.first().unwrap_or_else(|| {
            panic!(
                "Dataset 'Test Labels' not found in project '{}'",
                project.name()
            )
        });

        let labels = dataset.labels(&client).await?;
        for label in labels {
            label.remove(&client).await?;
        }

        let labels = dataset.labels(&client).await?;
        assert_eq!(labels.len(), 0);

        dataset.add_label(&client, "test").await?;
        let labels = dataset.labels(&client).await?;
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name(), "test");

        dataset.remove_label(&client, "test").await?;
        let labels = dataset.labels(&client).await?;
        assert_eq!(labels.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_coco() -> Result<(), Error> {
        let coco_labels = HashMap::from([
            (0, "person"),
            (1, "bicycle"),
            (2, "car"),
            (3, "motorcycle"),
            (4, "airplane"),
            (5, "bus"),
            (6, "train"),
            (7, "truck"),
            (8, "boat"),
            (9, "traffic light"),
            (10, "fire hydrant"),
            (11, "stop sign"),
            (12, "parking meter"),
            (13, "bench"),
            (14, "bird"),
            (15, "cat"),
            (16, "dog"),
            (17, "horse"),
            (18, "sheep"),
            (19, "cow"),
            (20, "elephant"),
            (21, "bear"),
            (22, "zebra"),
            (23, "giraffe"),
            (24, "backpack"),
            (25, "umbrella"),
            (26, "handbag"),
            (27, "tie"),
            (28, "suitcase"),
            (29, "frisbee"),
            (30, "skis"),
            (31, "snowboard"),
            (32, "sports ball"),
            (33, "kite"),
            (34, "baseball bat"),
            (35, "baseball glove"),
            (36, "skateboard"),
            (37, "surfboard"),
            (38, "tennis racket"),
            (39, "bottle"),
            (40, "wine glass"),
            (41, "cup"),
            (42, "fork"),
            (43, "knife"),
            (44, "spoon"),
            (45, "bowl"),
            (46, "banana"),
            (47, "apple"),
            (48, "sandwich"),
            (49, "orange"),
            (50, "broccoli"),
            (51, "carrot"),
            (52, "hot dog"),
            (53, "pizza"),
            (54, "donut"),
            (55, "cake"),
            (56, "chair"),
            (57, "couch"),
            (58, "potted plant"),
            (59, "bed"),
            (60, "dining table"),
            (61, "toilet"),
            (62, "tv"),
            (63, "laptop"),
            (64, "mouse"),
            (65, "remote"),
            (66, "keyboard"),
            (67, "cell phone"),
            (68, "microwave"),
            (69, "oven"),
            (70, "toaster"),
            (71, "sink"),
            (72, "refrigerator"),
            (73, "book"),
            (74, "clock"),
            (75, "vase"),
            (76, "scissors"),
            (77, "teddy bear"),
            (78, "hair drier"),
            (79, "toothbrush"),
        ]);

        let client = get_client().await?;
        let project = client.projects(Some("Sample Project")).await?;
        assert!(!project.is_empty());
        let project = project.first().unwrap();
        let datasets = client.datasets(project.id(), Some("COCO")).await?;
        assert!(!datasets.is_empty());
        // Filter to avoid fetching the COCO People dataset.
        let dataset = datasets.iter().find(|d| d.name() == "COCO").unwrap();

        let labels = dataset.labels(&client).await?;
        assert_eq!(labels.len(), 80);

        for label in &labels {
            assert_eq!(label.name(), coco_labels[&label.index()]);
        }

        let n_samples = client
            .samples_count(dataset.id(), None, &[], &["val".to_string()], &[])
            .await?;
        assert_eq!(n_samples.total, 5000);

        let samples = client
            .samples(dataset.id(), None, &[], &["val".to_string()], &[], None)
            .await?;
        assert_eq!(samples.len(), 5000);

        Ok(())
    }

    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_coco_dataframe() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Sample Project")).await?;
        assert!(!project.is_empty());
        let project = project.first().unwrap();
        let datasets = client.datasets(project.id(), Some("COCO")).await?;
        assert!(!datasets.is_empty());
        // Filter to avoid fetching the COCO People dataset.
        let dataset = datasets.iter().find(|d| d.name() == "COCO").unwrap();

        let annotation_set_id = dataset
            .annotation_sets(&client)
            .await?
            .first()
            .unwrap()
            .id();

        let annotations = client
            .annotations(annotation_set_id, &["val".to_string()], &[], None)
            .await?;
        let df = annotations_dataframe(&annotations);
        let df = df
            .unique_stable(Some(&["name".to_string()]), UniqueKeepStrategy::First, None)
            .unwrap();
        assert_eq!(df.height(), 5000);

        Ok(())
    }

    #[tokio::test]
    async fn test_snapshots() -> Result<(), Error> {
        let client = get_client().await?;
        let snapshots = client.snapshots(None).await?;

        for snapshot in snapshots {
            let snapshot_id = snapshot.id();
            let result = client.snapshot(snapshot_id).await?;
            assert_eq!(result.id(), snapshot_id);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_experiments() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        assert!(!project.is_empty());
        let project = project.first().unwrap();
        let experiments = client.experiments(project.id(), None).await?;

        for experiment in experiments {
            let experiment_id = experiment.id();
            let result = client.experiment(experiment_id).await?;
            assert_eq!(result.id(), experiment_id);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_training_session() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        assert!(!project.is_empty());
        let project = project.first().unwrap();
        let experiment = client
            .experiments(project.id(), Some("Unit Testing"))
            .await?;
        let experiment = experiment.first().unwrap();

        let sessions = client
            .training_sessions(experiment.id(), Some("modelpack-usermanaged"))
            .await?;
        assert_ne!(sessions.len(), 0);
        let session = sessions.first().unwrap();

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
        let project = project.first().unwrap();

        let sessions = client.validation_sessions(project.id()).await?;
        for session in &sessions {
            let s = client.validation_session(session.id()).await?;
            assert_eq!(s.id(), session.id());
            assert_eq!(s.description(), session.description());
        }

        let session = sessions
            .into_iter()
            .find(|s| s.name() == "modelpack-usermanaged")
            .unwrap_or_else(|| {
                panic!(
                    "Validation session 'modelpack-usermanaged' not found in project '{}'",
                    project.name()
                )
            });

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
        let project = project.first().unwrap();
        let experiment = client
            .experiments(project.id(), Some("Unit Testing"))
            .await?;
        let experiment = experiment.first().unwrap();
        let trainer = client
            .training_sessions(experiment.id(), Some("modelpack-960x540"))
            .await?;
        let trainer = trainer.first().unwrap();
        let artifacts = client.artifacts(trainer.id()).await?;
        assert!(!artifacts.is_empty());

        for artifact in artifacts {
            client
                .download_artifact(
                    trainer.id(),
                    artifact.name(),
                    Some(artifact.name().into()),
                    None,
                )
                .await?;
        }

        let res = client
            .download_artifact(
                trainer.id(),
                "fakefile.txt",
                Some("fakefile.txt".into()),
                None,
            )
            .await;
        assert!(res.is_err());
        assert_eq!(Path::new("fakefile.txt").exists(), false);

        Ok(())
    }

    #[tokio::test]
    async fn test_checkpoints() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        assert!(!project.is_empty());
        let project = project.first().unwrap();
        let experiment = client
            .experiments(project.id(), Some("Unit Testing"))
            .await?;
        let experiment = experiment.first().unwrap_or_else(|| {
            panic!(
                "Experiment 'Unit Testing' not found in project '{}'",
                project.name()
            )
        });
        let trainer = client
            .training_sessions(experiment.id(), Some("modelpack-usermanaged"))
            .await?;
        let trainer = trainer.first().unwrap();

        {
            let mut chkpt = File::create("checkpoint.txt")?;
            chkpt.write_all(b"Test Checkpoint")?;
        }

        trainer
            .upload(
                &client,
                &[(
                    "checkpoints/checkpoint.txt".to_string(),
                    "checkpoint.txt".into(),
                )],
            )
            .await?;

        client
            .download_checkpoint(
                trainer.id(),
                "checkpoint.txt",
                Some("checkpoint2.txt".into()),
                None,
            )
            .await?;

        let chkpt = read_to_string("checkpoint2.txt")?;
        assert_eq!(chkpt, "Test Checkpoint");

        let res = client
            .download_checkpoint(
                trainer.id(),
                "fakefile.txt",
                Some("fakefile.txt".into()),
                None,
            )
            .await;
        assert!(res.is_err());
        assert_eq!(Path::new("fakefile.txt").exists(), false);

        Ok(())
    }

    #[tokio::test]
    async fn test_tasks() -> Result<(), Error> {
        let client = get_client().await?;
        let project = client.projects(Some("Unit Testing")).await?;
        let project = project.first().unwrap();
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
