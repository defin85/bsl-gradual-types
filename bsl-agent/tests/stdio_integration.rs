use std::path::{Path, PathBuf};

use bsl_agent::types::{
    BslSymbolSearchResponse, BuildInfoResponse, ContextExpandResponse, ContextPackResponse,
    JobStartResponse, JobStateDto, JobStatusResponse, UiUrlResponse, WorkspaceDocumentsSetResponse,
    WorkspaceListResponse, WorkspaceOpenResponse, WorkspaceStatusResponse,
};
use bsl_shared::api::dtos::{AnalysisResultDto, MetricsDto, SnapshotMetaDto};
use rmcp::model::CallToolRequestParam;
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceError, ServiceExt};
use serde::de::DeserializeOwned;
use serde_json::json;
use tokio::process::Command;

fn bsl_agent_bin() -> &'static str {
    env!("CARGO_BIN_EXE_bsl-agent")
}

async fn spawn_agent(extra_env: &[(&str, &str)]) -> RunningService<RoleClient, ()> {
    let mut cmd = Command::new(bsl_agent_bin()).configure(|cmd| {
        cmd.env("RUST_LOG", "error");
    });
    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    let transport = TokioChildProcess::new(cmd).expect("spawn bsl-agent");
    ().serve(transport).await.expect("connect client")
}

fn json_object(value: serde_json::Value) -> rmcp::model::JsonObject {
    value.as_object().expect("json object").clone()
}

fn extract_json_text(result: rmcp::model::CallToolResult) -> serde_json::Value {
    let content = result.content.into_iter().next().expect("tool content");
    let text = content.as_text().expect("text content").text.clone();
    serde_json::from_str(&text).expect("json decode")
}

async fn call_tool<T: DeserializeOwned>(
    service: &RunningService<RoleClient, ()>,
    name: &'static str,
    args: serde_json::Value,
) -> T {
    let result = service
        .call_tool(CallToolRequestParam {
            name: name.into(),
            arguments: Some(json_object(args)),
        })
        .await
        .expect("call_tool");

    let value = extract_json_text(result);
    serde_json::from_value(value).expect("decode result")
}

async fn call_tool_expect_invalid_params(
    service: &RunningService<RoleClient, ()>,
    name: &'static str,
    args: serde_json::Value,
    message_contains: &str,
) {
    let err = service
        .call_tool(CallToolRequestParam {
            name: name.into(),
            arguments: Some(json_object(args)),
        })
        .await
        .expect_err("expected error");

    let ServiceError::McpError(err) = err else {
        panic!("unexpected error: {err:?}");
    };
    assert_eq!(err.code.0, rmcp::model::ErrorCode::INVALID_PARAMS.0);
    assert!(
        err.message.contains(message_contains),
        "message={:?} does not contain {:?}",
        err.message,
        message_contains
    );
}

fn ensure_dir_empty(path: &Path) {
    let mut entries = std::fs::read_dir(path).expect("read_dir");
    assert!(
        entries.next().is_none(),
        "expected empty dir: {}",
        path.display()
    );
}

fn ensure_dir_empty_except_bsl_agent_state(path: &Path) {
    let entries = std::fs::read_dir(path).expect("read_dir");
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name != "bsl-agent-state" {
            panic!(
                "expected cache dir to contain only bsl-agent-state, found: {}",
                entry.path().display()
            );
        }
    }
}

