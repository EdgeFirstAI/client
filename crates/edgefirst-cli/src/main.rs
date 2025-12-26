// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use clap::{Parser, Subcommand};
use edgefirst_client::{
    AnnotationSetID, AnnotationType, Client, Dataset, DatasetID, Error, FileType, Progress,
    SnapshotID, TaskID, TrainingSession,
};
use inquire::{Password, PasswordDisplayMode};
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// EdgeFirst Studio Server Name
    #[clap(long, env = "STUDIO_SERVER")]
    server: Option<String>,

    /// EdgeFirst Studio Username
    #[clap(long, env = "STUDIO_USERNAME")]
    username: Option<String>,

    /// EdgeFirst Studio Password
    #[clap(long, env = "STUDIO_PASSWORD")]
    password: Option<String>,

    /// EdgeFirst Studio Token
    #[clap(long, env = "STUDIO_TOKEN")]
    token: Option<String>,

    /// Client Command
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
enum Command {
    /// Returns the EdgeFirst Studio Server version.
    Version,
    /// Login to the EdgeFirst Studio Server with the provided username and
    /// password.  The token is stored in the application configuration file.
    Login,
    /// Logout by removing the token from the application configuration file.
    Logout,
    /// Sleep for the specified number of seconds (for testing purposes).
    Sleep {
        /// Number of seconds to sleep
        seconds: u64,
    },
    /// Returns the EdgeFirst Studio authentication token for the provided
    /// username and password.  This would typically be stored into the
    /// STUDIO_TOKEN environment variable for subsequent commands to avoid
    /// re-entering username/password.
    Token,
    /// Show the user's organization information.
    Organization,
    /// List all projects available to the authenticated user.
    Projects {
        /// Filter projects by name
        #[clap(long)]
        name: Option<String>,
    },
    /// Retrieve project information for the provided project ID.
    Project {
        /// Project ID
        project_id: String,
    },
    /// List all datasets available to the authenticated user.  If a project ID
    /// is provided, only datasets for that project are listed.
    Datasets {
        /// Project ID
        project_id: Option<String>,

        /// List available annotation sets for the datasets
        #[clap(long, short)]
        annotation_sets: bool,

        /// List available labels for the datasets
        #[clap(long, short)]
        labels: bool,

        /// Filter datasets by name
        #[clap(long)]
        name: Option<String>,
    },
    /// Retrieve dataset information for the provided dataset ID.
    Dataset {
        /// Dataset ID
        dataset_id: String,

        /// List available annotation sets for the dataset
        #[clap(long, short)]
        annotation_sets: bool,

        /// List available labels for the dataset
        #[clap(long, short)]
        labels: bool,
    },
    /// Create a new dataset in the specified project.
    CreateDataset {
        /// Project ID
        project_id: String,

        /// Dataset name
        name: String,

        /// Dataset description
        #[clap(long)]
        description: Option<String>,
    },
    /// Delete a dataset by marking it as deleted.
    DeleteDataset {
        /// Dataset ID
        dataset_id: String,
    },
    /// Create a new annotation set for the specified dataset.
    CreateAnnotationSet {
        /// Dataset ID
        dataset_id: String,

        /// Annotation set name
        name: String,

        /// Annotation set description
        #[clap(long)]
        description: Option<String>,
    },
    /// Delete an annotation set by marking it as deleted.
    DeleteAnnotationSet {
        /// Annotation Set ID
        annotation_set_id: String,
    },
    /// Download a dataset to the local filesystem from the EdgeFirst Studio
    /// server.  The dataset ID is required along with an optional output file
    /// path, if none is provided the dataset is downloaded to the current
    /// working directory.
    DownloadDataset {
        /// Dataset ID
        dataset_id: String,

        /// Only fetch samples belonging to the provided dataset groups.
        #[clap(long, value_delimiter = ',')]
        groups: Vec<String>,

        /// Fetch the data types for the dataset, if empty then nothing is
        /// fetched.
        #[clap(long, default_value = "image", value_delimiter = ',')]
        types: Vec<FileType>,

        /// Output File Path
        #[clap(long)]
        output: Option<PathBuf>,

        /// Download all files to the output directory without creating sequence
        /// subdirectories. When enabled, filenames are automatically prefixed
        /// with the sequence name and frame number to avoid conflicts.
        #[clap(long)]
        flatten: bool,
    },
    /// Download dataset annotations to a local file.  This command accompanies
    /// the `DownloadDataset` command and is used to download the annotations
    /// rather than the dataset file samples (images, radar, lidar, etc.).
    ///
    /// The annotations will be fetched into a format matching the output file
    /// extension.  Currently we support `.json` and `.arrow` formats.  The
    /// arrow format is the EdgeFirst Dataset Format and documented at the
    /// following link: https://doc.edgefirst.ai/latest/datasets/format/
    DownloadAnnotations {
        /// Annotation Set ID
        annotation_set_id: String,

        /// Only fetch samples belonging to the provided dataset groups.
        #[clap(long, value_delimiter = ',')]
        groups: Vec<String>,

        /// Annotation Types to download, if empty then every type is fetched.
        #[clap(long, default_value = "box2d", value_delimiter = ',')]
        types: Vec<AnnotationType>,

        /// Output File Path
        output: PathBuf,
    },
    /// Upload samples to a dataset from images and/or Arrow annotations file.
    /// Supports flexible workflows: images-only, annotations-only, or both.
    /// Arrow file must follow EdgeFirst Dataset Format
    /// (https://doc.edgefirst.ai/latest/datasets/format/).
    ///
    /// Image discovery (if --images not provided):
    /// - Looks for folder named after Arrow file (e.g., "data/" for
    ///   "data.arrow")
    /// - Or folder named "dataset/"
    /// - Or ZIP file with same basename (e.g., "data.zip" for "data.arrow")
    /// - Or "dataset.zip"
    UploadDataset {
        /// Dataset ID to upload samples to
        dataset_id: String,

        /// Path to Arrow file with annotations (EdgeFirst Dataset Format).
        /// If omitted, only images will be uploaded.
        #[clap(long)]
        annotations: Option<PathBuf>,

        /// Path to folder or ZIP containing images.
        /// If omitted, auto-discovers based on Arrow filename or "dataset"
        /// convention.
        #[clap(long)]
        images: Option<PathBuf>,

        /// Annotation Set ID for the annotations.
        /// Required if Arrow file contains annotations.
        #[clap(long)]
        annotation_set_id: Option<String>,
    },
    /// List training experiments for the provided project ID (optional).  The
    /// experiments are a method of grouping training sessions together.
    Experiments {
        /// Project ID
        project_id: Option<String>,

        /// Filter experiments by name
        #[clap(long)]
        name: Option<String>,
    },
    /// Retrieve the experiment with the provided ID.
    Experiment {
        /// Experiment ID
        experiment_id: String,
    },
    /// List training sessions for the provided experiment ID (optional).  The
    /// sessions are individual training jobs that can be queried for more
    /// detailed information.
    TrainingSessions {
        /// Optional experiment ID to limit the training sessions.
        experiment_id: Option<String>,

        /// Filter sessions by name
        #[clap(long)]
        name: Option<String>,
    },
    /// Retrieve training session information for the provided session ID.  The
    /// trainer session ID can be either be an integer or a string with the
    /// format t-xxx where xxx is the session ID in hex as shown in the Web UI.
    TrainingSession {
        /// Training Session ID
        training_session_id: String,

        /// List the model parameters for the training session
        #[clap(long, short)]
        model: bool,

        /// List the dataset parameters for the training session
        #[clap(long, short)]
        dataset: bool,

        /// List available artifacts for the training session
        #[clap(long, short)]
        artifacts: bool,
    },
    /// Download an artifact from the provided session ID.  The session ID can
    /// be either be an integer or a string with the format t-xxx where xxx is
    /// the session ID in hex as shown in the Web UI.  The artifact name is the
    /// name of the file to download.  The output file path is optional, if none
    /// is provided the artifact is downloaded to the current working directory.
    DownloadArtifact {
        /// Training Session ID
        session_id: String,

        /// Name of the artifact to download
        name: String,

        /// Optional output file path, otherwise the artifact is downloaded to
        /// the current working directory.
        #[clap(long)]
        output: Option<PathBuf>,
    },
    /// Upload an artifact to the provided training session ID.
    UploadArtifact {
        /// Training Session ID
        session_id: String,

        /// Artifact to upload, the name of the artifact will be the file's
        /// base name unless the name parameter is provided.
        path: PathBuf,

        /// Optional name of the artifact, otherwise the file's base name is
        /// used.
        #[clap(long)]
        name: Option<String>,
    },
    /// List all tasks for the current user.
    Tasks {
        /// Retrieve the task stages.
        #[clap(long)]
        stages: bool,

        /// Filter tasks by name
        #[clap(long)]
        name: Option<String>,

        /// Filter tasks by workflow
        #[clap(long)]
        workflow: Option<String>,

        /// Filter tasks by status
        #[clap(long)]
        status: Option<String>,

        /// Filter tasks by manager type
        #[clap(long)]
        manager: Option<String>,
    },
    /// Retrieve information about a specific task.
    Task {
        /// Task ID to retrieve
        task_id: String,

        /// Monitor the task progress until completion
        #[clap(long)]
        monitor: bool,
    },
    /// List validation sessions for the provided project ID.
    ValidationSessions {
        /// Project ID
        project_id: String,
    },
    /// Retrieve validation session information for the provided session ID.
    ValidationSession {
        /// Validation Session ID
        session_id: String,
    },
    /// List all snapshots available to the user.
    Snapshots,
    /// Retrieve snapshot information for the provided snapshot ID.
    Snapshot {
        /// Snapshot ID
        snapshot_id: String,
    },
    /// Create a snapshot from a local file/directory or server-side dataset.
    ///
    /// Supports multiple source types with smart argument interpretation:
    /// - Dataset ID (ds-xxx): Create from server dataset
    /// - Annotation Set ID (as-xxx): Create from annotation set's parent
    ///   dataset
    /// - Local path: Upload MCAP, Arrow manifest, folder, or ZIP file
    ///
    /// Examples:
    ///   edgefirst-client create-snapshot ds-123
    ///   edgefirst-client create-snapshot as-456
    ///   edgefirst-client create-snapshot ./data.mcap
    ///   edgefirst-client create-snapshot ./dataset/
    CreateSnapshot {
        /// Source: dataset ID (ds-xxx), annotation set ID (as-xxx), or local
        /// path
        source: String,

        /// Optional annotation set when source is a dataset ID
        #[clap(long)]
        annotation_set: Option<String>,

        /// Description for the snapshot (auto-generated from source if not
        /// provided)
        #[clap(long, short = 'd')]
        description: Option<String>,

        /// Explicit: treat source as local path (--from-path)
        #[clap(long, conflicts_with = "from_dataset")]
        from_path: bool,

        /// Explicit: treat source as dataset ID (--from-dataset)
        #[clap(long, conflicts_with = "from_path")]
        from_dataset: bool,

        /// Monitor the task progress until completion (server-side only)
        #[clap(long, short = 'm')]
        monitor: bool,
    },
    /// Download a snapshot to local storage.
    DownloadSnapshot {
        /// Snapshot ID
        snapshot_id: String,

        /// Output directory path
        #[clap(long)]
        output: PathBuf,
    },
    /// Restore a snapshot to a dataset in EdgeFirst Studio.
    /// Supports MCAP uploads with optional AGTG (auto-annotation) and
    /// auto-depth generation.
    RestoreSnapshot {
        /// Project ID to restore snapshot into
        project_id: String,

        /// Snapshot ID to restore
        snapshot_id: String,

        /// MCAP topics to include (comma-separated, empty = all)
        #[clap(long, value_delimiter = ',')]
        topics: Vec<String>,

        /// Object labels for AGTG auto-annotation (comma-separated, empty = no
        /// AGTG)
        #[clap(long, value_delimiter = ',')]
        autolabel: Vec<String>,

        /// Generate depthmaps (Maivin/Raivin cameras only)
        #[clap(long)]
        autodepth: bool,

        /// Custom dataset name
        #[clap(long)]
        dataset_name: Option<String>,

        /// Dataset description
        #[clap(long)]
        dataset_description: Option<String>,

        /// Monitor the restore task progress until completion
        #[clap(long)]
        monitor: bool,
    },
    /// Delete a snapshot from EdgeFirst Studio.
    DeleteSnapshot {
        /// Snapshot ID to delete
        snapshot_id: String,
    },
    /// Generate an Arrow annotation file from a folder of images.
    ///
    /// Creates an Arrow file with null annotations for each image found.
    /// Useful for importing existing image collections into EdgeFirst format.
    ///
    /// The command will:
    /// 1. Scan the folder recursively for image files (JPEG, PNG)
    /// 2. Optionally detect sequence patterns (name_frame.ext)
    /// 3. Create an Arrow file with the 2025.10 schema
    ///
    /// Examples:
    ///   edgefirst generate-arrow ./images --output dataset.arrow
    ///   edgefirst generate-arrow ./images -o my_data/my_data.arrow
    /// --detect-sequences
    GenerateArrow {
        /// Folder containing images to process
        folder: PathBuf,

        /// Output Arrow file path
        #[clap(long, short = 'o')]
        output: PathBuf,

        /// Detect sequence patterns (name_frame.ext) in filenames
        #[clap(long, default_value = "true")]
        detect_sequences: bool,
    },
    /// Validate a snapshot directory structure.
    ///
    /// Checks that the directory follows the EdgeFirst Dataset Format:
    /// - Arrow file exists at expected location
    /// - Sensor container directory exists
    /// - All files referenced in Arrow file exist
    ///
    /// Examples:
    ///   edgefirst validate-snapshot ./my_dataset
    ValidateSnapshot {
        /// Snapshot directory to validate
        path: PathBuf,

        /// Show detailed validation issues
        #[clap(long, short = 'v')]
        verbose: bool,
    },
    /// Convert COCO annotations to EdgeFirst Arrow format.
    ///
    /// Reads a COCO annotation JSON file or ZIP archive and converts it to
    /// the EdgeFirst Dataset Format (Arrow). Supports bbox and polygon
    /// segmentation annotations.
    ///
    /// Examples:
    ///   edgefirst coco-to-arrow instances.json -o dataset.arrow
    ///   edgefirst coco-to-arrow coco.zip -o dataset.arrow --group train
    CocoToArrow {
        /// Path to COCO annotation file (JSON) or ZIP archive
        coco_path: PathBuf,

        /// Output Arrow file path
        #[clap(long, short = 'o')]
        output: PathBuf,

        /// Include segmentation masks
        #[clap(long, default_value = "true")]
        masks: bool,

        /// Group name for all samples (e.g., "train", "val")
        #[clap(long)]
        group: Option<String>,
    },
    /// Convert EdgeFirst Arrow format to COCO annotations.
    ///
    /// Reads an EdgeFirst Arrow file and converts it to COCO JSON format.
    /// Supports bbox and polygon segmentation annotations.
    ///
    /// Examples:
    ///   edgefirst arrow-to-coco dataset.arrow -o instances.json
    ///   edgefirst arrow-to-coco dataset.arrow -o instances.json --groups train,val
    ArrowToCoco {
        /// Path to EdgeFirst Arrow file
        arrow_path: PathBuf,

        /// Output COCO JSON file path
        #[clap(long, short = 'o')]
        output: PathBuf,

        /// Include segmentation masks
        #[clap(long, default_value = "true")]
        masks: bool,

        /// Filter by group names (comma-separated)
        #[clap(long, value_delimiter = ',')]
        groups: Vec<String>,

        /// Pretty-print JSON output
        #[clap(long)]
        pretty: bool,
    },
}

