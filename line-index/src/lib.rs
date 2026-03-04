//! Positioning helpers for UTF-16 (LSP) <-> UTF-8 byte offsets.
//!
//! Invariants:
//! - LSP `Position.character` is measured in UTF-16 code units.
//! - Rust `&str` indexing uses UTF-8 byte offsets (must be on char boundaries).

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
    text[..capped].chars().map(|c| c.len_utf16() as u32).sum()
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
        let line = self.clamp_line(line);
        let (start, end) = self.line_bounds(line, source.len());
        let end = self.trim_line_ending(source, start, end);
        &source[start..end]
    }

    /// Convert a byte column into a UTF-16 column within the specified line.
    pub fn byte_column_to_utf16(&self, source: &str, line: usize, byte_column: usize) -> u32 {
        let line_text = self.line_text(source, line);
        byte_offset_to_utf16(line_text, byte_column)
    }

    /// Convert a UTF-16 column into a byte column within the specified line.
    pub fn utf16_column_to_byte(&self, source: &str, line: usize, utf16_column: u32) -> usize {
        let line_text = self.line_text(source, line);
        utf16_to_byte_offset(line_text, utf16_column)
    }

    /// Convert a UTF-16 position (LSP) into an absolute UTF-8 byte offset in the document.
    ///
    /// Out-of-range `(line, utf16_column)` is clamped to the end of the document or line.
    pub fn utf16_position_to_byte_offset(
        &self,
        source: &str,
        line: u32,
        utf16_column: u32,
    ) -> usize {
        let row = self.clamp_line(line as usize);
        let (start, end) = self.line_bounds(row, source.len());
        let end = self.trim_line_ending(source, start, end);
        start + utf16_to_byte_offset(&source[start..end], utf16_column)
    }

    /// Convert a UTF-16 position (LSP) into a (row, byte column) pair.
    ///
    /// Out-of-range `(line, utf16_column)` is clamped to the last line and/or end of line.
    pub fn utf16_position_to_point(
        &self,
        source: &str,
        line: u32,
        utf16_column: u32,
    ) -> (usize, usize) {
        let row = self.clamp_line(line as usize);
        let column = self.utf16_column_to_byte(source, row, utf16_column);
        (row, column)
    }

    /// Convert an absolute UTF-8 byte offset into a (row, byte column) pair.
    pub fn byte_offset_to_point(&self, source: &str, byte_offset: usize) -> (usize, usize) {
        let capped = byte_offset.min(source.len());
        let line = self
            .line_starts
            .partition_point(|&start| start <= capped)
            .saturating_sub(1);
        let line_start = self.line_starts.get(line).copied().unwrap_or(0);
        (line, capped - line_start)
    }

    /// Convert an absolute UTF-8 byte offset into a UTF-16 position (LSP).
    pub fn byte_offset_to_utf16_position(&self, source: &str, byte_offset: usize) -> (u32, u32) {
        let (line, byte_column) = self.byte_offset_to_point(source, byte_offset);
        let utf16_column = self.byte_column_to_utf16(source, line, byte_column);
        (line as u32, utf16_column)
    }

    fn clamp_line(&self, line: usize) -> usize {
        line.min(self.line_count().saturating_sub(1))
    }

    fn trim_line_ending(&self, source: &str, start: usize, end: usize) -> usize {
        let bytes = source.as_bytes();
        let mut trimmed = end;
        if trimmed > start && bytes[trimmed - 1] == b'\n' {
            trimmed -= 1;
            if trimmed > start && bytes[trimmed - 1] == b'\r' {
                trimmed -= 1;
            }
        }
        trimmed
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
#[path = "lib/tests.rs"]
mod tests;
