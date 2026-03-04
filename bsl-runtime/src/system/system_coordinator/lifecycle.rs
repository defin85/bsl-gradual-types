//! Lifecycle management для SystemCoordinator
//!
//! Инициализация системы, загрузка типов платформы

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use anyhow::anyhow;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{RawDataSource, RawTypeData};
use serde::{Deserialize, Serialize};

use crate::data::adapters::{convert_syntax_helper_global_functions, convert_syntax_helper_to_raw};
use crate::data::loaders::config_metadata_parser::ConfigurationDiscovery;
use crate::data::loaders::{
    hbk_recovery, progress::ProgressUpdate, IndexedConfigSignatures, OptimizationSettings,
    SyntaxHelperDatabase, SyntaxHelperLoader,
};
use crate::system::keyword_index::keyword_items_from_syntax_or_default;
use crate::system::parser_coordinator::ParserCoordinator;
use crate::system::platform_version::{
    format_platform_version, parse_platform_version, PlatformVersion,
};
use crate::system::DiskCacheKey;
use crate::system::{IndexItem, IndexItemKind, IndexKind, TypeKind};
use bsl_shared::api::StartupProgressDto;

use super::config_loader::ConfigCombinedCacheMeta;
use super::coordinator::SystemCoordinator;
use super::types::{DomainBundle, StartupError};

#[path = "lifecycle/helpers.rs"]
mod helpers;

