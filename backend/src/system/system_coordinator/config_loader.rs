//! Загрузка метаданных конфигураций
//!
//! Методы для загрузки метаданных из базовых конфигураций и расширений

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

use crate::data::loaders::progress::{IndexingPhase, ProgressUpdate};
use crate::data::loaders::{
    index_configuration_bsl_modules_with_progress_parallel, ModuleIndexProgress,
    UniversalMetadataObject,
};
use bsl_shared::domain::types::RawTypeData;

use super::coordinator::SystemCoordinator;
use super::types::LoadMetadataResult;
use crate::system::DiskCacheKey;

impl SystemCoordinator {
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
    /// use bsl_backend::system::SystemCoordinator;
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

        let metadata_objects = if let Some(config_info) = config_info {
            let config_set_id = config_set_id_from_single(&config_info);
            let cache_key =
                self.build_config_cache_key(config_path, &config_info, Some(&config_set_id))?;
            let cache = self.disk_cache();
            let entry = cache
                .get_or_build_with(
                    &cache_key,
                    || {
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

        // Получаем текущий AnalysisEngine или создаём новый
        let engine = self.analysis_engine().ok_or_else(|| {
            anyhow::anyhow!("AnalysisEngine не инициализирован. Вызовите start() сначала.")
        })?;

        // Получаем TypeRepository из AnalysisEngine
        let repository = engine.get_repository();

        // Конвертируем все объекты в RawTypeData
        let indexed = index_configuration_bsl_modules_with_progress_parallel(
            config_path,
            &metadata_objects,
            None::<fn(ModuleIndexProgress)>,
        );
        if let Ok(indexed) = indexed {
            let config_methods_count = indexed.config_methods.len();
            let global_functions_count = indexed.global_functions.len();
            let def_locations_count = indexed.definition_locations.len();
            let global_def_locations_count = indexed.global_definition_locations.len();

            for (owner_type, sig) in indexed.config_methods {
                repository.add_config_method_signature(&owner_type, sig);
            }
            for (name, sig) in indexed.global_functions {
                repository.add_global_function_signature(&name, sig);
            }
            for (owner_type, method_name, location) in indexed.definition_locations {
                repository.add_config_method_definition_location(&owner_type, &method_name, location);
            }
            for (function_name, location) in indexed.global_definition_locations {
                repository.add_global_function_definition_location(&function_name, location);
            }

            info!(
                "Проиндексированы экспортные методы из *.bsl: методов={}, глобальных функций={}, locations={}, global_locations={}",
                config_methods_count,
                global_functions_count,
                def_locations_count,
                global_def_locations_count
            );
        } else if let Err(e) = indexed {
            warn!("Не удалось проиндексировать экспортные методы из *.bsl модулей: {}", e);
        }

        let raw_types: Vec<RawTypeData> = metadata_objects
            .into_iter()
            .flat_map(|obj| obj.to_raw_type_data_with_forms(None))
            .collect();

        let count = raw_types.len();

        // Загружаем все типы в репозиторий за один вызов
        repository.load_types(raw_types)?;

        info!("Загружено {} типов из конфигурации", count);
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

        // Получаем AnalysisEngine и TypeRepository один раз
        let engine = self.analysis_engine().ok_or_else(|| {
            anyhow::anyhow!("AnalysisEngine не инициализирован. Вызовите start() сначала.")
        })?;
        let repository = engine.get_repository();

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
            let entry = cache
                .get_or_build_with(
                    &cache_key,
                    || {
                        // НОВОЕ: Используем новый метод с прогрессом через 4 фазы парсинга
                        discovery.discover_metadata_in_configuration_with_progress(
                            &config_info,
                            progress_callback.clone(),
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
            match index_configuration_bsl_modules_with_progress_parallel(
                &config_info.path,
                &metadata_for_indexing,
                Some({
                    let terminal_progress = Arc::clone(&terminal_progress);
                    let config_name = Arc::clone(&config_name);
                    let coordinator_for_progress = coordinator_for_progress.clone();
                    let progress_callback = progress_callback_for_modules;
                    move |p: ModuleIndexProgress| {
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
                    }
                }),
            ) {
                Ok(indexed) => {
                    let config_methods_count = indexed.config_methods.len();
                    let global_functions_count = indexed.global_functions.len();
                    let def_locations_count = indexed.definition_locations.len();
                    let global_def_locations_count = indexed.global_definition_locations.len();

                    for (owner_type, sig) in indexed.config_methods {
                        repository.add_config_method_signature(&owner_type, sig);
                    }
                    for (name, sig) in indexed.global_functions {
                        repository.add_global_function_signature(&name, sig);
                    }
                    for (owner_type, method_name, location) in indexed.definition_locations {
                        repository.add_config_method_definition_location(
                            &owner_type,
                            &method_name,
                            location,
                        );
                    }
                    for (function_name, location) in indexed.global_definition_locations {
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
                }
                Err(e) => warn!(
                    "Не удалось проиндексировать экспортные методы из *.bsl для {}: {}",
                    config_info.name, e
                ),
            }

            let raw_types: Vec<RawTypeData> = metadata
                .into_iter()
                .flat_map(|obj| obj.to_raw_type_data_with_forms(prefix))
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
    /// ```ignore
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

        // Получаем AnalysisEngine и TypeRepository один раз
        let engine = self.analysis_engine().ok_or_else(|| {
            anyhow::anyhow!("AnalysisEngine не инициализирован. Вызовите start() сначала.")
        })?;
        let repository = engine.get_repository();

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
            let entry = cache
                .get_or_build_with(
                    &cache_key,
                    || {
                        // Без progress_callback в публичном методе (для обратной совместимости)
                        discovery
                            .discover_metadata_in_configuration(
                                &config_info,
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
            match index_configuration_bsl_modules_with_progress_parallel(
                &config_info.path,
                &metadata_for_indexing,
                Some({
                    let terminal_progress = Arc::clone(&terminal_progress);
                    let config_name = Arc::clone(&config_name);
                    let coordinator_for_progress = coordinator_for_progress.clone();
                    move |p: ModuleIndexProgress| {
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
                }
                }),
            ) {
                Ok(indexed) => {
                    let config_methods_count = indexed.config_methods.len();
                    let global_functions_count = indexed.global_functions.len();
                    let def_locations_count = indexed.definition_locations.len();
                    let global_def_locations_count = indexed.global_definition_locations.len();

                    for (owner_type, sig) in indexed.config_methods {
                        repository.add_config_method_signature(&owner_type, sig);
                    }
                    for (name, sig) in indexed.global_functions {
                        repository.add_global_function_signature(&name, sig);
                    }
                    for (owner_type, method_name, location) in indexed.definition_locations {
                        repository.add_config_method_definition_location(
                            &owner_type,
                            &method_name,
                            location,
                        );
                    }
                    for (function_name, location) in indexed.global_definition_locations {
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
                }
                Err(e) => warn!(
                    "Не удалось проиндексировать экспортные методы из *.bsl для {}: {}",
                    config_info.name, e
                ),
            }

            let raw_types: Vec<RawTypeData> = metadata
                .into_iter()
                .flat_map(|obj| obj.to_raw_type_data_with_forms(prefix))
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

    fn build_config_cache_key(
        &self,
        root_path: &Path,
        config_info: &crate::data::loaders::config_metadata_parser::ConfigurationInfo,
        config_set_id: Option<&str>,
    ) -> Result<DiskCacheKey> {
        let canonical_root =
            std::fs::canonicalize(root_path).unwrap_or_else(|_| root_path.to_path_buf());
        let canonical_config =
            std::fs::canonicalize(&config_info.path).unwrap_or_else(|_| config_info.path.clone());

        let project_id = blake3::hash(canonical_root.to_string_lossy().as_bytes())
            .to_hex()
            .to_string();
        let config_id = config_info
            .uuid
            .clone()
            .unwrap_or_else(|| {
                blake3::hash(canonical_config.to_string_lossy().as_bytes())
                    .to_hex()
                    .to_string()
            });

        let config_set_id = config_set_id.unwrap_or_default();
        let source_identity = format!(
            "{}|{}|{}",
            canonical_config.to_string_lossy(),
            config_info.uuid.clone().unwrap_or_default(),
            config_set_id
        );
        let source_fingerprint =
            config_fingerprint(&config_info.path).map_err(|e| {
                anyhow::anyhow!("Ошибка вычисления fingerprint конфигурации: {}", e)
            })?;
        let settings_fingerprint = config_settings_fingerprint();

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
        .with_project_id(project_id)
        .with_config_id(config_id))
    }
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

fn config_fingerprint(config_path: &Path) -> Result<String> {
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

    files.sort();

    let strict = std::env::var("BSL_CACHE_STRICT_FINGERPRINT").is_ok();
    let mut hasher = blake3::Hasher::new();
    for path in files {
        let path_str = path.to_string_lossy();
        hasher.update(path_str.as_bytes());
        if let Ok(metadata) = std::fs::metadata(&path) {
            hasher.update(&metadata.len().to_le_bytes());
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            hasher.update(&modified.to_le_bytes());
        }
        if strict {
            if let Ok(contents) = std::fs::read(&path) {
                let content_hash = blake3::hash(&contents);
                hasher.update(content_hash.as_bytes());
            }
        }
    }

    Ok(hasher.finalize().to_hex().to_string())
}

fn config_settings_fingerprint() -> String {
    let strict = std::env::var("BSL_CACHE_STRICT_FINGERPRINT").is_ok();
    format!(
        "config_parser_v1;modules_indexing_v1;strict_fingerprint={}",
        strict
    )
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

    blake3::hash(parts.join("|").as_bytes()).to_hex().to_string()
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
    let config_xml = if config_path
        .file_name()
        .and_then(|name| name.to_str())
        == Some("Configuration.xml")
    {
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
