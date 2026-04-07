use super::*;
use crate::ast::{Expression, Statement};
use crate::tree_sitter_adapter::span::LineIndex;
use bsl_shared::domain::types::ErrorType;

#[test]
fn parse_valid_code_has_no_syntax_errors() {
    let source = "Процедура Тест()\nКонецПроцедуры";
    let result = parse(source, &ParseOptions::default()).unwrap();
    assert!(result.syntax_errors.is_empty());
    assert!(!result.program.statements.is_empty());
}

#[test]
fn parse_missing_semicolons_reports_heuristic_errors() {
    let source = "Функция Тест()\n    МассивДанных = Новый Массив()\n    МассивДанных.Добавить(42)\n    Возврат МассивДанных\nКонецФункции";
    let result = parse(source, &ParseOptions::default()).unwrap();

    assert!(result.has_errors());
    assert!(result.syntax_errors.iter().any(|error| {
        error.message.contains("точка с запятой") || error.message.contains("точки с запятой")
    }));
}

#[test]
fn parse_broken_code_returns_partial_result_with_errors() {
    let source = "Процедура Тест(\nКонецПроцедуры";
    let result = parse(source, &ParseOptions::default()).unwrap();
    assert!(!result.syntax_errors.is_empty());
}

#[test]
fn parse_reports_byte_spans_for_incomplete_new_with_emoji_prefix() {
    let source = "y = \"😀\"; x = Новый\nz = 1";
    let result = parse(source, &ParseOptions::default()).unwrap();

    let err = result
        .syntax_errors
        .iter()
        .find(|e| e.message.contains("Отсутствует тип после 'Новый'"))
        .expect("expected incomplete 'Новый' diagnostic");

    let line = "y = \"😀\"; x = Новый";
    let start_of_line = source.find(line).expect("line offset");
    let expected_offset = (start_of_line + line.len()) as u32;

    assert_eq!(err.span.start, expected_offset);
    assert_eq!(err.span.end, expected_offset);
}

#[test]
fn parse_incomplete_member_access_preserves_receiver_expression() {
    let source = "Процедура Тест()\n    ДляCompletion = Map[\"k\"].\nКонецПроцедуры";
    let result = parse(source, &ParseOptions::default()).unwrap();

    let Statement::ProcedureDecl { body, .. } = &result.program.statements[0] else {
        panic!(
            "expected procedure declaration, got: {:?}",
            result.program.statements
        );
    };
    let Statement::Assignment { value, .. } = &body[0] else {
        panic!("expected assignment, got: {:?}", body);
    };

    match value {
        Expression::IndexAccess { object, index, .. } => {
            assert!(
                matches!(object.as_ref(), Expression::Identifier { name, .. } if name == "Map")
            );
            assert!(matches!(
                index.as_ref(),
                Expression::String { value, .. } if value == "k"
            ));
        }
        other => panic!(
            "incomplete member access must preserve receiver expression, got: {:?}",
            other
        ),
    }
}

#[test]
fn parse_is_deterministic_for_same_input() {
    let source = "Процедура Тест(\nКонецПроцедуры";
    let a = parse(source, &ParseOptions::default()).unwrap();
    let b = parse(source, &ParseOptions::default()).unwrap();

    let project =
        |e: &bsl_shared::domain::types::ParseError| (e.error_type, e.message.clone(), e.span);

    let a_projected: Vec<_> = a.syntax_errors.iter().map(project).collect();
    let b_projected: Vec<_> = b.syntax_errors.iter().map(project).collect();

    assert_eq!(a_projected, b_projected);
}

#[test]
fn parse_rewrites_for_step_clause_span_to_step_keyword() {
    let source = "Для Индекс = 10 По 0 Шаг -1 Цикл\nКонецЦикла";
    let result = parse(source, &ParseOptions::default()).unwrap();

    let err = result
        .syntax_errors
        .iter()
        .find(|e| e.message.contains("Шаг <expr>") || e.message.contains("Шаг"))
        .expect("expected rewritten diagnostic for `Шаг` clause");

    assert_eq!(err.error_type, ErrorType::InvalidSyntax);
    let step_offset = source.find("Шаг").expect("Шаг offset") as u32;
    assert_eq!(err.span.start, step_offset);
    assert_eq!(err.span.end, step_offset + "Шаг".len() as u32);
}

#[test]
fn parse_valid_for_header_has_no_syntax_errors() {
    let source = "Для Индекс = 10 По 0 Цикл\nКонецЦикла";
    let result = parse(source, &ParseOptions::default()).unwrap();
    assert!(
        result.syntax_errors.is_empty(),
        "expected no syntax errors, got: {:?}",
        result.syntax_errors
    );
}

#[test]
fn parse_if_without_then_reports_single_helpful_error_on_header_line() {
    let source = "Если x = 1\n    Сообщить(x);\nКонецЕсли;";
    let result = parse(source, &ParseOptions::default()).unwrap();
    assert!(!result.syntax_errors.is_empty());

    let line_index = LineIndex::new(source);

    let err = result
        .syntax_errors
        .iter()
        .find(|e| e.message.contains("Тогда"))
        .expect("expected rewritten diagnostic mentioning `Тогда`");

    assert_eq!(err.error_type, ErrorType::InvalidSyntax);

    let (line, _) = line_index.byte_offset_to_point(source, err.span.start as usize);
    assert_eq!(line, 0, "expected error to point to `Если` header line");

    let header = "Если x = 1";
    let expected = header.len() as u32;
    assert_eq!(err.span.start, expected);
    assert_eq!(err.span.end, expected);

    let errors_on_header_line = result
        .syntax_errors
        .iter()
        .filter(|e| {
            line_index
                .byte_offset_to_point(source, e.span.start as usize)
                .0
                == 0
        })
        .count();
    assert_eq!(errors_on_header_line, 1);
}

