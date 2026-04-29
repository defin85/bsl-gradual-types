//! Golden tests for IntelliSense completion output (M8).

mod intellisense_testkit;

use bsl_analysis_v2::{
    AnalysisHostV2, Change as ChangeV2, DepsSnapshotId, FileId as V2FileId, SettingsId,
};
use bsl_backend::application::get_completion_with_semantic_program_snapshot_with_trigger_hint;
use bsl_backend::system::{
    IndexItem, IndexItemKind, IndexKind, IntellisenseIndexStore, SymbolKind, SymbolScope, TypeKind,
};
use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{RawDataSource, RawMethodData, RawTypeData};
use bsl_shared::formatting::DetailLevel;
use bsl_shared::TypeRepository;
use std::sync::Arc;

fn build_index() -> IntellisenseIndexStore {
    let index = IntellisenseIndexStore::new("m8", "platform");
    index.set_keywords(vec![
        IndexItem::new("Процедура", IndexItemKind::Keyword, IndexKind::Keyword),
        IndexItem::new("Функция", IndexItemKind::Keyword, IndexKind::Keyword),
        IndexItem::new("Перем", IndexItemKind::Keyword, IndexKind::Keyword),
        IndexItem::new("Старт", IndexItemKind::Keyword, IndexKind::Keyword),
    ]);
    index.upsert_type(IndexItem::new(
        "Массив",
        IndexItemKind::Type(TypeKind::Platform),
        IndexKind::Type,
    ));
    index.upsert_type(IndexItem::new(
        "Строка",
        IndexItemKind::Type(TypeKind::Primitive),
        IndexKind::Type,
    ));
    let uri = "file:///m8_minimal_completion.bsl";
    let symbols = vec![
        symbol_item("Локальная", SymbolScope::Local, Some(uri)),
        symbol_item("Глобальная", SymbolScope::Global, None),
    ];
    index.replace_symbols_for_uri(uri, symbols);
    index
}

fn symbol_item(name: &str, scope: SymbolScope, uri: Option<&str>) -> IndexItem {
    let mut item = IndexItem::new(
        name,
        IndexItemKind::Symbol(SymbolKind::Variable),
        IndexKind::Symbol,
    );
    item.scope = Some(scope);
    item.uri = uri.map(|value| value.to_string());
    item
}

fn function_item(name: &str, scope: Option<SymbolScope>) -> IndexItem {
    let mut item = IndexItem::new(
        name,
        IndexItemKind::Symbol(SymbolKind::Function),
        IndexKind::Symbol,
    );
    item.scope = scope;
    item
}

fn build_lookup_and_deps() -> (TypeMetadataLookup, Arc<bsl_analysis_v2::SemanticDeps>) {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
                name: "Массив".to_string(),
                source: RawDataSource::Platform,
                methods: vec![RawMethodData {
                    name: "Добавить".to_string(),
                    return_type: "Булево".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
            RawTypeData {
                name: "Строка".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ])
        .expect("load types");
    let lookup = TypeMetadataLookup::new(repository.clone());
    let deps_repo = repository as Arc<dyn TypeRepository>;
    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        repository: deps_repo.clone(),
        signature_index: SignatureIndex::new(),
        resolver: Some(Arc::new(TypeResolver::new(deps_repo))),
        platform_signatures_loaded: false,
        common_module_factory_registry: Default::default(),
        global_context_index: Default::default(),
    });
    (lookup, deps)
}

