//! SystemCoordinator - основная структура и core методы
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture

use std::sync::{Arc, RwLock};
use anyhow::Result;
use tracing::warn;

use crate::application::TypeSystemService;
use crate::system::basic_observability::BasicObservability;
use crate::system::disk_cache::DiskCache;
use crate::system::ir_cache::IrCache;
use crate::system::intellisense_index::IntellisenseIndexStore;
use crate::system::intellisense_index_store::IntellisenseIndexDiskStore;
use crate::system::parser_coordinator::ParserCoordinator;
use crate::system::simple_cache::AnalysisCache;
use bsl_shared::domain::repository::RepositoryStats;
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::api::StartupProgressDto;
use super::types::{
    CacheClearReport, CacheScope, CacheStatsReport, CacheToggleResult, ConfigIndexCache,
    DiskCacheStatsReport,
};

// ============================================================================
// LOCK ORDER CONVENTION
// ============================================================================
//
// To prevent deadlocks, ALWAYS acquire RwLocks in this order:
//
// 1. analysis_engine_cache (first)
// 2. type_service_cache (second)
//
// NEVER acquire locks in reverse order!
//
// Example of CORRECT usage:
//   let engine = self.analysis_engine_cache.read().unwrap_or_else(...);
//   let service = self.type_service_cache.read().unwrap_or_else(...);
//
// Example of INCORRECT usage (DEADLOCK RISK):
//   let service = self.type_service_cache.read().unwrap_or_else(...);  // WRONG ORDER
//   let engine = self.analysis_engine_cache.read().unwrap_or_else(...);
//
// ============================================================================

/// Упрощенный системный координатор
///
/// Заменяет CentralTypeSystem, координирует только System Layer компоненты
#[derive(Clone)]
pub struct SystemCoordinator {
    // === SYSTEM LAYER COMPONENTS ONLY ===
    pub(crate) cache: Arc<AnalysisCache>,
    pub(crate) ir_cache: Arc<IrCache>, // Milestone 2.13: IR кеширование для LSP hover
    /// ParserCoordinator обёрнут в RwLock для обновления с TypeResolver (Milestone 3.17)
    pub(crate) parser: Arc<RwLock<Arc<ParserCoordinator>>>,
    pub(crate) observability: Arc<BasicObservability>,
    pub(crate) disk_cache: Arc<DiskCache>,
    pub(crate) intellisense_index: Arc<IntellisenseIndexStore>,

    // === ANALYSIS ENGINE CACHE ===
    // Используем Arc<RwLock> для оптимизации read-heavy паттернов
    pub(crate) analysis_engine_cache: Arc<RwLock<Option<Arc<AnalysisEngine>>>>,

    // === TYPE SERVICE CACHE ===
    pub(crate) type_service_cache: Arc<RwLock<Option<Arc<TypeSystemService>>>>,

    // === STARTUP PROGRESS (WEB API) ===
    pub(crate) startup_progress: Arc<RwLock<StartupProgressDto>>,

    // === CONFIG INDEX CACHE ===
    pub(crate) config_index_cache: Arc<RwLock<Option<ConfigIndexCache>>>,

    // === PLATFORM VERSION (for platform docs matching) ===
    pub(crate) platform_version: Arc<RwLock<Option<String>>>,
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

        // 1. Simple caching
        let cache = Arc::new(AnalysisCache::new(1000)); // Simple LRU

        // 2. IR caching (Milestone 2.13)
        let ir_cache = Arc::new(IrCache::new(100)); // 100 файлов (~10 MB RAM)

        // 3. Disk cache (Milestone D1)
        let disk_cache = match DiskCache::new(1) {
            Ok(cache) => Arc::new(cache),
            Err(err) => {
                warn!("Disk cache disabled: {}", err);
                Arc::new(DiskCache::disabled(1))
            }
        };

        let intellisense_index =
            Arc::new(IntellisenseIndexStore::new("unknown-config", env!("CARGO_PKG_VERSION")));

        // 4. Simple parsing (будет обновлён с TypeResolver в start_with_paths_blocking)
        // Milestone 3.17: Используем RwLock для возможности обновления
        let parser_instance =
            ParserCoordinator::with_fallback().with_disk_cache(disk_cache.clone());
        parser_instance.set_intellisense_index(intellisense_index.clone());
        let parser = Arc::new(RwLock::new(Arc::new(parser_instance)));

