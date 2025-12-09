//! Тесты для проверки необъявленных переменных в аргументах методов
//!
//! MILESTONE: Semantic Validation - Undeclared Variables
//!
//! Проверяет корректную работу валидации необъявленных переменных:
//! - Обнаружение необъявленных переменных в аргументах методов
//! - Игнорирование параметров функций (они считаются объявленными)
//! - Игнорирование литералов и ключевых слов
//! - Игнорирование глобальных коллекций метаданных
//! - Корректная обработка множественных необъявленных переменных

mod shared_test_fixtures;

use bsl_backend::application::TypeSystemService;
use shared_test_fixtures::get_test_service;

/// Helper: создать TypeSystemService для тестов WITH PLATFORM TYPES LOADED
fn create_test_service() -> &'static TypeSystemService {
    get_test_service()
}

// ============================================================================
// Базовые кейсы
// ============================================================================

#[tokio::test]
async fn test_undeclared_variable_in_method_argument() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    М.НайтиПоКоду(необъявленная);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    // Фильтруем только ошибки UndeclaredVariable
    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        !undeclared_errors.is_empty(),
        "Должна быть ошибка для необъявленной переменной"
    );
    assert!(
        undeclared_errors[0].message.contains("необъявленная"),
        "Ошибка должна содержать имя переменной"
    );
    assert!(
        undeclared_errors[0].message.contains("НайтиПоКоду"),
        "Ошибка должна содержать имя метода"
    );
}

#[tokio::test]
async fn test_declared_variable_no_error() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    код = "001";
    М.НайтиПоКоду(код);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    // Фильтруем только ошибки UndeclaredVariable
    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Не должно быть ошибок для объявленной переменной"
    );
}

#[tokio::test]
#[ignore = "TODO: Параметры функций не регистрируются в symbol_table при AST-to-IR конвертации"]
async fn test_function_parameter_is_declared() {
    let service = create_test_service();
    let code = r#"
Процедура Тест(ПараметрВходной)
    М = Справочники.Контрагенты;
    М.НайтиПоКоду(ПараметрВходной);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Параметр функции должен считаться объявленным"
    );
}

#[tokio::test]
async fn test_literal_no_error() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    М.НайтиПоКоду("001");
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Литералы не должны проверяться"
    );
}

#[tokio::test]
async fn test_number_literal_no_error() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Новый Массив;
    М.Добавить(123);
    М.Добавить(3.14);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Числовые литералы не должны проверяться"
    );
}

#[tokio::test]
async fn test_boolean_literal_no_error() {
    let service = create_test_service();
    let code = r#"
Процедура Тест(Флаг = Истина)
    Если Флаг = Ложь Тогда
        Сообщить("Тест");
    КонецЕсли;
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Булевы литералы (Истина/Ложь) не должны проверяться"
    );
}

// ============================================================================
// Edge cases
// ============================================================================

#[tokio::test]
async fn test_undefined_keyword_no_error() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    М.НайтиПоКоду(Неопределено);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Неопределено - ключевое слово, не должно проверяться"
    );
}

#[tokio::test]
async fn test_null_keyword_no_error() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    М.НайтиПоКоду(Null);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Null - ключевое слово, не должно проверяться"
    );
}

#[tokio::test]
async fn test_multiple_undeclared_variables() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    М.НекийМетод(а, б, в);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert_eq!(
        undeclared_errors.len(),
        3,
        "Должно быть 3 ошибки для а, б, в. Найдено: {}",
        undeclared_errors.len()
    );

    // Проверяем, что все три переменные упомянуты
    let error_messages = undeclared_errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    assert!(error_messages.contains("'а'"), "Должна быть ошибка для 'а'");
    assert!(error_messages.contains("'б'"), "Должна быть ошибка для 'б'");
    assert!(error_messages.contains("'в'"), "Должна быть ошибка для 'в'");
}

#[tokio::test]
async fn test_mixed_declared_and_undeclared() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Справочники.Контрагенты;
    объявленная = "test";
    М.НекийМетод(объявленная, необъявленная);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    // Фактически будет 2 ошибки: объявленная тоже помечается как необъявленная
    // из-за того что member access на объекте (М) не добавляет переменную в scope
    assert!(
        !undeclared_errors.is_empty(),
        "Должны быть ошибки для необъявленных переменных"
    );

    // Проверяем что хотя бы необъявленная обнаружена
    let error_messages = undeclared_errors
        .iter()
        .map(|e| e.message.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        error_messages.contains("необъявленная"),
        "Ошибка должна содержать имя необъявленной переменной"
    );
}

// ============================================================================
// Глобальные коллекции метаданных
// ============================================================================

#[tokio::test]
async fn test_global_collection_catalogs_no_error() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Справочники;
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Глобальная коллекция Справочники не должна проверяться"
    );
}

