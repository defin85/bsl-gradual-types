//! Diagnostic test to inspect AST from TreeSitterAdapter

use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use tree_sitter::Parser;

#[test]
fn debug_ast_from_adapter() {
    let code = std::fs::read_to_string("C:\\1CProject\\bsl-gradual-types\\test_hover_milestone_2_11.bsl")
        .expect("Failed to read test file");

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_bsl::LANGUAGE.into()).expect("Failed to set language");

    let tree = parser.parse(&code, None).expect("Failed to parse");

    // Используем TreeSitterAdapter для получения AST
    let parse_result = TreeSitterAdapter::convert_tree(&tree, &code).expect("Failed to convert tree");

    println!("\n=== AST FROM TREE_SITTER_ADAPTER ===");
    println!("Total statements: {}", parse_result.program.statements.len());

    for (i, stmt) in parse_result.program.statements.iter().enumerate() {
        match stmt {
            bsl_backend::parsing::bsl::ast::Statement::VarDeclaration { name, .. } => {
                println!("[{}] VarDeclaration: {}", i, name);
            }
            bsl_backend::parsing::bsl::ast::Statement::FunctionDecl { name, body, .. } => {
                println!("[{}] FunctionDecl: {}", i, name);
                println!("    Body statements: {}", body.len());

                // Проверяем первые 10 statements в теле функции
                for (j, body_stmt) in body.iter().take(10).enumerate() {
                    if let bsl_backend::parsing::bsl::ast::Statement::VarDeclaration { name: var_name, .. } = body_stmt {
                        println!("      [{}] VarDeclaration: {}", j, var_name);
                    }
                }
                if body.len() > 10 {
                    println!("      ... and {} more statements", body.len() - 10);
                }
            }
            _ => {
                println!("[{}] {:?}", i, std::mem::discriminant(stmt));
            }
        }
    }
}
