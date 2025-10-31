//! Тесты для проверки лимитов методов и свойств в HoverFormatter

use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter, OutputFormat};
use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::types::{
    Certainty, ConcreteType, PlatformType, RawDataSource, RawMethodData, RawPropertyData,
    RawTypeData, ResolutionMetadata, ResolutionResult, ResolutionSource, TypeResolution,
};
use std::sync::Arc;

fn create_test_repository_with_n_methods(method_count: usize) -> Arc<InMemoryTypeRepository> {
    use bsl_shared::domain::types::FacetKind;

    let repo = Arc::new(InMemoryTypeRepository::new());

    let methods: Vec<RawMethodData> = (0..method_count)
        .map(|i| RawMethodData {
            name: format!("Метод{}", i),
            english_name: format!("Method{}", i),
            return_type: "Строка".to_string(),
            params: vec![],
        })
        .collect();

    let test_type = RawTypeData {
        name: "ТестовыйТип".to_string(),
        english_name: "TestType".to_string(),
        description: "Тип для тестирования лимитов".to_string(),
        category: "Test".to_string(),
        source: RawDataSource::Platform,
        methods,
        properties: vec![],
        facets: vec![FacetKind::Object],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    };

    repo.load_types(vec![test_type]).unwrap();
    repo
}

fn create_test_repository_with_n_properties(property_count: usize) -> Arc<InMemoryTypeRepository> {
    use bsl_shared::domain::types::FacetKind;

    let repo = Arc::new(InMemoryTypeRepository::new());

    let properties: Vec<RawPropertyData> = (0..property_count)
        .map(|i| RawPropertyData {
            name: format!("Свойство{}", i),
            prop_type: "Строка".to_string(),
            is_readonly: false,
        })
        .collect();

    let test_type = RawTypeData {
        name: "ТестовыйТип".to_string(),
        english_name: "TestType".to_string(),
        description: "Тип для тестирования лимитов свойств".to_string(),
        category: "Test".to_string(),
        source: RawDataSource::Platform,
        methods: vec![],
        properties,
        facets: vec![FacetKind::Object],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
    };

    repo.load_types(vec![test_type]).unwrap();
    repo
}

fn create_test_resolution() -> TypeResolution {
    TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
            name: "ТестовыйТип".to_string(),
        })),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    }
}

#[test]
fn test_methods_limit_respected() {
    // Создаём тип с 50 методами
    let repo = create_test_repository_with_n_methods(50);
    let metadata_lookup = TypeMetadataLookup::new(repo);

    let config = HoverFormatConfig {
        max_methods: 10,
        ..Default::default()
    };

    let formatter = HoverFormatter::new(config, metadata_lookup);
    let resolution = create_test_resolution();
    let hover = formatter.format_variable("Переменная", &resolution);

    // Проверка 1: Hover содержит "показано 10 из 50"
    assert!(
        hover.contains("показано 10 из 50"),
        "Hover должен содержать информацию о лимите: {}",
        hover
    );

    // Проверка 2: Показаны первые 10 методов
    assert!(hover.contains("Метод0"), "Должен быть Метод0");
    assert!(hover.contains("Метод9"), "Должен быть Метод9");

    // Проверка 3: НЕ показаны методы за пределами лимита
    assert!(
        !hover.contains("Метод10"),
        "Метод10 не должен отображаться (за пределами лимита)"
    );
    assert!(
        !hover.contains("Метод49"),
        "Метод49 не должен отображаться (за пределами лимита)"
    );

    // Проверка 4: Есть сообщение "... и ещё N методов"
    assert!(
        hover.contains("... и ещё 40 методов"),
        "Должно быть сообщение о дополнительных методах: {}",
        hover
    );
}

#[test]
fn test_properties_limit_respected() {
    // Создаём тип с 20 свойствами
    let repo = create_test_repository_with_n_properties(20);
    let metadata_lookup = TypeMetadataLookup::new(repo);

    let config = HoverFormatConfig {
        max_properties: 5,
        ..Default::default()
    };

    let formatter = HoverFormatter::new(config, metadata_lookup);
    let resolution = create_test_resolution();
    let hover = formatter.format_variable("Переменная", &resolution);

    // Проверка 1: Hover содержит "показано 5 из 20"
    assert!(
        hover.contains("показано 5 из 20"),
        "Hover должен содержать информацию о лимите свойств: {}",
        hover
    );

    // Проверка 2: Показаны первые 5 свойств
    assert!(hover.contains("Свойство0"), "Должно быть Свойство0");
    assert!(hover.contains("Свойство4"), "Должно быть Свойство4");

    // Проверка 3: НЕ показаны свойства за пределами лимита
    assert!(
        !hover.contains("Свойство5"),
        "Свойство5 не должно отображаться (за пределами лимита)"
    );
    assert!(
        !hover.contains("Свойство19"),
        "Свойство19 не должно отображаться (за пределами лимита)"
    );

    // Проверка 4: Есть сообщение "... и ещё N свойств"
    assert!(
        hover.contains("... и ещё 15 свойств"),
        "Должно быть сообщение о дополнительных свойствах: {}",
        hover
    );
}

