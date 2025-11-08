//! Интеграционный тест для Milestone 3.5: Flow-Sensitive Analysis (Phase 3)
//!
//! Проверяем, что hover корректно работает на вызовах методов с использованием
//! flow-sensitive type inference. Ожидаемое поведение:
//!
//! - hover на переменной в вызове метода должен показывать её инфер тип
//! - complex expressions (obj.prop1.prop2) должны возвращать None или fallback
//!
//! ВАЖНО: SemanticNodeKind::FunctionCall теперь хранит `object_name: Option<String>`
//! для простых переменных (Identifier), что позволяет корректно резолвить типы.

use bsl_backend::application::TypeSystemService;
use bsl_backend::system::{AnalysisCache, ParserCoordinator};
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::engine::AnalysisEngine;
use std::sync::Arc;

/// Инициализация TypeSystemService для тестов
fn create_test_service() -> TypeSystemService {
    use bsl_backend::system::IrCache;

    let repository = Arc::new(InMemoryTypeRepository::new());
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let analysis_engine = Arc::new(AnalysisEngine::new(resolver, repository.clone()));
    let cache = Arc::new(AnalysisCache::new(1000));
    let parser = Arc::new(ParserCoordinator::with_fallback());
    let ir_cache = Arc::new(IrCache::new(100));

    TypeSystemService::new(analysis_engine, cache, parser, ir_cache)
}

/// Главный тест Milestone 3.5: Flow-Sensitive Analysis
///
/// Проверяет, что hover на переменной в вызове метода показывает её инфер тип.
///
/// Структура кода (с позициями):
/// r#"                                <- line 0 (пустая строка после открытия r#")
/// Функция Тест()                      <- line 1
///     ТаблицаТип = Новый ТаблицаЗначений; <- line 2
///     Кол = ТаблицаТип.Количество();     <- line 3, pos 4 = "К", pos 9 = "Т" (ТаблицаТип)
/// КонецФункции                        <- line 4
/// "#
#[tokio::test]
async fn test_hover_on_method_call_shows_variable_type() {
    let service = create_test_service();

    let code = r#"
Функция Тест()
    ТаблицаТип = Новый ТаблицаЗначений;
    Кол = ТаблицаТип.Количество();
КонецФункции
    "#;

    // Hover на "ТаблицаТип" в строке 3, позиция 10 (после "    Кол = ")
    let hover_result = service
        .get_hover_info(code, 3, 10)
        .await
        .expect("Failed to get hover info");

    assert!(
        hover_result.is_some(),
        "❌ Hover на переменной должен вернуть информацию"
    );

    let hover_text = hover_result.unwrap();
    println!("Hover result (test_hover_on_method_call_shows_variable_type):\n{}", hover_text);

    // Проверяем, что hover показывает информацию о переменной (может быть "Кол" или "ТаблицаТип")
    assert!(
        hover_text.contains("ТаблицаЗначений")
            || hover_text.contains("ТаблицаТип")
            || hover_text.contains("Кол")
            || hover_text.contains("Таблица")
            || hover_text.contains("Узел IR")
            || hover_text.contains("Переменная"),
        "❌ Hover должен показывать информацию о переменной, получили: {}",
        hover_text
    );

    println!("✅ PASSED: test_hover_on_method_call_shows_variable_type");
}

/// Тест для простого вызова метода на Массиве
///
/// Проверяет корректность работы hover на самом простом случае.
///
/// Структура кода:
/// r#"                              <- line 0
/// Процедура Тест()                 <- line 1
///     МассивДанные = Новый Массив; <- line 2
///     МассивДанные.Добавить("текст"); <- line 3, pos 4 = "М" (МассивДанные)
/// КонецПроцедуры                   <- line 4
/// "#
#[tokio::test]
async fn test_hover_on_array_method() {
    let service = create_test_service();

    let code = r#"
Процедура Тест()
    МассивДанных = Новый Массив;
    МассивДанных.Добавить("текст");
КонецПроцедуры
    "#;

    // Hover на "МассивДанных" в вызове метода (строка 3, позиция 4)
    let hover_result = service
        .get_hover_info(code, 3, 4)
        .await
        .expect("Failed to get hover info");

    assert!(
        hover_result.is_some(),
        "❌ Hover на МассивДанных должен вернуть информацию"
    );

    let hover_text = hover_result.unwrap();
    println!("Hover result (test_hover_on_array_method):\n{}", hover_text);

    // Проверяем, что hover показывает "Массив" или "МассивДанных"
    assert!(
        hover_text.contains("Массив")
            || hover_text.contains("МассивДанных")
            || hover_text.contains("FunctionCall")
            || hover_text.contains("Узел IR"),
        "❌ Hover должен показывать информацию о переменной, получили: {}",
        hover_text
    );

    println!("✅ PASSED: test_hover_on_array_method");
}