fn ensure_dir_has_non_state_entries(path: &Path) {
    let entries = std::fs::read_dir(path).expect("read_dir");
    let mut has_non_state = false;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name != "bsl-agent-state" {
            has_non_state = true;
            break;
        }
    }
    assert!(
        has_non_state,
        "expected disk cache artifacts outside bsl-agent-state in {}",
        path.display()
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

async fn wait_job_terminal(
    service: &RunningService<RoleClient, ()>,
    job_id: &str,
) -> JobStatusResponse {
    loop {
        let status: JobStatusResponse = call_tool(
            service,
            "job_wait",
            json!({ "job_id": job_id, "timeout_ms": 60_000 }),
        )
        .await;

        match status.state {
            JobStateDto::Queued | JobStateDto::Running => continue,
            _ => return status,
        }
    }
}

async fn wait_job_succeeded(service: &RunningService<RoleClient, ()>, job_id: &str) {
    let status = wait_job_terminal(service, job_id).await;
    assert_eq!(
        status.state,
        JobStateDto::Succeeded,
        "job did not succeed: state={:?} error={:?}",
        status.state,
        status.error
    );
}

async fn wait_workspace_ready(
    service: &RunningService<RoleClient, ()>,
    open: &WorkspaceOpenResponse,
) -> WorkspaceStatusResponse {
    let startup_job_id = open
        .startup_job_id
        .as_deref()
        .expect("startup_job_id missing");

    wait_job_succeeded(service, startup_job_id).await;

    loop {
        let status: WorkspaceStatusResponse = call_tool(
            service,
            "workspace_status",
            json!({ "session_id": &open.session_id }),
        )
        .await;
        if status.ready {
            return status;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn stdio_tools_list_and_lifecycle_smoke() {
    let service = spawn_agent(&[]).await;

    let tools = service.list_all_tools().await.expect("list_all_tools");
    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();

    for required in [
        "ui_url",
        "workspace_open",
        "workspace_status",
        "workspace_close",
        "workspace_resume",
        "workspace_list",
        "workspace_documents_set",
        "workspace_documents_clear",
        "job_status",
        "job_wait",
        "job_result",
        "job_cancel",
        "bsl_diagnostics_start",
        "bsl_symbol_search_start",
        "bsl_type_at_position_start",
        "bsl_members_start",
        "bsl_definition_start",
        "bsl_references_start",
        "context_pack_start",
        "context_expand_start",
    ] {
        assert!(
            names.contains(&required),
            "missing tool {required}; got {names:?}"
        );
    }

    let temp_root = tempfile::TempDir::new().expect("tempdir");
    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [temp_root.path().to_string_lossy()],
        }),
    )
    .await;
    let session_id = open.session_id.clone();

    let _status = wait_workspace_ready(&service, &open).await;

    let _list: WorkspaceListResponse = call_tool(&service, "workspace_list", json!({})).await;

    let _close: serde_json::Value = call_tool(
        &service,
        "workspace_close",
        json!({ "session_id": &session_id }),
    )
    .await;

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_ui_url_disabled_returns_not_enabled() {
    let service = spawn_agent(&[]).await;
    let resp: UiUrlResponse = call_tool(&service, "ui_url", json!({})).await;
    assert!(!resp.enabled);
    assert!(resp.ui_url.is_none());
    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_ui_url_enabled_returns_url() {
    let cache_dir = tempfile::TempDir::new().expect("tempdir");
    let static_dir = tempfile::TempDir::new().expect("tempdir");
    std::fs::write(
        static_dir.path().join("index.html"),
        "<!doctype html><html><body>MCP UI test</body></html>",
    )
    .expect("write index.html");

    let service = spawn_agent(&[
        ("BSL_CACHE_DIR", cache_dir.path().to_string_lossy().as_ref()),
        ("BSL_AGENT_HTTP_ADDR", "127.0.0.1:0"),
        (
            "BSL_AGENT_HTTP_STATIC_DIR",
            static_dir.path().to_string_lossy().as_ref(),
        ),
    ])
    .await;

    let resp: UiUrlResponse = call_tool(&service, "ui_url", json!({})).await;
    assert!(resp.enabled);
    let url = resp.ui_url.expect("ui_url");
    assert!(url.starts_with("http://localhost:"), "ui_url={url:?}");

    let client = reqwest::Client::new();
    let status = client
        .get(format!("{url}/api/mcp/status"))
        .send()
        .await
        .expect("GET /api/mcp/status");
    assert!(status.status().is_success());

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_build_info_returns_version() {
    let service = spawn_agent(&[]).await;
    let resp: BuildInfoResponse = call_tool(&service, "build_info", json!({})).await;

    assert_eq!(resp.package, env!("CARGO_PKG_NAME"));
    assert_eq!(resp.version, env!("CARGO_PKG_VERSION"));
    assert!(!resp.profile.is_empty());
    assert!(!resp.target.is_empty());
    assert!(resp.pid > 0);

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_http_ui_parity_endpoints_return_dtos_when_ready() {
    let cache_dir = tempfile::TempDir::new().expect("tempdir");
    let static_dir = tempfile::TempDir::new().expect("tempdir");
    std::fs::write(
        static_dir.path().join("index.html"),
        "<!doctype html><html><body>MCP UI parity test</body></html>",
    )
    .expect("write index.html");

    let root = tempfile::TempDir::new().expect("root");
    std::fs::create_dir_all(root.path().join("src")).expect("mkdir");

    let service = spawn_agent(&[
        ("BSL_CACHE_DIR", cache_dir.path().to_string_lossy().as_ref()),
        ("BSL_AGENT_HTTP_ADDR", "127.0.0.1:0"),
        (
            "BSL_AGENT_HTTP_STATIC_DIR",
            static_dir.path().to_string_lossy().as_ref(),
        ),
    ])
    .await;

    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [root.path().to_string_lossy()],
        }),
    )
    .await;

    let resp: UiUrlResponse = call_tool(&service, "ui_url", json!({})).await;
    let url = resp.ui_url.expect("ui_url");

    let client = reqwest::Client::new();

    let status: WorkspaceStatusResponse = call_tool(
        &service,
        "workspace_status",
        json!({ "session_id": &open.session_id }),
    )
    .await;
    if !status.ready {
        let not_ready = client
            .get(format!(
                "{url}/api/mcp/types?page=1&limit=10&sessionId={}",
                open.session_id
            ))
            .send()
            .await
            .expect("GET /api/mcp/types (not ready)");
        assert_eq!(not_ready.status().as_u16(), 400);
    }

    let _ = wait_workspace_ready(&service, &open).await;

    let types: AnalysisResultDto = client
        .get(format!("{url}/api/mcp/types?page=1&limit=10"))
        .send()
        .await
        .expect("GET /api/mcp/types")
        .error_for_status()
        .expect("200 /api/mcp/types")
        .json()
        .await
        .expect("parse AnalysisResultDto");
    assert!(types.pagination.is_some(), "expected pagination for /types");

    let _search: AnalysisResultDto = client
        .get(format!("{url}/api/mcp/search?q=Test"))
        .send()
        .await
        .expect("GET /api/mcp/search")
        .error_for_status()
        .expect("200 /api/mcp/search")
        .json()
        .await
        .expect("parse AnalysisResultDto");

    let _metrics: MetricsDto = client
        .get(format!("{url}/api/mcp/metrics"))
        .send()
        .await
        .expect("GET /api/mcp/metrics")
        .error_for_status()
        .expect("200 /api/mcp/metrics")
        .json()
        .await
        .expect("parse MetricsDto");

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_workspace_open_rejects_empty_roots() {
    let service = spawn_agent(&[]).await;

    call_tool_expect_invalid_params(
        &service,
        "workspace_open",
        json!({ "roots": [] }),
        "roots must be non-empty",
    )
    .await;

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_workspace_open_deduplicates_roots() {
    let service = spawn_agent(&[]).await;
    let temp_root = tempfile::TempDir::new().expect("tempdir");
    let root_str = temp_root.path().to_string_lossy().to_string();

    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [root_str, temp_root.path().to_string_lossy()],
        }),
    )
    .await;

    assert_eq!(open.roots.len(), 1);

    let _ = wait_workspace_ready(&service, &open).await;

    let _close: serde_json::Value = call_tool(
        &service,
        "workspace_close",
        json!({ "session_id": &open.session_id }),
    )
    .await;

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_context_pack_stale_ids_are_rejected() {
    let service = spawn_agent(&[]).await;
    let temp_root = tempfile::TempDir::new().expect("tempdir");

    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [temp_root.path().to_string_lossy()],
        }),
    )
    .await;
    let session_id = open.session_id.clone();
    let root_id = open.roots[0].root_id.clone();

    let _ = wait_workspace_ready(&service, &open).await;

    let set: WorkspaceDocumentsSetResponse = call_tool(
        &service,
        "workspace_documents_set",
        json!({
            "session_id": &session_id,
            "files": [
                {
                    "doc": { "root_id": &root_id, "path": "src/CommonModules/Foo/Module.bsl" },
                    "text": "Procedure Test()\\n    A = 1;\\nEndProcedure\\n",
                    "version": 1
                }
            ],
            "mark_hot": true
        }),
    )
    .await;
    assert!(set.analysis_revision > 0);

    let pack_job: JobStartResponse = call_tool(
        &service,
        "context_pack_start",
        json!({
            "session_id": &session_id,
            "goal": "Test pack",
            "focus": {
                "kind": "position",
                "file": { "doc": { "root_id": &root_id, "path": "src/CommonModules/Foo/Module.bsl" } },
                "position": { "line": 1, "character": 4 }
            },
            "budget_chars": 800
        }),
    )
    .await;
    wait_job_succeeded(&service, &pack_job.job_id).await;
    let pack: ContextPackResponse = call_tool(
        &service,
        "job_result",
        json!({ "job_id": &pack_job.job_id }),
    )
    .await;
    let pack_id = pack.pack_id.clone();
    let item_id = pack.items[0].item_id.clone();

    let expand_job: JobStartResponse = call_tool(
        &service,
        "context_expand_start",
        json!({
            "session_id": &session_id,
            "pack_id": &pack_id,
            "item_id": &item_id,
            "budget_chars": 300
        }),
    )
    .await;
    wait_job_succeeded(&service, &expand_job.job_id).await;
    let expand: ContextExpandResponse = call_tool(
        &service,
        "job_result",
        json!({ "job_id": &expand_job.job_id }),
    )
    .await;
    assert!(expand.text.contains("```bsl"));

    let _bump: WorkspaceDocumentsSetResponse = call_tool(
        &service,
        "workspace_documents_set",
        json!({
            "session_id": &session_id,
            "files": [
                {
                    "doc": { "root_id": &root_id, "path": "src/CommonModules/Foo/Module.bsl" },
                    "text": "Procedure Test()\\n    A = 2;\\nEndProcedure\\n",
                    "version": 2
                }
            ],
            "mark_hot": true
        }),
    )
    .await;

    let stale_expand_job: JobStartResponse = call_tool(
        &service,
        "context_expand_start",
        json!({
            "session_id": &session_id,
            "pack_id": &pack_id,
            "item_id": &item_id,
            "budget_chars": 200
        }),
    )
    .await;
    let stale_status = wait_job_terminal(&service, &stale_expand_job.job_id).await;
    assert_eq!(stale_status.state, JobStateDto::Failed);

    call_tool_expect_invalid_params(
        &service,
        "job_result",
        json!({ "job_id": &stale_expand_job.job_id }),
        "stale or unknown pack_id/item_id",
    )
    .await;

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_workspace_documents_set_requires_version_with_text() {
    let service = spawn_agent(&[]).await;
    let temp_root = tempfile::TempDir::new().expect("tempdir");

    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [temp_root.path().to_string_lossy()],
        }),
    )
    .await;

    call_tool_expect_invalid_params(
        &service,
        "workspace_documents_set",
        json!({
            "session_id": &open.session_id,
            "files": [
                {
                    "doc": { "root_id": &open.roots[0].root_id, "path": "src/CommonModules/Foo/Module.bsl" },
                    "text": "Procedure Test() EndProcedure"
                }
            ],
            "mark_hot": true
        }),
        "version is required when text is provided",
    )
    .await;

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_workspace_documents_set_rejects_large_overlay() {
    let service = spawn_agent(&[]).await;
    let temp_root = tempfile::TempDir::new().expect("tempdir");

    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [temp_root.path().to_string_lossy()],
        }),
    )
    .await;

    let big_text = "x".repeat(2 * 1024 * 1024 + 1);
    call_tool_expect_invalid_params(
        &service,
        "workspace_documents_set",
        json!({
            "session_id": &open.session_id,
            "files": [
                {
                    "doc": { "root_id": &open.roots[0].root_id, "path": "src/CommonModules/Foo/Module.bsl" },
                    "text": big_text,
                    "version": 1
                }
            ],
            "mark_hot": true
        }),
        "MAX_OVERLAY_BYTES",
    )
    .await;

    let _ = service.cancel().await;
}