// Command handler functions

async fn handle_version(client: &Client) -> Result<(), Error> {
    let version = client.version().await?;
    println!(
        "EdgeFirst Studio Server [{}]: {} Client: {}",
        client.url(),
        version,
        env!("CARGO_PKG_VERSION")
    );
    Ok(())
}

async fn handle_login(
    client: Client,
    username: Option<String>,
    password: Option<String>,
) -> Result<(), Error> {
    let (username, password) = match (username, password) {
        (Some(username), Some(password)) => (username, password),
        (Some(username), None) => {
            let password = Password::new("EdgeFirst Studio Password")
                .with_display_mode(PasswordDisplayMode::Masked)
                .without_confirmation()
                .prompt()
                .unwrap();
            (username, password)
        }
        _ => {
            let username = Password::new("EdgeFirst Studio Username")
                .with_display_mode(PasswordDisplayMode::Full)
                .without_confirmation()
                .prompt()
                .unwrap();
            let password = Password::new("EdgeFirst Studio Password")
                .with_display_mode(PasswordDisplayMode::Masked)
                .without_confirmation()
                .prompt()
                .unwrap();
            (username, password)
        }
    };

    let client = client.with_login(&username, &password).await?;
    client.save_token().await?;

    let username = client.username().await?;
    let expires = client.token_expiration().await?;

    println!("Successfully logged into EdgeFirst Studio as {}", username);
    println!("Token for {} expires at {}", client.url(), expires);

    Ok(())
}

async fn handle_logout(client: &Client) -> Result<(), Error> {
    client.logout().await?;
    println!("Successfully logged out of EdgeFirst Studio");
    Ok(())
}

async fn handle_sleep(seconds: u64) -> Result<(), Error> {
    println!("Sleeping for {} seconds...", seconds);
    tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
    println!("Sleep complete");
    Ok(())
}

async fn handle_token(client: &Client) -> Result<(), Error> {
    let token = client.token().await;
    println!("{}", token);
    Ok(())
}

async fn handle_organization(client: &Client) -> Result<(), Error> {
    let org = client.organization().await?;
    println!(
        "Username: {}\nOrganization: {}\nID: {}\nCredits: {}",
        client.username().await?,
        org.name(),
        org.id(),
        org.credits()
    );
    Ok(())
}

async fn handle_projects(client: &Client, name: Option<String>) -> Result<(), Error> {
    let projects = client.projects(name.as_deref()).await?;
    for project in projects {
        println!(
            "[{}] {}: {}",
            project.id(),
            project.name(),
            project.description()
        );
    }
    Ok(())
}

async fn handle_project(client: &Client, project_id: String) -> Result<(), Error> {
    let project = client.project(project_id.try_into()?).await?;
    println!(
        "[{}] {}: {}",
        project.id(),
        project.name(),
        project.description()
    );
    Ok(())
}

async fn print_dataset_details(
    client: &Client,
    dataset: &Dataset,
    show_labels: bool,
    show_annotation_sets: bool,
) -> Result<(), Error> {
    println!(
        "[{}] {}: {}",
        dataset.id(),
        dataset.name(),
        dataset.description()
    );

    if show_labels {
        let labels = client.labels(dataset.id()).await?;
        println!("Labels:");
        for label in labels {
            println!("    [{}] {}", label.id(), label.name());
        }
    }

    if show_annotation_sets {
        let annotation_sets = client.annotation_sets(dataset.id()).await?;
        println!("Annotation Sets:");
        for annotation_set in annotation_sets {
            println!(
                "[{}] {}: {}",
                annotation_set.id(),
                annotation_set.name(),
                annotation_set.description(),
            );
        }
    }
    Ok(())
}

async fn handle_datasets(
    client: &Client,
    project_id: Option<String>,
    annotation_sets: bool,
    labels: bool,
    name: Option<String>,
) -> Result<(), Error> {
    if let Some(project_id) = project_id {
        let datasets = client
            .datasets(project_id.try_into()?, name.as_deref())
            .await?;
        for dataset in datasets {
            print_dataset_details(client, &dataset, labels, annotation_sets).await?;
        }
    } else {
        let projects = client.projects(None).await?;
        for project in projects {
            let datasets = client.datasets(project.id(), name.as_deref()).await?;
            for dataset in datasets {
                println!(
                    "[{}] {}: {}",
                    dataset.id(),
                    dataset.name(),
                    dataset.description()
                );
            }
        }
    }
    Ok(())
}

async fn handle_dataset(
    client: &Client,
    dataset_id: String,
    annotation_sets: bool,
    labels: bool,
) -> Result<(), Error> {
    let dataset = client.dataset(dataset_id.clone().try_into()?).await?;
    println!(
        "[{}] {}: {}",
        dataset.id(),
        dataset.name(),
        dataset.description()
    );

    if labels {
        let labels = client.labels(dataset_id.clone().try_into()?).await?;
        println!("Labels:");
        for label in labels {
            println!("    [{}] {}", label.id(), label.name());
        }
    }

    if annotation_sets {
        let annotation_sets = client.annotation_sets(dataset_id.try_into()?).await?;
        println!("Annotation Sets:");
        for annotation_set in annotation_sets {
            println!(
                "[{}] {}: {}",
                annotation_set.id(),
                annotation_set.name(),
                annotation_set.description(),
            );
        }
    }
    Ok(())
}

async fn handle_create_dataset(
    client: &Client,
    project_id: String,
    name: String,
    description: Option<String>,
) -> Result<(), Error> {
    let project_id: edgefirst_client::ProjectID = project_id.try_into()?;
    let dataset_id = client
        .create_dataset(
            project_id.to_string().as_str(),
            &name,
            description.as_deref(),
        )
        .await?;
    println!("Created dataset with ID: {}", dataset_id);
    Ok(())
}

async fn handle_delete_dataset(client: &Client, dataset_id: String) -> Result<(), Error> {
    let dataset_id: edgefirst_client::DatasetID = dataset_id.try_into()?;
    client.delete_dataset(dataset_id).await?;
    println!("Dataset {} marked as deleted", dataset_id);
    Ok(())
}

async fn handle_create_annotation_set(
    client: &Client,
    dataset_id: String,
    name: String,
    description: Option<String>,
) -> Result<(), Error> {
    let dataset_id: edgefirst_client::DatasetID = dataset_id.try_into()?;
    let annotation_set_id = client
        .create_annotation_set(dataset_id, &name, description.as_deref())
        .await?;
    println!("Created annotation set with ID: {}", annotation_set_id);
    Ok(())
}

async fn handle_delete_annotation_set(
    client: &Client,
    annotation_set_id: String,
) -> Result<(), Error> {
    let annotation_set_id: edgefirst_client::AnnotationSetID = annotation_set_id.try_into()?;
    client.delete_annotation_set(annotation_set_id).await?;
    println!("Annotation set {} marked as deleted", annotation_set_id);
    Ok(())
}

async fn handle_download_dataset(
    client: &Client,
    dataset_id: String,
    groups: Vec<String>,
    types: Vec<edgefirst_client::FileType>,
    output: PathBuf,
    flatten: bool,
) -> Result<(), Error> {
    use indicatif::{ProgressBar, ProgressStyle};
    use tokio::sync::mpsc;

    let bar = ProgressBar::new(0);
    bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise} ETA: {eta}] {msg}: {wide_bar:.yellow} {human_pos}/{human_len}",
        )
        .unwrap()
        .progress_chars("█▇▆▅▄▃▂▁  "),
    );

    let (tx, mut rx) = mpsc::channel::<Progress>(1);

    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            if progress.total > 0 {
                bar.set_length(progress.total as u64);
                bar.set_position(progress.current as u64);
            }
        }
    });

    client
        .download_dataset(
            dataset_id.try_into()?,
            &groups,
            &types,
            output,
            flatten,
            Some(tx),
        )
        .await?;
    Ok(())
}

async fn handle_download_annotations(
    client: &Client,
    annotation_set_id: String,
    groups: Vec<String>,
    types: Vec<edgefirst_client::AnnotationType>,
    output: PathBuf,
) -> Result<(), Error> {
    use indicatif::{ProgressBar, ProgressStyle};
    use std::io::Write;
    use tokio::sync::mpsc;

    let bar = ProgressBar::new(0);
    bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise} ETA: {eta}] {msg}: {wide_bar:.yellow} {human_pos}/{human_len}",
        )
        .unwrap()
        .progress_chars("█▇▆▅▄▃▂▁  "),
    );

    let (tx, mut rx) = mpsc::channel::<Progress>(1);

    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            if progress.total > 0 {
                bar.set_length(progress.total as u64);
                bar.set_position(progress.current as u64);
            }
        }
    });

    // Get the dataset_id from the annotation set
    let annotation_set_id = annotation_set_id.try_into()?;
    let annotation_set = client.annotation_set(annotation_set_id).await?;
    let dataset_id = annotation_set.dataset_id();

    let format = output
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());

    match format {
        Some(ext) if ext == "json" => {
            let annotations = client
                .annotations(annotation_set_id, &groups, &types, Some(tx))
                .await?;
            let mut file = File::create(&output)?;
            file.write_all(serde_json::to_string_pretty(&annotations)?.as_bytes())?;
        }
        Some(ext) if ext == "arrow" => {
            #[cfg(feature = "polars")]
            {
                use polars::{io::SerWriter as _, prelude::IpcWriter};

                let mut df = client
                    .samples_dataframe(
                        dataset_id,
                        Some(annotation_set_id),
                        &groups,
                        &types,
                        Some(tx),
                    )
                    .await?;
                IpcWriter::new(File::create(output).unwrap())
                    .finish(&mut df)
                    .unwrap();
            }
            #[cfg(not(feature = "polars"))]
            {
                return Err(Error::FeatureNotEnabled("polars".to_owned()));
            }
        }
        _ => {
            return Err(Error::InvalidParameters(format!(
                "Unsupported output format: {:?}",
                format
            )));
        }
    }
    Ok(())
}

#[cfg(feature = "polars")]
fn find_image_folder(base_dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    let folder = base_dir.join(name);
    if folder.exists() && folder.is_dir() {
        Some(folder)
    } else {
        None
    }
}
#[cfg(feature = "polars")]
fn find_image_zip(base_dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    let zip_file = base_dir.join(format!("{}.zip", name));
    if zip_file.exists() {
        Some(zip_file)
    } else {
        None
    }
}

#[cfg(feature = "polars")]
fn find_image_source(arrow_path: &Path) -> Result<PathBuf, Error> {
    use std::path::Path;

    let arrow_dir = arrow_path.parent().unwrap_or_else(|| Path::new("."));
    let arrow_stem = arrow_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("dataset");

    let mut tried_paths = Vec::new();

    // Try folder with arrow basename
    if let Some(folder) = find_image_folder(arrow_dir, arrow_stem) {
        return Ok(folder);
    }
    tried_paths.push(arrow_dir.join(arrow_stem).display().to_string() + "/");

    // Try folder with "dataset" name
    if let Some(folder) = find_image_folder(arrow_dir, "dataset") {
        return Ok(folder);
    }
    tried_paths.push(arrow_dir.join("dataset").display().to_string() + "/");

    // Try zip with arrow basename
    if let Some(zip) = find_image_zip(arrow_dir, arrow_stem) {
        return Ok(zip);
    }
    tried_paths.push(
        arrow_dir
            .join(format!("{}.zip", arrow_stem))
            .display()
            .to_string(),
    );

    // Try zip with "dataset" name
    if let Some(zip) = find_image_zip(arrow_dir, "dataset") {
        return Ok(zip);
    }
    tried_paths.push(arrow_dir.join("dataset.zip").display().to_string());

    Err(Error::InvalidParameters(format!(
        "Could not find images. Tried:\n  - {}\nPlease specify --images explicitly.",
        tried_paths.join("\n  - ")
    )))
}

#[cfg(feature = "polars")]
fn determine_images_path(
    annotations: &Option<PathBuf>,
    images: &Option<PathBuf>,
) -> Result<PathBuf, Error> {
    let images_path = if let Some(img_path) = images {
        img_path.clone()
    } else if let Some(arrow_path) = annotations {
        find_image_source(arrow_path)?
    } else {
        return Err(Error::InvalidParameters(
            "When --annotations is not provided, --images must be specified".to_owned(),
        ));
    };

    if !images_path.exists() {
        return Err(Error::InvalidParameters(format!(
            "Images path does not exist: {}",
            images_path.display()
        )));
    }

    Ok(images_path)
}

/// Extract a 2D bounding box from a DataFrame row.
///
/// Attempts to read the "box2d" column as an array of f32 values at the
/// specified row index. If successful and the array contains at least 4
/// values, converts from center-based format (cx, cy, width, height) to
/// corner-based format (x, y, width, height) and returns a Box2d.
///
/// Returns Ok(None) if the column doesn't exist, has wrong type, or
/// insufficient data.
#[cfg(feature = "polars")]
fn parse_box2d_from_dataframe(
    df: &polars::prelude::DataFrame,
    idx: usize,
) -> Result<Option<edgefirst_client::Box2d>, Error> {
    // Try to get the box2d column
    let box2d_col = match df.column("box2d") {
        Ok(col) => col,
        Err(_) => return Ok(None),
    };

    let extract_coords = |series: polars::prelude::Series| -> Option<Vec<f32>> {
        if let Ok(vals) = series.f32() {
            return Some(vals.into_iter().flatten().collect());
        }

        if let Ok(vals) = series.f64() {
            return Some(vals.into_iter().flatten().map(|v| v as f32).collect());
        }

        None
    };

    let coords = if let Ok(array_chunked) = box2d_col.array() {
        array_chunked
            .get_as_series(idx)
            .and_then(|series| extract_coords(series.clone()))
    } else if let Ok(list_chunked) = box2d_col.list() {
        list_chunked
            .get_as_series(idx)
            .and_then(|series| extract_coords(series.clone()))
    } else {
        None
    };

    let coords = match coords {
        Some(values) => values,
        None => return Ok(None),
    };

    if coords.len() >= 4 {
        // Convert from center-based (cx, cy, w, h) to corner-based (x, y, w, h)
        let cx = coords[0];
        let cy = coords[1];
        let w = coords[2];
        let h = coords[3];
        let x = cx - w / 2.0;
        let y = cy - h / 2.0;
        let bbox = edgefirst_client::Box2d::new(x, y, w, h);
        return Ok(Some(bbox));
    }

    Ok(None)
}

