//! Integration tests for flow-sensitive analysis

use bsl_shared::domain::types::{ConcreteType, PlatformType, ResolutionResult, TypeResolution};
use bsl_shared::domain::{ControlFlowGraph, FlowAnalysisContext};

#[test]
fn test_simple_variable_assignment() {
    let mut context = FlowAnalysisContext::new();

    // Присваивание: x = 42
    let number_type = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Число".to_string(),
    }));

    context.set_variable("x".to_string(), number_type.clone());

    // Проверяем, что переменная имеет правильный тип
    assert!(context.get_variable("x").is_some());
    if let Some(x_type) = context.get_variable("x") {
        assert!(matches!(x_type.result, ResolutionResult::Concrete(_)));
    }
}

#[test]
fn test_variable_reassignment() {
    let mut context = FlowAnalysisContext::new();

    // Первое присваивание: x = 42
    let number_type = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Число".to_string(),
    }));
    context.set_variable("x".to_string(), number_type);

    // Второе присваивание: x = "текст"
    let string_type = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Строка".to_string(),
    }));
    context.set_variable("x".to_string(), string_type.clone());

    // Проверяем, что тип переменной изменился
    if let Some(x_type) = context.get_variable("x") {
        if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &x_type.result {
            assert_eq!(pt.name, "Строка");
        } else {
            panic!("Expected Concrete(Platform) type");
        }
    }
}

#[test]
fn test_conditional_branching_union_type() {
    let mut context = FlowAnalysisContext::new();

    // Создаём две ветки условия
    let mut then_context = context.fork();
    let mut else_context = context.fork();

    // В then ветке: x = "текст"
    let string_type = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Строка".to_string(),
    }));
    then_context.set_variable("x".to_string(), string_type);

    // В else ветке: x = 42
    let number_type = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Число".to_string(),
    }));
    else_context.set_variable("x".to_string(), number_type);

    // Объединяем контексты
    then_context.merge(&else_context);

    // Проверяем, что получился union type
    if let Some(x_type) = then_context.get_variable("x") {
        assert!(matches!(x_type.result, ResolutionResult::Union(_)));

        if let ResolutionResult::Union(types) = &x_type.result {
            assert_eq!(types.len(), 2);
        }
    } else {
        panic!("Variable x should exist after merge");
    }
}

#[test]
fn test_scope_tracking() {
    let mut context = FlowAnalysisContext::new();

    assert_eq!(context.get_scope_depth(), 0);

    context.enter_scope();
    assert_eq!(context.get_scope_depth(), 1);

    context.enter_scope();
    assert_eq!(context.get_scope_depth(), 2);

    context.exit_scope();
    assert_eq!(context.get_scope_depth(), 1);

    context.exit_scope();
    assert_eq!(context.get_scope_depth(), 0);
}

#[test]
fn test_cfg_basic_blocks() {
    let mut cfg = ControlFlowGraph::new();

    use bsl_shared::domain::{CfgNode, CfgNodeKind, EdgeKind};

    // Entry узел
    let entry_id = cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::Entry,
        context_in: None,
        context_out: None,
    });

    // Basic block
    let block_id = cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::BasicBlock {
            statements: vec!["x = 42".to_string()],
        },
        context_in: None,
        context_out: None,
    });

    // Exit узел
    let exit_id = cfg.add_node(CfgNode {
        id: 2,
        kind: CfgNodeKind::Exit,
        context_in: None,
        context_out: None,
    });

    // Связываем узлы
    cfg.add_edge(entry_id, block_id, EdgeKind::Unconditional);
    cfg.add_edge(block_id, exit_id, EdgeKind::Unconditional);

    assert_eq!(cfg.nodes().len(), 3);
    assert_eq!(cfg.edges().len(), 2);
}

#[test]
fn test_cfg_conditional_flow() {
    let mut cfg = ControlFlowGraph::new();

    use bsl_shared::domain::{CfgNode, CfgNodeKind, EdgeKind};

    // Entry
    let entry_id = cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::Entry,
        context_in: None,
        context_out: None,
    });

    // Conditional узел (if condition)
    let cond_id = cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::Conditional {
            condition: "x > 0".to_string(),
        },
        context_in: None,
        context_out: None,
    });

    // Then блок
    let then_id = cfg.add_node(CfgNode {
        id: 2,
        kind: CfgNodeKind::BasicBlock {
            statements: vec!["y = 1".to_string()],
        },
        context_in: None,
        context_out: None,
    });

    // Else блок
    let else_id = cfg.add_node(CfgNode {
        id: 3,
        kind: CfgNodeKind::BasicBlock {
            statements: vec!["y = 0".to_string()],
        },
        context_in: None,
        context_out: None,
    });

    // Merge узел
    let merge_id = cfg.add_node(CfgNode {
        id: 4,
        kind: CfgNodeKind::BasicBlock {
            statements: vec!["merge".to_string()],
        },
        context_in: None,
        context_out: None,
    });

    // Exit
    let exit_id = cfg.add_node(CfgNode {
        id: 5,
        kind: CfgNodeKind::Exit,
        context_in: None,
        context_out: None,
    });

    // Связываем узлы
    cfg.add_edge(entry_id, cond_id, EdgeKind::Unconditional);
    cfg.add_edge(cond_id, then_id, EdgeKind::ConditionalTrue);
    cfg.add_edge(cond_id, else_id, EdgeKind::ConditionalFalse);
    cfg.add_edge(then_id, merge_id, EdgeKind::Unconditional);
    cfg.add_edge(else_id, merge_id, EdgeKind::Unconditional);
    cfg.add_edge(merge_id, exit_id, EdgeKind::Unconditional);

    assert_eq!(cfg.nodes().len(), 6);
    assert_eq!(cfg.edges().len(), 6);
}

#[test]
fn test_flow_event_history() {
    let mut context = FlowAnalysisContext::new();

    let type1 = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Число".to_string(),
    }));
    context.set_variable("x".to_string(), type1);

    context.enter_scope();

    let type2 = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Строка".to_string(),
    }));
    context.set_variable("y".to_string(), type2);

    context.exit_scope();

    let history = context.get_history();
    assert!(!history.is_empty());

    // Должно быть 4 события: Assignment(x), EnterScope, Assignment(y), ExitScope
    assert!(history.len() >= 4);
}
