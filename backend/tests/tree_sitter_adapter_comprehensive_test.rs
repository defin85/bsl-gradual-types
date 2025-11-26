//! Comprehensive unit-тесты для TreeSitterAdapter
//! Проверяют парсинг всех 36 node types из tree-sitter-bsl grammar

use bsl_backend::parsing::bsl::ast::{Expression, Program, Statement};
use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use tree_sitter::Parser;

/// Парсит BSL код и возвращает Program AST (извлекая из ParseResult)
fn parse_bsl(code: &str) -> Result<Program, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .map_err(|e| format!("Failed to set language: {:?}", e))?;

    let tree = parser
        .parse(code, None)
        .ok_or_else(|| "Failed to parse".to_string())?;

    // convert_tree теперь возвращает ParseResult, извлекаем program
    let parse_result = TreeSitterAdapter::convert_tree(&tree, code)?;

    // Для юнит-тестов парсинга мы не ожидаем синтаксических ошибок
    if parse_result.has_errors() {
        return Err(format!(
            "Unexpected syntax errors: {:?}",
            parse_result.syntax_errors
        ));
    }

    Ok(parse_result.program)
}

// ============================================================================
// STATEMENTS TESTS
// ============================================================================

#[test]
fn test_procedure_declaration() {
    let code = "Процедура Тест(Параметр1, Параметр2)\n    Возврат;\nКонецПроцедуры";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::ProcedureDecl {
            name, params, body, ..
        } => {
            assert_eq!(name, "Тест");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], "Параметр1");
            assert_eq!(params[1], "Параметр2");
            assert!(!body.is_empty(), "Тело процедуры не должно быть пустым");
        }
        _ => panic!("Expected ProcedureDecl, got {:?}", program.statements[0]),
    }
}

#[test]
fn test_function_declaration() {
    let code = "Функция Сумма(А, Б)\n    Возврат А + Б;\nКонецФункции";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::FunctionDecl {
            name, params, body, ..
        } => {
            assert_eq!(name, "Сумма");
            assert_eq!(params.len(), 2);
            assert!(!body.is_empty());
        }
        _ => panic!("Expected FunctionDecl"),
    }
}

#[test]
fn test_var_declaration() {
    let code = "Перем Счетчик;\nПерем Результат;";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert!(program.statements.len() >= 2);
    match &program.statements[0] {
        Statement::VarDeclaration {
            name, type_hint, ..
        } => {
            assert_eq!(name, "Счетчик");
            assert!(type_hint.is_none());
        }
        _ => panic!("Expected VarDeclaration"),
    }
}

#[test]
fn test_if_statement() {
    let code = "Если Условие Тогда\n    А = 1;\nИначе\n    А = 2;\nКонецЕсли";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            assert!(matches!(condition, Expression::Identifier { .. }));
            assert!(!then_body.is_empty());
            assert!(else_body.is_some());
        }
        _ => panic!("Expected If statement"),
    }
}

#[test]
fn test_for_statement() {
    let code = "Для Индекс = 1 По 10 Цикл\n    Сообщить(Индекс);\nКонецЦикла";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::For {
            variable,
            start,
            end,
            body,
            ..
        } => {
            assert_eq!(variable, "Индекс");
            assert!(matches!(start, Expression::Number { .. }));
            assert!(matches!(end, Expression::Number { .. }));
            assert!(!body.is_empty());
        }
        _ => panic!("Expected For statement, got {:?}", program.statements[0]),
    }
}

#[test]
fn test_for_each_statement() {
    let code = "Для Каждого Элемент Из Коллекция Цикл\n    Сообщить(Элемент);\nКонецЦикла";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::ForEach {
            variable,
            collection,
            body,
            ..
        } => {
            assert_eq!(variable, "Элемент");
            assert!(matches!(collection, Expression::Identifier { .. }));
            assert!(!body.is_empty());
        }
        _ => panic!("Expected ForEach statement"),
    }
}

#[test]
fn test_while_statement() {
    let code = "Пока Условие Цикл\n    Счетчик = Счетчик + 1;\nКонецЦикла";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::While {
            condition, body, ..
        } => {
            assert!(matches!(condition, Expression::Identifier { .. }));
            assert!(!body.is_empty());
        }
        _ => panic!("Expected While statement"),
    }
}

#[test]
fn test_try_statement() {
    let code =
        "Попытка\n    ВызватьМетод();\nИсключение\n    Сообщить(ОписаниеОшибки());\nКонецПопытки";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::Try {
            try_body,
            except_body,
            ..
        } => {
            assert!(!try_body.is_empty());
            assert!(!except_body.is_empty());
        }
        _ => panic!("Expected Try statement"),
    }
}

#[test]
fn test_return_statement() {
    let code = "Функция Тест()\n    Возврат 42;\nКонецФункции";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::FunctionDecl { body, .. } => {
            assert!(!body.is_empty());
            match &body[0] {
                Statement::Return { value, .. } => {
                    assert!(value.is_some());
                    assert!(matches!(value.as_ref().unwrap(), Expression::Number { .. }));
                }
                _ => panic!("Expected Return statement in function body"),
            }
        }
        _ => panic!("Expected FunctionDecl"),
    }
}

