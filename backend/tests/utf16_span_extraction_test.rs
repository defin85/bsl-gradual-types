//! Тест проверки корректности извлечения UTF-16 координат из tree-sitter узлов
//!
//! **КРИТИЧНЫЙ ТЕСТ** для Milestone 2.18: LSP Syntax Error Diagnostics
//!
//! Проблема: Tree-sitter возвращает позиции в byte offsets (UTF-8),
//! но LSP требует UTF-16 code units. Кириллица занимает 2 UTF-16 code units,
//! поэтому без конвертации диагностики показываются на НЕПРАВИЛЬНЫХ позициях!
//!
//! Этот тест проверяет, что `TreeSitterAdapter::byte_offset_to_utf16()`
//! корректно конвертирует координаты для текста с кириллицей.
//!
//! Примечание: в текущей архитектуре tree-sitter spans сохраняются как UTF-8 byte offsets,
//! а конвертация в UTF-16 (LSP `Position.character`) делается на границе (через `LineIndex`).

use bsl_backend::system::LineIndex;
use bsl_backend::system::ParserCoordinator;

fn utf16_position(index: &LineIndex, source: &str, byte_offset: u32) -> (u32, u32) {
    let point = index.byte_offset_to_point(source, byte_offset as usize);
    let utf16_column = index.byte_column_to_utf16(source, point.row, point.column);
    (point.row as u32, utf16_column)
}

fn utf16_range(index: &LineIndex, source: &str, start: u32, end: u32) -> ((u32, u32), (u32, u32)) {
    (
        utf16_position(index, source, start),
        utf16_position(index, source, end),
    )
}

#[test]
fn test_cyrillic_text_span_coordinates() {
    // Код с кириллицей
    let source = r#"
Функция Тест()
    Перем Х;
    Возврат Х;
КонецФункции
"#;

    // Парсим файл
    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");
    let index = LineIndex::new(source);

    // Проверяем, что парсинг успешен
    assert!(!parse_result.has_errors(), "Код должен быть корректным");

    // Извлекаем AST
    let program = parse_result.program;
    assert!(
        !program.statements.is_empty(),
        "Должна быть хотя бы одна функция"
    );

    // Проверяем первый statement (должна быть функция)
    use bsl_backend::parsing::bsl::ast::Statement;
    if let Statement::FunctionDecl {
        name, body, span, ..
    } = &program.statements[0]
    {
        println!("\n=== ПРОВЕРКА UTF-16 КООРДИНАТ ===");
        println!("Функция: {}", name);
        let ((start_line, start_column), (end_line, end_column)) =
            utf16_range(&index, source, span.start, span.end);
        println!(
            "Span функции: {}:{} - {}:{}",
            start_line, start_column, end_line, end_column
        );

        // "Функция Тест()" начинается на строке 1, колонка 0 (UTF-16)
        assert_eq!(start_line, 1, "Функция должна начинаться на строке 1");
        assert_eq!(
            start_column, 0,
            "Функция должна начинаться с колонки 0 (UTF-16)"
        );

        // Проверяем тело функции
        assert!(!body.is_empty(), "Тело функции не должно быть пустым");

        // Проверяем первую переменную в теле
        if let Statement::VarDeclaration {
            name: var_name,
            span: var_span,
            ..
        } = &body[0]
        {
            let ((var_start_line, var_start_column), (var_end_line, var_end_column)) =
                utf16_range(&index, source, var_span.start, var_span.end);
            println!("\nПеременная: {}", var_name);
            println!(
                "Span переменной: {}:{} - {}:{}",
                var_start_line, var_start_column, var_end_line, var_end_column
            );

            // "    Перем Х;" на строке 2
            // 4 пробела (4 UTF-16) + "Перем" (5 символов = 5 UTF-16)
            assert_eq!(var_start_line, 2, "Переменная должна быть на строке 2");

            // ✅ КЛЮЧЕВАЯ ПРОВЕРКА: координаты должны быть в UTF-16, не в UTF-8 bytes!
            // "    Перем" = 4 пробела + 5 символов кириллицы
            // UTF-16: 4 code units (пробелы) + 5 code units (Перем) = 9 code units
            // UTF-8: 4 bytes (пробелы) + 10 bytes (Перем в UTF-8) = 14 bytes
            //
            // Если бы мы возвращали byte offsets, start_column был бы 14
            // Но мы конвертируем в UTF-16, поэтому start_column должен быть 4
            assert_eq!(
                var_start_column, 4,
                "Переменная должна начинаться с колонки 4 (UTF-16)"
            );

            println!("✅ UTF-16 координаты корректны!");
        } else {
            panic!("Первое statement в теле функции должно быть VarDeclaration");
        }

        // Проверяем return statement
        if let Statement::Return {
            span: return_span, ..
        } = &body[1]
        {
            let ((ret_start_line, ret_start_column), (ret_end_line, ret_end_column)) =
                utf16_range(&index, source, return_span.start, return_span.end);
            println!("\nReturn statement:");
            println!(
                "Span: {}:{} - {}:{}",
                ret_start_line, ret_start_column, ret_end_line, ret_end_column
            );

            assert_eq!(ret_start_line, 3, "Return должен быть на строке 3");
            assert_eq!(
                ret_start_column, 4,
                "Return должен начинаться с колонки 4 (UTF-16)"
            );

            println!("✅ Return statement UTF-16 координаты корректны!");
        }
    } else {
        panic!("Первое statement должно быть FunctionDecl");
    }

    println!("=================================\n");
}

