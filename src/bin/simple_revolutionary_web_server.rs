//! Простой революционный веб-сервер на CentralTypeSystem

use anyhow::Result;
use std::sync::Arc;

use bsl_gradual_types::target::system::{CentralTypeSystem, CentralSystemConfig};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting revolutionary web server...");
    
    // Создаём конфигурацию
    let config = CentralSystemConfig {
        html_path: "examples/syntax_helper/rebuilt.shcntx_ru".to_string(),
        configuration_path: None,
        verbose_logging: true,
        ..Default::default()
    };
    
    // Создаём центральную систему
    let central_system = Arc::new(CentralTypeSystem::new(config));
    
    // Инициализируем
    println!("Initializing CentralTypeSystem...");
    match central_system.initialize().await {
        Ok(_) => {
            println!("CentralTypeSystem initialized successfully!");
            
            // Показываем метрики
            let metrics = central_system.get_system_metrics().await;
            println!("Types loaded: {}", metrics.total_types);
            println!("Platform types: {}", metrics.platform_types);
            println!("Memory usage: {:.2} MB", metrics.cache_memory_mb);
            
            // Проверяем здоровье
            let health = central_system.health_check().await;
            println!("Health status: {}", health.status);
            
            println!("Revolutionary architecture is ready!");
        }
        Err(e) => {
            println!("Initialization failed: {}", e);
        }
    }
    
    Ok(())
}
