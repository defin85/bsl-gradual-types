//! System Coordinator - упрощенная замена CentralTypeSystem
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture
//! Координирует только System Layer компоненты

use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::application::type_system_service::TypeSystemService;
use crate::data::adapters::convert_syntax_helper_to_raw;
use crate::data::loaders::{hbk_recovery, progress::ProgressUpdate, SyntaxHelperParser};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::RawTypeData;
use bsl_shared::engine::AnalysisEngine;

use super::basic_observability::BasicObservability;
use super::ir_cache::IrCache;
use super::parser_coordinator::ParserCoordinator;
use super::simple_cache::AnalysisCache;

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
//   let service = self.type_service_cache.read().unwrap_or_else(...);  // ❌ WRONG ORDER
//   let engine = self.analysis_engine_cache.read().unwrap_or_else(...);
//
// ============================================================================

/// Упрощенный системный координатор
///
/// Заменяет CentralTypeSystem, координирует только System Layer компоненты
#[derive(Clone)]
pub struct SystemCoordinator {
    // === SYSTEM LAYER COMPONENTS ONLY ===
    cache: Arc<AnalysisCache>,
    ir_cache: Arc<IrCache>, // Milestone 2.13: IR кеширование для LSP hover
    /// ParserCoordinator обёрнут в RwLock для обновления с TypeResolver (Milestone 3.17)
    parser: Arc<RwLock<Arc<ParserCoordinator>>>,
    observability: Arc<BasicObservability>,

    // === ANALYSIS ENGINE CACHE ===
    // Используем Arc<RwLock> для оптимизации read-heavy паттернов
    analysis_engine_cache: Arc<RwLock<Option<Arc<AnalysisEngine>>>>,

    // === TYPE SERVICE CACHE ===
    type_service_cache: Arc<RwLock<Option<Arc<TypeSystemService>>>>,
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
    fn clone_for_blocking(&self) -> Self {
        self.clone()
    }

    /// Инициализация системы с реальным парсингом синтаксис-помощника
    pub async fn start(&self) -> Result<(), StartupError> {
        self.start_with_paths(None, None, None).await
    }

    /// Инициализация системы с настраиваемыми путями (async версия)
    ///
    /// ВНИМАНИЕ: Эта функция выполняется в основном tokio event loop.
    /// Для CPU-intensive парсинга используйте start_with_paths_blocking() через spawn_blocking()
    pub async fn start_with_paths(
        &self,
        syntax_helper_path: Option<&Path>,
        config_path: Option<&Path>,
        progress_tx: Option<mpsc::UnboundedSender<ProgressUpdate>>,
    ) -> Result<(), StartupError> {
        info!("🔄 Starting platform types parser in blocking thread...");

        // Делегируем синхронной версии через spawn_blocking для предотвращения блокировки event loop
        let coordinator = self.clone_for_blocking();
        let syntax_path_owned = syntax_helper_path.map(|p| p.to_path_buf());
        let config_path_owned = config_path.map(|p| p.to_path_buf());

        let parser_handle = tokio::task::spawn_blocking(move || {
            info!("🔄 [BLOCKING THREAD] Parser started");
            let result = coordinator.start_with_paths_blocking(
                syntax_path_owned.as_deref(),
                config_path_owned.as_deref(),
                progress_tx,
            );
            info!("🔄 [BLOCKING THREAD] Parser finished");
            result
        });

        info!("🔄 Parser spawned in blocking thread, event loop remains free");

        match parser_handle.await {
            Ok(result) => result,
            Err(e) => Err(StartupError::CacheError(format!(
                "Blocking task panicked: {}",
                e
            ))),
        }
    }

