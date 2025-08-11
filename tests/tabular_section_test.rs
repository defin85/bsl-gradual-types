#[cfg(test)]
mod tests {
    use bsl_gradual_types::adapters::config_parser_xml::{ConfigParserXml, MetadataKind};
    use bsl_gradual_types::core::types::{ResolutionResult, ConcreteType};
    
    #[test]
    fn test_parse_tabular_sections() {
        // Используем реальную конфигурацию из старого проекта
        let config_path = r"C:\1CProject\bsl_type_safety_analyzer\examples\ConfTest";
        
        let mut parser = ConfigParserXml::new(config_path);
        let resolutions = parser.parse_configuration().unwrap();
        
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
            println!("Количество атрибутов: {}", cfg.attributes.len());
            println!("Количество табличных частей: {}", cfg.tabular_sections.len());
            
            // Проверяем табличную часть "Стороны"
            let ts_storony = cfg.tabular_sections.iter()
                .find(|ts| ts.name == "Стороны");
                
            assert!(ts_storony.is_some(), "Должна быть табличная часть Стороны");
            
            let ts_storony = ts_storony.unwrap();
            println!("\nТабличная часть: {}", ts_storony.name);
            println!("Количество атрибутов: {}", ts_storony.attributes.len());
            
            // Проверяем атрибут "Сторона" с составным типом
            let attr_storona = ts_storony.attributes.iter()
                .find(|a| a.name == "Сторона");
                
            assert!(attr_storona.is_some(), "Должен быть атрибут Сторона в табличной части");
            
            let attr = attr_storona.unwrap();
            println!("\nАтрибут табличной части:");
            println!("  Имя: {}", attr.name);
            println!("  Составной: {}", attr.is_composite);
            println!("  Типы: {:?}", attr.types);
            
            // Проверяем, что это составной тип
            assert!(attr.is_composite, "Атрибут Сторона должен быть составным");
            assert_eq!(attr.types.len(), 3, "Должно быть 3 типа");
            assert!(attr.types.contains(&"СправочникСсылка.Контрагенты".to_string()));
            assert!(attr.types.contains(&"СправочникСсылка.Организации".to_string()));
            assert!(attr.types.contains(&"Строка".to_string()));
            
            // Проверяем табличную часть "Работы"
            let ts_raboty = cfg.tabular_sections.iter()
                .find(|ts| ts.name == "Работы");
                
            assert!(ts_raboty.is_some(), "Должна быть табличная часть Работы");
            
            let ts_raboty = ts_raboty.unwrap();
            println!("\nТабличная часть: {}", ts_raboty.name);
            println!("Количество атрибутов: {}", ts_raboty.attributes.len());
            
            // Должна быть как минимум одна табличная часть
            assert!(cfg.tabular_sections.len() >= 2, "Должно быть минимум 2 табличные части");
        } else {
            panic!("Неверный тип результата");
        }
    }
}