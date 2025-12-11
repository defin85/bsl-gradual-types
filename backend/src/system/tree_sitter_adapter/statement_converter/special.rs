//! Специальные statements: goto, label, execute
//!
//! Эти statements являются специфичными для языка 1С
//! и не содержат вложенных блоков кода.

use crate::parsing::bsl::ast::Statement;
use tree_sitter::Node;

use crate::system::tree_sitter_adapter::expression_converter::convert_expression;
use crate::system::tree_sitter_adapter::span::node_to_span_cached;
use crate::system::tree_sitter_adapter::utils::node_text;

/// Конвертировать goto_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_goto_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let label = node_text(&child, source);
            return Ok(Statement::Goto { label, span });
        }
    }
    Err("goto_statement without label".to_string())
}

/// Конвертировать label_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_label_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let name = node_text(&child, source);
            return Ok(Statement::Label { name, span });
        }
    }
    Err("label_statement without name".to_string())
}

/// Конвертировать execute_statement с использованием кеша строк (Milestone 2.19)
pub(crate) fn convert_execute_statement_cached(
    node: &Node,
    source: &str,
    lines: &[String],
) -> Result<Statement, String> {
    let span = node_to_span_cached(node, source, lines);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(expr) = convert_expression(&child, source)? {
            return Ok(Statement::Execute { code: expr, span });
        }
    }
    Err("execute_statement without code".to_string())
}
