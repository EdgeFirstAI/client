// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Wiremock-backed integration tests for the DE-2565 surface.
//!
//! These tests stand up a local wiremock HTTP server, point the client at
//! it via [`Client::with_url`], and exercise the full request/response
//! cycle for the new methods without a live Studio server. The goal is
//! coverage of the wire paths that pure unit tests can't reach:
//!
//! * [`Client::rpc_download`] (binary stream, JSON file payload, JSON-RPC
//!   error envelope, HTTP 413, empty-parent guard for bare filenames)
//! * [`Client::post_multipart`] (success, HTTP 413, JSON-RPC error
//!   envelope)
//! * [`Client::job_run`] / [`Client::jobs`] / [`Client::job_stop`]
//! * [`TaskInfo::data_list`] / `upload_data` / `download_data` /
//!   `list_charts` / `get_chart` / `add_chart`
//! * [`ValidationSession::data_list`] / `upload_data` / `download_data`
//!
//! All tests are offline — they require no Studio credentials and run
//! deterministically in CI.
//!
//! Wire conventions in this client:
//!
//! * Regular JSON-RPC calls go to `POST {url}/api` with the method name
//!   in the JSON body (`{"method": "<name>", ...}`). Matched here via
//!   `body_partial_json`.
//! * Binary downloads (`Client::rpc_download`) hit the same `/api` path
//!   with the method in the body, but the response is raw bytes.
//! * Multipart uploads (`Client::post_multipart`) hit
//!   `POST {url}/api?method=<name>` with the method in the query string.
//!   Matched here via `query_param`.

use base64::Engine as _;
use edgefirst_client::{
    Client, DatasetID, Error, Parameter, SampleDimensionUpdate, SampleID, TaskID,
    ValidationSessionID,
};
use serde_json::json;
use serial_test::serial;
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD_NO_PAD.encode(bytes)
}

/// Build a minimal JWT carrying `server="test"` and a far-future `exp`,
/// so the client treats it as valid and never tries to renew during a
/// test. The signature is opaque — the client never verifies it.
fn fake_jwt() -> String {
    let header = b64(b"{\"alg\":\"none\",\"typ\":\"JWT\"}");
    // exp = 2_000_000_000 → 2033-05-18. Well past the test deadline.
    let payload = b64(b"{\"server\":\"test\",\"exp\":2000000000}");
    let signature = b64(b"signature");
    format!("{header}.{payload}.{signature}")
}

/// Build a Client pointed at `mock_url`, pre-seeded with a fake JWT so
/// `rpc()` skips the auth-renewal path. Token storage is in-memory.
fn client_for(mock_url: &str) -> Client {
    Client::new()
        .expect("Client::new")
        .with_memory_storage()
        .with_token(&fake_jwt())
        .expect("with_token")
        .with_url(mock_url)
        .expect("with_url")
}

/// Standard JSON-RPC envelope. The Studio RpcResponse type deserializes
/// `id` as a `String`, so the mock must serialize it that way.
fn rpc_result(result: serde_json::Value) -> serde_json::Value {
    json!({ "jsonrpc": "2.0", "id": "0", "result": result })
}

/// Standard JSON-RPC error envelope.
fn rpc_error(code: i32, message: &str) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": "0",
        "error": { "code": code, "message": message }
    })
}

/// Match a JSON-RPC body whose `method` field is exactly `m`.
fn rpc_method_body(m: &str) -> wiremock::matchers::BodyPartialJsonMatcher {
    body_partial_json(json!({ "method": m }))
}

// ---------------------------------------------------------------------------
// Mock setup for the auxiliary `task.info` / `val.get` round-trips used
// to obtain a TaskInfo / ValidationSession before exercising the real
// method under test.
// ---------------------------------------------------------------------------

async fn mock_task_info(server: &MockServer, id: u64) -> edgefirst_client::TaskInfo {
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "id": id,
            "type": "edgefirst-validator:2.10.0",
            "task_description": "wiremock test task",
            "status": "running",
        }))))
        .mount(server)
        .await;
    let client = client_for(&server.uri());
    client
        .task_info(TaskID::from(id))
        .await
        .expect("task_info via mock")
}

