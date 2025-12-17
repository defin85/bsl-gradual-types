//! Go To Definition handler for LSP
//!
//! Handles textDocument/definition requests.
//! Milestone 3.14: Navigates to type definitions (configuration types, platform types)

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::{debug, error, info, warn};

use bsl_backend::application::TypeSystemService;
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::types::{ConcreteType, ResolutionResult};

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
