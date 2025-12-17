// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for domain struct types (offline tests).
///
/// These tests verify Dataset, Experiment, Project, Organization,
/// Snapshot, AnnotationSet, and Artifact struct construction,
/// equality, and hashability without requiring API access.

import XCTest

@testable import EdgeFirstClient

final class DomainStructTests: XCTestCase {

  // MARK: - Dataset Tests

  /// Test Dataset construction.
  func testDatasetConstruction() {
    let dataset = Dataset(
      id: DatasetId(value: 100),
      projectId: ProjectId(value: 50),
      name: "COCO Dataset",
      description: "Common Objects in Context",
      created: "2024-01-15T10:00:00Z"
    )

    XCTAssertEqual(dataset.id.value, 100)
    XCTAssertEqual(dataset.projectId.value, 50)
    XCTAssertEqual(dataset.name, "COCO Dataset")
    XCTAssertEqual(dataset.description, "Common Objects in Context")
    XCTAssertEqual(dataset.created, "2024-01-15T10:00:00Z")
  }

  /// Test Dataset equality.
  func testDatasetEquality() {
    let dataset1 = Dataset(
      id: DatasetId(value: 1),
      projectId: ProjectId(value: 1),
      name: "Dataset",
      description: "Test",
      created: "2024-01-01"
    )
    let dataset2 = Dataset(
      id: DatasetId(value: 1),
      projectId: ProjectId(value: 1),
      name: "Dataset",
      description: "Test",
      created: "2024-01-01"
    )
    let dataset3 = Dataset(
      id: DatasetId(value: 2),
      projectId: ProjectId(value: 1),
      name: "Dataset",
      description: "Test",
      created: "2024-01-01"
    )

    XCTAssertEqual(dataset1, dataset2)
    XCTAssertNotEqual(dataset1, dataset3)
  }

  /// Test Dataset hashability.
  func testDatasetHashability() {
    var datasetSet: Set<Dataset> = []

    datasetSet.insert(Dataset(
      id: DatasetId(value: 1),
      projectId: ProjectId(value: 1),
      name: "D1", description: "", created: ""
    ))
    datasetSet.insert(Dataset(
      id: DatasetId(value: 2),
      projectId: ProjectId(value: 1),
      name: "D2", description: "", created: ""
    ))
    datasetSet.insert(Dataset(
      id: DatasetId(value: 1),
      projectId: ProjectId(value: 1),
      name: "D1", description: "", created: ""
    ))  // Duplicate

    XCTAssertEqual(datasetSet.count, 2)
  }

  /// Test Dataset with empty strings.
  func testDatasetEmptyStrings() {
    let dataset = Dataset(
      id: DatasetId(value: 1),
      projectId: ProjectId(value: 1),
      name: "",
      description: "",
      created: ""
    )

    XCTAssertTrue(dataset.name.isEmpty)
    XCTAssertTrue(dataset.description.isEmpty)
  }

  /// Test Dataset with unicode name.
  func testDatasetUnicodeName() {
    let dataset = Dataset(
      id: DatasetId(value: 1),
      projectId: ProjectId(value: 1),
      name: "数据集 🎯",
      description: "中文描述",
      created: "2024-01-01"
    )

    XCTAssertEqual(dataset.name, "数据集 🎯")
    XCTAssertEqual(dataset.description, "中文描述")
  }

  // MARK: - Experiment Tests

  /// Test Experiment construction.
  func testExperimentConstruction() {
    let experiment = Experiment(
      id: ExperimentId(value: 200),
      projectId: ProjectId(value: 50),
      name: "YOLOv5 Training",
      description: "Object detection experiment"
    )

    XCTAssertEqual(experiment.id.value, 200)
    XCTAssertEqual(experiment.projectId.value, 50)
    XCTAssertEqual(experiment.name, "YOLOv5 Training")
    XCTAssertEqual(experiment.description, "Object detection experiment")
  }

