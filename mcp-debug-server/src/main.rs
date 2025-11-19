use mcp_debug_server::server::DebugServerHandler;
use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Инициализация structured logging (вывод в stderr для MCP)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mcp_debug_server=debug,info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .init();

    tracing::info!("MCP Debug Server starting...");

    // Создать server handler
    let handler = DebugServerHandler::new();

    // Запустить MCP server через stdio
    let service = handler
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("Server error: {:?}", e))?;

    // Ждать завершения
    service.waiting().await?;

    Ok(())
}