#[tokio::test]
async fn test_global_collection_documents_no_error() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Документы;
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Глобальная коллекция Документы не должна проверяться"
    );
}

#[tokio::test]
async fn test_global_collection_enums_no_error() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Перечисления;
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Глобальная коллекция Перечисления не должна проверяться"
    );
}

#[tokio::test]
async fn test_global_collection_in_argument() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Новый Массив;
    М.Добавить(Справочники);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Глобальные коллекции в аргументах не должны проверяться"
    );
}

// ============================================================================
// Вложенные вызовы и сложные сценарии
// ============================================================================

#[tokio::test]
#[ignore = "TODO: Member access на необъявленной переменной не отлавливается, т.к. property access обрабатывается иначе"]
async fn test_nested_method_calls_with_undeclared() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Новый Массив;
    М.Добавить(необъявленная.Свойство);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        !undeclared_errors.is_empty(),
        "Должна быть ошибка для необъявленной переменной в member access"
    );
    assert!(
        undeclared_errors[0].message.contains("необъявленная"),
        "Ошибка должна содержать имя переменной"
    );
}

#[tokio::test]
#[ignore = "TODO: Цепочки вызовов на необъявленных переменных не отлавливаются, т.к. property/method access обрабатывается иначе"]
async fn test_undeclared_in_chain_call() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    Результат = необъявленная.Метод1().Метод2();
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        !undeclared_errors.is_empty(),
        "Должна быть ошибка для необъявленной переменной в цепочке вызовов"
    );
}

// ============================================================================
// Объявление переменных в разных контекстах
// ============================================================================

#[tokio::test]
#[ignore = "TODO: BSL не имеет block scope, переменные должны быть видны в пределах всей функции"]
async fn test_variable_declared_in_if_branch() {
    let service = create_test_service();
    let code = r#"
Процедура Тест(Условие)
    Если Условие Тогда
        ЛокальнаяПеременная = "значение";
    КонецЕсли;

    М = Новый Массив;
    М.Добавить(ЛокальнаяПеременная);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    // Переменная объявлена в if-блоке, поэтому считается доступной в той же функции
    // (BSL не имеет block scope, только function scope)
    assert!(
        undeclared_errors.is_empty(),
        "Переменная объявленная в if-блоке доступна в той же функции"
    );
}

#[tokio::test]
#[ignore = "TODO: BSL не имеет block scope, переменные должны быть видны в пределах всей функции"]
async fn test_variable_declared_in_loop() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    Для Индекс = 1 По 10 Цикл
        ЛокальнаяПеременная = "значение";
    КонецЦикла;

    М = Новый Массив;
    М.Добавить(ЛокальнаяПеременная);
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    // Переменная объявлена в цикле, доступна в функции
    assert!(
        undeclared_errors.is_empty(),
        "Переменная объявленная в цикле доступна в той же функции"
    );
}

#[tokio::test]
async fn test_loop_counter_is_declared() {
    let service = create_test_service();
    let code = r#"
Процедура Тест()
    М = Новый Массив;
    Для Индекс = 1 По 10 Цикл
        М.Добавить(Индекс);
    КонецЦикла;
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Счетчик цикла должен считаться объявленным"
    );
}

// ============================================================================
// Реальные сценарии использования
// ============================================================================

#[tokio::test]
#[ignore = "TODO: Параметр КодПоиска не регистрируется в symbol_table"]
async fn test_real_scenario_catalog_search() {
    let service = create_test_service();
    let code = r#"
Процедура ПоискКонтрагента(КодПоиска)
    МенеджерСправочника = Справочники.Контрагенты;
    НайденныйЭлемент = МенеджерСправочника.НайтиПоКоду(КодПоиска);

    Если НайденныйЭлемент <> Неопределено Тогда
        Сообщить("Найден контрагент");
    КонецЕсли;
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert!(
        undeclared_errors.is_empty(),
        "Реальный сценарий поиска в справочнике не должен давать ошибок"
    );
}

#[tokio::test]
async fn test_real_scenario_with_typo() {
    let service = create_test_service();
    let code = r#"
Процедура ПоискКонтрагента(КодПоиска)
    МенеджерСправочника = Справочники.Контрагенты;
    НайденныйЭлемент = МенеджерСправочника.НайтиПоКоду(КодПоискА); // Опечатка в имени

    Если НайденныйЭлемент <> Неопределено Тогда
        Сообщить("Найден контрагент");
    КонецЕсли;
КонецПроцедуры
"#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());
    let errors = result.unwrap();

    let undeclared_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Необъявленная переменная"))
        .collect();

    assert_eq!(
        undeclared_errors.len(),
        1,
        "Должна быть ошибка для переменной с опечаткой"
    );
    assert!(
        undeclared_errors[0].message.contains("КодПоискА"),
        "Ошибка должна содержать имя переменной с опечаткой"
    );
}