async fn mock_validation_session(
    server: &MockServer,
    id: u64,
) -> edgefirst_client::ValidationSession {
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("validate.session.get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "id": id,
            "experiment_id": 1,
            "training_session_id": 2,
            "dataset_id": 3,
            // Wire-rename: ValidationSession.annotation_set_id is serialized
            // as gt_annotation_set_id (matches server schema).
            "gt_annotation_set_id": 4,
            "description": "wiremock test session",
            // The session embeds the model `params` object (validator
            // params live under .model_params.validation) and the
            // underlying docker task — both required by the deserializer.
            "params": {
                "model_params": { "validation": {} },
                "validate_params": { "model": "wiremock-model" }
            },
            "docker_task": {
                "id": id,
                "name": "wiremock-task",
                "type": "edgefirst-validator:2.10.0",
                "status": "running",
                "manage_type": null,
                "instance_type": "wiremock",
                "date": "2026-05-15T00:00:00Z"
            }
        }))))
        .mount(server)
        .await;
    let client = client_for(&server.uri());
    client
        .validation_session(ValidationSessionID::from(id))
        .await
        .expect("validation_session via mock")
}

// ---------------------------------------------------------------------------
// rpc_download
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rpc_download_streams_binary_body_to_disk() {
    let server = MockServer::start().await;
    let payload: &[u8] = b"\x00\x01\x02 binary blob \xff\xfe\xfd";

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.data.download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(payload.to_vec())
                .insert_header("content-type", "application/octet-stream"),
        )
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let tmp = tempfile::NamedTempFile::new().unwrap();
    task.download_data(&client, "blob.bin", None, tmp.path(), None)
        .await
        .expect("binary download");
    assert_eq!(tokio::fs::read(tmp.path()).await.unwrap(), payload);
}

#[tokio::test]
async fn rpc_download_persists_json_artifact_when_not_an_envelope() {
    // A legit JSON metrics file with a `Content-Type: application/json` —
    // must NOT be misread as a JSON-RPC error envelope, even though it has
    // a free-form `error` field.
    let server = MockServer::start().await;
    let body = json!({
        "metric": "loss",
        "value": 0.42,
        "error": { "code": "ENV_NOT_FOUND", "message": "ignored" }
    });

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.data.download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(body.clone())
                .insert_header("content-type", "application/json"),
        )
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let tmp = tempfile::NamedTempFile::new().unwrap();
    task.download_data(&client, "metrics.json", None, tmp.path(), None)
        .await
        .expect("json artifact download");

    let on_disk: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(tmp.path()).await.unwrap()).unwrap();
    assert_eq!(on_disk, body);
}

#[tokio::test]
async fn rpc_download_decodes_jsonrpc_error_envelope_into_typed_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.data.download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(rpc_error(101, "Cannot find task with id 0x42"))
                .insert_header("content-type", "application/json"),
        )
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let err = task
        .download_data(&client, "any.bin", None, tmp.path(), None)
        .await
        .expect_err("envelope should surface as typed TaskNotFound");
    assert!(
        matches!(err, Error::TaskNotFound(_)),
        "expected TaskNotFound, got {err:?}"
    );
}

#[tokio::test]
async fn rpc_download_maps_http_413_to_payload_too_large() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.data.download"))
        .respond_with(ResponseTemplate::new(413).set_body_string("request too large"))
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let err = task
        .download_data(&client, "any.bin", None, tmp.path(), None)
        .await
        .expect_err("HTTP 413 should map to PayloadTooLarge");
    assert!(
        matches!(err, Error::PayloadTooLarge { .. }),
        "expected PayloadTooLarge, got {err:?}"
    );
}

/// RAII guard around `std::env::set_current_dir`.
///
/// CWD is process-global, so the `Drop` impl restores it even on panic.
/// Combined with `#[serial]` on the surrounding test, no other test
/// observes a mid-flight CWD.
struct CwdGuard {
    original: std::path::PathBuf,
}

impl CwdGuard {
    fn enter(new_cwd: &std::path::Path) -> std::io::Result<Self> {
        let original = std::env::current_dir()?;
        std::env::set_current_dir(new_cwd)?;
        Ok(Self { original })
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        // Best-effort restore — if this fails, the process is already in
        // a weird state and `expect` would just trip a double-panic.
        let _ = std::env::set_current_dir(&self.original);
    }
}

