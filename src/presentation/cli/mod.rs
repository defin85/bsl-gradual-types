//! CLI presentation layer
//! 
//! Содержит все компоненты для командной строки:
//! - Type checking CLI
//! - Report formatting  
//! - Output rendering

use clap::Parser;
use colored::Colorize;

/// CLI аргументы для type checker
#[derive(Parser, Debug)]
#[command(name = "type-check")]
#[command(about = "BSL Type Checker with SystemCoordinator")]
pub struct TypeCheckArgs {
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

/// Форматтер результатов для CLI
pub struct CliFormatter;

impl CliFormatter {
    pub fn format_type_info(type_name: &str, certainty: u8, category: &str) -> String {
        let certainty_color = match certainty {
            90..=100 => certainty.to_string().green(),
            50..=89 => certainty.to_string().yellow(), 
            _ => certainty.to_string().red(),
        };
        
        format!(
            "{} {} [{}%]", 
            type_name.bold(),
            category.dimmed(),
            certainty_color
        )
    }
    
    pub fn format_summary(total: usize, known: usize, inferred: usize) -> String {
        format!(
            "📊 Summary: {} total, {} known, {} inferred",
            total.to_string().bold(),
            known.to_string().green(),
            inferred.to_string().yellow()
        )
    }
}
