//! Тесты для Span

use crate::ir::Span;

#[test]
fn test_span_contains() {
    let span = Span::new(5, 10, 5, 20);

    assert!(span.contains(5, 15)); // В середине
    assert!(span.contains(5, 10)); // Начало
    assert!(span.contains(5, 20)); // Конец
    assert!(!span.contains(4, 15)); // До начала
    assert!(!span.contains(6, 15)); // После конца
    assert!(!span.contains(5, 5)); // До start_column
    assert!(!span.contains(5, 25)); // После end_column
}
