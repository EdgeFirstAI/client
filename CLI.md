---
title: EDGEFIRST-CLIENT
section: 1
header: EdgeFirst Client Manual
footer: edgefirst-client 2.6.2
date: 2025-12-11
---

# NAME

edgefirst-client - Command-line interface for EdgeFirst Studio MLOps platform

# SYNOPSIS

**edgefirst-client** [*OPTIONS*] *COMMAND*

# DESCRIPTION

**edgefirst-client** is a command-line tool for interacting with EdgeFirst Studio, an MLOps platform for 3D/4D spatial perception AI. It provides comprehensive dataset management, training workflow orchestration, and artifact handling capabilities.

The client supports various authentication methods including environment variables, configuration files, and command-line options. Authentication tokens are cached in the OS-specific configuration directory for persistent sessions.

# GLOBAL OPTIONS

**\--server** *SERVER*
:   EdgeFirst Studio server name (test, stage, or saas). Can be set via **STUDIO_SERVER** environment variable.

**\--username** *USERNAME*
:   EdgeFirst Studio username for authentication. Can be set via **STUDIO_USERNAME** environment variable.

**\--password** *PASSWORD*
:   EdgeFirst Studio password for authentication. Can be set via **STUDIO_PASSWORD** environment variable.

**\--token** *TOKEN*
:   EdgeFirst Studio authentication token. Can be set via **STUDIO_TOKEN** environment variable. Overrides username/password authentication.

**-h**, **\--help**
:   Print help information.

**-V**, **\--version**
:   Print version information.

# COMMANDS

## AUTHENTICATION

### version

Returns the EdgeFirst Studio server version.

**edgefirst-client version**

### login

Login to the EdgeFirst Studio server with the provided username and password. The authentication token is stored in the application configuration file for subsequent commands.

**edgefirst-client** [**\--server** *SERVER*] **\--username** *USERNAME* **\--password** *PASSWORD* **login**

Token storage locations:

- Linux: `~/.config/EdgeFirst Studio/token`
- macOS: `~/Library/Application Support/ai.EdgeFirst.EdgeFirst Studio/token`
- Windows: `%APPDATA%\EdgeFirst\EdgeFirst Studio\config\token`

### logout

Logout by removing the authentication token from the application configuration file.

**edgefirst-client logout**

### token

Returns the EdgeFirst Studio authentication token for the provided username and password. This is typically stored in the **STUDIO_TOKEN** environment variable for subsequent commands to avoid re-entering credentials.

**edgefirst-client** **\--username** *USERNAME* **\--password** *PASSWORD* **token**

Example:

```bash
export STUDIO_TOKEN=$(edgefirst-client --username user --password pass token)
```

## ORGANIZATION

### organization

Show the authenticated user's organization information.

**edgefirst-client organization**

Displays organization name, ID, and other metadata.

## PROJECTS

### projects

List all projects available to the authenticated user.

**edgefirst-client projects** [**\--name** *NAME*]

**Options:**

**\--name** *NAME*
:   Filter projects by name (case-insensitive substring match).

### project

Retrieve detailed information for a specific project.

**edgefirst-client project** *PROJECT_ID*

**Arguments:**

*PROJECT_ID*
:   The unique identifier of the project.

## DATASETS

### datasets

List all datasets available to the authenticated user. If a project ID is provided, only datasets for that project are listed.

**edgefirst-client datasets** [*OPTIONS*] [*PROJECT_ID*]

**Arguments:**

*PROJECT_ID*
:   Optional project ID to filter datasets.

**Options:**

**-a**, **\--annotation-sets**
:   List available annotation sets for each dataset.

**-l**, **\--labels**
:   List available labels for each dataset.

**\--name** *NAME*
:   Filter datasets by name (case-insensitive substring match).

### dataset

Retrieve detailed information for a specific dataset.

**edgefirst-client dataset** [*OPTIONS*] *DATASET_ID*

