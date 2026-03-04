use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
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

struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        crate::system::runtime_config::global_runtime_config().reload_env_bootstrap_from_env();
        Self { key, prev }
    }

    fn remove(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::remove_var(key);
        crate::system::runtime_config::global_runtime_config().reload_env_bootstrap_from_env();
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(prev) = &self.prev {
            std::env::set_var(self.key, prev);
        } else {
            std::env::remove_var(self.key);
        }
        crate::system::runtime_config::global_runtime_config().reload_env_bootstrap_from_env();
    }
}

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn test_disk_cache_roundtrip() {
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();
    let _disable = EnvGuard::remove("BSL_CACHE_DISABLE");
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
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();
    let _disable = EnvGuard::remove("BSL_CACHE_DISABLE");
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
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();
    let _disable = EnvGuard::remove("BSL_CACHE_DISABLE");
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
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();
    let _disable = EnvGuard::remove("BSL_CACHE_DISABLE");
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

#[test]
fn test_disk_cache_ttl_expired() {
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();
    let _ttl = EnvGuard::set("BSL_CACHE_TTL_SECS", "1");
    let _swr = EnvGuard::set("BSL_CACHE_SWR", "0");
    let _disable = EnvGuard::remove("BSL_CACHE_DISABLE");
    let cache = DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();
    let key = test_key();

    let entry = cache
        .get_or_build(&key, || {
            Ok(TestValue {
                value: "old".into(),
            })
        })
        .unwrap();
    assert!(!entry.from_cache);

    let manifest_path = cache.cache_dir(&key).join("manifest.json");
    let manifest_bytes = fs::read(&manifest_path).unwrap();
    let mut manifest: CacheManifest = serde_json::from_slice(&manifest_bytes).unwrap();
    manifest.created_at = 0;
    manifest.last_used_at = 0;
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    write_atomic(&manifest_path, &manifest_bytes).unwrap();

    let entry = cache
        .get_or_build(&key, || {
            Ok(TestValue {
                value: "new".into(),
            })
        })
        .unwrap();
    assert!(!entry.from_cache);
    assert_eq!(entry.value.value, "new");
}

#[test]
fn test_disk_cache_swr_rebuild() {
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();
    let _ttl = EnvGuard::set("BSL_CACHE_TTL_SECS", "1");
    let _swr = EnvGuard::set("BSL_CACHE_SWR", "1");
    let _disable = EnvGuard::remove("BSL_CACHE_DISABLE");
    let cache = DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();
    let key = test_key();

    let entry = cache
        .get_or_build(&key, || {
            Ok(TestValue {
                value: "old".into(),
            })
        })
        .unwrap();
    assert!(!entry.from_cache);

    let manifest_path = cache.cache_dir(&key).join("manifest.json");
    let manifest_bytes = fs::read(&manifest_path).unwrap();
    let mut manifest: CacheManifest = serde_json::from_slice(&manifest_bytes).unwrap();
    manifest.created_at = 0;
    manifest.last_used_at = 0;
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    write_atomic(&manifest_path, &manifest_bytes).unwrap();

    let entry = cache
        .get_or_build_with_swr(
            &key,
            || {
                Ok(TestValue {
                    value: "new".into(),
                })
            },
            |_| true,
        )
        .unwrap();
    assert!(entry.from_cache);
    assert_eq!(entry.value.value, "old");

    let started = Instant::now();
    loop {
        if let Some(value) = read_cached_value::<TestValue>(&cache, &key) {
            if value.value == "new" {
                break;
            }
        }
        if started.elapsed() > Duration::from_secs(3) {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    let value = read_cached_value::<TestValue>(&cache, &key);
    assert!(value.is_some(), "Ожидали обновлённый кэш");
    assert_eq!(value.unwrap().value, "new");
}

#[test]
fn test_disk_cache_cleanup_by_size() {
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();
    let _interval = EnvGuard::set("BSL_CACHE_CLEANUP_INTERVAL_SECS", "1");
    let _disable = EnvGuard::remove("BSL_CACHE_DISABLE");
    let cache = DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();

    let key1 = DiskCacheKey::new("test", "key1", "id1", "fp1", "settings");
    let key2 = DiskCacheKey::new("test", "key2", "id2", "fp2", "settings");

    let entry = cache
        .get_or_build(&key1, || Ok(TestValue { value: "a".into() }))
        .unwrap();
    assert!(!entry.from_cache);

    let manifest_path = cache.cache_dir(&key1).join("manifest.json");
    let manifest_bytes = fs::read(&manifest_path).unwrap();
    let manifest: CacheManifest = serde_json::from_slice(&manifest_bytes).unwrap();

    let _max = EnvGuard::set("BSL_CACHE_MAX_BYTES", &manifest.size_bytes.to_string());
    thread::sleep(Duration::from_secs(1));

    let entry = cache
        .get_or_build(&key2, || Ok(TestValue { value: "b".into() }))
        .unwrap();
    assert!(!entry.from_cache);

    let first = cache.try_get::<TestValue>(&key1).unwrap();
    let second = cache.try_get::<TestValue>(&key2).unwrap();
    assert!(first.is_none(), "Ожидали вытеснение первого entry");
    assert!(second.is_some(), "Ожидали сохранение второго entry");
}

#[test]
fn test_disk_cache_cleanup_skips_locked_entries() {
    let _guard = env_lock();
    let temp = TempDir::new().unwrap();
    let _ttl = EnvGuard::set("BSL_CACHE_TTL_SECS", "1");
    let _disable = EnvGuard::remove("BSL_CACHE_DISABLE");
    let cache = DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();
    let key = test_key();

    let entry = cache
        .get_or_build(&key, || {
            Ok(TestValue {
                value: "old".into(),
            })
        })
        .unwrap();
    assert!(!entry.from_cache);

    let cache_dir = cache.cache_dir(&key);
    let manifest_path = cache_dir.join("manifest.json");
    let artifact_path = cache_dir.join("artifact.json");

    let manifest_bytes = fs::read(&manifest_path).unwrap();
    let mut manifest: CacheManifest = serde_json::from_slice(&manifest_bytes).unwrap();
    manifest.created_at = 0;
    manifest.last_used_at = 0;
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    write_atomic(&manifest_path, &manifest_bytes).unwrap();

    let lock_path = cache_dir.join(".lock");
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .unwrap();
    lock_file.lock_exclusive().unwrap();

    let report = cache.cleanup().unwrap();
    assert_eq!(report.removed_entries, 0);
    assert!(
        manifest_path.exists(),
        "manifest should remain while locked"
    );
    assert!(
        artifact_path.exists(),
        "artifact should remain while locked"
    );

    drop(lock_file);

    let report = cache.cleanup().unwrap();
    assert_eq!(report.removed_entries, 1);
    assert!(!manifest_path.exists(), "manifest should be purged");
    assert!(!artifact_path.exists(), "artifact should be purged");
}

fn read_cached_value<T>(cache: &DiskCache, key: &DiskCacheKey) -> Option<T>
where
    T: super::DeserializeOwned,
{
    let cache_dir = cache.cache_dir(key);
    let manifest_path = cache_dir.join("manifest.json");
    let artifact_path = cache_dir.join("artifact.json");
    if !manifest_path.exists() || !artifact_path.exists() {
        return None;
    }

    let manifest_bytes = fs::read(&manifest_path).ok()?;
    let manifest: CacheManifest = serde_json::from_slice(&manifest_bytes).ok()?;
    if manifest.schema_version != cache.schema_version
        || manifest.source_kind != key.source_kind
        || manifest.source_identity != key.source_identity
        || manifest.source_fingerprint != key.source_fingerprint
        || manifest.settings_fingerprint != key.settings_fingerprint
    {
        return None;
    }

    let artifact_bytes = fs::read(&artifact_path).ok()?;
    let payload = zstd::stream::decode_all(&artifact_bytes[..]).unwrap_or(artifact_bytes);
    serde_json::from_slice(&payload).ok()
}
