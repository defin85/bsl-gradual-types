//! Тесты для SemanticProgram

use crate::ir::{SemanticNode, SemanticNodeKind, SemanticProgram, Span};
use bsl_line_index::LineIndex;

#[test]
fn test_variable_resolution() {
    let mut program = SemanticProgram::new();
    let child_scope = program.symbols.create_scope(program.symbols.root_scope);

    // Регистрируем переменную в root scope
    program.symbols.register_variable(
        program.symbols.root_scope,
        "globalVar".to_string(),
        Span::stub(),
    );

    // Регистрируем переменную в child scope
    program
        .symbols
        .register_variable(child_scope, "localVar".to_string(), Span::stub());

    // Поиск в child scope должен найти обе переменные
    assert!(program.resolve_variable("localVar", child_scope).is_some());
    assert!(program.resolve_variable("globalVar", child_scope).is_some());

    // Поиск в root scope должен найти только globalVar
    assert!(program
        .resolve_variable("globalVar", program.symbols.root_scope)
        .is_some());
    assert!(program
        .resolve_variable("localVar", program.symbols.root_scope)
        .is_none());
}

#[test]
fn test_find_node_at_byte_offset() {
    let mut program = SemanticProgram::new();

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::VariableDeclaration {
            name: "x".to_string(),
            type_hint: Some("Число".to_string()),
            is_export: false,
            initial_value_node: None,
        },
        span: Span::new(0, 10),
        scope_id: program.symbols.root_scope,
    });

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::Assignment {
            variable: "x".to_string(),
            value_node: None,
            value_span: Span::stub(),
        },
        span: Span::new(20, 30),
        scope_id: program.symbols.root_scope,
    });

    // Поиск первого узла
    let node = program.find_node_at_byte_offset(5);
    assert!(node.is_some());
    assert!(matches!(
        node.unwrap().kind,
        SemanticNodeKind::VariableDeclaration { .. }
    ));

    // Поиск второго узла
    let node = program.find_node_at_byte_offset(25);
    assert!(node.is_some());
    assert!(matches!(
        node.unwrap().kind,
        SemanticNodeKind::Assignment { .. }
    ));

    // Поиск вне узлов
    assert!(program.find_node_at_byte_offset(15).is_none());
}

#[test]
fn test_if_statement_children_include_condition_node_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "Если Истина Тогда\nКонецЕсли";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::BooleanLiteral { value: true },
        span: Span::new(5, 11),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::IfStatement {
            condition_node: Some(0),
            then_branch: Vec::new(),
            else_branch: None,
        },
        span: Span::new(0, source.len() as u32),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let if_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "IfStatement")
        .expect("if root node");
    assert_eq!(if_node.children.len(), 1);
    assert_eq!(if_node.children[0].kind, "BooleanLiteral");
}

#[test]
fn test_while_loop_children_include_condition_node_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "Пока Истина Цикл\nКонецЦикла";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::BooleanLiteral { value: true },
        span: Span::new(5, 11),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::WhileLoop {
            condition_node: Some(0),
            body: Vec::new(),
        },
        span: Span::new(0, source.len() as u32),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let while_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "WhileLoop")
        .expect("while root node");
    assert_eq!(while_node.children.len(), 1);
    assert_eq!(while_node.children[0].kind, "BooleanLiteral");
}

#[test]
fn test_for_loop_children_include_range_nodes_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "Для Счетчик = 1 По 10 Цикл\nКонецЦикла";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NumberLiteral { value: 1.0 },
        span: Span::new(14, 15),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NumberLiteral { value: 10.0 },
        span: Span::new(19, 21),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::ForLoop {
            variable: "Счетчик".to_string(),
            start_node: Some(0),
            end_node: Some(1),
            body: Vec::new(),
        },
        span: Span::new(0, source.len() as u32),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let for_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "ForLoop")
        .expect("for root node");
    assert_eq!(for_node.children.len(), 2);
    assert_eq!(for_node.children[0].kind, "NumberLiteral");
    assert_eq!(for_node.children[1].kind, "NumberLiteral");
}

#[test]
fn test_foreach_loop_children_include_collection_node_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "Для Каждого Элемент Из Коллекция Цикл\nКонецЦикла";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::VariableAccess {
            name: "Коллекция".to_string(),
        },
        span: Span::new(24, 33),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::ForEachLoop {
            variable: "Элемент".to_string(),
            collection_node: Some(0),
            body: Vec::new(),
        },
        span: Span::new(0, source.len() as u32),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let foreach_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "ForEachLoop")
        .expect("foreach root node");
    assert_eq!(foreach_node.children.len(), 1);
    assert_eq!(foreach_node.children[0].kind, "VariableAccess");
}

