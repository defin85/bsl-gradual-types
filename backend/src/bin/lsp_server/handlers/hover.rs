//! Hover handler for LSP
//!
//! Handles textDocument/hover requests.

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::debug;

use bsl_backend::application::get_hover_info_with_semantic_program;
use bsl_backend::helpers::hover_formatter::HoverFormatter;
use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverOutputFormat};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::{normalize_user_facing_type_name, DetailLevel};
use bsl_shared::ir::SemanticProgram;

use crate::config::HoverSettings;

#[allow(clippy::too_many_arguments)]
pub fn handle_hover_v2(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: Arc<str>,
    file_path: Arc<str>,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    position: Position,
    uri: &Url,
    settings: &HoverSettings,
    include_flow_sensitive: bool,
) -> Option<Hover> {
    // Get syntax_helper path from environment or standard locations
    let syntax_helper_path = bsl_runtime::system::global_runtime_config()
        .get_pathbuf(bsl_runtime::system::RuntimeKey::SyntaxHelperPath)
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
        "Hover v2 requested: uri={}, file_path={}, detailLevel={:?}, maxMethods={}, maxProperties={}, showCertainty={}, syntax_helper={:?}",
        uri,
        file_path,
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

    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
    let hover_formatter = HoverFormatter::new(hover_config, metadata_lookup.clone());

    let hover_info = get_hover_info_with_semantic_program(
        analysis,
        file_id,
        file_content.as_ref(),
        position.line,
        position.character,
        include_flow_sensitive,
        &metadata_lookup,
        &hover_formatter,
        None,
        resolver.as_ref(),
        ir_program,
    );

    hover_info
        .map(|info| Hover {
            contents: HoverContents::Scalar(MarkedString::String(info)),
            range: None,
        })
        .map(|mut hover| {
            if let HoverContents::Scalar(MarkedString::String(value)) = &mut hover.contents {
                *value = normalize_user_facing_type_name(value);
            }
            hover
        })
}

#[cfg(test)]
#[path = "hover/tests.rs"]
mod tests;
