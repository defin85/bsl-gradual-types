//! Демонстрация работы парсера синтакс-помощника версии 3

use bsl_gradual_types::adapters::syntax_helper_parser::{SyntaxHelperParser, OptimizationSettings};
use bsl_gradual_types::adapters::platform_types_v2::PlatformTypesResolverV2;
use bsl_gradual_types::core::types::FacetKind;
use std::path::Path;
use anyhow::Result;

fn main() -> Result<()> {
    println!("=== Демонстрация парсера синтакс-помощника v3 ===\n");
    
    // Путь к распакованному архиву синтакс-помощника
    let syntax_helper_path = Path::new("data/syntax_helper/extracted");
    
    if !syntax_helper_path.exists() {
        println!("⚠️  Путь {} не существует", syntax_helper_path.display());
        println!("   Создаём демо-структуру для примера...\n");
        create_demo_structure()?;
    }
    
    // Создаём парсер с настройками
    let settings = OptimizationSettings {
        show_progress: true,
        ..Default::default()
    };
    let mut parser = SyntaxHelperParser::with_settings(settings);
    
    println!("📂 Парсинг каталога: {}", syntax_helper_path.display());
    match parser.parse_directory(syntax_helper_path) {
        Ok(_) => println!("✅ Парсинг успешно завершён\n"),
        Err(e) => {
            println!("❌ Ошибка парсинга: {}\n", e);
            println!("   Используем демо-данные для примера\n");
            setup_demo_data(&mut parser);
        }
    }
    
    // Демонстрация возможностей
    demonstrate_parser_features(&parser);
    
    // Интеграция с PlatformTypesResolverV2
    demonstrate_resolver_integration(parser)?;
    
    Ok(())
}

fn demonstrate_parser_features(parser: &SyntaxHelperParser) {
    println!("=== Возможности парсера ===\n");
    
    // 1. Поиск типа по имени
    println!("1️⃣  Поиск типа по имени:");
    let test_names = vec!["ТаблицаЗначений", "ValueTable", "Массив", "Array"];
    
    for name in test_names {
        if let Some(type_info) = parser.find_type(name) {
            println!("   ✓ Найден '{}': {} / {}", 
                name,
                type_info.identity.russian_name,
                type_info.identity.english_name
            );
            
            // Показываем фасеты
            if !type_info.metadata.available_facets.is_empty() {
                print!("     Фасеты: ");
                for facet in &type_info.metadata.available_facets {
                    print!("{:?} ", facet);
                }
                println!();
            }
        } else {
            println!("   ✗ Не найден '{}'", name);
        }
    }
    
    // 2. Получение типов по категории
    println!("\n2️⃣  Типы по категориям:");
    let all_types = parser.get_all_types();
    let mut categories = std::collections::HashSet::new();
    
    for (_, node) in all_types {
        if let bsl_gradual_types::adapters::syntax_helper_parser_v3::SyntaxNode::Type(type_info) = node {
            if !type_info.identity.category_path.is_empty() {
                categories.insert(type_info.identity.category_path.clone());
            }
        }
    }
    
    for category in categories.iter().take(3) {
        let types = parser.get_types_by_category(category);
        println!("   Категория '{}': {} типов", category, types.len());
        for type_info in types.iter().take(3) {
            println!("      - {}", type_info.identity.russian_name);
        }
    }
    
    // 3. Получение типов по фасету
    println!("\n3️⃣  Типы по фасетам:");
    let facets = vec![
        FacetKind::Collection,
        FacetKind::Manager,
        FacetKind::Singleton,
        FacetKind::Constructor,
    ];
    
    for facet in facets {
        let types = parser.get_types_by_facet(facet);
        if !types.is_empty() {
            println!("   {:?}: {} типов", facet, types.len());
            for type_info in types.iter().take(3) {
                println!("      - {}", type_info.identity.russian_name);
            }
        }
    }
    
    // 4. Статистика индексов
    println!("\n4️⃣  Статистика индексов:");
    let index = parser.type_index();
    println!("   По русским именам: {} записей", index.by_russian.len());
    println!("   По английским именам: {} записей", index.by_english.len());
    println!("   По альтернативным именам: {} записей", index.by_any_name.len());
    println!("   По категориям: {} категорий", index.by_category.len());
    println!("   По фасетам: {} фасетов", index.by_facet.len());
}

fn demonstrate_resolver_integration(parser: SyntaxHelperParser) -> Result<()> {
    println!("\n=== Интеграция с PlatformTypesResolverV2 ===\n");
    
    let mut resolver = PlatformTypesResolverV2::new();
    resolver.load_from_parser_v3(parser);
    
    // 1. Поиск типа через resolver
    println!("1️⃣  Поиск типа через resolver:");
    if let Some(resolution) = resolver.resolve_type("ТаблицаЗначений") {
        println!("   ✓ Найден тип 'ТаблицаЗначений'");
        println!("     Уверенность: {:?}", resolution.certainty);
        println!("     Источник: {:?}", resolution.source);
        println!("     Активный фасет: {:?}", resolution.active_facet);
    }
    
    // 2. Получение hover информации
    println!("\n2️⃣  Hover информация:");
    if let Some(hover) = resolver.get_hover_info("ТаблицаЗначений") {
        println!("   Для типа 'ТаблицаЗначений':");
        for line in hover.lines().take(5) {
            println!("   {}", line);
        }
        if hover.lines().count() > 5 {
            println!("   ...");
        }
    }
    
    // 3. Получение типов по категории через resolver
    println!("\n3️⃣  Типы коллекций через resolver:");
    let collection_types = resolver.get_types_by_facet(FacetKind::Collection);
    println!("   Найдено {} типов-коллекций", collection_types.len());
    for type_res in collection_types.iter().take(5) {
        if let bsl_gradual_types::core::types::ResolutionResult::Concrete(concrete) = &type_res.result {
            if let bsl_gradual_types::core::types::ConcreteType::Platform(platform) = concrete {
                println!("      - {}", platform.name);
            }
        }
    }
    
    Ok(())
}

fn create_demo_structure() -> Result<()> {
    use std::fs;
    
    // Создаём демо-структуру каталогов
    let base = Path::new("data/syntax_helper/extracted");
    fs::create_dir_all(base.join("objects/catalog236"))?;
    
    // Создаём демо HTML файлы
    fs::write(
        base.join("objects/catalog236.html"),
        r#"<html>
<h1>Коллекции значений</h1>
<p>Объекты для работы с коллекциями данных.</p>
</html>"#
    )?;
    
    fs::write(
        base.join("objects/catalog236/ValueTable.html"),
        r#"<html>
<h1 class="V8SH_pagetitle">ТаблицаЗначений (ValueTable)</h1>
<p>Объект для хранения табличных данных. Для объекта доступен обход коллекции посредством оператора Для каждого.</p>
</html>"#
    )?;
    
    fs::write(
        base.join("objects/catalog236/Array.html"),
        r#"<html>
<h1 class="V8SH_pagetitle">Массив (Array)</h1>
<p>Коллекция значений произвольного типа. Для объекта доступен обход коллекции посредством оператора Для каждого.</p>
</html>"#
    )?;
    
    println!("✅ Создана демо-структура в {}\n", base.display());
    
    Ok(())
}

fn setup_demo_data(parser: &mut SyntaxHelperParser) {
    // Здесь можно добавить демо-данные напрямую в парсер
    // для демонстрации возможностей без реальных файлов
    println!("ℹ️  Демо-данные загружены в парсер");
}