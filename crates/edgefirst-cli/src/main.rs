// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use clap::{Parser, Subcommand};
use edgefirst_client::{AnnotationType, Client, Error, FileType, Progress};
use indicatif::ProgressStyle;
use inquire::{Password, PasswordDisplayMode};
use std::{fs::File, io::Write as _, path::PathBuf};
use tokio::sync::mpsc;

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

    if args.cmd == Command::Version {
        let version = client.version().await?;
        println!(
            "EdgeFirst Studio Server [{}]: {} Client: {}",
            client.url(),
            version,
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    } else if args.cmd == Command::Login {
        let (username, password) = match (args.username, args.password) {
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

        return Ok(());
    } else if args.cmd == Command::Logout {
        client.logout().await?;
        println!("Successfully logged out of EdgeFirst Studio");
        return Ok(());
    }

    client.renew_token().await?;

    match args.cmd {
        Command::Version => (), // Already handled above
        Command::Login => (),   // Already handled above
        Command::Logout => (),  // Already handled above
        Command::Token => {
            let token = client.token().await;
            println!("{}", token);
        }
        Command::Organization => {
            let org = client.organization().await?;
            println!(
                "Username: {}\nOrganization: {}\nID: {}\nCredits: {}",
                client.username().await?,
                org.name(),
                org.id(),
                org.credits()
            );
        }
        Command::Projects { name } => {
            let projects = client.projects(name.as_deref()).await?;
            for project in projects {
                println!(
                    "[{}] {}: {}",
                    project.id(),
                    project.name(),
                    project.description()
                );
            }
        }
        Command::Project { project_id } => {
            let project = client.project(project_id.try_into()?).await?;
            println!(
                "[{}] {}: {}",
                project.id(),
                project.name(),
                project.description()
            );
        }
        Command::Datasets {
            project_id,
            annotation_sets,
            labels,
            name,
        } => {
            if let Some(project_id) = project_id {
                let datasets = client
                    .datasets(project_id.try_into()?, name.as_deref())
                    .await?;
                for dataset in datasets {
                    println!(
                        "[{}] {}: {}",
                        dataset.uid(),
                        dataset.name(),
                        dataset.description()
                    );

                    if labels {
                        let labels = client.labels(dataset.id()).await?;
                        println!("Labels:");
                        for label in labels {
                            println!("    [{}] {}", label.id(), label.name(),);
                        }
                    }

                    if annotation_sets {
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

                        if labels {
                            let labels = client.labels(dataset.id()).await?;
                            println!("Labels:");
                            for label in labels {
                                println!("    [{}] {}", label.id(), label.name(),);
                            }
                        }

                        if annotation_sets {
                            let annotation_sets = client.annotation_sets(dataset.id()).await?;
                            println!("Annotation Sets:");
                            for annotation_set in annotation_sets {
                                println!(
                                    "    [{}] {}: {}",
                                    annotation_set.uid(),
                                    annotation_set.name(),
                                    annotation_set.description(),
                                );
                            }
                        }
                    }
                }
            }
        }
        Command::Dataset {
            dataset_id,
            annotation_sets,
            labels,
        } => {
            let dataset = client.dataset(dataset_id.try_into()?).await?;
            println!(
                "[{}] {}: {}",
                dataset.uid(),
                dataset.name(),
                dataset.description()
            );

            if labels {
                let labels = client.labels(dataset.id()).await?;
                println!("Labels:");
                for label in labels {
                    println!("    [{}] {}", label.id(), label.name(),);
                }
            }

            if annotation_sets {
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
        }
        Command::DownloadDataset {
            dataset_id,
            groups,
            types,
            output,
        } => {
            let bar = indicatif::ProgressBar::new(0);
            bar.set_style(
                ProgressStyle::with_template(
                    "[{elapsed_precise} ETA: {eta}] {msg}: {wide_bar:.yellow} {human_pos}/{human_len}",
                )
                .unwrap()
                .progress_chars("█▇▆▅▄▃▂▁  "),
            );

            let output = output.unwrap_or_else(|| ".".into());

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
        }
        Command::DownloadAnnotations {
            annotation_set_id,
            groups,
            types,
            output,
        } => {
            let bar = indicatif::ProgressBar::new(0);
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

                        let mut df = edgefirst_client::annotations_dataframe(&annotations);
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
        }
        Command::UploadDataset {
            dataset_id,
            annotation_set_id,
            annotations,
            images,
        } => {
            #[cfg(not(feature = "polars"))]
            {
                return Err(Error::FeatureNotEnabled("polars".to_owned()));
            }

            #[cfg(feature = "polars")]
            {
                use polars::prelude::*;
                use std::{collections::HashMap, path::Path};

                // Validate inputs
                if annotations.is_none() && images.is_none() {
                    return Err(Error::InvalidParameters(
                        "Must provide at least one of --annotations or --images".to_owned(),
                    ));
                }

                // Warning: annotations exist but no annotation_set_id
                if annotations.is_some() && annotation_set_id.is_none() {
                    eprintln!(
                        "⚠️  Warning: Arrow file provided but no --annotation-set-id specified."
                    );
                    eprintln!("   Annotations in the Arrow file will NOT be uploaded.");
                    eprintln!("   Only images will be imported.");
                }

                // Warning: annotation_set_id provided but no annotations
                if annotation_set_id.is_some() && annotations.is_none() {
                    eprintln!(
                        "⚠️  Warning: --annotation-set-id provided but no --annotations file."
                    );
                    eprintln!("   No annotations will be read or uploaded.");
                    eprintln!("   Only images will be imported.");
                }

                // Determine images path
                let images_path = if let Some(ref img_path) = images {
                    // Explicit images path provided
                    img_path.clone()
                } else if let Some(ref arrow_path) = annotations {
                    // Auto-discover based on arrow filename
                    let arrow_dir = arrow_path.parent().unwrap_or_else(|| Path::new("."));
                    let arrow_stem = arrow_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("dataset");

                    // Try: <arrow_stem>/ folder
                    let folder = arrow_dir.join(arrow_stem);
                    if folder.exists() && folder.is_dir() {
                        folder
                    } else {
                        // Try: dataset/ folder
                        let dataset_folder = arrow_dir.join("dataset");
                        if dataset_folder.exists() && dataset_folder.is_dir() {
                            dataset_folder
                        } else {
                            // Try: <arrow_stem>.zip
                            let zip_file = arrow_dir.join(format!("{}.zip", arrow_stem));
                            if zip_file.exists() {
                                zip_file
                            } else {
                                // Try: dataset.zip
                                let dataset_zip = arrow_dir.join("dataset.zip");
                                if dataset_zip.exists() {
                                    dataset_zip
                                } else {
                                    return Err(Error::InvalidParameters(format!(
                                        "Could not find images. Tried:\n  - {}/\n  - {}/\n  - {}\n  - {}\nPlease specify --images explicitly.",
                                        folder.display(),
                                        dataset_folder.display(),
                                        zip_file.display(),
                                        dataset_zip.display()
                                    )));
                                }
                            }
                        }
                    }
                } else {
                    // No annotations, images must be provided
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

                // Parse annotations from Arrow if provided
                let mut samples_map: HashMap<
                    String,
                    (Option<String>, Vec<edgefirst_client::Annotation>),
                > = HashMap::new();
                let should_upload_annotations =
                    annotations.is_some() && annotation_set_id.is_some();

                if let Some(ref arrow_path) = annotations {
                    // Read Arrow file
                    let mut file = File::open(arrow_path)?;
                    let df = IpcReader::new(&mut file).finish().map_err(|e| {
                        Error::InvalidParameters(format!("Failed to read Arrow file: {}", e))
                    })?;

                    // Parse DataFrame into samples
                    // Schema: one row per annotation (or one row for sample without annotations)
                    // Columns: name, frame, object_id, label, label_index, group, mask, box2d,
                    // box3d
                    for idx in 0..df.height() {
                        // Get name (required)
                        let name = df
                            .column("name")
                            .map_err(|e| {
                                Error::InvalidParameters(format!("Missing 'name' column: {}", e))
                            })?
                            .str()
                            .map_err(|e| {
                                Error::InvalidParameters(format!(
                                    "Invalid 'name' column type: {}",
                                    e
                                ))
                            })?
                            .get(idx)
                            .ok_or_else(|| {
                                Error::InvalidParameters("Missing name value".to_owned())
                            })?
                            .to_string();

                        // Get group (optional, categorical column)
                        let sample_group = df
                            .column("group")
                            .ok()
                            .and_then(|c| c.str().ok())
                            .and_then(|s| s.get(idx))
                            .map(|s| s.to_string());

                        // Get or create sample entry (group is stored with sample)
                        let entry = samples_map
                            .entry(name.clone())
                            .or_insert((sample_group, Vec::new()));

                        // Only parse annotations if we're going to upload them
                        if should_upload_annotations {
                            // Check if this row has any annotations (box2d, box3d, or mask)
                            let mut has_annotation = false;
                            let mut geometry_count = 0; // Track number of geometries in this row
                            let mut annotation = edgefirst_client::Annotation::new();

                            // Get label (optional, categorical column)
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

                            // Get object_id (optional) - we'll check later if we need to generate
                            // one
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

                            // Get box2d (optional, Array<Float32, 4> format: [cx, cy, width,
                            // height])
                            if let Ok(box2d_col) = df.column("box2d")
                                && let Ok(array_chunked) = box2d_col.array()
                                && let Some(array_series) = array_chunked.get_as_series(idx)
                                && let Ok(values) = array_series.f32()
                            {
                                let coords: Vec<f32> = values.into_iter().flatten().collect();
                                if coords.len() >= 4 {
                                    // Convert from [cx, cy, w, h] to [x, y, w, h]
                                    let cx = coords[0];
                                    let cy = coords[1];
                                    let w = coords[2];
                                    let h = coords[3];
                                    let x = cx - w / 2.0;
                                    let y = cy - h / 2.0;
                                    let bbox = edgefirst_client::Box2d::new(x, y, w, h);
                                    annotation.set_box2d(Some(bbox));
                                    has_annotation = true;
                                    geometry_count += 1;
                                }
                            }

                            // Get box3d (optional, Array<Float32, 6> format: [cx, cy, cz, w, h, l])
                            if let Ok(box3d_col) = df.column("box3d")
                                && let Ok(array_chunked) = box3d_col.array()
                                && let Some(array_series) = array_chunked.get_as_series(idx)
                                && let Ok(values) = array_series.f32()
                            {
                                let coords: Vec<f32> = values.into_iter().flatten().collect();
                                if coords.len() >= 6 {
                                    // Box3d::new(cx, cy, cz, width, height, length)
                                    let box3d = edgefirst_client::Box3d::new(
                                        coords[0], coords[1], coords[2], coords[3], coords[4],
                                        coords[5],
                                    );
                                    annotation.set_box3d(Some(box3d));
                                    has_annotation = true;
                                    geometry_count += 1;
                                }
                            }

                            // Get mask (optional, List<Float32> format: flat array of x,y pairs)
                            if let Ok(mask_col) = df.column("mask")
                                && let Ok(list_chunked) = mask_col.list()
                                && let Some(mask_series) = list_chunked.get_as_series(idx)
                                && let Ok(values) = mask_series.f32()
                            {
                                let coords: Vec<f32> = values.into_iter().flatten().collect();
                                if !coords.is_empty() && coords.len().is_multiple_of(2) {
                                    // Convert flat array to Vec<Vec<(f32, f32)>>
                                    // Split on NaN to separate polygons
                                    let mut polygons: Vec<Vec<(f32, f32)>> = Vec::new();
                                    let mut current_polygon: Vec<(f32, f32)> = Vec::new();

                                    let mut i = 0;
                                    while i < coords.len() {
                                        let x = coords[i];
                                        let y = coords[i + 1];

                                        if x.is_nan() || y.is_nan() {
                                            // End current polygon
                                            if !current_polygon.is_empty() {
                                                polygons.push(current_polygon.clone());
                                                current_polygon.clear();
                                            }
                                        } else {
                                            current_polygon.push((x, y));
                                        }
                                        i += 2;
                                    }

                                    // Add final polygon
                                    if !current_polygon.is_empty() {
                                        polygons.push(current_polygon);
                                    }

                                    if !polygons.is_empty() {
                                        let mask = edgefirst_client::Mask::new(polygons);
                                        annotation.set_mask(Some(mask));
                                        has_annotation = true;
                                        geometry_count += 1;
                                    }
                                }
                            }

                            // If multiple geometries on same row and no object_id, generate UUID
                            // This ensures all geometries belong to the same object on the server
                            if geometry_count > 1 && object_id.is_none() {
                                let generated_uuid = uuid::Uuid::new_v4().to_string();
                                annotation.set_object_id(Some(generated_uuid));
                            }

                            // Only add annotation if it has at least one geometry or label
                            // (samples without annotations are represented by name/group only - no
                            // annotation added)
                            if has_annotation {
                                entry.1.push(annotation);
                            }
                        }
                    }
                }

                // Find image files and create samples
                let mut samples = Vec::new();

                // If we have Arrow data, use those sample names
                if !samples_map.is_empty() {
                    for (image_name, (sample_group, annotations)) in samples_map {
                        // Find matching image file
                        let image_path = if images_path.is_dir() {
                            // Search directory for matching file
                            let entries = std::fs::read_dir(&images_path)?;
                            let mut found_path = None;

                            for entry in entries {
                                let entry = entry?;
                                let path = entry.path();
                                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                                    // Match base name (without extension)
                                    if file_name.starts_with(&image_name)
                                        && (file_name == image_name
                                            || file_name
                                                .strip_prefix(&image_name)
                                                .map(|s| s.starts_with('.'))
                                                .unwrap_or(false))
                                    {
                                        found_path = Some(path);
                                        break;
                                    }
                                }
                            }

                            found_path.ok_or_else(|| {
                                Error::InvalidParameters(format!(
                                    "Image file not found for sample: {}",
                                    image_name
                                ))
                            })?
                        } else {
                            return Err(Error::InvalidParameters(
                                "ZIP file support not yet implemented".to_owned(),
                            ));
                        };

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
                } else {
                    // No Arrow file, upload all images in the directory
                    if images_path.is_dir() {
                        let entries = std::fs::read_dir(&images_path)?;

                        for entry in entries {
                            let entry = entry?;
                            let path = entry.path();

                            // Skip directories and non-image files
                            if !path.is_file() {
                                continue;
                            }

                            // Check if it's likely an image file
                            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                let ext_lower = ext.to_lowercase();
                                if !matches!(
                                    ext_lower.as_str(),
                                    "jpg" | "jpeg" | "png" | "bmp" | "tiff" | "tif" | "webp"
                                ) {
                                    continue;
                                }
                            } else {
                                continue;
                            }

                            let file_name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .ok_or_else(|| {
                                    Error::InvalidParameters(format!(
                                        "Invalid filename: {}",
                                        path.display()
                                    ))
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
                    } else {
                        return Err(Error::InvalidParameters(
                            "ZIP file support not yet implemented".to_owned(),
                        ));
                    }
                }

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

                // Set up progress bar
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

                // Upload samples - API limit is 500 samples per batch
                const BATCH_SIZE: usize = 500;
                let mut all_results = Vec::new();

                let dataset_id_parsed: edgefirst_client::DatasetID = dataset_id.try_into()?;
                let annotation_set_id_parsed = if should_upload_annotations {
                    Some(annotation_set_id.unwrap().try_into()?)
                } else {
                    None
                };

                // Upload in batches
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

                drop(tx); // Close channel to let progress bar finish

                println!("Successfully uploaded {} samples", all_results.len());
                for result in all_results.iter().take(10) {
                    println!("  Sample UUID: {}", result.uuid);
                }
                if all_results.len() > 10 {
                    println!("  ... and {} more", all_results.len() - 10);
                }
            }
        }
        Command::Experiments { project_id, name } => {
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
        }
        Command::Experiment { experiment_id } => {
            let experiment = client.experiment(experiment_id.try_into()?).await?;
            println!(
                "[{}] {}: {}",
                experiment.uid(),
                experiment.name(),
                experiment.description()
            );
        }
        Command::TrainingSessions {
            experiment_id,
            name,
        } => {
            if let Some(experiment_id) = experiment_id {
                let sessions = client
                    .training_sessions(experiment_id.try_into()?, name.as_deref())
                    .await?;
                for session in sessions {
                    println!(
                        "{} ({}) {}",
                        session.uid(),
                        session.task().status(),
                        session.name()
                    );

                    for artifact in client.artifacts(session.id()).await? {
                        println!("    - {}", artifact.name());
                    }
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
                            println!(
                                "{} ({}) {}",
                                session.uid(),
                                session.task().status(),
                                session.name()
                            );

                            for artifact in client.artifacts(session.id()).await? {
                                println!("    - {}", artifact.name());
                            }
                        }
                    }
                }
            }
        }
        Command::TrainingSession {
            training_session_id,
            model,
            dataset,
            artifacts,
        } => {
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
        }
        Command::DownloadArtifact {
            session_id,
            name,
            output,
        } => {
            let bar = indicatif::ProgressBar::new(0);
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
        }
        Command::UploadArtifact {
            session_id,
            path,
            name,
        } => {
            let name = name.unwrap_or_else(|| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_owned())
                    .unwrap()
            });
            let session = client.training_session(session_id.try_into()?).await?;
            session.upload_artifact(&client, &name, path).await?;
        }
        Command::Tasks {
            stages,
            name,
            workflow,
            status,
            manager,
        } => {
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
        }
        Command::Task { task_id } => {
            let info = client.task_info(task_id.try_into()?).await?;
            println!("{:?}", info);
        }
        Command::ValidationSessions { project_id } => {
            let sessions = client.validation_sessions(project_id.try_into()?).await?;
            for session in sessions {
                println!(
                    "[{}] {}: {}",
                    session.id(),
                    session.name(),
                    session.description()
                );
            }
        }
        Command::ValidationSession { session_id } => {
            let session = client.validation_session(session_id.try_into()?).await?;
            println!(
                "[{}] {}: {}",
                session.id(),
                session.name(),
                session.description()
            );
        }
    }

    Ok(())
}
