//! MILESTONE 3.6 Phase 1: Форматирование hover с настраиваемыми уровнями детализации
//!
//! Общие типы форматирования используемые в backend и presentation слоях.

use crate::domain::metadata_constants::get_collection_kind;
use crate::domain::types::{
    FacetKind, TypeResolution, FORM_DATA_CANONICAL_TYPE_NAME, FORM_DATA_SEMANTICS_NOTE,
};

const LEGACY_FORM_OBJECT_PREFIX: &str = "ДанныеФормыОбъект.";

/// Уровень детализации hover информации
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailLevel {
    /// Только тип переменной (минимум)
    Compact,

    /// Тип + методы (до max_methods) - стандартный режим
    Full,

    /// Тип + методы + свойства + фасеты + документация (максимум)
    Detailed,
}

impl DetailLevel {
    /// Конвертация из строки (из VSCode settings)
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "compact" => Self::Compact,
            "detailed" => Self::Detailed,
            _ => Self::Full, // default
        }
    }
}

/// Тема оформления для форматирования
///
/// Унифицированный тип для использования во всех модулях форматирования.
/// Используется в hover_formatter, semantic_html_generator и других компонентах.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Theme {
    /// Тёмная тема (по умолчанию для VSCode)
    #[default]
    Dark,
    /// Светлая тема
    Light,
}

/// Возвращает `true`, если строка содержит legacy alias form-object типа.
pub fn contains_legacy_form_object_alias(value: &str) -> bool {
    value.contains(LEGACY_FORM_OBJECT_PREFIX)
}

/// Нормализует user-facing строку, заменяя `ДанныеФормыОбъект.<Коллекция>.<Имя>`
/// на платформенное фасетное имя `<ФасетОбъект>.<Имя>`.
///
/// Неподдерживаемые/неполные конструкции остаются без изменений.
pub fn normalize_user_facing_type_name(value: &str) -> String {
    if !contains_legacy_form_object_alias(value) {
        return value.to_string();
    }

    let mut normalized = String::with_capacity(value.len());
    let mut cursor = 0;

    while let Some(rel_start) = value[cursor..].find(LEGACY_FORM_OBJECT_PREFIX) {
        let start = cursor + rel_start;
        normalized.push_str(&value[cursor..start]);

        let payload_start = start + LEGACY_FORM_OBJECT_PREFIX.len();
        let payload = &value[payload_start..];
        let Some(collection_sep) = payload.find('.') else {
            normalized.push_str(LEGACY_FORM_OBJECT_PREFIX);
            cursor = payload_start;
            continue;
        };

        let collection = &payload[..collection_sep];
        let object_start = payload_start + collection_sep + 1;
        let object_len = value[object_start..]
            .char_indices()
            .take_while(|(_, ch)| is_type_name_char(*ch))
            .map(|(idx, ch)| idx + ch.len_utf8())
            .last()
            .unwrap_or(0);

        if object_len == 0 {
            normalized.push_str(LEGACY_FORM_OBJECT_PREFIX);
            cursor = payload_start;
            continue;
        }

        let object_name = &value[object_start..object_start + object_len];
        if let Some(mapped) = map_legacy_form_object_alias(collection, object_name) {
            normalized.push_str(&mapped);
            cursor = object_start + object_len;
        } else {
            normalized.push_str(LEGACY_FORM_OBJECT_PREFIX);
            cursor = payload_start;
        }
    }

    normalized.push_str(&value[cursor..]);
    normalized
}

/// Возвращает user-facing имя типа для каналов diagnostics/hover/type-at-position.
///
/// Для `FormModule.Объект` с form-data семантикой всегда возвращается
/// canonical label `ДанныеФормыСтруктура`.
pub fn user_facing_resolution_type_name(resolution: &TypeResolution) -> String {
    let is_form_data = resolution
        .metadata
        .notes
        .iter()
        .any(|note| note == FORM_DATA_SEMANTICS_NOTE);
    if is_form_data {
        return FORM_DATA_CANONICAL_TYPE_NAME.to_string();
    }

    normalize_user_facing_type_name(&resolution.type_name())
}

fn map_legacy_form_object_alias(collection: &str, object_name: &str) -> Option<String> {
    let kind = get_collection_kind(collection)?;
    let object_facet = kind.faceted_type_prefix(&FacetKind::Object);
    Some(format!("{}.{}", object_facet, object_name))
}

fn is_type_name_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detail_level_from_str() {
        assert_eq!(DetailLevel::parse("compact"), DetailLevel::Compact);
        assert_eq!(DetailLevel::parse("full"), DetailLevel::Full);
        assert_eq!(DetailLevel::parse("detailed"), DetailLevel::Detailed);
        assert_eq!(DetailLevel::parse("FULL"), DetailLevel::Full);
        assert_eq!(DetailLevel::parse("unknown"), DetailLevel::Full); // default
    }

    #[test]
    fn normalize_user_facing_type_name_rewrites_legacy_alias() {
        let value = "ДанныеФормыОбъект.Документы.РеализацияТоваровУслуг";
        assert_eq!(
            normalize_user_facing_type_name(value),
            "ДокументОбъект.РеализацияТоваровУслуг"
        );
    }

    #[test]
    fn normalize_user_facing_type_name_keeps_unknown_collection() {
        let value = "ДанныеФормыОбъект.Неизвестные.Объект1";
        assert_eq!(normalize_user_facing_type_name(value), value);
    }

    #[test]
    fn normalize_user_facing_type_name_rewrites_multiple_occurrences() {
        let value = "ДанныеФормыОбъект.Документы.Док1 | ДанныеФормыОбъект.Справочники.Спр1";
        assert_eq!(
            normalize_user_facing_type_name(value),
            "ДокументОбъект.Док1 | СправочникОбъект.Спр1"
        );
    }

    #[test]
    fn user_facing_resolution_type_name_prefers_form_data_canonical_label() {
        let mut resolution = TypeResolution::metadata_type(
            crate::domain::types::MetadataKind::Document,
            "Док1",
            Some(FacetKind::Object),
        );
        resolution
            .metadata
            .notes
            .push(FORM_DATA_SEMANTICS_NOTE.to_string());

        assert_eq!(
            user_facing_resolution_type_name(&resolution),
            FORM_DATA_CANONICAL_TYPE_NAME
        );
    }
}
