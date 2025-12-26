//! C7: Go To Definition для методов/функций
//!
//! Проверяет:
//! - переход к объявлению метода по индексу `method -> location`
//! - переход к объявлению глобальной функции по индексу
//! - fallback на локальное объявление в текущем файле

use bsl_backend::application::TypeSystemService;
use bsl_backend::system::{AnalysisCache, IntellisenseIndexStore, IrCache, ParserCoordinator};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::{
    ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::TypeResolver;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

fn create_minimal_service() -> (TypeSystemService, Arc<InMemoryTypeRepository>) {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());

    let mut signature_index = SignatureIndex::new();
    signature_index.add_platform_method(
        TypeId::new("ТестТип"),
        MethodSignature::new(
            "СерверныйМетод".to_string(),
            Some("ТестТип".to_string()),
            vec![],
            Some("Неопределено".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        ),
    );
    repository_impl.set_signature_index(signature_index);

    let repository = repository_impl.clone() as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let analysis_engine = Arc::new(AnalysisEngine::new(resolver.clone(), repository.clone()));
    let cache = Arc::new(AnalysisCache::new(10));
    let ir_cache = Arc::new(IrCache::new(10));
    let parser = Arc::new(ParserCoordinator::new_with_resolver(
        repository.clone(),
        resolver,
    ));
    let intellisense_index = Arc::new(IntellisenseIndexStore::new("test-config", "test-platform"));

    (
        TypeSystemService::new(analysis_engine, cache, parser, ir_cache, intellisense_index),
        repository_impl,
    )
}

#[tokio::test]
async fn test_goto_definition_for_indexed_method() {
    let (service, repo) = create_minimal_service();

    let tmp = TempDir::new().unwrap();
    let def_path = tmp.path().join("def.bsl");

    repo.add_config_method_definition_location(
        "ТестТип",
        "СерверныйМетод",
        TypeDefinitionLocation::user_defined(def_path.clone(), 10, 3, 10, 20),
    );

    let code = r#"
Процедура Тест()
    М = Новый ТестТип();
    М.СерверныйМетод();
КонецПроцедуры
"#;

    // line=3: "    М.СерверныйМетод();", column=6: на букве 'С'
    let loc = service
        .get_method_definition_at_position_for_file(code, "test_form_module.bsl", 3, 6)
        .await
        .expect("get_method_definition_at_position_for_file");

    let Some(TypeDefinitionLocation::UserDefined { file_path, .. }) = loc else {
        panic!("expected UserDefined location, got: {loc:?}");
    };

    assert_eq!(file_path, def_path);
}

#[tokio::test]
async fn test_goto_definition_for_indexed_global_function() {
    let (service, repo) = create_minimal_service();

    let tmp = TempDir::new().unwrap();
    let def_path = tmp.path().join("globals.bsl");

    repo.add_global_function_definition_location(
        "ГлобальнаяФункция",
        TypeDefinitionLocation::user_defined(def_path.clone(), 2, 0, 2, 10),
    );

    let code = r#"
Процедура Тест()
    ГлобальнаяФункция();
КонецПроцедуры
"#;

    // line=2: "    ГлобальнаяФункция();", column=6: на букве 'б'
    let loc = service
        .get_method_definition_at_position_for_file(code, "test_globals.bsl", 2, 6)
        .await
        .expect("get_method_definition_at_position_for_file");

    let Some(TypeDefinitionLocation::UserDefined { file_path, .. }) = loc else {
        panic!("expected UserDefined location, got: {loc:?}");
    };

    assert_eq!(file_path, def_path);
}

#[tokio::test]
async fn test_goto_definition_falls_back_to_local_declaration() {
    let (service, _repo) = create_minimal_service();

    let file_path = "test_local.bsl";
    let code = r#"Функция Локальная()
    Возврат 1;
КонецФункции

Процедура Тест()
    Локальная();
КонецПроцедуры
"#;

    // line=5: "    Локальная();", column=6: на букве 'к'
    let loc = service
        .get_method_definition_at_position_for_file(code, file_path, 5, 6)
        .await
        .expect("get_method_definition_at_position_for_file");

    let Some(TypeDefinitionLocation::UserDefined { file_path, start_line, .. }) = loc else {
        panic!("expected UserDefined location, got: {loc:?}");
    };

    assert_eq!(file_path, PathBuf::from("test_local.bsl"));
    assert_eq!(start_line, 0);
}