    /// Синхронная версия инициализации системы (для spawn_blocking)
    ///
    /// Выполняет CPU-intensive парсинг типов платформы в отдельном блокирующем потоке,
    /// не блокируя основной tokio event loop.
    pub fn start_with_paths_blocking(
        &self,
        syntax_helper_path: Option<&Path>,
        config_path: Option<&Path>,
        progress_tx: Option<mpsc::UnboundedSender<ProgressUpdate>>,
    ) -> Result<(), StartupError> {
        self.observability.log_startup();

        info!("🎯 [BLOCKING THREAD] SystemCoordinator: инициализация System Layer...");

        // ✅ КРИТИЧЕСКИ ВАЖНО: Очищаем кеши при повторной инициализации
        // Это гарантирует, что TypeSystemService получит НОВЫЙ AnalysisEngine с НОВЫМ TypeRepository
        // Соблюдаем lock order convention: analysis_engine_cache → type_service_cache
        {
            let mut engine_cache = self.analysis_engine_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("⚠️ Analysis engine cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            let mut service_cache = self.type_service_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("⚠️ Type service cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });

            if engine_cache.is_some() || service_cache.is_some() {
                info!("🔄 [BLOCKING THREAD] Очищаем кеши AnalysisEngine и TypeSystemService для повторной инициализации");
                *engine_cache = None;
                *service_cache = None;
            }
        }

        // === PHASE 3: Infrastructure инициализация в SystemCoordinator ===

        // 1. Создаем Infrastructure компоненты (Data Layer)
        info!("📦 SystemCoordinator: инициализация Data Layer loaders...");
        let mut syntax_parser = SyntaxHelperParser::new();

        // 2. Загружаем синтаксис-помощник если путь указан
        if let Some(syntax_path) = syntax_helper_path {
            // 🔧 HBK Recovery: Восстанавливаем .hbk файлы перед парсингом
            info!("🔍 Проверяем наличие .hbk файлов для восстановления...");

            // ✅ PHASE 2: Интеграция Progress Callback для HBK Recovery
            let hbk_result = if let Some(ref tx) = progress_tx {
                let tx_clone = tx.clone();
                hbk_recovery::auto_recover_directory_with_progress(
                    syntax_path,
                    Some(move |update: crate::data::loaders::progress::ProgressUpdateType| {
                        if let crate::data::loaders::progress::ProgressUpdateType::HbkExtraction { file_name, extracted_files, total_files, percentage } = update {
                            let _ = tx_clone.send(crate::data::loaders::progress::ProgressUpdate {
                                phase: crate::data::loaders::progress::IndexingPhase::HbkExtraction,
                                current: extracted_files,
                                total: total_files,
                                percentage: percentage as f32,
                                message: Some(format!("Извлекаем файлы из {}", file_name)),
                            });
                        }
                    }),
                )
            } else {
                hbk_recovery::auto_recover_directory(syntax_path)
            };

            match hbk_result {
                Ok(results) if !results.is_empty() => {
                    info!("✅ Восстановлено {} .hbk файлов", results.len());
                    for result in &results {
                        info!(
                            "   📦 Файл: {:?} → {:?}",
                            result.repaired_zip_path.file_name().unwrap_or_default(),
                            result
                                .extracted_dir
                                .as_ref()
                                .map(|d| d.file_name().unwrap_or_default())
                        );
                    }
                }
                Ok(_) => {
                    info!("📁 .hbk файлы не найдены (возможно уже распакованы)");
                }
                Err(e) => {
                    warn!("⚠️ Ошибка восстановления .hbk файлов: {}. Продолжаем с существующими файлами...", e);
                }
            }

            info!("📂 Загружаем синтаксис-помощник: {}", syntax_path.display());

            // ✅ MILESTONE 2.20.2.3: Парсим с прогрессом если передан callback
            if let Some(ref tx) = progress_tx {
                let tx_clone = tx.clone();
                match syntax_parser.parse_with_progress(
                    syntax_path,
                    move |update: ProgressUpdate| {
                        let _ = tx_clone.send(update); // Отправляем в channel
                    },
                ) {
                    Ok(()) => {
                        info!("✅ Парсинг синтаксис-помощника завершен успешно");
                    }
                    Err(e) => {
                        warn!("⚠️ Ошибка парсинга синтаксис-помощника: {}", e);
                        info!("📦 Будем использовать базовые типы платформы 1С...");
                    }
                }
            } else {
                // Обратная совместимость: парсим без прогресса
                match syntax_parser.parse_syntax_helper(syntax_path) {
                    Ok(()) => {
                        info!("✅ Парсинг синтаксис-помощника завершен успешно");
                    }
                    Err(e) => {
                        warn!("⚠️ Ошибка парсинга синтаксис-помощника: {}", e);
                        info!("📦 Будем использовать базовые типы платформы 1С...");
                    }
                }
            }
        }

