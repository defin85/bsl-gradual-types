//! Константы метаданных 1С (коллекции и фасетные типы)
//!
//! Централизованное хранилище констант для устранения дублирования между:
//! - `backend/src/application/ast_to_ir.rs` (is_configuration_type_pattern)
//! - `shared/src/domain/resolver.rs` (resolve_member_access)

use super::types::{FacetKind, MetadataKind};

// =============================================================================
// ГЛОБАЛЬНЫЕ КОЛЛЕКЦИИ МЕТАДАННЫХ
// =============================================================================

/// Глобальные коллекции метаданных (возвращают Manager facet)
/// Формат: (русское_имя, английское_имя, MetadataKind)
pub const METADATA_COLLECTIONS: &[(&str, &str, MetadataKind)] = &[
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
    (
        "ПланыВидовХарактеристик",
        "ChartsOfCharacteristicTypes",
        MetadataKind::ChartOfCharacteristicTypes,
    ),
    (
        "ПланыСчетов",
        "ChartsOfAccounts",
        MetadataKind::ChartOfAccounts,
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
    ("ПланыОбмена", "ExchangePlans", MetadataKind::ExchangePlan),
];

// =============================================================================
// ФАСЕТНЫЕ ТИПЫ
// =============================================================================

/// Фасетные типы (уже содержат facet в имени)
/// Формат: (русское_имя, английское_имя, MetadataKind, FacetKind)
pub const FACETED_TYPES: &[(&str, &str, MetadataKind, FacetKind)] = &[
    // Справочники
    (
        "СправочникМенеджер",
        "CatalogManager",
        MetadataKind::Catalog,
        FacetKind::Manager,
    ),
    (
        "СправочникОбъект",
        "CatalogObject",
        MetadataKind::Catalog,
        FacetKind::Object,
    ),
    (
        "СправочникСсылка",
        "CatalogRef",
        MetadataKind::Catalog,
        FacetKind::Reference,
    ),
    (
        "СправочникВыборка",
        "CatalogSelection",
        MetadataKind::Catalog,
        FacetKind::Selection,
    ),
    (
        "СправочникСписок",
        "CatalogList",
        MetadataKind::Catalog,
        FacetKind::List,
    ),
    // Документы
    (
        "ДокументМенеджер",
        "DocumentManager",
        MetadataKind::Document,
        FacetKind::Manager,
    ),
    (
        "ДокументОбъект",
        "DocumentObject",
        MetadataKind::Document,
        FacetKind::Object,
    ),
    (
        "ДокументСсылка",
        "DocumentRef",
        MetadataKind::Document,
        FacetKind::Reference,
    ),
    (
        "ДокументВыборка",
        "DocumentSelection",
        MetadataKind::Document,
        FacetKind::Selection,
    ),
    (
        "ДокументСписок",
        "DocumentList",
        MetadataKind::Document,
        FacetKind::List,
    ),
    // Регистры сведений
    (
        "РегистрСведенийМенеджер",
        "InformationRegisterManager",
        MetadataKind::InformationRegister,
        FacetKind::Manager,
    ),
    (
        "РегистрСведенийНаборЗаписей",
        "InformationRegisterRecordSet",
        MetadataKind::InformationRegister,
        FacetKind::Object,
    ),
    (
        "РегистрСведенийВыборка",
        "InformationRegisterSelection",
        MetadataKind::InformationRegister,
        FacetKind::Selection,
    ),
    // Регистры накопления
    (
        "РегистрНакопленияМенеджер",
        "AccumulationRegisterManager",
        MetadataKind::AccumulationRegister,
        FacetKind::Manager,
    ),
    (
        "РегистрНакопленияНаборЗаписей",
        "AccumulationRegisterRecordSet",
        MetadataKind::AccumulationRegister,
        FacetKind::Object,
    ),
    (
        "РегистрНакопленияВыборка",
        "AccumulationRegisterSelection",
        MetadataKind::AccumulationRegister,
        FacetKind::Selection,
    ),
    // Планы видов характеристик
    (
        "ПланВидовХарактеристикМенеджер",
        "ChartOfCharacteristicTypesManager",
        MetadataKind::ChartOfCharacteristicTypes,
        FacetKind::Manager,
    ),
    (
        "ПланВидовХарактеристикОбъект",
        "ChartOfCharacteristicTypesObject",
        MetadataKind::ChartOfCharacteristicTypes,
        FacetKind::Object,
    ),
    (
        "ПланВидовХарактеристикСсылка",
        "ChartOfCharacteristicTypesRef",
        MetadataKind::ChartOfCharacteristicTypes,
        FacetKind::Reference,
    ),
    // Планы счетов
    (
        "ПланСчетовМенеджер",
        "ChartOfAccountsManager",
        MetadataKind::ChartOfAccounts,
        FacetKind::Manager,
    ),
    (
        "ПланСчетовОбъект",
        "ChartOfAccountsObject",
        MetadataKind::ChartOfAccounts,
        FacetKind::Object,
    ),
    (
        "ПланСчетовСсылка",
        "ChartOfAccountsRef",
        MetadataKind::ChartOfAccounts,
        FacetKind::Reference,
    ),
    // Бизнес-процессы
    (
        "БизнесПроцессМенеджер",
        "BusinessProcessManager",
        MetadataKind::BusinessProcess,
        FacetKind::Manager,
    ),
    (
        "БизнесПроцессОбъект",
        "BusinessProcessObject",
        MetadataKind::BusinessProcess,
        FacetKind::Object,
    ),
    (
        "БизнесПроцессСсылка",
        "BusinessProcessRef",
        MetadataKind::BusinessProcess,
        FacetKind::Reference,
    ),
    // Задачи
    (
        "ЗадачаМенеджер",
        "TaskManager",
        MetadataKind::Task,
        FacetKind::Manager,
    ),
    (
        "ЗадачаОбъект",
        "TaskObject",
        MetadataKind::Task,
        FacetKind::Object,
    ),
    (
        "ЗадачаСсылка",
        "TaskRef",
        MetadataKind::Task,
        FacetKind::Reference,
    ),
];

// =============================================================================
// ФУНКЦИИ ПРОВЕРКИ
// =============================================================================

/// Проверить, является ли строка коллекцией метаданных
/// Возвращает Some(MetadataKind) если это коллекция
///
/// # Note
/// Для кириллицы используется точное сравнение (регистрозависимое).
/// Для латиницы используется регистронезависимое сравнение (eq_ignore_ascii_case).
pub fn get_collection_kind(name: &str) -> Option<MetadataKind> {
    METADATA_COLLECTIONS
        .iter()
        .find(|(ru, en, _)| *ru == name || en.eq_ignore_ascii_case(name))
        .map(|(_, _, kind)| *kind)
}

/// Проверить, является ли строка коллекцией метаданных
pub fn is_metadata_collection(name: &str) -> bool {
    get_collection_kind(name).is_some()
}

/// Проверить, является ли строка фасетным типом
/// Возвращает Some((MetadataKind, FacetKind)) если это фасетный тип
///
/// # Note
/// Для кириллицы используется точное сравнение (регистрозависимое).
/// Для латиницы используется регистронезависимое сравнение (eq_ignore_ascii_case).
pub fn get_faceted_type_info(name: &str) -> Option<(MetadataKind, FacetKind)> {
    FACETED_TYPES
        .iter()
        .find(|(ru, en, _, _)| *ru == name || en.eq_ignore_ascii_case(name))
        .map(|(_, _, kind, facet)| (*kind, *facet))
}

/// Проверить, является ли строка фасетным типом
pub fn is_faceted_type(name: &str) -> bool {
    get_faceted_type_info(name).is_some()
}

/// Получить MetadataKind и FacetKind для базового типа (коллекция или фасетный)
/// Коллекции всегда возвращают Manager facet
pub fn get_base_type_info(name: &str) -> Option<(MetadataKind, FacetKind)> {
    // Сначала проверяем коллекции (они возвращают Manager facet)
    if let Some(kind) = get_collection_kind(name) {
        return Some((kind, FacetKind::Manager));
    }

    // Затем проверяем фасетные типы
    get_faceted_type_info(name)
}

/// Проверить, является ли тип конфигурационным (коллекция или фасетный)
/// Используется для определения, когда нужно применять TypeResolver
pub fn is_configuration_type_pattern(type_str: &str) -> bool {
    // Проверяем паттерн "Коллекция.Имя" или "ФасетныйТип.Имя"
    if let Some(dot_pos) = type_str.find('.') {
        let prefix = &type_str[..dot_pos];
        is_metadata_collection(prefix) || is_faceted_type(prefix)
    } else {
        // Одиночное имя (без точки) - проверяем на коллекцию или фасетный тип
        is_metadata_collection(type_str) || is_faceted_type(type_str)
    }
}

// =============================================================================
// ТЕСТЫ
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_collections() {
        // Русские имена (регистрозависимые)
        assert!(is_metadata_collection("Справочники"));
        assert!(is_metadata_collection("Документы"));
        assert!(is_metadata_collection("РегистрыСведений"));

        // Английские имена
        assert!(is_metadata_collection("Catalogs"));
        assert!(is_metadata_collection("Documents"));
        assert!(is_metadata_collection("InformationRegisters"));

        // Регистронезависимость только для латиницы
        assert!(is_metadata_collection("CATALOGS"));
        assert!(is_metadata_collection("catalogs"));

        // Кириллица регистрозависима
        assert!(!is_metadata_collection("справочники")); // lowercase не работает

        // Не коллекции
        assert!(!is_metadata_collection("СправочникМенеджер"));
        assert!(!is_metadata_collection("Unknown"));
    }

    #[test]
    fn test_faceted_types() {
        // Русские имена
        assert!(is_faceted_type("СправочникМенеджер"));
        assert!(is_faceted_type("ДокументОбъект"));
        assert!(is_faceted_type("РегистрСведенийНаборЗаписей"));

        // Английские имена
        assert!(is_faceted_type("CatalogManager"));
        assert!(is_faceted_type("DocumentObject"));

        // Не фасетные типы
        assert!(!is_faceted_type("Справочники"));
        assert!(!is_faceted_type("Unknown"));
    }

    #[test]
    fn test_get_collection_kind() {
        assert_eq!(
            get_collection_kind("Справочники"),
            Some(MetadataKind::Catalog)
        );
        assert_eq!(get_collection_kind("Catalogs"), Some(MetadataKind::Catalog));
        assert_eq!(
            get_collection_kind("Документы"),
            Some(MetadataKind::Document)
        );
        assert_eq!(get_collection_kind("Unknown"), None);
    }

    #[test]
    fn test_get_faceted_type_info() {
        assert_eq!(
            get_faceted_type_info("СправочникМенеджер"),
            Some((MetadataKind::Catalog, FacetKind::Manager))
        );
        assert_eq!(
            get_faceted_type_info("ДокументОбъект"),
            Some((MetadataKind::Document, FacetKind::Object))
        );
        assert_eq!(get_faceted_type_info("Unknown"), None);
    }

    #[test]
    fn test_get_base_type_info() {
        // Коллекции возвращают Manager facet
        assert_eq!(
            get_base_type_info("Справочники"),
            Some((MetadataKind::Catalog, FacetKind::Manager))
        );

        // Фасетные типы возвращают свой facet
        assert_eq!(
            get_base_type_info("СправочникОбъект"),
            Some((MetadataKind::Catalog, FacetKind::Object))
        );
    }

    #[test]
    fn test_is_configuration_type_pattern() {
        // Паттерны с точкой
        assert!(is_configuration_type_pattern("Справочники.Контрагенты"));
        assert!(is_configuration_type_pattern("Catalogs.Counterparties"));
        assert!(is_configuration_type_pattern(
            "СправочникМенеджер.Контрагенты"
        ));

        // Без точки - сами коллекции/фасеты
        assert!(is_configuration_type_pattern("Справочники"));
        assert!(is_configuration_type_pattern("СправочникМенеджер"));

        // Не конфигурационные
        assert!(!is_configuration_type_pattern("Массив"));
        assert!(!is_configuration_type_pattern("Unknown.Something"));
    }
}
