//! Реестр паттернов для определения MetadataKind из имён типов платформы 1С
//!
//! Поддерживает только автоизвлечённые паттерны из Syntax Helper при загрузке.

use crate::type_id::TypeId;
use crate::types::MetadataKind;
use std::collections::HashMap;

/// Паттерн извлечённый из Syntax Helper
#[derive(Debug, Clone)]
pub struct ExtractedPattern {
    /// Русский префикс метаданных: "Справочник", "РегистрСведений"
    pub prefix: String,
    /// MetadataKind для этого префикса
    pub kind: MetadataKind,
    /// Суффикс из placeholder (опционально): "справочника", "регистра сведений"
    pub placeholder_suffix: Option<String>,
}

/// Реестр паттернов для определения MetadataKind из имени типа
#[derive(Debug, Clone, Default)]
pub struct MetadataPatternRegistry {
    /// Извлечённые из Syntax Helper: русский префикс (TypeId) -> MetadataKind
    extracted_prefixes: HashMap<TypeId, MetadataKind>,
}

impl MetadataPatternRegistry {
    /// Создать пустой реестр
    pub fn new() -> Self {
        Self {
            extracted_prefixes: HashMap::new(),
        }
    }

    /// Обновить реестр данными из Syntax Helper
    pub fn update_from_patterns(&mut self, patterns: Vec<ExtractedPattern>) {
        for pattern in patterns {
            self.extracted_prefixes
                .insert(TypeId::new(&pattern.prefix), pattern.kind);
        }
    }

    /// Определить MetadataKind из фасетного префикса
    pub fn get_metadata_kind(&self, prefix: &str) -> Option<MetadataKind> {
        // 1. Поиск в автоизвлечённых паттернах на нормализованных строках
        // Сортируем по длине (длинные первыми) для корректного матчинга
        let prefix_normalized = crate::type_id::normalize(prefix);

        let mut sorted_prefixes: Vec<_> = self.extracted_prefixes.iter().collect();
        sorted_prefixes.sort_by(|a, b| b.0.normalized().len().cmp(&a.0.normalized().len()));

        for (extracted_type_id, kind) in sorted_prefixes {
            if prefix_normalized.starts_with(extracted_type_id.normalized()) {
                return Some(*kind);
            }
        }

        None
    }

    /// Проверить, есть ли извлечённые паттерны
    pub fn has_extracted_patterns(&self) -> bool {
        !self.extracted_prefixes.is_empty()
    }

    /// Получить количество извлечённых паттернов
    pub fn extracted_count(&self) -> usize {
        self.extracted_prefixes.len()
    }

    /// Извлечь паттерн из имени типа Syntax Helper
    ///
    /// # Примеры
    /// - "СправочникМенеджер.<Имя справочника>" -> Some(ExtractedPattern { prefix: "Справочник", kind: Catalog })
    /// - "РегистрСведенийНаборЗаписей.<Имя регистра сведений>" -> Some(ExtractedPattern { prefix: "РегистрСведений", kind: InformationRegister })
    /// - "Массив" -> None (не фасетный тип)
    pub fn extract_pattern_from_type_name(type_name: &str) -> Option<ExtractedPattern> {
        // Ищем placeholder в формате ".<Имя ...>" или ".&lt;Имя ...&gt;"
        let dot_pos = type_name.find(".<").or_else(|| type_name.find(".&lt;"))?;

        let full_prefix = &type_name[..dot_pos];

        // Извлекаем текст placeholder для определения MetadataKind
        let placeholder_start = dot_pos
            + if type_name[dot_pos..].starts_with(".&lt;") {
                5
            } else {
                2
            };
        let placeholder_end = type_name.find('>').or_else(|| type_name.find("&gt;"))?;
        let placeholder_text = &type_name[placeholder_start..placeholder_end];
        let placeholder_lower = placeholder_text.to_lowercase();

        // Определяем MetadataKind по placeholder
        let kind = Self::metadata_kind_from_placeholder(&placeholder_lower)?;

        // Убираем фасетный суффикс чтобы получить базовый префикс
        let prefix = Self::strip_facet_suffix(full_prefix);

        Some(ExtractedPattern {
            prefix: prefix.to_string(),
            kind,
            placeholder_suffix: Some(placeholder_lower),
        })
    }

    /// Определить MetadataKind по тексту placeholder
    fn metadata_kind_from_placeholder(placeholder: &str) -> Option<MetadataKind> {
        // Используем contains для гибкости ("Имя справочника" -> "справочника")
        if placeholder.contains("справочника") {
            return Some(MetadataKind::Catalog);
        }
        if placeholder.contains("документа") {
            return Some(MetadataKind::Document);
        }
        if placeholder.contains("перечисления") {
            return Some(MetadataKind::Enum);
        }
        if placeholder.contains("регистра сведений") {
            return Some(MetadataKind::InformationRegister);
        }
        if placeholder.contains("регистра накопления") {
            return Some(MetadataKind::AccumulationRegister);
        }
        if placeholder.contains("регистра бухгалтерии") {
            return Some(MetadataKind::AccountingRegister);
        }
        if placeholder.contains("регистра расчета") {
            return Some(MetadataKind::CalculationRegister);
        }
        if placeholder.contains("плана видов характеристик") {
            return Some(MetadataKind::ChartOfCharacteristicTypes);
        }
        if placeholder.contains("плана видов расчета") {
            return Some(MetadataKind::ChartOfCalculationTypes);
        }
        if placeholder.contains("плана счетов") {
            return Some(MetadataKind::ChartOfAccounts);
        }
        if placeholder.contains("плана обмена") {
            return Some(MetadataKind::ExchangePlan);
        }
        if placeholder.contains("бизнес-процесса") || placeholder.contains("бизнеспроцесса")
        {
            return Some(MetadataKind::BusinessProcess);
        }
        if placeholder.contains("задачи") {
            return Some(MetadataKind::Task);
        }
        None
    }

