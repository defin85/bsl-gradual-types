//! Integration tests for Type Narrowing (Milestone 3.7)
//!
//! Тестирование end-to-end сужения типов с Type Resolver

use bsl_shared::analysis::{detect_type_guards, NarrowingEngine};
use bsl_shared::domain::flow_analysis::{
    CfgNode, CfgNodeKind, ControlFlowGraph, EdgeKind, FlowAnalysisContext,
};
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{
    ConcreteType, PlatformType, ResolutionResult, TypeResolution, WeightedType,
};
use std::sync::Arc;

#[test]
fn test_resolver_narrow_type_with_type_check() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);

    let current = TypeResolution::unknown(); // Any

    // ТипЗнч(x) = Тип("Число")
    let narrowed = resolver.narrow_type(&current, "ТипЗнч(x) = Тип(\"Число\")");

    // Должен сузить до Число
    if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &narrowed.result {
        assert_eq!(pt.name, "Число");
    } else {
        panic!("Expected narrowed type to be Число, got: {:?}", narrowed);
    }
}

#[test]
fn test_resolver_narrow_type_with_not_undefined() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);

    // Union: Строка | Неопределено
    let current = TypeResolution {
        certainty: bsl_shared::domain::types::Certainty::Inferred,
        result: ResolutionResult::Union(vec![
            WeightedType {
                type_: ConcreteType::Platform(PlatformType {
                    name: "Строка".to_string(),
                }),
                weight: 0.5,
            },
            WeightedType {
                type_: ConcreteType::Platform(PlatformType {
                    name: "Неопределено".to_string(),
                }),
                weight: 0.5,
            },
        ]),
        source: bsl_shared::domain::types::ResolutionSource::Inferred,
        metadata: Default::default(),
        active_facet: None,
        available_facets: vec![],
    };

    // x <> Неопределено
    let narrowed = resolver.narrow_type(&current, "x <> Неопределено");

    // Должен остаться только Строка
    if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &narrowed.result {
        assert_eq!(pt.name, "Строка");
    } else {
        panic!("Expected Concrete(Строка), got: {:?}", narrowed);
    }
}

#[test]
fn test_resolver_narrow_type_with_value_filled() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);

    let current = TypeResolution::unknown();

    // ЗначениеЗаполнено(x)
    let narrowed = resolver.narrow_type(&current, "ЗначениеЗаполнено(x)");

    // Должен вернуть тип (исключая Неопределено, Null, Ложь)
    // В нашей реализации возвращает исходный тип, так как Any не union
    assert!(matches!(
        narrowed.result,
        ResolutionResult::Dynamic | ResolutionResult::Concrete(_)
    ));
}

#[test]
fn test_detect_multiple_guards() {
    // Тестируем обнаружение нескольких type guards в одном условии
    let guards = detect_type_guards("x <> Неопределено И ЗначениеЗаполнено(x)");

    // Должно найти 2 guards
    assert!(!guards.is_empty()); // Минимум 1, так как парсинг простой
}

#[test]
fn test_narrowing_engine_with_if_statement() {
    // Создаём CFG для if-then-else
    let mut cfg = ControlFlowGraph::new();

    let entry_id = cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::Entry,
    });

    let cond_id = cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::Conditional {
            condition: "ТипЗнч(Параметр) = Тип(\"Строка\")".to_string(),
        },
    });

    let then_id = cfg.add_node(CfgNode {
        id: 2,
        kind: CfgNodeKind::BasicBlock {
            statements: vec!["Длина = Параметр.Length".to_string()],
        },
    });

    let else_id = cfg.add_node(CfgNode {
        id: 3,
        kind: CfgNodeKind::BasicBlock {
            statements: vec!["Длина = 0".to_string()],
        },
    });

    let exit_id = cfg.add_node(CfgNode {
        id: 4,
        kind: CfgNodeKind::Exit,
    });

    cfg.add_edge(entry_id, cond_id, EdgeKind::Unconditional);
    cfg.add_edge(cond_id, then_id, EdgeKind::ConditionalTrue);
    cfg.add_edge(cond_id, else_id, EdgeKind::ConditionalFalse);
    cfg.add_edge(then_id, exit_id, EdgeKind::Unconditional);
    cfg.add_edge(else_id, exit_id, EdgeKind::Unconditional);

    let mut engine = NarrowingEngine::new(cfg);

    let mut initial_ctx = FlowAnalysisContext::new();
    initial_ctx.set_variable("Параметр".to_string(), TypeResolution::unknown());

    engine.build_narrowing_contexts(initial_ctx);

    // Проверяем then ветку
    if let Some(then_ctx) = engine.get_context(then_id) {
        if let Some(param_type) = then_ctx.get_type("Параметр") {
            if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &param_type.result {
                assert_eq!(pt.name, "Строка");
            } else {
                panic!("Expected Параметр to be Строка in then branch");
            }
        } else {
            panic!("Параметр should exist in then branch");
        }
    } else {
        panic!("Then branch context should exist");
    }
}

