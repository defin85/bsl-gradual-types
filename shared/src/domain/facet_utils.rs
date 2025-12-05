//! Facet Utilities - централизованная логика работы с фасетными типами
//!
//! Этот модуль консолидирует логику извлечения и проверки фасетных типов,
//! которая ранее была дублирована в metadata_lookup.rs и signature_index.rs.
//!
//! ## Фасетная система типов
//!
//! В 1С один объект метаданных (например, Справочник.Контрагенты) имеет
//! несколько представлений (фасетов):
//! - **Manager** (СправочникМенеджер) - управление объектами
//! - **Object** (СправочникОбъект) - редактируемый экземпляр
//! - **Reference** (СправочникСсылка) - ссылка на объект
//! - **Selection** (СправочникВыборка) - выборка объектов
//! - **List** (СправочникСписок) - динамический список
//!
//! ## Примеры
//!
//! ```rust
//! use bsl_shared::domain::facet_utils;
//!
//! // Извлечение базового типа из конкретизированного
//! assert_eq!(
//!     facet_utils::extract_base_facet_type("СправочникМенеджер.Контрагенты"),
//!     Some("СправочникМенеджер")
//! );
//!
//! // Проверка известного фасетного префикса
//! assert!(facet_utils::is_known_facet_prefix("ДокументОбъект"));
//! assert!(!facet_utils::is_known_facet_prefix("Массив"));
//! ```

/// Извлечь базовый фасетный тип из полного имени типа
///
/// # Параметры
/// * `type_name` - полное имя типа, например "СправочникМенеджер.Контрагенты"
///
/// # Возвращает
/// * `Some(&str)` - базовый тип (например "СправочникМенеджер")
/// * `None` - если тип не является фасетным или уже базовый
///
/// # Примеры
/// ```rust
/// use bsl_shared::domain::facet_utils::extract_base_facet_type;
///
/// // Конкретизированные фасетные типы
/// assert_eq!(extract_base_facet_type("СправочникМенеджер.Контрагенты"), Some("СправочникМенеджер"));
/// assert_eq!(extract_base_facet_type("ДокументОбъект.ЗаказКлиента"), Some("ДокументОбъект"));
/// assert_eq!(extract_base_facet_type("РегистрСведенийНаборЗаписей.КурсыВалют"), Some("РегистрСведенийНаборЗаписей"));
///
/// // Не-фасетные типы
/// assert_eq!(extract_base_facet_type("Массив"), None);
/// assert_eq!(extract_base_facet_type("СправочникМенеджер"), None); // уже базовый
/// ```
pub fn extract_base_facet_type(type_name: &str) -> Option<&str> {
    // Проверяем наличие точки (признак конкретизированного типа)
    let dot_pos = type_name.find('.')?;

    let prefix = &type_name[..dot_pos];

    // Проверяем что префикс - известный фасетный тип
    if is_known_facet_prefix(prefix) {
        Some(prefix)
    } else {
        None
    }
}

/// Проверить является ли строка известным фасетным префиксом
///
/// Фасетные префиксы - это базовые типы платформы 1С, которые
/// конкретизируются именем объекта метаданных через точку.
///
/// # Примеры
/// ```rust
/// use bsl_shared::domain::facet_utils::is_known_facet_prefix;
///
/// assert!(is_known_facet_prefix("СправочникМенеджер"));
/// assert!(is_known_facet_prefix("ДокументОбъект"));
/// assert!(is_known_facet_prefix("РегистрСведенийНаборЗаписей"));
///
/// assert!(!is_known_facet_prefix("Массив"));
/// assert!(!is_known_facet_prefix("ТаблицаЗначений"));
/// ```
pub fn is_known_facet_prefix(prefix: &str) -> bool {
    matches!(
        prefix,
        // Справочники
        "СправочникМенеджер" | "СправочникОбъект" | "СправочникСсылка" |
        "СправочникВыборка" | "СправочникСписок" |
        // Документы
        "ДокументМенеджер" | "ДокументОбъект" | "ДокументСсылка" |
        "ДокументВыборка" | "ДокументСписок" |
        // Планы видов характеристик
        "ПланВидовХарактеристикМенеджер" | "ПланВидовХарактеристикОбъект" |
        "ПланВидовХарактеристикСсылка" | "ПланВидовХарактеристикВыборка" |
        // Планы счетов
        "ПланСчетовМенеджер" | "ПланСчетовОбъект" | "ПланСчетовСсылка" |
        "ПланСчетовВыборка" |
        // Планы видов расчета
        "ПланВидовРасчетаМенеджер" | "ПланВидовРасчетаОбъект" |
        "ПланВидовРасчетаСсылка" | "ПланВидовРасчетаВыборка" |
        // Планы обмена
        "ПланОбменаМенеджер" | "ПланОбменаОбъект" | "ПланОбменаСсылка" |
        "ПланОбменаВыборка" |
        // Бизнес-процессы
        "БизнесПроцессМенеджер" | "БизнесПроцессОбъект" | "БизнесПроцессСсылка" |
        "БизнесПроцессВыборка" | "БизнесПроцессТочкаМаршрутаБизнесПроцесса" |
        // Задачи
        "ЗадачаМенеджер" | "ЗадачаОбъект" | "ЗадачаСсылка" | "ЗадачаВыборка" |
        // Перечисления
        "ПеречислениеМенеджер" | "ПеречислениеСсылка" |
        // Регистры сведений
        "РегистрСведенийМенеджер" | "РегистрСведенийНаборЗаписей" |
        "РегистрСведенийВыборка" | "РегистрСведенийЗапись" |
        "РегистрСведенийМенеджерЗаписи" |
        // Регистры накопления
        "РегистрНакопленияМенеджер" | "РегистрНакопленияНаборЗаписей" |
        "РегистрНакопленияВыборка" | "РегистрНакопленияЗапись" |
        // Регистры бухгалтерии
        "РегистрБухгалтерииМенеджер" | "РегистрБухгалтерииНаборЗаписей" |
        "РегистрБухгалтерииВыборка" | "РегистрБухгалтерииЗапись" |
        // Регистры расчета
        "РегистрРасчетаМенеджер" | "РегистрРасчетаНаборЗаписей" |
        "РегистрРасчетаВыборка" | "РегистрРасчетаЗапись"
    )
}

