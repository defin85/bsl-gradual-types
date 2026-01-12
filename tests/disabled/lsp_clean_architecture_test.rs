//! Disabled legacy test for LSP Clean Architecture.
//!
//! Изначально файл проверял DI вокруг legacy фасада системы типов, который был удалён при переходе
//! на v2 (salsa) как единственный путь анализа.
//!
//! Актуальные smoke-тесты v2: `tests/simplified_architecture_test.rs`.

#[test]
fn disabled_placeholder() {
    assert!(true);
}
