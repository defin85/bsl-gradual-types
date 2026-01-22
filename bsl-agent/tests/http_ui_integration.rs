use bsl_agent::http_ui::start_http_ui;
use bsl_agent::jobs::JobManager;
use bsl_agent::session::SessionManager;
use bsl_shared::api::{
    AnalysisResultDto, McpBackendModeDto, McpJobsResponseDto, McpSessionsResponseDto, McpStatusDto,
    MetricsDto,
};
use reqwest::Method;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::test]
async fn http_ui_serves_spa_and_readonly_api() {
    let static_dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        static_dir.path().join("index.html"),
        "<!doctype html><html><body>MCP UI test</body></html>",
    )
    .expect("write index.html");

    let session_manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new_in_memory());

    let handle = start_http_ui(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        Some(static_dir.path().to_path_buf()),
        "test-instance".to_string(),
        Some("/tmp/bsl-cache-test".to_string()),
        session_manager,
        job_manager,
    )
    .await
    .expect("start http ui");

    let client = reqwest::Client::new();

    let index_body = client
        .get(format!("{}/", handle.ui_url))
        .send()
        .await
        .expect("GET /")
        .text()
        .await
        .expect("read body");
    assert!(index_body.contains("MCP UI test"));

    let status: McpStatusDto = client
        .get(format!("{}/api/mcp/status", handle.ui_url))
        .send()
        .await
        .expect("GET /api/mcp/status")
        .json()
        .await
        .expect("parse json");
    assert_eq!(status.mode, McpBackendModeDto::McpAgent);
    assert!(status.supported);
    assert!(status.read_only);
    assert_eq!(status.instance_id.as_deref(), Some("test-instance"));
    assert_eq!(status.ui_url.as_deref(), Some(handle.ui_url.as_str()));
    assert_eq!(status.cache_dir.as_deref(), Some("/tmp/bsl-cache-test"));

    let sessions: McpSessionsResponseDto = client
        .get(format!("{}/api/mcp/sessions", handle.ui_url))
        .send()
        .await
        .expect("GET /api/mcp/sessions")
        .json()
        .await
        .expect("parse json");
    assert!(sessions.sessions.is_empty());

    let jobs: McpJobsResponseDto = client
        .get(format!("{}/api/mcp/jobs", handle.ui_url))
        .send()
        .await
        .expect("GET /api/mcp/jobs")
        .json()
        .await
        .expect("parse json");
    assert!(jobs.jobs.is_empty());

    let post_status = client
        .post(format!("{}/api/mcp/status", handle.ui_url))
        .send()
        .await
        .expect("POST /api/mcp/status");
    assert!(
        post_status.status().as_u16() == 405 || post_status.status().as_u16() == 404,
        "expected 405/404, got {}",
        post_status.status()
    );

    let endpoints = [
        "/api/mcp/status",
        "/api/mcp/sessions",
        "/api/mcp/jobs",
        "/api/mcp/jobs/00000000-0000-0000-0000-000000000000",
        "/api/mcp/deps/meta",
        "/api/mcp/deps/meta?sessionId=00000000-0000-0000-0000-000000000000",
        "/api/mcp/types",
        "/api/mcp/search?q=Test",
        "/api/mcp/metrics",
    ];
    let methods = [Method::POST, Method::PUT, Method::PATCH, Method::DELETE];

    for endpoint in endpoints {
        for method in methods.iter() {
            let resp = client
                .request(method.clone(), format!("{}{}", handle.ui_url, endpoint))
                .send()
                .await
                .unwrap_or_else(|_| panic!("{method} {endpoint}"));
            assert!(
                resp.status().as_u16() == 405 || resp.status().as_u16() == 404,
                "expected 405/404, got {} for {method} {endpoint}",
                resp.status()
            );
        }
    }

    handle.task.abort();
}

#[tokio::test]
async fn http_ui_serves_embedded_spa_by_default() {
    let session_manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new_in_memory());

    let handle = start_http_ui(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        None,
        "test-instance".to_string(),
        Some("/tmp/bsl-cache-test".to_string()),
        session_manager,
        job_manager,
    )
    .await
    .expect("start http ui");

    let client = reqwest::Client::new();

    let index_body = client
        .get(format!("{}/", handle.ui_url))
        .send()
        .await
        .expect("GET /")
        .text()
        .await
        .expect("read body");
    assert!(
        index_body.contains("bsl-frontend-"),
        "expected embedded index.html, got body length {}",
        index_body.len()
    );

    let status: McpStatusDto = client
        .get(format!("{}/api/mcp/status", handle.ui_url))
        .send()
        .await
        .expect("GET /api/mcp/status")
        .json()
        .await
        .expect("parse json");
    assert_eq!(status.instance_id.as_deref(), Some("test-instance"));
    assert_eq!(status.ui_url.as_deref(), Some(handle.ui_url.as_str()));

    handle.task.abort();
}

#[tokio::test]
async fn http_ui_parity_endpoints_require_ready_session() {
    let session_manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new_in_memory());

    let handle = start_http_ui(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        None,
        "test-instance".to_string(),
        Some("/tmp/bsl-cache-test".to_string()),
        session_manager,
        job_manager,
    )
    .await
    .expect("start http ui");

    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/mcp/types?page=1&limit=50", handle.ui_url))
        .send()
        .await
        .expect("GET /api/mcp/types");
    assert_eq!(resp.status().as_u16(), 400);

    let resp = client
        .get(format!(
            "{}/api/mcp/types?page=1&limit=50&sessionId=00000000-0000-0000-0000-000000000000",
            handle.ui_url
        ))
        .send()
        .await
        .expect("GET /api/mcp/types with sessionId");
    assert_eq!(resp.status().as_u16(), 400);

    let resp = client
        .get(format!("{}/api/mcp/search?q=Test", handle.ui_url))
        .send()
        .await
        .expect("GET /api/mcp/search");
    assert_eq!(resp.status().as_u16(), 400);

    let resp = client
        .get(format!("{}/api/mcp/metrics", handle.ui_url))
        .send()
        .await
        .expect("GET /api/mcp/metrics");
    assert_eq!(resp.status().as_u16(), 400);

    // Sanity check: responses are JSON (error object).
    let _error_json: serde_json::Value = resp.json().await.expect("parse error json");

    // Ensure types/search/metrics decode when success (compile-time check of DTOs).
    let _ = std::mem::size_of::<AnalysisResultDto>();
    let _ = std::mem::size_of::<MetricsDto>();

    handle.task.abort();
}
