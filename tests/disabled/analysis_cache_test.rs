//! Disabled legacy tests for the removed in-memory analysis cache.
//!
//! Старый in-memory кэш анализа был удалён при переходе на v2.
//! Актуальные cache-проверки живут рядом с `SystemCoordinator` и disk/AST cache.

#[test]
fn disabled_placeholder() {
    assert!(true);
}
