//! Go To Definition handler for LSP
//!
//! Handles textDocument/definition requests.
//! Milestone 3.14: Navigates to type definitions (configuration types, platform types)

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::info;

use bsl_backend::application::type_system;
use bsl_shared::ir::SemanticProgram;

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

    let target = type_system::goto_definition_v2(
        file_path.as_ref(),
        ir_program,
        deps,
        position.line,
        position.character,
    )?;

    let target_uri = Url::from_file_path(&target.file_path).ok()?;
    let range = match target.span {
        Some(span) => Range {
            start: Position {
                line: span.start_line,
                character: span.start_column,
            },
            end: Position {
                line: span.end_line,
                character: span.end_column,
            },
        },
        None => Range::default(),
    };

    Some(GotoDefinitionResponse::Scalar(Location {
        uri: target_uri,
        range,
    }))
}
