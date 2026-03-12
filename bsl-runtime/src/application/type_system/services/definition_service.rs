use std::path::PathBuf;
use std::sync::Arc;

use bsl_line_index::LineIndex;
use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::types::{ConcreteType, MetadataKind, ResolutionResult, TypeResolution};
use bsl_shared::ir::{SemanticNode, SemanticNodeKind, SemanticProgram, Span};

use crate::system::SystemCoordinator;

#[derive(Debug, Clone)]
pub struct DefinitionTarget {
    pub file_path: PathBuf,
    pub span: Option<Span>,
}

fn common_module_definition_target(
    repo: &dyn TypeRepository,
    module_name: &str,
) -> Option<DefinitionTarget> {
    let type_name = format!("ОбщиеМодули.{}", module_name);
    let raw = repo.find_type(&type_name)?;
    let paths = raw.module_paths?;
    let target_path = paths
        .object_module
        .or(paths.manager_module)
        .or(paths.recordset_module)?;
    Some(DefinitionTarget {
        file_path: target_path,
        span: None,
    })
}

pub fn goto_definition_v2(
    current_file_path: &str,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    line: u32,
    character: u32,
) -> Option<DefinitionTarget> {
    goto_definition_v2_with_source_opt(
        current_file_path,
        None,
        None,
        None,
        ir_program,
        deps,
        line,
        character,
        None,
    )
}

pub fn goto_definition_v2_with_source(
    current_file_path: &str,
    current_file_text: &str,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    line: u32,
    character: u32,
) -> Option<DefinitionTarget> {
    goto_definition_v2_with_source_opt(
        current_file_path,
        Some(current_file_text),
        None,
        None,
        ir_program,
        deps,
        line,
        character,
        None,
    )
}

pub fn goto_definition_v2_with_source_and_analysis(
    current_file_path: &str,
    current_file_text: &str,
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    line: u32,
    character: u32,
    coordinator: Option<&SystemCoordinator>,
) -> Option<DefinitionTarget> {
    goto_definition_v2_with_source_opt(
        current_file_path,
        Some(current_file_text),
        Some(analysis),
        Some(file_id),
        ir_program,
        deps,
        line,
        character,
        coordinator,
    )
}

pub fn definition_exact_type_index_available_at_position(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    line: u32,
    character: u32,
) -> bool {
    let _ = (line, character);
    exact_type_index_ready(analysis, file_id, None)
}

fn goto_definition_v2_with_source_opt(
    _current_file_path: &str,
    current_file_text: Option<&str>,
    analysis: Option<&bsl_analysis_v2::AnalysisV2>,
    file_id: Option<bsl_analysis_v2::FileId>,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    line: u32,
    character: u32,
    coordinator: Option<&SystemCoordinator>,
) -> Option<DefinitionTarget> {
    let repo = deps.repository.clone();
    let strict_semantic_mode = analysis.is_some() && file_id.is_some();

    let text = current_file_text?;

    let index = LineIndex::new(text);
    let offset = index.utf16_position_to_byte_offset(text, line, character) as u32;
    if let (Some(analysis), Some(file_id)) = (analysis, file_id) {
        if !exact_type_index_ready(analysis, file_id, coordinator) {
            return None;
        }
    }
    let type_at_position =
        semantic_type_at_offset(ir_program.as_ref(), analysis, file_id, offset, coordinator);

    if let Some(node) = ir_program.find_node_at_byte_offset(offset) {
        if let SemanticNodeKind::VariableAccess { name } = &node.kind {
            // If it's not a local variable, allow treating it as a global common module.
            if ir_program.resolve_variable(name, node.scope_id).is_none() {
                if strict_semantic_mode {
                    return semantic_definition_target_at_offset(
                        ir_program.as_ref(),
                        analysis,
                        file_id,
                        offset,
                    );
                }
                if let Some(target) = common_module_definition_target(repo.as_ref(), name) {
                    return Some(target);
                }
            }
        }

        if let SemanticNodeKind::MemberAccess { access_kind, .. } = &node.kind {
            if access_kind.is_method() {
                return semantic_method_definition_target(
                    ir_program.as_ref(),
                    analysis,
                    file_id,
                    node,
                    offset,
                );
            }
        }

        if let SemanticNodeKind::FunctionCall { object_name, .. } = &node.kind {
            let receiver_type =
                semantic_receiver_type(ir_program.as_ref(), analysis, file_id, node, coordinator);
            let cursor_targets_receiver = receiver_span_for_node(ir_program.as_ref(), node)
                .map(|span| span.contains(offset) || (offset > 0 && span.contains(offset - 1)))
                .unwrap_or(false);

            // Support "CommonModules.<Name>" namespace navigation in calls like "Модуль.Экспорт()":
            // go-to-definition on receiver should open the module file.
            if cursor_targets_receiver {
                if strict_semantic_mode {
                    return semantic_definition_target_at_offset(
                        ir_program.as_ref(),
                        analysis,
                        file_id,
                        offset,
                    );
                }

                if let Some(obj_name) = object_name.as_deref() {
                    // Avoid misrouting when a local variable shadows a common module name.
                    let is_local_var = ir_program
                        .resolve_variable(obj_name, node.scope_id)
                        .is_some();
                    if !is_local_var {
                        if let Some(target) =
                            common_module_definition_target(repo.as_ref(), obj_name)
                        {
                            return Some(target);
                        }
                    }

                    if let Some(obj_type) = receiver_type.as_ref() {
                        if is_common_module_type(obj_type) {
                            let type_name = obj_type.type_name();
                            if let Some(raw) = repo.find_type(&type_name) {
                                if let Some(paths) = raw.module_paths.clone() {
                                    let target_path = paths
                                        .object_module
                                        .or(paths.manager_module)
                                        .or(paths.recordset_module)?;
                                    return Some(DefinitionTarget {
                                        file_path: target_path,
                                        span: None,
                                    });
                                }
                            }
                        }
                    }
                }
                return None;
            }

            return semantic_method_definition_target(
                ir_program.as_ref(),
                analysis,
                file_id,
                node,
                offset,
            );
        }
    }

    if strict_semantic_mode {
        return semantic_definition_target_at_offset(ir_program.as_ref(), analysis, file_id, offset);
    }

    let type_resolution = type_at_position?;

    let module_paths = if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) =
        &type_resolution.result
    {
        let type_key = format!("{}.{}", cfg.kind.to_prefix(), cfg.name);
        repo.find_type(&type_key)
            .and_then(|raw| raw.module_paths.clone())
    } else {
        None
    };

    let definition_location =
        type_resolution.get_definition_location_with_modules(module_paths.as_ref())?;

    definition_target_from_location(definition_location)
}

