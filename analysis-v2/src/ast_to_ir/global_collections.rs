//! Глобальные коллекции метаданных платформы 1С
//!
//! Модуль содержит информацию о глобальных коллекциях метаданных
//! (Справочники, Документы, Регистры и т.д.) и функции для работы с ними.
//!
//! Global collections mapping used by the AST -> IR converter.

/// Информация о глобальной коллекции метаданных платформы 1С
#[derive(Debug, Clone, Copy)]
pub struct GlobalCollectionInfo {
    /// Русское имя коллекции (Справочники)
    pub name_ru: &'static str,
    /// Английское имя коллекции (Catalogs)
    pub name_en: &'static str,
    /// Тип менеджера коллекции (СправочникиМенеджер)
    pub collection_manager_type: &'static str,
    /// Базовый тип менеджера элемента (СправочникМенеджер)
    pub item_manager_type: &'static str,
}

/// Информация о коллекции, доступной через глобальный объект `Метаданные`.
#[derive(Debug, Clone, Copy)]
pub struct LegacyMetadataObjectCollectionFallbackInfo {
    /// Русское имя свойства коллекции.
    pub name_ru: &'static str,
    /// Английское имя свойства коллекции.
    pub name_en: &'static str,
    /// Тип элемента коллекции из синтаксис-помощника.
    pub item_type_name: &'static str,
}

/// Полный список глобальных коллекций метаданных платформы 1С
///
/// Включает все типы объектов метаданных:
/// - Справочники, Документы, Перечисления, Константы
/// - Регистры: сведений, накопления, бухгалтерии, расчета
/// - Планы: обмена, видов характеристик, счетов, видов расчета
/// - Бизнес-процессы, Задачи
pub const GLOBAL_COLLECTIONS_INFO: &[GlobalCollectionInfo] = &[
    GlobalCollectionInfo {
        name_ru: "Справочники",
        name_en: "Catalogs",
        collection_manager_type: "СправочникиМенеджер",
        item_manager_type: "СправочникМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "Документы",
        name_en: "Documents",
        collection_manager_type: "ДокументыМенеджер",
        item_manager_type: "ДокументМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "РегистрыСведений",
        name_en: "InformationRegisters",
        collection_manager_type: "РегистрСведенийМенеджерКоллекция",
        item_manager_type: "РегистрСведенийМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "РегистрыНакопления",
        name_en: "AccumulationRegisters",
        collection_manager_type: "РегистрНакопленияМенеджерКоллекция",
        item_manager_type: "РегистрНакопленияМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "РегистрыБухгалтерии",
        name_en: "AccountingRegisters",
        collection_manager_type: "РегистрБухгалтерииМенеджерКоллекция",
        item_manager_type: "РегистрБухгалтерииМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "РегистрыРасчета",
        name_en: "CalculationRegisters",
        collection_manager_type: "РегистрРасчетаМенеджерКоллекция",
        item_manager_type: "РегистрРасчетаМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "Перечисления",
        name_en: "Enums",
        collection_manager_type: "ПеречисленияМенеджер",
        item_manager_type: "ПеречислениеМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "Константы",
        name_en: "Constants",
        collection_manager_type: "КонстантыМенеджер",
        item_manager_type: "КонстантаМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "ПланыОбмена",
        name_en: "ExchangePlans",
        collection_manager_type: "ПланОбменаМенеджерКоллекция",
        item_manager_type: "ПланОбменаМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "ПланыВидовХарактеристик",
        name_en: "ChartsOfCharacteristicTypes",
        collection_manager_type: "ПланВидовХарактеристикМенеджерКоллекция",
        item_manager_type: "ПланВидовХарактеристикМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "ПланыСчетов",
        name_en: "ChartsOfAccounts",
        collection_manager_type: "ПланСчетовМенеджерКоллекция",
        item_manager_type: "ПланСчетовМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "ПланыВидовРасчета",
        name_en: "ChartsOfCalculationTypes",
        collection_manager_type: "ПланВидовРасчетаМенеджерКоллекция",
        item_manager_type: "ПланВидовРасчетаМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "БизнесПроцессы",
        name_en: "BusinessProcesses",
        collection_manager_type: "БизнесПроцессМенеджерКоллекция",
        item_manager_type: "БизнесПроцессМенеджер",
    },
    GlobalCollectionInfo {
        name_ru: "Задачи",
        name_en: "Tasks",
        collection_manager_type: "ЗадачаМенеджерКоллекция",
        item_manager_type: "ЗадачаМенеджер",
    },
];

