use bsl_backend::system::SystemCoordinator;
use std::path::Path;

#[tokio::test]
async fn test_method_return_certainty() {
    let coordinator = SystemCoordinator::new();
    let syntax_helper_path = Path::new("examples/syntax_helper");
    coordinator
        .start_with_paths(Some(syntax_helper_path), None, None)
        .await
        .expect("Failed to start coordinator");

    let service = coordinator
        .type_service()
        .expect("Failed to get TypeSystemService");

    let code = r#"
ТЗ = Новый ТаблицаЗначений;
Результат = ТЗ.Выгрузить();
"#;

    let result = service
        .get_semantic_tree(code, "test.bsl", false, true, true)
        .await
        .expect("Failed to get semantic tree");

    println!("\n=== Symbol Table ===");
    for (var, info) in &result.symbol_table {
        if let Some(ref type_res) = info.resolved_type {
            println!("\n{}: {} ({}%)", var, type_res.certainty, type_res.certainty_percent);
        } else {
            println!("\n{}: No type", var);
        }
    }

    // Проверяем что у Результат certainty = Known (100%)
    let rezultat = result.symbol_table.get("Результат").expect("Результат not found");
    let type_res = rezultat.resolved_type.as_ref().expect("Результат has no type");

    println!("\nРезультат type: {}", type_res.name);
    println!("Результат certainty: {} ({}%)", type_res.certainty, type_res.certainty_percent);
    println!("Expected: Known (100%)");

    assert_eq!(
        type_res.certainty,
        "Known",
        "Method return should have Known certainty"
    );
    assert_eq!(
        type_res.certainty_percent,
        100,
        "Known certainty should be 100%"
    );
}
