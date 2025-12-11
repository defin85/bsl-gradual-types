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
/// Делегирует в `facet_utils::get_facet_kind_from_prefix()`.
///
/// # Примеры
/// - "СправочникМенеджер" -> Some(FacetKind::Manager)
/// - "ДокументОбъект" -> Some(FacetKind::Object)
/// - "ПеречислениеСсылка" -> Some(FacetKind::Reference)
/// - "Массив" -> None (не фасетный тип)
pub fn get_facet_kind_from_prefix(prefix: &str) -> Option<FacetKind> {
    facet_utils::get_facet_kind_from_prefix(prefix)
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
/// Делегирует в `facet_utils::substitute_type_name()`.
///
/// # Примеры
/// - ("СправочникОбъект", "Контрагенты") -> "СправочникОбъект.Контрагенты"
/// - ("СправочникОбъект.<Имя справочника>", "Контрагенты") -> "СправочникОбъект.Контрагенты"
/// - ("СправочникСсылка", "Номенклатура") -> "СправочникСсылка.Номенклатура"
/// - ("Неопределено", "Контрагенты") -> "Неопределено" (не фасетный тип)
pub fn substitute_type_name(return_type: &str, actual_name: &str) -> String {
    facet_utils::substitute_type_name(return_type, actual_name)
}

/// Извлечь имя объекта метаданных из фасетного типа
///
/// Делегирует в `facet_utils::extract_metadata_name()`.
///
/// # Примеры
/// - "СправочникМенеджер.Контрагенты" -> Some("Контрагенты")
/// - "ДокументОбъект.ЗаказКлиента" -> Some("ЗаказКлиента")
/// - "Массив" -> None
pub fn extract_metadata_name(type_name: &str) -> Option<&str> {
    facet_utils::extract_metadata_name(type_name)
}
