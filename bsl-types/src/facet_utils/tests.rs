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

// ================= extract_base_facet_type_universal tests =================

#[test]
fn test_extract_base_facet_type_universal_placeholder() {
    // Placeholder формат (основной use case для исправления)
    assert_eq!(
        extract_base_facet_type_universal("СправочникМенеджер.<Имя справочника>"),
        Some("СправочникМенеджер")
    );
    assert_eq!(
        extract_base_facet_type_universal("ДокументОбъект.<Имя документа>"),
        Some("ДокументОбъект")
    );
    assert_eq!(
        extract_base_facet_type_universal("РегистрСведенийМенеджер.<Имя регистра сведений>"),
        Some("РегистрСведенийМенеджер")
    );
}

#[test]
fn test_extract_base_facet_type_universal_concrete() {
    // Конкретизированный формат (делегирует в extract_base_facet_type)
    assert_eq!(
        extract_base_facet_type_universal("СправочникМенеджер.Контрагенты"),
        Some("СправочникМенеджер")
    );
    assert_eq!(
        extract_base_facet_type_universal("ДокументОбъект.ЗаказКлиента"),
        Some("ДокументОбъект")
    );
}

#[test]
fn test_extract_base_facet_type_universal_already_base() {
    // Уже базовый тип — возвращает None
    assert_eq!(
        extract_base_facet_type_universal("СправочникМенеджер"),
        None
    );
    assert_eq!(extract_base_facet_type_universal("ДокументОбъект"), None);
}

#[test]
fn test_extract_base_facet_type_universal_non_facet() {
    // Не фасетные типы
    assert_eq!(extract_base_facet_type_universal("Массив"), None);
    assert_eq!(extract_base_facet_type_universal("ТаблицаЗначений"), None);
    assert_eq!(extract_base_facet_type_universal("Строка"), None);
}

#[test]
fn test_extract_base_facet_type_universal_empty() {
    assert_eq!(extract_base_facet_type_universal(""), None);
}

// ================= get_facet_kind_from_prefix tests =================

#[test]
fn test_get_facet_kind_from_prefix_manager() {
    assert_eq!(
        get_facet_kind_from_prefix("СправочникМенеджер"),
        Some(FacetKind::Manager)
    );
    assert_eq!(
        get_facet_kind_from_prefix("ДокументМенеджер"),
        Some(FacetKind::Manager)
    );
    assert_eq!(
        get_facet_kind_from_prefix("РегистрСведенийМенеджерЗаписи"),
        Some(FacetKind::Manager)
    );
}

#[test]
fn test_get_facet_kind_from_prefix_object() {
    assert_eq!(
        get_facet_kind_from_prefix("СправочникОбъект"),
        Some(FacetKind::Object)
    );
    assert_eq!(
        get_facet_kind_from_prefix("ДокументОбъект"),
        Some(FacetKind::Object)
    );
    assert_eq!(
        get_facet_kind_from_prefix("РегистрСведенийЗапись"),
        Some(FacetKind::Object)
    );
}

#[test]
fn test_get_facet_kind_from_prefix_reference() {
    assert_eq!(
        get_facet_kind_from_prefix("СправочникСсылка"),
        Some(FacetKind::Reference)
    );
    assert_eq!(
        get_facet_kind_from_prefix("ПеречислениеСсылка"),
        Some(FacetKind::Reference)
    );
}

#[test]
fn test_get_facet_kind_from_prefix_selection() {
    assert_eq!(
        get_facet_kind_from_prefix("СправочникВыборка"),
        Some(FacetKind::Selection)
    );
    assert_eq!(
        get_facet_kind_from_prefix("ДокументВыборка"),
        Some(FacetKind::Selection)
    );
}

#[test]
fn test_get_facet_kind_from_prefix_list() {
    assert_eq!(
        get_facet_kind_from_prefix("СправочникСписок"),
        Some(FacetKind::List)
    );
}

#[test]
fn test_get_facet_kind_from_prefix_collection() {
    assert_eq!(
        get_facet_kind_from_prefix("РегистрСведенийНаборЗаписей"),
        Some(FacetKind::Collection)
    );
    assert_eq!(
        get_facet_kind_from_prefix("РегистрНакопленияНаборЗаписей"),
        Some(FacetKind::Collection)
    );
}

#[test]
fn test_get_facet_kind_from_prefix_non_facet() {
    assert_eq!(get_facet_kind_from_prefix("Массив"), None);
    assert_eq!(get_facet_kind_from_prefix("ТаблицаЗначений"), None);
}

// ================= substitute_type_name tests =================

#[test]
fn test_substitute_type_name_base_facet() {
    assert_eq!(
        substitute_type_name("СправочникОбъект", "Контрагенты"),
        "СправочникОбъект.Контрагенты"
    );
    assert_eq!(
        substitute_type_name("ДокументСсылка", "ЗаказКлиента"),
        "ДокументСсылка.ЗаказКлиента"
    );
}

#[test]
fn test_substitute_type_name_with_placeholder() {
    assert_eq!(
        substitute_type_name("СправочникОбъект.<Имя справочника>", "Контрагенты"),
        "СправочникОбъект.Контрагенты"
    );
    assert_eq!(
        substitute_type_name("ДокументОбъект.&lt;Имя документа&gt;", "ЗаказКлиента"),
        "ДокументОбъект.ЗаказКлиента"
    );
}

#[test]
fn test_substitute_type_name_non_facet() {
    assert_eq!(
        substitute_type_name("Неопределено", "Контрагенты"),
        "Неопределено"
    );
    assert_eq!(substitute_type_name("Массив", "Тест"), "Массив");
}

// ================= extract_metadata_name tests =================

#[test]
fn test_extract_metadata_name_success() {
    assert_eq!(
        extract_metadata_name("СправочникМенеджер.Контрагенты"),
        Some("Контрагенты")
    );
    assert_eq!(
        extract_metadata_name("ДокументОбъект.ЗаказКлиента"),
        Some("ЗаказКлиента")
    );
}

#[test]
fn test_extract_metadata_name_non_facet() {
    assert_eq!(extract_metadata_name("Массив"), None);
    assert_eq!(extract_metadata_name("Файл.Существует"), None);
}
