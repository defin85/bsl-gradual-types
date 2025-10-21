//! Data Layer: Type Repository trait and implementations

use crate::domain::types::RawTypeData;
use anyhow::Result;
use std::sync::RwLock;

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
}

/// Статистика репозитория
#[derive(Debug, Clone, Default)]
pub struct RepositoryStats {
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
    pub user_defined_types: usize,
}

// --- In-Memory Implementation ---

/// In-memory реализация репозитория
#[derive(Default)]
pub struct InMemoryTypeRepository {
    types: RwLock<Vec<RawTypeData>>,
}

impl InMemoryTypeRepository {
    pub fn new() -> Self {
        Self {
            types: RwLock::new(Vec::new()),
        }
    }
}

impl TypeRepository for InMemoryTypeRepository {
    fn load_types(&self, new_types: Vec<RawTypeData>) -> Result<()> {
        let mut types = self.types.write().unwrap();
        types.extend(new_types);
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
        // TODO: Differentiate between platform, config, etc. based on RawTypeData.source
        RepositoryStats {
            total_types: types.len(),
            platform_types: types.len(), // Placeholder
            configuration_types: 0,      // Placeholder
            user_defined_types: 0,       // Placeholder
        }
    }
}
