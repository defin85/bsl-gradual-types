//! Data Layer: Type Repository trait and implementations

use crate::signature_index::SignatureValidationResult;
use crate::signature_index::{ConstructorSignature, MethodSignature, SignatureIndex};
pub use crate::RepositoryStats;
use anyhow::{bail, Result};
use bsl_types::types::{GenericInfo, MetadataKind, ParameterInfo, RawDataSource, RawTypeData};
use bsl_types::{TypeDefinitionLocation, TypeId};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use std::time::SystemTime;

#[path = "repository/completion.rs"]
mod completion;
#[path = "repository/traits.rs"]
mod traits;
pub use completion::{CompletionItem, CompletionKind};
pub use traits::TypeRepository;

// --- In-Memory Implementation ---

/// In-memory реализация репозитория
pub struct InMemoryTypeRepository {
    types: RwLock<Vec<RawTypeData>>,
    last_updated: RwLock<Option<SystemTime>>,
    platform_docs_loaded: RwLock<bool>,
    /// Индекс сигнатур методов для валидации (thread-safe)
    signature_index: RwLock<SignatureIndex>,
    /// Индекс локаций объявлений методов/функций (для Go To Definition на метод)
    method_definition_index: RwLock<HashMap<(TypeId, TypeId), TypeDefinitionLocation>>,
    /// Индекс типов: TypeId -> индекс в vectors types (O(1) lookup)
    /// Индексирует по русским именам, английским именам и CamelCase-вариантам
    type_index: RwLock<HashMap<TypeId, usize>>,
}

impl InMemoryTypeRepository {
    pub fn new() -> Self {
        Self {
            types: RwLock::new(Vec::new()),
            last_updated: RwLock::new(None),
            platform_docs_loaded: RwLock::new(true),
            signature_index: RwLock::new(SignatureIndex::new()),
            method_definition_index: RwLock::new(HashMap::new()),
            type_index: RwLock::new(HashMap::new()),
        }
    }

    /// Получить мутабельный доступ к SignatureIndex для заполнения
    ///
    /// Используется при загрузке платформенных типов для заполнения индекса
    pub fn populate_signature_index<F>(&self, populate_fn: F)
    where
        F: FnOnce(&mut SignatureIndex),
    {
        let mut index = self.signature_index.write().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "SignatureIndex RwLock poisoned in populate_signature_index, recovering"
            );
            poisoned.into_inner()
        });
        populate_fn(&mut index);
    }

    /// Установить SignatureIndex напрямую (для Registry паттерна)
    ///
    /// Заменяет текущий индекс на предоставленный.
    /// Используется с SignatureSourceRegistry для декларативной настройки.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::InMemoryTypeRepository;
    /// # use bsl_repository::signature_registry::{SignatureDataSource, SignatureSourceRegistry};
    /// # use bsl_types::types::RawTypeData;
    /// # struct DummySource;
    /// # impl SignatureDataSource for DummySource {
    /// #     fn name(&self) -> &str { "dummy" }
    /// #     fn priority(&self) -> u32 { 0 }
    /// #     fn load(&self) -> Vec<RawTypeData> { Vec::new() }
    /// # }
    /// let index = SignatureSourceRegistry::new()
    ///     .register(DummySource)
    ///     .build();
    /// let repository = InMemoryTypeRepository::new();
    /// repository.set_signature_index(index);
    /// ```
    pub fn set_signature_index(&self, index: SignatureIndex) {
        let mut sig_index = self.signature_index.write().unwrap_or_else(|poisoned| {
            tracing::warn!("SignatureIndex RwLock poisoned in set_signature_index, recovering");
            poisoned.into_inner()
        });
        *sig_index = index;
    }
}

impl Default for InMemoryTypeRepository {
    fn default() -> Self {
        Self::new()
    }
}

const FORBIDDEN_INSTANCE_LOCAL_TYPE_PREFIXES: [&str; 4] = [
    "__bsl_v2_instance_effect__",
    "__bsl_v2_collection_schema__",
    "__bsl_v2_typed_structure__",
    "__bsl_v2_typed_row__",
];

