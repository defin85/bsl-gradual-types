//! IntelliSense индексы и snapshot-консистентность (M2)

use arc_swap::ArcSwap;
use bsl_shared::domain::types::{FacetKind, MetadataKind, RawDataSource};
use bsl_shared::ir::Span;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub const INDEX_SCHEMA_VERSION: &str = "intellisense-index-v1";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndexSnapshotId(String);

impl IndexSnapshotId {
    pub fn new(config_fingerprint: &str, platform_version: &str) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(config_fingerprint.as_bytes());
        hasher.update(b"|");
        hasher.update(platform_version.as_bytes());
        hasher.update(b"|");
        hasher.update(INDEX_SCHEMA_VERSION.as_bytes());
        Self(hasher.finalize().to_hex().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_hash(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndexKind {
    Type,
    Symbol,
    Module,
    Metadata,
    Keyword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolScope {
    Local,
    Module,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Variable,
    Parameter,
    Field,
    Function,
    Procedure,
    Method,
    Constant,
    Module,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeKind {
    Platform,
    Configuration,
    Primitive,
    Faceted,
    Generic,
}

impl TypeKind {
    pub fn from_raw_source(source: &RawDataSource) -> Self {
        match source {
            RawDataSource::Platform => TypeKind::Platform,
            RawDataSource::Configuration => TypeKind::Configuration,
            RawDataSource::UserDefined => TypeKind::Generic,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndexItemKind {
    Symbol(SymbolKind),
    Type(TypeKind),
    Metadata(MetadataKind),
    Keyword,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexItem {
    pub name: String,
    pub kind: IndexItemKind,
    pub source: IndexKind,
    pub uri: Option<String>,
    pub span: Option<Span>,
    pub signature: Option<String>,
    pub facets: Vec<FacetKind>,
    pub visibility: Option<Visibility>,
    pub scope: Option<SymbolScope>,
    pub payload_version: u32,
}

impl IndexItem {
    pub fn new(name: impl Into<String>, kind: IndexItemKind, source: IndexKind) -> Self {
        Self {
            name: name.into(),
            kind,
            source,
            uri: None,
            span: None,
            signature: None,
            facets: Vec::new(),
            visibility: None,
            scope: None,
            payload_version: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSnapshot {
    pub id: IndexSnapshotId,
    pub type_index: Arc<HashMap<String, Arc<IndexItem>>>,
    pub symbol_index: Arc<HashMap<String, Arc<Vec<IndexItem>>>>,
    pub module_index: Arc<HashMap<String, Arc<Vec<IndexItem>>>>,
    pub metadata_index: Arc<HashMap<MetadataKind, Arc<Vec<IndexItem>>>>,
    pub keyword_index: Arc<Vec<IndexItem>>,
}

impl IndexSnapshot {
    pub fn empty(id: IndexSnapshotId) -> Self {
        Self {
            id,
            type_index: Arc::new(HashMap::new()),
            symbol_index: Arc::new(HashMap::new()),
            module_index: Arc::new(HashMap::new()),
            metadata_index: Arc::new(HashMap::new()),
            keyword_index: Arc::new(Vec::new()),
        }
    }
}

pub struct IntellisenseIndexStore {
    schema_version: &'static str,
    inner: ArcSwap<IndexSnapshot>,
}

impl IntellisenseIndexStore {
    pub fn new(config_fingerprint: &str, platform_version: &str) -> Self {
        let id = IndexSnapshotId::new(config_fingerprint, platform_version);
        Self {
            schema_version: INDEX_SCHEMA_VERSION,
            inner: ArcSwap::from_pointee(IndexSnapshot::empty(id)),
        }
    }

    pub fn schema_version(&self) -> &'static str {
        self.schema_version
    }

    pub fn snapshot_id(&self) -> IndexSnapshotId {
        self.inner.load().id.clone()
    }

    pub fn snapshot(&self) -> IndexSnapshot {
        self.inner.load_full().as_ref().clone()
    }

    pub fn replace_snapshot(&self, snapshot: IndexSnapshot) {
        self.inner.store(Arc::new(snapshot));
    }

    pub fn update_snapshot_id(&self, config_fingerprint: &str, platform_version: &str) {
        let new_id = IndexSnapshotId::new(config_fingerprint, platform_version);
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            snapshot.id = new_id.clone();
            Arc::new(snapshot)
        });
    }

    pub fn reset_metadata_snapshot(&self, config_fingerprint: &str, platform_version: &str) {
        let new_id = IndexSnapshotId::new(config_fingerprint, platform_version);
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            snapshot.id = new_id.clone();
            snapshot.metadata_index = Arc::new(HashMap::new());
            snapshot.type_index = Arc::new(HashMap::new());
            Arc::new(snapshot)
        });
    }

    pub fn reset_metadata_snapshot_preserving_platform_types(
        &self,
        config_fingerprint: &str,
        platform_version: &str,
    ) {
        let new_id = IndexSnapshotId::new(config_fingerprint, platform_version);
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();

            let preserved_platform: Vec<(String, Arc<IndexItem>)> = current
                .type_index
                .iter()
                .filter_map(|(name, item)| {
                    matches!(
                        item.kind,
                        IndexItemKind::Type(TypeKind::Platform | TypeKind::Primitive)
                    )
                    .then_some((name.clone(), item.clone()))
                })
                .collect();

            snapshot.id = new_id.clone();
            snapshot.metadata_index = Arc::new(HashMap::new());
            snapshot.type_index = Arc::new(HashMap::new());
            if !preserved_platform.is_empty() {
                let map = Arc::make_mut(&mut snapshot.type_index);
                for (name, item) in preserved_platform {
                    map.insert(name, item);
                }
            }
            Arc::new(snapshot)
        });
    }

    pub fn upsert_type(&self, item: IndexItem) {
        let item = Arc::new(item);
        let name = item.name.clone();
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            Arc::make_mut(&mut snapshot.type_index).insert(name.clone(), item.clone());
            Arc::new(snapshot)
        });
    }

    pub fn upsert_types(&self, items: Vec<IndexItem>) {
        if items.is_empty() {
            return;
        }

        let items: Vec<Arc<IndexItem>> = items.into_iter().map(Arc::new).collect();
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            let map = Arc::make_mut(&mut snapshot.type_index);
            for item in &items {
                map.insert(item.name.clone(), item.clone());
            }
            Arc::new(snapshot)
        });
    }

    pub fn replace_symbols_for_uri(&self, uri: &str, items: Vec<IndexItem>) {
        let uri = uri.to_string();
        let items = Arc::new(items);
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            Arc::make_mut(&mut snapshot.symbol_index).insert(uri.clone(), items.clone());
            Arc::new(snapshot)
        });
    }

    pub fn replace_modules_for_key(&self, module_key: &str, items: Vec<IndexItem>) {
        let module_key = module_key.to_string();
        let items = Arc::new(items);
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            Arc::make_mut(&mut snapshot.module_index).insert(module_key.clone(), items.clone());
            Arc::new(snapshot)
        });
    }

    pub fn replace_metadata_for_kind(&self, kind: MetadataKind, items: Vec<IndexItem>) {
        let items = Arc::new(items);
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            Arc::make_mut(&mut snapshot.metadata_index).insert(kind, items.clone());
            Arc::new(snapshot)
        });
    }

    pub fn set_keywords(&self, items: Vec<IndexItem>) {
        let items = Arc::new(items);
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            snapshot.keyword_index = items.clone();
            Arc::new(snapshot)
        });
    }

    pub fn invalidate_file(&self, uri: &str, module_key: Option<&str>) {
        let uri = uri.to_string();
        let module_key = module_key.map(str::to_string);
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            Arc::make_mut(&mut snapshot.symbol_index).remove(&uri);
            if let Some(key) = module_key.as_ref() {
                Arc::make_mut(&mut snapshot.module_index).remove(key);
            }
            Arc::new(snapshot)
        });
    }

    pub fn invalidate_metadata(&self) {
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            snapshot.metadata_index = Arc::new(HashMap::new());
            snapshot.type_index = Arc::new(HashMap::new());
            Arc::new(snapshot)
        });
    }

    pub fn invalidate_platform_types(&self) {
        self.inner.rcu(|current| {
            let mut snapshot = current.as_ref().clone();
            Arc::make_mut(&mut snapshot.type_index).retain(|_, item| {
                let item = item.as_ref();
                !matches!(
                    item.kind,
                    IndexItemKind::Type(TypeKind::Platform | TypeKind::Primitive)
                )
            });
            Arc::new(snapshot)
        });
    }

    pub fn invalidate_all(&self) {
        self.inner
            .rcu(|current| Arc::new(IndexSnapshot::empty(current.id.clone())));
    }
}

#[cfg(test)]
mod tests {
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
}