/// Извлечь базовый фасетный тип из имени типа с placeholder (template параметром)
///
/// Эта функция отличается от `extract_base_facet_type` тем, что работает с
/// **placeholder форматом** типов из Syntax Helper, а не с конкретизированными типами.
///
/// # Поддерживаемые форматы
/// * Стандартный: `"СправочникМенеджер.<Имя справочника>"`
/// * HTML-encoded: `"СправочникМенеджер.&lt;Имя справочника&gt;"`
/// * Английский: `"CatalogManager.<Catalog name>"`
///
/// # Параметры
/// * `type_name` - имя типа с placeholder, например `"СправочникМенеджер.<Имя справочника>"`
///
/// # Возвращает
/// * `Some(&str)` - базовый тип (например `"СправочникМенеджер"`)
/// * `None` - если тип не содержит placeholder
///
/// # Примеры
/// ```rust
/// use bsl_shared::domain::facet_utils::extract_placeholder_base_type;
///
/// // Стандартный placeholder формат
/// assert_eq!(
///     extract_placeholder_base_type("СправочникМенеджер.<Имя справочника>"),
///     Some("СправочникМенеджер")
/// );
///
/// // HTML-encoded формат
/// assert_eq!(
///     extract_placeholder_base_type("ДокументОбъект.&lt;Имя документа&gt;"),
///     Some("ДокументОбъект")
/// );
///
/// // Не-placeholder типы
/// assert_eq!(extract_placeholder_base_type("Массив"), None);
/// assert_eq!(extract_placeholder_base_type("СправочникМенеджер.Контрагенты"), None);
/// ```
///
/// # Отличие от `extract_base_facet_type`
///
/// | Функция | Формат входа | Пример |
/// |---------|--------------|--------|
/// | `extract_base_facet_type` | Конкретизированный | `"СправочникМенеджер.Контрагенты"` |
/// | `extract_placeholder_base_type` | С placeholder | `"СправочникМенеджер.<Имя справочника>"` |
pub fn extract_placeholder_base_type(type_name: &str) -> Option<&str> {
    // Стандартный формат: .<Имя
    if let Some(pos) = type_name.find(".<Имя") {
        return Some(&type_name[..pos]);
    }

    // HTML-encoded формат: .&lt;Имя
    if let Some(pos) = type_name.find(".&lt;Имя") {
        return Some(&type_name[..pos]);
    }

    // Альтернативный HTML-encoded: .&lt; с любым содержимым
    if let Some(pos) = type_name.find(".&lt;") {
        return Some(&type_name[..pos]);
    }

    // Английский вариант: .< с " name>"
    if let Some(pos) = type_name.find(".<") {
        if type_name[pos..].contains(" name>") {
            return Some(&type_name[..pos]);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================= extract_base_facet_type tests =================

    #[test]
    fn test_extract_base_facet_type_catalog() {
        assert_eq!(
            extract_base_facet_type("СправочникМенеджер.Контрагенты"),
            Some("СправочникМенеджер")
        );
        assert_eq!(
            extract_base_facet_type("СправочникОбъект.Номенклатура"),
            Some("СправочникОбъект")
        );
        assert_eq!(
            extract_base_facet_type("СправочникСсылка.Валюты"),
            Some("СправочникСсылка")
        );
        assert_eq!(
            extract_base_facet_type("СправочникВыборка.Контрагенты"),
            Some("СправочникВыборка")
        );
        assert_eq!(
            extract_base_facet_type("СправочникСписок.Товары"),
            Some("СправочникСписок")
        );
    }

    #[test]
    fn test_extract_base_facet_type_document() {
        assert_eq!(
            extract_base_facet_type("ДокументМенеджер.ЗаказКлиента"),
            Some("ДокументМенеджер")
        );
        assert_eq!(
            extract_base_facet_type("ДокументОбъект.РасходнаяНакладная"),
            Some("ДокументОбъект")
        );
        assert_eq!(
            extract_base_facet_type("ДокументСсылка.ПриходнаяНакладная"),
            Some("ДокументСсылка")
        );
    }

    #[test]
    fn test_extract_base_facet_type_registers() {
        assert_eq!(
            extract_base_facet_type("РегистрСведенийМенеджер.КурсыВалют"),
            Some("РегистрСведенийМенеджер")
        );
        assert_eq!(
            extract_base_facet_type("РегистрСведенийНаборЗаписей.ЦеныТоваров"),
            Some("РегистрСведенийНаборЗаписей")
        );
        assert_eq!(
            extract_base_facet_type("РегистрНакопленияМенеджер.ОстаткиТоваров"),
            Some("РегистрНакопленияМенеджер")
        );
        assert_eq!(
            extract_base_facet_type("РегистрБухгалтерииЗапись.Хозрасчетный"),
            Some("РегистрБухгалтерииЗапись")
        );
    }

    #[test]
    fn test_extract_base_facet_type_exchange_plan() {
        assert_eq!(
            extract_base_facet_type("ПланОбменаМенеджер.РаспределённаяБаза"),
            Some("ПланОбменаМенеджер")
        );
        assert_eq!(
            extract_base_facet_type("ПланОбменаОбъект.РаспределённаяБаза"),
            Some("ПланОбменаОбъект")
        );
        assert_eq!(
            extract_base_facet_type("ПланОбменаСсылка.РаспределённаяБаза"),
            Some("ПланОбменаСсылка")
        );
    }

    #[test]
    fn test_extract_base_facet_type_non_faceted() {
        // Не-фасетные типы должны возвращать None
        assert_eq!(extract_base_facet_type("Массив"), None);
        assert_eq!(extract_base_facet_type("Строка"), None);
        assert_eq!(extract_base_facet_type("ТаблицаЗначений"), None);
        // Базовые фасетные типы без имени объекта тоже None
        assert_eq!(extract_base_facet_type("СправочникМенеджер"), None);
        assert_eq!(extract_base_facet_type("ДокументОбъект"), None);
    }

    #[test]
    fn test_extract_base_facet_type_non_facet_with_dot() {
        // Типы с точкой, но не фасетные
        assert_eq!(extract_base_facet_type("Файл.Существует"), None);
        assert_eq!(extract_base_facet_type("System.String"), None);
    }

    // ================= is_known_facet_prefix tests =================

    #[test]
    fn test_is_known_facet_prefix_catalogs() {
        assert!(is_known_facet_prefix("СправочникМенеджер"));
        assert!(is_known_facet_prefix("СправочникОбъект"));
        assert!(is_known_facet_prefix("СправочникСсылка"));
        assert!(is_known_facet_prefix("СправочникВыборка"));
        assert!(is_known_facet_prefix("СправочникСписок"));
    }

    #[test]
    fn test_is_known_facet_prefix_documents() {
        assert!(is_known_facet_prefix("ДокументМенеджер"));
        assert!(is_known_facet_prefix("ДокументОбъект"));
        assert!(is_known_facet_prefix("ДокументСсылка"));
        assert!(is_known_facet_prefix("ДокументВыборка"));
        assert!(is_known_facet_prefix("ДокументСписок"));
    }

    #[test]
    fn test_is_known_facet_prefix_registers() {
        assert!(is_known_facet_prefix("РегистрСведенийМенеджер"));
        assert!(is_known_facet_prefix("РегистрСведенийНаборЗаписей"));
        assert!(is_known_facet_prefix("РегистрСведенийВыборка"));
        assert!(is_known_facet_prefix("РегистрСведенийЗапись"));
        assert!(is_known_facet_prefix("РегистрСведенийМенеджерЗаписи"));
        assert!(is_known_facet_prefix("РегистрНакопленияМенеджер"));
        assert!(is_known_facet_prefix("РегистрБухгалтерииНаборЗаписей"));
        assert!(is_known_facet_prefix("РегистрРасчетаЗапись"));
    }

    #[test]
    fn test_is_known_facet_prefix_plans() {
        assert!(is_known_facet_prefix("ПланВидовХарактеристикМенеджер"));
        assert!(is_known_facet_prefix("ПланСчетовОбъект"));
        assert!(is_known_facet_prefix("ПланВидовРасчетаСсылка"));
        assert!(is_known_facet_prefix("ПланОбменаМенеджер"));
    }

    #[test]
    fn test_is_known_facet_prefix_other() {
        assert!(is_known_facet_prefix("БизнесПроцессМенеджер"));
        assert!(is_known_facet_prefix("ЗадачаОбъект"));
        assert!(is_known_facet_prefix("ПеречислениеМенеджер"));
        assert!(is_known_facet_prefix("ПеречислениеСсылка"));
    }

    #[test]
    fn test_is_known_facet_prefix_non_faceted() {
        assert!(!is_known_facet_prefix("Массив"));
        assert!(!is_known_facet_prefix("Строка"));
        assert!(!is_known_facet_prefix("ТаблицаЗначений"));
        assert!(!is_known_facet_prefix("HTTPСоединение"));
        assert!(!is_known_facet_prefix("Файл"));
    }

    // ================= extract_base_facet_type edge cases =================

    #[test]
    fn test_extract_base_facet_type_empty_string() {
        assert_eq!(extract_base_facet_type(""), None);
    }

    #[test]
    fn test_extract_base_facet_type_only_dot() {
        assert_eq!(extract_base_facet_type("."), None);
    }

    #[test]
    fn test_extract_base_facet_type_dot_at_end() {
        // Тип с точкой в конце - если префикс известен, он извлекается
        // (даже если имя объекта пустое)
        assert_eq!(
            extract_base_facet_type("СправочникМенеджер."),
            Some("СправочникМенеджер")
        );
        // Но если префикс не фасетный - None
        assert_eq!(extract_base_facet_type("Массив."), None);
    }

    #[test]
    fn test_extract_base_facet_type_multiple_dots() {
        // Только первая точка используется для извлечения базового типа
        assert_eq!(
            extract_base_facet_type("СправочникМенеджер.Контрагенты.Подчиненный"),
            Some("СправочникМенеджер")
        );
    }

    // ================= extract_placeholder_base_type tests =================

    #[test]
    fn test_extract_placeholder_base_type_standard() {
        assert_eq!(
            extract_placeholder_base_type("СправочникМенеджер.<Имя справочника>"),
            Some("СправочникМенеджер")
        );
        assert_eq!(
            extract_placeholder_base_type("ДокументОбъект.<Имя документа>"),
            Some("ДокументОбъект")
        );
        assert_eq!(
            extract_placeholder_base_type("РегистрСведенийНаборЗаписей.<Имя регистра сведений>"),
            Some("РегистрСведенийНаборЗаписей")
        );
    }

    #[test]
    fn test_extract_placeholder_base_type_html_encoded() {
        assert_eq!(
            extract_placeholder_base_type("СправочникМенеджер.&lt;Имя справочника&gt;"),
            Some("СправочникМенеджер")
        );
        assert_eq!(
            extract_placeholder_base_type("ДокументОбъект.&lt;Имя документа&gt;"),
            Some("ДокументОбъект")
        );
    }

    #[test]
    fn test_extract_placeholder_base_type_english() {
        assert_eq!(
            extract_placeholder_base_type("CatalogManager.<Catalog name>"),
            Some("CatalogManager")
        );
        assert_eq!(
            extract_placeholder_base_type("DocumentObject.<Document name>"),
            Some("DocumentObject")
        );
    }

    #[test]
    fn test_extract_placeholder_base_type_non_placeholder() {
        // Конкретизированные типы (не placeholder) - возвращают None
        assert_eq!(
            extract_placeholder_base_type("СправочникМенеджер.Контрагенты"),
            None
        );
        assert_eq!(extract_placeholder_base_type("Массив"), None);
        assert_eq!(extract_placeholder_base_type("СправочникМенеджер"), None);
    }

    #[test]
    fn test_extract_placeholder_base_type_empty_string() {
        assert_eq!(extract_placeholder_base_type(""), None);
    }

    #[test]
    fn test_extract_placeholder_base_type_only_dot() {
        assert_eq!(extract_placeholder_base_type("."), None);
    }

    #[test]
    fn test_extract_placeholder_base_type_html_encoded_generic() {
        // HTML-encoded с любым содержимым после &lt;
        assert_eq!(
            extract_placeholder_base_type("ТестТип.&lt;Произвольный текст&gt;"),
            Some("ТестТип")
        );
    }
}
