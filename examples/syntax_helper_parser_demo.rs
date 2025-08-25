//! Демонстрация парсера синтакс-помощника 1С

use bsl_gradual_types::data::loaders::syntax_helper_parser::SyntaxHelperParser;
use std::path::Path;
use tracing::{info, Level};
use tracing_subscriber;

fn main() -> anyhow::Result<()> {
    // Настраиваем логирование
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("=== Демонстрация парсера синтакс-помощника 1С ===");

    // Пути к директориям с распакованными файлами синтакс-помощника
    let context_path = "examples/syntax_helper/rebuilt.shcntx_ru";
    let lang_path = "examples/syntax_helper/rebuilt.shlang_ru";

    // Проверяем наличие директорий
    if !Path::new(context_path).exists() {
        anyhow::bail!(
            "Директория контекстной справки не найдена: {}",
            context_path
        );
    }

    if !Path::new(lang_path).exists() {
        anyhow::bail!("Директория справки по языку не найдена: {}", lang_path);
    }

    // Создаём парсер
    let mut parser = SyntaxHelperParser::new();

    // Запускаем парсинг директорий
    info!("Запускаем парсинг контекстной справки...");
    parser.parse_directory(context_path)?;

    info!("Запускаем парсинг справки по языку...");
    parser.parse_directory(lang_path)?;

    // Получаем статистику парсинга
    let stats = parser.get_stats();

    // Выводим статистику
    println!("\n📊 Статистика парсинга:");
    println!("  📂 Обработано файлов: {}", stats.files_parsed);
    println!("  ⏱️ Время парсинга: {:?}", stats.parse_duration);
    println!("  📦 Найдено типов: {}", stats.types_count);
    println!("  🎯 Найдено методов: {}", stats.methods_count);
    println!("  ⚙️ Найдено свойств: {}", stats.properties_count);
    println!("  📑 Найдено категорий: {}", stats.categories_count);

    // Получаем базу данных
    let database = parser.export_database();

    // Показываем примеры типов
    println!("\n📦 Примеры найденных типов:");
    for (name, _node) in database.nodes.iter().take(5) {
        println!("  - {}", name);
    }

    // Показываем примеры методов
    println!("\n🎯 Примеры найденных методов:");
    for (name, method) in database.methods.iter().take(5) {
        println!("  - {} (параметры: {})", name, method.parameters.len());
    }

    // Показываем примеры свойств
    println!("\n⚙️ Примеры найденных свойств:");
    for (name, prop) in database.properties.iter().take(5) {
        println!("  - {} (тип: {})", name, prop.property_type);
    }

    // Показываем примеры категорий
    println!("\n📑 Примеры категорий:");
    for (name, category) in database.categories.iter().take(5) {
        println!("  - {} ({} типов)", name, category.types.len());
    }

    // Получаем индекс для поиска
    let index = parser.export_index();

    println!("\n🔍 Индексы для поиска:");
    println!("  - Русских названий: {}", index.by_russian.len());
    println!("  - Английских названий: {}", index.by_english.len());

    // Пример поиска типа
    if let Some(type_info) = parser.find_type("Массив") {
        println!("\n✨ Найден тип 'Массив':");
        println!("  - Русское имя: {}", type_info.identity.russian_name);
        if let Some(en) = type_info.identity.english_name.as_ref() {
            println!("  - Английское имя: {}", en);
        }
        println!("  - Методов: {}", type_info.structure.methods.len());
        println!("  - Свойств: {}", type_info.structure.properties.len());
    }

    println!("\n✅ Парсинг завершён успешно!");

    Ok(())
}
