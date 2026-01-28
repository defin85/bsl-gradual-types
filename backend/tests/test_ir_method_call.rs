//! Тест для проверки IR генерации для вызова метода

use bsl_analysis_v2::AstToIrConverter;
use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use bsl_shared::domain::signature_index::SignatureIndex;
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
    let signature_index = SignatureIndex::new();
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository,
        signature_index,
    )
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
            // Phase 3: value_type теперь TypeResolution, используем type_name()
            println!(
                "    Assignment: {} = {} | value_node={:?}",
                variable,
                value_type.type_name(),
                value_node
            );
        }
    }
}
