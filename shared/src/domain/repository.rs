//! Data Layer: Type Repository trait and implementations

use crate::domain::signature_index::{SignatureIndex, SignatureValidationResult};
use crate::domain::types::{MetadataKind, RawDataSource, RawTypeData};
use anyhow::Result;
use chrono::{DateTime, Utc};
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
}

// --- Type Repository Trait ---

/// Trait для репозитория типов
pub trait TypeRepository: Send + Sync {
    /// Загрузить типы в репозиторий
    fn load_types(&self, types: Vec<RawTypeData>) -> Result<()>;

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
    /// ```ignore
    /// let catalogs = repository.get_metadata_objects_by_kind(MetadataKind::Catalog);
    /// // → ["Контрагенты", "Номенклатура", "Склады", ...]
    /// ```
    fn get_metadata_objects_by_kind(&self, kind: MetadataKind) -> Vec<String>;
}

/// Статистика репозитория
#[derive(Debug, Clone, Default, serde::Serialize)]
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
    /// Индекс сигнатур методов для валидации (thread-safe)
    signature_index: RwLock<SignatureIndex>,
}

impl InMemoryTypeRepository {
    pub fn new() -> Self {
        Self {
            types: RwLock::new(Vec::new()),
            last_updated: RwLock::new(None),
            signature_index: RwLock::new(SignatureIndex::new()),
        }
    }

    /// Получить мутабельный доступ к SignatureIndex для заполнения
    ///
    /// Используется при загрузке платформенных типов для заполнения индекса
    pub fn populate_signature_index<F>(&self, populate_fn: F)
    where
        F: FnOnce(&mut SignatureIndex),
    {
        let mut index = self.signature_index.write().unwrap();
        populate_fn(&mut index);
    }
}

impl Default for InMemoryTypeRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRepository for InMemoryTypeRepository {
    fn load_types(&self, new_types: Vec<RawTypeData>) -> Result<()> {
        let mut types = self.types.write().unwrap();
        types.extend(new_types);

        // Обновляем timestamp
        *self.last_updated.write().unwrap() = Some(SystemTime::now());

        Ok(())
    }

    fn get_all_types(&self) -> Vec<RawTypeData> {
        self.types.read().unwrap().clone()
    }

    fn find_type(&self, name: &str) -> Option<RawTypeData> {
        let types = self.types.read().unwrap();
        let result = types
            .iter()
            .find(|t| t.name == name || t.english_name == name)
            .cloned();

        // DEBUG: Логируем поиск типа
        if result.is_none() {
            tracing::debug!(
                "TypeRepository.find_type('{}') → NOT FOUND (total types: {})",
                name,
                types.len()
            );
            // Показываем первые 5 типов для отладки
            if !types.is_empty() {
                let sample: Vec<String> = types
                    .iter()
                    .take(5)
                    .map(|t| format!("'{}' (en: '{}')", t.name, t.english_name))
                    .collect();
                tracing::debug!("Sample types in repository: {}", sample.join(", "));
            }
        } else {
            tracing::debug!("TypeRepository.find_type('{}') → FOUND", name);
        }

        result
    }

    fn get_stats(&self) -> RepositoryStats {
        let types = self.types.read().unwrap();
        let last_updated = self.last_updated.read().unwrap();

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

        let index = self.signature_index.read().unwrap();

        // Ищем метод в индексе
        let expected_signature = index.find_method(owner_type, method_name);

        // Создаём актуальную сигнатуру
        let actual_signature = MethodSignature::new(
            method_name.to_string(),
            Some(owner_type.to_string()),
            actual_params.to_vec(),
            actual_return_type.map(|s| s.to_string()),
            SignatureSource::UserCode,
            None,
            crate::domain::signature_index::ContextRequirements::default(),
        );

        // Валидируем
        index.validate_signature(expected_signature, &actual_signature)
    }

    fn find_method_signature(
        &self,
        owner_type: Option<&str>,
        method_name: &str,
    ) -> Option<crate::domain::signature_index::MethodSignature> {
        let index = self.signature_index.read().unwrap();

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
        self.signature_index.read().unwrap().clone()
    }

    fn get_metadata_objects_by_kind(&self, kind: MetadataKind) -> Vec<String> {
        let types = self.types.read().unwrap();
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
}
