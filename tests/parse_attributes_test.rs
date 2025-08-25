#[cfg(test)]
mod tests {
    use bsl_gradual_types::data::loaders::config_parser_xml::ConfigParserXml;
    use bsl_gradual_types::core::types::{ConcreteType, ResolutionResult};

    #[test]
    fn test_parse_catalog_attributes() {
        // Use test configuration from old project
        let config_path = r"C:\1CProject\bsl_type_safety_analyzer\examples\ConfTest";

        let mut parser = ConfigParserXml::new(config_path);
        let resolutions = parser.parse_configuration().unwrap();

        // Find Организации catalog
        let org_resolution = resolutions.iter().find(|r| {
            if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &r.result {
                cfg.name == "Организации"
            } else {
                false
            }
        });

        assert!(
            org_resolution.is_some(),
            "Должен найти справочник Организации"
        );

        let org = org_resolution.unwrap();
        if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &org.result {
            println!("Справочник: {}", cfg.name);
            println!("Атрибуты: {:?}", cfg.attributes);

            // Check that we found КраткоеНаименование attribute
            let attr = cfg
                .attributes
                .iter()
                .find(|a| a.name == "КраткоеНаименование");

            assert!(attr.is_some(), "Должен найти атрибут КраткоеНаименование");

            let attr = attr.unwrap();
            assert_eq!(attr.name, "КраткоеНаименование");
            // Type should be parsed from XML
            println!("Тип атрибута: {}", attr.type_);
        } else {
            panic!("Неверный тип результата");
        }
    }

    #[test]
    fn test_catalog_without_attributes() {
        let config_path = r"C:\1CProject\bsl_type_safety_analyzer\examples\ConfTest";

        let mut parser = ConfigParserXml::new(config_path);
        let resolutions = parser.parse_configuration().unwrap();

        // Find Контрагенты catalog
        let kontr_resolution = resolutions.iter().find(|r| {
            if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &r.result {
                cfg.name == "Контрагенты"
            } else {
                false
            }
        });

        assert!(
            kontr_resolution.is_some(),
            "Должен найти справочник Контрагенты"
        );

        let kontr = kontr_resolution.unwrap();
        if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &kontr.result {
            println!("Справочник: {}", cfg.name);
            println!("Количество атрибутов: {}", cfg.attributes.len());

            // This catalog has no attributes in XML
            assert_eq!(cfg.attributes.len(), 0, "Контрагенты не имеет атрибутов");
        }
    }
}
