use super::*;

impl DiskCache {
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
    pub(super) fn cleanup_if_needed(&self) -> Result<()> {
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
}
