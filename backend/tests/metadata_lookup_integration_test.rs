//! Integration test для TypeMetadataLookup с реальными данными из SyntaxHelper

mod support;

use bsl_shared::domain::types::{
    Certainty, ConcreteType, PlatformType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution,
};
use bsl_shared::domain::TypeMetadataLookup;

#[test]
fn test_metadata_lookup_with_real_syntax_helper() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let repository = deps_bundle.semantic_deps.repository.clone();

    let stats = repository.get_stats();
    println!("📊 Repository stats: {} total types", stats.total_types);

    // 2. Создаем TypeMetadataLookup
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

        // Создаем TypeResolution для платформенного типа
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
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let repository = deps_bundle.semantic_deps.repository.clone();

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
