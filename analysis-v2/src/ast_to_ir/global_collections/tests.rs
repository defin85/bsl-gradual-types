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
