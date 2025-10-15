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
    }

    Ok(())
}