fn is_common_module_type(resolution: &TypeResolution) -> bool {
    match &resolution.result {
        ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => {
            cfg.kind == MetadataKind::CommonModule
        }
        ResolutionResult::Concrete(ConcreteType::Platform(platform)) => {
            platform.name.starts_with("ОбщиеМодули.")
        }
        _ => false,
    }
}

fn semantic_type_at_offset(
    program: &SemanticProgram,
    analysis: Option<&bsl_analysis_v2::AnalysisV2>,
    file_id: Option<bsl_analysis_v2::FileId>,
    offset: u32,
    coordinator: Option<&SystemCoordinator>,
) -> Option<TypeResolution> {
    let _ = coordinator;
    if let (Some(analysis), Some(file_id)) = (analysis, file_id) {
        return analysis
            .type_at_byte_offset_serve_only(file_id, offset)
            .ok()
            .flatten()
            .or_else(|| {
                program.find_node_at_byte_offset(offset).and_then(|node| {
                    analysis.type_for_span_serve_only(file_id, node.span).ok().flatten()
                })
            });
    }
    program
        .semantic_facts
        .type_at_byte_offset(offset)
        .or_else(|| {
            program
                .find_node_at_byte_offset(offset)
                .and_then(|node| program.semantic_facts.type_resolution_for_span(node.span))
        })
}

fn receiver_span_for_node(
    program: &SemanticProgram,
    node: &bsl_shared::ir::SemanticNode,
) -> Option<Span> {
    match &node.kind {
        SemanticNodeKind::MemberAccess {
            object_node,
            object_span,
            ..
        }
        | SemanticNodeKind::FunctionCall {
            object_node,
            object_span,
            ..
        } => object_span.or_else(|| {
            object_node.and_then(|idx| program.nodes.get(idx).map(|candidate| candidate.span))
        }),
        _ => None,
    }
}

fn semantic_receiver_type(
    program: &SemanticProgram,
    analysis: Option<&bsl_analysis_v2::AnalysisV2>,
    file_id: Option<bsl_analysis_v2::FileId>,
    node: &bsl_shared::ir::SemanticNode,
    coordinator: Option<&SystemCoordinator>,
) -> Option<TypeResolution> {
    let _ = coordinator;

    let span_fallback = |span: Span| {
        semantic_type_at_offset(program, analysis, file_id, span.start, None)
            .or_else(|| {
                span.end
                    .checked_sub(1)
                    .and_then(|offset| semantic_type_at_offset(program, analysis, file_id, offset, None))
            })
            .or_else(|| {
                if let (Some(analysis), Some(file_id)) = (analysis, file_id) {
                    analysis.type_for_span_serve_only(file_id, span).ok().flatten()
                } else {
                    program.semantic_facts.type_resolution_for_span(span)
                }
            })
    };

    if let (Some(analysis), Some(file_id)) = (analysis, file_id) {
        return match &node.kind {
            SemanticNodeKind::MemberAccess { .. } => analysis
                .member_access_object_type_for_span_serve_only(file_id, node.span)
                .ok()
                .flatten()
                .or_else(|| receiver_span_for_node(program, node).and_then(span_fallback)),
            SemanticNodeKind::FunctionCall { .. } => analysis
                .call_receiver_type_for_span_serve_only(file_id, node.span)
                .ok()
                .flatten()
                .or_else(|| receiver_span_for_node(program, node).and_then(span_fallback)),
            _ => None,
        };
    }

    match &node.kind {
        SemanticNodeKind::MemberAccess { .. } => program
            .semantic_facts
            .member_access_object_type_by_span
            .get(&node.span)
            .cloned()
            .or_else(|| receiver_span_for_node(program, node).and_then(span_fallback)),
        SemanticNodeKind::FunctionCall { .. } => program
            .semantic_facts
            .call_receiver_type_by_span
            .get(&node.span)
            .cloned()
            .or_else(|| receiver_span_for_node(program, node).and_then(span_fallback)),
        _ => None,
    }
}