/// Extract a 3D bounding box from a DataFrame row.
///
/// Attempts to read the "box3d" column as an array of f32 values at the
/// specified row index. If successful and the array contains at least 6
/// values (x, y, z, width, height, depth), returns a Box3d.
///
/// Returns Ok(None) if the column doesn't exist, has wrong type, or
/// insufficient data.
#[cfg(feature = "polars")]
fn parse_box3d_from_dataframe(
    df: &polars::prelude::DataFrame,
    idx: usize,
) -> Result<Option<edgefirst_client::Box3d>, Error> {
    // Try to get the box3d column
    let box3d_col = match df.column("box3d") {
        Ok(col) => col,
        Err(_) => return Ok(None),
    };

    let extract_coords = |series: polars::prelude::Series| -> Option<Vec<f32>> {
        if let Ok(vals) = series.f32() {
            return Some(vals.into_iter().flatten().collect());
        }

        if let Ok(vals) = series.f64() {
            return Some(vals.into_iter().flatten().map(|v| v as f32).collect());
        }

        None
    };

    let coords = if let Ok(array_chunked) = box3d_col.array() {
        array_chunked
            .get_as_series(idx)
            .and_then(|series| extract_coords(series.clone()))
    } else if let Ok(list_chunked) = box3d_col.list() {
        list_chunked
            .get_as_series(idx)
            .and_then(|series| extract_coords(series.clone()))
    } else {
        None
    };

    let coords = match coords {
        Some(values) => values,
        None => return Ok(None),
    };

    if coords.len() >= 6 {
        let box3d = edgefirst_client::Box3d::new(
            coords[0], coords[1], coords[2], coords[3], coords[4], coords[5],
        );
        return Ok(Some(box3d));
    }

    Ok(None)
}

/// Extract a polygon mask from a DataFrame row.
///
/// Attempts to read the "mask" column as a list of f32 coordinates at the
/// specified row index. Coordinates are pairs of (x, y) values, with NaN
/// values used as separators between multiple polygons in the same mask.
///
/// Returns Ok(None) if the column doesn't exist, has wrong type, or
/// insufficient data.
#[cfg(feature = "polars")]
fn parse_mask_from_dataframe(
    df: &polars::prelude::DataFrame,
    idx: usize,
) -> Result<Option<edgefirst_client::Mask>, Error> {
    // Try to get the mask column
    let mask_col = match df.column("mask") {
        Ok(col) => col,
        Err(_) => return Ok(None),
    };

    // Convert to list type
    let list_chunked = match mask_col.list() {
        Ok(list) => list,
        Err(_) => return Ok(None),
    };

    // Get the series at the specified index
    let mask_series = match list_chunked.get_as_series(idx) {
        Some(series) => series,
        None => return Ok(None),
    };

    let coords: Vec<f32> = if let Ok(values) = mask_series.f32() {
        values.into_iter().flatten().collect()
    } else if let Ok(values) = mask_series.f64() {
        values.into_iter().flatten().map(|val| val as f32).collect()
    } else {
        return Ok(None);
    };
    if !coords.is_empty() {
        // Use the unflatten helper to convert flat coords with NaN separators back to
        // nested polygons
        let polygons = edgefirst_client::unflatten_polygon_coordinates(&coords);

        if !polygons.is_empty() {
            let mask = edgefirst_client::Mask::new(polygons);
            return Ok(Some(mask));
        }
    }

    Ok(None)
}

/// Generates deterministic UUIDs for sequences and samples during upload.
///
/// For sequences: Uses SHA-1 hash of "{dataset_id}/{sequence_name}" to generate
/// a deterministic UUID conforming to RFC 4122 version 5. This ensures all
/// samples in the same sequence across multiple uploads get the same
/// sequence_uuid.
///
/// For samples: Generates a random UUID for each sample (required by Studio).
///
/// These UUIDs are only set during upload and are not persisted to Arrow/JSON
/// files as they are internal Studio details.
///
/// # Arguments
///
/// * `samples` - Mutable vector of samples to process
/// * `dataset_id` - Dataset ID for hash generation
fn generate_upload_uuids(samples: &mut [edgefirst_client::Sample], dataset_id: &str) {
    use sha1::{Digest, Sha1};
    use std::collections::HashMap;

    let mut sequence_uuid_map: HashMap<String, String> = HashMap::new();

    for sample in samples.iter_mut() {
        // Generate sample UUID (required by Studio for all samples)
        sample.uuid = Some(uuid::Uuid::new_v4().to_string());

        // Generate sequence_uuid if sample is part of a sequence
        if let Some(seq_name) = &sample.sequence_name {
            let seq_uuid = sequence_uuid_map
                .entry(seq_name.clone())
                .or_insert_with(|| {
                    // Create deterministic UUID from hash of dataset_id/sequence_name
                    // Server respects client-provided sequence_uuid values
                    // Using SHA-1 to properly conform to RFC 4122 version 5 UUID
                    let input = format!("{}/{}", dataset_id, seq_name);
                    let hash = Sha1::digest(input.as_bytes());

                    // Convert first 16 bytes of hash to UUID v5 format
                    let uuid_bytes: [u8; 16] = hash[..16].try_into().unwrap();
                    let mut uuid = uuid::Uuid::from_bytes(uuid_bytes);

                    // Set version to 5 (SHA-1 name-based) and variant bits per RFC 4122
                    let mut bytes = *uuid.as_bytes();
                    bytes[6] = (bytes[6] & 0x0f) | 0x50; // Version 5
                    bytes[8] = (bytes[8] & 0x3f) | 0x80; // Variant 10
                    uuid = uuid::Uuid::from_bytes(bytes);

                    uuid.to_string()
                })
                .clone();
            sample.sequence_uuid = Some(seq_uuid);
        }
    }
}

/// Parse annotations from an Arrow IPC file into a sample map.
///
/// Reads an Arrow file containing annotation data and builds a HashMap that
/// groups annotations by sample name. Each entry contains an optional group
/// designation and a vector of annotations for that sample.
///
/// The function handles multiple annotation geometries per sample (box2d,
/// box3d, mask) and automatically generates object IDs when multiple
/// geometries exist for the same annotation to enable object tracking.
///
/// # Arguments
///
/// * `annotations` - Optional path to Arrow IPC file containing annotations
/// * `should_upload_annotations` - Whether to parse annotation data or just
///   sample names
///
/// # Returns
///
/// Vec of samples with image files, optional groups, and annotations. If
/// `should_upload_annotations` is false, annotation vectors will be empty.
#[cfg(feature = "polars")]
fn create_sequence_aware_batches(
    samples: Vec<edgefirst_client::Sample>,
    max_batch_size: usize,
) -> Vec<Vec<edgefirst_client::Sample>> {
    use std::collections::HashMap;

    // Group samples by (sequence_uuid, group) to ensure consistent metadata within
    // each batch. The server has bugs where it only reads certain metadata from
    // the first sample in each batch:
    // 1. Sequence metadata (sequence_uuid, sequence_name, sequence_description)
    // 2. Group assignment
    //
    // Therefore, all samples in a batch must belong to the same sequence AND the
    // same group.
    //
    // Non-sequence samples (sequence_uuid == None) are still grouped by group to
    // avoid the group assignment bug.

    let mut by_sequence_and_group: HashMap<
        (Option<String>, Option<String>),
        Vec<edgefirst_client::Sample>,
    > = HashMap::new();

    for sample in samples {
        let key = (sample.sequence_uuid.clone(), sample.group.clone());
        by_sequence_and_group.entry(key).or_default().push(sample);
    }

    let mut all_batches = Vec::new();

    // Process each sequence+group combination
    for (_key, mut seq_samples) in by_sequence_and_group {
        // Split large groups into batches respecting max_batch_size
        while !seq_samples.is_empty() {
            let batch_size = seq_samples.len().min(max_batch_size);
            let batch = seq_samples.drain(..batch_size).collect();
            all_batches.push(batch);
        }
    }

    all_batches
}

#[cfg(feature = "polars")]
/// Parses annotations from an Arrow file and matches them with image files.
///
/// Supports both nested and flattened directory structures:
/// - **Nested**: Images in sequence subdirectories
///   (sequence_name/sequence_name_frame.ext)
/// - **Flattened**: All images in root directory with sequence prefix
///   (sequence_name_frame.ext)
///
/// The function uses the Arrow file's `name` and `frame` columns as the
/// authoritative source for sequence information, regardless of how files are
/// organized on disk. The image_index built by walking the directory tree works
/// for both structures.
///
/// # Arguments
///
/// * `annotations` - Optional path to Arrow file containing annotations and
///   metadata
/// * `images_path` - Path to directory (or ZIP) containing image files
/// * `should_upload_annotations` - Whether to parse and include annotation
///   geometries
///
/// # Returns
///
/// Vector of Sample objects with matched images and parsed annotations
fn parse_annotations_from_arrow(
    annotations: &Option<PathBuf>,
    images_path: &Path,
    should_upload_annotations: bool,
) -> Result<Vec<edgefirst_client::Sample>, Error> {
    use polars::prelude::*;
    use std::{collections::HashMap, fs::File};

    // Helper struct to store sample metadata during parsing
    struct SampleMetadata {
        group: Option<String>,
        sequence_name: Option<String>,
        frame_number: Option<u32>,
        annotations: Vec<edgefirst_client::Annotation>,
    }

    // Map: sample_name -> metadata
    // sequence_name is Some(name) when frame is not-null, indicating this sample is
    // part of a sequence
    let mut samples_map: HashMap<String, SampleMetadata> = HashMap::new();

    if let Some(arrow_path) = annotations {
        let mut file = File::open(arrow_path)?;
        let df = IpcReader::new(&mut file)
            .finish()
            .map_err(|e| Error::InvalidParameters(format!("Failed to read Arrow file: {}", e)))?;

        // Process each row in the DataFrame
        for idx in 0..df.height() {
            // Extract required sample name (must exist)
            let name = df
                .column("name")
                .map_err(|e| Error::InvalidParameters(format!("Missing 'name' column: {}", e)))?
                .str()
                .map_err(|e| {
                    Error::InvalidParameters(format!("Invalid 'name' column type: {}", e))
                })?
                .get(idx)
                .ok_or_else(|| Error::InvalidParameters("Missing name value".to_owned()))?
                .to_string();

            // Strip extension from name if present (handles test data with full filenames)
            let base_name = name
                .rsplit_once('.')
                .and_then(|(base, ext)| {
                    // Only strip if it looks like an image extension
                    if matches!(ext, "jpg" | "jpeg" | "png" | "camera") {
                        Some(base)
                    } else {
                        None
                    }
                })
                .unwrap_or(&name);

            // Extract optional frame number (try both string and u32 types)
            let frame_str = df.column("frame").ok().and_then(|c| {
                // Try as string first (common format in Arrow files)
                c.str()
                    .ok()
                    .and_then(|s| s.get(idx))
                    .map(|s| s.to_string())
                    .or_else(|| {
                        // Try as u32 if string fails
                        c.u32().ok().and_then(|s| s.get(idx)).map(|n| n.to_string())
                    })
            });

            // Determine if this is a sequence sample based on frame column
            // Sequence: frame is not-null → sequence_name = name, image =
            // name/name_frame.ext Non-sequence: frame is null → no
            // sequence_name, image = whatever.ext
            let (sequence_name, frame_number) = match &frame_str {
                Some(f) if !f.is_empty() => {
                    let frame_num = f.parse::<u32>().map_err(|_| {
                        Error::InvalidParameters(format!(
                            "Invalid frame number '{}' for sequence '{}'",
                            f, base_name
                        ))
                    })?;
                    (Some(base_name.to_string()), Some(frame_num))
                }
                _ => (None, None),
            };

            // Construct full sample name: "{base_name}_{frame}" if frame exists
            let sample_name = match &frame_str {
                Some(f) if !f.is_empty() => format!("{}_{}", base_name, f),
                _ => base_name.to_string(),
            };

            // Extract optional group designation (train/val/test)
            // Handle both Categorical and String column types by casting to String
            let sample_group = df
                .column("group")
                .ok()
                .and_then(|c| c.cast(&DataType::String).ok())
                .and_then(|col| col.str().ok()?.get(idx).map(|s| s.to_string()));

            // Get or create entry for this sample, validating group consistency
            let entry = match samples_map.entry(sample_name.clone()) {
                std::collections::hash_map::Entry::Occupied(e) => {
                    // Sample exists - validate group is consistent
                    let existing = e.into_mut();
                    if existing.group != sample_group {
                        return Err(Error::InvalidParameters(format!(
                            "Inconsistent group for image '{}': row has group {:?} but previous row had {:?}. \
                            All rows for the same image must have identical group values.",
                            sample_name, sample_group, existing.group
                        )));
                    }
                    existing
                }
                std::collections::hash_map::Entry::Vacant(e) => e.insert(SampleMetadata {
                    group: sample_group,
                    sequence_name: sequence_name.clone(),
                    frame_number,
                    annotations: Vec::new(),
                }),
            };

            if should_upload_annotations {
                let mut has_annotation = false;
                let mut geometry_count = 0;
                let mut annotation = edgefirst_client::Annotation::new();

                // Set frame_number if available (parsed earlier from Arrow file)
                if let Some(ref frame) = frame_str
                    && let Ok(frame_num) = frame.parse::<u32>()
                {
                    annotation.set_frame_number(Some(frame_num));
                }

                // Extract label if present and non-empty
                // Handle both Categorical and String column types by casting to String
                let label = df
                    .column("label")
                    .ok()
                    .and_then(|c| c.cast(&DataType::String).ok())
                    .and_then(|col| col.str().ok()?.get(idx).map(|s| s.to_string()))
                    .filter(|s: &String| !s.is_empty());

                if let Some(lbl) = label {
                    annotation.set_label(Some(lbl));
                    has_annotation = true;
                }

                // Extract object_id if present and non-empty
                let object_id = df
                    .column("object_id")
                    .or_else(|_| df.column("object_reference"))
                    .ok()
                    .and_then(|c| c.str().ok())
                    .and_then(|s| s.get(idx))
                    .and_then(|s| {
                        if s.is_empty() {
                            None
                        } else {
                            Some(s.to_string())
                        }
                    });

                if let Some(ref obj_id) = object_id {
                    annotation.set_object_id(Some(obj_id.clone()));
                }

                // Try to extract each geometry type - samples can have multiple
                if let Some(box2d) = parse_box2d_from_dataframe(&df, idx)? {
                    annotation.set_box2d(Some(box2d));
                    has_annotation = true;
                    geometry_count += 1;
                }

                if let Some(box3d) = parse_box3d_from_dataframe(&df, idx)? {
                    annotation.set_box3d(Some(box3d));
                    has_annotation = true;
                    geometry_count += 1;
                }

                if let Some(mask) = parse_mask_from_dataframe(&df, idx)? {
                    annotation.set_mask(Some(mask));
                    has_annotation = true;
                    geometry_count += 1;
                }

                // Auto-generate object_id for multi-geometry annotations to enable
                // tracking
                if geometry_count > 1 && object_id.is_none() {
                    let generated_uuid = uuid::Uuid::new_v4().to_string();
                    annotation.set_object_id(Some(generated_uuid));
                }

                // Only add annotation if it has at least one geometry or label
                if has_annotation {
                    entry.annotations.push(annotation);
                }
            }
        }
    }

    if samples_map.is_empty() {
        return Ok(Vec::new());
    }

    let image_index = build_image_index(images_path)?;

    // Convert HashMap to Vec<Sample> by resolving image paths
    let mut samples = Vec::new();
    for (sample_name, metadata) in samples_map {
        let sample_group = metadata.group;
        let arrow_sequence_name = metadata.sequence_name;
        let arrow_frame_number = metadata.frame_number;
        let mut annotations = metadata.annotations;
        let image_path = find_image_path_for_sample(&image_index, &sample_name)?;

        // Get the actual image filename with extension from the resolved path
        let image_filename = image_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| Error::InvalidParameters("Invalid image path".to_string()))?
            .to_string();

        // Use sequence_name and frame_number from Arrow file (not from filesystem)
        // Requirement: sequence_name is based on the "name" column when "frame" is
        // not-null
        let sequence_name = arrow_sequence_name.clone();
        let frame_number = arrow_frame_number;

        // Update all annotations with sample metadata (name, sequence, group, frame)
        for annotation in &mut annotations {
            annotation.set_name(Some(image_filename.clone()));
            annotation.set_sequence_name(sequence_name.clone());
            annotation.set_group(sample_group.clone());
            if let Some(frame) = frame_number {
                annotation.set_frame_number(Some(frame));
            }
        }

        let image_file = edgefirst_client::SampleFile::with_filename(
            "image".to_string(),
            image_path.to_str().unwrap().to_string(),
        );

        let sample = edgefirst_client::Sample {
            image_name: Some(image_filename),
            group: sample_group,
            sequence_name,
            frame_number,
            files: vec![image_file],
            annotations,
            ..Default::default()
        };

        samples.push(sample);
    }

    Ok(samples)
}

