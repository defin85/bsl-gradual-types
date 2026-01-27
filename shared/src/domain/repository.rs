//! Data Layer: Type Repository trait and implementations

use crate::domain::signature_index::{SignatureIndex, SignatureValidationResult};
use crate::domain::type_definition_location::TypeDefinitionLocation;
use crate::domain::type_id::TypeId;
use crate::domain::types::{MetadataKind, RawDataSource, RawTypeData};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use std::time::SystemTime;

// Completion items are part of the repository as it's the source of truth for them.
// --- Completion Item Structures ---

/// Элемент автодополнения (совместимый с LSP)
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
}

impl CompletionItem {
    pub fn new(label: String, kind: CompletionKind) -> Self {
        Self {
            insert_text: Some(label.clone()),
            filter_text: Some(label.clone()),
            sort_text: Some(label.clone()),
            label,
            kind,
            detail: None,
            documentation: None,
        }
    }

    pub fn with_details(
        label: String,
        kind: CompletionKind,
        detail: Option<String>,
        documentation: Option<String>,
    ) -> Self {
        Self {
            insert_text: Some(label.clone()),
            filter_text: Some(label.clone()),
            sort_text: Some(label.clone()),
            label,
            kind,
            detail,
            documentation,
        }
    }
}

/// Тип элемента автодополнения
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum CompletionKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Type,
    Event,
    Operator,
    TypeParameter,
    Global,
    Catalog,
    Document,
    MetadataUnknown,
    Report,
    DataProcessor,
    Register,
    InformationRegister,
    AccumulationRegister,
    AccountingRegister,
    CalculationRegister,
    ChartOfAccounts,
    ChartOfCharacteristicTypes,
    ChartOfCalculationTypes,
    BusinessProcess,
    Task,
    ExchangePlan,
    CommonModule,
    Role,
    Subsystem,
    Language,
}

impl CompletionKind {
    pub fn from_metadata_kind(kind: MetadataKind) -> Self {
        match kind {
            MetadataKind::Unknown => CompletionKind::MetadataUnknown,
            MetadataKind::Catalog => CompletionKind::Catalog,
            MetadataKind::Document => CompletionKind::Document,
            MetadataKind::Register => CompletionKind::Register,
            MetadataKind::Report => CompletionKind::Report,
            MetadataKind::DataProcessor => CompletionKind::DataProcessor,
            MetadataKind::Enum => CompletionKind::Enum,
            MetadataKind::ChartOfAccounts => CompletionKind::ChartOfAccounts,
            MetadataKind::ChartOfCharacteristicTypes => CompletionKind::ChartOfCharacteristicTypes,
            MetadataKind::ChartOfCalculationTypes => CompletionKind::ChartOfCalculationTypes,
            MetadataKind::InformationRegister => CompletionKind::InformationRegister,
            MetadataKind::AccumulationRegister => CompletionKind::AccumulationRegister,
            MetadataKind::AccountingRegister => CompletionKind::AccountingRegister,
            MetadataKind::CalculationRegister => CompletionKind::CalculationRegister,
            MetadataKind::BusinessProcess => CompletionKind::BusinessProcess,
            MetadataKind::Task => CompletionKind::Task,
            MetadataKind::ExchangePlan => CompletionKind::ExchangePlan,
            MetadataKind::Constant => CompletionKind::Constant,
            MetadataKind::CommonModule => CompletionKind::CommonModule,
            MetadataKind::Role => CompletionKind::Role,
            MetadataKind::Subsystem => CompletionKind::Subsystem,
            MetadataKind::Language => CompletionKind::Language,
        }
    }

