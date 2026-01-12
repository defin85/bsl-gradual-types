//! Text editing utilities for LSP.
//!
//! IntelliSense v2 keeps the source of truth in `analysis-v2` inputs; this module contains
//! pure helpers for applying LSP edits to text snapshots.

use tower_lsp::lsp_types::Range;

use crate::converters::position::utf16_to_byte_offset;

/// Apply text edit to source string
pub fn apply_text_edit(source: &str, range: Range, new_text: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let start_line = range.start.line as usize;
    let end_line = range.end.line as usize;

    // Convert UTF-16 offsets to UTF-8 byte offsets
    let start_char = if let Some(start_line_text) = lines.get(start_line) {
        utf16_to_byte_offset(start_line_text, range.start.character)
    } else {
        0
    };

    let end_char = if let Some(end_line_text) = lines.get(end_line) {
        utf16_to_byte_offset(end_line_text, range.end.character)
    } else {
        0
    };

    let mut result = String::new();

    // Lines before change
    for line in lines.iter().take(start_line) {
        result.push_str(line);
        result.push('\n');
    }

    // Start of changed line
    if let Some(start_line_text) = lines.get(start_line) {
        result.push_str(&start_line_text[..start_char.min(start_line_text.len())]);
    }

    // New text
    result.push_str(new_text);

    // End of changed line
    if let Some(end_line_text) = lines.get(end_line) {
        result.push_str(&end_line_text[end_char.min(end_line_text.len())..]);
    }

    // Lines after change
    for line in lines.iter().skip(end_line + 1) {
        result.push('\n');
        result.push_str(line);
    }

    result
}
