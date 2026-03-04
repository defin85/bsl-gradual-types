//! Symbol extraction utilities for BSL source code
//!
//! Provides functions to extract identifiers and symbols from source code
//! at specific positions, handling UTF-16 to UTF-8 conversion for LSP compatibility.

use crate::system::positioning;

/// Converts UTF-16 offset (LSP character) to UTF-8 byte offset
///
/// LSP protocol uses UTF-16 code units for positions, but Rust strings are UTF-8.
/// This function correctly converts UTF-16 offset to byte offset for working with &str[..].
///
/// # Arguments
/// * `line` - The line content as a string slice
/// * `utf16_offset` - The UTF-16 offset from LSP
///
/// # Returns
/// The corresponding byte offset in the UTF-8 string
pub fn utf16_to_byte_offset(line: &str, utf16_offset: u32) -> usize {
    positioning::utf16_to_byte_offset(line, utf16_offset)
}

/// Extracts the word at the specified position (line, column)
///
/// # Arguments
/// * `file_content` - The entire file content
/// * `line` - Zero-based line number
/// * `column` - UTF-16 column offset (as per LSP protocol)
///
/// # Returns
/// The word under cursor or None if no valid identifier found
pub fn extract_word_at_position(file_content: &str, line: u32, column: u32) -> Option<String> {
    let lines: Vec<&str> = file_content.lines().collect();
    let current_line = lines.get(line as usize)?;

    // Convert UTF-16 offset -> UTF-8 byte offset
    let byte_offset = utf16_to_byte_offset(current_line, column);

    // Find the character at byte_offset position (in terms of char indices, not bytes!)
    let mut char_index = 0;

    let chars: Vec<char> = current_line.chars().collect();
    for (idx, _ch) in current_line.char_indices() {
        if idx >= byte_offset {
            break;
        }
        char_index += 1;
    }

    if chars.is_empty() {
        return None;
    }

    if char_index >= chars.len() {
        let last_index = chars.len() - 1;
        if is_identifier_char(chars[last_index]) {
            char_index = last_index;
        } else {
            return None;
        }
    }

    // Find word start
    let mut start = char_index;
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }

    // Find word end
    let mut end = char_index;
    while end < chars.len() && is_identifier_char(chars[end]) {
        end += 1;
    }

    if start < end {
        Some(chars[start..end].iter().collect())
    } else {
        None
    }
}

/// Checks if a character is part of a BSL identifier
///
/// BSL identifiers can contain:
/// - Alphanumeric characters (ASCII and Unicode)
/// - Underscore
/// - Cyrillic characters (Unicode range 0x0400-0x04FF)
#[inline]
pub fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || ('\u{0400}'..='\u{04FF}').contains(&c)
}

#[cfg(test)]
#[path = "symbol_extractor/tests.rs"]
mod tests;