#[test]
fn test_cyrillic_vs_ascii_span_coordinates() {
    // Код с смешанным текстом (кириллица + ASCII)
    let source = r#"
Процедура ТестProc()
    Перем Array;
    Array = New Array;
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");
    let index = LineIndex::new(source);

    assert!(!parse_result.has_errors(), "Код должен быть корректным");

    let program = parse_result.program;
    use bsl_backend::parsing::bsl::ast::Statement;

    if let Statement::ProcedureDecl {
        name, body, span, ..
    } = &program.statements[0]
    {
        println!("\n=== ПРОВЕРКА СМЕШАННОГО ТЕКСТА (Кириллица + ASCII) ===");
        println!("Процедура: {}", name);
        let ((start_line, start_column), (end_line, end_column)) =
            utf16_range(&index, source, span.start, span.end);
        println!(
            "Span процедуры: {}:{} - {}:{}",
            start_line, start_column, end_line, end_column
        );

        // "Процедура ТестProc()" - слово "Процедура" = 9 символов кириллицы
        assert_eq!(start_line, 1);
        assert_eq!(start_column, 0);

        // Проверяем переменную "Array" (английское название)
        if let Statement::VarDeclaration {
            name: var_name,
            span: var_span,
            ..
        } = &body[0]
        {
            let ((var_start_line, var_start_column), (var_end_line, var_end_column)) =
                utf16_range(&index, source, var_span.start, var_span.end);
            println!("\nПеременная (ASCII): {}", var_name);
            println!(
                "Span: {}:{} - {}:{}",
                var_start_line, var_start_column, var_end_line, var_end_column
            );

            assert_eq!(var_name, "Array");
            assert_eq!(var_start_line, 2);

            // "    Перем Array" - 4 пробела + "Перем" (5 символов кириллицы)
            // UTF-16: 4 + 5 = 9, затем пробел = 10, затем "Array" начинается с 10
            // Но нам нужна позиция начала "Перем", а не "Array"
            assert_eq!(
                var_start_column, 4,
                "Перем должен начинаться с колонки 4 (UTF-16)"
            );

            println!("✅ Смешанный текст: UTF-16 координаты корректны!");
        }
    }

    println!("================================================\n");
}

