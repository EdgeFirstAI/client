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

    /// Increase logging verbosity (-v for debug, -vv for trace)
    #[clap(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Write trace output to file (Perfetto/CTF-compatible JSON format).
    /// Requires build with --features trace-file.
    #[clap(long, global = true, env = "TRACE_FILE")]
    trace_file: Option<PathBuf>,

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

        /// List available groups for the dataset
        #[clap(long, short)]
        groups: bool,
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
    ///
    /// Valid sensor types: image, lidar.pcd, lidar.png, lidar.jpg, radar.pcd,
    /// radar.png, all. Use "all" to download all sensor types.
    DownloadDataset {
        /// Dataset ID (optional if --list-types is used)
        dataset_id: Option<String>,

        /// Only fetch samples belonging to the provided dataset groups.
        #[clap(long, value_delimiter = ',')]
        groups: Vec<String>,

        /// Fetch the data types for the dataset. Valid types: image, lidar.pcd,
        /// lidar.png, lidar.jpg, radar.pcd, radar.png, all.
        /// Use "all" to download all sensor types.
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

        /// List all valid sensor types and exit.
        #[clap(long)]
        list_types: bool,
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

        /// Annotation types to download (box2d, box3d, mask). Downloads all
        /// types if not specified.
        #[clap(long, value_delimiter = ',')]
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

        /// Include segmentation masks (default: true, use --masks=false to
        /// disable)
        #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
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
    ///   edgefirst arrow-to-coco dataset.arrow -o instances.json --groups
    /// train,val
    ArrowToCoco {
        /// Path to EdgeFirst Arrow file
        arrow_path: PathBuf,

        /// Output COCO JSON file path
        #[clap(long, short = 'o')]
        output: PathBuf,

        /// Include segmentation masks (default: true, use --masks=false to
        /// disable)
        #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
        masks: bool,

        /// Filter by group names (comma-separated)
        #[clap(long, value_delimiter = ',')]
        groups: Vec<String>,

        /// Pretty-print JSON output
        #[clap(long)]
        pretty: bool,
    },
    /// Import COCO dataset into EdgeFirst Studio.
    ///
    /// COCO datasets must be extracted before import. ZIP archives are not
    /// supported directly - extract annotations and images first.
    ///
    /// You can either specify existing dataset/annotation-set IDs, or use
    /// --name to create a new dataset automatically.
    ///
    /// Setup:
    ///   cd ~/Datasets/COCO
    ///   unzip annotations_trainval2017.zip
    ///   unzip val2017.zip
    ///
    /// Examples:
    ///   # Create new dataset automatically:
    ///   edgefirst import-coco ./coco --project proj-123 --name "COCO 2017"
    ///
    ///   # Use existing dataset:
    ///   edgefirst import-coco ./coco --dataset ds-123 --annotation-set as-456
    ImportCoco {
        /// Path to COCO annotation JSON file or extracted directory
        coco_path: PathBuf,

        /// Project ID (required when creating new dataset with --name)
        #[clap(long, short = 'p')]
        project: Option<String>,

        /// Create new dataset with this name (alternative to --dataset)
        #[clap(long, short = 'n')]
        name: Option<String>,

        /// Description for new dataset (used with --name)
        #[clap(long, short = 'd')]
        description: Option<String>,

        /// Target dataset ID (alternative to --name)
        #[clap(long)]
        dataset: Option<String>,

        /// Target annotation set ID (defaults to first set if not specified)
        #[clap(long)]
        annotation_set: Option<String>,

        /// Group name for samples (auto-detected from filename if not
        /// specified)
        #[clap(long)]
        group: Option<String>,

        /// Include segmentation masks (default: true, use --masks=false to
        /// disable)
        #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
        masks: bool,

        /// Include images in upload (default: true, use --images=false to
        /// disable)
        #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
        images: bool,

        /// Batch size for uploads
        #[clap(long, default_value = "100")]
        batch_size: usize,

        /// Maximum concurrent uploads (default: 64)
        #[clap(long, default_value = "64")]
        concurrency: usize,

        /// Verify import instead of uploading (compares local COCO to Studio)
        #[clap(long)]
        verify: bool,

        /// Update annotations on existing samples without re-uploading images.
        /// Use this to add masks to samples that were imported without them,
        /// or to sync updated annotations to Studio.
        #[clap(long)]
        update: bool,
    },
    /// Export EdgeFirst Studio dataset to COCO format.
    ///
    /// Downloads samples and annotations from Studio and converts to COCO
    /// format.
    ///
    /// Examples:
    ///   edgefirst export-coco dataset-123 annset-456 -o instances.json
    ///   edgefirst export-coco dataset-123 annset-456 -o coco.zip --images
    /// --groups train,val
    ExportCoco {
        /// Source dataset ID in Studio
        dataset_id: String,

        /// Source annotation set ID
        annotation_set_id: String,

        /// Output file path (JSON or ZIP)
        #[clap(long, short = 'o')]
        output: PathBuf,

        /// Filter by group names (comma-separated)
        #[clap(long, value_delimiter = ',')]
        groups: Vec<String>,

        /// Include segmentation masks (default: true, use --masks=false to
        /// disable)
        #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
        masks: bool,

        /// Include images in output (creates ZIP)
        #[clap(long)]
        images: bool,

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
    groups: bool,
) -> Result<(), Error> {
    let dataset_id_parsed: edgefirst_client::DatasetID = dataset_id.clone().try_into()?;
    let dataset = client.dataset(dataset_id_parsed).await?;
    println!(
        "[{}] {}: {}",
        dataset.id(),
        dataset.name(),
        dataset.description()
    );

    if labels {
        let labels = client.labels(dataset_id_parsed).await?;
        println!("Labels:");
        for label in labels {
            println!("    [{}] {}", label.id(), label.name());
        }
    }

    if groups {
        let groups = client.groups(dataset_id_parsed).await?;
        println!("Groups:");
        for group in groups {
            println!("    [{}] {}", group.id, group.name);
        }
    }

    if annotation_sets {
        let annotation_sets = client.annotation_sets(dataset_id_parsed).await?;
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
            "[{elapsed_precise} ETA: {eta}] {msg} {wide_bar:.yellow} {human_pos}/{human_len}",
        )
        .unwrap()
        .progress_chars("█▇▆▅▄▃▂▁  "),
    );
    bar.set_message("Starting");

    let (tx, mut rx) = mpsc::channel::<Progress>(1);

    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            // Use status field if present, otherwise default message
            let msg = progress.status.as_deref().unwrap_or("Downloading");
            bar.set_message(msg.to_string());
            if progress.total > 0 {
                bar.set_length(progress.total as u64);
                bar.set_position(progress.current as u64);
            }
        }
        bar.finish_with_message("Done");
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

/// Sensor file types and their expected extensions for EdgeFirst Dataset Format.
/// Maps server API type name to file extension pattern.
#[cfg(feature = "polars")]
const SENSOR_FILE_TYPES: &[(&str, &str)] = &[
    ("lidar.pcd", ".lidar.pcd"),
    ("lidar.depth", ".lidar.png"),
    ("lidar.reflect", ".lidar.jpg"),
    ("radar.pcd", ".radar.pcd"),
    ("radar.png", ".radar.png"),
];

/// Index of sensor files that can be backed by either a directory or a ZIP archive.
///
/// This abstraction allows the upload workflow to work with both file sources
/// without extracting ZIP archives to disk.
#[cfg(feature = "polars")]
#[derive(Debug)]
enum SensorFileIndex {
    /// Files stored in a directory on disk.
    Directory {
        index: std::collections::HashMap<String, Vec<PathBuf>>,
    },
    /// Files stored in a ZIP archive (read directly without extraction).
    Zip {
        archive_path: PathBuf,
        /// Maps sample names to ZIP entry names.
        index: std::collections::HashMap<String, Vec<String>>,
    },
}

#[cfg(feature = "polars")]
impl SensorFileIndex {
    /// Returns true if this index is backed by a ZIP archive.
    fn is_zip(&self) -> bool {
        matches!(self, SensorFileIndex::Zip { .. })
    }

    /// Get the ZIP archive path if this is a ZIP-backed index.
    fn archive_path(&self) -> Option<&Path> {
        match self {
            SensorFileIndex::Zip { archive_path, .. } => Some(archive_path),
            _ => None,
        }
    }

