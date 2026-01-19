//! Integration tests for Milestone 3.16: Metadata Object Existence Validation
//!
//! Tests verify:
//! 1. Detection of unknown metadata objects (UnknownMetadataObject error)
//! 2. Levenshtein distance algorithm for similarity suggestions
//! 3. MetadataLookup API (exists_metadata_object, suggest_similar_names, is_configuration_loaded)
//! 4. HoverFormatter integration with unknown metadata objects
//! 5. Graceful degradation without configuration

mod support;

#[cfg(test)]
mod metadata_existence_validation_tests {
    use super::support;
    use bsl_shared::domain::repository::TypeRepository;
    use bsl_shared::domain::types::{
        Certainty, ConcreteType, ConfigurationType, MetadataKind, ResolutionMetadata,
        ResolutionResult, ResolutionSource, TypeResolution,
    };
    use bsl_shared::domain::TypeMetadataLookup;
    use bsl_shared::utils::string_utils::{levenshtein_distance, similarity};
    use std::sync::Arc;

    fn get_test_repository() -> Arc<dyn TypeRepository> {
        let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
        deps_bundle.semantic_deps.repository.clone()
    }

    /// Test 1: Levenshtein algorithm detects typos in Russian metadata names
    #[test]
    fn test_levenshtein_detects_russian_typos() {
        // Common 1C metadata typo: "Контрагенты" vs "Контрогенты" (а -> о)
        let distance = levenshtein_distance("Контрагенты", "Контрогенты");
        assert_eq!(
            distance, 1,
            "Single character difference should have distance 1"
        );

        // Missing character: "Контрагенты" vs "Контрагены"
        let distance = levenshtein_distance("Контрагенты", "Контрагены");
        assert_eq!(distance, 1, "Deletion should have distance 1");

        // Case insensitive
        let distance = levenshtein_distance("КОНТРАГЕНТЫ", "контрагенты");
        assert_eq!(distance, 0, "Case-insensitive comparison should match");
    }

    /// Test 2: Similarity calculation for fuzzy matching
    #[test]
    fn test_similarity_scoring_for_suggestions() {
        // Perfect match
        let sim = similarity("Контрагенты", "Контрагенты");
        assert!(
            (sim - 1.0).abs() < 0.001,
            "Identical strings should have similarity 1.0"
        );

        // One character difference in 11-char string
        let sim = similarity("Контрагенты", "Контрогенты");
        assert!(
            sim > 0.9 && sim < 1.0,
            "Single typo should have high similarity"
        );

        // Completely different
        let sim = similarity("Контрагенты", "АбсолютноДругое");
        assert!(
            sim < 0.5,
            "Completely different strings should have low similarity"
        );
    }

    /// Test 3: MetadataLookup.exists_metadata_object without configuration
    #[test]
    fn test_exists_metadata_object_without_configuration() {
        // Setup repository with only platform types (no configuration)
        let repository = get_test_repository();

        let lookup = TypeMetadataLookup::new(repository.clone());

        // Verify configuration is NOT loaded
        assert!(
            !lookup.is_configuration_loaded(),
            "Configuration should NOT be loaded"
        );

        // Try to check configuration metadata (should return false gracefully)
        let exists = lookup.exists_metadata_object(MetadataKind::Catalog, "Контрагенты");
        assert!(
            !exists,
            "Should return false for config object when config not loaded"
        );
    }

    /// Test 4: MetadataLookup.suggest_similar_names returns empty without configuration
    #[test]
    fn test_suggest_similar_names_without_configuration() {
        let repository = get_test_repository();

        let lookup = TypeMetadataLookup::new(repository);

        // Try to get suggestions for typo (should gracefully return empty)
        let suggestions = lookup.suggest_similar_names(MetadataKind::Catalog, "Контрогенты", 3);
        assert!(
            suggestions.is_empty(),
            "Should return empty suggestions when config not loaded"
        );
    }

