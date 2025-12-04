//! Интеграционный тест для проверки void return type для методов НайтиПоКоду/НайтиПоНаименованию

use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::signature_index::{
    ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::types::{
    Certainty, ConcreteType, ConfigurationType, FacetKind, MetadataKind, ParameterInfo,
    ResolutionMetadata, ResolutionResult, ResolutionSource, TypeResolution,
};
use std::sync::Arc;

#[test]
fn test_find_by_code_return_type_from_signature_index() {
    // 1. Создаём репозиторий
    let repo = Arc::new(InMemoryTypeRepository::new());
    
    // 2. Заполняем signature_index с методом НайтиПоКоду с правильным return_type
    repo.populate_signature_index(|index: &mut SignatureIndex| {
        let method = MethodSignature::new(
            "НайтиПоКоду".to_string(),
            Some("СправочникМенеджер.<Имя справочника>".to_string()),
            vec![ParameterInfo {
                name: "Код".to_string(),
                type_name: Some("Строка".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            }],
            Some("СправочникСсылка.<Имя справочника>".to_string()),
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("СправочникМенеджер.<Имя справочника>".to_string(), method);
    });
    
    // 3. Создаём TypeResolution для Справочники.Контрагенты с Manager фасетом
    let resolution = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::Manager),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Manager),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![FacetKind::Manager],
    };
    
    // 4. Получаем методы через TypeMetadataLookup
    let lookup = TypeMetadataLookup::new(repo.clone());
    let methods = lookup.get_methods(&resolution);
    
    // 5. Проверяем, что метод НайтиПоКоду найден с правильным return_type
    let find_by_code = methods.iter().find(|m| m.name == "НайтиПоКоду");
    assert!(find_by_code.is_some(), "Метод НайтиПоКоду должен быть найден");
    
    let method = find_by_code.unwrap();
    assert_eq!(
        method.return_type, "СправочникСсылка.<Имя справочника>",
        "return_type должен быть СправочникСсылка.<Имя справочника>, а не void"
    );
}
