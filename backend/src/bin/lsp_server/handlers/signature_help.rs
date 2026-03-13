//! Signature Help handler for LSP
//!
//! Handles textDocument/signatureHelp requests.

use std::sync::Arc;

use tower_lsp::lsp_types::*;
use tracing::debug;

use bsl_backend::application::type_system;
use bsl_shared::ir::SemanticProgram;

pub fn handle_signature_help_v2(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: Arc<str>,
    position: Position,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    coordinator: Option<&bsl_runtime::system::SystemCoordinator>,
) -> Option<SignatureHelp> {
    debug!(
        "SignatureHelp v2 requested at {}:{}",
        position.line, position.character
    );

    let _ = deps;
    let data =
        type_system::get_signature_help_v2_with_analysis(type_system::SignatureHelpRequest {
            file_content: file_content.as_ref(),
            line: position.line,
            character: position.character,
            analysis: Some(analysis),
            file_id: Some(file_id),
            ir_program,
            coordinator,
        })?;

    let parameters = data
        .parameters
        .into_iter()
        .map(|label| ParameterInformation {
            label: ParameterLabel::Simple(label),
            documentation: None,
        })
        .collect();

    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label: data.label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: Some(data.active_parameter),
        }],
        active_signature: Some(0),
        active_parameter: Some(data.active_parameter),
    })
}

#[cfg(test)]
#[path = "signature_help/tests.rs"]
mod tests;
