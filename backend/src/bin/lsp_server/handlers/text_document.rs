//! Text editing utilities for LSP.
//!
//! IntelliSense v2 keeps the source of truth in `analysis-v2` inputs; this module contains
//! pure helpers for applying LSP edits to text snapshots.

use tower_lsp::lsp_types::Range;

use bsl_line_index::byte_offset_to_utf16;
use bsl_runtime::system::positioning::LineIndex;

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

    let index = LineIndex::new(source);
    let start_byte =
        index.utf16_position_to_byte_offset(source, range.start.line, range.start.character);
    let end_byte = index.utf16_position_to_byte_offset(source, range.end.line, range.end.character);
    let start_byte = start_byte.min(source.len());
    let end_byte = end_byte.min(source.len()).max(start_byte);

    let mut result =
        String::with_capacity(source.len().saturating_sub(end_byte - start_byte) + new_text.len());
    result.push_str(&source[..start_byte]);
    result.push_str(new_text);
    result.push_str(&source[end_byte..]);
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

    #[test]
    fn apply_text_edit_preserves_trailing_newline_for_interior_edit() {
        let source = "Процедура Тест()\n    Сообщить(\"один два\");\nКонецПроцедуры\n";
        let updated = apply_text_edit(
            source,
            Range {
                start: Position::new(1, "    Сообщить(\"один ".encode_utf16().count() as u32),
                end: Position::new(1, "    Сообщить(\"один два".encode_utf16().count() as u32),
            },
            "три",
        );

        assert_eq!(
            updated,
            "Процедура Тест()\n    Сообщить(\"один три\");\nКонецПроцедуры\n"
        );
    }
}
