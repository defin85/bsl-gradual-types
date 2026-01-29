/// Интеграционный тест для проверки Span Extraction (Milestone 2.11)
///
/// Проверяет:
/// - TreeSitter парсер извлекает span корректно
/// - AST → IR конверсия передаёт span для всех Statement типов
/// - find_node_at_byte_offset() находит узлы по byte offset
use bsl_analysis_v2::AstToIrConverter;
use bsl_backend::system::parser_coordinator::ParserCoordinator;
use bsl_backend::system::LineIndex;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::signature_index::SignatureIndex;
use std::sync::Arc;

#[test]
fn test_span_extraction_from_tree_sitter() {
    // Arrange: Простой BSL код с известными позициями
    let code = r#"Функция Тест()
    Перем А;
    А = 42;
    Возврат А;
КонецФункции"#;

    let parser = ParserCoordinator::with_fallback();

    // Act: Парсинг через TreeSitter
    let parse_result = parser.parse(code).expect("Парсинг должен пройти успешно");

    // Assert: Проверяем, что span не пустой
    assert!(
        !parse_result.program.statements.is_empty(),
        "Должна быть хотя бы одна функция"
    );

    // Первый statement - объявление функции
    if let Some(first_stmt) = parse_result.program.statements.first() {
        let span = match first_stmt {
            bsl_backend::parsing::bsl::ast::Statement::FunctionDecl { span, .. } => span,
            _ => panic!("Первый statement должен быть FunctionDecl"),
        };

        // Для первого узла start может быть 0 (начало файла), поэтому проверяем непустой диапазон.
        assert!(
            span.end > span.start,
            "Span должен быть непустым (end > start), получен: {:?}",
            span
        );

        println!("✅ Span извлечён из tree-sitter: {:?}", span);
    }
}

#[test]
fn test_span_propagation_ast_to_ir() {
    // Arrange: BSL код
    let code = r#"Функция Тест()
    Перем А;
    Перем Б;
    А = Справочники.Контрагенты;
    Б = Новый Массив;
    Возврат А;
КонецФункции"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(code).expect("Парсинг должен пройти успешно");

    // Act: Конвертация AST → IR
    let repository = Arc::new(InMemoryTypeRepository::new());
    let signature_index = SignatureIndex::new();
    let ir_program = AstToIrConverter::convert(
        parse_result.program,
        code.to_string(),
        "test_span_propagation.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия AST → IR должна пройти успешно");

    // Assert: Проверяем, что span передан в IR для разных типов Statement
    let nodes_with_real_spans: Vec<_> = ir_program
        .nodes
        .iter()
        .filter(|node| node.span.end > node.span.start)
        .collect();

    println!(
        "✅ Узлов с реальными span в IR: {}",
        nodes_with_real_spans.len()
    );
    for (idx, node) in nodes_with_real_spans.iter().enumerate() {
        println!("   - Узел {}: span = {:?}", idx + 1, node.span);
    }

    assert!(
        !nodes_with_real_spans.is_empty(),
        "Должны быть узлы с реальными span в IR"
    );

    // Проверяем, что минимум половина узлов имеет реальные span
    let total_nodes = ir_program.nodes.len();
    let nodes_with_spans = nodes_with_real_spans.len();
    assert!(
        nodes_with_spans * 2 >= total_nodes,
        "Большинство узлов должно иметь реальный span ({}/{})",
        nodes_with_spans,
        total_nodes
    );
}

