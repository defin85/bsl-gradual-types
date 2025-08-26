//! Демонстрация работы унифицированного парсера синтакс-помощника

use anyhow::Result;
use bsl_gradual_types::data::loaders::platform_types_v2::PlatformTypesResolverV2;
use bsl_gradual_types::data::loaders::syntax_helper_parser::{
    OptimizationSettings, SyntaxHelperParser, SyntaxNode,
};
use bsl_gradual_types::domain::types::FacetKind;
use std::path::Path;

fn main() -> Result<()> {
    println!("=== Демонстрация унифицированного парсера синтакс-помощника ===\n");

    // Путь к распакованному архиву синтакс-помощника
    let syntax_helper_path = if Path::new("examples/syntax_helper").exists() {
        Path::new("examples/syntax_helper")
    } else if Path::new("data/syntax_helper/extracted").exists() {
        Path::new("data/syntax_helper/extracted")
    } else {
        Path::new("data/syntax_helper")
    };

    // Создаём парсер с настройками
    let settings = OptimizationSettings {
        show_progress: true,
        parallel_indexing: true,
        ..Default::default()
    };
    let mut parser = SyntaxHelperParser::with_settings(settings);

    println!("📂 Парсинг каталога: {}", syntax_helper_path.display());

    if syntax_helper_path.exists() {
        match parser.parse_directory(syntax_helper_path) {
            Ok(_) => println!("✅ Парсинг успешно завершён\n"),
            Err(e) => {
                println!("❌ Ошибка парсинга: {}\n", e);
                return Err(e);
            }
        }
    } else {
        println!("⚠️  Путь {} не существует", syntax_helper_path.display());
        println!("   Создайте директорию и распакуйте туда файлы синтакс-помощника\n");
        return Ok(());
    }

    // Демонстрация возможностей
    demonstrate_parser_features(&parser);

    // Интеграция с PlatformTypesResolverV2
    demonstrate_resolver_integration(&parser)?;

    Ok(())
}

fn demonstrate_parser_features(parser: &SyntaxHelperParser) {
    println!("=== Возможности парсера ===\n");

    // Экспортируем данные для анализа
    let database = parser.export_database();
    let index = parser.export_index();

    // 1. Статистика загруженных данных
    println!("1️⃣  Статистика загруженных данных:");
    let stats = parser.get_stats();
    println!("   Всего файлов: {}", stats.total_files);
    println!("   Файлов обработано: {}", stats.processed_files);
    println!("   Ошибок при парсинге: {}", stats.error_count);
    println!("   Узлов создано: {}", stats.total_nodes);
    println!("   Типов: {}", stats.types_count);
    println!("   Методов: {}", stats.methods_count);
    println!("   Свойств: {}", stats.properties_count);

    // 2. Примеры типов по категориям
    println!("\n2️⃣  Типы по категориям:");
    let mut categories_map = std::collections::HashMap::new();

    for (_, node) in database.nodes.iter() {
        if let SyntaxNode::Type(type_info) = node {
            let category = if type_info.identity.category_path.is_empty() {
                "Без категории"
            } else {
                &type_info.identity.category_path
            };
            categories_map
                .entry(category.to_string())
                .or_insert_with(Vec::new)
                .push(type_info.identity.russian_name.clone());
        }
    }

    for (category, types) in categories_map.iter().take(5) {
        println!("   {} ({} типов)", category, types.len());
        for type_name in types.iter().take(3) {
            println!("      - {}", type_name);
        }
    }

    // 3. Индексы для быстрого поиска
    println!("\n3️⃣  Индексы для быстрого поиска:");
    println!("   По русским именам: {} записей", index.by_russian.len());
    println!(
        "   По английским именам: {} записей",
        index.by_english.len()
    );
    println!("   По категориям: {} категорий", index.by_category.len());
    println!("   По фасетам: {} типов фасетов", index.by_facet.len());

    // 4. Примеры типов по фасетам
    println!("\n4️⃣  Типы по фасетам:");
    for (facet, type_paths) in index.by_facet.iter() {
        let facet_name = match facet {
            FacetKind::Collection => "Collection (коллекции)",
            FacetKind::Manager => "Manager (менеджеры)",
            FacetKind::Singleton => "Singleton (глобальные)",
            FacetKind::Constructor => "Constructor (конструируемые)",
            _ => "Other",
        };
        println!("   {} - {} типов", facet_name, type_paths.len());

        // Показываем первые 3 типа
        for path in type_paths.iter().take(3) {
            if let Some(SyntaxNode::Type(type_info)) = database.nodes.get(path) {
                println!("      - {}", type_info.identity.russian_name);
            }
        }
    }

    // 5. Примеры методов и свойств
    println!("\n5️⃣  Методы и свойства:");
    println!("   Всего методов: {}", database.methods.len());
    println!("   Всего свойств: {}", database.properties.len());

    // Показываем несколько примеров методов
    for (name, method) in database.methods.iter().take(3) {
        println!(
            "   Метод: {} -> {}",
            method.name,
            method.return_type.as_deref().unwrap_or("Неизвестно")
        );
    }
}

fn demonstrate_resolver_integration(parser: &SyntaxHelperParser) -> Result<()> {
    println!("\n=== Интеграция с PlatformTypesResolverV2 ===\n");

    // Создаём resolver и сохраняем базу данных
    let temp_file = std::env::temp_dir().join("syntax_helper_db.json");
    let database = parser.export_database();

    // Сохраняем в JSON
    let json_str = serde_json::to_string_pretty(&database)?;
    std::fs::write(&temp_file, json_str)?;

    // Загружаем в resolver
    let mut resolver = PlatformTypesResolverV2::new();
    resolver.load_from_file(&temp_file)?;

    // 1. Разрешение типов
    println!("1️⃣  Разрешение типов через resolver:");
    let test_types = vec!["Строка", "Массив", "ТаблицаЗначений", "Неизвестный"];

    for type_name in test_types {
        let resolution = resolver.resolve(type_name);
        println!(
            "   {} -> Уверенность: {:?}",
            type_name, resolution.certainty
        );
    }

    // 2. Глобальные функции
    println!("\n2️⃣  Глобальные функции:");
    let functions = resolver.get_global_functions();
    println!("   Всего функций: {}", functions.len());
    for (name, _) in functions.iter().take(5) {
        println!("      - {}", name);
    }

    // 3. Глобальные объекты
    println!("\n3️⃣  Глобальные объекты:");
    let objects = resolver.get_global_objects();
    println!("   Всего объектов: {}", objects.len());
    for (name, _) in objects.iter().take(5) {
        println!("      - {}", name);
    }

    // 4. Методы и свойства объектов
    println!("\n4️⃣  Методы и свойства объектов:");
    if let Some((type_name, _)) = objects.iter().next() {
        let methods = resolver.get_object_methods(type_name);
        let properties = resolver.get_object_properties(type_name);

        println!("   Тип '{}:'", type_name);
        println!("      Методов: {}", methods.len());
        println!("      Свойств: {}", properties.len());

        for method in methods.iter().take(3) {
            println!(
                "      • {}() -> {}",
                method.name,
                method.return_type.as_deref().unwrap_or("Неизвестно")
            );
        }
    }

    // Удаляем временный файл
    std::fs::remove_file(temp_file).ok();

    Ok(())
}
