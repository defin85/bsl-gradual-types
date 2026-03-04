use super::*;

#[test]
fn test_utf16_to_byte_offset_ascii() {
    let line = "hello";
    assert_eq!(utf16_to_byte_offset(line, 0), 0);
    assert_eq!(utf16_to_byte_offset(line, 3), 3);
    assert_eq!(utf16_to_byte_offset(line, 5), 5);
}

#[test]
fn test_utf16_to_byte_offset_cyrillic() {
    let line = "Привет";
    // Each Cyrillic letter is 2 bytes in UTF-8
    assert_eq!(utf16_to_byte_offset(line, 0), 0);
    assert_eq!(utf16_to_byte_offset(line, 1), 2); // After 'П'
    assert_eq!(utf16_to_byte_offset(line, 2), 4); // After 'р'
}

#[test]
fn test_utf16_to_char_index() {
    let text = "Hello";
    assert_eq!(utf16_to_char_index(text, 0), Some(0));
    assert_eq!(utf16_to_char_index(text, 3), Some(3));
    assert_eq!(utf16_to_char_index(text, 5), Some(5));
}

#[test]
fn test_char_to_utf16_index() {
    let text = "Hello";
    assert_eq!(char_to_utf16_index(text, 0), 0);
    assert_eq!(char_to_utf16_index(text, 3), 3);
    assert_eq!(char_to_utf16_index(text, 5), 5);
}