#[test]
fn test_no_limit_message_when_below_threshold_methods() {
    // Если методов меньше лимита (7 методов, лимит 10),
    // НЕ должно быть сообщения "показано X из Y"

    let repo = create_test_repository_with_n_methods(7);
    let metadata_lookup = TypeMetadataLookup::new(repo);

    let config = HoverFormatConfig {
        max_methods: 10,
        ..Default::default()
    };

    let formatter = HoverFormatter::new(config, metadata_lookup);
    let resolution = create_test_resolution();
    let hover = formatter.format_variable("Переменная", &resolution);

    // Проверка 1: Все 7 методов показаны
    assert!(hover.contains("Метод0"), "Должен быть Метод0");
    assert!(hover.contains("Метод6"), "Должен быть Метод6");

    // Проверка 2: Показано "показано 7 из 7" (т.к. total_count = display_count)
    assert!(
        hover.contains("показано 7 из 7"),
        "Должен быть счётчик '7 из 7': {}",
        hover
    );

    // Проверка 3: НЕТ сообщения "и ещё" (т.к. методов меньше лимита)
    assert!(
        !hover.contains("и ещё"),
        "Не должно быть сообщения 'и ещё' когда методов меньше лимита: {}",
        hover
    );
}

#[test]
fn test_no_limit_message_when_below_threshold_properties() {
    // Если свойств меньше лимита (3 свойства, лимит 5),
    // НЕ должно быть сообщения "и ещё"

    let repo = create_test_repository_with_n_properties(3);
    let metadata_lookup = TypeMetadataLookup::new(repo);

    let config = HoverFormatConfig {
        max_properties: 5,
        ..Default::default()
    };

    let formatter = HoverFormatter::new(config, metadata_lookup);
    let resolution = create_test_resolution();
    let hover = formatter.format_variable("Переменная", &resolution);

    // Проверка 1: Все 3 свойства показаны
    assert!(hover.contains("Свойство0"), "Должно быть Свойство0");
    assert!(hover.contains("Свойство2"), "Должно быть Свойство2");

    // Проверка 2: Показано "показано 3 из 3"
    assert!(
        hover.contains("показано 3 из 3"),
        "Должен быть счётчик '3 из 3': {}",
        hover
    );

    // Проверка 3: НЕТ сообщения "и ещё"
    assert!(
        !hover.contains("и ещё"),
        "Не должно быть сообщения 'и ещё' когда свойств меньше лимита: {}",
        hover
    );
}

#[test]
fn test_markdown_vs_plaintext_formatting() {
    let repo = create_test_repository_with_n_methods(3);
    let metadata_lookup_md = TypeMetadataLookup::new(repo.clone());
    let metadata_lookup_pt = TypeMetadataLookup::new(repo);

    // Markdown
    let config_md = HoverFormatConfig {
        max_methods: 10,
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let formatter_md = HoverFormatter::new(config_md, metadata_lookup_md);
    let resolution = create_test_resolution();
    let hover_md = formatter_md.format_variable("Переменная", &resolution);

    // PlainText
    let config_pt = HoverFormatConfig {
        max_methods: 10,
        output_format: OutputFormat::PlainText,
        ..Default::default()
    };
    let formatter_pt = HoverFormatter::new(config_pt, metadata_lookup_pt);
    let hover_pt = formatter_pt.format_variable("Переменная", &resolution);

    // Markdown форматирование
    assert!(
        hover_md.contains("**Переменная:**"),
        "Markdown должен содержать ** для bold: {}",
        hover_md
    );
    assert!(
        hover_md.contains("🟢"),
        "Markdown должен содержать emoji для certainty"
    );
    assert!(
        hover_md.contains("•"),
        "Markdown должен использовать • для списков"
    );

    // PlainText форматирование
    assert!(
        hover_pt.contains("Переменная:"),
        "PlainText должен содержать обычный текст: {}",
        hover_pt
    );
    assert!(
        !hover_pt.contains("**"),
        "PlainText НЕ должен содержать Markdown: {}",
        hover_pt
    );
    assert!(
        hover_pt.contains("-"),
        "PlainText должен использовать - для списков"
    );
}

#[test]
fn test_exact_limit_boundary() {
    // Граничный случай: ровно max_methods методов (10 методов, лимит 10)

    let repo = create_test_repository_with_n_methods(10);
    let metadata_lookup = TypeMetadataLookup::new(repo);

    let config = HoverFormatConfig {
        max_methods: 10,
        ..Default::default()
    };

    let formatter = HoverFormatter::new(config, metadata_lookup);
    let resolution = create_test_resolution();
    let hover = formatter.format_variable("Переменная", &resolution);

    // Проверка 1: Все 10 методов показаны
    assert!(hover.contains("Метод0"), "Должен быть Метод0");
    assert!(hover.contains("Метод9"), "Должен быть Метод9");

    // Проверка 2: Показано "показано 10 из 10"
    assert!(
        hover.contains("показано 10 из 10"),
        "Должен быть счётчик '10 из 10': {}",
        hover
    );

    // Проверка 3: НЕТ сообщения "и ещё" (ровно на границе лимита)
    assert!(
        !hover.contains("и ещё"),
        "Не должно быть сообщения 'и ещё' при точном совпадении с лимитом: {}",
        hover
    );
}