    /// Find the image entry name without reading bytes (for parallel resolution).
    /// Returns (entry_name, filename) for ZIP, (path_str, filename) for Directory.
    fn find_image_entry(&self, sample_name: &str) -> Result<(String, String), Error> {
        const EXTENSIONS: &[&str] = &[
            ".camera.jpg",
            ".camera.jpeg",
            ".camera.png",
            ".jpg",
            ".jpeg",
            ".png",
        ];

        match self {
            SensorFileIndex::Directory { index } => {
                for ext in EXTENSIONS {
                    let candidate = format!("{}{}", sample_name, ext);
                    if let Some(paths) = index.get(&candidate) {
                        match paths.len() {
                            0 => continue,
                            1 => {
                                let path_str = paths[0].to_str().unwrap().to_string();
                                let filename = paths[0]
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(&candidate)
                                    .to_string();
                                return Ok((path_str, filename));
                            }
                            _ => {
                                return Err(Error::InvalidParameters(format!(
                                    "Multiple image matches found for '{}': {:?}",
                                    candidate, paths
                                )));
                            }
                        }
                    }
                }
                Err(Error::MissingImages(format!(
                    "No image found for sample '{}'",
                    sample_name
                )))
            }
            SensorFileIndex::Zip { index, .. } => {
                for ext in EXTENSIONS {
                    let candidate = format!("{}{}", sample_name, ext);
                    if let Some(entries) = index.get(&candidate) {
                        match entries.len() {
                            0 => continue,
                            1 => {
                                let filename = std::path::Path::new(&entries[0])
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(&entries[0])
                                    .to_string();
                                return Ok((entries[0].clone(), filename));
                            }
                            _ => {
                                return Err(Error::InvalidParameters(format!(
                                    "Multiple image matches found for '{}': {:?}",
                                    candidate, entries
                                )));
                            }
                        }
                    }
                }
                Err(Error::MissingImages(format!(
                    "No image found for sample '{}' in ZIP",
                    sample_name
                )))
            }
        }
    }

    /// Find a sensor entry name without reading bytes (for parallel resolution).
    /// Returns Some((entry_name, filename)) if found.
    fn find_sensor_entry(
        &self,
        sample_name: &str,
        extension: &str,
    ) -> Option<(String, String)> {
        let candidate = format!("{}{}", sample_name, extension);

        match self {
            SensorFileIndex::Directory { index } => {
                if let Some(paths) = index.get(&candidate) {
                    if paths.len() == 1 {
                        let path_str = paths[0].to_str().unwrap().to_string();
                        let filename = paths[0]
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&candidate)
                            .to_string();
                        return Some((path_str, filename));
                    }
                }
                None
            }
            SensorFileIndex::Zip { index, .. } => {
                if let Some(entries) = index.get(&candidate) {
                    if entries.len() == 1 {
                        let filename = std::path::Path::new(&entries[0])
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&entries[0])
                            .to_string();
                        return Some((entries[0].clone(), filename));
                    }
                }
                None
            }
        }
    }
}

/// Parse the size column [width, height] from an Arrow DataFrame row.
/// Returns (Option<width>, Option<height>).
#[cfg(feature = "polars")]
fn parse_size_from_dataframe(
    df: &polars::prelude::DataFrame,
    idx: usize,
) -> (Option<u32>, Option<u32>) {
    use polars::prelude::*;

    let size_col = match df.column("size") {
        Ok(col) => col,
        Err(_) => return (None, None),
    };

    // Try to extract as array of u32
    let extract_size = |series: Series| -> Option<Vec<u32>> {
        if let Ok(vals) = series.u32() {
            return Some(vals.into_iter().flatten().collect());
        }
        // Try u64 and convert
        if let Ok(vals) = series.u64() {
            return Some(vals.into_iter().flatten().map(|v| v as u32).collect());
        }
        // Try f32 and convert
        if let Ok(vals) = series.f32() {
            return Some(
                vals.into_iter()
                    .flatten()
                    .map(|v| v as u32)
                    .collect(),
            );
        }
        None
    };

    let coords = if let Ok(array_chunked) = size_col.array() {
        array_chunked
            .get_as_series(idx)
            .and_then(|series| extract_size(series.clone()))
    } else if let Ok(list_chunked) = size_col.list() {
        list_chunked
            .get_as_series(idx)
            .and_then(|series| extract_size(series.clone()))
    } else {
        None
    };

    match coords {
        Some(values) if values.len() >= 2 => (Some(values[0]), Some(values[1])),
        _ => (None, None),
    }
}

