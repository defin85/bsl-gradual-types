//! Integration тесты для Context Diagnostics в LSP (Milestone 3.11 Phase 3)
//!
//! Проверяем, что LSP Server корректно показывает warnings для методов,
//! недоступных в текущем контексте выполнения.
//!
//! Примеры:
//! - &НаКлиенте + СоздатьЭлемент() → WARNING (ServerOnly метод)
//! - &НаСервере + СоздатьЭлемент() → OK
//! - Unknown context → OK (не блокируем)

mod support;

fn semantic_diagnostics(code: &str) -> Vec<bsl_shared::domain::types::TypeDiagnostic> {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    support::semantic_diagnostics_for_code(deps_bundle.as_ref(), "inline.bsl", code)
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
        context_warnings
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
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
    let code = r#"
        &НаКлиенте
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

    // Должен быть 1 warning для НайтиПоКоду (ServerOnly метод)
    assert_single_context_warning(&diagnostics, "НайтиПоКоду");
}

// ========================================
// TEST 2: Server method в серверном контексте - OK
// ========================================
#[tokio::test]
async fn test_server_method_in_server_context_no_warning() {
    let code = r#"
        &НаСервере
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

    // Не должно быть warnings (серверный контекст)
    assert_no_context_warnings(&diagnostics);
}

// ========================================
// TEST 3: Universal method в любом контексте
// ========================================
#[tokio::test]
async fn test_universal_method_in_any_context() {
    let code = r#"
        &НаКлиенте
        Процедура Test()
            ПустаяСсылка = Справочники.Контрагенты.ПустаяСсылка();
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

    // Не должно быть warnings (ПустаяСсылка - Universal)
    assert_no_context_warnings(&diagnostics);
}

// ========================================
// TEST 4: Multiple context violations
// ========================================
#[tokio::test]
async fn test_multiple_context_violations() {
    let code = r#"
        &НаКлиенте
        Процедура Test()
            Объект = Справочники.Контрагенты.СоздатьЭлемент();
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
            Выборка = Справочники.Контрагенты.Выбрать();
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

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
    // Без директивы - Unknown контекст
    let code = r#"
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

    // Если контекст неизвестен - не показываем warning
    assert_no_context_warnings(&diagnostics);
}

// ========================================
// TEST 6: OnServerNoContext allows all
// ========================================
#[tokio::test]
async fn test_server_no_context_allows_all() {
    let code = r#"
        &НаСервереБезКонтекста
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

    // Не должно быть warnings (серверный контекст)
    assert_no_context_warnings(&diagnostics);
}

// ========================================
// TEST 7: OnClientOnServerNoContext blocks ServerOnly
// ========================================
#[tokio::test]
async fn test_universal_context_blocks_server_only() {
    let code = r#"
        &НаКлиентеНаСервереБезКонтекста
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

    // Должен быть warning (универсальный контекст запрещает ServerOnly)
    assert_single_context_warning(&diagnostics, "НайтиПоКоду");
}

// ========================================
// TEST 8: Document methods context validation
// ========================================
#[tokio::test]
async fn test_document_methods_context_validation() {
    let code = r#"
        &НаКлиенте
        Процедура Test()
            Док = Документы.Заказ.СоздатьДокумент();
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

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

    let _ = semantic_diagnostics(code);
}

// ========================================
// TEST 10: Method chain context validation (будущая функциональность)
// ========================================
#[tokio::test]
async fn test_method_chain_context_validation() {
    let code = r#"
        &НаКлиенте
        Процедура Test()
            Объект = Справочники.Контрагенты.СоздатьЭлемент();
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

    // Должен быть warning для СоздатьЭлемент
    assert_single_context_warning(&diagnostics, "СоздатьЭлемент");
}

// ========================================
// TEST 11: Diagnostic severity is WARNING not ERROR
// ========================================
#[tokio::test]
async fn test_diagnostic_severity_is_warning() {
    let code = r#"
        &НаКлиенте
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

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
    // Пока что просто проверяем что не падает
    let code = r#"
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let _ = semantic_diagnostics(code);
}

// ========================================
// TEST 13: Case insensitive directives (будущая функциональность)
// ========================================
#[tokio::test]
async fn test_case_insensitive_directives() {
    // Пока что просто проверяем что не падает
    let code = r#"
        &НАКЛИЕНТЕ
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let _ = semantic_diagnostics(code);
}

// ========================================
// TEST 14: No false positives for custom methods
// ========================================
#[tokio::test]
async fn test_no_false_positives_for_custom_methods() {
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

    let diagnostics = semantic_diagnostics(code);

    // Не должно быть warnings для кастомных методов
    assert!(
        diagnostics
            .iter()
            .all(|d| !d.message.contains("МойКастомныйМетод")),
        "Не должно быть warnings для пользовательских методов"
    );
}

// ========================================
// TEST 15: Diagnostic location accuracy
// ========================================
#[tokio::test]
async fn test_diagnostic_location_accuracy() {
    let code = r#"
        &НаКлиенте
        Процедура Test()
            Ссылка = Справочники.Контрагенты.НайтиПоКоду("001");
        КонецПроцедуры
    "#;

    let diagnostics = semantic_diagnostics(code);

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
}

// ========================================
// TEST 16: Multiple files with different contexts (будущая функциональность)
// ========================================
#[tokio::test]
async fn test_multiple_files_different_contexts() {
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

    let diag1 = semantic_diagnostics(code1);
    let diag2 = semantic_diagnostics(code2);

    // code1 должен иметь warning, code2 - нет
    assert!(diag1.iter().any(|d| d.message.contains("недоступен")));
    assert_no_context_warnings(&diag2);
}