// `#[serial]` (single-threaded) is mandatory: CWD is process-wide, so
// running this in parallel with other tests would race their file-path
// assertions or fixture lookups. The `CwdGuard` restores CWD on drop so
// a panic mid-test still leaves the process clean for the next test.
#[tokio::test]
#[serial]
async fn rpc_download_accepts_bare_filename_without_dirname() {
    // Regression: `Path::parent` for "metrics.bin" returns `Some("")`, and
    // `create_dir_all("")` errors. The guard should let the bare filename
    // land in the current directory.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.data.download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"hello".to_vec())
                .insert_header("content-type", "application/octet-stream"),
        )
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let tmp_dir = tempfile::tempdir().unwrap();
    let bare = tmp_dir.path().join("metrics.bin");
    let _cwd = CwdGuard::enter(tmp_dir.path()).expect("enter tempdir");
    task.download_data(
        &client,
        "any.bin",
        None,
        std::path::Path::new("metrics.bin"),
        None,
    )
    .await
    .expect("bare filename download");
    assert_eq!(tokio::fs::read(&bare).await.unwrap(), b"hello");
    // `_cwd` drops here and restores CWD even if the assertions panic.
}

#[tokio::test]
async fn rpc_download_emits_completion_progress_for_json_path() {
    let server = MockServer::start().await;
    let body = json!({ "ok": true });

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.data.download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(body.clone())
                .insert_header("content-type", "application/json"),
        )
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    let tmp = tempfile::NamedTempFile::new().unwrap();
    task.download_data(&client, "metrics.json", None, tmp.path(), Some(tx))
        .await
        .expect("json download with progress");

    let mut last = None;
    while let Some(p) = rx.recv().await {
        last = Some(p);
    }
    let p = last.expect("at least one progress event");
    assert!(p.total > 0, "expected total > 0, got {}", p.total);
    assert_eq!(
        p.current, p.total,
        "completion progress should have current == total"
    );
}

// ---------------------------------------------------------------------------
// post_multipart — exercised via TaskInfo::upload_data and
// ValidationSession::upload_data.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn post_multipart_uploads_file_and_returns_ok() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(query_param("method", "task.data.upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "message": "ok",
            "path": "/uploads/1.bin",
            "size": 16,
        }))))
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let tmp = tempfile::NamedTempFile::new().unwrap();
    tokio::fs::write(tmp.path(), b"hello multipart!")
        .await
        .unwrap();

    task.upload_data(&client, tmp.path(), Some("predictions"), None)
        .await
        .expect("multipart upload");
}

#[tokio::test]
async fn post_multipart_maps_http_413_to_payload_too_large() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(query_param("method", "val.data.upload"))
        .respond_with(ResponseTemplate::new(413).set_body_string("too big"))
        .mount(&server)
        .await;

    let session = mock_validation_session(&server, 2707).await;
    let client = client_for(&server.uri());

    let tmp = tempfile::NamedTempFile::new().unwrap();
    tokio::fs::write(tmp.path(), b"payload").await.unwrap();

    let err = session
        .upload_data(
            &client,
            &[("result.bin".to_string(), tmp.path().to_path_buf())],
            None,
            None,
        )
        .await
        .expect_err("413 should map");
    assert!(
        matches!(err, Error::PayloadTooLarge { .. }),
        "expected PayloadTooLarge, got {err:?}"
    );
}

#[tokio::test]
async fn post_multipart_surfaces_jsonrpc_error_envelope() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(query_param("method", "task.data.upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_error(403, "forbidden")))
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let tmp = tempfile::NamedTempFile::new().unwrap();
    tokio::fs::write(tmp.path(), b"x").await.unwrap();

    let err = task
        .upload_data(&client, tmp.path(), None, None)
        .await
        .expect_err("403 envelope should map");
    assert!(
        matches!(err, Error::PermissionDenied(_)),
        "expected PermissionDenied, got {err:?}"
    );
}

#[tokio::test]
async fn upload_data_emits_terminal_progress_event() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(query_param("method", "task.data.upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "message": "ok",
            "path": "/",
            "size": 0,
        }))))
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let body = b"completion event probe payload";
    tokio::fs::write(tmp.path(), body).await.unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    task.upload_data(&client, tmp.path(), None, Some(tx))
        .await
        .expect("upload");

    let mut last = None;
    while let Some(p) = rx.recv().await {
        last = Some(p);
    }
    let p = last.expect("terminal progress must fire");
    assert_eq!(p.current, body.len());
    assert_eq!(p.total, body.len());
}

