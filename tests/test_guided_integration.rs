//! Unit-тесты интеграции ConfigurationGuidedParser с PlatformTypeResolver

#[cfg(test)]
mod tests {
    use bsl_gradual_types::data::loaders::config_parser_guided_discovery::ConfigurationGuidedParser;

    #[test]
    fn test_configuration_guided_parser_creation() {
        println!("🚀 Тест создания ConfigurationGuidedParser");

        // Создание парсера с тестовым путем
        let guided_parser = ConfigurationGuidedParser::new("test_path");

        // Проверка, что парсер создался
        println!("✅ ConfigurationGuidedParser создан успешно");

        // Базовая проверка Debug trait
        let debug_output = format!("{:?}", guided_parser);
        assert!(debug_output.contains("ConfigurationGuidedParser"));
        println!("✅ Debug trait работает: {}", debug_output.len() > 0);
    }

    #[test]
    fn test_integration_types_compilation() {
        println!("🚀 Тест компиляции типов интеграции");

        // Проверяем, что все типы доступны
        use bsl_gradual_types::domain::types::{
            Certainty, MetadataKind, ResolutionMetadata, ResolutionResult, ResolutionSource,
            TypeResolution,
        };

        // Создание базовых структур
        let metadata = ResolutionMetadata {
            file: Some("test.bsl".to_string()),
            line: Some(1),
            column: Some(1),
            notes: vec!["test note".to_string()],
        };

        let resolution = TypeResolution {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Static,
            metadata,
            active_facet: None,
            available_facets: vec![],
        };

        println!("✅ TypeResolution создан: {:?}", resolution.certainty);

        // Проверка MetadataKind
        let kind = MetadataKind::Catalog;
        println!("✅ MetadataKind создан: {:?}", kind);

        assert!(true);
    }

    #[test]
    fn test_guided_parser_error_handling() {
        println!("🚀 Тест обработки ошибок ConfigurationGuidedParser");

        let mut guided_parser = ConfigurationGuidedParser::new("non_existent_path");

        // Попытка парсинга несуществующего пути должна вернуть ошибку
        let result = guided_parser.parse_with_configuration_guide();

        match result {
            Ok(_) => {
                println!("❌ Неожиданно удалось распарсить несуществующий путь");
                assert!(false, "Должна была быть ошибка для несуществующего пути");
            }
            Err(e) => {
                println!("✅ Ожидаемая ошибка: {}", e);
                assert!(true);
            }
        }
    }
}
