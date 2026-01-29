//! Тесты для SemanticProgram

use crate::ir::{SemanticNode, SemanticNodeKind, SemanticProgram, Span};

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
    program.symbols.register_variable(
        child_scope,
        "localVar".to_string(),
        Span::stub(),
    );

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
fn test_find_node_at_position() {
    let mut program = SemanticProgram::new();

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::VariableDeclaration {
            name: "x".to_string(),
            type_hint: Some("Число".to_string()),
            is_export: false,
            initial_value_node: None,
        },
        span: Span::new(1, 0, 1, 15),
        scope_id: program.symbols.root_scope,
    });

    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::Assignment {
            variable: "x".to_string(),
            value_node: None,
        },
        span: Span::new(2, 0, 2, 10),
        scope_id: program.symbols.root_scope,
    });

    // Поиск первого узла
    let node = program.find_node_at_position(1, 5);
    assert!(node.is_some());
    assert!(matches!(
        node.unwrap().kind,
        SemanticNodeKind::VariableDeclaration { .. }
    ));

    // Поиск второго узла
    let node = program.find_node_at_position(2, 5);
    assert!(node.is_some());
    assert!(matches!(
        node.unwrap().kind,
        SemanticNodeKind::Assignment { .. }
    ));

    // Поиск вне узлов
    assert!(program.find_node_at_position(10, 5).is_none());
}
