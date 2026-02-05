//! SystemCoordinator - основная структура и core методы
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture

use std::sync::{Arc, RwLock};

use anyhow::Result;
use serde_json::Value;
use tracing::warn;

use super::types::{
    CacheClearReport, CacheScope, CacheStatsReport, CacheToggleResult, ConfigIndexCache,
    DiskCacheStatsReport, DomainBundle,
};
use crate::system::basic_observability::BasicObservability;
use crate::system::disk_cache::DiskCache;
use crate::system::intellisense_index::IntellisenseIndexStore;
use crate::system::intellisense_index_store::IntellisenseIndexDiskStore;
use crate::system::parser_coordinator::ParserCoordinator;
use crate::system::runtime_config::{global_runtime_config, RuntimeKey};
use bsl_shared::api::StartupProgressDto;
use bsl_shared::domain::repository::RepositoryStats;

/// Упрощенный системный координатор
///
/// Заменяет CentralTypeSystem, координирует только System Layer компоненты
#[derive(Clone)]
pub struct SystemCoordinator {
    // === SYSTEM LAYER COMPONENTS ONLY ===
    /// ParserCoordinator обёрнут в RwLock для обновления с TypeResolver (Milestone 3.17)
    pub(crate) parser: Arc<RwLock<Arc<ParserCoordinator>>>,
    pub(crate) observability: Arc<BasicObservability>,
    pub(crate) disk_cache: Arc<DiskCache>,
    pub(crate) intellisense_index: Arc<IntellisenseIndexStore>,

    // === DOMAIN LAYER CACHE ===
    // Repository + resolver created during startup, shared across the system.
    pub(crate) domain_bundle_cache: Arc<RwLock<Option<Arc<DomainBundle>>>>,

    // === STARTUP PROGRESS (WEB API) ===
    pub(crate) startup_progress: Arc<RwLock<StartupProgressDto>>,

    // === CONFIG INDEX CACHE ===
    pub(crate) config_index_cache: Arc<RwLock<Option<ConfigIndexCache>>>,

    // === PLATFORM VERSION (for platform docs matching) ===
    pub(crate) platform_version: Arc<RwLock<Option<String>>>,

    // === FINGERPRINT MODE (fast vs strict) ===
    // Used for cache keys and deps/config/index fingerprints.
    pub(crate) strict_fingerprint: Arc<RwLock<bool>>,
}

impl Default for SystemCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCoordinator {
    /// Создать новый системный координатор
    pub fn new() -> Self {
        // ТОЛЬКО System Layer компоненты согласно архитектурной диаграмме

        // 1. Disk cache (Milestone D1)
        let disk_cache = match DiskCache::new(1) {
            Ok(cache) => Arc::new(cache),
            Err(err) => {
                warn!("Disk cache disabled: {}", err);
                Arc::new(DiskCache::disabled(1))
            }
        };

        let intellisense_index = Arc::new(IntellisenseIndexStore::new(
            "unknown-config",
            env!("CARGO_PKG_VERSION"),
        ));

        // 2. Simple parsing (будет обновлён с TypeResolver в start_with_paths_blocking)
        // Milestone 3.17: Используем RwLock для возможности обновления
        let parser_instance =
            ParserCoordinator::with_fallback().with_disk_cache(disk_cache.clone());
        parser_instance.set_intellisense_index(intellisense_index.clone());
        let parser = Arc::new(RwLock::new(Arc::new(parser_instance)));

        // 3. Basic observability
        let observability = Arc::new(BasicObservability::default());
        Self {
            parser,
            observability,
            disk_cache,
            intellisense_index,
            domain_bundle_cache: Arc::new(RwLock::new(None)),
            startup_progress: Arc::new(RwLock::new(StartupProgressDto::default())),
            config_index_cache: Arc::new(RwLock::new(None)),
            platform_version: Arc::new(RwLock::new(None)),
            strict_fingerprint: Arc::new(RwLock::new(
                global_runtime_config()
                    .get_bool(RuntimeKey::CacheStrictFingerprint)
                    .unwrap_or(false),
            )),
        }
    }

