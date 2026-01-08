//! Positioning helpers for UTF-16 (LSP) <-> UTF-8 byte offsets and tree-sitter points.
//!
//! Invariants:
//! - LSP `Position.character` is measured in UTF-16 code units.
//! - Rust `&str` indexing uses UTF-8 byte offsets (must be on char boundaries).
//! - tree-sitter `Point.column` is a byte column within the line (UTF-8 bytes).
//!
//! Note: core conversion logic lives in `bsl-line-index` crate. This module provides a thin
//! adapter that keeps legacy `tree_sitter::Point`-based API.

use tree_sitter::Point;

pub use bsl_line_index::{byte_offset_to_utf16, utf16_to_byte_offset};

#[derive(Debug, Clone)]
pub struct LineIndex {
    inner: bsl_line_index::LineIndex,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        Self {
            inner: bsl_line_index::LineIndex::new(source),
        }
    }

    pub fn line_count(&self) -> usize {
        self.inner.line_count()
    }

    pub fn line_text<'a>(&self, source: &'a str, line: usize) -> &'a str {
        self.inner.line_text(source, line)
    }

    /// Convert a byte column (tree-sitter `Point.column`) into a UTF-16 column within `line`.
    pub fn byte_column_to_utf16(&self, source: &str, line: usize, byte_column: usize) -> u32 {
        self.inner.byte_column_to_utf16(source, line, byte_column)
    }

    /// Convert an UTF-16 column (LSP `Position.character`) into a byte column within `line`.
    pub fn utf16_column_to_byte(&self, source: &str, line: usize, utf16_column: u32) -> usize {
        self.inner.utf16_column_to_byte(source, line, utf16_column)
    }

    /// Convert an UTF-16 position (LSP) into an absolute UTF-8 byte offset in the document.
    ///
    /// Out-of-range `(line, utf16_column)` is clamped (LSP-like behavior).
    pub fn utf16_position_to_byte_offset(
        &self,
        source: &str,
        line: u32,
        utf16_column: u32,
    ) -> usize {
        self.inner
            .utf16_position_to_byte_offset(source, line, utf16_column)
    }

    /// Convert an UTF-16 position (LSP) into a tree-sitter point (row + byte column).
    pub fn utf16_position_to_point(&self, source: &str, line: u32, utf16_column: u32) -> Point {
        let (row, column) = self.inner.utf16_position_to_point(source, line, utf16_column);
        Point::new(row, column)
    }

    /// Convert an absolute UTF-8 byte offset into a tree-sitter point (row + byte column).
    pub fn byte_offset_to_point(&self, source: &str, byte_offset: usize) -> Point {
        let (row, column) = self.inner.byte_offset_to_point(source, byte_offset);
        Point::new(row, column)
    }

    /// Convert a byte column into an UTF-16 column using cached line starts.
    ///
    /// `byte_offset` is treated as a byte column within the `line`.
    pub fn byte_offset_to_utf16(&self, source: &str, line: usize, byte_offset: usize) -> u32 {
        self.byte_column_to_utf16(source, line, byte_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_index_point_api_matches_shared_point() {
        let source = "\u{0430}\u{1F600}\u{0431}\n\u{0432}\u{0433}";
        let index = LineIndex::new(source);
        let shared = bsl_line_index::LineIndex::new(source);

        let point = index.utf16_position_to_point(source, 0, 3);
        let (row, column) = shared.utf16_position_to_point(source, 0, 3);
        assert_eq!(point, Point::new(row, column));

        let point = index.byte_offset_to_point(source, "\u{0430}\u{1F600}".len());
        let (row, column) = shared.byte_offset_to_point(source, "\u{0430}\u{1F600}".len());
        assert_eq!(point, Point::new(row, column));
    }
}