        // 3. Создаем Domain Layer компоненты
        info!("🧠 SystemCoordinator: инициализация Domain Layer...");
        let repository = Arc::new(InMemoryTypeRepository::new());

        // 4. Загружаем данные в репозиторий (через Adapters)
        let database = syntax_parser.export_database();
        if !database.nodes.is_empty() {
            let platform_raw_data = convert_syntax_helper_to_raw(&database);

            // ✅ MILESTONE 2.20.5: Заполняем SignatureIndex из загруженных типов
            let platform_types_clone = platform_raw_data.clone(); // Клонируем для SignatureIndex

            repository
                .load_types(platform_raw_data)
                .map_err(StartupError::PlatformTypesError)?;

            // Заполняем SignatureIndex через Registry паттерн
            // Это гарантирует что все источники типов (SyntaxHelper + PlatformFacetTypes)
            // всегда применяются в правильном порядке и невозможно забыть какой-либо источник
            use bsl_shared::domain::SignatureSourceRegistry;
            use crate::data::loaders::{SyntaxHelperSource, PlatformFacetTypesSource};

            let index = SignatureSourceRegistry::new()
                .register(SyntaxHelperSource::new(platform_types_clone))
                .register(PlatformFacetTypesSource)
                .build();
            repository.set_signature_index(index);

            let stats = repository.get_stats();
            info!(
                "📊 Загружено {} типов из синтаксис-помощника",
                stats.total_types
            );
            info!("📇 SignatureIndex заполнен платформенными методами и конструкторами");
        } else {
            // Загружаем базовые типы как fallback
            Self::load_fallback_types(&repository)?;
        }

        // 5. Создаем Domain resolver
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        // MILESTONE 3.17: Обновляем ParserCoordinator с TypeResolver для резолюции active_facet
        {
            let new_parser = Arc::new(ParserCoordinator::new_with_resolver(
                repository.clone(),
                resolver.clone(),
            ));
            let mut parser_guard = self.parser.write()
                .unwrap_or_else(|poisoned| {
                    warn!("⚠️ Parser RwLock poisoned (write), recovering data.");
                    poisoned.into_inner()
                });
            *parser_guard = new_parser;
            info!("📇 ParserCoordinator обновлён с TypeResolver для Milestone 3.17");
        }

        // 6. Создаем упрощенный AnalysisEngine (без Infrastructure зависимостей)
        let analysis_engine = AnalysisEngine::new(resolver, repository);