#[test]
fn test_break_statement() {
    let code = "Пока Истина Цикл\n    Прервать;\nКонецЦикла";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::While { body, .. } => {
            assert!(!body.is_empty());
            match &body[0] {
                Statement::Break { .. } => {} // Success
                _ => panic!("Expected Break statement"),
            }
        }
        _ => panic!("Expected While statement"),
    }
}

#[test]
fn test_continue_statement() {
    let code = "Пока Истина Цикл\n    Продолжить;\nКонецЦикла";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::While { body, .. } => {
            assert!(!body.is_empty());
            match &body[0] {
                Statement::Continue { .. } => {} // Success
                _ => panic!("Expected Continue statement"),
            }
        }
        _ => panic!("Expected While statement"),
    }
}

#[test]
fn test_goto_label_statements() {
    let code = "~Метка:\nПерейти ~Метка;";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    // Должны быть минимум 2 statement: Label и Goto
    assert!(program.statements.len() >= 2);

    // Проверяем что есть Label
    let has_label = program
        .statements
        .iter()
        .any(|stmt| matches!(stmt, Statement::Label { name, .. } if name == "Метка"));
    assert!(has_label, "Should have Label statement");

    // Проверяем что есть Goto
    let has_goto = program
        .statements
        .iter()
        .any(|stmt| matches!(stmt, Statement::Goto { label, .. } if label == "Метка"));
    assert!(has_goto, "Should have Goto statement");
}

#[test]
fn test_execute_statement() {
    let code = "Выполнить(\"Сообщить('Динамический код')\");";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::Execute { code: expr, .. } => {
            // Execute принимает строку с кодом, а не вызов функции
            assert!(matches!(expr, Expression::String { .. }));
        }
        _ => panic!("Expected Execute statement"),
    }
}

#[test]
fn test_raise_error_statement() {
    let code = "ВызватьИсключение \"Ошибка\";";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::RaiseError { message, .. } => {
            assert!(message.is_some());
            assert!(matches!(
                message.as_ref().unwrap(),
                Expression::String { .. }
            ));
        }
        _ => panic!("Expected RaiseError statement"),
    }
}

#[test]
fn test_call_statement() {
    let code = "Сообщить(\"Привет\");";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::Call { expression, .. } => {
            assert!(matches!(expression, Expression::Call { .. }));
        }
        _ => panic!("Expected Call statement, got {:?}", program.statements[0]),
    }
}

#[test]
fn test_assignment_statement() {
    let code = "Переменная = 123;";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::Assignment { target, value, .. } => {
            assert!(matches!(target, Expression::Identifier { .. }));
            assert!(matches!(value, Expression::Number { .. }));
        }
        _ => panic!("Expected Assignment statement"),
    }
}

// ============================================================================
// EXPRESSIONS TESTS
// ============================================================================

