//! Тесты для Span

use crate::ir::Span;

#[test]
fn test_span_contains() {
    let span = Span::new(10, 20);

    assert!(span.contains(10)); // Начало
    assert!(span.contains(15)); // В середине
    assert!(span.contains(19)); // Последний байт внутри (end exclusive)
    assert!(!span.contains(9)); // До начала
    assert!(!span.contains(20)); // На end (exclusive)
    assert!(!span.contains(25)); // После end
}
