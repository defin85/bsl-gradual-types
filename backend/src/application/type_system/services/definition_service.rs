use std::path::PathBuf;
use std::sync::Arc;

use bsl_line_index::LineIndex;
use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::types::{ConcreteType, MetadataKind, ResolutionResult, TypeResolution};
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
    type_at_position_hint: Option<TypeResolution>,
    receiver_type_hint: Option<TypeResolution>,
) -> Option<DefinitionTarget> {
    goto_definition_v2_with_source_opt(
        current_file_path,
        None,
        ir_program,
        deps,
        line,
        character,
        type_at_position_hint,
        receiver_type_hint,
    )
}

pub fn goto_definition_v2_with_source(
    current_file_path: &str,
    current_file_text: &str,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    line: u32,
    character: u32,
    type_at_position_hint: Option<TypeResolution>,
    receiver_type_hint: Option<TypeResolution>,
) -> Option<DefinitionTarget> {
    goto_definition_v2_with_source_opt(
        current_file_path,
        Some(current_file_text),
        ir_program,
        deps,
        line,
        character,
        type_at_position_hint,
        receiver_type_hint,
    )
}

fn goto_definition_v2_with_source_opt(
    current_file_path: &str,
    current_file_text: Option<&str>,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    line: u32,
    character: u32,
    type_at_position_hint: Option<TypeResolution>,
    receiver_type_hint: Option<TypeResolution>,
) -> Option<DefinitionTarget> {
    let repo = deps.repository.clone();

    let Some(text) = current_file_text else {
        return None;
    };

    let index = LineIndex::new(text);
    let offset = index.utf16_position_to_byte_offset(text, line, character) as u32;

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
                if let Some(owner_type) = receiver_type_hint.as_ref().map(TypeResolution::type_name)
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
            }
        }

        if let SemanticNodeKind::FunctionCall {
            function_name,
            object_name,
            ..
        } = &node.kind
        {
            let token_at_position = current_file_text
                .and_then(|text| identifier_at_utf16_position(text, line, character));

            // Support "CommonModules.<Name>" namespace navigation in calls like "Модуль.Экспорт()":
            // go-to-definition on receiver should open the module file.
            if let (Some(token), Some(obj_name)) =
                (token_at_position.as_deref(), object_name.as_deref())
            {
                if token.eq_ignore_ascii_case(obj_name) {
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

                    if let Some(obj_type) = receiver_type_hint.as_ref() {
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
            }

            let owner_type = receiver_type_hint.as_ref().map(|value| value.type_name());

            if let Some(loc) =
                repo.find_method_definition_location(owner_type.as_deref(), function_name)
            {
                return definition_target_from_location(loc);
            }

            // Common module calls may have Dynamic receiver type (e.g. during partial indexing);
            // fall back to "ОбщиеМодули.<ModuleName>" when we have a receiver identifier.
            if let Some(obj_name) = object_name.as_deref() {
                let common_module_owner = format!("ОбщиеМодули.{}", obj_name);
                if let Some(loc) =
                    repo.find_method_definition_location(Some(&common_module_owner), function_name)
                {
                    return definition_target_from_location(loc);
                }
            }

            if owner_type.is_none() {
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
    }

    let type_resolution = type_at_position_hint?;

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

fn identifier_at_utf16_position(text: &str, line: u32, col_utf16: u32) -> Option<String> {
    let line_str = text.lines().nth(line as usize)?;
    let mut byte_idx = utf16_col_to_byte_offset(line_str, col_utf16);
    if byte_idx > line_str.len() {
        byte_idx = line_str.len();
    }

    // If cursor is at a boundary after the identifier, prefer the character to the left.
    if byte_idx == line_str.len() || !is_ident_char_at_byte(line_str, byte_idx) {
        if let Some(prev) = prev_char_boundary(line_str, byte_idx) {
            if is_ident_char_at_byte(line_str, prev) {
                byte_idx = prev;
            }
        }
    }

    if !is_ident_char_at_byte(line_str, byte_idx) {
        return None;
    }

    let start = scan_ident_left(line_str, byte_idx);
    let end = scan_ident_right(line_str, byte_idx);
    Some(line_str[start..end].to_string())
}

fn utf16_col_to_byte_offset(line: &str, col_utf16: u32) -> usize {
    let mut units: u32 = 0;
    for (idx, ch) in line.char_indices() {
        if units >= col_utf16 {
            return idx;
        }
        units = units.saturating_add(ch.len_utf16() as u32);
    }
    line.len()
}

fn prev_char_boundary(line: &str, byte_idx: usize) -> Option<usize> {
    if byte_idx == 0 {
        return None;
    }
    let mut prev = None;
    for (idx, _) in line.char_indices() {
        if idx >= byte_idx {
            return prev;
        }
        prev = Some(idx);
    }
    prev
}

fn is_ident_char_at_byte(line: &str, byte_idx: usize) -> bool {
    match line[byte_idx..].chars().next() {
        Some(ch) => is_ident_char(ch),
        None => false,
    }
}

fn is_ident_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn scan_ident_left(line: &str, byte_idx: usize) -> usize {
    let mut start = byte_idx;
    let mut chars: Vec<(usize, char)> = line[..byte_idx].char_indices().collect();
    while let Some((idx, ch)) = chars.pop() {
        if !is_ident_char(ch) {
            break;
        }
        start = idx;
    }
    start
}

fn scan_ident_right(line: &str, byte_idx: usize) -> usize {
    let mut end = line.len();
    for (idx, ch) in line[byte_idx..].char_indices() {
        if !is_ident_char(ch) {
            end = byte_idx + idx;
            break;
        }
    }
    end
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