**Arguments:**

*DATASET_ID*
:   The unique identifier of the dataset.

**Options:**

**-a**, **\--annotation-sets**
:   List available annotation sets for the dataset.

**-l**, **\--labels**
:   List available labels for the dataset.

### create-dataset

Create a new dataset in the specified project.

**edgefirst-client create-dataset** [**\--description** *DESCRIPTION*] *PROJECT_ID* *NAME*

**Arguments:**

*PROJECT_ID*
:   The project ID where the dataset will be created.

*NAME*
:   Name of the new dataset.

**Options:**

**\--description** *DESCRIPTION*
:   Optional description for the dataset.

**Example:**

```bash
edgefirst-client create-dataset 12345 "Training Data" \
    --description "Q4 2025 training dataset"
```

### delete-dataset

Delete a dataset by marking it as deleted.

**edgefirst-client delete-dataset** *DATASET_ID*

**Arguments:**

*DATASET_ID*
:   The unique identifier of the dataset to delete.

**Note:** This operation marks the dataset as deleted but may not immediately remove all associated data. Deletion is typically soft and may be reversible through the web interface.

## ANNOTATION SETS

### create-annotation-set

Create a new annotation set for the specified dataset.

**edgefirst-client create-annotation-set** [**\--description** *DESCRIPTION*] *DATASET_ID* *NAME*

**Arguments:**

*DATASET_ID*
:   The dataset ID where the annotation set will be created.

*NAME*
:   Name of the new annotation set.

**Options:**

**\--description** *DESCRIPTION*
:   Optional description for the annotation set.

**Example:**

```bash
edgefirst-client create-annotation-set 67890 "Manual Review" \
    --description "Human-verified annotations"
```

### delete-annotation-set

Delete an annotation set by marking it as deleted.

**edgefirst-client delete-annotation-set** *ANNOTATION_SET_ID*

**Arguments:**

*ANNOTATION_SET_ID*
:   The unique identifier of the annotation set to delete.

## DATASET OPERATIONS

### download-dataset

Download a dataset to the local filesystem from the EdgeFirst Studio server.

**edgefirst-client download-dataset** [*OPTIONS*] *DATASET_ID*

**Arguments:**

*DATASET_ID*
:   The unique identifier of the dataset to download.

**Options:**

**\--groups** *GROUPS*
:   Only fetch samples belonging to the provided dataset groups (comma-separated list).

**\--types** *TYPES*
:   Fetch specific data types for the dataset (comma-separated list). Default: **image**.
    Supported types: image, radar, lidar, pointcloud, video.

**\--output** *OUTPUT*
:   Output directory path. If not provided, downloads to the current working directory.

**\--flatten**
:   Download all files to the output directory without creating sequence subdirectories.
    When enabled, filenames are automatically prefixed to avoid conflicts between sequences.
    The prefix format is `{sequence_name}_{frame}_` when the frame number is available,
    and `{sequence_name}_` when the frame number is not available. Default: creates subdirectories for sequences.

**Example:**

```bash
# Download only images to specific directory
edgefirst-client download-dataset 12345 \
    --types image --output ./my-dataset

# Download multiple types with group filtering
edgefirst-client download-dataset 12345 \
    --types image,lidar --groups train,validation \
    --output /data/datasets/

# Download with flattened directory structure
# Files from sequences are prefixed with sequence_name_frame_
edgefirst-client download-dataset 12345 \
    --types image --output ./flat-dataset --flatten
```

**Directory Structure:**

By default, downloads are organized by sequence:

```
output/
├── sequence_A/
│   ├── sequence_A_001.camera.jpeg
│   └── sequence_A_002.camera.jpeg
└── sequence_B/
    ├── sequence_B_001.camera.jpeg
    └── sequence_B_002.camera.jpeg
```

With **\--flatten**, all files are placed in the output root with sequence prefixes:

