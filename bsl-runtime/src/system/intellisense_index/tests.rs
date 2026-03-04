use super::*;

#[test]
fn snapshot_id_is_stable_for_same_inputs() {
    let a = IndexSnapshotId::new("config-a", "platform-1");
    let b = IndexSnapshotId::new("config-a", "platform-1");
    assert_eq!(a, b);
}

#[test]
fn snapshot_is_copy_on_write() {
    let store = IntellisenseIndexStore::new("cfg", "platform");
    store.upsert_type(IndexItem::new(
        "A",
        IndexItemKind::Type(TypeKind::Platform),
        IndexKind::Type,
    ));
    let before = store.snapshot();

    store.upsert_type(IndexItem::new(
        "B",
        IndexItemKind::Type(TypeKind::Platform),
        IndexKind::Type,
    ));
    let after = store.snapshot();

    assert!(before.type_index.contains_key("A"));
    assert!(!before.type_index.contains_key("B"));
    assert!(after.type_index.contains_key("A"));
    assert!(after.type_index.contains_key("B"));
    assert!(!Arc::ptr_eq(&before.type_index, &after.type_index));
}

#[test]
fn invalidate_file_removes_symbol_and_module() {
    let store = IntellisenseIndexStore::new("cfg", "platform");
    store.replace_symbols_for_uri(
        "file:///a.bsl",
        vec![IndexItem::new(
            "Var",
            IndexItemKind::Symbol(SymbolKind::Variable),
            IndexKind::Symbol,
        )],
    );
    store.replace_modules_for_key(
        "module-a",
        vec![IndexItem::new(
            "Proc",
            IndexItemKind::Symbol(SymbolKind::Procedure),
            IndexKind::Module,
        )],
    );

    store.invalidate_file("file:///a.bsl", Some("module-a"));

    let snapshot = store.snapshot();
    assert!(snapshot.symbol_index.is_empty());
    assert!(snapshot.module_index.is_empty());
}

#[test]
fn invalidate_metadata_clears_type_and_metadata() {
    let store = IntellisenseIndexStore::new("cfg", "platform");
    store.upsert_type(IndexItem::new(
        "Catalog",
        IndexItemKind::Type(TypeKind::Configuration),
        IndexKind::Type,
    ));
    store.replace_metadata_for_kind(
        MetadataKind::Catalog,
        vec![IndexItem::new(
            "Контрагенты",
            IndexItemKind::Metadata(MetadataKind::Catalog),
            IndexKind::Metadata,
        )],
    );

    store.invalidate_metadata();

    let snapshot = store.snapshot();
    assert!(snapshot.type_index.is_empty());
    assert!(snapshot.metadata_index.is_empty());
}

#[test]
fn invalidate_platform_types_keeps_config_types() {
    let store = IntellisenseIndexStore::new("cfg", "platform");
    store.upsert_type(IndexItem::new(
        "ТаблицаЗначений",
        IndexItemKind::Type(TypeKind::Platform),
        IndexKind::Type,
    ));
    store.upsert_type(IndexItem::new(
        "Справочник.Контрагенты",
        IndexItemKind::Type(TypeKind::Configuration),
        IndexKind::Type,
    ));

    store.invalidate_platform_types();

    let snapshot = store.snapshot();
    assert_eq!(snapshot.type_index.len(), 1);
    assert!(snapshot.type_index.contains_key("Справочник.Контрагенты"));
}

#[test]
fn reset_metadata_snapshot_updates_id_and_clears_indexes() {
    let store = IntellisenseIndexStore::new("cfg", "platform");
    store.upsert_type(IndexItem::new(
        "Catalog",
        IndexItemKind::Type(TypeKind::Configuration),
        IndexKind::Type,
    ));
    store.replace_metadata_for_kind(
        MetadataKind::Catalog,
        vec![IndexItem::new(
            "Контрагенты",
            IndexItemKind::Metadata(MetadataKind::Catalog),
            IndexKind::Metadata,
        )],
    );
    let before_id = store.snapshot_id();

    store.reset_metadata_snapshot("cfg-next", "platform-next");

    let snapshot = store.snapshot();
    assert_ne!(snapshot.id, before_id);
    assert!(snapshot.type_index.is_empty());
    assert!(snapshot.metadata_index.is_empty());
}

#[test]
fn reset_metadata_snapshot_preserves_platform_and_clears_config_types() {
    let store = IntellisenseIndexStore::new("cfg", "platform");
    store.upsert_type(IndexItem::new(
        "ТаблицаЗначений",
        IndexItemKind::Type(TypeKind::Platform),
        IndexKind::Type,
    ));
    store.upsert_type(IndexItem::new(
        "Справочник.Контрагенты",
        IndexItemKind::Type(TypeKind::Configuration),
        IndexKind::Type,
    ));
    store.replace_metadata_for_kind(
        MetadataKind::Catalog,
        vec![IndexItem::new(
            "Контрагенты",
            IndexItemKind::Metadata(MetadataKind::Catalog),
            IndexKind::Metadata,
        )],
    );
    let before_id = store.snapshot_id();

    store.reset_metadata_snapshot_preserving_platform_types("cfg-next", "platform-next");

    let snapshot = store.snapshot();
    assert_ne!(snapshot.id, before_id);
    assert!(snapshot.type_index.contains_key("ТаблицаЗначений"));
    assert!(!snapshot.type_index.contains_key("Справочник.Контрагенты"));
    assert!(snapshot.metadata_index.is_empty());
}
