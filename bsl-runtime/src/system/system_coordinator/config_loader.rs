//! Загрузка метаданных конфигураций
//!
//! Методы для загрузки метаданных из базовых конфигураций и расширений

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{info, warn};

use crate::data::loaders::progress::{IndexingPhase, ProgressUpdate};
use crate::data::loaders::{
    collect_module_paths, index_configuration_bsl_modules_with_progress_parallel_cached,
    IndexedConfigSignatures, ModuleIndexProgress, ParsedModuleData, UniversalMetadataObject,
};
use bsl_shared::domain::types::{MetadataKind, RawTypeData};
use serde::{Deserialize, Serialize};

use super::coordinator::SystemCoordinator;
use super::types::LoadMetadataResult;
use crate::system::{
    DiskCacheKey, IndexItem, IndexItemKind, IndexKind, IndexSnapshotId, SymbolKind, SymbolScope,
    TypeKind, Visibility,
};

#[derive(Debug, Serialize, Deserialize)]
struct ConfigLayerBCachePayload {
    raw_types: Vec<RawTypeData>,
    indexed: IndexedConfigSignatures,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CombinedConfigCachePayload {
    pub raw_types: Vec<RawTypeData>,
    pub indexed: IndexedConfigSignatures,
}

#[derive(Debug, Clone)]
pub(crate) struct ConfigCombinedCacheMeta {
    pub project_id: String,
    pub config_set_id: String,
    pub source_identity: String,
    pub source_fingerprint: String,
    pub settings_fingerprint: String,
}

impl SystemCoordinator {
    pub fn cache_scope_for_config_path(
        &self,
        config_path: &Path,
    ) -> Result<super::types::CacheScope> {
        use crate::data::loaders::config_metadata_parser::ConfigurationDiscovery;

        let config_root = normalize_config_root(config_path);
        let discovery = ConfigurationDiscovery::new(config_root.clone(), true);
        let configurations = discovery
            .discover_all_configurations()
            .map_err(|e| anyhow::anyhow!("Ошибка обнаружения конфигураций: {}", e))?;
        if configurations.is_empty() {
            return Err(anyhow::anyhow!(
                "Конфигурации не найдены в {}",
                config_root.display()
            ));
        }

        let config_set_id = config_set_id_from_configs(&configurations);
        let project_id = project_id_from_root(&config_root);
        let mut config_ids: Vec<String> = configurations.iter().map(config_id_for_info).collect();
        config_ids.sort();
        config_ids.dedup();

        Ok(super::types::CacheScope {
            project_id,
            config_set_id,
            config_ids,
        })
    }
    fn apply_prefix_for_indexing(
        metadata: &[UniversalMetadataObject],
        prefix: Option<&str>,
    ) -> Vec<UniversalMetadataObject> {
        let Some(prefix) = prefix.filter(|p| !p.is_empty()) else {
            return metadata.to_vec();
        };

        let mut out = metadata.to_vec();
        for obj in &mut out {
            obj.name = format!("{}{}", prefix, obj.name);
        }
        out
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
    /// use bsl_runtime::SystemCoordinator;
    ///
    /// let coordinator = SystemCoordinator::new();
    /// let config_path = Path::new("examples/conf/conf_test");
    /// let loaded = coordinator.load_configuration_metadata(config_path).unwrap();
    /// println!("Загружено {} объектов метаданных", loaded);
    /// ```
    pub fn load_configuration_metadata(&self, config_path: &Path) -> Result<usize> {
        use crate::data::loaders::ConfigurationDiscovery;

        info!("Загрузка метаданных конфигурации из {:?}", config_path);

        // Создаём discovery и обнаруживаем все объекты
        // show_progress = false - нет терминального прогресс-бара для этого метода
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), false);

        let config_info = discover_single_config(&discovery, config_path);
        if let Some(ref config_info) = config_info {
            let project_id = project_id_from_root(config_path);
            let config_set_id = config_set_id_from_single(config_info);
            self.try_warmup_intellisense_index(config_path, &project_id, &config_set_id);
        }

        let metadata_objects = if let Some(ref config_info) = config_info {
            let config_set_id = config_set_id_from_single(config_info);
            let cache_key =
                self.build_config_cache_key(config_path, config_info, Some(&config_set_id))?;
            let cache = self.disk_cache();
            let discovery_root = config_path.to_path_buf();
            let config_info = config_info.clone();
            let entry = cache.get_or_build_with_swr(
                &cache_key,
                move || {
                    let discovery = ConfigurationDiscovery::new(discovery_root, false);
                    // Без progress_callback в публичном методе (для обратной совместимости)
                    discovery
                        .discover_metadata_in_configuration(
                            &config_info,
                            None::<fn(crate::data::loaders::progress::ProgressUpdate)>,
                        )
                        .map_err(|e| anyhow::anyhow!("Не удалось обнаружить метаданные: {}", e))
                },
                |metadata| !metadata.is_empty(),
            )?;
            entry.value
        } else {
            // Без progress_callback в публичном методе (для обратной совместимости)
            discovery
                .discover_all_metadata(None::<fn(crate::data::loaders::progress::ProgressUpdate)>)
                .map_err(|e| anyhow::anyhow!("Не удалось обнаружить метаданные: {}", e))?
        };

        info!("Обнаружено {} объектов метаданных", metadata_objects.len());

        let bundle = self.domain_bundle().ok_or_else(|| {
            anyhow::anyhow!("Domain bundle не инициализирован. Вызовите start() сначала.")
        })?;
        let repository = bundle.repository.clone();

        let mut payload = None;
        let mut metadata_for_indexing = metadata_objects.clone();
        if let Some(ref config_info) = config_info {
            let config_set_id = config_set_id_from_single(config_info);
            let prefix = config_info.prefix.as_deref();
            metadata_for_indexing = Self::apply_prefix_for_indexing(&metadata_objects, prefix);

            let cache_key = self.build_config_layer_b_cache_key(
                config_path,
                config_info,
                Some(&config_set_id),
                &metadata_for_indexing,
            )?;
            let cache = self.disk_cache();
            let coordinator = self.clone_for_blocking();
            let config_path_for_build = config_path.to_path_buf();
            let config_info = config_info.clone();
            let config_set_id = config_set_id.clone();
            let metadata_objects = metadata_objects.clone();
            let metadata_for_indexing = metadata_for_indexing.clone();
            let prefix = prefix.map(str::to_string);
            let entry = cache.get_or_build_with_swr(
                &cache_key,
                move || {
                    coordinator.build_config_layer_b_payload(
                        &config_path_for_build,
                        &config_info,
                        &config_set_id,
                        &metadata_objects,
                        &metadata_for_indexing,
                        prefix.as_deref(),
                        None::<fn(ModuleIndexProgress)>,
                    )
                },
                |payload| !payload.raw_types.is_empty(),
            )?;
            payload = Some(entry.value);
        }

        let payload = match payload {
            Some(payload) => payload,
            None => {
                let indexed = match index_configuration_bsl_modules_with_progress_parallel_cached(
                    config_path,
                    &metadata_objects,
                    None::<fn(ModuleIndexProgress)>,
                    |_| Ok(None),
                    |_, _| Ok(()),
                ) {
                    Ok(indexed) => indexed,
                    Err(e) => {
                        warn!(
                            "Не удалось проиндексировать экспортные методы из *.bsl модулей: {}",
                            e
                        );
                        IndexedConfigSignatures::default()
                    }
                };
                let raw_types: Vec<RawTypeData> = metadata_objects
                    .into_iter()
                    .flat_map(|obj| obj.to_raw_type_data_with_forms(None))
                    .collect();
                ConfigLayerBCachePayload { raw_types, indexed }
            }
        };

        let config_methods_count = payload.indexed.config_methods.len();
        let global_functions_count = payload.indexed.global_functions.len();
        let def_locations_count = payload.indexed.definition_locations.len();
        let global_def_locations_count = payload.indexed.global_definition_locations.len();

        for (owner_type, sig) in payload.indexed.config_methods {
            repository.add_config_method_signature(&owner_type, sig);
        }
        for (name, sig) in payload.indexed.global_functions {
            repository.add_global_function_signature(&name, sig);
        }
        for (owner_type, method_name, location) in payload.indexed.definition_locations {
            repository.add_config_method_definition_location(&owner_type, &method_name, location);
        }
        for (function_name, location) in payload.indexed.global_definition_locations {
            repository.add_global_function_definition_location(&function_name, location);
        }

