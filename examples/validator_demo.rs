//! Демонстрация работы TypeValidator с TypeMetadataLookup
//!
//! Показывает как интегрированная валидация проверяет методы и свойства
//! на реальных данных платформы 1С

use bsl_backend::data::adapters::converters::convert_syntax_helper_to_raw;
use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::types::{
    Certainty, ConcreteType, PlatformType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution,
};
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::domain::TypeMetadataLookup;
use std::sync::Arc;

fn main() {
    println!("🧪 Демонстрация TypeValidator с TypeMetadataLookup\n");

    // 1. Парсим синтаксис-помощник
    println!("📚 Загружаем типы платформы 1С...");
    let mut parser = SyntaxHelperParser::new();
    parser
        .parse_directory("examples/syntax_helper", None::<fn(_)>)
        .expect("Failed to parse syntax helper");

    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);
    println!("✅ Загружено {} типов\n", parsed_types.len());

    // 2. Создаём repository и lookup
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(parsed_types)
        .expect("Failed to load types");

    let lookup = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&lookup);

    println!("═══════════════════════════════════════════════════════════════\n");

    // 3. Демонстрация валидации методов
    demonstrate_method_validation(&validator);

    println!("\n═══════════════════════════════════════════════════════════════\n");

    // 4. Демонстрация валидации свойств
    demonstrate_property_validation(&validator);

    println!("\n═══════════════════════════════════════════════════════════════\n");

    // 5. Case-insensitive поиск
    demonstrate_case_insensitive(&validator);

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("\n✅ Валидация успешно интегрирована с TypeMetadataLookup!");
    println!("📋 Теперь можно проверять существование методов и свойств на реальных данных");
}

fn demonstrate_method_validation(validator: &TypeValidator) {
    println!("🔍 ДЕМОНСТРАЦИЯ: Валидация методов типа Массив\n");

    let array_resolution = TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
            name: "Массив".to_string(),
        })),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };

    // Существующий метод
    println!("  Проверяем: массив.Добавить(значение)");
    match validator.validate_method_exists(&array_resolution, "Добавить") {
        None => println!("  ✅ Метод 'Добавить' существует\n"),
        Some(err) => println!("  ❌ Ошибка: {:?}\n", err),
    }

    // Несуществующий метод
    println!("  Проверяем: массив.НесуществующийМетод()");
    match validator.validate_method_exists(&array_resolution, "НесуществующийМетод")
    {
        None => println!("  ✅ Метод существует\n"),
        Some(err) => {
            use bsl_shared::ir::Span;
            let span = Span { start_line: 10, start_column: 5, end_line: 10, end_column: 20 };
            let diagnostic = err.to_diagnostic(span);
            println!("  ❌ ОШИБКА ВАЛИДАЦИИ:");
            println!("     {}", diagnostic.message);
            println!(
                "     Строка: {}, Колонка: {}\n",
                diagnostic.line, diagnostic.column
            );
        }
    }

    // Другой существующий метод
    println!("  Проверяем: массив.Количество()");
    match validator.validate_method_exists(&array_resolution, "Количество") {
        None => println!("  ✅ Метод 'Количество' существует"),
        Some(err) => println!("  ❌ Ошибка: {:?}", err),
    }
}

fn demonstrate_property_validation(validator: &TypeValidator) {
    println!("🔍 ДЕМОНСТРАЦИЯ: Валидация свойств типа ТаблицаЗначений\n");

    let table_resolution = TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
            name: "ТаблицаЗначений".to_string(),
        })),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };

    // Существующее свойство
    println!("  Проверяем: таблица.Колонки");
    match validator.validate_property_exists(&table_resolution, "Колонки") {
        None => println!("  ✅ Свойство 'Колонки' существует\n"),
        Some(err) => println!("  ❌ Ошибка: {:?}\n", err),
    }

    // Несуществующее свойство
    println!("  Проверяем: таблица.НесуществующееСвойство");
    match validator.validate_property_exists(&table_resolution, "НесуществующееСвойство")
    {
        None => println!("  ✅ Свойство существует\n"),
        Some(err) => {
            use bsl_shared::ir::Span;
            let span = Span { start_line: 15, start_column: 10, end_line: 15, end_column: 25 };
            let diagnostic = err.to_diagnostic(span);
            println!("  ❌ ОШИБКА ВАЛИДАЦИИ:");
            println!("     {}", diagnostic.message);
            println!(
                "     Строка: {}, Колонка: {}\n",
                diagnostic.line, diagnostic.column
            );
        }
    }

    // Другое существующее свойство
    println!("  Проверяем: таблица.Индексы");
    match validator.validate_property_exists(&table_resolution, "Индексы") {
        None => println!("  ✅ Свойство 'Индексы' существует"),
        Some(err) => println!("  ❌ Ошибка: {:?}", err),
    }
}

fn demonstrate_case_insensitive(validator: &TypeValidator) {
    println!("🔍 ДЕМОНСТРАЦИЯ: Case-insensitive поиск\n");

    let array_resolution = TypeResolution {
        certainty: Certainty::Known,
        result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
            name: "Массив".to_string(),
        })),
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };

    let test_cases = vec![
        "Добавить",
        "добавить",
        "ДОБАВИТЬ",
        "ДоБаВиТь",
        "Add", // Английское имя
        "add",
    ];

    for method_name in test_cases {
        match validator.validate_method_exists(&array_resolution, method_name) {
            None => println!("  ✅ '{}' → найден", method_name),
            Some(_) => println!("  ❌ '{}' → НЕ найден", method_name),
        }
    }
}
