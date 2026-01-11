//! Go To Definition handler for LSP
//!
//! Handles textDocument/definition requests.
//! Milestone 3.14: Navigates to type definitions (configuration types, platform types)

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::{debug, error, info, warn};

use bsl_backend::application::TypeSystemService;
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{ConcreteType, ResolutionResult};
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::ir::{SemanticNodeKind, SemanticProgram};

/// Handle textDocument/definition request
pub async fn handle_goto_definition(
    file_content: &str,
    file_path: Option<&str>,
    position: Position,
    type_service: Option<Arc<TypeSystemService>>,
) -> Option<GotoDefinitionResponse> {
    info!(
        "Go to definition requested at {}:{}",
        position.line, position.character
    );

    let service = type_service?;

    // C7: Go To Definition for methods (configuration/module methods)
    if let Ok(Some(TypeDefinitionLocation::UserDefined {
        file_path,
        start_line,
        start_column,
        end_line,
        end_column,
    })) = service
        .get_method_definition_at_position_for_file(
            file_content,
            file_path.unwrap_or("definition_request.bsl"),
            position.line,
            position.character,
        )
        .await
    {
        if let Ok(target_uri) = Url::from_file_path(&file_path) {
            return Some(GotoDefinitionResponse::Scalar(Location {
                uri: target_uri,
                range: Range {
                    start: Position {
                        line: start_line,
                        character: start_column,
                    },
                    end: Position {
                        line: end_line,
                        character: end_column,
                    },
                },
            }));
        }
    }

    // Get TypeResolution at position
    let type_resolution = match service
        .get_type_at_position(file_content, position.line, position.character)
        .await
    {
        Ok(Some(resolution)) => resolution,
        Ok(None) => {
            debug!(
                "No type resolution at position {}:{}",
                position.line, position.character
            );
            return None;
        }
        Err(e) => {
            error!("Failed to get type at position: {}", e);
            return None;
        }
    };

    // Get module paths for configuration types
    let module_paths = if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) =
        &type_resolution.result
    {
        let type_key = format!("{}.{}", cfg.kind.to_prefix(), cfg.name);
        service.get_module_paths_for_type(&type_key)
    } else {
        None
    };

    // Get definition location
    let definition_location =
        type_resolution.get_definition_location_with_modules(module_paths.as_ref())?;

    // Convert to LSP GotoDefinitionResponse
    match definition_location {
        TypeDefinitionLocation::Configuration {
            metadata_path,
            module_paths,
        } => {
            // Priority: object_module > manager_module > metadata_path
            let target_path = module_paths
                .object_module
                .or(module_paths.manager_module)
                .unwrap_or(metadata_path);

            match Url::from_file_path(&target_path) {
                Ok(target_uri) => {
                    info!("Navigating to configuration type: {:?}", target_path);
                    Some(GotoDefinitionResponse::Scalar(Location {
                        uri: target_uri,
                        range: Range::default(),
                    }))
                }
                Err(_) => {
                    warn!("Invalid file path for definition: {:?}", target_path);
                    None
                }
            }
        }

        TypeDefinitionLocation::UserDefined {
            file_path,
            start_line,
            start_column,
            end_line,
            end_column,
        } => match Url::from_file_path(&file_path) {
            Ok(target_uri) => {
                info!(
                    "Navigating to user-defined type: {:?} at {}:{}",
                    file_path, start_line, start_column
                );
                Some(GotoDefinitionResponse::Scalar(Location {
                    uri: target_uri,
                    range: Range {
                        start: Position {
                            line: start_line,
                            character: start_column,
                        },
                        end: Position {
                            line: end_line,
                            character: end_column,
                        },
                    },
                }))
            }
            Err(_) => {
                warn!("Invalid file path for definition: {:?}", file_path);
                None
            }
        },

        TypeDefinitionLocation::Platform {
            type_name,
            docs_uri,
        } => {
            info!(
                "Platform type '{}' has no navigable definition, docs: {:?}",
                type_name, docs_uri
            );
            None
        }

        TypeDefinitionLocation::Primitive => {
            debug!("Primitive type has no definition location");
            None
        }
    }
}

