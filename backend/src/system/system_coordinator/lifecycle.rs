//! Lifecycle management для SystemCoordinator
//!
//! Инициализация системы, загрузка типов платформы

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{RawDataSource, RawTypeData};
use bsl_shared::engine::AnalysisEngine;
use serde::{Deserialize, Serialize};

use crate::data::adapters::{convert_syntax_helper_global_functions, convert_syntax_helper_to_raw};
use crate::data::loaders::{
    hbk_recovery, progress::ProgressUpdate, IndexedConfigSignatures, SyntaxHelperDatabase,
    SyntaxHelperLoader,
};
use crate::system::parser_coordinator::ParserCoordinator;
use crate::system::DiskCacheKey;
use bsl_shared::api::StartupProgressDto;

use super::coordinator::SystemCoordinator;
use super::config_loader::ConfigCombinedCacheMeta;
use super::types::StartupError;

#[derive(Debug, Clone)]
struct PlatformCacheMeta {
    source_identity: String,
    source_fingerprint: String,
    settings_fingerprint: String,
}

#[derive(Debug, Clone)]
struct SyntaxHelperLoadResult {
    database: SyntaxHelperDatabase,
    cache_meta: Option<PlatformCacheMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyntaxHelperCachePayload {
    database: SyntaxHelperDatabase,
    parse_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CombinedCachePayload {
    config_raw_types: Vec<RawTypeData>,
    config_indexed: IndexedConfigSignatures,
}

impl SystemCoordinator {
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
        info!("Starting platform types parser in blocking thread...");

        // Делегируем синхронной версии через spawn_blocking для предотвращения блокировки event loop
        let coordinator = self.clone_for_blocking();
        let syntax_path_owned = syntax_helper_path.map(|p| p.to_path_buf());
        let config_path_owned = config_path.map(|p| p.to_path_buf());

        let parser_handle = tokio::task::spawn_blocking(move || {
            info!("[BLOCKING THREAD] Parser started");
            let result = coordinator.start_with_paths_blocking(
                syntax_path_owned.as_deref(),
                config_path_owned.as_deref(),
                progress_tx,
            );
            info!("[BLOCKING THREAD] Parser finished");
            result
        });

        info!("Parser spawned in blocking thread, event loop remains free");

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

        info!("[BLOCKING THREAD] SystemCoordinator: инициализация System Layer...");
        self.set_startup_progress(StartupProgressDto {
            phase: "Инициализация".to_string(),
            message: Some("Старт системы".to_string()),
            ..StartupProgressDto::default()
        });

        // КРИТИЧЕСКИ ВАЖНО: Очищаем кеши при повторной инициализации
        // Это гарантирует, что TypeSystemService получит НОВЫЙ AnalysisEngine с НОВЫМ TypeRepository
        // Соблюдаем lock order convention: analysis_engine_cache -> type_service_cache
        {
            let mut engine_cache = self.analysis_engine_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("Analysis engine cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            let mut service_cache = self.type_service_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("Type service cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });

            if engine_cache.is_some() || service_cache.is_some() {
                info!("[BLOCKING THREAD] Очищаем кеши AnalysisEngine и TypeSystemService для повторной инициализации");
                *engine_cache = None;
                *service_cache = None;
            }
        }

        // === PHASE 3: Infrastructure инициализация в SystemCoordinator ===

        // 1. Создаем Infrastructure компоненты (Data Layer)
        info!("SystemCoordinator: инициализация Data Layer loaders...");

        // 2. Загружаем синтаксис-помощник если путь указан
        let syntax_result = if let Some(syntax_path) = syntax_helper_path {
            self.set_startup_progress(StartupProgressDto {
                phase: "Загрузка Syntax Helper".to_string(),
                message: Some(format!("Путь: {}", syntax_path.display())),
                ..self.startup_progress()
            });
            self.load_syntax_helper(syntax_path, &progress_tx)?
        } else {
            SyntaxHelperLoadResult {
                database: SyntaxHelperDatabase::default(),
                cache_meta: None,
            }
        };

        // 3. Создаем Domain Layer компоненты
        info!("SystemCoordinator: инициализация Domain Layer...");
        let repository = Arc::new(InMemoryTypeRepository::new());

        // 4. Загружаем данные в репозиторий (через Adapters)
        let _platform_raw_data = if !syntax_result.database.nodes.is_empty() {
            self.populate_repository_from_syntax_helper(
                &repository,
                syntax_result.database,
                syntax_result.cache_meta.as_ref(),
            )?
        } else {
            // Загружаем базовые типы как fallback
            Self::load_fallback_types(&repository)?
        };

        // 5. Создаем Domain resolver
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        // MILESTONE 3.17: Обновляем ParserCoordinator с TypeResolver для резолюции active_facet
        {
            let new_parser = Arc::new(
                ParserCoordinator::new_with_resolver(repository.clone(), resolver.clone())
                    .with_disk_cache(self.disk_cache()),
            );
            let mut parser_guard = self.parser.write().unwrap_or_else(|poisoned| {
                warn!("Parser RwLock poisoned (write), recovering data.");
                poisoned.into_inner()
            });
            *parser_guard = new_parser;
            info!("ParserCoordinator обновлён с TypeResolver для Milestone 3.17");
        }

        // 6. Создаем упрощенный AnalysisEngine (без Infrastructure зависимостей)
        let analysis_engine = AnalysisEngine::new(resolver, repository.clone());

        // Кешируем AnalysisEngine
        {
            let mut cache = self.analysis_engine_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("Analysis engine cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            *cache = Some(Arc::new(analysis_engine));
        }

        // 7. Загружаем метаданные конфигурации если путь указан
        if let Some(config_path) = config_path {
            info!(
                "Загружаем метаданные конфигурации: {}",
                config_path.display()
            );

            self.set_startup_progress(StartupProgressDto {
                phase: "Загрузка конфигурации".to_string(),
                message: Some(format!("Путь: {}", config_path.display())),
                ..self.startup_progress()
            });

            // Всегда используем версию с прогрессом, чтобы:
            // - Web API мог показывать честный прогресс старта через /api/startup/progress
            // - LSP мог проксировать updates через progress_tx (если передан)
            let tx_opt = progress_tx.clone();
            let coordinator_for_progress = self.clone_for_blocking();
            let mut combined_cache_hit = false;
            let mut combined_cache_key = None;

            if let Some(platform_meta) = syntax_result.cache_meta.as_ref() {
                match self.build_config_combined_cache_meta(config_path) {
                    Ok(config_meta) => {
                        let parser_guard = self.parser.read().unwrap_or_else(|poisoned| {
                            warn!("Parser RwLock poisoned (read), recovering data.");
                            poisoned.into_inner()
                        });
                        parser_guard.set_cache_scope(
                            Some(config_meta.project_id.clone()),
                            Some(config_meta.config_set_id.clone()),
                        );
                        let key = self.build_combined_cache_key(platform_meta, &config_meta);
                        combined_cache_key = Some(key.clone());
                        let cache = self.disk_cache();
                        match cache.try_get::<CombinedCachePayload>(&key) {
                            Ok(Some(payload)) => {
                                info!("Используем combined cache конфигурации");
                                Self::apply_combined_config_payload(&repository, &payload)?;
                                combined_cache_hit = true;
                                let prev = self.startup_progress();
                                self.set_startup_progress(StartupProgressDto {
                                    phase: "Загрузка конфигурации".to_string(),
                                    current: 1,
                                    total: 1,
                                    percentage: 100.0,
                                    message: Some("Конфигурация: из combined cache".to_string()),
                                    done: false,
                                    ..prev
                                });
                                info!(
                                    "Загружено {} типов из combined cache",
                                    payload.config_raw_types.len()
                                );
                            }
                            Ok(None) => {}
                            Err(e) => {
                                warn!("Ошибка чтения combined cache: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Не удалось подготовить combined cache meta: {}", e);
                    }
                }
            }

            if !combined_cache_hit {
                let progress_callback = move |update: ProgressUpdate| {
                    if let Some(ref tx) = tx_opt {
                        let _ = tx.send(update.clone());
                    }
                    // Пишем прогресс в shared storage для Web API
                    // (проценты из loader’ов уже монотонны, а set_startup_progress зажимает назад).
                    let progress = StartupProgressDto {
                        phase: update.phase.display_name().to_string(),
                        current: update.current as u64,
                        total: update.total as u64,
                        percentage: update.percentage,
                        message: update.message.clone(),
                        done: false,
                    };
                    coordinator_for_progress.set_startup_progress(progress);
                };

                let result = if combined_cache_key.is_some() {
                    self.load_all_configurations_with_progress_collect(
                        config_path,
                        progress_callback,
                    )
                    .map(|(result, payload)| (result, Some(payload)))
                } else {
                    self.load_all_configurations_with_progress(
                        config_path,
                        progress_callback,
                    )
                    .map(|result| (result, None))
                };

                match result {
                    Ok((result_data, payload)) => {
                        info!(
                            "Загружено {} типов из {} базовых конфигураций и {} расширений",
                            result_data.total_types,
                            result_data.base_config_count,
                            result_data.extensions_count
                        );

                        if let (Some(key), Some(payload)) = (combined_cache_key, payload) {
                            if let Some(_platform_meta) = syntax_result.cache_meta.as_ref() {
                                let combined_payload = CombinedCachePayload {
                                    config_raw_types: payload.raw_types,
                                    config_indexed: payload.indexed,
                                };
                                let cache = self.disk_cache();
                                let _ = cache.get_or_build_with(
                                    &key,
                                    || Ok(combined_payload),
                                    |payload| !payload.config_raw_types.is_empty(),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Ошибка загрузки метаданных конфигурации: {}", e);
                        info!("Продолжаем работу с типами платформы...");
                    }
                }
            }
        }

        info!("[BLOCKING THREAD] SystemCoordinator: прогрев кеша...");
        self.cache.warm_cache()?;

        self.set_startup_progress(StartupProgressDto {
            phase: "Готово".to_string(),
            current: 1,
            total: 1,
            percentage: 100.0,
            message: Some("Система готова".to_string()),
            done: true,
        });

        info!("[BLOCKING THREAD] SystemCoordinator: система готова!");
        Ok(())
    }

    /// Загрузка синтаксис-помощника с HBK recovery
    fn load_syntax_helper(
        &self,
        syntax_path: &Path,
        progress_tx: &Option<mpsc::UnboundedSender<ProgressUpdate>>,
    ) -> Result<SyntaxHelperLoadResult, StartupError> {
        let syntax_parser = SyntaxHelperLoader::new();

        // HBK Recovery: Восстанавливаем .hbk файлы перед парсингом
        info!("Проверяем наличие .hbk файлов для восстановления...");

        // PHASE 2: Интеграция Progress Callback для HBK Recovery
        let hbk_result = if let Some(ref tx) = progress_tx {
            let tx_clone = tx.clone();
            hbk_recovery::auto_recover_directory_with_progress(
                syntax_path,
                Some(
                    move |update: crate::data::loaders::progress::ProgressUpdateType| {
                        if let crate::data::loaders::progress::ProgressUpdateType::HbkExtraction {
                            file_name,
                            extracted_files,
                            total_files,
                            percentage,
                        } = update
                        {
                            let _ = tx_clone.send(crate::data::loaders::progress::ProgressUpdate {
                                phase: crate::data::loaders::progress::IndexingPhase::HbkExtraction,
                                current: extracted_files,
                                total: total_files,
                                percentage: percentage as f32,
                                message: Some(format!("Извлекаем файлы из {}", file_name)),
                            });
                        }
                    },
                ),
            )
        } else {
            hbk_recovery::auto_recover_directory(syntax_path)
        };

        match hbk_result {
            Ok(results) if !results.is_empty() => {
                info!("Восстановлено {} .hbk файлов", results.len());
                for result in &results {
                    info!(
                        "   Файл: {:?} -> {:?}",
                        result.repaired_zip_path.file_name().unwrap_or_default(),
                        result
                            .extracted_dir
                            .as_ref()
                            .map(|d| d.file_name().unwrap_or_default())
                    );
                }
            }
            Ok(_) => {
                info!(".hbk файлы не найдены (возможно уже распакованы)");
            }
            Err(e) => {
                warn!(
                    "Ошибка восстановления .hbk файлов: {}. Продолжаем с существующими файлами...",
                    e
                );
            }
        }

        info!("Загружаем синтаксис-помощник: {}", syntax_path.display());

        let cache_key = self.build_syntax_helper_cache_key(syntax_path, &syntax_parser)?;
        let cache = self.disk_cache();
        let syntax_path = syntax_path.to_path_buf();
        let progress_tx = progress_tx.clone();
        let entry = cache
            .get_or_build_with_swr(
                &cache_key,
                move || {
                    let mut parse_ok = true;
                    let mut syntax_parser = SyntaxHelperLoader::new();
                    // MILESTONE 2.20.2.3: Парсим с прогрессом если передан callback
                    if let Some(ref tx) = progress_tx {
                        let tx_clone = tx.clone();
                        match syntax_parser.parse_with_progress(
                            &syntax_path,
                            move |update: ProgressUpdate| {
                                let _ = tx_clone.send(update); // Отправляем в channel
                            },
                        ) {
                            Ok(()) => {
                                info!("Парсинг синтаксис-помощника завершен успешно");
                            }
                            Err(e) => {
                                warn!("Ошибка парсинга синтаксис-помощника: {}", e);
                                info!("Будем использовать базовые типы платформы 1С...");
                                parse_ok = false;
                            }
                        }
                    } else {
                        // Обратная совместимость: парсим без прогресса
                        match syntax_parser.parse_syntax_helper(&syntax_path) {
                            Ok(()) => {
                                info!("Парсинг синтаксис-помощника завершен успешно");
                            }
                            Err(e) => {
                                warn!("Ошибка парсинга синтаксис-помощника: {}", e);
                                info!("Будем использовать базовые типы платформы 1С...");
                                parse_ok = false;
                            }
                        }
                    }

                    Ok(SyntaxHelperCachePayload {
                        database: syntax_parser.export_database(),
                        parse_ok,
                    })
                },
                |payload| payload.parse_ok && !payload.database.nodes.is_empty(),
            )
            .map_err(StartupError::PlatformTypesError)?;

        if entry.from_cache {
            info!("Используем кэш синтаксис-помощника");
        }

        let cache_meta = PlatformCacheMeta {
            source_identity: cache_key.source_identity.clone(),
            source_fingerprint: cache_key.source_fingerprint.clone(),
            settings_fingerprint: cache_key.settings_fingerprint.clone(),
        };

        Ok(SyntaxHelperLoadResult {
            database: entry.value.database,
            cache_meta: Some(cache_meta),
        })
    }

    fn build_syntax_helper_cache_key(
        &self,
        syntax_path: &Path,
        syntax_parser: &SyntaxHelperLoader,
    ) -> Result<DiskCacheKey, StartupError> {
        let canonical = fs::canonicalize(syntax_path).unwrap_or_else(|_| syntax_path.to_path_buf());
        let source_identity = canonical.to_string_lossy().to_string();
        let source_fingerprint =
            syntax_helper_fingerprint(syntax_parser, syntax_path)
                .map_err(StartupError::PlatformTypesError)?;
        let settings_fingerprint = syntax_helper_settings_fingerprint(syntax_parser);
        let key_hash = blake3::hash(
            format!(
                "{}|{}|{}",
                source_identity, source_fingerprint, settings_fingerprint
            )
            .as_bytes(),
        )
        .to_hex()
        .to_string();

        Ok(DiskCacheKey::new(
            "platform",
            key_hash,
            source_identity,
            source_fingerprint,
            settings_fingerprint,
        ))
    }

    fn build_platform_raw_cache_key(&self, meta: &PlatformCacheMeta) -> DiskCacheKey {
        let settings_fingerprint = format!("{};raw_v1", meta.settings_fingerprint);
        let key_hash = blake3::hash(
            format!(
                "{}|{}|{}",
                meta.source_identity, meta.source_fingerprint, settings_fingerprint
            )
            .as_bytes(),
        )
        .to_hex()
        .to_string();

        DiskCacheKey::new(
            "platform_raw",
            key_hash,
            meta.source_identity.clone(),
            meta.source_fingerprint.clone(),
            settings_fingerprint,
        )
    }

    fn build_combined_cache_key(
        &self,
        platform_meta: &PlatformCacheMeta,
        config_meta: &ConfigCombinedCacheMeta,
    ) -> DiskCacheKey {
        let source_identity = format!(
            "{}||{}",
            platform_meta.source_identity, config_meta.source_identity
        );
        let source_fingerprint = format!(
            "{}||{}",
            platform_meta.source_fingerprint, config_meta.source_fingerprint
        );
        let settings_fingerprint = format!(
            "{}||{}",
            platform_meta.settings_fingerprint, config_meta.settings_fingerprint
        );
        let key_hash = blake3::hash(
            format!(
                "{}|{}|{}",
                source_identity, source_fingerprint, settings_fingerprint
            )
            .as_bytes(),
        )
        .to_hex()
        .to_string();

        DiskCacheKey::new(
            "combined",
            key_hash,
            source_identity,
            source_fingerprint,
            settings_fingerprint,
        )
        .with_project_id(config_meta.project_id.clone())
        .with_config_id(config_meta.config_set_id.clone())
    }

    /// Заполнение репозитория из синтаксис-помощника
    fn populate_repository_from_syntax_helper(
        &self,
        repository: &Arc<InMemoryTypeRepository>,
        database: crate::data::loaders::syntax_helper::SyntaxHelperDatabase,
        cache_meta: Option<&PlatformCacheMeta>,
    ) -> Result<Vec<RawTypeData>, StartupError> {
        let platform_raw_data = if let Some(meta) = cache_meta {
            let cache_key = self.build_platform_raw_cache_key(meta);
            let cache = self.disk_cache();
            let database_for_build = database.clone();
            let entry = cache
                .get_or_build_with_swr(
                    &cache_key,
                    move || Ok(convert_syntax_helper_to_raw(&database_for_build)),
                    |types| !types.is_empty(),
                )
                .map_err(StartupError::PlatformTypesError)?;
            if entry.from_cache {
                info!("Используем кэш platform raw types");
            }
            entry.value
        } else {
            convert_syntax_helper_to_raw(&database)
        };

        // MILESTONE 2.20.5: Заполняем SignatureIndex из загруженных типов
        let platform_types_clone = platform_raw_data.clone(); // Клонируем для SignatureIndex

        repository
            .load_types(platform_raw_data.clone())
            .map_err(StartupError::PlatformTypesError)?;
        repository.set_platform_docs_loaded(true);

        // Заполняем SignatureIndex через Registry паттерн
        // Единственный источник данных - syntax_helper (документация 1С)
        use crate::data::loaders::{apply_generic_info_to_repository, SyntaxHelperSource};
        use bsl_shared::domain::SignatureSourceRegistry;

        let index = SignatureSourceRegistry::new()
            .register(SyntaxHelperSource::new(platform_types_clone))
            .build();
        repository.set_signature_index(index);

        let global_function_signatures =
            convert_syntax_helper_global_functions(&database);
        if !global_function_signatures.is_empty() {
            let count = global_function_signatures.len();
            for signature in global_function_signatures {
                let name = signature.name.clone();
                repository.add_global_function_signature(&name, signature);
            }
            info!(
                "SignatureIndex заполнен {} глобальными функциями",
                count
            );
        }

        // Milestone 3.x: Применяем GenericInfo для типов-коллекций (inference rules)
        let generic_count = apply_generic_info_to_repository(repository.as_ref());

        let stats = repository.get_stats();
        info!(
            "Загружено {} типов из синтаксис-помощника",
            stats.total_types
        );
        info!("SignatureIndex заполнен платформенными методами");
        info!("GenericInfo применён к {} типам-коллекциям", generic_count);

        Ok(platform_raw_data)
    }

    fn apply_combined_config_payload(
        repository: &Arc<InMemoryTypeRepository>,
        payload: &CombinedCachePayload,
    ) -> Result<(), StartupError> {
        for (owner_type, sig) in &payload.config_indexed.config_methods {
            repository.add_config_method_signature(owner_type, sig.clone());
        }
        for (name, sig) in &payload.config_indexed.global_functions {
            repository.add_global_function_signature(name, sig.clone());
        }
        for (owner_type, method_name, location) in &payload.config_indexed.definition_locations {
            repository.add_config_method_definition_location(
                owner_type,
                method_name,
                location.clone(),
            );
        }
        for (function_name, location) in &payload.config_indexed.global_definition_locations {
            repository.add_global_function_definition_location(
                function_name,
                location.clone(),
            );
        }

        repository
            .load_types(payload.config_raw_types.clone())
            .map_err(StartupError::PlatformTypesError)?;

        Ok(())
    }

    /// Загрузка базовых типов как fallback
    ///
    /// Используется когда syntax_helper не доступен.
    /// Загружает только примитивные типы и типы-коллекции без методов.
    /// Методы будут недоступны, но GenericInfo для inference будет работать.
    pub(crate) fn load_fallback_types(
        repository: &Arc<InMemoryTypeRepository>,
    ) -> Result<Vec<RawTypeData>, StartupError> {
        info!("Загружаем базовые типы платформы 1С (fallback mode)...");
        repository.set_platform_docs_loaded(false);

        // Примитивные типы
        let platform_types = vec![
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
            // Типы-коллекции (без методов, только для GenericInfo)
            RawTypeData {
                name: "Массив".to_string(),
                english_name: "Array".to_string(),
                description: "Динамический массив".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Соответствие".to_string(),
                english_name: "Map".to_string(),
                description: "Ассоциативный массив (ключ-значение)".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "СписокЗначений".to_string(),
                english_name: "ValueList".to_string(),
                description: "Список значений с представлениями".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "ТабличнаяЧасть".to_string(),
                english_name: "TabularSection".to_string(),
                description: "Табличная часть объекта".to_string(),
                category: "Универсальные коллекции значений".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ];

        let platform_types_clone = platform_types.clone();
        let type_count = platform_types.len();

        repository
            .load_types(platform_types.clone())
            .map_err(StartupError::PlatformTypesError)?;

        // Заполняем SignatureIndex (будет пустой в fallback mode)
        use crate::data::loaders::{apply_generic_info_to_repository, SyntaxHelperSource};
        use bsl_shared::domain::SignatureSourceRegistry;

        let index = SignatureSourceRegistry::new()
            .register(SyntaxHelperSource::new(platform_types_clone))
            .build();
        repository.set_signature_index(index);

        // Применяем GenericInfo для типов-коллекций
        let generic_count = apply_generic_info_to_repository(repository.as_ref());

        info!(
            "Базовые типы загружены: {} типов (fallback mode)",
            type_count
        );
        info!("GenericInfo применён к {} типам-коллекциям", generic_count);
        warn!("Методы недоступны в fallback mode. Укажите путь к syntax_helper для полной функциональности.");
        Ok(platform_types)
    }
}

fn syntax_helper_fingerprint(
    syntax_parser: &SyntaxHelperLoader,
    syntax_path: &Path,
) -> anyhow::Result<String> {
    let mut roots = Vec::new();
    let context_help_path = syntax_path.join("rebuilt.shcntx_ru");
    let language_help_path = syntax_path.join("rebuilt.shlang_ru");

    if context_help_path.exists() {
        roots.push(context_help_path);
    }
    if language_help_path.exists() {
        roots.push(language_help_path);
    }
    if roots.is_empty() && syntax_path.exists() {
        roots.push(syntax_path.to_path_buf());
    }

    let mut files: Vec<PathBuf> = Vec::new();
    for root in roots {
        files.extend(syntax_parser.collect_html_files(&root)?);
    }
    files.sort();

    let mut hasher = blake3::Hasher::new();
    for path in files {
        let path_str = path.to_string_lossy();
        hasher.update(path_str.as_bytes());
        if let Ok(metadata) = fs::metadata(&path) {
            hasher.update(&metadata.len().to_le_bytes());
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            hasher.update(&modified.to_le_bytes());
        }
        if std::env::var("BSL_CACHE_STRICT_FINGERPRINT").is_ok() {
            if let Ok(contents) = fs::read(&path) {
                let content_hash = blake3::hash(&contents);
                hasher.update(content_hash.as_bytes());
            }
        }
    }

    Ok(hasher.finalize().to_hex().to_string())
}

fn syntax_helper_settings_fingerprint(syntax_parser: &SyntaxHelperLoader) -> String {
    let settings = &syntax_parser.settings;
    let strict = std::env::var("BSL_CACHE_STRICT_FINGERPRINT").is_ok();
    format!(
        "syntax_helper_parser_v1;threads={:?};batch={};show={};limit={:?};skip={:?};parallel={};strict_fingerprint={}",
        settings.max_threads,
        settings.batch_size,
        settings.show_progress,
        settings.file_limit,
        settings.skip_dirs,
        settings.parallel_indexing,
        strict
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }

        fn remove(key: &'static str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::remove_var(key);
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
        }
    }

    #[test]
    fn test_syntax_helper_disk_cache_reuse() {
        let syntax_path = Path::new("examples/syntax_helper");
        if !syntax_path.exists() {
            eprintln!("⚠️ Syntax Helper не найден в examples/syntax_helper");
            return;
        }

        let temp = TempDir::new().unwrap();
        let cache = crate::system::DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();
        let builds = AtomicUsize::new(0);
        let coordinator = SystemCoordinator::new();

        let mut parser = SyntaxHelperLoader::new();
        let key = coordinator
            .build_syntax_helper_cache_key(syntax_path, &parser)
            .unwrap();
        let entry = cache
            .get_or_build_with(
                &key,
                || {
                    builds.fetch_add(1, Ordering::SeqCst);
                    parser.parse_syntax_helper(syntax_path)?;
                    Ok(SyntaxHelperCachePayload {
                        database: parser.export_database(),
                        parse_ok: true,
                    })
                },
                |payload| payload.parse_ok && !payload.database.nodes.is_empty(),
            )
            .unwrap();
        assert!(!entry.from_cache);

        let mut parser = SyntaxHelperLoader::new();
        let key = coordinator
            .build_syntax_helper_cache_key(syntax_path, &parser)
            .unwrap();
        let entry = cache
            .get_or_build_with(
                &key,
                || {
                    builds.fetch_add(1, Ordering::SeqCst);
                    parser.parse_syntax_helper(syntax_path)?;
                    Ok(SyntaxHelperCachePayload {
                        database: parser.export_database(),
                        parse_ok: true,
                    })
                },
                |payload| payload.parse_ok && !payload.database.nodes.is_empty(),
            )
            .unwrap();
        assert!(entry.from_cache);
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_platform_raw_cache_produces_signature_index() {
        let syntax_path = Path::new("examples/syntax_helper");
        if !syntax_path.exists() {
            eprintln!("⚠️ Syntax Helper не найден в examples/syntax_helper");
            return;
        }

        let temp = TempDir::new().unwrap();
        let _cache_dir_guard = EnvGuard::set("BSL_CACHE_DIR", temp.path());
        let _cache_disable_guard = EnvGuard::remove("BSL_CACHE_DISABLE");

        let coordinator = SystemCoordinator::new();
        let load_result = coordinator.load_syntax_helper(syntax_path, &None).unwrap();

        let repository = Arc::new(InMemoryTypeRepository::new());
        coordinator
            .populate_repository_from_syntax_helper(
                &repository,
                load_result.database,
                load_result.cache_meta.as_ref(),
            )
            .unwrap();

        let index = repository.get_signature_index_clone();
        let methods = index.get_type_methods("Массив");
        assert!(
            methods.iter().any(|method| method.name == "Добавить"),
            "Ожидали метод Массив.Добавить в SignatureIndex"
        );
    }

    #[test]
    fn test_combined_cache_roundtrip() {
        let syntax_path = Path::new("examples/syntax_helper");
        let config_path = Path::new("examples/conf/conf_test");
        if !syntax_path.exists() || !config_path.exists() {
            eprintln!("⚠️ Не найдены примеры syntax_helper или конфигурации");
            return;
        }

        let temp = TempDir::new().unwrap();
        let _cache_dir_guard = EnvGuard::set("BSL_CACHE_DIR", temp.path());
        let _cache_disable_guard = EnvGuard::remove("BSL_CACHE_DISABLE");

        let coordinator = SystemCoordinator::new();
        let load_result = coordinator
            .load_syntax_helper(syntax_path, &None)
            .unwrap();
        let platform_meta = match load_result.cache_meta.as_ref() {
            Some(meta) => meta.clone(),
            None => return,
        };

        coordinator
            .start_with_paths_blocking(Some(syntax_path), Some(config_path), None)
            .unwrap();

        let config_meta = coordinator
            .build_config_combined_cache_meta(config_path)
            .unwrap();
        let key = coordinator.build_combined_cache_key(&platform_meta, &config_meta);
        let cache = coordinator.disk_cache();
        let cached = cache.try_get::<CombinedCachePayload>(&key).unwrap();
        assert!(cached.is_some(), "Combined cache entry отсутствует");
    }
}
