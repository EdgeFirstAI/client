---
title: EDGEFIRST-CLIENT
section: 1
header: EdgeFirst Client Manual
footer: edgefirst-client 2.10.2
date: 2026-06-03
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

**-g**, **\--groups**
:   List available groups (dataset splits, e.g. `train`/`val`) for the dataset.

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
    - **path/to/folder/**: Local directory upload
    - **path/to/file.zip**: Local ZIP file upload

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
