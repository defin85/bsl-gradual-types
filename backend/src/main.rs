//! CSR server: serves API and static SPA (no SSR)

#[cfg(feature = "web-ui")]
use bsl_backend::{
    config::{load_config, CliConfig},
    presentation::web::{create_router, AppState},
    system::{startup_v2, StartupInputs, SystemCoordinator},
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

    /// Path to semantic rules config (bsl-rules.toml)
    #[arg(long, value_name = "PATH")]
    rules_config_path: Option<PathBuf>,

    /// Path to 1C syntax helper directory
    #[arg(long, value_name = "PATH")]
    syntax_helper_path: Option<PathBuf>,

    /// 1C platform version (e.g., "8.3.25")
    #[arg(long, value_name = "VERSION")]
    platform_version: Option<String>,

    /// Enable CORS for development
    #[arg(long)]
    enable_cors: Option<bool>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL")]
    log_level: Option<String>,

    /// Enable/disable disk cache (overrides env)
    #[arg(long)]
    cache_enabled: Option<bool>,

    /// Use strict fingerprint mode for caches and deps ids
    #[arg(long)]
    strict_fingerprint: Option<bool>,

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
        rules_config_path: args.rules_config_path,
        syntax_helper_path: args.syntax_helper_path,
        platform_version: args.platform_version,
        enable_cors: args.enable_cors,
        log_level: args.log_level,
        config_file: args.config,
        cache_enabled: args.cache_enabled,
        strict_fingerprint: args.strict_fingerprint,
    };

    let config = load_config(cli_config)?;

    // Initialize logging with configured level
    let log_level = config.log_level.parse().unwrap_or(tracing::Level::INFO);
    tracing_subscriber::fmt().with_max_level(log_level).init();

    // 🏷️ Вывод версии и времени сборки для идентификации сборки
    let version = env!("CARGO_PKG_VERSION");
    let build_time = env!("BUILD_TIMESTAMP");
    let git_hash = env!("GIT_HASH");
    tracing::info!(
        "🚀 BSL Web Server v{} (build: {}, git: {})",
        version,
        build_time,
        git_hash
    );

    let system_coord = Arc::new(SystemCoordinator::new());

    let inputs = StartupInputs::from_web_flags(
        config.syntax_helper_path.clone(),
        config.project_path.clone(),
        config.platform_version.clone(),
        config.rules_config_path.clone(),
        config.cache_enabled,
        config.strict_fingerprint,
    );

    let startup = startup_v2(system_coord.clone(), inputs, None).await?;

    let app_state = AppState {
        deps_bundle_v2: Arc::new(tokio::sync::RwLock::new(Arc::new(startup.deps_bundle_v2))),
        system_coordinator: system_coord.clone(),
        syntax_helper_path: startup.inputs.syntax_helper_path.clone(),
        startup_inputs: Arc::new(tokio::sync::RwLock::new(startup.inputs)),
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
