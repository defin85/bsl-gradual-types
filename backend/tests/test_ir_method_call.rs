//! Тест для проверки IR генерации для вызова метода

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use std::sync::Arc;
use tree_sitter::Parser;

#[test]
fn test_ir_for_method_call_assignment() {
    let code = r#"
Функция Тест()
    ТаблицаЗначенійТип = Новый ТаблицаЗначеній;
    Кол = ТаблицаЗначенійТип.Количество();
КонецФункції
"#;

    // Парсинг через tree-sitter
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("Failed to set language");

    let tree = parser.parse(code, None).expect("Failed to parse");
    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).expect("Failed to convert AST");

    // Конвертация AST → IR
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let metadata =
        Arc::new(bsl_shared::domain::metadata_lookup::TypeMetadataLookup::new(repository.clone()));
    let ir = AstToIrConverter::convert(&ast_result.program, "test.bsl", repository, metadata)
        .expect("Failed to convert to IR");

    println!("\n=== IR NODES ===");
    for (i, node) in ir.nodes.iter().enumerate() {
        println!(
            "[{}] {:?} span=[{},{}..{},{}]",
            i,
            std::mem::discriminant(&node.kind),
            node.span.start_line,
            node.span.start_column,
            node.span.end_line,
            node.span.end_column
        );

        // Детали для FunctionCall
        if let bsl_shared::ir::SemanticNodeKind::FunctionCall {
            function_name,
            object_name,
            object_type,
            ..
        } = &node.kind
        {
            println!(
                "    FunctionCall: {} | object_name={:?} | object_type={:?}",
                function_name, object_name, object_type
            );
        }

        // Детали для Assignment
        if let bsl_shared::ir::SemanticNodeKind::Assignment {
            variable,
            value_type,
            value_node,
        } = &node.kind
        {
            println!(
                "    Assignment: {} = {} | value_node={:?}",
                variable, value_type, value_node
            );
        }
    }
}