    pub fn metadata_kind(self) -> Option<MetadataKind> {
        Some(match self {
            CompletionKind::Catalog => MetadataKind::Catalog,
            CompletionKind::Document => MetadataKind::Document,
            CompletionKind::MetadataUnknown => MetadataKind::Unknown,
            CompletionKind::Report => MetadataKind::Report,
            CompletionKind::DataProcessor => MetadataKind::DataProcessor,
            CompletionKind::Register => MetadataKind::Register,
            CompletionKind::InformationRegister => MetadataKind::InformationRegister,
            CompletionKind::AccumulationRegister => MetadataKind::AccumulationRegister,
            CompletionKind::AccountingRegister => MetadataKind::AccountingRegister,
            CompletionKind::CalculationRegister => MetadataKind::CalculationRegister,
            CompletionKind::ChartOfAccounts => MetadataKind::ChartOfAccounts,
            CompletionKind::ChartOfCharacteristicTypes => MetadataKind::ChartOfCharacteristicTypes,
            CompletionKind::ChartOfCalculationTypes => MetadataKind::ChartOfCalculationTypes,
            CompletionKind::BusinessProcess => MetadataKind::BusinessProcess,
            CompletionKind::Task => MetadataKind::Task,
            CompletionKind::ExchangePlan => MetadataKind::ExchangePlan,
            CompletionKind::Role => MetadataKind::Role,
            CompletionKind::Subsystem => MetadataKind::Subsystem,
            CompletionKind::Language => MetadataKind::Language,
            CompletionKind::CommonModule => MetadataKind::CommonModule,
            CompletionKind::Enum => MetadataKind::Enum,
            CompletionKind::Constant => MetadataKind::Constant,
            _ => return None,
        })
    }
}

// --- Type Repository Trait ---

/// Trait для репозитория типов
pub trait TypeRepository: Send + Sync {
    /// Установить флаг загрузки документации платформы (Syntax Helper)
    fn set_platform_docs_loaded(&self, loaded: bool);

    /// Проверить, загружена ли документация платформы (Syntax Helper)
    fn platform_docs_loaded(&self) -> bool;

    /// Загрузить типы в репозиторий
    fn load_types(&self, types: Vec<RawTypeData>) -> Result<()>;

    /// Обновить или добавить типы в репозиторий (по имени типа)
    fn upsert_types(&self, types: Vec<RawTypeData>) -> Result<()>;

    /// Удалить типы из репозитория по именам (регистронезависимо)
    fn remove_types(&self, type_names: &[String]) -> Result<usize>;

    /// Получить все типы из репозитория
    fn get_all_types(&self) -> Vec<RawTypeData>;

    /// Найти тип по имени
    fn find_type(&self, name: &str) -> Option<RawTypeData>;

    /// Получить статистику
    fn get_stats(&self) -> RepositoryStats;

    /// Валидировать сигнатуру метода
    ///
    /// Проверяет соответствие типов параметров и возвращаемого значения
    fn validate_method_signature(
        &self,
        owner_type: &str,
        method_name: &str,
        actual_params: &[crate::domain::types::ParameterInfo],
        actual_return_type: Option<&str>,
    ) -> SignatureValidationResult;

    /// Найти сигнатуру метода (для SignatureHelp)
    ///
    /// Ищет сигнатуру метода в индексе платформенных и конфигурационных типов
    fn find_method_signature(
        &self,
        owner_type: Option<&str>,
        method_name: &str,
    ) -> Option<crate::domain::signature_index::MethodSignature>;

    /// Найти конструктор для указанного типа
    ///
    /// # Arguments
    /// * `type_name` - Имя типа (регистронезависимо)
    ///
    /// # Returns
    /// * `Some(ConstructorSignature)` - если конструктор найден
    /// * `None` - если конструктор не найден или произошла ошибка
    fn find_constructor(
        &self,
        type_name: &str,
    ) -> Option<crate::domain::signature_index::ConstructorSignature>;

    /// Получить клон SignatureIndex для валидации (Milestone 3.10)
    ///
    /// Возвращает клон индекса сигнатур для использования в валидаторах
    fn get_signature_index_clone(&self) -> SignatureIndex;

    /// Добавить конфигурационный метод в SignatureIndex (экспорт из модулей конфигурации)
    ///
    /// Используется после загрузки конфигурации, чтобы методы из `*.bsl` модулей
    /// участвовали в hover/validation/signatureHelp.
    fn add_config_method_signature(
        &self,
        owner_type: &str,
        method: crate::domain::signature_index::MethodSignature,
    );

    /// Добавить глобальную функцию в SignatureIndex (например, из global common module)
    fn add_global_function_signature(
        &self,
        function_name: &str,
        method: crate::domain::signature_index::MethodSignature,
    );

    /// Удалить конфигурационные методы по именам (регистронезависимо)
    fn remove_config_method_signatures(&self, owner_type: &str, method_names: &[String]);

    /// Удалить глобальные функции по именам (регистронезависимо)
    fn remove_global_function_signatures(&self, function_names: &[String]);

    /// Добавить location определения конфигурационного метода (для Go To Definition на метод)
    fn add_config_method_definition_location(
        &self,
        owner_type: &str,
        method_name: &str,
        location: TypeDefinitionLocation,
    );

