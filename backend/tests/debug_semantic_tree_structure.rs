//! Diagnostic test to debug semantic tree structure

use bsl_backend::system::SystemCoordinator;
use std::sync::Arc;

#[tokio::test]
#[ignore = "Debug test requires specific Windows file path"]
async fn debug_test_hover_file_structure() {
    // Читаем реальный файл
    let code =
        std::fs::read_to_string("C:\\1CProject\\bsl-gradual-types\\test_hover_milestone_2_11.bsl")
            .expect("Failed to read test file");

    // Создаём coordinator
    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator
        .start()
        .await
        .expect("Failed to start coordinator");

    let type_service = coordinator.type_service().expect("No type service");

    // Получаем SemanticProgram через TypeSystemService
    let dto = type_service
        .get_semantic_tree(&code, "test_hover_milestone_2_11.bsl", false, true, true)
        .await
        .expect("Failed to get semantic tree");

    println!("\n=== SEMANTIC TREE STRUCTURE ===");
    println!("Total root nodes: {}", dto.root_nodes.len());
    println!("\nRoot nodes:");

    for (i, node) in dto.root_nodes.iter().enumerate() {
        println!("\n[{}] {} {:?}", i, node.kind, node.name);
        println!("    Children: {}", node.children.len());

        if !node.children.is_empty() {
            println!("    First 5 children:");
            for (j, child) in node.children.iter().take(5).enumerate() {
                println!("      [{}] {} {:?}", j, child.kind, child.name);
            }
            if node.children.len() > 5 {
                println!("      ... and {} more", node.children.len() - 5);
            }
        }
    }

    println!("\n=== SYMBOL TABLE ===");
    println!("Total symbols: {}", dto.symbol_table.len());

    // Проверяем дублирование
    let var_count = dto
        .root_nodes
        .iter()
        .filter(|n| n.kind == "Variable")
        .count();

    println!("\nVariables at root level: {}", var_count);

    if var_count > 0 {
        println!("\n⚠️ WARNING: Variables found at root level!");
        println!("Expected: All variables should be inside Function node");
    }

    // Проверяем функцию
    let func_nodes: Vec<_> = dto
        .root_nodes
        .iter()
        .filter(|n| n.kind == "Function")
        .collect();

    println!("\nFunctions at root level: {}", func_nodes.len());

    for func in func_nodes {
        println!("\nFunction {:?}:", func.name);
        println!("  Children: {}", func.children.len());

        let var_in_func = func
            .children
            .iter()
            .filter(|n| n.kind == "Variable")
            .count();
        println!("  Variables inside: {}", var_in_func);
    }
}
