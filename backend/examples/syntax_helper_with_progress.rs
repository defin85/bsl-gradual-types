//! Пример использования SyntaxHelperParser с progress callback
//!
//! Демонстрирует 4 фазы парсинга:
//! - Collecting Files (0-10%)
//! - Parsing Files (10-70%)
//! - Linking Categories (70-90%)
//! - Building Indexes (90-100%)
//!
//! Запуск:
//! ```bash
//! cargo run --example syntax_helper_with_progress
//! ```

use bsl_backend::data::loaders::progress::{IndexingPhase, ProgressUpdate};
use bsl_backend::data::loaders::syntax_helper::types::OptimizationSettings;
use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;

fn main() {
    // Настройка логирования
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 Запуск парсинга Syntax Helper с отслеживанием прогресса\n");

    // Создаём парсер с оптимизированными настройками
    let settings = OptimizationSettings {
        max_threads: Some(4),
        batch_size: 100,
        show_progress: false, // Отключаем indicatif, используем свой callback
        file_limit: Some(50), // Ограничиваем для быстрой демонстрации
        ..Default::default()
    };

    let mut parser = SyntaxHelperParser::with_settings(settings);

    // Создаём callback для отслеживания прогресса
    let progress_callback = |update: ProgressUpdate| {
        let phase_icon = match update.phase {
            IndexingPhase::CollectingFiles => "📂",
            IndexingPhase::ParsingFiles => "📄",
            IndexingPhase::LinkingCategories => "🔗",
            IndexingPhase::BuildingIndexes => "📚",
            IndexingPhase::ConfigurationDiscovery => "🔍",
            IndexingPhase::ConfigurationParsing => "📋",
            IndexingPhase::ConfigurationLinking => "⛓",
            IndexingPhase::ConfigurationFinalizing => "✅",
        };

        let message = update.message.as_deref().unwrap_or("Обработка...");

        println!(
            "{} [{:?}] {:.1}% ({}/{}) - {}",
            phase_icon, update.phase, update.percentage, update.current, update.total, message
        );
    };

    // Парсим с callback (указываем путь к rebuilt.shcntx_ru)
    match parser.parse_directory(
        "../examples/syntax_helper/rebuilt.shcntx_ru",
        Some(progress_callback),
    ) {
        Ok(()) => {
            println!("\n✅ Парсинг завершён успешно!\n");

            // Выводим статистику
            let stats = parser.get_stats();
            println!("📊 Статистика парсинга:");
            println!("   Обработано файлов: {}", stats.processed_files);
            println!("   Найдено типов: {}", stats.types_count);
            println!("   Найдено методов: {}", stats.methods_count);
            println!("   Найдено свойств: {}", stats.properties_count);
            println!("   Категорий: {}", stats.categories_count);
            println!("   Размер индекса: {}", stats.index_size);
            println!("   Ошибок: {}", stats.error_count);
        }
        Err(e) => {
            eprintln!("❌ Ошибка парсинга: {}", e);
            std::process::exit(1);
        }
    }
}
