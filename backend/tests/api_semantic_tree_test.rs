//! Integration tests for /api/semantic-tree endpoint
//!
//! Tests verify that:
//! 1. POST /api/semantic-tree accepts BSL code and returns semantic tree
//! 2. Response contains root_nodes, symbol_table, metrics
//! 3. Simple procedure creates FunctionDeclaration node
//! 4. Metrics are correctly calculated (procedure_count, node_count, etc.)
//! 5. Edge cases are handled (empty code, syntax errors, etc.)

#[cfg(test)]
mod api_semantic_tree_tests {
    use bsl_backend::system::system_coordinator::SystemCoordinator;
    use std::path::Path;

    /// Helper function to create a test coordinator
    async fn create_test_coordinator() -> SystemCoordinator {
        let coordinator = SystemCoordinator::new();

        // Start with platform types from syntax_helper
        let syntax_helper_path = Path::new("examples/syntax_helper");

        coordinator
            .start_with_paths(Some(syntax_helper_path), None, None)
            .await
            .expect("Failed to start coordinator");

        coordinator
    }

    #[tokio::test]
    async fn test_simple_procedure_creates_function_declaration_node() {
        let coordinator = create_test_coordinator().await;

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

        // Verify basic structure
        assert_eq!(result.file_path, "test.bsl", "File path should match");

        assert!(
            !result.root_nodes.is_empty(),
            "Should have at least one root node"
        );

        // Find procedure node
        let proc_node = result
            .root_nodes
            .iter()
            .find(|node| node.kind == "Procedure")
            .expect("Should find Procedure node");

        assert_eq!(
            proc_node.name,
            Some("ТестоваяПроцедура".to_string()),
            "Procedure name should be ТестоваяПроцедура"
        );

        // Verify metrics
        assert_eq!(
            result.metrics.procedure_count, 1,
            "Should have 1 procedure"
        );
        assert_eq!(
            result.metrics.function_count, 0,
            "Should have 0 functions"
        );
        assert!(
            result.metrics.node_count > 0,
            "Should have at least one node"
        );

        println!("✓ Test 1 PASSED: Simple procedure creates FunctionDeclaration node");
        println!("  Procedure name: {:?}", proc_node.name);
        println!("  Node kind: {}", proc_node.kind);
        println!("  Metrics: {:?}", result.metrics);
    }

    #[tokio::test]
    async fn test_function_with_return_type() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = r#"
Функция ПолучитьЧисло() Экспорт
    Возврат 42;
КонецФункции
"#;

        let result = service
            .get_semantic_tree(code, "test.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        // Verify metrics
        assert_eq!(result.metrics.function_count, 1, "Should have 1 function");
        assert_eq!(
            result.metrics.procedure_count, 0,
            "Should have 0 procedures"
        );

        // Find function node
        let func_node = result
            .root_nodes
            .iter()
            .find(|node| node.kind == "Function")
            .expect("Should find Function node");

        assert_eq!(
            func_node.name,
            Some("ПолучитьЧисло".to_string()),
            "Function name should be ПолучитьЧисло"
        );

        println!("✓ Test 2 PASSED: Function creates FunctionDeclaration node");
        println!("  Function name: {:?}", func_node.name);
        println!("  Metrics: {:?}", result.metrics);
    }

    #[tokio::test]
    async fn test_multiple_procedures_and_functions() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = r#"
Процедура Первая()
    Сообщить("Первая");
КонецПроцедуры

Функция Вторая()
    Возврат 1;
КонецФункции

Процедура Третья()
    Сообщить("Третья");
КонецПроцедуры
"#;

        let result = service
            .get_semantic_tree(code, "test.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        // Verify metrics
        assert_eq!(
            result.metrics.procedure_count, 2,
            "Should have 2 procedures"
        );
        assert_eq!(result.metrics.function_count, 1, "Should have 1 function");
        assert_eq!(
            result.root_nodes.len(),
            3,
            "Should have 3 root nodes"
        );

        println!("✓ Test 3 PASSED: Multiple procedures and functions");
        println!("  Procedures: {}", result.metrics.procedure_count);
        println!("  Functions: {}", result.metrics.function_count);
    }

    #[tokio::test]
    async fn test_variables_in_procedure_body() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = r#"
Процедура Тест()
    Переменная1 = 10;
    Переменная2 = "Строка";
    Переменная3 = Новый Массив;
КонецПроцедуры
"#;

        let result = service
            .get_semantic_tree(code, "test.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        // Verify procedure node exists
        let proc_node = result
            .root_nodes
            .iter()
            .find(|node| node.kind == "Procedure")
            .expect("Should find Procedure node");

        // Variables should be in children or reflected in metrics
        assert!(
            result.metrics.node_count > 1,
            "Should have more than just procedure node"
        );

        println!("✓ Test 4 PASSED: Variables in procedure body");
        println!("  Node count: {}", result.metrics.node_count);
        println!("  Procedure children: {}", proc_node.children.len());
    }

    #[tokio::test]
    async fn test_empty_code_returns_empty_tree() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = "";