#[cfg(unix)]
#[tokio::test]
async fn stdio_symlink_escape_is_rejected() {
    use std::os::unix::fs::symlink;

    let service = spawn_agent(&[]).await;
    let root = tempfile::TempDir::new().expect("root");
    let outside = tempfile::TempDir::new().expect("outside");

    let outside_file = outside.path().join("outside.bsl");
    std::fs::write(&outside_file, "Procedure Test() EndProcedure\n").expect("write outside");

    let link_path = root.path().join("outside.bsl");
    symlink(&outside_file, &link_path).expect("symlink");

    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [root.path().to_string_lossy()],
        }),
    )
    .await;

    let _ = wait_workspace_ready(&service, &open).await;

    let start: JobStartResponse = call_tool(
        &service,
        "bsl_type_at_position_start",
        json!({
            "session_id": &open.session_id,
            "file": { "doc": { "root_id": &open.roots[0].root_id, "path": "outside.bsl" } },
            "position": { "line": 0, "character": 0 }
        }),
    )
    .await;
    let status = wait_job_terminal(&service, &start.job_id).await;
    assert_eq!(status.state, JobStateDto::Failed);

    call_tool_expect_invalid_params(
        &service,
        "job_result",
        json!({ "job_id": &start.job_id }),
        "path escapes roots",
    )
    .await;

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_disk_cache_can_be_disabled() {
    let cache_dir = tempfile::TempDir::new().expect("cache");

    let config_path = repo_root()
        .join("examples")
        .join("conf")
        .join("conf_test")
        .canonicalize()
        .expect("config fixture");

    ensure_dir_empty(cache_dir.path());

    // Cache enabled: expect at least some cache artifacts after startup with configuration.
    {
        let cache_dir_value = cache_dir.path().to_string_lossy().to_string();
        let service = spawn_agent(&[("BSL_CACHE_DIR", cache_dir_value.as_str())]).await;
        let temp_root = tempfile::TempDir::new().expect("root");

        let open: WorkspaceOpenResponse = call_tool(
            &service,
            "workspace_open",
            json!({
                "roots": [temp_root.path().to_string_lossy()],
                "configuration_path": config_path.to_string_lossy(),
                "platform_version": "8.3.25"
            }),
        )
        .await;
        let _ = wait_workspace_ready(&service, &open).await;

        let _ = service.cancel().await;
    }

    ensure_dir_has_non_state_entries(cache_dir.path());

    // Cache disabled: should not create new artifacts in a fresh cache dir.
    let cache_dir_disabled = tempfile::TempDir::new().expect("cache2");
    ensure_dir_empty(cache_dir_disabled.path());

    {
        let cache_dir_value = cache_dir_disabled.path().to_string_lossy().to_string();
        let service = spawn_agent(&[
            ("BSL_CACHE_DIR", cache_dir_value.as_str()),
            ("BSL_CACHE_DISABLE", "1"),
        ])
        .await;
        let temp_root = tempfile::TempDir::new().expect("root");

        let open: WorkspaceOpenResponse = call_tool(
            &service,
            "workspace_open",
            json!({
                "roots": [temp_root.path().to_string_lossy()],
                "configuration_path": config_path.to_string_lossy(),
                "platform_version": "8.3.25"
            }),
        )
        .await;
        let _ = wait_workspace_ready(&service, &open).await;

        let _ = service.cancel().await;
    }

    ensure_dir_empty_except_bsl_agent_state(cache_dir_disabled.path());
}

