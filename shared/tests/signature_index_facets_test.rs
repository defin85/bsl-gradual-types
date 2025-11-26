//! Unit тесты для Milestone 3.11 Phase 1 - Method Signature Enhancement
//!
//! Тестирование:
//! - ContextRequirements enum
//! - MethodSignature с facets
//! - SignatureIndex с facet методами
//! - Интеграция с platform_types

use bsl_shared::domain::signature_index::{
    ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::types::{FacetKind, ParameterInfo};

// ============================================================================
// ТЕСТЫ ДЛЯ ContextRequirements
// ============================================================================

#[test]
fn test_context_requirements_default() {
    let ctx = ContextRequirements::default();
    assert_eq!(ctx, ContextRequirements::Universal);
}

#[test]
fn test_context_requirements_is_universal() {
    assert!(ContextRequirements::Universal.is_universal());
    assert!(ContextRequirements::ServerPreferred.is_universal());
    assert!(!ContextRequirements::ServerOnly.is_universal());
    assert!(!ContextRequirements::ClientOnly.is_universal());
}

#[test]
fn test_context_requirements_allows_server() {
    assert!(ContextRequirements::ServerOnly.allows_server());
    assert!(ContextRequirements::Universal.allows_server());
    assert!(ContextRequirements::ServerPreferred.allows_server());
    assert!(!ContextRequirements::ClientOnly.allows_server());
}

#[test]
fn test_context_requirements_allows_client() {
    assert!(ContextRequirements::ClientOnly.allows_client());
    assert!(ContextRequirements::Universal.allows_client());
    assert!(ContextRequirements::ServerPreferred.allows_client());
    assert!(!ContextRequirements::ServerOnly.allows_client());
}

#[test]
fn test_context_requirements_server_only() {
    let ctx = ContextRequirements::ServerOnly;
    assert!(!ctx.is_universal());
    assert!(ctx.allows_server());
    assert!(!ctx.allows_client());
}

#[test]
fn test_context_requirements_client_only() {
    let ctx = ContextRequirements::ClientOnly;
    assert!(!ctx.is_universal());
    assert!(!ctx.allows_server());
    assert!(ctx.allows_client());
}

#[test]
fn test_context_requirements_server_preferred() {
    let ctx = ContextRequirements::ServerPreferred;
    assert!(ctx.is_universal());
    assert!(ctx.allows_server());
    assert!(ctx.allows_client());
}

// ============================================================================
// ТЕСТЫ ДЛЯ MethodSignature с facets
// ============================================================================

#[test]
fn test_method_signature_with_facet_object() {
    let sig = MethodSignature::new(
        "СоздатьЭлемент".to_string(),
        Some("СправочникМенеджер.Номенклатура".to_string()),
        vec![],
        Some("СправочникОбъект.Номенклатура".to_string()),
        SignatureSource::Platform,
        Some(FacetKind::Object),
        ContextRequirements::ServerOnly,
    );

    assert_eq!(sig.name, "СоздатьЭлемент");
    assert_eq!(sig.return_facet, Some(FacetKind::Object));
    assert_eq!(sig.context_requirements, ContextRequirements::ServerOnly);
}

#[test]
fn test_method_signature_with_facet_reference() {
    let sig = MethodSignature::new(
        "НайтиПоКоду".to_string(),
        Some("СправочникМенеджер.Номенклатура".to_string()),
        vec![ParameterInfo {
            name: "Код".to_string(),
            type_name: Some("Строка".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        }],
        Some("СправочникСсылка.Номенклатура".to_string()),
        SignatureSource::Platform,
        Some(FacetKind::Reference),
        ContextRequirements::ServerOnly,
    );

    assert_eq!(sig.name, "НайтиПоКоду");
    assert_eq!(sig.return_facet, Some(FacetKind::Reference));
    assert_eq!(sig.params.len(), 1);
    assert_eq!(sig.params[0].name, "Код");
}

#[test]
fn test_method_signature_with_facet_selection() {
    let sig = MethodSignature::new(
        "Выбрать".to_string(),
        Some("СправочникМенеджер.Номенклатура".to_string()),
        vec![],
        Some("СправочникВыборка.Номенклатура".to_string()),
        SignatureSource::Platform,
        Some(FacetKind::Selection),
        ContextRequirements::ServerOnly,
    );

    assert_eq!(sig.name, "Выбрать");
    assert_eq!(sig.return_facet, Some(FacetKind::Selection));
}

#[test]
fn test_method_signature_without_facet() {
    let sig = MethodSignature::new(
        "Количество".to_string(),
        Some("Массив".to_string()),
        vec![],
        Some("Число".to_string()),
        SignatureSource::Platform,
        None,
        ContextRequirements::Universal,
    );

    assert_eq!(sig.name, "Количество");
    assert_eq!(sig.return_facet, None);
    assert_eq!(sig.context_requirements, ContextRequirements::Universal);
}

#[test]
fn test_method_signature_universal_context() {
    let sig = MethodSignature::new(
        "ПустаяСсылка".to_string(),
        Some("СправочникМенеджер.Номенклатура".to_string()),
        vec![],
        Some("СправочникСсылка.Номенклатура".to_string()),
        SignatureSource::Platform,
        Some(FacetKind::Reference),
        ContextRequirements::Universal,
    );

    assert_eq!(sig.name, "ПустаяСсылка");
    assert_eq!(sig.context_requirements, ContextRequirements::Universal);
    assert!(sig.context_requirements.is_universal());
}

// ============================================================================
// ТЕСТЫ ДЛЯ SignatureIndex с facet методами
// ============================================================================

#[test]
fn test_signature_index_add_facet_method() {
    let mut index = SignatureIndex::new();

    let sig = MethodSignature::new(
        "СоздатьЭлемент".to_string(),
        Some("СправочникМенеджер.Номенклатура".to_string()),
        vec![],
        Some("СправочникОбъект.Номенклатура".to_string()),
        SignatureSource::Platform,
        Some(FacetKind::Object),
        ContextRequirements::ServerOnly,
    );

    index.add_platform_method("СправочникМенеджер.Номенклатура".to_string(), sig);

    let found = index.find_method("СправочникМенеджер.Номенклатура", "СоздатьЭлемент");
    assert!(found.is_some());

    let method = found.unwrap();
    assert_eq!(method.name, "СоздатьЭлемент");
    assert_eq!(method.return_facet, Some(FacetKind::Object));
    assert_eq!(method.context_requirements, ContextRequirements::ServerOnly);
}

#[test]
fn test_signature_index_multiple_facet_methods() {
    let mut index = SignatureIndex::new();

    // Добавляем метод Manager facet
    index.add_platform_method(
        "СправочникМенеджер".to_string(),
        MethodSignature::new(
            "СоздатьЭлемент".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникОбъект".to_string()),
            SignatureSource::Platform,
            Some(FacetKind::Object),
            ContextRequirements::ServerOnly,
        ),
    );

    // Добавляем метод Object facet
    index.add_platform_method(
        "СправочникОбъект".to_string(),
        MethodSignature::new(
            "Записать".to_string(),
            Some("СправочникОбъект".to_string()),
            vec![],
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::ServerOnly,
        ),
    );

    // Проверяем оба метода
    assert!(index
        .find_method("СправочникМенеджер", "СоздатьЭлемент")
        .is_some());
    assert!(index
        .find_method("СправочникОбъект", "Записать")
        .is_some());
}

#[test]
fn test_signature_index_facet_method_case_insensitive() {
    let mut index = SignatureIndex::new();

    index.add_platform_method(
        "СправочникМенеджер".to_string(),
        MethodSignature::new(
            "СоздатьЭлемент".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникОбъект".to_string()),
            SignatureSource::Platform,
            Some(FacetKind::Object),
            ContextRequirements::ServerOnly,
        ),
    );

    // Разный регистр должен работать
    assert!(index
        .find_method("СправочникМенеджер", "создатьэлемент")
        .is_some());
    assert!(index
        .find_method("СправочникМенеджер", "СОЗДАТЬЭЛЕМЕНТ")
        .is_some());
}

#[test]
fn test_signature_index_backward_compatibility() {
    let mut index = SignatureIndex::new();

    // Новый способ через MethodSignature::new()
    let sig = MethodSignature::new(
        "Добавить".to_string(),
        Some("Массив".to_string()),
        vec![],
        None,
        SignatureSource::Platform,
        None,
        ContextRequirements::default(),
    );

    index.add_platform_method("Массив".to_string(), sig);

    let found = index.find_method("Массив", "Добавить");
    assert!(found.is_some());

    let method = found.unwrap();
    assert_eq!(method.return_facet, None);
    assert_eq!(method.context_requirements, ContextRequirements::Universal);
}

// ============================================================================
// ИНТЕГРАЦИОННЫЕ ТЕСТЫ (требуют backend)
// ============================================================================

#[cfg(feature = "integration_tests")]
mod integration {
    use super::*;

    #[test]
    fn test_catalog_manager_methods_in_index() {
        use bsl_backend::data::loaders::platform_types::{
            load_all_platform_types, populate_signature_index_from_platform_types,
        };

        let platform_types = load_all_platform_types();
        let mut index = SignatureIndex::new();

        populate_signature_index_from_platform_types(&platform_types, &mut index);

        // Проверяем методы СправочникМенеджер
        let create = index.find_method("СправочникМенеджер", "СоздатьЭлемент");
        assert!(create.is_some());
        assert_eq!(create.unwrap().return_facet, Some(FacetKind::Object));

        let find_by_code = index.find_method("СправочникМенеджер", "НайтиПоКоду");
        assert!(find_by_code.is_some());
        assert_eq!(
            find_by_code.unwrap().return_facet,
            Some(FacetKind::Reference)
        );

        let select = index.find_method("СправочникМенеджер", "Выбрать");
        assert!(select.is_some());
        assert_eq!(select.unwrap().return_facet, Some(FacetKind::Selection));
    }

    #[test]
    fn test_catalog_object_methods_in_index() {
        use bsl_backend::data::loaders::platform_types::{
            load_all_platform_types, populate_signature_index_from_platform_types,
        };

        let platform_types = load_all_platform_types();
        let mut index = SignatureIndex::new();

        populate_signature_index_from_platform_types(&platform_types, &mut index);

        // Проверяем методы СправочникОбъект
        let write = index.find_method("СправочникОбъект", "Записать");
        assert!(write.is_some());
        assert_eq!(
            write.unwrap().context_requirements,
            ContextRequirements::ServerOnly
        );

        let copy = index.find_method("СправочникОбъект", "Скопировать");
        assert!(copy.is_some());
        assert_eq!(copy.unwrap().return_facet, Some(FacetKind::Object));
    }

    #[test]
    fn test_catalog_reference_methods_in_index() {
        use bsl_backend::data::loaders::platform_types::{
            load_all_platform_types, populate_signature_index_from_platform_types,
        };

        let platform_types = load_all_platform_types();
        let mut index = SignatureIndex::new();

        populate_signature_index_from_platform_types(&platform_types, &mut index);

        // Проверяем методы СправочникСсылка
        let get_object = index.find_method("СправочникСсылка", "ПолучитьОбъект");
        assert!(get_object.is_some());
        assert_eq!(get_object.unwrap().return_facet, Some(FacetKind::Object));

        let is_empty = index.find_method("СправочникСсылка", "Пустая");
        assert!(is_empty.is_some());
        assert_eq!(
            is_empty.unwrap().context_requirements,
            ContextRequirements::Universal
        );
    }

    #[test]
    fn test_document_methods_in_index() {
        use bsl_backend::data::loaders::platform_types::{
            load_all_platform_types, populate_signature_index_from_platform_types,
        };

        let platform_types = load_all_platform_types();
        let mut index = SignatureIndex::new();

        populate_signature_index_from_platform_types(&platform_types, &mut index);

        // Проверяем методы ДокументМенеджер
        let create_doc = index.find_method("ДокументМенеджер", "СоздатьДокумент");
        assert!(create_doc.is_some());
        assert_eq!(create_doc.unwrap().return_facet, Some(FacetKind::Object));

        // Проверяем методы ДокументОбъект
        let post = index.find_method("ДокументОбъект", "Провести");
        assert!(post.is_some());
        assert_eq!(
            post.unwrap().context_requirements,
            ContextRequirements::ServerOnly
        );
    }
}
