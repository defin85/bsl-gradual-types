//! Persistent storage for IntelliSense indexes (M2).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;

use crate::system::intellisense_index::{
    IndexItem, IndexSnapshot, IndexSnapshotId, INDEX_SCHEMA_VERSION,
};
use bsl_shared::utils::hash::hash_content;

pub const INDEX_STORE_VERSION: &str = "intellisense-index-store-v1";
const MAX_COMPRESSED_BYTES: u64 = 20 * 1024 * 1024;
const MAX_DECODED_BYTES: usize = 100 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexStoreVersion(pub String);

impl IndexStoreVersion {
    pub fn current() -> Self {
        Self(INDEX_STORE_VERSION.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexStoreHeader {
    store_version: IndexStoreVersion,
    index_schema_version: String,
    snapshot_id: String,
    created_at: u64,
}

impl IndexStoreHeader {
    fn new(snapshot_id: &IndexSnapshotId) -> Self {
        Self {
            store_version: IndexStoreVersion::current(),
            index_schema_version: INDEX_SCHEMA_VERSION.to_string(),
            snapshot_id: snapshot_id.as_str().to_string(),
            created_at: current_timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexStorePayload<T> {
    header: IndexStoreHeader,
    payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SymbolIndexPayload {
    uri: String,
    items: Arc<Vec<IndexItem>>,
}

pub struct IntellisenseIndexDiskStore {
    root: PathBuf,
}

impl IntellisenseIndexDiskStore {
    pub fn new(root: PathBuf) -> Result<Self> {
        Self::new_with_root(root.join("index"))
    }

    pub fn new_with_root(root: PathBuf) -> Result<Self> {
        let store = Self { root };
        store.ensure_dirs()?;
        Ok(store)
    }

    pub fn root_path(&self) -> &Path {
        &self.root
    }

    pub fn store_snapshot(&self, snapshot: &IndexSnapshot) -> Result<()> {
        let header = IndexStoreHeader::new(&snapshot.id);
        self.ensure_dirs()?;

        let types_payload = IndexStorePayload {
            header: header.clone(),
            payload: snapshot.type_index.clone(),
        };
        write_payload(self.types_path(&snapshot.id), &types_payload)?;

        let modules_payload = IndexStorePayload {
            header: header.clone(),
            payload: snapshot.module_index.clone(),
        };
        write_payload(self.modules_path(&snapshot.id), &modules_payload)?;

        let metadata_payload = IndexStorePayload {
            header: header.clone(),
            payload: snapshot.metadata_index.clone(),
        };
        write_payload(self.metadata_path(&snapshot.id), &metadata_payload)?;

        let keywords_payload = IndexStorePayload {
            header: header.clone(),
            payload: snapshot.keyword_index.clone(),
        };
        write_payload(self.keywords_path(&snapshot.id), &keywords_payload)?;

        let symbols_root = self.symbols_root(&snapshot.id);
        fs::create_dir_all(&symbols_root).context("Failed to create symbol index directory")?;
        for (uri, items) in snapshot.symbol_index.iter() {
            let symbol_payload = IndexStorePayload {
                header: header.clone(),
                payload: SymbolIndexPayload {
                    uri: uri.clone(),
                    items: items.clone(),
                },
            };
            write_payload(self.symbols_path(&snapshot.id, uri), &symbol_payload)?;
        }

        Ok(())
    }

    pub fn load_snapshot(&self, snapshot_id: &IndexSnapshotId) -> Result<Option<IndexSnapshot>> {
        let mut loaded_any = false;
        let types_path = self.types_path(snapshot_id);
        let types_result = read_payload(types_path.as_path(), snapshot_id)?;
        let types = if types_result.invalid {
            return Ok(None);
        } else if let Some(payload) = types_result.payload {
            loaded_any = true;
            payload
        } else {
            Arc::new(HashMap::new())
        };

        let modules_path = self.modules_path(snapshot_id);
        let modules_result = read_payload(modules_path.as_path(), snapshot_id)?;
        let modules = if modules_result.invalid {
            return Ok(None);
        } else if let Some(payload) = modules_result.payload {
            loaded_any = true;
            payload
        } else {
            Arc::new(HashMap::new())
        };

        let metadata_path = self.metadata_path(snapshot_id);
        let metadata_result = read_payload(metadata_path.as_path(), snapshot_id)?;
        let metadata = if metadata_result.invalid {
            return Ok(None);
        } else if let Some(payload) = metadata_result.payload {
            loaded_any = true;
            payload
        } else {
            Arc::new(HashMap::new())
        };

        let keywords_path = self.keywords_path(snapshot_id);
        let keywords_result = read_payload(keywords_path.as_path(), snapshot_id)?;
        let keywords = if keywords_result.invalid {
            return Ok(None);
        } else if let Some(payload) = keywords_result.payload {
            loaded_any = true;
            payload
        } else {
            Arc::new(Vec::new())
        };

        let mut symbols: HashMap<String, Arc<Vec<IndexItem>>> = HashMap::new();
        let symbols_root = self.symbols_root(snapshot_id);
        if symbols_root.exists() {
            let entries = fs::read_dir(&symbols_root)
                .with_context(|| format!("Failed to read {}", symbols_root.display()))?;
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                match read_payload::<SymbolIndexPayload>(path.as_path(), snapshot_id) {
                    Ok(result) if result.invalid => {
                        warn!("Invalid symbol index payload, skipping {}", path.display());
                    }
                    Ok(result) if result.payload.is_some() => {
                        let payload = result.payload.expect("payload available");
                        loaded_any = true;
                        symbols.insert(payload.uri, payload.items);
                    }
                    Ok(_) => {}
                    Err(err) => {
                        warn!("Failed to read symbol index payload: {}", err);
                    }
                }
            }
        }

        if !loaded_any
            && types.is_empty()
            && modules.is_empty()
            && metadata.is_empty()
            && keywords.is_empty()
            && symbols.is_empty()
        {
            return Ok(None);
        }

        Ok(Some(IndexSnapshot {
            id: snapshot_id.clone(),
            type_index: types,
            symbol_index: Arc::new(symbols),
            module_index: modules,
            metadata_index: metadata,
            keyword_index: keywords,
        }))
    }

    fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("types"))
            .context("Failed to create type index directory")?;
        fs::create_dir_all(self.root.join("symbols"))
            .context("Failed to create symbol index directory")?;
        fs::create_dir_all(self.root.join("modules"))
            .context("Failed to create module index directory")?;
        fs::create_dir_all(self.root.join("metadata"))
            .context("Failed to create metadata index directory")?;
        fs::create_dir_all(self.root.join("keywords"))
            .context("Failed to create keyword index directory")?;
        Ok(())
    }

    fn types_path(&self, snapshot_id: &IndexSnapshotId) -> PathBuf {
        self.root
            .join("types")
            .join(format!("{}.bin", snapshot_id.as_str()))
    }

    fn modules_path(&self, snapshot_id: &IndexSnapshotId) -> PathBuf {
        self.root
            .join("modules")
            .join(format!("{}.bin", snapshot_id.as_str()))
    }

    fn metadata_path(&self, snapshot_id: &IndexSnapshotId) -> PathBuf {
        self.root
            .join("metadata")
            .join(format!("{}.bin", snapshot_id.as_str()))
    }

    fn keywords_path(&self, snapshot_id: &IndexSnapshotId) -> PathBuf {
        self.root
            .join("keywords")
            .join(format!("{}.bin", snapshot_id.as_str()))
    }

    fn symbols_root(&self, snapshot_id: &IndexSnapshotId) -> PathBuf {
        self.root.join("symbols").join(snapshot_id.as_str())
    }

    fn symbols_path(&self, snapshot_id: &IndexSnapshotId, uri: &str) -> PathBuf {
        let uri_hash = hash_content(uri);
        self.symbols_root(snapshot_id)
            .join(format!("{:016x}.bin", uri_hash))
    }
}

fn write_payload<T: Serialize>(path: PathBuf, payload: &IndexStorePayload<T>) -> Result<()> {
    let bytes = bincode::serialize(payload).context("Failed to serialize index payload")?;
    let compressed = zstd::stream::encode_all(std::io::Cursor::new(&bytes), 0).unwrap_or(bytes);
    write_atomic(&path, &compressed)
}

struct ReadPayloadResult<T> {
    payload: Option<T>,
    invalid: bool,
}

fn read_payload<T: for<'de> Deserialize<'de>>(
    path: &Path,
    snapshot_id: &IndexSnapshotId,
) -> Result<ReadPayloadResult<T>> {
    if !path.exists() {
        return Ok(ReadPayloadResult {
            payload: None,
            invalid: false,
        });
    }
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to read metadata {}", path.display()))?;
    if metadata.len() > MAX_COMPRESSED_BYTES {
        warn!(
            "Index payload too large ({} bytes), skipping {}",
            metadata.len(),
            path.display()
        );
        return Ok(ReadPayloadResult {
            payload: None,
            invalid: false,
        });
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            warn!("Failed to read {}: {}", path.display(), err);
            return Ok(ReadPayloadResult {
                payload: None,
                invalid: false,
            });
        }
    };
    if bytes.len() as u64 > MAX_COMPRESSED_BYTES {
        warn!(
            "Index payload too large ({} bytes), skipping {}",
            bytes.len(),
            path.display()
        );
        return Ok(ReadPayloadResult {
            payload: None,
            invalid: false,
        });
    }
    let decoded = decode_payload(&bytes).unwrap_or_else(|err| {
        warn!("Failed to decode {}: {}", path.display(), err);
        Vec::new()
    });
    if decoded.is_empty() {
        return Ok(ReadPayloadResult {
            payload: None,
            invalid: false,
        });
    }
    let payload: IndexStorePayload<T> = match bincode::deserialize(&decoded) {
        Ok(payload) => payload,
        Err(err) => {
            warn!("Failed to parse index payload {}: {}", path.display(), err);
            return Ok(ReadPayloadResult {
                payload: None,
                invalid: false,
            });
        }
    };
    if payload.header.store_version != IndexStoreVersion::current()
        || payload.header.index_schema_version != INDEX_SCHEMA_VERSION
        || payload.header.snapshot_id != snapshot_id.as_str()
    {
        return Ok(ReadPayloadResult {
            payload: None,
            invalid: true,
        });
    }
    Ok(ReadPayloadResult {
        payload: Some(payload.payload),
        invalid: false,
    })
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path
        .parent()
        .context("Failed to resolve parent directory")?;
    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    temp.write_all(bytes)?;
    temp.flush()?;
    if path.exists() {
        fs::remove_file(path).ok();
    }
    temp.persist(path)
        .map_err(|err| anyhow::anyhow!("Failed to persist {}: {}", path.display(), err))?;
    Ok(())
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn decode_payload(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut decoded = Vec::new();
    if let Ok(decoder) = zstd::stream::Decoder::new(bytes) {
        let mut limited = decoder.take((MAX_DECODED_BYTES + 1) as u64);
        if let Err(err) = limited.read_to_end(&mut decoded) {
            warn!("Failed to decode zstd payload: {}", err);
            decoded.clear();
        }
        if !decoded.is_empty() {
            if decoded.len() > MAX_DECODED_BYTES {
                return Err(anyhow::anyhow!("decoded payload exceeds limit"));
            }
            return Ok(decoded);
        }
    }
    if bytes.len() > MAX_DECODED_BYTES {
        return Err(anyhow::anyhow!("raw payload exceeds limit"));
    }
    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
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
        let compressed =
            zstd::stream::encode_all(std::io::Cursor::new(&altered), 0).unwrap_or(altered);
        write_atomic(&types_path, &compressed).unwrap();

        let loaded = store.load_snapshot(&snapshot_id).unwrap();
        assert!(loaded.is_none());
    }
}
