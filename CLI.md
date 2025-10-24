---
title: EDGEFIRST-CLIENT
section: 1
header: EdgeFirst Client Manual
footer: edgefirst-client 2.3.0
date: October 2025
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

**Example:**

```bash
# Download only images to specific directory
edgefirst-client download-dataset 12345 \
    --types image --output ./my-dataset

# Download multiple types with group filtering
edgefirst-client download-dataset 12345 \
    --types image,lidar --groups train,validation \
    --output /data/datasets/
```

Downloads are organized by sample with progress tracking.

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
