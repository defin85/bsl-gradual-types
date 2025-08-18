#[cfg(test)]
mod tests {
    use bsl_gradual_types::adapters::config_parser_xml::ConfigParserXml;
    use bsl_gradual_types::core::types::{ResolutionResult, ConcreteType};
    
    #[test]
    fn test_parse_composite_type_attribute() {
        // Используем документ ЗаказНаряды с составным полем Сторона из реальной конфигурации
        let config_path = "../conf/conf_test";
        
        let mut parser = ConfigParserXml::new(config_path);
        let resolutions = parser.parse_configuration().unwrap();
        
        // Выведем все найденные документы для отладки
        println!("Найдено объектов: {}", resolutions.len());
        for r in &resolutions {
            if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &r.result {
                println!("  - {:?}: {}", cfg.kind, cfg.name);
            }
        }
        
        // Найдём документ ЗаказНаряды
        let doc_resolution = resolutions.iter()
            .find(|r| {
                if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &r.result {
                    cfg.name == "ЗаказНаряды"
                } else {
                    false
                }
            });
            
        assert!(doc_resolution.is_some(), "Должен найти документ ЗаказНаряды");
        
        let doc = doc_resolution.unwrap();
        if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &doc.result {
            println!("Документ: {}", cfg.name);
            
            // Найдём табличную часть Стороны
            let ts = cfg.tabular_sections.iter()
                .find(|ts| ts.name == "Стороны");
                
            assert!(ts.is_some(), "Должна быть табличная часть Стороны");
            
            let ts = ts.unwrap();
            println!("Табличная часть: {}", ts.name);
            
            // Найдём атрибут Сторона в табличной части
            let attr = ts.attributes.iter()
                .find(|a| a.name == "Сторона");
                
            if let Some(attr) = attr {
                println!("Атрибут: {}", attr.name);
                println!("  Составной: {}", attr.is_composite);
                println!("  Тип: {}", attr.type_);
                println!("  Типы: {:?}", attr.types);
                
                assert!(attr.is_composite, "Поле Сторона должно быть составным");
                assert_eq!(attr.types.len(), 3, "Должно быть 3 типа");
                
                // Проверим что типы нормализованы правильно
                assert!(attr.types.contains(&"СправочникСсылка.Контрагенты".to_string()));
                assert!(attr.types.contains(&"СправочникСсылка.Организации".to_string()));
                assert!(attr.types.contains(&"Строка".to_string()));
            } else {
                println!("Доступные атрибуты в табличной части:");
                for a in &ts.attributes {
                    println!("  - {}: {}", a.name, a.type_);
                }
                panic!("Не найден атрибут Сторона в табличной части");
            }
        }
    }
    
    #[test]
    fn test_simple_type_attribute() {
        let config_path = "../conf/conf_test";
        
        let mut parser = ConfigParserXml::new(config_path);
        let resolutions = parser.parse_configuration().unwrap();
        
        // Найдём справочник Организации с простым атрибутом
        let org_resolution = resolutions.iter()
            .find(|r| {
                if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &r.result {
                    cfg.name == "Организации"
                } else {
                    false
                }
            });
            
        assert!(org_resolution.is_some());
        
        let org = org_resolution.unwrap();
        if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &org.result {
            let attr = cfg.attributes.iter()
                .find(|a| a.name == "КраткоеНаименование")
                .expect("Должен найти атрибут КраткоеНаименование");
            
            println!("Простой атрибут: {}", attr.name);
            println!("  Составной: {}", attr.is_composite);
            println!("  Тип: {}", attr.type_);
            
            assert!(!attr.is_composite, "КраткоеНаименование не должно быть составным");
            assert_eq!(attr.types.len(), 1, "Должен быть один тип");
            assert_eq!(attr.types[0], "Строка");
        }
    }
}