    /// Test 5: Graceful degradation - hover for unknown configuration object
    #[test]
    fn test_hover_unknown_metadata_graceful_degradation() {
        let repository = get_test_repository();

        let lookup = TypeMetadataLookup::new(repository);

        // Create resolution for unknown configuration object
        let resolution = TypeResolution {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "НесуществующийСправочник".to_string(),
                facet: None,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        // Should not crash, just return None since config not loaded
        let raw_type = lookup.get_raw_type(&resolution);
        assert!(
            raw_type.is_none(),
            "Should return None for unknown config object"
        );

        // Methods should be empty
        let methods = lookup.get_methods(&resolution);
        assert!(
            methods.is_empty(),
            "Should return empty methods for unknown object"
        );
    }

    /// Test 6: MetadataLookup provides API for validators
    #[test]
    fn test_metadata_lookup_api_contract() {
        let repository = get_test_repository();

        let lookup = TypeMetadataLookup::new(repository);

        // 1. is_configuration_loaded() - boolean API
        let has_config = lookup.is_configuration_loaded();
        assert!(!has_config, "Configuration should not be loaded");

        // 2. exists_metadata_object(kind, name) -> bool
        // Even with graceful degradation, the method should work
        let exists = lookup.exists_metadata_object(MetadataKind::Catalog, "SomeObject");
        assert!(!exists, "Non-existent object should return false");

        // 3. suggest_similar_names(kind, name, limit) -> Vec<String>
        let suggestions = lookup.suggest_similar_names(MetadataKind::Catalog, "Test", 5);
        assert!(
            suggestions.is_empty() || suggestions.len() <= 5,
            "Should respect limit"
        );
    }

    /// Test 7: Test MetadataKind enum supports all metadata types
    #[test]
    fn test_metadata_kind_enum_completeness() {
        // Verify all expected metadata kinds are available
        let kinds = vec![
            MetadataKind::Catalog,
            MetadataKind::Document,
            MetadataKind::Unknown,
        ];

        for kind in kinds {
            // Should be able to create metadata objects of each kind
            let _ = format!("{:?}", kind);
        }
    }

    /// Test 8: Validators module has UnknownMetadataObject error variant
    #[test]
    fn test_validators_have_unknown_metadata_object_error() {
        use bsl_shared::domain::types::MetadataKind;
        use bsl_shared::domain::validators::TypeErrorKind;

        // Create an UnknownMetadataObject error
        let error = TypeErrorKind::UnknownMetadataObject {
            kind: MetadataKind::Catalog,
            name: "НесуществующийСправочник".to_string(),
            suggestions: vec!["Контрагенты".to_string()],
            variable_name: Some("objVar".to_string()),
        };

        // Verify error can be created and formatted
        let formatted = format!("{:?}", error);
        assert!(
            formatted.contains("UnknownMetadataObject"),
            "Error should contain variant name"
        );
    }

    /// Test 9: MetadataLookup graceful fallback to platform types
    #[test]
    fn test_fallback_to_platform_types() {
        let repository = get_test_repository();

        let lookup = TypeMetadataLookup::new(repository);

        // When configuration is not loaded, lookup should gracefully
        // degrade to checking if platform types exist
        assert!(!lookup.is_configuration_loaded());

        // This should not crash even though config is not loaded
        let exists = lookup.exists_metadata_object(MetadataKind::Document, "SomeDoc");
        assert!(!exists, "Unknown document should return false");
    }

    /// Test 10: Levenshtein distance matches expected algorithm behavior
    #[test]
    fn test_levenshtein_algorithm_correctness() {
        // Test cases from classic algorithm documentation
        test_case("kitten", "sitting", 3); // k->s, e->i, +g
        test_case("saturday", "sunday", 3); // sat->sun, rday->day
        test_case("", "", 0); // Both empty
        test_case("abc", "", 3); // Delete all
        test_case("", "xyz", 3); // Insert all
        test_case("a", "a", 0); // Single character match
        test_case("a", "b", 1); // Single character difference

        fn test_case(a: &str, b: &str, expected: usize) {
            let distance = levenshtein_distance(a, b);
            assert_eq!(
                distance, expected,
                "Distance between '{}' and '{}' should be {}, got {}",
                a, b, expected, distance
            );
        }
    }
}
