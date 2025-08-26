//! Type repository trait and implementations

use anyhow::Result;
use std::sync::Arc;
use crate::domain::search::RawTypeData;

/// Trait для репозитория типов
pub trait TypeRepository: Send + Sync {
    /// Получить все типы платформы
    fn get_platform_types(&self) -> Result<Vec<RawTypeData>>;
    
    /// Получить типы конфигурации
    fn get_configuration_types(&self, config_path: &str) -> Result<Vec<RawTypeData>>;
}

/// In-memory реализация репозитория для тестирования
pub struct InMemoryTypeRepository {
    // TODO: Implement after migration complete
}

impl InMemoryTypeRepository {
    pub fn new() -> Self {
        Self {}
    }
}

impl TypeRepository for InMemoryTypeRepository {
    fn get_platform_types(&self) -> Result<Vec<RawTypeData>> {
        // TODO: Implement after migration complete
        Ok(vec![])
    }
    
    fn get_configuration_types(&self, _config_path: &str) -> Result<Vec<RawTypeData>> {
        // TODO: Implement after migration complete
        Ok(vec![])
    }
}

/// Сервис разрешения типов
pub struct TypeResolutionService {
    repository: Arc<dyn TypeRepository>,
}

impl TypeResolutionService {
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        Self { repository }
    }
}

/// Сервис проверки типов
pub struct TypeCheckerService {
    // TODO: Implement after migration complete
}

impl TypeCheckerService {
    pub fn new() -> Self {
        Self {}
    }
}
