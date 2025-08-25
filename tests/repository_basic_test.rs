use bsl_gradual_types::core::types::FacetKind;
use bsl_gradual_types::unified::data::{
    InMemoryTypeRepository, ParseMetadata, RawMethodData, RawParameterData, RawPropertyData,
    RawTypeData, TypeFilter, TypeSource,
};

#[tokio::test]
async fn repository_save_load_stats_and_filter() {
    let repo = InMemoryTypeRepository::new();

    let platform_type = RawTypeData {
        id: "platform.Array".to_string(),
        russian_name: "Массив".to_string(),
        english_name: "Array".to_string(),
        source: TypeSource::Platform {
            version: "8.3".to_string(),
        },
        category_path: vec!["Коллекции".to_string()],
        methods: vec![RawMethodData {
            name: "Добавить".to_string(),
            documentation: String::new(),
            parameters: vec![RawParameterData {
                name: "Значение".to_string(),
                type_name: "Произвольный".to_string(),
                description: String::new(),
                is_optional: false,
                is_by_value: true,
            }],
            return_type: None,
            return_type_name: None,
            params: vec![],
            is_function: false,
            examples: vec![],
        }],
        properties: vec![RawPropertyData {
            name: "Количество".to_string(),
            type_name: "Число".to_string(),
            is_readonly: true,
            description: String::new(),
        }],
        documentation: "Коллекция упорядоченных значений".to_string(),
        examples: vec![],
        available_facets: vec![],
        parse_metadata: ParseMetadata {
            file_path: "test.html".to_string(),
            line: 0,
            column: 0,
        },
    };

    let config_type = RawTypeData {
        id: "config.Catalog.Контрагенты".to_string(),
        russian_name: "Контрагенты".to_string(),
        english_name: "Contractors".to_string(),
        source: TypeSource::Configuration {
            config_version: "8.3".to_string(),
        },
        category_path: vec!["Справочник".to_string()],
        methods: vec![],
        properties: vec![],
        documentation: String::new(),
        examples: vec![],
        available_facets: vec![],
        parse_metadata: ParseMetadata {
            file_path: "cfg.xml".to_string(),
            line: 0,
            column: 0,
        },
    };

    repo.save_types(vec![platform_type, config_type])
        .await
        .unwrap();

    let all = repo.load_all_types().await.unwrap();
    assert_eq!(all.len(), 2);

    // Stats by source
    let stats = repo.get_stats();
    assert_eq!(stats.total_types, 2);
    assert_eq!(stats.platform_types, 1);
    assert_eq!(stats.configuration_types, 1);

    // Filter: platform only
    let filtered = repo
        .load_types_filtered(&TypeFilter {
            source: Some(TypeSource::Platform {
                version: "8.3".to_string(),
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1);

    // Search by substring
    let found = repo.search_types("масс").await.unwrap();
    assert_eq!(found.len(), 1);
    assert!(found[0].russian_name.contains("Массив"));
}