#[test]
fn test_find_node_at_byte_offset() {
    // Arrange: BSL код с известными позициями
    let code = r#"Функция Тест()
    Перем СправочникКонтрагенты;
    СправочникКонтрагенты = Справочники.Контрагенты;
    Возврат СправочникКонтрагенты;
КонецФункции"#;

    let index = LineIndex::new(code);

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(code).expect("Парсинг должен пройти успешно");

    let repository = Arc::new(InMemoryTypeRepository::new());
    let signature_index = SignatureIndex::new();
    let ir_program = AstToIrConverter::convert(
        parse_result.program,
        code.to_string(),
        "test_find_node.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия AST → IR должна пройти успешно");

    // Act: Поиск узла по позиции в строке 2 (Перем СправочникКонтрагенты)
    // Линии/колонки в LSP 0-based и `Position.character` измеряется в UTF-16 code units.
    let byte_offset_1 = index.utf16_position_to_byte_offset(code, 1, 10) as u32;
    let node_at_line_1 = ir_program.find_node_at_byte_offset(byte_offset_1); // "Перем" на строке 1

    // Assert: Должен найти узел
    assert!(
        node_at_line_1.is_some(),
        "Должен найти узел VarDeclaration на позиции (1, 10)"
    );

    if let Some(node) = node_at_line_1 {
        println!("✅ find_node_at_byte_offset(...) нашёл узел:");
        println!("   Span: {:?}", node.span);
        println!("   Scope ID: {}", node.scope_id.0);

        // Проверяем, что span действительно содержит позицию (1, 10)
        assert!(
            node.span.contains(byte_offset_1),
            "Span должен содержать byte offset {}",
            byte_offset_1
        );
    }

    // Act: Поиск узла в строке 2 (присваивание)
    let byte_offset_2 = index.utf16_position_to_byte_offset(code, 2, 10) as u32;
    let node_at_line_2 = ir_program.find_node_at_byte_offset(byte_offset_2);

    // Assert: Должен найти узел присваивания
    assert!(
        node_at_line_2.is_some(),
        "Должен найти узел Assignment на позиции (2, 10)"
    );

    if let Some(node) = node_at_line_2 {
        println!("✅ find_node_at_byte_offset(...) нашёл узел:");
        println!("   Span: {:?}", node.span);
        println!("   Scope ID: {}", node.scope_id.0);

        // Проверяем, что span действительно содержит позицию (2, 10)
        assert!(
            node.span.contains(byte_offset_2),
            "Span должен содержать byte offset {}",
            byte_offset_2
        );
    }
}

#[test]
fn test_span_for_all_statement_types() {
    // Arrange: BSL код с разными типами Statement
    let code = r#"Функция ТестВсехОператоров()
    Перем А;
    А = 42;

    Если А > 0 Тогда
        А = А + 1;
    КонецЕсли;

    Пока А < 100 Цикл
        А = А * 2;
    КонецЦикла;

    Для Сч = 1 По 10 Цикл
        А = А + Сч;
    КонецЦикла;

    Возврат А;
КонецФункции"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(code).expect("Парсинг должен пройти успешно");

    let repository = Arc::new(InMemoryTypeRepository::new());
    let signature_index = SignatureIndex::new();
    let ir_program = AstToIrConverter::convert(
        parse_result.program,
        code.to_string(),
        "test_all_statements.bsl".to_string(),
        repository,
        signature_index,
    )
    .expect("Конверсия AST → IR должна пройти успешно");

    // Assert: Проверяем наличие span для разных типов узлов
    let nodes_with_real_spans: Vec<_> = ir_program
        .nodes
        .iter()
        .filter(|node| node.span.end > node.span.start)
        .collect();

    println!("✅ Проверка span для разных типов Statement:");
    println!("   Всего узлов: {}", ir_program.nodes.len());
    println!("   Узлов с реальными span: {}", nodes_with_real_spans.len());

    // Проверяем образцы span
    for (idx, node) in nodes_with_real_spans.iter().take(5).enumerate() {
        println!("   - Узел {}: span = {:?}", idx + 1, node.span);
    }

    // Минимум 80% узлов должно иметь реальные span
    let total_nodes = ir_program.nodes.len();
    let nodes_with_spans = nodes_with_real_spans.len();
    assert!(
        nodes_with_spans * 100 / total_nodes >= 80,
        "Минимум 80% узлов должно иметь реальный span, получено {}/{} ({}%)",
        nodes_with_spans,
        total_nodes,
        nodes_with_spans * 100 / total_nodes
    );
}
