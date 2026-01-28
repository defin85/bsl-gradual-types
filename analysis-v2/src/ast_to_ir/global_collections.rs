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

/// Поиск глобальной коллекции по имени.
///
/// Сравнение case-insensitive для латиницы (eq_ignore_ascii_case).
/// Для кириллицы используется точное сравнение, т.к. 1С всегда использует корректный регистр.
pub fn lookup_global_collection(name: &str) -> Option<&'static GlobalCollectionInfo> {
    GLOBAL_COLLECTIONS_INFO
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
mod tests {
    use super::*;

    #[test]
    fn test_is_global_collection() {
        // Проверяем русские имена (точное совпадение)
        assert_eq!(
            is_global_collection("Справочники"),
            Some("СправочникиМенеджер")
        );
        assert_eq!(is_global_collection("Документы"), Some("ДокументыМенеджер"));
        assert_eq!(
            is_global_collection("РегистрыСведений"),
            Some("РегистрСведенийМенеджерКоллекция")
        );
        assert_eq!(
            is_global_collection("РегистрыНакопления"),
            Some("РегистрНакопленияМенеджерКоллекция")
        );
        assert_eq!(
            is_global_collection("РегистрыБухгалтерии"),
            Some("РегистрБухгалтерииМенеджерКоллекция")
        );
        assert_eq!(
            is_global_collection("РегистрыРасчета"),
            Some("РегистрРасчетаМенеджерКоллекция")
        );
        assert_eq!(
            is_global_collection("Перечисления"),
            Some("ПеречисленияМенеджер")
        );
        assert_eq!(is_global_collection("Константы"), Some("КонстантыМенеджер"));

        // Проверяем английские имена
        assert_eq!(
            is_global_collection("Catalogs"),
            Some("СправочникиМенеджер")
        );
        assert_eq!(is_global_collection("Documents"), Some("ДокументыМенеджер"));
        assert_eq!(
            is_global_collection("InformationRegisters"),
            Some("РегистрСведенийМенеджерКоллекция")
        );
        assert_eq!(
            is_global_collection("AccumulationRegisters"),
            Some("РегистрНакопленияМенеджерКоллекция")
        );
        assert_eq!(
            is_global_collection("AccountingRegisters"),
            Some("РегистрБухгалтерииМенеджерКоллекция")
        );
        assert_eq!(
            is_global_collection("CalculationRegisters"),
            Some("РегистрРасчетаМенеджерКоллекция")
        );

        // Проверяем case-insensitive для латиницы
        assert_eq!(
            is_global_collection("catalogs"),
            Some("СправочникиМенеджер")
        );
        assert_eq!(is_global_collection("DOCUMENTS"), Some("ДокументыМенеджер"));
        assert_eq!(
            is_global_collection("accountingregisters"),
            Some("РегистрБухгалтерииМенеджерКоллекция")
        );
        assert_eq!(
            is_global_collection("CALCULATIONREGISTERS"),
            Some("РегистрРасчетаМенеджерКоллекция")
        );

        // Кириллица с другим регистром НЕ совпадает (1С всегда использует корректный регистр)
        assert_eq!(is_global_collection("СПРАВОЧНИКИ"), None);

        // Не глобальные коллекции
        assert_eq!(is_global_collection("МояПеременная"), None);
        assert_eq!(is_global_collection("Массив"), None);
    }

    #[test]
    fn test_get_manager_type_for_metadata() {
        // Справочники и Документы
        assert_eq!(
            get_manager_type_for_metadata("Справочники", "Контрагенты"),
            "СправочникМенеджер.Контрагенты"
        );
        assert_eq!(
            get_manager_type_for_metadata("Документы", "ЗаказКлиента"),
            "ДокументМенеджер.ЗаказКлиента"
        );
        assert_eq!(
            get_manager_type_for_metadata("Catalogs", "Partners"),
            "СправочникМенеджер.Partners"
        );

        // Регистры сведений и накопления
        assert_eq!(
            get_manager_type_for_metadata("РегистрыСведений", "ЦеныНоменклатуры"),
            "РегистрСведенийМенеджер.ЦеныНоменклатуры"
        );
        assert_eq!(
            get_manager_type_for_metadata("РегистрыНакопления", "ОстаткиТоваров"),
            "РегистрНакопленияМенеджер.ОстаткиТоваров"
        );

        // Регистры бухгалтерии и расчета
        assert_eq!(
            get_manager_type_for_metadata("РегистрыБухгалтерии", "Хозрасчетный"),
            "РегистрБухгалтерииМенеджер.Хозрасчетный"
        );
        assert_eq!(
            get_manager_type_for_metadata("РегистрыРасчета", "ОсновныеНачисления"),
            "РегистрРасчетаМенеджер.ОсновныеНачисления"
        );

        // Планы
        assert_eq!(
            get_manager_type_for_metadata("ПланыВидовХарактеристик", "ДополнительныеРеквизиты"),
            "ПланВидовХарактеристикМенеджер.ДополнительныеРеквизиты"
        );
        assert_eq!(
            get_manager_type_for_metadata("ПланыСчетов", "Хозрасчетный"),
            "ПланСчетовМенеджер.Хозрасчетный"
        );

        // Неизвестная коллекция
        assert_eq!(
            get_manager_type_for_metadata("НеизвестнаяКоллекция", "Объект"),
            "Неопределено.Объект"
        );
    }

    #[test]
    fn test_lookup_global_collection() {
        // Проверяем lookup функцию возвращает полную информацию
        let info = lookup_global_collection("Справочники").unwrap();
        assert_eq!(info.name_ru, "Справочники");
        assert_eq!(info.name_en, "Catalogs");
        assert_eq!(info.collection_manager_type, "СправочникиМенеджер");
        assert_eq!(info.item_manager_type, "СправочникМенеджер");

        let info = lookup_global_collection("AccountingRegisters").unwrap();
        assert_eq!(info.name_ru, "РегистрыБухгалтерии");
        assert_eq!(info.name_en, "AccountingRegisters");
        assert_eq!(
            info.collection_manager_type,
            "РегистрБухгалтерииМенеджерКоллекция"
        );
        assert_eq!(info.item_manager_type, "РегистрБухгалтерииМенеджер");

        let info = lookup_global_collection("CalculationRegisters").unwrap();
        assert_eq!(info.name_ru, "РегистрыРасчета");
        assert_eq!(info.name_en, "CalculationRegisters");
        assert_eq!(
            info.collection_manager_type,
            "РегистрРасчетаМенеджерКоллекция"
        );
        assert_eq!(info.item_manager_type, "РегистрРасчетаМенеджер");

        // Case-insensitive для английских имён
        let info = lookup_global_collection("accountingregisters").unwrap();
        assert_eq!(info.name_ru, "РегистрыБухгалтерии");

        // Несуществующая коллекция
        assert!(lookup_global_collection("НесуществующаяКоллекция").is_none());
    }
}
