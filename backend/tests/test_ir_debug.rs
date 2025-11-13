//! DEBUG тест для проверки IR генерации

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use bsl_shared::domain::signature_index::SignatureIndex;
use std::sync::Arc;
use tree_sitter::Parser;

#[test]
fn test_ir_outside_function() {
    let code = "ТЗ = Новый ТаблицаЗначений;\nТЗ.НесуществующийМетод();";

    println!("\n=== CODE ===\n{}\n", code);

    // Парсинг
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_bsl::LANGUAGE.into()).unwrap();
    let tree = parser.parse(code, None).unwrap();

    println!("Tree-sitter nodes: {}", tree.root_node().descendant_count());

    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).unwrap();
    println!("AST statements: {}", ast_result.program.statements.len());

    for (i, stmt) in ast_result.program.statements.iter().enumerate() {
        println!("  Statement[{}]: {:?}", i, std::mem::discriminant(stmt));
    }

    // IR конвертация
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let signature_index = SignatureIndex::new();
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    ).unwrap();

    println!("\nIR nodes: {}", ir.nodes.len());
    for (i, node) in ir.nodes.iter().enumerate() {
        println!("  [{}] {:?} at [{},{}..{},{}]",
            i,
            node.kind,
            node.span.start_line,
            node.span.start_column,
            node.span.end_line,
            node.span.end_column
        );
    }

    // Проверяем наличие MemberAccess
    let has_member_access = ir.nodes.iter().any(|n| {
        matches!(n.kind, bsl_shared::ir::SemanticNodeKind::MemberAccess { .. })
    });

    println!("\nHas MemberAccess nodes: {}", has_member_access);
}

#[test]
fn test_ir_inside_function() {
    let code = r#"
Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.НесуществующийМетод();
КонецПроцедуры
"#;

    println!("\n=== CODE ===\n{}\n", code);

    // Парсинг
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_bsl::LANGUAGE.into()).unwrap();
    let tree = parser.parse(code, None).unwrap();

    println!("Tree-sitter nodes: {}", tree.root_node().descendant_count());

    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).unwrap();
    println!("AST statements: {}", ast_result.program.statements.len());

    // IR конвертация
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let signature_index = SignatureIndex::new();
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    ).unwrap();

    println!("\nIR nodes: {}", ir.nodes.len());
    for (i, node) in ir.nodes.iter().enumerate() {
        println!("  [{}] {:?} at [{},{}..{},{}]",
            i,
            node.kind,
            node.span.start_line,
            node.span.start_column,
            node.span.end_line,
            node.span.end_column
        );
    }

    // Проверяем наличие MemberAccess
    let has_member_access = ir.nodes.iter().any(|n| {
        matches!(n.kind, bsl_shared::ir::SemanticNodeKind::MemberAccess { .. })
    });

    println!("\nHas MemberAccess nodes: {}", has_member_access);
}