  /// Test Experiment equality.
  func testExperimentEquality() {
    let exp1 = Experiment(
      id: ExperimentId(value: 1),
      projectId: ProjectId(value: 1),
      name: "Experiment",
      description: "Test"
    )
    let exp2 = Experiment(
      id: ExperimentId(value: 1),
      projectId: ProjectId(value: 1),
      name: "Experiment",
      description: "Test"
    )
    let exp3 = Experiment(
      id: ExperimentId(value: 2),
      projectId: ProjectId(value: 1),
      name: "Experiment",
      description: "Test"
    )

    XCTAssertEqual(exp1, exp2)
    XCTAssertNotEqual(exp1, exp3)
  }

  /// Test Experiment hashability.
  func testExperimentHashability() {
    var expSet: Set<Experiment> = []

    expSet.insert(Experiment(
      id: ExperimentId(value: 1),
      projectId: ProjectId(value: 1),
      name: "E1", description: ""
    ))
    expSet.insert(Experiment(
      id: ExperimentId(value: 2),
      projectId: ProjectId(value: 1),
      name: "E2", description: ""
    ))
    expSet.insert(Experiment(
      id: ExperimentId(value: 1),
      projectId: ProjectId(value: 1),
      name: "E1", description: ""
    ))  // Duplicate

    XCTAssertEqual(expSet.count, 2)
  }

  /// Test Experiment with long description.
  func testExperimentLongDescription() {
    let longDesc = String(repeating: "Test experiment. ", count: 100)
    let experiment = Experiment(
      id: ExperimentId(value: 1),
      projectId: ProjectId(value: 1),
      name: "Test",
      description: longDesc
    )

    XCTAssertTrue(experiment.description.count > 1000)
  }

  // MARK: - Project Tests

  /// Test Project construction.
  func testProjectConstruction() {
    let project = Project(
      id: ProjectId(value: 300),
      name: "Autonomous Driving",
      description: "Self-driving car perception models"
    )

    XCTAssertEqual(project.id.value, 300)
    XCTAssertEqual(project.name, "Autonomous Driving")
    XCTAssertEqual(project.description, "Self-driving car perception models")
  }

  /// Test Project equality.
  func testProjectEquality() {
    let proj1 = Project(
      id: ProjectId(value: 1),
      name: "Project",
      description: "Test"
    )
    let proj2 = Project(
      id: ProjectId(value: 1),
      name: "Project",
      description: "Test"
    )
    let proj3 = Project(
      id: ProjectId(value: 2),
      name: "Project",
      description: "Test"
    )

    XCTAssertEqual(proj1, proj2)
    XCTAssertNotEqual(proj1, proj3)
  }

  /// Test Project hashability.
  func testProjectHashability() {
    var projSet: Set<Project> = []

    projSet.insert(Project(id: ProjectId(value: 1), name: "P1", description: ""))
    projSet.insert(Project(id: ProjectId(value: 2), name: "P2", description: ""))
    projSet.insert(Project(id: ProjectId(value: 1), name: "P1", description: ""))  // Duplicate

    XCTAssertEqual(projSet.count, 2)
  }

  /// Test Project as dictionary key.
  func testProjectAsDictionaryKey() {
    var projectData: [Project: Int] = [:]

    let proj1 = Project(id: ProjectId(value: 1), name: "P1", description: "")
    let proj2 = Project(id: ProjectId(value: 2), name: "P2", description: "")

    projectData[proj1] = 100
    projectData[proj2] = 200

    XCTAssertEqual(projectData[proj1], 100)
    XCTAssertEqual(projectData[proj2], 200)
  }

  // MARK: - Organization Tests

  /// Test Organization construction.
  func testOrganizationConstruction() {
    let org = Organization(
      id: OrganizationId(value: 400),
      name: "Au-Zone Technologies",
      credits: 10000
    )

    XCTAssertEqual(org.id.value, 400)
    XCTAssertEqual(org.name, "Au-Zone Technologies")
    XCTAssertEqual(org.credits, 10000)
  }

