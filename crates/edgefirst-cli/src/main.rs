// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use clap::{Parser, Subcommand};
use edgefirst_client::{
    AnnotationType, Client, Dataset, Error, FileType, Progress, TrainingSession,
};
use inquire::{Password, PasswordDisplayMode};
use std::{fs::File, path::PathBuf};

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
        dataset.uid(),
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
                annotation_set.uid(),
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
                    dataset.uid(),
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
        dataset.uid(),
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
                annotation_set.uid(),
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
        .download_dataset(dataset_id.try_into()?, &groups, &types, output, Some(tx))
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

    let annotations = client
        .annotations(annotation_set_id.try_into()?, &groups, &types, Some(tx))
        .await?;

    let format = output
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());

    match format {
        Some(ext) if ext == "json" => {
            let mut file = File::create(&output)?;
            file.write_all(serde_json::to_string_pretty(&annotations)?.as_bytes())?;
        }
        Some(ext) if ext == "arrow" => {
            #[cfg(feature = "polars")]
            {
                use polars::{io::SerWriter as _, prelude::IpcWriter};

                let mut df = edgefirst_client::annotations_dataframe(&annotations)?;
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
fn find_image_source(arrow_path: &PathBuf) -> Result<PathBuf, Error> {
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

    // Convert to array type
    let array_chunked = match box2d_col.array() {
        Ok(arr) => arr,
        Err(_) => return Ok(None),
    };

    // Get the series at the specified index
    let array_series = match array_chunked.get_as_series(idx) {
        Some(series) => series,
        None => return Ok(None),
    };

    // Extract f32 values
    let values = match array_series.f32() {
        Ok(vals) => vals,
        Err(_) => return Ok(None),
    };

    let coords: Vec<f32> = values.into_iter().flatten().collect();
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

    // Convert to array type
    let array_chunked = match box3d_col.array() {
        Ok(arr) => arr,
        Err(_) => return Ok(None),
    };

    // Get the series at the specified index
    let array_series = match array_chunked.get_as_series(idx) {
        Some(series) => series,
        None => return Ok(None),
    };

    // Extract f32 values
    let values = match array_series.f32() {
        Ok(vals) => vals,
        Err(_) => return Ok(None),
    };

    let coords: Vec<f32> = values.into_iter().flatten().collect();
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

    // Extract f32 values
    let values = match mask_series.f32() {
        Ok(vals) => vals,
        Err(_) => return Ok(None),
    };

    let coords: Vec<f32> = values.into_iter().flatten().collect();
    if !coords.is_empty() && coords.len().is_multiple_of(2) {
        let mut polygons: Vec<Vec<(f32, f32)>> = Vec::new();
        let mut current_polygon: Vec<(f32, f32)> = Vec::new();

        // Parse coordinate pairs, using NaN as polygon separator
        let mut i = 0;
        while i < coords.len() {
            let x = coords[i];
            let y = coords[i + 1];

            if x.is_nan() || y.is_nan() {
                // NaN signals end of current polygon
                if !current_polygon.is_empty() {
                    polygons.push(current_polygon.clone());
                    current_polygon.clear();
                }
            } else {
                current_polygon.push((x, y));
            }
            i += 2;
        }

        // Add final polygon if not empty
        if !current_polygon.is_empty() {
            polygons.push(current_polygon);
        }

        if !polygons.is_empty() {
            let mask = edgefirst_client::Mask::new(polygons);
            return Ok(Some(mask));
        }
    }

    Ok(None)
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
/// HashMap mapping sample names to (group, annotations) tuples. If
/// `should_upload_annotations` is false, annotation vectors will be empty.
#[cfg(feature = "polars")]
fn parse_annotations_from_arrow(
    annotations: &Option<PathBuf>,
    should_upload_annotations: bool,
) -> Result<
    std::collections::HashMap<String, (Option<String>, Vec<edgefirst_client::Annotation>)>,
    Error,
> {
    use polars::prelude::*;
    use std::{collections::HashMap, fs::File};

    let mut samples_map: HashMap<String, (Option<String>, Vec<edgefirst_client::Annotation>)> =
        HashMap::new();

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

            // Extract optional group designation (train/val/test)
            let sample_group = df
                .column("group")
                .ok()
                .and_then(|c| c.str().ok())
                .and_then(|s| s.get(idx))
                .map(|s| s.to_string());

            // Get or create entry for this sample
            let entry = samples_map
                .entry(name.clone())
                .or_insert((sample_group, Vec::new()));

            if should_upload_annotations {
                let mut has_annotation = false;
                let mut geometry_count = 0;
                let mut annotation = edgefirst_client::Annotation::new();

                // Extract label if present and non-empty
                if let Some(label) = df
                    .column("label")
                    .ok()
                    .and_then(|c| c.str().ok())
                    .and_then(|s| s.get(idx))
                    .map(|s| s.to_string())
                    && !label.is_empty()
                {
                    annotation.set_label(Some(label));
                    has_annotation = true;
                }

                // Extract object_id if present and non-empty
                let object_id = df
                    .column("object_id")
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

                // Auto-generate object_id for multi-geometry annotations to enable tracking
                if geometry_count > 1 && object_id.is_none() {
                    let generated_uuid = uuid::Uuid::new_v4().to_string();
                    annotation.set_object_id(Some(generated_uuid));
                }

                // Only add annotation if it has at least one geometry or label
                if has_annotation {
                    entry.1.push(annotation);
                }
            }
        }
    }

    Ok(samples_map)
}

#[cfg(feature = "polars")]
fn find_image_path_for_sample(images_path: &PathBuf, image_name: &str) -> Result<PathBuf, Error> {
    if images_path.is_dir() {
        let entries = std::fs::read_dir(images_path)?;
        let mut found_path = None;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                && file_name.starts_with(image_name)
                && (file_name == image_name
                    || file_name
                        .strip_prefix(image_name)
                        .map(|s| s.starts_with('.'))
                        .unwrap_or(false))
            {
                found_path = Some(path);
                break;
            }
        }

        found_path.ok_or_else(|| {
            Error::InvalidParameters(format!("Image file not found for sample: {}", image_name))
        })
    } else {
        Err(Error::InvalidParameters(
            "ZIP file support not yet implemented".to_owned(),
        ))
    }
}

#[cfg(feature = "polars")]
fn build_samples_from_map(
    samples_map: std::collections::HashMap<
        String,
        (Option<String>, Vec<edgefirst_client::Annotation>),
    >,
    images_path: &PathBuf,
) -> Result<Vec<edgefirst_client::Sample>, Error> {
    let mut samples = Vec::new();

    for (image_name, (sample_group, annotations)) in samples_map {
        let image_path = find_image_path_for_sample(images_path, &image_name)?;

        let image_file = edgefirst_client::SampleFile::with_filename(
            "image".to_string(),
            image_path.to_str().unwrap().to_string(),
        );

        let sample = edgefirst_client::Sample {
            image_name: Some(image_name.clone()),
            group: sample_group,
            files: vec![image_file],
            annotations,
            ..Default::default()
        };

        samples.push(sample);
    }

    Ok(samples)
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
    let mut samples = Vec::new();

    if !images_path.is_dir() {
        return Err(Error::InvalidParameters(
            "ZIP file support not yet implemented".to_owned(),
        ));
    }

    let entries = std::fs::read_dir(images_path)?;

    for entry in entries {
        let entry = entry?;
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
            group: None,
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

    // Parse annotations from Arrow if provided
    let should_upload_annotations = annotations.is_some() && annotation_set_id.is_some();
    let samples_map = parse_annotations_from_arrow(&annotations, should_upload_annotations)?;

    // Build samples
    let samples = if !samples_map.is_empty() {
        build_samples_from_map(samples_map, &images_path)?
    } else {
        build_samples_from_directory(&images_path)?
    };

    if samples.is_empty() {
        return Err(Error::InvalidParameters(
            "No samples to upload. Check that images exist.".to_owned(),
        ));
    }

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

    for (batch_num, batch) in samples.chunks(BATCH_SIZE).enumerate() {
        if samples.len() > BATCH_SIZE {
            println!(
                "Uploading batch {}/{} ({} samples)...",
                batch_num + 1,
                samples.len().div_ceil(BATCH_SIZE),
                batch.len()
            );
        }

        let results = client
            .populate_samples(
                dataset_id_parsed,
                annotation_set_id_parsed,
                batch.to_vec(),
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
                experiment.uid(),
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
        experiment.uid(),
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
        session.uid(),
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
        session.uid(),
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
        _ => {}
    }

    // Renew token for all other commands
    client.renew_token().await?;

    // Handle all other commands
    match args.cmd {
        Command::Version | Command::Login | Command::Logout => unreachable!(),
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
        } => {
            let output = output.unwrap_or_else(|| ".".into());
            handle_download_dataset(&client, dataset_id, groups, types, output).await
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