const INSTANCE_LOCAL_MARKERS: [&str; 4] =
    ["snapshot_id=", "scope_id=", "instance_id=", "creation_span"];

const UNIVERSAL_COLLECTION_MARKERS: [&str; 6] = [
    "соответствие",
    "map",
    "структура",
    "structure",
    "таблицазначений",
    "valuetable",
];

fn is_forbidden_instance_local_type_name(type_name: &str) -> bool {
    let lowered = type_name.trim().to_lowercase();
    if lowered.is_empty() {
        return false;
    }

    if FORBIDDEN_INSTANCE_LOCAL_TYPE_PREFIXES
        .iter()
        .any(|prefix| lowered.starts_with(prefix))
    {
        return true;
    }

    let has_instance_local_marker = INSTANCE_LOCAL_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker));
    if !has_instance_local_marker {
        return false;
    }

    UNIVERSAL_COLLECTION_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker))
}

fn ensure_no_forbidden_instance_local_types(
    operation: &str,
    candidate_types: &[RawTypeData],
) -> Result<()> {
    let forbidden: Vec<&str> = candidate_types
        .iter()
        .filter_map(|type_data| {
            is_forbidden_instance_local_type_name(&type_data.name)
                .then_some(type_data.name.as_str())
        })
        .collect();

    if forbidden.is_empty() {
        return Ok(());
    }

    bail!(
        "TypeRepository.{operation}: forbidden per-instance synthetic type names for universal collections: {}. Use snapshot-local InstanceEffectStore instead.",
        forbidden.join(", ")
    );
}

impl TypeRepository for InMemoryTypeRepository {
    fn set_platform_docs_loaded(&self, loaded: bool) {
        let mut flag = self
            .platform_docs_loaded
            .write()
            .unwrap_or_else(|poisoned| {
                tracing::warn!("platform_docs_loaded RwLock poisoned, recovering");
                poisoned.into_inner()
            });
        *flag = loaded;
    }

    fn platform_docs_loaded(&self) -> bool {
        *self.platform_docs_loaded.read().unwrap_or_else(|poisoned| {
            tracing::warn!("platform_docs_loaded RwLock poisoned, recovering");
            poisoned.into_inner()
        })
    }

    fn load_types(&self, new_types: Vec<RawTypeData>) -> Result<()> {
        ensure_no_forbidden_instance_local_types("load_types", &new_types)?;

        let mut types = self.types.write().unwrap_or_else(|poisoned| {
            tracing::warn!("types RwLock poisoned in load_types, recovering");
            poisoned.into_inner()
        });
        let mut index = self.type_index.write().unwrap_or_else(|poisoned| {
            tracing::warn!("type_index RwLock poisoned in load_types, recovering");
            poisoned.into_inner()
        });

        let types_count = new_types.len();
        let start_time = std::time::Instant::now();

        let start_idx = types.len();
        for (i, type_data) in new_types.iter().enumerate() {
            let idx = start_idx + i;

            // Индексируем по основному имени (TypeId нормализует автоматически)
            let id = TypeId::new(&type_data.name);
            // Не перезаписываем если ключ уже существует (приоритет первому)
            index.entry(id).or_insert(idx);

            // Индексируем по английскому имени (если есть и отличается)
            if !type_data.english_name.is_empty() && type_data.english_name != type_data.name {
                let en_id = TypeId::new(&type_data.english_name);
                index.entry(en_id).or_insert(idx);
            }

            tracing::trace!(
                "TypeRepository: indexed '{}' (en: '{}') at idx {}",
                type_data.name,
                type_data.english_name,
                idx
            );
        }

        types.extend(new_types);

        // Обновляем timestamp
        *self.last_updated.write().unwrap_or_else(|poisoned| {
            tracing::warn!("last_updated RwLock poisoned in load_types, recovering");
            poisoned.into_inner()
        }) = Some(SystemTime::now());

        let elapsed = start_time.elapsed();
        tracing::debug!(
            "TypeRepository.load_types: loaded {} types in {:.2}ms (total types now: {})",
            types_count,
            elapsed.as_secs_f64() * 1000.0,
            types.len()
        );

        Ok(())
    }