  /// Test Organization equality.
  func testOrganizationEquality() {
    let org1 = Organization(
      id: OrganizationId(value: 1),
      name: "Org",
      credits: 100
    )
    let org2 = Organization(
      id: OrganizationId(value: 1),
      name: "Org",
      credits: 100
    )
    let org3 = Organization(
      id: OrganizationId(value: 2),
      name: "Org",
      credits: 100
    )

    XCTAssertEqual(org1, org2)
    XCTAssertNotEqual(org1, org3)
  }

  /// Test Organization hashability.
  func testOrganizationHashability() {
    var orgSet: Set<Organization> = []

    orgSet.insert(Organization(id: OrganizationId(value: 1), name: "O1", credits: 0))
    orgSet.insert(Organization(id: OrganizationId(value: 2), name: "O2", credits: 0))
    orgSet.insert(Organization(id: OrganizationId(value: 1), name: "O1", credits: 0))  // Duplicate

    XCTAssertEqual(orgSet.count, 2)
  }

  /// Test Organization with zero credits.
  func testOrganizationZeroCredits() {
    let org = Organization(
      id: OrganizationId(value: 1),
      name: "Free Tier",
      credits: 0
    )

    XCTAssertEqual(org.credits, 0)
  }

  /// Test Organization with large credits.
  func testOrganizationLargeCredits() {
    let org = Organization(
      id: OrganizationId(value: 1),
      name: "Enterprise",
      credits: Int64.max
    )

    XCTAssertEqual(org.credits, Int64.max)
  }

  /// Test Organization with negative credits.
  func testOrganizationNegativeCredits() {
    let org = Organization(
      id: OrganizationId(value: 1),
      name: "Overdrawn",
      credits: -500
    )

    XCTAssertEqual(org.credits, -500)
  }

  // MARK: - Snapshot Tests

  /// Test Snapshot construction.
  func testSnapshotConstruction() {
    let snapshot = Snapshot(
      id: SnapshotId(value: 500),
      description: "Training data v1.0",
      status: "completed",
      path: "/snapshots/training-v1.0.zip",
      created: "2024-01-15T10:00:00Z"
    )

    XCTAssertEqual(snapshot.id.value, 500)
    XCTAssertEqual(snapshot.description, "Training data v1.0")
    XCTAssertEqual(snapshot.status, "completed")
    XCTAssertEqual(snapshot.path, "/snapshots/training-v1.0.zip")
    XCTAssertEqual(snapshot.created, "2024-01-15T10:00:00Z")
  }

  /// Test Snapshot equality.
  func testSnapshotEquality() {
    let snap1 = Snapshot(
      id: SnapshotId(value: 1),
      description: "Snapshot",
      status: "completed",
      path: "/path",
      created: "2024-01-01"
    )
    let snap2 = Snapshot(
      id: SnapshotId(value: 1),
      description: "Snapshot",
      status: "completed",
      path: "/path",
      created: "2024-01-01"
    )
    let snap3 = Snapshot(
      id: SnapshotId(value: 2),
      description: "Snapshot",
      status: "completed",
      path: "/path",
      created: "2024-01-01"
    )

    XCTAssertEqual(snap1, snap2)
    XCTAssertNotEqual(snap1, snap3)
  }

  /// Test Snapshot hashability.
  func testSnapshotHashability() {
    var snapSet: Set<Snapshot> = []

    snapSet.insert(Snapshot(
      id: SnapshotId(value: 1),
      description: "S1", status: "completed", path: "/s1", created: ""
    ))
    snapSet.insert(Snapshot(
      id: SnapshotId(value: 2),
      description: "S2", status: "pending", path: "/s2", created: ""
    ))
    snapSet.insert(Snapshot(
      id: SnapshotId(value: 1),
      description: "S1", status: "completed", path: "/s1", created: ""
    ))  // Duplicate

    XCTAssertEqual(snapSet.count, 2)
  }

  /// Test Snapshot with various statuses.
  func testSnapshotStatuses() {
    let statuses = ["pending", "processing", "completed", "failed", "cancelled"]

    for status in statuses {
      let snapshot = Snapshot(
        id: SnapshotId(value: 1),
        description: "Test",
        status: status,
        path: "/path",
        created: ""
      )
      XCTAssertEqual(snapshot.status, status)
    }
  }

  // MARK: - AnnotationSet Tests

