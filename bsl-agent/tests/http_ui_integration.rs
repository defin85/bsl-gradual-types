use bsl_agent::http_ui::start_http_ui;
use bsl_agent::jobs::JobManager;
use bsl_agent::session::SessionManager;
use bsl_shared::api::{
    McpBackendModeDto, McpJobsResponseDto, McpSessionsResponseDto, McpStatusDto,
};
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
        static_dir.path().to_path_buf(),
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

    handle.task.abort();
}
