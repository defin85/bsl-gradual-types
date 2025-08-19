//! Демонстрация системы поиска DocumentationSearchEngine

use anyhow::Result;
use bsl_gradual_types::documentation::{
    PlatformDocumentationProvider, 
    DocumentationSearchEngine, AdvancedSearchQuery
};
use bsl_gradual_types::documentation::core::{DocumentationProvider, ProviderConfig};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Демонстрация системы поиска BSL документации");
    
    // Создаем поисковую систему
    let search_engine = DocumentationSearchEngine::new();
    println!("✅ DocumentationSearchEngine создан");
    
    // Создаем платформенный провайдер
    let mut platform_provider = PlatformDocumentationProvider::new();
    
    // Инициализируем провайдер (требуется для получения типов)
    let config = ProviderConfig::default();
    match platform_provider.initialize(&config).await {
        Ok(_) => println!("✅ PlatformDocumentationProvider инициализирован"),
        Err(e) => {
            println!("⚠️ Предупреждение при инициализации провайдера: {}", e);
            println!("   Это нормально, если нет файлов справки синтакс-помощника");
        }
    }
    
    // Получаем количество типов
    let types_count = platform_provider.get_loaded_types_count().await;
    println!("📊 Загружено {} типов в провайдер", types_count);
    
    if types_count > 0 {
        // Строим индексы
        println!("\n=== 🏗️ Построение индексов ===");
        let config_provider = bsl_gradual_types::documentation::ConfigurationDocumentationProvider::new();
        search_engine.build_indexes(&platform_provider, &config_provider).await?;
        
        // Тест 1: Простой поиск
        println!("\n=== 🔍 Тест 1: Простой поиск ===");
        let simple_query = AdvancedSearchQuery {
            query: "Таблица".to_string(),
            ..Default::default()
        };
        
        let results = search_engine.search(simple_query).await?;
        println!("Найдено {} результатов", results.total_count);
        
        for (i, item) in results.items.iter().take(3).enumerate() {
            println!("  {}. {} - {}", i + 1, item.display_name, 
                item.description.chars().take(50).collect::<String>());
        }
        
        // Тест 2: Автодополнение
        println!("\n=== 💡 Тест 2: Автодополнение ===");
        let suggestions = search_engine.get_suggestions("Табли").await?;
        println!("Предложения для 'Табли': {:?}", suggestions.iter().take(5).collect::<Vec<_>>());
        
        // Тест 3: Статистика
        println!("\n=== 📊 Тест 3: Статистика ===");
        let stats = search_engine.get_statistics().await?;
        println!("Всего запросов: {}", stats.total_queries);
        println!("Среднее время поиска: {:.2}ms", stats.average_search_time_ms);
        
    } else {
        println!("⚠️ Нет загруженных типов для демонстрации поиска");
        println!("   Убедитесь, что есть файлы справки синтакс-помощника в examples/syntax_helper/");
    }
    
    println!("\n🎉 Демонстрация завершена!");
    Ok(())
}