#[cfg(feature = "polars")]
/// Builds an index mapping filenames to their full paths by walking directory
/// tree.
///
/// This index works for both nested and flattened directory structures:
/// - **Nested**: Walks subdirectories to find files like
///   sequence_A/sequence_A_001.jpeg
/// - **Flattened**: Finds files directly in root like sequence_A_001.jpeg
///
/// The index maps multiple filename variations to the same file:
/// - Full filename: "sequence_A_001.camera.jpeg"
/// - Without extension: "sequence_A_001.camera"
/// - Without .camera suffix: "sequence_A_001"
///
/// This flexible matching ensures compatibility with Arrow files that may use
/// different naming conventions.
fn build_image_index(
    images_path: &Path,
) -> Result<std::collections::HashMap<String, Vec<PathBuf>>, Error> {
    if !images_path.is_dir() {
        return Err(Error::InvalidParameters(
            "ZIP file support not yet implemented".to_owned(),
        ));
    }

    let mut index: std::collections::HashMap<String, Vec<PathBuf>> =
        std::collections::HashMap::new();

    // Recursively walk directory tree - works for both nested and flattened
    // structures
    for entry in WalkDir::new(images_path) {
        let entry = entry.map_err(|e| {
            Error::InvalidParameters(format!("Failed to read images directory: {}", e))
        })?;

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path().to_path_buf();

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if !is_valid_image_extension(ext) {
                continue;
            }
        } else {
            continue;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            Error::InvalidParameters(format!("Invalid filename: {}", path.display()))
        })?;

        for key in generate_image_lookup_keys(file_name) {
            index.entry(key).or_default().push(path.clone());
        }
    }

    Ok(index)
}

#[cfg(feature = "polars")]
fn generate_image_lookup_keys(file_name: &str) -> Vec<String> {
    let mut keys = vec![file_name.to_string()];

    if let Some((stem, _ext)) = file_name.rsplit_once('.') {
        keys.push(stem.to_string());
        if let Some(stripped) = stem.strip_suffix(".camera")
            && !stripped.is_empty()
        {
            keys.push(stripped.to_string());
        }
    }

    keys.sort();
    keys.dedup();
    keys
}

#[cfg(feature = "polars")]
fn find_image_path_for_sample(
    image_index: &std::collections::HashMap<String, Vec<PathBuf>>,
    image_name: &str,
) -> Result<PathBuf, Error> {
    // Finds image file for a sample, supporting both nested and flattened directory
    // structures.
    //
    // For nested structure (sequences in subdirectories):
    //   - Images are in sequence_name/sequence_name_frame.ext
    //   - Index contains filenames like "sequence_A_001.camera.jpeg"
    //
    // For flattened structure (all files in one directory):
    //   - Images are in root with prefix: sequence_name_frame.ext or
    //     sequence_name_frame_original.ext
    //   - Index contains same filenames
    //
    // The image_index is built by walking the entire directory tree, so it works
    // for both structures.

    // Extension priority order: .camera.* takes precedence over plain extensions
    const EXTENSIONS: &[&str] = &[
        ".camera.jpg",
        ".camera.jpeg",
        ".camera.png",
        ".jpg",
        ".jpeg",
        ".png",
    ];

    // Try each extension in priority order
    for ext in EXTENSIONS {
        let candidate = format!("{}{}", image_name, ext);
        if let Some(paths) = image_index.get(&candidate) {
            match paths.len() {
                0 => continue,
                1 => return Ok(paths[0].clone()),
                _ => {
                    return Err(Error::InvalidParameters(format!(
                        "Multiple image matches found for '{}': {:?}",
                        candidate, paths
                    )));
                }
            }
        }
    }

    Err(Error::InvalidParameters(format!(
        "Image file not found for sample: {}",
        image_name
    )))
}

#[cfg(feature = "polars")]
fn extract_sequence_name(images_root: &Path, image_path: &Path) -> Option<String> {
    // First try: Check if image is in a subdirectory (nested structure)
    if let Ok(relative) = image_path.strip_prefix(images_root)
        && let Some(parent) = relative.parent()
    {
        let mut components = parent.components();
        if let Some(std::path::Component::Normal(os_str)) = components.next() {
            let sequence = os_str.to_string_lossy().into_owned();
            if !sequence.is_empty() {
                return Some(sequence);
            }
        }
    }

    // Second try: Check if filename contains sequence prefix (flattened structure)
    // Format: {sequence_name}_{frame}_{rest}.ext or {sequence_name}_{frame}.ext
    if let Some(filename) = image_path.file_stem()
        && let Some(name_str) = filename.to_str()
    {
        // Look for pattern: something_digits (sequence_frame)
        // Split on underscores and check if we have at least 2 parts with second being
        // numeric
        let parts: Vec<&str> = name_str.split('_').collect();
        if parts.len() >= 2 {
            // Check if second part is a number (frame)
            if parts[1].parse::<u32>().is_ok() {
                // First part is the sequence name
                return Some(parts[0].to_string());
            }
        }
    }

    None
}

#[cfg(feature = "polars")]
fn is_valid_image_extension(ext: &str) -> bool {
    let ext_lower = ext.to_lowercase();
    matches!(
        ext_lower.as_str(),
        "jpg" | "jpeg" | "png" | "bmp" | "tiff" | "tif" | "webp"
    )
}

#[cfg(feature = "polars")]
fn build_samples_from_directory(
    images_path: &PathBuf,
) -> Result<Vec<edgefirst_client::Sample>, Error> {
    if !images_path.is_dir() {
        return Err(Error::InvalidParameters(
            "ZIP file support not yet implemented".to_owned(),
        ));
    }

    let mut samples = Vec::new();

    for entry in WalkDir::new(images_path) {
        let entry = entry.map_err(|e| {
            Error::InvalidParameters(format!("Failed to read images directory: {}", e))
        })?;

        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if !is_valid_image_extension(ext) {
                continue;
            }
        } else {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                Error::InvalidParameters(format!("Invalid filename: {}", path.display()))
            })?
            .to_string();

        let image_file = edgefirst_client::SampleFile::with_filename(
            "image".to_string(),
            path.to_str().unwrap().to_string(),
        );

        let sample = edgefirst_client::Sample {
            image_name: Some(file_name),
            sequence_name: extract_sequence_name(images_path.as_path(), path),
            files: vec![image_file],
            annotations: Vec::new(),
            ..Default::default()
        };

        samples.push(sample);
    }

    Ok(samples)
}

#[cfg(feature = "polars")]
async fn handle_upload_dataset(
    client: &Client,
    dataset_id: String,
    annotation_set_id: Option<String>,
    annotations: Option<PathBuf>,
    images: Option<PathBuf>,
) -> Result<(), Error> {
    // Validate inputs
    if annotations.is_none() && images.is_none() {
        return Err(Error::InvalidParameters(
            "Must provide at least one of --annotations or --images".to_owned(),
        ));
    }

    // Warning: annotations exist but no annotation_set_id
    if annotations.is_some() && annotation_set_id.is_none() {
        eprintln!("⚠️  Warning: Arrow file provided but no --annotation-set-id specified.");
        eprintln!("   Annotations in the Arrow file will NOT be uploaded.");
        eprintln!("   Only images will be imported.");
    }

    // Warning: annotation_set_id provided but no annotations
    if annotation_set_id.is_some() && annotations.is_none() {
        eprintln!("⚠️  Warning: --annotation-set-id provided but no --annotations file.");
        eprintln!("   No annotations will be read or uploaded.");
        eprintln!("   Only images will be imported.");
    }

    // Determine images path
    let images_path = determine_images_path(&annotations, &images)?;

    // Parse annotations from Arrow if provided, or build samples from directory
    let should_upload_annotations = annotations.is_some() && annotation_set_id.is_some();
    let mut samples = if annotations.is_some() {
        parse_annotations_from_arrow(&annotations, &images_path, should_upload_annotations)?
    } else {
        build_samples_from_directory(&images_path)?
    };

    if samples.is_empty() {
        return Err(Error::InvalidParameters(
            "No samples to upload. Check that images exist.".to_owned(),
        ));
    }

    // Generate UUIDs for upload (sample.uuid and sequence_uuid for sequences)
    // These are required by Studio but not persisted to Arrow/JSON files
    generate_upload_uuids(&mut samples, &dataset_id);

    println!(
        "Uploading {} samples to dataset {}...",
        samples.len(),
        dataset_id
    );

    let bar = indicatif::ProgressBar::new(samples.len() as u64);
    bar.set_style(
        indicatif::ProgressStyle::with_template(
            "[{elapsed_precise} ETA: {eta}] Uploading samples: {wide_bar:.yellow} {human_pos}/{human_len}"
        )
        .unwrap()
        .progress_chars("█▇▆▅▄▃▂▁  "),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<edgefirst_client::Progress>(1);
    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            if progress.total > 0 {
                bar.set_length(progress.total as u64);
                bar.set_position(progress.current as u64);
            }
        }
        bar.finish_with_message("Upload complete");
    });

    const BATCH_SIZE: usize = 500;
    let mut all_results = Vec::new();

    let dataset_id_parsed: edgefirst_client::DatasetID = dataset_id.try_into()?;
    let annotation_set_id_parsed = if should_upload_annotations {
        Some(annotation_set_id.unwrap().try_into()?)
    } else {
        None
    };

    // Group samples by sequence to work around server bug where all samples in a
    // batch are assigned to the sequence of the first sample only.
    // Non-sequence samples (sequence_uuid == None) are grouped together.
    let batches = create_sequence_aware_batches(samples, BATCH_SIZE);

    println!(
        "Created {} batches (grouped by sequence+group to avoid server bugs)",
        batches.len()
    );

    for (batch_num, batch) in batches.iter().enumerate() {
        let info = if let Some(first) = batch.first() {
            let seq_str = first.sequence_name.as_deref().unwrap_or("no sequence");
            let grp_str = first.group.as_deref().unwrap_or("no group");
            format!(" [seq: {}, group: {}]", seq_str, grp_str)
        } else {
            String::new()
        };

        println!(
            "Uploading batch {}/{} ({} samples){}...",
            batch_num + 1,
            batches.len(),
            batch.len(),
            info
        );

        let results = client
            .populate_samples(
                dataset_id_parsed,
                annotation_set_id_parsed,
                batch.clone(),
                Some(tx.clone()),
            )
            .await?;

        all_results.extend(results);
    }

    drop(tx);

    println!("Successfully uploaded {} samples", all_results.len());
    for result in all_results.iter().take(10) {
        println!("  Sample UUID: {}", result.uuid);
    }
    if all_results.len() > 10 {
        println!("  ... and {} more", all_results.len() - 10);
    }

    Ok(())
}

#[cfg(not(feature = "polars"))]
async fn handle_upload_dataset(
    _client: &Client,
    _dataset_id: String,
    _annotation_set_id: Option<String>,
    _annotations: Option<PathBuf>,
    _images: Option<PathBuf>,
) -> Result<(), Error> {
    Err(Error::FeatureNotEnabled("polars".to_owned()))
}

async fn handle_experiments(
    client: &Client,
    project_id: Option<String>,
    name: Option<String>,
) -> Result<(), Error> {
    let projects = if let Some(project_id) = project_id {
        vec![client.project(project_id.try_into()?).await?]
    } else {
        client.projects(None).await?
    };

    for project in projects {
        println!("{}", project.name());

        let experiments = client.experiments(project.id(), name.as_deref()).await?;
        for experiment in experiments {
            println!(
                "    [{}] {}: {}",
                experiment.id(),
                experiment.name(),
                experiment.description()
            );
        }
    }
    Ok(())
}

async fn handle_experiment(client: &Client, experiment_id: String) -> Result<(), Error> {
    let experiment = client.experiment(experiment_id.try_into()?).await?;
    println!(
        "[{}] {}: {}",
        experiment.id(),
        experiment.name(),
        experiment.description()
    );
    Ok(())
}

async fn print_training_session_with_artifacts(
    client: &Client,
    session: &TrainingSession,
) -> Result<(), Error> {
    println!(
        "{} ({}) {}",
        session.id(),
        session.task().status(),
        session.name()
    );

    for artifact in client.artifacts(session.id()).await? {
        println!("    - {}", artifact.name());
    }
    Ok(())
}