        info!(
            "Проиндексированы экспортные методы из *.bsl: методов={}, глобальных функций={}, locations={}, global_locations={}",
            config_methods_count,
            global_functions_count,
            def_locations_count,
            global_def_locations_count
        );

        let count = payload.raw_types.len();

        // Загружаем все типы в репозиторий за один вызов
        repository.load_types(payload.raw_types)?;

        info!("Загружено {} типов из конфигурации", count);
        self.update_intellisense_index_from_metadata(
            config_path,
            repository.get_all_types(),
            &metadata_for_indexing,
        );
        let config_root = config_info
            .as_ref()
            .map(|info| info.path.as_path())
            .unwrap_or(config_path);
        self.update_intellisense_index_from_modules(
            config_root,
            &payload.indexed.module_signatures,
        );
        if let Some(ref config_info) = config_info {
            let project_id = project_id_from_root(config_path);
            let config_set_id = config_set_id_from_single(config_info);
            self.persist_intellisense_index_snapshot(&project_id, &config_set_id);
        }
        Ok(count)
    }

    /// Загружает метаданные из ВСЕХ конфигураций с прогрессом (4 фазы парсинга + индексация модулей)
    ///
    /// Новая версия с поддержкой прогресса через callback.
    /// Автоматически обнаруживает все конфигурации в указанной папке:
    /// - Базовую конфигурацию (ObjectBelonging = "Own")
    /// - Расширения конфигурации (ObjectBelonging = "Adopted")
    ///
    /// Для каждой конфигурации отправляет прогресс через 4 фазы парсинга:
    /// - ConfigurationDiscovery (0-5%)
    /// - ConfigurationParsing (5-80%)
    /// - ConfigurationLinking (80-90%)
    /// - ConfigurationFinalizing (90-95%)
    ///   Затем отдельно сообщает прогресс индексации BSL-модулей.
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
        let (result, _payload) = self.load_all_configurations_with_progress_inner(
            config_path,
            progress_callback,
            false,
        )?;
        Ok(result)
    }

    pub(crate) fn load_all_configurations_with_progress_collect<F>(
        &self,
        config_path: &Path,
        progress_callback: F,
    ) -> Result<(LoadMetadataResult, CombinedConfigCachePayload)>
    where
        F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
    {
        let (result, payload) =
            self.load_all_configurations_with_progress_inner(config_path, progress_callback, true)?;
        let payload =
            payload.ok_or_else(|| anyhow::anyhow!("Combined config payload not collected"))?;
        Ok((result, payload))
    }

    fn load_all_configurations_with_progress_inner<F>(
        &self,
        config_path: &Path,
        progress_callback: F,
        collect_payload: bool,
    ) -> Result<(LoadMetadataResult, Option<CombinedConfigCachePayload>)>
    where
        F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
    {
        use crate::data::loaders::config_metadata_parser::{
            ConfigurationDiscovery, ConfigurationType,
        };

        info!(
            "[WITH PROGRESS] Обнаружение конфигураций в: {}",
            config_path.display()
        );

        // show_progress = true - показываем терминальный прогресс-бар для Web Server
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), true);
        let configurations = discovery
            .discover_all_configurations()
            .map_err(|e| anyhow::anyhow!("Ошибка обнаружения конфигураций: {}", e))?;

        info!("Найдено {} конфигураций", configurations.len());

        let mut base_count = 0;
        let mut ext_count = 0;
        let mut total_types = 0;
        let mut all_metadata: Vec<UniversalMetadataObject> = Vec::new();

        let bundle = self.domain_bundle().ok_or_else(|| {
            anyhow::anyhow!("Domain bundle не инициализирован. Вызовите start() сначала.")
        })?;
        let repository = bundle.repository.clone();

        let config_set_id = config_set_id_from_configs(&configurations);
        let project_id = project_id_from_root(config_path);
        self.try_warmup_intellisense_index(config_path, &project_id, &config_set_id);

        let mut combined_payload = if collect_payload {
            Some(CombinedConfigCachePayload {
                raw_types: Vec::new(),
                indexed: IndexedConfigSignatures::default(),
            })
        } else {
            None
        };

        for config_info in configurations {
            info!(
                "Загрузка конфигурации: {} ({}{})",
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

            let cache_key =
                self.build_config_cache_key(config_path, &config_info, Some(&config_set_id))?;
            let cache = self.disk_cache();
            let discovery_root = config_path.to_path_buf();
            let config_info_clone = config_info.clone();
            let progress_callback_clone = progress_callback.clone();
            let entry = cache.get_or_build_with_swr(
                &cache_key,
                move || {
                    let discovery = ConfigurationDiscovery::new(discovery_root, true);
                    // НОВОЕ: Используем новый метод с прогрессом через 4 фазы парсинга
                    discovery
                        .discover_metadata_in_configuration_with_progress(
                            &config_info_clone,
                            progress_callback_clone.clone(),
                        )
                        .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))
                },
                |metadata| !metadata.is_empty(),
            )?;

            if entry.from_cache {
                emit_cached_config_progress(&config_info, &progress_callback);
            }

            let metadata = entry.value;

            let prefix = config_info.prefix.as_deref();
            let metadata_for_indexing = Self::apply_prefix_for_indexing(&metadata, prefix);
            all_metadata.extend(metadata_for_indexing.clone());

            // Для Web API startup progress (P7): отмечаем отдельную стадию индексации модулей BSL.
            let prev = self.startup_progress();
            self.set_startup_progress(bsl_shared::api::StartupProgressDto {
                phase: "Индексация BSL-модулей".to_string(),
                message: Some(format!("Конфигурация: {}", config_info.name)),
                ..prev
            });

            let config_root = config_info.path.clone();
            let config_name = Arc::new(config_info.name.clone());
            let progress_callback_for_modules = progress_callback.clone();
            let terminal_progress: Arc<Mutex<Option<ProgressBar>>> = Arc::new(Mutex::new(None));
            let coordinator_for_progress = self.clone_for_blocking();

            let layer_key = self.build_config_layer_b_cache_key(
                config_path,
                &config_info,
                Some(&config_set_id),
                &metadata_for_indexing,
            )?;
            let cache = self.disk_cache();
            let coordinator = self.clone_for_blocking();
            let config_path_for_build = config_path.to_path_buf();
            let config_info_for_build = config_info.clone();
            let config_set_id = config_set_id.clone();
            let metadata = metadata.clone();
            let metadata_for_indexing = metadata_for_indexing.clone();
            let prefix = prefix.map(str::to_string);
            let entry = cache.get_or_build_with_swr(
                &layer_key,
                move || {
                    let terminal_progress = Arc::clone(&terminal_progress);
                    let config_name = Arc::clone(&config_name);
                    let coordinator_for_progress = coordinator_for_progress.clone();
                    let progress_callback = progress_callback_for_modules;
                    let progress = Some(move |p: ModuleIndexProgress| {
                        let prev = coordinator_for_progress.startup_progress();
                        let module_display = p
                            .module_path
                            .strip_prefix(&config_root)
                            .unwrap_or(&p.module_path)
                            .display()
                            .to_string();
                        if p.current.is_multiple_of(5) || p.current == p.total {
                            let message = format!(
                                "Индексация BSL-модулей: {} {}/{} — {}",
                                config_name.as_str(),
                                p.current,
                                p.total,
                                module_display
                            );
                            progress_callback(ProgressUpdate::new(
                                IndexingPhase::ConfigurationIndexingModules,
                                p.current,
                                p.total,
                                Some(message),
                            ));
                        }
                        coordinator_for_progress.set_startup_progress(bsl_shared::api::StartupProgressDto {
                            phase: "Индексация BSL-модулей".to_string(),
                            current: p.current as u64,
                            total: p.total as u64,
                            percentage: prev.percentage,
                            message: Some(format!(
                                "{}: {}/{} — {}",
                                config_name.as_str(),
                                p.current,
                                p.total,
                                module_display
                            )),
                            done: false,
                        });
                        if let Ok(mut guard) = terminal_progress.lock() {
                            let pb = guard.get_or_insert_with(|| {
                                let pb = ProgressBar::new(p.total as u64);
                                let style = match ProgressStyle::default_bar().template(
                                    "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg} [{per_sec}]",
                                ) {
                                    Ok(style) => style.progress_chars("##-"),
                                    Err(_) => ProgressStyle::default_bar(),
                                };
                                pb.set_style(style);
                                pb.set_message(format!("Индексация {}", config_name.as_str()));
                                pb
                            });
                            pb.set_position(p.current as u64);
                            pb.set_message(module_display.clone());
                            if p.current == p.total {
                                pb.finish_with_message(format!(
                                    "Индексация {} завершена ({} модулей)",
                                    config_name.as_str(),
                                    p.total
                                ));
                            }
                        }
                    });

                    coordinator.build_config_layer_b_payload(
                        &config_path_for_build,
                        &config_info_for_build,
                        &config_set_id,
                        &metadata,
                        &metadata_for_indexing,
                        prefix.as_deref(),
                        progress,
                    )
                },
                |payload| !payload.raw_types.is_empty(),
            )?;

            if entry.from_cache {
                emit_cached_module_index_progress(&config_info, &progress_callback);
                let prev = self.startup_progress();
                self.set_startup_progress(bsl_shared::api::StartupProgressDto {
                    phase: "Индексация BSL-модулей".to_string(),
                    message: Some(format!("{}: из кэша", config_info.name)),
                    ..prev
                });
            }

            let payload = entry.value;
            let config_methods_count = payload.indexed.config_methods.len();
            let global_functions_count = payload.indexed.global_functions.len();
            let def_locations_count = payload.indexed.definition_locations.len();
            let global_def_locations_count = payload.indexed.global_definition_locations.len();

            if let Some(ref mut combined) = combined_payload {
                combined.raw_types.extend(payload.raw_types.clone());
                extend_indexed_signatures(&mut combined.indexed, &payload.indexed);
            }

            for (owner_type, sig) in payload.indexed.config_methods {
                repository.add_config_method_signature(&owner_type, sig);
            }
            for (name, sig) in payload.indexed.global_functions {
                repository.add_global_function_signature(&name, sig);
            }
            for (owner_type, method_name, location) in payload.indexed.definition_locations {
                repository.add_config_method_definition_location(
                    &owner_type,
                    &method_name,
                    location,
                );
            }
            for (function_name, location) in payload.indexed.global_definition_locations {
                repository.add_global_function_definition_location(&function_name, location);
            }

            info!(
                "Проиндексированы экспортные методы из *.bsl для {}: методов={}, глобальных функций={}, locations={}, global_locations={}",
                config_info.name,
                config_methods_count,
                global_functions_count,
                def_locations_count,
                global_def_locations_count
            );

            let prev = self.startup_progress();
            self.set_startup_progress(bsl_shared::api::StartupProgressDto {
                phase: "Индексация BSL-модулей".to_string(),
                message: Some(format!(
                    "{}: методов={}, глобальных функций={}",
                    config_info.name, config_methods_count, global_functions_count
                )),
                ..prev
            });

            total_types += payload.raw_types.len();

            // Загружаем типы в репозиторий
            repository
                .load_types(payload.raw_types)
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки типов: {}", e))?;

            self.update_intellisense_index_from_modules(
                config_path,
                &payload.indexed.module_signatures,
            );

            match config_info.config_type {
                ConfigurationType::Base => base_count += 1,
                ConfigurationType::Extension => ext_count += 1,
            }
        }

        self.update_intellisense_index_from_metadata(
            config_path,
            repository.get_all_types(),
            &all_metadata,
        );
        self.persist_intellisense_index_snapshot(&project_id, &config_set_id);

        Ok((
            LoadMetadataResult {
                base_config_count: base_count,
                extensions_count: ext_count,
                total_types,
            },
            combined_payload,
        ))
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
    /// ```text
    /// use std::path::Path;
    /// use bsl_runtime::SystemCoordinator;
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

        info!("Обнаружение конфигураций в: {}", config_path.display());

        // show_progress = true - показываем терминальный прогресс-бар при парсинге конфигурации
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), true);
        let configurations = discovery
            .discover_all_configurations()
            .map_err(|e| anyhow::anyhow!("Ошибка обнаружения конфигураций: {}", e))?;

        info!("Найдено {} конфигураций", configurations.len());

        let mut base_count = 0;
        let mut ext_count = 0;
        let mut total_types = 0;
        let all_metadata: Vec<UniversalMetadataObject> = Vec::new();

        let bundle = self.domain_bundle().ok_or_else(|| {
            anyhow::anyhow!("Domain bundle не инициализирован. Вызовите start() сначала.")
        })?;
        let repository = bundle.repository.clone();

        let config_set_id = config_set_id_from_configs(&configurations);

        for config_info in configurations {
            info!(
                "Загрузка конфигурации: {} ({}{})",
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

            let cache_key =
                self.build_config_cache_key(config_path, &config_info, Some(&config_set_id))?;
            let cache = self.disk_cache();
            let discovery_root = config_path.to_path_buf();
            let config_info_clone = config_info.clone();
            let entry = cache.get_or_build_with_swr(
                &cache_key,
                move || {
                    let discovery = ConfigurationDiscovery::new(discovery_root, true);
                    // Без progress_callback в публичном методе (для обратной совместимости)
                    discovery
                        .discover_metadata_in_configuration(
                            &config_info_clone,
                            None::<fn(crate::data::loaders::progress::ProgressUpdate)>,
                        )
                        .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))
                },
                |metadata| !metadata.is_empty(),
            )?;

            let metadata = entry.value;

            let prefix = config_info.prefix.as_deref();
            let metadata_for_indexing = Self::apply_prefix_for_indexing(&metadata, prefix);

            let prev = self.startup_progress();
            self.set_startup_progress(bsl_shared::api::StartupProgressDto {
                phase: "Индексация BSL-модулей".to_string(),
                message: Some(format!("Конфигурация: {}", config_info.name)),
                ..prev
            });

            let config_root = config_info.path.clone();
            let config_name = Arc::new(config_info.name.clone());
            let terminal_progress: Arc<Mutex<Option<ProgressBar>>> = Arc::new(Mutex::new(None));
            let coordinator_for_progress = self.clone_for_blocking();

            let layer_key = self.build_config_layer_b_cache_key(
                config_path,
                &config_info,
                Some(&config_set_id),
                &metadata_for_indexing,
            )?;
            let cache = self.disk_cache();
            let coordinator = self.clone_for_blocking();
            let config_path_for_build = config_path.to_path_buf();
            let config_info_for_build = config_info.clone();
            let config_set_id = config_set_id.clone();
            let metadata = metadata.clone();
            let metadata_for_indexing = metadata_for_indexing.clone();
            let prefix = prefix.map(str::to_string);
            let entry = cache.get_or_build_with_swr(
                &layer_key,
                move || {
                    let terminal_progress = Arc::clone(&terminal_progress);
                    let config_name = Arc::clone(&config_name);
                    let coordinator_for_progress = coordinator_for_progress.clone();
                    let progress = Some(move |p: ModuleIndexProgress| {
                        let prev = coordinator_for_progress.startup_progress();
                        let module_display = p
                            .module_path
                            .strip_prefix(&config_root)
                            .unwrap_or(&p.module_path)
                            .display()
                            .to_string();
                        coordinator_for_progress.set_startup_progress(bsl_shared::api::StartupProgressDto {
                            phase: "Индексация BSL-модулей".to_string(),
                            current: p.current as u64,
                            total: p.total as u64,
                            percentage: prev.percentage,
                            message: Some(format!(
                                "{}: {}/{} — {}",
                                config_name.as_str(),
                                p.current,
                                p.total,
                                module_display
                            )),
                            done: false,
                        });
                        if let Ok(mut guard) = terminal_progress.lock() {
                            let pb = guard.get_or_insert_with(|| {
                                let pb = ProgressBar::new(p.total as u64);
                                pb.set_style(
                                    ProgressStyle::default_bar()
                                        .template(
                                            "[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg} [{per_sec}]",
                                        )
                                        .unwrap()
                                        .progress_chars("##-"),
                                );
                                pb.set_message(format!("Индексация {}", config_name.as_str()));
                                pb
                            });
                            pb.set_position(p.current as u64);
                            pb.set_message(module_display.clone());
                            if p.current == p.total {
                                pb.finish_with_message(format!(
                                    "Индексация {} завершена ({} модулей)",
                                    config_name.as_str(),
                                    p.total
                                ));
                            }
                        }
                    });

                    coordinator.build_config_layer_b_payload(
                        &config_path_for_build,
                        &config_info_for_build,
                        &config_set_id,
                        &metadata,
                        &metadata_for_indexing,
                        prefix.as_deref(),
                        progress,
                    )
                },
                |payload| !payload.raw_types.is_empty(),
            )?;

            if entry.from_cache {
                let prev = self.startup_progress();
                self.set_startup_progress(bsl_shared::api::StartupProgressDto {
                    phase: "Индексация BSL-модулей".to_string(),
                    message: Some(format!("{}: из кэша", config_info.name)),
                    ..prev
                });
            }

            let payload = entry.value;
            let config_methods_count = payload.indexed.config_methods.len();
            let global_functions_count = payload.indexed.global_functions.len();
            let def_locations_count = payload.indexed.definition_locations.len();
            let global_def_locations_count = payload.indexed.global_definition_locations.len();

            for (owner_type, sig) in payload.indexed.config_methods {
                repository.add_config_method_signature(&owner_type, sig);
            }
            for (name, sig) in payload.indexed.global_functions {
                repository.add_global_function_signature(&name, sig);
            }
            for (owner_type, method_name, location) in payload.indexed.definition_locations {
                repository.add_config_method_definition_location(
                    &owner_type,
                    &method_name,
                    location,
                );
            }
            for (function_name, location) in payload.indexed.global_definition_locations {
                repository.add_global_function_definition_location(&function_name, location);
            }

            info!(
                "Проиндексированы экспортные методы из *.bsl для {}: методов={}, глобальных функций={}, locations={}, global_locations={}",
                config_info.name,
                config_methods_count,
                global_functions_count,
                def_locations_count,
                global_def_locations_count
            );

            let prev = self.startup_progress();
            self.set_startup_progress(bsl_shared::api::StartupProgressDto {
                phase: "Индексация BSL-модулей".to_string(),
                message: Some(format!(
                    "{}: методов={}, глобальных функций={}",
                    config_info.name, config_methods_count, global_functions_count
                )),
                ..prev
            });

            total_types += payload.raw_types.len();

            // Загружаем типы в репозиторий
            repository
                .load_types(payload.raw_types)
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки типов: {}", e))?;

            self.update_intellisense_index_from_modules(
                config_path,
                &payload.indexed.module_signatures,
            );

            match config_info.config_type {
                ConfigurationType::Base => base_count += 1,
                ConfigurationType::Extension => ext_count += 1,
            }
        }

        self.update_intellisense_index_from_metadata(
            config_path,
            repository.get_all_types(),
            &all_metadata,
        );

        Ok(LoadMetadataResult {
            base_config_count: base_count,
            extensions_count: ext_count,
            total_types,
        })
    }

    fn update_intellisense_index_from_metadata(
        &self,
        config_path: &Path,
        raw_types: Vec<RawTypeData>,
        metadata: &[UniversalMetadataObject],
    ) {
        let config_fingerprint = match config_fingerprint(config_path, self.strict_fingerprint()) {
            Ok(fingerprint) => fingerprint,
            Err(err) => {
                warn!(
                    "Не удалось вычислить fingerprint конфигурации для индекса: {}",
                    err
                );
                format!("path:{}", config_path.display())
            }
        };
        let platform_version = self
            .platform_version()
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
        self.intellisense_index
            .reset_metadata_snapshot_preserving_platform_types(
                &config_fingerprint,
                &platform_version,
            );

        let mut type_items: Vec<IndexItem> = Vec::new();
        for raw_type in raw_types.iter() {
            if raw_type.source != bsl_shared::domain::types::RawDataSource::Configuration {
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

        let mut by_kind: std::collections::HashMap<MetadataKind, Vec<IndexItem>> =
            std::collections::HashMap::new();
        for obj in metadata {
            let Some(kind) = obj.object_type else {
                continue;
            };
            let mut item = IndexItem::new(
                obj.name.clone(),
                IndexItemKind::Metadata(kind),
                IndexKind::Metadata,
            );
            item.facets = obj.facets.clone();
            by_kind.entry(kind).or_default().push(item);
        }
        for (kind, items) in by_kind {
            self.intellisense_index
                .replace_metadata_for_kind(kind, items);
        }
    }

    fn try_warmup_intellisense_index(
        &self,
        config_path: &Path,
        project_id: &str,
        config_set_id: &str,
    ) {
        if index_warmup_disabled() {
            self.observability.record_index_warmup_skip("disabled");
            return;
        }
        if self.disk_cache.is_disabled() {
            self.observability
                .record_index_warmup_skip("disk_cache_disabled");
            return;
        }
        let config_fingerprint = match config_fingerprint(config_path, self.strict_fingerprint()) {
            Ok(fingerprint) => fingerprint,
            Err(err) => {
                warn!(
                    "Не удалось вычислить fingerprint конфигурации для warmup индекса: {}",
                    err
                );
                self.observability
                    .record_index_warmup_skip("fingerprint_error");
                return;
            }
        };
        let snapshot_id = IndexSnapshotId::new(&config_fingerprint, env!("CARGO_PKG_VERSION"));
        let store = match self.intellisense_index_store_for_ids(project_id, config_set_id) {
            Ok(store) => store,
            Err(err) => {
                warn!("Не удалось инициализировать store индекса: {}", err);
                self.observability
                    .record_index_warmup_skip("store_init_error");
                return;
            }
        };
        let coordinator = self.clone_for_blocking();
        let project_id = project_id.to_string();
        let config_set_id = config_set_id.to_string();
        std::thread::spawn(move || {
            let started = Instant::now();
            let observability = coordinator.observability.clone();
            match store.load_snapshot(&snapshot_id) {
                Ok(Some(snapshot)) => {
                    let current = coordinator.intellisense_index.snapshot();
                    if !should_apply_warmup(&current, &snapshot) {
                        log_warmup_skip_reason(
                            &current,
                            &snapshot,
                            &project_id,
                            &config_set_id,
                            &observability,
                        );
                        return;
                    }
                    let mut merged = snapshot;
                    if merged.keyword_index.is_empty() && !current.keyword_index.is_empty() {
                        merged.keyword_index = current.keyword_index;
                    }
                    coordinator.intellisense_index.replace_snapshot(merged);
                    observability.record_index_warmup_hit(started.elapsed());
                    info!(
                        "Warmup индекса: hit project={}, config={} ({} ms)",
                        project_id,
                        config_set_id,
                        started.elapsed().as_millis()
                    );
                }
                Ok(None) => {
                    observability.record_index_warmup_miss(started.elapsed());
                    info!(
                        "Warmup индекса: miss project={}, config={} ({} ms)",
                        project_id,
                        config_set_id,
                        started.elapsed().as_millis()
                    );
                }
                Err(err) => {
                    warn!("Не удалось загрузить индекс из store: {}", err);
                    observability.record_index_warmup_skip("load_error");
                }
            }
        });
    }

    fn persist_intellisense_index_snapshot(&self, project_id: &str, config_set_id: &str) {
        if self.disk_cache.is_disabled() {
            return;
        }

        let store = match self.intellisense_index_store_for_ids(project_id, config_set_id) {
            Ok(store) => store,
            Err(err) => {
                warn!("Не удалось инициализировать store индекса: {}", err);
                return;
            }
        };
        let snapshot = self.intellisense_index.snapshot();
        if let Err(err) = store.store_snapshot(&snapshot) {
            warn!("Не удалось сохранить индекс в store: {}", err);
        }
    }

    fn update_intellisense_index_from_modules(
        &self,
        config_root: &Path,
        module_signatures: &[crate::data::loaders::config_bsl_modules::ModuleSignatureSnapshot],
    ) {
        for module in module_signatures {
            let module_key = module
                .module_path
                .strip_prefix(config_root)
                .unwrap_or(&module.module_path)
                .to_string_lossy()
                .to_string();
            let mut items = Vec::new();

            for name in &module.method_names {
                let mut item = IndexItem::new(
                    name.clone(),
                    IndexItemKind::Symbol(SymbolKind::Method),
                    IndexKind::Module,
                );
                item.uri = Some(module_key.clone());
                item.scope = Some(SymbolScope::Module);
                item.visibility = Some(Visibility::Public);
                items.push(item);
            }

            for name in &module.global_function_names {
                let mut item = IndexItem::new(
                    name.clone(),
                    IndexItemKind::Symbol(SymbolKind::Function),
                    IndexKind::Module,
                );
                item.uri = Some(module_key.clone());
                item.scope = Some(SymbolScope::Global);
                item.visibility = Some(Visibility::Public);
                items.push(item);
            }

            self.intellisense_index
                .replace_modules_for_key(&module_key, items);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_config_layer_b_payload<F>(
        &self,
        root_path: &Path,
        config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
        config_set_id: &str,
        metadata: &[UniversalMetadataObject],
        metadata_for_indexing: &[UniversalMetadataObject],
        prefix: Option<&str>,
        progress_callback: Option<F>,
    ) -> Result<ConfigLayerBCachePayload>
    where
        F: Fn(ModuleIndexProgress) + Send + Sync + 'static,
    {
        let indexed = match self.index_config_bsl_modules_with_cache(
            root_path,
            config_info,
            config_set_id,
            metadata_for_indexing,
            progress_callback,
        ) {
            Ok(indexed) => indexed,
            Err(e) => {
                warn!(
                    "Не удалось проиндексировать экспортные методы из *.bsl для {}: {}",
                    config_info.name, e
                );
                IndexedConfigSignatures::default()
            }
        };

        let raw_types: Vec<RawTypeData> = metadata
            .iter()
            .flat_map(|obj| obj.to_raw_type_data_with_forms(prefix))
            .collect();

        Ok(ConfigLayerBCachePayload { raw_types, indexed })
    }

    fn index_config_bsl_modules_with_cache<F>(
        &self,
        root_path: &Path,
        config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
        config_set_id: &str,
        metadata_for_indexing: &[UniversalMetadataObject],
        progress_callback: Option<F>,
    ) -> Result<IndexedConfigSignatures>
    where
        F: Fn(ModuleIndexProgress) + Send + Sync + 'static,
    {
        let cache = self.disk_cache();

        let load_cached = |module_path: &Path| -> Result<Option<ParsedModuleData>> {
            if !module_path.exists() {
                return Ok(None);
            }
            let key = self.build_config_module_cache_key(
                root_path,
                config_info,
                Some(config_set_id),
                module_path,
            )?;
            cache.try_get(&key)
        };

        let store_cached = |module_path: &Path, parsed: &ParsedModuleData| -> Result<()> {
            if !module_path.exists() {
                return Ok(());
            }
            let key = self.build_config_module_cache_key(
                root_path,
                config_info,
                Some(config_set_id),
                module_path,
            )?;
            let parsed = parsed.clone();
            let _ = cache.get_or_build_with(&key, || Ok(parsed), |_| true)?;
            Ok(())
        };

        index_configuration_bsl_modules_with_progress_parallel_cached(
            root_path,
            metadata_for_indexing,
            progress_callback,
            load_cached,
            store_cached,
        )
    }

    fn build_config_layer_b_cache_key(
        &self,
        root_path: &Path,
        config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
        config_set_id: Option<&str>,
        metadata_for_indexing: &[UniversalMetadataObject],
    ) -> Result<DiskCacheKey> {
        let identity = config_cache_identity(root_path, config_info);
        let config_set_id = config_set_id.unwrap_or_default();
        let source_identity = format!(
            "{}|{}|{}",
            identity.canonical_config.to_string_lossy(),
            config_info.uuid.clone().unwrap_or_default(),
            config_set_id
        );
        let strict = self.strict_fingerprint();
        let source_fingerprint =
            config_layer_b_fingerprint(&config_info.path, metadata_for_indexing, strict).map_err(
                |e| anyhow::anyhow!("Ошибка вычисления fingerprint конфигурации: {}", e),
            )?;
        let settings_fingerprint = config_layer_b_settings_fingerprint(strict);

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
            "config-layer-b",
            key_hash,
            source_identity,
            source_fingerprint,
            settings_fingerprint,
        )
        .with_project_id(identity.project_id)
        .with_config_id(identity.config_id))
    }

    fn build_config_module_cache_key(
        &self,
        root_path: &Path,
        config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
        config_set_id: Option<&str>,
        module_path: &Path,
    ) -> Result<DiskCacheKey> {
        let identity = config_cache_identity(root_path, config_info);
        let config_set_id = config_set_id.unwrap_or_default();
        let source_identity = format!("{}|{}", module_path.to_string_lossy(), config_set_id);
        let strict = self.strict_fingerprint();
        let source_fingerprint = file_fingerprint(module_path, strict)?;
        let settings_fingerprint = module_cache_settings_fingerprint(strict);

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
            "config-module-parse",
            key_hash,
            source_identity,
            source_fingerprint,
            settings_fingerprint,
        )
        .with_project_id(identity.project_id)
        .with_config_id(identity.config_id))
    }

    fn build_config_cache_key(
        &self,
        root_path: &Path,
        config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
        config_set_id: Option<&str>,
    ) -> Result<DiskCacheKey> {
        let identity = config_cache_identity(root_path, config_info);

        let config_set_id = config_set_id.unwrap_or_default();
        let source_identity = format!(
            "{}|{}|{}",
            identity.canonical_config.to_string_lossy(),
            config_info.uuid.clone().unwrap_or_default(),
            config_set_id
        );
        let strict = self.strict_fingerprint();
        let source_fingerprint = config_fingerprint(&config_info.path, strict)
            .map_err(|e| anyhow::anyhow!("Ошибка вычисления fingerprint конфигурации: {}", e))?;
        let settings_fingerprint = config_settings_fingerprint(strict);

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
            "config",
            key_hash,
            source_identity,
            source_fingerprint,
            settings_fingerprint,
        )
        .with_project_id(identity.project_id)
        .with_config_id(identity.config_id))
    }

    pub(crate) fn build_config_combined_cache_meta(
        &self,
        config_path: &Path,
    ) -> Result<ConfigCombinedCacheMeta> {
        use crate::data::loaders::config_metadata_parser::ConfigurationDiscovery;

        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf(), true);
        let configurations = discovery
            .discover_all_configurations()
            .map_err(|e| anyhow::anyhow!("Ошибка обнаружения конфигураций: {}", e))?;
        if configurations.is_empty() {
            return Err(anyhow::anyhow!(
                "Конфигурации не найдены в {}",
                config_path.display()
            ));
        }

        let config_set_id = config_set_id_from_configs(&configurations);
        let mut combined_items: Vec<DiskCacheKey> = Vec::new();
        let mut project_id = None;

        for config_info in &configurations {
            let identity = config_cache_identity(config_path, config_info);
            if project_id.is_none() {
                project_id = Some(identity.project_id.clone());
            }

            let cache_key =
                self.build_config_cache_key(config_path, config_info, Some(&config_set_id))?;
            let cache = self.disk_cache();
            let entry = match cache.try_get(&cache_key) {
                Ok(Some(metadata)) => metadata,
                Ok(None) | Err(_) => {
                    let discovery_root = config_path.to_path_buf();
                    let config_info = config_info.clone();
                    cache
                        .get_or_build_with_swr(
                            &cache_key,
                            move || {
                                let discovery = ConfigurationDiscovery::new(discovery_root, true);
                                discovery
                                    .discover_metadata_in_configuration(
                                        &config_info,
                                        None::<fn(crate::data::loaders::progress::ProgressUpdate)>,
                                    )
                                    .map_err(|e| {
                                        anyhow::anyhow!("Не удалось обнаружить метаданные: {}", e)
                                    })
                            },
                            |metadata| !metadata.is_empty(),
                        )?
                        .value
                }
            };

            let prefix = config_info.prefix.as_deref();
            let metadata_for_indexing = Self::apply_prefix_for_indexing(&entry, prefix);

            let layer_key = self.build_config_layer_b_cache_key(
                config_path,
                config_info,
                Some(&config_set_id),
                &metadata_for_indexing,
            )?;
            combined_items.push(layer_key);
        }

        combined_items.sort_by(|a, b| a.source_identity.cmp(&b.source_identity));

        let source_identity = combined_items
            .iter()
            .map(|key| key.source_identity.clone())
            .collect::<Vec<_>>()
            .join("||");
        let source_fingerprint = combined_items
            .iter()
            .map(|key| key.source_fingerprint.clone())
            .collect::<Vec<_>>()
            .join("||");
        let settings_fingerprint = combined_items
            .iter()
            .map(|key| key.settings_fingerprint.clone())
            .collect::<Vec<_>>()
            .join("||");

        Ok(ConfigCombinedCacheMeta {
            project_id: project_id.unwrap_or_else(|| project_id_from_root(config_path)),
            config_set_id: config_set_id.clone(),
            source_identity: format!("config_set_id={};{}", config_set_id, source_identity),
            source_fingerprint,
            settings_fingerprint,
        })
    }
}

