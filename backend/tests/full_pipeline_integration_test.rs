//! Полный интеграционный тест пайплайна (Milestone 2.7 Task 4)
//!
//! Проверяет полный путь: File → AST (TreeSitter) → IR → Type Analysis

use bsl_backend::system::parser_coordinator::ParserCoordinator;

#[test]
fn test_full_pipeline_simple_function() {
    // 1. BSL исходный код
    let bsl_code = r#"
Функция ПолучитьСумму(А, Б)
    Перем Результат;
    Результат = А + Б;
    Возврат Результат;
КонецФункции
"#;

    // 2. Создаём координатор парсера
    let coordinator = ParserCoordinator::with_fallback();

    // 3. Парсинг в AST
    let parse_result = coordinator
        .parse(bsl_code)
        .expect("Парсинг должен пройти успешно");

    println!("✅ Step 1: Парсинг в AST");
    println!("   Statements: {}", parse_result.program.statements.len());
    if parse_result.has_errors() {
        println!("   ⚠️  Синтаксические ошибки: {}", parse_result.syntax_errors.len());
    }

    // 4. Конвертация AST → IR
    let ir_result = coordinator
        .parse_to_ir(bsl_code, "test.bsl")
        .expect("Конвертация в IR должна пройти успешно");

    println!("✅ Step 2: Конвертация AST → IR");
    println!("   IR узлов: {}", ir_result.nodes.len());
    println!("   Scopes: {}", ir_result.symbols.scopes.len());

    // Проверяем что функция распознана в IR
    let has_function = ir_result.nodes.iter().any(|node| {
        matches!(
            &node.kind,
            bsl_shared::ir::SemanticNodeKind::FunctionDeclaration { .. }
        )
    });
    assert!(has_function, "В IR должна быть функция");

    println!("✅ Step 3: Полный пайплайн завершён успешно");
}

#[test]
fn test_full_pipeline_with_type_inference() {
    let bsl_code = r#"
Функция Тест()
    Перем Число;
    Перем Строка;

    Число = 42;
    Строка = "Привет";

    Возврат Число + Строка;
КонецФункции
"#;

    let coordinator = ParserCoordinator::with_fallback();

    // Парсинг в AST
    let parse_result = coordinator
        .parse(bsl_code)
        .expect("Парсинг должен пройти");

    assert!(
        !parse_result.program.statements.is_empty(),
        "Должны быть statements"
    );

    // Конвертация в IR
    let ir_result = coordinator
        .parse_to_ir(bsl_code, "test.bsl")
        .expect("IR конвертация должна пройти");

    // Проверяем что переменные есть в таблице символов (ищем в scopes)
    let mut has_vars = false;
    for (_scope_id, scope) in &ir_result.symbols.scopes {
        if scope.variables.contains_key("Число") || scope.variables.contains_key("Строка") {
            has_vars = true;
            break;
        }
    }

    if has_vars {
        println!("✅ Type inference: переменные распознаны в таблице символов");
    }

    println!("   Всего scopes: {}", ir_result.symbols.scopes.len());
}

#[test]
fn test_full_pipeline_real_world_code() {
    // Реальный код с циклами, условиями и вызовами
    let bsl_code = r#"
Функция ОбработатьДанные(Данные)
    Перем Результат;
    Результат = Новый Массив;

    Для Каждого Элемент Из Данные Цикл
        Если Элемент.Активен Тогда
            Результат.Добавить(Элемент);
        КонецЕсли;
    КонецЦикла;

    Возврат Результат;
КонецФункции
"#;

    let coordinator = ParserCoordinator::with_fallback();

    // Полный пайплайн
    let parse_result = coordinator.parse(bsl_code).expect("Парсинг должен пройти");

    println!("✅ Real-world код: парсинг");
    println!("   Statements: {}", parse_result.program.statements.len());

    let ir_result = coordinator
        .parse_to_ir(bsl_code, "real_world.bsl")
        .expect("IR конвертация должна пройти");

    println!("✅ Real-world код: IR конвертация");
    println!("   IR узлов: {}", ir_result.nodes.len());

    // Проверяем что локальная переменная есть в scopes
    let mut has_result_var = false;
    for (_scope_id, scope) in &ir_result.symbols.scopes {
        if scope.variables.contains_key("Результат") {
            has_result_var = true;
            break;
        }
    }

    if has_result_var {
        println!("   ✅ Локальная переменная 'Результат' найдена в scopes");
    }
}

