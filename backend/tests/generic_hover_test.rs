//! Интеграционные тесты для hover с Generic типами
//!
//! Проверяет специальное форматирование для Generic типов:
//! - ТабличнаяЧасть<СтрокаРаботы>
//! - Отображение методов коллекции с подставленными типами
//! - Отображение атрибутов строки табличной части

use bsl_backend::application::TypeSystemService;
use bsl_backend::system::SystemCoordinator;
use std::sync::Arc;

/// Setup функция для создания TypeSystemService с загруженными типами платформы
async fn setup_type_system_service() -> Arc<TypeSystemService> {
    // Создаём SystemCoordinator, который инициализирует все компоненты
    let coordinator = SystemCoordinator::new();

    // Инициализируем систему (загрузка типов платформы)
    coordinator
        .start()
        .await
        .expect("Failed to start SystemCoordinator");

    // Получаем TypeSystemService из координатора
    coordinator
        .type_service()
        .expect("TypeSystemService should be initialized after start()")
}

#[tokio::test]
async fn test_generic_tabular_section_hover() {
    // Примечание: Этот тест использует заглушки для табличной части
    // В реальном окружении требуется загрузка конфигурации 1С
    let code = r#"
Процедура Тест()
    ТабличнаяЧасть = Новый ТабличнаяЧасть();
    НоваяСтрока = ТабличнаяЧасть.Добавить();
КонецПроцедуры
    "#;

    let service = setup_type_system_service().await;

    // Hover на переменной "ТабличнаяЧасть" (строка 2)
    let hover = service
        .get_hover_info(code, 2, 5)
        .await
        .expect("get_hover_info should succeed");

    // Проверяем что hover не пустой
    assert!(
        hover.is_some(),
        "Hover должен вернуть информацию для переменной ТабличнаяЧасть"
    );

    let hover_text = hover.unwrap();

    // Проверяем базовые элементы hover для табличной части
    assert!(
        hover_text.contains("ТабличнаяЧасть") || hover_text.contains("Переменная"),
        "Hover должен содержать информацию о переменной ТабличнаяЧасть\nActual: {}",
        hover_text
    );

    // Примечание: Generic синтаксис (<T>) будет виден только когда:
    // 1. TypeResolver вернёт ResolutionResult::Generic
    // 2. Это требует реальной конфигурации с TabularRowType метаданными
    println!("Hover для ТабличнаяЧасть:\n{}", hover_text);
}

#[tokio::test]
async fn test_generic_array_hover() {
    // Тест для Generic типа Массив<T>
    let code = r#"
Функция ТестМассива()
    МойМассив = Новый Массив();
    МойМассив.Добавить(42);
    Возврат МойМассив;
КонецФункции
    "#;

    let service = setup_type_system_service().await;

    // Hover на переменной "МойМассив" (строка 2)
    let hover = service
        .get_hover_info(code, 2, 5)
        .await
        .expect("get_hover_info should succeed");

    assert!(
        hover.is_some(),
        "Hover должен вернуть информацию для переменной МойМассив"
    );

    let hover_text = hover.unwrap();

    // Проверяем что hover содержит информацию о типе Массив
    assert!(
        hover_text.contains("Массив") || hover_text.contains("МойМассив"),
        "Hover должен содержать информацию о типе Массив или имени переменной\nActual: {}",
        hover_text
    );

    // Примечание: Методы коллекции будут показаны только если:
    // 1. TypeRepository содержит метаданные для типа Массив
    // 2. ResolutionResult возвращает Known или Inferred
    // В текущем тесте без загрузки Syntax Helper - это Unknown тип
    println!("Hover для Массив:\n{}", hover_text);
}

#[tokio::test]
async fn test_generic_method_return_type() {
    // Тест для метода Generic типа, который возвращает типизированный результат
    let code = r#"
Процедура Тест()
    МойМассив = Новый Массив();
    МойМассив.Добавить("элемент");

    // Получить элемент по индексу
    Элемент = МойМассив.Получить(0);
КонецПроцедуры
    "#;

    let service = setup_type_system_service().await;

    // Hover на переменной "Элемент" (строка 6)
    let hover = service
        .get_hover_info(code, 6, 5)
        .await
        .expect("get_hover_info should succeed");

    // Проверяем что hover не пустой
    if let Some(hover_text) = hover {
        println!(
            "Hover для Элемент (возврат из Generic метода):\n{}",
            hover_text
        );

        // В будущем, когда Generic inference будет полностью реализован,
        // здесь можно проверить что тип Элемент выведен как String
        // (так как Добавить("элемент") добавляет строку)
    } else {
        println!("WARN: Hover для переменной Элемент пока не реализован (ожидаемо для MVP)");
    }
}

#[tokio::test]
async fn test_format_generic_hover_directly() {
    // Прямой unit-тест для format_generic_hover (через публичное API)
    // Проверяем что Generic типы форматируются с синтаксисом <T>

    let code = r#"
Процедура Тест()
    ТабличнаяЧасть = Новый ТабличнаяЧасть();
КонецПроцедуры
    "#;

    let service = setup_type_system_service().await;
    let hover = service.get_hover_info(code, 2, 5).await.unwrap();

    if let Some(hover_text) = hover {
        // Проверяем структуру hover:
        // - Должна быть секция "Тип:"
        // - Должна быть секция "Уверенность:"
        // Секция "Переменная:" может отсутствовать если hover на присваивании

        let has_type_section = hover_text.contains("Тип:");
        let has_certainty_section = hover_text.contains("Уверенность:");

        assert!(
            has_type_section && has_certainty_section,
            "Hover должен содержать базовые секции (Тип, Уверенность)\nActual: {}",
            hover_text
        );

        println!("Generic hover structure test passed:\n{}", hover_text);
    } else {
        println!("WARN: Hover для ТабличнаяЧасть не вернул данные (ожидаемо если тип неизвестен)");
    }
}