    fn upsert_types(&self, new_types: Vec<RawTypeData>) -> Result<()> {
        if new_types.is_empty() {
            return Ok(());
        }

        ensure_no_forbidden_instance_local_types("upsert_types", &new_types)?;

        let mut types = self.types.write().unwrap_or_else(|poisoned| {
            tracing::warn!("types RwLock poisoned in upsert_types, recovering");
            poisoned.into_inner()
        });
        let mut index = self.type_index.write().unwrap_or_else(|poisoned| {
            tracing::warn!("type_index RwLock poisoned in upsert_types, recovering");
            poisoned.into_inner()
        });

        let mut primary_index: HashMap<TypeId, usize> = HashMap::new();
        for (idx, t) in types.iter().enumerate() {
            primary_index.entry(TypeId::new(&t.name)).or_insert(idx);
        }

        for type_data in new_types {
            let id = TypeId::new(&type_data.name);
            if let Some(&idx) = primary_index.get(&id) {
                if let Some(slot) = types.get_mut(idx) {
                    *slot = type_data;
                }
            } else {
                types.push(type_data);
                primary_index.insert(id, types.len().saturating_sub(1));
            }
        }

        index.clear();
        for (idx, type_data) in types.iter().enumerate() {
            let id = TypeId::new(&type_data.name);
            index.entry(id).or_insert(idx);

            if !type_data.english_name.is_empty() && type_data.english_name != type_data.name {
                let en_id = TypeId::new(&type_data.english_name);
                index.entry(en_id).or_insert(idx);
            }
        }

        *self.last_updated.write().unwrap_or_else(|poisoned| {
            tracing::warn!("last_updated RwLock poisoned in upsert_types, recovering");
            poisoned.into_inner()
        }) = Some(SystemTime::now());

        Ok(())
    }

    fn remove_types(&self, type_names: &[String]) -> Result<usize> {
        if type_names.is_empty() {
            return Ok(0);
        }

        let mut types = self.types.write().unwrap_or_else(|poisoned| {
            tracing::warn!("types RwLock poisoned in remove_types, recovering");
            poisoned.into_inner()
        });
        let mut index = self.type_index.write().unwrap_or_else(|poisoned| {
            tracing::warn!("type_index RwLock poisoned in remove_types, recovering");
            poisoned.into_inner()
        });

        let targets: HashSet<TypeId> = type_names.iter().map(|n| TypeId::new(n)).collect();
        let before = types.len();
        types.retain(|t| !targets.contains(&TypeId::new(&t.name)));
        let removed = before.saturating_sub(types.len());

        index.clear();
        for (idx, type_data) in types.iter().enumerate() {
            let id = TypeId::new(&type_data.name);
            index.entry(id).or_insert(idx);

            if !type_data.english_name.is_empty() && type_data.english_name != type_data.name {
                let en_id = TypeId::new(&type_data.english_name);
                index.entry(en_id).or_insert(idx);
            }
        }

        *self.last_updated.write().unwrap_or_else(|poisoned| {
            tracing::warn!("last_updated RwLock poisoned in remove_types, recovering");
            poisoned.into_inner()
        }) = Some(SystemTime::now());

        Ok(removed)
    }

    fn get_all_types(&self) -> Vec<RawTypeData> {
        self.types
            .read()
            .unwrap_or_else(|poisoned| {
                tracing::warn!("types RwLock poisoned in get_all_types, recovering");
                poisoned.into_inner()
            })
            .clone()
    }