    /// Добавить location определения глобальной функции (например, из global common module)
    fn add_global_function_definition_location(
        &self,
        function_name: &str,
        location: TypeDefinitionLocation,
    );

    /// Удалить locations для конфигурационных методов по именам (регистронезависимо)
    fn remove_config_method_definition_locations(&self, owner_type: &str, method_names: &[String]);

    /// Удалить locations для глобальных функций по именам (регистронезависимо)
    fn remove_global_function_definition_locations(&self, function_names: &[String]);

    /// Найти location определения метода/функции (case-insensitive)
    ///
    /// `owner_type=None` означает глобальную функцию.
    fn find_method_definition_location(
        &self,
        owner_type: Option<&str>,
        method_name: &str,
    ) -> Option<TypeDefinitionLocation>;

    /// Получить все locations определений методов/функций (для переноса в snapshot’ы).
    ///
    /// Формат:
    /// - `(Some(owner_type), method_name, location)` для методов конфигурации
    /// - `(None, function_name, location)` для глобальных функций
    ///
    /// Важно: возвращает **клон** данных.
    fn get_method_definition_locations_clone(
        &self,
    ) -> Vec<(Option<String>, String, TypeDefinitionLocation)>;

    /// Получить все объекты метаданных указанного вида (Milestone 3.16)
    ///
    /// Возвращает имена объектов без префикса (например, "Контрагенты" вместо "Справочники.Контрагенты")
    ///
    /// # Параметры
    ///
    /// * `kind` - вид метаданных (Catalog, Document, etc.)
    ///
    /// # Возвращает
    ///
    /// Вектор имён объектов метаданных
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// # use bsl_shared::domain::repository::TypeRepository;
    /// # use bsl_shared::domain::types::MetadataKind;
    /// # let repository: &dyn TypeRepository = todo!();
    /// let catalogs = repository.get_metadata_objects_by_kind(MetadataKind::Catalog);
    /// // → ["Контрагенты", "Номенклатура", "Склады", ...]
    /// # let _ = catalogs;
    /// ```
    fn get_metadata_objects_by_kind(&self, kind: MetadataKind) -> Vec<String>;

    /// Найти метод по имени типа и имени метода из signature_index
    ///
    /// Использует обогащённые сигнатуры из signature_index, где return_type корректный.
    /// Поддерживает фасетные типы с fallback к базовому типу.
    ///
    /// # Параметры
    ///
    /// * `owner_type` - имя типа-владельца (например, "СправочникМенеджер.Контрагенты")
    /// * `method_name` - имя метода
    ///
    /// # Возвращает
    ///
    /// `Some(MethodSignature)` если метод найден, иначе `None`
    fn find_method(
        &self,
        owner_type: Option<&str>,
        method_name: &str,
    ) -> Option<crate::domain::signature_index::MethodSignature>;

    /// Получить все методы для указанного типа из signature_index
    ///
    /// Возвращает методы с обогащёнными сигнатурами (включая return_type).
    /// Поддерживает фасетные типы с fallback к базовому типу.
    ///
    /// # Параметры
    ///
    /// * `type_name` - имя типа (например, "СправочникМенеджер" или "СправочникМенеджер.Контрагенты")
    ///
    /// # Возвращает
    ///
    /// Вектор сигнатур методов (клонированных)
    fn get_methods_from_signature_index(
        &self,
        type_name: &str,
    ) -> Vec<crate::domain::signature_index::MethodSignature>;

    /// Установить GenericInfo для типа (Milestone 3.x: Унификация источников данных)
    ///
    /// Применяет GenericInfo к существующему типу в репозитории.
    /// Используется для добавления inference metadata к типам из syntax_helper.
    ///
    /// # Параметры
    ///
    /// * `type_name` - имя типа (например, "Массив", "Соответствие")
    /// * `generic_info` - метаданные для Generic inference
    ///
    /// # Возвращает
    ///
    /// `true` если тип найден и GenericInfo установлен, `false` если тип не найден
    fn set_generic_info(
        &self,
        type_name: &str,
        generic_info: crate::domain::types::GenericInfo,
    ) -> bool;
}

