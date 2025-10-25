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
    use polars::frame::UniqueKeepStrategy;
    use rand::Rng;
    use std::{
        collections::HashMap,
        env,
        fs::{File, read_to_string},
        io::Write,
        path::PathBuf,
    };
    use tokio::time::{Duration, sleep};

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
        let project = project
            .first()
            .expect("'Unit Testing' project should exist");
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
        let project = project
            .first()
            .expect("'Unit Testing' project should exist");
        let datasets = client.datasets(project.id(), Some("Test Labels")).await?;
        let dataset = datasets.first().ok_or_else(|| {
            Error::InvalidParameters(format!(
                "Dataset 'Test Labels' not found in project '{}'",
                project.name()
            ))
        })?;

        // Generate a random label name to avoid conflicts with parallel tests
        // Use a random 8-character hex string
        let random_suffix: u64 = rand::rng().random();
        let test_label = format!("test_{:x}", random_suffix);

        // Get initial label count
        let initial_labels = dataset.labels(&client).await?;
        let initial_count = initial_labels.len();

        // Verify the random label doesn't already exist
        assert!(
            !initial_labels.iter().any(|l| l.name() == test_label),
            "Random label '{}' should not exist yet",
            test_label
        );

        // Add the test label
        dataset.add_label(&client, &test_label).await?;
        let labels_after_add = dataset.labels(&client).await?;
        assert_eq!(
            labels_after_add.len(),
            initial_count + 1,
            "Should have one more label after adding"
        );
        assert!(
            labels_after_add.iter().any(|l| l.name() == test_label),
            "Label '{}' should exist after adding",
            test_label
        );

        // Remove the test label
        dataset.remove_label(&client, &test_label).await?;
        let labels_after_remove = dataset.labels(&client).await?;
        assert_eq!(
            labels_after_remove.len(),
            initial_count,
            "Should have same label count as initial after removing"
        );
        assert!(
            !labels_after_remove.iter().any(|l| l.name() == test_label),
            "Label '{}' should not exist after removing",
            test_label
        );

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
        let project = project.first().expect("'Sample Project' should exist");
        let datasets = client.datasets(project.id(), Some("COCO")).await?;
        assert!(!datasets.is_empty());
        // Filter to avoid fetching the COCO People dataset.
        let dataset = datasets
            .iter()
            .find(|d| d.name() == "COCO")
            .expect("'COCO' dataset should exist");

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
        let project = project.first().expect("'Sample Project' should exist");
        let datasets = client.datasets(project.id(), Some("COCO")).await?;
        assert!(!datasets.is_empty());
        // Filter to avoid fetching the COCO People dataset.
        let dataset = datasets
            .iter()
            .find(|d| d.name() == "COCO")
            .expect("'COCO' dataset should exist");

        let annotation_set_id = dataset
            .annotation_sets(&client)
            .await?
            .first()
            .expect("COCO dataset should have annotation sets")
            .id();

        let annotations = client
            .annotations(annotation_set_id, &["val".to_string()], &[], None)
            .await?;
        let df = annotations_dataframe(&annotations)?;
        let df = df
            .unique_stable(Some(&["name".to_string()]), UniqueKeepStrategy::First, None)
            .expect("DataFrame unique_stable operation should succeed");
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
        let project = project
            .first()
            .expect("'Unit Testing' project should exist");
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

    /// Generate a 640x480 PNG image with a red circle and return the image data
    /// plus the bounding box coordinates (x, y, w, h) in pixels.
    /// Generate a 640x480 image with a red circle in the specified format.
    /// Returns the image data plus the bounding box coordinates (x, y, w, h) in
    /// pixels. Supported formats: "png", "jpeg"
    #[allow(clippy::type_complexity)]
    fn generate_test_image_with_circle_format(
        format: &str,
    ) -> Result<(Vec<u8>, (f32, f32, f32, f32)), Error> {
        use image::{ImageBuffer, Rgb, RgbImage};
        use std::io::Cursor;

        let width = 640u32;
        let height = 480u32;

        // Create white image
        let mut img: RgbImage = ImageBuffer::from_pixel(width, height, Rgb([255u8, 255u8, 255u8]));

        // Draw a red circle in the top-left quadrant
        let center_x = 150.0;
        let center_y = 120.0;
        let radius = 50.0;

        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance <= radius {
                    img.put_pixel(x, y, Rgb([255u8, 0u8, 0u8])); // Red
                }
            }
        }

        // Encode in the specified format
        let mut image_data = Vec::new();
        let mut cursor = Cursor::new(&mut image_data);

        match format {
            "jpeg" | "jpg" => {
                img.write_to(&mut cursor, image::ImageFormat::Jpeg)
                    .expect("Failed to write JPEG image");
            }
            "png" => {
                img.write_to(&mut cursor, image::ImageFormat::Png)
                    .expect("Failed to write PNG image");
            }
            _ => return Err(Error::UnsupportedFormat(format.to_string())),
        }

        // Calculate bounding box around the circle (with some padding)
        let bbox_x = center_x - radius - 5.0;
        let bbox_y = center_y - radius - 5.0;
        let bbox_w = (radius * 2.0) + 10.0;
        let bbox_h = (radius * 2.0) + 10.0;

        Ok((image_data, (bbox_x, bbox_y, bbox_w, bbox_h)))
    }

    #[tokio::test]
    async fn test_populate_samples() -> Result<(), Error> {
        let client = get_client().await?;

        // Find the Unit Testing project
        let projects = client.projects(Some("Unit Testing")).await?;
        let project = projects
            .first()
            .expect("'Unit Testing' project should exist");

        // Create a temporary test dataset with random suffix
        let random_suffix: u64 = rand::rng().random();
        let test_dataset_name = format!("Test Populate {:x}", random_suffix);

        eprintln!("Creating test dataset: {}", test_dataset_name);

        let dataset_id = client
            .create_dataset(
                &project.id().to_string(),
                &test_dataset_name,
                Some("Automated test: populate_samples verification"),
            )
            .await?;

        eprintln!("Created test dataset: {}", dataset_id);

        // Create an annotation set
        eprintln!("Creating annotation set...");
        let annotation_set_id = client
            .create_annotation_set(dataset_id, "Default", Some("Default annotation set"))
            .await?;

        eprintln!("Created annotation set: {}", annotation_set_id);

        // Generate a 640x480 PNG image with a red circle
        // (Tested with JPEG too - server doesn't return width/height for either format)
        let test_format = "png";
        let file_extension = "png";

        // Generate a 640x480 image with a red circle
        let (image_data, circle_bbox) = generate_test_image_with_circle_format(test_format)?;
        eprintln!(
            "Generated {} image with circle at bbox: ({:.1}, {:.1}, {:.1}, {:.1})",
            test_format, circle_bbox.0, circle_bbox.1, circle_bbox.2, circle_bbox.3
        );

        // Create temporary file
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time should be after UNIX epoch")
            .as_secs();
        let temp_dir = std::env::temp_dir();
        let test_image_path =
            temp_dir.join(format!("test_populate_{}.{}", timestamp, file_extension));
        std::fs::write(&test_image_path, &image_data)?;

        // Also save a copy to target/testdata for manual inspection
        let testdata_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("CARGO_MANIFEST_DIR should have parent")
            .parent()
            .expect("workspace root should exist")
            .join("target")
            .join("testdata");
        std::fs::create_dir_all(&testdata_dir).ok();
        let local_copy = testdata_dir.join(format!(
            "test_populate_circle_{}.{}",
            timestamp, file_extension
        ));
        std::fs::write(&local_copy, &image_data)?;
        eprintln!("Test image saved to: {:?}", local_copy);

        // Create sample with annotation
        let mut sample = Sample::new();
        let img_width = 640.0;
        let img_height = 480.0;
        // Don't set width/height - let populate_samples() extract from image
        sample.group = Some("train".to_string());
        // UUID will be auto-generated

        // Add file
        sample.files = vec![SampleFile::with_filename(
            "image".to_string(),
            test_image_path
                .to_str()
                .expect("Test image path should be valid UTF-8")
                .to_string(),
        )];

        // Add bounding box annotation with NORMALIZED coordinates
        let mut annotation = Annotation::new();
        annotation.set_label(Some("circle".to_string()));
        annotation.set_object_id(Some("circle-obj-1".to_string()));

        // Normalize coordinates: divide pixel values by image dimensions
        let normalized_x = circle_bbox.0 / img_width;
        let normalized_y = circle_bbox.1 / img_height;
        let normalized_w = circle_bbox.2 / img_width;
        let normalized_h = circle_bbox.3 / img_height;

        eprintln!(
            "Normalized bbox: ({:.3}, {:.3}, {:.3}, {:.3})",
            normalized_x, normalized_y, normalized_w, normalized_h
        );

        let bbox = Box2d::new(normalized_x, normalized_y, normalized_w, normalized_h);
        annotation.set_box2d(Some(bbox));
        sample.annotations = vec![annotation];

        // Populate the sample
        let populate_result = async {
            let results = client
                .populate_samples(dataset_id, Some(annotation_set_id), vec![sample], None)
                .await?;

            assert_eq!(results.len(), 1);
            let result = &results[0];
            assert_eq!(result.urls.len(), 1);

            // The image filename we'll search for when fetching back
            let image_filename = format!("test_populate_{}.{}", timestamp, file_extension);

            // Give the server a moment to process the upload
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            // Verify the sample was created by fetching it back and searching by image_name
            let samples = client
                .samples(
                    dataset_id,
                    Some(annotation_set_id),
                    &[],
                    &[], // Don't filter by group - get all samples
                    &[],
                    None,
                )
                .await?;

            eprintln!("Looking for image: {}", image_filename);
            eprintln!("Found {} samples total", samples.len());

            // Find the sample by image_name (server doesn't return UUID we sent)
            let created_sample = samples
                .iter()
                .find(|s| s.image_name.as_deref() == Some(&image_filename));

            assert!(
                created_sample.is_some(),
                "Sample with image_name '{}' should exist in dataset",
                image_filename
            );
            let created_sample =
                created_sample.expect("Sample should exist (verified by assert above)");

            eprintln!("✓ Found sample by image_name: {}", image_filename);

            // Verify basic properties
            assert_eq!(
                created_sample.image_name.as_deref(),
                Some(&image_filename[..])
            );
            assert_eq!(created_sample.group, Some("train".to_string()));

            eprintln!("\nSample verification:");
            eprintln!("  ✓ image_name: {:?}", created_sample.image_name);
            eprintln!("  ✓ group: {:?}", created_sample.group);
            eprintln!(
                "  ✓ annotations: {} item(s)",
                created_sample.annotations.len()
            );

            // Note: The server currently doesn't return width/height or UUID fields in
            // samples.list This is a known server limitation (bug report
            // submitted).
            eprintln!(
                "  ⚠ uuid: {:?} (not returned by server)",
                created_sample.uuid
            );
            eprintln!(
                "  ⚠ width: {:?} (not returned by server)",
                created_sample.width
            );
            eprintln!(
                "  ⚠ height: {:?} (not returned by server)",
                created_sample.height
            );

            // Verify annotations are returned correctly
            let annotations = &created_sample.annotations;
            assert_eq!(annotations.len(), 1, "Should have exactly one annotation");

            let annotation = &annotations[0];
            assert_eq!(annotation.label(), Some(&"circle".to_string()));
            assert!(
                annotation.box2d().is_some(),
                "Bounding box should be present"
            );

            let returned_bbox = annotation
                .box2d()
                .expect("Bounding box should exist (verified by assert above)");
            eprintln!("\nAnnotation verification:");
            eprintln!("  ✓ label: {:?}", annotation.label());
            eprintln!(
                "  ✓ bbox: x={:.3}, y={:.3}, w={:.3}, h={:.3}",
                returned_bbox.left(),
                returned_bbox.top(),
                returned_bbox.width(),
                returned_bbox.height()
            );

            // Verify the bounding box coordinates match what we sent (within tolerance)
            assert!(
                (returned_bbox.left() - normalized_x).abs() < 0.01,
                "bbox.x should match (sent: {:.3}, got: {:.3})",
                normalized_x,
                returned_bbox.left()
            );
            assert!(
                (returned_bbox.top() - normalized_y).abs() < 0.01,
                "bbox.y should match (sent: {:.3}, got: {:.3})",
                normalized_y,
                returned_bbox.top()
            );
            assert!(
                (returned_bbox.width() - normalized_w).abs() < 0.01,
                "bbox.w should match (sent: {:.3}, got: {:.3})",
                normalized_w,
                returned_bbox.width()
            );
            assert!(
                (returned_bbox.height() - normalized_h).abs() < 0.01,
                "bbox.h should match (sent: {:.3}, got: {:.3})",
                normalized_h,
                returned_bbox.height()
            );

            // Verify the uploaded image matches what we sent (byte-for-byte)
            eprintln!("\nImage verification:");
            let downloaded_image = created_sample.download(&client, FileType::Image).await?;
            assert!(
                downloaded_image.is_some(),
                "Should be able to download the image"
            );
            let downloaded_data =
                downloaded_image.expect("Downloaded image should exist (verified by assert above)");

            assert_eq!(
                image_data.len(),
                downloaded_data.len(),
                "Downloaded image should have same size as uploaded"
            );
            assert_eq!(
                image_data, downloaded_data,
                "Downloaded image should match uploaded image byte-for-byte"
            );
            eprintln!("  ✓ Image data matches ({} bytes)", image_data.len());

            Ok::<(), Error>(())
        }
        .await;

        // Clean up temporary files
        let _ = std::fs::remove_file(&test_image_path);

        // Clean up test dataset (always execute, even if test failed)
        eprintln!("\nCleaning up test dataset...");
        client.delete_dataset(dataset_id).await?;
        eprintln!("  ✓ Deleted test dataset");

        // Propagate test result
        populate_result?;

        eprintln!("\n✓ Test passed: populate_samples with automatic upload");
        Ok(())
    }

    #[tokio::test]
    async fn test_deer_dataset_roundtrip() -> Result<(), Error> {
        use rand::Rng;

        let client = get_client().await?;

        // Find the Unit Testing project and Deer dataset (read-only)
        let projects = client.projects(Some("Unit Testing")).await?;
        let project = projects
            .first()
            .expect("'Unit Testing' project should exist");

        let datasets = client.datasets(project.id(), Some("Deer")).await?;
        let deer_dataset = datasets
            .iter()
            .find(|d| d.name() == "Deer")
            .expect("Deer dataset should exist");

        eprintln!("Found Deer dataset: {}", deer_dataset.id());

        // Get annotation sets
        let annotation_sets = client.annotation_sets(deer_dataset.id()).await?;
        let annotation_set = annotation_sets
            .first()
            .expect("Deer dataset should have annotation sets");

        eprintln!("Using annotation set: {}", annotation_set.id());

        // Download all samples from Deer dataset
        eprintln!("Downloading samples from Deer dataset...");
        let deer_samples = client
            .samples(
                deer_dataset.id(),
                Some(annotation_set.id()),
                &[],
                &[],
                &[],
                None,
            )
            .await?;

        eprintln!("Downloaded {} samples", deer_samples.len());
        assert!(!deer_samples.is_empty(), "Deer dataset should have samples");

        // Download annotations
        eprintln!("Downloading annotations...");
        let deer_annotations = client
            .annotations(annotation_set.id(), &[], &[], None)
            .await?;

        eprintln!("Downloaded {} annotations", deer_annotations.len());

        // Download a few sample images to verify data integrity
        eprintln!("Downloading sample images...");
        let test_dir = get_test_data_dir();
        let download_dir = test_dir.join(format!("deer_download_{}", std::process::id()));
        std::fs::create_dir_all(&download_dir)?;

        let mut downloaded_images = HashMap::new();
        for (idx, sample) in deer_samples.iter().take(5).enumerate() {
            if let Some(image_data) = sample.download(&client, FileType::Image).await? {
                let default_name = format!("sample_{}.jpg", idx);
                let image_name = sample.image_name().unwrap_or(&default_name);
                let image_path = download_dir.join(image_name);
                std::fs::write(&image_path, &image_data)?;
                downloaded_images.insert(image_name.to_string(), image_data);
                eprintln!("  Downloaded: {}", image_name);
            }
        }

        eprintln!(
            "Downloaded {} sample images for verification",
            downloaded_images.len()
        );

        // Create a test dataset with random suffix to avoid conflicts
        let random_suffix: u64 = rand::rng().random();
        let test_dataset_name = format!("Deer Test {:x}", random_suffix);

        eprintln!("Creating test dataset: {}", test_dataset_name);

        // Create the test dataset
        let test_dataset_id = client
            .create_dataset(
                &project.id().to_string(),
                &test_dataset_name,
                Some("Automated test: Deer dataset round-trip verification"),
            )
            .await?;

        eprintln!("Created test dataset: {}", test_dataset_id);

        // Create an annotation set
        eprintln!("Creating annotation set...");
        let test_annotation_set_id = client
            .create_annotation_set(test_dataset_id, "Default", Some("Default annotation set"))
            .await?;

        eprintln!("Created annotation set: {}", test_annotation_set_id);

        // Copy labels from Deer dataset
        let deer_labels = deer_dataset.labels(&client).await?;
        for label in &deer_labels {
            client.add_label(test_dataset_id, label.name()).await?;
        }
        eprintln!("Copied {} labels", deer_labels.len());

        // Prepare samples for upload
        eprintln!("Preparing samples for upload...");
        let mut upload_samples = Vec::new();

        for sample in deer_samples.iter().take(10) {
            // Limit to first 10 samples for testing
            let mut new_sample = Sample::new();

            // Copy metadata
            new_sample.group = sample.group().cloned();
            new_sample.sequence_name = sample.sequence_name().cloned();
            new_sample.frame_number = sample.frame_number();

            // Download and save image to temp file
            if let Some(image_data) = sample.download(&client, FileType::Image).await? {
                let default_name = format!("sample_{}.jpg", upload_samples.len());
                let image_name = sample.image_name().unwrap_or(&default_name);
                let temp_path = download_dir.join(image_name);
                std::fs::write(&temp_path, &image_data)?;
                new_sample.files = vec![SampleFile::with_filename(
                    "image".to_string(),
                    temp_path
                        .to_str()
                        .expect("Temp path should be valid UTF-8")
                        .to_string(),
                )];

                // Copy annotations for this sample
                let sample_annotations: Vec<Annotation> = deer_annotations
                    .iter()
                    .filter(|ann| {
                        ann.name()
                            .map(|n| sample.image_name().map(|s| n == s).unwrap_or(false))
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                new_sample.annotations = sample_annotations;

                upload_samples.push(new_sample);
            }
        }

        eprintln!("Prepared {} samples for upload", upload_samples.len());
        assert!(
            !upload_samples.is_empty(),
            "Should have samples prepared for upload"
        );

        // Upload samples
        eprintln!("Uploading samples to test dataset...");
        let results = client
            .populate_samples(
                test_dataset_id,
                Some(test_annotation_set_id),
                upload_samples,
                None,
            )
            .await?;

        eprintln!("Uploaded {} samples", results.len());

        // Give the server time to process
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        // Verify uploaded data
        eprintln!("Verifying uploaded data...");
        let uploaded_samples = client
            .samples(
                test_dataset_id,
                Some(test_annotation_set_id),
                &[],
                &[],
                &[],
                None,
            )
            .await?;

        eprintln!("Found {} uploaded samples", uploaded_samples.len());
        assert_eq!(
            uploaded_samples.len(),
            results.len(),
            "Should have same number of uploaded samples"
        );

        // Verify a few images match byte-for-byte
        let mut verified_count = 0;
        for (original_name, original_data) in downloaded_images.iter().take(3) {
            if let Some(uploaded_sample) = uploaded_samples
                .iter()
                .find(|s| s.image_name().map(|n| n == original_name).unwrap_or(false))
                && let Some(uploaded_data) =
                    uploaded_sample.download(&client, FileType::Image).await?
            {
                assert_eq!(
                    original_data.len(),
                    uploaded_data.len(),
                    "Image {} should have same size",
                    original_name
                );
                assert_eq!(
                    original_data, &uploaded_data,
                    "Image {} should match byte-for-byte",
                    original_name
                );
                verified_count += 1;
                eprintln!("  ✓ Verified: {}", original_name);
            }
        }

        assert!(
            verified_count > 0,
            "Should have verified at least one image"
        );
        eprintln!("Verified {} images match byte-for-byte", verified_count);

        // Verify annotations were uploaded
        let uploaded_annotations = client
            .annotations(test_annotation_set_id, &[], &[], None)
            .await?;

        eprintln!("Found {} uploaded annotations", uploaded_annotations.len());
        assert!(
            !uploaded_annotations.is_empty(),
            "Should have uploaded annotations"
        );

        // Clean up: Delete the test dataset
        eprintln!("Cleaning up test dataset...");
        client.delete_dataset(test_dataset_id).await?;
        eprintln!("  ✓ Deleted test dataset");

        // Clean up downloaded files
        std::fs::remove_dir_all(&download_dir)?;
        eprintln!("  ✓ Cleaned up downloaded files");

        eprintln!("\n✅ Round-trip test completed successfully!");

        Ok(())
    }
}
