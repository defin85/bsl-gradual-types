//! Signature Help handler for LSP
//!
//! Handles textDocument/signatureHelp requests.

use std::sync::Arc;

use tower_lsp::lsp_types::*;
use tracing::debug;

use bsl_backend::application::type_system;
use bsl_shared::ir::SemanticProgram;

pub async fn handle_signature_help_v2(
    file_content: Arc<str>,
    position: Position,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
) -> Option<SignatureHelp> {
    debug!(
        "SignatureHelp v2 requested at {}:{}",
        position.line, position.character
    );

    let data = type_system::get_signature_help_v2(
        file_content.as_ref(),
        position.line,
        position.character,
        ir_program,
        deps,
    )?;

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
