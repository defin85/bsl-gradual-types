use bsl_agent::types::WorkspaceOpenResponse;
use bsl_agent::ui_discovery::HttpUiDiscoveryRecord;
use bsl_shared::api::dtos::McpStatusDto;
use rmcp::model::CallToolRequestParam;
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::path::Path;
use std::path::PathBuf;
use tokio::process::Command;

fn skip_ui_discovery_test_when_loopback_tcp_is_unavailable() -> bool {
    match std::net::TcpListener::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0))) {
        Ok(listener) => {
            drop(listener);
            false
        }
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("skipping ui discovery integration test: loopback TCP unavailable: {err}");
            true
        }
        Err(err) => panic!("loopback TCP must be available for ui discovery test: {err}"),
    }
}

fn bsl_agent_bin() -> &'static str {
    env!("CARGO_BIN_EXE_bsl-agent")
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

fn registry_dir(cache_dir: &Path) -> PathBuf {
    cache_dir
        .join("bsl-agent-state")
        .join("v1")
        .join("runtime")
        .join("http-ui")
}

async fn wait_for_single_registry_record(cache_dir: &Path) -> HttpUiDiscoveryRecord {
    let dir = registry_dir(cache_dir);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let records = std::fs::read_dir(&dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                    return None;
                }
                let bytes = std::fs::read(path).ok()?;
                serde_json::from_slice::<HttpUiDiscoveryRecord>(&bytes).ok()
            })
            .collect::<Vec<_>>();

        if records.len() == 1 {
            return records.into_iter().next().expect("record");
        }

        if std::time::Instant::now() > deadline {
            panic!(
                "expected exactly 1 registry record in {}, found {}",
                dir.display(),
                records.len()
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

async fn wait_for_live_registry_records(
    cache_dir: &Path,
    expected: usize,
) -> Vec<HttpUiDiscoveryRecord> {
    let dir = registry_dir(cache_dir);
    let client = reqwest::Client::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let mut records = std::fs::read_dir(&dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                    return None;
                }
                let bytes = std::fs::read(path).ok()?;
                serde_json::from_slice::<HttpUiDiscoveryRecord>(&bytes).ok()
            })
            .collect::<Vec<_>>();

        records.sort_by(|a, b| a.started_at.cmp(&b.started_at).reverse());

        let mut live = Vec::new();
        for record in records {
            let Ok(resp) = client
                .get(format!("{}/api/mcp/status", record.ui_url))
                .send()
                .await
            else {
                continue;
            };
            if !resp.status().is_success() {
                continue;
            }
            let Ok(status) = resp.json::<McpStatusDto>().await else {
                continue;
            };
            if status.instance_id.as_deref() == Some(record.instance_id.as_str())
                && status.ui_url.as_deref() == Some(record.ui_url.as_str())
            {
                live.push(record);
            }
        }

        if live.len() == expected {
            return live;
        }

        if std::time::Instant::now() > deadline {
            panic!(
                "expected {} live registry records in {}, got {}",
                expected,
                dir.display(),
                live.len()
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

async fn run_ui_url(cache_dir: &Path) -> (std::process::ExitStatus, String, String) {
    let output = Command::new(bsl_agent_bin())
        .arg("ui")
        .arg("url")
        .env("RUST_LOG", "error")
        .env("BSL_CACHE_DIR", cache_dir.to_string_lossy().as_ref())
        .output()
        .await
        .expect("spawn ui url");

    (
        output.status,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

async fn run_ui_url_with_roots(
    cache_dir: &Path,
    root_path: &str,
) -> (std::process::ExitStatus, String, String) {
    let output = Command::new(bsl_agent_bin())
        .arg("ui")
        .arg("url")
        .arg("--roots")
        .arg(root_path)
        .env("RUST_LOG", "error")
        .env("BSL_CACHE_DIR", cache_dir.to_string_lossy().as_ref())
        .output()
        .await
        .expect("spawn ui url --roots");

    (
        output.status,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[tokio::test]
async fn ui_registry_record_written_and_points_to_live_ui() {
    if skip_ui_discovery_test_when_loopback_tcp_is_unavailable() {
        return;
    }

    let cache_dir = tempfile::tempdir().expect("tempdir");
    let cache_dir_path = cache_dir.path().to_path_buf();

    let static_dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        static_dir.path().join("index.html"),
        "<!doctype html><html><body>MCP UI discovery test</body></html>",
    )
    .expect("write index.html");

    let _service = spawn_agent(&[
        ("BSL_CACHE_DIR", cache_dir_path.to_string_lossy().as_ref()),
        ("BSL_AGENT_HTTP_ADDR", "127.0.0.1:0"),
        (
            "BSL_AGENT_HTTP_STATIC_DIR",
            static_dir.path().to_string_lossy().as_ref(),
        ),
    ])
    .await;

    let record = wait_for_single_registry_record(&cache_dir_path).await;

    assert!(
        record.ui_url.starts_with("http://localhost:"),
        "ui_url={:?}",
        record.ui_url
    );
    assert!(
        record.addr.starts_with("127.0.0.1:"),
        "addr={:?}",
        record.addr
    );

    let client = reqwest::Client::new();
    let status: McpStatusDto = client
        .get(format!("{}/api/mcp/status", record.ui_url))
        .send()
        .await
        .expect("GET /api/mcp/status")
        .json()
        .await
        .expect("parse json");

    assert_eq!(
        status.instance_id.as_deref(),
        Some(record.instance_id.as_str())
    );
    assert_eq!(status.ui_url.as_deref(), Some(record.ui_url.as_str()));
}

#[tokio::test]
async fn ui_url_single_instance_returns_url_and_multiple_is_ambiguous() {
    if skip_ui_discovery_test_when_loopback_tcp_is_unavailable() {
        return;
    }

    let cache_dir = tempfile::tempdir().expect("tempdir");
    let cache_dir_path = cache_dir.path().to_path_buf();

    let static_dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        static_dir.path().join("index.html"),
        "<!doctype html><html><body>MCP UI discovery test</body></html>",
    )
    .expect("write index.html");

    let _service_a = spawn_agent(&[
        ("BSL_CACHE_DIR", cache_dir_path.to_string_lossy().as_ref()),
        ("BSL_AGENT_HTTP_ADDR", "127.0.0.1:0"),
        (
            "BSL_AGENT_HTTP_STATIC_DIR",
            static_dir.path().to_string_lossy().as_ref(),
        ),
    ])
    .await;

    let live_a = wait_for_live_registry_records(&cache_dir_path, 1).await;
    let record_a = live_a.into_iter().next().expect("record a");

    let (status, stdout, stderr) = run_ui_url(&cache_dir_path).await;
    assert!(status.success(), "status={status} stderr={stderr:?}");
    assert_eq!(stdout.trim(), record_a.ui_url);

    let _service_b = spawn_agent(&[
        ("BSL_CACHE_DIR", cache_dir_path.to_string_lossy().as_ref()),
        ("BSL_AGENT_HTTP_ADDR", "127.0.0.1:0"),
        (
            "BSL_AGENT_HTTP_STATIC_DIR",
            static_dir.path().to_string_lossy().as_ref(),
        ),
    ])
    .await;

    let _live = wait_for_live_registry_records(&cache_dir_path, 2).await;

    let (status, _stdout, stderr) = run_ui_url(&cache_dir_path).await;
    assert_eq!(status.code(), Some(2), "stderr={stderr:?}");
    assert!(stderr.contains("Multiple live HTTP UI instances found"));
    assert!(stderr.contains("instance_id"));
    assert!(stderr.contains("ui_url"));
}

#[tokio::test]
async fn ui_url_roots_selector_picks_matching_instance() {
    if skip_ui_discovery_test_when_loopback_tcp_is_unavailable() {
        return;
    }

    let cache_dir = tempfile::tempdir().expect("tempdir");
    let cache_dir_path = cache_dir.path().to_path_buf();

    let static_dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        static_dir.path().join("index.html"),
        "<!doctype html><html><body>MCP UI discovery test</body></html>",
    )
    .expect("write index.html");

    let root_a = tempfile::tempdir().expect("tempdir");
    let root_b = tempfile::tempdir().expect("tempdir");
    let root_a_str = root_a.path().to_string_lossy().to_string();
    let root_b_str = root_b.path().to_string_lossy().to_string();

    let service_a = spawn_agent(&[
        ("BSL_CACHE_DIR", cache_dir_path.to_string_lossy().as_ref()),
        ("BSL_AGENT_HTTP_ADDR", "127.0.0.1:0"),
        (
            "BSL_AGENT_HTTP_STATIC_DIR",
            static_dir.path().to_string_lossy().as_ref(),
        ),
    ])
    .await;
    let service_b = spawn_agent(&[
        ("BSL_CACHE_DIR", cache_dir_path.to_string_lossy().as_ref()),
        ("BSL_AGENT_HTTP_ADDR", "127.0.0.1:0"),
        (
            "BSL_AGENT_HTTP_STATIC_DIR",
            static_dir.path().to_string_lossy().as_ref(),
        ),
    ])
    .await;

    let _live = wait_for_live_registry_records(&cache_dir_path, 2).await;

    let _open_a: WorkspaceOpenResponse = call_tool(
        &service_a,
        "workspace_open",
        json!({ "roots": [root_a_str.clone()] }),
    )
    .await;

    let _open_b: WorkspaceOpenResponse = call_tool(
        &service_b,
        "workspace_open",
        json!({ "roots": [root_b_str.clone()] }),
    )
    .await;

    let (status, stdout, stderr) =
        run_ui_url_with_roots(&cache_dir_path, root_a_str.as_str()).await;
    assert!(status.success(), "status={status} stderr={stderr:?}");
    let ui_url = stdout.trim().to_string();
    assert!(ui_url.starts_with("http://localhost:"), "ui_url={ui_url:?}");

    let client = reqwest::Client::new();
    let sessions: bsl_shared::api::dtos::McpSessionsResponseDto = client
        .get(format!("{}/api/mcp/sessions", ui_url))
        .send()
        .await
        .expect("GET /api/mcp/sessions")
        .json()
        .await
        .expect("parse json");

    assert!(
        sessions
            .sessions
            .iter()
            .flat_map(|s| s.roots.iter())
            .any(|root| root.path == root_a_str),
        "expected selected instance to contain root_a"
    );
    assert!(
        sessions
            .sessions
            .iter()
            .flat_map(|s| s.roots.iter())
            .all(|root| root.path != root_b_str),
        "expected selected instance not to contain root_b"
    );
}
