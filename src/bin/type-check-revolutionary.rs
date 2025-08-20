//! Революционный type-checker на CentralTypeSystem

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use std::path::PathBuf;

use bsl_gradual_types::ideal::system::{CentralTypeSystem, CentralSystemConfig};
use bsl_gradual_types::ideal::presentation::{CliAnalysisRequest, CliOutputFormat};

#[derive(Parser)]
#[command(name = "type-check-revolutionary")]
#[command(about = "Революционный анализатор типов BSL")]
struct Cli {
    /// Путь к BSL файлу или проекту для анализа
    #[arg(short, long)]
    input: PathBuf,
    
    /// Формат вывода
    #[arg(long, default_value = "text")]
    output_format: String,
    
    /// Включить анализ покрытия типизации
    #[arg(long)]
    coverage: bool,
    
    /// Показать все ошибки типов
    #[arg(long)]
    show_errors: bool,
    
    /// Детальный вывод
    #[arg(long)]
    verbose: bool,
    
    /// Путь к HTML справке
    #[arg(long, default_value = "examples/syntax_helper/rebuilt.shcntx_ru")]
    html_path: String,
    
    /// Путь к XML конфигурации (опционально)
    #[arg(long)]
    config_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    println!("Revolutionary BSL Type Checker");
    println!("Analyzing: {}", cli.input.display());
    
    // === ЕДИНАЯ ИНИЦИАЛИЗАЦИЯ ===
    let central_config = CentralSystemConfig {
        html_path: cli.html_path.clone(),
        configuration_path: cli.config_path.map(|p| p.to_string_lossy().to_string()),
        verbose_logging: cli.verbose,
        ..Default::default()
    };
    
    let central_system = Arc::new(CentralTypeSystem::new(central_config));
    
    if cli.verbose {
        println!("Initializing CentralTypeSystem...");
    }
    
    central_system.initialize().await?;
    
    if cli.verbose {
        let metrics = central_system.get_system_metrics().await;
        println!("System ready with {} types", metrics.total_types);
    }
    
    // === АНАЛИЗ ЧЕРЕЗ CLI ИНТЕРФЕЙС ===
    let output_format = match cli.output_format.as_str() {
        "json" => CliOutputFormat::Json,
        "csv" => CliOutputFormat::Csv,
        "html" => CliOutputFormat::Html,
        _ => CliOutputFormat::Text,
    };
    
    let analysis_request = CliAnalysisRequest {
        project_path: cli.input.clone(),
        output_format,
        include_coverage: cli.coverage,
        include_errors: cli.show_errors,
        verbose: cli.verbose,
    };
    
    println!("Analyzing project...");
    
    match central_system.cli_interface().handle_analysis_request(analysis_request).await {
        Ok(response) => {
            println!("{}", response.formatted_output);
            
            if cli.verbose {
                println!("\nAnalysis summary:");
                println!("  - Files: {}/{}", response.summary.analyzed_files, response.summary.total_files);
                println!("  - Functions: {}", response.summary.total_functions);
                println!("  - Variables: {}", response.summary.total_variables);
                println!("  - Errors: {}", response.summary.error_count);
                println!("  - Time: {:.2}s", response.summary.analysis_time_seconds);
            }
        }
        Err(e) => {
            println!("Analysis failed: {}", e);
            std::process::exit(1);
        }
    }
    
    println!("Revolutionary type checking completed!");
    Ok(())
}