  /// Test AnnotationSet construction.
  func testAnnotationSetConstruction() {
    let annotationSet = AnnotationSet(
      id: AnnotationSetId(value: 600),
      datasetId: DatasetId(value: 100),
      name: "Ground Truth",
      description: "Human-verified annotations",
      created: "2024-01-15T10:00:00Z"
    )

    XCTAssertEqual(annotationSet.id.value, 600)
    XCTAssertEqual(annotationSet.datasetId.value, 100)
    XCTAssertEqual(annotationSet.name, "Ground Truth")
    XCTAssertEqual(annotationSet.description, "Human-verified annotations")
    XCTAssertEqual(annotationSet.created, "2024-01-15T10:00:00Z")
  }

  /// Test AnnotationSet equality.
  func testAnnotationSetEquality() {
    let set1 = AnnotationSet(
      id: AnnotationSetId(value: 1),
      datasetId: DatasetId(value: 1),
      name: "Set",
      description: "Test",
      created: "2024-01-01"
    )
    let set2 = AnnotationSet(
      id: AnnotationSetId(value: 1),
      datasetId: DatasetId(value: 1),
      name: "Set",
      description: "Test",
      created: "2024-01-01"
    )
    let set3 = AnnotationSet(
      id: AnnotationSetId(value: 2),
      datasetId: DatasetId(value: 1),
      name: "Set",
      description: "Test",
      created: "2024-01-01"
    )

    XCTAssertEqual(set1, set2)
    XCTAssertNotEqual(set1, set3)
  }

  /// Test AnnotationSet hashability.
  func testAnnotationSetHashability() {
    var setCollection: Set<AnnotationSet> = []

    setCollection.insert(AnnotationSet(
      id: AnnotationSetId(value: 1),
      datasetId: DatasetId(value: 1),
      name: "AS1", description: "", created: ""
    ))
    setCollection.insert(AnnotationSet(
      id: AnnotationSetId(value: 2),
      datasetId: DatasetId(value: 1),
      name: "AS2", description: "", created: ""
    ))
    setCollection.insert(AnnotationSet(
      id: AnnotationSetId(value: 1),
      datasetId: DatasetId(value: 1),
      name: "AS1", description: "", created: ""
    ))  // Duplicate

    XCTAssertEqual(setCollection.count, 2)
  }

  /// Test AnnotationSet relationship to Dataset.
  func testAnnotationSetDatasetRelationship() {
    let datasetId = DatasetId(value: 100)

    let annotationSet1 = AnnotationSet(
      id: AnnotationSetId(value: 1),
      datasetId: datasetId,
      name: "Set1", description: "", created: ""
    )
    let annotationSet2 = AnnotationSet(
      id: AnnotationSetId(value: 2),
      datasetId: datasetId,
      name: "Set2", description: "", created: ""
    )

    XCTAssertEqual(annotationSet1.datasetId.value, annotationSet2.datasetId.value)
  }

  // MARK: - Artifact Tests

  /// Test Artifact construction.
  func testArtifactConstruction() {
    let artifact = Artifact(
      name: "model.onnx",
      modelType: "onnx"
    )

    XCTAssertEqual(artifact.name, "model.onnx")
    XCTAssertEqual(artifact.modelType, "onnx")
  }

  /// Test Artifact equality.
  func testArtifactEquality() {
    let artifact1 = Artifact(name: "model.onnx", modelType: "onnx")
    let artifact2 = Artifact(name: "model.onnx", modelType: "onnx")
    let artifact3 = Artifact(name: "model.pt", modelType: "pytorch")

    XCTAssertEqual(artifact1, artifact2)
    XCTAssertNotEqual(artifact1, artifact3)
  }

  /// Test Artifact hashability.
  func testArtifactHashability() {
    var artifactSet: Set<Artifact> = []

    artifactSet.insert(Artifact(name: "model.onnx", modelType: "onnx"))
    artifactSet.insert(Artifact(name: "model.pt", modelType: "pytorch"))
    artifactSet.insert(Artifact(name: "model.onnx", modelType: "onnx"))  // Duplicate

    XCTAssertEqual(artifactSet.count, 2)
  }

