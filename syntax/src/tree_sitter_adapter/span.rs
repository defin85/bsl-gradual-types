//! Функции для извлечения `Span` из tree-sitter узлов.
//!
//! В v2 позиции в IR/AST храним **только в byte offsets** (UTF-8).
//! Конвертация byte offsets -> UTF-16 (LSP) выполняется на границе (через `bsl-line-index`).

pub use bsl_line_index::{byte_offset_to_utf16, LineIndex};
use bsl_shared::ir::Span;
use tracing::debug;
use tree_sitter::Node;

/// Извлечь Span из tree-sitter Node.
///
/// Данные берутся напрямую из tree-sitter (`start_byte`, `end_byte`).
/// Конвертацию в LSP UTF-16 позиции делайте отдельно на границе протокола.
#[allow(dead_code)]
pub fn node_to_span(node: &Node, source: &str) -> Span {
    let index = LineIndex::new(source);
    node_to_span_cached(node, source, &index)
}

/// Извлечь Span с использованием кеша строк.
///
/// Сейчас `LineIndex` не нужен для вычисления `Span` (byte offsets),
/// но сигнатура сохранена для совместимости с остальным кодом.
pub fn node_to_span_cached(node: &Node, source: &str, line_index: &LineIndex) -> Span {
    let _ = (source, line_index);
    let span = Span::new(node.start_byte() as u32, node.end_byte() as u32);

    // Milestone 2.11 Task B1: DEBUG логи для Span extraction
    debug!(
        "Extracted Span (bytes): {}..{} (node kind: {})",
        span.start,
        span.end,
        node.kind(),
    );

    span
}
