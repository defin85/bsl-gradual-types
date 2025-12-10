//! Вспомогательные функции для работы с фасетными типами
//!
//! Статические методы для:
//! - Извлечения базового фасетного типа
//! - Определения FacetKind из префикса
//! - Определения MetadataKind из префикса
//! - Подстановки имени объекта в return type

use super::super::facet_utils;
use super::super::metadata_patterns::MetadataPatternRegistry;
use super::super::types::{FacetKind, MetadataKind};

/// Извлечь базовый фасетный тип из полного имени типа
///
/// Делегирует в `facet_utils::extract_base_facet_type()`.
///
/// # Примеры
/// - "СправочникМенеджер.Контрагенты" -> Some("СправочникМенеджер")
/// - "ДокументОбъект.ЗаказКлиента" -> Some("ДокументОбъект")
/// - "Массив" -> None (не фасетный тип)
/// - "СправочникМенеджер" -> None (уже базовый тип)
pub fn extract_base_facet_type(type_name: &str) -> Option<&str> {
    facet_utils::extract_base_facet_type(type_name)
}

/// Получить FacetKind из фасетного префикса по суффиксу
///
/// # Примеры
/// - "СправочникМенеджер" -> Some(FacetKind::Manager)
/// - "ДокументОбъект" -> Some(FacetKind::Object)
/// - "ПеречислениеСсылка" -> Some(FacetKind::Reference)
/// - "Массив" -> None (не фасетный тип)
pub fn get_facet_kind_from_prefix(prefix: &str) -> Option<FacetKind> {
    // Порядок проверки важен: сначала длинные суффиксы!
    if prefix.ends_with("НаборЗаписей") {
        return Some(FacetKind::Collection); // Набор записей регистра - коллекция
    }
    if prefix.ends_with("МенеджерЗаписи") {
        return Some(FacetKind::Manager); // РегистрСведенийМенеджерЗаписи
    }
    if prefix.ends_with("Менеджер") {
        return Some(FacetKind::Manager);
    }
    if prefix.ends_with("Объект") {
        return Some(FacetKind::Object);
    }
    if prefix.ends_with("Ссылка") {
        return Some(FacetKind::Reference);
    }
    if prefix.ends_with("Выборка") {
        return Some(FacetKind::Selection);
    }
    if prefix.ends_with("Список") {
        return Some(FacetKind::List);
    }
    if prefix.ends_with("Запись") {
        return Some(FacetKind::Object); // Запись регистра - объект
    }
    None
}

/// Получить MetadataKind из фасетного префикса по его началу
///
/// # Примеры
/// - "СправочникМенеджер" -> Some(MetadataKind::Catalog)
/// - "ДокументОбъект" -> Some(MetadataKind::Document)
/// - "РегистрСведенийНаборЗаписей" -> Some(MetadataKind::InformationRegister)
///
/// Делегирует вызов в `MetadataPatternRegistry::hardcoded_metadata_kind()`.
pub fn get_metadata_kind_from_prefix(prefix: &str) -> Option<MetadataKind> {
    MetadataPatternRegistry::hardcoded_metadata_kind(prefix)
}

/// Подставить реальное имя объекта в return type вместо placeholder
///
/// # Примеры
/// - ("СправочникОбъект", "Контрагенты") -> "СправочникОбъект.Контрагенты"
/// - ("СправочникОбъект.<Имя справочника>", "Контрагенты") -> "СправочникОбъект.Контрагенты"
/// - ("СправочникСсылка", "Номенклатура") -> "СправочникСсылка.Номенклатура"
/// - ("Неопределено", "Контрагенты") -> "Неопределено" (не фасетный тип)
pub fn substitute_type_name(return_type: &str, actual_name: &str) -> String {
    // 1. Если return_type содержит placeholder с точкой (FacetPrefix.<placeholder>)
    //    Примеры: "СправочникОбъект.<Имя справочника>", "ДокументОбъект.&lt;Имя документа&gt;"
    if let Some(dot_pos) = return_type.find('.') {
        let prefix = &return_type[..dot_pos];
        if facet_utils::is_known_facet_prefix(prefix) {
            // Заменяем placeholder на actual_name
            return format!("{}.{}", prefix, actual_name);
        }
    }

    // 2. Если return_type - базовый фасетный тип (без точки), добавляем имя
    if facet_utils::is_known_facet_prefix(return_type) {
        format!("{}.{}", return_type, actual_name)
    } else {
        // Не фасетный тип - возвращаем как есть
        return_type.to_string()
    }
}

/// Извлечь имя объекта метаданных из фасетного типа
///
/// # Примеры
/// - "СправочникМенеджер.Контрагенты" -> Some("Контрагенты")
/// - "ДокументОбъект.ЗаказКлиента" -> Some("ЗаказКлиента")
/// - "Массив" -> None
pub fn extract_metadata_name(type_name: &str) -> Option<&str> {
    let dot_pos = type_name.find('.')?;
    let prefix = &type_name[..dot_pos];

    if facet_utils::is_known_facet_prefix(prefix) {
        Some(&type_name[dot_pos + 1..])
    } else {
        None
    }
}
