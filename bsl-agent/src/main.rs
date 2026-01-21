use bsl_agent::http_ui::start_http_ui;
use bsl_agent::jobs::JobManager;
use bsl_agent::server::BslAgentHandler;
use bsl_agent::session::SessionManager;
use rmcp::{transport::stdio, ServiceExt};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

fn parse_http_addr(raw: &str) -> anyhow::Result<SocketAddr> {
    let raw = raw.trim();
    if raw.is_empty() {
        anyhow::bail!("empty address");
    }

    let normalized = if let Some(port) = raw.strip_prefix(':') {
        format!("127.0.0.1:{port}")
    } else if let Some(port) = raw.strip_prefix("localhost:") {
        format!("127.0.0.1:{port}")
    } else {
        raw.to_string()
    };

    Ok(normalized.parse()?)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bsl_agent=debug,info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .init();

    tracing::info!("bsl-agent starting...");

    let session_manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new());
    let handler =
        BslAgentHandler::with_state(Arc::clone(&session_manager), Arc::clone(&job_manager));

    if let Ok(addr_raw) = std::env::var("BSL_AGENT_HTTP_ADDR") {
        match parse_http_addr(&addr_raw) {
            Ok(addr) => {
                let static_dir = std::env::var("BSL_AGENT_HTTP_STATIC_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("target/site"));
                let instance_id = Uuid::new_v4().to_string();
                let cache_dir = std::env::var("BSL_CACHE_DIR").ok();

                match start_http_ui(
                    addr,
                    static_dir,
                    instance_id,
                    cache_dir,
                    handler.session_manager(),
                    handler.job_manager(),
                )
                .await
                {
                    Ok(handle) => {
                        tracing::info!("http ui listening on {}", handle.ui_url);
                    }
                    Err(err) => {
                        tracing::warn!("http ui disabled: {err}");
                    }
                }
            }
            Err(err) => {
                tracing::warn!("http ui disabled: invalid BSL_AGENT_HTTP_ADDR={addr_raw:?}: {err}");
            }
        }
    }

    let service = handler
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("server error: {:?}", e))?;
    service.waiting().await?;

    Ok(())
}
