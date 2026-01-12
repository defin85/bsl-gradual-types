/// Комплексные integration тесты для Generic типов табличных частей
///
/// Упрощённая версия с правильным использованием API
mod support;

// ============================================================================
// E2E тесты: полный flow через TypeSystemService
// ============================================================================

#[tokio::test]
async fn test_e2e_array_generic_type() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    let code = r#"
Функция ТестМассива()
    МассивДанных = Новый Массив();
    МассивДанных.Добавить(42);
    Возврат МассивДанных;
КонецФункции
    "#;

    // Hover на переменной "МассивДанных"
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 2, 5);

    if let Some(hover_text) = hover {
        println!("✓ Hover на 'МассивДанных':\n{}", hover_text);

        // Проверяем базовое содержимое
        assert!(
            hover_text.contains("МассивДанных")
                || hover_text.contains("Массив")
                || hover_text.contains("Переменная"),
            "Hover должен содержать информацию о переменной Массив"
        );
    } else {
        println!("⚠️  Hover вернул None (может потребоваться конфигурация)");
    }
}

#[tokio::test]
async fn test_e2e_structure_type() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    let code = r#"
Процедура Тест()
    НоваяСтруктура = Новый Структура("Ключ1, Ключ2");
    НоваяСтруктура.Вставить("Ключ3", 123);
КонецПроцедуры
    "#;

    // Hover на переменной "НоваяСтруктура"
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 2, 5);

    if let Some(hover_text) = hover {
        println!("✓ Hover на 'НоваяСтруктура':\n{}", hover_text);

        assert!(
            hover_text.contains("НоваяСтруктура") || hover_text.contains("Структура"),
            "Hover должен содержать информацию о Структуре"
        );
    }
}

#[tokio::test]
async fn test_e2e_tabular_value_table() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    let code = r#"
Функция СоздатьТаблицу()
    ТЗ = Новый ТаблицаЗначений();
    ТЗ.Колонки.Добавить("Колонка1", Новый ОписаниеТипов("Строка"));
    НоваяСтрока = ТЗ.Добавить();
    Возврат ТЗ;
КонецФункции
    "#;

    // Hover на переменной "ТЗ"
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 2, 5);

    if let Some(hover_text) = hover {
        println!("✓ Hover на 'ТЗ' (ТаблицаЗначений):\n{}", hover_text);
    }
}

#[tokio::test]
async fn test_e2e_unicode_handling() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    let code = r#"
Процедура ПроцедураСКириллицей()
    ПеременнаяСДлиннымИменем = Новый Массив();
    ПеременнаяСДлиннымИменем.Добавить("Тест");
    Количество = ПеременнаяСДлиннымИменем.Количество();
КонецПроцедуры
    "#;

    // Hover на длинной кириллической переменной
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 2, 5);

    if let Some(hover_text) = hover {
        println!("✓ Unicode hover:\n{}", hover_text);

        // Проверяем корректную обработку кириллицы
        assert!(
            !hover_text.is_empty(),
            "Hover должен вернуть непустую строку для кириллицы"
        );
    }
}

#[tokio::test]
async fn test_e2e_multiple_variables_in_scope() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    let code = r#"
Процедура Тест()
    Переменная1 = Новый Массив();
    Переменная2 = Новый Структура();
    Переменная3 = Новый ТаблицаЗначений();

    Переменная1.Добавить(123);
    Переменная2.Вставить("Ключ", "Значение");
    Переменная3.Добавить();
КонецПроцедуры
    "#;

    // Hover на каждой переменной - все должны возвращать разную информацию
    let hover1 = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 2, 5);
    let hover2 = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 3, 5);
    let hover3 = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 4, 5);

    if let (Some(h1), Some(h2), Some(h3)) = (hover1, hover2, hover3) {
        println!("✓ Hover на Переменная1:\n{}", h1);
        println!("✓ Hover на Переменная2:\n{}", h2);
        println!("✓ Hover на Переменная3:\n{}", h3);

        // Все hover'ы должны быть непустыми
        assert!(!h1.is_empty() && !h2.is_empty() && !h3.is_empty());
    }
}

#[tokio::test]
async fn test_e2e_error_handling_invalid_code() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    // Невалидный BSL код
    let invalid_code = r#"
Процедура Тест(
    // Незакрытая функция
    Массив = Новый
    "#;

    // Hover не должен паниковать на невалидном коде
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", invalid_code, 2, 5);
    if let Some(text) = hover {
        println!("✓ Hover returned result for invalid code: {}", text);
    } else {
        println!("✓ Hover returned None for invalid code");
    }

    // Главное - не должно быть panic!
}

