//! Тесты для валидации конкатенации строк

mod shared_test_fixtures;

use bsl_backend::application::TypeSystemService;
use bsl_shared::domain::types::DiagnosticSeverity;
use shared_test_fixtures::get_test_service;

fn create_test_service() -> &'static TypeSystemService {
    get_test_service()
}

#[tokio::test]
async fn test_invalid_string_concat_reports_error() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    Текст = "текст" + 1;
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let concat_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Конкатенация строк"))
        .collect();

    assert!(
        !concat_errors.is_empty(),
        "Должна быть ошибка для конкатенации строк с не-строкой"
    );
    assert_eq!(concat_errors[0].severity, DiagnosticSeverity::Error);
}