    pub fn strict_fingerprint(&self) -> bool {
        *self.strict_fingerprint.read().unwrap_or_else(|poisoned| {
            warn!("Strict fingerprint RwLock poisoned (read), recovering data.");
            poisoned.into_inner()
        })
    }

    pub fn set_strict_fingerprint(&self, strict: bool) {
        let mut guard = self.strict_fingerprint.write().unwrap_or_else(|poisoned| {
            warn!("Strict fingerprint RwLock poisoned (write), recovering data.");
            poisoned.into_inner()
        });
        *guard = strict;
    }

    pub fn platform_version(&self) -> Option<String> {
        self.platform_version
            .read()
            .unwrap_or_else(|poisoned| {
                warn!("Platform version RwLock poisoned (read), recovering data.");
                poisoned.into_inner()
            })
            .clone()
    }

    pub fn set_platform_version(&self, version: Option<String>) {
        let mut guard = self.platform_version.write().unwrap_or_else(|poisoned| {
            warn!("Platform version RwLock poisoned (write), recovering data.");
            poisoned.into_inner()
        });
        *guard = version;
    }

    /// Клонирование для передачи в spawn_blocking
    ///
    /// Все поля уже обёрнуты в Arc, так что это дешёвая операция
    pub(crate) fn clone_for_blocking(&self) -> Self {
        self.clone()
    }

    pub fn record_completion_latency(&self, duration: std::time::Duration) {
        self.observability.record_completion_latency(duration);
    }

    pub fn record_completion_stage_latency(&self, stage: &str, duration: std::time::Duration) {
        self.observability
            .record_completion_stage_latency(stage, duration);
    }

    pub fn record_completion_error(&self) {
        self.observability.record_completion_error();
    }

    pub fn record_completion_resolve_latency(&self, duration: std::time::Duration) {
        self.observability
            .record_completion_resolve_latency(duration);
    }

    pub fn record_completion_incomplete(&self) {
        self.observability.record_completion_incomplete();
    }

    pub fn observability_metrics(&self) -> Value {
        self.observability.get_metrics().export_metrics()
    }

    pub fn record_signature_help_latency(&self, duration: std::time::Duration) {
        self.observability.record_signature_help_latency(duration);
    }

    pub fn record_signature_help_empty(&self) {
        self.observability.record_signature_help_empty();
    }

    pub fn record_intellisense_v2_wait_for_file_version(
        &self,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_wait_for_file_version(kind, duration);
    }

    pub fn record_intellisense_v2_snapshot_latency(
        &self,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_snapshot_latency(kind, duration);
    }

    pub fn record_intellisense_v2_ir_query_latency(
        &self,
        kind: &str,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_ir_query_latency(kind, duration);
    }

    pub fn record_intellisense_v2_syntax_diagnostics_query_latency(
        &self,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_syntax_diagnostics_query_latency(duration);
    }

    pub fn record_intellisense_v2_semantic_diagnostics_query_latency(
        &self,
        duration: std::time::Duration,
    ) {
        self.observability
            .record_intellisense_v2_semantic_diagnostics_query_latency(duration);
    }

    pub fn record_intellisense_v2_deps_update_build_latency(&self, duration: std::time::Duration) {
        self.observability
            .record_intellisense_v2_deps_update_build_latency(duration);
    }

    pub fn record_intellisense_v2_deps_update_apply_latency(&self, duration: std::time::Duration) {
        self.observability
            .record_intellisense_v2_deps_update_apply_latency(duration);
    }

    pub fn record_intellisense_v2_deps_update_success(&self) {
        self.observability
            .record_intellisense_v2_deps_update_success();
    }

