use super::*;

#[test]
fn utf16_to_byte_offset_ascii() {
    let text = "hello";
    assert_eq!(utf16_to_byte_offset(text, 0), 0);
    assert_eq!(utf16_to_byte_offset(text, 3), 3);
    assert_eq!(utf16_to_byte_offset(text, 5), 5);
    assert_eq!(utf16_to_byte_offset(text, 999), 5);
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
    assert_eq!(utf16_to_byte_offset(text, 4), text.len());
}

#[test]
fn byte_offset_to_utf16_clamps_to_char_boundary() {
    let text = "\u{0430}\u{1F600}\u{0431}";
    let emoji_byte_mid = "\u{0430}".len() + 1;
    assert_eq!(byte_offset_to_utf16(text, emoji_byte_mid), 1);
}

#[test]
fn line_index_converts_positions_and_clamps() {
    let source = "\u{0430}\u{1F600}\u{0431}\n\u{0432}\u{0433}";
    let index = LineIndex::new(source);

    assert_eq!(index.line_count(), 2);

    // line 0: "\u{0430}\u{1F600}\u{0431}"
    // UTF-16 columns: U+0430=1, U+1F600=2, U+0431=1
    assert_eq!(index.utf16_position_to_byte_offset(source, 0, 0), 0);
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
    assert_eq!(
        index.utf16_position_to_byte_offset(source, 0, 4),
        "\u{0430}\u{1F600}\u{0431}".len()
    );
    assert_eq!(
        index.utf16_position_to_byte_offset(source, 0, 999),
        "\u{0430}\u{1F600}\u{0431}".len()
    );

    assert_eq!(index.utf16_position_to_point(source, 999, 999), (1, 4));
}

#[test]
fn line_index_roundtrip_byte_offset_utf16_position() {
    let source = "\u{0430}\u{1F600}\u{0431}\n\u{0432}\u{0433}";
    let index = LineIndex::new(source);
    let byte_offset = "\u{0430}\u{1F600}".len();

    let (line, utf16_col) = index.byte_offset_to_utf16_position(source, byte_offset);
    assert_eq!((line, utf16_col), (0, 3));
    assert_eq!(
        index.utf16_position_to_byte_offset(source, line, utf16_col),
        byte_offset
    );
}

#[test]
fn line_text_trims_line_endings() {
    let source = "a\r\nb\nc";
    let index = LineIndex::new(source);
    assert_eq!(index.line_text(source, 0), "a");
    assert_eq!(index.line_text(source, 1), "b");
    assert_eq!(index.line_text(source, 2), "c");
}
