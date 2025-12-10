//! Тесты для NewExpression

use crate::domain::types::TypeResolution;
use crate::ir::{SemanticNode, SemanticNodeKind, SemanticProgram, Span};

#[test]
fn test_new_expression_simple() {
    let mut program = SemanticProgram::new();

    // Простой конструктор: Новый Массив
    // Phase 3: result_type и arg_types теперь TypeResolution
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NewExpression {
            type_name: "Массив".to_string(),
            result_type: TypeResolution::explicit("Массив"),
            arg_types: vec![],
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
fn test_new_expression_with_args() {
    let mut program = SemanticProgram::new();

    // Конструктор с параметром: Новый Массив(10)
    // Phase 3: result_type и arg_types теперь TypeResolution
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NewExpression {
            type_name: "Массив".to_string(),
            result_type: TypeResolution::explicit("Массив"),
            arg_types: vec![TypeResolution::primitive("Число")],
            generic_params: None,
            is_dynamic: false,
        },
        span: Span::new(1, 0, 1, 16),
        scope_id: program.symbols.root_scope,
    });

    if let SemanticNodeKind::NewExpression { arg_types, .. } = &program.nodes[0].kind {
        assert_eq!(arg_types.len(), 1);
        assert_eq!(arg_types[0].type_name(), "Число");
    } else {
        panic!("Expected NewExpression node");
    }
}

#[test]
fn test_new_expression_dynamic() {
    let mut program = SemanticProgram::new();

    // Динамический конструктор: Новый("СправочникСсылка.Номенклатура")
    // Phase 3: result_type и arg_types теперь TypeResolution
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NewExpression {
            type_name: "СправочникСсылка.Номенклатура".to_string(),
            result_type: TypeResolution::explicit("СправочникСсылка.Номенклатура"),
            arg_types: vec![],
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
    // Phase 3: result_type теперь TypeResolution::generic
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NewExpression {
            type_name: "Массив".to_string(),
            result_type: TypeResolution::generic("Массив", &["Число"], 1.0),
            arg_types: vec![],
            generic_params: Some(vec!["Число".to_string()]),
            is_dynamic: false,
        },
        span: Span::new(1, 0, 1, 20),
        scope_id: program.symbols.root_scope,
    });

    if let SemanticNodeKind::NewExpression {
        result_type,
        generic_params,
        ..
    } = &program.nodes[0].kind
    {
        // Phase 3: result_type теперь TypeResolution
        assert_eq!(result_type.type_name(), "Массив<Число>");
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
    // Phase 3: result_type и arg_types теперь TypeResolution
    program.nodes.push(SemanticNode {
        kind: SemanticNodeKind::NewExpression {
            type_name: "Массив".to_string(),
            result_type: TypeResolution::explicit("Массив"),
            arg_types: vec![TypeResolution::primitive("Число")],
            generic_params: None,
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
    assert_eq!(node_dto.attributes.get("arg_count"), Some(&"1".to_string()));
    assert_eq!(
        node_dto.attributes.get("is_dynamic"),
        Some(&"false".to_string())
    );
}
