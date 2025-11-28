//! Реализации источников данных для SignatureIndex
//!
//! Конкретные источники типов для Registry паттерна.

use bsl_shared::domain::signature_registry::SignatureDataSource;
use bsl_shared::domain::types::RawTypeData;

use super::platform_types::load_all_platform_types;

/// Источник: типы из syntax_helper (загруженные из Syntax Helper файлов)
///
/// Это основной источник данных о типах платформы,
/// содержащий методы, свойства и другие метаданные.
///
/// # Приоритет: 10 (обрабатывается первым)
///
/// Методы из syntax_helper могут не иметь return_type,
/// поэтому PlatformFacetTypesSource с приоритетом 20
/// может дополнить их.
pub struct SyntaxHelperSource {
    types: Vec<RawTypeData>,
}

impl SyntaxHelperSource {
    /// Создать источник из уже загруженных типов
    pub fn new(types: Vec<RawTypeData>) -> Self {
        Self { types }
    }
}

impl SignatureDataSource for SyntaxHelperSource {
    fn name(&self) -> &str {
        "SyntaxHelper"
    }

    fn priority(&self) -> u32 {
        10
    }

    fn load(&self) -> Vec<RawTypeData> {
        self.types.clone()
    }
}

/// Источник: базовые фасетные типы платформы
///
/// Содержит типы:
/// - СправочникМенеджер, СправочникОбъект, СправочникСсылка, СправочникВыборка
/// - ДокументМенеджер, ДокументОбъект, ДокументСсылка, ДокументВыборка
/// - РегистрСведенийМенеджер, РегистрСведенийНаборЗаписей и т.д.
///
/// Эти типы определены в platform_types.rs и содержат методы:
/// СоздатьЭлемент(), НайтиПоКоду(), Записать(), Провести() и т.д.
///
/// # Приоритет: 20 (обрабатывается после SyntaxHelper)
///
/// Методы из этого источника дополняют (merge) методы из SyntaxHelper:
/// - Добавляют return_type если его не было
/// - Добавляют return_facet и context_requirements
pub struct PlatformFacetTypesSource;

impl SignatureDataSource for PlatformFacetTypesSource {
    fn name(&self) -> &str {
        "PlatformFacetTypes"
    }

    fn priority(&self) -> u32 {
        20
    }

    fn load(&self) -> Vec<RawTypeData> {
        load_all_platform_types()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_helper_source() {
        let types = vec![RawTypeData {
            name: "TestType".to_string(),
            ..Default::default()
        }];

        let source = SyntaxHelperSource::new(types);

        assert_eq!(source.name(), "SyntaxHelper");
        assert_eq!(source.priority(), 10);
        assert_eq!(source.load().len(), 1);
    }

    #[test]
    fn test_platform_facet_types_source() {
        let source = PlatformFacetTypesSource;

        assert_eq!(source.name(), "PlatformFacetTypes");
        assert_eq!(source.priority(), 20);

        // Должен загружать типы из platform_types.rs
        let types = source.load();
        assert!(!types.is_empty());

        // Проверяем наличие ключевых типов
        let has_catalog_manager = types
            .iter()
            .any(|t| t.name.contains("СправочникМенеджер"));
        assert!(has_catalog_manager);
    }

    #[test]
    fn test_priority_order() {
        let syntax = SyntaxHelperSource::new(vec![]);
        let facet = PlatformFacetTypesSource;

        // SyntaxHelper должен обрабатываться раньше (меньший приоритет)
        assert!(syntax.priority() < facet.priority());
    }
}
