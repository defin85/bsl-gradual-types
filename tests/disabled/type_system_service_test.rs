//! Disabled legacy tests for the removed application facade.
//!
//! Эти тесты относились к старому API уровня приложения и больше не актуальны после перехода на v2.
//! См. актуальные проверки: `backend/tests` и `tests/simplified_architecture_test.rs`.

#[test]
fn disabled_placeholder() {
    assert!(true);
}
