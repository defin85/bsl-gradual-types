//! Helper functions for metadata collection detection.
//!
//! This is used by the semantic layer (AST -> IR) to map global collection names
//! (e.g. "Справочники" / "Catalogs") to `MetadataKind`.

use bsl_shared::domain::types::MetadataKind;

/// Mapping of metadata collection names to MetadataKind
/// (russian name, english name, MetadataKind)
pub(crate) static METADATA_COLLECTIONS: &[(&str, &str, MetadataKind)] = &[
    ("Справочники", "Catalogs", MetadataKind::Catalog),
    ("Документы", "Documents", MetadataKind::Document),
    ("Перечисления", "Enums", MetadataKind::Enum),
    (
        "РегистрыСведений",
        "InformationRegisters",
        MetadataKind::InformationRegister,
    ),
    (
        "РегистрыНакопления",
        "AccumulationRegisters",
        MetadataKind::AccumulationRegister,
    ),
    (
        "РегистрыБухгалтерии",
        "AccountingRegisters",
        MetadataKind::AccountingRegister,
    ),
    (
        "РегистрыРасчета",
        "CalculationRegisters",
        MetadataKind::CalculationRegister,
    ),
    ("Отчеты", "Reports", MetadataKind::Report),
    ("Обработки", "DataProcessors", MetadataKind::DataProcessor),
    (
        "ПланыСчетов",
        "ChartsOfAccounts",
        MetadataKind::ChartOfAccounts,
    ),
    (
        "ПланыВидовХарактеристик",
        "ChartsOfCharacteristicTypes",
        MetadataKind::ChartOfCharacteristicTypes,
    ),
    (
        "ПланыВидовРасчета",
        "ChartsOfCalculationTypes",
        MetadataKind::ChartOfCalculationTypes,
    ),
    (
        "БизнесПроцессы",
        "BusinessProcesses",
        MetadataKind::BusinessProcess,
    ),
    ("Задачи", "Tasks", MetadataKind::Task),
];

/// Converts metadata collection name to MetadataKind.
pub(crate) fn collection_name_to_metadata_kind(name: &str) -> Option<MetadataKind> {
    METADATA_COLLECTIONS
        .iter()
        .find(|(ru, en, _)| *ru == name || *en == name)
        .map(|(_, _, kind)| *kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_name_to_metadata_kind_handles_ru_and_en() {
        assert_eq!(
            collection_name_to_metadata_kind("Справочники"),
            Some(MetadataKind::Catalog)
        );
        assert_eq!(
            collection_name_to_metadata_kind("Catalogs"),
            Some(MetadataKind::Catalog)
        );
        assert_eq!(collection_name_to_metadata_kind("Массив"), None);
    }
}
