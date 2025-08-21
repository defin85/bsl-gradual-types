//! Революционный LSP сервер на CentralTypeSystem

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;

use bsl_gradual_types::target::system::{CentralTypeSystem, CentralSystemConfig};
use bsl_gradual_types::target::presentation::{LspCompletionRequest, LspHoverRequest};

#[derive(Parser)]
#[command(name = "lsp-server-revolutionary")]
#[command(about = "Революционный LSP сервер BSL на идеальной архитектуре")]
struct Cli {
    /// Путь к HTML справке
    #[arg(long, default_value = "examples/syntax_helper/rebuilt.shcntx_ru")]
    html_path: String,
    
    /// Путь к XML конфигурации (опционально)
    #[arg(long)]
    config_path: Option<std::path::PathBuf>,
    
    /// Включить детальное логирование
    #[arg(long)]
    verbose: bool,
    
    /// Порт для LSP (опционально)
    #[arg(long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    println!("Starting revolutionary LSP server...");
    
    // === ЕДИНАЯ ИНИЦИАЛИЗАЦИЯ ===
    let central_config = CentralSystemConfig {
        html_path: cli.html_path.clone(),
        configuration_path: cli.config_path.map(|p| p.to_string_lossy().to_string()),
        verbose_logging: cli.verbose,
        ..Default::default()
    };
    
    let central_system = Arc::new(CentralTypeSystem::new(central_config));
    
    println!("Initializing CentralTypeSystem for LSP...");
    central_system.initialize().await?;
    
    // Показываем метрики LSP
    let metrics = central_system.get_system_metrics().await;
    println!("LSP System ready:");
    println!("  - Types: {}", metrics.total_types);
    println!("  - Memory: {:.2} MB", metrics.cache_memory_mb);
    
    // Тестируем LSP функциональность
    test_lsp_functionality(&central_system).await?;
    
    println!("Revolutionary LSP server ready to serve!");
    
    // В реальном LSP здесь был бы tower-lsp сервер
    println!("LSP would listen for requests here...");
    
    Ok(())
}

/// Тестирование LSP функциональности
async fn test_lsp_functionality(central_system: &CentralTypeSystem) -> Result<()> {
    println!("Testing LSP functionality...");
    
    let lsp_interface = central_system.lsp_interface();
    
    // Тест автодополнения
    let completion_request = LspCompletionRequest {
        file_path: "test.bsl".to_string(),
        line: 10,
        column: 5,
        prefix: "Массив".to_string(),
        trigger_character: None,
    };
    
    match lsp_interface.handle_completion_request(completion_request).await {
        Ok(response) => {
            println!("✅ Completion test: {} items", response.items.len());
        }
        Err(e) => {
            println!("⚠️ Completion test failed: {}", e);
        }
    }
    
    // Тест hover
    let hover_request = LspHoverRequest {
        file_path: "test.bsl".to_string(),
        line: 10,
        column: 5,
        expression: "ТаблицаЗначений".to_string(),
    };
    
    match lsp_interface.handle_hover_request(hover_request).await {
        Ok(Some(response)) => {
            println!("✅ Hover test: {} content items", response.contents.len());
        }
        Ok(None) => {
            println!("⚠️ Hover test: no content");
        }
        Err(e) => {
            println!("⚠️ Hover test failed: {}", e);
        }
    }
    
    // Тест метрик производительности
    match lsp_interface.get_performance_metrics().await {
        Ok(perf_metrics) => {
            println!("✅ Performance metrics:");
            println!("  - Total requests: {}", perf_metrics.total_requests);
            println!("  - Avg response: {:.2}ms", perf_metrics.average_response_time_ms);
        }
        Err(e) => {
            println!("⚠️ Performance metrics failed: {}", e);
        }
    }
    
    println!("LSP functionality test completed!");
    Ok(())
}
