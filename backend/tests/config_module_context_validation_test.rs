//! C4: Контексты выполнения для конфигурационных методов
//!
//! Минимальная проверка: серверный метод диагностируется при вызове из клиентского контекста.

use bsl_backend::application::TypeSystemService;
use bsl_backend::system::{AnalysisCache, IrCache, ParserCoordinator};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::{
    ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::types::{RawDataSource, RawTypeData};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::TypeResolver;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::test]
async fn test_server_only_method_warns_in_client_context_for_form_module() {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());

    // Нужен реальный тип в repository, иначе семантическая валидация даст "Тип '<X>' не найден".
    repository_impl
        .load_types(vec![RawTypeData {
            name: "ТестТип".to_string(),
            english_name: String::new(),
            description: String::new(),
            category: "Platform".to_string(),
            source: RawDataSource::Platform,
            ..Default::default()
        }])
        .expect("load_types");

    let mut signature_index = SignatureIndex::new();
    signature_index.add_platform_method(
        TypeId::new("ТестТип"),
        MethodSignature::new(
            "СерверныйМетод".to_string(),
            Some("ТестТип".to_string()),
            vec![],
            Some("Неопределено".to_string()),
            SignatureSource::Platform,
            None,
            ContextRequirements::ServerOnly,
        ),
    );
    repository_impl.set_signature_index(signature_index.clone());

    // Для C4 важно, что контекст берётся из сигнатуры, а проверка идёт в семантической валидации.
    repository_impl.add_config_method_definition_location(
        "ТестТип",
        "СерверныйМетод",
        TypeDefinitionLocation::user_defined(PathBuf::from("server_method.bsl"), 0, 0, 0, 1),
    );

    let repository = repository_impl.clone() as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let analysis_engine = Arc::new(AnalysisEngine::new(resolver.clone(), repository.clone()));
    let parser = Arc::new(ParserCoordinator::new_with_resolver(
        repository.clone(),
        resolver,
    ));

    let cache = Arc::new(AnalysisCache::new(10));
    let ir_cache = Arc::new(IrCache::new(10));
    let service = TypeSystemService::new(analysis_engine, cache, parser, ir_cache);
    service.initialize().expect("initialize");
    let _metadata_lookup = TypeMetadataLookup::new(repository.clone());

    // Имитация формы: validate_semantics_with_file_path использует file_path
    // для seeding контекста формы (это важно для реальных формных модулей).
    let form_module_path = "Catalogs/Контрагенты/Forms/ФормаЭлемента/Ext/Module.bsl";

    let code = r#"
&НаКлиенте
Процедура Тест()
    М = Новый ТестТип();
    М.СерверныйМетод();
КонецПроцедуры
"#;

    let diags = service
        .validate_semantics_for_file(code, form_module_path, None)
        .await
        .expect("validate_semantics_for_file");

    assert!(
        diags.iter().any(|d| {
            d.message.contains("СерверныйМетод") && d.message.contains("недоступен")
        }),
        "expected context warning for ServerOnly method in client context, got: {diags:?}"
    );
}