pub async fn handle_goto_definition_v2(
    file_path: Arc<str>,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    position: Position,
    uri: &Url,
) -> Option<GotoDefinitionResponse> {
    info!(
        "Go to definition v2 requested at {}:{} (uri={}, file_path={})",
        position.line, position.character, uri, file_path
    );

    let repo = deps.repository.clone();
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(repo.clone())));

    // 1) Method/function definition (including local procedures/functions)
    if let Some(node) = ir_program.find_node_at_position(position.line, position.character) {
        if let SemanticNodeKind::FunctionCall {
            function_name,
            object_type,
            ..
        } = &node.kind
        {
            let owner_type = object_type.as_ref().map(|value| value.type_name());

            // a) repo-backed definition location (configuration/global modules, etc.)
            if let Some(loc) = repo.find_method_definition_location(
                owner_type.as_deref(),
                function_name,
            ) {
                return definition_location_to_lsp(loc);
            }

            // b) local function/procedure in the current file (private methods, etc.)
            if owner_type.is_none() {
                if let Some(span) =
                    find_local_method_declaration_span(ir_program.as_ref(), function_name)
                {
                    return Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: span.start_line,
                                character: span.start_column,
                            },
                            end: Position {
                                line: span.end_line,
                                character: span.end_column,
                            },
                        },
                    }));
                }
            }
        }
    }

    // 2) Type definition at cursor
    let type_resolution = get_type_at_position_with_semantic_program(
        ir_program.as_ref(),
        resolver.as_ref(),
        position.line,
        position.character,
    )?;

    // Module paths for configuration types (object module, manager module, etc.)
    let module_paths =
        if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &type_resolution.result
        {
            let type_key = format!("{}.{}", cfg.kind.to_prefix(), cfg.name);
            repo.find_type(&type_key)
                .and_then(|raw| raw.module_paths.clone())
        } else {
            None
        };

    let definition_location =
        type_resolution.get_definition_location_with_modules(module_paths.as_ref())?;

    definition_location_to_lsp(definition_location)
}

fn find_local_method_declaration_span(
    ir_program: &SemanticProgram,
    method_name: &str,
) -> Option<bsl_shared::ir::Span> {
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
    // Variable via SymbolTable context (flow-sensitive)
    if let Some((var_name, _type_hint, scope_id)) = ir_program.find_variable_with_scope(line, column)
    {
        return Some(resolver.resolve_variable_with_context(
            &var_name,
            &ir_program.symbols,
            scope_id,
        ));
    }

    // Fallback: try find_node_at_position for other nodes
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

fn definition_location_to_lsp(
    definition_location: TypeDefinitionLocation,
) -> Option<GotoDefinitionResponse> {
    match definition_location {
        TypeDefinitionLocation::Configuration {
            metadata_path,
            module_paths,
        } => {
            // Priority: object_module > manager_module > metadata_path
            let target_path = module_paths
                .object_module
                .or(module_paths.manager_module)
                .unwrap_or(metadata_path);

            match Url::from_file_path(&target_path) {
                Ok(target_uri) => Some(GotoDefinitionResponse::Scalar(Location {
                    uri: target_uri,
                    range: Range::default(),
                })),
                Err(_) => None,
            }
        }
        TypeDefinitionLocation::UserDefined {
            file_path,
            start_line,
            start_column,
            end_line,
            end_column,
        } => match Url::from_file_path(&file_path) {
            Ok(target_uri) => Some(GotoDefinitionResponse::Scalar(Location {
                uri: target_uri,
                range: Range {
                    start: Position {
                        line: start_line,
                        character: start_column,
                    },
                    end: Position {
                        line: end_line,
                        character: end_column,
                    },
                },
            })),
            Err(_) => None,
        },
        TypeDefinitionLocation::Platform { .. } | TypeDefinitionLocation::Primitive => None,
    }
}
