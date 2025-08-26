//! Type repository trait and implementations

use anyhow::Result;
use std::sync::Arc;
use crate::domain::search::RawTypeData;
use crate::domain::types::TypeResolution;
use crate::domain::analysis::type_checker::TypeContext;

/// Trait для репозитория типов
pub trait TypeRepository: Send + Sync {
    /// Получить все типы платформы
    fn get_platform_types(&self) -> Result<Vec<RawTypeData>>;
    
    /// Получить типы конфигурации
    fn get_configuration_types(&self, config_path: &str) -> Result<Vec<RawTypeData>>;
    
    /// Получить статистику репозитория
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
    
    fn get_stats(&self) -> RepositoryStats {
        // TODO: Implement actual stats
        RepositoryStats::default()
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
    
    /// Инициализировать сервис разрешения типов
    pub async fn initialize(&self) -> Result<()> {
        // TODO: Implement initialization logic
        Ok(())
    }
    
    /// Разрешить выражение в типе
    pub async fn resolve_expression(&self, _expression: &str, _context: &TypeContext) -> Result<TypeResolution> {
        // TODO: Implement expression resolution
        Ok(TypeResolution::unknown())
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

    /// Проверить совместимость присваивания типов
    pub fn is_assignment_compatible(&self, _from: &TypeResolution, _to: &TypeResolution) -> bool {
        // TODO: Implement proper type compatibility check
        true
    }
}
