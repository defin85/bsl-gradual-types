use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use bsl_shared::api::dtos::{
    McpBackendModeDto, McpJobDto, McpJobsResponseDto, McpSessionsResponseDto, McpStatusDto,
};
use serde::Deserialize;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::jobs::JobManager;
use crate::session::SessionManager;

use include_dir::{include_dir, Dir};

static EMBEDDED_SITE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../target/site");

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

#[derive(Debug, Clone)]
pub enum HttpUiStaticSource {
    Embedded,
    Disk(PathBuf),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepsMetaQuery {
    #[serde(default)]
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TypesQuery {
    page: Option<usize>,
    limit: Option<usize>,
    #[serde(default)]
    category: Vec<String>,
    #[serde(default)]
    certainty_level: Vec<String>,
    flow_sensitive_only: Option<bool>,
    #[serde(default)]
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default)]
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MetricsQuery {
    #[serde(default)]
    #[serde(rename = "sessionId")]
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

async fn get_mcp_types(
    Query(query): Query<TypesQuery>,
    State(state): State<HttpUiState>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(50).clamp(1, 1000);
    let offset = (page - 1) * limit;

    match state
        .session_manager
        .http_parity_types(
            query.session_id.as_deref(),
            limit,
            offset,
            query.category,
            query.certainty_level,
            query.flow_sensitive_only.unwrap_or(false),
        )
        .await
    {
        Ok(dto) => Json(dto).into_response(),
        Err(err) => {
            let (status, msg) = map_rmcp_error(err);
            json_error(status, msg).into_response()
        }
    }
}

async fn get_mcp_search(
    Query(query): Query<SearchQuery>,
    State(state): State<HttpUiState>,
) -> impl IntoResponse {
    match state
        .session_manager
        .http_parity_search(query.session_id.as_deref(), query.q.as_str())
        .await
    {
        Ok(dto) => Json(dto).into_response(),
        Err(err) => {
            let (status, msg) = map_rmcp_error(err);
            json_error(status, msg).into_response()
        }
    }
}

async fn get_mcp_metrics(
    Query(query): Query<MetricsQuery>,
    State(state): State<HttpUiState>,
) -> impl IntoResponse {
    match state
        .session_manager
        .http_parity_metrics(query.session_id.as_deref())
        .await
    {
        Ok(dto) => Json(dto).into_response(),
        Err(err) => {
            let (status, msg) = map_rmcp_error(err);
            json_error(status, msg).into_response()
        }
    }
}

fn serve_embedded_response(path: &str) -> Option<Response> {
    let file = EMBEDDED_SITE.get_file(path)?;
    let mut headers = HeaderMap::new();
    let content_type = if path.ends_with(".wasm") {
        "application/wasm".to_string()
    } else {
        mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string()
    };
    if let Ok(value) = HeaderValue::from_str(content_type.as_str()) {
        headers.insert(header::CONTENT_TYPE, value);
    }

    Some((headers, file.contents()).into_response())
}

fn normalize_embedded_path(uri: &Uri) -> String {
    let raw = uri.path().trim_start_matches('/');
    if raw.is_empty() {
        return "index.html".to_string();
    }
    if raw.ends_with('/') {
        return format!("{raw}index.html");
    }
    raw.to_string()
}

async fn serve_embedded_fallback(method: Method, uri: Uri) -> impl IntoResponse {
    if method != Method::GET && method != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }

    let mut path = normalize_embedded_path(&uri);
    if let Some(resp) = serve_embedded_response(path.as_str()) {
        return if method == Method::HEAD {
            let mut resp = resp;
            *resp.body_mut() = axum::body::Body::empty();
            resp
        } else {
            resp
        };
    }

    path = "index.html".to_string();
    let Some(resp) = serve_embedded_response(path.as_str()) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    if method == Method::HEAD {
        let mut resp = resp;
        *resp.body_mut() = axum::body::Body::empty();
        resp
    } else {
        resp
    }
}

fn router(state: HttpUiState, static_source: HttpUiStaticSource) -> Router {
    let cross_origin_resource_policy = HeaderName::from_static("cross-origin-resource-policy");
    let x_frame_options = HeaderName::from_static("x-frame-options");

    let app = Router::new()
        .route("/api/mcp/status", get(get_mcp_status))
        .route("/api/mcp/sessions", get(get_mcp_sessions))
        .route("/api/mcp/jobs", get(get_mcp_jobs))
        .route("/api/mcp/jobs/:job_id", get(get_mcp_job))
        .route("/api/mcp/deps/meta", get(get_mcp_deps_meta))
        .route("/api/mcp/types", get(get_mcp_types))
        .route("/api/mcp/search", get(get_mcp_search))
        .route("/api/mcp/metrics", get(get_mcp_metrics))
        .with_state(state)
        .layer(SetResponseHeaderLayer::overriding(
            x_frame_options,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            cross_origin_resource_policy,
            HeaderValue::from_static("same-origin"),
        ));

    match static_source {
        HttpUiStaticSource::Disk(static_dir) => {
            let index_path = static_dir.join("index.html");
            let static_dir = ServeDir::new(static_dir)
                .not_found_service(ServeFile::new(index_path))
                .append_index_html_on_directories(true);
            app.fallback_service(static_dir)
        }
        HttpUiStaticSource::Embedded => app.fallback(serve_embedded_fallback),
    }
}

pub async fn start_http_ui(
    addr: SocketAddr,
    static_dir_override: Option<PathBuf>,
    instance_id: String,
    cache_dir: Option<String>,
    session_manager: Arc<SessionManager>,
    job_manager: Arc<JobManager>,
) -> anyhow::Result<HttpUiHandle> {
    match addr.ip() {
        IpAddr::V4(ipv4) if ipv4 == Ipv4Addr::LOCALHOST => {}
        _ => anyhow::bail!("HTTP UI must bind to 127.0.0.1 (localhost-only)"),
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
    let static_source = static_dir_override
        .and_then(|dir| {
            if dir.is_dir() {
                Some(HttpUiStaticSource::Disk(dir))
            } else {
                tracing::warn!(
                    "http ui static dir override {} is not a directory, falling back to embedded",
                    dir.display()
                );
                None
            }
        })
        .unwrap_or(HttpUiStaticSource::Embedded);
    let app = router(state, static_source);

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
