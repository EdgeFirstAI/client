// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use clap::{Parser, Subcommand};
use edgefirst_client::{
    AnnotationType, Client, Dataset, Error, FileType, Progress, TrainingSession,
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
    Task { task_id: String },
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

            // Get or create entry for this sample
            let entry = samples_map
                .entry(sample_name.clone())
                .or_insert(SampleMetadata {
                    group: sample_group,
                    sequence_name: sequence_name.clone(),
                    frame_number,
                    annotations: Vec::new(),
                });

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
    // Format: {sequence_name}_{frame}.ext
    // This is a best-effort heuristic with known limitations:
    // - Requires at least 3 parts when split by underscore to reduce false positives
    // - Standalone images like "image_42.jpg" won't be detected as sequences (only 2 parts)
    // - Sequence names must contain at least one underscore (e.g., "seq_a_001.jpg" → sequence="seq_a")
    // - Simple sequences like "deer_042.jpg" won't be detected (only 2 parts)
    if let Some(filename) = image_path.file_stem()
        && let Some(name_str) = filename.to_str()
    {
        // Strip .camera suffix if present (special extension marker used by Studio)
        let base_name = name_str.strip_suffix(".camera").unwrap_or(name_str);
        
        // Split on underscores to find pattern: {sequence}_{frame}
        let parts: Vec<&str> = base_name.split('_').collect();
        
        // Require at least 3 parts to reduce false positives
        // (e.g., "image_42.jpg" has 2 parts and won't match,
        //  but "sequence_name_042.jpg" has 3 parts and will)
        if parts.len() >= 3 {
            // Check if the last part is numeric (the frame number)
            if parts.last().unwrap().parse::<u32>().is_ok() {
                // Join all parts except the last to get sequence name
                return Some(parts[..parts.len() - 1].join("_"));
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

async fn handle_task(client: &Client, task_id: String) -> Result<(), Error> {
    let info = client.task_info(task_id.try_into()?).await?;
    println!("{:?}", info);
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let client = Client::new()?.with_token_path(None)?;
    let client = match args.server {
        Some(server) => client.with_server(&server)?,
        None => client,
    };

    let client = match (&args.username, &args.password) {
        (Some(username), Some(password)) => client.with_login(username, password).await?,
        _ => match &args.token {
            Some(token) => client.with_token(token)?,
            _ => client,
        },
    };

    // Handle commands that don't need token renewal
    match &args.cmd {
        Command::Version => return handle_version(&client).await,
        Command::Login => {
            return handle_login(client, args.username, args.password).await;
        }
        Command::Logout => return handle_logout(&client).await,
        Command::Sleep { seconds } => return handle_sleep(*seconds).await,
        _ => {}
    }

    // Renew token for all other commands
    client.renew_token().await?;

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
        Command::Task { task_id } => handle_task(&client, task_id).await,
        Command::ValidationSessions { project_id } => {
            handle_validation_sessions(&client, project_id).await
        }
        Command::ValidationSession { session_id } => {
            handle_validation_session(&client, session_id).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            box2d_data: Option<Vec<Option<(f64, f64, f64, f64)>>>,
            mask_data: Option<Vec<Option<Vec<(f32, f32)>>>>,
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
            box2d_data: Option<Vec<Option<(f64, f64, f64, f64)>>>,
            mask_data: Option<Vec<Option<Vec<(f32, f32)>>>>,
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
    }
}