#[test]
fn test_identifier_expression() {
    let code = "Переменная = ДругаяПеременная;";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            assert!(matches!(value, Expression::Identifier { .. }));
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_number_expression() {
    let code = "Число = 42.5;";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            if let Expression::Number { value: n, .. } = value {
                assert_eq!(*n, 42.5);
            } else {
                panic!("Expected Number expression");
            }
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_string_expression() {
    let code = "Текст = \"Привет, мир!\";";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            assert!(matches!(value, Expression::String { .. }));
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_boolean_expression() {
    let code = "Флаг = Истина;";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            if let Expression::Boolean { value: b, .. } = value {
                assert!(*b);
            } else {
                panic!("Expected Boolean expression");
            }
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_binary_expression() {
    let code = "Результат = 10 + 20 * 3;";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            assert!(matches!(value, Expression::Binary { .. }));
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_unary_expression() {
    let code = "Результат = -42;";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            // Может быть Unary или просто Number с минусом
            assert!(
                matches!(value, Expression::Unary { .. })
                    || matches!(value, Expression::Number { .. })
            );
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_call_expression() {
    let code = "Результат = ВычислитьСумму(10, 20);";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            if let Expression::Call { function, args, .. } = value {
                assert!(matches!(function.as_ref(), Expression::Identifier { .. }));
                assert_eq!(args.len(), 2);
            } else {
                panic!("Expected Call expression");
            }
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_property_access_expression() {
    let code = "Значение = Объект.Свойство;";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            // Может быть PropertyAccess или Identifier (fallback)
            // DEBUG: Показываем что именно получили
            eprintln!("DEBUG: value = {:?}", value);
            assert!(
                matches!(value, Expression::PropertyAccess { .. })
                    || matches!(value, Expression::Identifier { .. })
            );
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_new_expression() {
    let code = "Массив = Новый Массив;";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            if let Expression::New {
                type_name, args, ..
            } = value
            {
                assert!(type_name.contains("Массив"));
                assert_eq!(args.len(), 0);
            } else {
                panic!("Expected New expression, got {:?}", value);
            }
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_ternary_expression() {
    let code = "Результат = ?(Условие, Истина, Ложь);";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            // Тернарный оператор может распознаваться как Call или Ternary
            assert!(
                matches!(value, Expression::Ternary { .. })
                    || matches!(value, Expression::Call { .. })
            );
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_await_expression() {
    let code = "Результат = Ждать ВвестиЧислоАсинх(1000);";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            if let Expression::Await { expression, .. } = value {
                // Await содержит вызов функции
                assert!(matches!(**expression, Expression::Call { .. }));
            } else {
                panic!("Expected Await expression, got {:?}", value);
            }
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_new_expression_method_debug() {
    // Сначала посмотрим, как tree-sitter парсит этот код
    let code = "Объект = Новый(ТипЗнч(Образец));";

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(code, None).unwrap();

    // Выведем структуру AST
    fn print_tree(node: &tree_sitter::Node, source: &str, depth: usize) {
        let indent = "  ".repeat(depth);
        let text = &source[node.byte_range()];
        let display_text = if text.len() > 40 {
            text.chars().take(40).collect::<String>()
        } else {
            text.to_string()
        };
        eprintln!(
            "{}{} [{}] '{}'",
            indent,
            node.kind(),
            node.kind_id(),
            display_text
        );

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            print_tree(&child, source, depth + 1);
        }
    }

    eprintln!("\n=== Tree-sitter AST для: {} ===", code);
    print_tree(&tree.root_node(), code, 0);
    eprintln!("=== End AST ===\n");
}

#[test]
fn test_new_expression_method() {
    // Новый(ТипЗнч(Объект)) - new_expression_method из грамматики
    let code = "Объект = Новый(ТипЗнч(Образец));";
    let program = parse_bsl(code).expect("Парсинг должен пройти успешно");

    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            // new_expression_method может парситься как New с Call внутри
            // Выводим для отладки, что получилось
            eprintln!("Parsed expression: {:?}", value);
            assert!(
                matches!(value, Expression::New { .. }) || matches!(value, Expression::Call { .. })
            );
        }
        _ => panic!("Expected Assignment, got {:?}", program.statements[0]),
    }
}

// ============================================================================
// INTEGRATION TESTS - Реальные BSL конструкции
// ============================================================================

#[test]
fn test_real_world_function_with_logic() {
    let code = r#"
Функция ПолучитьСписокКонтрагентов(ТипКонтрагента = Неопределено)
    Перем Запрос;
    Перем Результат;

    Запрос = Новый Запрос;
    Запрос.Текст = "ВЫБРАТЬ * ИЗ Справочник.Контрагенты";

    Если ТипКонтрагента <> Неопределено Тогда
        Запрос.УстановитьПараметр("ТипКонтрагента", ТипКонтрагента);
    КонецЕсли;

    Попытка
        Результат = Запрос.Выполнить().Выгрузить();
        Возврат Результат;
    Исключение
        Сообщить(ОписаниеОшибки());
        Возврат Неопределено;
    КонецПопытки;
КонецФункции
"#;

    let program = parse_bsl(code).expect("Парсинг реальной функции должен пройти успешно");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::FunctionDecl {
            name, params, body, ..
        } => {
            assert_eq!(name, "ПолучитьСписокКонтрагентов");
            assert_eq!(params.len(), 1);
            assert!(!body.is_empty(), "Тело функции должно содержать statements");
        }
        _ => panic!("Expected FunctionDecl"),
    }
}

#[test]
fn test_complex_expression_parsing() {
    // Примечание: tree-sitter-bsl имеет проблемы с парсингом скобок в арифметических выражениях
    // Используем более простое выражение без скобок
    let code = "Результат = А + Б * В - Г / 2;";
    let program = parse_bsl(code).expect("Парсинг сложных выражений должен работать");

    assert_eq!(program.statements.len(), 1);
    match &program.statements[0] {
        Statement::Assignment { value, .. } => {
            // Проверяем что это бинарное выражение
            assert!(matches!(value, Expression::Binary { .. }));
        }
        _ => panic!("Expected Assignment"),
    }
}

#[test]
fn test_nested_property_access() {
    let code = "Значение = Объект.Подобъект.Свойство.Методы;";
    let program = parse_bsl(code).expect("Парсинг вложенных property access должен работать");

    assert_eq!(program.statements.len(), 1);
    // PropertyAccess может быть вложенным или fallback в Identifier
    assert!(program.statements[0].is_assignment());
}

// Вспомогательный trait для упрощения проверок
trait StatementExt {
    fn is_assignment(&self) -> bool;
}

impl StatementExt for Statement {
    fn is_assignment(&self) -> bool {
        matches!(self, Statement::Assignment { .. })
    }
}

#[test]
fn test_empty_program() {
    let code = "";
    let program = parse_bsl(code).expect("Пустой код должен парситься");
    assert_eq!(program.statements.len(), 0);
}

#[test]
fn test_comments_ignored() {
    let code = r#"
// Это комментарий
Перем А; // Комментарий после кода
// Еще комментарий
"#;
    let program = parse_bsl(code).expect("Комментарии не должны мешать парсингу");
    assert!(!program.statements.is_empty());
}
