//! Integration tests for LSP completion pipeline (M3).

#[cfg(test)]
mod lsp_completion_tests {
    use std::sync::Arc;

    use bsl_backend::application::completion_service::get_completion;
    use bsl_backend::system::{
        keyword_index::default_keyword_items, IndexItem, IndexItemKind, IndexKind,
        IntellisenseIndexStore, TypeKind,
    };
    use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::types::{RawDataSource, RawMethodData, RawTypeData};

    #[tokio::test]
    async fn completion_returns_methods_for_platform_type() {
        let index = IntellisenseIndexStore::new("cfg", "platform");
        index.upsert_type(IndexItem::new(
            "Массив",
            IndexItemKind::Type(TypeKind::Platform),
            IndexKind::Type,
        ));

        let repository = Arc::new(InMemoryTypeRepository::new());
        repository
            .load_types(vec![RawTypeData {
                name: "Массив".to_string(),
                source: RawDataSource::Platform,
                methods: vec![RawMethodData {
                    name: "Добавить".to_string(),
                    return_type: "Булево".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }])
            .expect("load types");

        let lookup = TypeMetadataLookup::new(repository);
        let result = get_completion("Массив.", 0, 7, None, &index, &lookup)
            .await
            .expect("completion ok");

        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();
        assert!(labels.contains(&"Добавить".to_string()));
    }

    #[tokio::test]
    async fn completion_falls_back_to_default_keywords_when_index_empty() {
        let index = IntellisenseIndexStore::new("cfg", "platform");
        let repository = Arc::new(InMemoryTypeRepository::new());
        let lookup = TypeMetadataLookup::new(repository);

        let result = get_completion("", 0, 0, None, &index, &lookup)
            .await
            .expect("completion ok");
        let labels: Vec<String> = result.items.into_iter().map(|c| c.item.label).collect();

        let default_keywords = default_keyword_items();
        assert!(default_keywords
            .iter()
            .any(|item| labels.contains(&item.name)));
    }
}