/// Legacy fallback for metadata object collections available through
/// `Метаданные.<ИмяКоллекции>`.
///
/// Loaded Syntax Helper repository properties must be used before this table.
/// Keep this list as a degraded/bootstrap fallback only.
pub const LEGACY_METADATA_OBJECT_COLLECTION_FALLBACKS:
    &[LegacyMetadataObjectCollectionFallbackInfo] = &[
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "Справочники",
        name_en: "Catalogs",
        item_type_name: "ОбъектМетаданных: Справочник",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "Документы",
        name_en: "Documents",
        item_type_name: "ОбъектМетаданных: Документ",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "РегистрыСведений",
        name_en: "InformationRegisters",
        item_type_name: "ОбъектМетаданных: РегистрСведений",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "РегистрыНакопления",
        name_en: "AccumulationRegisters",
        item_type_name: "ОбъектМетаданных: РегистрНакопления",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "РегистрыБухгалтерии",
        name_en: "AccountingRegisters",
        item_type_name: "ОбъектМетаданных: РегистрБухгалтерии",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "РегистрыРасчета",
        name_en: "CalculationRegisters",
        item_type_name: "ОбъектМетаданных: РегистрРасчета",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "Перечисления",
        name_en: "Enums",
        item_type_name: "ОбъектМетаданных: Перечисление",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "Константы",
        name_en: "Constants",
        item_type_name: "ОбъектМетаданных: Константа",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "ПланыОбмена",
        name_en: "ExchangePlans",
        item_type_name: "ОбъектМетаданных: ПланОбмена",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "ПланыВидовХарактеристик",
        name_en: "ChartsOfCharacteristicTypes",
        item_type_name: "ОбъектМетаданных: ПланВидовХарактеристик",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "ПланыСчетов",
        name_en: "ChartsOfAccounts",
        item_type_name: "ОбъектМетаданных: ПланСчетов",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "ПланыВидовРасчета",
        name_en: "ChartsOfCalculationTypes",
        item_type_name: "ОбъектМетаданных: ПланВидовРасчета",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "БизнесПроцессы",
        name_en: "BusinessProcesses",
        item_type_name: "ОбъектМетаданных: БизнесПроцесс",
    },
    LegacyMetadataObjectCollectionFallbackInfo {
        name_ru: "Задачи",
        name_en: "Tasks",
        item_type_name: "ОбъектМетаданных: Задача",
    },
];

/// Поиск глобальной коллекции по имени.
///
/// Сравнение case-insensitive для латиницы (eq_ignore_ascii_case).
/// Для кириллицы используется точное сравнение, т.к. 1С всегда использует корректный регистр.
pub fn lookup_global_collection(name: &str) -> Option<&'static GlobalCollectionInfo> {
    GLOBAL_COLLECTIONS_INFO
        .iter()
        .find(|info| info.name_ru == name || info.name_en.eq_ignore_ascii_case(name))
}

/// Поиск глобальной коллекции по типу менеджера коллекции из Syntax Helper.
///
/// Например, глобальное свойство `Документы` в Syntax Helper имеет тип
/// `ДокументыМенеджер`, но дальнейший доступ `Документы.Заказ` должен
/// обрабатываться как доступ к коллекции `Документы`.
pub fn lookup_global_collection_by_manager_type(
    type_name: &str,
) -> Option<&'static GlobalCollectionInfo> {
    GLOBAL_COLLECTIONS_INFO
        .iter()
        .find(|info| info.collection_manager_type.eq_ignore_ascii_case(type_name))
}

/// Legacy fallback lookup for a metadata object collection property.
///
/// Callers must try repository/Syntax Helper property data before this fallback.
pub fn lookup_legacy_metadata_object_collection_fallback(
    name: &str,
) -> Option<&'static LegacyMetadataObjectCollectionFallbackInfo> {
    LEGACY_METADATA_OBJECT_COLLECTION_FALLBACKS
        .iter()
        .find(|info| info.name_ru == name || info.name_en.eq_ignore_ascii_case(name))
}

/// Проверяет, является ли имя глобальной коллекцией метаданных.
/// Возвращает тип менеджера коллекции, если это глобальная коллекция.
pub fn is_global_collection(name: &str) -> Option<&'static str> {
    lookup_global_collection(name).map(|info| info.collection_manager_type)
}

/// Возвращает тип менеджера для конкретного объекта метаданных.
/// Например: "Справочники" + "Контрагенты" -> "СправочникМенеджер.Контрагенты"
pub fn get_manager_type_for_metadata(collection_name: &str, object_name: &str) -> String {
    let base_manager = lookup_global_collection(collection_name)
        .map(|info| info.item_manager_type)
        .unwrap_or("Неопределено");
    format!("{}.{}", base_manager, object_name)
}

#[cfg(test)]
#[path = "global_collections/tests.rs"]
mod tests;