// ---------------------------------------------------------------------------
// job_run / jobs / job_stop
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_run_returns_full_job_record_from_bk_batch_shape() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("job.run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "code": "edgefirst-validator:2.10.0",
            "title": "EdgeFirst Validator",
            "job_name": "smoke",
            "job_id": "aws-batch-xyz",
            "state": "SUBMITTED",
            "task_id": 0x1234,
        }))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let job = client
        .job_run(
            "edgefirst-validator",
            "smoke",
            std::collections::HashMap::new(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("job.run");
    assert_eq!(job.code, "edgefirst-validator:2.10.0");
    assert_eq!(job.task_id, 0x1234);
    assert_eq!(job.task_id().value(), 0x1234);
}

#[tokio::test]
async fn job_run_surfaces_permission_denied() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("job.run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_error(403, "no access")))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let err = client
        .job_run(
            "edgefirst-validator",
            "smoke",
            std::collections::HashMap::new(),
            std::collections::HashMap::new(),
        )
        .await
        .expect_err("403 maps");
    assert!(matches!(err, Error::PermissionDenied(_)));
}

#[tokio::test]
async fn jobs_returns_vec_of_job() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("job.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([
            { "code": "a", "task_id": 1, "state": "RUNNING" },
            { "code": "b", "task_id": 2, "state": "SUCCEEDED" }
        ]))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let jobs = client.jobs(None).await.expect("job.list");
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].task_id, 1);
}

#[tokio::test]
async fn jobs_substring_filter_is_client_side() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("job.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([
            { "code": "a", "task_id": 1, "state": "RUNNING", "job_name": "alpha-run" },
            { "code": "b", "task_id": 2, "state": "SUCCEEDED", "job_name": "beta-run" }
        ]))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let filtered = client.jobs(Some("alpha")).await.expect("filtered job.list");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].job_name, "alpha-run");
}

#[tokio::test]
async fn job_stop_maps_typed_errors() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("job.stop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_error(101, "Cannot find task")))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let err = client
        .job_stop(TaskID::from(0xdeadu64))
        .await
        .expect_err("101 maps");
    assert!(matches!(err, Error::TaskNotFound(_)));
}

#[tokio::test]
async fn job_stop_success_returns_unit() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("job.stop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({}))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    client
        .job_stop(TaskID::from(0x42u64))
        .await
        .expect("job.stop");
}

// ---------------------------------------------------------------------------
// TaskInfo data + chart APIs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn task_data_list_returns_folder_map() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.data.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "server": "studio.example.com",
            "organization_uid": "org-1",
            "traces": ["trace/imx95.json"],
            "data": {
                "predictions": ["a.parquet", "b.parquet"],
                "metrics": ["loss.json"]
            }
        }))))
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let listing = task.data_list(&client).await.expect("task.data.list");
    assert_eq!(listing.server, "studio.example.com");
    assert_eq!(listing.data["predictions"].len(), 2);
    assert_eq!(listing.traces, vec!["trace/imx95.json".to_string()]);
}

#[tokio::test]
async fn task_data_list_maps_task_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.data.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_error(101, "Cannot find task")))
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let err = task.data_list(&client).await.expect_err("101 maps");
    assert!(matches!(err, Error::TaskNotFound(_)));
}

#[tokio::test]
async fn task_list_charts_round_trips() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.chart.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "server": "studio.example.com",
            "organization_uid": "org-1",
            "traces": [],
            "data": { "metrics": ["loss.json", "accuracy.json"] }
        }))))
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let listing = task
        .list_charts(&client, Some("metrics"))
        .await
        .expect("task.chart.list");
    assert_eq!(listing.data["metrics"].len(), 2);
}