#[derive(Debug)]
struct ConfigCacheIdentity {
    project_id: String,
    config_id: String,
    canonical_config: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::loaders::config_metadata_parser::ConfigurationDiscovery;
    use tempfile::TempDir;

    #[test]
    fn test_config_metadata_disk_cache_reuse() {
        let config_root = Path::new("examples/conf/conf_test");
        if !config_root.exists() {
            eprintln!("⚠️ Конфигурация не найдена в examples/conf/conf_test");
            return;
        }

        let temp = TempDir::new().unwrap();
        let cache = crate::system::DiskCache::with_root(temp.path().to_path_buf(), 1).unwrap();
        let coordinator = SystemCoordinator::new();

        let discovery = ConfigurationDiscovery::new(config_root.to_path_buf(), false);
        let configs = match discovery.discover_all_configurations() {
            Ok(list) if !list.is_empty() => list,
            _ => return,
        };
        let config_set_id = config_set_id_from_configs(&configs);
        let config_info = &configs[0];

        let key = coordinator
            .build_config_cache_key(config_root, config_info, Some(&config_set_id))
            .unwrap();

        let entry = cache
            .get_or_build_with(
                &key,
                || {
                    discovery
                        .discover_metadata_in_configuration(
                            config_info,
                            None::<fn(crate::data::loaders::progress::ProgressUpdate)>,
                        )
                        .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))
                },
                |metadata| !metadata.is_empty(),
            )
            .unwrap();
        assert!(!entry.from_cache);

        let entry = cache
            .get_or_build_with(
                &key,
                || {
                    discovery
                        .discover_metadata_in_configuration(
                            config_info,
                            None::<fn(crate::data::loaders::progress::ProgressUpdate)>,
                        )
                        .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))
                },
                |metadata| !metadata.is_empty(),
            )
            .unwrap();
        assert!(entry.from_cache);
    }
}
fn emit_cached_config_progress<F>(
    config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
    progress_callback: &F,
) where
    F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
{
    let phases = [
        IndexingPhase::ConfigurationDiscovery,
        IndexingPhase::ConfigurationParsing,
        IndexingPhase::ConfigurationLinking,
        IndexingPhase::ConfigurationFinalizing,
    ];
    for phase in phases {
        progress_callback(ProgressUpdate::new(
            phase,
            1,
            1,
            Some(format!("{} (кэш)", config_info.name)),
        ));
    }
}

