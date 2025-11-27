//! Тест для проверки Документы.ЗаказКлиента IR

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use bsl_shared::domain::signature_index::SignatureIndex;
use std::sync::Arc;
use tree_sitter::Parser;

#[test]
fn test_documents_property_access() {
    let code = "x = Документы.ЗаказКлиента;";

    println!("\n=== Документы.ЗаказКлиента ===\n{}\n", code);

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_bsl::LANGUAGE.into()).unwrap();
    let tree = parser.parse(code, None).unwrap();
    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).unwrap();

    println!("=== AST Statements ===");
    for (i, stmt) in ast_result.program.statements.iter().enumerate() {
        println!("  [{}] {:?}", i, stmt);
    }

    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let signature_index = SignatureIndex::new();
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    ).unwrap();

    println!("\n=== IR nodes: {} ===", ir.nodes.len());
    for (i, node) in ir.nodes.iter().enumerate() {
        println!("  [{}] {:?}", i, node.kind);
    }

    let has_member_access = ir.nodes.iter().any(|n| {
        matches!(n.kind, bsl_shared::ir::SemanticNodeKind::MemberAccess { .. })
    });

    println!("\nHas MemberAccess: {}", has_member_access);
    assert!(has_member_access, "Should have MemberAccess for Документы.ЗаказКлиента");
}
