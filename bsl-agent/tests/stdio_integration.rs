use std::path::{Path, PathBuf};

use bsl_agent::types::{
    BslDiagnosticsResponse, BslSymbolSearchResponse, BuildInfoResponse, ContextExpandResponse,
    ContextPackResponse, JobStartResponse, JobStateDto, JobStatusResponse, UiUrlResponse,
    WorkspaceDocumentsSetResponse, WorkspaceGetSettingsResponse, WorkspaceListResponse,
    WorkspaceOpenResponse, WorkspaceStatusResponse, WorkspaceUpdateSettingsResponse,
};
use bsl_shared::api::dtos::TypeDto;
use bsl_shared::api::dtos::{AnalysisResultDto, MetricsDto, SnapshotMetaDto};
use rmcp::model::CallToolRequestParam;
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceError, ServiceExt};
use serde::de::DeserializeOwned;
use serde_json::json;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

fn bsl_agent_bin() -> &'static str {
    env!("CARGO_BIN_EXE_bsl-agent")
}

async fn spawn_agent(extra_env: &[(&str, &str)]) -> RunningService<RoleClient, ()> {
    spawn_agent_in_dir(&repo_root(), extra_env).await
}

async fn spawn_agent_in_dir(
    cwd: &Path,
    extra_env: &[(&str, &str)],
) -> RunningService<RoleClient, ()> {
    let mut cmd = Command::new(bsl_agent_bin()).configure(|cmd| {
        cmd.env("RUST_LOG", "error");
        cmd.current_dir(cwd);
    });
    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    let transport = TokioChildProcess::new(cmd).expect("spawn bsl-agent");
    ().serve(transport).await.expect("connect client")
}

