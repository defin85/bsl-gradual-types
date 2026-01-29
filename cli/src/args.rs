//! Command-line argument definitions for BSL Type System CLI
//!
//! Соответствует архитектурной диаграмме: CLI Tool -> AnalysisHostV2/AnalysisV2 (v2 pipeline)

use clap::{Parser, Subcommand};

/// BSL Gradual Type System CLI - согласно архитектурной диаграмме
#[derive(Parser, Debug)]
#[command(name = "bsl-cli")]
#[command(about = "BSL Gradual Type System CLI Tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct CliArgs {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output format (table, json, plain)
    #[arg(short, long, default_value = "table", global = true)]
    pub format: CliOutputFormat,

    /// Subcommands
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Analyze BSL files for type information
    Analyze {
        /// File or directory to analyze
        path: String,

        /// Show only errors
        #[arg(long)]
        errors_only: bool,

        /// Include type inference details
        #[arg(long)]
        show_inference: bool,
    },

    /// Check types in BSL files
    Check {
        /// File or directory to check
        path: String,

        /// Fail on warnings
        #[arg(long)]
        strict: bool,
    },

    /// Get type completions for expression
    Complete {
        /// Expression to complete
        expression: String,

        /// Maximum number of completions
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show type information for expression
    Info {
        /// Expression to get info for
        expression: String,
    },

    /// Milestone 2.8: Analyze file using IR-based flow (Parser → IR → Type Analysis)
    AnalyzeIr {
        /// File to analyze
        path: String,

        /// Show IR structure
        #[arg(long)]
        show_ir: bool,

        /// Show symbol table
        #[arg(long)]
        show_symbols: bool,
    },

    /// Cache management (per-project/per-config)
    Cache {
        /// Path to configuration root or Configuration.xml
        #[arg(long)]
        config_path: String,

        /// Cache action
        #[command(subcommand)]
        action: CacheCommand,
    },
}

/// Cache subcommands
#[derive(Subcommand, Debug)]
pub enum CacheCommand {
    /// Show cache stats (JSON)
    Stats,
    /// Clear cache for project/config (JSON)
    Clear,
}

/// Output format options
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CliOutputFormat {
    Table,
    Json,
    Plain,
}