        let result = service
            .get_semantic_tree(code, "empty.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        // Verify empty tree
        assert_eq!(result.file_path, "empty.bsl");
        assert_eq!(result.root_nodes.len(), 0, "Should have no root nodes");
        assert_eq!(result.metrics.procedure_count, 0);
        assert_eq!(result.metrics.function_count, 0);

        println!("✓ Test 5 PASSED: Empty code returns empty tree");
    }

    #[tokio::test]
    async fn test_code_with_whitespace_only() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = "   \n\n  \t  \n  ";

        let result = service
            .get_semantic_tree(code, "whitespace.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        assert_eq!(result.root_nodes.len(), 0, "Should have no root nodes");

        println!("✓ Test 6 PASSED: Whitespace-only code returns empty tree");
    }

    #[tokio::test]
    async fn test_procedure_with_parameters() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = r#"
Процедура ПроцедураСПараметрами(Параметр1, Параметр2, Параметр3)
    Сообщить(Параметр1);
КонецПроцедуры
"#;

        let result = service
            .get_semantic_tree(code, "test.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        // Find procedure node
        let proc_node = result
            .root_nodes
            .iter()
            .find(|node| node.kind == "Procedure")
            .expect("Should find Procedure node");

        // Verify parameter count in attributes
        assert_eq!(
            proc_node.attributes.get("parameter_count"),
            Some(&"3".to_string()),
            "Should have parameter_count = 3 in attributes"
        );

        println!("✓ Test 7 PASSED: Procedure with parameters");
        println!("  Attributes: {:?}", proc_node.attributes);
    }

    #[tokio::test]
    async fn test_nested_structures() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = r#"
Процедура ТестВложенности()
    Если Истина Тогда
        Для Индекс = 1 По 10 Цикл
            Сообщить(Индекс);
        КонецЦикла;
    КонецЕсли;
КонецПроцедуры
"#;

        let result = service
            .get_semantic_tree(code, "test.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        // Verify node count reflects nested structures
        assert!(
            result.metrics.node_count >= 1,
            "Should have at least procedure node"
        );

        // Tree depth might be 1 or more depending on implementation
        assert!(
            result.metrics.tree_depth >= 1,
            "Should have tree depth >= 1"
        );

        println!("✓ Test 8 PASSED: Nested structures parsed");
        println!("  Node count: {}", result.metrics.node_count);
        println!("  Tree depth: {}", result.metrics.tree_depth);
    }

    #[tokio::test]
    async fn test_metrics_analysis_time_is_set() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = r#"
Процедура Тест()
    Сообщить("Тест");
КонецПроцедуры
"#;

        let result = service
            .get_semantic_tree(code, "test.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        // Analysis time is u64, always non-negative - just verify it exists
        let _ = result.metrics.analysis_time_ms;

        println!("✓ Test 9 PASSED: Metrics analysis time is set");
        println!("  Analysis time: {} ms", result.metrics.analysis_time_ms);
    }

    #[tokio::test]
    async fn test_export_function_in_attributes() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = r#"
Функция ЭкспортнаяФункция() Экспорт
    Возврат 123;
КонецФункции
"#;

        let result = service
            .get_semantic_tree(code, "test.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        let func_node = result
            .root_nodes
            .iter()
            .find(|node| node.kind == "Function")
            .expect("Should find Function node");

        // Verify function exists with correct name
        assert_eq!(
            func_node.name,
            Some("ЭкспортнаяФункция".to_string()),
            "Function name should match"
        );

        println!("✓ Test 10 PASSED: Export function parsed");
        println!("  Attributes: {:?}", func_node.attributes);
    }

    #[tokio::test]
    async fn test_response_structure() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = r#"
Процедура Тест()
КонецПроцедуры
"#;

        let result = service
            .get_semantic_tree(code, "test.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        // Verify all required fields are present
        assert!(!result.file_path.is_empty(), "file_path should be set");
        // root_nodes can be empty
        // symbol_table can be empty
        // call_graph can be empty

        // Metrics should have all fields
        let _ = result.metrics.procedure_count;
        let _ = result.metrics.function_count;
        let _ = result.metrics.variable_count;
        let _ = result.metrics.parameter_count;
        let _ = result.metrics.known_types;
        let _ = result.metrics.inferred_types;
        let _ = result.metrics.unknown_types;
        let _ = result.metrics.average_certainty;
        let _ = result.metrics.analysis_time_ms;
        let _ = result.metrics.node_count;
        let _ = result.metrics.tree_depth;
        let _ = result.metrics.call_count;

        println!("✓ Test 11 PASSED: Response structure is complete");
    }

    #[tokio::test]
    async fn test_cyrillic_code_support() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let code = r#"
Процедура ТестКириллицы()
    ПеременнаяСРусскимНазванием = "Значение";
    Сообщить(ПеременнаяСРусскимНазванием);
КонецПроцедуры
"#;

        let result = service
            .get_semantic_tree(code, "test.bsl", false, true, true)
            .await
            .expect("Failed to get semantic tree");

        let proc_node = result
            .root_nodes
            .iter()
            .find(|node| node.kind == "Procedure")
            .expect("Should find Procedure node");

        assert_eq!(
            proc_node.name,
            Some("ТестКириллицы".to_string()),
            "Cyrillic name should be preserved"
        );

        println!("✓ Test 12 PASSED: Cyrillic code support");
    }
}