#[tokio::test]
async fn task_get_chart_returns_parameter_body() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.chart.get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "type": "line",
            "series": [{ "x": [1, 2, 3], "y": [0.1, 0.2, 0.3] }]
        }))))
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let chart = task
        .get_chart(&client, "metrics", "loss")
        .await
        .expect("task.chart.get");
    let json: serde_json::Value = serde_json::to_value(&chart).unwrap();
    assert_eq!(json["type"], "line");
}

#[tokio::test]
async fn task_get_chart_rejects_empty_group_locally() {
    // validate_chart_args fires before the request — should not call the
    // server. We register no mock and assert we still get InvalidParameters.
    let server = MockServer::start().await;
    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let err = task
        .get_chart(&client, "", "loss")
        .await
        .expect_err("empty group rejected locally");
    assert!(matches!(err, Error::InvalidParameters(_)));
}

#[tokio::test]
async fn task_add_chart_round_trips_with_param_body() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("task.chart.add"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({}))))
        .mount(&server)
        .await;

    let task = mock_task_info(&server, 0x42).await;
    let client = client_for(&server.uri());

    let body = Parameter::Object(std::collections::HashMap::from([(
        "type".into(),
        Parameter::String("line".into()),
    )]));
    task.add_chart(&client, "metrics", "loss", body, None)
        .await
        .expect("task.chart.add");
}

// ---------------------------------------------------------------------------
// ValidationSession data APIs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn val_data_list_returns_flat_vec_of_strings() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("val.data.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([
            "trace/imx95.json",
            "metrics/loss.parquet"
        ]))))
        .mount(&server)
        .await;

    let session = mock_validation_session(&server, 2707).await;
    let client = client_for(&server.uri());

    let files = session.data_list(&client).await.expect("val.data.list");
    assert_eq!(files.len(), 2);
    assert!(files.contains(&"trace/imx95.json".to_string()));
}

#[tokio::test]
async fn val_data_download_writes_file() {
    let server = MockServer::start().await;
    let payload: &[u8] = b"validation result bytes";

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("val.data.download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(payload.to_vec())
                .insert_header("content-type", "application/octet-stream"),
        )
        .mount(&server)
        .await;

    let session = mock_validation_session(&server, 2707).await;
    let client = client_for(&server.uri());

    let tmp = tempfile::NamedTempFile::new().unwrap();
    session
        .download_data(&client, "result.bin", tmp.path(), None)
        .await
        .expect("val.data.download");
    assert_eq!(tokio::fs::read(tmp.path()).await.unwrap(), payload);
}

// ---------------------------------------------------------------------------
// `Client::start_validation_session` / `Client::delete_validation_sessions`
// ---------------------------------------------------------------------------