async fn completion_with_shared_snapshot(
    content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
    index: &IntellisenseIndexStore,
    lookup: &TypeMetadataLookup,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
) -> (Vec<bsl_shared::domain::CompletionItem>, bool) {
    let mut host = AnalysisHostV2::default();
    let file_id = V2FileId(1);
    let file_path = file_uri.unwrap_or("inline.bsl").to_string();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("golden-completion-shared-snapshot"),
        deps: deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("golden-completion-shared-snapshot"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id,
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from(file_path),
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

    let result = get_completion_with_semantic_program_snapshot_with_trigger_hint(
        content,
        line,
        column,
        file_uri,
        &index_snapshot,
        lookup,
        resolved_file_path.as_ref(),
        resolver.as_ref(),
        ir_program,
        None,
        false,
        None,
    )
    .await
    .expect("completion ok");

    (
        result
            .items
            .into_iter()
            .map(|candidate| candidate.item)
            .collect(),
        result.is_incomplete,
    )
}

async fn snapshot_completion(
    content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
) -> serde_json::Value {
    let index = build_index();
    let (lookup, deps) = build_lookup_and_deps();
    let (items, is_incomplete) =
        completion_with_shared_snapshot(content, line, column, file_uri, &index, &lookup, deps)
            .await;
    intellisense_testkit::completion_snapshot_domain(&items, is_incomplete)
}

#[tokio::test]
async fn golden_completion_prefix_keyword() {
    let content = "Про";
    let snapshot = snapshot_completion(content, 0, 3, None).await;
    intellisense_testkit::assert_snapshot("m8_completion_prefix_keyword.json", &snapshot);
}

#[tokio::test]
async fn golden_completion_member_access_without_semantic_owner_is_empty() {
    let content = "Массив.";
    let snapshot = snapshot_completion(content, 0, 7, None).await;
    intellisense_testkit::assert_snapshot("m8_completion_member_access.json", &snapshot);
}

#[tokio::test]
async fn golden_completion_non_member_includes_keywords() {
    let content = "";
    let snapshot = snapshot_completion(content, 0, 0, None).await;
    intellisense_testkit::assert_snapshot("m8_completion_non_member.json", &snapshot);
}

#[tokio::test]
async fn golden_completion_dedup_sources() {
    let index = build_index();
    index.replace_symbols_for_uri(
        "file:///m8_minimal_completion.bsl",
        vec![
            function_item("Дубль", Some(SymbolScope::Local)),
            function_item("Дубль", Some(SymbolScope::Local)),
        ],
    );
    index.replace_modules_for_key(
        "module::m8",
        vec![function_item("Дубль", Some(SymbolScope::Local))],
    );

    let (lookup, deps) = build_lookup_and_deps();
    let (items, is_incomplete) = completion_with_shared_snapshot(
        "Дуб",
        0,
        3,
        Some("file:///m8_minimal_completion.bsl"),
        &index,
        &lookup,
        deps,
    )
    .await;
    let snapshot = intellisense_testkit::completion_snapshot_domain(&items, is_incomplete);

    intellisense_testkit::assert_snapshot("m8_completion_dedup_sources.json", &snapshot);
}

#[tokio::test]
async fn golden_completion_ordering_stable() {
    let index = build_index();
    index.replace_symbols_for_uri(
        "file:///m8_minimal_completion.bsl",
        vec![
            function_item("Абв", Some(SymbolScope::Local)),
            function_item("Абг", Some(SymbolScope::Local)),
        ],
    );

    let (lookup, deps) = build_lookup_and_deps();
    let (items, is_incomplete) = completion_with_shared_snapshot(
        "Аб",
        0,
        2,
        Some("file:///m8_minimal_completion.bsl"),
        &index,
        &lookup,
        deps,
    )
    .await;
    let snapshot = intellisense_testkit::completion_snapshot_domain(&items, is_incomplete);

    intellisense_testkit::assert_snapshot("m8_completion_ordering.json", &snapshot);
}

#[tokio::test]
async fn golden_completion_types_and_keywords() {
    let snapshot = snapshot_completion("Ст", 0, 2, None).await;
    intellisense_testkit::assert_snapshot("m8_completion_types_keywords.json", &snapshot);
}

#[tokio::test]
async fn golden_completion_symbols_in_scope() {
    let snapshot =
        snapshot_completion("Лок", 0, 3, Some("file:///m8_minimal_completion.bsl")).await;
    intellisense_testkit::assert_snapshot("m8_completion_symbols.json", &snapshot);
}

#[tokio::test]
async fn golden_completion_case_insensitive_prefix() {
    let snapshot = snapshot_completion("масс", 0, 4, None).await;
    intellisense_testkit::assert_snapshot("m8_completion_case_insensitive.json", &snapshot);
}

#[tokio::test]
async fn golden_completion_incomplete_flag() {
    let index = build_index();
    let keywords = (0..230)
        .map(|i| {
            IndexItem::new(
                format!("Ключ{}", i),
                IndexItemKind::Keyword,
                IndexKind::Keyword,
            )
        })
        .collect();
    index.set_keywords(keywords);

    let (lookup, deps) = build_lookup_and_deps();
    let (items, is_incomplete) =
        completion_with_shared_snapshot("", 0, 0, None, &index, &lookup, deps).await;
    let snapshot = intellisense_testkit::completion_snapshot_domain(&items, is_incomplete);

    intellisense_testkit::assert_snapshot("m8_completion_incomplete.json", &snapshot);
}

#[tokio::test]
async fn golden_completion_member_access_stays_empty_without_semantic_owner() {
    let snapshot = snapshot_completion("Строка.", 0, 7, None).await;
    intellisense_testkit::assert_snapshot("m8_completion_member_fallback_keywords.json", &snapshot);
}
