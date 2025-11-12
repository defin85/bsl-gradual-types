//! Минимальный тест для проверки IR генерации

use bsl_backend::application::ast_to_ir::AstToIrConverter;
use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use std::sync::Arc;
use tree_sitter::Parser;

fn main() {
    let code1 = "ТЗ = Новый ТаблицаЗначений;\nТЗ.НесуществующийМетод();";

    let code2 = r#"
Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.НесуществующийМетод();
КонецПроцедуры
"#;

    println!("=== TEST 1: Код ВНЕ функции ===");
    test_code(code1);

    println!("\n=== TEST 2: Код ВНУТРИ функции ===");
    test_code(code2);
}

fn test_code(code: &str) {
    // Парсинг через tree-sitter
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("Failed to set language");

    let tree = parser.parse(code, None).expect("Failed to parse");
    println!("Tree parsed: {} nodes", tree.root_node().descendant_count());

    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).expect("Failed to convert AST");
    println!("AST statements: {}", ast_result.program.statements.len());

    // Конвертация AST → IR
    let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository,
    )
    .expect("Failed to convert to IR");

    println!("IR nodes: {}", ir.nodes.len());
    for (i, node) in ir.nodes.iter().enumerate() {
        println!("  [{}] {:?}", i, node.kind);
    }
}
