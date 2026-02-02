use bsl_shared::analysis::{detect_type_guards, NarrowingEngine};
use bsl_shared::domain::flow_analysis::FlowAnalysisContext;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::ir::{CfgNodeAtByteOffsetBias, ControlFlowGraph, SemanticProgram};

fn build_initial_flow_context_for_narrowing(
    cfg: &ControlFlowGraph,
    variable_name: &str,
    base_type: TypeResolution,
) -> FlowAnalysisContext {
    let mut ctx = FlowAnalysisContext::new();
    ctx.set_variable(variable_name, base_type);

    for node in cfg.nodes() {
        match &node.kind {
            bsl_shared::ir::CfgNodeKind::Conditional { condition }
            | bsl_shared::ir::CfgNodeKind::LoopHeader { condition } => {
                for guard in detect_type_guards(condition) {
                    let var = guard.variable_name();
                    if ctx.get_variable(var).is_none() {
                        ctx.set_variable(var, TypeResolution::unknown());
                    }
                }
            }
            _ => {}
        }
    }

    ctx
}

pub(crate) fn narrow_type_for_variable_at(
    program: &SemanticProgram,
    byte_offset: u32,
    variable_name: &str,
    base_type: TypeResolution,
    bias: CfgNodeAtByteOffsetBias,
) -> Option<TypeResolution> {
    let cfg = program.cfg.as_ref()?;
    let node_id = cfg.node_at_byte_offset(byte_offset, bias)?;

    let initial = build_initial_flow_context_for_narrowing(cfg, variable_name, base_type);
    let mut engine = NarrowingEngine::new(cfg.clone());
    engine.build_narrowing_contexts(initial);

    engine
        .get_context(node_id)
        .and_then(|ctx| ctx.get_type(variable_name))
        .cloned()
        .filter(|t| !t.is_unknown())
}
