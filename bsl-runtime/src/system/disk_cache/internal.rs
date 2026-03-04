use super::*;

pub(super) struct DiskCacheLock {
    pub(super) _file: File,
}

#[derive(Debug)]
pub(super) struct CacheLoadResult<T> {
    pub(super) value: Option<T>,
    pub(super) is_expired: bool,
}

impl<T> CacheLoadResult<T> {
    pub(super) fn empty() -> Self {
        Self {
            value: None,
            is_expired: false,
        }
    }
}

#[derive(Debug)]
pub(super) struct CacheEntryInfo {
    pub(super) path: PathBuf,
    pub(super) size_bytes: u64,
    pub(super) last_used_at: u64,
}

pub(super) fn resolve_cache_root() -> PathBuf {
    if let Some(dir) = crate::system::runtime_config::global_runtime_config()
        .get_pathbuf(crate::system::runtime_config::RuntimeKey::CacheDir)
    {
        return dir;
    }
    if let Ok(dir) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(dir).join("bsl-gradual-types");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("bsl-gradual-types");
    }
    PathBuf::from(".bsl_cache")
}

pub(super) fn is_cache_disabled() -> bool {
    crate::system::runtime_config::global_runtime_config()
        .get_bool(crate::system::runtime_config::RuntimeKey::CacheDisable)
        .unwrap_or(false)
}

pub(super) fn cache_ttl_secs() -> Option<u64> {
    crate::system::runtime_config::global_runtime_config()
        .get_u64(crate::system::runtime_config::RuntimeKey::CacheTtlSecs)
}

pub(super) fn cache_max_bytes() -> Option<u64> {
    crate::system::runtime_config::global_runtime_config()
        .get_u64(crate::system::runtime_config::RuntimeKey::CacheMaxBytes)
}

pub(super) fn cache_cleanup_interval_secs() -> Option<u64> {
    crate::system::runtime_config::global_runtime_config()
        .get_u64(crate::system::runtime_config::RuntimeKey::CacheCleanupIntervalSecs)
}

pub(super) fn cache_touch_interval_secs() -> u64 {
    crate::system::runtime_config::global_runtime_config()
        .get_u64(crate::system::runtime_config::RuntimeKey::CacheTouchIntervalSecs)
        .unwrap_or(60)
}

pub(super) fn cache_ttl_mode() -> String {
    crate::system::runtime_config::global_runtime_config()
        .get_string(crate::system::runtime_config::RuntimeKey::CacheTtlMode)
        .unwrap_or_else(|| "created".to_string())
}

pub(super) fn cache_swr_enabled() -> bool {
    crate::system::runtime_config::global_runtime_config()
        .get_bool(crate::system::runtime_config::RuntimeKey::CacheSwr)
        .unwrap_or(true)
}

pub(super) fn cache_cleanup_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn sanitize_component(value: &str) -> String {
    value
        .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
        .chars()
        .take(120)
        .collect()
}

pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
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

pub(super) fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(super) fn collect_manifest_paths(root: &Path) -> Vec<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut manifests = Vec::new();

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|n| n.to_str()) == Some("manifest.json") {
                manifests.push(path);
            }
        }
    }

    manifests
}
