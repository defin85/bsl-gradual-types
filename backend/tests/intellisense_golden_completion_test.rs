//! Golden tests for IntelliSense completion output (M8).

mod intellisense_testkit;

use bsl_backend::application::get_completion;
use bsl_backend::system::{
    IndexItem, IndexItemKind, IndexKind, IntellisenseIndexStore, SymbolKind, SymbolScope, TypeKind,
};
use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::types::{RawDataSource, RawMethodData, RawTypeData};
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

fn build_lookup() -> TypeMetadataLookup {
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
    TypeMetadataLookup::new(repository)
}

async fn snapshot_completion(
    content: &str,
    line: u32,
    column: u32,
    file_uri: Option<&str>,
) -> serde_json::Value {
    let index = build_index();
    let lookup = build_lookup();
    let result = get_completion(content, line, column, file_uri, &index, &lookup)
        .await
        .expect("completion ok");
    let items = result.items.into_iter().map(|c| c.item).collect::<Vec<_>>();
    intellisense_testkit::completion_snapshot_domain(&items, result.is_incomplete)
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

    let lookup = build_lookup();
    let result = get_completion(
        "Дуб",
        0,
        3,
        Some("file:///m8_minimal_completion.bsl"),
        &index,
        &lookup,
    )
    .await
    .expect("completion ok");
    let items = result.items.into_iter().map(|c| c.item).collect::<Vec<_>>();
    let snapshot = intellisense_testkit::completion_snapshot_domain(&items, result.is_incomplete);

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

    let lookup = build_lookup();
    let result = get_completion(
        "Аб",
        0,
        2,
        Some("file:///m8_minimal_completion.bsl"),
        &index,
        &lookup,
    )
    .await
    .expect("completion ok");
    let items = result.items.into_iter().map(|c| c.item).collect::<Vec<_>>();
    let snapshot = intellisense_testkit::completion_snapshot_domain(&items, result.is_incomplete);

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

    let lookup = build_lookup();
    let result = get_completion("", 0, 0, None, &index, &lookup)
        .await
        .expect("completion ok");
    let items = result.items.into_iter().map(|c| c.item).collect::<Vec<_>>();
    let snapshot = intellisense_testkit::completion_snapshot_domain(&items, result.is_incomplete);

    intellisense_testkit::assert_snapshot("m8_completion_incomplete.json", &snapshot);
}

#[tokio::test]
async fn golden_completion_member_access_falls_back_to_keywords() {
    let snapshot = snapshot_completion("Строка.", 0, 7, None).await;
    intellisense_testkit::assert_snapshot("m8_completion_member_fallback_keywords.json", &snapshot);
}
