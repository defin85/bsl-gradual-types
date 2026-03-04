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
