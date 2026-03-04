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

#[path = "config_loader/helpers.rs"]
mod helpers;
#[path = "config_loader/load_single.rs"]
mod load_single;

use self::helpers::*;

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
}