async fn handle_training_sessions(
    client: &Client,
    experiment_id: Option<String>,
    name: Option<String>,
) -> Result<(), Error> {
    if let Some(experiment_id) = experiment_id {
        let sessions = client
            .training_sessions(experiment_id.try_into()?, name.as_deref())
            .await?;
        for session in sessions {
            print_training_session_with_artifacts(client, &session).await?;
        }
    } else {
        let projects = client.projects(None).await?;
        for project in projects {
            let trainers = client.experiments(project.id(), None).await?;
            for trainer in trainers {
                let sessions = client
                    .training_sessions(trainer.id(), name.as_deref())
                    .await?;
                for session in sessions {
                    print_training_session_with_artifacts(client, &session).await?;
                }
            }
        }
    }
    Ok(())
}

async fn handle_training_session(
    client: &Client,
    training_session_id: String,
    model: bool,
    dataset: bool,
    artifacts: bool,
) -> Result<(), Error> {
    let session = client
        .training_session(training_session_id.clone().try_into()?)
        .await?;
    println!(
        "{} ({}) {}",
        session.id(),
        session.task().status(),
        session.name()
    );

    if model {
        println!("Model Parameters: {:?}", session.model_params());
    }

    if dataset {
        println!("Dataset Parameters: {:?}", session.dataset_params());
    }

    if artifacts {
        println!("Artifacts:");
        for artifact in client.artifacts(training_session_id.try_into()?).await? {
            println!("    - {}", artifact.name());
        }
    }
    Ok(())
}

async fn handle_download_artifact(
    client: &Client,
    session_id: String,
    name: String,
    output: Option<PathBuf>,
) -> Result<(), Error> {
    use indicatif::{ProgressBar, ProgressStyle};
    use tokio::sync::mpsc;

    let bar = ProgressBar::new(0);
    bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar:.yellow}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").unwrap().progress_chars("█▇▆▅▄▃▂▁  "));

    let (tx, mut rx) = mpsc::channel::<Progress>(1);

    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            if progress.total > 0 {
                bar.set_length(progress.total as u64);
                bar.set_position(progress.current as u64);
            }
        }
    });

    client
        .download_artifact(session_id.try_into()?, &name, output, Some(tx))
        .await?;
    Ok(())
}

async fn handle_upload_artifact(
    client: &Client,
    session_id: String,
    path: PathBuf,
    name: Option<String>,
) -> Result<(), Error> {
    let name = name.unwrap_or_else(|| {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_owned())
            .unwrap()
    });
    let session = client.training_session(session_id.try_into()?).await?;
    session.upload_artifact(client, &name, path).await?;
    Ok(())
}

async fn handle_tasks(
    client: &Client,
    stages: bool,
    name: Option<String>,
    workflow: Option<String>,
    status: Option<String>,
    manager: Option<String>,
) -> Result<(), Error> {
    let tasks = client
        .tasks(
            name.as_deref(),
            workflow.as_deref(),
            status.as_deref(),
            manager.as_deref(),
        )
        .await?;
    for task in tasks {
        println!("{} => {}", task, task.status());

        if stages {
            let info = client.task_info(task.id()).await?;
            println!("    {:?}", info.stages());
        }
    }
    Ok(())
}

async fn handle_task(client: &Client, task_id: String, monitor: bool) -> Result<(), Error> {
    let task_id = task_id.try_into()?;
    if monitor {
        monitor_task(client, task_id).await
    } else {
        let info = client.task_info(task_id).await?;

        // Display formatted task information
        println!("[{}] {}", info.id(), info.workflow());
        println!("  Description: {}", info.description());
        println!(
            "  Status:      {}",
            info.status()
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        );
        if let Some(project_id) = info.project_id() {
            println!("  Project:     {}", project_id);
        }
        println!(
            "  Created:     {}",
            info.created().format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!(
            "  Completed:   {}",
            info.completed().format("%Y-%m-%d %H:%M:%S UTC")
        );

        // Display stages if present
        let stages = info.stages();
        if !stages.is_empty() {
            println!("  Stages:");
            for (name, stage) in &stages {
                let status = stage.status().clone().unwrap_or_default();
                let pct = stage.percentage();
                let message = stage.message().clone().unwrap_or_default();
                if message.is_empty() {
                    println!("    {:<20} {:>3}% ({})", name, pct, status);
                } else {
                    println!("    {:<20} {:>3}% ({}) - {}", name, pct, status, message);
                }
            }
        }

        Ok(())
    }
}

/// Monitor a task's progress with a progress bar until completion.
/// Polls the task status every 2 seconds and displays stage progress.
async fn monitor_task(client: &Client, task_id: TaskID) -> Result<(), Error> {
    use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
    use std::{collections::HashMap, time::Duration};

    let multi = MultiProgress::new();
    let mut stage_bars: HashMap<String, ProgressBar> = HashMap::new();

    let spinner_style = ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

    let progress_style =
        ProgressStyle::with_template("  {spinner:.cyan} {msg:<30} [{bar:40.cyan/blue}] {pos:>3}%")
            .unwrap()
            .progress_chars("█▇▆▅▄▃▂▁  ");

    // Main task progress bar
    let main_bar = multi.add(ProgressBar::new_spinner());
    main_bar.set_style(spinner_style.clone());
    main_bar.enable_steady_tick(Duration::from_millis(100));

    loop {
        let info = client.task_info(task_id).await?;
        let status = info
            .status()
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let stages = info.stages();

        // Update main bar with task status
        main_bar.set_message(format!("Task {} - Status: {}", task_id, status));

        // Update or create stage progress bars
        for (stage_name, stage) in &stages {
            let bar = stage_bars.entry(stage_name.clone()).or_insert_with(|| {
                let bar = multi.add(ProgressBar::new(100));
                bar.set_style(progress_style.clone());
                bar
            });

            let stage_status = stage.status().clone().unwrap_or_default();
            let percentage = stage.percentage() as u64;

            bar.set_position(percentage);
            bar.set_message(format!("{}: {}", stage_name, stage_status));

            // Mark completed stages
            if percentage >= 100 || stage_status == "completed" || stage_status == "done" {
                bar.finish();
            }
        }

        // Check if task is complete
        let is_complete = matches!(
            status.to_lowercase().as_str(),
            "complete" | "completed" | "done" | "failed" | "error" | "cancelled" | "canceled"
        );

        if is_complete {
            main_bar.finish_with_message(format!("Task {} - Status: {} ✓", task_id, status));

            // Finish all stage bars
            for bar in stage_bars.values() {
                bar.finish();
            }

            // Print final summary
            println!("\nTask completed with status: {}", status);
            if !stages.is_empty() {
                println!("Stages:");
                for (name, stage) in &stages {
                    let stage_status = stage.status().clone().unwrap_or_default();
                    let message = stage.message().clone().unwrap_or_default();
                    if message.is_empty() {
                        println!("  {} - {}% ({})", name, stage.percentage(), stage_status);
                    } else {
                        println!(
                            "  {} - {}% ({}) - {}",
                            name,
                            stage.percentage(),
                            stage_status,
                            message
                        );
                    }
                }
            }

            if status.to_lowercase() == "failed" || status.to_lowercase() == "error" {
                // Find and print any error messages from stages
                for (name, stage) in &stages {
                    if let Some(msg) = stage.message()
                        && !msg.is_empty()
                    {
                        eprintln!("Error in stage '{}': {}", name, msg);
                    }
                }
                eprintln!("Task {} failed with status: {}", task_id, status);
                std::process::exit(1);
            }
            break;
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}

async fn handle_validation_sessions(client: &Client, project_id: String) -> Result<(), Error> {
    let sessions = client.validation_sessions(project_id.try_into()?).await?;
    for session in sessions {
        println!(
            "[{}] {}: {}",
            session.id(),
            session.name(),
            session.description()
        );
    }
    Ok(())
}

async fn handle_validation_session(client: &Client, session_id: String) -> Result<(), Error> {
    let session = client.validation_session(session_id.try_into()?).await?;
    println!(
        "[{}] {}: {}",
        session.id(),
        session.name(),
        session.description()
    );
    Ok(())
}

async fn handle_snapshots(client: &Client) -> Result<(), Error> {
    let snapshots = client.snapshots(None).await?;
    for snapshot in snapshots {
        println!(
            "[{}] {}: {} ({})",
            snapshot.id(),
            snapshot.description(),
            snapshot.path(),
            snapshot.status()
        );
    }
    Ok(())
}

async fn handle_snapshot(client: &Client, snapshot_id: String) -> Result<(), Error> {
    let snapshot_id = SnapshotID::try_from(snapshot_id.as_str())?;
    let snapshot = client.snapshot(snapshot_id).await?;
    println!(
        "[{}] {}\nPath: {}\nStatus: {}\nCreated: {}",
        snapshot.id(),
        snapshot.description(),
        snapshot.path(),
        snapshot.status(),
        snapshot.created()
    );
    Ok(())
}

/// Parameters for the unified create-snapshot command.
struct CreateSnapshotParams {
    source: String,
    annotation_set: Option<String>,
    description: Option<String>,
    from_path: bool,
    from_dataset: bool,
    monitor: bool,
}

/// Source type for snapshot creation, determined from user input.
enum SnapshotSource {
    /// Server-side dataset (optionally with specific annotation set)
    Dataset {
        dataset_id: DatasetID,
        annotation_set_id: Option<AnnotationSetID>,
    },
    /// Annotation set (parent dataset will be looked up)
    AnnotationSet(AnnotationSetID),
    /// Local file path (MCAP, Arrow, folder, or ZIP)
    LocalPath(PathBuf),
}

/// Parse the source argument to determine snapshot source type.
fn parse_snapshot_source(
    source: &str,
    annotation_set: Option<&str>,
    from_path: bool,
    from_dataset: bool,
) -> Result<SnapshotSource, Error> {
    // Explicit flags take precedence
    if from_path {
        return Ok(SnapshotSource::LocalPath(PathBuf::from(source)));
    }

    if from_dataset {
        let dataset_id = DatasetID::try_from(source)?;
        let annotation_set_id = annotation_set.map(AnnotationSetID::try_from).transpose()?;
        return Ok(SnapshotSource::Dataset {
            dataset_id,
            annotation_set_id,
        });
    }

    // Smart detection: check for ID prefixes
    if source.starts_with("ds-") {
        let dataset_id = DatasetID::try_from(source)?;
        let annotation_set_id = annotation_set.map(AnnotationSetID::try_from).transpose()?;
        return Ok(SnapshotSource::Dataset {
            dataset_id,
            annotation_set_id,
        });
    }

    if source.starts_with("as-") {
        let annotation_set_id = AnnotationSetID::try_from(source)?;
        return Ok(SnapshotSource::AnnotationSet(annotation_set_id));
    }

    // Check if it looks like a path (contains path separators or exists on disk)
    let path = PathBuf::from(source);
    if path.exists() || source.contains('/') || source.contains('\\') || source.contains('.') {
        return Ok(SnapshotSource::LocalPath(path));
    }

    // Default to treating unknown input as an error
    Err(Error::InvalidParameters(format!(
        "Could not determine source type for '{}'. Use --from-path or --from-dataset to be explicit.",
        source
    )))
}

/// Generate a description for the snapshot based on source.
fn generate_snapshot_description(source: &SnapshotSource, provided: Option<&str>) -> String {
    use chrono::Local;

    if let Some(desc) = provided {
        return desc.to_string();
    }

    let date = Local::now().format("%Y-%m-%d %H:%M");

    match source {
        SnapshotSource::Dataset { dataset_id, .. } => {
            format!("{} snapshot {}", dataset_id, date)
        }
        SnapshotSource::AnnotationSet(as_id) => {
            format!("{} snapshot {}", as_id, date)
        }
        SnapshotSource::LocalPath(path) => {
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("local");
            format!("{} {}", name, date)
        }
    }
}

async fn handle_create_snapshot(
    client: &Client,
    params: CreateSnapshotParams,
) -> Result<(), Error> {
    use indicatif::{ProgressBar, ProgressStyle};
    use tokio::sync::mpsc;

    // Parse the source to determine what type of snapshot creation
    let source = parse_snapshot_source(
        &params.source,
        params.annotation_set.as_deref(),
        params.from_path,
        params.from_dataset,
    )?;

    // Generate description
    let description = generate_snapshot_description(&source, params.description.as_deref());

    match source {
        SnapshotSource::Dataset {
            dataset_id,
            annotation_set_id,
        } => {
            // Server-side creation from dataset
            let result = client
                .create_snapshot_from_dataset(dataset_id, &description, annotation_set_id)
                .await?;

            println!("Snapshot creation initiated: [{}]", result.id);

            if let Some(task_id) = result.task_id {
                println!("Task: [{}]", task_id);

                if params.monitor {
                    monitor_task(client, task_id).await?;
                }
            } else if params.monitor {
                println!("No task ID returned - operation may be synchronous");
            }
        }
        SnapshotSource::AnnotationSet(as_id) => {
            // Look up parent dataset from annotation set
            let annotation_set = client.annotation_set(as_id).await?;
            let dataset_id = annotation_set.dataset_id();

            // Pass annotation_set_id explicitly for this annotation set
            let result = client
                .create_snapshot_from_dataset(dataset_id, &description, Some(as_id))
                .await?;

            println!(
                "Snapshot creation initiated from annotation set {}: [{}]",
                as_id, result.id
            );

            if let Some(task_id) = result.task_id {
                println!("Task: [{}]", task_id);

                if params.monitor {
                    monitor_task(client, task_id).await?;
                }
            } else if params.monitor {
                println!("No task ID returned - operation may be synchronous");
            }
        }
        SnapshotSource::LocalPath(path) => {
            // For directories, validate the structure before upload
            if path.is_dir() {
                use edgefirst_client::format::{ValidationIssue, validate_dataset_structure};

                let issues = validate_dataset_structure(&path)?;
                if !issues.is_empty() {
                    // Separate errors from warnings
                    let errors: Vec<_> = issues
                        .iter()
                        .filter(|i| {
                            matches!(
                                i,
                                ValidationIssue::MissingArrowFile { .. }
                                    | ValidationIssue::MissingSensorContainer { .. }
                            )
                        })
                        .collect();

                    let warnings: Vec<_> = issues
                        .iter()
                        .filter(|i| {
                            !matches!(
                                i,
                                ValidationIssue::MissingArrowFile { .. }
                                    | ValidationIssue::MissingSensorContainer { .. }
                            )
                        })
                        .collect();

                    // Print warnings but continue
                    for warning in &warnings {
                        eprintln!("Warning: {}", warning);
                    }

                    // Abort on errors
                    if !errors.is_empty() {
                        for error in &errors {
                            eprintln!("Error: {}", error);
                        }
                        return Err(Error::InvalidParameters(format!(
                            "Invalid snapshot structure: {} error(s) found. \
                            Use 'edgefirst generate-arrow' to create an Arrow file from images.",
                            errors.len()
                        )));
                    }
                }
            }

            // Local file upload with progress bar
            let (tx, mut rx) = mpsc::channel(1);

            let pb = ProgressBar::new(100);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})",
                    )
                    .unwrap()
                    .progress_chars("=>-"),
            );

            tokio::spawn(async move {
                while let Some(Progress { current, total }) = rx.recv().await {
                    pb.set_length(total as u64);
                    pb.set_position(current as u64);
                }
                pb.finish_with_message("Upload complete");
            });

            let path_str = path.to_str().ok_or_else(|| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Path contains invalid UTF-8",
                ))
            })?;

            let snapshot = client.create_snapshot(path_str, Some(tx)).await?;
            println!(
                "Snapshot created: [{}] {}",
                snapshot.id(),
                snapshot.description()
            );
        }
    }

    Ok(())
}

