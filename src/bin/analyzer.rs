//! BSL Type Analyzer CLI

use anyhow::Result;
use clap::Parser;
use bsl_gradual_types::core::{
    resolution::{BasicTypeResolver, TypeResolver},
    context::Context,
};
use tracing::info;

#[derive(Parser)]
#[command(name = "bsl-analyzer")]
#[command(about = "BSL Gradual Type System Analyzer")]
struct Args {
    /// Path to BSL file to analyze
    #[arg(short, long)]
    file: String,
    
    /// Configuration path
    #[arg(short, long)]
    config: Option<String>,
    
    /// Platform version
    #[arg(short = 'v', long, default_value = "8.3.25")]
    platform_version: String,
    
    /// Enable verbose output
    #[arg(short = 'V', long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize tracing
    if args.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("info")
            .init();
    }
    
    info!("BSL Gradual Type Analyzer v{}", env!("CARGO_PKG_VERSION"));
    info!("Analyzing file: {}", args.file);
    
    // Create type resolver
    let mut resolver = BasicTypeResolver::new();
    
    // Load platform types
    resolver.load_platform_types(&args.platform_version)?;
    
    // Load configuration if provided
    if let Some(config_path) = args.config {
        info!("Loading configuration from: {}", config_path);
        resolver.load_config_types(&config_path)?;
    }
    
    // TODO: Parse BSL file and analyze
    
    info!("Analysis complete");
    
    Ok(())
}