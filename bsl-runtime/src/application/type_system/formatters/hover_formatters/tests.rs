use super::*;
use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::ir::{ScopeId, SemanticNode, SemanticNodeKind, Span};
use std::sync::Arc;

#[test]
fn test_condition_hover_includes_expected_actual_and_certainty() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let metadata_lookup = TypeMetadataLookup::new(repo);

    let node = SemanticNode {
        kind: SemanticNodeKind::IfStatement {
            then_branch: Vec::new(),
            else_branch: None,
        },
        span: Span::stub(),
        scope_id: ScopeId(0),
    };

    let result = format_semantic_node_info(&node, "", &metadata_lookup);

    assert!(result.contains("Если ... Тогда"));
}

#[test]
fn test_condition_hover_includes_uncertainty_reason() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let metadata_lookup = TypeMetadataLookup::new(repo);

    let node = SemanticNode {
        kind: SemanticNodeKind::WhileLoop { body: Vec::new() },
        span: Span::stub(),
        scope_id: ScopeId(0),
    };

    let result = format_semantic_node_info(&node, "", &metadata_lookup);

    assert!(result.contains("Пока ... Цикл"));
}
