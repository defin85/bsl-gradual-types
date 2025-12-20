//! Загрузка метаданных конфигураций
//!
//! Методы для загрузки метаданных из базовых конфигураций и расширений

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

use crate::data::loaders::progress::ProgressUpdate;
use crate::data::loaders::{
    index_configuration_bsl_modules_with_progress_parallel, ModuleIndexProgress,
    UniversalMetadataObject,
};
use bsl_shared::domain::types::RawTypeData;

use super::coordinator::SystemCoordinator;
use super::types::LoadMetadataResult;

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

        // Без progress_callback в публичном методе (для обратной совместимости)
        let metadata_objects = discovery
            .discover_all_metadata(None::<fn(crate::data::loaders::progress::ProgressUpdate)>)
            .map_err(|e| anyhow::anyhow!("Не удалось обнаружить метаданные: {}", e))?;

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

            // НОВОЕ: Используем новый метод с прогрессом через 4 фазы
            let metadata = discovery
                .discover_metadata_in_configuration_with_progress(
                    &config_info,
                    progress_callback.clone(),
                )
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))?;

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

            // Без progress_callback в публичном методе (для обратной совместимости)
            let metadata = discovery
                .discover_metadata_in_configuration(
                    &config_info,
                    None::<fn(crate::data::loaders::progress::ProgressUpdate)>,
                )
                .map_err(|e| anyhow::anyhow!("Ошибка загрузки метаданных: {}", e))?;

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
}