        // Кешируем AnalysisEngine
        {
            let mut cache = self.analysis_engine_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("⚠️ Analysis engine cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            *cache = Some(Arc::new(analysis_engine));
        }

        // 7. Загружаем метаданные конфигурации если путь указан
        if let Some(config_path) = config_path {
            info!(
                "📂 Загружаем метаданные конфигурации: {}",
                config_path.display()
            );

            // ✅ НОВОЕ: Используем версию с прогрессом если передан progress_tx
            let result = if let Some(ref tx) = progress_tx {
                let tx_clone = tx.clone();
                self.load_all_configurations_with_progress(config_path, move |update| {
                    let _ = tx_clone.send(update); // Отправляем прогресс в channel
                })
            } else {
                // Обратная совместимость: используем версию без прогресса
                self.load_all_configurations_metadata(config_path)
            };

            match result {
                Ok(result_data) => {
                    info!(
                        "✅ Загружено {} типов из {} базовых конфигураций и {} расширений",
                        result_data.total_types,
                        result_data.base_config_count,
                        result_data.extensions_count
                    );
                }
                Err(e) => {
                    warn!("⚠️ Ошибка загрузки метаданных конфигурации: {}", e);
                    info!("📦 Продолжаем работу с типами платформы...");
                }
            }
        }

        info!("💾 [BLOCKING THREAD] SystemCoordinator: прогрев кеша...");
        self.cache.warm_cache()?;

        info!("✅ [BLOCKING THREAD] SystemCoordinator: система готова!");
        Ok(())
    }