fn emit_cached_module_index_progress<F>(
    config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
    progress_callback: &F,
) where
    F: Fn(ProgressUpdate) + Send + Sync + Clone + 'static,
{
    progress_callback(ProgressUpdate::new(
        IndexingPhase::ConfigurationIndexingModules,
        1,
        1,
        Some(format!(
            "Индексация BSL-модулей: {} (кэш)",
            config_info.name
        )),
    ));
}

fn config_cache_identity(
    root_path: &Path,
    config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
) -> ConfigCacheIdentity {
    let canonical_root =
        std::fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());
    let canonical_config =
        std::fs::canonicalize(&config_info.path).unwrap_or_else(|_| config_info.path.clone());
    let project_id = blake3::hash(canonical_root.to_string_lossy().as_bytes())
        .to_hex()
        .to_string();
    let config_id = config_id_for_info(config_info);

    ConfigCacheIdentity {
        project_id,
        config_id,
        canonical_config,
    }
}

fn project_id_from_root(root_path: &Path) -> String {
    let canonical_root =
        std::fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());
    blake3::hash(canonical_root.to_string_lossy().as_bytes())
        .to_hex()
        .to_string()
}

fn config_id_for_info(
    config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
) -> String {
    if let Some(uuid) = config_info.uuid.clone() {
        return uuid;
    }
    let canonical_config =
        std::fs::canonicalize(&config_info.path).unwrap_or_else(|_| config_info.path.clone());
    blake3::hash(canonical_config.to_string_lossy().as_bytes())
        .to_hex()
        .to_string()
}

