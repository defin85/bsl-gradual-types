//! Renderers for semantic nodes and symbol tables
//!
//! Contains functions for rendering semantic tree elements to HTML.

use bsl_shared::ir::SemanticNodeKind;

use super::styles::ColorScheme;
use super::utils::escape_html;

/// Renders semantic nodes as HTML
pub(crate) fn render_nodes(
    nodes: &[bsl_shared::ir::SemanticNode],
    colors: &ColorScheme,
    _compact: bool,
) -> String {
    nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| render_single_node(idx, node, colors))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Renders a single semantic node
fn render_single_node(
    _idx: usize,
    node: &bsl_shared::ir::SemanticNode,
    colors: &ColorScheme,
) -> String {
    let (class, emoji, kind) = match &node.kind {
        SemanticNodeKind::ProcedureDeclaration { name, params, .. } => (
            "node-procedure",
            "🔵",
            format!(
                "Procedure <span class=\"node-name\">{}</span> with {} parameters",
                escape_html(name),
                params.len()
            ),
        ),
        SemanticNodeKind::FunctionDeclaration {
            name,
            params,
            return_type,
            ..
        } => (
            "node-function",
            "⚙️",
            // Phase 3: return_type теперь Option<TypeResolution>
            format!(
                "Function <span class=\"node-name\">{}</span> ({} params) → <span class=\"detail-value\">{}</span>",
                escape_html(name),
                params.len(),
                return_type.as_ref().map(|t| escape_html(&t.type_name())).unwrap_or_else(|| "Void".to_string())
            ),
        ),
        SemanticNodeKind::VariableDeclaration {
            name,
            type_hint,
            ..
        } => (
            "node-variable",
            "📌",
            // Phase 3: type_hint теперь Option<TypeResolution>
            format!(
                "Variable <span class=\"node-name\">{}</span>: <span class=\"detail-value\">{}</span>",
                escape_html(name),
                type_hint.as_ref().map(|t| escape_html(&t.type_name())).unwrap_or_else(|| "Unknown".to_string())
            ),
        ),
        SemanticNodeKind::Assignment {
            variable,
            value_type,
            ..
        } => (
            "node-assignment",
            "✏️",
            // Phase 3: value_type теперь TypeResolution, используем type_name()
            format!(
                "Assignment <span class=\"node-name\">{}</span> = <span class=\"detail-value\">{}</span>",
                escape_html(variable),
                escape_html(&value_type.type_name())
            ),
        ),
        SemanticNodeKind::IfStatement { .. } => (
            "node-procedure",
            "🔀",
            "Conditional Statement (If...Then...Else...EndIf)".to_string(),
        ),
        SemanticNodeKind::WhileLoop { .. } => (
            "node-procedure",
            "🔄",
            "While Loop (While...Do...EndDo)".to_string(),
        ),
        SemanticNodeKind::ForLoop { variable, .. } => (
            "node-procedure",
            "🔁",
            format!("For Loop (For {} in ...)", escape_html(variable)),
        ),
        SemanticNodeKind::Return { value_type } => (
            "node-function",
            "↩️",
            format!(
                "Return Statement: <span class=\"detail-value\">{}</span>",
                // Phase 3: value_type теперь Option<TypeResolution>
                value_type.as_ref().map(|t| escape_html(&t.type_name())).unwrap_or_else(|| "Void".to_string())
            ),
        ),
        _ => (
            "node-variable",
            "❓",
            "Other Node".to_string(),
        ),
    };

    format!(
        r#"<div class="node {class}">
    <div class="node-header">
        <span class="node-kind">{emoji}</span>
        <span class="node-kind" style="background-color: {color}; color: white;">
            {kind_text}
        </span>
    </div>
    <div class="node-details">
        {kind}
    </div>
</div>"#,
        class = class,
        emoji = emoji,
        kind_text = match &node.kind {
            SemanticNodeKind::ProcedureDeclaration { .. } => "PROCEDURE",
            SemanticNodeKind::FunctionDeclaration { .. } => "FUNCTION",
            SemanticNodeKind::VariableDeclaration { .. } => "VARIABLE",
            SemanticNodeKind::Assignment { .. } => "ASSIGNMENT",
            SemanticNodeKind::IfStatement { .. } => "IF_STATEMENT",
            SemanticNodeKind::WhileLoop { .. } => "WHILE_LOOP",
            SemanticNodeKind::ForLoop { .. } => "FOR_LOOP",
            SemanticNodeKind::Return { .. } => "RETURN",
            _ => "OTHER",
        },
        kind = kind,
        color = match &node.kind {
            SemanticNodeKind::ProcedureDeclaration { .. } => colors.accent_procedure,
            SemanticNodeKind::FunctionDeclaration { .. } => colors.accent_function,
            SemanticNodeKind::VariableDeclaration { .. } => colors.accent_variable,
            SemanticNodeKind::Assignment { .. } => colors.accent_assignment,
            _ => colors.accent_procedure,
        },
    )
}

/// Renders symbol table as HTML table
pub(crate) fn render_symbol_table(symbols: &bsl_shared::ir::SymbolTable) -> String {
    // Count total symbols using public API methods
    let total_functions = symbols.functions_count();
    let total_procedures = symbols.procedures_count();

    if total_functions == 0 && total_procedures == 0 {
        return "<div class=\"info-block\">No symbols in table</div>".to_string();
    }

    let mut table_rows = String::new();

    // Add functions using public API
    // Phase 3: return_type теперь Option<TypeResolution>
    for (name, sig) in symbols.iter_functions() {
        let return_type = sig
            .return_type
            .as_ref()
            .map(|t| escape_html(&t.type_name()))
            .unwrap_or_else(|| "void".to_string());

        table_rows.push_str(&format!(
            r#"<tr>
    <td><code>{}</code></td>
    <td>Function</td>
    <td><code>{}</code></td>
</tr>"#,
            escape_html(name),
            return_type
        ));
    }

    // Add procedures using public API
    for (name, _sig) in symbols.iter_procedures() {
        table_rows.push_str(&format!(
            r#"<tr>
    <td><code>{}</code></td>
    <td>Procedure</td>
    <td><code>void</code></td>
</tr>"#,
            escape_html(name)
        ));
    }

    format!(
        r#"<table>
    <thead>
        <tr>
            <th>Name</th>
            <th>Kind</th>
            <th>Type</th>
        </tr>
    </thead>
    <tbody>
        {}
    </tbody>
</table>"#,
        table_rows
    )
}