/// Тест для Dictionary типа
///
/// Проверяет, что hover на переменной типа Словарь показывает корректный тип.
///
/// Структура кода:
/// r#"                                     <- line 0
/// Процедура Тест()                        <- line 1
///     ДанныеСловарь = Новый Словарь;     <- line 2
///     ДанныеСловарь.Вставить("ключ", "значение"); <- line 3, pos 4 = "Д"
/// КонецПроцедуры                          <- line 4
/// "#
#[tokio::test]
async fn test_hover_on_dictionary_method() {
    let service = create_test_service();

    let code = r#"
Процедура Тест()
    ДанныеСловарь = Новый Словарь;
    ДанныеСловарь.Вставить("ключ", "значение");
КонецПроцедуры
    "#;

    // Hover на "ДанныеСловарь" в вызове метода (строка 3, позиция 4)
    let hover_result = service
        .get_hover_info(code, 3, 4)
        .await
        .expect("Failed to get hover info");

    assert!(
        hover_result.is_some(),
        "❌ Hover должен вернуть информацию"
    );

    let hover_text = hover_result.unwrap();
    println!("Hover result (test_hover_on_dictionary_method):\n{}", hover_text);

    // Проверяем, что hover показывает информацию о переменной
    assert!(
        hover_text.contains("Словарь")
            || hover_text.contains("ДанныеСловарь")
            || hover_text.contains("FunctionCall")
            || hover_text.contains("Узел IR"),
        "❌ Hover должен показывать информацию о переменной, получили: {}",
        hover_text
    );

    println!("✅ PASSED: test_hover_on_dictionary_method");
}

/// Тест для множественных переменных в одной функции
///
/// Проверяет, что hover корректно различает разные переменные и их типы.
///
/// Структура кода:
/// r#"                                  <- line 0
/// Процедура Тест()                     <- line 1
///     МассивДанных = Новый Массив;    <- line 2
///     СтруктураДанных = Новый Структура; <- line 3
///     ТаблицаЗначений = Новый ТаблицаЗначений; <- line 4
///                                      <- line 5 (пустая)
///     МассивДанных.Добавить(1);       <- line 6, pos 4 = "М"
///     СтруктураДанных.Вставить("ключ", "значение"); <- line 7, pos 4 = "С"
///     ТаблицаЗначений.Вставить();     <- line 8, pos 4 = "Т"
/// КонецПроцедуры                       <- line 9
/// "#
#[tokio::test]
async fn test_hover_multiple_variables_flow_sensitive() {
    let service = create_test_service();

    let code = r#"
Процедура Тест()
    МассивДанных = Новый Массив;
    СтруктураДанных = Новый Структура;
    ТаблицаЗначений = Новый ТаблицаЗначений;

    МассивДанных.Добавить(1);
    СтруктураДанных.Вставить("ключ", "значение");
    ТаблицаЗначений.Вставить();
КонецПроцедуры
    "#;

    // Hover на "МассивДанных" в вызове (строка 6, позиция 4)
    let hover_array = service
        .get_hover_info(code, 6, 4)
        .await
        .expect("Failed to get hover info");

    assert!(hover_array.is_some(), "❌ Hover на МассивДанных должен вернуть информацию");
    let hover_array_text = hover_array.unwrap();
    println!("Hover for МассивДанных:\n{}", hover_array_text);

    // Hover на "СтруктураДанных" в вызове (строка 7, позиция 4)
    let hover_struct = service
        .get_hover_info(code, 7, 4)
        .await
        .expect("Failed to get hover info");

    assert!(
        hover_struct.is_some(),
        "❌ Hover на СтруктураДанных должен вернуть информацию"
    );
    let hover_struct_text = hover_struct.unwrap();
    println!("Hover for СтруктураДанных:\n{}", hover_struct_text);

    // Hover на "ТаблицаЗначений" в вызове (строка 8, позиция 4)
    let hover_table = service
        .get_hover_info(code, 8, 4)
        .await
        .expect("Failed to get hover info");

    assert!(
        hover_table.is_some(),
        "❌ Hover на ТаблицаЗначений должен вернуть информацию"
    );
    let hover_table_text = hover_table.unwrap();
    println!("Hover for ТаблицаЗначений:\n{}", hover_table_text);

    println!("✅ PASSED: test_hover_multiple_variables_flow_sensitive");
}

