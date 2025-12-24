//! Disk cache with manifest and file locking for cross-process reuse.

use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct DiskCacheKey {
    pub source_kind: String,
    pub key: String,
    pub project_id: Option<String>,
    pub config_id: Option<String>,
    pub source_identity: String,
    pub source_fingerprint: String,
    pub settings_fingerprint: String,
}

impl DiskCacheKey {
    pub fn new(
        source_kind: impl Into<String>,
        key: impl Into<String>,
        source_identity: impl Into<String>,
        source_fingerprint: impl Into<String>,
        settings_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            source_kind: source_kind.into(),
            key: key.into(),
            project_id: None,
            config_id: None,
            source_identity: source_identity.into(),
            source_fingerprint: source_fingerprint.into(),
            settings_fingerprint: settings_fingerprint.into(),
        }
    }

    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    pub fn with_config_id(mut self, config_id: impl Into<String>) -> Self {
        self.config_id = Some(config_id.into());
        self
    }
}

#[derive(Debug)]
pub struct CacheEntry<T> {
    pub value: T,
    pub from_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheManifest {
    pub schema_version: u32,
    pub source_kind: String,
    pub source_identity: String,
    pub source_fingerprint: String,
    pub settings_fingerprint: String,
    pub created_at: u64,
    pub size_bytes: u64,
    pub build_time_ms: u64,
}

pub struct DiskCache {
    root: PathBuf,
    schema_version: u32,
    disabled: bool,
}

impl DiskCache {
    pub fn new(schema_version: u32) -> Result<Self> {
        let root = resolve_cache_root();
        Self::with_root(root, schema_version)
    }

    pub fn with_root(root: PathBuf, schema_version: u32) -> Result<Self> {
        let disabled = is_cache_disabled();
        if !disabled {
            fs::create_dir_all(&root).context("Failed to create disk cache root")?;
        }
        Ok(Self {
            root,
            schema_version,
            disabled,
        })
    }

    pub fn disabled(schema_version: u32) -> Self {
        Self {
            root: PathBuf::from(".bsl_cache"),
            schema_version,
            disabled: true,
        }
    }

    pub fn get_or_build<T, F>(&self, key: &DiskCacheKey, build: F) -> Result<CacheEntry<T>>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Result<T>,
    {
        self.get_or_build_with(key, build, |_| true)
    }

    pub fn get_or_build_with<T, F, P>(
        &self,
        key: &DiskCacheKey,
        build: F,
        should_cache: P,
    ) -> Result<CacheEntry<T>>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Result<T>,
        P: Fn(&T) -> bool,
    {
        if self.disabled {
            let value = build()?;
            return Ok(CacheEntry {
                value,
                from_cache: false,
            });
        }

        let cache_dir = self.cache_dir(key);
        fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

        let _lock = self.lock_key(&cache_dir)?;

        match self.try_load::<T>(key, &cache_dir) {
            Ok(Some(value)) => {
                return Ok(CacheEntry {
                    value,
                    from_cache: true,
                })
            }
            Ok(None) => {}
            Err(err) => {
                warn!("Disk cache read failed: {}", err);
            }
        }

        let started = Instant::now();
        let value = build()?;
        let build_time_ms = started.elapsed().as_millis() as u64;

        if should_cache(&value) {
            if let Err(err) = self.store(key, &cache_dir, &value, build_time_ms) {
                warn!("Disk cache store failed: {}", err);
            }
        } else {
            debug!("Skip caching for key {}", key.key);
        }

        Ok(CacheEntry {
            value,
            from_cache: false,
        })
    }

    fn cache_dir(&self, key: &DiskCacheKey) -> PathBuf {
        let project_id = key
            .project_id
            .as_deref()
            .unwrap_or("global")
            .to_string();
        let config_id = key.config_id.as_deref().unwrap_or("none").to_string();
        self.root
            .join(format!("v{}", self.schema_version))
            .join(sanitize_component(&key.source_kind))
            .join(sanitize_component(&project_id))
            .join(sanitize_component(&config_id))
            .join(sanitize_component(&key.key))
    }

    fn lock_key(&self, cache_dir: &Path) -> Result<DiskCacheLock> {
        let lock_path = cache_dir.join(".lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock_path)
            .with_context(|| format!("Failed to open lock file {}", lock_path.display()))?;
        file.lock_exclusive()
            .context("Failed to acquire cache lock")?;
        Ok(DiskCacheLock { _file: file })
    }

    fn try_load<T>(&self, key: &DiskCacheKey, cache_dir: &Path) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let manifest_path = cache_dir.join("manifest.json");
        let artifact_path = cache_dir.join("artifact.json");

        if !manifest_path.exists() || !artifact_path.exists() {
            return Ok(None);
        }

        let manifest_bytes =
            fs::read(&manifest_path).context("Failed to read cache manifest")?;
        let manifest: CacheManifest = match serde_json::from_slice(&manifest_bytes) {
            Ok(parsed) => parsed,
            Err(err) => {
                warn!("Failed to parse cache manifest: {}", err);
                return Ok(None);
            }
        };

        if manifest.schema_version != self.schema_version
            || manifest.source_kind != key.source_kind
            || manifest.source_identity != key.source_identity
            || manifest.source_fingerprint != key.source_fingerprint
            || manifest.settings_fingerprint != key.settings_fingerprint
        {
            debug!("Cache manifest mismatch, skipping");
            return Ok(None);
        }

        let artifact_bytes =
            fs::read(&artifact_path).context("Failed to read cache artifact")?;
        let payload = match zstd::stream::decode_all(&artifact_bytes[..]) {
            Ok(bytes) => bytes,
            Err(_) => artifact_bytes,
        };
        let value: T = match serde_json::from_slice(&payload) {
            Ok(parsed) => parsed,
            Err(err) => {
                warn!("Failed to parse cache artifact: {}", err);
                return Ok(None);
            }
        };

        Ok(Some(value))
    }