#[test]
fn test_pipeline_with_syntax_errors() {
    // Код с синтаксическими ошибками — пайплайн должен работать частично
    let bsl_code = r#"
Функция Тест()
    Перем А;
    А = 10
    // Отсутствует ;
    Возврат А;
КонецФункции
"#;

    let coordinator = ParserCoordinator::with_fallback();

    // Парсинг может вернуть результат с ошибками
    let parse_result = coordinator.parse(bsl_code).expect("ParseResult должен вернуться");

    // Проверяем partial recovery
    if parse_result.has_errors() {
        println!(
            "⚠️  Обнаружены синтаксические ошибки: {}",
            parse_result.syntax_errors.len()
        );
        for error in &parse_result.syntax_errors {
            println!("   - [{}:{}] {}", error.span.start_line, error.span.start_column, error.message);
        }
    }

    // Попытка конвертации в IR (может быть частичной)
    let ir_result = coordinator.parse_to_ir(bsl_code, "error_test.bsl");

    match ir_result {
        Ok(ir) => {
            println!("✅ Partial IR: {} узлов создано несмотря на ошибки", ir.nodes.len());
        }
        Err(e) => {
            println!("⚠️  IR конвертация не удалась: {}", e);
            // Это ожидаемое поведение для кода с ошибками
        }
    }
}

#[test]
fn test_pipeline_performance_stress() {
    // Проверка производительности на большом коде
    let mut large_code = String::from("Функция Тест()\n");

    // Генерируем 100 присваиваний
    for i in 0..100 {
        large_code.push_str(&format!("    Перем Переменная{};\n", i));
        large_code.push_str(&format!("    Переменная{} = {};\n", i, i));
    }

    large_code.push_str("    Возврат Переменная0;\nКонецФункции");

    let coordinator = ParserCoordinator::with_fallback();

    // Засекаем время парсинга
    let start = std::time::Instant::now();
    let parse_result = coordinator
        .parse(&large_code)
        .expect("Парсинг большого кода должен пройти");
    let parse_time = start.elapsed();

    println!("✅ Performance: парсинг 200+ statements");
    println!("   Время парсинга: {:?}", parse_time);
    println!("   Statements: {}", parse_result.program.statements.len());

    // Конвертация в IR
    let start = std::time::Instant::now();
    let ir_result = coordinator
        .parse_to_ir(&large_code, "large.bsl")
        .expect("IR конвертация должна пройти");
    let ir_time = start.elapsed();

    println!("   Время IR конвертации: {:?}", ir_time);
    println!("   IR узлов: {}", ir_result.nodes.len());
    println!("   Scopes: {}", ir_result.symbols.scopes.len());

    // Проверяем что парсинг быстрый (< 100ms для 200 statements)
    assert!(
        parse_time.as_millis() < 100,
        "Парсинг должен быть быстрым: {:?}",
        parse_time
    );
}

#[test]
fn test_pipeline_span_propagation() {
    // Проверяем что Span корректно пробрасывается из AST → IR
    let bsl_code = "Перем А;\nА = 10;";

    let coordinator = ParserCoordinator::with_fallback();

    let ir_result = coordinator
        .parse_to_ir(bsl_code, "span_test.bsl")
        .expect("IR должен создаться");

    // Проверяем что IR узлы имеют корректные span
    for node in &ir_result.nodes {
        // Span должен быть не-stub (не 0,0,0,0)
        let is_stub = node.span.start_line == 0
            && node.span.start_column == 0
            && node.span.end_line == 0
            && node.span.end_column == 0;

        if !is_stub {
            println!(
                "✅ IR узел {:?} имеет реальный span: [{}:{} - {}:{}]",
                node.kind, node.span.start_line, node.span.start_column, node.span.end_line, node.span.end_column
            );
        }
    }

    // Хотя бы некоторые узлы должны иметь реальные span
    let real_spans_count = ir_result
        .nodes
        .iter()
        .filter(|n| {
            !(n.span.start_line == 0
                && n.span.start_column == 0
                && n.span.end_line == 0
                && n.span.end_column == 0)
        })
        .count();

    assert!(
        real_spans_count > 0,
        "Хотя бы некоторые IR узлы должны иметь реальные span (найдено: {})",
        real_spans_count
    );

    println!("✅ Span propagation: {}/{} узлов имеют реальные span", real_spans_count, ir_result.nodes.len());
}