#[tokio::test]
async fn stdio_workspace_open_accepts_platform_docs_file() {
    let service = spawn_agent(&[]).await;
    let temp_root = tempfile::TempDir::new().expect("root");

    // Avoid writing extracted docs into the repo during tests: copy fixtures to a temp dir.
    // Use the smaller language HBK here, since this test only checks that file paths are accepted.
    let docs_dir = tempfile::TempDir::new().expect("docs_dir");
    let fixture_dir = repo_root().join("examples").join("syntax_helper");
    std::fs::copy(
        fixture_dir.join("shlang_ru.hbk"),
        docs_dir.path().join("shlang_ru.hbk"),
    )
    .expect("copy shlang_ru.hbk");
    let platform_docs = docs_dir.path().join("shlang_ru.hbk");

    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [temp_root.path().to_string_lossy()],
            "platform_docs_archive": platform_docs.to_string_lossy(),
            "platform_version": "8.3.25"
        }),
    )
    .await;

    let _ = wait_workspace_ready(&service, &open).await;

    let _close: serde_json::Value = call_tool(
        &service,
        "workspace_close",
        json!({ "session_id": &open.session_id }),
    )
    .await;

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_platform_docs_file_loads_platform_types_via_parent_dir() {
    let cache_dir = tempfile::TempDir::new().expect("tempdir");
    let static_dir = tempfile::TempDir::new().expect("tempdir");
    std::fs::write(
        static_dir.path().join("index.html"),
        "<!doctype html><html><body>MCP UI platform docs test</body></html>",
    )
    .expect("write index.html");

    let service = spawn_agent(&[
        ("BSL_CACHE_DIR", cache_dir.path().to_string_lossy().as_ref()),
        ("BSL_AGENT_HTTP_ADDR", "127.0.0.1:0"),
        (
            "BSL_AGENT_HTTP_STATIC_DIR",
            static_dir.path().to_string_lossy().as_ref(),
        ),
    ])
    .await;

    let root = tempfile::TempDir::new().expect("root");
    std::fs::create_dir_all(root.path().join("src")).expect("mkdir");

    // Copy both HBK files into a temp folder to avoid extracting large docs into the repo during tests.
    let docs_dir = tempfile::TempDir::new().expect("docs_dir");
    let fixture_dir = repo_root().join("examples").join("syntax_helper");
    std::fs::copy(
        fixture_dir.join("shcntx_ru.hbk"),
        docs_dir.path().join("shcntx_ru.hbk"),
    )
    .expect("copy shcntx_ru.hbk");
    std::fs::copy(
        fixture_dir.join("shlang_ru.hbk"),
        docs_dir.path().join("shlang_ru.hbk"),
    )
    .expect("copy shlang_ru.hbk");

    let platform_docs_file = docs_dir.path().join("shcntx_ru.hbk");
    let platform_docs_dir = docs_dir.path().to_path_buf();

    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [root.path().to_string_lossy()],
            "platform_docs_archive": platform_docs_file.to_string_lossy(),
            "platform_version": "8.3.25"
        }),
    )
    .await;

    let _ = wait_workspace_ready(&service, &open).await;

    let resp: UiUrlResponse = call_tool(&service, "ui_url", json!({})).await;
    let url = resp.ui_url.expect("ui_url");

    let client = reqwest::Client::new();
    let meta: SnapshotMetaDto = client
        .get(format!("{url}/api/mcp/deps/meta"))
        .send()
        .await
        .expect("GET /api/mcp/deps/meta")
        .error_for_status()
        .expect("200 /api/mcp/deps/meta")
        .json()
        .await
        .expect("parse SnapshotMetaDto");

    assert_eq!(
        meta.inputs.syntax_helper_path.as_deref(),
        Some(platform_docs_dir.to_string_lossy().as_ref())
    );
    assert!(
        meta.repository_stats.platform_types > 500,
        "expected platform types from syntax helper; got platform_types={}",
        meta.repository_stats.platform_types
    );

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_workspace_open_is_idempotent_for_same_params() {
    let service = spawn_agent(&[]).await;
    let root = tempfile::TempDir::new().expect("root");

    let open_a: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [root.path().to_string_lossy()],
        }),
    )
    .await;

    let open_b: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [root.path().to_string_lossy()],
        }),
    )
    .await;

    assert_eq!(open_a.session_id, open_b.session_id);
    assert_eq!(open_a.startup_job_id, open_b.startup_job_id);

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_workspace_open_rejects_second_session_with_different_params() {
    let service = spawn_agent(&[]).await;

    let root_a = tempfile::TempDir::new().expect("rootA");
    let root_b = tempfile::TempDir::new().expect("rootB");

    let _open_a: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [root_a.path().to_string_lossy()],
        }),
    )
    .await;

    call_tool_expect_invalid_params(
        &service,
        "workspace_open",
        json!({
            "roots": [root_b.path().to_string_lossy()],
        }),
        "only one session is allowed",
    )
    .await;

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_workspace_resume_rejects_second_session_when_another_is_active() {
    let cache_dir = tempfile::TempDir::new().expect("cache");
    let cache_dir_str = cache_dir.path().to_string_lossy().to_string();

    let root_a = tempfile::TempDir::new().expect("rootA");
    let root_b = tempfile::TempDir::new().expect("rootB");

    let service = spawn_agent(&[("BSL_CACHE_DIR", cache_dir_str.as_str())]).await;
    let open_a: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({ "roots": [root_a.path().to_string_lossy()] }),
    )
    .await;
    let session_id_a = open_a.session_id.clone();
    let _ = service.cancel().await;

    let service = spawn_agent(&[("BSL_CACHE_DIR", cache_dir_str.as_str())]).await;
    let _open_b: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({ "roots": [root_b.path().to_string_lossy()] }),
    )
    .await;

    call_tool_expect_invalid_params(
        &service,
        "workspace_resume",
        json!({ "session_id": session_id_a }),
        "only one session is allowed",
    )
    .await;

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_disk_cache_concurrent_startup_same_dir() {
    let cache_dir = tempfile::TempDir::new().expect("cache");
    let cache_dir_str = cache_dir.path().to_string_lossy().to_string();

    let config_path = repo_root()
        .join("examples")
        .join("conf")
        .join("conf_test")
        .canonicalize()
        .expect("config fixture");

    let service_a = spawn_agent(&[("BSL_CACHE_DIR", cache_dir_str.as_str())]).await;
    let service_b = spawn_agent(&[("BSL_CACHE_DIR", cache_dir_str.as_str())]).await;

    let temp_root_a = tempfile::TempDir::new().expect("rootA");
    let temp_root_b = tempfile::TempDir::new().expect("rootB");

    let open_a = call_tool::<WorkspaceOpenResponse>(
        &service_a,
        "workspace_open",
        json!({
            "roots": [temp_root_a.path().to_string_lossy()],
            "configuration_path": config_path.to_string_lossy(),
            "platform_version": "8.3.25"
        }),
    );
    let open_b = call_tool::<WorkspaceOpenResponse>(
        &service_b,
        "workspace_open",
        json!({
            "roots": [temp_root_b.path().to_string_lossy()],
            "configuration_path": config_path.to_string_lossy(),
            "platform_version": "8.3.25"
        }),
    );

    let (open_a, open_b) = tokio::join!(open_a, open_b);
    let _ = wait_workspace_ready(&service_a, &open_a).await;
    let _ = wait_workspace_ready(&service_b, &open_b).await;

    ensure_dir_has_non_state_entries(cache_dir.path());

    let _ = service_a.cancel().await;
    let _ = service_b.cancel().await;
}