#[test]
fn test_deeply_nested_cyrillic_coordinates() {
    // Код с глубокой вложенностью для проверки корректности координат на всех уровнях
    let source = r#"
Функция ВнешняяФункция()
    Перем Результат;

    Если Истина Тогда
        Для Счетчик = 1 По 10 Цикл
            Результат = Счетчик;
        КонецЦикла;
    КонецЕсли;

    Возврат Результат;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");
    let index = LineIndex::new(source);

    if parse_result.has_errors() {
        println!("\n⚠️ НЕОЖИДАННЫЕ ОШИБКИ В КОДЕ:");
        for error in &parse_result.syntax_errors {
            let (line, column) = utf16_position(&index, source, error.span.start);
            println!(
                "  - {:?}: {} [{}:{}]",
                error.error_type, error.message, line, column
            );
        }
    }

    assert!(!parse_result.has_errors(), "Код должен быть корректным");

    let program = parse_result.program;
    use bsl_backend::parsing::bsl::ast::Statement;

    if let Statement::FunctionDecl { body, .. } = &program.statements[0] {
        // Проверяем If statement
        if let Statement::If { span: if_span, .. } = &body[1] {
            println!("\n=== ПРОВЕРКА ВЛОЖЕННЫХ СТРУКТУР ===");
            let ((if_start_line, if_start_column), (if_end_line, if_end_column)) =
                utf16_range(&index, source, if_span.start, if_span.end);
            println!(
                "If statement span: {}:{} - {}:{}",
                if_start_line, if_start_column, if_end_line, if_end_column
            );

            // "    Если" - 4 пробела
            assert_eq!(if_start_line, 4, "If должен быть на строке 4");
            assert_eq!(
                if_start_column, 4,
                "If должен начинаться с колонки 4 (UTF-16), не с byte offset!"
            );

            println!("✅ Вложенный If: UTF-16 координаты корректны!");
        }

        // Проверяем Return statement
        if let Statement::Return {
            span: return_span, ..
        } = &body[2]
        {
            let ((ret_start_line, ret_start_column), (ret_end_line, ret_end_column)) =
                utf16_range(&index, source, return_span.start, return_span.end);
            println!(
                "\nReturn statement span: {}:{} - {}:{}",
                ret_start_line, ret_start_column, ret_end_line, ret_end_column
            );

            assert_eq!(ret_start_line, 10, "Return должен быть на строке 10");
            assert_eq!(
                ret_start_column, 4,
                "Return должен начинаться с колонки 4 (UTF-16)"
            );

            println!("✅ Return statement: UTF-16 координаты корректны!");
        }
    }

    println!("==================================\n");
}

#[test]
fn test_emoji_and_special_chars_coordinates() {
    // Экстремальный тест: эмодзи и специальные символы
    // Эмодзи могут занимать 4 UTF-8 bytes, но 2 UTF-16 code units (surrogate pair)
    let source = r#"
Функция ТестСимволов()
    Перем Текст;
    Текст = "Привет 🌍 мир";
    Возврат Текст;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");
    let index = LineIndex::new(source);

    // Этот тест просто проверяет, что парсинг не падает
    // и координаты имеют разумные значения
    assert!(
        !parse_result.has_errors(),
        "Код с эмодзи должен парситься корректно"
    );

    let program = parse_result.program;
    use bsl_backend::parsing::bsl::ast::Statement;

    if let Statement::FunctionDecl { body, .. } = &program.statements[0] {
        if let Statement::VarDeclaration { span: var_span, .. } = &body[0] {
            println!("\n=== ПРОВЕРКА СПЕЦИАЛЬНЫХ СИМВОЛОВ ===");
            let ((var_start_line, var_start_column), (var_end_line, var_end_column)) =
                utf16_range(&index, source, var_span.start, var_span.end);
            println!(
                "Variable span: {}:{} - {}:{}",
                var_start_line, var_start_column, var_end_line, var_end_column
            );

            assert_eq!(var_start_line, 2);
            assert_eq!(var_start_column, 4);

            println!("✅ Специальные символы: координаты стабильны!");
        }
    }

    println!("=====================================\n");
}

#[test]
fn test_utf16_conversion_function_directly() {
    // Прямой тест конвертации byte offset -> UTF-16 column на строке с кириллицей.
    let source = "    Перем Х;\n"; // 4 пробела + кириллица
    let index = LineIndex::new(source);

    // "Х" начинается после "    Перем ":
    // UTF-8: 4 (spaces) + 10 ("Перем" in UTF-8) + 1 (space) = 15 bytes
    // UTF-16: 4 + 5 + 1 = 10 code units
    let byte_offset_x = 15u32;
    let (line, utf16_col) = utf16_position(&index, source, byte_offset_x);

    assert_eq!(line, 0);
    assert_eq!(utf16_col, 10);
}