```
output/
├── sequence_A_001.camera.jpeg
├── sequence_A_002.camera.jpeg
├── sequence_B_001.camera.jpeg
└── sequence_B_002.camera.jpeg
```

### download-annotations

Download dataset annotations to a local file. This command accompanies **download-dataset** and is used to download the annotations rather than the dataset file samples (images, radar, lidar, etc.).

**edgefirst-client download-annotations** [*OPTIONS*] *ANNOTATION_SET_ID* *OUTPUT*

**Arguments:**

*ANNOTATION_SET_ID*
:   The unique identifier of the annotation set to download.

*OUTPUT*
:   Output file path. The format is determined by the file extension:
    - **.json** - COCO-style JSON format
    - **.arrow** - EdgeFirst Dataset Format (Apache Arrow)

**Options:**

**\--groups** *GROUPS*
:   Only fetch samples belonging to the provided dataset groups (comma-separated list).

**\--types** *TYPES*
:   Annotation types to download (comma-separated list). Default: **box2d**.
    Supported types: box2d, box3d, mask, polygon, polyline, keypoint.

**Example:**

```bash
# Download 2D bounding boxes as JSON
edgefirst-client download-annotations 54321 annotations.json \
    --types box2d

# Download all annotation types in Arrow format
edgefirst-client download-annotations 54321 annotations.arrow \
    --types box2d,box3d,mask --groups train
```

For Arrow format documentation, see: https://doc.edgefirst.ai/latest/datasets/format/

### upload-dataset

Upload samples to a dataset from images and/or Arrow annotations file. Supports flexible workflows: images-only, annotations-only, or both.

**edgefirst-client upload-dataset** [*OPTIONS*] *DATASET_ID*

**Arguments:**

*DATASET_ID*
:   Dataset ID to upload samples to.

**Options:**

**\--annotations** *ANNOTATIONS*
:   Path to Arrow file with annotations (EdgeFirst Dataset Format). If omitted, only images will be uploaded.

**\--images** *IMAGES*
:   Path to folder or ZIP containing images. If omitted, auto-discovers based on Arrow filename or "dataset" convention.

**\--annotation-set-id** *ANNOTATION_SET_ID*
:   Annotation Set ID for the annotations. Required if Arrow file contains annotations.

**Image Discovery (when \--images not provided):**

The tool automatically searches for images in the following order:

1. Folder named after Arrow file (e.g., `data/` for `data.arrow`)
2. Folder named `dataset/`
3. ZIP file with same basename (e.g., `data.zip` for `data.arrow`)
4. `dataset.zip`

**Examples:**

```bash
# Upload images only
edgefirst-client upload-dataset 12345 --images ./photos/

# Upload Arrow annotations with auto-discovered images
edgefirst-client upload-dataset 12345 \
    --annotations dataset.arrow \
    --annotation-set-id 54321

# Upload both with explicit paths
edgefirst-client upload-dataset 12345 \
    --annotations labels.arrow \
    --images ./images/ \
    --annotation-set-id 54321
```

**Note:** Uploads are batched (500 samples per batch) with progress tracking. Arrow files must conform to the EdgeFirst Dataset Format.

## SNAPSHOTS

Dataset snapshots preserve a complete copy of raw sensor data (MCAP files), directories, or EdgeFirst Dataset Format data at a specific point in time. They can be restored to create new datasets with optional automatic annotation (AGTG) and depth map generation.

For detailed information about snapshots, see: https://doc.edgefirst.ai/saas/studio/snapshots/

### snapshots

List all snapshots available to the authenticated user.

**edgefirst-client snapshots**

Displays snapshot ID, dataset reference, creation date, and username.

### snapshot

Retrieve detailed information for a specific snapshot.

**edgefirst-client snapshot** *SNAPSHOT_ID*

**Arguments:**

*SNAPSHOT_ID*
:   The unique identifier of the snapshot (format: **ss-xxx**).

### create-snapshot

