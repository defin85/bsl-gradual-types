//! R6: Интеграционные тесты для контрольных цепочек вывода/резолвинга типов.

use super::*;
use crate::domain::metadata_lookup::TypeMetadataLookup;
use crate::domain::repository::{InMemoryTypeRepository, TypeRepository};
use crate::domain::types::{Certainty, RawDataSource, RawTypeData};
use crate::ir::{Span, SymbolTable};
use std::sync::Arc;

fn create_repo_with_tabular_section_type() -> Arc<InMemoryTypeRepository> {
    let repo = InMemoryTypeRepository::new();

    // Тип в репозитории хранится с пробелом, но должен находиться и по CamelCase варианту.
    repo.load_types(vec![RawTypeData {
        name: "Табличная часть".to_string(),
        english_name: "TabularSection".to_string(),
        source: RawDataSource::Platform,
        ..Default::default()
    }])
    .unwrap();

    // Встроенные методы добавляются через SignatureIndex и должны совпадать по TypeId нормализации.
    repo.populate_signature_index(|index| index.initialize_builtin_methods());

    Arc::new(repo)
}

#[test]
fn test_context_variable_resolution_delegates_to_type_resolver_and_type_id_index() {
    let repo = create_repo_with_tabular_section_type();
    let resolver = TypeResolver::new(repo.clone());

    let mut table = SymbolTable::new();
    table.register_variable(
        table.root_scope,
        "x".to_string(),
        // В SymbolTable лежит тип как строка (CamelCase),
        // а репозиторий хранит display-вариант с пробелом.
        crate::domain::types::TypeResolution::explicit("ТабличнаяЧасть"),
        Span::stub(),
    );

    let resolved = resolver.resolve_variable_with_context("x", &table, table.root_scope);

    assert_eq!(resolved.certainty, Certainty::Known);
    assert_eq!(resolved.type_name(), "Табличная часть");
}

#[test]
fn test_metadata_lookup_methods_chain_works_with_type_id_normalization() {
    let repo = create_repo_with_tabular_section_type();
    let resolver = TypeResolver::new(repo.clone());
    let lookup = TypeMetadataLookup::new(repo);

    // Резолв через TypeResolver должен найти тип по CamelCase варианту и вернуть display с пробелом.
    let resolution = resolver.resolve_expression_sync("ТабличнаяЧасть");
    assert_eq!(resolution.type_name(), "Табличная часть");

    // Дальше TypeMetadataLookup должен получить методы через SignatureIndex,
    // несмотря на то, что SignatureIndex был заполнен ключом "ТабличнаяЧасть".
    let methods = lookup.get_methods(&resolution);
    assert!(
        methods.iter().any(|m| m.name == "Выгрузить"),
        "Ожидали метод ТабличнаяЧасть.Выгрузить из встроенных сигнатур"
    );
}
