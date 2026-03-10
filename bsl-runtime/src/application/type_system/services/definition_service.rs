use std::path::PathBuf;
use std::sync::Arc;

use bsl_line_index::LineIndex;
use bsl_shared::domain::code_location::{CodeLocation, ModuleType};
use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::types::{
    ConcreteType, FacetKind, MetadataKind, ResolutionResult, TypeResolution,
};
use bsl_shared::ir::{SemanticNodeKind, SemanticProgram, Span};

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
    goto_definition_v2_with_source_opt(current_file_path, None, ir_program, deps, line, character)
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
        ir_program,
        deps,
        line,
        character,
    )
}

fn goto_definition_v2_with_source_opt(
    current_file_path: &str,
    current_file_text: Option<&str>,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    line: u32,
    character: u32,
) -> Option<DefinitionTarget> {
    let repo = deps.repository.clone();

    let text = current_file_text?;

    let index = LineIndex::new(text);
    let offset = index.utf16_position_to_byte_offset(text, line, character) as u32;
    let type_at_position = semantic_type_at_offset(ir_program.as_ref(), offset);

    if let Some(node) = ir_program.find_node_at_byte_offset(offset) {
        if let SemanticNodeKind::VariableAccess { name } = &node.kind {
            // If it's not a local variable, allow treating it as a global common module.
            if ir_program.resolve_variable(name, node.scope_id).is_none() {
                if let Some(target) = common_module_definition_target(repo.as_ref(), name) {
                    return Some(target);
                }
            }
        }

        if let SemanticNodeKind::MemberAccess {
            object_name,
            member_name,
            access_kind,
            ..
        } = &node.kind
        {
            if access_kind.is_method() {
                if let Some(owner_type) = semantic_receiver_type(ir_program.as_ref(), node)
                    .as_ref()
                    .map(TypeResolution::type_name)
                {
                    if let Some(loc) =
                        repo.find_method_definition_location(Some(&owner_type), member_name)
                    {
                        return definition_target_from_location(loc);
                    }
                }

                if let Some(obj_name) = object_name.as_deref() {
                    let common_module_owner = format!("ОбщиеМодули.{}", obj_name);
                    if let Some(loc) = repo
                        .find_method_definition_location(Some(&common_module_owner), member_name)
                    {
                        return definition_target_from_location(loc);
                    }
                }

                if let Some(target) = current_module_method_definition_target(
                    repo.as_ref(),
                    current_file_path,
                    member_name,
                ) {
                    return Some(target);
                }

                if let Some(span) =
                    find_local_method_declaration_span(ir_program.as_ref(), member_name)
                {
                    return Some(DefinitionTarget {
                        file_path: PathBuf::from(current_file_path),
                        span: Some(span),
                    });
                }
            }
        }

        if let SemanticNodeKind::FunctionCall {
            function_name,
            object_name,
            ..
        } = &node.kind
        {
            let receiver_type = semantic_receiver_type(ir_program.as_ref(), node);
            let owner_type = receiver_type.as_ref().map(|value| value.type_name());
            let cursor_targets_receiver = receiver_span_for_node(ir_program.as_ref(), node)
                .map(|span| span.contains(offset) || (offset > 0 && span.contains(offset - 1)))
                .unwrap_or(false);

            // Support "CommonModules.<Name>" namespace navigation in calls like "Модуль.Экспорт()":
            // go-to-definition on receiver should open the module file.
            if cursor_targets_receiver {
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
            } else if let Some(loc) =
                repo.find_method_definition_location(owner_type.as_deref(), function_name)
            {
                return definition_target_from_location(loc);
            } else if let Some(obj_name) = object_name.as_deref() {
                // Common module calls may have Dynamic receiver type (e.g. during partial indexing);
                // fall back to "ОбщиеМодули.<ModuleName>" when we have a receiver identifier.
                let common_module_owner = format!("ОбщиеМодули.{}", obj_name);
                if let Some(loc) =
                    repo.find_method_definition_location(Some(&common_module_owner), function_name)
                {
                    return definition_target_from_location(loc);
                }
            }

            if let Some(target) = current_module_method_definition_target(
                repo.as_ref(),
                current_file_path,
                function_name,
            ) {
                return Some(target);
            }

            if let Some(span) =
                find_local_method_declaration_span(ir_program.as_ref(), function_name)
            {
                return Some(DefinitionTarget {
                    file_path: PathBuf::from(current_file_path),
                    span: Some(span),
                });
            }
        }
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

fn current_module_method_definition_target(
    repo: &dyn TypeRepository,
    current_file_path: &str,
    method_name: &str,
) -> Option<DefinitionTarget> {
    for owner_type in current_module_owner_type_candidates(current_file_path) {
        if let Some(loc) = repo.find_method_definition_location(Some(&owner_type), method_name) {
            return definition_target_from_location(loc);
        }
    }

    None
}

fn current_module_owner_type_candidates(current_file_path: &str) -> Vec<String> {
    let Ok(location) = CodeLocation::determine_from_path(std::path::Path::new(current_file_path))
    else {
        return Vec::new();
    };

    let Some(owner_type) = location.get_owner_type() else {
        return Vec::new();
    };

    let Some((xml_kind, object_name)) = owner_type.split_once('.') else {
        return vec![owner_type.to_string()];
    };
    let Some(kind) = MetadataKind::from_xml_tag(xml_kind) else {
        return vec![owner_type.to_string()];
    };

    let facet = match location.module_type {
        ModuleType::ObjectModule { .. } | ModuleType::RecordSetModule { .. } => {
            Some(FacetKind::Object)
        }
        ModuleType::ManagerModule { .. } => Some(FacetKind::Manager),
        ModuleType::FormModule { .. } => None,
        ModuleType::CommonModule { .. } | ModuleType::Unknown => None,
    };

    let mut candidates = Vec::with_capacity(2);
    if let Some(facet) = facet {
        candidates.push(format!(
            "{}.{}",
            kind.faceted_type_prefix(&facet),
            object_name
        ));
    }
    candidates.push(owner_type.to_string());
    candidates
}

fn semantic_type_at_offset(program: &SemanticProgram, offset: u32) -> Option<TypeResolution> {
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
    node: &bsl_shared::ir::SemanticNode,
) -> Option<TypeResolution> {
    let span_fallback = |span: Span| {
        semantic_type_at_offset(program, span.start)
            .or_else(|| {
                span.end
                    .checked_sub(1)
                    .and_then(|offset| semantic_type_at_offset(program, offset))
            })
            .or_else(|| program.semantic_facts.type_resolution_for_span(span))
    };

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

fn find_local_method_declaration_span(
    ir_program: &SemanticProgram,
    method_name: &str,
) -> Option<Span> {
    for node in &ir_program.nodes {
        match &node.kind {
            SemanticNodeKind::FunctionDeclaration { name, .. }
            | SemanticNodeKind::ProcedureDeclaration { name, .. }
                if name.eq_ignore_ascii_case(method_name) =>
            {
                return Some(node.span);
            }
            _ => {}
        }
    }

    None
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