/// Parse the location [lat, lon] and pose [yaw, pitch, roll] columns from an Arrow
/// DataFrame row into a Location struct.
#[cfg(feature = "polars")]
fn parse_location_from_dataframe(
    df: &polars::prelude::DataFrame,
    idx: usize,
) -> Option<edgefirst_client::Location> {
    use polars::prelude::*;

    let extract_floats = |col: &Column, idx: usize| -> Option<Vec<f64>> {
        let extract_from_series = |series: Series| -> Option<Vec<f64>> {
            if let Ok(vals) = series.f32() {
                return Some(vals.into_iter().flatten().map(|v| v as f64).collect());
            }
            if let Ok(vals) = series.f64() {
                return Some(vals.into_iter().flatten().collect());
            }
            None
        };

        if let Ok(array_chunked) = col.array() {
            array_chunked
                .get_as_series(idx)
                .and_then(|series| extract_from_series(series.clone()))
        } else if let Ok(list_chunked) = col.list() {
            list_chunked
                .get_as_series(idx)
                .and_then(|series| extract_from_series(series.clone()))
        } else {
            None
        }
    };

    // Parse location [lat, lon]
    let gps = df
        .column("location")
        .ok()
        .and_then(|col| extract_floats(col, idx))
        .and_then(|coords| {
            if coords.len() >= 2 {
                Some(edgefirst_client::GpsData {
                    lat: coords[0],
                    lon: coords[1],
                })
            } else {
                None
            }
        });

    // Parse pose [yaw, pitch, roll]
    let imu = df
        .column("pose")
        .ok()
        .and_then(|col| extract_floats(col, idx))
        .and_then(|coords| {
            if coords.len() >= 3 {
                Some(edgefirst_client::ImuData {
                    yaw: coords[0],
                    pitch: coords[1],
                    roll: coords[2],
                })
            } else {
                None
            }
        });

    if gps.is_some() || imu.is_some() {
        Some(edgefirst_client::Location { gps, imu })
    } else {
        None
    }
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

/// Helper struct to store sample metadata during parsing.
/// Used to collect all annotations for a sample before creating the final Sample object.
#[cfg(feature = "polars")]
struct SampleMetadata {
    group: Option<String>,
    sequence_name: Option<String>,
    frame_number: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    location: Option<edgefirst_client::Location>,
    degradation: Option<String>,
    annotations: Vec<edgefirst_client::Annotation>,
}

fn parse_annotations_from_arrow(
    annotations: &Option<PathBuf>,
    images_path: &Path,
    should_upload_annotations: bool,
    progress: &indicatif::ProgressBar,
) -> Result<Vec<edgefirst_client::Sample>, Error> {
    use polars::prelude::*;
    use std::{collections::HashMap, fs::File};

    // Map: sample_name -> metadata
    // sequence_name is Some(name) when frame is not-null, indicating this sample is
    // part of a sequence
    let mut samples_map: HashMap<String, SampleMetadata> = HashMap::new();

    if let Some(arrow_path) = annotations {
        let mut file = File::open(arrow_path)?;
        let df = IpcReader::new(&mut file)
            .finish()
            .map_err(|e| Error::InvalidParameters(format!("Failed to read Arrow file: {}", e)))?;

        let total_rows = df.height();

        // Switch to a progress bar with known total for Arrow parsing
        progress.set_length(total_rows as u64);
        progress.set_style(
            indicatif::ProgressStyle::with_template(
                "[{elapsed_precise} ETA: {eta}] Parsing Arrow: {wide_bar:.yellow} {human_pos}/{human_len} rows"
            )
            .unwrap()
            .progress_chars("█▇▆▅▄▃▂▁  "),
        );
        progress.set_position(0);

        // Process each row in the DataFrame
        for idx in 0..total_rows {
            progress.set_position(idx as u64);
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

            // Extract optional size [width, height] from Arrow file (2025.10 format)
            let (sample_width, sample_height) = parse_size_from_dataframe(&df, idx);

            // Extract optional location [lat, lon] and pose [yaw, pitch, roll] (2025.10 format)
            let sample_location = parse_location_from_dataframe(&df, idx);

            // Extract optional degradation field (2025.10 format)
            let sample_degradation = df
                .column("degradation")
                .ok()
                .and_then(|c| c.str().ok())
                .and_then(|s| s.get(idx))
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

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
                    width: sample_width,
                    height: sample_height,
                    location: sample_location,
                    degradation: sample_degradation,
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

                // Extract label_index if present (numeric class index)
                let label_index = df
                    .column("label_index")
                    .ok()
                    .and_then(|c| c.u64().ok())
                    .and_then(|s| s.get(idx));

                if let Some(lbl_idx) = label_index {
                    annotation.set_label_index(Some(lbl_idx));
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

    // Build sensor file index for all sensor types (images, lidar, radar, etc.)
    // Reset progress bar to spinner for indexing phase
    progress.set_style(
        indicatif::ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    progress.set_message("Indexing sensor files...");
    let sensor_index = build_sensor_file_index(images_path, progress)?;

    // Convert HashMap to Vec<Sample> by resolving file paths or reading from ZIP
    let total_samples = samples_map.len();

    // Switch to progress bar with known total for sample resolution
    progress.set_length(total_samples as u64);
    progress.set_style(
        indicatif::ProgressStyle::with_template(
            "[{elapsed_precise} ETA: {eta}] Resolving samples: {wide_bar:.yellow} {human_pos}/{human_len}"
        )
        .unwrap()
        .progress_chars("█▇▆▅▄▃▂▁  "),
    );
    progress.set_position(0);

    // Use parallel resolution for ZIP files, sequential for directories
    let samples = if sensor_index.is_zip() {
        resolve_samples_parallel_zip(samples_map, &sensor_index, progress)?
    } else {
        resolve_samples_sequential(samples_map, &sensor_index, progress)?
    };

    Ok(samples)
}

/// Resolve samples sequentially for directory-based sources.
/// Fast because no I/O needed - just path resolution.
#[cfg(feature = "polars")]
fn resolve_samples_sequential(
    samples_map: std::collections::HashMap<String, SampleMetadata>,
    sensor_index: &SensorFileIndex,
    progress: &indicatif::ProgressBar,
) -> Result<Vec<edgefirst_client::Sample>, Error> {
    let mut samples = Vec::with_capacity(samples_map.len());

    for (idx, (sample_name, metadata)) in samples_map.into_iter().enumerate() {
        progress.set_position(idx as u64);

        let (image_path, image_filename) = sensor_index.find_image_entry(&sample_name)?;
        let image_file = edgefirst_client::SampleFile::with_filename(
            "image".to_string(),
            image_path,
        );

        let mut annotations = metadata.annotations;
        for annotation in &mut annotations {
            annotation.set_name(Some(image_filename.clone()));
            annotation.set_sequence_name(metadata.sequence_name.clone());
            annotation.set_group(metadata.group.clone());
            if let Some(frame) = metadata.frame_number {
                annotation.set_frame_number(Some(frame));
            }
        }

        let mut files = vec![image_file];
        for (sensor_type, extension) in SENSOR_FILE_TYPES {
            if let Some((sensor_path, _)) = sensor_index.find_sensor_entry(&sample_name, extension) {
                files.push(edgefirst_client::SampleFile::with_filename(
                    sensor_type.to_string(),
                    sensor_path,
                ));
            }
        }

        samples.push(edgefirst_client::Sample {
            image_name: Some(image_filename),
            group: metadata.group,
            sequence_name: metadata.sequence_name,
            frame_number: metadata.frame_number,
            width: metadata.width,
            height: metadata.height,
            location: metadata.location,
            degradation: metadata.degradation,
            files,
            annotations,
            ..Default::default()
        });
    }

    progress.set_position(samples.len() as u64);
    Ok(samples)
}

/// Intermediate structure holding resolved entries before parallel byte loading.
#[cfg(feature = "polars")]
struct ResolvedSampleEntries {
    image_entry: String,
    image_filename: String,
    sensor_entries: Vec<(String, String, String)>, // (sensor_type, entry_name, filename)
    metadata: SampleMetadata,
}

/// Resolve samples in parallel for ZIP-backed sources.
/// Uses rayon with thread-local ZIP archive handles for efficient parallel I/O.
#[cfg(feature = "polars")]
fn resolve_samples_parallel_zip(
    samples_map: std::collections::HashMap<String, SampleMetadata>,
    sensor_index: &SensorFileIndex,
    progress: &indicatif::ProgressBar,
) -> Result<Vec<edgefirst_client::Sample>, Error> {
    use rayon::prelude::*;
    use std::sync::atomic::AtomicU64;

    let archive_path = sensor_index
        .archive_path()
        .ok_or_else(|| Error::InvalidParameters("Expected ZIP index".to_string()))?
        .to_path_buf();

    // Phase 1: Resolve all entries without reading bytes (fast, single-threaded)
    // This validates all samples exist before we start parallel I/O
    let mut resolved_entries = Vec::with_capacity(samples_map.len());
    for (sample_name, metadata) in samples_map {
        let (image_entry, image_filename) = sensor_index.find_image_entry(&sample_name)?;

        let mut sensor_entries = Vec::new();
        for (sensor_type, extension) in SENSOR_FILE_TYPES {
            if let Some((entry_name, filename)) = sensor_index.find_sensor_entry(&sample_name, extension) {
                sensor_entries.push((sensor_type.to_string(), entry_name, filename));
            }
        }

        resolved_entries.push(ResolvedSampleEntries {
            image_entry,
            image_filename,
            sensor_entries,
            metadata,
        });
    }

    // Phase 2: Read bytes in parallel using thread-local ZIP archive handles
    // Using fold pattern so each rayon thread opens the archive ONCE and reuses it
    let progress_counter = AtomicU64::new(0);
    let total = resolved_entries.len() as u64;
    let archive_path_ref = &archive_path;

    // fold: each thread accumulates results with its own archive handle
    // reduce: merge results from all threads
    let thread_results: Vec<Vec<Result<edgefirst_client::Sample, Error>>> = resolved_entries
        .into_par_iter()
        .fold(
            || -> (Option<zip::ZipArchive<std::fs::File>>, Vec<Result<edgefirst_client::Sample, Error>>) {
                (None, Vec::new())
            },
            |(mut archive_opt, mut results), entry| {
                // Initialize archive on first use in this thread (lazy, once per thread)
                let archive = match archive_opt.as_mut() {
                    Some(a) => a,
                    None => {
                        let file = match std::fs::File::open(archive_path_ref) {
                            Ok(f) => f,
                            Err(e) => {
                                results.push(Err(Error::InvalidParameters(format!(
                                    "Failed to open ZIP file {}: {}",
                                    archive_path_ref.display(),
                                    e
                                ))));
                                return (archive_opt, results);
                            }
                        };
                        let new_archive = match zip::ZipArchive::new(file) {
                            Ok(a) => a,
                            Err(e) => {
                                results.push(Err(Error::InvalidParameters(format!(
                                    "Failed to read ZIP archive {}: {}",
                                    archive_path_ref.display(),
                                    e
                                ))));
                                return (archive_opt, results);
                            }
                        };
                        archive_opt.insert(new_archive)
                    }
                };

                // Process this entry
                let result = process_zip_entry(archive, entry, &progress_counter, total, progress);
                results.push(result);

                (archive_opt, results)
            },
        )
        .map(|(_, results)| results)
        .collect();

    // Flatten and collect results, propagating any errors
    let mut samples = Vec::with_capacity(total as usize);
    for thread_result in thread_results {
        for result in thread_result {
            samples.push(result?);
        }
    }

    progress.set_position(total);
    Ok(samples)
}

/// Process a single ZIP entry to create a Sample.
/// Extracted to keep the fold closure cleaner.
#[cfg(feature = "polars")]
fn process_zip_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    entry: ResolvedSampleEntries,
    progress_counter: &std::sync::atomic::AtomicU64,
    total: u64,
    progress: &indicatif::ProgressBar,
) -> Result<edgefirst_client::Sample, Error> {
    use std::sync::atomic::Ordering;

    // Read image bytes
    let image_bytes = read_zip_entry_from_archive(archive, &entry.image_entry)?;
    let image_file = edgefirst_client::SampleFile::with_bytes(
        "image".to_string(),
        entry.image_filename.clone(),
        image_bytes,
    );

    // Read sensor bytes
    let mut files = vec![image_file];
    for (sensor_type, sensor_entry, sensor_filename) in &entry.sensor_entries {
        let sensor_bytes = read_zip_entry_from_archive(archive, sensor_entry)?;
        files.push(edgefirst_client::SampleFile::with_bytes(
            sensor_type.clone(),
            sensor_filename.clone(),
            sensor_bytes,
        ));
    }

    // Update annotations with sample metadata
    let mut annotations = entry.metadata.annotations;
    for annotation in &mut annotations {
        annotation.set_name(Some(entry.image_filename.clone()));
        annotation.set_sequence_name(entry.metadata.sequence_name.clone());
        annotation.set_group(entry.metadata.group.clone());
        if let Some(frame) = entry.metadata.frame_number {
            annotation.set_frame_number(Some(frame));
        }
    }

    // Update progress (atomic for thread safety)
    let current = progress_counter.fetch_add(1, Ordering::Relaxed);
    if current % 50 == 0 || current == total - 1 {
        progress.set_position(current + 1);
    }

    Ok(edgefirst_client::Sample {
        image_name: Some(entry.image_filename),
        group: entry.metadata.group,
        sequence_name: entry.metadata.sequence_name,
        frame_number: entry.metadata.frame_number,
        width: entry.metadata.width,
        height: entry.metadata.height,
        location: entry.metadata.location,
        degradation: entry.metadata.degradation,
        files,
        annotations,
        ..Default::default()
    })
}

/// Read bytes from an already-opened ZIP archive.
#[cfg(feature = "polars")]
fn read_zip_entry_from_archive(
    archive: &mut zip::ZipArchive<std::fs::File>,
    entry_name: &str,
) -> Result<Vec<u8>, Error> {
    use std::io::Read;

    let mut entry = archive.by_name(entry_name).map_err(|e| {
        Error::InvalidParameters(format!("Failed to read ZIP entry '{}': {}", entry_name, e))
    })?;

    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut bytes).map_err(|e| {
        Error::InvalidParameters(format!(
            "Failed to read ZIP entry '{}' contents: {}",
            entry_name, e
        ))
    })?;

    Ok(bytes)
}

#[cfg(feature = "polars")]
/// Builds an index mapping filenames to their source (path or ZIP entry).
///
/// This function handles all sensor data files (LiDAR, radar, etc.)
/// in addition to camera images. The index is used for:
/// - Camera images: .jpg, .jpeg, .png, .camera.jpeg, .camera.png
/// - LiDAR PCD: .lidar.pcd
/// - LiDAR depth: .lidar.png, .depth.png
/// - LiDAR reflect: .lidar.jpg, .lidar.jpeg
/// - Radar PCD: .radar.pcd
/// - Radar cube: .radar.png
///
/// Supports both directories and ZIP archives. For ZIP files, files are read
/// directly from the archive without extraction for efficiency.
fn build_sensor_file_index(
    images_path: &Path,
    progress: &indicatif::ProgressBar,
) -> Result<SensorFileIndex, Error> {
    // Handle ZIP files - read directly without extraction
    if images_path.extension().is_some_and(|e| e == "zip") {
        return build_sensor_file_index_from_zip(images_path, progress);
    }

    if !images_path.is_dir() {
        return Err(Error::InvalidParameters(format!(
            "Images path must be a directory or ZIP file: {}",
            images_path.display()
        )));
    }

    build_sensor_file_index_from_dir(images_path, progress)
}

#[cfg(feature = "polars")]
/// Build sensor file index from a directory.
fn build_sensor_file_index_from_dir(
    dir_path: &Path,
    progress: &indicatif::ProgressBar,
) -> Result<SensorFileIndex, Error> {
    let mut index: std::collections::HashMap<String, Vec<PathBuf>> =
        std::collections::HashMap::new();

    progress.set_message(format!("Scanning directory: {}...", dir_path.display()));

    let mut file_count = 0;
    // Recursively walk directory tree - works for both nested and flattened structures
    for entry in WalkDir::new(dir_path) {
        let entry = entry.map_err(|e| {
            Error::InvalidParameters(format!("Failed to read sensor directory: {}", e))
        })?;

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path().to_path_buf();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Check if this is a valid sensor file (image or sensor data)
        if !is_valid_sensor_file(file_name) {
            continue;
        }

        file_count += 1;
        // Update progress every 500 files
        if file_count % 500 == 0 {
            progress.set_message(format!("Indexed {} sensor files...", file_count));
        }

        // Generate lookup keys for this file
        for key in generate_sensor_lookup_keys(file_name) {
            index.entry(key).or_default().push(path.clone());
        }
    }

    progress.set_message(format!("Indexed {} sensor files from directory", file_count));
    Ok(SensorFileIndex::Directory { index })
}

#[cfg(feature = "polars")]
/// Build sensor file index from a ZIP archive without extracting.
///
/// Files are read directly from the archive on demand, avoiding the need to
/// extract to a temporary directory. This is efficient because dataset ZIPs
/// typically use store mode (no compression) since images are already compressed.
fn build_sensor_file_index_from_zip(
    zip_path: &Path,
    progress: &indicatif::ProgressBar,
) -> Result<SensorFileIndex, Error> {
    use std::fs::File;

    progress.set_message(format!("Opening ZIP archive: {}...", zip_path.display()));

    let file = File::open(zip_path).map_err(|e| {
        Error::InvalidParameters(format!("Failed to open ZIP file {}: {}", zip_path.display(), e))
    })?;

    let archive = zip::ZipArchive::new(file).map_err(|e| {
        Error::InvalidParameters(format!(
            "Failed to read ZIP archive {}: {}",
            zip_path.display(),
            e
        ))
    })?;

    let total_entries = archive.len();
    progress.set_message(format!("Indexing {} ZIP entries...", total_entries));

    let mut index: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    let mut sensor_file_count = 0;
    // Build index of entry names without extracting
    for i in 0..total_entries {
        // Update progress every 1000 entries
        if i % 1000 == 0 {
            progress.set_message(format!(
                "Indexing ZIP: {}/{} entries ({} sensor files)...",
                i, total_entries, sensor_file_count
            ));
        }

        let entry_name = archive
            .name_for_index(i)
            .ok_or_else(|| {
                Error::InvalidParameters(format!("Failed to read ZIP entry name at index {}", i))
            })?
            .to_string();

        // Skip directories (they end with /)
        if entry_name.ends_with('/') {
            continue;
        }

        // Get just the filename from the path
        let file_name = std::path::Path::new(&entry_name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&entry_name);

        if !is_valid_sensor_file(file_name) {
            continue;
        }

        sensor_file_count += 1;

        // Generate lookup keys for this file
        for key in generate_sensor_lookup_keys(file_name) {
            index.entry(key).or_default().push(entry_name.clone());
        }
    }

    progress.set_message(format!(
        "Indexed {} sensor files from ZIP",
        sensor_file_count
    ));
    log::info!(
        "Indexed {} sensor files from ZIP {}",
        index.len(),
        zip_path.display()
    );

    Ok(SensorFileIndex::Zip {
        archive_path: zip_path.to_path_buf(),
        index,
    })
}

#[cfg(feature = "polars")]
/// Check if a filename represents a valid sensor file (camera, lidar, radar, etc.)
fn is_valid_sensor_file(file_name: &str) -> bool {
    let name_lower = file_name.to_lowercase();

    // Camera images
    if name_lower.ends_with(".jpg")
        || name_lower.ends_with(".jpeg")
        || name_lower.ends_with(".png")
    {
        // Exclude sensor-specific PNGs (they're handled separately)
        if name_lower.ends_with(".radar.png") || name_lower.ends_with(".lidar.png") {
            return true; // Valid sensor file
        }
        return true; // Regular image
    }

    // LiDAR files
    if name_lower.ends_with(".lidar.pcd")
        || name_lower.ends_with(".lidar.png")
        || name_lower.ends_with(".lidar.jpg")
        || name_lower.ends_with(".lidar.jpeg")
        || name_lower.ends_with(".depth.png")
    {
        return true;
    }

    // Radar files
    if name_lower.ends_with(".radar.pcd") || name_lower.ends_with(".radar.png") {
        return true;
    }

    // PCD files (generic point cloud)
    if name_lower.ends_with(".pcd") {
        return true;
    }

    false
}

#[cfg(feature = "polars")]
/// Generate lookup keys for sensor files.
///
/// For a file like "sequence_A_001.lidar.pcd", generates:
/// - "sequence_A_001.lidar.pcd" (full name)
/// - "sequence_A_001.lidar" (without final extension)
/// - "sequence_A_001" (base sample name)
fn generate_sensor_lookup_keys(file_name: &str) -> Vec<String> {
    let mut keys = vec![file_name.to_string()];

    // Strip extensions progressively to build lookup keys
    let mut current = file_name.to_string();

    // First pass: remove the final extension
    if let Some((stem, _ext)) = current.rsplit_once('.') {
        keys.push(stem.to_string());
        current = stem.to_string();
    }

    // Second pass: handle sensor type suffixes (.camera, .lidar, .radar, .depth)
    for suffix in &[".camera", ".lidar", ".radar", ".depth"] {
        if let Some(stripped) = current.strip_suffix(suffix)
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
    progress: &indicatif::ProgressBar,
) -> Result<Vec<edgefirst_client::Sample>, Error> {
    if !images_path.is_dir() {
        return Err(Error::InvalidParameters(
            "ZIP file support not yet implemented".to_owned(),
        ));
    }

    progress.set_message(format!("Scanning: {}...", images_path.display()));

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

        // Update progress every 500 samples
        if samples.len() % 500 == 0 {
            progress.set_message(format!("Found {} images...", samples.len()));
        }
    }

    progress.set_message(format!("Found {} images", samples.len()));
    Ok(samples)
}

#[cfg(feature = "polars")]
#[cfg_attr(feature = "tracy", tracing::instrument(skip(client), fields(dataset_id = %dataset_id)))]
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

    #[cfg(feature = "tracy")]
    let _resolve_span = tracing::info_span!("resolve_annotation_set").entered();

    let dataset_id_parsed: edgefirst_client::DatasetID = dataset_id.clone().try_into()?;

    // Resolve annotation_set_id: use provided, find existing, or create new
    let annotation_set_id = if let Some(as_id) = annotation_set_id {
        Some(as_id)
    } else if annotations.is_some() {
        // Annotations provided but no annotation_set_id - auto-resolve
        let existing_sets = client.annotation_sets(dataset_id_parsed).await?;
        if let Some(first_set) = existing_sets.first() {
            let set_id = first_set.id().to_string();
            println!(
                "Using existing annotation set: {} ({})",
                first_set.name(),
                set_id
            );
            Some(set_id)
        } else {
            // Create new annotation set named "annotations"
            println!("Creating new annotation set: annotations");
            let new_id = client
                .create_annotation_set(dataset_id_parsed, "annotations", None)
                .await?;
            println!("Created annotation set: {}", new_id);
            Some(new_id.to_string())
        }
    } else {
        None
    };

    #[cfg(feature = "tracy")]
    drop(_resolve_span);

    // Warning: annotation_set_id provided but no annotations
    if annotation_set_id.is_some() && annotations.is_none() {
        eprintln!("⚠️  Warning: --annotation-set-id provided but no --annotations file.");
        eprintln!("   No annotations will be read or uploaded.");
        eprintln!("   Only images will be imported.");
    }

    // Determine images path
    let images_path = determine_images_path(&annotations, &images)?;

    #[cfg(feature = "tracy")]
    let _prep_span = tracing::info_span!("preparation").entered();

    // Create a spinner for the preparation phase
    let prep_bar = indicatif::ProgressBar::new_spinner();
    prep_bar.set_style(
        indicatif::ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    prep_bar.enable_steady_tick(std::time::Duration::from_millis(100));

    // Parse annotations from Arrow if provided, or build samples from directory
    let should_upload_annotations = annotations.is_some() && annotation_set_id.is_some();
    let mut samples = if annotations.is_some() {
        #[cfg(feature = "tracy")]
        let _arrow_span = tracing::info_span!("parse_arrow").entered();
        prep_bar.set_message("Reading Arrow file...");
        parse_annotations_from_arrow(&annotations, &images_path, should_upload_annotations, &prep_bar)?
    } else {
        #[cfg(feature = "tracy")]
        let _scan_span = tracing::info_span!("scan_directory").entered();
        prep_bar.set_message("Scanning directory for images...");
        build_samples_from_directory(&images_path, &prep_bar)?
    };
    prep_bar.finish_and_clear();

    #[cfg(feature = "tracy")]
    drop(_prep_span);

    if samples.is_empty() {
        return Err(Error::InvalidParameters(
            "No samples to upload. Check that images exist.".to_owned(),
        ));
    }

    // Generate UUIDs for upload (sample.uuid and sequence_uuid for sequences)
    // These are required by Studio but not persisted to Arrow/JSON files
    generate_upload_uuids(&mut samples, &dataset_id);

    let total_samples = samples.len();
    println!("Uploading {} samples to dataset {}...", total_samples, dataset_id);

    let bar = indicatif::ProgressBar::new(total_samples as u64);
    bar.set_style(
        indicatif::ProgressStyle::with_template(
            "[{elapsed_precise} ETA: {eta}] Uploading samples: {wide_bar:.yellow} {human_pos}/{human_len}"
        )
        .unwrap()
        .progress_chars("█▇▆▅▄▃▂▁  "),
    );

    // Track cumulative offset of completed samples from previous batches
    let offset = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let offset_for_task = offset.clone();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<edgefirst_client::Progress>(1);
    tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            if progress.total > 0 {
                // Add offset from previous batches to get cumulative progress
                let batch_offset = offset_for_task.load(std::sync::atomic::Ordering::SeqCst);
                bar.set_position((batch_offset + progress.current) as u64);
            }
        }
        bar.finish_with_message("Upload complete");
    });

    // Batch size of 50 chosen for retry resilience: if a batch fails, only 50 samples
    // need to be retried instead of 500. This adds ~10x more API calls but improves
    // reliability for large uploads over unreliable connections.
    const BATCH_SIZE: usize = 50;
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

    log::debug!(
        "Created {} batches (grouped by sequence+group to avoid server bugs)",
        batches.len()
    );

    #[cfg(feature = "tracy")]
    let _upload_span = tracing::info_span!(
        "upload",
        total_batches = batches.len(),
        total_samples = total_samples
    )
    .entered();

    for (_batch_num, batch) in batches.iter().enumerate() {
        #[cfg(feature = "tracy")]
        let batch_num = _batch_num; // Shadow with non-underscore name for Tracy

        let batch_size = batch.len();

        #[cfg(feature = "tracy")]
        let _batch_span =
            tracing::info_span!("batch", batch_num = batch_num + 1, size = batch_size).entered();

        let results = client
            .populate_samples(
                dataset_id_parsed,
                annotation_set_id_parsed,
                batch.clone(),
                Some(tx.clone()),
            )
            .await?;

        all_results.extend(results);

        // Update offset for next batch so progress continues from where we left off
        offset.fetch_add(batch_size, std::sync::atomic::Ordering::SeqCst);
    }

    #[cfg(feature = "tracy")]
    drop(_upload_span);

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
    /// EdgeFirst Dataset Format: paired .arrow and .zip files
    EdgeFirstFormat {
        arrow_path: PathBuf,
        zip_path: PathBuf,
    },
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
        // Check for EdgeFirst Dataset Format: .arrow file with paired .zip
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && ext.eq_ignore_ascii_case("arrow")
            && path.exists()
        {
            // Look for paired .zip file with same base name
            let zip_path = path.with_extension("zip");
            if zip_path.exists() {
                return Ok(SnapshotSource::EdgeFirstFormat {
                    arrow_path: path,
                    zip_path,
                });
            }
        }
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
        SnapshotSource::EdgeFirstFormat { arrow_path, .. } => {
            let name = arrow_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("edgefirst");
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
            let (tx, mut rx) = mpsc::channel::<Progress>(1);

            let bar = ProgressBar::new(0);
            bar.set_style(
                ProgressStyle::with_template(
                    "[{elapsed_precise} ETA: {eta}] {msg} {wide_bar:.yellow} {bytes}/{total_bytes}",
                )
                .unwrap()
                .progress_chars("█▇▆▅▄▃▂▁  "),
            );
            bar.set_message("Uploading");

            tokio::spawn(async move {
                while let Some(progress) = rx.recv().await {
                    let msg = progress.status.as_deref().unwrap_or("Uploading");
                    bar.set_message(msg.to_string());
                    if progress.total > 0 {
                        bar.set_length(progress.total as u64);
                        bar.set_position(progress.current as u64);
                    }
                }
                bar.finish_with_message("Done");
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
        SnapshotSource::EdgeFirstFormat {
            arrow_path,
            zip_path,
        } => {
            // EdgeFirst Dataset Format: upload paired .arrow and .zip files
            let (tx, mut rx) = mpsc::channel::<Progress>(1);

            let bar = ProgressBar::new(0);
            bar.set_style(
                ProgressStyle::with_template(
                    "[{elapsed_precise} ETA: {eta}] {msg} {wide_bar:.yellow} {bytes}/{total_bytes}",
                )
                .unwrap()
                .progress_chars("█▇▆▅▄▃▂▁  "),
            );
            bar.set_message("Uploading");

            tokio::spawn(async move {
                while let Some(progress) = rx.recv().await {
                    let msg = progress.status.as_deref().unwrap_or("Uploading");
                    bar.set_message(msg.to_string());
                    if progress.total > 0 {
                        bar.set_length(progress.total as u64);
                        bar.set_position(progress.current as u64);
                    }
                }
                bar.finish_with_message("Done");
            });

            let arrow_str = arrow_path.to_str().ok_or_else(|| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Arrow path contains invalid UTF-8",
                ))
            })?;
            let zip_str = zip_path.to_str().ok_or_else(|| {
                Error::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "ZIP path contains invalid UTF-8",
                ))
            })?;

            println!(
                "Uploading EdgeFirst Dataset Format:\n  Arrow: {}\n  ZIP: {}",
                arrow_path.display(),
                zip_path.display()
            );

            let snapshot = client
                .create_snapshot_edgefirst_format(
                    arrow_str,
                    zip_str,
                    Some(&description),
                    Some(tx),
                )
                .await?;
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
        while let Some(Progress { current, total, .. }) = rx.recv().await {
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
    use edgefirst_client::coco::{CocoToArrowOptions, coco_to_arrow};
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
    use edgefirst_client::coco::{ArrowToCocoOptions, CocoInfo, arrow_to_coco};
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

/// Arguments for COCO import CLI command.
/// Groups all parameters to reduce function parameter count.
struct CocoCliImportArgs {
    /// Path to COCO annotation JSON file or extracted directory.
    coco_path: PathBuf,
    /// Project ID (required when creating new dataset).
    project: Option<String>,
    /// Create new dataset with this name.
    name: Option<String>,
    /// Description for new dataset.
    description: Option<String>,
    /// Target dataset ID.
    dataset: Option<String>,
    /// Target annotation set ID.
    annotation_set: Option<String>,
    /// Group name for samples.
    group: Option<String>,
    /// Include segmentation masks.
    masks: bool,
    /// Include images in upload.
    images: bool,
    /// Batch size for uploads.
    batch_size: usize,
    /// Maximum concurrent uploads.
    concurrency: usize,
    /// Verify import instead of uploading.
    verify: bool,
    /// Update annotations on existing samples.
    update: bool,
}

/// Context for COCO import operations after resolving IDs.
struct CocoImportContext {
    coco_path: PathBuf,
    dataset_id: DatasetID,
    annotation_set_id: AnnotationSetID,
    group: Option<String>,
    masks: bool,
    images: bool,
    batch_size: usize,
    concurrency: usize,
}

/// Validate COCO import parameters.
fn validate_coco_import_params(args: &CocoCliImportArgs) -> Result<(), Error> {
    if args.verify && args.name.is_some() {
        return Err(Error::InvalidParameters(
            "--verify cannot be used with --name (cannot verify a dataset that doesn't exist yet)."
                .to_owned(),
        ));
    }

    if args.update && args.name.is_some() {
        return Err(Error::InvalidParameters(
            "--update cannot be used with --name (cannot update a dataset that doesn't exist yet)."
                .to_owned(),
        ));
    }

    if args.verify && args.update {
        return Err(Error::InvalidParameters(
            "--verify and --update cannot be used together.".to_owned(),
        ));
    }

    Ok(())
}

/// Resolve dataset and annotation set IDs from command line arguments.
async fn resolve_dataset_ids(
    client: &Client,
    name: Option<String>,
    project: Option<String>,
    description: Option<String>,
    dataset: Option<String>,
    annotation_set: Option<String>,
) -> Result<(DatasetID, AnnotationSetID), Error> {
    match (name, dataset) {
        (Some(dataset_name), None) => {
            create_new_dataset_with_annotation_set(client, &dataset_name, project, description)
                .await
        }
        (None, Some(ds_id)) => {
            resolve_existing_dataset(client, &ds_id, annotation_set).await
        }
        (Some(_), Some(_)) => Err(Error::InvalidParameters(
            "Cannot specify both --name and --dataset. Use --name to create a new dataset or --dataset to use an existing one.".to_owned()
        )),
        (None, None) => Err(Error::InvalidParameters(
            "Must specify either --name (to create a new dataset) or --dataset (to use an existing one).".to_owned()
        )),
    }
}

/// Create a new dataset with a default annotation set.
async fn create_new_dataset_with_annotation_set(
    client: &Client,
    dataset_name: &str,
    project: Option<String>,
    description: Option<String>,
) -> Result<(DatasetID, AnnotationSetID), Error> {
    let project_id = project.ok_or_else(|| {
        Error::InvalidParameters(
            "--project is required when creating a new dataset with --name".to_owned(),
        )
    })?;

    println!("Creating new dataset '{}'...", dataset_name);
    let ds_id = client
        .create_dataset(&project_id, dataset_name, description.as_deref())
        .await?;
    println!("  Created dataset: {}", ds_id);

    let ann_set_name = "annotations";
    println!("Creating annotation set '{}'...", ann_set_name);
    let ann_set_id = client
        .create_annotation_set(ds_id, ann_set_name, None)
        .await?;
    println!("  Created annotation set: {}", ann_set_id);

    Ok((ds_id, ann_set_id))
}

/// Resolve an existing dataset and its annotation set.
async fn resolve_existing_dataset(
    client: &Client,
    ds_id: &str,
    annotation_set: Option<String>,
) -> Result<(DatasetID, AnnotationSetID), Error> {
    let dataset_id: DatasetID = ds_id.to_string().try_into()?;

    let annotation_set_id = if let Some(as_id) = annotation_set {
        as_id.try_into()?
    } else {
        let ann_sets = client.annotation_sets(dataset_id).await?;
        if ann_sets.is_empty() {
            return Err(Error::InvalidParameters(
                "Dataset has no annotation sets. Create one first or use --name to create a new dataset.".to_owned()
            ));
        }
        println!("  Using annotation set: {}", ann_sets[0].id());
        ann_sets[0].id()
    };

    Ok((dataset_id, annotation_set_id))
}

/// Create a progress bar with standard styling.
fn create_progress_bar() -> indicatif::ProgressBar {
    use indicatif::{ProgressBar, ProgressStyle};

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    pb
}

/// Print COCO operation header.
fn print_coco_header(
    operation: &str,
    coco_path: &Path,
    dataset_id: &DatasetID,
    annotation_set_id: &AnnotationSetID,
    group: &Option<String>,
) {
    println!("{}...", operation);
    println!("  Source:         {:?}", coco_path);
    println!("  Dataset:        {}", dataset_id);
    println!("  Annotation Set: {}", annotation_set_id);
    if let Some(g) = group {
        println!("  Group:          {}", g);
    }
}

/// Handle COCO import to Studio.
async fn handle_import_coco(client: &Client, args: CocoCliImportArgs) -> Result<(), Error> {
    validate_coco_import_params(&args)?;

    let (dataset_id, annotation_set_id) = resolve_dataset_ids(
        client,
        args.name,
        args.project,
        args.description,
        args.dataset,
        args.annotation_set,
    )
    .await?;

    let ctx = CocoImportContext {
        coco_path: args.coco_path,
        dataset_id,
        annotation_set_id,
        group: args.group,
        masks: args.masks,
        images: args.images,
        batch_size: args.batch_size,
        concurrency: args.concurrency,
    };

    if args.verify {
        handle_coco_verify(client, &ctx).await
    } else if args.update {
        handle_coco_update(client, &ctx).await
    } else {
        handle_coco_import_normal(client, &ctx).await
    }
}

/// Handle COCO verify mode.
async fn handle_coco_verify(client: &Client, ctx: &CocoImportContext) -> Result<(), Error> {
    use edgefirst_client::coco::studio::{CocoVerifyOptions, verify_coco_import};

    print_coco_header(
        "Verifying COCO import",
        &ctx.coco_path,
        &ctx.dataset_id,
        &ctx.annotation_set_id,
        &ctx.group,
    );

    let pb = create_progress_bar();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Progress>(100);

    let verify_options = CocoVerifyOptions {
        verify_masks: ctx.masks,
        group: ctx.group.clone(),
    };

    let coco_path_owned = ctx.coco_path.clone();
    let dataset_id = ctx.dataset_id;
    let annotation_set_id = ctx.annotation_set_id;
    let client = client.clone();
    let task = tokio::spawn(async move {
        verify_coco_import(
            &client,
            &coco_path_owned,
            dataset_id,
            annotation_set_id,
            &verify_options,
            Some(tx),
        )
        .await
    });

    while let Some(progress) = rx.recv().await {
        pb.set_length(progress.total as u64);
        pb.set_position(progress.current as u64);
    }

    let result = task.await??;
    pb.finish_and_clear();

    println!("\n{}", result);

    if result.is_valid() {
        println!("Verification passed!");
        Ok(())
    } else {
        Err(Error::InvalidParameters(
            "Verification failed. See details above.".to_owned(),
        ))
    }
}

/// Handle COCO update mode.
async fn handle_coco_update(client: &Client, ctx: &CocoImportContext) -> Result<(), Error> {
    use edgefirst_client::coco::studio::{CocoUpdateOptions, update_coco_annotations};

    print_coco_header(
        "Updating annotations on existing samples",
        &ctx.coco_path,
        &ctx.dataset_id,
        &ctx.annotation_set_id,
        &ctx.group,
    );

    let pb = create_progress_bar();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Progress>(100);

    let update_options = CocoUpdateOptions {
        include_masks: ctx.masks,
        group: ctx.group.clone(),
        batch_size: ctx.batch_size,
        concurrency: ctx.concurrency,
    };

    let coco_path_owned = ctx.coco_path.clone();
    let dataset_id = ctx.dataset_id;
    let annotation_set_id = ctx.annotation_set_id;
    let client = client.clone();
    let task = tokio::spawn(async move {
        update_coco_annotations(
            &client,
            &coco_path_owned,
            dataset_id,
            annotation_set_id,
            &update_options,
            Some(tx),
        )
        .await
    });

    while let Some(progress) = rx.recv().await {
        pb.set_length(progress.total as u64);
        pb.set_position(progress.current as u64);
    }

    let result = task.await??;
    pb.finish_with_message("done");

    println!(
        "\n✓ Update complete: {} updated, {} not found in Studio, {} total",
        result.updated, result.not_found, result.total_images
    );

    if result.not_found > 0 {
        println!(
            "  Note: {} samples from COCO were not found in Studio. Run import without --update to add them.",
            result.not_found
        );
    }

    Ok(())
}

/// Handle normal COCO import mode.
async fn handle_coco_import_normal(client: &Client, ctx: &CocoImportContext) -> Result<(), Error> {
    use edgefirst_client::coco::studio::{CocoImportOptions, import_coco_to_studio};

    print_coco_header(
        "Importing COCO dataset to Studio",
        &ctx.coco_path,
        &ctx.dataset_id,
        &ctx.annotation_set_id,
        &ctx.group,
    );

    let pb = create_progress_bar();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Progress>(100);

    let options = CocoImportOptions {
        include_masks: ctx.masks,
        include_images: ctx.images,
        group: ctx.group.clone(),
        batch_size: ctx.batch_size,
        concurrency: ctx.concurrency,
        resume: true,
    };

    let coco_path_owned = ctx.coco_path.clone();
    let dataset_id = ctx.dataset_id;
    let annotation_set_id = ctx.annotation_set_id;
    let client = client.clone();
    let task = tokio::spawn(async move {
        import_coco_to_studio(
            &client,
            &coco_path_owned,
            dataset_id,
            annotation_set_id,
            &options,
            Some(tx),
        )
        .await
    });

    while let Some(progress) = rx.recv().await {
        pb.set_length(progress.total as u64);
        pb.set_position(progress.current as u64);
    }

    let result = task.await??;
    pb.finish_with_message("done");

    if result.skipped > 0 {
        println!(
            "\n✓ Import complete: {} imported, {} skipped (already existed), {} total",
            result.imported, result.skipped, result.total_images
        );
    } else {
        println!("\n✓ Imported {} samples to Studio", result.imported);
    }

    Ok(())
}

/// Handle Studio export to COCO.
async fn handle_export_coco(
    client: &Client,
    dataset_id: String,
    annotation_set_id: String,
    output: PathBuf,
    groups: Vec<String>,
    masks: bool,
    images: bool,
    pretty: bool,
) -> Result<(), Error> {
    use chrono::Datelike;
    use edgefirst_client::coco::{
        CocoInfo,
        studio::{CocoExportOptions, export_studio_to_coco},
    };
    use indicatif::{ProgressBar, ProgressStyle};

    let output_zip = output.extension().map(|e| e == "zip").unwrap_or(false);

    println!("Exporting Studio dataset to COCO format...");
    println!("  Dataset:        {}", dataset_id);
    println!("  Annotation Set: {}", annotation_set_id);
    println!("  Output:         {:?}", output);
    if !groups.is_empty() {
        println!("  Groups:         {}", groups.join(", "));
    }

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Progress>(100);

    let options = CocoExportOptions {
        groups,
        include_masks: masks,
        include_images: images,
        output_zip,
        pretty_json: pretty,
        info: Some(CocoInfo {
            description: Some("Exported from EdgeFirst Studio".to_string()),
            version: Some("1.0".to_string()),
            year: Some(chrono::Utc::now().year() as u32),
            ..Default::default()
        }),
    };

    let dataset_id: DatasetID = dataset_id.try_into()?;
    let annotation_set_id: AnnotationSetID = annotation_set_id.try_into()?;

    let output_clone = output.clone();
    let client = client.clone();
    let task = tokio::spawn(async move {
        export_studio_to_coco(
            &client,
            dataset_id,
            annotation_set_id,
            &output_clone,
            &options,
            Some(tx),
        )
        .await
    });

    while let Some(progress) = rx.recv().await {
        pb.set_length(progress.total as u64);
        pb.set_position(progress.current as u64);
    }

    let count = task.await??;
    pb.finish_with_message("done");

    println!("\n✓ Exported {} annotations to COCO format", count);

    Ok(())
}

/// Initialize logging/tracing based on feature flags and runtime configuration.
///
/// When the `tracy` feature is enabled:
/// - Uses `tracing` crate with a fmt layer for console output
/// - Adds TracyLayer when `TRACY_ENABLE=1` environment variable is set
/// - Adds ChromeLayer when `--trace-file` argument or `TRACE_FILE` env is set
///
/// When `profiling` feature is disabled:
/// - Falls back to standard `env_logger` for minimal overhead
///
/// Returns a guard that must be held until program exit to ensure trace files are flushed.
#[cfg(all(feature = "profiling", feature = "trace-file"))]
fn init_tracing(args: &Args) -> Option<tracing_chrome::FlushGuard> {
    use tracing_subscriber::prelude::*;

    // Determine log level from verbosity flag
    let default_level = match args.verbose {
        0 => tracing::level_filters::LevelFilter::INFO,
        1 => tracing::level_filters::LevelFilter::DEBUG,
        _ => tracing::level_filters::LevelFilter::TRACE,
    };

    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(default_level.into())
        .from_env_lossy();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_filter(filter);

    let mut chrome_guard = None;

    // Check for Tracy profiling
    #[cfg(feature = "tracy")]
    let tracy_enabled = std::env::var("TRACY_ENABLE")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);
    #[cfg(not(feature = "tracy"))]
    let tracy_enabled = false;

    // Check for trace file output
    let trace_path = args.trace_file.clone();

    // Build subscriber with enabled layers
    match (tracy_enabled, trace_path) {
        #[cfg(feature = "tracy")]
        (true, Some(path)) => {
            // Both Tracy and trace file enabled
            let _client = tracy_client::Client::start();
            let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
                .file(path.clone())
                .include_args(true)
                .build();
            chrome_guard = Some(guard);

            tracing_subscriber::registry()
                .with(fmt_layer)
                .with(tracing_tracy::TracyLayer::default())
                .with(chrome_layer)
                .init();

            eprintln!("Tracy profiling enabled");
            eprintln!("Trace output: {}", path.display());
        }
        #[cfg(feature = "tracy")]
        (true, None) => {
            // Tracy only
            let _client = tracy_client::Client::start();
            tracing_subscriber::registry()
                .with(fmt_layer)
                .with(tracing_tracy::TracyLayer::default())
                .init();

            eprintln!("Tracy profiling enabled");
        }
        (false, Some(path)) => {
            // Trace file only
            let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
                .file(path.clone())
                .include_args(true)
                .build();
            chrome_guard = Some(guard);

            tracing_subscriber::registry()
                .with(fmt_layer)
                .with(chrome_layer)
                .init();

            eprintln!("Trace output: {}", path.display());
        }
        _ => {
            // No profiling backends, just fmt layer
            tracing_subscriber::registry().with(fmt_layer).init();
        }
    }

    chrome_guard
}

