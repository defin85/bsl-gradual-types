//! Semantic visualization command handlers
//!
//! MILESTONE 2.12: Handles bsl/getSemanticTree and bsl/getSemanticHtml commands.

use tracing::info;

use bsl_line_index::LineIndex;
use bsl_shared::api::semantic_dtos::{RenderedHtmlDto, SemanticNodeDto, SemanticTreeDto};
use bsl_type_visualization::{HtmlRenderer, RenderOptions, ThemeMode};

pub fn semantic_tree_from_ir(
    ir_program: &bsl_shared::ir::SemanticProgram,
    include_call_graph: bool,
    include_flow_sensitive: bool,
    source: &str,
    line_index: &LineIndex,
) -> SemanticTreeDto {
    info!(
        "Building semantic tree from IR (call_graph={}, flow_sensitive={})",
        include_call_graph, include_flow_sensitive
    );
    ir_program.to_dto(include_call_graph, include_flow_sensitive, source, line_index)
}

pub fn semantic_html_from_tree(
    semantic_tree: &SemanticTreeDto,
    theme: Option<&str>,
    compact: bool,
) -> RenderedHtmlDto {
    // Determine theme
    let theme_mode = match theme {
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
        compact,
    });

    // Generate HTML body
    let body = format_semantic_tree_html(semantic_tree);

    // Generate full HTML document
    let html = renderer.render_document("BSL Semantic Analysis", &body);

    RenderedHtmlDto {
        file_path: semantic_tree.file_path.clone(),
        html,
        metrics: semantic_tree.metrics.clone(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        theme: Some(format!("{:?}", theme_mode)),
    }
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
