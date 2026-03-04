use super::*;

impl SystemCoordinator {
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
