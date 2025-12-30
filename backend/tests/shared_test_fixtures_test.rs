mod shared_test_fixtures;

use bsl_shared::TypeRepository;

#[test]
fn test_shared_service_initialization() {
    // Проверяем что shared service инициализируется
    let _service = shared_test_fixtures::get_test_service();
    // Если дошли сюда без паники - всё работает
    // Инициализация прошла без паники — считаем успешной.
}

#[test]
fn test_shared_repository_initialization() {
    // Проверяем что shared repository инициализируется
    let repo = shared_test_fixtures::get_test_repository();
    // Если дошли сюда без паники - всё работает
    assert!(repo
        .get_signature_index_clone()
        .find_method("Массив", "Добавить")
        .is_some());
}
