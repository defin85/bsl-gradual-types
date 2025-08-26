//! Web Type Service - сервис для веб-интерфейса с богатыми данными

use anyhow::Result;
use std::sync::Arc;

// Временно используем заглушки пока не создадим сервисы в domain/
// use crate::domain::{TypeResolutionService, TypeSearchResult};
use crate::domain::types::TypeResolution;

/// Сервис типов для веб-интерфейса (богатые данные)
pub struct WebTypeService {
    // /// Центральный сервис разрешения
    // resolution_service: Arc<TypeResolutionService>,
}

impl WebTypeService {
    /// Создать новый веб сервис
    pub fn new(/* resolution_service: Arc<TypeResolutionService> */) -> Self {
        Self {
            // resolution_service 
        }
    }

    /// Поиск типов для веб-интерфейса (временная заглушка)
    pub async fn search_types(&self, query: &str) -> Result<Vec<TypeSearchResult>> {
        // TODO: Implement when TypeResolutionService is available
        Ok(vec![])
    }

    /// Получить детальную информацию о типе для веб UI (временная заглушка)
    pub async fn get_type_details(&self, expression: &str) -> Result<Option<TypeResolution>> {
        // TODO: Implement when TypeResolutionService is available
        Ok(None)
    }

    /// Получить статистику типов для dashboard
    pub async fn get_type_statistics(&self) -> Result<TypeStatistics> {
        // TODO: Implement statistics collection
        Ok(TypeStatistics::default())
    }
}

/// Статистика типов для веб-интерфейса
#[derive(Debug, Default)]
pub struct TypeStatistics {
    pub total_types: usize,
    pub platform_types: usize,
    pub custom_types: usize,
    pub configuration_types: usize,
}

/// Результат поиска типов (временная заглушка)
#[derive(Debug, Default)]
pub struct TypeSearchResult {
    pub name: String,
    pub description: String,
    pub source: String,
}