fn normalize_config_root(config_path: &Path) -> PathBuf {
    if config_path.file_name().and_then(|name| name.to_str()) == Some("Configuration.xml") {
        return config_path.parent().unwrap_or(config_path).to_path_buf();
    }
    config_path.to_path_buf()
}

fn extend_indexed_signatures(
    target: &mut IndexedConfigSignatures,
    source: &IndexedConfigSignatures,
) {
    target.config_methods.extend(source.config_methods.clone());
    target
        .global_functions
        .extend(source.global_functions.clone());
    target
        .definition_locations
        .extend(source.definition_locations.clone());
    target
        .global_definition_locations
        .extend(source.global_definition_locations.clone());
    target
        .module_signatures
        .extend(source.module_signatures.clone());
}

fn config_fingerprint(config_path: &Path, strict: bool) -> Result<String> {
    use walkdir::WalkDir;

    let mut files: Vec<PathBuf> = WalkDir::new(config_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("xml") {
                Some(path.to_path_buf())
            } else {
                None
            }
        })
        .collect();

    if files.is_empty() {
        let config_xml = config_path.join("Configuration.xml");
        if config_xml.exists() {
            files.push(config_xml);
        }
    }

    Ok(merkle_fingerprint_paths(config_path, &files, strict))
}

fn config_settings_fingerprint(strict: bool) -> String {
    format!(
        "config_parser_v3;modules_indexing_v1;strict_fingerprint={}",
        strict
    )
}