#[tokio::test]
async fn start_validation_session_round_trips_user_managed_request() {
    let server = MockServer::start().await;

    // The server returns a BackgroundTask row (subset of fields used by
    // our deserializer). `val_session_id` is the freshly-minted session
    // handle the caller will use for downstream data uploads and the
    // matching delete on teardown.
    Mock::given(method("POST"))
        .and(path("/api"))
        // The frontend send shape: `type` must be "validation" and
        // `is_local: true` flips the server to user-managed mode.
        .and(body_partial_json(json!({
            "method": "cloud.server.start",
            "params": {
                "type": "validation",
                "is_local": true,
                "is_kubernetes": false,
                "name": "smoke-session",
                "training_session_id": 0x111,
                "model_file": "best.pt",
                "val_type": "modelpack",
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "id": 0x1234,
            "val_session_id": 0x5678,
        }))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let req = edgefirst_client::StartValidationRequest {
        project_id: edgefirst_client::ProjectID::from(0x222u64),
        name: "smoke-session".into(),
        training_session_id: edgefirst_client::TrainingSessionID::from(0x111u64),
        model_file: "best.pt".into(),
        val_type: "modelpack".into(),
        params: std::collections::HashMap::new(),
        is_local: true,
        is_kubernetes: false,
        description: None,
        dataset_id: Some(edgefirst_client::DatasetID::from(0x333u64)),
        annotation_set_id: Some(edgefirst_client::AnnotationSetID::from(0x444u64)),
        snapshot_id: None,
    };

    let session = client
        .start_validation_session(req)
        .await
        .expect("cloud.server.start via mock");
    assert_eq!(session.task_id, TaskID::from(0x1234u64));
    assert_eq!(
        session.session_id,
        Some(ValidationSessionID::from(0x5678u64))
    );
}

#[tokio::test]
async fn delete_validation_sessions_passes_session_ids_array() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(body_partial_json(json!({
            "method": "validate.session.delete",
            "params": { "session_ids": [0x5678, 0x9abc] }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!("ok"))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    client
        .delete_validation_sessions(&[
            ValidationSessionID::from(0x5678u64),
            ValidationSessionID::from(0x9abcu64),
        ])
        .await
        .expect("validate.session.delete via mock");
}

#[tokio::test]
async fn delete_validation_sessions_maps_permission_denied() {
    let server = MockServer::start().await;
    // 100 = generic permission code on the Studio server.
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("validate.session.delete"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_error(100, "permission denied")))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let err = client
        .delete_validation_sessions(&[ValidationSessionID::from(0x5678u64)])
        .await
        .expect_err("expected permission failure");
    assert!(
        matches!(err, Error::PermissionDenied(_) | Error::RpcError(100, _)),
        "expected PermissionDenied or RpcError(100), got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// update_sample_dimensions
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_update_sample_dimensions_empty() {
    let client = client_for("http://localhost:1");
    // Empty updates should short-circuit without any RPC call.
    let updated = client
        .update_sample_dimensions(DatasetID::from(1u64), vec![])
        .await
        .expect("empty updates should succeed");
    assert_eq!(updated, 0);
}

#[tokio::test]
#[serial]
async fn test_update_sample_dimensions_single_batch() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.update_dimensions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "updated": 3 }))))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let updates = vec![
        SampleDimensionUpdate {
            id: SampleID::from(1u64),
            width: 640,
            height: 480,
        },
        SampleDimensionUpdate {
            id: SampleID::from(2u64),
            width: 1920,
            height: 1080,
        },
        SampleDimensionUpdate {
            id: SampleID::from(3u64),
            width: 800,
            height: 600,
        },
    ];
    let updated = client
        .update_sample_dimensions(DatasetID::from(42u64), updates)
        .await
        .expect("update should succeed");
    assert_eq!(updated, 3);
}

#[tokio::test]
#[serial]
async fn test_update_sample_dimensions_multi_batch() {
    let server = MockServer::start().await;

    // With 500-item batching, 501 items should produce exactly 2 RPC calls.
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.update_dimensions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "updated": 250 }))),
        )
        .expect(2)
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let updates: Vec<SampleDimensionUpdate> = (1..=501)
        .map(|i| SampleDimensionUpdate {
            id: SampleID::from(i as u64),
            width: 100,
            height: 100,
        })
        .collect();
    let updated = client
        .update_sample_dimensions(DatasetID::from(7u64), updates)
        .await
        .expect("batched update should succeed");
    // 250 + 250 from the two mocked responses
    assert_eq!(updated, 500);
}

// ---------------------------------------------------------------------------
// backfill_sample_dimensions
// ---------------------------------------------------------------------------

/// Minimal 1×1 red PNG (67 bytes). Used to mock image downloads so
/// `imagesize::blob_size` can extract dimensions.
fn png_1x1() -> Vec<u8> {
    // Pre-computed minimal valid PNG with IHDR(1×1, 8-bit RGB), IDAT, IEND.
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR length + type
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, // 8-bit RGB + CRC
        0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT length + type
        0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, // deflated data
        0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, // + CRC
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND
        0xAE, 0x42, 0x60, 0x82, // IEND CRC
    ]
}

#[tokio::test]
#[serial]
async fn test_backfill_sample_dimensions_no_missing() {
    let server = MockServer::start().await;

    // labels
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("label.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([]))))
        .mount(&server)
        .await;

    // samples.count
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.count"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "total": 1 }))))
        .mount(&server)
        .await;

    // samples.list - all samples already have dimensions
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "samples": [{
                "id": 10,
                "image_name": "img001.png",
                "group_name": "train",
                "width": 640,
                "height": 480,
            }],
            "continue_token": null
        }))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let updated = client
        .backfill_sample_dimensions(DatasetID::from(1u64), None)
        .await
        .expect("backfill should succeed when nothing to update");
    assert_eq!(updated, 0);
}

