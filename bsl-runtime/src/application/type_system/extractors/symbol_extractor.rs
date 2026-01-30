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
mod tests {
    use super::*;

    #[test]
    fn test_utf16_to_byte_offset_ascii() {
        let line = "hello world";
        assert_eq!(utf16_to_byte_offset(line, 0), 0);
        assert_eq!(utf16_to_byte_offset(line, 5), 5);
        assert_eq!(utf16_to_byte_offset(line, 6), 6);
    }

    #[test]
    fn test_utf16_to_byte_offset_cyrillic() {
        let line = "Привет мир";
        // Each Cyrillic char is 2 bytes in UTF-8, 1 UTF-16 code unit
        assert_eq!(utf16_to_byte_offset(line, 0), 0);
        assert_eq!(utf16_to_byte_offset(line, 6), 12); // After "Привет"
    }

    #[test]
    fn test_extract_word_at_position() {
        let content = "Переменная = Новый Массив;";
        let word = extract_word_at_position(content, 0, 5);
        assert_eq!(word, Some("Переменная".to_string()));
    }

    #[test]
    fn test_is_identifier_char() {
        assert!(is_identifier_char('a'));
        assert!(is_identifier_char('Z'));
        assert!(is_identifier_char('_'));
        assert!(is_identifier_char('0'));
        assert!(is_identifier_char('А')); // Cyrillic
        assert!(is_identifier_char('я')); // Cyrillic
        assert!(!is_identifier_char(' '));
        assert!(!is_identifier_char('.'));
        assert!(!is_identifier_char(';'));
    }
}