    fn find_type(&self, name: &str) -> Option<RawTypeData> {
        let types = self.types.read().unwrap_or_else(|poisoned| {
            tracing::warn!("types RwLock poisoned in find_type, recovering");
            poisoned.into_inner()
        });
        let index = self.type_index.read().unwrap_or_else(|poisoned| {
            tracing::warn!("type_index RwLock poisoned in find_type, recovering");
            poisoned.into_inner()
        });

        // Убираем generic параметры: "ТабличнаяЧасть<Работы>" -> "ТабличнаяЧасть"
        let base_name = if let Some(idx) = name.find('<') {
            &name[..idx]
        } else {
            name
        };

        // O(1) lookup через TypeId (нормализация происходит внутри TypeId::new)
        let id = TypeId::new(base_name);
        if let Some(&idx) = index.get(&id) {
            if let Some(type_data) = types.get(idx) {
                if tracing::enabled!(tracing::Level::DEBUG) {
                    tracing::debug!(
                        type_name = %name,
                        found_name = %type_data.name,
                        base_name = %base_name,
                        "TypeRepository: type found"
                    );
                }
                return Some(type_data.clone());
            }
        }

        // DEBUG: Логируем неудачный поиск только на DEBUG уровне
        if tracing::enabled!(tracing::Level::DEBUG) {
            tracing::debug!(
                type_name = %name,
                total_types = types.len(),
                index_entries = index.len(),
                "TypeRepository: type not found"
            );
        }

        None
    }

    fn get_stats(&self) -> RepositoryStats {
        let types = self.types.read().unwrap_or_else(|poisoned| {
            tracing::warn!("types RwLock poisoned in get_stats, recovering");
            poisoned.into_inner()
        });
        let last_updated = self.last_updated.read().unwrap_or_else(|poisoned| {
            tracing::warn!("last_updated RwLock poisoned in get_stats, recovering");
            poisoned.into_inner()
        });

        // Различаем типы по источнику
        let platform_count = types
            .iter()
            .filter(|t| matches!(t.source, RawDataSource::Platform))
            .count();

        let configuration_count = types
            .iter()
            .filter(|t| matches!(t.source, RawDataSource::Configuration))
            .count();

        let user_defined_count = types
            .iter()
            .filter(|t| matches!(t.source, RawDataSource::UserDefined))
            .count();

        // Конвертируем SystemTime → ISO 8601 строку
        let last_update_time = last_updated.as_ref().map(|time| {
            let datetime: DateTime<Utc> = (*time).into();
            datetime.to_rfc3339() // "2025-01-18T14:30:00Z"
        });

        RepositoryStats {
            total_types: types.len(),
            platform_types: platform_count,
            configuration_types: configuration_count,
            user_defined_types: user_defined_count,
            last_update_time,
        }
    }

