//! Integration test для TypeMetadataLookup с реальными данными из SyntaxHelper

use bsl_backend::data::adapters::converters::convert_syntax_helper_to_raw;
use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;
use bsl_backend::data::loaders::progress::ProgressUpdate;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::types::{
    Certainty, ConcreteType, PlatformType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution,
};
use bsl_shared::domain::TypeMetadataLookup;
use std::sync::Arc;

#[test]
fn test_metadata_lookup_with_real_syntax_helper() {
    // 1. Парсим синтаксис-помощник
    let mut parser = SyntaxHelperParser::new();
    parser
        .parse_directory("examples/syntax_helper", None::<fn(ProgressUpdate)>)
        .expect("Failed to parse syntax helper");

    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);

    println!("✅ Parsed {} types from syntax helper", parsed_types.len());

    // 2. Загружаем в repository
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(parsed_types)
        .expect("Failed to load types");

    let stats = repository.get_stats();
    println!("📊 Repository stats: {} total types", stats.total_types);

    // 3. Создаем TypeMetadataLookup
    let lookup = TypeMetadataLookup::new(repository.clone());

    // 4. Проверяем известные типы с методами
    let test_types = vec!["Массив", "ТаблицаЗначений", "Строка"];

    for type_name in test_types {
        println!("\n🔍 Testing type: {}", type_name);

        // Проверяем что тип есть в repository
        let raw_type = repository.find_type(type_name);
        match &raw_type {
            Some(rt) => {
                println!(
                    "  ✅ Found in repository: {} methods, {} properties",
                    rt.methods.len(),
                    rt.properties.len()
                );
                if !rt.methods.is_empty() {
                    println!("     First method: {}", rt.methods[0].name);
                }
            }
            None => println!("  ❌ NOT found in repository!"),
        }

        // Создаем TypeResolution как это делает TypeInferenceService
        let resolution = TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: type_name.to_string(),
            })),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        // Проверяем TypeMetadataLookup
        let methods = lookup.get_methods(&resolution);
        println!("  🔍 TypeMetadataLookup found {} methods", methods.len());

        // Сравниваем
        if let Some(rt) = raw_type {
            assert_eq!(
                methods.len(),
                rt.methods.len(),
                "Methods count mismatch for type {}",
                type_name
            );
            println!("  ✅ PASS: Methods count matches!");
        }
    }
}

#[test]
fn test_repository_content_sample() {
    // Быстрый тест - просто посмотрим что есть в repository
    let mut parser = SyntaxHelperParser::new();
    parser
        .parse_directory("examples/syntax_helper", None::<fn(ProgressUpdate)>)
        .expect("Failed to parse");

    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);

    let repository = Arc::new(InMemoryTypeRepository::new());
    repository.load_types(parsed_types).expect("Failed to load");

    let all_types = repository.get_all_types();

    println!("\n📋 First 10 types with methods:");
    let types_with_methods: Vec<_> = all_types
        .iter()
        .filter(|t| !t.methods.is_empty())
        .take(10)
        .collect();

    for raw_type in types_with_methods {
        println!("  - {} ({} methods)", raw_type.name, raw_type.methods.len());
    }

    // Ищем конкретно "Массив"
    println!("\n🔍 Looking for 'Массив' type...");
    if let Some(array_type) = repository.find_type("Массив") {
        println!(
            "  ✅ Found! {} methods, {} properties",
            array_type.methods.len(),
            array_type.properties.len()
        );
    } else {
        println!("  ❌ NOT FOUND");

        // Поищем похожие
        let similar: Vec<_> = all_types
            .iter()
            .filter(|t| t.name.contains("Масс") || t.english_name.to_lowercase().contains("array"))
            .take(5)
            .collect();

        if !similar.is_empty() {
            println!("  📝 Similar types found:");
            for t in similar {
                println!("     - {} (en: {})", t.name, t.english_name);
            }
        }
    }
}