fn config_layer_b_fingerprint(
    config_path: &Path,
    metadata_for_indexing: &[UniversalMetadataObject],
    strict: bool,
) -> Result<String> {
    use walkdir::WalkDir;

    let mut files: Vec<PathBuf> = WalkDir::new(config_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("xml") {
                Some(path.to_path_buf())
            } else {
                None
            }
        })
        .collect();

    let module_paths = collect_module_paths(config_path, metadata_for_indexing);

    if files.is_empty() {
        let config_xml = config_path.join("Configuration.xml");
        if config_xml.exists() {
            files.push(config_xml);
        }
    }

    Ok(merkle_fingerprint_paths_with_modules(
        config_path,
        &files,
        &module_paths,
        strict,
    ))
}

fn config_layer_b_settings_fingerprint(strict: bool) -> String {
    format!(
        "config_layer_b_v2;modules_indexing_v1;strict_fingerprint={}",
        strict
    )
}

fn module_cache_settings_fingerprint(strict: bool) -> String {
    format!("config_module_parse_v2;strict_fingerprint={}", strict)
}

fn file_fingerprint(path: &Path, strict: bool) -> Result<String> {
    if !path.exists() {
        return Ok(String::new());
    }
    Ok(merkle_fingerprint_single(path, strict))
}

struct MerkleArtifact {
    kind: &'static str,
    path: PathBuf,
    path_norm: String,
}

