//! Helper functions for metadata collection detection
//!
//! MILESTONE 3.16: Helper functions for detecting metadata collections

use bsl_shared::domain::types::MetadataKind;

/// Mapping of metadata collection names to MetadataKind
/// (russian name, english name, MetadataKind)
///
/// Single source of truth for converting collection names to MetadataKind.
/// Used in is_metadata_collection_name() and collection_name_to_metadata_kind().
pub(crate) static METADATA_COLLECTIONS: &[(&str, &str, MetadataKind)] = &[
    ("Справочники", "Catalogs", MetadataKind::Catalog),
    ("Документы", "Documents", MetadataKind::Document),
    ("Перечисления", "Enums", MetadataKind::Enum),
    ("РегистрыСведений", "InformationRegisters", MetadataKind::InformationRegister),
    ("РегистрыНакопления", "AccumulationRegisters", MetadataKind::AccumulationRegister),
    ("РегистрыБухгалтерии", "AccountingRegisters", MetadataKind::AccountingRegister),
    ("РегистрыРасчета", "CalculationRegisters", MetadataKind::CalculationRegister),
    ("Отчеты", "Reports", MetadataKind::Report),
    ("Обработки", "DataProcessors", MetadataKind::DataProcessor),
    ("ПланыСчетов", "ChartsOfAccounts", MetadataKind::ChartOfAccounts),
    ("ПланыВидовХарактеристик", "ChartsOfCharacteristicTypes", MetadataKind::ChartOfCharacteristicTypes),
    ("ПланыВидовРасчета", "ChartsOfCalculationTypes", MetadataKind::ChartOfCalculationTypes),
    ("БизнесПроцессы", "BusinessProcesses", MetadataKind::BusinessProcess),
    ("Задачи", "Tasks", MetadataKind::Task),
];

/// Checks if the name is a metadata collection
///
/// # Examples
/// - "Справочники" / "Catalogs" -> true
/// - "Документы" / "Documents" -> true
/// - "Массив" -> false
pub fn is_metadata_collection_name(name: &str) -> bool {
    METADATA_COLLECTIONS.iter().any(|(ru, en, _)| *ru == name || *en == name)
}

/// Converts metadata collection name to MetadataKind
///
/// # Examples
/// - "Справочники" -> Some(MetadataKind::Catalog)
/// - "Документы" -> Some(MetadataKind::Document)
/// - "Массив" -> None
pub fn collection_name_to_metadata_kind(name: &str) -> Option<MetadataKind> {
    METADATA_COLLECTIONS
        .iter()
        .find(|(ru, en, _)| *ru == name || *en == name)
        .map(|(_, _, kind)| *kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_metadata_collection_name() {
        // Russian names
        assert!(is_metadata_collection_name("Справочники"));
        assert!(is_metadata_collection_name("Документы"));
        assert!(is_metadata_collection_name("РегистрыСведений"));
        assert!(is_metadata_collection_name("Перечисления"));

        // English names
        assert!(is_metadata_collection_name("Catalogs"));
        assert!(is_metadata_collection_name("Documents"));
        assert!(is_metadata_collection_name("InformationRegisters"));
        assert!(is_metadata_collection_name("Enums"));

        // Not metadata collections
        assert!(!is_metadata_collection_name("Массив"));
        assert!(!is_metadata_collection_name("ТаблицаЗначений"));
        assert!(!is_metadata_collection_name("Строка"));
    }

    #[test]
    fn test_collection_name_to_metadata_kind() {
        // Russian names
        assert_eq!(collection_name_to_metadata_kind("Справочники"), Some(MetadataKind::Catalog));
        assert_eq!(collection_name_to_metadata_kind("Документы"), Some(MetadataKind::Document));
        assert_eq!(collection_name_to_metadata_kind("РегистрыСведений"), Some(MetadataKind::InformationRegister));
        assert_eq!(collection_name_to_metadata_kind("РегистрыНакопления"), Some(MetadataKind::AccumulationRegister));

        // English names
        assert_eq!(collection_name_to_metadata_kind("Catalogs"), Some(MetadataKind::Catalog));
        assert_eq!(collection_name_to_metadata_kind("Documents"), Some(MetadataKind::Document));

        // Unknown
        assert_eq!(collection_name_to_metadata_kind("Массив"), None);
        assert_eq!(collection_name_to_metadata_kind("Unknown"), None);
    }
}