    pub fn record_intellisense_v2_deps_update_error(&self) {
        self.observability
            .record_intellisense_v2_deps_update_error();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_completion_quality(
        &self,
        total_candidates: usize,
        dedup_removed: usize,
        score_samples: &[f32],
        prefix_exact: usize,
        prefix_starts: usize,
        prefix_contains: usize,
        prefix_none: usize,
        member_access: usize,
        has_owner: usize,
    ) {
        self.observability.record_completion_quality(
            total_candidates,
            dedup_removed,
            score_samples,
            prefix_exact,
            prefix_starts,
            prefix_contains,
            prefix_none,
            member_access,
            has_owner,
        );
    }

    /// Получить ParserCoordinator (Milestone 2.18: для синтаксических ошибок в LSP)
    pub fn parser_coordinator(&self) -> Option<Arc<ParserCoordinator>> {
        let parser = self.parser.read().unwrap_or_else(|poisoned| {
            warn!("Parser RwLock poisoned (read), recovering data.");
            poisoned.into_inner()
        });
        Some(parser.clone())
    }

    /// Получить кэш индекса конфигурации (для инкрементального обновления)
    pub fn config_index_cache(&self) -> Arc<RwLock<Option<ConfigIndexCache>>> {
        self.config_index_cache.clone()
    }

    /// Получить DiskCache (Milestone D1)
    pub fn disk_cache(&self) -> Arc<DiskCache> {
        self.disk_cache.clone()
    }

    pub fn intellisense_index_snapshot_id(&self) -> crate::system::IndexSnapshotId {
        self.intellisense_index.snapshot_id()
    }

    pub fn intellisense_index(&self) -> Arc<IntellisenseIndexStore> {
        self.intellisense_index.clone()
    }

    pub fn intellisense_index_store_for_scope(
        &self,
        scope: &CacheScope,
    ) -> Result<IntellisenseIndexDiskStore> {
        self.intellisense_index_store_for_ids(&scope.project_id, &scope.config_set_id)
    }

    pub fn intellisense_index_store_for_ids(
        &self,
        project_id: &str,
        config_id: &str,
    ) -> Result<IntellisenseIndexDiskStore> {
        let root = self
            .disk_cache
            .root_path()
            .join("index")
            .join(project_id)
            .join(config_id);
        IntellisenseIndexDiskStore::new_with_root(root)
    }

    /// Включить/выключить кэш с учетом ENV-приоритета
    pub async fn set_cache_enabled(&self, enabled: bool) -> CacheToggleResult {
        let env_disabled = self.disk_cache.env_disabled();
        let effective = self.disk_cache.set_enabled(enabled);

        if let Some(parser) = self.parser_coordinator() {
            parser.set_cache_enabled(effective);
            if !effective {
                parser.clear_ast_cache();
            }
        }

        if !effective {
            self.intellisense_index.invalidate_all();
        }

        CacheToggleResult {
            requested: enabled,
            effective,
            env_disabled,
        }
    }

    /// Получить статистику кэша для указанного scope
    pub async fn cache_stats(&self, scope: &CacheScope) -> anyhow::Result<CacheStatsReport> {
        let mut scope_ids = scope.config_ids.clone();
        if !scope.config_set_id.is_empty() {
            scope_ids.push(scope.config_set_id.clone());
        }
        scope_ids.sort();
        scope_ids.dedup();

        let disk_cache = self.disk_cache.clone();
        let project_id = scope.project_id.clone();
        let disk_scope = tokio::task::spawn_blocking(move || {
            disk_cache.scope_usage_for_ids(&project_id, &scope_ids)
        })
        .await??;
        let disk_runtime = self.disk_cache.stats();
        let cache_root = self.disk_cache.root_path().to_string_lossy().into_owned();
        let ast_stats = self
            .parser_coordinator()
            .map(|parser| parser.ast_cache_stats())
            .unwrap_or_default();

        Ok(CacheStatsReport {
            cache_enabled: self.disk_cache.is_enabled(),
            env_disabled: self.disk_cache.env_disabled(),
            swr_enabled: self.disk_cache.swr_enabled(),
            cache_root,
            scope: scope.clone(),
            disk: DiskCacheStatsReport {
                runtime: disk_runtime,
                scope: disk_scope,
            },
            ast: ast_stats,
        })
    }

    /// Очистить кэш для указанного scope
    pub async fn clear_cache_scope(&self, scope: &CacheScope) -> anyhow::Result<CacheClearReport> {
        let mut scope_ids = scope.config_ids.clone();
        if !scope.config_set_id.is_empty() {
            scope_ids.push(scope.config_set_id.clone());
        }
        scope_ids.sort();
        scope_ids.dedup();

        let disk = self
            .disk_cache
            .clear_scope_for_ids(&scope.project_id, &scope_ids)?;

        if let Some(parser) = self.parser_coordinator() {
            parser.clear_ast_cache();
        }
        self.intellisense_index.invalidate_all();

        Ok(CacheClearReport {
            scope: scope.clone(),
            disk,
            ast_cleared: true,
        })
    }

    /// Получить AnalysisEngine (делегирует Domain Layer логику)
    pub fn get_domain_bundle(&self) -> Option<Arc<DomainBundle>> {
        let cache = self.domain_bundle_cache.read()
            .unwrap_or_else(|poisoned| {
                warn!("Domain bundle cache RwLock poisoned (read), recovering data. This indicates a panic in another thread.");
                poisoned.into_inner()
            });
        cache.clone()
    }

    pub fn domain_bundle(&self) -> Option<Arc<DomainBundle>> {
        let cache = self.domain_bundle_cache.read()
            .unwrap_or_else(|poisoned| {
                warn!("Domain bundle cache RwLock poisoned (read), recovering data. This indicates a panic in another thread.");
                poisoned.into_inner()
            });
        cache.clone()
    }

    /// Health check
    pub fn health_status(&self) -> crate::system::basic_observability::HealthStatus {
        self.observability.health_check()
    }

    /// Получить статистику TypeRepository (Task 2.20.4)
    pub fn get_type_repository_stats(&self) -> RepositoryStats {
        if let Some(bundle) = self.domain_bundle() {
            bundle.repository.get_stats()
        } else {
            RepositoryStats::default()
        }
    }

    /// Получить текущий прогресс старта (для Web API polling).
    pub fn startup_progress(&self) -> StartupProgressDto {
        let guard = self.startup_progress.read().unwrap_or_else(|poisoned| {
            warn!("Startup progress RwLock poisoned (read), recovering data.");
            poisoned.into_inner()
        });
        guard.clone()
    }

    pub(crate) fn set_startup_progress(&self, progress: StartupProgressDto) {
        let mut guard = self.startup_progress.write().unwrap_or_else(|poisoned| {
            warn!("Startup progress RwLock poisoned (write), recovering data.");
            poisoned.into_inner()
        });

        // Проценты не должны "доезжать" до 100% до реального конца.
        // 100% разрешаем только когда done=true.
        let mut progress = progress;
        if !progress.done && progress.percentage >= 100.0 {
            progress.percentage = 99.0;
        }

        // Гарантия монотонности процентов
        if progress.percentage < guard.percentage {
            progress.percentage = guard.percentage;
        }
        *guard = progress;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_progress_is_monotonic() {
        let coordinator = SystemCoordinator::new();

        coordinator.set_startup_progress(StartupProgressDto {
            percentage: 10.0,
            ..StartupProgressDto::default()
        });
        assert_eq!(coordinator.startup_progress().percentage, 10.0);

        coordinator.set_startup_progress(StartupProgressDto {
            percentage: 5.0,
            ..StartupProgressDto::default()
        });
        assert_eq!(coordinator.startup_progress().percentage, 10.0);
    }

    #[test]
    fn startup_progress_clamps_100_before_done() {
        let coordinator = SystemCoordinator::new();

        coordinator.set_startup_progress(StartupProgressDto {
            percentage: 100.0,
            done: false,
            ..StartupProgressDto::default()
        });
        assert_eq!(coordinator.startup_progress().percentage, 99.0);

        coordinator.set_startup_progress(StartupProgressDto {
            percentage: 100.0,
            done: true,
            ..StartupProgressDto::default()
        });
        assert_eq!(coordinator.startup_progress().percentage, 100.0);
    }
}
