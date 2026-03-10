use super::*;
use bsl_shared::ir::{SemanticFacts, SemanticProgram, SymbolTable};

#[test]
fn test_generate_dark_theme() {
    let program = SemanticProgram {
        symbols: SymbolTable::new(),
        nodes: vec![],
        source_info: bsl_shared::ir::SourceInfo {
            path: "test.bsl".to_string(),
            content_hash: 0,
        },
        cfg: None,
        semantic_facts: SemanticFacts::default(),
    };
    let html = generate_semantic_html(
        &program,
        "test.bsl",
        RenderOptions {
            theme: Theme::Dark,
            compact: false,
        },
    );
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("bg-gray-900"));
    assert!(html.contains("Semantic Tree Visualization"));
}

#[test]
fn test_generate_light_theme() {
    let program = SemanticProgram {
        symbols: SymbolTable::new(),
        nodes: vec![],
        source_info: bsl_shared::ir::SourceInfo {
            path: "test.bsl".to_string(),
            content_hash: 0,
        },
        cfg: None,
        semantic_facts: SemanticFacts::default(),
    };
    let html = generate_semantic_html(
        &program,
        "test.bsl",
        RenderOptions {
            theme: Theme::Light,
            compact: false,
        },
    );
    assert!(html.contains("bg-white"));
    assert!(html.contains("Semantic Tree Visualization"));
}