    /// Загрузка базовых типов как fallback
    fn load_fallback_types(repository: &Arc<InMemoryTypeRepository>) -> Result<(), StartupError> {
        use bsl_shared::domain::types::{RawDataSource, RawTypeData};

        info!("📦 Загружаем базовые типы платформы 1С...");

        // Используем платформенные типы из loaders
        let mut platform_types = crate::data::loaders::load_all_platform_types();

        // Добавляем примитивные типы
        platform_types.extend(vec![
            RawTypeData {
                name: "Строка".to_string(),
                english_name: "String".to_string(),
                description: "Строковый тип данных".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Число".to_string(),
                english_name: "Number".to_string(),
                description: "Числовой тип данных".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Булево".to_string(),
                english_name: "Boolean".to_string(),
                description: "Логический тип данных".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Дата".to_string(),
                english_name: "Date".to_string(),
                description: "Тип данных для работы с датой и временем".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ]);

        // ✅ MILESTONE 2.20.5: Клонируем для SignatureIndex
        let platform_types_clone = platform_types.clone();

        repository
            .load_types(platform_types)
            .map_err(StartupError::PlatformTypesError)?;

        // Заполняем SignatureIndex через Registry паттерн
        // ВАЖНО: PlatformFacetTypesSource гарантирует что фасетные типы всегда добавлены!
        // Раньше в fallback пути забывали добавлять PlatformFacetTypes - Registry исключает эту ошибку
        use bsl_shared::domain::SignatureSourceRegistry;
        use crate::data::loaders::{SyntaxHelperSource, PlatformFacetTypesSource};

        let index = SignatureSourceRegistry::new()
            .register(SyntaxHelperSource::new(platform_types_clone.clone()))
            .register(PlatformFacetTypesSource)
            .build();
        repository.set_signature_index(index);

        info!(
            "✅ Базовые типы загружены: {} типов",
            platform_types_clone.len()
        );
        info!("📇 SignatureIndex заполнен платформенными методами и конструкторами");
        Ok(())
    }

    /// Получить компоненты для создания TypeSystemService
    pub fn get_system_components(&self) -> (Arc<AnalysisCache>, Arc<ParserCoordinator>) {
        let parser = self.parser.read()
            .unwrap_or_else(|poisoned| {
                warn!("⚠️ Parser RwLock poisoned (read), recovering data.");
                poisoned.into_inner()
            });
        (self.cache.clone(), parser.clone())
    }

    /// Получить ParserCoordinator (Milestone 2.18: для синтаксических ошибок в LSP)
    pub fn parser_coordinator(&self) -> Option<Arc<ParserCoordinator>> {
        let parser = self.parser.read()
            .unwrap_or_else(|poisoned| {
                warn!("⚠️ Parser RwLock poisoned (read), recovering data.");
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
                warn!("⚠️ Analysis engine cache RwLock poisoned (read), recovering data. This indicates a panic in another thread.");
                poisoned.into_inner()
            });
        cache.clone()
    }

    /// Создать TypeSystemService (singleton)
    ///
    /// Согласно архитектуре: TypeSystemService использует AnalysisEngine для доступа к Domain Layer
    ///
    /// # Lock Order
    /// Соблюдает lock order convention: analysis_engine_cache → type_service_cache
    pub fn type_service(&self) -> Option<Arc<TypeSystemService>> {
        // Сначала пробуем прочитать из кеша (read lock - быстро)
        {
            let cache = self.type_service_cache.read()
                .unwrap_or_else(|poisoned| {
                    warn!("⚠️ Type service cache RwLock poisoned (read), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            if let Some(service) = cache.as_ref() {
                return Some(service.clone());
            }
        } // 🔓 Освобождаем read lock

        // Кеш пуст, нужно создать service

        // Читаем analysis_engine (read lock, соблюдаем lock order)
        let analysis_engine = {
            let engine_cache = self.analysis_engine_cache.read()
                .unwrap_or_else(|poisoned| {
                    warn!("⚠️ Analysis engine cache RwLock poisoned (read), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            engine_cache.clone()
        }; // 🔓 Освобождаем read lock

        if let Some(engine) = analysis_engine {
            // TypeSystemService теперь использует AnalysisEngine вместо прямого доступа к Domain Layer
            // Milestone 3.17: Получаем ParserCoordinator через RwLock
            let parser = {
                let parser_guard = self.parser.read()
                    .unwrap_or_else(|poisoned| {
                        warn!("⚠️ Parser RwLock poisoned (read), recovering data.");
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
                        warn!("⚠️ Type service cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                        poisoned.into_inner()
                    });
                *cache = Some(service.clone());
            } // 🔓 Освобождаем write lock

            Some(service)
        } else {
            None
        }
    }

    /// Получить AnalysisEngine для CLI/прямого использования
    pub fn analysis_engine(&self) -> Option<Arc<AnalysisEngine>> {
        let cache = self.analysis_engine_cache.read()
            .unwrap_or_else(|poisoned| {
                warn!("⚠️ Analysis engine cache RwLock poisoned (read), recovering data. This indicates a panic in another thread.");
                poisoned.into_inner()
            });
        cache.clone()
    }

    /// Health check
    pub fn health_status(&self) -> crate::system::basic_observability::HealthStatus {
        self.observability.health_check()
    }

    /// Получить статистику TypeRepository (Task 2.20.4)
    pub fn get_type_repository_stats(&self) -> bsl_shared::domain::repository::RepositoryStats {
        // Получаем AnalysisEngine
        if let Some(engine) = self.analysis_engine() {
            let repository = engine.get_repository();
            repository.get_stats()
        } else {
            // Если AnalysisEngine не инициализирован - возвращаем пустую статистику
            bsl_shared::domain::repository::RepositoryStats::default()
        }
    }

    /// Загрузить метаданные конфигурации через универсальный парсер
    ///
    /// Использует ConfigurationDiscovery для автоматического обнаружения всех объектов метаданных
    /// и загружает их в TypeRepository текущего AnalysisEngine.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use bsl_backend::system::SystemCoordinator;
    ///
    /// let coordinator = SystemCoordinator::new();
    /// let config_path = Path::new("examples/conf/conf_test");
    /// let loaded = coordinator.load_configuration_metadata(config_path).unwrap();
    /// println!("Загружено {} объектов метаданных", loaded);
    /// ```
    pub fn load_configuration_metadata(&self, config_path: &Path) -> Result<usize> {
        use crate::data::loaders::ConfigurationDiscovery;

        info!("🔍 Загрузка метаданных конфигурации из {:?}", config_path);

        // Создаём discovery и обнаруживаем все объекты
        // show_progress = false - нет терминального прогресс-бара для этого метода
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), false);

        // Без progress_callback в публичном методе (для обратной совместимости)
        let metadata_objects = discovery
            .discover_all_metadata(None::<fn(crate::data::loaders::progress::ProgressUpdate)>)
            .map_err(|e| anyhow::anyhow!("Не удалось обнаружить метаданные: {}", e))?;

        info!(
            "📦 Обнаружено {} объектов метаданных",
            metadata_objects.len()
        );

        // Получаем текущий AnalysisEngine или создаём новый
        let engine = self.analysis_engine().ok_or_else(|| {
            anyhow::anyhow!("AnalysisEngine не инициализирован. Вызовите start() сначала.")
        })?;

        // Получаем TypeRepository из AnalysisEngine
        let repository = engine.get_repository();

        // Конвертируем все объекты в RawTypeData
        let raw_types: Vec<RawTypeData> = metadata_objects
            .into_iter()
            .map(|obj| obj.to_raw_type_data(None))
            .collect();

        let count = raw_types.len();

        // Загружаем все типы в репозиторий за один вызов
        repository.load_types(raw_types)?;

        info!("✅ Загружено {} типов из конфигурации", count);
        Ok(count)
    }

    /// Загружает метаданные из ВСЕХ конфигураций с прогрессом (4 фазы для каждой)
    ///
    /// Новая версия с поддержкой прогресса через callback.
    /// Автоматически обнаруживает все конфигурации в указанной папке:
    /// - Базовую конфигурацию (ObjectBelonging = "Own")
    /// - Расширения конфигурации (ObjectBelonging = "Adopted")
    ///
    /// Для каждой конфигурации отправляет прогресс через 4 фазы:
    /// - ConfigurationDiscovery (0-5%)
    /// - ConfigurationParsing (5-85%)
    /// - ConfigurationLinking (85-95%)
    /// - ConfigurationFinalizing (95-100%)
    ///
    /// # Аргументы
    /// * `config_path` - Путь к папке с конфигурациями (содержит Configuration.xml или подпапки)
    /// * `progress_callback` - Callback для отслеживания прогресса
    ///
    /// # Возвращает
    /// * `LoadMetadataResult` - Статистика загруженных типов
    pub fn load_all_configurations_with_progress<F>(
        &self,
        config_path: &Path,
        progress_callback: F,
    ) -> Result<LoadMetadataResult>
    where
        F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
    {
        use crate::data::loaders::config_metadata_parser::{
            ConfigurationDiscovery, ConfigurationType,
        };

        info!(
            "🔍 [WITH PROGRESS] Обнаружение конфигураций в: {}",
            config_path.display()
        );

        // show_progress = true - показываем терминальный прогресс-бар для Web Server
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), true);
        let configurations = discovery
            .discover_all_configurations()
            .map_err(|e| anyhow::anyhow!("Ошибка обнаружения конфигураций: {}", e))?;

        info!("✅ Найдено {} конфигураций", configurations.len());

        let mut base_count = 0;
        let mut ext_count = 0;
        let mut total_types = 0;

        // Получаем AnalysisEngine и TypeRepository один раз
        let engine = self.analysis_engine().ok_or_else(|| {
            anyhow::anyhow!("AnalysisEngine не инициализирован. Вызовите start() сначала.")
        })?;
        let repository = engine.get_repository();

        for config_info in configurations {
            info!(
                "📦 Загрузка конфигурации: {} ({}{})",
                config_info.name,
                if config_info.is_base() {
                    "Base"
                } else {
                    "Extension"
                },
                config_info
                    .prefix
                    .as_ref()
                    .map(|p| format!(", префикс: {}", p))
                    .unwrap_or_default()
            );

            // ✅ НОВОЕ: Используем новый метод с прогрессом через 4 фазы
            let metadata = discovery
                .discover_metadata_in_configuration_with_progress(
                    &config_info,
                    progress_callback.clone(),
                )
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))?;

            let prefix = config_info.prefix.as_deref();
            let raw_types: Vec<RawTypeData> = metadata
                .into_iter()
                .map(|obj| obj.to_raw_type_data(prefix))
                .collect();

            total_types += raw_types.len();

            // Загружаем типы в репозиторий
            repository
                .load_types(raw_types)
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки типов: {}", e))?;

            match config_info.config_type {
                ConfigurationType::Base => base_count += 1,
                ConfigurationType::Extension => ext_count += 1,
            }
        }

        Ok(LoadMetadataResult {
            base_config_count: base_count,
            extensions_count: ext_count,
            total_types,
        })
    }

    /// Загружает метаданные из ВСЕХ конфигураций (базовой + расширений) с применением префиксов
    ///
    /// Автоматически обнаруживает все конфигурации в указанной папке:
    /// - Базовую конфигурацию (ObjectBelonging = "Own")
    /// - Расширения конфигурации (ObjectBelonging = "Adopted")
    ///
    /// Для каждого расширения извлекает префикс из метаданных и применяет его к именам объектов,
    /// создавая корректные имена типов вида "Справочники.Префикс_Имя".
    ///
    /// # Аргументы
    /// * `config_path` - Путь к папке с конфигурациями (содержит Configuration.xml или подпапки)
    ///
    /// # Возвращает
    /// * `LoadMetadataResult` - Статистика загруженных типов
    ///
    /// # Примеры
    /// ```no_run
    /// use std::path::Path;
    /// use bsl_backend::system::SystemCoordinator;
    ///
    /// let coordinator = SystemCoordinator::new();
    /// coordinator.start().await.unwrap();
    ///
    /// let config_path = Path::new("examples/conf");
    /// let result = coordinator.load_all_configurations_metadata(config_path).unwrap();
    ///
    /// println!("Загружено {} типов", result.total_types);
    /// println!("Базовых конфигураций: {}", result.base_config_count);
    /// println!("Расширений: {}", result.extensions_count);
    /// ```
    pub fn load_all_configurations_metadata(
        &self,
        config_path: &Path,
    ) -> Result<LoadMetadataResult> {
        use crate::data::loaders::config_metadata_parser::{
            ConfigurationDiscovery, ConfigurationType,
        };

        info!("🔍 Обнаружение конфигураций в: {}", config_path.display());

        // show_progress = true - показываем терминальный прогресс-бар при парсинге конфигурации
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), true);
        let configurations = discovery
            .discover_all_configurations()
            .map_err(|e| anyhow::anyhow!("Ошибка обнаружения конфигураций: {}", e))?;

