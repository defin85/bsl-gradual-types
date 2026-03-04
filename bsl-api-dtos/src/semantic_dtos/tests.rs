use super::*;

#[test]
fn test_semantic_tree_empty() {
    let tree = SemanticTreeDto::empty("test.bsl".to_string());
    assert_eq!(tree.file_path, "test.bsl");
    assert_eq!(tree.count_nodes(), 0);
    assert_eq!(tree.calculate_depth(), 0);
}

#[test]
fn test_semantic_node_count() {
    let node = SemanticNodeDto {
        kind: "Procedure".to_string(),
        name: Some("Test".to_string()),
        location: SourceLocationDto::new(1, 1),
        range: None,
        children: vec![
            SemanticNodeDto {
                kind: "Variable".to_string(),
                name: Some("x".to_string()),
                location: SourceLocationDto::new(2, 5),
                range: None,
                children: Vec::new(),
                attributes: HashMap::new(),
            },
            SemanticNodeDto {
                kind: "Variable".to_string(),
                name: Some("y".to_string()),
                location: SourceLocationDto::new(3, 5),
                range: None,
                children: Vec::new(),
                attributes: HashMap::new(),
            },
        ],
        attributes: HashMap::new(),
    };

    assert_eq!(node.count_nodes(), 3); // 1 parent + 2 children
    assert_eq!(node.calculate_depth(), 2); // 2 levels
}

#[test]
fn test_source_range() {
    let range = SourceRangeDto::new(SourceLocationDto::new(1, 1), SourceLocationDto::new(10, 20));

    assert_eq!(range.start.line, 1);
    assert_eq!(range.end.column, 20);
}

#[test]
fn test_metrics_default() {
    let metrics = SemanticMetricsDto::default();
    assert_eq!(metrics.procedure_count, 0);
    assert_eq!(metrics.average_certainty, 0.0);
}

#[test]
fn test_serialization() {
    let tree = SemanticTreeDto::empty("test.bsl".to_string());
    let json = serde_json::to_string(&tree).unwrap();
    let deserialized: SemanticTreeDto = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.file_path, "test.bsl");
}