async fn handle_download_snapshot(
    client: &Client,
    snapshot_id: String,
    output: PathBuf,
) -> Result<(), Error> {
    use indicatif::{ProgressBar, ProgressStyle};
    use tokio::sync::mpsc;

    let (tx, mut rx) = mpsc::channel(1);

    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("=>-"),
    );

    tokio::spawn(async move {
        while let Some(Progress { current, total }) = rx.recv().await {
            pb.set_length(total as u64);
            pb.set_position(current as u64);
        }
        pb.finish_with_message("Download complete");
    });

    let snapshot_id = SnapshotID::try_from(snapshot_id.as_str())?;
    client
        .download_snapshot(snapshot_id, output, Some(tx))
        .await?;
    println!("Snapshot downloaded successfully");
    Ok(())
}

struct RestoreSnapshotParams {
    project_id: String,
    snapshot_id: String,
    topics: Vec<String>,
    autolabel: Vec<String>,
    autodepth: bool,
    dataset_name: Option<String>,
    dataset_description: Option<String>,
    monitor: bool,
}

async fn handle_restore_snapshot(
    client: &Client,
    params: RestoreSnapshotParams,
) -> Result<(), Error> {
    let snapshot_id = SnapshotID::try_from(params.snapshot_id.as_str())?;
    let result = client
        .restore_snapshot(
            params.project_id.try_into()?,
            snapshot_id,
            &params.topics,
            &params.autolabel,
            params.autodepth,
            params.dataset_name.as_deref(),
            params.dataset_description.as_deref(),
        )
        .await?;
    println!(
        "Snapshot restore initiated for dataset: [{}]",
        result.dataset_id
    );

    // Always print task ID if available (enables async workflow)
    if let Some(task_id) = result.task_id {
        println!("Task: [{}]", task_id);

        // If monitoring is enabled, wait for completion
        if params.monitor {
            monitor_task(client, task_id).await?;
        }
    } else if params.monitor {
        println!("No task ID returned - restore may be synchronous or already complete");
    }

    Ok(())
}

async fn handle_delete_snapshot(client: &Client, snapshot_id: String) -> Result<(), Error> {
    let snapshot_id = SnapshotID::try_from(snapshot_id.as_str())?;
    client.delete_snapshot(snapshot_id).await?;
    println!("Snapshot deleted successfully");
    Ok(())
}

fn handle_generate_arrow(
    folder: PathBuf,
    output: PathBuf,
    detect_sequences: bool,
) -> Result<(), Error> {
    use edgefirst_client::format::generate_arrow_from_folder;

    if !folder.exists() {
        return Err(Error::InvalidParameters(format!(
            "Folder does not exist: {:?}",
            folder
        )));
    }

    if !folder.is_dir() {
        return Err(Error::InvalidParameters(format!(
            "Path is not a directory: {:?}",
            folder
        )));
    }

    println!("Scanning folder: {:?}", folder);

    let count = generate_arrow_from_folder(&folder, &output, detect_sequences)?;

    println!("Generated Arrow file with {} samples: {:?}", count, output);

    if detect_sequences {
        println!("Sequence detection: enabled");
    }

    Ok(())
}

fn handle_validate_snapshot(path: PathBuf, verbose: bool) -> Result<(), Error> {
    use edgefirst_client::format::{ValidationIssue, validate_dataset_structure};

    if !path.exists() {
        return Err(Error::InvalidParameters(format!(
            "Path does not exist: {:?}",
            path
        )));
    }

    if !path.is_dir() {
        return Err(Error::InvalidParameters(format!(
            "Path is not a directory: {:?}",
            path
        )));
    }

    println!("Validating snapshot structure: {:?}", path);

    let issues = validate_dataset_structure(&path)?;

    if issues.is_empty() {
        println!("✓ Snapshot structure is valid");
        return Ok(());
    }

    // Categorize issues
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| {
            matches!(
                i,
                ValidationIssue::MissingArrowFile { .. }
                    | ValidationIssue::MissingSensorContainer { .. }
            )
        })
        .collect();

    let missing_files: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i, ValidationIssue::MissingFile { .. }))
        .collect();

    let unreferenced: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i, ValidationIssue::UnreferencedFile { .. }))
        .collect();

    // Print summary
    if !errors.is_empty() {
        println!("\n✗ {} critical error(s):", errors.len());
        for error in &errors {
            println!("  - {}", error);
        }
    }

    if !missing_files.is_empty() {
        println!("\n⚠ {} missing file(s):", missing_files.len());
        if verbose {
            for issue in &missing_files {
                println!("  - {}", issue);
            }
        } else {
            for issue in missing_files.iter().take(5) {
                println!("  - {}", issue);
            }
            if missing_files.len() > 5 {
                println!(
                    "  ... and {} more (use -v to see all)",
                    missing_files.len() - 5
                );
            }
        }
    }

    if !unreferenced.is_empty() {
        println!("\n○ {} unreferenced file(s):", unreferenced.len());
        if verbose {
            for issue in &unreferenced {
                println!("  - {}", issue);
            }
        } else {
            for issue in unreferenced.iter().take(5) {
                println!("  - {}", issue);
            }
            if unreferenced.len() > 5 {
                println!(
                    "  ... and {} more (use -v to see all)",
                    unreferenced.len() - 5
                );
            }
        }
    }

    // Return error if there are critical issues
    if !errors.is_empty() {
        return Err(Error::InvalidParameters(format!(
            "Snapshot validation failed with {} critical error(s)",
            errors.len()
        )));
    }

    // Warn but succeed if only minor issues
    if !missing_files.is_empty() {
        println!(
            "\nWarning: {} file(s) referenced in Arrow but not found in container",
            missing_files.len()
        );
    }

    Ok(())
}

/// Handle COCO to Arrow conversion.
async fn handle_coco_to_arrow(
    coco_path: PathBuf,
    output: PathBuf,
    masks: bool,
    group: Option<String>,
) -> Result<(), Error> {
    use edgefirst_client::coco::{coco_to_arrow, CocoToArrowOptions};
    use indicatif::{ProgressBar, ProgressStyle};

    println!("Converting COCO to Arrow format...");
    println!("  Input:  {:?}", coco_path);
    println!("  Output: {:?}", output);

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Progress>(100);

    let options = CocoToArrowOptions {
        include_masks: masks,
        group,
        ..Default::default()
    };

    let coco_path_clone = coco_path.clone();
    let output_clone = output.clone();
    let task = tokio::spawn(async move {
        coco_to_arrow(&coco_path_clone, &output_clone, &options, Some(tx)).await
    });

    while let Some(progress) = rx.recv().await {
        pb.set_length(progress.total as u64);
        pb.set_position(progress.current as u64);
    }

    let count = task.await??;
    pb.finish_with_message("done");

    println!("\n✓ Converted {} annotations to Arrow format", count);

    Ok(())
}