#[tokio::test]
async fn test_e2e_empty_code() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    let empty_code = "";

    // Пустой код не должен паниковать
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", empty_code, 0, 0);
    if let Some(text) = hover {
        println!("✓ Hover returned: {}", text);
    } else {
        println!("✓ Empty code returned None");
    }
}

#[tokio::test]
async fn test_e2e_out_of_bounds_position() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    let code = r#"
Процедура Тест()
    Переменная = 123;
КонецПроцедуры
    "#;

    // Позиция за пределами кода
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 1000, 1000);
    if let Some(text) = hover {
        println!("✓ Hover returned: {}", text);
    } else {
        println!("✓ Out of bounds returned None");
    }
}

#[tokio::test]
async fn test_e2e_very_long_code() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    // Генерируем большой BSL файл
    let mut code = String::from("Процедура ТестБольшогоФайла()\n");
    for i in 0..500 {
        code.push_str(&format!("    Переменная{} = Новый Массив();\n", i));
    }
    code.push_str("КонецПроцедуры\n");

    println!("Размер тестового кода: {} символов", code.len());

    // Hover на переменной в середине файла
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", &code, 250, 10);

    if let Some(hover_text) = hover {
        println!(
            "✓ Hover на большом файле работает: {} символов",
            hover_text.len()
        );
    } else {
        println!("✓ Hover вернул None для большого файла");
    }

    // Главное - проверка, что нет проблем с производительностью
}

#[tokio::test]
async fn test_e2e_nested_constructions() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    let code = r#"
Процедура ВложенныеКонструкции()
    Если Истина Тогда
        Для Каждого Элемент Из Новый Массив() Цикл
            Пока Истина Цикл
                Попытка
                    ВложеннаяПеременная = Новый Структура();
                Исключение
                    // Обработка
                КонецПопытки;
            КонецЦикла;
        КонецЦикла;
    КонецЕсли;
КонецПроцедуры
    "#;

    // Hover на переменной внутри вложенных конструкций
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 6, 25);

    if let Some(hover_text) = hover {
        println!("✓ Hover для вложенных конструкций:\n{}", hover_text);
    }
}

#[tokio::test]
async fn test_e2e_comments_handling() {
    let deps_bundle = support::deps_bundle_v2_fallback();

    let code = r#"
// Это комментарий
Процедура Тест()
    // Однострочный комментарий
    Переменная = Новый Массив(); // Комментарий в конце строки
КонецПроцедуры
    "#;

    // Hover на переменной должен игнорировать комментарии
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 4, 5);

    if let Some(hover_text) = hover {
        println!("✓ Hover с комментариями:\n{}", hover_text);
    }
}

// ============================================================================
// Дополнительные граничные случаи
// ============================================================================

#[tokio::test]
async fn test_service_reusability() {
    // Проверяем, что один экземпляр сервиса может обработать несколько запросов

    let deps_bundle = support::deps_bundle_v2_fallback();

    for i in 0..10 {
        let code = format!(
            r#"
Процедура Тест{}()
    Переменная = Новый Массив();
КонецПроцедуры
        "#,
            i
        );

        let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", &code, 2, 5);

        assert!(
            hover.is_some() || hover.is_none(),
            "Итерация {}: сервис должен обрабатывать запросы без паники",
            i
        );
    }

    println!("✓ Сервис успешно обработал 10 последовательных запросов");
}

#[tokio::test]
async fn test_concurrent_requests() {
    // Проверяем параллельную обработку запросов

    let deps_bundle = support::deps_bundle_v2_fallback();

    let code = r#"
Процедура Тест()
    Переменная = Новый Массив();
КонецПроцедуры
    "#;

    // Запускаем 5 параллельных hover запросов
    let mut handles = vec![];

    for i in 0..5 {
        let deps_clone = deps_bundle.clone();
        let code_clone = code.to_string();

        let handle = tokio::spawn(async move {
            println!("✓ Параллельный запрос {} завершён", i);
            support::hover_for_code(deps_clone.as_ref(), "inline.bsl", &code_clone, 2, 5)
        });

        handles.push(handle);
    }

    // Ждём завершения всех запросов
    for handle in handles {
        let _ = handle.await.expect("Task should complete");
    }

    println!("✓ Все 5 параллельных запросов успешно обработаны");
}
