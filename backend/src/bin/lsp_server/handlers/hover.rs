//! Hover handler for LSP
//!
//! Handles textDocument/hover requests.

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::{debug, error};

use bsl_backend::application::TypeSystemService;
use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverOutputFormat};
use bsl_shared::formatting::DetailLevel;

use crate::config::HoverSettings;

/// Handle textDocument/hover request
pub async fn handle_hover(
    file_content: &str,
    position: Position,
    type_service: Option<Arc<TypeSystemService>>,
    settings: &HoverSettings,
) -> Option<Hover> {
    // Get syntax_helper path from environment or standard locations
    let syntax_helper_path = std::env::var("BSL_SYNTAX_HELPER_PATH")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let candidates = vec![
                std::path::PathBuf::from("examples/syntax_helper"),
                std::path::PathBuf::from("../examples/syntax_helper"),
                std::path::PathBuf::from("C:/examples/syntax_helper"),
            ];
            candidates.into_iter().find(|p| p.exists())
        });

    let detail_level = DetailLevel::parse(&settings.detail_level);

    debug!(
        "Hover request: detailLevel={:?}, maxMethods={}, maxProperties={}, showCertainty={}, syntax_helper={:?}",
        detail_level,
        settings.max_methods,
        settings.max_properties,
        settings.show_certainty,
        syntax_helper_path.as_ref().map(|p| p.display().to_string())
    );

    let hover_config = HoverFormatConfig {
        max_methods: settings.max_methods,
        max_properties: settings.max_properties,
        detail_level,
        show_certainty: settings.show_certainty,
        syntax_helper_path,
        output_format: HoverOutputFormat::Markdown,
        ..Default::default()
    };

    if let Some(service) = type_service {
        match service
            .get_hover_info(file_content, position.line, position.character, Some(hover_config))
            .await
        {
            Ok(hover_info) => {
                hover_info.map(|info| Hover {
                        contents: HoverContents::Scalar(MarkedString::String(info)),
                        range: None,
                    })
            }
            Err(e) => {
                error!("Failed to get hover info: {}", e);
                None
            }
        }
    } else {
        None
    }
}