fn exact_type_index_ready(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    coordinator: Option<&SystemCoordinator>,
) -> bool {
    let ready = analysis
        .current_type_index_serve_only_ready(file_id)
        .ok()
        .unwrap_or(false);
    if let Some(coordinator) = coordinator {
        coordinator.record_intellisense_v2_type_index_reason(if ready {
            bsl_analysis_v2::TypeIndexServeReasonCode::TypeIndexExactHit.as_str()
        } else {
            bsl_analysis_v2::TypeIndexServeReasonCode::TypeIndexFallbackUnavailable.as_str()
        });
    }
    ready
}

fn semantic_method_definition_target(
    ir_program: &SemanticProgram,
    analysis: Option<&bsl_analysis_v2::AnalysisV2>,
    file_id: Option<bsl_analysis_v2::FileId>,
    node: &SemanticNode,
    byte_offset: u32,
) -> Option<DefinitionTarget> {
    let target = if let (Some(analysis), Some(file_id)) = (analysis, file_id) {
        match &node.kind {
            SemanticNodeKind::MemberAccess { access_kind, .. } if access_kind.is_method() => analysis
                .member_method_target_for_span_serve_only(file_id, node.span)
                .ok()
                .flatten()
                .or_else(|| {
                    analysis
                        .member_method_target_at_byte_offset_serve_only(file_id, byte_offset)
                        .ok()
                        .flatten()
                }),
            SemanticNodeKind::FunctionCall { .. } => analysis
                .call_method_target_for_span_serve_only(file_id, node.span)
                .ok()
                .flatten()
                .or_else(|| {
                    analysis
                        .call_method_target_at_byte_offset_serve_only(file_id, byte_offset)
                        .ok()
                        .flatten()
                }),
            _ => None,
        }
    } else {
        let exact = match &node.kind {
            SemanticNodeKind::MemberAccess { access_kind, .. } if access_kind.is_method() => ir_program
                .semantic_facts
                .member_method_targets_by_span
                .get(&node.span)
                .cloned(),
            SemanticNodeKind::FunctionCall { .. } => ir_program
                .semantic_facts
                .call_method_targets_by_span
                .get(&node.span)
                .cloned(),
            _ => None,
        };
        exact.or_else(|| semantic_method_definition_target_at_offset(ir_program, byte_offset).cloned())
    }?;

    target
        .definition_location
        .clone()
        .and_then(definition_target_from_location)
}

fn semantic_method_definition_target_at_offset(
    ir_program: &SemanticProgram,
    byte_offset: u32,
) -> Option<&bsl_shared::ir::SemanticMethodTarget> {
    let call_target = ir_program
        .semantic_facts
        .call_method_targets_by_span
        .iter()
        .filter(|(span, target)| span.contains(byte_offset) && target.definition_location.is_some())
        .min_by_key(|(span, _)| span.len())
        .map(|(_, target)| target);
    if call_target.is_some() {
        return call_target;
    }

    ir_program
        .semantic_facts
        .member_method_targets_by_span
        .iter()
        .filter(|(span, target)| span.contains(byte_offset) && target.definition_location.is_some())
        .min_by_key(|(span, _)| span.len())
        .map(|(_, target)| target)
}

fn semantic_definition_target_at_offset(
    ir_program: &SemanticProgram,
    analysis: Option<&bsl_analysis_v2::AnalysisV2>,
    file_id: Option<bsl_analysis_v2::FileId>,
    byte_offset: u32,
) -> Option<DefinitionTarget> {
    if let (Some(analysis), Some(file_id)) = (analysis, file_id) {
        return analysis
            .definition_location_at_byte_offset_serve_only(file_id, byte_offset)
            .ok()
            .flatten()
            .and_then(definition_target_from_location);
    }

    ir_program
        .semantic_facts
        .definition_location_at_byte_offset(byte_offset)
        .and_then(definition_target_from_location)
}

fn definition_target_from_location(
    definition_location: TypeDefinitionLocation,
) -> Option<DefinitionTarget> {
    match definition_location {
        TypeDefinitionLocation::Configuration {
            metadata_path,
            module_paths,
        } => {
            let target_path = module_paths
                .object_module
                .or(module_paths.manager_module)
                .unwrap_or(metadata_path);

            Some(DefinitionTarget {
                file_path: target_path,
                span: None,
            })
        }
        TypeDefinitionLocation::UserDefined {
            file_path,
            start,
            end,
        } => Some(DefinitionTarget {
            file_path,
            span: Some(Span::new(start, end)),
        }),
        TypeDefinitionLocation::Platform { .. } | TypeDefinitionLocation::Primitive => None,
    }
}
