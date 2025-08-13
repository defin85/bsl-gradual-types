//! Демонстрация парсера синтакс-помощника 1С

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;
use std::path::Path;
use tracing::{info, Level};
use tracing_subscriber;

fn main() -> anyhow::Result<()> {
    // Настраиваем логирование
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("=== Демонстрация парсера синтакс-помощника 1С ===");
    
    // Пути к файлам синтакс-помощника
    let context_path = "examples/syntax_helper/rebuilt.shcntx_ru.zip";
    let lang_path = "examples/syntax_helper/rebuilt.shlang_ru.zip";
    
    // Проверяем наличие файлов
    if !Path::new(context_path).exists() {
        anyhow::bail!("Файл контекстной справки не найден: {}", context_path);
    }
    
    if !Path::new(lang_path).exists() {
        anyhow::bail!("Файл справки по языку не найден: {}", lang_path);
    }
    
    // Создаём парсер
    let mut parser = SyntaxHelperParser::new()
        .with_context_archive(context_path)
        .with_lang_archive(lang_path);
    
    // Запускаем парсинг
    info!("Запускаем парсинг...");
    parser.parse()?;
    
    // Получаем базу знаний
    let database = parser.database();
    
    // Выводим статистику
    println!("\n📊 Статистика парсинга:");
    println!("  🔧 Глобальных функций: {}", database.global_functions.len());
    println!("  📦 Глобальных объектов: {}", database.global_objects.len());
    println!("  🎯 Методов объектов: {}", database.object_methods.len());
    println!("  ⚙️ Свойств объектов: {}", database.object_properties.len());
    println!("  📝 Системных перечислений: {}", database.system_enums.len());
    println!("  🔤 Ключевых слов: {}", database.keywords.len());
    println!("  ➕ Операторов: {}", database.operators.len());
    
    // Показываем примеры найденных функций
    println!("\n🔧 Примеры глобальных функций:");
    for (name, func) in database.global_functions.iter().take(10) {
        println!("  - {} ({})", name, func.description.as_deref().unwrap_or("Без описания"));
    }
    
    // Показываем примеры ключевых слов
    println!("\n🔤 Примеры ключевых слов:");
    for keyword in database.keywords.iter().take(15) {
        println!("  - {} ({})", keyword.russian, keyword.english);
    }
    
    // Сохраняем результат в JSON файл
    let output_path = "examples/syntax_helper/syntax_database.json";
    parser.save_to_file(output_path)?;
    info!("База знаний сохранена в: {}", output_path);
    
    println!("\n✅ Парсинг завершён успешно!");
    
    Ok(())
}