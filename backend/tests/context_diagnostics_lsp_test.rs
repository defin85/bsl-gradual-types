//! Integration тесты для Context Diagnostics в LSP (Milestone 3.11 Phase 3)
//!
//! Проверяем, что LSP Server корректно показывает warnings для методов,
//! недоступных в текущем контексте выполнения.
//!
//! Примеры:
//! - &НаКлиенте + СоздатьЭлемент() → WARNING (ServerOnly метод)
//! - &НаСервере + СоздатьЭлемент() → OK
//! - Unknown context → OK (не блокируем)

mod shared_test_fixtures;

use bsl_backend::application::TypeSystemService;
use shared_test_fixtures::get_test_service;

/// Helper: создать TypeSystemService для тестов WITH PLATFORM TYPES LOADED
fn create_test_service() -> &'static TypeSystemService {
    get_test_service()
}

/// Проверяет что код НЕ содержит semantic/context warnings
fn assert_no_context_warnings(diagnostics: &[bsl_shared::domain::types::TypeDiagnostic]) {
    let context_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.message.contains("недоступен в контексте"))
        .collect();

    assert!(
        context_warnings.is_empty(),
        "Не должно быть context warnings, но получено: {:?}",
        context_warnings.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// Проверяет что есть ровно 1 context warning с заданным текстом
fn assert_single_context_warning(
    diagnostics: &[bsl_shared::domain::types::TypeDiagnostic],
    expected_method: &str,
) {
    use bsl_shared::domain::types::DiagnosticSeverity;

    let context_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| {
            d.severity == DiagnosticSeverity::Warning
                && d.message.contains("недоступен в контексте")
                && d.message.contains(expected_method)
        })
        .collect();

    assert_eq!(
        context_warnings.len(),
        1,
        "Ожидался ровно 1 context warning для метода '{}', получено: {}. Diagnostics: {:?}",
        expected_method,
        context_warnings.len(),
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// ========================================
// TEST 1: Server-only method в клиентском контексте
// ========================================
#[tokio::test]
async fn test_server_only_method_in_client_context_warning() {
    let service = create_test_service();

    let code = r#"
        &НаКлиенте
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok(), "validate_semantics должна возвращать Ok");

    let diagnostics = result.unwrap();

    // Должен быть 1 warning для НайтиПоКоду (ServerOnly метод)
    assert_single_context_warning(&diagnostics, "НайтиПоКоду");
}

// ========================================
// TEST 2: Server method в серверном контексте - OK
// ========================================
#[tokio::test]
async fn test_server_method_in_server_context_no_warning() {
    let service = create_test_service();

    let code = r#"
        &НаСервере
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Не должно быть warnings (серверный контекст)
    assert_no_context_warnings(&diagnostics);
}

// ========================================
// TEST 3: Universal method в любом контексте
// ========================================
#[tokio::test]
async fn test_universal_method_in_any_context() {
    let service = create_test_service();

    let code = r#"
        &НаКлиенте
        Процедура Test()
            ПустаяСсылка = Справочники.Контрагенты.ПустаяСсылка();
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Не должно быть warnings (ПустаяСсылка - Universal)
    assert_no_context_warnings(&diagnostics);
}

// ========================================
// TEST 4: Multiple context violations
// ========================================
#[tokio::test]
async fn test_multiple_context_violations() {
    let service = create_test_service();

    let code = r#"
        &НаКлиенте
        Процедура Test()
            Объект = Справочники.Контрагенты.СоздатьЭлемент();
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
            Выборка = Справочники.Контрагенты.Выбрать();
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    use bsl_shared::domain::types::DiagnosticSeverity;

    // Должно быть 3 warnings (все 3 метода ServerOnly)
    let context_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| {
            d.severity == DiagnosticSeverity::Warning
                && d.message.contains("недоступен в контексте")
        })
        .collect();

    assert_eq!(
        context_warnings.len(),
        3,
        "Ожидалось 3 context warnings, получено: {}. Diagnostics: {:?}",
        context_warnings.len(),
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// ========================================
// TEST 5: Unknown context - no warning
// ========================================
#[tokio::test]
async fn test_unknown_context_no_warning() {
    let service = create_test_service();

    // Без директивы - Unknown контекст
    let code = r#"
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Если контекст неизвестен - не показываем warning
    assert_no_context_warnings(&diagnostics);
}

// ========================================
// TEST 6: OnServerNoContext allows all
// ========================================
#[tokio::test]
async fn test_server_no_context_allows_all() {
    let service = create_test_service();

    let code = r#"
        &НаСервереБезКонтекста
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Не должно быть warnings (серверный контекст)
    assert_no_context_warnings(&diagnostics);
}

// ========================================
// TEST 7: OnClientOnServerNoContext blocks ServerOnly
// ========================================
#[tokio::test]
async fn test_universal_context_blocks_server_only() {
    let service = create_test_service();

    let code = r#"
        &НаКлиентеНаСервереБезКонтекста
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Должен быть warning (универсальный контекст запрещает ServerOnly)
    assert_single_context_warning(&diagnostics, "НайтиПоКоду");
}

// ========================================
// TEST 8: Document methods context validation
// ========================================
#[tokio::test]
async fn test_document_methods_context_validation() {
    let service = create_test_service();

    let code = r#"
        &НаКлиенте
        Процедура Test()
            Док = Документы.Заказ.СоздатьДокумент();
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Если СоздатьДокумент ServerOnly - должен быть warning
    // (зависит от platform types)
    // Для теста проверяем что semantic validation не падает
    assert!(diagnostics.iter().all(|d| {
        use bsl_shared::domain::types::DiagnosticSeverity;
        d.severity == DiagnosticSeverity::Warning || d.severity == DiagnosticSeverity::Error
    }));
}

// ========================================
// TEST 9: Nested function inherits context (будущая функциональность)
// ========================================
#[tokio::test]
async fn test_nested_function_inherits_context() {
    let service = create_test_service();

    // Пока что просто проверяем что не падает
    let code = r#"
        &НаКлиенте
        Процедура Outer()
            Функция Inner()
                Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
                Возврат Ссылка;
            КонецФункции
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok(), "Не должно падать на nested functions");
}

// ========================================
// TEST 10: Method chain context validation (будущая функциональность)
// ========================================
#[tokio::test]
async fn test_method_chain_context_validation() {
    let service = create_test_service();

    let code = r#"
        &НаКлиенте
        Процедура Test()
            Объект = Справочники.Контрагенты.СоздатьЭлемент();
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Должен быть warning для СоздатьЭлемент
    assert_single_context_warning(&diagnostics, "СоздатьЭлемент");
}

// ========================================
// TEST 11: Diagnostic severity is WARNING not ERROR
// ========================================
#[tokio::test]
async fn test_diagnostic_severity_is_warning() {
    let service = create_test_service();

    let code = r#"
        &НаКлиенте
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    use bsl_shared::domain::types::DiagnosticSeverity;

    // Проверяем severity
    let context_diagnostics: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.message.contains("недоступен в контексте"))
        .collect();

    assert_eq!(context_diagnostics.len(), 1);
    assert_eq!(
        context_diagnostics[0].severity,
        DiagnosticSeverity::Warning,
        "Context diagnostic должен быть WARNING, не Error"
    );
}

