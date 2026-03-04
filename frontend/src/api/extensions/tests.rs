use super::*;

#[test]
fn test_certainty_color() {
    let type_dto = TypeDto {
        id: "test".to_string(),
        name: "TestType".to_string(),
        category: "Platform".to_string(),
        certainty: 100,
        certainty_text: "Known 100%".to_string(),
        facets: vec![],
        methods_count: None,
        methods: vec![],
        attributes_count: None,
        properties: vec![],
        enum_values: None,
        tabular_sections: vec![],
        source: "Test".to_string(),
        flow_sensitive: false,
        description: "Test".to_string(),
        union_types: None,
        flow_analysis: None,
        connections: None,
        warning: None,
        recommendation: None,
    };

    assert_eq!(type_dto.certainty_color(), "#28a745"); // Green for Known
}

#[test]
fn test_filters_matches() {
    let type_dto = TypeDto {
        id: "test".to_string(),
        name: "Массив".to_string(),
        category: "Platform".to_string(),
        certainty: 100,
        certainty_text: "Known 100%".to_string(),
        facets: vec!["Collection".to_string()],
        methods_count: None,
        methods: vec![],
        attributes_count: None,
        properties: vec![],
        enum_values: None,
        tabular_sections: vec![],
        source: "Platform".to_string(),
        flow_sensitive: false,
        description: "Array".to_string(),
        union_types: None,
        flow_analysis: None,
        connections: None,
        warning: None,
        recommendation: None,
    };

    let mut filters = TypeFilters::new();
    assert!(filters.matches(&type_dto)); // Default matches all

    filters.search_query = Some("Масс".to_string());
    assert!(filters.matches(&type_dto));

    filters.search_query = Some("Строка".to_string());
    assert!(!filters.matches(&type_dto));
}
