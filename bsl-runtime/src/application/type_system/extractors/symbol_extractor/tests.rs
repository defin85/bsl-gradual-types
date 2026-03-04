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
