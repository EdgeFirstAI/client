# EdgeFirst Client SDK for Swift

The EdgeFirst Client SDK provides native Swift bindings for interacting with EdgeFirst Studio, enabling you to manage machine learning projects, datasets, experiments, and trained model snapshots from your iOS and macOS applications.

## Requirements

- iOS 13.0+ / macOS 10.15+
- Swift 5.5+
- Xcode 14.0+

## Installation

### Swift Package Manager

1. Download `edgefirst-client-swift-{version}.zip` from the [GitHub Releases](https://github.com/EdgeFirstAI/client/releases) page.

2. Extract the archive to a location accessible to your Xcode project.

3. In Xcode, go to **File > Add Package Dependencies...**

4. Click **Add Local...** and select the extracted `EdgeFirstClient` directory.

5. Select your target and click **Add Package**.

### Manual Installation

1. Download and extract `edgefirst-client-swift-{version}.zip`.

2. Drag `EdgeFirstClient.xcframework` into your Xcode project.

3. Ensure the framework is set to **Embed & Sign** in your target's **Frameworks, Libraries, and Embedded Content**.

4. Add `EdgeFirstClient.swift` to your project sources.

## Quick Start

### Basic Usage

```swift
import EdgeFirstClient

// Create a client and authenticate
let client = try Client()
    .withServer("test")  // Use "test" server instance
    .withLogin(username: "username", password: "password")

// List all projects
let projects = try client.projects(name: nil)
for project in projects {
    print("Project: \(project.name) (ID: \(project.id.value))")
}

// Get datasets for a project
let datasets = try client.datasets(projectId: projects.first!.id, name: nil)

// Get experiments for a project
let experiments = try client.experiments(projectId: projects.first!.id, name: nil)

// Logout and clear credentials
try client.logout()
```

### Async/Await Support

All API methods have async variants for use with Swift concurrency:

```swift
import EdgeFirstClient

func fetchProjects() async throws -> [Project] {
    let client = try await Client()
        .withServer("test")
        .withLoginAsync(username: "username", password: "password")

    return try await client.projectsAsync(name: nil)
}

// Usage
Task {
    do {
        let projects = try await fetchProjects()
        // Update UI with projects
    } catch let error as ClientError {
        // Handle error
    }
}
```

## Token Storage

By default, the SDK uses in-memory token storage. For production applications, you should implement persistent secure storage using the iOS/macOS Keychain.

### Implementing Keychain Token Storage

```swift
import Foundation
import Security
import EdgeFirstClient

class KeychainTokenStorage: TokenStorage {
    private let service = "ai.edgefirst.client"
    private let account = "auth_token"

    func store(token: String) throws {
        let data = token.data(using: .utf8)!

        // Delete existing item first
        let deleteQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
        SecItemDelete(deleteQuery as CFDictionary)

        // Add new item
        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]

        let status = SecItemAdd(addQuery as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw StorageError.writeError(message: "Keychain write failed: \(status)")
        }
    }

    func load() throws -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        switch status {
        case errSecSuccess:
            guard let data = result as? Data,
                  let token = String(data: data, encoding: .utf8) else {
                return nil
            }
            return token
        case errSecItemNotFound:
            return nil
        default:
            throw StorageError.readError(message: "Keychain read failed: \(status)")
        }
    }

    func clear() throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]

        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw StorageError.clearError(message: "Keychain delete failed: \(status)")
        }
    }
}
```

### Using Custom Token Storage

```swift
let storage = KeychainTokenStorage()
let client = try createClientWithStorage(storage: storage)
    .withServer("test")
    .withLogin(username: "username", password: "password")

// Token is now automatically persisted in Keychain
// On next app launch, create client with same storage to restore session
let restoredClient = try createClientWithStorage(storage: storage)
    .withServer("test")

// If token is valid, you can use the client immediately
do {
    try restoredClient.verifyToken()
    // Token is valid, proceed with API calls
} catch ClientError.authenticationError {
    // Token expired or invalid, need to re-authenticate
}
```

## API Reference

### Client

The main entry point for interacting with EdgeFirst Studio.

#### Constructors

| Method | Description |
|--------|-------------|
| `Client()` | Create a new client with in-memory token storage |
| `Client.withMemoryStorage()` | Explicitly create with in-memory storage |
| `createClientWithStorage(storage:)` | Create with custom token storage |

#### Configuration Methods

| Method | Description |
|--------|-------------|
| `.withServer(_:)` | Set the server instance ("test", "stage", "dev", or custom) |
| `.withToken(_:)` | Set authentication token directly |
| `.withLogin(username:password:)` | Authenticate with username and password |
| `.withLoginAsync(username:password:)` | Async version for Swift concurrency |

#### Authentication

| Method | Description |
|--------|-------------|
| `verifyToken()` | Verify the current token is valid |
| `verifyTokenAsync()` | Async version |
| `logout()` | Clear authentication and stored token |
| `logoutAsync()` | Async version |

#### Organization

| Method | Description |
|--------|-------------|
| `organization()` | Get the current user's organization |
| `organizationAsync()` | Async version |

#### Projects

| Method | Description |
|--------|-------------|
| `projects(name:)` | List projects, optionally filtered by name |
| `projectsAsync(name:)` | Async version |
| `project(id:)` | Get a specific project by ID |
| `projectAsync(id:)` | Async version |

#### Datasets

| Method | Description |
|--------|-------------|
| `datasets(projectId:name:)` | List datasets in a project |
| `datasetsAsync(projectId:name:)` | Async version |
| `dataset(id:)` | Get a specific dataset by ID |
| `datasetAsync(id:)` | Async version |
| `annotationSets(datasetId:)` | Get annotation sets for a dataset |
| `annotationSetsAsync(datasetId:)` | Async version |
| `labels(datasetId:)` | Get labels defined in a dataset |
| `labelsAsync(datasetId:)` | Async version |

#### Experiments

| Method | Description |
|--------|-------------|
| `experiments(projectId:name:)` | List experiments in a project |
| `experimentsAsync(projectId:name:)` | Async version |
| `experiment(id:)` | Get a specific experiment by ID |
| `experimentAsync(id:)` | Async version |
| `trainingSessions(experimentId:)` | List training sessions |
| `trainingSessionsAsync(experimentId:)` | Async version |
| `trainingSession(id:)` | Get a specific training session |
| `trainingSessionAsync(id:)` | Async version |
| `validationSessions(experimentId:)` | List validation sessions |
| `validationSessionsAsync(experimentId:)` | Async version |
| `artifacts(trainingSessionId:)` | List training artifacts |
| `artifactsAsync(trainingSessionId:)` | Async version |

#### Snapshots

| Method | Description |
|--------|-------------|
| `snapshots(name:)` | List model snapshots |
| `snapshotsAsync(name:)` | Async version |
| `snapshot(id:)` | Get a specific snapshot by ID |
| `snapshotAsync(id:)` | Async version |

#### Tasks

| Method | Description |
|--------|-------------|
| `taskInfo(id:)` | Get information about a background task |
| `taskInfoAsync(id:)` | Async version |

### Data Types

#### IDs

All entity IDs are wrapped in type-safe structs:

- `OrganizationId` - Organization identifier
- `ProjectId` - Project identifier
- `DatasetId` - Dataset identifier
- `ExperimentId` - Experiment identifier
- `TrainingSessionId` - Training session identifier
- `ValidationSessionId` - Validation session identifier
- `SnapshotId` - Model snapshot identifier
- `TaskId` - Background task identifier
- `AnnotationSetId` - Annotation set identifier
- `SampleId` - Sample identifier
- `ImageId` - Image identifier
- `AppId` - Application identifier

#### Structs

| Type | Description |
|------|-------------|
| `Organization` | User's organization with name and ID |
| `Project` | ML project with name, description, and settings |
| `Dataset` | Dataset with samples and annotations |
| `Sample` | Individual data sample in a dataset |
| `AnnotationSet` | Set of annotations for a dataset |
| `Annotation` | Individual annotation with geometry data |
| `Label` | Label definition with name and color |
| `Experiment` | Training experiment configuration |
| `TrainingSession` | Training run with metrics and status |
| `ValidationSession` | Validation run with results |
| `Artifact` | Training artifact (model file, logs, etc.) |
| `Snapshot` | Deployed model snapshot |
| `TaskInfo` | Background task status and progress |

### Error Handling

All methods can throw `ClientError` with the following cases:

| Error | Description |
|-------|-------------|
| `.authenticationError(message:)` | Authentication failed or token expired |
| `.networkError(message:)` | Network or HTTP error |
| `.invalidParameters(message:)` | Invalid parameters provided |
| `.notFound(message:)` | Requested resource not found |
| `.storageError(message:)` | Token storage operation failed |
| `.internalError(message:)` | Unexpected internal error |

Example error handling:

```swift
do {
    let projects = try client.projects(name: nil)
} catch ClientError.authenticationError(let message) {
    // Re-authenticate
    print("Auth error: \(message)")
} catch ClientError.networkError(let message) {
    // Check network connectivity
    print("Network error: \(message)")
} catch let error as ClientError {
    // Handle other errors
    print("Error: \(error)")
}
```

## Server Instances

The SDK supports connecting to different EdgeFirst Studio instances:

| Instance | URL | Description |
|----------|-----|-------------|
| (default) | `https://edgefirst.studio` | Production SaaS |
| `"saas"` | `https://edgefirst.studio` | Production SaaS |
| `"test"` | `https://test.edgefirst.studio` | Test environment |
| `"stage"` | `https://stage.edgefirst.studio` | Staging environment |
| `"dev"` | `https://dev.edgefirst.studio` | Development environment |
| `"{name}"` | `https://{name}.edgefirst.studio` | Custom instance |

## SwiftUI Integration

Example of using the SDK in a SwiftUI application:

```swift
import SwiftUI
import EdgeFirstClient

class ProjectsViewModel: ObservableObject {
    @Published var projects: [Project] = []
    @Published var isLoading = false
    @Published var errorMessage: String?

    private var client: Client?

    func login(username: String, password: String) async {
        await MainActor.run { isLoading = true }

        do {
            client = try await Client()
                .withServer("test")
                .withLoginAsync(username: username, password: password)

            await fetchProjects()
        } catch let error as ClientError {
            await MainActor.run {
                errorMessage = error.localizedDescription
                isLoading = false
            }
        } catch {
            await MainActor.run {
                errorMessage = "Unexpected error: \(error)"
                isLoading = false
            }
        }
    }

    func fetchProjects() async {
        guard let client = client else { return }

        do {
            let fetchedProjects = try await client.projectsAsync(name: nil)
            await MainActor.run {
                projects = fetchedProjects
                isLoading = false
            }
        } catch let error as ClientError {
            await MainActor.run {
                errorMessage = error.localizedDescription
                isLoading = false
            }
        } catch {
            await MainActor.run {
                errorMessage = "Unexpected error: \(error)"
                isLoading = false
            }
        }
    }
}

struct ProjectsView: View {
    @StateObject private var viewModel = ProjectsViewModel()

    var body: some View {
        NavigationView {
            List(viewModel.projects, id: \.id.value) { project in
                VStack(alignment: .leading) {
                    Text(project.name)
                        .font(.headline)
                    if let description = project.description {
                        Text(description)
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .navigationTitle("Projects")
            .overlay {
                if viewModel.isLoading {
                    ProgressView()
                }
            }
            .alert("Error", isPresented: .constant(viewModel.errorMessage != nil)) {
                Button("OK") { viewModel.errorMessage = nil }
            } message: {
                Text(viewModel.errorMessage ?? "")
            }
        }
        .task {
            await viewModel.login(username: "user", password: "pass")
        }
    }
}
```

## Platform-Specific Notes

### iOS

- The SDK is built for both device (arm64) and simulator (arm64 + x86_64) architectures
- Minimum deployment target is iOS 13.0
- App Transport Security (ATS) is compatible with EdgeFirst Studio endpoints

### macOS

- The SDK is built as a universal binary (arm64 + x86_64)
- Minimum deployment target is macOS 10.15
- Supports both Intel and Apple Silicon Macs

## Thread Safety

The `Client` class is thread-safe and can be used from any thread or actor. For SwiftUI applications:

- Use `@MainActor` for UI updates
- Prefer async methods for network operations
- Create a single `Client` instance and reuse it

## Debugging

Enable verbose logging by setting the `EDGEFIRST_LOG` environment variable:

```swift
// In your scheme's environment variables
EDGEFIRST_LOG=debug
```

## Support

- [GitHub Issues](https://github.com/EdgeFirstAI/client/issues) - Report bugs or request features
- [EdgeFirst Studio Documentation](https://edgefirst.studio/docs) - Platform documentation

## License

Copyright (c) 2024-2025 Au-Zone Technologies Inc. All rights reserved.
