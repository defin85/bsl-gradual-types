//! Реестр паттернов для определения MetadataKind из имён типов платформы 1С
//!
//! Поддерживает два источника паттернов:
//! 1. Автоизвлечённые из Syntax Helper при загрузке
//! 2. Хардкод fallback для случаев без Syntax Helper

use std::collections::HashMap;
use super::types::MetadataKind;

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
    /// Извлечённые из Syntax Helper: русский префикс -> MetadataKind
    extracted_prefixes: HashMap<String, MetadataKind>,
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
            self.extracted_prefixes.insert(pattern.prefix, pattern.kind);
        }
    }

    /// Определить MetadataKind из фасетного префикса
    /// Сначала ищет в извлечённых, затем в хардкоде
    pub fn get_metadata_kind(&self, prefix: &str) -> Option<MetadataKind> {
        // 1. Поиск в автоизвлечённых паттернах (по точному совпадению или starts_with)
        // Сортируем по длине (длинные первыми) для корректного матчинга
        let mut sorted_prefixes: Vec<_> = self.extracted_prefixes.iter().collect();
        sorted_prefixes.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        for (extracted_prefix, kind) in sorted_prefixes {
            if prefix.starts_with(extracted_prefix.as_str()) {
                return Some(*kind);
            }
        }

        // 2. Fallback на хардкод (важен порядок - длинные первыми!)
        Self::hardcoded_metadata_kind(prefix)
    }

    /// Хардкод fallback для MetadataKind (используется без Syntax Helper)
    ///
    /// Публичный метод для использования из SignatureIndex и других компонентов.
    pub fn hardcoded_metadata_kind(prefix: &str) -> Option<MetadataKind> {
        // Порядок важен: сначала более длинные префиксы!
        if prefix.starts_with("ПланВидовХарактеристик") {
            return Some(MetadataKind::ChartOfCharacteristicTypes);
        }
        if prefix.starts_with("ПланВидовРасчета") {
            return Some(MetadataKind::ChartOfCalculationTypes);
        }
        if prefix.starts_with("ПланСчетов") {
            return Some(MetadataKind::ChartOfAccounts);
        }
        if prefix.starts_with("ПланОбмена") {
            return Some(MetadataKind::ExchangePlan);
        }
        if prefix.starts_with("РегистрСведений") {
            return Some(MetadataKind::InformationRegister);
        }
        if prefix.starts_with("РегистрНакопления") {
            return Some(MetadataKind::AccumulationRegister);
        }
        if prefix.starts_with("РегистрБухгалтерии") {
            return Some(MetadataKind::AccountingRegister);
        }
        if prefix.starts_with("РегистрРасчета") {
            return Some(MetadataKind::CalculationRegister);
        }
        if prefix.starts_with("БизнесПроцесс") {
            return Some(MetadataKind::BusinessProcess);
        }
        if prefix.starts_with("Справочник") {
            return Some(MetadataKind::Catalog);
        }
        if prefix.starts_with("Документ") {
            return Some(MetadataKind::Document);
        }
        if prefix.starts_with("Перечисление") {
            return Some(MetadataKind::Enum);
        }
        if prefix.starts_with("Задача") {
            return Some(MetadataKind::Task);
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
        let dot_pos = type_name.find(".<")
            .or_else(|| type_name.find(".&lt;"))?;

        let full_prefix = &type_name[..dot_pos];

        // Извлекаем текст placeholder для определения MetadataKind
        let placeholder_start = dot_pos + if type_name[dot_pos..].starts_with(".&lt;") { 5 } else { 2 };
        let placeholder_end = type_name.find('>')
            .or_else(|| type_name.find("&gt;"))?;
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
        if placeholder.contains("бизнес-процесса") || placeholder.contains("бизнеспроцесса") {
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
            "НаборЗаписей", "МенеджерЗаписи", "Менеджер", "Объект",
            "Ссылка", "Выборка", "Список", "Запись"
        ];

        for suffix in FACET_SUFFIXES {
            if prefix.ends_with(suffix) {
                return &prefix[..prefix.len() - suffix.len()];
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
            "СправочникМенеджер.<Имя справочника>"
        );
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.prefix, "Справочник");
        assert_eq!(p.kind, MetadataKind::Catalog);
    }

    #[test]
    fn test_extract_pattern_document_object() {
        let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
            "ДокументОбъект.<Имя документа>"
        );
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.prefix, "Документ");
        assert_eq!(p.kind, MetadataKind::Document);
    }

    #[test]
    fn test_extract_pattern_info_register_recordset() {
        let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
            "РегистрСведенийНаборЗаписей.<Имя регистра сведений>"
        );
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.prefix, "РегистрСведений");
        assert_eq!(p.kind, MetadataKind::InformationRegister);
    }

    #[test]
    fn test_extract_pattern_exchange_plan() {
        let pattern = MetadataPatternRegistry::extract_pattern_from_type_name(
            "ПланОбменаМенеджер.<Имя плана обмена>"
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
    fn test_registry_fallback() {
        let registry = MetadataPatternRegistry::new();
        assert_eq!(registry.get_metadata_kind("СправочникМенеджер"), Some(MetadataKind::Catalog));
        assert_eq!(registry.get_metadata_kind("ДокументОбъект"), Some(MetadataKind::Document));
        assert_eq!(registry.get_metadata_kind("РегистрСведенийНаборЗаписей"), Some(MetadataKind::InformationRegister));
        assert_eq!(registry.get_metadata_kind("ПланОбменаОбъект"), Some(MetadataKind::ExchangePlan));
    }

    #[test]
    fn test_registry_with_extracted_patterns() {
        let mut registry = MetadataPatternRegistry::new();
        registry.update_from_patterns(vec![
            ExtractedPattern {
                prefix: "Справочник".to_string(),
                kind: MetadataKind::Catalog,
                placeholder_suffix: Some("справочника".to_string()),
            },
        ]);

        assert!(registry.has_extracted_patterns());
        assert_eq!(registry.extracted_count(), 1);
        assert_eq!(registry.get_metadata_kind("СправочникМенеджер"), Some(MetadataKind::Catalog));
    }

    #[test]
    fn test_strip_facet_suffix() {
        assert_eq!(MetadataPatternRegistry::strip_facet_suffix("СправочникМенеджер"), "Справочник");
        assert_eq!(MetadataPatternRegistry::strip_facet_suffix("ДокументОбъект"), "Документ");
        assert_eq!(MetadataPatternRegistry::strip_facet_suffix("РегистрСведенийНаборЗаписей"), "РегистрСведений");
        assert_eq!(MetadataPatternRegistry::strip_facet_suffix("Массив"), "Массив");
        assert_eq!(MetadataPatternRegistry::strip_facet_suffix("ПланОбменаСсылка"), "ПланОбмена");
    }

    #[test]
    fn test_registry_extracted_priority_over_hardcoded() {
        // Тест что extracted паттерны имеют приоритет над hardcoded
        let mut registry = MetadataPatternRegistry::new();

        // Добавляем кастомный паттерн
        registry.update_from_patterns(vec![
            ExtractedPattern {
                prefix: "Справочник".to_string(),
                kind: MetadataKind::Catalog,
                placeholder_suffix: Some("справочника".to_string()),
            },
        ]);

        // Извлечённый паттерн должен матчить
        assert_eq!(registry.get_metadata_kind("СправочникОбъект"), Some(MetadataKind::Catalog));
    }

    #[test]
    fn test_registry_fallback_when_no_extracted() {
        let registry = MetadataPatternRegistry::new();

        // Без extracted паттернов, используется fallback
        assert!(!registry.has_extracted_patterns());
        assert_eq!(registry.get_metadata_kind("БизнесПроцессОбъект"), Some(MetadataKind::BusinessProcess));
        assert_eq!(registry.get_metadata_kind("ЗадачаМенеджер"), Some(MetadataKind::Task));
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
            "СправочникОбъект.&lt;Имя справочника&gt;"
        );
        // Должен распознать
        assert!(pattern.is_some());
    }
}