#[tokio::test]
async fn stdio_workspace_resume_persists_jobs_and_results() {
    let cache_dir = tempfile::TempDir::new().expect("cache");
    let cache_dir_str = cache_dir.path().to_string_lossy().to_string();

    let root = tempfile::TempDir::new().expect("root");
    let module_path = root.path().join("src/CommonModules/Foo/Module.bsl");
    std::fs::create_dir_all(module_path.parent().expect("parent")).expect("mkdir");
    std::fs::write(
        &module_path,
        "Procedure TestProc()\n    A = 1;\nEndProcedure\n",
    )
    .expect("write module");

    let service = spawn_agent(&[("BSL_CACHE_DIR", cache_dir_str.as_str())]).await;
    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({ "roots": [root.path().to_string_lossy()] }),
    )
    .await;
    let _ = wait_workspace_ready(&service, &open).await;

    let start: JobStartResponse = call_tool(
        &service,
        "bsl_symbol_search_start",
        json!({
            "session_id": &open.session_id,
            "query": "TestProc",
            "limit": 50
        }),
    )
    .await;
    wait_job_succeeded(&service, &start.job_id).await;
    let result: BslSymbolSearchResponse =
        call_tool(&service, "job_result", json!({ "job_id": &start.job_id })).await;
    assert!(!result.symbols.is_empty(), "expected at least one symbol");

    let session_id = open.session_id.clone();
    let job_id = start.job_id.clone();

    let _ = service.cancel().await;

    let service = spawn_agent(&[("BSL_CACHE_DIR", cache_dir_str.as_str())]).await;
    let resumed: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_resume",
        json!({ "session_id": &session_id }),
    )
    .await;
    let _ = wait_workspace_ready(&service, &resumed).await;

    let result_after_restart: BslSymbolSearchResponse =
        call_tool(&service, "job_result", json!({ "job_id": &job_id })).await;
    assert_eq!(
        result_after_restart.symbols.len(),
        result.symbols.len(),
        "expected persisted job_result across restart"
    );

    let _ = service.cancel().await;
}
