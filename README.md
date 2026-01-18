# EdgeFirst Studio Client

[![Test](https://github.com/EdgeFirstAI/client/workflows/CI/badge.svg)](https://github.com/EdgeFirstAI/client/actions/workflows/test.yml)
[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=EdgeFirstAI_client&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=EdgeFirstAI_client)
[![codecov](https://codecov.io/gh/EdgeFirstAI/client/branch/main/graph/badge.svg)](https://codecov.io/gh/EdgeFirstAI/client)
[![Crates.io](https://img.shields.io/crates/v/edgefirst-client.svg)](https://crates.io/crates/edgefirst-client)
[![PyPI](https://img.shields.io/pypi/v/edgefirst-client.svg)](https://pypi.org/project/edgefirst-client/)
[![Android SDK](https://img.shields.io/badge/Android-SDK-3DDC84?logo=android&logoColor=white)](ANDROID.md)
[![iOS/macOS SDK](https://img.shields.io/badge/iOS%2FmacOS-SDK-000000?logo=apple&logoColor=white)](APPLE.md)
[![Documentation](https://docs.rs/edgefirst-client/badge.svg)](https://docs.rs/edgefirst-client)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![EdgeFirst Studio](https://img.shields.io/badge/EdgeFirst-Studio-green)](https://edgefirst.studio)

**EdgeFirst Studio Client** is the official command-line application and library for [EdgeFirst Studio](https://edgefirst.studio) - the MLOps platform for 3D visual and 4D spatial perception AI. Available for **Rust**, **Python**, **Android (Kotlin)**, and **iOS/macOS (Swift)**. Automate dataset management, annotation workflows, model training, validation, and deployment for off-road vehicles, robotics, construction equipment, and industrial applications.

## Overview

EdgeFirst Client provides seamless programmatic access to EdgeFirst Studio's comprehensive MLOps capabilities. Whether you're integrating Studio into your CI/CD pipeline, building custom training workflows, or automating data processing systems, EdgeFirst Client delivers the production-grade reliability you need.

**Trusted by EdgeFirst Studio**: This client library powers EdgeFirst Studio's internal training and validation services, providing a battle-tested foundation for production workloads.

### Key Capabilities

- 📦 **MCAP Publishing**: Upload sensor recordings for [automated ground-truth generation (AGTG)](https://doc.edgefirst.ai/latest/datasets/tutorials/annotations/automatic/)
- 🏷️ **Dataset Management**: Download datasets and annotations in multiple formats
- 🎯 **Training & Validation**: Monitor sessions, publish metrics, manage model artifacts
- 🚀 **Model Artifacts**: Upload and download trained models (ONNX, TensorFlow Lite, H5, etc.)
- 📊 **Multiple Formats**: Darknet/YOLO, EdgeFirst Dataset Format (Arrow), user-defined formats
- 🔌 **Seamless Integration**: Direct REST API access to all EdgeFirst Studio features

## Features

### Dataset Management

- **Create snapshots** from MCAP files, directories, or EdgeFirst Dataset format (Zip/Arrow)
- **Upload MCAP recordings** for [AGTG (Automated Ground-Truth Generation)](https://doc.edgefirst.ai/latest/datasets/tutorials/annotations/automatic/) workflow
- **Restore snapshots** with automatic annotation (`--autolabel`) and depth map generation (`--autodepth`)
- **Download datasets** with support for images, LiDAR PCD, depth maps, and radar data
- **Download annotations** in JSON or Arrow format ([EdgeFirst Dataset Format](https://doc.edgefirst.ai/latest/datasets/format/))
- **Dataset groups and filtering** for flexible data organization

### Training Workflows

- **List and manage experiments** (training session groups)
- **Monitor training sessions** with real-time status tracking
- **Publish training metrics** to EdgeFirst Studio during model training
- **Upload custom training artifacts** for experiment tracking
- **Download model artifacts** and training logs
- **Access model and dataset parameters** for reproducibility

### Validation Workflows

- **List and manage validation sessions** across projects
- **Publish validation metrics** to EdgeFirst Studio
- **Upload validation files and results** for analysis
- **Download validation artifacts** including performance reports
- **Track validation task progress** with status monitoring

### Model Artifact Management

- **Publish (upload) model artifacts** from training sessions
- **Download trained models** in various formats (ONNX, TensorFlow Lite, H5, PyTorch, etc.)
- **Used internally by EdgeFirst Studio** trainers and validators
- **Artifact versioning** and experiment tracking

### Multiple Dataset Formats

- **Darknet/YOLO**: Industry-standard annotation formats for object detection
- **EdgeFirst Dataset Format**: Arrow-based format for efficient data handling and 3D perception
- **User-defined formats**: API flexibility for custom dataset structures

### EdgeFirst Studio Integration

- **One-click deployment** from EdgeFirst Studio UI
- **Automatic optimization** for edge devices
- **Performance monitoring** and analytics
- **A/B testing** and gradual rollouts
- **Direct API access** to all Studio features

### Additional Features

- **Task management**: List and monitor background processing tasks
- **Project operations**: Browse and search projects and datasets
- **Annotation sets**: Support for multiple annotation versions per dataset
- **Progress tracking**: Real-time progress updates for uploads and downloads
- **3D perception support**: LiDAR, RADAR, Point Cloud, depth maps

## Installation

### Via Cargo (Rust)

```bash
cargo install edgefirst-cli
```

### Via Pip (Python)

```bash
pip install edgefirst-client
```

### Mobile SDKs (Android & iOS/macOS)

Download the SDK packages from [GitHub Releases](https://github.com/EdgeFirstAI/client/releases):

- **Android**: `edgefirst-client-android-{version}.zip` - Kotlin bindings with JNI libraries
- **iOS/macOS**: `edgefirst-client-swift-{version}.zip` - Swift bindings with XCFramework

See platform-specific documentation for integration instructions:
- [Android SDK Documentation](ANDROID.md)
- [iOS/macOS SDK Documentation](APPLE.md)

### From Source

```bash
git clone https://github.com/EdgeFirstAI/edgefirst-client
cd edgefirst-client
cargo build --release
```

### System Requirements

- **MSRV (Minimum Supported Rust Version)**: Rust 1.90+ (Rust 2024 Edition)
- **Python**: 3.8+ (for Python bindings)
- **Network**: Access to EdgeFirst Studio (*.edgefirst.studio)

### Performance Profiling

EdgeFirst Client includes optional profiling instrumentation for performance analysis and debugging.

> **Note:** Profiling instrumentation is **disabled by default** and compiles to zero-cost no-ops. All release binaries (CLI, Python wheels, mobile SDKs) published to GitHub Releases, PyPI, and crates.io are built **without** tracing features enabled.

**Build with tracing support:**

```bash
# Standard release build with tracing
cargo build --release --features trace-file

# Profiling build with debug symbols preserved (for detailed stack traces)
cargo build --profile profiling --features trace-file
```

The `profiling` build profile inherits from `release` but preserves debug symbols (`debug = true`, `strip = false`), which provides more detailed information in trace visualizations.

**Generate trace files:**

Use `--trace-file` to capture execution traces. Format is determined by file extension:

```bash
# Chrome JSON format (viewable in Perfetto UI)
edgefirst-client --trace-file trace.json download-dataset ds-123 ./output

# Native Perfetto format (smaller files, also viewable in Perfetto UI)
edgefirst-client --trace-file trace.pftrace download-dataset ds-123 ./output
```

View traces at https://ui.perfetto.dev/ by dragging the file into the browser.

**What traces capture:**

- Function call hierarchy and timing
- Dataset IDs, project IDs, and snapshot IDs
- RPC method names and parameters
- File paths for uploads/downloads
- API request/response data (truncated to 4KB for large responses)

> **⚠️ Security Note:** Trace files may contain sensitive information including dataset IDs, file paths, and API response data. Credentials and passwords are automatically redacted, but review trace file contents before sharing, especially for production environments.

## Quick Start

### CLI Authentication

```bash
# Login (stores token locally for 7 days)
edgefirst-client login

# View your organization info
edgefirst-client organization

# Use environment variables (recommended for CI/CD)
export STUDIO_TOKEN="your-token"
edgefirst-client organization
```

### Common CLI Workflows

#### Download Datasets and Annotations

```bash
# List projects and datasets
edgefirst-client projects
edgefirst-client datasets --project-id <PROJECT_ID>

# Download dataset with images
edgefirst-client download-dataset <DATASET_ID> --types image --output ./data

# Download annotations in Arrow format (EdgeFirst Dataset Format)
edgefirst-client download-annotations <ANNOTATION_SET_ID> \
  --types box2d,box3d,segmentation \
  --output annotations.arrow

# Upload samples to dataset
edgefirst-client upload-dataset <DATASET_ID> \
  --annotations annotations.arrow \
  --annotation-set-id <ANNOTATION_SET_ID> \
  --images ./images/
```

For complete upload format specifications, see [EdgeFirst Dataset Format](https://doc.edgefirst.ai/latest/datasets/format/).

#### Monitor Training and Download Models

```bash
# List training experiments
edgefirst-client experiments --project-id <PROJECT_ID>

# Monitor training sessions
edgefirst-client training-sessions --experiment-id <EXP_ID>

# Get training session details with artifacts
edgefirst-client training-session <SESSION_ID> --artifacts

# Download trained model
edgefirst-client download-artifact <SESSION_ID> modelpack.onnx --output ./models/
```

#### Work with Snapshots

Snapshots preserve complete copies of sensor data, datasets, or directories for versioning and backup. Restore them with optional automatic annotation (AGTG) and depth map generation.

```bash
# List all snapshots
edgefirst-client snapshots

# Create snapshot from MCAP file
edgefirst-client create-snapshot <DATASET_ID> recording.mcap

# Create snapshot from directory
edgefirst-client create-snapshot <DATASET_ID> ./sensor_data/

# Download snapshot
edgefirst-client download-snapshot <SNAPSHOT_ID> ./snapshot_backup/

# Restore snapshot to new dataset
edgefirst-client restore-snapshot <SNAPSHOT_ID>

# Restore with automatic annotation (AGTG)
edgefirst-client restore-snapshot <SNAPSHOT_ID> --autolabel

# Restore with AGTG and depth map generation
edgefirst-client restore-snapshot <SNAPSHOT_ID> --autolabel --autodepth

# Delete snapshot
edgefirst-client delete-snapshot <SNAPSHOT_ID>
```

For detailed snapshot documentation, see the [EdgeFirst Studio Snapshots Guide](https://doc.edgefirst.ai/saas/studio/snapshots/).

#### EdgeFirst Dataset Format

EdgeFirst Client provides tools for working with the [EdgeFirst Dataset Format](https://doc.edgefirst.ai/latest/datasets/format/) - an Arrow-based format optimized for 3D perception AI workflows.

##### What the CLI Provides

The `create-snapshot` command intelligently handles multiple input types:

- **Folder of images**: Automatically generates `dataset.arrow` manifest and `dataset.zip`, then uploads
- **Arrow manifest file**: Auto-discovers matching `dataset.zip` or `dataset/` folder for images
- **Complete dataset directory**: Validates structure and uploads as-is
- **Server-side dataset**: Creates snapshot from existing dataset in EdgeFirst Studio

##### Supported Input Structures

**1. Simple folder of images** (CLI handles conversion automatically):

```text
my_images/
├── image001.jpg
├── image002.jpg
└── image003.png
```

**2. Sequence-based dataset** (video frames with temporal ordering):

```text
my_dataset.arrow               # Annotation manifest
my_dataset/                    # Sensor container (or my_dataset.zip)
└── sequence_name/
    ├── sequence_name_001.camera.jpeg
    ├── sequence_name_002.camera.jpeg
    └── sequence_name_003.camera.jpeg
```

**3. Mixed dataset** (sequences + standalone images):

```text
my_dataset.arrow
my_dataset/
├── video_sequence/
│   └── video_sequence_*.camera.jpeg
├── standalone_image1.jpg
└── standalone_image2.png
```

##### CLI Examples

```bash
# Upload a folder of images (auto-generates Arrow manifest and ZIP)
edgefirst-client create-snapshot ./my_images/

# Upload using existing Arrow manifest (auto-discovers dataset.zip or dataset/)
edgefirst-client create-snapshot ./my_dataset/my_dataset.arrow

# Upload complete dataset directory
edgefirst-client create-snapshot ./my_dataset/

# Create snapshot from server-side dataset (with default annotation set)
edgefirst-client create-snapshot ds-12345

# Create snapshot from server-side dataset with specific annotation set
edgefirst-client create-snapshot ds-12345 --annotation-set as-67890

# Monitor server-side snapshot creation progress
edgefirst-client create-snapshot ds-12345 --monitor

# Generate Arrow manifest from images (without uploading)
edgefirst-client generate-arrow ./images --output dataset.arrow

# Generate with sequence detection for video frames
edgefirst-client generate-arrow ./frames -o video.arrow --detect-sequences

# Validate dataset structure before upload
edgefirst-client validate-snapshot ./my_dataset
edgefirst-client validate-snapshot ./my_dataset --verbose
```

##### Sequence Detection (`--detect-sequences`)

The `--detect-sequences` flag enables automatic detection of video frame sequences based on filename patterns. When enabled, the CLI parses filenames to identify temporal ordering.

**How it works:**

1. **Pattern matching**: Looks for `{name}_{frame}.{ext}` pattern (e.g., `video_001.jpg`, `camera_042.png`)
2. **Extracts frame number**: The trailing numeric part after the last underscore becomes the frame index
3. **Groups by name**: Files with the same prefix are grouped into sequences

**Detection behavior:**

| Input | `--detect-sequences` OFF | `--detect-sequences` ON |
|-------|--------------------------|-------------------------|
| `image.jpg` | name=`image`, frame=null | name=`image`, frame=null |
| `seq_001.jpg` | name=`seq_001`, frame=null | name=`seq`, frame=1 |
| `camera_042.camera.jpeg` | name=`camera_042`, frame=null | name=`camera`, frame=42 |
| `video/video_100.jpg` | name=`video_100`, frame=null | name=`video`, frame=100 |

**Supported structures:**

- **Nested**: `sequence_name/sequence_name_001.jpg` (frames in subdirectories)
- **Flattened**: `sequence_name_001.jpg` (frames at root level)

**⚠️ False positive considerations:**

Files with names like `model_v2.jpg` or `sample_2024.png` may be incorrectly detected as sequences when `--detect-sequences` is enabled. If your dataset contains non-sequence files with `_number` suffixes, consider:

- Renaming files to avoid the `_N` pattern (e.g., `model-v2.jpg`)
- Omitting `--detect-sequences` and manually organizing sequences into subdirectories

##### Supported File Types

**Images**: `.jpg`, `.jpeg`, `.png`, `.camera.jpeg`, `.camera.png`
**Point Clouds**: `.lidar.pcd` (LiDAR), `.radar.pcd` (Radar)
**Depth Maps**: `.depth.png` (16-bit PNG)
**Radar Cubes**: `.radar.png` (16-bit PNG with embedded dimension metadata)

See [DATASET_FORMAT.md](DATASET_FORMAT.md#radar-data-cube) for technical details on radar cube encoding.

##### Annotation Support

The `create-snapshot` command uploads datasets **with or without annotations**:

- **With annotations**: Provide an Arrow file containing annotations (see [DATASET_FORMAT.md](DATASET_FORMAT.md) for schema)
- **Without annotations**: The CLI generates an Arrow manifest with null annotation fields

When uploading unannotated datasets, EdgeFirst Studio can populate annotations via:

- **Manual annotation** in the Studio web interface
- **AGTG (Automated Ground-Truth Generation)** via `restore-snapshot --autolabel` (MCAP snapshots only)

**Note**: The CLI does not currently parse annotations from other formats (e.g., COCO, YOLO). To upload pre-annotated datasets from these formats, first convert them to EdgeFirst Dataset Format using the annotation schema in [DATASET_FORMAT.md](DATASET_FORMAT.md).

##### Rust API

```rust
use edgefirst_client::format::{
    generate_arrow_from_folder, validate_dataset_structure, ValidationIssue
};
use std::path::PathBuf;

// Generate Arrow manifest from images
let images_dir = PathBuf::from("./images");
let output = PathBuf::from("./dataset.arrow");
let count = generate_arrow_from_folder(&images_dir, &output, true)?;
println!("Generated manifest for {} images", count);

// Validate dataset structure before upload
let issues = validate_dataset_structure(&PathBuf::from("./my_dataset"))?;
for issue in &issues {
    match issue {
        ValidationIssue::MissingArrowFile { .. } => eprintln!("Error: {}", issue),
        ValidationIssue::MissingSensorContainer { .. } => eprintln!("Error: {}", issue),
        _ => println!("Warning: {}", issue),
    }
}
```

##### Python API

```python
from pathlib import Path
from edgefirst_client import Client

# Create snapshot from local folder (auto-generates manifest)
client = Client().with_token_path(None)
snapshot = client.create_snapshot("./my_images/")
print(f"Created snapshot: {snapshot.id()}")

# Create snapshot from server-side dataset
result = client.create_snapshot_from_dataset("ds-12345", "My backup")
print(f"Snapshot: {result.id}, Task: {result.task_id}")

# Create snapshot with explicit annotation set
result = client.create_snapshot_from_dataset(
    "ds-12345", "Backup with annotations", "as-67890"
)
```

For complete format specification, see [EdgeFirst Dataset Format Documentation](https://doc.edgefirst.ai/latest/datasets/format/) or [DATASET_FORMAT.md](DATASET_FORMAT.md).

### Rust Library

```rust
use edgefirst_client::{Client, TrainingSessionID};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client and authenticate
    let client = Client::new()?;
    let client = client.with_login("email@example.com", "password").await?;

    // List projects
    let projects = client.projects(None).await?;
    for project in projects {
        println!("Project: {} ({})", project.name(), project.id());

        // List datasets for this project
        let datasets = client.datasets(project.id(), None).await?;
        for dataset in datasets {
            println!("  Dataset: {}", dataset.name());
        }
    }

    // Publish training metrics (used by trainers/validators)
    // Note: Replace with your actual training session ID
    let session_id = TrainingSessionID::from(12345);
    use std::collections::HashMap;
    let session = client.training_session(session_id).await?;
    let mut metrics = HashMap::new();
    metrics.insert("loss".to_string(), 0.123.into());
    metrics.insert("accuracy".to_string(), 0.956.into());
    session.set_metrics(&client, metrics).await?;

    Ok(())
}
```

### Python Library

```python
from edgefirst_client import Client

# Create client and authenticate
client = Client()
client = client.with_login("email@example.com", "password")

# List projects and datasets
projects = client.projects()
for project in projects:
    print(f"Project: {project.name} ({project.id})")

    datasets = client.datasets(project.id)
    for dataset in datasets:
        print(f"  Dataset: {dataset.name}")

# Publish validation metrics (used by validators)
# Note: Replace with your actual validation session ID
session = client.validation_session("vs-12345")
metrics = {
    "mAP": 0.87,
    "precision": 0.92,
    "recall": 0.85
}
session.set_metrics(client, metrics)
```

## Architecture

EdgeFirst Client is a REST API client built with:

- **TLS 1.2+ enforcement** for secure communication with EdgeFirst Studio
- **Session token authentication** with automatic renewal
- **Progress tracking** for long-running uploads/downloads
- **Async operations** powered by Tokio runtime (Rust)
- **Memory-efficient streaming** for large dataset transfers

## Documentation

- **EdgeFirst Studio Docs**: [doc.edgefirst.ai](https://doc.edgefirst.ai)
- **Rust API Documentation**: [docs.rs/edgefirst-client](https://docs.rs/edgefirst-client)
- **Python API Documentation**: Available on [PyPI](https://pypi.org/project/edgefirst-client/)
- **Android SDK Documentation**: See [ANDROID.md](ANDROID.md)
- **iOS/macOS SDK Documentation**: See [APPLE.md](APPLE.md)
- **CLI Man Page**: See [CLI.md](CLI.md)
- **Dataset Format Specification**: [EdgeFirst Dataset Format](https://doc.edgefirst.ai/latest/datasets/format/)
- **AGTG Workflow Tutorial**: [Automated Ground-Truth Generation](https://doc.edgefirst.ai/latest/datasets/tutorials/annotations/automatic/)

## Support

### Community Resources

- 📚 **[Documentation](https://doc.edgefirst.ai)** - Comprehensive guides and tutorials
- 💬 **[GitHub Discussions](https://github.com/orgs/EdgeFirstAI/discussions)** - Ask questions and share ideas
- 🐛 **[Issue Tracker](https://github.com/EdgeFirstAI/edgefirst-client/issues)** - Report bugs and request features

### EdgeFirst Ecosystem

This client is the official API gateway for **[EdgeFirst Studio](https://edgefirst.studio)** - the complete MLOps platform for 3D visual and 4D spatial perception AI:

**🚀 EdgeFirst Studio Features:**

- **Dataset Management**: Organize, annotate, and version your perception datasets
- **Automated Ground-Truth Generation (AGTG)**: Upload MCAP recordings and get automatic annotations
- **Model Training**: Train custom perception models with your datasets
- **Validation & Testing**: Comprehensive model validation and performance analysis
- **Deployment**: Deploy models to edge devices with optimized inference
- **Monitoring**: Real-time performance monitoring and analytics
- **Collaboration**: Team workspaces and project management

**💰 Free Tier Available:**

- 100,000 images
- 10 hours of training per month
- Full access to all features
- No credit card required

**[Try EdgeFirst Studio Free →](https://edgefirst.studio)**

### Hardware Platforms

EdgeFirst Client works seamlessly with **[EdgeFirst Modules](https://www.edgefirst.ai/edgefirstmodules)**:

- Operates reliably in harsh conditions with an IP67-rated enclosure and -40°C to +65°C range
- On-device integrated dataset collection, playback, and publishing
- Deploy models onto EdgeFirst Modules with full AI Acceleration up-to 40-TOPS
- Reference designs and custom hardware development services

### Professional Services

Au-Zone Technologies offers comprehensive support for production deployments:

- **Training & Workshops** - Accelerate your team's expertise with EdgeFirst Studio
- **Custom Development** - Extend capabilities for your specific use cases
- **Integration Services** - Seamlessly connect with your existing systems and workflows
- **Enterprise Support** - SLAs, priority fixes, and dedicated support channels

📧 **Contact**: [support@au-zone.com](mailto:support@au-zone.com)
🌐 **Learn more**: [au-zone.com](https://au-zone.com)

## Contributing

Contributions are welcome! Please:

1. Read the [Contributing Guidelines](CONTRIBUTING.md)
2. Check [existing issues](https://github.com/EdgeFirstAI/edgefirst-client/issues) or create a new one
3. Fork the repository and create a feature branch
4. Submit a pull request with clear descriptions

**Using AI Coding Agents?** See [AGENTS.md](AGENTS.md) for project conventions, build commands, and pre-commit requirements.

### Code Quality

This project uses [SonarCloud](https://sonarcloud.io/project/overview?id=EdgeFirstAI_client) for automated code quality analysis. Contributors can download findings and use GitHub Copilot to help fix issues:

```bash
python3 sonar.py --branch main --output sonar-issues.json --verbose
```

See [CONTRIBUTING.md](CONTRIBUTING.md#sonarcloud-code-quality-analysis) for details.

## Security

For security vulnerabilities, please use our responsible disclosure process:

- **GitHub Security Advisories**: [Report a vulnerability](https://github.com/EdgeFirstAI/client/security/advisories)
- **Email**: support@au-zone.com with subject "[SECURITY] EdgeFirst Client"

See [SECURITY.md](SECURITY.md) for complete security policy and best practices.

## License

Licensed under the Apache License 2.0 - see [LICENSE](LICENSE) for details.

**Copyright 2025 Au-Zone Technologies**

See [NOTICE](NOTICE.md) for third-party software attributions included in binary releases.

---

**🚀 Ready to streamline your perception AI workflows?**

[Try EdgeFirst Studio Free](https://edgefirst.studio) - No credit card required • 100,000 images • 10 hours training/month
