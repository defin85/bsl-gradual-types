//! Демонстрация Configuration-guided Discovery парсера

use bsl_gradual_types::data::loaders::config_parser_guided_discovery::ConfigurationGuidedParser;

fn main() {
    println!("🧪 Демо Configuration-guided парсера");

    let mut parser = ConfigurationGuidedParser::new("tests/fixtures/xml_full");

    match parser.parse_with_configuration_guide() {
        Ok(resolutions) => {
            let stats = parser.get_guided_discovery_stats();

            println!("✅ Результаты:");
            println!("   TypeResolution: {}", resolutions.len());
            println!("   Объекты: {}", stats.found_objects);
            println!("   Справочники: {}", stats.catalogs);
            println!("   Документы: {}", stats.documents);
            println!("   Регистры: {}", stats.registers);

            // Проверка регистра
            if let Some(metadata) =
                parser.get_discovered_metadata("РегистрыСведений.ТестовыйРегистрСведений")
            {
                println!("📊 Регистр ТестовыйРегистрСведений:");
                println!("   Атрибуты: {}", metadata.attributes.len());
                for attr in &metadata.attributes {
                    println!("   - {} ({})", attr.name, attr.type_definition);
                }

                // Проверки
                assert_eq!(metadata.attributes.len(), 5);
                let names: Vec<&str> = metadata
                    .attributes
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect();
                assert!(names.contains(&"ТестовыйРесурс"));
                assert!(names.contains(&"ТестовыйРеквизит"));
                assert!(names.contains(&"ТестовоеИзмерение"));
                assert!(names.contains(&"Период"));
                assert!(names.contains(&"Активность"));

                println!("✅ Все проверки пройдены!");
            }
        }
        Err(e) => {
            println!("❌ Ошибка: {}", e);
        }
    }
}