    /// Убрать фасетный суффикс из префикса
    /// "СправочникМенеджер" -> "Справочник"
    /// "РегистрСведенийНаборЗаписей" -> "РегистрСведений"
    pub fn strip_facet_suffix(prefix: &str) -> &str {
        const FACET_SUFFIXES: &[&str] = &[
            "НаборЗаписей",
            "МенеджерЗаписи",
            "Менеджер",
            "Объект",
            "Ссылка",
            "Выборка",
            "Список",
            "Запись",
        ];

        for suffix in FACET_SUFFIXES {
            if let Some(stripped) = prefix.strip_suffix(suffix) {
                return stripped;
            }
        }
        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pattern_catalog_manager() {
        let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
            "СправочникМенеджер.<Имя справочника>",
        );
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.prefix, "Справочник");
        assert_eq!(p.kind, MetadataKind::Catalog);
    }

    #[test]
    fn test_extract_pattern_document_object() {
        let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
            "ДокументОбъект.<Имя документа>",
        );
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.prefix, "Документ");
        assert_eq!(p.kind, MetadataKind::Document);
    }

    #[test]
    fn test_extract_pattern_info_register_recordset() {
        let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
            "РегистрСведенийНаборЗаписей.<Имя регистра сведений>",
        );
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.prefix, "РегистрСведений");
        assert_eq!(p.kind, MetadataKind::InformationRegister);
    }

    #[test]
    fn test_extract_pattern_exchange_plan() {
        let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
            "ПланОбменаМенеджер.<Имя плана обмена>",
        );
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.prefix, "ПланОбмена");
        assert_eq!(p.kind, MetadataKind::ExchangePlan);
    }

    #[test]
    fn test_extract_pattern_non_faceted() {
        let pattern = MetadataPatternRegistry::extract_pattern_from_type_name("Массив");
        assert!(pattern.is_none());
    }

    #[test]
    fn test_registry_without_patterns_returns_none() {
        let registry = MetadataPatternRegistry::new();
        assert_eq!(registry.get_metadata_kind("СправочникМенеджер"), None);
        assert_eq!(registry.get_metadata_kind("ДокументОбъект"), None);
        assert_eq!(
            registry.get_metadata_kind("РегистрСведенийНаборЗаписей"),
            None
        );
        assert_eq!(registry.get_metadata_kind("ПланОбменаОбъект"), None);
    }

    #[test]
    fn test_registry_with_extracted_patterns() {
        let mut registry = MetadataPatternRegistry::new();
        registry.update_from_patterns(vec![ExtractedPattern {
            prefix: "Справочник".to_string(),
            kind: MetadataKind::Catalog,
            placeholder_suffix: Some("справочника".to_string()),
        }]);

        assert!(registry.has_extracted_patterns());
        assert_eq!(registry.extracted_count(), 1);
        assert_eq!(
            registry.get_metadata_kind("СправочникМенеджер"),
            Some(MetadataKind::Catalog)
        );
    }

    #[test]
    fn test_strip_facet_suffix() {
        assert_eq!(
            MetadataPatternRegistry::strip_facet_suffix("СправочникМенеджер"),
            "Справочник"
        );
        assert_eq!(
            MetadataPatternRegistry::strip_facet_suffix("ДокументОбъект"),
            "Документ"
        );
        assert_eq!(
            MetadataPatternRegistry::strip_facet_suffix("РегистрСведенийНаборЗаписей"),
            "РегистрСведений"
        );
        assert_eq!(
            MetadataPatternRegistry::strip_facet_suffix("Массив"),
            "Массив"
        );
        assert_eq!(
            MetadataPatternRegistry::strip_facet_suffix("ПланОбменаСсылка"),
            "ПланОбмена"
        );
    }

    #[test]
    fn test_registry_extracted_patterns_match() {
        let mut registry = MetadataPatternRegistry::new();

        registry.update_from_patterns(vec![ExtractedPattern {
            prefix: "Справочник".to_string(),
            kind: MetadataKind::Catalog,
            placeholder_suffix: Some("справочника".to_string()),
        }]);

        assert_eq!(
            registry.get_metadata_kind("СправочникОбъект"),
            Some(MetadataKind::Catalog)
        );
    }

    #[test]
    fn test_metadata_kind_from_placeholder_variations() {
        // Тест различных форматов placeholder текста
        assert_eq!(
            MetadataPatternRegistry::metadata_kind_from_placeholder("имя справочника"),
            Some(MetadataKind::Catalog)
        );
        assert_eq!(
            MetadataPatternRegistry::metadata_kind_from_placeholder("имя регистра сведений"),
            Some(MetadataKind::InformationRegister)
        );
        assert_eq!(
            MetadataPatternRegistry::metadata_kind_from_placeholder("имя плана обмена"),
            Some(MetadataKind::ExchangePlan)
        );
        // Неизвестный placeholder
        assert_eq!(
            MetadataPatternRegistry::metadata_kind_from_placeholder("неизвестный тип"),
            None
        );
    }

    #[test]
    fn test_extract_pattern_with_html_entities() {
        // Тест для HTML-encoded placeholder (некоторые источники могут кодировать)
        let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
            "СправочникОбъект.&lt;Имя справочника&gt;",
        );
        // Должен распознать
        assert!(pattern.is_some());
    }
}