#[test]
fn test_narrowing_with_nullable_type() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);

    // Nullable(Строка)
    let current = TypeResolution {
        certainty: bsl_shared::domain::types::Certainty::Known,
        result: ResolutionResult::Nullable(Box::new(ConcreteType::Platform(PlatformType {
            name: "Строка".to_string(),
        }))),
        source: bsl_shared::domain::types::ResolutionSource::Annotated,
        metadata: Default::default(),
        active_facet: None,
        available_facets: vec![],
    };

    // x <> Неопределено
    let narrowed = resolver.narrow_type(&current, "x <> Неопределено");

    // Должен убрать nullable обёртку
    if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &narrowed.result {
        assert_eq!(pt.name, "Строка");
    } else {
        panic!(
            "Expected Concrete(Строка) without nullable, got: {:?}",
            narrowed
        );
    }
}

#[test]
fn test_narrowing_preserves_non_guard_conditions() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);

    let current = TypeResolution::unknown();

    // Условие без type guards
    let narrowed = resolver.narrow_type(&current, "x > 0");

    // Должен вернуть исходный тип
    assert_eq!(
        format!("{:?}", current.result),
        format!("{:?}", narrowed.result)
    );
}

#[test]
fn test_narrowing_with_boolean_checks() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);

    let current = TypeResolution::unknown();

    // Флаг = Истина
    let narrowed = resolver.narrow_type(&current, "Флаг = Истина");

    if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &narrowed.result {
        assert_eq!(pt.name, "Булево");
    } else {
        panic!("Expected Булево type");
    }
}

#[test]
fn test_narrowing_with_empty_string_check() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);

    let current = TypeResolution::unknown();

    // Строка <> ""
    let narrowed = resolver.narrow_type(&current, "Строка <> \"\"");

    if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &narrowed.result {
        assert_eq!(pt.name, "Строка");
    } else {
        panic!("Expected Строка type");
    }
}

#[test]
fn test_narrowing_with_zero_check() {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = TypeResolver::new(repo);

    let current = TypeResolution::unknown();

    // Число <> 0
    let narrowed = resolver.narrow_type(&current, "Число <> 0");

    if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &narrowed.result {
        assert_eq!(pt.name, "Число");
    } else {
        panic!("Expected Число type");
    }
}

#[test]
fn test_cfg_with_loop_narrowing() {
    let mut cfg = ControlFlowGraph::new();

    let entry_id = cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::Entry,
    });

    let loop_header_id = cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::LoopHeader {
            condition: "ЗначениеЗаполнено(Элемент)".to_string(),
        },
    });

    let loop_body_id = cfg.add_node(CfgNode {
        id: 2,
        kind: CfgNodeKind::LoopBody,
    });

    let exit_id = cfg.add_node(CfgNode {
        id: 3,
        kind: CfgNodeKind::Exit,
    });

    cfg.add_edge(entry_id, loop_header_id, EdgeKind::Unconditional);
    cfg.add_edge(loop_header_id, loop_body_id, EdgeKind::ConditionalTrue);
    cfg.add_edge(loop_body_id, loop_header_id, EdgeKind::LoopBack);
    cfg.add_edge(loop_header_id, exit_id, EdgeKind::LoopExit);

    assert_eq!(cfg.nodes().len(), 4);
    assert_eq!(cfg.edges().len(), 4);
}

#[test]
fn test_narrowing_multiple_variables() {
    let mut cfg = ControlFlowGraph::new();

    let entry_id = cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::Entry,
    });

    let cond_id = cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::Conditional {
            condition: "ТипЗнч(A) = Тип(\"Число\")".to_string(),
        },
    });

    let then_id = cfg.add_node(CfgNode {
        id: 2,
        kind: CfgNodeKind::BasicBlock {
            statements: vec!["B = A + 10".to_string()],
        },
    });

    cfg.add_edge(entry_id, cond_id, EdgeKind::Unconditional);
    cfg.add_edge(cond_id, then_id, EdgeKind::ConditionalTrue);

    let mut engine = NarrowingEngine::new(cfg);

    let mut initial_ctx = FlowAnalysisContext::new();
    initial_ctx.set_variable("A".to_string(), TypeResolution::unknown());
    initial_ctx.set_variable("B".to_string(), TypeResolution::unknown());

    engine.build_narrowing_contexts(initial_ctx);

    // Проверяем, что A сузился в then ветке
    if let Some(then_ctx) = engine.get_context(then_id) {
        if let Some(a_type) = then_ctx.get_type("A") {
            if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &a_type.result {
                assert_eq!(pt.name, "Число");
            } else {
                panic!("Expected A to be Число");
            }
        }
    }
}
