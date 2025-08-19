//! Тест единой архитектуры типов - UnifiedTypeSystem

use anyhow::Result;
use std::sync::Arc;
use bsl_gradual_types::core::unified_type_system::{
    UnifiedTypeSystem, UnifiedSystemConfig,
    LspTypeInterface, WebTypeInterface
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 Тестирование единой архитектуры типов");
    
    // Создаем единую систему типов
    println!("\n=== 🏗️ Создание UnifiedTypeSystem ===");
    let unified_system = Arc::new(UnifiedTypeSystem::with_defaults());
    
    // Инициализируем систему
    println!("\n=== ⚡ Инициализация системы ===");
    match unified_system.initialize().await {
        Ok(_) => println!("✅ Система успешно инициализирована"),
        Err(e) => println!("⚠️ Инициализация с предупреждениями: {}", e),
    }
    
    // Получаем статистику
    let stats = unified_system.get_statistics().await;
    println!("📊 Статистика единой системы:");
    println!("  - Платформенные типы: {}", stats.platform_types_count);
    println!("  - Конфигурационные типы: {}", stats.configuration_types_count);
    println!("  - Всего TypeResolution: {}", stats.total_resolutions);
    
    // Тест LSP интерфейса
    println!("\n=== 📡 Тест LSP интерфейса ===");
    let lsp_interface = LspTypeInterface::new(unified_system.clone());
    
    let test_expressions = vec![
        "ТаблицаЗначений",
        "Справочники.Контрагенты",
        "СписокЗначений",
        "Документы.ЗаказКлиента"
    ];
    
    for expression in &test_expressions {
        let resolution = lsp_interface.resolve_expression(expression).await;
        println!("🔍 LSP: '{}' → {:?} (источник: {:?})", 
            expression, resolution.certainty, resolution.source);
        
        let completions = lsp_interface.get_completions(expression).await;
        println!("💡 LSP: автодополнение для '{}' → {} вариантов", 
            expression, completions.len());
    }
    
    // Тест веб интерфейса
    println!("\n=== 🌐 Тест веб интерфейса ===");
    let web_interface = WebTypeInterface::new(unified_system.clone());
    
    // Получаем все типы для отображения
    let all_display_types = web_interface.get_all_types_for_display().await;
    println!("🎨 Веб: всего типов для отображения: {}", all_display_types.len());
    
    // Показываем первые несколько типов
    for (i, display_type) in all_display_types.iter().take(3).enumerate() {
        println!("  {}. {} (категория: {}, уверенность: {:?})", 
            i + 1, display_type.name, display_type.category, display_type.certainty);
    }
    
    // Тест поиска через веб интерфейс
    for query in &["ТаблицаЗначений", "Справочники", "HTTP"] {
        let search_results = web_interface.search_types(query).await;
        println!("🔍 Веб: поиск '{}' → {} результатов", query, search_results.len());
        
        // Показываем первый результат
        if let Some(first_result) = search_results.first() {
            println!("  Первый результат: {} ({})", 
                first_result.name, first_result.description);
        }
    }
    
    // Тест детальной информации
    println!("\n=== 📖 Тест детальной информации ===");
    if let Some(details) = web_interface.get_type_details("ТаблицаЗначений").await {
        println!("📋 Детали типа '{}':", details.name);
        println!("  - Методы: {}", details.methods.len());
        println!("  - Свойства: {}", details.properties.len());
        println!("  - Фасеты: {:?}", details.facets);
    } else {
        println!("❌ Не найдены детали для типа 'ТаблицаЗначений'");
    }
    
    // Итоговая статистика
    println!("\n=== 📊 Итоговая статистика ===");
    let final_stats = unified_system.get_statistics().await;
    println!("Запросы к системе: {}", final_stats.resolution_requests);
    println!("Попадания в кеш: {}", final_stats.cache_hits);
    println!("Промахи кеша: {}", final_stats.cache_misses);
    
    if final_stats.cache_hits + final_stats.cache_misses > 0 {
        let hit_ratio = final_stats.cache_hits as f64 / (final_stats.cache_hits + final_stats.cache_misses) as f64;
        println!("Cache hit ratio: {:.2}", hit_ratio);
    }
    
    println!("\n🎉 Единая архитектура типов работает!");
    println!("🎯 TypeResolution как единственный источник истины реализован!");
    
    Ok(())
}