    fn store<T>(
        &self,
        key: &DiskCacheKey,
        cache_dir: &Path,
        value: &T,
        build_time_ms: u64,
    ) -> Result<()>
    where
        T: Serialize,
    {
        let artifact_path = cache_dir.join("artifact.json");
        let manifest_path = cache_dir.join("manifest.json");

        let artifact_json = serde_json::to_vec(value).context("Failed to serialize artifact")?;
        let compressed = zstd::stream::encode_all(std::io::Cursor::new(&artifact_json), 0)
            .unwrap_or(artifact_json);
        let size_bytes = compressed.len() as u64;

        write_atomic(&artifact_path, &compressed)?;

        let manifest = CacheManifest {
            schema_version: self.schema_version,
            source_kind: key.source_kind.clone(),
            source_identity: key.source_identity.clone(),
            source_fingerprint: key.source_fingerprint.clone(),
            settings_fingerprint: key.settings_fingerprint.clone(),
            created_at: current_timestamp(),
            size_bytes,
            build_time_ms,
        };
        let manifest_bytes =
            serde_json::to_vec(&manifest).context("Failed to serialize manifest")?;
        write_atomic(&manifest_path, &manifest_bytes)?;

        Ok(())
    }
}

struct DiskCacheLock {
    _file: File,
}

fn resolve_cache_root() -> PathBuf {
    if let Ok(dir) = std::env::var("BSL_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(dir).join("bsl-gradual-types");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("bsl-gradual-types");
    }
    PathBuf::from(".bsl_cache")
}

fn is_cache_disabled() -> bool {
    matches!(
        std::env::var("BSL_CACHE_DISABLE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    )
}

fn sanitize_component(value: &str) -> String {
    value
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
        .chars()
        .take(120)
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestValue {
        value: String,
    }

    fn test_key() -> DiskCacheKey {
        DiskCacheKey::new("test", "key", "identity", "fingerprint", "settings")
            .with_project_id("project")
            .with_config_id("config")
    }

    #[test]
    fn test_disk_cache_roundtrip() {
        let temp = TempDir::new().unwrap();
        let cache = DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();
        let key = test_key();

        let entry = cache
            .get_or_build(&key, || Ok(TestValue { value: "a".into() }))
            .unwrap();
        assert!(!entry.from_cache);

        let entry = cache
            .get_or_build(&key, || Ok(TestValue { value: "b".into() }))
            .unwrap();
        assert!(entry.from_cache);
        assert_eq!(entry.value.value, "a");
    }

    #[test]
    fn test_disk_cache_schema_mismatch() {
        let temp = TempDir::new().unwrap();
        let key = test_key();
        let builds = Arc::new(AtomicUsize::new(0));

        let cache_v1 = DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();
        let counter = builds.clone();
        cache_v1
            .get_or_build(&key, move || {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(TestValue { value: "v1".into() })
            })
            .unwrap();

        let cache_v2 = DiskCache::with_root(temp.path().to_path_buf(), 2).unwrap();
        let counter = builds.clone();
        cache_v2
            .get_or_build(&key, move || {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(TestValue { value: "v2".into() })
            })
            .unwrap();

        assert_eq!(builds.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_disk_cache_lock_single_builder() {
        let temp = TempDir::new().unwrap();
        let cache = Arc::new(DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap());
        let key = Arc::new(test_key());
        let builds = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(2));

        let mut handles = Vec::new();
        for _ in 0..2 {
            let cache = cache.clone();
            let key = key.clone();
            let builds = builds.clone();
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                cache
                    .get_or_build(&key, move || {
                        builds.fetch_add(1, Ordering::SeqCst);
                        thread::sleep(Duration::from_millis(50));
                        Ok(TestValue {
                            value: "value".into(),
                        })
                    })
                    .unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_disk_cache_skip_store() {
        let temp = TempDir::new().unwrap();
        let cache = DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();
        let key = test_key();

        let entry = cache
            .get_or_build_with(&key, || Ok(TestValue { value: "a".into() }), |_| false)
            .unwrap();
        assert!(!entry.from_cache);

        let entry = cache
            .get_or_build_with(&key, || Ok(TestValue { value: "b".into() }), |_| false)
            .unwrap();
        assert!(!entry.from_cache);
        assert_eq!(entry.value.value, "b");
    }
}
