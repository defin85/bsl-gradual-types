use super::*;
use crate::domain::flow_analysis::{CfgNode, CfgNodeKind, EdgeKind};
use crate::domain::types::{ConcreteType, PlatformType, TypeResolution};

#[test]
fn test_narrowing_context_new() {
    let ctx = NarrowingContext::new();
    assert!(ctx.narrowed_types.is_empty());
    assert!(ctx.active_guards.is_empty());
    assert!(ctx.parent.is_none());
}

#[test]
fn test_narrowing_context_set_get() {
    let mut ctx = NarrowingContext::new();
    let resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Строка".to_string(),
    }));

    ctx.set_type("x", resolution.clone());
    assert!(ctx.get_type("x").is_some());
}

#[test]
fn test_narrowing_context_child() {
    let mut parent = NarrowingContext::new();
    let resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Число".to_string(),
    }));

    parent.set_type("x", resolution.clone());

    let child = parent.child();
    assert!(child.get_type("x").is_some()); // Должен найти в parent
}

#[test]
fn test_narrowing_context_apply_guard() {
    let mut ctx = NarrowingContext::new();
    let current = TypeResolution::unknown();

    let guard = TypeGuard::TypeCheck {
        variable: "x".to_string(),
        expected_type: "Строка".to_string(),
    };

    ctx.apply_guard(guard.clone(), &current);

    assert_eq!(ctx.active_guards.len(), 1);
    assert!(ctx.get_type("x").is_some());
}

#[test]
fn test_narrowing_engine_narrow_type() {
    let cfg = ControlFlowGraph::new();
    let mut engine = NarrowingEngine::new(cfg);

    let current = TypeResolution::unknown();
    let narrowed = engine.narrow_type(&current, "ТипЗнч(Параметр) = Тип(\"Число\")");

    // Должен сузить до Число
    if let crate::domain::types::ResolutionResult::Concrete(ConcreteType::Platform(pt)) =
        &narrowed.result
    {
        assert_eq!(pt.name, "Число");
    } else {
        panic!("Expected narrowed type to be Число");
    }
}

#[test]
fn test_narrowing_engine_no_guards() {
    let cfg = ControlFlowGraph::new();
    let mut engine = NarrowingEngine::new(cfg);

    let current = TypeResolution::unknown();
    let narrowed = engine.narrow_type(&current, "x > 0"); // Нет type guards

    // Должен вернуть исходный тип
    assert_eq!(format!("{:?}", current), format!("{:?}", narrowed));
}

#[test]
fn test_narrowing_engine_build_contexts() {
    let mut cfg = ControlFlowGraph::new();

    let entry_id = cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::Entry,
    });

    let cond_id = cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::Conditional {
            condition: "ТипЗнч(x) = Тип(\"Строка\")".to_string(),
        },
    });

    let then_id = cfg.add_node(CfgNode {
        id: 2,
        kind: CfgNodeKind::BasicBlock {
            statements: vec!["y = x.Length".to_string()],
        },
    });

    cfg.add_edge(entry_id, cond_id, EdgeKind::Unconditional);
    cfg.add_edge(cond_id, then_id, EdgeKind::ConditionalTrue);

    let mut engine = NarrowingEngine::new(cfg);

    let mut initial_ctx = FlowAnalysisContext::new();
    initial_ctx.set_variable(
        "x",
        TypeResolution::unknown(), // Any
    );

    engine.build_narrowing_contexts(initial_ctx);

    // Проверяем, что в then ветке x имеет тип Строка
    if let Some(then_ctx) = engine.get_context(then_id) {
        if let Some(x_type) = then_ctx.get_type("x") {
            if let crate::domain::types::ResolutionResult::Concrete(ConcreteType::Platform(pt)) =
                &x_type.result
            {
                assert_eq!(pt.name, "Строка");
            } else {
                panic!("Expected x to be narrowed to Строка in then branch");
            }
        } else {
            panic!("Variable x should exist in then branch context");
        }
    } else {
        panic!("Then branch context should exist");
    }
}

#[test]
fn test_narrowing_engine_build_contexts_multiple_entries() {
    let mut cfg = ControlFlowGraph::new();

    // Component A
    let entry_a = cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::Entry,
    });
    let cond_a = cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::Conditional {
            condition: "ТипЗнч(x) = Тип(\"Строка\")".to_string(),
        },
    });
    let then_a = cfg.add_node(CfgNode {
        id: 2,
        kind: CfgNodeKind::BasicBlock {
            statements: vec!["a = x".to_string()],
        },
    });
    cfg.add_edge(entry_a, cond_a, EdgeKind::Unconditional);
    cfg.add_edge(cond_a, then_a, EdgeKind::ConditionalTrue);

    // Component B (separate entry)
    let entry_b = cfg.add_node(CfgNode {
        id: 3,
        kind: CfgNodeKind::Entry,
    });
    let cond_b = cfg.add_node(CfgNode {
        id: 4,
        kind: CfgNodeKind::Conditional {
            condition: "ТипЗнч(x) = Тип(\"Число\")".to_string(),
        },
    });
    let then_b = cfg.add_node(CfgNode {
        id: 5,
        kind: CfgNodeKind::BasicBlock {
            statements: vec!["b = x".to_string()],
        },
    });
    cfg.add_edge(entry_b, cond_b, EdgeKind::Unconditional);
    cfg.add_edge(cond_b, then_b, EdgeKind::ConditionalTrue);

    let mut engine = NarrowingEngine::new(cfg);
    let mut initial = FlowAnalysisContext::new();
    initial.set_variable("x", TypeResolution::unknown());
    engine.build_narrowing_contexts(initial);

    assert!(engine.get_context(then_a).is_some());
    assert!(engine.get_context(then_b).is_some());
}

#[test]
fn test_narrowing_context_merge() {
    let mut ctx1 = NarrowingContext::new();
    let mut ctx2 = NarrowingContext::new();

    ctx1.set_type(
        "x",
        TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Строка".to_string(),
        })),
    );

    ctx2.set_type(
        "x",
        TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Число".to_string(),
        })),
    );

    let mut flow_ctx = FlowAnalysisContext::new();
    ctx1.merge(&ctx2, &mut flow_ctx);

    // После merge x должен иметь union type
    if let Some(merged_type) = flow_ctx.get_variable("x") {
        assert!(matches!(
            merged_type.result,
            crate::domain::types::ResolutionResult::Union(_)
        ));
    } else {
        panic!("Variable x should exist after merge");
    }
}
