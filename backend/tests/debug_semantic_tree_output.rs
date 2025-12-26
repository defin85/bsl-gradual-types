//! Debug test to see actual semantic tree output structure

#[cfg(test)]
mod debug_semantic_tree {
    use bsl_backend::system::system_coordinator::SystemCoordinator;
    use std::path::Path;

    #[tokio::test]
    #[ignore] // Run manually with: cargo test -p bsl-backend --test debug_semantic_tree_output -- --ignored --nocapture
    async fn debug_simple_procedure_output() {
        let coordinator = SystemCoordinator::new();
        let syntax_helper_path = Path::new("examples/syntax_helper");
        coordinator
            .start_with_paths(Some(syntax_helper_path), None, None, None)
            .await
            .expect("Failed to start coordinator");

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = r#"
Процедура ТестоваяПроцедура()
    Сообщить("Привет");
КонецПроцедуры
"#;

        let result = service
            .get_semantic_tree(code, "test.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        println!("\n=== SEMANTIC TREE DEBUG ===");
        println!("File path: {}", result.file_path);
        println!("Root nodes count: {}", result.root_nodes.len());

        println!("\n=== ROOT NODES ===");
        for (i, node) in result.root_nodes.iter().enumerate() {
            println!(
                "Node {}: kind='{}', name={:?}, children={}",
                i,
                node.kind,
                node.name,
                node.children.len()
            );
            println!("  Location: {:?}", node.location);
            println!("  Attributes: {:?}", node.attributes);
        }

        println!("\n=== SYMBOL TABLE ===");
        println!("Symbol count: {}", result.symbol_table.len());
        for (name, symbol) in result.symbol_table.iter() {
            println!(
                "  Symbol '{}': kind={}, scope={}",
                name, symbol.kind, symbol.scope
            );
        }

        println!("\n=== METRICS ===");
        println!("{:#?}", result.metrics);

        println!("\n=== CALL GRAPH ===");
        println!("Calls count: {}", result.call_graph.len());

        println!("\n=== TIMESTAMP ===");
        println!("{:?}", result.timestamp);
    }
}
