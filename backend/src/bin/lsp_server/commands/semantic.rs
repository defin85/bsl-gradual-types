//! Semantic visualization command handlers
//!
//! MILESTONE 2.12: Handles bsl/getSemanticTree and bsl/getSemanticHtml commands.

use std::sync::Arc;
use tower_lsp::lsp_types::Url;
use tracing::{error, info};

use bsl_backend::application::TypeSystemService;
use bsl_backend::system::fs_utils::read_bsl_file;
use bsl_shared::api::semantic_dtos::{
    GetSemanticHtmlRequest, GetSemanticTreeRequest, RenderedHtmlDto, SemanticNodeDto,
    SemanticTreeDto,
};
use bsl_type_visualization::{HtmlRenderer, RenderOptions, ThemeMode};

/// Handle bsl/getSemanticTree command
pub async fn handle_get_semantic_tree(
    params: GetSemanticTreeRequest,
    type_service: Option<Arc<TypeSystemService>>,
    get_document_content: impl Fn(&Url) -> Option<String>,
) -> Result<SemanticTreeDto, String> {
    info!("Custom request: bsl/getSemanticTree - {}", params.uri);

    let uri = Url::parse(&params.uri).map_err(|e| format!("Invalid URI: {}", e))?;

    let file_path = uri
        .to_file_path()
        .map_err(|_| "Could not convert URI to file path")?;

    let file_path_str = file_path.to_string_lossy().to_string();

    // Read file content (from cache or disk)
    let file_content = get_document_content(&uri)
        .or_else(|| read_bsl_file(&file_path).ok())
        .ok_or("Could not read file content")?;

    let service = type_service.ok_or("TypeSystemService not initialized")?;

    match service
        .get_semantic_tree(&file_content, &file_path_str, false, true, true)
        .await
    {
        Ok(dto) => {
            info!(
                "Semantic tree generated: {} nodes, {} symbols",
                dto.root_nodes.len(),
                dto.symbol_table.len()
            );
            Ok(dto)
        }
        Err(e) => {
            error!("Failed to generate semantic tree: {}", e);
            Err(format!("Failed to generate semantic tree: {}", e))
        }
    }
}

/// Handle bsl/getSemanticHtml command
pub async fn handle_get_semantic_html(
    params: GetSemanticHtmlRequest,
    type_service: Option<Arc<TypeSystemService>>,
    get_document_content: impl Fn(&Url) -> Option<String>,
) -> Result<RenderedHtmlDto, String> {
    info!(
        "Custom request: bsl/getSemanticHtml - {} (theme: {:?})",
        params.uri, params.theme
    );

    // First get semantic tree
    let tree_request = GetSemanticTreeRequest {
        uri: params.uri.clone(),
        include_call_graph: true,
        include_flow_sensitive: true,
        max_depth: None,
    };

    let semantic_tree =
        handle_get_semantic_tree(tree_request, type_service, get_document_content).await?;

    // Determine theme
    let theme_mode = match params.theme.as_deref() {
        Some("dark") => ThemeMode::Dark,
        Some("light") => ThemeMode::Light,
        Some("high-contrast") => ThemeMode::HighContrast,
        _ => ThemeMode::Auto,
    };

    // Create HTML renderer
    let renderer = HtmlRenderer::new(RenderOptions {
        theme: theme_mode.clone(),
        syntax_highlight: true,
        enable_links: true,
        compact: params.compact,
    });

    // Generate HTML body
    let body = format_semantic_tree_html(&semantic_tree);

    // Generate full HTML document
    let html = renderer.render_document("BSL Semantic Analysis", &body);

    Ok(RenderedHtmlDto {
        file_path: semantic_tree.file_path.clone(),
        html,
        metrics: semantic_tree.metrics.clone(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        theme: Some(format!("{:?}", theme_mode)),
    })
}

/// Format SemanticTreeDto as HTML
fn format_semantic_tree_html(tree: &SemanticTreeDto) -> String {
    let mut html = String::new();

    // Header with metrics
    html.push_str(&format!(
        r#"
        <div class="semantic-header">
            <h1>Semantic Analysis: {}</h1>
            <div class="metrics">
                <span class="metric">Procedures: {}</span>
                <span class="metric">Functions: {}</span>
                <span class="metric">Variables: {}</span>
                <span class="metric">Known Types: {}</span>
                <span class="metric">Inferred Types: {}</span>
                <span class="metric">Unknown Types: {}</span>
                <span class="metric">Analysis: {}ms</span>
            </div>
        </div>
    "#,
        tree.file_path,
        tree.metrics.procedure_count,
        tree.metrics.function_count,
        tree.metrics.variable_count,
        tree.metrics.known_types,
        tree.metrics.inferred_types,
        tree.metrics.unknown_types,
        tree.metrics.analysis_time_ms
    ));

    // Node tree
    html.push_str("<div class='semantic-tree'><h2>Semantic Tree</h2>");
    for node in &tree.root_nodes {
        html.push_str(&format_node_html(node, 0));
    }
    html.push_str("</div>");

    // Symbol table
    html.push_str("<div class='symbol-table'><h2>Symbol Table</h2><table>");
    html.push_str("<tr><th>Symbol</th><th>Type</th><th>Category</th><th>Scope</th></tr>");
    for (name, symbol) in &tree.symbol_table {
        let type_name = symbol
            .resolved_type
            .as_ref()
            .map(|t| t.name.as_str())
            .unwrap_or("?");
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            name, type_name, symbol.kind, symbol.scope
        ));
    }
    html.push_str("</table></div>");

    // CSS styles
    html.push_str(
        r#"
        <style>
            .semantic-header { background: #f0f0f0; padding: 20px; border-radius: 8px; margin-bottom: 20px; }
            .semantic-header h1 { margin: 0 0 10px 0; }
            .metrics { display: flex; gap: 15px; flex-wrap: wrap; }
            .metric { background: white; padding: 8px 12px; border-radius: 4px; font-size: 14px; }
            .semantic-tree, .symbol-table { margin: 20px 0; }
            .tree-node { margin-left: 20px; padding: 5px; border-left: 2px solid #ccc; }
            .node-header { font-weight: bold; color: #0066cc; }
            .node-name { color: #009900; }
            table { width: 100%; border-collapse: collapse; }
            th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
            th { background-color: #f2f2f2; }
        </style>
    "#,
    );

    html
}

/// Format tree node as HTML
fn format_node_html(node: &SemanticNodeDto, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let mut html = format!(
        r#"{}<div class="tree-node">
            <span class="node-header">{}</span>
            {}"#,
        indent,
        node.kind,
        node.name
            .as_ref()
            .map(|n| format!(r#"<span class="node-name">{}</span>"#, n))
            .unwrap_or_default()
    );

    // Recursively add children
    for child in &node.children {
        html.push_str(&format_node_html(child, depth + 1));
    }

    html.push_str("</div>");
    html
}
