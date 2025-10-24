# EdgeFirst Studio Client

[![Test](https://github.com/EdgeFirstAI/client/workflows/CI/badge.svg)](https://github.com/EdgeFirstAI/client/actions/workflows/test.yml)
[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=EdgeFirstAI_client&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=EdgeFirstAI_client)
[![codecov](https://codecov.io/gh/EdgeFirstAI/client/branch/main/graph/badge.svg)](https://codecov.io/gh/EdgeFirstAI/client)
[![Crates.io](https://img.shields.io/crates/v/edgefirst-client.svg)](https://crates.io/crates/edgefirst-client)
[![PyPI](https://img.shields.io/pypi/v/edgefirst-client.svg)](https://pypi.org/project/edgefirst-client/)
[![Documentation](https://docs.rs/edgefirst-client/badge.svg)](https://docs.rs/edgefirst-client)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**EdgeFirst Studio Client** is a command-line application and library (Rust + Python) for programmatic access to [EdgeFirst Studio](https://edgefirst.studio), the MLOps platform for 3D visual and 4D spatial perception AI. Automate dataset management, annotation workflows, model training, validation, and deployment for off-road vehicles, robotics, construction equipment, and industrial applications.

## Overview

EdgeFirst Client enables developers to integrate EdgeFirst Studio's capabilities into their CI/CD pipelines, custom training workflows, and automated data processing systems. The client is used internally by EdgeFirst Studio's training and validation services, providing a battle-tested foundation for production workloads.

**Key capabilities:**
- 📦 **MCAP Publishing**: Upload sensor recordings for automated ground-truth generation (AGTG)
- 🏷️ **Dataset Management**: Download datasets and annotations in multiple formats
- 🎯 **Training & Validation**: Monitor sessions, publish metrics, manage model artifacts
- 🚀 **Model Artifacts**: Upload and download trained models (ONNX, TensorFlow Lite, H5, etc.)
- 📊 **Multiple Formats**: Darknet/YOLO, EdgeFirst Dataset Format (Arrow), user-defined formats

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

#### Upload MCAP and Create Dataset with AGTG

```bash
# Create snapshot from MCAP recording
edgefirst-client create-snapshot recording.mcap

# List available snapshots
edgefirst-client snapshots

# Restore snapshot with automatic annotation (COCO labels)
edgefirst-client restore-snapshot <PROJECT_ID> <SNAPSHOT_ID> \
  --dataset-name "Autonomous Vehicle Dataset" \
  --dataset-description "Highway driving scenarios" \
  --autolabel "person car truck bicycle motorcycle" \
  --autodepth
```

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
# Full mode: annotations + images
edgefirst-client upload-dataset <DATASET_ID> \
  --annotations annotations.arrow \
  --annotation-set-id <ANNOTATION_SET_ID> \
  --images ./images/

# Images-only mode: upload images without annotations
edgefirst-client upload-dataset <DATASET_ID> --images ./images/

# Auto-discovery: finds images in folder named after Arrow file
edgefirst-client upload-dataset <DATASET_ID> \
  --annotations data.arrow \
  --annotation-set-id <ANNOTATION_SET_ID>
  # Automatically looks for: data/, dataset/, data.zip, dataset.zip
```

**Upload Dataset Format**: The Arrow file must follow the [EdgeFirst Dataset Format](https://doc.edgefirst.ai/latest/datasets/format/) with columns: `name`, `frame`, `object_id`, `label`, `label_index`, `group`, `mask`, `box2d`, `box3d`. Key features:
- **Flexible parameters**: All parameters except `DATASET_ID` are optional (must provide at least one of `--annotations` or `--images`)
- **Auto-discovery**: If `--images` not specified, searches for folder/ZIP named after Arrow file or "dataset"
- **Images-only mode**: Upload images without annotations by omitting `--annotations` and `--annotation-set-id`
- **Warning system**: Warns if annotations provided without annotation_set_id (annotations will be skipped)
- **Samples without annotations**: Include row with `name`/`group` but null geometries
- **Multiple annotations per sample**: Multiple rows with same `name`
- **Multiple geometries per annotation**: `box2d`, `box3d`, and `mask` in same row belong to same annotation
- **Auto-generated object_id**: If multiple geometries appear in same row without `object_id`, a UUID is generated automatically

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

#### Work with Validation Sessions

```bash
# List validation sessions
edgefirst-client validation-sessions <PROJECT_ID>

# Get validation session details
edgefirst-client validation-session <SESSION_ID>
```

### Rust Library

```rust
use edgefirst_client::{Client, ProjectID};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client and authenticate
    let mut client = Client::new()?;
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
session = client.validation_session(session_id)
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
- **Dataset Format Specification**: [EdgeFirst Dataset Format](https://doc.edgefirst.ai/latest/datasets/format/)
- **AGTG Workflow Tutorial**: [Automated Ground-Truth Generation](https://doc.edgefirst.ai/latest/datasets/tutorials/annotations/automatic/)

## Support

- **Documentation**: [doc.edgefirst.ai](https://doc.edgefirst.ai)
- **Community Support**: [GitHub Discussions](https://github.com/orgs/EdgeFirstAI/discussions)
- **Bug Reports**: [GitHub Issues](https://github.com/EdgeFirstAI/edgefirst-client/issues)
- **Commercial Support**: [support@au-zone.com](mailto:support@au-zone.com)
- **Security Issues**: See [SECURITY.md](SECURITY.md)

For detailed support options and response expectations, see [SUPPORT.md](SUPPORT.md).

## Contributing

Contributions are welcome! Please:
1. Read the [Contributing Guidelines](CONTRIBUTING.md)
2. Check [existing issues](https://github.com/EdgeFirstAI/edgefirst-client/issues) or create a new one
3. Fork the repository and create a feature branch
4. Submit a pull request with clear descriptions

See [SECURITY.md](SECURITY.md) for security vulnerability reporting procedures.

## License

Licensed under the Apache License 2.0 - see [LICENSE](LICENSE) for details.

**Copyright 2025 Au-Zone Technologies**

See [NOTICE](NOTICE) for third-party software attributions included in binary releases.

---

**Try EdgeFirst Studio**: [edgefirst.studio](https://edgefirst.studio) - Free tier available with 10,000 images and 1 hour of training per month.
