//! SystemCoordinator - основная структура и core методы
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture

use std::sync::{Arc, RwLock};
use tracing::warn;

use crate::application::TypeSystemService;
use crate::system::basic_observability::BasicObservability;
use crate::system::ir_cache::IrCache;
use crate::system::parser_coordinator::ParserCoordinator;
use crate::system::simple_cache::AnalysisCache;
use bsl_shared::domain::repository::RepositoryStats;
use bsl_shared::engine::AnalysisEngine;

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

    // === ANALYSIS ENGINE CACHE ===
    // Используем Arc<RwLock> для оптимизации read-heavy паттернов
    pub(crate) analysis_engine_cache: Arc<RwLock<Option<Arc<AnalysisEngine>>>>,

    // === TYPE SERVICE CACHE ===
    pub(crate) type_service_cache: Arc<RwLock<Option<Arc<TypeSystemService>>>>,
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

        // 3. Simple parsing (будет обновлён с TypeResolver в start_with_paths_blocking)
        // Milestone 3.17: Используем RwLock для возможности обновления
        let parser = Arc::new(RwLock::new(Arc::new(ParserCoordinator::with_fallback())));

        // 4. Basic observability
        let observability = Arc::new(BasicObservability::default());

        Self {
            cache,
            ir_cache,
            parser,
            observability,
            analysis_engine_cache: Arc::new(RwLock::new(None)),
            type_service_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Клонирование для передачи в spawn_blocking
    ///
    /// Все поля уже обёрнуты в Arc, так что это дешёвая операция
    pub(crate) fn clone_for_blocking(&self) -> Self {
        self.clone()
    }

    /// Получить компоненты для создания TypeSystemService
    pub fn get_system_components(&self) -> (Arc<AnalysisCache>, Arc<ParserCoordinator>) {
        let parser = self.parser.read()
            .unwrap_or_else(|poisoned| {
                warn!("Parser RwLock poisoned (read), recovering data.");
                poisoned.into_inner()
            });
        (self.cache.clone(), parser.clone())
    }

    /// Получить ParserCoordinator (Milestone 2.18: для синтаксических ошибок в LSP)
    pub fn parser_coordinator(&self) -> Option<Arc<ParserCoordinator>> {
        let parser = self.parser.read()
            .unwrap_or_else(|poisoned| {
                warn!("Parser RwLock poisoned (read), recovering data.");
                poisoned.into_inner()
            });
        Some(parser.clone())
    }

    /// Получить IR Cache
    pub fn ir_cache(&self) -> Arc<IrCache> {
        self.ir_cache.clone()
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
                let parser_guard = self.parser.read()
                    .unwrap_or_else(|poisoned| {
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
}
