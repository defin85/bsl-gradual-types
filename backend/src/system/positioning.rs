//! Positioning helpers for UTF-16 (LSP) <-> UTF-8 byte offsets and tree-sitter points.
//!
//! Invariants:
//! - LSP `Position.character` is measured in UTF-16 code units.
//! - Rust `&str` indexing uses UTF-8 byte offsets (must be on char boundaries).
//! - tree-sitter `Point.column` is a byte column within the line (UTF-8 bytes).

use tree_sitter::Point;

/// Convert UTF-16 code unit offset into a UTF-8 byte offset within `text`.
///
/// If `utf16_offset` points into the middle of a surrogate pair (should not happen for well-formed
/// LSP clients), the returned offset is clamped to the start of the corresponding scalar value.
pub fn utf16_to_byte_offset(text: &str, utf16_offset: u32) -> usize {
    let mut utf16_count = 0u32;
    for (byte_offset, ch) in text.char_indices() {
        if utf16_count >= utf16_offset {
            return byte_offset;
        }

        let char_utf16 = ch.len_utf16() as u32;
        if utf16_count + char_utf16 > utf16_offset {
            return byte_offset;
        }

        utf16_count += char_utf16;
    }

    text.len()
}

/// Convert UTF-8 byte offset into a UTF-16 code unit offset within `text`.
///
/// If `byte_offset` is not on a char boundary, it is clamped to the previous boundary.
pub fn byte_offset_to_utf16(text: &str, byte_offset: usize) -> u32 {
    let mut capped = byte_offset.min(text.len());
    while capped > 0 && !text.is_char_boundary(capped) {
        capped -= 1;
    }
    text[..capped]
        .chars()
        .map(|c| c.len_utf16() as u32)
        .sum()
}

#[derive(Debug, Clone)]
pub struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut line_starts = Vec::new();
        line_starts.push(0);
        for (idx, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self { line_starts }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn line_text<'a>(&self, source: &'a str, line: usize) -> &'a str {
        let (start, end) = self.line_bounds(line, source.len());
        &source[start..end]
    }

    /// Convert a byte column (tree-sitter `Point.column`) into a UTF-16 column within `line`.
    pub fn byte_column_to_utf16(&self, source: &str, line: usize, byte_column: usize) -> u32 {
        let line_text = self.line_text(source, line);
        byte_offset_to_utf16(line_text, byte_column)
    }

    /// Convert an UTF-16 column (LSP `Position.character`) into a byte column within `line`.
    pub fn utf16_column_to_byte(&self, source: &str, line: usize, utf16_column: u32) -> usize {
        let line_text = self.line_text(source, line);
        utf16_to_byte_offset(line_text, utf16_column)
    }

    /// Convert an UTF-16 position (LSP) into an absolute UTF-8 byte offset in the document.
    pub fn utf16_position_to_byte_offset(
        &self,
        source: &str,
        line: u32,
        utf16_column: u32,
    ) -> usize {
        let (start, end) = self.line_bounds(line as usize, source.len());
        start + utf16_to_byte_offset(&source[start..end], utf16_column)
    }

    /// Convert an UTF-16 position (LSP) into a tree-sitter point (row + byte column).
    pub fn utf16_position_to_point(&self, source: &str, line: u32, utf16_column: u32) -> Point {
        let row = line as usize;
        let column = self.utf16_column_to_byte(source, row, utf16_column);
        Point::new(row, column)
    }

    /// Convert an absolute UTF-8 byte offset into a tree-sitter point (row + byte column).
    pub fn byte_offset_to_point(&self, source: &str, byte_offset: usize) -> Point {
        let capped = byte_offset.min(source.len());
        let line = self
            .line_starts
            .partition_point(|&start| start <= capped)
            .saturating_sub(1);
        let line_start = self.line_starts.get(line).copied().unwrap_or(0);
        Point::new(line, capped - line_start)
    }

    /// Convert a byte column into an UTF-16 column using cached line starts.
    ///
    /// `byte_offset` is treated as a byte column within the `line`.
    pub fn byte_offset_to_utf16(&self, source: &str, line: usize, byte_offset: usize) -> u32 {
        self.byte_column_to_utf16(source, line, byte_offset)
    }

    fn line_bounds(&self, line: usize, source_len: usize) -> (usize, usize) {
        let start = self.line_starts.get(line).copied().unwrap_or(source_len);
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(source_len);
        (start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_to_byte_offset_ascii() {
        let text = "hello";
        assert_eq!(utf16_to_byte_offset(text, 0), 0);
        assert_eq!(utf16_to_byte_offset(text, 3), 3);
        assert_eq!(utf16_to_byte_offset(text, 5), 5);
    }

    #[test]
    fn utf16_to_byte_offset_cyrillic() {
        let text = "\u{041F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}";
        assert_eq!(utf16_to_byte_offset(text, 0), 0);
        assert_eq!(utf16_to_byte_offset(text, 1), "\u{041F}".len());
        assert_eq!(utf16_to_byte_offset(text, 2), "\u{041F}\u{0440}".len());
    }

    #[test]
    fn utf16_to_byte_offset_emoji_clamps_surrogate_midpoint() {
        let text = "a\u{1F600}b";
        let emoji_start = "a".len();
        let emoji_end = "a\u{1F600}".len();

        assert_eq!(utf16_to_byte_offset(text, 1), emoji_start);
        assert_eq!(utf16_to_byte_offset(text, 2), emoji_start);
        assert_eq!(utf16_to_byte_offset(text, 3), emoji_end);
    }

    #[test]
    fn line_index_converts_positions() {
        let source = "\u{0430}\u{1F600}\u{0431}\n\u{0432}\u{0433}";
        let index = LineIndex::new(source);

        assert_eq!(index.line_count(), 2);

        // line 0: "\u{0430}\u{1F600}\u{0431}\n"
        // UTF-16 columns: U+0430=1, U+1F600=2, U+0431=1
        assert_eq!(
            index.utf16_position_to_byte_offset(source, 0, 0),
            0
        );
        assert_eq!(
            index.utf16_position_to_byte_offset(source, 0, 1),
            "\u{0430}".len()
        );
        assert_eq!(
            index.utf16_position_to_byte_offset(source, 0, 2),
            "\u{0430}".len()
        );
        assert_eq!(
            index.utf16_position_to_byte_offset(source, 0, 3),
            "\u{0430}\u{1F600}".len()
        );
    }
}