#[tokio::test]
#[serial]
async fn test_backfill_sample_dimensions_with_missing() {
    let server = MockServer::start().await;

    // labels
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("label.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([]))))
        .mount(&server)
        .await;

    // samples.count
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.count"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "total": 2 }))))
        .mount(&server)
        .await;

    // samples.list - one has dimensions, one does not
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "samples": [
                {
                    "id": 10,
                    "image_name": "img001.png",
                    "group_name": "train",
                    "width": 640,
                    "height": 480,
                },
                {
                    "id": 20,
                    "image_name": "img002.png",
                    "image_url": format!("{}/images/img002.png", server.uri()),
                    "group_name": "train",
                }
            ],
            "continue_token": null
        }))))
        .mount(&server)
        .await;

    // Mock image download for the sample missing dimensions
    Mock::given(method("GET"))
        .and(path("/images/img002.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(png_1x1()))
        .expect(1)
        .mount(&server)
        .await;

    // samples.update_dimensions
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.update_dimensions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "updated": 1 }))))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    let updated = client
        .backfill_sample_dimensions(DatasetID::from(1u64), Some(tx))
        .await
        .expect("backfill should succeed");
    assert_eq!(updated, 1);

    // Verify progress was emitted
    let mut progress_messages = vec![];
    while let Ok(p) = rx.try_recv() {
        progress_messages.push(p);
    }
    assert_eq!(
        progress_messages.len(),
        1,
        "should emit exactly 1 progress update for 1 sample missing dims"
    );
    assert_eq!(progress_messages[0].current, 1);
    assert_eq!(progress_messages[0].total, 1);
}

#[tokio::test]
#[serial]
async fn test_backfill_sample_dimensions_no_image_url() {
    let server = MockServer::start().await;

    // labels
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("label.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([]))))
        .mount(&server)
        .await;

    // samples.count
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.count"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "total": 1 }))))
        .mount(&server)
        .await;

    // samples.list - sample has no image_url and no dimensions
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "samples": [{
                "id": 30,
                "image_name": "img003.png",
                "group_name": "test",
            }],
            "continue_token": null
        }))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    // Should gracefully skip sample with no image_url and update 0.
    let updated = client
        .backfill_sample_dimensions(DatasetID::from(1u64), None)
        .await
        .expect("backfill should succeed even with no image URL");
    assert_eq!(updated, 0);
}

#[tokio::test]
#[serial]
async fn test_backfill_sample_dimensions_no_image_url_with_progress() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("label.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([]))))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.count"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "total": 1 }))))
        .mount(&server)
        .await;

    // Sample with no image_url and no dimensions
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "samples": [{
                "id": 30,
                "image_name": "img003.png",
                "group_name": "test",
            }],
            "continue_token": null
        }))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    let updated = client
        .backfill_sample_dimensions(DatasetID::from(1u64), Some(tx))
        .await
        .expect("backfill should succeed");
    assert_eq!(updated, 0);

    // Progress should still be emitted for the skipped sample
    let mut progress_messages = vec![];
    while let Ok(p) = rx.try_recv() {
        progress_messages.push(p);
    }
    assert_eq!(progress_messages.len(), 1);
    assert_eq!(progress_messages[0].current, 1);
    assert_eq!(progress_messages[0].total, 1);
}

#[tokio::test]
#[serial]
async fn test_backfill_sample_dimensions_download_failure() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("label.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([]))))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.count"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "total": 1 }))))
        .mount(&server)
        .await;

    // Sample with image_url pointing to an unreachable host (connection refused)
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "samples": [{
                "id": 40,
                "image_name": "broken.png",
                "image_url": "http://127.0.0.1:1/images/broken.png",
                "group_name": "train",
            }],
            "continue_token": null
        }))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    let updated = client
        .backfill_sample_dimensions(DatasetID::from(1u64), Some(tx))
        .await
        .expect("backfill should succeed even when download fails");
    assert_eq!(updated, 0);

    // Progress still emitted for skipped sample
    let mut progress_messages = vec![];
    while let Ok(p) = rx.try_recv() {
        progress_messages.push(p);
    }
    assert_eq!(progress_messages.len(), 1);
}

