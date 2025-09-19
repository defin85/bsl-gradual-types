//! Command-line argument definitions and formatters for the CLI

use clap::Parser;

/// CLI arguments for the type checker
#[derive(Parser, Debug)]
#[command(name = "bsl-type-check")]
#[command(about = "BSL Gradual Type System CLI")]
pub struct CliArgs {
    /// File to analyze
    pub file: String,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Output format
    #[arg(short, long, default_value = "table")]
    pub format: String,

    /// Show only errors
    #[arg(long)]
    pub errors_only: bool,
}