  /// Test Artifact with various model types.
  func testArtifactModelTypes() {
    let modelTypes = ["onnx", "pytorch", "tensorflow", "tflite", "coreml", "openvino"]

    for modelType in modelTypes {
      let artifact = Artifact(name: "model.\(modelType)", modelType: modelType)
      XCTAssertEqual(artifact.modelType, modelType)
    }
  }

  /// Test Artifact with path-like name.
  func testArtifactPathName() {
    let artifact = Artifact(
      name: "artifacts/models/best_model.onnx",
      modelType: "onnx"
    )

    XCTAssertTrue(artifact.name.contains("/"))
    XCTAssertTrue(artifact.name.hasSuffix(".onnx"))
  }

  // MARK: - SnapshotId Tests

  /// Test SnapshotId construction.
  func testSnapshotIdConstruction() {
    let id = SnapshotId(value: 12345)
    XCTAssertEqual(id.value, 12345)
  }

  /// Test SnapshotId equality.
  func testSnapshotIdEquality() {
    let id1 = SnapshotId(value: 100)
    let id2 = SnapshotId(value: 100)
    let id3 = SnapshotId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test SnapshotId hashability.
  func testSnapshotIdHashability() {
    var idSet: Set<SnapshotId> = []

    idSet.insert(SnapshotId(value: 1))
    idSet.insert(SnapshotId(value: 2))
    idSet.insert(SnapshotId(value: 1))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  // MARK: - AnnotationSetId Tests

  /// Test AnnotationSetId construction.
  func testAnnotationSetIdConstruction() {
    let id = AnnotationSetId(value: 67890)
    XCTAssertEqual(id.value, 67890)
  }

  /// Test AnnotationSetId equality.
  func testAnnotationSetIdEquality() {
    let id1 = AnnotationSetId(value: 100)
    let id2 = AnnotationSetId(value: 100)
    let id3 = AnnotationSetId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test AnnotationSetId hashability.
  func testAnnotationSetIdHashability() {
    var idSet: Set<AnnotationSetId> = []

    idSet.insert(AnnotationSetId(value: 1))
    idSet.insert(AnnotationSetId(value: 2))
    idSet.insert(AnnotationSetId(value: 1))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  // MARK: - Cross-Entity Relationship Tests

  /// Test Dataset-Project relationship.
  func testDatasetProjectRelationship() {
    let projectId = ProjectId(value: 50)

    let project = Project(
      id: projectId,
      name: "Test Project",
      description: ""
    )

    let dataset = Dataset(
      id: DatasetId(value: 100),
      projectId: projectId,
      name: "Test Dataset",
      description: "",
      created: ""
    )

    XCTAssertEqual(project.id.value, dataset.projectId.value)
  }

  /// Test Experiment-Project relationship.
  func testExperimentProjectRelationship() {
    let projectId = ProjectId(value: 50)

    let project = Project(
      id: projectId,
      name: "Test Project",
      description: ""
    )

    let experiment = Experiment(
      id: ExperimentId(value: 200),
      projectId: projectId,
      name: "Test Experiment",
      description: ""
    )

    XCTAssertEqual(project.id.value, experiment.projectId.value)
  }

  /// Test multiple entities in same project.
  func testMultipleEntitiesInProject() {
    let projectId = ProjectId(value: 50)

    let datasets = [
      Dataset(id: DatasetId(value: 1), projectId: projectId, name: "D1", description: "", created: ""),
      Dataset(id: DatasetId(value: 2), projectId: projectId, name: "D2", description: "", created: ""),
    ]

    let experiments = [
      Experiment(id: ExperimentId(value: 1), projectId: projectId, name: "E1", description: ""),
      Experiment(id: ExperimentId(value: 2), projectId: projectId, name: "E2", description: ""),
    ]

    for dataset in datasets {
      XCTAssertEqual(dataset.projectId.value, projectId.value)
    }

    for experiment in experiments {
      XCTAssertEqual(experiment.projectId.value, projectId.value)
    }
  }
}
