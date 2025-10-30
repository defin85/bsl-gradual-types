//! CSR server: serves API and static SPA (no SSR)

#[cfg(feature = "web-ui")]
use bsl_backend::{
    config::{load_config, CliConfig},
    presentation::web::{create_router, AppState},
    system::SystemCoordinator,
};
#[cfg(feature = "web-ui")]
use clap::Parser;
#[cfg(feature = "web-ui")]
use std::{path::PathBuf, sync::Arc};

#[cfg(feature = "web-ui")]
#[derive(Parser)]
#[command(name = "bsl-web-server")]
#[command(about = "BSL Type System Web Server")]
struct Args {
    /// Server host address
    #[arg(long, value_name = "HOST")]
    host: Option<String>,

    /// Server port
    #[arg(short, long, value_name = "PORT")]
    port: Option<u16>,

    /// Path to static files directory
    #[arg(long, value_name = "PATH")]
    static_files_path: Option<PathBuf>,

    /// Path to BSL project configuration
    #[arg(long, value_name = "PATH")]
    project_path: Option<PathBuf>,

    /// Path to 1C syntax helper directory
    #[arg(long, value_name = "PATH")]
    syntax_helper_path: Option<PathBuf>,

    /// Enable CORS for development
    #[arg(long)]
    enable_cors: Option<bool>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL")]
    log_level: Option<String>,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

#[cfg(feature = "web-ui")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load configuration
    let cli_config = CliConfig {
        host: args.host,
        port: args.port,
        static_files_path: args.static_files_path,
        project_path: args.project_path,
        syntax_helper_path: args.syntax_helper_path,
        enable_cors: args.enable_cors,
        log_level: args.log_level,
        config_file: args.config,
    };

    let config = load_config(cli_config)?;

    // Initialize logging with configured level
    let log_level = config.log_level.parse().unwrap_or(tracing::Level::INFO);
    tracing_subscriber::fmt().with_max_level(log_level).init();

    let system_coord = Arc::new(SystemCoordinator::new());

    // 🚀 КЛЮЧЕВОЕ ИЗМЕНЕНИЕ: запускаем полную инициализацию с парсингом синтаксис-помощника
    system_coord
        .start_with_paths(
            config.syntax_helper_path.as_deref(),
            config.project_path.as_deref(),
            None, // ✅ MILESTONE 2.20.2.3: progress_tx для web сервера не требуется
        )
        .await?;

    // Create TypeSystemService using SystemCoordinator's singleton method
    let type_service = system_coord
        .type_service()
        .expect("AnalysisEngine should be initialized after start_with_paths");

    let app_state = AppState {
        type_service: type_service.clone(),
        system_coordinator: system_coord.clone(), // ВРЕМЕННО для отладки
    };

    // Static SPA from configured path or default
    let static_path = config
        .static_files_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "target/site".to_string());

    let app = create_router(app_state, &static_path, config.enable_cors);

    let addr = config.address();
    println!(
        "\u{1F680} BSL Type System Web UI (CSR) listening on {}",
        config.server_url()
    );
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

#[cfg(not(feature = "web-ui"))]
fn main() {
    println!("BSL Type System - LSP only mode");
    println!("Web UI disabled. Use --features web-ui to enable.");
    println!("This would start the LSP server...");
}
