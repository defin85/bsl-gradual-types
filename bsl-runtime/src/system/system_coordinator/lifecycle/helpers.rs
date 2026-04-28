use super::*;

pub(super) fn syntax_helper_fingerprint(
    syntax_parser: &SyntaxHelperLoader,
    syntax_path: &Path,
    strict: bool,
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
        if strict {
            if let Ok(contents) = fs::read(&path) {
                let content_hash = blake3::hash(&contents);
                hasher.update(content_hash.as_bytes());
            }
        }
    }

    Ok(hasher.finalize().to_hex().to_string())
}

pub(super) fn syntax_helper_settings_fingerprint(
    syntax_parser: &SyntaxHelperLoader,
    strict: bool,
) -> String {
    let settings = &syntax_parser.settings;
    format!(
        "syntax_helper_parser_v2;threads={:?};batch={};show={};limit={:?};skip={:?};parallel={};keywords={};strict_fingerprint={}",
        settings.max_threads,
        settings.batch_size,
        settings.show_progress,
        settings.file_limit,
        settings.skip_dirs,
        settings.parallel_indexing,
        settings.collect_keywords,
        strict
    )
}

impl SystemCoordinator {
    pub(super) fn apply_combined_config_payload(
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
            repository.add_global_function_definition_location(function_name, location.clone());
        }

        repository
            .load_types(payload.config_raw_types.clone())
            .map_err(StartupError::PlatformTypesError)?;

        Ok(())
    }

    pub(super) fn update_intellisense_index_from_config_raw_types(
        &self,
        config_fingerprint: &str,
        platform_version: &str,
        raw_types: &[RawTypeData],
    ) {
        use std::collections::HashMap;

        self.intellisense_index
            .reset_metadata_snapshot_preserving_platform_types(
                config_fingerprint,
                platform_version,
            );

        let mut type_items: Vec<IndexItem> = Vec::new();
        for raw_type in raw_types.iter() {
            if raw_type.source != RawDataSource::Configuration {
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

        let mut by_kind: HashMap<bsl_shared::domain::types::MetadataKind, Vec<IndexItem>> =
            HashMap::new();
        for raw_type in raw_types.iter() {
            if raw_type.source != RawDataSource::Configuration {
                continue;
            }
            let Some(kind) = raw_type.kind else {
                continue;
            };
            let Some((_, object_name)) = raw_type.name.split_once('.') else {
                continue;
            };

            let mut item = IndexItem::new(
                object_name.to_string(),
                IndexItemKind::Metadata(kind),
                IndexKind::Metadata,
            );
            item.facets = raw_type.facets.clone();
            by_kind.entry(kind).or_default().push(item);
        }
        for (kind, items) in by_kind {
            self.intellisense_index
                .replace_metadata_for_kind(kind, items);
        }
    }

    /// Загрузка базовых типов как fallback.
    ///
    /// Используется когда syntax_helper не доступен.
    /// Загружает только примитивные типы и типы-коллекции без методов.
    /// Методы будут недоступны, но GenericInfo для inference будет работать.
    pub(crate) fn load_fallback_types(
        repository: &Arc<InMemoryTypeRepository>,
    ) -> Result<Vec<RawTypeData>, StartupError> {
        info!("Загружаем базовые типы платформы 1С (fallback mode)...");
        repository.set_platform_docs_loaded(false);

        // Примитивные типы.
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
            // Типы-коллекции (без методов, только для GenericInfo).
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

        // Заполняем SignatureIndex (будет пустой в fallback mode).
        use crate::data::loaders::{apply_generic_info_to_repository, SyntaxHelperSource};
        use bsl_shared::domain::SignatureSourceRegistry;

        let index = SignatureSourceRegistry::new()
            .register(SyntaxHelperSource::new(platform_types_clone))
            .build();
        repository.set_signature_index(index);

        // Применяем GenericInfo для типов-коллекций.
        let generic_count = apply_generic_info_to_repository(repository.as_ref());

        info!(
            "Базовые типы загружены: {} типов (fallback mode)",
            type_count
        );
        info!("GenericInfo применён к {} типам-коллекциям", generic_count);
        warn!(
            "Методы недоступны в fallback mode. Укажите путь к syntax_helper для полной функциональности."
        );
        Ok(platform_types)
    }
}