#[test]
fn test_unary_expression_children_include_operand_node_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "-42";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NumberLiteral { value: 42.0 },
        span: Span::new(1, 3),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::UnaryExpression {
            operator: "-".to_string(),
            operand_node: Some(0),
        },
        span: Span::new(0, 3),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let unary_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "UnaryExpression")
        .expect("unary root node");
    assert_eq!(unary_node.children.len(), 1);
    assert_eq!(unary_node.children[0].kind, "NumberLiteral");
}

#[test]
fn test_ternary_expression_children_include_all_branches_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "?(Истина, 1, 2)";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::BooleanLiteral { value: true },
        span: Span::new(2, 8),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NumberLiteral { value: 1.0 },
        span: Span::new(10, 11),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NumberLiteral { value: 2.0 },
        span: Span::new(13, 14),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::TernaryExpression {
            condition_node: Some(0),
            then_node: Some(1),
            else_node: Some(2),
        },
        span: Span::new(0, source.len() as u32),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let ternary_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "TernaryExpression")
        .expect("ternary root node");
    assert_eq!(ternary_node.children.len(), 3);
    assert_eq!(ternary_node.children[0].kind, "BooleanLiteral");
    assert_eq!(ternary_node.children[1].kind, "NumberLiteral");
    assert_eq!(ternary_node.children[2].kind, "NumberLiteral");
}

#[test]
fn test_execute_statement_children_include_code_node_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "Выполнить(\"Код\")";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::StringLiteral {
            value: "Код".to_string(),
        },
        span: Span::new(10, 15),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::ExecuteStatement { code_node: Some(0) },
        span: Span::new(0, source.len() as u32),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let execute_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "ExecuteStatement")
        .expect("execute root node");
    assert_eq!(execute_node.children.len(), 1);
    assert_eq!(execute_node.children[0].kind, "StringLiteral");
}

#[test]
fn test_add_handler_statement_children_include_event_and_handler_nodes_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "ДобавитьОбработчик Событие, Обработчик";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::VariableAccess {
            name: "Событие".to_string(),
        },
        span: Span::new(19, 26),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::VariableAccess {
            name: "Обработчик".to_string(),
        },
        span: Span::new(28, 38),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::AddHandlerStatement {
            event_node: Some(0),
            handler_node: Some(1),
        },
        span: Span::new(0, source.len() as u32),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let add_handler_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "AddHandlerStatement")
        .expect("add handler root node");
    assert_eq!(add_handler_node.children.len(), 2);
    assert_eq!(add_handler_node.children[0].kind, "VariableAccess");
    assert_eq!(add_handler_node.children[1].kind, "VariableAccess");
}

#[test]
fn test_raise_error_statement_children_include_message_node_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "ВызватьИсключение(\"Ошибка\")";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::StringLiteral {
            value: "Ошибка".to_string(),
        },
        span: Span::new(18, 26),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::RaiseErrorStatement {
            message_node: Some(0),
        },
        span: Span::new(0, source.len() as u32),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let raise_error_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "RaiseErrorStatement")
        .expect("raise error root node");
    assert_eq!(raise_error_node.children.len(), 1);
    assert_eq!(raise_error_node.children[0].kind, "StringLiteral");
}

#[test]
fn test_remove_handler_statement_children_include_event_and_handler_nodes_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "УдалитьОбработчик Событие, Обработчик";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::VariableAccess {
            name: "Событие".to_string(),
        },
        span: Span::new(18, 25),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::VariableAccess {
            name: "Обработчик".to_string(),
        },
        span: Span::new(27, 37),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::RemoveHandlerStatement {
            event_node: Some(0),
            handler_node: Some(1),
        },
        span: Span::new(0, source.len() as u32),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let remove_handler_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "RemoveHandlerStatement")
        .expect("remove handler root node");
    assert_eq!(remove_handler_node.children.len(), 2);
    assert_eq!(remove_handler_node.children[0].kind, "VariableAccess");
    assert_eq!(remove_handler_node.children[1].kind, "VariableAccess");
}

#[test]
fn test_await_statement_children_include_expression_node_in_dto() {
    let mut program = SemanticProgram::new();
    let source = "Ждать АсинхронныйВызов";

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::VariableAccess {
            name: "АсинхронныйВызов".to_string(),
        },
        span: Span::new(6, 22),
        scope_id: program.symbols.root_scope,
    });
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::AwaitStatement {
            expression_node: Some(0),
        },
        span: Span::new(0, source.len() as u32),
        scope_id: program.symbols.root_scope,
    });

    let line_index = LineIndex::new(source);
    let dto = program.to_dto(false, false, source, &line_index);

    let await_node = dto
        .root_nodes
        .iter()
        .find(|node| node.kind == "AwaitStatement")
        .expect("await root node");
    assert_eq!(await_node.children.len(), 1);
    assert_eq!(await_node.children[0].kind, "VariableAccess");
}