async fn run_agent_output_in_dir(cwd: &Path, extra_env: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(bsl_agent_bin());
    cmd.current_dir(cwd);
    cmd.env("RUST_LOG", "error");
    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    timeout(Duration::from_secs(5), cmd.output())
        .await
        .expect("bsl-agent output timeout")
        .expect("bsl-agent output")
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

    for tool in &tools {
        let description = tool.description.as_deref().unwrap_or_default();
        assert!(
            !description.contains('\n') && !description.contains('\r'),
            "tool description must be one-line: {} => {:?}",
            tool.name,
            description
        );
        assert!(
            description.len() <= 200,
            "tool description too long: {} => len={}",
            tool.name,
            description.len()
        );
    }

    for required in [
        "mcp_help",
        "ui_url",
        "workspace_open",
        "workspace_status",
        "workspace_get_settings",
        "workspace_update_settings",
        "workspace_get_observability_metrics",
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
        "bsl_types_list_start",
        "bsl_types_search_start",
        "bsl_type_get_start",
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
async fn stdio_workspace_settings_runtime_overrides_roundtrip() {
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
    let _status = wait_workspace_ready(&service, &open).await;

    let before_raw: serde_json::Value = call_tool(
        &service,
        "workspace_get_settings",
        json!({ "session_id": &session_id }),
    )
    .await;
    assert!(before_raw.get("envOverrides").is_some());
    assert!(before_raw.get("devEnvOverrides").is_some());
    assert!(before_raw.get("allowDevOverrides").is_some());
    assert!(before_raw.get("env_overrides").is_none());
    let before: WorkspaceGetSettingsResponse =
        serde_json::from_value(before_raw).expect("decode workspace_get_settings");
    assert!(before
        .runtime_config
        .get("effective")
        .and_then(|v| v.get("BSL_CACHE_DISABLE"))
        .is_some());

    let updated_raw: serde_json::Value = call_tool(
        &service,
        "workspace_update_settings",
        json!({
            "session_id": &session_id,
            "envOverrides": {
                "BSL_CACHE_DISABLE": true
            }
        }),
    )
    .await;
    assert!(updated_raw.get("envOverrides").is_some());
    assert!(updated_raw.get("devEnvOverrides").is_some());
    assert!(updated_raw.get("allowDevOverrides").is_some());
    assert!(updated_raw.get("env_overrides").is_none());
    let updated: WorkspaceUpdateSettingsResponse =
        serde_json::from_value(updated_raw).expect("decode workspace_update_settings");
    assert_eq!(
        updated
            .runtime_config
            .get("effective")
            .and_then(|v| v.get("BSL_CACHE_DISABLE"))
            .cloned()
            .unwrap_or_default(),
        serde_json::Value::Bool(true)
    );

    let legacy_snake_case: WorkspaceUpdateSettingsResponse = call_tool(
        &service,
        "workspace_update_settings",
        json!({
            "session_id": &session_id,
            "env_overrides": {
                "BSL_CACHE_DISABLE": false
            }
        }),
    )
    .await;
    assert_eq!(
        legacy_snake_case
            .runtime_config
            .get("effective")
            .and_then(|v| v.get("BSL_CACHE_DISABLE"))
            .cloned()
            .unwrap_or_default(),
        serde_json::Value::Bool(false)
    );

    let startup_only_report: WorkspaceUpdateSettingsResponse = call_tool(
        &service,
        "workspace_update_settings",
        json!({
            "session_id": &session_id,
            "envOverrides": {
                "BSL_CACHE_DIR": "/tmp/bsl-agent-restart-needed"
            }
        }),
    )
    .await;
    assert!(
        startup_only_report
            .report
            .requires_restart_keys
            .contains(&"BSL_CACHE_DIR".to_string()),
        "expected BSL_CACHE_DIR in requires_restart_keys, got {:?}",
        startup_only_report.report.requires_restart_keys
    );

    let removed: WorkspaceUpdateSettingsResponse = call_tool(
        &service,
        "workspace_update_settings",
        json!({
            "session_id": &session_id,
            "envOverrides": {
                "BSL_CACHE_DISABLE": null
            }
        }),
    )
    .await;
    assert_eq!(
        removed
            .runtime_config
            .get("effective")
            .and_then(|v| v.get("BSL_CACHE_DISABLE"))
            .cloned()
            .unwrap_or_default(),
        serde_json::Value::Bool(false)
    );

    let dev_ignored: WorkspaceUpdateSettingsResponse = call_tool(
        &service,
        "workspace_update_settings",
        json!({
            "session_id": &session_id,
            "allowDevOverrides": false,
            "devEnvOverrides": {
                "BSL_COMPLETION_TRACE": true
            }
        }),
    )
    .await;
    assert!(dev_ignored.report.dev_overrides_ignored);

    let dev_enabled: WorkspaceUpdateSettingsResponse = call_tool(
        &service,
        "workspace_update_settings",
        json!({
            "session_id": &session_id,
            "allow_dev_overrides": true,
            "dev_env_overrides": {
                "BSL_COMPLETION_TRACE": true
            }
        }),
    )
    .await;
    assert_eq!(
        dev_enabled
            .runtime_config
            .get("effective")
            .and_then(|v| v.get("BSL_COMPLETION_TRACE"))
            .cloned()
            .unwrap_or_default(),
        serde_json::Value::Bool(true)
    );

    let _close: serde_json::Value = call_tool(
        &service,
        "workspace_close",
        json!({ "session_id": &session_id }),
    )
    .await;
    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_workspace_observability_metrics_tool_ready_and_not_ready() {
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

    let status: WorkspaceStatusResponse = call_tool(
        &service,
        "workspace_status",
        json!({ "session_id": &session_id }),
    )
    .await;
    if status.ready {
        let metrics: serde_json::Value = call_tool(
            &service,
            "workspace_get_observability_metrics",
            json!({ "session_id": &session_id }),
        )
        .await;
        assert!(metrics.get("metrics").is_some());
    } else {
        let attempt = service
            .call_tool(CallToolRequestParam {
                name: "workspace_get_observability_metrics".into(),
                arguments: Some(json_object(json!({ "session_id": &session_id }))),
            })
            .await;

        match attempt {
            Ok(result) => {
                let value = extract_json_text(result);
                assert!(value.get("metrics").is_some());
            }
            Err(ServiceError::McpError(err))
                if err.code.0 == rmcp::model::ErrorCode::INVALID_PARAMS.0 =>
            {
                assert!(
                    err.message.contains("workspace not ready"),
                    "message={:?} does not contain workspace-not-ready hint",
                    err.message
                );
                let _ = wait_workspace_ready(&service, &open).await;
                let metrics: serde_json::Value = call_tool(
                    &service,
                    "workspace_get_observability_metrics",
                    json!({ "session_id": &session_id }),
                )
                .await;
                assert!(metrics.get("metrics").is_some());
            }
            Err(err) => panic!("unexpected error: {err:?}"),
        }
    }

    let _close: serde_json::Value = call_tool(
        &service,
        "workspace_close",
        json!({ "session_id": &session_id }),
    )
    .await;
    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_type_tools_reject_invalid_params() {
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
    let _status = wait_workspace_ready(&service, &open).await;

    call_tool_expect_invalid_params(
        &service,
        "bsl_types_list_start",
        json!({ "session_id": &session_id, "page": 1, "limit": 0 }),
        "limit must be in 1..=1000",
    )
    .await;

    call_tool_expect_invalid_params(
        &service,
        "bsl_types_search_start",
        json!({ "session_id": &session_id, "query": "Документ", "limit": 0 }),
        "limit must be in 1..=1000",
    )
    .await;

    call_tool_expect_invalid_params(
        &service,
        "bsl_types_search_start",
        json!({ "session_id": &session_id, "query": "   " }),
        "query must be non-empty",
    )
    .await;

    call_tool_expect_invalid_params(
        &service,
        "bsl_type_get_start",
        json!({ "session_id": &session_id, "type_name": "   " }),
        "type_name must be non-empty",
    )
    .await;

    let _close: serde_json::Value = call_tool(
        &service,
        "workspace_close",
        json!({ "session_id": &session_id }),
    )
    .await;
    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_type_tools_reject_invalid_view() {
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
    let _status = wait_workspace_ready(&service, &open).await;

    call_tool_expect_invalid_params(
        &service,
        "bsl_types_list_start",
        json!({ "session_id": &session_id, "page": 1, "limit": 50, "view": "nope" }),
        "view must be one of: names_only, summary, full",
    )
    .await;

    call_tool_expect_invalid_params(
        &service,
        "bsl_types_search_start",
        json!({ "session_id": &session_id, "query": "Документ", "limit": 200, "view": "nope" }),
        "view must be one of: names_only, summary, full",
    )
    .await;

    let _close: serde_json::Value = call_tool(
        &service,
        "workspace_close",
        json!({ "session_id": &session_id }),
    )
    .await;
    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_type_tools_require_ready_session() {
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

    // Startup for an empty workspace can complete very fast on some machines.
    // Accept both outcomes and verify they are consistent with ready state.
    let attempt = service
        .call_tool(CallToolRequestParam {
            name: "bsl_types_list_start".into(),
            arguments: Some(json_object(
                json!({ "session_id": &session_id, "page": 1, "limit": 50 }),
            )),
        })
        .await;

    match attempt {
        Ok(result) => {
            let value = extract_json_text(result);
            let start: JobStartResponse = serde_json::from_value(value).expect("decode job start");
            assert!(!start.job_id.is_empty());

            let status: WorkspaceStatusResponse = call_tool(
                &service,
                "workspace_status",
                json!({ "session_id": &session_id }),
            )
            .await;
            assert!(
                status.ready,
                "type tool accepted request while workspace is still not ready"
            );
        }
        Err(err) => {
            let ServiceError::McpError(err) = err else {
                panic!("unexpected error: {err:?}");
            };
            assert_eq!(err.code.0, rmcp::model::ErrorCode::INVALID_PARAMS.0);
            assert!(
                err.message.contains("workspace not ready"),
                "message={:?} does not contain {:?}",
                err.message,
                "workspace not ready"
            );
        }
    }

    let _close: serde_json::Value = call_tool(
        &service,
        "workspace_close",
        json!({ "session_id": &session_id }),
    )
    .await;
    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_types_list_is_deterministic_on_basic_workspace() {
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
    let _status = wait_workspace_ready(&service, &open).await;

    let start: JobStartResponse = call_tool(
        &service,
        "bsl_types_list_start",
        json!({ "session_id": &session_id, "page": 1, "limit": 50, "view": "names_only" }),
    )
    .await;
    wait_job_succeeded(&service, &start.job_id).await;
    let a: Vec<String> =
        call_tool(&service, "job_result", json!({ "job_id": &start.job_id })).await;

    let start: JobStartResponse = call_tool(
        &service,
        "bsl_types_list_start",
        json!({ "session_id": &session_id, "page": 1, "limit": 50, "view": "names_only" }),
    )
    .await;
    wait_job_succeeded(&service, &start.job_id).await;
    let b: Vec<String> =
        call_tool(&service, "job_result", json!({ "job_id": &start.job_id })).await;

    assert_eq!(a, b, "expected deterministic ordering and paging");

    let mut sorted = a.clone();
    sorted.sort_by(|x, y| {
        x.to_lowercase()
            .cmp(&y.to_lowercase())
            .then_with(|| x.cmp(y))
    });
    assert_eq!(a, sorted, "expected names_only list to be sorted");

    let _close: serde_json::Value = call_tool(
        &service,
        "workspace_close",
        json!({ "session_id": &session_id }),
    )
    .await;
    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_type_get_returns_properties_and_tabular_sections_for_conf_type() {
    let service = spawn_agent(&[]).await;

    let repo = repo_root();
    let config_root = repo.join("examples/conf/conf_test");

    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({
            "roots": [config_root.to_string_lossy()],
            "configuration_path": config_root.to_string_lossy(),
        }),
    )
    .await;
    let session_id = open.session_id.clone();
    let _status = wait_workspace_ready(&service, &open).await;

    let start: JobStartResponse = call_tool(
        &service,
        "bsl_types_search_start",
        json!({
            "session_id": &session_id,
            "query": "ЗаказНаряды",
            "limit": 1000,
            "source": "configuration",
            "view": "names_only"
        }),
    )
    .await;
    wait_job_succeeded(&service, &start.job_id).await;
    let names: Vec<String> =
        call_tool(&service, "job_result", json!({ "job_id": &start.job_id })).await;
    assert!(
        !names.is_empty(),
        "expected at least one configuration type for query 'ЗаказНаряды'"
    );
    let type_name = names
        .iter()
        .find(|name| name.contains("DocumentObject") && name.contains("ЗаказНаряды"))
        .or_else(|| {
            names
                .iter()
                .find(|name| name.contains("ДокументОбъект") && name.contains("ЗаказНаряды"))
        })
        .or_else(|| names.iter().find(|name| name.contains("ЗаказНаряды")))
        .expect("type name from search");

    let start: JobStartResponse = call_tool(
        &service,
        "bsl_type_get_start",
        json!({
            "session_id": &session_id,
            "type_name": type_name,
            "source": "configuration",
            "include_methods": false
        }),
    )
    .await;
    wait_job_succeeded(&service, &start.job_id).await;
    let dto: TypeDto = call_tool(&service, "job_result", json!({ "job_id": &start.job_id })).await;

    assert!(
        !dto.properties.is_empty(),
        "expected properties to be present for DocumentObject.ЗаказНаряды"
    );
    assert!(
        !dto.tabular_sections.is_empty(),
        "expected tabularSections to be present for DocumentObject.ЗаказНаряды"
    );
    assert!(
        dto.methods.is_empty(),
        "expected methods to be omitted when include_methods=false"
    );
    assert!(
        dto.methods_count.is_some(),
        "expected methodsCount to be present when include_methods=false"
    );

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
async fn stdio_bsl_diagnostics_tagged_file_scope_and_string_file_hint() {
    let service = spawn_agent(&[]).await;

    let temp_root = tempfile::TempDir::new().expect("tempdir");
    let file_path = temp_root.path().join("src/CommonModules/Foo/Module.bsl");
    std::fs::create_dir_all(file_path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file_path, "Procedure P()\nEndProcedure\n").expect("write Module.bsl");
    let file_abs = file_path.to_string_lossy().to_string();

    let open: WorkspaceOpenResponse = call_tool(
        &service,
        "workspace_open",
        json!({ "roots": [temp_root.path().to_string_lossy()] }),
    )
    .await;
    let session_id = open.session_id.clone();
    let _status = wait_workspace_ready(&service, &open).await;

    let start: JobStartResponse = call_tool(
        &service,
        "bsl_diagnostics_start",
        json!({
            "session_id": &session_id,
            "scope": { "kind": "file", "document": { "path": &file_abs } },
            "limit": 50
        }),
    )
    .await;
    wait_job_succeeded(&service, &start.job_id).await;
    let result: BslDiagnosticsResponse =
        call_tool(&service, "job_result", json!({ "job_id": &start.job_id })).await;
    assert_eq!(result.analysis_revision, 0);

    call_tool_expect_invalid_params(
        &service,
        "bsl_diagnostics_start",
        json!({ "session_id": &session_id, "scope": "file" }),
        "use tagged file scope",
    )
    .await;

    let _close: serde_json::Value = call_tool(
        &service,
        "workspace_close",
        json!({ "session_id": &session_id }),
    )
    .await;
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
async fn stdio_creates_persistent_log_file_in_process_cwd() {
    let cwd = tempfile::TempDir::new().expect("tempdir");
    let log_path = cwd.path().join(".bsl-agent").join("mcp.log");

    let service = spawn_agent_in_dir(cwd.path(), &[("RUST_LOG", "bsl_agent=info")]).await;

    for _ in 0..20 {
        if log_path.is_file() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    assert!(
        log_path.is_file(),
        "missing log file: {}",
        log_path.display()
    );

    let log = std::fs::read_to_string(&log_path).expect("read mcp.log");
    assert!(
        log.contains("bsl-agent starting"),
        "log does not contain startup record: {log}"
    );
    assert!(
        log.contains(&cwd.path().display().to_string()),
        "log does not contain cwd: {log}"
    );

    let _ = service.cancel().await;

    assert!(log_path.is_file(), "log file disappeared after cancel");
}

#[tokio::test]
async fn stdio_logging_keeps_stdout_clean() {
    let cwd = tempfile::TempDir::new().expect("tempdir");
    let log_path = cwd.path().join(".bsl-agent").join("mcp.log");

    let output = run_agent_output_in_dir(cwd.path(), &[("RUST_LOG", "bsl_agent=info")]).await;

    assert!(
        !output.status.success(),
        "expected startup to stop on closed stdin, stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "stdout must stay reserved for MCP transport, got: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        log_path.is_file(),
        "missing log file: {}",
        log_path.display()
    );

    let log = std::fs::read_to_string(&log_path).expect("read mcp.log");
    assert!(
        log.contains("failed to start stdio MCP service"),
        "log does not contain early startup failure record: {log}"
    );
}

#[tokio::test]
async fn stdio_prefers_explicit_log_file_override() {
    let cwd = tempfile::TempDir::new().expect("tempdir");
    let overridden_dir = tempfile::TempDir::new().expect("tempdir");
    let explicit_log_path = overridden_dir.path().join("custom-mcp.log");
    let directory_override_path = overridden_dir.path().join("directory-override");
    let default_log_path = cwd.path().join(".bsl-agent").join("mcp.log");

    let service = spawn_agent_in_dir(
        cwd.path(),
        &[
            ("RUST_LOG", "bsl_agent=info"),
            (
                "BSL_AGENT_LOG_FILE",
                explicit_log_path.to_string_lossy().as_ref(),
            ),
            (
                "BSL_AGENT_LOG_DIR",
                directory_override_path.to_string_lossy().as_ref(),
            ),
        ],
    )
    .await;

    for _ in 0..20 {
        if explicit_log_path.is_file() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    assert!(
        explicit_log_path.is_file(),
        "missing overridden log file: {}",
        explicit_log_path.display()
    );
    assert!(
        !default_log_path.exists(),
        "default log path should stay unused: {}",
        default_log_path.display()
    );
    assert!(
        !directory_override_path.join("mcp.log").exists(),
        "directory override should stay unused when BSL_AGENT_LOG_FILE is set"
    );

    let log = std::fs::read_to_string(&explicit_log_path).expect("read custom log");
    assert!(
        log.contains(&explicit_log_path.display().to_string()),
        "log does not contain effective overridden path: {log}"
    );

    let _ = service.cancel().await;
}

#[tokio::test]
async fn stdio_fails_fast_when_log_path_cannot_be_initialized() {
    let cwd = tempfile::TempDir::new().expect("tempdir");
    let blocking_path = cwd.path().join("not-a-dir");
    std::fs::write(&blocking_path, "blocker").expect("write blocker file");

    let output = run_agent_output_in_dir(
        cwd.path(),
        &[(
            "BSL_AGENT_LOG_DIR",
            blocking_path.to_string_lossy().as_ref(),
        )],
    )
    .await;

    assert!(
        !output.status.success(),
        "expected startup failure, stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("file logging bootstrap failed"),
        "stderr={stderr:?}"
    );
    assert!(
        stderr.contains(&blocking_path.display().to_string()),
        "stderr={stderr:?}"
    );
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
