//! Disk cache with manifest and file locking for cross-process reuse.

use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
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
    #[serde(default)]
    pub last_used_at: u64,
    pub size_bytes: u64,
    pub build_time_ms: u64,
    #[serde(default)]
    pub load_time_ms: u64,
    #[serde(default)]
    pub hit_count: u64,
}

pub struct DiskCache {
    root: PathBuf,
    schema_version: u32,
    disabled: Arc<AtomicBool>,
    stats: Arc<DiskCacheStats>,
    last_cleanup_at: Arc<AtomicU64>,
}

#[derive(Debug, Default)]
struct DiskCacheStats {
    hit_count: AtomicU64,
    miss_count: AtomicU64,
    stale_hit_count: AtomicU64,
    load_time_ms_total: AtomicU64,
    build_time_ms_total: AtomicU64,
    stored_entries: AtomicU64,
    expired_entries: AtomicU64,
    evicted_entries: AtomicU64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskCacheStatsSnapshot {
    pub hit_count: u64,
    pub miss_count: u64,
    pub stale_hit_count: u64,
    pub load_time_ms_total: u64,
    pub build_time_ms_total: u64,
    pub stored_entries: u64,
    pub expired_entries: u64,
    pub evicted_entries: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheCleanupReport {
    pub removed_entries: u64,
    pub freed_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DiskCacheScopeStats {
    pub entries: u64,
    pub size_bytes: u64,
}

impl DiskCache {
    pub fn new(schema_version: u32) -> Result<Self> {
        let root = resolve_cache_root();
        Self::with_root(root, schema_version)
    }

    pub fn with_root(root: PathBuf, schema_version: u32) -> Result<Self> {
        let disabled_at_start = is_cache_disabled();
        if !disabled_at_start {
            fs::create_dir_all(&root).context("Failed to create disk cache root")?;
        }
        Ok(Self {
            root,
            schema_version,
            disabled: Arc::new(AtomicBool::new(disabled_at_start)),
            stats: Arc::new(DiskCacheStats::default()),
            last_cleanup_at: Arc::new(AtomicU64::new(0)),
        })
    }

    pub fn disabled(schema_version: u32) -> Self {
        Self {
            root: PathBuf::from(".bsl_cache"),
            schema_version,
            disabled: Arc::new(AtomicBool::new(true)),
            stats: Arc::new(DiskCacheStats::default()),
            last_cleanup_at: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn env_disabled(&self) -> bool {
        is_cache_disabled()
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled.load(Ordering::Relaxed)
    }

    pub fn is_enabled(&self) -> bool {
        !self.is_disabled()
    }

    pub fn set_enabled(&self, enabled: bool) -> bool {
        if is_cache_disabled() {
            self.disabled.store(true, Ordering::Relaxed);
            return false;
        }

        if enabled {
            if let Err(err) = fs::create_dir_all(&self.root) {
                warn!("Failed to create disk cache root: {}", err);
                self.disabled.store(true, Ordering::Relaxed);
                return false;
            }
        }

        self.disabled.store(!enabled, Ordering::Relaxed);
        enabled
    }

    pub fn swr_enabled(&self) -> bool {
        cache_swr_enabled()
    }

    pub fn root_path(&self) -> &Path {
        &self.root
    }

    pub fn stats(&self) -> DiskCacheStatsSnapshot {
        DiskCacheStatsSnapshot {
            hit_count: self.stats.hit_count.load(Ordering::Relaxed),
            miss_count: self.stats.miss_count.load(Ordering::Relaxed),
            stale_hit_count: self.stats.stale_hit_count.load(Ordering::Relaxed),
            load_time_ms_total: self.stats.load_time_ms_total.load(Ordering::Relaxed),
            build_time_ms_total: self.stats.build_time_ms_total.load(Ordering::Relaxed),
            stored_entries: self.stats.stored_entries.load(Ordering::Relaxed),
            expired_entries: self.stats.expired_entries.load(Ordering::Relaxed),
            evicted_entries: self.stats.evicted_entries.load(Ordering::Relaxed),
        }
    }

    pub fn scope_usage(
        &self,
        project_id: Option<&str>,
        config_id: Option<&str>,
    ) -> Result<DiskCacheScopeStats> {
        if self.is_disabled() {
            return Ok(DiskCacheScopeStats::default());
        }

        let mut stats = DiskCacheScopeStats::default();
        for scope_root in self.collect_scope_roots(project_id, config_id)? {
            let scope_stats = self.collect_scope_stats(&scope_root);
            stats.entries = stats.entries.saturating_add(scope_stats.entries);
            stats.size_bytes = stats.size_bytes.saturating_add(scope_stats.size_bytes);
        }
        Ok(stats)
    }

    pub fn scope_usage_for_ids(
        &self,
        project_id: &str,
        config_ids: &[String],
    ) -> Result<DiskCacheScopeStats> {
        if self.is_disabled() || config_ids.is_empty() {
            return Ok(DiskCacheScopeStats::default());
        }

        let mut stats = DiskCacheScopeStats::default();
        for scope_root in self.collect_scope_roots_for_ids(project_id, config_ids)? {
            let scope_stats = self.collect_scope_stats(&scope_root);
            stats.entries = stats.entries.saturating_add(scope_stats.entries);
            stats.size_bytes = stats.size_bytes.saturating_add(scope_stats.size_bytes);
        }
        Ok(stats)
    }

    pub fn clear_scope(&self, project_id: &str, config_id: &str) -> Result<CacheCleanupReport> {
        if self.is_disabled() {
            return Ok(CacheCleanupReport {
                removed_entries: 0,
                freed_bytes: 0,
            });
        }

        self.clear_scope_for_ids(project_id, &[config_id.to_string()])
    }

    pub fn clear_scope_for_ids(
        &self,
        project_id: &str,
        config_ids: &[String],
    ) -> Result<CacheCleanupReport> {
        if self.is_disabled() || config_ids.is_empty() {
            return Ok(CacheCleanupReport {
                removed_entries: 0,
                freed_bytes: 0,
            });
        }

        let mut removed_entries = 0u64;
        let mut freed_bytes = 0u64;
        for scope_root in self.collect_scope_roots_for_ids(project_id, config_ids)? {
            for manifest_path in collect_manifest_paths(&scope_root) {
                let cache_dir = match manifest_path.parent() {
                    Some(dir) => dir.to_path_buf(),
                    None => continue,
                };

                let manifest_bytes = match fs::read(&manifest_path) {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        let _ = self.try_purge_entry_dir(&cache_dir, None);
                        continue;
                    }
                };
                let manifest: CacheManifest = match serde_json::from_slice(&manifest_bytes) {
                    Ok(parsed) => parsed,
                    Err(_) => {
                        let _ = self.try_purge_entry_dir(&cache_dir, None);
                        continue;
                    }
                };

                if self.try_purge_entry_dir(&cache_dir, Some(&manifest))? {
                    removed_entries = removed_entries.saturating_add(1);
                    freed_bytes = freed_bytes.saturating_add(manifest.size_bytes);
                }
            }
        }

        Ok(CacheCleanupReport {
            removed_entries,
            freed_bytes,
        })
    }

    pub fn get_or_build<T, F>(&self, key: &DiskCacheKey, build: F) -> Result<CacheEntry<T>>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Result<T>,
    {
        self.get_or_build_with(key, build, |_| true)
    }

    pub fn try_get<T>(&self, key: &DiskCacheKey) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        if self.is_disabled() {
            return Ok(None);
        }

        let cache_dir = self.cache_dir(key);
        if !cache_dir.exists() {
            return Ok(None);
        }

        let _lock = self.lock_key(&cache_dir)?;
        match self.try_load::<T>(key, &cache_dir, false) {
            Ok(load) => {
                if load.value.is_some() {
                    self.stats.hit_count.fetch_add(1, Ordering::Relaxed);
                } else {
                    self.stats.miss_count.fetch_add(1, Ordering::Relaxed);
                }
                Ok(load.value)
            }
            Err(err) => {
                warn!("Disk cache read failed: {}", err);
                self.stats.miss_count.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
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
        if self.is_disabled() {
            let value = build()?;
            return Ok(CacheEntry {
                value,
                from_cache: false,
            });
        }

        let cache_dir = self.cache_dir(key);
        fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

        let _lock = self.lock_key(&cache_dir)?;

        match self.try_load::<T>(key, &cache_dir, false) {
            Ok(load) => {
                if let Some(value) = load.value {
                    self.stats.hit_count.fetch_add(1, Ordering::Relaxed);
                    return Ok(CacheEntry {
                        value,
                        from_cache: true,
                    });
                }
                self.stats.miss_count.fetch_add(1, Ordering::Relaxed);
            }
            Err(err) => {
                warn!("Disk cache read failed: {}", err);
                self.stats.miss_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        let started = Instant::now();
        let value = build()?;
        let build_time_ms = started.elapsed().as_millis() as u64;
        self.stats
            .build_time_ms_total
            .fetch_add(build_time_ms, Ordering::Relaxed);

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

    pub fn get_or_build_with_swr<T, F, P>(
        &self,
        key: &DiskCacheKey,
        build: F,
        should_cache: P,
    ) -> Result<CacheEntry<T>>
    where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
        P: Fn(&T) -> bool + Send + Sync + 'static,
    {
        if self.is_disabled() {
            let value = build()?;
            return Ok(CacheEntry {
                value,
                from_cache: false,
            });
        }

        let should_cache = Arc::new(should_cache);
        let mut build = Some(build);
        let cache_dir = self.cache_dir(key);
        fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;
        let allow_stale = cache_swr_enabled();
        let mut stale_value = None;
        let mut should_rebuild = false;

        {
            let _lock = self.lock_key(&cache_dir)?;
            match self.try_load::<T>(key, &cache_dir, allow_stale) {
                Ok(load) => {
                    if let Some(value) = load.value {
                        if load.is_expired && allow_stale {
                            stale_value = Some(value);
                            should_rebuild = true;
                        } else {
                            self.stats.hit_count.fetch_add(1, Ordering::Relaxed);
                            return Ok(CacheEntry {
                                value,
                                from_cache: true,
                            });
                        }
                    } else {
                        self.stats.miss_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(err) => {
                    warn!("Disk cache read failed: {}", err);
                    self.stats.miss_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        if let Some(value) = stale_value {
            self.stats.stale_hit_count.fetch_add(1, Ordering::Relaxed);
            if should_rebuild {
                if let Some(build) = build.take() {
                    self.spawn_rebuild(
                        key.clone(),
                        cache_dir.clone(),
                        build,
                        Arc::clone(&should_cache),
                    );
                }
            }
            return Ok(CacheEntry {
                value,
                from_cache: true,
            });
        }

        let started = Instant::now();
        let value = build.take().expect("build closure must be available")()?;
        let build_time_ms = started.elapsed().as_millis() as u64;
        self.stats
            .build_time_ms_total
            .fetch_add(build_time_ms, Ordering::Relaxed);

        if (should_cache)(&value) {
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
        let project_id = key.project_id.as_deref().unwrap_or("global").to_string();
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
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("Failed to open lock file {}", lock_path.display()))?;
        file.lock_exclusive()
            .context("Failed to acquire cache lock")?;
        Ok(DiskCacheLock { _file: file })
    }

    fn try_lock_key(&self, cache_dir: &Path) -> Result<Option<DiskCacheLock>> {
        let lock_path = cache_dir.join(".lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("Failed to open lock file {}", lock_path.display()))?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(DiskCacheLock { _file: file })),
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(err) => Err(err).context("Failed to acquire cache lock")?,
        }
    }

    fn purge_entry_files(cache_dir: &Path) {
        let entries = match fs::read_dir(cache_dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if file_name == ".lock" {
                continue;
            }
            if path.is_dir() {
                let _ = fs::remove_dir_all(&path);
            } else {
                let _ = fs::remove_file(&path);
            }
        }
    }

    fn try_purge_entry_dir(
        &self,
        cache_dir: &Path,
        manifest: Option<&CacheManifest>,
    ) -> Result<bool> {
        if !cache_dir.exists() {
            return Ok(false);
        }
        let Some(_lock) = self.try_lock_key(cache_dir)? else {
            debug!(
                "Skip cache purge for {}: entry is locked",
                cache_dir.display()
            );
            return Ok(false);
        };

        Self::purge_entry_files(cache_dir);
        if let Some(manifest) = manifest {
            debug!(
                "Purged cache entry {} ({} bytes)",
                cache_dir.display(),
                manifest.size_bytes
            );
        } else {
            debug!("Purged cache entry {}", cache_dir.display());
        }
        Ok(true)
    }

    fn spawn_rebuild<T, F, P>(
        &self,
        key: DiskCacheKey,
        cache_dir: PathBuf,
        build: F,
        should_cache: Arc<P>,
    ) where
        T: Serialize + DeserializeOwned + Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
        P: Fn(&T) -> bool + Send + Sync + 'static,
    {
        let cache = self.clone_for_thread();
        std::thread::spawn(move || {
            let started = Instant::now();
            let value = match build() {
                Ok(value) => value,
                Err(err) => {
                    warn!("Disk cache SWR build failed: {}", err);
                    return;
                }
            };
            let build_time_ms = started.elapsed().as_millis() as u64;
            cache
                .stats
                .build_time_ms_total
                .fetch_add(build_time_ms, Ordering::Relaxed);

            if (should_cache)(&value) {
                if let Err(err) = cache.store(&key, &cache_dir, &value, build_time_ms) {
                    warn!("Disk cache SWR store failed: {}", err);
                }
            }
        });
    }

    fn update_manifest(&self, cache_dir: &Path, manifest: &CacheManifest) -> Result<()> {
        let manifest_path = cache_dir.join("manifest.json");
        let bytes = serde_json::to_vec(manifest).context("Failed to serialize manifest")?;
        write_atomic(&manifest_path, &bytes)?;
        Ok(())
    }

    fn cleanup_if_needed(&self) -> Result<()> {
        if self.is_disabled() {
            return Ok(());
        }
        let interval_secs = cache_cleanup_interval_secs();
        if let Some(interval) = interval_secs {
            let now = current_timestamp();
            let last = self.last_cleanup_at.load(Ordering::Relaxed);
            if last != 0 && now.saturating_sub(last) < interval {
                return Ok(());
            }
            self.last_cleanup_at.store(now, Ordering::Relaxed);
        }

        let _ = self.cleanup();
        Ok(())
    }

    pub fn cleanup(&self) -> Result<CacheCleanupReport> {
        if self.is_disabled() {
            return Ok(CacheCleanupReport {
                removed_entries: 0,
                freed_bytes: 0,
            });
        }

        let _lock = cache_cleanup_lock();
        let ttl_secs = cache_ttl_secs();
        let max_bytes = cache_max_bytes();
        let now = current_timestamp();
        let mut entries = Vec::new();
        let mut total_size = 0u64;
        let mut removed = 0u64;
        let mut freed = 0u64;

        let version_root = self.root.join(format!("v{}", self.schema_version));
        for manifest_path in collect_manifest_paths(&version_root) {
            let cache_dir = match manifest_path.parent() {
                Some(dir) => dir.to_path_buf(),
                None => continue,
            };
            let manifest_bytes = match fs::read(&manifest_path) {
                Ok(bytes) => bytes,
                Err(_) => {
                    let _ = self.try_purge_entry_dir(&cache_dir, None);
                    continue;
                }
            };
            let manifest: CacheManifest = match serde_json::from_slice(&manifest_bytes) {
                Ok(parsed) => parsed,
                Err(_) => {
                    let _ = self.try_purge_entry_dir(&cache_dir, None);
                    continue;
                }
            };

            let last_used_at = if manifest.last_used_at == 0 {
                manifest.created_at
            } else {
                manifest.last_used_at
            };

            let expired = ttl_secs
                .map(|ttl| {
                    let base_ts = match cache_ttl_mode().as_str() {
                        "idle" => last_used_at,
                        _ => manifest.created_at,
                    };
                    now >= base_ts.saturating_add(ttl)
                })
                .unwrap_or(false);

            if expired {
                if self.try_purge_entry_dir(&cache_dir, Some(&manifest))? {
                    removed += 1;
                    freed += manifest.size_bytes;
                    self.stats.expired_entries.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                total_size = total_size.saturating_add(manifest.size_bytes);
                entries.push(CacheEntryInfo {
                    path: cache_dir,
                    size_bytes: manifest.size_bytes,
                    last_used_at: 0,
                });
                continue;
            }

            total_size = total_size.saturating_add(manifest.size_bytes);
            entries.push(CacheEntryInfo {
                path: cache_dir,
                size_bytes: manifest.size_bytes,
                last_used_at,
            });
        }

        if let Some(limit) = max_bytes {
            if total_size > limit {
                entries.sort_by_key(|entry| entry.last_used_at);
                for entry in entries {
                    if total_size <= limit {
                        break;
                    }
                    if self.try_purge_entry_dir(&entry.path, None)? {
                        total_size = total_size.saturating_sub(entry.size_bytes);
                        removed += 1;
                        freed += entry.size_bytes;
                        self.stats.evicted_entries.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }

        Ok(CacheCleanupReport {
            removed_entries: removed,
            freed_bytes: freed,
        })
    }

    fn clone_for_thread(&self) -> Self {
        Self {
            root: self.root.clone(),
            schema_version: self.schema_version,
            disabled: Arc::clone(&self.disabled),
            stats: Arc::clone(&self.stats),
            last_cleanup_at: Arc::clone(&self.last_cleanup_at),
        }
    }

    fn collect_scope_roots(
        &self,
        project_id: Option<&str>,
        config_id: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        let version_root = self.root.join(format!("v{}", self.schema_version));
        if project_id.is_none() && config_id.is_none() {
            return Ok(vec![version_root]);
        }

        let project = sanitize_component(project_id.unwrap_or("global"));
        let config = sanitize_component(config_id.unwrap_or("none"));
        let mut roots = Vec::new();
        let entries = match fs::read_dir(&version_root) {
            Ok(entries) => entries,
            Err(_) => return Ok(roots),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let scope_root = path.join(&project).join(&config);
            if scope_root.exists() {
                roots.push(scope_root);
            }
        }

        Ok(roots)
    }

    fn collect_scope_roots_for_ids(
        &self,
        project_id: &str,
        config_ids: &[String],
    ) -> Result<Vec<PathBuf>> {
        use std::collections::HashSet;

        let version_root = self.root.join(format!("v{}", self.schema_version));
        let entries = match fs::read_dir(&version_root) {
            Ok(entries) => entries,
            Err(_) => return Ok(Vec::new()),
        };

        let project = sanitize_component(project_id);
        let mut roots = HashSet::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            for config_id in config_ids {
                let config = sanitize_component(config_id);
                let scope_root = path.join(&project).join(&config);
                if scope_root.exists() {
                    roots.insert(scope_root);
                }
            }
        }

        Ok(roots.into_iter().collect())
    }

    fn collect_scope_stats(&self, root: &Path) -> DiskCacheScopeStats {
        if !root.exists() {
            return DiskCacheScopeStats::default();
        }

        let mut stats = DiskCacheScopeStats::default();
        for manifest_path in collect_manifest_paths(root) {
            let manifest_bytes = match fs::read(&manifest_path) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            let manifest: CacheManifest = match serde_json::from_slice(&manifest_bytes) {
                Ok(parsed) => parsed,
                Err(_) => continue,
            };
            stats.entries = stats.entries.saturating_add(1);
            stats.size_bytes = stats.size_bytes.saturating_add(manifest.size_bytes);
        }

        stats
    }

    fn try_load<T>(
        &self,
        key: &DiskCacheKey,
        cache_dir: &Path,
        allow_stale: bool,
    ) -> Result<CacheLoadResult<T>>
    where
        T: DeserializeOwned,
    {
        let manifest_path = cache_dir.join("manifest.json");
        let artifact_path = cache_dir.join("artifact.json");

        if !manifest_path.exists() || !artifact_path.exists() {
            return Ok(CacheLoadResult::empty());
        }

        let manifest_bytes = fs::read(&manifest_path).context("Failed to read cache manifest")?;
        let manifest: CacheManifest = match serde_json::from_slice(&manifest_bytes) {
            Ok(parsed) => parsed,
            Err(err) => {
                warn!("Failed to parse cache manifest: {}", err);
                return Ok(CacheLoadResult::empty());
            }
        };

        if manifest.schema_version != self.schema_version
            || manifest.source_kind != key.source_kind
            || manifest.source_identity != key.source_identity
            || manifest.source_fingerprint != key.source_fingerprint
            || manifest.settings_fingerprint != key.settings_fingerprint
        {
            debug!("Cache manifest mismatch, skipping");
            return Ok(CacheLoadResult::empty());
        }

        let ttl_secs = cache_ttl_secs();
        let mut is_expired = false;
        if let Some(ttl) = ttl_secs {
            let base_ts = match cache_ttl_mode().as_str() {
                "idle" => {
                    if manifest.last_used_at == 0 {
                        manifest.created_at
                    } else {
                        manifest.last_used_at
                    }
                }
                _ => manifest.created_at,
            };
            let expires_at = base_ts.saturating_add(ttl);
            if current_timestamp() >= expires_at {
                is_expired = true;
                if !allow_stale {
                    self.stats.expired_entries.fetch_add(1, Ordering::Relaxed);
                    Self::purge_entry_files(cache_dir);
                    return Ok(CacheLoadResult::empty());
                }
            }
        }

        let load_started = Instant::now();
        let artifact_bytes = fs::read(&artifact_path).context("Failed to read cache artifact")?;
        let payload = match zstd::stream::decode_all(&artifact_bytes[..]) {
            Ok(bytes) => bytes,
            Err(_) => artifact_bytes,
        };
        let value: T = match serde_json::from_slice(&payload) {
            Ok(parsed) => parsed,
            Err(err) => {
                warn!("Failed to parse cache artifact: {}", err);
                return Ok(CacheLoadResult::empty());
            }
        };

        let load_time_ms = load_started.elapsed().as_millis() as u64;
        self.stats
            .load_time_ms_total
            .fetch_add(load_time_ms, Ordering::Relaxed);

        let touch_interval = cache_touch_interval_secs();
        let last_used_at = if manifest.last_used_at == 0 {
            manifest.created_at
        } else {
            manifest.last_used_at
        };
        if current_timestamp().saturating_sub(last_used_at) >= touch_interval {
            let updated = CacheManifest {
                last_used_at: current_timestamp(),
                hit_count: manifest.hit_count.saturating_add(1),
                load_time_ms,
                ..manifest
            };
            let _ = self.update_manifest(cache_dir, &updated);
        }

        Ok(CacheLoadResult {
            value: Some(value),
            is_expired,
        })
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
            last_used_at: current_timestamp(),
            size_bytes,
            build_time_ms,
            load_time_ms: 0,
            hit_count: 0,
        };
        let manifest_bytes =
            serde_json::to_vec(&manifest).context("Failed to serialize manifest")?;
        write_atomic(&manifest_path, &manifest_bytes)?;

        self.stats.stored_entries.fetch_add(1, Ordering::Relaxed);
        let _ = self.cleanup_if_needed();

        Ok(())
    }
}

struct DiskCacheLock {
    _file: File,
}

#[derive(Debug)]
struct CacheLoadResult<T> {
    value: Option<T>,
    is_expired: bool,
}

impl<T> CacheLoadResult<T> {
    fn empty() -> Self {
        Self {
            value: None,
            is_expired: false,
        }
    }
}

#[derive(Debug)]
struct CacheEntryInfo {
    path: PathBuf,
    size_bytes: u64,
    last_used_at: u64,
}

fn resolve_cache_root() -> PathBuf {
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

fn is_cache_disabled() -> bool {
    crate::system::runtime_config::global_runtime_config()
        .get_bool(crate::system::runtime_config::RuntimeKey::CacheDisable)
        .unwrap_or(false)
}

fn cache_ttl_secs() -> Option<u64> {
    crate::system::runtime_config::global_runtime_config()
        .get_u64(crate::system::runtime_config::RuntimeKey::CacheTtlSecs)
}

fn cache_max_bytes() -> Option<u64> {
    crate::system::runtime_config::global_runtime_config()
        .get_u64(crate::system::runtime_config::RuntimeKey::CacheMaxBytes)
}

fn cache_cleanup_interval_secs() -> Option<u64> {
    crate::system::runtime_config::global_runtime_config()
        .get_u64(crate::system::runtime_config::RuntimeKey::CacheCleanupIntervalSecs)
}

fn cache_touch_interval_secs() -> u64 {
    crate::system::runtime_config::global_runtime_config()
        .get_u64(crate::system::runtime_config::RuntimeKey::CacheTouchIntervalSecs)
        .unwrap_or(60)
}

fn cache_ttl_mode() -> String {
    crate::system::runtime_config::global_runtime_config()
        .get_string(crate::system::runtime_config::RuntimeKey::CacheTtlMode)
        .unwrap_or_else(|| "created".to_string())
}

fn cache_swr_enabled() -> bool {
    crate::system::runtime_config::global_runtime_config()
        .get_bool(crate::system::runtime_config::RuntimeKey::CacheSwr)
        .unwrap_or(true)
}

fn cache_cleanup_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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

fn collect_manifest_paths(root: &Path) -> Vec<PathBuf> {
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

#[cfg(test)]
mod tests {
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
            crate::system::runtime_config::global_runtime_config()
                .reload_env_bootstrap_from_env();
            Self { key, prev }
        }

        fn remove(key: &'static str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::remove_var(key);
            crate::system::runtime_config::global_runtime_config()
                .reload_env_bootstrap_from_env();
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
            crate::system::runtime_config::global_runtime_config()
                .reload_env_bootstrap_from_env();
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
}
