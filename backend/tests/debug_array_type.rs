use bsl_backend::system::system_coordinator::SystemCoordinator;
use std::path::Path;

#[tokio::test]
async fn debug_array_type_resolution() {
    // Инициализируем SystemCoordinator с Syntax Helper
    let coordinator = SystemCoordinator::new();
    let syntax_path = Path::new("../examples/syntax_helper");

    coordinator
        .start_with_paths(Some(syntax_path), None, None, None)
        .await
        .expect("Failed to start SystemCoordinator");

    let type_service = coordinator
        .type_service()
        .expect("TypeSystemService not available");

    // Тестируем hover для переменной типа Массив
    let source = r#"
Функция Тест()
    Перем МассивЗначений;
    МассивЗначений = Новый Массив;
    Возврат МассивЗначений;
КонецФункции
"#;

    println!("=== Testing hover for Array type ===");
    let hover_result = type_service
        .get_hover_info(source, 3, 10, None)
        .await
        .expect("get_hover_info failed");

    if let Some(hover_text) = hover_result {
        println!("\n✅ Hover text:\n{}", hover_text);

        // Проверяем, что hover НЕ содержит ошибку "Тип не найден"
        if hover_text.contains("⚠️") && hover_text.contains("Тип не найден в TypeRepository")
        {
            println!("\n❌ ERROR: Hover shows 'Type not found' warning for Array type!");
            println!("This means either:");
            println!("  1. TypeResolver returns wrong ConcreteType");
            println!("  2. extract_type_name() returns None");
            println!("  3. repository.find_type() returns None");
            panic!("Array type should be found in TypeRepository!");
        } else {
            println!("\n✅ Hover correctly shows Array type information");
        }
    } else {
        panic!("Hover returned None!");
    }
}
