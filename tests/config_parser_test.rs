#[cfg(test)]
mod tests {
    use bsl_gradual_types::adapters::config_parser_xml::{ConfigParserXml, MetadataKind};
    use bsl_gradual_types::core::types::Certainty;
    use std::path::Path;

    #[test]
    fn test_config_parser_with_example() {
        // Use example config from old project if exists
        let example_path = Path::new("C:/1CProject/bsl_type_safety_analyzer/examples/ConfTest");
        
        if example_path.exists() {
            let mut parser = ConfigParserXml::new(example_path);
            let resolutions = parser.parse_configuration().unwrap();
            
            // Check that we parsed something
            assert!(!resolutions.is_empty(), "Should parse at least one object");
            
            // Check that resolutions have Known certainty
            for resolution in &resolutions {
                assert_eq!(resolution.certainty, Certainty::Known);
            }
        }
    }

    #[test]
    fn test_metadata_kind_prefix() {
        assert_eq!(MetadataKind::Catalog.to_prefix(), "Справочники");
        assert_eq!(MetadataKind::Document.to_prefix(), "Документы");
        assert_eq!(MetadataKind::InformationRegister.to_prefix(), "РегистрыСведений");
    }
}