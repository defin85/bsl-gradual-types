use super::*;
use crate::domain::types::{ConcreteType, PlatformType, TypeResolution};

#[test]
fn test_cfg_is_canonical_ir_type() {
    // Compile-time check: domain re-export должен быть тем же типом, что и IR CFG.
    let cfg: crate::domain::ControlFlowGraph = crate::ir::ControlFlowGraph::new();
    let _same: crate::ir::ControlFlowGraph = cfg;
}

#[test]
fn test_flow_context_set_get() {
    let mut ctx = FlowAnalysisContext::new();
    let resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Строка".to_string(),
    }));

    ctx.set_variable("x".to_string(), resolution.clone());
    assert!(ctx.get_variable("x").is_some());
}

#[test]
fn test_flow_context_scope() {
    let mut ctx = FlowAnalysisContext::new();
    assert_eq!(ctx.scope_depth, 0);

    ctx.enter_scope();
    assert_eq!(ctx.scope_depth, 1);

    ctx.exit_scope();
    assert_eq!(ctx.scope_depth, 0);
}

#[test]
fn test_flow_context_fork() {
    let mut ctx = FlowAnalysisContext::new();
    let resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Строка".to_string(),
    }));

    ctx.set_variable("x".to_string(), resolution.clone());

    let forked = ctx.fork();
    assert!(forked.get_variable("x").is_some());
}
