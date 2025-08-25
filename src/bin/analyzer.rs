//! BSL Type Analyzer CLI (target-only)

use anyhow::Result;
use bsl_gradual_types::system::{CentralSystemConfig, CentralTypeSystem};
use clap::Parser;
use tracing::info;

#[derive(Parser)]
#[command(name = "bsl-analyzer")]
#[command(about = "BSL Gradual Type System Analyzer (target)")]
struct Args {
    /// Path to BSL file to analyze
    #[arg(short, long)]
    file: String,

    /// Configuration path
    #[arg(short, long)]
    config: Option<String>,

    /// Enable verbose output
    #[arg(short = 'V', long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    if args.verbose {
        tracing_subscriber::fmt().with_env_filter("debug").init();
    } else {
        tracing_subscriber::fmt().with_env_filter("info").init();
    }
    info!("BSL Gradual Type Analyzer v{}", env!("CARGO_PKG_VERSION"));

    // Target-only: инициализируем центральную систему и выводим статус
    let mut cfg = CentralSystemConfig::default();
    if let Some(ref path) = args.config {
        cfg.configuration_path = Some(path.clone());
    }
    let central = CentralTypeSystem::new(cfg);
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        if let Err(e) = central.initialize().await {
            eprintln!("Initialization error: {}", e);
        }
        let health = central.health_check().await;
        println!(
            "Health: {} (score {:.2})",
            health.status, health.overall_score
        );
        let metrics = central.get_system_metrics().await;
        println!(
            "Types loaded: {} (platform: {}, config: {})",
            metrics.total_types, metrics.platform_types, metrics.configuration_types
        );
    });

    Ok(())
}