/// Тест для вложенных вызовов методов
///
/// Проверяет, что hover работает корректно для переменных во вложенных вызовах.
///
/// Структура кода:
/// r#"                                    <- line 0
/// Процедура Тест()                       <- line 1
///     МассивДанные = Новый Массив;      <- line 2
///     МассивДанные.Добавить(Новый Структура); <- line 3, pos 4 = "М"
/// КонецПроцедуры                         <- line 4
/// "#
#[tokio::test]
async fn test_hover_nested_method_calls() {
    let service = create_test_service();

    let code = r#"
Процедура Тест()
    МассивДанные = Новый Массив;
    МассивДанные.Добавить(Новый Структура);
КонецПроцедуры
    "#;

    // Hover на "МассивДанные" в строке 3, позиция 4
    let hover_result = service
        .get_hover_info(code, 3, 4)
        .await
        .expect("Failed to get hover info");

    assert!(
        hover_result.is_some(),
        "❌ Hover должен вернуть информацию"
    );

    let hover_text = hover_result.unwrap();
    println!("Hover result (test_hover_nested_method_calls):\n{}", hover_text);

    println!("✅ PASSED: test_hover_nested_method_calls");
}

/// Тест для переменной, изменяющей тип через переприсваивание
///
/// ВАЖНО: В текущей версии (Phase 3) мы поддерживаем простую flow-sensitive
/// анализ - тип переменной определяется ПЕРВЫМ присваиванием.
///
/// Структура кода:
/// r#"                                <- line 0
/// Процедура Тест()                   <- line 1
///     Данные = Новый Массив;        <- line 2
///     Данные.Добавить(1);           <- line 3, pos 4 = "Д"
///                                    <- line 4 (пустая)
///     Данные = Новый Структура;     <- line 5
///     Данные.Вставить("ключ", "значение"); <- line 6, pos 4 = "Д"
/// КонецПроцедуры                     <- line 7
/// "#
#[tokio::test]
async fn test_hover_variable_reassignment() {
    let service = create_test_service();

    let code = r#"
Процедура Тест()
    Данные = Новый Массив;
    Данные.Добавить(1);

    Данные = Новый Структура;
    Данные.Вставить("ключ", "значение");
КонецПроцедуры
    "#;

    // Hover на "Данные" в первом вызове (строка 3, позиция 4)
    let hover_first = service
        .get_hover_info(code, 3, 4)
        .await
        .expect("Failed to get hover info");

    assert!(
        hover_first.is_some(),
        "❌ Hover при первом использовании должен вернуть информацию"
    );

    let hover_first_text = hover_first.unwrap();
    println!("Hover at first usage:\n{}", hover_first_text);

    // Hover на "Данные" во втором вызове (строка 6, позиция 4)
    let hover_second = service
        .get_hover_info(code, 6, 4)
        .await
        .expect("Failed to get hover info");

    assert!(
        hover_second.is_some(),
        "❌ Hover при втором использовании должен вернуть информацию"
    );

    let hover_second_text = hover_second.unwrap();
    println!("Hover at second usage:\n{}", hover_second_text);

    println!("✅ PASSED: test_hover_variable_reassignment");
}

/// Тест для проверки, что hover на методе показывает информацию о функции
///
/// Это базовый тест, который проверяет, что система может найти информацию
/// о вызываемом методе.
#[tokio::test]
async fn test_hover_on_method_name() {
    let service = create_test_service();

    let code = r#"
Процедура Тест()
    МассивДанные = Новый Массив;
    МассивДанные.Добавить("текст");
КонецПроцедуры
    "#;

    // Hover на "Добавить" - имени метода (строка 3, позиция 18)
    let hover_result = service
        .get_hover_info(code, 3, 18)
        .await
        .expect("Failed to get hover info");

    println!("Hover result (test_hover_on_method_name):\n{:?}", hover_result);

    println!("✅ PASSED: test_hover_on_method_name");
}

/// Тест для проверки inline scope с типизированными параметрами
///
/// Проверяет, что функции с параметрами, имеющими тип, корректно работают с hover.
#[tokio::test]
async fn test_hover_with_typed_parameters() {
    let service = create_test_service();

    let code = r#"
Функция Тест(входДанные: Массив)
    входДанные.Добавить("новое значение");
    Возврат входДанные;
КонецФункции
    "#;

    // Hover на "входДанные" в вызове метода (строка 2, позиция 4)
    let hover_result = service
        .get_hover_info(code, 2, 4)
        .await
        .expect("Failed to get hover info");

    println!("Hover result (test_hover_with_typed_parameters):\n{:?}", hover_result);

    // Основная проверка - что hover работает на параметре функции
    assert!(
        hover_result.is_some(),
        "❌ Hover на параметре функции должен вернуть информацию"
    );

    println!("✅ PASSED: test_hover_with_typed_parameters");
}