/// Статистика репозитория
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryStats {
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
    pub user_defined_types: usize,
    pub last_update_time: Option<String>, // ISO 8601 timestamp
}

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
    /// # use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// # use bsl_shared::domain::signature_registry::{SignatureDataSource, SignatureSourceRegistry};
    /// # use bsl_shared::domain::types::RawTypeData;
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
        actual_params: &[crate::domain::types::ParameterInfo],
        actual_return_type: Option<&str>,
    ) -> SignatureValidationResult {
        use crate::domain::signature_index::{MethodSignature, SignatureSource};

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
            crate::domain::signature_index::ContextRequirements::default(),
        );

        // Валидируем
        index.validate_overloaded_signature(&expected_overloads, &actual_signature)
    }

    fn find_method_signature(
        &self,
        owner_type: Option<&str>,
        method_name: &str,
    ) -> Option<crate::domain::signature_index::MethodSignature> {
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

    fn find_constructor(
        &self,
        type_name: &str,
    ) -> Option<crate::domain::signature_index::ConstructorSignature> {
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

    fn add_config_method_signature(
        &self,
        owner_type: &str,
        method: crate::domain::signature_index::MethodSignature,
    ) {
        let mut index = self.signature_index.write().unwrap_or_else(|poisoned| {
            tracing::warn!(
                "SignatureIndex RwLock poisoned in add_config_method_signature, recovering"
            );
            poisoned.into_inner()
        });
        index.add_config_method(TypeId::new(owner_type), method);
    }

    fn add_global_function_signature(
        &self,
        function_name: &str,
        method: crate::domain::signature_index::MethodSignature,
    ) {
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

    fn find_method(
        &self,
        owner_type: Option<&str>,
        method_name: &str,
    ) -> Option<crate::domain::signature_index::MethodSignature> {
        self.find_method_signature(owner_type, method_name)
    }

    fn get_methods_from_signature_index(
        &self,
        type_name: &str,
    ) -> Vec<crate::domain::signature_index::MethodSignature> {
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
        if let Some(base_type) = super::facet_utils::extract_base_facet_type_universal(type_name) {
            let base_methods = index.get_type_methods(base_type);
            return base_methods.into_iter().cloned().collect();
        }

        vec![]
    }

    fn set_generic_info(
        &self,
        type_name: &str,
        generic_info: crate::domain::types::GenericInfo,
    ) -> bool {
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
mod tests {
    use super::*;
    use crate::domain::types::RawDataSource;

    #[test]
    fn test_find_type_by_camel_alias() {
        let repo = InMemoryTypeRepository::new();

        // Загружаем тип с пробелом в имени
        let type_data = RawTypeData {
            name: "Табличная часть".to_string(),
            english_name: "TabularSection".to_string(),
            source: RawDataSource::Platform,
            ..Default::default()
        };
        repo.load_types(vec![type_data]).unwrap();

        // Все варианты должны находить один тип благодаря нормализации TypeId:
        // - Оригинальное имя с пробелом
        assert!(repo.find_type("Табличная часть").is_some());
        assert_eq!(
            repo.find_type("Табличная часть").unwrap().name,
            "Табличная часть"
        );

        // - CamelCase вариант (нормализуется к тому же ключу)
        assert!(repo.find_type("ТабличнаяЧасть").is_some());
        assert_eq!(
            repo.find_type("ТабличнаяЧасть").unwrap().name,
            "Табличная часть"
        );

        // - lowercase вариант
        assert!(repo.find_type("табличная часть").is_some());
        assert_eq!(
            repo.find_type("табличная часть").unwrap().name,
            "Табличная часть"
        );

        // - Английское имя
        assert!(repo.find_type("TabularSection").is_some());
        assert_eq!(
            repo.find_type("TabularSection").unwrap().name,
            "Табличная часть"
        );

        // - lowercase английское
        assert!(repo.find_type("tabularsection").is_some());
    }

    #[test]
    fn test_type_index_not_overwrites_existing() {
        let repo = InMemoryTypeRepository::new();

        // Два типа с одинаковым нормализованным именем (разный регистр)
        let type1 = RawTypeData {
            name: "Тест алиас".to_string(),
            english_name: "TestAlias1".to_string(),
            source: RawDataSource::Platform,
            ..Default::default()
        };
        let type2 = RawTypeData {
            name: "ТЕСТ АЛИАС".to_string(), // Нормализуется к тому же ключу
            english_name: "TestAlias2".to_string(),
            source: RawDataSource::Platform,
            ..Default::default()
        };

        repo.load_types(vec![type1]).unwrap();
        repo.load_types(vec![type2]).unwrap();

        // Поиск должен вернуть первый загруженный тип (entry().or_insert)
        let found = repo.find_type("ТестАлиас");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Тест алиас");

        // Через разные варианты написания тоже возвращается первый тип
        let found = repo.find_type("тесталиас");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Тест алиас");
    }

    #[test]
    fn test_find_type_with_generic_params() {
        let repo = InMemoryTypeRepository::new();

        let type_data = RawTypeData {
            name: "Массив".to_string(),
            english_name: "Array".to_string(),
            source: RawDataSource::Platform,
            ..Default::default()
        };
        repo.load_types(vec![type_data]).unwrap();

        // Поиск с generic параметрами должен работать
        let found = repo.find_type("Массив<Строка>");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Массив");

        // И без параметров тоже
        let found = repo.find_type("Массив");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Массив");
    }

    #[test]
    fn test_find_type_by_english_name() {
        let repo = InMemoryTypeRepository::new();

        let type_data = RawTypeData {
            name: "Строка".to_string(),
            english_name: "String".to_string(),
            source: RawDataSource::Platform,
            ..Default::default()
        };
        repo.load_types(vec![type_data]).unwrap();

        // Поиск по русскому имени
        assert!(repo.find_type("Строка").is_some());

        // Поиск по английскому имени (регистронезависимо)
        assert!(repo.find_type("String").is_some());
        assert!(repo.find_type("string").is_some());
        assert!(repo.find_type("STRING").is_some());
    }

    #[test]
    fn test_empty_repository() {
        let repo = InMemoryTypeRepository::new();

        assert!(repo.find_type("НесуществующийТип").is_none());
        assert_eq!(repo.get_all_types().len(), 0);
    }

    #[test]
    fn test_upsert_types_updates_existing() {
        let repo = InMemoryTypeRepository::new();

        let type_data = RawTypeData {
            name: "Документ".to_string(),
            english_name: "Document".to_string(),
            description: "old".to_string(),
            source: RawDataSource::Configuration,
            ..Default::default()
        };
        repo.load_types(vec![type_data]).unwrap();

        let updated = RawTypeData {
            name: "Документ".to_string(),
            english_name: "Document".to_string(),
            description: "new".to_string(),
            source: RawDataSource::Configuration,
            ..Default::default()
        };
        repo.upsert_types(vec![updated]).unwrap();

        let found = repo.find_type("Документ").unwrap();
        assert_eq!(found.description, "new");
    }

    #[test]
    fn test_remove_types_removes_indexed_entries() {
        let repo = InMemoryTypeRepository::new();

        let type_a = RawTypeData {
            name: "ТипА".to_string(),
            english_name: "TypeA".to_string(),
            source: RawDataSource::Configuration,
            ..Default::default()
        };
        let type_b = RawTypeData {
            name: "ТипБ".to_string(),
            english_name: "TypeB".to_string(),
            source: RawDataSource::Configuration,
            ..Default::default()
        };
        repo.load_types(vec![type_a, type_b]).unwrap();

        let removed = repo.remove_types(&["ТипА".to_string()]).unwrap();
        assert_eq!(removed, 1);
        assert!(repo.find_type("ТипА").is_none());
        assert!(repo.find_type("TypeA").is_none());
        assert!(repo.find_type("ТипБ").is_some());
    }

    #[test]
    fn test_remove_signatures_by_name() {
        use crate::domain::runtime_context::ContextRequirements;
        use crate::domain::signature_index::{MethodSignature, SignatureSource};

        let repo = InMemoryTypeRepository::new();
        let owner = "СправочникМенеджер.Контрагенты";

        let sig = MethodSignature::new(
            "Тест".to_string(),
            Some(owner.to_string()),
            vec![],
            None,
            None,
            None,
            SignatureSource::Configuration,
            None,
            ContextRequirements::default(),
        );
        repo.add_config_method_signature(owner, sig);
        assert!(repo.find_method_signature(Some(owner), "Тест").is_some());

        repo.remove_config_method_signatures(owner, &["Тест".to_string()]);
        assert!(repo.find_method_signature(Some(owner), "Тест").is_none());

        let global_sig = MethodSignature::new(
            "Глобальная".to_string(),
            None,
            vec![],
            None,
            None,
            None,
            SignatureSource::Configuration,
            None,
            ContextRequirements::default(),
        );
        repo.add_global_function_signature("Глобальная", global_sig);
        assert!(repo.find_method_signature(None, "Глобальная").is_some());

        repo.remove_global_function_signatures(&["Глобальная".to_string()]);
        assert!(repo.find_method_signature(None, "Глобальная").is_none());
    }
}
