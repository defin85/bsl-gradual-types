//! Тесты для NewExpression

use crate::ir::{SemanticNode, SemanticNodeKind, SemanticProgram, Span};

#[test]
fn test_new_expression_simple() {
    let mut program = SemanticProgram::new();

    // Простой конструктор: Новый Массив
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NewExpression {
            type_name: "Массив".to_string(),
            generic_params: None,
            is_dynamic: false,
        },
        span: Span::new(1, 0, 1, 12),
        scope_id: program.symbols.root_scope,
    });

    assert_eq!(program.nodes.len(), 1);

    if let SemanticNodeKind::NewExpression {
        type_name,
        is_dynamic,
        ..
    } = &program.nodes[0].kind
    {
        assert_eq!(type_name, "Массив");
        assert!(!is_dynamic);
    } else {
        panic!("Expected NewExpression node");
    }
}

#[test]
fn test_new_expression_dynamic() {
    let mut program = SemanticProgram::new();

    // Динамический конструктор: Новый("СправочникСсылка.Номенклатура")
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NewExpression {
            type_name: "СправочникСсылка.Номенклатура".to_string(),
            generic_params: None,
            is_dynamic: true,
        },
        span: Span::new(1, 0, 1, 40),
        scope_id: program.symbols.root_scope,
    });

    if let SemanticNodeKind::NewExpression {
        type_name,
        is_dynamic,
        ..
    } = &program.nodes[0].kind
    {
        assert_eq!(type_name, "СправочникСсылка.Номенклатура");
        assert!(is_dynamic);
    } else {
        panic!("Expected NewExpression node");
    }
}

#[test]
fn test_new_expression_with_generics() {
    let mut program = SemanticProgram::new();

    // Generic конструктор: Новый Массив<Число>
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NewExpression {
            type_name: "Массив".to_string(),
            generic_params: Some(vec!["Число".to_string()]),
            is_dynamic: false,
        },
        span: Span::new(1, 0, 1, 20),
        scope_id: program.symbols.root_scope,
    });

    if let SemanticNodeKind::NewExpression {
        generic_params,
        ..
    } = &program.nodes[0].kind
    {
        assert!(generic_params.is_some());
        let params = generic_params.as_ref().unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "Число");
    } else {
        panic!("Expected NewExpression node");
    }
}

#[test]
fn test_new_expression_to_dto() {
    let mut program = SemanticProgram::new();

    // Добавляем NewExpression узел
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NewExpression {
            type_name: "Массив".to_string(),
            generic_params: Some(vec!["Число".to_string()]),
            is_dynamic: false,
        },
        span: Span::new(1, 0, 1, 16),
        scope_id: program.symbols.root_scope,
    });

    // Конвертируем в DTO
    let dto = program.to_dto(false, false);

    assert_eq!(dto.root_nodes.len(), 1);

    let node_dto = &dto.root_nodes[0];
    assert_eq!(node_dto.kind, "NewExpression");
    assert!(node_dto.name.is_some());

    // Проверяем атрибуты
    assert_eq!(
        node_dto.attributes.get("type_name"),
        Some(&"Массив".to_string())
    );
    assert_eq!(
        node_dto.attributes.get("is_dynamic"),
        Some(&"false".to_string())
    );
    assert_eq!(
        node_dto.attributes.get("generic_params"),
        Some(&"Число".to_string())
    );
}
