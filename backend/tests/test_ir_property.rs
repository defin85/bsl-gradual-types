//! Тест для проверки MemberAccess vs FunctionCall

use bsl_analysis_v2::AstToIrConverter;
use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use bsl_shared::domain::signature_index::SignatureIndex;
use std::sync::Arc;
use tree_sitter::Parser;

#[test]
fn test_property_access() {
    let code = "ТЗ = Новый ТаблицаЗначений;\nКол = ТЗ.Количество;";

    println!("\n=== Property access (без скобок) ===\n{}\n", code);

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(code, None).unwrap();
    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).unwrap();

    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let signature_index = SignatureIndex::new();
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    println!("IR nodes: {}", ir.nodes.len());
    for (i, node) in ir.nodes.iter().enumerate() {
        println!("  [{}] {:?}", i, node.kind);
    }

    let has_member_access = ir.nodes.iter().any(|n| {
        matches!(
            n.kind,
            bsl_shared::ir::SemanticNodeKind::MemberAccess { .. }
        )
    });
    let has_function_call = ir.nodes.iter().any(|n| {
        matches!(
            n.kind,
            bsl_shared::ir::SemanticNodeKind::FunctionCall { .. }
        )
    });

    println!("\nHas MemberAccess: {}", has_member_access);
    println!("Has FunctionCall: {}", has_function_call);
}

#[test]
fn test_method_call() {
    let code = "ТЗ = Новый ТаблицаЗначений;\nКол = ТЗ.Количество();";

    println!("\n=== Method call (со скобками) ===\n{}\n", code);

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(code, None).unwrap();
    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).unwrap();

    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let signature_index = SignatureIndex::new();
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    println!("IR nodes: {}", ir.nodes.len());
    for (i, node) in ir.nodes.iter().enumerate() {
        println!("  [{}] {:?}", i, node.kind);
    }

    let has_member_access = ir.nodes.iter().any(|n| {
        matches!(
            n.kind,
            bsl_shared::ir::SemanticNodeKind::MemberAccess { .. }
        )
    });
    let has_function_call = ir.nodes.iter().any(|n| {
        matches!(
            n.kind,
            bsl_shared::ir::SemanticNodeKind::FunctionCall { .. }
        )
    });

    println!("\nHas MemberAccess: {}", has_member_access);
    println!("Has FunctionCall: {}", has_function_call);
}

#[test]
fn test_statement_property_access() {
    let code = "ТЗ = Новый ТаблицаЗначений;\nТЗ.Колонки;";

    println!(
        "\n=== Statement property access (свойство как statement) ===\n{}\n",
        code
    );

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(code, None).unwrap();
    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).unwrap();

    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let signature_index = SignatureIndex::new();
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
    .unwrap();

    println!("IR nodes: {}", ir.nodes.len());
    for (i, node) in ir.nodes.iter().enumerate() {
        println!("  [{}] {:?}", i, node.kind);
    }

    let has_member_access = ir.nodes.iter().any(|n| {
        matches!(
            n.kind,
            bsl_shared::ir::SemanticNodeKind::MemberAccess { .. }
        )
    });

    println!("\nHas MemberAccess: {}", has_member_access);
}