#[tokio::test]
#[serial]
async fn test_backfill_sample_dimensions_http_error_status() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("label.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([]))))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.count"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "total": 1 }))))
        .mount(&server)
        .await;

    // Sample with image_url that returns HTTP 404
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "samples": [{
                "id": 45,
                "image_name": "missing.png",
                "image_url": format!("{}/images/missing.png", server.uri()),
                "group_name": "train",
            }],
            "continue_token": null
        }))))
        .mount(&server)
        .await;

    // Image download returns 404 Not Found
    Mock::given(method("GET"))
        .and(path("/images/missing.png"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    let updated = client
        .backfill_sample_dimensions(DatasetID::from(1u64), Some(tx))
        .await
        .expect("backfill should succeed even with HTTP error");
    assert_eq!(updated, 0);

    // Progress still emitted for skipped sample
    let mut progress_messages = vec![];
    while let Ok(p) = rx.try_recv() {
        progress_messages.push(p);
    }
    assert_eq!(progress_messages.len(), 1);
}

#[tokio::test]
#[serial]
async fn test_backfill_sample_dimensions_invalid_image_data() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("label.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([]))))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.count"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "total": 1 }))))
        .mount(&server)
        .await;

    // Sample with image_url that returns garbage bytes
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "samples": [{
                "id": 50,
                "image_name": "garbage.png",
                "image_url": format!("{}/images/garbage.png", server.uri()),
                "group_name": "train",
            }],
            "continue_token": null
        }))))
        .mount(&server)
        .await;

    // Return non-image bytes (imagesize::blob_size will fail)
    Mock::given(method("GET"))
        .and(path("/images/garbage.png"))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(b"this is not a valid image file".to_vec()),
        )
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    let updated = client
        .backfill_sample_dimensions(DatasetID::from(1u64), Some(tx))
        .await
        .expect("backfill should succeed even with invalid image data");
    assert_eq!(updated, 0);

    // Progress still emitted for skipped sample
    let mut progress_messages = vec![];
    while let Ok(p) = rx.try_recv() {
        progress_messages.push(p);
    }
    assert_eq!(progress_messages.len(), 1);
}

#[tokio::test]
#[serial]
async fn test_backfill_sample_dimensions_null_id_with_progress() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("label.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!([]))))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.count"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({ "total": 1 }))))
        .mount(&server)
        .await;

    // Sample with null id (missing dimensions, id is null)
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("samples.list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "samples": [{
                "id": null,
                "image_name": "orphan.png",
                "group_name": "train",
            }],
            "continue_token": null
        }))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    let updated = client
        .backfill_sample_dimensions(DatasetID::from(1u64), Some(tx))
        .await
        .expect("backfill should succeed with null id sample");
    assert_eq!(updated, 0);

    // Progress still emitted
    let mut progress_messages = vec![];
    while let Ok(p) = rx.try_recv() {
        progress_messages.push(p);
    }
    assert_eq!(progress_messages.len(), 1);
}

// ---------------------------------------------------------------------------
// usage_summary (accounting.get_usage_summary)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn usage_summary_parses_accounting_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("accounting.get_usage_summary"))
        .respond_with(ResponseTemplate::new(200).set_body_json(rpc_result(json!({
            "credits": 12.5,
            "funds": 49092.92,
            // Renamed on the wire to `total` in UsageSummary.
            "total_funds_and_credits": 49105.42
        }))))
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let usage = client
        .usage_summary()
        .await
        .expect("usage_summary via mock");
    assert_eq!(usage.credits(), 12.5);
    assert_eq!(usage.funds(), 49092.92);
    assert_eq!(usage.total(), 49105.42);
}

#[tokio::test]
async fn usage_summary_surfaces_jsonrpc_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api"))
        .and(rpc_method_body("accounting.get_usage_summary"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(rpc_error(-32000, "no billing account")),
        )
        .mount(&server)
        .await;

    let client = client_for(&server.uri());
    let err = client
        .usage_summary()
        .await
        .expect_err("usage_summary should surface the JSON-RPC error");
    assert!(
        matches!(err, Error::RpcError(-32000, _)),
        "expected RpcError(-32000, _), got {err:?}"
    );
}