        // 5. Basic observability
        let observability = Arc::new(BasicObservability::default());
        Self {
            cache,
            ir_cache,
            parser,
            observability,
            disk_cache,
            intellisense_index,
            analysis_engine_cache: Arc::new(RwLock::new(None)),
            type_service_cache: Arc::new(RwLock::new(None)),
            startup_progress: Arc::new(RwLock::new(StartupProgressDto::default())),
            config_index_cache: Arc::new(RwLock::new(None)),
            platform_version: Arc::new(RwLock::new(None)),
        }
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

    /// Получить компоненты для создания TypeSystemService
    pub fn get_system_components(&self) -> (Arc<AnalysisCache>, Arc<ParserCoordinator>) {
        let parser = self.parser.read().unwrap_or_else(|poisoned| {
            warn!("Parser RwLock poisoned (read), recovering data.");
            poisoned.into_inner()
        });
        (self.cache.clone(), parser.clone())
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

    /// Получить IR Cache
    pub fn ir_cache(&self) -> Arc<IrCache> {
        self.ir_cache.clone()
    }

    /// Получить DiskCache (Milestone D1)
    pub fn disk_cache(&self) -> Arc<DiskCache> {
        self.disk_cache.clone()
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
            self.ir_cache.clear().await;
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
        let ir_stats = self.ir_cache.get_stats().await;

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
            ir: ir_stats,
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
        self.ir_cache.clear().await;
        self.intellisense_index.invalidate_all();

        Ok(CacheClearReport {
            scope: scope.clone(),
            disk,
            ast_cleared: true,
            ir_cleared: true,
        })
    }

    /// Получить AnalysisEngine (делегирует Domain Layer логику)
    pub fn get_analysis_engine(&self) -> Option<Arc<AnalysisEngine>> {
        let cache = self.analysis_engine_cache.read()
            .unwrap_or_else(|poisoned| {
                warn!("Analysis engine cache RwLock poisoned (read), recovering data. This indicates a panic in another thread.");
                poisoned.into_inner()
            });
        cache.clone()
    }

    /// Создать TypeSystemService (singleton)
    ///
    /// Согласно архитектуре: TypeSystemService использует AnalysisEngine для доступа к Domain Layer
    ///
    /// # Lock Order
    /// Соблюдает lock order convention: analysis_engine_cache -> type_service_cache
    pub fn type_service(&self) -> Option<Arc<TypeSystemService>> {
        // Сначала пробуем прочитать из кеша (read lock - быстро)
        {
            let cache = self.type_service_cache.read()
                .unwrap_or_else(|poisoned| {
                    warn!("Type service cache RwLock poisoned (read), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            if let Some(service) = cache.as_ref() {
                return Some(service.clone());
            }
        } // Освобождаем read lock

        // Кеш пуст, нужно создать service

        // Читаем analysis_engine (read lock, соблюдаем lock order)
        let analysis_engine = {
            let engine_cache = self.analysis_engine_cache.read()
                .unwrap_or_else(|poisoned| {
                    warn!("Analysis engine cache RwLock poisoned (read), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            engine_cache.clone()
        }; // Освобождаем read lock

        if let Some(engine) = analysis_engine {
            // TypeSystemService теперь использует AnalysisEngine вместо прямого доступа к Domain Layer
            // Milestone 3.17: Получаем ParserCoordinator через RwLock
            let parser = {
                let parser_guard = self.parser.read().unwrap_or_else(|poisoned| {
                    warn!("Parser RwLock poisoned (read), recovering data.");
                    poisoned.into_inner()
                });
                parser_guard.clone()
            };

            let service = Arc::new(TypeSystemService::new(
                engine,
                self.cache.clone(),
                parser,
                self.ir_cache.clone(), // Milestone 2.13: передаём IR Cache
                self.intellisense_index.clone(),
            ));

            // Обновляем кеш (write lock - эксклюзивный)
            {
                let mut cache = self.type_service_cache.write()
                    .unwrap_or_else(|poisoned| {
                        warn!("Type service cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                        poisoned.into_inner()
                    });
                *cache = Some(service.clone());
            } // Освобождаем write lock

            Some(service)
        } else {
            None
        }
    }

    /// Получить AnalysisEngine для CLI/прямого использования
    pub fn analysis_engine(&self) -> Option<Arc<AnalysisEngine>> {
        let cache = self.analysis_engine_cache.read()
            .unwrap_or_else(|poisoned| {
                warn!("Analysis engine cache RwLock poisoned (read), recovering data. This indicates a panic in another thread.");
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
        // Получаем AnalysisEngine
        if let Some(engine) = self.analysis_engine() {
            let repository = engine.get_repository();
            repository.get_stats()
        } else {
            // Если AnalysisEngine не инициализирован - возвращаем пустую статистику
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
