//! Тесты для резолюции табличных частей через Generic типы

use super::*;
use crate::domain::repository::{InMemoryTypeRepository, TypeRepository};
use crate::domain::types::*;
use std::sync::Arc;

/// Вспомогательная функция для создания тестового репозитория с табличными частями
fn create_test_repository_with_tabular_sections() -> Arc<InMemoryTypeRepository> {
    let repo = InMemoryTypeRepository::new();

    // Создаём тип "Документы.ЗаказНаряды" с табличными частями "Работы" и "Материалы"
    let document_type = RawTypeData {
        name: "Документы.ЗаказНаряды".to_string(),
        english_name: "Documents.ЗаказНаряды".to_string(),
        description: "Документ Заказ-наряды".to_string(),
        category: "Document".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![],
        properties: vec![],
        facets: vec![FacetKind::Manager, FacetKind::Object],
        kind: Some(MetadataKind::Document),
        attributes: vec![],
        tabular_sections: vec![
            RawTabularSectionData {
                name: "Работы".to_string(),
                attributes: vec![
                    RawAttributeData {
                        name: "ВидРаботы".to_string(),
                        attr_type: "ПланВидовХарактеристикСсылка.ВидыРабот".to_string(),
                    },
                    RawAttributeData {
                        name: "Стоимость".to_string(),
                        attr_type: "Число".to_string(),
                    },
                ],
            },
            RawTabularSectionData {
                name: "Материалы".to_string(),
                attributes: vec![RawAttributeData {
                    name: "Номенклатура".to_string(),
                    attr_type: "СправочникСсылка.Номенклатура".to_string(),
                }],
            },
        ],
        enum_values: vec![],
        generic_info: None,
        module_paths: None,
    };

    repo.load_types(vec![document_type]).unwrap();
    Arc::new(repo)
}

#[test]
fn test_resolve_tabular_section_basic() {
    let repo = create_test_repository_with_tabular_sections();
    let resolver = TypeResolver::new(repo.clone());

    // Эмулируем: Документ.Работы
    let resolution = resolver.resolve_expression_sync("Документы.ЗаказНаряды.Работы");

    // Проверяем, что вернулся Generic тип
    assert!(matches!(resolution.result, ResolutionResult::Generic(_)));
    assert!(matches!(resolution.certainty, Certainty::Known));
    assert_eq!(resolution.active_facet, Some(FacetKind::Collection));

    // Проверяем структуру Generic типа
    if let ResolutionResult::Generic(generic) = resolution.result {
        assert_eq!(generic.base_type, "ТабличнаяЧасть");
        assert_eq!(generic.type_params.len(), 1);

        // Проверяем TabularRowType
        if let ConcreteType::TabularRow(row_type) = &generic.type_params[0] {
            assert_eq!(row_type.parent_type, "Документы.ЗаказНаряды");
            assert_eq!(row_type.tabular_section_name, "Работы");
            assert_eq!(row_type.get_full_name(), "СтрокаРаботы");
            assert_eq!(row_type.attributes.len(), 2);

            // Проверяем атрибуты
            assert_eq!(row_type.get_attribute_name(0), Some("ВидРаботы"));
            assert_eq!(
                row_type.get_attribute_type("Стоимость"),
                Some(&"Число".to_string())
            );
        } else {
            panic!("Expected TabularRow in Generic type_params");
        }
    } else {
        panic!("Expected Generic type");
    }
}

#[test]
fn test_resolve_multiple_tabular_sections() {
    let repo = create_test_repository_with_tabular_sections();
    let resolver = TypeResolver::new(repo.clone());

    // Проверяем первую табличную часть
    let resolution1 = resolver.resolve_expression_sync("Документы.ЗаказНаряды.Работы");
    if let ResolutionResult::Generic(generic) = resolution1.result {
        if let ConcreteType::TabularRow(row_type) = &generic.type_params[0] {
            assert_eq!(row_type.tabular_section_name, "Работы");
            assert_eq!(row_type.attributes.len(), 2);
        }
    }

    // Проверяем вторую табличную часть
    let resolution2 = resolver.resolve_expression_sync("Документы.ЗаказНаряды.Материалы");
    if let ResolutionResult::Generic(generic) = resolution2.result {
        if let ConcreteType::TabularRow(row_type) = &generic.type_params[0] {
            assert_eq!(row_type.tabular_section_name, "Материалы");
            assert_eq!(row_type.attributes.len(), 1);
        }
    }
}

#[test]
fn test_tabular_section_not_found() {
    let repo = create_test_repository_with_tabular_sections();
    let resolver = TypeResolver::new(repo.clone());

    // Эмулируем доступ к несуществующей табличной части
    let resolution =
        resolver.resolve_expression_sync("Документы.ЗаказНаряды.НесуществующаяТабличнаяЧасть");

    // Должен вернуться Inferred (найден тип, но не найдена табличная часть)
    // или Unknown, если считаем это несуществующим членом
    assert!(matches!(
        resolution.certainty,
        Certainty::Inferred(_) | Certainty::Unknown
    ));
}

#[test]
fn test_tabular_section_facet_is_collection() {
    let repo = create_test_repository_with_tabular_sections();
    let resolver = TypeResolver::new(repo.clone());

    let resolution = resolver.resolve_expression_sync("Документы.ЗаказНаряды.Работы");

    // Табличная часть всегда имеет фасет Collection
    assert_eq!(resolution.active_facet, Some(FacetKind::Collection));
}

#[test]
fn test_tabular_row_type_full_name() {
    let repo = create_test_repository_with_tabular_sections();
    let resolver = TypeResolver::new(repo.clone());

    let resolution = resolver.resolve_expression_sync("Документы.ЗаказНаряды.Работы");

    if let ResolutionResult::Generic(generic) = resolution.result {
        if let ConcreteType::TabularRow(row_type) = &generic.type_params[0] {
            // Проверяем формирование имени типа строки
            assert_eq!(row_type.get_full_name(), "СтрокаРаботы");
        }
    }
}

#[test]
fn test_tabular_section_certainty_is_known() {
    let repo = create_test_repository_with_tabular_sections();
    let resolver = TypeResolver::new(repo.clone());

    let resolution = resolver.resolve_expression_sync("Документы.ЗаказНаряды.Работы");

    // Табличная часть из метаданных должна иметь certainty = Known
    assert_eq!(resolution.certainty, Certainty::Known);
    assert_eq!(resolution.source, ResolutionSource::Static);
}

#[test]
fn test_tabular_section_metadata_notes() {
    let repo = create_test_repository_with_tabular_sections();
    let resolver = TypeResolver::new(repo.clone());

    let resolution = resolver.resolve_expression_sync("Документы.ЗаказНаряды.Работы");

    // Проверяем, что есть информативная заметка в metadata
    assert!(!resolution.metadata.notes.is_empty());
    let note = &resolution.metadata.notes[0];
    assert!(note.contains("Табличная часть"));
    assert!(note.contains("Работы"));
    assert!(note.contains("2")); // количество атрибутов
}