    fn validate_method_signature(
        &self,
        owner_type: &str,
        method_name: &str,
        actual_params: &[ParameterInfo],
        actual_return_type: Option<&str>,
    ) -> SignatureValidationResult {
        use crate::signature_index::{MethodSignature, SignatureSource};

        let index = self.signature_index.read().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "SignatureIndex RwLock poisoned in validate_method_signature, recovering"
            );
            poisoned.into_inner()
        });

        // Ищем все overload'ы метода в индексе
        let expected_overloads = index.find_methods(owner_type, method_name);

        // Создаём актуальную сигнатуру
        let actual_signature = MethodSignature::new(
            method_name.to_string(),
            Some(owner_type.to_string()),
            actual_params.to_vec(),
            actual_return_type.map(|s| s.to_string()),
            None,
            None,
            SignatureSource::UserCode,
            None,
            bsl_types::ContextRequirements::default(),
        );

        // Валидируем
        index.validate_overloaded_signature(&expected_overloads, &actual_signature)
    }

    fn find_method_signature(
        &self,
        owner_type: Option<&str>,
        method_name: &str,
    ) -> Option<MethodSignature> {
        let index = match self.signature_index.read() {
            Ok(idx) => idx,
            Err(poisoned) => {
                tracing::warn!(
                    "SignatureIndex RwLock poisoned in find_method_signature, recovering"
                );
                poisoned.into_inner()
            }
        };

        if let Some(owner) = owner_type {
            // Ищем метод типа
            index.find_method(owner, method_name).cloned()
        } else {
            // Ищем глобальную функцию
            index.find_global_function(method_name).cloned()
        }
    }

    fn find_constructor(&self, type_name: &str) -> Option<ConstructorSignature> {
        match self.signature_index.read() {
            Ok(index) => index.find_constructor(type_name).cloned(),
            Err(poisoned) => {
                tracing::warn!("SignatureIndex RwLock is poisoned, recovering");
                poisoned.into_inner().find_constructor(type_name).cloned()
            }
        }
    }

    fn get_signature_index_clone(&self) -> SignatureIndex {
        self.signature_index
            .read()
            .unwrap_or_else(|poisoned| {
                tracing::warn!(
                    "SignatureIndex RwLock poisoned in get_signature_index_clone, recovering"
                );
                poisoned.into_inner()
            })
            .clone()
    }

    fn add_config_method_signature(&self, owner_type: &str, method: MethodSignature) {
        let mut index = self.signature_index.write().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "SignatureIndex RwLock poisoned in add_config_method_signature, recovering"
            );
            poisoned.into_inner()
        });
        index.add_config_method(TypeId::new(owner_type), method);
    }

    fn add_global_function_signature(&self, function_name: &str, method: MethodSignature) {
        let mut index = self.signature_index.write().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "SignatureIndex RwLock poisoned in add_global_function_signature, recovering"
            );
            poisoned.into_inner()
        });
        index.add_global_function(TypeId::new(function_name), method);
    }

    fn remove_config_method_signatures(&self, owner_type: &str, method_names: &[String]) {
        let mut index = self.signature_index.write().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "SignatureIndex RwLock poisoned in remove_config_method_signatures, recovering"
            );
            poisoned.into_inner()
        });
        index.remove_config_methods(owner_type, method_names);
    }

    fn remove_global_function_signatures(&self, function_names: &[String]) {
        let mut index = self.signature_index.write().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "SignatureIndex RwLock poisoned in remove_global_function_signatures, recovering"
            );
            poisoned.into_inner()
        });
        index.remove_global_functions(function_names);
    }

    fn add_config_method_definition_location(
        &self,
        owner_type: &str,
        method_name: &str,
        location: TypeDefinitionLocation,
    ) {
        let mut map = self
            .method_definition_index
            .write()
            .unwrap_or_else(|poisoned| {
                tracing::warn!(
                    "method_definition_index RwLock poisoned in add_config_method_definition_location, recovering"
                );
                poisoned.into_inner()
            });
        map.insert(
            (TypeId::new(owner_type), TypeId::new(method_name)),
            location,
        );
    }

    fn add_global_function_definition_location(
        &self,
        function_name: &str,
        location: TypeDefinitionLocation,
    ) {
        let mut map = self
            .method_definition_index
            .write()
            .unwrap_or_else(|poisoned| {
                tracing::warn!(
                    "method_definition_index RwLock poisoned in add_global_function_definition_location, recovering"
                );
                poisoned.into_inner()
            });
        let k = TypeId::new(function_name);
        map.insert((k.clone(), k), location);
    }

    fn remove_config_method_definition_locations(&self, owner_type: &str, method_names: &[String]) {
        if method_names.is_empty() {
            return;
        }

        let mut map = self
            .method_definition_index
            .write()
            .unwrap_or_else(|poisoned| {
                tracing::warn!(
                    "method_definition_index RwLock poisoned in remove_config_method_definition_locations, recovering"
                );
                poisoned.into_inner()
            });

        for name in method_names {
            map.remove(&(TypeId::new(owner_type), TypeId::new(name)));
        }
    }

    fn remove_global_function_definition_locations(&self, function_names: &[String]) {
        if function_names.is_empty() {
            return;
        }

        let mut map = self
            .method_definition_index
            .write()
            .unwrap_or_else(|poisoned| {
                tracing::warn!(
                    "method_definition_index RwLock poisoned in remove_global_function_definition_locations, recovering"
                );
                poisoned.into_inner()
            });

        for name in function_names {
            let k = TypeId::new(name);
            map.remove(&(k.clone(), k));
        }
    }

    fn find_method_definition_location(
        &self,
        owner_type: Option<&str>,
        method_name: &str,
    ) -> Option<TypeDefinitionLocation> {
        let map = self.method_definition_index.read().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "method_definition_index RwLock poisoned in find_method_definition_location, recovering"
            );
            poisoned.into_inner()
        });

        match owner_type {
            Some(owner) => map
                .get(&(TypeId::new(owner), TypeId::new(method_name)))
                .cloned(),
            None => {
                let k = TypeId::new(method_name);
                map.get(&(k.clone(), k)).cloned()
            }
        }
    }

    fn get_method_definition_locations_clone(
        &self,
    ) -> Vec<(Option<String>, String, TypeDefinitionLocation)> {
        let map = self.method_definition_index.read().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "method_definition_index RwLock poisoned in get_method_definition_locations_clone, recovering"
            );
            poisoned.into_inner()
        });

        map.iter()
            .map(|((owner, name), loc)| {
                if owner == name {
                    (None, owner.display().to_string(), loc.clone())
                } else {
                    (
                        Some(owner.display().to_string()),
                        name.display().to_string(),
                        loc.clone(),
                    )
                }
            })
            .collect()
    }

    fn get_metadata_objects_by_kind(&self, kind: MetadataKind) -> Vec<String> {
        let types = self.types.read().unwrap_or_else(|poisoned| {
            tracing::warn!("types RwLock poisoned in get_metadata_objects_by_kind, recovering");
            poisoned.into_inner()
        });
        let prefix = kind.to_prefix();

        types
            .iter()
            .filter_map(|t| {
                // Фильтруем только типы с нужным kind
                if t.kind != Some(kind) {
                    return None;
                }
                // Убираем префикс из имени: "Справочники.Контрагенты" -> "Контрагенты"
                t.name
                    .strip_prefix(&format!("{}.", prefix))
                    .map(|s| s.to_string())
            })
            .collect()
    }

    fn find_method(&self, owner_type: Option<&str>, method_name: &str) -> Option<MethodSignature> {
        self.find_method_signature(owner_type, method_name)
    }

    fn get_methods_from_signature_index(&self, type_name: &str) -> Vec<MethodSignature> {
        let index = match self.signature_index.read() {
            Ok(idx) => idx,
            Err(poisoned) => {
                tracing::warn!("SignatureIndex RwLock poisoned in get_methods_from_signature_index, recovering");
                poisoned.into_inner()
            }
        };

        // 1. Сначала пробуем найти методы по точному имени типа
        let methods = index.get_type_methods(type_name);
        if !methods.is_empty() {
            return methods.into_iter().cloned().collect();
        }

        // 2. Если не найдено, пробуем извлечь базовый фасетный тип
        //    Используем universal функцию, которая обрабатывает как placeholder формат
        //    ("СправочникМенеджер.<Имя справочника>"), так и конкретизированный
        //    ("СправочникМенеджер.Контрагенты")
        if let Some(base_type) =
            bsl_types::facet_utils::extract_base_facet_type_universal(type_name)
        {
            let base_methods = index.get_type_methods(base_type);
            return base_methods.into_iter().cloned().collect();
        }

        vec![]
    }

    fn set_generic_info(&self, type_name: &str, generic_info: GenericInfo) -> bool {
        let mut types = self.types.write().unwrap_or_else(|poisoned| {
            tracing::warn!("types RwLock poisoned in set_generic_info, recovering");
            poisoned.into_inner()
        });
        let index = self.type_index.read().unwrap_or_else(|poisoned| {
            tracing::warn!("type_index RwLock poisoned in set_generic_info, recovering");
            poisoned.into_inner()
        });

        // O(1) lookup через TypeId
        let id = TypeId::new(type_name);
        if let Some(&idx) = index.get(&id) {
            if let Some(type_data) = types.get_mut(idx) {
                type_data.generic_info = Some(generic_info);
                tracing::debug!(
                    "set_generic_info: установлен GenericInfo для типа '{}' (inference_methods: {})",
                    type_name,
                    type_data.generic_info.as_ref().map(|g| g.inference_methods.len()).unwrap_or(0)
                );
                return true;
            }
        }

        tracing::debug!(
            "set_generic_info: тип '{}' не найден в репозитории",
            type_name
        );
        false
    }
}

#[cfg(test)]
#[path = "repository/tests.rs"]
mod tests;