fn merkle_fingerprint_paths(config_root: &Path, xml_paths: &[PathBuf], strict: bool) -> String {
    let mut artifacts: Vec<MerkleArtifact> = xml_paths
        .iter()
        .filter(|path| path.is_file())
        .map(|path| MerkleArtifact {
            kind: "xml",
            path: path.to_path_buf(),
            path_norm: normalize_path(path, Some(config_root)),
        })
        .collect();

    artifacts.sort_by(|a, b| {
        a.kind
            .cmp(b.kind)
            .then_with(|| a.path_norm.cmp(&b.path_norm))
    });
    artifacts.dedup_by(|a, b| a.kind == b.kind && a.path_norm == b.path_norm);

    merkle_root_for_artifacts(&artifacts, strict)
}

fn merkle_fingerprint_paths_with_modules(
    config_root: &Path,
    xml_paths: &[PathBuf],
    module_paths: &[PathBuf],
    strict: bool,
) -> String {
    let mut artifacts: Vec<MerkleArtifact> = Vec::new();

    for path in xml_paths {
        if path.is_file() {
            artifacts.push(MerkleArtifact {
                kind: "xml",
                path: path.to_path_buf(),
                path_norm: normalize_path(path, Some(config_root)),
            });
        }
    }

    for path in module_paths {
        if path.is_file() {
            artifacts.push(MerkleArtifact {
                kind: "bsl",
                path: path.to_path_buf(),
                path_norm: normalize_path(path, Some(config_root)),
            });
        }
    }

    artifacts.sort_by(|a, b| {
        a.kind
            .cmp(b.kind)
            .then_with(|| a.path_norm.cmp(&b.path_norm))
    });
    artifacts.dedup_by(|a, b| a.kind == b.kind && a.path_norm == b.path_norm);

    merkle_root_for_artifacts(&artifacts, strict)
}

fn merkle_fingerprint_single(path: &Path, strict: bool) -> String {
    let artifacts = [MerkleArtifact {
        kind: "file",
        path: path.to_path_buf(),
        path_norm: normalize_path(path, None),
    }];
    merkle_root_for_artifacts(&artifacts, strict)
}

fn merkle_root_for_artifacts(artifacts: &[MerkleArtifact], strict: bool) -> String {
    let mut leaves: Vec<blake3::Hash> = Vec::with_capacity(artifacts.len());
    for artifact in artifacts {
        if strict {
            let content_hash = match std::fs::read(&artifact.path) {
                Ok(contents) => blake3::hash(&contents),
                Err(_) => blake3::hash(&[]),
            };
            leaves.push(merkle_leaf_hash_strict(
                artifact.kind,
                &artifact.path_norm,
                &content_hash,
            ));
        } else {
            let (size, mtime_ns) = file_metadata_fields(&artifact.path);
            leaves.push(merkle_leaf_hash_fast(
                artifact.kind,
                &artifact.path_norm,
                size,
                mtime_ns,
            ));
        }
    }

    let root_raw = merkle_root_raw(&leaves);
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0x02]);
    hasher.update(b"merkle-root-v1");
    hasher.update(&[0x00]);
    hasher.update(root_raw.as_bytes());
    hasher.finalize().to_hex().to_string()
}

fn merkle_leaf_hash_fast(kind: &str, path_norm: &str, size: u64, mtime_ns: u64) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0x00]);
    hasher.update(kind.as_bytes());
    hasher.update(&[0x00]);
    hasher.update(path_norm.as_bytes());
    hasher.update(&[0x00]);
    hasher.update(&size.to_le_bytes());
    hasher.update(&[0x00]);
    hasher.update(&mtime_ns.to_le_bytes());
    hasher.finalize()
}

fn merkle_leaf_hash_strict(
    kind: &str,
    path_norm: &str,
    content_hash: &blake3::Hash,
) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0x00]);
    hasher.update(kind.as_bytes());
    hasher.update(&[0x00]);
    hasher.update(path_norm.as_bytes());
    hasher.update(&[0x00]);
    hasher.update(content_hash.as_bytes());
    hasher.finalize()
}

fn merkle_root_raw(leaves: &[blake3::Hash]) -> blake3::Hash {
    let empty = [0u8; 32];
    if leaves.is_empty() {
        return blake3::Hash::from(merkle_node_hash(&empty, &empty));
    }

    let mut level: Vec<[u8; 32]> = leaves.iter().map(|hash| *hash.as_bytes()).collect();
    while level.len() > 1 {
        let mut next_level: Vec<[u8; 32]> = Vec::with_capacity(level.len().div_ceil(2));
        let mut idx = 0;
        while idx < level.len() {
            let left = level[idx];
            let right = if idx + 1 < level.len() {
                level[idx + 1]
            } else {
                left
            };
            next_level.push(merkle_node_hash(&left, &right));
            idx += 2;
        }
        level = next_level;
    }

    blake3::Hash::from(level[0])
}

fn merkle_node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0x01]);
    hasher.update(left);
    hasher.update(right);
    *hasher.finalize().as_bytes()
}

fn file_metadata_fields(path: &Path) -> (u64, u64) {
    if let Ok(metadata) = std::fs::metadata(path) {
        let size = metadata.len();
        let mtime_ns = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(duration_to_u64_nanos)
            .unwrap_or(0);
        (size, mtime_ns)
    } else {
        (0, 0)
    }
}

fn duration_to_u64_nanos(duration: std::time::Duration) -> u64 {
    let nanos = duration.as_nanos();
    if nanos > u64::MAX as u128 {
        u64::MAX
    } else {
        nanos as u64
    }
}

