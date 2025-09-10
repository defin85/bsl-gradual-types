//! CSR server: serves API and static SPA (no SSR)

#[cfg(feature = "web-ui")]
use bsl_backend::{
    application::TypeSystemService,
    config::{load_config, CliConfig},
    system::SystemCoordinator,
    presentation::web::{AppState, create_router},
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
        enable_cors: args.enable_cors,
        log_level: args.log_level,
        config_file: args.config,
    };
    
    let config = load_config(cli_config)?;
    
    // Initialize logging with configured level
    let log_level = config.log_level.parse().unwrap_or(tracing::Level::INFO);
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    let system_coord = Arc::new(SystemCoordinator::new());
    
    // Create TypeSystemService using components from SystemCoordinator
    let (type_resolver, cache, parser) = system_coord.get_components();
    let type_service = Arc::new(TypeSystemService::new(type_resolver, cache, parser));
    
    // Initialize the type service
    type_service.initialize()?;

    let app_state = AppState {
        type_service: type_service.clone(),
    };

    // Static SPA from configured path or default
    let static_path = config.static_files_path
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
    axum::serve(listener, app.into_make_service())
        .await?;
    Ok(())
}

#[cfg(not(feature = "web-ui"))]
fn main() {
    println!("BSL Type System - LSP only mode");
    println!("Web UI disabled. Use --features web-ui to enable.");
    println!("This would start the LSP server...");
}
