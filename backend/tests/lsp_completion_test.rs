//! Integration tests for LSP completion pipeline (M3).

#[cfg(test)]
mod lsp_completion_tests {
    use std::sync::Arc;

    use bsl_analysis_v2::{
        AnalysisHostV2, Change as ChangeV2, DepsSnapshotId, FileId as V2FileId, SettingsId,
    };
    use bsl_backend::application::get_completion_with_semantic_program_snapshot_with_trigger_hint;
    use bsl_backend::system::{
        keyword_index::default_keyword_items, IndexItem, IndexItemKind, IndexKind,
        IntellisenseIndexStore, TypeKind,
    };
    use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;
    use bsl_shared::domain::signature_index::SignatureIndex;
    use bsl_shared::domain::types::{RawDataSource, RawMethodData, RawTypeData, TypeResolution};
    use bsl_shared::formatting::DetailLevel;

    struct CompletionQuery<'a> {
        content: &'a str,
        line: u32,
        column: u32,
        file_uri: Option<&'a str>,
        owner_hint: Option<TypeResolution>,
        trigger_char_hint: Option<char>,
    }

    async fn completion_with_shared_snapshot(
        query: CompletionQuery<'_>,
        index: &IntellisenseIndexStore,
        metadata_lookup: &TypeMetadataLookup,
        deps: Arc<bsl_analysis_v2::SemanticDeps>,
    ) -> Vec<String> {
        let CompletionQuery {
            content,
            line,
            column,
            file_uri,
            owner_hint,
            trigger_char_hint,
        } = query;
        let mut host = AnalysisHostV2::default();
        let file_id = V2FileId(1);
        let file_path = file_uri.unwrap_or("inline.bsl").to_string();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("lsp-completion-shared-snapshot-test"),
            deps: deps.clone(),
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("lsp-completion-shared-snapshot-test"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id,
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from(file_path.clone()),
        });

        let analysis = host.analysis();
        analysis
            .precompute_type_index_for_file(file_id, Some(0), 0)
            .expect("precompute exact type index");
        let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");
        let resolved_file_path = analysis
            .file_path(file_id)
            .ok()
            .flatten()
            .expect("file path");
        let resolver = deps
            .resolver
            .clone()
            .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
        let index_snapshot = index.snapshot();

        get_completion_with_semantic_program_snapshot_with_trigger_hint(
            content,
            line,
            column,
            file_uri,
            &index_snapshot,
            metadata_lookup,
            resolved_file_path.as_ref(),
            resolver.as_ref(),
            ir_program,
            owner_hint,
            false,
            trigger_char_hint,
        )
        .await
        .expect("completion ok")
        .items
        .into_iter()
        .map(|candidate| candidate.item.label)
        .collect()
    }

    #[tokio::test]
    async fn completion_member_access_without_semantic_owner_stays_empty() {
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
        let deps_repo: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            repository: deps_repo.clone(),
            signature_index: SignatureIndex::new(),
            resolver: Some(Arc::new(TypeResolver::new(deps_repo))),
            platform_signatures_loaded: false,
            global_context_index: Default::default(),
        });
        let result = completion_with_shared_snapshot(
            CompletionQuery {
                content: "Массив.",
                line: 0,
                column: 7,
                file_uri: None,
                owner_hint: None,
                trigger_char_hint: Some('.'),
            },
            &index,
            &lookup,
            deps,
        )
        .await;

        assert!(
            result.is_empty(),
            "raw helper without canonical member owner must stay fail-closed, labels: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn completion_falls_back_to_default_keywords_when_index_empty() {
        let index = IntellisenseIndexStore::new("cfg", "platform");
        let repository = Arc::new(InMemoryTypeRepository::new());
        let lookup = TypeMetadataLookup::new(repository.clone());
        let deps_repo = repository as Arc<dyn TypeRepository>;
        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            repository: deps_repo.clone(),
            signature_index: SignatureIndex::new(),
            resolver: Some(Arc::new(TypeResolver::new(deps_repo))),
            platform_signatures_loaded: false,
            global_context_index: Default::default(),
        });

        let result = completion_with_shared_snapshot(
            CompletionQuery {
                content: "",
                line: 0,
                column: 0,
                file_uri: None,
                owner_hint: None,
                trigger_char_hint: None,
            },
            &index,
            &lookup,
            deps,
        )
        .await;
        let default_keywords = default_keyword_items();
        assert!(default_keywords
            .iter()
            .any(|item| result.contains(&item.name)));
    }
}