        info!("✅ Найдено {} конфигураций", configurations.len());

        let mut base_count = 0;
        let mut ext_count = 0;
        let mut total_types = 0;

        // Получаем AnalysisEngine и TypeRepository один раз
        let engine = self.analysis_engine().ok_or_else(|| {
            anyhow::anyhow!("AnalysisEngine не инициализирован. Вызовите start() сначала.")
        })?;
        let repository = engine.get_repository();

        for config_info in configurations {
            info!(
                "📦 Загрузка конфигурации: {} ({}{})",
                config_info.name,
                if config_info.is_base() {
                    "Base"
                } else {
                    "Extension"
                },
                config_info
                    .prefix
                    .as_ref()
                    .map(|p| format!(", префикс: {}", p))
                    .unwrap_or_default()
            );

            // Без progress_callback в публичном методе (для обратной совместимости)
            let metadata = discovery
                .discover_metadata_in_configuration(
                    &config_info,
                    None::<fn(crate::data::loaders::progress::ProgressUpdate)>,
                )
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))?;

            let prefix = config_info.prefix.as_deref();
            let raw_types: Vec<RawTypeData> = metadata
                .into_iter()
                .map(|obj| obj.to_raw_type_data(prefix))
                .collect();

            total_types += raw_types.len();

            // Загружаем типы в репозиторий
            repository
                .load_types(raw_types)
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки типов: {}", e))?;

            match config_info.config_type {
                ConfigurationType::Base => base_count += 1,
                ConfigurationType::Extension => ext_count += 1,
            }
        }

        Ok(LoadMetadataResult {
            base_config_count: base_count,
            extensions_count: ext_count,
            total_types,
        })
    }
}