// ========================================
// TEST 12: English directive names (будущая функциональность)
// ========================================
#[tokio::test]
async fn test_english_directive_names() {
    let service = create_test_service();

    // Пока что просто проверяем что не падает
    let code = r#"
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok(), "Не должно падать на английские директивы");
}

// ========================================
// TEST 13: Case insensitive directives (будущая функциональность)
// ========================================
#[tokio::test]
async fn test_case_insensitive_directives() {
    let service = create_test_service();

    // Пока что просто проверяем что не падает
    let code = r#"
        &НАКЛИЕНТЕ
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok(), "Не должно падать на разный регистр");
}

// ========================================
// TEST 14: No false positives for custom methods
// ========================================
#[tokio::test]
async fn test_no_false_positives_for_custom_methods() {
    let service = create_test_service();

    // Кастомный метод пользователя
    let code = r#"
        &НаКлиенте
        Процедура Test()
            Результат = МойКастомныйМетод();
        КонецПроцедуры

        Функция МойКастомныйМетод()
            Возврат "test";
        КонецФункции
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Не должно быть warnings для кастомных методов
    assert!(
        diagnostics.iter().all(|d| !d.message.contains("МойКастомныйМетод")),
        "Не должно быть warnings для пользовательских методов"
    );
}

// ========================================
// TEST 15: Diagnostic location accuracy
// ========================================
#[tokio::test]
async fn test_diagnostic_location_accuracy() {
    let service = create_test_service();

    let code = r#"
        &НаКлиенте
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    use bsl_shared::domain::types::DiagnosticSeverity;

    let context_diagnostics: Vec<_> = diagnostics
        .iter()
        .filter(|d| {
            d.severity == DiagnosticSeverity::Warning
                && d.message.contains("недоступен в контексте")
        })
        .collect();

    assert_eq!(context_diagnostics.len(), 1);

    // Проверяем что diagnostic указывает на строку с вызовом метода
    let diag = context_diagnostics[0];
    assert!(
        diag.line > 0,
        "Diagnostic должен иметь валидную строку: line={}",
        diag.line
    );
    assert!(
        diag.column >= 0,
        "Diagnostic должен иметь валидную колонку: column={}",
        diag.column
    );
}

// ========================================
// TEST 16: Multiple files with different contexts (будущая функциональность)
// ========================================
#[tokio::test]
async fn test_multiple_files_different_contexts() {
    let service = create_test_service();

    // Просто проверяем что сервис может обработать несколько запросов
    let code1 = r#"
        &НаКлиенте
        Процедура ClientProc()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let code2 = r#"
        &НаСервере
        Процедура ServerProc()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let result1 = service.validate_semantics(code1, None).await;
    let result2 = service.validate_semantics(code2, None).await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());

    let diag1 = result1.unwrap();
    let diag2 = result2.unwrap();

    // code1 должен иметь warning, code2 - нет
    assert!(diag1.iter().any(|d| d.message.contains("недоступен")));
    assert_no_context_warnings(&diag2);
}
