use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use bsl_shared::api::dtos::{
    McpBackendModeDto, McpJobDto, McpJobsResponseDto, McpSessionsResponseDto, McpStatusDto,
};
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tower_http::services::{ServeDir, ServeFile};

use crate::jobs::JobManager;
use crate::session::SessionManager;

#[derive(Clone)]
pub struct HttpUiState {
    pub instance_id: String,
    pub ui_url: String,
    pub cache_dir: Option<String>,
    pub session_manager: Arc<SessionManager>,
    pub job_manager: Arc<JobManager>,
}

pub struct HttpUiHandle {
    pub addr: SocketAddr,
    pub ui_url: String,
    pub task: JoinHandle<()>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepsMetaQuery {
    #[serde(default)]
    session_id: Option<String>,
}

fn json_error(
    status: StatusCode,
    message: impl Into<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": message.into() })))
}

fn map_rmcp_error(err: rmcp::ErrorData) -> (StatusCode, String) {
    let msg = err.message.to_string();
    if err.code == rmcp::model::ErrorCode::INVALID_PARAMS {
        return (StatusCode::BAD_REQUEST, msg);
    }
    if err.code == rmcp::model::ErrorCode::RESOURCE_NOT_FOUND {
        return (StatusCode::NOT_FOUND, msg);
    }
    (StatusCode::INTERNAL_SERVER_ERROR, msg)
}

async fn get_mcp_status(State(state): State<HttpUiState>) -> impl IntoResponse {
    Json(McpStatusDto {
        mode: McpBackendModeDto::McpAgent,
        supported: true,
        read_only: true,
        instance_id: Some(state.instance_id),
        ui_url: Some(state.ui_url),
        cache_dir: state.cache_dir,
    })
}

async fn get_mcp_sessions(State(state): State<HttpUiState>) -> impl IntoResponse {
    let sessions = state.session_manager.http_list_sessions().await;
    Json(McpSessionsResponseDto { sessions })
}

async fn get_mcp_jobs(State(state): State<HttpUiState>) -> impl IntoResponse {
    let jobs = state.job_manager.list_statuses().await;
    let jobs = jobs
        .into_iter()
        .map(|job| McpJobDto {
            job_id: job.job_id,
            state: job.state.as_str().to_string(),
            phase: job.phase,
            progress_percent: job.progress.percent,
            error: job.error,
        })
        .collect();
    Json(McpJobsResponseDto { jobs })
}

async fn get_mcp_job(
    Path(job_id): Path<String>,
    State(state): State<HttpUiState>,
) -> impl IntoResponse {
    match state.job_manager.status(&job_id).await {
        Ok(job) => Json(McpJobDto {
            job_id: job.job_id,
            state: job.state.as_str().to_string(),
            phase: job.phase,
            progress_percent: job.progress.percent,
            error: job.error,
        })
        .into_response(),
        Err(err) => {
            let (status, msg) = map_rmcp_error(err);
            json_error(status, msg).into_response()
        }
    }
}

async fn get_mcp_deps_meta(
    Query(query): Query<DepsMetaQuery>,
    State(state): State<HttpUiState>,
) -> impl IntoResponse {
    match state
        .session_manager
        .http_deps_meta(query.session_id.as_deref())
        .await
    {
        Ok(meta) => Json(meta).into_response(),
        Err(err) => {
            let (status, msg) = map_rmcp_error(err);
            json_error(status, msg).into_response()
        }
    }
}

fn router(state: HttpUiState, static_dir: PathBuf) -> Router {
    let index_path = static_dir.join("index.html");
    let static_dir = ServeDir::new(static_dir)
        .not_found_service(ServeFile::new(index_path))
        .append_index_html_on_directories(true);

    Router::new()
        .route("/api/mcp/status", get(get_mcp_status))
        .route("/api/mcp/sessions", get(get_mcp_sessions))
        .route("/api/mcp/jobs", get(get_mcp_jobs))
        .route("/api/mcp/jobs/:job_id", get(get_mcp_job))
        .route("/api/mcp/deps/meta", get(get_mcp_deps_meta))
        .fallback_service(static_dir)
        .with_state(state)
}

pub async fn start_http_ui(
    addr: SocketAddr,
    static_dir: PathBuf,
    instance_id: String,
    cache_dir: Option<String>,
    session_manager: Arc<SessionManager>,
    job_manager: Arc<JobManager>,
) -> anyhow::Result<HttpUiHandle> {
    if !addr.ip().is_loopback() {
        anyhow::bail!("HTTP UI must bind to loopback address (localhost-only)");
    }

    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    let ui_url = format!("http://localhost:{}", local_addr.port());

    let state = HttpUiState {
        instance_id,
        ui_url: ui_url.clone(),
        cache_dir,
        session_manager,
        job_manager,
    };
    let app = router(state, static_dir);

    let task = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, app.into_make_service()).await {
            tracing::error!("http ui server error: {err}");
        }
    });

    Ok(HttpUiHandle {
        addr: local_addr,
        ui_url,
        task,
    })
}
