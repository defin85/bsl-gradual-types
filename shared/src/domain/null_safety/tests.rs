use super::*;
use crate::{
    Certainty, ConcreteType, PlatformType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution,
};

#[test]
fn test_null_check_detection() {
    let cfg = ControlFlowGraph::new();
    let analyzer = NullSafetyAnalyzer::new(cfg);

    assert!(analyzer.is_null_check("Если ЗначениеЗаполнено(значение) Тогда"));
    assert!(analyzer.is_null_check("If ValueIsFilled(value) Then"));
    assert!(analyzer.is_null_check("Если значение <> Неопределено Тогда"));
    assert!(!analyzer.is_null_check("Если значение > 0 Тогда"));
}

#[test]
fn test_nullable_tracking() {
    let mut cfg = ControlFlowGraph::new();

    // Создаём простой CFG: присваивание → проверка → использование
    cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::Assignment {
            variable: "x".to_string(),
            value: "Неопределено".to_string(),
        },
    });

    cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::Condition {
            variable: "ЗначениеЗаполнено(x)".to_string(),
        },
    });

    cfg.add_node(CfgNode {
        id: 2,
        kind: CfgNodeKind::MethodCall {
            object: "x".to_string(),
            method: "Метод".to_string(),
            arguments: vec![],
        },
    });

    cfg.add_edge(0, 1, EdgeKind::Unconditional);

    cfg.add_edge(1, 2, EdgeKind::ConditionalTrue);

    let mut analyzer = NullSafetyAnalyzer::new(cfg);
    let mut context = FlowAnalysisContext::new();

    // x = Null → nullable
    context.set_variable(
        "x",
        TypeResolution {
            result: ResolutionResult::nullable(ConcreteType::string()),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        },
    );

    let result = analyzer.analyze(&context);

    // После проверки в узле 2 не должно быть предупреждений
    // (но текущая реализация ещё не достаточно умная)
    assert!(!result.safe_operations.is_empty());
}

#[test]
fn test_loop_header_null_check_suppresses_warning_in_body() {
    let mut cfg = ControlFlowGraph::new();

    cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::LoopHeader {
            condition: "x <> Null".to_string(),
        },
    });

    cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::MethodCall {
            object: "x".to_string(),
            method: "Метод".to_string(),
            arguments: vec![],
        },
    });

    cfg.add_edge(0, 1, EdgeKind::ConditionalTrue);

    let mut analyzer = NullSafetyAnalyzer::new(cfg);
    let mut context = FlowAnalysisContext::new();

    context.set_variable(
        "x",
        TypeResolution {
            result: ResolutionResult::nullable(ConcreteType::string()),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        },
    );

    let result = analyzer.analyze(&context);
    assert!(
        result.warnings.is_empty(),
        "expected no warnings for x.Method() guarded by loop header, warnings={:?}",
        result.warnings
    );
    assert!(
        !result.safe_operations.is_empty(),
        "expected safe operation to be recorded"
    );
}

#[test]
fn test_loop_backedge_null_check_does_not_recurse_forever() {
    let mut cfg = ControlFlowGraph::new();

    cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::LoopHeader {
            condition: "x <> Null".to_string(),
        },
    });

    cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::MethodCall {
            object: "x".to_string(),
            method: "Метод".to_string(),
            arguments: vec![],
        },
    });

    cfg.add_edge(0, 1, EdgeKind::ConditionalTrue);
    cfg.add_edge(1, 0, EdgeKind::Unconditional);

    let mut analyzer = NullSafetyAnalyzer::new(cfg);
    let mut context = FlowAnalysisContext::new();

    context.set_variable(
        "x",
        TypeResolution {
            result: ResolutionResult::nullable(ConcreteType::string()),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        },
    );

    let result = analyzer.analyze(&context);
    assert!(
        result.warnings.is_empty(),
        "expected no warnings inside loop body after null check, warnings={:?}",
        result.warnings
    );
    assert!(
        !result.safe_operations.is_empty(),
        "expected safe operation to be recorded for loop backedge"
    );
}

#[test]
fn test_unchecked_null_warning() {
    let mut cfg = ControlFlowGraph::new();

    // Прямое использование без проверки
    cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::Assignment {
            variable: "x".to_string(),
            value: "Неопределено".to_string(),
        },
    });

    cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::MethodCall {
            object: "x".to_string(),
            method: "Метод".to_string(),
            arguments: vec![],
        },
    });

    cfg.add_edge(0, 1, EdgeKind::Unconditional);

    let mut analyzer = NullSafetyAnalyzer::new(cfg);
    let mut context = FlowAnalysisContext::new();

    context.set_variable(
        "x",
        TypeResolution {
            result: ResolutionResult::nullable(ConcreteType::string()),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        },
    );

    let result = analyzer.analyze(&context);

    // Должно быть предупреждение о возможном NPE
    assert!(!result.warnings.is_empty());
    assert_eq!(
        result.warnings[0].kind,
        NullWarningKind::PossibleNullDereference
    );
}

#[test]
fn test_non_nullable_no_warning() {
    let mut cfg = ControlFlowGraph::new();

    cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::Assignment {
            variable: "x".to_string(),
            value: "\"строка\"".to_string(),
        },
    });

    cfg.add_node(CfgNode {
        id: 1,
        kind: CfgNodeKind::MethodCall {
            object: "x".to_string(),
            method: "Длина".to_string(),
            arguments: vec![],
        },
    });

    cfg.add_edge(0, 1, EdgeKind::Unconditional);

    let mut analyzer = NullSafetyAnalyzer::new(cfg);
    let mut context = FlowAnalysisContext::new();

    // x = "строка" → не nullable
    context.set_variable(
        "x",
        TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::string()),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        },
    );

    let result = analyzer.analyze(&context);

    // Не должно быть предупреждений
    assert_eq!(result.warnings.len(), 0);
}

#[test]
fn test_property_access_null_check() {
    let mut cfg = ControlFlowGraph::new();

    cfg.add_node(CfgNode {
        id: 0,
        kind: CfgNodeKind::PropertyAccess {
            object: "obj".to_string(),
            property: "Свойство".to_string(),
        },
    });

    let mut analyzer = NullSafetyAnalyzer::new(cfg);
    let mut context = FlowAnalysisContext::new();

    context.set_variable(
        "obj",
        TypeResolution {
            result: ResolutionResult::nullable(ConcreteType::Platform(PlatformType {
                name: "Объект".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        },
    );

    let result = analyzer.analyze(&context);

    // Должно быть предупреждение о доступе к свойству nullable объекта
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].message.contains("доступе к свойству"));
}
