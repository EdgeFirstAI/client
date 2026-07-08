---
title: EDGEFIRST-CLIENT
section: 1
header: EdgeFirst Client Manual
footer: edgefirst-client 2.12.1
date: 2026-07-05
---

# NAME

edgefirst-client - Command-line interface for EdgeFirst Studio MLOps platform

# SYNOPSIS

**edgefirst-client** [*OPTIONS*] *COMMAND*

# DESCRIPTION

**edgefirst-client** is a command-line tool for interacting with EdgeFirst Studio, an MLOps platform for 3D/4D spatial perception AI. It provides comprehensive dataset management, training workflow orchestration, and artifact handling capabilities.

Install the CLI and Python API together with:

```bash
pip install edgefirst-client
```

This places both the **edgefirst-client** executable and the **edgefirst_client** Python module on your PATH (inside a virtual environment, use `python -m venv .venv` first). See [examples/README.md](examples/README.md) for tutorials.

The client supports various authentication methods including environment variables, configuration files, and command-line options. Authentication tokens are cached in the OS-specific configuration directory for persistent sessions.

# GLOBAL OPTIONS

**\--server** *SERVER*
:   EdgeFirst Studio server name. Maps to `https://{SERVER}.edgefirst.studio`, except "saas" or empty which maps to `https://edgefirst.studio`. Can be set via **STUDIO_SERVER** environment variable.

    **Server Selection Priority:**

    1. **Token's server** (highest) - JWT tokens encode their server. If you have a valid stored or provided token, its server is used regardless of **\--server**.
    2. **\--server** option - Used when logging in with username/password, or when no token is available. If a token exists with a different server, a warning is displayed.
    3. **Default "saas"** - If no token and no server specified, the production server is used.

**\--username** *USERNAME*
:   EdgeFirst Studio username for authentication. Can be set via **STUDIO_USERNAME** environment variable.

**\--password** *PASSWORD*
:   EdgeFirst Studio password for authentication. Can be set via **STUDIO_PASSWORD** environment variable.

**\--token** *TOKEN*
:   EdgeFirst Studio authentication token. Can be set via **STUDIO_TOKEN** environment variable. The server is extracted from the token and takes priority over **\--server**.

**\--token-path** *TOKEN_PATH*
:   Path to the token file, overriding the default platform-specific location. Useful for testing or running multiple instances with different tokens. Can be set via **STUDIO_TOKEN_PATH** environment variable.

**-v**, **\--verbose**
:   Increase logging verbosity. Repeatable: `-v` enables debug logging and `-vv` enables trace logging. Applies to all commands.

**\--trace-file** *TRACE_FILE*
:   Write trace output to a file. The format is determined by the extension: `.json` for Chrome JSON format (viewable in the Perfetto UI) or `.pftrace` for native Perfetto format. Requires a build with the `trace-file` feature. Can be set via **TRACE_FILE** environment variable.

**-h**, **\--help**
:   Print help information.

**-V**, **\--version**
:   Print version information.

# COMMANDS

## AUTHENTICATION

### server-version

Returns the EdgeFirst Studio server version and the client version. Does not require
authentication.

**edgefirst-client server-version**

