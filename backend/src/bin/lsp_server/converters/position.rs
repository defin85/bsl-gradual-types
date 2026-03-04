//! Position and Range converters for UTF-16 <-> UTF-8 conversion
//!
//! LSP protocol uses UTF-16 code units for positions (due to VSCode/TypeScript),
//! but Rust strings are UTF-8. This module provides conversion functions.

/// Converts UTF-16 offset (LSP character) to UTF-8 byte offset
///
/// LSP protocol uses UTF-16 code units for positions, but Rust strings use UTF-8.
/// This function correctly converts UTF-16 offset to byte offset for working with &str[..].
pub fn utf16_to_byte_offset(line: &str, utf16_offset: u32) -> usize {
    bsl_backend::system::utf16_to_byte_offset(line, utf16_offset)
}

/// Converts UTF-16 code unit index to char index
///
/// LSP positions use UTF-16 code units (due to VSCode/TypeScript),
/// while Rust strings use UTF-8 bytes and char indices.
/// This function converts UTF-16 position to char index for safe
/// work with chars() iterator.
#[cfg(test)]
pub fn utf16_to_char_index(text: &str, utf16_index: usize) -> Option<usize> {
    let mut current_utf16 = 0;

    for (char_idx, ch) in text.chars().enumerate() {
        if current_utf16 >= utf16_index {
            return Some(char_idx);
        }
        current_utf16 += ch.len_utf16();
    }

    // If we reached the end of string and utf16_index exactly matches
    if current_utf16 == utf16_index {
        Some(text.chars().count())
    } else {
        None
    }
}

/// Converts char index to UTF-16 code unit index
///
/// Reverse operation: char index -> UTF-16 position for LSP.
#[cfg(test)]
pub fn char_to_utf16_index(text: &str, char_index: usize) -> usize {
    text.chars().take(char_index).map(|ch| ch.len_utf16()).sum()
}

#[cfg(test)]
#[path = "position/tests.rs"]
mod tests;