#[cfg(test)]
mod merkle_tests {
    use super::*;
    use tempfile::TempDir;

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents.as_bytes()).unwrap();
    }

    #[test]
    fn merkle_root_for_empty_artifacts_matches_spec() {
        let artifacts: Vec<MerkleArtifact> = Vec::new();
        let root = merkle_root_for_artifacts(&artifacts, true);

        let empty = [0u8; 32];
        let root_raw = blake3::Hash::from(merkle_node_hash(&empty, &empty));

        let mut hasher = blake3::Hasher::new();
        hasher.update(&[0x02]);
        hasher.update(b"merkle-root-v1");
        hasher.update(&[0x00]);
        hasher.update(root_raw.as_bytes());
        let expected = hasher.finalize().to_hex().to_string();

        assert_eq!(root, expected);
    }

    #[test]
    fn merkle_root_raw_duplicates_last_leaf_for_odd_count() {
        let a = blake3::hash(b"a");
        let b = blake3::hash(b"b");
        let c = blake3::hash(b"c");

        let raw3 = merkle_root_raw(&[a, b, c]);
        let raw4 = merkle_root_raw(&[a, b, c, c]);

        assert_eq!(raw3, raw4);
    }

    #[test]
    fn merkle_fingerprint_paths_is_order_independent_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let a = root.join("A.xml");
        let b = root.join("B.xml");
        write_file(&a, "<a/>");
        write_file(&b, "<b/>");

        let fp1 = merkle_fingerprint_paths(root, &[b.clone(), a.clone()], true);
        let fp2 = merkle_fingerprint_paths(root, &[a.clone(), b.clone()], true);
        let fp3 = merkle_fingerprint_paths(root, &[a, b], true);

        assert_eq!(fp1, fp2);
        assert_eq!(fp1, fp3);
    }

    #[test]
    fn merkle_fingerprint_paths_is_stable_for_same_inputs_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let a = root.join("A.xml");
        let b = root.join("B.xml");
        write_file(&a, "<a/>");
        write_file(&b, "<b/>");

        let paths = [a, b];
        let fp1 = merkle_fingerprint_paths(root, &paths, true);
        let fp2 = merkle_fingerprint_paths(root, &paths, true);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn merkle_fingerprint_changes_when_one_file_changes_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let a = root.join("A.xml");
        let b = root.join("B.xml");
        write_file(&a, "<a/>");
        write_file(&b, "<b/>");

        let paths = [a.clone(), b.clone()];
        let before = merkle_fingerprint_paths(root, &paths, true);

        write_file(&b, "<b>changed</b>");
        let after = merkle_fingerprint_paths(root, &paths, true);

        assert_ne!(before, after);
    }

    #[test]
    fn merkle_fingerprint_paths_dedups_duplicates_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let a = root.join("A.xml");
        write_file(&a, "<a/>");

        let fp_unique = merkle_fingerprint_paths(root, std::slice::from_ref(&a), true);
        let fp_dup = merkle_fingerprint_paths(root, &[a.clone(), a], true);

        assert_eq!(fp_unique, fp_dup);
    }

    #[test]
    fn merkle_fingerprint_paths_with_modules_matches_paths_when_no_modules_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let a = root.join("A.xml");
        write_file(&a, "<a/>");
        let xml_paths = [a];

        let empty_modules: Vec<PathBuf> = Vec::new();
        let fp_xml = merkle_fingerprint_paths(root, &xml_paths, true);
        let fp_with = merkle_fingerprint_paths_with_modules(root, &xml_paths, &empty_modules, true);

        assert_eq!(fp_xml, fp_with);
    }

    #[test]
    fn merkle_fingerprint_paths_with_modules_includes_bsl_artifacts_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let xml = root.join("Configuration.xml");
        let bsl = root
            .join("CommonModules")
            .join("M")
            .join("Ext")
            .join("Module.bsl");
        write_file(&xml, "<Configuration/>");
        write_file(&bsl, "Процедура X() Экспорт\nКонецПроцедуры\n");

        let xml_paths = [xml];

        let empty_modules: Vec<PathBuf> = Vec::new();
        let no_modules =
            merkle_fingerprint_paths_with_modules(root, &xml_paths, &empty_modules, true);
        let with_modules = merkle_fingerprint_paths_with_modules(root, &xml_paths, &[bsl], true);

        assert_ne!(no_modules, with_modules);
    }

    #[test]
    fn normalize_path_strips_root_and_uses_forward_slashes() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let path = root
            .join("CommonModules")
            .join("M")
            .join("Ext")
            .join("Module.bsl");
        write_file(&path, ""); // файл должен существовать для реалистичного кейса

        assert_eq!(
            normalize_path(&path, Some(root)),
            "CommonModules/M/Ext/Module.bsl"
        );
    }

    #[test]
    fn merkle_root_depends_on_artifact_kind_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let path = root.join("A.any");
        write_file(&path, "same");

        let fp_one = merkle_root_for_artifacts(
            &[MerkleArtifact {
                kind: "xml",
                path: path.clone(),
                path_norm: "A".to_string(),
            }],
            true,
        );
        let fp_two = merkle_root_for_artifacts(
            &[
                MerkleArtifact {
                    kind: "xml",
                    path: path.clone(),
                    path_norm: "A".to_string(),
                },
                MerkleArtifact {
                    kind: "bsl",
                    path,
                    path_norm: "A".to_string(),
                },
            ],
            true,
        );

        assert_ne!(fp_one, fp_two);
    }

    #[test]
    fn merkle_fingerprint_paths_with_modules_dedups_duplicate_bsl_paths_strict() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let xml = root.join("Configuration.xml");
        let bsl = root.join("CommonModules").join("M").join("Module.bsl");
        write_file(&xml, "<Configuration/>");
        write_file(&bsl, "Процедура X() Экспорт\nКонецПроцедуры\n");

        let xml_paths = [xml];

        let fp_unique = merkle_fingerprint_paths_with_modules(
            root,
            &xml_paths,
            std::slice::from_ref(&bsl),
            true,
        );
        let fp_dup =
            merkle_fingerprint_paths_with_modules(root, &xml_paths, &[bsl.clone(), bsl], true);

        assert_eq!(fp_unique, fp_dup);
    }
}

fn snapshot_is_empty(snapshot: &crate::system::IndexSnapshot) -> bool {
    snapshot.type_index.is_empty()
        && snapshot.symbol_index.is_empty()
        && snapshot.module_index.is_empty()
        && snapshot.metadata_index.is_empty()
}

fn should_apply_warmup(
    current: &crate::system::IndexSnapshot,
    candidate: &crate::system::IndexSnapshot,
) -> bool {
    current.id == candidate.id && snapshot_is_empty(current)
}

fn log_warmup_skip_reason(
    current: &crate::system::IndexSnapshot,
    candidate: &crate::system::IndexSnapshot,
    project_id: &str,
    config_set_id: &str,
    observability: &crate::system::BasicObservability,
) {
    if current.id != candidate.id {
        info!(
            "Warmup индекса пропущен (snapshot_id изменился) project={}, config={}",
            project_id, config_set_id
        );
        observability.record_index_warmup_skip("snapshot_changed");
        return;
    }
    if !snapshot_is_empty(current) {
        info!(
            "Warmup индекса пропущен (индекс уже заполнен) project={}, config={}",
            project_id, config_set_id
        );
        observability.record_index_warmup_skip("already_populated");
    }
}

fn index_warmup_disabled() -> bool {
    !crate::system::runtime_config::global_runtime_config()
        .get_bool(crate::system::runtime_config::RuntimeKey::IndexWarmup)
        .unwrap_or(true)
}

#[cfg(test)]
mod warmup_tests {
    use super::*;
    use crate::system::IndexSnapshot;
    use std::sync::Arc;

    #[test]
    fn warmup_skips_when_snapshot_id_changed() {
        let current = IndexSnapshot::empty(IndexSnapshotId::from_hash("a"));
        let candidate = IndexSnapshot::empty(IndexSnapshotId::from_hash("b"));
        assert!(!should_apply_warmup(&current, &candidate));
    }

    #[test]
    fn warmup_skips_when_current_has_data() {
        let mut current = IndexSnapshot::empty(IndexSnapshotId::from_hash("a"));
        Arc::make_mut(&mut current.type_index).insert(
            "Type".to_string(),
            Arc::new(IndexItem::new(
                "Type",
                IndexItemKind::Type(TypeKind::Platform),
                IndexKind::Type,
            )),
        );
        let candidate = IndexSnapshot::empty(IndexSnapshotId::from_hash("a"));
        assert!(!should_apply_warmup(&current, &candidate));
    }

    #[test]
    fn warmup_applies_when_snapshot_matches_and_empty() {
        let current = IndexSnapshot::empty(IndexSnapshotId::from_hash("a"));
        let candidate = IndexSnapshot::empty(IndexSnapshotId::from_hash("a"));
        assert!(should_apply_warmup(&current, &candidate));
    }
}

fn normalize_path(path: &Path, root: Option<&Path>) -> String {
    let relative = root
        .and_then(|base| path.strip_prefix(base).ok())
        .unwrap_or(path);
    let mut parts = Vec::new();
    for component in relative.components() {
        if let std::path::Component::Normal(value) = component {
            parts.push(value.to_string_lossy().to_string());
        }
    }
    parts.join("/")
}

fn config_set_id_from_configs(
    configs: &[crate::data::loaders::config_metadata_parser::ConfigurationInfo],
) -> String {
    let mut base = None;
    let mut extensions = Vec::new();

    for info in configs {
        let id = info.uuid.clone().unwrap_or_else(|| {
            blake3::hash(info.path.to_string_lossy().as_bytes())
                .to_hex()
                .to_string()
        });
        if info.is_base() {
            base = Some(id);
        } else {
            extensions.push(id);
        }
    }

    extensions.sort();
    let mut parts = Vec::new();
    if let Some(base) = base {
        parts.push(base);
    }
    parts.extend(extensions);

    if parts.is_empty() {
        return String::new();
    }

    blake3::hash(parts.join("|").as_bytes())
        .to_hex()
        .to_string()
}

fn config_set_id_from_single(
    info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
) -> String {
    let id = info.uuid.clone().unwrap_or_else(|| {
        blake3::hash(info.path.to_string_lossy().as_bytes())
            .to_hex()
            .to_string()
    });
    blake3::hash(id.as_bytes()).to_hex().to_string()
}

fn discover_single_config(
    discovery: &crate::data::loaders::config_metadata_parser::ConfigurationDiscovery,
    config_path: &Path,
) -> Option<crate::data::loaders::config_metadata_parser::ConfigurationInfo> {
    let config_xml =
        if config_path.file_name().and_then(|name| name.to_str()) == Some("Configuration.xml") {
            config_path.to_path_buf()
        } else {
            config_path.join("Configuration.xml")
        };

    if !config_xml.exists() {
        return None;
    }

    let configs = discovery.discover_all_configurations().ok()?;
    let config_dir = std::fs::canonicalize(config_xml.parent()?).ok()?;
    configs
        .into_iter()
        .find(|info| std::fs::canonicalize(&info.path).ok().as_ref() == Some(&config_dir))
}
