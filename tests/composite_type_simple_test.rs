#[cfg(test)]
mod tests {
    use bsl_gradual_types::adapters::config_parser_xml::{ConfigParserXml, MetadataKind};
    use bsl_gradual_types::core::types::{ResolutionResult, ConcreteType};
    use std::path::Path;
    
    #[test]
    fn test_composite_type_parsing() {
        // Создаём парсер напрямую для одного файла
        let test_file = Path::new("tests/test_xml/TestDocument.xml");
        assert!(test_file.exists(), "Тестовый файл должен существовать");
        
        let parser = ConfigParserXml::new(".");
        
        // Парсим файл напрямую через внутренний метод
        let metadata = parser.parse_metadata_xml(test_file, &MetadataKind::Document)
            .expect("Должен распарсить тестовый документ");
        
        println!("Документ: {}", metadata.name);
        println!("Количество атрибутов: {}", metadata.attributes.len());
        
        // Проверяем составное поле
        let composite_attr = metadata.attributes.iter()
            .find(|a| a.name == "СоставноеПоле")
            .expect("Должен найти СоставноеПоле");
            
        println!("\nСоставное поле:");
        println!("  Имя: {}", composite_attr.name);
        println!("  Составной: {}", composite_attr.is_composite);
        println!("  Типы: {:?}", composite_attr.types);
        
        assert!(composite_attr.is_composite, "Поле должно быть составным");
        assert_eq!(composite_attr.types.len(), 3, "Должно быть 3 типа");
        assert!(composite_attr.types.contains(&"СправочникСсылка.Контрагенты".to_string()));
        assert!(composite_attr.types.contains(&"СправочникСсылка.Организации".to_string()));
        assert!(composite_attr.types.contains(&"Строка".to_string()));
        
        // Проверяем простое поле
        let simple_attr = metadata.attributes.iter()
            .find(|a| a.name == "ПростоеПоле")
            .expect("Должен найти ПростоеПоле");
            
        println!("\nПростое поле:");
        println!("  Имя: {}", simple_attr.name);
        println!("  Составной: {}", simple_attr.is_composite);
        println!("  Тип: {}", simple_attr.type_);
        
        assert!(!simple_attr.is_composite, "Поле не должно быть составным");
        assert_eq!(simple_attr.types.len(), 1, "Должен быть 1 тип");
        assert_eq!(simple_attr.types[0], "Число");
    }
}