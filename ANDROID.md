# EdgeFirst Client SDK for Android

The EdgeFirst Client SDK provides native Android bindings for interacting with EdgeFirst Studio, enabling you to manage machine learning projects, datasets, experiments, and trained model snapshots from your Android applications.

## Requirements

- Android API level 24 (Android 7.0) or higher
- Kotlin 1.8.0 or higher
- Java 8 or higher

## Installation

### Manual Installation

1. Download `edgefirst-android-sdk-{version}.zip` from the [GitHub Releases](https://github.com/EdgeFirstAI/client/releases) page.

2. Extract the archive to your project's `libs` directory:

   ```
   app/
   ├── libs/
   │   ├── ai/edgefirst/client/
   │   │   └── EdgeFirstClient.kt
   │   └── jniLibs/
   │       ├── arm64-v8a/
   │       │   └── libedgefirst_client.so
   │       ├── armeabi-v7a/
   │       │   └── libedgefirst_client.so
   │       └── x86_64/
   │           └── libedgefirst_client.so
   ```

3. Add the JNA dependency to your `build.gradle.kts`:

   ```kotlin
   dependencies {
       implementation("net.java.dev.jna:jna:5.14.0@aar")
       implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.8.0")
   }
   ```

4. Configure source sets to include the SDK sources:

   ```kotlin
   android {
       sourceSets {
           getByName("main") {
               kotlin.srcDir("libs")
               jniLibs.srcDir("libs/jniLibs")
           }
       }
   }
   ```

## Quick Start

### Basic Usage

```kotlin
import ai.edgefirst.client.*

// Create a client and authenticate
val client = Client()
    .withServer("test")  // Use "test" server instance
    .withLogin("username", "password")

// List all projects
val projects = client.projects(null)
for (project in projects) {
    println("Project: ${project.name} (ID: ${project.id.value})")
}

// Get datasets for a project
val datasets = client.datasets(projects.first().id, null)

// Get experiments for a project
val experiments = client.experiments(projects.first().id, null)

// Logout and clear credentials
client.logout()
```

### Async/Coroutine Support

All API methods have async variants for use with Kotlin coroutines:

```kotlin
import kotlinx.coroutines.*

suspend fun fetchProjects(): List<Project> {
    val client = Client()
        .withServer("test")
        .withLoginAsync("username", "password")

    return client.projectsAsync(null)
}

// Usage in a CoroutineScope
lifecycleScope.launch {
    try {
        val projects = fetchProjects()
        // Update UI with projects
    } catch (e: ClientException) {
        // Handle error
    }
}
```

## Token Storage

By default, the SDK uses in-memory token storage. For production applications, you should implement persistent secure storage using Android's EncryptedSharedPreferences or the Android Keystore.

### Implementing Secure Token Storage

```kotlin
import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import ai.edgefirst.client.TokenStorage
import ai.edgefirst.client.StorageException

class SecureTokenStorage(context: Context) : TokenStorage {
    private val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    private val prefs = EncryptedSharedPreferences.create(
        context,
        "ai.edgefirst.client.secure_prefs",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    override fun store(token: String) {
        prefs.edit().putString("auth_token", token).apply()
    }

    override fun load(): String? {
        return prefs.getString("auth_token", null)
    }

    override fun clear() {
        prefs.edit().remove("auth_token").apply()
    }
}
```

### Using Custom Token Storage

```kotlin
val storage = SecureTokenStorage(applicationContext)
val client = createClientWithStorage(storage)
    .withServer("test")
    .withLogin("username", "password")

// Token is now automatically persisted
// On next app launch, create client with same storage to restore session
val restoredClient = createClientWithStorage(storage)
    .withServer("test")

// If token is valid, you can use the client immediately
try {
    restoredClient.verifyToken()
    // Token is valid, proceed with API calls
} catch (e: ClientException.AuthenticationException) {
    // Token expired or invalid, need to re-authenticate
}
```

Add the security crypto dependency to your `build.gradle.kts`:

```kotlin
dependencies {
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
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
| `createClientWithStorage(storage)` | Create with custom token storage |

#### Configuration Methods

| Method | Description |
|--------|-------------|
| `.withServer(name)` | Set the server instance ("test", "stage", "dev", or custom) |
| `.withToken(token)` | Set authentication token directly |
| `.withLogin(username, password)` | Authenticate with username and password |
| `.withLoginAsync(username, password)` | Async version for coroutines |

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
| `projects(name: String?)` | List projects, optionally filtered by name |
| `projectsAsync(name: String?)` | Async version |
| `project(id: ProjectId)` | Get a specific project by ID |
| `projectAsync(id: ProjectId)` | Async version |

#### Datasets

| Method | Description |
|--------|-------------|
| `datasets(projectId: ProjectId, name: String?)` | List datasets in a project |
| `datasetsAsync(projectId: ProjectId, name: String?)` | Async version |
| `dataset(id: DatasetId)` | Get a specific dataset by ID |
| `datasetAsync(id: DatasetId)` | Async version |
| `annotationSets(datasetId: DatasetId)` | Get annotation sets for a dataset |
| `annotationSetsAsync(datasetId: DatasetId)` | Async version |
| `labels(datasetId: DatasetId)` | Get labels defined in a dataset |
| `labelsAsync(datasetId: DatasetId)` | Async version |

#### Experiments

| Method | Description |
|--------|-------------|
| `experiments(projectId: ProjectId, name: String?)` | List experiments in a project |
| `experimentsAsync(projectId: ProjectId, name: String?)` | Async version |
| `experiment(id: ExperimentId)` | Get a specific experiment by ID |
| `experimentAsync(id: ExperimentId)` | Async version |
| `trainingSessions(experimentId: ExperimentId)` | List training sessions |
| `trainingSessionsAsync(experimentId: ExperimentId)` | Async version |
| `trainingSession(id: TrainingSessionId)` | Get a specific training session |
| `trainingSessionAsync(id: TrainingSessionId)` | Async version |
| `validationSessions(experimentId: ExperimentId)` | List validation sessions |
| `validationSessionsAsync(experimentId: ExperimentId)` | Async version |
| `artifacts(trainingSessionId: TrainingSessionId)` | List training artifacts |
| `artifactsAsync(trainingSessionId: TrainingSessionId)` | Async version |

#### Snapshots

| Method | Description |
|--------|-------------|
| `snapshots(name: String?)` | List model snapshots |
| `snapshotsAsync(name: String?)` | Async version |
| `snapshot(id: SnapshotId)` | Get a specific snapshot by ID |
| `snapshotAsync(id: SnapshotId)` | Async version |

#### Tasks

| Method | Description |
|--------|-------------|
| `taskInfo(id: TaskId)` | Get information about a background task |
| `taskInfoAsync(id: TaskId)` | Async version |

### Data Types

#### IDs

All entity IDs are wrapped in type-safe record classes:

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

#### Records

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

All methods can throw `ClientException` with the following variants:

| Exception | Description |
|-----------|-------------|
| `ClientException.AuthenticationException` | Authentication failed or token expired |
| `ClientException.NetworkException` | Network or HTTP error |
| `ClientException.InvalidParametersException` | Invalid parameters provided |
| `ClientException.NotFoundException` | Requested resource not found |
| `ClientException.StorageException` | Token storage operation failed |
| `ClientException.InternalException` | Unexpected internal error |

Example error handling:

```kotlin
try {
    val projects = client.projects(null)
} catch (e: ClientException.AuthenticationException) {
    // Re-authenticate
} catch (e: ClientException.NetworkException) {
    // Check network connectivity
} catch (e: ClientException) {
    // Handle other errors
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

## ProGuard / R8

If you're using ProGuard or R8, add the following rules:

```proguard
# EdgeFirst Client SDK
-keep class ai.edgefirst.client.** { *; }
-keepclassmembers class ai.edgefirst.client.** { *; }

# JNA
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
```

## Thread Safety

The `Client` class is thread-safe and can be shared across multiple threads. However, for optimal performance, we recommend:

- Creating a single `Client` instance and reusing it
- Using the async methods with coroutines for UI applications
- Implementing proper error handling for network failures

## Support

- [GitHub Issues](https://github.com/EdgeFirstAI/client/issues) - Report bugs or request features
- [EdgeFirst Studio Documentation](https://edgefirst.studio/docs) - Platform documentation

## License

Copyright (c) 2024-2025 Au-Zone Technologies Inc. All rights reserved.
