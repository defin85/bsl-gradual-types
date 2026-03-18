//! Text editing utilities for LSP.
//!
//! IntelliSense v2 keeps the source of truth in `analysis-v2` inputs; this module contains
//! pure helpers for applying LSP edits to text snapshots.

use tower_lsp::lsp_types::Range;

use bsl_line_index::byte_offset_to_utf16;

use crate::converters::position::utf16_to_byte_offset;

fn utf16_end_position(source: &str) -> (u32, u32) {
    match source.rsplit_once('\n') {
        Some((head, tail)) => (
            head.bytes().filter(|byte| *byte == b'\n').count() as u32 + 1,
            byte_offset_to_utf16(tail, tail.len()),
        ),
        None => (0, byte_offset_to_utf16(source, source.len())),
    }
}

/// Apply text edit to source string
pub fn apply_text_edit(source: &str, range: Range, new_text: &str) -> String {
    let (end_line_utf16, end_char_utf16) = utf16_end_position(source);
    if range.start == range.end
        && range.start.line == end_line_utf16
        && range.start.character == end_char_utf16
        && !new_text.is_empty()
    {
        let mut result = String::with_capacity(source.len() + new_text.len());
        result.push_str(source);
        result.push_str(new_text);
        return result;
    }

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

#[cfg(test)]
mod tests {
    use super::apply_text_edit;
    use tower_lsp::lsp_types::{Position, Range};

    #[test]
    fn apply_text_edit_fast_path_appends_at_eof() {
        let source = "Строка1\nСтрока2";
        let updated = apply_text_edit(
            source,
            Range {
                start: Position {
                    line: 1,
                    character: 7,
                },
                end: Position {
                    line: 1,
                    character: 7,
                },
            },
            "\n",
        );

        assert_eq!(updated, "Строка1\nСтрока2\n");
    }
}