/// Результат загрузки метаданных конфигурации
#[derive(Debug, Clone)]
pub struct LoadMetadataResult {
    /// Количество загруженных базовых конфигураций
    pub base_config_count: usize,
    /// Количество загруженных расширений
    pub extensions_count: usize,
    /// Общее количество типов
    pub total_types: usize,
}

// === ERROR TYPES ===

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("Failed to load platform types: {0}")]
    PlatformTypesError(#[from] anyhow::Error),
    #[error("Cache initialization failed: {0}")]
    CacheError(String),
}

/// Информация о символе для LSP
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub symbol_type: String,
    pub line: u32,
    pub column: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use std::sync::Arc;

    /// Вспомогательная функция для создания тестового репозитория с инициализированными конструкторами
    fn create_test_repository() -> Arc<InMemoryTypeRepository> {
        let repo = Arc::new(InMemoryTypeRepository::new());

        // Загружаем базовые типы с конструкторами
        let platform_types = crate::data::loaders::load_all_platform_types();
        let platform_types_clone = platform_types.clone();

        repo.load_types(platform_types).unwrap();

        // Инициализируем SignatureIndex с конструкторами
        repo.populate_signature_index(|index| {
            index.initialize_builtin_constructors();
            crate::data::loaders::populate_signature_index_from_platform_types(
                &platform_types_clone,
                index,
            );
        });

        repo
    }

    #[test]
    fn test_signature_index_has_builtin_constructors() {
        use bsl_shared::domain::signature_index::SignatureIndex;

        let mut index = SignatureIndex::new();
        index.initialize_builtin_constructors();

        // Проверяем что встроенные конструкторы загружены
        assert!(
            index.find_constructor("Массив").is_some(),
            "Конструктор Массив должен быть загружен"
        );
        assert!(
            index.find_constructor("Соответствие").is_some(),
            "Конструктор Соответствие должен быть загружен"
        );
        assert!(
            index.find_constructor("ТаблицаЗначений").is_some(),
            "Конструктор ТаблицаЗначений должен быть загружен"
        );
        assert!(
            index.find_constructor("СписокЗначений").is_some(),
            "Конструктор СписокЗначений должен быть загружен"
        );
        assert!(
            index.find_constructor("ФиксированныйМассив").is_some(),
            "Конструктор ФиксированныйМассив должен быть загружен"
        );
    }

    #[test]
    fn test_repository_initialization_with_constructors() {
        let repo = create_test_repository();

        // Проверяем что SignatureIndex содержит конструкторы
        repo.populate_signature_index(|index| {
            // Проверяем наличие конструкторов
            assert!(
                index.find_constructor("Массив").is_some(),
                "Конструктор Массив должен быть в индексе"
            );
        });

        // Проверяем что репозиторий успешно инициализирован
        let stats = repo.get_stats();
        assert!(stats.total_types > 0, "Репозиторий должен содержать типы");
    }

    #[test]
    fn test_constructor_resolution_via_repository() {
        use bsl_shared::domain::resolver::TypeResolver;

        let repo = create_test_repository();
        let _resolver = TypeResolver::new(repo.clone());

        // Проверяем что можно резолвить конструктор через TypeResolver
        // Для этого нам нужен SignatureIndex из репозитория
        //
        // Примечание: TypeResolver.resolve_constructor() требует SignatureIndex,
        // который хранится в репозитории. Проверяем интеграцию.

        // Пока просто проверяем что репозиторий создан успешно
        // TODO: Добавить полноценный тест с resolve_constructor после интеграции
        assert!(
            repo.find_type("Массив").is_some(),
            "Тип Массив должен существовать в репозитории"
        );
    }
}