Create a new snapshot from a local file/directory or from an existing server-side dataset. Smart argument interpretation automatically detects the source type.

**edgefirst-client create-snapshot** [*OPTIONS*] *SOURCE* [*ANNOTATION_SET*]

**Arguments:**

*SOURCE*
:   Source for the snapshot. Automatically interpreted based on format:
    - **ds-xxx**: Dataset ID (creates snapshot from server dataset)
    - **as-xxx**: Annotation Set ID (creates snapshot from parent dataset)
    - **path/to/file.mcap**: Local MCAP file upload
    - **path/to/folder/**: Local directory upload
    - **path/to/file.zip**: Local ZIP file upload

*ANNOTATION_SET* (optional)
:   When SOURCE is a dataset ID, optionally specify an annotation set ID (format: **as-xxx**) to include in the snapshot. If not provided, the default "annotations" set is used, or the first available annotation set if no default exists.

**Options:**

**-d, \--description** *DESCRIPTION*
:   Custom description for the snapshot. If not provided, auto-generates from source name and current date/time.

**\--from-path**
:   Explicitly treat SOURCE as a local file path (overrides auto-detection).

**\--from-dataset**
:   Explicitly treat SOURCE as a dataset ID (overrides auto-detection).

**-m, \--monitor**
:   Monitor the task progress until completion (server-side creation only).

**Example:**

```bash
# Create snapshot from server dataset (auto-detected by ds- prefix)
edgefirst-client create-snapshot ds-12345

# Create snapshot with specific annotation set
edgefirst-client create-snapshot ds-12345 as-67890

# Create snapshot with custom description
edgefirst-client create-snapshot ds-12345 --description "Deer Dataset Backup"

# Create from annotation set (auto-detected by as- prefix, looks up parent dataset)
edgefirst-client create-snapshot as-abc123

# Create from server dataset and wait for completion
edgefirst-client create-snapshot ds-12345 --monitor

# Upload local MCAP file (auto-detected by file extension)
edgefirst-client create-snapshot ./recording.mcap

# Upload local directory
edgefirst-client create-snapshot ./sensor_data/

# Upload local ZIP file
edgefirst-client create-snapshot ./dataset.zip

# Explicitly specify source type
edgefirst-client create-snapshot ds-12345 --from-dataset
edgefirst-client create-snapshot ./my_data --from-path
```

**Note:** Server-side creation runs asynchronously. Use `--monitor` to wait for completion, or check status later with `edgefirst-client task <TASK_ID>`. Local uploads display progress during transfer.

### download-snapshot

Download a snapshot to a local directory.

**edgefirst-client download-snapshot** *SNAPSHOT_ID* *OUTPUT*

**Arguments:**

*SNAPSHOT_ID*
:   The unique identifier of the snapshot (format: **ss-xxx**).

*OUTPUT*
:   Output directory path where snapshot contents will be downloaded.

**Example:**

```bash
# Download snapshot
edgefirst-client download-snapshot ss-abc123 ./snapshot_data/
```

### restore-snapshot

Restore a snapshot to create a new dataset. Optionally enable automatic annotation (AGTG) and depth map generation for compatible camera data.

**edgefirst-client restore-snapshot** [*OPTIONS*] *SNAPSHOT_ID*

**Arguments:**

*SNAPSHOT_ID*
:   The unique identifier of the snapshot to restore (format: **ss-xxx**).

**Options:**

**\--autolabel**
:   Enable automatic annotation generation (AGTG) for restored dataset. Requires compatible sensor data and trained models.

**\--autodepth**
:   Enable automatic depth map generation for Maivin/Raivin camera data.

**Example:**

```bash
# Basic restore
edgefirst-client restore-snapshot ss-abc123

# Restore with automatic annotation
edgefirst-client restore-snapshot ss-abc123 --autolabel

# Restore with both AGTG and depth generation
edgefirst-client restore-snapshot ss-abc123 --autolabel --autodepth
```

**Note:** Restoration creates a new dataset. The original snapshot remains unchanged and can be restored multiple times. AGTG processing runs asynchronously - monitor task status for completion.

For more information about AGTG, see: https://doc.edgefirst.ai/latest/datasets/tutorials/annotations/automatic/

### delete-snapshot

Delete a snapshot permanently. This operation cannot be undone.

**edgefirst-client delete-snapshot** *SNAPSHOT_ID*

**Arguments:**

*SNAPSHOT_ID*
:   The unique identifier of the snapshot to delete (format: **ss-xxx**).

**Example:**

```bash
edgefirst-client delete-snapshot ss-abc123
```

**Warning:** Deletion is permanent. Ensure the snapshot is no longer needed before deleting.

### generate-arrow

Generate an Arrow annotation file from a folder of images. This is useful for importing existing image collections into EdgeFirst Dataset Format.

**edgefirst-client generate-arrow** [*OPTIONS*] **\--output** *OUTPUT* *FOLDER*

The command will:

1. Scan the folder recursively for image files (JPEG, PNG)
2. Optionally detect sequence patterns (name_frame.ext)
3. Create an Arrow file with the 2025.10 schema and null annotations

**Arguments:**

*FOLDER*
:   Folder containing images to process. The command scans recursively for supported image formats.

**Options:**

**-o, \--output** *OUTPUT*
:   Output Arrow file path (required). The file will be created with the EdgeFirst Dataset Format schema.

**\--detect-sequences**
:   Detect sequence patterns in filenames. Files matching patterns like `name_001.jpg`, `name_002.jpg` will be grouped into sequences.

**Example:**

```bash
# Generate Arrow file from images
edgefirst-client generate-arrow ./images --output dataset.arrow

# Generate with sequence detection
edgefirst-client generate-arrow ./images -o my_data/my_data.arrow --detect-sequences

# Create Arrow file for existing dataset structure
edgefirst-client generate-arrow ./sensor_data/camera/ --output ./sensor_data/my_data.arrow
```

**Note:** The generated Arrow file contains null annotations for each image. Use EdgeFirst Studio to add annotations, or use `create-snapshot` to upload the directory to EdgeFirst Studio.

### validate-snapshot

Validate a snapshot directory structure against the EdgeFirst Dataset Format specification.

**edgefirst-client validate-snapshot** [*OPTIONS*] *PATH*

The command checks that the directory follows the EdgeFirst Dataset Format:

- Arrow file exists at expected location (`<name>.arrow` or `<name>/<name>.arrow`)
- Sensor container directory exists (e.g., `camera/`, `lidar/`)
- All files referenced in the Arrow file exist on disk

**Arguments:**

*PATH*
:   Snapshot directory to validate. Can be a directory containing an Arrow file and sensor data.

**Options:**

**-v, \--verbose**
:   Show detailed validation issues including warnings and informational messages.

**Example:**

```bash
# Validate a snapshot directory
edgefirst-client validate-snapshot ./my_dataset

# Validate with detailed output
edgefirst-client validate-snapshot ./my_dataset --verbose

# Validate before uploading
edgefirst-client validate-snapshot ./sensor_data && edgefirst-client create-snapshot ./sensor_data
```

**Exit codes:**

- **0**: Validation passed (warnings may be present)
- **1**: Validation failed with errors

## TRAINING

### experiments

List training experiments for the provided project ID (optional). Experiments are a method of grouping training sessions together.

**edgefirst-client experiments** [**\--name** *NAME*] [*PROJECT_ID*]

**Arguments:**

*PROJECT_ID*
:   Optional project ID to filter experiments.

**Options:**

**\--name** *NAME*
:   Filter experiments by name (case-insensitive substring match).

### experiment

Retrieve detailed information for a specific experiment.

**edgefirst-client experiment** *EXPERIMENT_ID*

**Arguments:**

*EXPERIMENT_ID*
:   The unique identifier of the experiment.

### training-sessions

List training sessions for the provided experiment ID (optional). Sessions are individual training jobs that can be queried for detailed information.

**edgefirst-client training-sessions** [**\--name** *NAME*] [*EXPERIMENT_ID*]

**Arguments:**

*EXPERIMENT_ID*
:   Optional experiment ID to limit the training sessions.

**Options:**

**\--name** *NAME*
:   Filter sessions by name (case-insensitive substring match).

### training-session

Retrieve training session information for the provided session ID.

**edgefirst-client training-session** [*OPTIONS*] *TRAINING_SESSION_ID*

**Arguments:**

*TRAINING_SESSION_ID*
:   Training session ID. Can be either an integer or a string with the format **t-xxx** where xxx is the session ID in hexadecimal (as shown in the Web UI).

**Options:**

**-m**, **\--model**
:   List the model parameters for the training session.

**-d**, **\--dataset**
:   List the dataset parameters for the training session.

**-a**, **\--artifacts**
:   List available artifacts for the training session.

**Example:**

```bash
# Get basic session info
edgefirst-client training-session t-1a2b

# Get session with model parameters
edgefirst-client training-session 12345 --model --dataset
```

### download-artifact

Download an artifact from the provided training session ID.

**edgefirst-client download-artifact** [**\--output** *OUTPUT*] *SESSION_ID* *NAME*

**Arguments:**

*SESSION_ID*
:   Training session ID. Can be either an integer or a string with the format **t-xxx**.

*NAME*
:   Name of the artifact to download (e.g., **model.pth**, **metrics.json**).

**Options:**

**\--output** *OUTPUT*
:   Optional output file path. If not provided, the artifact is downloaded to the current working directory with its original name.

**Example:**

```bash
# Download to current directory
edgefirst-client download-artifact t-1a2b best_model.pth

# Download to specific location
edgefirst-client download-artifact 12345 model.pth \
    --output /models/production/model-v2.pth
```

### upload-artifact

Upload an artifact to the provided training session ID.

**edgefirst-client upload-artifact** [**\--name** *NAME*] *SESSION_ID* *PATH*

**Arguments:**

*SESSION_ID*
:   Training session ID.

*PATH*
:   Path to the artifact file to upload.

**Options:**

**\--name** *NAME*
:   Optional name for the artifact. If not provided, the file's basename is used.

**Example:**

```bash
# Upload with original filename
edgefirst-client upload-artifact 12345 ./checkpoint.pth

# Upload with custom name
edgefirst-client upload-artifact t-1a2b ./final.pth \
    --name production_model.pth
```

## TASKS

### tasks

List all tasks for the current user. Tasks represent asynchronous operations like training jobs, dataset imports, or model exports.

**edgefirst-client tasks** [*OPTIONS*]

**Options:**

**\--stages**
:   Retrieve and display the task stages (detailed progress information).

**\--name** *NAME*
:   Filter tasks by name.

**\--workflow** *WORKFLOW*
:   Filter tasks by workflow type.

**\--status** *STATUS*
:   Filter tasks by status (e.g., pending, running, completed, failed).

**\--manager** *MANAGER*
:   Filter tasks by manager type.

**Example:**

```bash
# List all tasks
edgefirst-client tasks

# List running training tasks with stages
edgefirst-client tasks --status running \
    --workflow training --stages
```

### task

Retrieve detailed information about a specific task.

**edgefirst-client task** *TASK_ID*

**Arguments:**

*TASK_ID*
:   The unique identifier of the task.

## VALIDATION

### validation-sessions

List validation sessions for the provided project ID.

**edgefirst-client validation-sessions** *PROJECT_ID*

**Arguments:**

*PROJECT_ID*
:   The project ID to list validation sessions for.

### validation-session

Retrieve validation session information for the provided session ID.

**edgefirst-client validation-session** *SESSION_ID*

**Arguments:**

*SESSION_ID*
:   The unique identifier of the validation session.

# ENVIRONMENT VARIABLES

**STUDIO_SERVER**
:   EdgeFirst Studio server name. Overridden by **\--server** option.

**STUDIO_USERNAME**
:   Username for authentication. Overridden by **\--username** option.

**STUDIO_PASSWORD**
:   Password for authentication. Overridden by **\--password** option.

**STUDIO_TOKEN**
:   Authentication token. Overridden by **\--token** option. Takes precedence over username/password.

**RUST_LOG**
:   Logging level (error, warn, info, debug, trace). Default: info.

# FILES

**~/.config/EdgeFirst Studio/token** (Linux)
:   Cached authentication token for persistent sessions.

**~/Library/Application Support/ai.EdgeFirst.EdgeFirst Studio/token** (macOS)
:   Cached authentication token for persistent sessions.

**%APPDATA%\\EdgeFirst\\EdgeFirst Studio\\config\\token** (Windows)
:   Cached authentication token for persistent sessions.

# EXIT STATUS

**0**
:   Success

**1**
:   General error (authentication failure, network error, invalid parameters)

**101**
:   Test failure (when running tests)

# EXAMPLES

## Authentication Workflow

```bash
# Login and cache token
edgefirst-client --server test \
    --username user@example.com \
    --password secret \
    login

# Subsequent commands use cached token
edgefirst-client projects
```

## Alternative: Environment Variables

```bash
# Set credentials in environment
export STUDIO_SERVER=test
export STUDIO_USERNAME=user@example.com
export STUDIO_PASSWORD=secret

# Commands automatically use environment variables
edgefirst-client datasets
```

## Alternative: Token-Based Authentication

```bash
# Get token and store in environment
export STUDIO_TOKEN=$(edgefirst-client \
    --username user@example.com \
    --password secret \
    token)

# Use token for subsequent commands
edgefirst-client --token $STUDIO_TOKEN projects
```

## Dataset Management Workflow

```bash
# Create a new project dataset
DATASET_ID=$(edgefirst-client create-dataset 12345 "Q4 Training" \
    --description "Fourth quarter training data" | \
    grep -oP '\d+$')

# Create annotation set
ANNSET_ID=$(edgefirst-client create-annotation-set $DATASET_ID "Manual" \
    --description "Human-verified annotations" | \
    grep -oP '\d+$')

# Upload data
edgefirst-client upload-dataset $DATASET_ID \
    --annotations data.arrow \
    --images ./photos/ \
    --annotation-set-id $ANNSET_ID

# Download for verification
edgefirst-client download-dataset $DATASET_ID \
    --output ./verify/

# Clean up
edgefirst-client delete-annotation-set $ANNSET_ID
edgefirst-client delete-dataset $DATASET_ID
```

## Training Artifact Management

```bash
# List experiments and sessions
edgefirst-client experiments 12345
edgefirst-client training-sessions

# Get session details
edgefirst-client training-session t-1a2b --artifacts

# Download training artifacts
edgefirst-client download-artifact t-1a2b model.pth \
    --output ./models/

# Upload post-training artifacts
edgefirst-client upload-artifact t-1a2b ./analysis.json
```

## Monitoring Tasks

```bash
# List all running tasks with stages
edgefirst-client tasks --status running --stages

# Monitor specific task
watch -n 5 'edgefirst-client task 98765'
```

# SEE ALSO

**EdgeFirst Studio Documentation**: https://doc.edgefirst.ai/

**EdgeFirst Dataset Format**: https://doc.edgefirst.ai/latest/datasets/format/

**GitHub Repository**: https://github.com/EdgeFirstAI/client

# BUGS

Report bugs at: https://github.com/EdgeFirstAI/client/issues

# AUTHORS

Au-Zone Technologies <support@au-zone.com>

# COPYRIGHT

Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

Licensed under the Apache License, Version 2.0.