/// Handle Arrow to COCO conversion.
async fn handle_arrow_to_coco(
    arrow_path: PathBuf,
    output: PathBuf,
    masks: bool,
    groups: Vec<String>,
    pretty: bool,
) -> Result<(), Error> {
    use chrono::Datelike;
    use edgefirst_client::coco::{arrow_to_coco, ArrowToCocoOptions, CocoInfo};
    use indicatif::{ProgressBar, ProgressStyle};

    println!("Converting Arrow to COCO format...");
    println!("  Input:  {:?}", arrow_path);
    println!("  Output: {:?}", output);

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Progress>(100);

    let options = ArrowToCocoOptions {
        include_masks: masks,
        groups,
        info: Some(CocoInfo {
            description: Some("Converted from EdgeFirst format".to_string()),
            version: Some("1.0".to_string()),
            year: Some(chrono::Utc::now().year() as u32),
            ..Default::default()
        }),
    };

    // Note: pretty option is not yet exposed in ArrowToCocoOptions
    // We would need to add it to the options struct
    let _ = pretty;

    let arrow_path_clone = arrow_path.clone();
    let output_clone = output.clone();
    let task = tokio::spawn(async move {
        arrow_to_coco(&arrow_path_clone, &output_clone, &options, Some(tx)).await
    });

    while let Some(progress) = rx.recv().await {
        pb.set_length(progress.total as u64);
        pb.set_position(progress.current as u64);
    }

    let count = task.await??;
    pb.finish_with_message("done");

    println!("\n✓ Converted {} annotations to COCO format", count);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    // Handle version command early - no authentication needed
    if args.cmd == Command::Version {
        let client = Client::new()?.with_token_path(None)?;
        let client = match args.server {
            Some(server) => client.with_server(&server)?,
            None => client,
        };
        return handle_version(&client).await;
    }

    // Handle sleep command - no authentication needed
    if let Command::Sleep { seconds } = args.cmd {
        return handle_sleep(seconds).await;
    }

    // Handle login command specially - ignore existing token, use --server or
    // default saas
    if args.cmd == Command::Login {
        let client = Client::new()?.with_token_path(None)?;
        // For login, use --server if provided, otherwise default to saas
        let server = args.server.as_deref().unwrap_or("");
        let client = client.with_server(server)?;
        return handle_login(client, args.username, args.password).await;
    }

    // For all other commands, implement server selection priority:
    // 1. Token's server (from --token or stored token) - highest priority
    // 2. --server override (used with username/password or when no token)
    // 3. Default saas
    let client = Client::new()?.with_token_path(None)?;

    // Check if using username/password authentication (will get new token)
    let using_credentials = args.username.is_some() && args.password.is_some();

    // Check if there's a stored token (client.server() returns the server from
    // the URL, which was set from the stored token if one exists)
    let has_stored_token = !client.token().await.is_empty();

    // If --token is provided, apply it first to get its server
    let client = match &args.token {
        Some(token) => client.with_token(token)?,
        None => client,
    };
    let effective_token_server = if args.token.is_some() || has_stored_token {
        Some(client.server().to_string())
    } else {
        None
    };

    // Build the client with appropriate server selection
    let client = if using_credentials {
        // Using username/password - honor --server (will get new token for that server)
        let client = match args.server {
            Some(server) => client.with_server(&server)?,
            None => client,
        };
        client
            .with_login(
                args.username.as_ref().unwrap(),
                args.password.as_ref().unwrap(),
            )
            .await?
    } else if let Some(ref token_server) = effective_token_server {
        // Using token - its server takes priority
        if let Some(ref requested_server) = args.server {
            // Normalize: "" and "saas" are equivalent
            let requested_normalized = match requested_server.as_str() {
                "" | "saas" => "saas",
                s => s,
            };
            if requested_normalized != token_server {
                eprintln!(
                    "Warning: --server '{}' will be ignored because your token is for server '{}'.",
                    requested_server, token_server
                );
                eprintln!(
                    "To switch servers, use: edgefirst-client login --server {}",
                    requested_server
                );
            }
        }
        // Token already applied above
        client
    } else {
        // No token available - use --server or default
        match args.server {
            Some(server) => client.with_server(&server)?,
            None => client,
        }
    };

    // Handle logout command
    if args.cmd == Command::Logout {
        return handle_logout(&client).await;
    }

    // Renew token for all other commands
    if let Err(e) = client.renew_token().await {
        // If token renewal fails, remove the corrupted token and ask user to login
        eprintln!("Authentication failed: {}", e);
        eprintln!("\nYour session token is invalid or has expired.");
        eprintln!("Please login again using:");
        eprintln!("  edgefirst-client login");

        // Attempt to clean up the invalid token
        if let Err(logout_err) = client.logout().await {
            eprintln!("Warning: Failed to clear invalid token: {}", logout_err);
        }

        std::process::exit(1);
    }

    // Handle all other commands
    match args.cmd {
        Command::Version | Command::Login | Command::Logout | Command::Sleep { .. } => {
            unreachable!()
        }
        Command::Token => handle_token(&client).await,
        Command::Organization => handle_organization(&client).await,
        Command::Projects { name } => handle_projects(&client, name).await,
        Command::Project { project_id } => handle_project(&client, project_id).await,
        Command::Datasets {
            project_id,
            annotation_sets,
            labels,
            name,
        } => handle_datasets(&client, project_id, annotation_sets, labels, name).await,
        Command::Dataset {
            dataset_id,
            annotation_sets,
            labels,
        } => handle_dataset(&client, dataset_id, annotation_sets, labels).await,
        Command::CreateDataset {
            project_id,
            name,
            description,
        } => handle_create_dataset(&client, project_id, name, description).await,
        Command::DeleteDataset { dataset_id } => handle_delete_dataset(&client, dataset_id).await,
        Command::CreateAnnotationSet {
            dataset_id,
            name,
            description,
        } => handle_create_annotation_set(&client, dataset_id, name, description).await,
        Command::DeleteAnnotationSet { annotation_set_id } => {
            handle_delete_annotation_set(&client, annotation_set_id).await
        }
        Command::DownloadDataset {
            dataset_id,
            groups,
            types,
            output,
            flatten,
        } => {
            let output = output.unwrap_or_else(|| ".".into());
            handle_download_dataset(&client, dataset_id, groups, types, output, flatten).await
        }
        Command::DownloadAnnotations {
            annotation_set_id,
            groups,
            types,
            output,
        } => handle_download_annotations(&client, annotation_set_id, groups, types, output).await,
        Command::UploadDataset {
            dataset_id,
            annotation_set_id,
            annotations,
            images,
        } => {
            handle_upload_dataset(&client, dataset_id, annotation_set_id, annotations, images).await
        }
        Command::Experiments { project_id, name } => {
            handle_experiments(&client, project_id, name).await
        }
        Command::Experiment { experiment_id } => handle_experiment(&client, experiment_id).await,
        Command::TrainingSessions {
            experiment_id,
            name,
        } => handle_training_sessions(&client, experiment_id, name).await,
        Command::TrainingSession {
            training_session_id,
            model,
            dataset,
            artifacts,
        } => handle_training_session(&client, training_session_id, model, dataset, artifacts).await,
        Command::DownloadArtifact {
            session_id,
            name,
            output,
        } => handle_download_artifact(&client, session_id, name, output).await,
        Command::UploadArtifact {
            session_id,
            path,
            name,
        } => handle_upload_artifact(&client, session_id, path, name).await,
        Command::Tasks {
            stages,
            name,
            workflow,
            status,
            manager,
        } => handle_tasks(&client, stages, name, workflow, status, manager).await,
        Command::Task { task_id, monitor } => handle_task(&client, task_id, monitor).await,
        Command::ValidationSessions { project_id } => {
            handle_validation_sessions(&client, project_id).await
        }
        Command::ValidationSession { session_id } => {
            handle_validation_session(&client, session_id).await
        }
        Command::Snapshots => handle_snapshots(&client).await,
        Command::Snapshot { snapshot_id } => handle_snapshot(&client, snapshot_id).await,
        Command::CreateSnapshot {
            source,
            annotation_set,
            description,
            from_path,
            from_dataset,
            monitor,
        } => {
            handle_create_snapshot(
                &client,
                CreateSnapshotParams {
                    source,
                    annotation_set,
                    description,
                    from_path,
                    from_dataset,
                    monitor,
                },
            )
            .await
        }
        Command::DownloadSnapshot {
            snapshot_id,
            output,
        } => handle_download_snapshot(&client, snapshot_id, output).await,
        Command::RestoreSnapshot {
            project_id,
            snapshot_id,
            topics,
            autolabel,
            autodepth,
            dataset_name,
            dataset_description,
            monitor,
        } => {
            handle_restore_snapshot(
                &client,
                RestoreSnapshotParams {
                    project_id,
                    snapshot_id,
                    topics,
                    autolabel,
                    autodepth,
                    dataset_name,
                    dataset_description,
                    monitor,
                },
            )
            .await
        }
        Command::DeleteSnapshot { snapshot_id } => {
            handle_delete_snapshot(&client, snapshot_id).await
        }
        Command::GenerateArrow {
            folder,
            output,
            detect_sequences,
        } => handle_generate_arrow(folder, output, detect_sequences),
        Command::ValidateSnapshot { path, verbose } => handle_validate_snapshot(path, verbose),
        Command::CocoToArrow {
            coco_path,
            output,
            masks,
            group,
        } => handle_coco_to_arrow(coco_path, output, masks, group).await,
        Command::ArrowToCoco {
            arrow_path,
            output,
            masks,
            groups,
            pretty,
        } => handle_arrow_to_coco(arrow_path, output, masks, groups, pretty).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Type aliases for cleaner test function signatures
    type BoundingBox = (f64, f64, f64, f64); // (cx, cy, w, h)
    type PolygonPoint = (f32, f32); // (x, y)
    type Polygon = Vec<PolygonPoint>;
    type OptionalBox2dData = Option<Vec<Option<BoundingBox>>>;
    type OptionalMaskData = Option<Vec<Option<Polygon>>>;

    #[test]
    fn test_is_valid_image_extension() {
        // Valid extensions
        assert!(is_valid_image_extension("jpg"));
        assert!(is_valid_image_extension("JPG"));
        assert!(is_valid_image_extension("jpeg"));
        assert!(is_valid_image_extension("JPEG"));
        assert!(is_valid_image_extension("png"));
        assert!(is_valid_image_extension("PNG"));
        assert!(is_valid_image_extension("bmp"));
        assert!(is_valid_image_extension("tiff"));
        assert!(is_valid_image_extension("tif"));
        assert!(is_valid_image_extension("webp"));

        // Invalid extensions
        assert!(!is_valid_image_extension("txt"));
        assert!(!is_valid_image_extension("pdf"));
        assert!(!is_valid_image_extension("doc"));
        assert!(!is_valid_image_extension(""));
    }

    #[test]
    fn test_find_image_folder_exists() {
        let temp_dir = std::env::temp_dir();
        let test_folder = temp_dir.join("test_images_folder");
        std::fs::create_dir_all(&test_folder).unwrap();

        let result = find_image_folder(&temp_dir, "test_images_folder");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), test_folder);

        std::fs::remove_dir(&test_folder).unwrap();
    }

    #[test]
    fn test_find_image_folder_not_exists() {
        let temp_dir = std::env::temp_dir();
        let result = find_image_folder(&temp_dir, "nonexistent_folder_12345");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_image_zip_exists() {
        let temp_dir = std::env::temp_dir();
        let test_zip = temp_dir.join("test_images.zip");
        std::fs::File::create(&test_zip).unwrap();

        let result = find_image_zip(&temp_dir, "test_images");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), test_zip);

        std::fs::remove_file(&test_zip).unwrap();
    }

    #[test]
    fn test_find_image_zip_not_exists() {
        let temp_dir = std::env::temp_dir();
        let result = find_image_zip(&temp_dir, "nonexistent_zip_12345");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_image_source_with_folder() {
        let temp_dir = std::env::temp_dir();
        let arrow_file = temp_dir.join("test_annotations.arrow");
        let test_folder = temp_dir.join("test_annotations");
        std::fs::create_dir_all(&test_folder).unwrap();

        let result = find_image_source(&arrow_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_folder);

        std::fs::remove_dir(&test_folder).unwrap();
    }

    #[test]
    fn test_find_image_source_with_zip() {
        let temp_dir = std::env::temp_dir();
        let arrow_file = temp_dir.join("test_annotations2.arrow");
        let test_zip = temp_dir.join("test_annotations2.zip");
        std::fs::File::create(&test_zip).unwrap();

        let result = find_image_source(&arrow_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_zip);

        std::fs::remove_file(&test_zip).unwrap();
    }

    #[test]
    fn test_determine_images_path_explicit() {
        let temp_dir = std::env::temp_dir();
        let test_folder = temp_dir.join("test_images_explicit");
        std::fs::create_dir_all(&test_folder).unwrap();

        let result = determine_images_path(&None, &Some(test_folder.clone()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_folder);

        std::fs::remove_dir(&test_folder).unwrap();
    }

    #[test]
    fn test_determine_images_path_from_annotations() {
        let temp_dir = std::env::temp_dir();
        let arrow_file = temp_dir.join("annotations.arrow");
        let test_folder = temp_dir.join("annotations");
        std::fs::create_dir_all(&test_folder).unwrap();

        let result = determine_images_path(&Some(arrow_file), &None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_folder);

        std::fs::remove_dir(&test_folder).unwrap();
    }

    #[cfg(feature = "polars")]
    mod arrow_parsing_tests {
        use super::*;
        use polars::prelude::*;
        use std::{io::Write, path::PathBuf};

        fn create_test_arrow_file(
            path: &PathBuf,
            names: Vec<&str>,
            groups: Option<Vec<Option<&str>>>,
            labels: Option<Vec<Option<&str>>>,
            box2d_data: OptionalBox2dData,
            mask_data: OptionalMaskData,
        ) -> Result<(), Box<dyn std::error::Error>> {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut columns = vec![Series::new("name".into(), names).into_column()];

            // Add optional group column
            if let Some(group_values) = groups {
                columns.push(Series::new("group".into(), group_values).into_column());
            }

            // Add optional label column
            if let Some(label_values) = labels {
                columns.push(Series::new("label".into(), label_values).into_column());
            }

            // Add optional box2d column (list of [cx, cy, w, h])
            if let Some(boxes) = box2d_data {
                let list = ListChunked::from_iter(boxes.into_iter().map(|box_opt| {
                    box_opt.map(|(cx, cy, w, h)| {
                        Series::new(
                            PlSmallStr::from_static(""),
                            vec![cx as f32, cy as f32, w as f32, h as f32],
                        )
                    })
                }));

                let mut series = list.into_series();
                series.rename(PlSmallStr::from_static("box2d"));
                columns.push(series.into_column());
            }

            // Add optional mask column (list of polygon coordinates)
            if let Some(masks) = mask_data {
                let list = ListChunked::from_iter(masks.into_iter().map(|mask_opt| {
                    mask_opt.map(|polygon| {
                        let mut coords = Vec::with_capacity(polygon.len() * 2);
                        for (x, y) in polygon {
                            coords.push(x);
                            coords.push(y);
                        }
                        Series::new(PlSmallStr::from_static(""), coords)
                    })
                }));

                let mut series = list.into_series();
                series.rename(PlSmallStr::from_static("mask"));
                columns.push(series.into_column());
            }

            let mut df = DataFrame::new(columns)?;

            let mut file = std::fs::File::create(path)?;
            IpcWriter::new(&mut file).finish(&mut df)?;
            Ok(())
        }

        fn create_test_arrow_file_with_frame(
            path: &PathBuf,
            samples: Vec<(&str, &str)>, // (name, frame)
            groups: Option<Vec<Option<&str>>>,
            labels: Option<Vec<Option<&str>>>,
            box2d_data: OptionalBox2dData,
            mask_data: OptionalMaskData,
        ) -> Result<(), Box<dyn std::error::Error>> {
            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let (names, frames): (Vec<_>, Vec<_>) = samples.into_iter().unzip();
            let mut columns = vec![
                Series::new("name".into(), names).into_column(),
                Series::new("frame".into(), frames).into_column(),
            ];

            // Add optional group column
            if let Some(group_values) = groups {
                columns.push(Series::new("group".into(), group_values).into_column());
            }

            // Add optional label column
            if let Some(label_values) = labels {
                columns.push(Series::new("label".into(), label_values).into_column());
            }

            // Add optional box2d column (list of [cx, cy, w, h])
            if let Some(boxes) = box2d_data {
                let list = ListChunked::from_iter(boxes.into_iter().map(|box_opt| {
                    box_opt.map(|(cx, cy, w, h)| {
                        Series::new(
                            PlSmallStr::from_static(""),
                            vec![cx as f32, cy as f32, w as f32, h as f32],
                        )
                    })
                }));

                let mut series = list.into_series();
                series.rename(PlSmallStr::from_static("box2d"));
                columns.push(series.into_column());
            }

            // Add optional mask column (list of polygon coordinates)
            if let Some(masks) = mask_data {
                let list = ListChunked::from_iter(masks.into_iter().map(|mask_opt| {
                    mask_opt.map(|polygon| {
                        let mut coords = Vec::with_capacity(polygon.len() * 2);
                        for (x, y) in polygon {
                            coords.push(x);
                            coords.push(y);
                        }
                        Series::new(PlSmallStr::from_static(""), coords)
                    })
                }));

                let mut series = list.into_series();
                series.rename(PlSmallStr::from_static("mask"));
                columns.push(series.into_column());
            }

            let mut df = DataFrame::new(columns)?;

            let mut file = std::fs::File::create(path)?;
            IpcWriter::new(&mut file).finish(&mut df)?;
            Ok(())
        }

        fn create_test_images_dir(
            dir: &PathBuf,
            image_names: Vec<&str>,
        ) -> Result<(), std::io::Error> {
            std::fs::create_dir_all(dir)?;
            for name in image_names {
                let image_path = dir.join(name);
                let mut file = std::fs::File::create(image_path)?;
                // Write minimal valid PNG header
                file.write_all(&[
                    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
                ])?;
            }
            Ok(())
        }

        #[test]
        fn test_parse_annotations_from_arrow_with_groups() {
            let test_dir = std::env::temp_dir().join("arrow_test_with_groups");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            // Create test data
            create_test_arrow_file(
                &arrow_file,
                vec!["image1.jpg", "image2.jpg", "image3.jpg"],
                Some(vec![Some("train"), Some("val"), Some("test")]),
                Some(vec![Some("cat"), Some("dog"), Some("bird")]),
                Some(vec![
                    Some((10.0, 20.0, 30.0, 40.0)),
                    None,
                    Some((5.0, 5.0, 15.0, 15.0)),
                ]),
                None,
            )
            .unwrap();

            create_test_images_dir(&images_dir, vec!["image1.jpg", "image2.jpg", "image3.jpg"])
                .unwrap();

            // Test parsing with annotations
            let result = parse_annotations_from_arrow(&Some(arrow_file.clone()), &images_dir, true);
            assert!(result.is_ok());
            let samples = result.unwrap();
            assert_eq!(samples.len(), 3);

            // Verify groups are present
            let train_sample = samples
                .iter()
                .find(|s| s.group() == Some(&"train".to_string()));
            assert!(train_sample.is_some());

            let val_sample = samples
                .iter()
                .find(|s| s.group() == Some(&"val".to_string()));
            assert!(val_sample.is_some());

            let test_sample = samples
                .iter()
                .find(|s| s.group() == Some(&"test".to_string()));
            assert!(test_sample.is_some());

            // Verify annotations were parsed
            let cat_sample = samples
                .iter()
                .find(|s| s.image_name.as_deref() == Some("image1.jpg"));
            assert!(cat_sample.is_some());
            let cat_sample = cat_sample.unwrap();
            assert_eq!(cat_sample.annotations.len(), 1);
            assert_eq!(cat_sample.annotations[0].label(), Some(&"cat".to_string()));
            let cat_bbox = cat_sample.annotations[0]
                .box2d()
                .expect("cat sample should include box2d geometry");
            assert!((cat_bbox.width() - 30.0).abs() < f32::EPSILON);
            assert!((cat_bbox.height() - 40.0).abs() < f32::EPSILON);

            // Verify image without box2d has only label annotation
            let dog_sample = samples
                .iter()
                .find(|s| s.image_name.as_deref() == Some("image2.jpg"));
            assert!(dog_sample.is_some());
            let dog_sample = dog_sample.unwrap();
            assert_eq!(dog_sample.annotations.len(), 1);
            assert_eq!(dog_sample.annotations[0].label(), Some(&"dog".to_string()));
            assert!(dog_sample.annotations[0].box2d().is_none());

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_from_arrow_with_masks() {
            let test_dir = std::env::temp_dir().join("arrow_test_with_masks");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            create_test_arrow_file(
                &arrow_file,
                vec!["mask1.png"],
                Some(vec![Some("train")]),
                Some(vec![Some("segment")]),
                None,
                Some(vec![Some(vec![
                    (0.0_f32, 0.0_f32),
                    (1.0_f32, 0.0_f32),
                    (1.0_f32, 1.0_f32),
                    (0.0_f32, 1.0_f32),
                ])]),
            )
            .unwrap();

            create_test_images_dir(&images_dir, vec!["mask1.png"]).unwrap();

            let samples =
                parse_annotations_from_arrow(&Some(arrow_file.clone()), &images_dir, true)
                    .expect("Arrow parsing should succeed");

            assert_eq!(samples.len(), 1);
            let annotations = &samples[0].annotations;
            assert_eq!(annotations.len(), 1, "Mask annotation should be preserved");

            let mask = annotations[0]
                .mask()
                .expect("Annotation should include mask geometry");
            assert_eq!(mask.polygon.len(), 1);
            let ring = &mask.polygon[0];
            assert_eq!(ring.len(), 4);
            assert_eq!(
                ring,
                &vec![
                    (0.0_f32, 0.0_f32),
                    (1.0_f32, 0.0_f32),
                    (1.0_f32, 1.0_f32),
                    (0.0_f32, 1.0_f32),
                ]
            );

            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_from_arrow_without_groups() {
            let test_dir = std::env::temp_dir().join("arrow_test_no_groups");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            // Create test data without groups column
            create_test_arrow_file(
                &arrow_file,
                vec!["img1.png", "img2.png"],
                None, // No groups
                Some(vec![Some("person"), Some("car")]),
                None,
                None,
            )
            .unwrap();

            create_test_images_dir(&images_dir, vec!["img1.png", "img2.png"]).unwrap();

            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true);
            assert!(result.is_ok());
            let samples = result.unwrap();
            assert_eq!(samples.len(), 2);

            // Verify all groups are None
            for sample in &samples {
                assert!(sample.group().is_none());
            }

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_from_arrow_without_annotations() {
            let test_dir = std::env::temp_dir().join("arrow_test_no_annotations");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            create_test_arrow_file(
                &arrow_file,
                vec!["photo1.jpg", "photo2.jpg"],
                Some(vec![Some("train"), Some("val")]),
                None, // No labels
                None, // No boxes
                None,
            )
            .unwrap();

            create_test_images_dir(&images_dir, vec!["photo1.jpg", "photo2.jpg"]).unwrap();

            // Test parsing WITHOUT uploading annotations
            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, false);
            assert!(result.is_ok());
            let samples = result.unwrap();
            assert_eq!(samples.len(), 2);

            // Verify no annotations were added
            for sample in &samples {
                assert_eq!(sample.annotations.len(), 0);
            }

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_from_arrow_missing_image() {
            let test_dir = std::env::temp_dir().join("arrow_test_missing_image");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            create_test_arrow_file(
                &arrow_file,
                vec!["exists.jpg", "missing.jpg"],
                None,
                None,
                None,
                None,
            )
            .unwrap();

            // Only create one image
            create_test_images_dir(&images_dir, vec!["exists.jpg"]).unwrap();

            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, false);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Image file not found")
            );

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_from_arrow_empty_file() {
            let test_dir = std::env::temp_dir().join("arrow_test_empty");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            create_test_arrow_file(&arrow_file, vec![], None, None, None, None).unwrap();
            std::fs::create_dir_all(&images_dir).unwrap();

            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, false);
            assert!(result.is_ok());
            let samples = result.unwrap();
            assert_eq!(samples.len(), 0);

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_from_arrow_none() {
            let images_dir = std::env::temp_dir().join("arrow_test_none");
            std::fs::create_dir_all(&images_dir).unwrap();

            let result = parse_annotations_from_arrow(&None, &images_dir, false);
            assert!(result.is_ok());
            let samples = result.unwrap();
            assert_eq!(samples.len(), 0);

            // Cleanup
            std::fs::remove_dir_all(&images_dir).ok();
        }

        #[test]
        fn test_find_image_path_for_sample_zip_not_supported() {
            let test_dir = std::env::temp_dir().join("arrow_test_zip");
            let zip_file = test_dir.join("images.zip");

            // Create a zip file (not a directory)
            std::fs::create_dir_all(&test_dir).unwrap();
            std::fs::File::create(&zip_file).unwrap();

            let err = build_image_index(zip_file.as_path()).unwrap_err();
            assert!(
                err.to_string()
                    .contains("ZIP file support not yet implemented")
            );

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_find_image_path_with_extension_variations() {
            let test_dir = std::env::temp_dir().join("arrow_test_extensions");
            std::fs::create_dir_all(&test_dir).unwrap();

            // Create image with .camera.jpg extension
            let image_path = test_dir.join("image1.camera.jpg");
            std::fs::File::create(&image_path).unwrap();

            // Should find the image when searching for "image1"
            let image_index = build_image_index(test_dir.as_path()).unwrap();
            let result = find_image_path_for_sample(&image_index, "image1");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), image_path);

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_from_arrow_with_sequences() {
            let test_dir = std::env::temp_dir().join("arrow_test_sequences");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");
            let sequence_dir = images_dir.join("sequence_a");

            // Create Arrow file with frame column to indicate sequence membership
            create_test_arrow_file_with_frame(
                &arrow_file,
                vec![("sequence_a", "1")],
                None,
                None,
                None,
                None,
            )
            .unwrap();

            create_test_images_dir(&sequence_dir, vec!["sequence_a_1.jpg"]).unwrap();

            let annotations = Some(arrow_file.clone());
            let samples = parse_annotations_from_arrow(&annotations, &images_dir, false)
                .expect("Arrow parsing should succeed");

            assert_eq!(samples.len(), 1);
            let sample = &samples[0];
            assert_eq!(sample.sequence_name(), Some(&"sequence_a".to_string()));
            assert_eq!(sample.frame_number(), Some(1));

            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_multiple_rows_same_image() {
            let test_dir = std::env::temp_dir().join("arrow_test_multi_annotations");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            // Create Arrow file with multiple annotations for same image
            create_test_arrow_file(
                &arrow_file,
                vec!["image1.jpg", "image1.jpg", "image2.jpg"],
                Some(vec![Some("train"), Some("train"), Some("val")]),
                Some(vec![Some("cat"), Some("dog"), Some("bird")]),
                Some(vec![
                    Some((10.0, 10.0, 20.0, 20.0)),
                    Some((30.0, 30.0, 40.0, 40.0)),
                    Some((5.0, 5.0, 10.0, 10.0)),
                ]),
                None,
            )
            .unwrap();

            create_test_images_dir(&images_dir, vec!["image1.jpg", "image2.jpg"]).unwrap();

            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true);
            assert!(result.is_ok());
            let samples = result.unwrap();

            // Should have 2 samples (image1 and image2)
            assert_eq!(samples.len(), 2);

            // image1 should have 2 annotations
            let image1_sample = samples
                .iter()
                .find(|s| s.image_name.as_deref() == Some("image1.jpg"));
            assert!(image1_sample.is_some());
            assert_eq!(image1_sample.unwrap().annotations.len(), 2);

            // image2 should have 1 annotation
            let image2_sample = samples
                .iter()
                .find(|s| s.image_name.as_deref() == Some("image2.jpg"));
            assert!(image2_sample.is_some());
            assert_eq!(image2_sample.unwrap().annotations.len(), 1);

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_flattened_structure() {
            // Test parsing annotations when images are in flattened structure
            // (all files in root directory with sequence prefix)
            let test_dir = std::env::temp_dir().join("arrow_test_flattened");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            // Create Arrow file with sequence samples (frame column not-null)
            create_test_arrow_file_with_frame(
                &arrow_file,
                vec![("seq_a", "1"), ("seq_a", "2"), ("seq_b", "1")],
                None,
                None,
                None,
                None,
            )
            .unwrap();

            // Create images in FLATTENED structure (all in root, no subdirectories)
            // Filenames have sequence prefix: sequence_name_frame.ext
            create_test_images_dir(
                &images_dir,
                vec![
                    "seq_a_1.camera.jpg",
                    "seq_a_2.camera.jpg",
                    "seq_b_1.camera.jpg",
                ],
            )
            .unwrap();

            let annotations = Some(arrow_file.clone());
            let samples = parse_annotations_from_arrow(&annotations, &images_dir, false)
                .expect("Should parse flattened structure");

            assert_eq!(samples.len(), 3, "Should find all 3 sequence samples");

            // Verify sequence metadata is preserved from Arrow file
            let seq_a_samples: Vec<_> = samples
                .iter()
                .filter(|s| s.sequence_name() == Some(&"seq_a".to_string()))
                .collect();
            assert_eq!(
                seq_a_samples.len(),
                2,
                "Should have 2 samples in sequence A"
            );

            let seq_b_samples: Vec<_> = samples
                .iter()
                .filter(|s| s.sequence_name() == Some(&"seq_b".to_string()))
                .collect();
            assert_eq!(seq_b_samples.len(), 1, "Should have 1 sample in sequence B");

            // Verify frame numbers are correct
            for sample in &samples {
                assert!(
                    sample.frame_number().is_some(),
                    "Frame number should be set for sequence samples"
                );
            }

            println!(
                "✓ Flattened structure test passed: {} samples parsed correctly",
                samples.len()
            );

            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_mixed_nested_and_standalone() {
            // Test parsing when dataset has both nested sequences and standalone images
            let test_dir = std::env::temp_dir().join("arrow_test_mixed");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");
            let sequence_dir = images_dir.join("sequence_x");

            // Create Arrow file with both sequence samples (with frames) and standalone
            // (without frames) We'll create two separate Arrow files and merge,
            // or use create_test_arrow_file twice For simplicity, test with all
            // having same structure but verify behavior

            // Create nested structure for sequence
            std::fs::create_dir_all(&sequence_dir).unwrap();
            std::fs::File::create(sequence_dir.join("sequence_x_1.jpg")).unwrap();
            std::fs::File::create(sequence_dir.join("sequence_x_2.jpg")).unwrap();

            // Create standalone image in root
            std::fs::create_dir_all(&images_dir).unwrap();
            std::fs::File::create(images_dir.join("standalone.jpg")).unwrap();

            // Create Arrow file with sequence samples
            create_test_arrow_file_with_frame(
                &arrow_file,
                vec![("sequence_x", "1"), ("sequence_x", "2")],
                None,
                None,
                None,
                None,
            )
            .unwrap();

            let annotations = Some(arrow_file.clone());
            let samples = parse_annotations_from_arrow(&annotations, &images_dir, false)
                .expect("Should parse mixed structure");

            assert_eq!(samples.len(), 2, "Should have 2 sequence samples");

            // Check all are sequence samples (since we only added sequence data to Arrow)
            for sample in &samples {
                assert!(
                    sample.sequence_name().is_some(),
                    "All samples should have sequence name"
                );
                assert!(
                    sample.frame_number().is_some(),
                    "All samples should have frame number"
                );
            }

            println!("✓ Mixed structure test passed (sequence samples only)");

            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_inconsistent_groups_fails() {
            // Test that parsing fails when same image has different group values across
            // rows
            let test_dir = std::env::temp_dir().join("arrow_test_inconsistent_groups");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            // Create Arrow file with INCONSISTENT groups for same image
            // image1.jpg appears twice with different group values (train vs val)
            create_test_arrow_file(
                &arrow_file,
                vec!["image1.jpg", "image1.jpg"],
                Some(vec![Some("train"), Some("val")]), // INCONSISTENT!
                Some(vec![Some("cat"), Some("dog")]),
                Some(vec![
                    Some((10.0, 10.0, 20.0, 20.0)),
                    Some((30.0, 30.0, 40.0, 40.0)),
                ]),
                None,
            )
            .unwrap();

            create_test_images_dir(&images_dir, vec!["image1.jpg"]).unwrap();

            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true);
            assert!(result.is_err(), "Should fail on inconsistent groups");
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("Inconsistent group"),
                "Error message should mention inconsistent group: {}",
                err_msg
            );
            assert!(
                err_msg.contains("image1"),
                "Error message should mention image name: {}",
                err_msg
            );

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_consistent_groups_succeeds() {
            // Test that parsing succeeds when same image has consistent group values
            let test_dir = std::env::temp_dir().join("arrow_test_consistent_groups");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            // Create Arrow file with CONSISTENT groups for same image
            create_test_arrow_file(
                &arrow_file,
                vec!["image1.jpg", "image1.jpg", "image2.jpg"],
                Some(vec![Some("train"), Some("train"), Some("val")]), // CONSISTENT
                Some(vec![Some("cat"), Some("dog"), Some("bird")]),
                Some(vec![
                    Some((10.0, 10.0, 20.0, 20.0)),
                    Some((30.0, 30.0, 40.0, 40.0)),
                    Some((5.0, 5.0, 10.0, 10.0)),
                ]),
                None,
            )
            .unwrap();

            create_test_images_dir(&images_dir, vec!["image1.jpg", "image2.jpg"]).unwrap();

            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true);
            assert!(result.is_ok(), "Should succeed with consistent groups");
            let samples = result.unwrap();
            assert_eq!(samples.len(), 2);

            // Verify groups are correct
            let image1 = samples
                .iter()
                .find(|s| s.image_name.as_deref() == Some("image1.jpg"))
                .unwrap();
            assert_eq!(image1.group(), Some(&"train".to_string()));
            assert_eq!(image1.annotations.len(), 2);

            let image2 = samples
                .iter()
                .find(|s| s.image_name.as_deref() == Some("image2.jpg"))
                .unwrap();
            assert_eq!(image2.group(), Some(&"val".to_string()));
            assert_eq!(image2.annotations.len(), 1);

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_inconsistent_null_vs_value_fails() {
            // Test that parsing fails when same image has null group and non-null group
            let test_dir = std::env::temp_dir().join("arrow_test_null_inconsistent");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            // Create Arrow file with null and non-null groups for same image
            create_test_arrow_file(
                &arrow_file,
                vec!["image1.jpg", "image1.jpg"],
                Some(vec![Some("train"), None]), // INCONSISTENT (value vs null)!
                Some(vec![Some("cat"), Some("dog")]),
                Some(vec![
                    Some((10.0, 10.0, 20.0, 20.0)),
                    Some((30.0, 30.0, 40.0, 40.0)),
                ]),
                None,
            )
            .unwrap();

            create_test_images_dir(&images_dir, vec!["image1.jpg"]).unwrap();

            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true);
            assert!(
                result.is_err(),
                "Should fail when group is null on some rows but not others"
            );
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("Inconsistent group"),
                "Error message should mention inconsistent group: {}",
                err_msg
            );

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }

        #[test]
        fn test_parse_annotations_all_null_groups_succeeds() {
            // Test that parsing succeeds when all rows for same image have null group
            let test_dir = std::env::temp_dir().join("arrow_test_all_null_groups");
            let arrow_file = test_dir.join("annotations.arrow");
            let images_dir = test_dir.join("images");

            // Create Arrow file with all null groups for same image (consistent)
            create_test_arrow_file(
                &arrow_file,
                vec!["image1.jpg", "image1.jpg"],
                Some(vec![None, None]), // CONSISTENT (both null)
                Some(vec![Some("cat"), Some("dog")]),
                Some(vec![
                    Some((10.0, 10.0, 20.0, 20.0)),
                    Some((30.0, 30.0, 40.0, 40.0)),
                ]),
                None,
            )
            .unwrap();

            create_test_images_dir(&images_dir, vec!["image1.jpg"]).unwrap();

            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true);
            assert!(result.is_ok(), "Should succeed when all groups are null");
            let samples = result.unwrap();
            assert_eq!(samples.len(), 1);
            assert_eq!(samples[0].group(), None);
            assert_eq!(samples[0].annotations.len(), 2);

            // Cleanup
            std::fs::remove_dir_all(&test_dir).ok();
        }
    }
}