Note: this command was named **version** prior to 2.12.0. That name now refers to the
dataset-versioning subcommand group (**tag**, **changelog**, **current**, **summary**;
**restore** lives under **version tag restore**) — see
[DATASET VERSIONING](#dataset-versioning).

### login

Login to the EdgeFirst Studio server. The authentication token is stored in the application configuration file for subsequent commands.

**edgefirst-client** [**\--server** *SERVER*] **login**

When **\--username** and **\--password** are omitted, the CLI prompts for them
interactively (recommended). Do not pass passwords on the command line.

Optional flags **\--username** and **\--password** exist for non-interactive
automation; prefer **STUDIO_TOKEN** or **STUDIO_USERNAME** / **STUDIO_PASSWORD**
environment variables for scripts instead.

Token storage locations:

- Linux: `~/.config/edgefirststudio/token`
- macOS: `~/Library/Application Support/ai.EdgeFirst.EdgeFirst-Studio/token`
- Windows: `%APPDATA%\EdgeFirst\EdgeFirst Studio\config\token`

After CLI login, Python code can reuse the cached token with a bare `Client()` call
(it loads the same `FileTokenStorage`-backed token automatically). See
[examples/01_authentication.py](examples/01_authentication.py). Passing credentials or
storage options as constructor keywords (`Client(username=..., password=...)`,
`Client(token=...)`, `Client(use_token_file=False)`) is the deprecated pre-2.6.0 style
and now emits a `DeprecationWarning`; prefer the builder pattern
(`Client().with_login(...)`, `Client().with_token(...)`,
`Client().with_memory_storage()`) for new code.

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

**-g**, **\--groups**
:   List available groups (dataset splits, e.g. `train`/`val`) for the dataset.

**Example (public Coffee Cup dataset on SaaS):**

```bash
edgefirst-client dataset ds-145f --annotation-sets --labels --groups
```

See also [examples/02_explore_dataset.py](examples/02_explore_dataset.py).

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

**edgefirst-client download-dataset** [*OPTIONS*] [*DATASET_ID*]

**Arguments:**

*DATASET_ID*
:   The unique identifier of the dataset to download. Optional when **\--list-types** is used.

**Options:**

**\--groups** *GROUPS*
:   Only fetch samples belonging to the provided dataset groups (comma-separated list).

**\--types** *TYPES*
:   Fetch specific data types for the dataset (comma-separated list). Default: **image**.
    Valid types: `image`, `lidar.pcd`, `lidar.png`, `lidar.jpg`, `radar.pcd`, `radar.png`, `all`. Use `all` to download every sensor type.

**\--output** *OUTPUT*
:   Output directory path. If not provided, downloads to the current working directory.

**\--flatten**
:   Download all files to the output directory without creating sequence subdirectories.
    When enabled, filenames are automatically prefixed to avoid conflicts between sequences.
    The prefix format is `{sequence_name}_{frame}_` when the frame number is available,
    and `{sequence_name}_` when the frame number is not available. Default: creates subdirectories for sequences.

**\--tag** *TAG*
:   Download files from the specified tagged version instead of the current HEAD state.
    The tag must exist for the dataset. When omitted, downloads the current live data.

**\--list-types**
:   List all valid sensor types and exit. *DATASET_ID* is not required when this flag is used.

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

# Download from a tagged version for reproducible training
edgefirst-client download-dataset 12345 \
    --tag v1.0 --types image --output ./versioned-data

# Public Coffee Cup dataset (SaaS)
edgefirst-client download-dataset ds-145f --groups val --types image \
    --output ./coffee_cup_images/
```

See [examples/05_download_dataset.py](examples/05_download_dataset.py).

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
:   Annotation types to download (comma-separated list). If omitted, all annotation
    types are downloaded. Supported types: box2d, box3d, polygon, raster. For backward
    compatibility `mask` and `seg` are accepted as aliases for `polygon` — note that
    `mask` therefore selects vector polygons, **not** raster masks; use `raster` for
    raster pixel masks. There is no `polyline` or `keypoint` annotation type; an
    unrecognized value is rejected with an error listing the accepted types.

**\--tag** *TAG*
:   Download annotations from the specified tagged version instead of the current HEAD state.
    The tag must exist for the annotation set's dataset.

**Example:**

```bash
# Download 2D bounding boxes as JSON
edgefirst-client download-annotations 54321 annotations.json \
    --types box2d

# Download all annotation types in Arrow format
edgefirst-client download-annotations 54321 annotations.arrow \
    --types box2d,box3d,mask --groups train

# Download annotations from a tagged version
edgefirst-client download-annotations 54321 annotations.arrow \
    --tag v1.0 --types box2d

# Coffee Cup public dataset (resolve annotation set ID from dataset command)
edgefirst-client download-annotations <as-id> coffee_cup.arrow --groups val
```

For Arrow format documentation, see: https://doc.edgefirst.ai/latest/datasets/format/

Python: load with `polars.read_ipc()` or `client.samples_dataframe()` — see [examples/04_polars_dataframe.py](examples/04_polars_dataframe.py).

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

**Note:** Uploads are batched (50 samples per batch, so a failed batch only requires
retrying 50 samples) with progress tracking. Batch concurrency defaults to 4 and is
configurable via **EDGEFIRST_UPLOAD_BATCHES**. Arrow files must conform to the
EdgeFirst Dataset Format.

### update-dimensions

Backfill missing image width/height metadata for an existing dataset. Useful for datasets uploaded before the client started extracting image dimensions at upload time, or for samples where dimensions could not be determined.

**edgefirst-client update-dimensions** *DATASET_ID*

**Arguments:**

*DATASET_ID*
:   The unique identifier of the dataset to backfill. Accepts integer IDs or the **ds-xxx** form shown in the Web UI.

**Behavior:**

1. Fetches every sample in the dataset and filters to those missing **width** or **height**. If none are missing, the command prints `Updated dimensions for 0 samples` and exits successfully.
2. For each remaining sample, downloads the associated image, extracts the pixel dimensions locally, and queues a `(sample_id, width, height)` update. Samples that lack an image URL, return a non-success HTTP status (e.g. `404`, `500`), or cannot be parsed as a recognized image format are skipped silently — the command continues with the next sample.
3. Sends queued updates to the server in batches of **500** via the `samples.update_dimensions` JSON-RPC method.

**Progress output:**

Progress is reported on stdout in the form:

```text
[CURRENT/TOTAL] Computing dimensions
```

`TOTAL` is the count of samples missing dimensions (not the full dataset), and `CURRENT` advances once per sample processed (including skipped ones). A final summary line is printed when the operation completes:

```text
Updated dimensions for N samples
```

`N` is the count returned by the server — the number of samples actually updated, which may be less than `TOTAL` if some samples were skipped.

**Example:**

```bash
# Backfill dimensions for a legacy dataset
edgefirst-client update-dimensions 12345

# Using the ds- form
edgefirst-client update-dimensions ds-12345
```

**Notes:**

- This is a **one-time repair operation**. After it completes, the dataset's sample width/height columns are populated server-side and subsequent calls will report `Updated dimensions for 0 samples`.
- The operation downloads each image in serial, so runtime scales linearly with the number of samples missing dimensions and the size of those images. For very large datasets, run the command from a host with good bandwidth to the EdgeFirst Studio object store.
- Equivalent programmatic APIs:
  - **Rust:** `Client::backfill_sample_dimensions(dataset_id, progress)` (and `Client::update_sample_dimensions` for already-known dimensions).
  - **Python:** `client.backfill_sample_dimensions(dataset_id, progress=cb)`.
  - **Swift/Kotlin (UniFFI):** `client.backfillSampleDimensions(datasetId)` — blocking, **no progress callback** in the FFI layer; for progress reporting on mobile, call the underlying `samples.update_dimensions` RPC directly or use the Python/Rust API on the server side.

### delete-samples

Delete one or more samples (images) from a dataset.

**edgefirst-client delete-samples** *DATASET_ID* *SAMPLE_IDS*...

**Arguments:**

*DATASET_ID*
:   The unique identifier of the dataset the samples belong to.

*SAMPLE_IDS*
:   One or more sample (image) IDs to delete.

**Example:**

```bash
edgefirst-client delete-samples 12345 1001 1002
```

**Note:** Annotations belonging to the deleted samples are removed automatically by the server (cascade delete) — no separate step is needed. Deletion is asynchronous on the server: the command returns once the request is accepted, before the delete has actually completed, so samples may not disappear from subsequent queries immediately.

## DATASET VERSIONING

The `version` subcommand group manages dataset version tags, changelog inspection, and dataset restore. Every dataset modification is recorded with a monotonic serial number. Named tags capture the complete dataset state at a point in time, enabling reproducible training runs and controlled rollbacks.

**Key concepts:**

- **Serial** — Per-dataset monotonic counter that increments with each logged change.
- **Tag** — Named reference to a serial number and full database snapshot (images, annotations, labels, annotation sets, sensor data).
- **Changelog** — Append-only audit trail of every dataset modification.

Tag names may contain alphanumeric characters, dots, dashes, and underscores (e.g., `v1.0`, `training-2026-04`, `baseline_v3`).

### version tag create

Create a named version tag capturing the current dataset state.

**edgefirst-client version tag create** *DATASET* *NAME* [*OPTIONS*]

**Arguments:**

*DATASET*
:   Dataset identifier (ID string, for example `ds-1a2b3c`).

*NAME*
:   Tag name. Allowed characters: alphanumeric, `.`, `-`, `_`. Max 100 characters. Case-sensitive.

**Options:**

**\-d**, **\--description** *DESCRIPTION*
:   Human-readable description for the tag.

**Example:**

```bash
# Tag current dataset state for a training run
edgefirst-client version tag create ds-1a2b3c v1.0 -d "Initial production dataset"

# Tag using dataset name
edgefirst-client version tag create "My Dataset" training-2026-04
```

### version tag list

List all version tags for a dataset, ordered by serial number (most recent first).

**edgefirst-client version tag list** *DATASET*

**Example:**

```bash
edgefirst-client version tag list ds-1a2b3c
```

### version tag get

Show detailed information for a specific version tag.

**edgefirst-client version tag get** *DATASET* *NAME*

**Example:**

```bash
edgefirst-client version tag get ds-1a2b3c v1.0
```

### version tag delete

Delete a version tag and its snapshot data. This operation is irreversible.

**edgefirst-client version tag delete** *DATASET* *NAME*

**Example:**

```bash
edgefirst-client version tag delete ds-1a2b3c v1.0-draft
```

### version tag restore

Restore a dataset to the state captured by a version tag. All changes made after the tag are discarded. The tag itself is preserved and can be restored again.

**edgefirst-client version tag restore** *DATASET* *NAME*

**Example:**

```bash
edgefirst-client version tag restore ds-1a2b3c v1.0
```

### version changelog

Show changelog entries for a dataset. Accepts serial numbers or tag names as range boundaries.

**edgefirst-client version changelog** *DATASET* [*OPTIONS*]

**Options:**

**\--from** *VERSION*
:   Start of the range (tag name or serial number, inclusive). Defaults to the beginning of the changelog.

**\--to** *VERSION*
:   End of the range (tag name or serial number, inclusive). Defaults to the current serial.

**\--types** *TYPES*
:   Filter by entity types (comma-separated). Valid values: `image`, `annotation`, `label`, `annotation_set`, `sensor_data`, `dataset`.

**\--limit** *LIMIT*
:   Maximum number of entries to return. Default: **100**.

**Example:**

```bash
# Show all recent changelog entries
edgefirst-client version changelog ds-1a2b3c

# Show changes between two tags
edgefirst-client version changelog ds-1a2b3c --from v1.0 --to v2.0

# Show only annotation changes since a specific serial
edgefirst-client version changelog ds-1a2b3c --from 10 --types annotation

# Show the 50 most recent entries
edgefirst-client version changelog ds-1a2b3c --limit 50
```

### version current

Show the current version information for a dataset: serial number, all tags, and a dataset summary.

**edgefirst-client version current** *DATASET*

**Example:**

```bash
edgefirst-client version current ds-1a2b3c
```

### version summary

Show cached dataset metrics: image count, annotation counts by type, label count, and annotation set count.

**edgefirst-client version summary** *DATASET*

**Example:**

```bash
edgefirst-client version summary ds-1a2b3c
```

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

**edgefirst-client create-snapshot** [*OPTIONS*] *SOURCE*

**Arguments:**

*SOURCE*
:   Source for the snapshot. Automatically interpreted based on format:
    - **ds-xxx**: Dataset ID (creates snapshot from server dataset)
    - **as-xxx**: Annotation Set ID (creates snapshot from parent dataset)
    - **path/to/file.mcap**: Local MCAP file upload
    - **path/to/folder/**: Local directory upload (pre-validated against the EdgeFirst
      Dataset Format; structural errors abort with a hint to run **generate-arrow**,
      lesser issues only warn)
    - **path/to/file.arrow** with a same-basename **path/to/file.zip** sibling in the
      same directory: uploaded together as a paired EdgeFirst Dataset Format source
    - **path/to/file.zip** (no matching **.arrow** sibling): Local ZIP file upload

**Options:**

**\--annotation-set** *ANNOTATION_SET*
:   When SOURCE is a dataset ID, optionally specify an annotation set ID (format: **as-xxx**) to include in the snapshot. If not provided, the default "annotations" set is used, or the first available annotation set if no default exists.

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
edgefirst-client create-snapshot ds-12345 --annotation-set as-67890

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

**edgefirst-client download-snapshot** [*OPTIONS*] **\--output** *OUTPUT* *SNAPSHOT_ID*

**Arguments:**

*SNAPSHOT_ID*
:   The unique identifier of the snapshot (format: **ss-xxx**).

**Options:**

**\--output** *OUTPUT*
:   Output directory path where snapshot contents will be downloaded (required).

**Example:**

```bash
# Download snapshot
edgefirst-client download-snapshot ss-abc123 --output ./snapshot_data/
```

### restore-snapshot

Restore a snapshot to a dataset in a project. Supports MCAP uploads with optional AGTG (auto-annotation) and automatic depth map generation for compatible camera data.

**edgefirst-client restore-snapshot** [*OPTIONS*] *PROJECT_ID* *SNAPSHOT_ID*

**Arguments:**

*PROJECT_ID*
:   The project ID to restore the snapshot into.

*SNAPSHOT_ID*
:   The unique identifier of the snapshot to restore (format: **ss-xxx**).

**Options:**

**\--topics** *TOPICS*
:   MCAP topics to include (comma-separated). Empty means all topics.

**\--autolabel** *AUTOLABEL*
:   Object labels for AGTG auto-annotation (comma-separated, e.g. `person,car`). Empty means no AGTG. Requires compatible sensor data and trained models.

**\--autodepth**
:   Generate depth maps. Maivin/Raivin cameras only.

**\--dataset-name** *DATASET_NAME*
:   Custom name for the restored dataset.

**\--dataset-description** *DATASET_DESCRIPTION*
:   Description for the restored dataset.

**\--monitor**
:   Monitor the restore task progress until completion.

**Example:**

```bash
# Basic restore into a project
edgefirst-client restore-snapshot p-abc123 ss-def456

# Restore with AGTG auto-annotation for the given labels
edgefirst-client restore-snapshot p-abc123 ss-def456 --autolabel person,car

# Restore with AGTG and depth generation, with a custom name, and wait for completion
edgefirst-client restore-snapshot p-abc123 ss-def456 \
    --autolabel person,car --autodepth \
    --dataset-name "Field Test Restore" --monitor
```

**Note:** Restoration creates a new dataset. The original snapshot remains unchanged and can be restored multiple times. AGTG processing runs asynchronously — use **\--monitor**, or check task status, for completion.

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

**validate-snapshot** takes no options of its own. Detailed output (warnings and
informational messages beyond the first 5 of each kind) is controlled by the
**global** **-v**, **\--verbose** flag documented under GLOBAL OPTIONS, not a
command-local flag — pass it anywhere on the command line.

**Example:**

```bash
# Validate a snapshot directory
edgefirst-client validate-snapshot ./my_dataset

# Validate with detailed output (global -v flag)
edgefirst-client -v validate-snapshot ./my_dataset

# Validate before uploading
edgefirst-client validate-snapshot ./sensor_data && edgefirst-client create-snapshot ./sensor_data
```

**Exit codes:**

- **0**: Validation passed (warnings may be present)
- **1**: Validation failed with errors

## COCO INTERCHANGE

Tools for converting between the COCO (Common Objects in Context) annotation format and the EdgeFirst Dataset Format, and for importing and exporting COCO datasets directly to and from EdgeFirst Studio. These commands support bounding boxes and polygon segmentation; RLE segmentation is decoded to polygons.

For details on the EdgeFirst Dataset Format and its COCO mapping, see: https://doc.edgefirst.ai/latest/datasets/format/

### coco-to-arrow

Convert COCO annotations to EdgeFirst Arrow format. Reads a COCO annotation JSON file or ZIP archive and converts it to the EdgeFirst Dataset Format (Arrow).

**edgefirst-client coco-to-arrow** [*OPTIONS*] **\--output** *OUTPUT* *COCO_PATH*

**Arguments:**

*COCO_PATH*
:   Path to a COCO annotation file (JSON) or ZIP archive.

**Options:**

**-o, \--output** *OUTPUT*
:   Output Arrow file path (required).

**\--masks** *MASKS*
:   Include segmentation masks. Defaults to **true**; pass `--masks=false` to convert bounding boxes only. [possible values: true, false]

**\--group** *GROUP*
:   Group name applied to all samples (e.g. `train`, `val`). Sets the dataset split for every converted sample.

**Examples:**

```bash
# Convert detection annotations (boxes + masks) to Arrow
edgefirst-client coco-to-arrow instances.json -o dataset.arrow

# Convert a COCO ZIP archive and tag every sample as the train split
edgefirst-client coco-to-arrow coco.zip -o dataset.arrow --group train

# Convert bounding boxes only (no segmentation)
edgefirst-client coco-to-arrow instances_val2017.json -o val.arrow --masks=false --group val
```

**Note:** Every image in the COCO `images` array produces at least one row. An image with no annotations is emitted as a single placeholder row with a null label, preserving the image and its `group` so dataset splits cover the full image set.

### arrow-to-coco

Convert EdgeFirst Arrow format to COCO annotations. Reads an EdgeFirst Arrow file and converts it to COCO JSON, optionally filtered by group.

**edgefirst-client arrow-to-coco** [*OPTIONS*] **\--output** *OUTPUT* *ARROW_PATH*

**Arguments:**

*ARROW_PATH*
:   Path to an EdgeFirst Arrow file.

**Options:**

**-o, \--output** *OUTPUT*
:   Output COCO JSON file path (required).

**\--masks** *MASKS*
:   Include segmentation masks. Defaults to **true**; pass `--masks=false` for bounding boxes only. [possible values: true, false]

**\--groups** *GROUPS*
:   Filter by group names (comma-separated, e.g. `train,val`). If omitted, all groups are exported.

**\--pretty**
:   Pretty-print the JSON output.

**Examples:**

```bash
# Convert an Arrow dataset to COCO JSON
edgefirst-client arrow-to-coco dataset.arrow -o instances.json

# Export only the train and val splits, pretty-printed
edgefirst-client arrow-to-coco dataset.arrow -o instances.json --groups train,val --pretty
```

### import-coco

Import a COCO dataset directly into EdgeFirst Studio. Converts COCO annotations and uploads the images and annotations to a dataset. Create a new dataset automatically with **\--name**, or target an existing dataset and annotation set.

COCO datasets must be extracted before import — ZIP archives are not supported directly. Extract the annotations and images first.

**edgefirst-client import-coco** [*OPTIONS*] *COCO_PATH*

**Arguments:**

*COCO_PATH*
:   Path to a COCO annotation JSON file or an extracted COCO directory.

**Options:**

**-p, \--project** *PROJECT*
:   Project ID. Required when creating a new dataset with **\--name**.

**-n, \--name** *NAME*
:   Create a new dataset with this name (alternative to **\--dataset**).

**-d, \--description** *DESCRIPTION*
:   Description for the new dataset (used with **\--name**).

**\--dataset** *DATASET*
:   Target dataset ID (alternative to **\--name**).

**\--annotation-set** *ANNOTATION_SET*
:   Target annotation set ID. Defaults to the dataset's first annotation set if not specified.

**\--group** *GROUP*
:   Group name for the imported samples. Acts as a filter against the group auto-detected from each image's path/filename; if omitted, the detected group is used.

**\--masks** *MASKS*
:   Include segmentation masks. Defaults to **true**; pass `--masks=false` for bounding boxes only. [possible values: true, false]

**\--images** *IMAGES*
:   Include images in the upload. Defaults to **true**; pass `--images=false` to upload annotations only. [possible values: true, false]

**\--batch-size** *BATCH_SIZE*
:   Number of samples per upload batch. [default: 100]

**\--concurrency** *CONCURRENCY*
:   Maximum number of concurrent uploads. [default: 64]

**\--verify**
:   Verify the import instead of uploading — compares the local COCO dataset against Studio and reports differences.

**\--update**
:   Update annotations on existing samples without re-uploading images. Use this to add masks to samples imported without them, or to sync updated annotations to Studio.

**Examples:**

```bash
# Create a new dataset and import (group auto-detected from image folders)
edgefirst-client import-coco ./coco --project p-123 --name "COCO 2017"

# Import into an existing dataset and annotation set
edgefirst-client import-coco ./coco --dataset ds-123 --annotation-set as-456

# Import detection boxes only into an existing dataset
edgefirst-client import-coco ./coco/annotations/instances_train2017.json \
    --dataset ds-123 --annotation-set as-456 --masks=false

# Verify a previous import without uploading
edgefirst-client import-coco ./coco --dataset ds-123 --verify
```

**Note:** **\--group** matches against the group EdgeFirst derives from each image's path; standard COCO files reference images by bare filename (e.g. `000000397133.jpg`) and therefore carry no detectable group, so passing a value that does not match excludes those images. To assign a split to images that have none, convert with `coco-to-arrow --group` and upload with `upload-dataset`.

### export-coco

Export an EdgeFirst Studio dataset to COCO format. Downloads samples and annotations from Studio and converts them to COCO JSON, optionally bundling the images into a ZIP archive.

**edgefirst-client export-coco** [*OPTIONS*] **\--output** *OUTPUT* *DATASET_ID* *ANNOTATION_SET_ID*

**Arguments:**

*DATASET_ID*
:   Source dataset ID in Studio.

*ANNOTATION_SET_ID*
:   Source annotation set ID.

**Options:**

**-o, \--output** *OUTPUT*
:   Output file path. Use a `.json` extension for annotations only, or `.zip` to bundle images (see **\--images**).

**\--groups** *GROUPS*
:   Filter by group names (comma-separated, e.g. `train,val`). If omitted, all groups are exported.

**\--masks** *MASKS*
:   Include segmentation masks. Defaults to **true**; pass `--masks=false` for bounding boxes only. [possible values: true, false]

**\--images**
:   Include images in the output. This produces a ZIP archive containing both the COCO JSON and the image files.

**\--pretty**
:   Pretty-print the JSON output.

**Examples:**

```bash
# Export annotations to COCO JSON
edgefirst-client export-coco ds-123 as-456 -o instances.json

# Export the train and val splits with images as a ZIP bundle
edgefirst-client export-coco ds-123 as-456 -o coco.zip --images --groups train,val
```

### migrate

Migrate an Arrow file from the 2025.10 schema to the 2026.04 schema. Converts the legacy NaN-separated `mask` column (`List(Float32)`) to the new nested `polygon` column (`List(List(Float32))`) and sets the `schema_version` metadata.

**edgefirst-client migrate** [*OPTIONS*] *INPUT*

**Arguments:**

*INPUT*
:   Path to the input Arrow file.

**Options:**

**\--output** *OUTPUT*
:   Output path. Defaults to overwriting the input file in place.

**Examples:**

```bash
# Migrate in place
edgefirst-client migrate dataset.arrow

# Migrate to a new file, preserving the original
edgefirst-client migrate dataset.arrow --output migrated.arrow
```

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

### trainer-schemas

List the trainer types available on the server. The reported schema type is used with the **trainer-schema** and **start-training-session** commands.

**edgefirst-client trainer-schemas**

### trainer-schema

Show the parameter schema for a trainer type. The schema describes the hyperparameters accepted by **start-training-session \--param**, including defaults, ranges, and nested parameter groups.

**edgefirst-client trainer-schema** *SCHEMA_TYPE*

**Arguments:**

*SCHEMA_TYPE*
:   Trainer schema type (see the **trainer-schemas** command).

**Example:**

```bash
edgefirst-client trainer-schema modelpack
```

### start-training-session

Launch a new training session for an experiment. The session trains on a single dataset using group-based train/validation splits. The dataset tag defaults to the latest tag and the split groups default to the dataset's standard **train** and **val** groups.

**edgefirst-client start-training-session** [*OPTIONS*] **\--name** *NAME* **\--experiment-id** *ID* **\--trainer-type** *TYPE* **\--dataset-id** *ID* **\--annotation-set-id** *ID* *PROJECT_ID*

**Arguments:**

*PROJECT_ID*
:   Project ID owning the experiment and dataset.

**Options:**

**\--name** *NAME*
:   Name for the training task (required).

**\--experiment-id** *ID*
:   Experiment ID the session belongs to (required).

**\--trainer-type** *TYPE*
:   Trainer schema type (required, see **trainer-schemas**).

**\--dataset-id** *ID*
:   Dataset ID to train on (required).

**\--annotation-set-id** *ID*
:   Annotation set ID providing the ground-truth labels (required).

**\--tag** *TAG*
:   Dataset tag to train against. Defaults to the latest tag; it is an error if the dataset has no tags and none is provided.

**\--train-group** *GROUP*
:   Training split group name. Defaults to **train**.

**\--val-group** *GROUP*
:   Validation split group name. Defaults to **val**.

**\--param** *KEY=VALUE*
:   Trainer hyperparameter, repeatable. Values are parsed as JSON (numbers, booleans) and fall back to strings. See **trainer-schema** for accepted parameters.

**\--session-name** *NAME*
:   Optional display name for the training session.

**\--session-description** *DESC*
:   Optional description for the training session.

**\--weights-session** *ID*
:   Optional source training session ID for transfer-learning weights.

**\--local**
:   Create a user-managed session. No cloud instance is provisioned; the caller runs the training loop and uploads artifacts/metrics.

**\--kubernetes**
:   Schedule onto the organization's Kubernetes runner instead of a cloud instance.

**\--monitor**
:   Monitor the launched task's progress until completion.

**Example:**

```bash
# Launch a cloud training session with the latest dataset tag
edgefirst-client start-training-session p-123 \
    --name nightly-run --experiment-id exp-45 \
    --trainer-type modelpack \
    --dataset-id ds-678 --annotation-set-id as-910 \
    --param epochs=100 --param batch_size=8 --monitor

# Launch a user-managed session against a specific tag and groups
edgefirst-client start-training-session p-123 \
    --name local-run --experiment-id exp-45 \
    --trainer-type modelpack \
    --dataset-id ds-678 --annotation-set-id as-910 \
    --tag v2.0 --train-group daylight --val-group night --local
```

### update-training-session

Update the name and/or description of a training session. At least one of **\--name** or **\--description** must be provided.

**edgefirst-client update-training-session** [*OPTIONS*] *SESSION_ID*

**Arguments:**

*SESSION_ID*
:   Training session ID.

**Options:**

**\--name** *NAME*
:   New session name.

**\--description** *DESC*
:   New session description.

**Example:**

```bash
edgefirst-client update-training-session t-1a2b \
    --name "baseline v2" --description "retrained with new tags"
```

### delete-training-sessions

Delete one or more training sessions.

**WARNING:** validation sessions attached to the deleted training sessions are removed as well, along with all artifacts and checkpoints.

**edgefirst-client delete-training-sessions** *SESSION_IDS*...

**Arguments:**

*SESSION_IDS*
:   One or more training session IDs to delete.

**Example:**

```bash
edgefirst-client delete-training-sessions t-1a2b t-3c4d
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

**edgefirst-client task** [*OPTIONS*] *TASK_ID*

**Arguments:**

*TASK_ID*
:   The unique identifier of the task.

**Options:**

**\--monitor**
:   Monitor the task progress until completion.

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

### update-validation-session

Update the name and/or description of a validation session. At least one of **\--name** or **\--description** must be provided.

**edgefirst-client update-validation-session** [*OPTIONS*] *SESSION_ID*

**Arguments:**

*SESSION_ID*
:   Validation session ID.

**Options:**

**\--name** *NAME*
:   New session name.

**\--description** *DESC*
:   New session description.

### delete-validation-sessions

Delete one or more validation sessions. Only the validation sessions are removed; the parent training session is never affected.

**edgefirst-client delete-validation-sessions** *SESSION_IDS*...

**Arguments:**

*SESSION_IDS*
:   One or more validation session IDs to delete.

**Example:**

```bash
edgefirst-client delete-validation-sessions v-5e6f v-7a8b
```

### validator-schemas

List the validator schemas available on the server. Each schema describes the parameters accepted by the matching validator type.

**edgefirst-client validator-schemas** [**\--type** *TYPE*]

**Options:**

**\--type** *TYPE*
:   Only show the schema with this type.

# ENVIRONMENT VARIABLES

**STUDIO_SERVER**
:   EdgeFirst Studio server name. Used when logging in or when no token is available. If a token exists with a different server, **STUDIO_SERVER** is ignored and a warning is displayed.

**STUDIO_USERNAME**
:   Username for authentication. Overridden by **\--username** option.

**STUDIO_PASSWORD**
:   Password for authentication. Overridden by **\--password** option.

**STUDIO_TOKEN**
:   Authentication token. The server is extracted from the token and takes priority over **STUDIO_SERVER**. Overridden by **\--token** option.

**RUST_LOG**
:   Logging level (error, warn, info, debug, trace). Default: info.

**EDGEFIRST_UPLOAD_BATCHES**
:   Number of concurrent batch-upload tasks used by **upload-dataset**. Default: 4.

**MAX_TASKS**
:   General upload/download task concurrency (e.g. **download-snapshot**). Default:
    half the available CPU cores, clamped to the 2-8 range. Distinct from
    **EDGEFIRST_UPLOAD_BATCHES**.

# FILES

**~/.config/edgefirststudio/token** (Linux)
:   Cached authentication token for persistent sessions.

**~/Library/Application Support/ai.EdgeFirst.EdgeFirst-Studio/token** (macOS)
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
# Login and cache token (prompts for username and password)
edgefirst-client --server test login

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

## Python API Integration (pip install)

```bash
# One install provides CLI + Python module
python3 -m venv .venv && source .venv/bin/activate
pip install edgefirst-client tqdm

edgefirst-client login
python examples/01_authentication.py
```

Hybrid CLI + Python workflow with the public Coffee Cup dataset (`ds-145f`):

```bash
# Inspect dataset
edgefirst-client dataset ds-145f --annotation-sets --labels

# Export annotations as Arrow, analyze in Python
edgefirst-client download-annotations <as-id> coffee_cup.arrow --groups val
python examples/04_polars_dataframe.py

# Download images
edgefirst-client download-dataset ds-145f --groups val --types image \
    --output ./coffee_cup_images/
```

Full tutorial index: [examples/README.md](examples/README.md).

# SEE ALSO

**EdgeFirst Studio Documentation**: https://doc.edgefirst.ai/

**EdgeFirst Dataset Format**: https://doc.edgefirst.ai/latest/datasets/format/

**Python Examples**: [examples/README.md](examples/README.md)

**GitHub Repository**: https://github.com/EdgeFirstAI/client

# BUGS

Report bugs at: https://github.com/EdgeFirstAI/client/issues

# AUTHORS

Au-Zone Technologies <support@au-zone.com>

# COPYRIGHT

Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

Licensed under the Apache License, Version 2.0.