#[test]
fn parse_unclosed_try_is_rewritten_and_preserves_related_info() {
    let source = "Попытка\n    Сообщить(1);\nИсключение\n    Сообщить(2);\nСообщить(3);\n";
    let result = parse(source, &ParseOptions::default()).unwrap();
    assert!(!result.syntax_errors.is_empty());

    let line_index = LineIndex::new(source);

    let err = result
        .syntax_errors
        .iter()
        .find(|e| e.message.contains("КонецПопытки"))
        .unwrap_or_else(|| {
            panic!(
                "expected rewritten diagnostic for unclosed `Попытка`, got: {:?}",
                result.syntax_errors
            )
        });

    assert_eq!(err.error_type, ErrorType::MissingToken);
    assert!(
        err.related
            .iter()
            .any(|r| r.message.contains("Начало блока: Попытка")),
        "expected related info to include try start, got: {:?}",
        err.related
    );

    let (line, _) = line_index.byte_offset_to_point(source, err.span.start as usize);
    assert_eq!(line, 0);
    assert_eq!(err.span.start, 0);
    assert_eq!(err.span.end, "Попытка".len() as u32);

    let related = err
        .related
        .iter()
        .find(|r| r.message.contains("Начало блока: Попытка"))
        .expect("related try start");
    assert_eq!(related.span.start, 0);
    assert_eq!(related.span.end, "Попытка".len() as u32);

    let errors_on_try_line = result
        .syntax_errors
        .iter()
        .filter(|e| {
            line_index
                .byte_offset_to_point(source, e.span.start as usize)
                .0
                == 0
        })
        .count();
    assert_eq!(errors_on_try_line, 1);
}

#[test]
fn syntax_errors_only_matches_parse_for_broken_code() {
    let source = "Процедура Тест(\nКонецПроцедуры";
    let parsed = parse(source, &ParseOptions::default()).unwrap();
    let syntax_only = syntax_errors_only(source).unwrap();

    let project =
        |e: &bsl_shared::domain::types::ParseError| (e.error_type, e.message.clone(), e.span);

    let parsed_projected: Vec<_> = parsed.syntax_errors.iter().map(project).collect();
    let syntax_only_projected: Vec<_> = syntax_only.iter().map(project).collect();
    assert_eq!(syntax_only_projected, parsed_projected);
}

#[test]
fn syntax_errors_only_keeps_heuristic_semicolon_diagnostics() {
    let source = "Функция Тест()\n    МассивДанных = Новый Массив()\n    Возврат 1\nКонецФункции";
    let syntax_only = syntax_errors_only(source).unwrap();
    assert!(syntax_only.iter().any(|error| {
        error.message.contains("точка с запятой") || error.message.contains("точки с запятой")
    }));
}

#[test]
fn parse_strict_line_cap_keeps_parser_error_over_heuristics_on_same_line() {
    let source = "Процедура Тест()\n    x = (1 + ) = Новый\n    y = 1;\nКонецПроцедуры";
    let result = parse(source, &ParseOptions::default()).unwrap();
    assert!(!result.syntax_errors.is_empty());

    let line_index = LineIndex::new(source);

    let line_with_errors = 1usize;
    let errors_on_line = result
        .syntax_errors
        .iter()
        .filter(|e| {
            line_index
                .byte_offset_to_point(source, e.span.start as usize)
                .0
                == line_with_errors
        })
        .collect::<Vec<_>>();

    assert_eq!(
        errors_on_line.len(),
        1,
        "expected strict line-cap to keep 1 error on line, got: {:?}",
        result.syntax_errors
    );

    let err = errors_on_line[0];
    assert!(
        !err.message
            .starts_with("Отсутствует точка с запятой после оператора '"),
        "semicolon heuristic should be suppressed, got: {:?}",
        err
    );
    assert_ne!(
        err.message, "Отсутствует тип после 'Новый'",
        "incomplete `Новый` heuristic should be suppressed"
    );
}

#[test]
fn parse_general_for_rule_points_to_first_garbage_token_and_says_expected_loop() {
    let source = "Для i = 10 По 0 abc def Цикл\nКонецЦикла";
    let result = parse(source, &ParseOptions::default()).unwrap();
    assert!(!result.syntax_errors.is_empty());

    let err = result
        .syntax_errors
        .iter()
        .find(|e| e.message.contains("ожидается `Цикл`"))
        .expect("expected generalized `Для` diagnostic");

    assert_eq!(err.error_type, ErrorType::InvalidSyntax);
    let abc_offset = source.find("abc").expect("abc offset") as u32;
    assert_eq!(err.span.start, abc_offset);
    assert_eq!(err.span.end, abc_offset + "abc".len() as u32);
}

#[test]
fn parse_step_word_inside_string_does_not_trigger_step_clause_rule() {
    let source = "Для i = 10 По 0 \"Шаг\" -1 Цикл\nКонецЦикла";
    let result = parse(source, &ParseOptions::default()).unwrap();
    assert!(!result.syntax_errors.is_empty());

    assert!(
        !result
            .syntax_errors
            .iter()
            .any(|e| e.message.contains("нет синтаксиса `Шаг <expr>`")),
        "step-clause rewrite should not trigger for `Шаг` inside string, got: {:?}",
        result.syntax_errors
    );
}
