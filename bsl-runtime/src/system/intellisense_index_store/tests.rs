use super::*;
use crate::system::intellisense_index::{IndexItemKind, IndexKind, SymbolKind};
use crate::system::TypeKind;
use std::sync::Arc;

#[test]
fn store_and_load_snapshot_roundtrip() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let store = IntellisenseIndexDiskStore::new(temp_dir.path().to_path_buf()).unwrap();
    let snapshot_id = IndexSnapshotId::from_hash("snapshot-1");

    let mut snapshot = IndexSnapshot::empty(snapshot_id.clone());
    Arc::make_mut(&mut snapshot.type_index).insert(
        "TestType".to_string(),
        Arc::new(IndexItem::new(
            "TestType",
            IndexItemKind::Type(TypeKind::Platform),
            IndexKind::Type,
        )),
    );
    Arc::make_mut(&mut snapshot.module_index).insert(
        "module".to_string(),
        Arc::new(vec![IndexItem::new(
            "Proc",
            IndexItemKind::Symbol(SymbolKind::Procedure),
            IndexKind::Module,
        )]),
    );
    Arc::make_mut(&mut snapshot.keyword_index).push(IndexItem::new(
        "If",
        IndexItemKind::Keyword,
        IndexKind::Keyword,
    ));
    Arc::make_mut(&mut snapshot.symbol_index).insert(
        "file:///a.bsl".to_string(),
        Arc::new(vec![IndexItem::new(
            "Var",
            IndexItemKind::Symbol(SymbolKind::Variable),
            IndexKind::Symbol,
        )]),
    );

    store.store_snapshot(&snapshot).unwrap();
    let loaded = store.load_snapshot(&snapshot_id).unwrap().expect("loaded");

    assert_eq!(loaded.id.as_str(), snapshot.id.as_str());
    assert!(loaded.type_index.contains_key("TestType"));
    assert!(loaded.module_index.contains_key("module"));
    assert_eq!(loaded.keyword_index.len(), 1);
    assert!(loaded.symbol_index.contains_key("file:///a.bsl"));
}

#[test]
fn load_returns_none_on_version_mismatch() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let store = IntellisenseIndexDiskStore::new(temp_dir.path().to_path_buf()).unwrap();
    let snapshot_id = IndexSnapshotId::from_hash("snapshot-2");

    let mut snapshot = IndexSnapshot::empty(snapshot_id.clone());
    Arc::make_mut(&mut snapshot.type_index).insert(
        "TestType".to_string(),
        Arc::new(IndexItem::new(
            "TestType",
            IndexItemKind::Type(TypeKind::Platform),
            IndexKind::Type,
        )),
    );

    store.store_snapshot(&snapshot).unwrap();

    let types_path = store.types_path(&snapshot_id);
    let bytes = fs::read(&types_path).unwrap();
    let decoded = zstd::stream::decode_all(&bytes[..]).unwrap_or(bytes);
    let mut payload: IndexStorePayload<Arc<HashMap<String, Arc<IndexItem>>>> =
        bincode::deserialize(&decoded).unwrap();
    payload.header.store_version = IndexStoreVersion("invalid-version".to_string());
    let altered = bincode::serialize(&payload).unwrap();
    let compressed = zstd::stream::encode_all(std::io::Cursor::new(&altered), 0).unwrap_or(altered);
    write_atomic(&types_path, &compressed).unwrap();

    let loaded = store.load_snapshot(&snapshot_id).unwrap();
    assert!(loaded.is_none());
}
