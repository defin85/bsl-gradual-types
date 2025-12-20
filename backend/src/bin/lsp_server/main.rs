//! BSL LSP Server - Language Server Protocol implementation
//!
//! This is the entry point for the BSL Language Server.
//! Uses Clean Architecture with modular components.

#![allow(clippy::needless_borrow)]
#![allow(clippy::only_used_in_recursion)]

mod commands;
mod config;
mod converters;
mod handlers;
mod progress;
mod progress_bridge;
mod server;
mod types;

use anyhow::{Context, Result};
use clap::Parser;
use std::io::Write;
use std::sync::Arc;
use tower_lsp::{LspService, Server};
use tracing::info;

use bsl_backend::system::SystemCoordinator;
use server::BslLanguageServer;

#[derive(Parser, Debug)]
#[command(name = "lsp-server")]
#[command(about = "BSL Language Server (Clean Architecture)", long_about = None)]
#[allow(dead_code)]
struct Args {}

#[tokio::main]
async fn main() -> Result<()> {
    // Create log file (overwrites on each start)
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("rust_lsp_server.log")
        .context("Failed to create log file")?;

    // Configure logging to FILE instead of stderr
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("bsl_gradual_types=debug".parse()?)
                .add_directive("tower_lsp=info".parse()?)
                .add_directive("html5ever=warn".parse()?)
                .add_directive("selectors=warn".parse()?)
                .add_directive("scraper=info".parse()?),
        )
        .with_writer(std::sync::Mutex::new(log_file))
        .init();

    // Version and build info
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");
    info!("====================================================================");
    info!(" BSL Language Server - Clean Architecture");
    info!(" Version: {}", VERSION);
    info!(" Build: {}", BUILD_TIMESTAMP);
    info!("====================================================================");

    // Clear progress_debug.log on each start
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("progress_debug.log")
    {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(
            file,
            "[{}] === BSL Progress Debug Log - LSP Server Started ===",
            timestamp
        );
    }

    // Parse command line arguments
    let _args = Args::parse();

    // Create SystemCoordinator as IoC Container
    let coordinator = Arc::new(SystemCoordinator::new());

    // Initialize coordinator with fallback types
    // Real types will be loaded in initialized() via start_with_paths()
    info!(
        "Initializing coordinator with fallback types (real types will be loaded in initialized())"
    );
    coordinator
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start coordinator: {}", e))?;

    // Setup stdin/stdout for client communication
    info!("Setting up STDIO communication channels...");
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    info!("STDIO channels created");

    // Create LSP service
    info!("Creating LSP service...");
    let coordinator_clone = coordinator.clone();
    let (service, socket) = LspService::build(move |client| {
        info!("Initializing BSL Language Server");
        BslLanguageServer::new(client, coordinator_clone.clone())
    })
    // Legacy custom requests used by VSCode extension
    .custom_method("bsl/buildIndex", BslLanguageServer::handle_build_index)
    .custom_method(
        "bsl/incrementalUpdate",
        BslLanguageServer::handle_incremental_update,
    )
    .finish();
    info!("LSP service created");

    // Start server
    info!("Starting LSP server loop (listening on STDIO)...");
    Server::new(stdin, stdout, socket).serve(service).await;
    info!("LSP server shut down");

    Ok(())
}
