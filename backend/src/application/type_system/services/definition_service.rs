use std::path::PathBuf;
use std::sync::Arc;

use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::types::{ConcreteType, ResolutionResult, TypeResolution};
use bsl_shared::ir::{SemanticNodeKind, SemanticProgram, Span};

#[derive(Debug, Clone)]
pub struct DefinitionTarget {
    pub file_path: PathBuf,
    pub span: Option<Span>,
}

pub fn goto_definition_v2(
    current_file_path: &str,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    line: u32,
    character: u32,
) -> Option<DefinitionTarget> {
    let repo = deps.repository.clone();
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(repo.clone())));

    if let Some(node) = ir_program.find_node_at_position(line, character) {
        if let SemanticNodeKind::FunctionCall {
            function_name,
            object_type,
            ..
        } = &node.kind
        {
            let owner_type = object_type.as_ref().map(|value| value.type_name());

            if let Some(loc) =
                repo.find_method_definition_location(owner_type.as_deref(), function_name)
            {
                return definition_target_from_location(loc);
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

    let type_resolution = get_type_at_position_with_semantic_program(
        ir_program.as_ref(),
        resolver.as_ref(),
        line,
        character,
    )?;

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

fn get_type_at_position_with_semantic_program(
    ir_program: &SemanticProgram,
    resolver: &TypeResolver,
    line: u32,
    column: u32,
) -> Option<TypeResolution> {
    if let Some((var_name, _type_hint, scope_id)) =
        ir_program.find_variable_with_scope(line, column)
    {
        return Some(resolver.resolve_variable_with_context(
            &var_name,
            &ir_program.symbols,
            scope_id,
        ));
    }

    if let Some(node) = ir_program.find_node_at_position(line, column) {
        match &node.kind {
            SemanticNodeKind::VariableDeclaration {
                type_hint: Some(resolution),
                ..
            } => Some(resolver.resolve_expression_sync(&resolution.type_name())),
            SemanticNodeKind::FunctionCall {
                object_type: Some(type_resolution),
                ..
            } => Some(resolver.resolve_expression_sync(&type_resolution.type_name())),
            SemanticNodeKind::MemberAccess { object_type, .. } => {
                Some(resolver.resolve_expression_sync(&object_type.type_name()))
            }
            SemanticNodeKind::NewExpression { type_name, .. } => {
                Some(resolver.resolve_expression_sync(type_name))
            }
            _ => None,
        }
    } else {
        None
    }
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
            start_line,
            start_column,
            end_line,
            end_column,
        } => Some(DefinitionTarget {
            file_path,
            span: Some(Span {
                start_line,
                start_column,
                end_line,
                end_column,
            }),
        }),
        TypeDefinitionLocation::Platform { .. } | TypeDefinitionLocation::Primitive => None,
    }
}
