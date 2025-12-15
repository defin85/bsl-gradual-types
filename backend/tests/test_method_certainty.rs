use bsl_backend::system::ParserCoordinator;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::signature_index::{
    ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::type_id::TypeId;
use std::sync::Arc;

#[test]
fn test_method_return_certainty() {
    // Тест должен быть детерминированным и быстрым: не парсим весь Syntax Helper.
    // Проверяем только то, что при наличии сигнатуры метода в SignatureIndex
    // результат вызова получает Known certainty.

    let repository = Arc::new(InMemoryTypeRepository::new());

    let mut signature_index = SignatureIndex::new();
    signature_index.add_platform_method(
        TypeId::new("Массив"),
        MethodSignature::new(
            "Количество".to_string(),
            Some("Массив".to_string()),
            vec![],
            Some("Число".to_string()),
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        ),
    );
    repository.set_signature_index(signature_index);

    let parser = ParserCoordinator::new(repository);

    let code = r#"
М = Новый Массив();
Результат = М.Количество();
"#;

    let program = parser.parse_to_ir(code, "test.bsl").expect("parse_to_ir failed");
    let dto = program.to_dto(false, false);

    let rezultat = dto
        .symbol_table
        .get("Результат")
        .expect("Результат not found");
    let type_res = rezultat
        .resolved_type
        .as_ref()
        .expect("Результат has no type");

    assert_eq!(
        type_res.certainty,
        "Known",
        "Method return should have Known certainty"
    );
    assert_eq!(
        type_res.certainty_percent,
        100,
        "Known certainty should be 100%"
    );
}

