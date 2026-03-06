use bsl_shared::analysis::{detect_type_guards, NarrowingEngine};
use bsl_shared::domain::flow_analysis::FlowAnalysisContext;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::ir::{
    ControlFlowGraph, EdgeKind, NodeAtByteOffsetBias, SemanticNodeKind, SemanticProgram,
};

#[allow(dead_code)]
fn conditional_branch_node_at_byte_offset(
    program: &SemanticProgram,
    cfg: &ControlFlowGraph,
    conditional_node_id: usize,
    byte_offset: u32,
) -> usize {
    let mut edge_kind = EdgeKind::ConditionalTrue;

    if let Some(ir_node_idx) = cfg.node_ir_node_index(conditional_node_id) {
        if let Some(ir_node) = program.nodes.get(ir_node_idx) {
            if let SemanticNodeKind::IfStatement { else_branch, .. } = &ir_node.kind {
                if let Some(else_branch) = else_branch.as_ref().filter(|b| !b.is_empty()) {
                    let else_start = else_branch
                        .iter()
                        .filter_map(|idx| program.nodes.get(*idx).map(|n| n.span.start))
                        .min();

                    if else_start.is_some_and(|start| byte_offset >= start) {
                        edge_kind = EdgeKind::ConditionalFalse;
                    }
                }
            }
        }
    }

    cfg.edges()
        .iter()
        .find(|e| e.from == conditional_node_id && e.kind == edge_kind)
        .map(|e| e.to)
        .unwrap_or(conditional_node_id)
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub(crate) fn narrow_type_for_variable_at(
    program: &SemanticProgram,
    byte_offset: u32,
    variable_name: &str,
    base_type: TypeResolution,
) -> Option<TypeResolution> {
    let cfg = program.cfg.as_ref()?;
    let mut node_id = cfg.node_at_byte_offset(byte_offset, NodeAtByteOffsetBias::PreferLeft)?;

    // Если позиция попала в span условного узла (например, внутри then/else блока),
    // смещаемся на соответствующую ветку, чтобы получить корректный контекст narrowing.
    match &cfg.nodes().get(node_id)?.kind {
        bsl_shared::ir::CfgNodeKind::Conditional { .. } => {
            node_id = conditional_branch_node_at_byte_offset(program, cfg, node_id, byte_offset);
        }
        bsl_shared::ir::CfgNodeKind::LoopHeader { .. } => {
            node_id = cfg
                .edges()
                .iter()
                .find(|e| e.from == node_id && e.kind == EdgeKind::ConditionalTrue)
                .map(|e| e.to)
                .unwrap_or(node_id);
        }
        _ => {}
    }

    let initial = build_initial_flow_context_for_narrowing(cfg, variable_name, base_type);
    let mut engine = NarrowingEngine::new(cfg.clone());
    engine.build_narrowing_contexts(initial);

    engine
        .get_context(node_id)
        .and_then(|ctx| ctx.get_type(variable_name))
        .cloned()
        .filter(|t| !t.is_unknown())
}
