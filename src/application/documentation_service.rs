//! Сервис документации (application layer)
//!
//! Временная заглушка на время миграции архитектуры
//! TODO: Восстановить функциональность после завершения миграции

// Импорты временно удалены до завершения миграции

// TODO: Restore documentation system imports after migration
// use crate::documentation::core::{
//     BslDocumentationSystem, DocumentationConfig, DocumentationStatistics, InitializationStatus,
//     TypeDocumentationFull, TypeHierarchy,
// };
// use crate::documentation::search::{AdvancedSearchQuery, SearchResults};

/// Временная заглушка для сервиса документации
#[derive(Debug)]
pub struct DocumentationService {
    // documentation_system: Arc<BslDocumentationSystem>,
}

impl DocumentationService {
    /// Создать новый сервис документации (заглушка)
    pub fn new() -> Self {
        Self {}
    }

    /// Создать из системы (заглушка)
    pub fn from_system(/* system: Arc<BslDocumentationSystem> */) -> Self {
        Self {}
    }

    // TODO: Restore all methods after migration
}

impl Default for DocumentationService {
    fn default() -> Self {
        Self::new()
    }
}