use self::helpers::{syntax_helper_fingerprint, syntax_helper_settings_fingerprint};

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
        self.start_with_paths(None, None, None, None).await
    }

    /// Инициализация системы с настраиваемыми путями (async версия)
    ///
    /// ВНИМАНИЕ: Эта функция выполняется в основном tokio event loop.
    /// Для CPU-intensive парсинга используйте start_with_paths_blocking() через spawn_blocking()
    pub async fn start_with_paths(
        &self,
        syntax_helper_path: Option<&Path>,
        config_path: Option<&Path>,
        platform_version: Option<&str>,
        progress_tx: Option<mpsc::UnboundedSender<ProgressUpdate>>,
    ) -> Result<(), StartupError> {
        info!("Starting platform types parser in blocking thread...");

        // Делегируем синхронной версии через spawn_blocking для предотвращения блокировки event loop
        let coordinator = self.clone_for_blocking();
        let syntax_path_owned = syntax_helper_path.map(|p| p.to_path_buf());
        let config_path_owned = config_path.map(|p| p.to_path_buf());

        let platform_version_owned = platform_version.map(str::to_string);
        let parser_handle = tokio::task::spawn_blocking(move || {
            info!("[BLOCKING THREAD] Parser started");
            let result = coordinator.start_with_paths_blocking(
                syntax_path_owned.as_deref(),
                config_path_owned.as_deref(),
                platform_version_owned.as_deref(),
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
        platform_version: Option<&str>,
        progress_tx: Option<mpsc::UnboundedSender<ProgressUpdate>>,
    ) -> Result<(), StartupError> {
        self.observability.log_startup();

        info!("[BLOCKING THREAD] SystemCoordinator: инициализация System Layer...");
        self.set_startup_progress(StartupProgressDto {
            phase: "Инициализация".to_string(),
            message: Some("Старт системы".to_string()),
            ..StartupProgressDto::default()
        });

        // КРИТИЧЕСКИ ВАЖНО: Очищаем кеш при повторной инициализации
        // Это гарантирует, что новая инициализация получит новый DomainBundle с новым TypeRepository
        {
            let mut engine_cache = self.domain_bundle_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("Domain bundle cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });

            if engine_cache.is_some() {
                info!("[BLOCKING THREAD] Очищаем кеш DomainBundle для повторной инициализации");
                *engine_cache = None;
            }
        }

        let required_platform_version = if let Some(path) = config_path {
            Some(self.required_platform_version(path)?)
        } else {
            None
        };

        let parsed_platform_version = if let Some(version) = platform_version {
            parse_platform_version(version).ok_or_else(|| {
                StartupError::PlatformTypesError(anyhow!(
                    "Некорректная версия платформы: {}",
                    version
                ))
            })?
        } else {
            PlatformVersion::default()
        };

        if let Some(required) = required_platform_version {
            if platform_version.is_none() {
                return Err(StartupError::PlatformTypesError(anyhow!(
                    "platform_version обязателен при загрузке конфигурации (CompatibilityMode={})",
                    required
                )));
            }
            if parsed_platform_version < required {
                return Err(StartupError::PlatformTypesError(anyhow!(
                    "Версия платформы {} ниже CompatibilityMode {}",
                    parsed_platform_version,
                    required
                )));
            }
        }

        let normalized_platform_version =
            platform_version.map(|_| format_platform_version(parsed_platform_version));

        if syntax_helper_path.is_some() {
            self.intellisense_index.invalidate_platform_types();
        }
        self.set_platform_version(normalized_platform_version.clone());

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
            self.load_syntax_helper(
                syntax_path,
                normalized_platform_version.as_deref(),
                &progress_tx,
            )?
        } else {
            SyntaxHelperLoadResult {
                database: SyntaxHelperDatabase::default(),
                cache_meta: None,
            }
        };

        let keyword_items = keyword_items_from_syntax_or_default(&syntax_result.database.keywords);
        if !keyword_items.is_empty() {
            self.intellisense_index.set_keywords(keyword_items);
        }

        // 3. Создаем Domain Layer компоненты
        info!("SystemCoordinator: инициализация Domain Layer...");
        let repository = Arc::new(InMemoryTypeRepository::new());

        // 4. Загружаем данные в репозиторий (через Adapters)
        let platform_raw_data = if !syntax_result.database.nodes.is_empty() {
            self.populate_repository_from_syntax_helper(
                &repository,
                syntax_result.database,
                syntax_result.cache_meta.as_ref(),
            )?
        } else {
            // Загружаем базовые типы как fallback
            Self::load_fallback_types(&repository)?
        };

        if !platform_raw_data.is_empty() {
            let mut type_items: Vec<IndexItem> = Vec::new();
            for raw_type in &platform_raw_data {
                if raw_type.source != RawDataSource::Platform {
                    continue;
                }
                let mut item = IndexItem::new(
                    raw_type.name.clone(),
                    IndexItemKind::Type(TypeKind::from_raw_source(&raw_type.source)),
                    IndexKind::Type,
                );
                item.facets = raw_type.facets.clone();
                type_items.push(item);
            }
            if !type_items.is_empty() {
                self.intellisense_index.upsert_types(type_items);
            }
        }

        // 5. Создаем Domain resolver
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        // MILESTONE 3.17: Обновляем ParserCoordinator с TypeResolver для резолюции active_facet
        {
            let new_parser_instance =
                ParserCoordinator::new_with_resolver(repository.clone(), resolver.clone())
                    .with_disk_cache(self.disk_cache());
            new_parser_instance.set_intellisense_index(self.intellisense_index.clone());
            let new_parser = Arc::new(new_parser_instance);
            let mut parser_guard = self.parser.write().unwrap_or_else(|poisoned| {
                warn!("Parser RwLock poisoned (write), recovering data.");
                poisoned.into_inner()
            });
            *parser_guard = new_parser;
            info!("ParserCoordinator обновлён с TypeResolver для Milestone 3.17");
        }

        // 6. Кешируем Domain layer bundle (repository + resolver)
        {
            let mut cache = self.domain_bundle_cache.write()
                .unwrap_or_else(|poisoned| {
                    warn!("Domain bundle cache RwLock poisoned (write), recovering data. This indicates a panic in another thread.");
                    poisoned.into_inner()
                });
            *cache = Some(Arc::new(DomainBundle {
                repository: repository.clone(),
                resolver: resolver.clone(),
            }));
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
                                let platform_version = self
                                    .platform_version()
                                    .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
                                self.update_intellisense_index_from_config_raw_types(
                                    &config_meta.source_fingerprint,
                                    &platform_version,
                                    &payload.config_raw_types,
                                );
                                combined_cache_hit = true;
                                self.set_startup_progress(StartupProgressDto {
                                    phase: "Загрузка конфигурации".to_string(),
                                    current: 1,
                                    total: 1,
                                    percentage: 100.0,
                                    message: Some("Конфигурация: из combined cache".to_string()),
                                    done: false,
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
                    self.load_all_configurations_with_progress(config_path, progress_callback)
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
        platform_version: Option<&str>,
        progress_tx: &Option<mpsc::UnboundedSender<ProgressUpdate>>,
    ) -> Result<SyntaxHelperLoadResult, StartupError> {
        let syntax_parser = SyntaxHelperLoader::with_settings(OptimizationSettings {
            collect_keywords: true,
            ..Default::default()
        });

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

        let cache_key =
            self.build_syntax_helper_cache_key(syntax_path, platform_version, &syntax_parser)?;
        let cache = self.disk_cache();
        let syntax_path = syntax_path.to_path_buf();
        let progress_tx = progress_tx.clone();
        let entry = cache
            .get_or_build_with_swr(
                &cache_key,
                move || {
                    let mut parse_ok = true;
                    let mut syntax_parser =
                        SyntaxHelperLoader::with_settings(OptimizationSettings {
                            collect_keywords: true,
                            ..Default::default()
                        });
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

    fn required_platform_version(
        &self,
        config_path: &Path,
    ) -> Result<PlatformVersion, StartupError> {
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), false);
        let configurations = discovery.discover_all_configurations().map_err(|e| {
            StartupError::PlatformTypesError(anyhow!("Не удалось обнаружить конфигурации: {}", e))
        })?;

        if configurations.is_empty() {
            return Err(StartupError::PlatformTypesError(anyhow!(
                "Конфигурации не найдены для определения CompatibilityMode"
            )));
        }

        let mut required: Option<PlatformVersion> = None;
        for config in configurations {
            let raw_mode = config.compatibility_mode.as_deref().ok_or_else(|| {
                StartupError::PlatformTypesError(anyhow!(
                    "CompatibilityMode не найден для конфигурации {}",
                    config.name
                ))
            })?;
            let parsed = parse_platform_version(raw_mode).ok_or_else(|| {
                StartupError::PlatformTypesError(anyhow!(
                    "Некорректный CompatibilityMode '{}' для конфигурации {}",
                    raw_mode,
                    config.name
                ))
            })?;
            required = Some(required.map_or(parsed, |current| current.max(parsed)));
        }

        Ok(required.expect("required platform version"))
    }

    fn build_syntax_helper_cache_key(
        &self,
        syntax_path: &Path,
        platform_version: Option<&str>,
        syntax_parser: &SyntaxHelperLoader,
    ) -> Result<DiskCacheKey, StartupError> {
        let canonical = fs::canonicalize(syntax_path).unwrap_or_else(|_| syntax_path.to_path_buf());
        let source_identity = canonical.to_string_lossy().to_string();
        let strict = self.strict_fingerprint();
        let source_fingerprint = syntax_helper_fingerprint(syntax_parser, syntax_path, strict)
            .map_err(StartupError::PlatformTypesError)?;
        let platform_version = platform_version.unwrap_or("unknown");
        let settings_fingerprint = format!(
            "{};platform_version={}",
            syntax_helper_settings_fingerprint(syntax_parser, strict),
            platform_version
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

        let global_function_signatures = convert_syntax_helper_global_functions(&database);
        if !global_function_signatures.is_empty() {
            let count = global_function_signatures.len();
            for signature in global_function_signatures {
                let name = signature.name.clone();
                repository.add_global_function_signature(&name, signature);
            }
            info!("SignatureIndex заполнен {} глобальными функциями", count);
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
}

#[cfg(test)]
#[path = "lifecycle/tests.rs"]
mod tests;