/// Initialize tracing without trace-file support (profiling only, no trace output).
#[cfg(all(feature = "profiling", not(feature = "trace-file")))]
fn init_tracing(args: &Args) -> Option<()> {
    use tracing_subscriber::prelude::*;

    // Determine log level from verbosity flag
    let default_level = match args.verbose {
        0 => tracing::level_filters::LevelFilter::INFO,
        1 => tracing::level_filters::LevelFilter::DEBUG,
        _ => tracing::level_filters::LevelFilter::TRACE,
    };

    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(default_level.into())
        .from_env_lossy();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_filter(filter);

    // Check for Tracy profiling
    #[cfg(feature = "tracy")]
    let tracy_enabled = std::env::var("TRACY_ENABLE")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);
    #[cfg(not(feature = "tracy"))]
    let tracy_enabled = false;

    if tracy_enabled {
        #[cfg(feature = "tracy")]
        {
            let _client = tracy_client::Client::start();
            tracing_subscriber::registry()
                .with(fmt_layer)
                .with(tracing_tracy::TracyLayer::default())
                .init();
            eprintln!("Tracy profiling enabled");
        }
    } else {
        tracing_subscriber::registry().with(fmt_layer).init();
    }

    None
}

#[cfg(not(feature = "profiling"))]
fn init_tracing(args: &Args) -> Option<()> {
    let log_level = match args.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    None
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Parse arguments first so verbosity and trace file can be configured
    let args = Args::parse();

    // Initialize tracing/logging after parsing args
    // Keep the guard alive until program exit to ensure trace files are flushed
    let _trace_guard = init_tracing(&args);

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

    // Handle local file commands - no authentication needed
    match &args.cmd {
        Command::CocoToArrow {
            coco_path,
            output,
            masks,
            group,
        } => {
            return handle_coco_to_arrow(coco_path.clone(), output.clone(), *masks, group.clone())
                .await;
        }
        Command::ArrowToCoco {
            arrow_path,
            output,
            masks,
            groups,
            pretty,
        } => {
            return handle_arrow_to_coco(
                arrow_path.clone(),
                output.clone(),
                *masks,
                groups.clone(),
                *pretty,
            )
            .await;
        }
        Command::GenerateArrow {
            folder,
            output,
            detect_sequences,
        } => {
            return handle_generate_arrow(folder.clone(), output.clone(), *detect_sequences);
        }
        Command::ValidateSnapshot { path, verbose } => {
            return handle_validate_snapshot(path.clone(), *verbose);
        }
        _ => {}
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
            groups,
        } => handle_dataset(&client, dataset_id, annotation_sets, labels, groups).await,
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
            list_types,
        } => {
            if list_types {
                println!("Valid sensor types:");
                for type_name in FileType::type_names() {
                    let description = match type_name {
                        "image" => "Standard image files (JPEG, PNG, etc.)",
                        "lidar.pcd" => "LiDAR point cloud data files (.pcd format)",
                        "lidar.png" => "LiDAR depth images (.png format)",
                        "lidar.jpg" => "LiDAR reflectance images (.jpg format)",
                        "radar.pcd" => "Radar point cloud data files (.pcd format)",
                        "radar.png" => "Radar cube data files (.png format)",
                        "all" => "All sensor types (expands to all of the above)",
                        _ => "",
                    };
                    println!("  {:<12} - {}", type_name, description);
                }
                return Ok(());
            }
            let dataset_id = dataset_id.ok_or_else(|| {
                Error::InvalidParameters("Dataset ID is required for download".to_string())
            })?;
            let output = output.unwrap_or_else(|| ".".into());
            let expanded_types = FileType::expand_types(&types);
            handle_download_dataset(&client, dataset_id, groups, expanded_types, output, flatten)
                .await
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
        // These are handled early without authentication
        Command::GenerateArrow { .. } => unreachable!(),
        Command::ValidateSnapshot { .. } => unreachable!(),
        Command::CocoToArrow { .. } => unreachable!(),
        Command::ArrowToCoco { .. } => unreachable!(),
        Command::ImportCoco {
            coco_path,
            project,
            name,
            description,
            dataset,
            annotation_set,
            group,
            masks,
            images,
            batch_size,
            concurrency,
            verify,
            update,
        } => {
            let args = CocoCliImportArgs {
                coco_path,
                project,
                name,
                description,
                dataset,
                annotation_set,
                group,
                masks,
                images,
                batch_size,
                concurrency,
                verify,
                update,
            };
            handle_import_coco(&client, args).await
        }
        Command::ExportCoco {
            dataset_id,
            annotation_set_id,
            output,
            groups,
            masks,
            images,
            pretty,
        } => {
            handle_export_coco(
                &client,
                dataset_id,
                annotation_set_id,
                output,
                groups,
                masks,
                images,
                pretty,
            )
            .await
        }
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
            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&Some(arrow_file.clone()), &images_dir, true, &progress);
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

            let progress = indicatif::ProgressBar::hidden();
            let samples =
                parse_annotations_from_arrow(&Some(arrow_file.clone()), &images_dir, true, &progress)
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

            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true, &progress);
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
            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, false, &progress);
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

            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, false, &progress);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("No image found for sample")
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

            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, false, &progress);
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

            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&None, &images_dir, false, &progress);
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

            let progress = indicatif::ProgressBar::hidden();
            let err = build_sensor_file_index(zip_file.as_path(), &progress).unwrap_err();
            // Empty file is not a valid ZIP archive
            assert!(
                err.to_string().contains("Failed to read ZIP archive"),
                "Expected ZIP read error, got: {}",
                err
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
            let progress = indicatif::ProgressBar::hidden();
            let sensor_index = build_sensor_file_index(test_dir.as_path(), &progress).unwrap();
            let result = sensor_index.find_image_entry("image1");
            assert!(result.is_ok());
            // Verify we got the right filename
            let (path, filename) = result.unwrap();
            assert!(
                path.ends_with("image1.camera.jpg"),
                "Expected path to end with 'image1.camera.jpg', got: {}",
                path
            );
            assert_eq!(filename, "image1.camera.jpg");

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

            let progress = indicatif::ProgressBar::hidden();
            let annotations = Some(arrow_file.clone());
            let samples = parse_annotations_from_arrow(&annotations, &images_dir, false, &progress)
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

            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true, &progress);
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

            let progress = indicatif::ProgressBar::hidden();
            let annotations = Some(arrow_file.clone());
            let samples = parse_annotations_from_arrow(&annotations, &images_dir, false, &progress)
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

            let progress = indicatif::ProgressBar::hidden();
            let annotations = Some(arrow_file.clone());
            let samples = parse_annotations_from_arrow(&annotations, &images_dir, false, &progress)
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

            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true, &progress);
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

            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true, &progress);
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

            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true, &progress);
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

            let progress = indicatif::ProgressBar::hidden();
            let result = parse_annotations_from_arrow(&Some(arrow_file), &images_dir, true, &progress);
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
