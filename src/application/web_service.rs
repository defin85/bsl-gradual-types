//! Web Type Service - сервис для веб-интерфейса с богатыми данными

use anyhow::Result;
use std::sync::Arc;

// Временно используем заглушки пока не создадим сервисы в domain/
// use crate::domain::{TypeResolutionService, TypeSearchResult};
use crate::domain::types::TypeResolution;
use crate::presentation::SearchFilters;

/// Результат поиска типов
#[derive(Debug, Clone, Default)]
pub struct TypeSearchResult {
    pub name: String,
    pub description: String,
    pub type_name: String,
    pub category: String,
    pub relevance_score: f64,
    pub url: Option<String>,
}

// Используем единое определение TypeHierarchy из services.rs
use super::services::TypeHierarchy;

/// Результаты расширенного поиска
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub total: usize,
    pub items: Vec<TypeSearchResult>,
}

impl SearchResults {
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl std::ops::Index<usize> for SearchResults {
    type Output = TypeSearchResult;

    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

/// Метрики производительности
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub average_response_time: f64,
    pub cache_hit_rate: f64,
    pub active_connections: u32,
}

/// Статистика типов
#[derive(Debug, Clone, Default)]
pub struct TypeStatistics {
    pub total_types: usize,
    pub platform_types: usize,
    pub user_types: usize,
}

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
    pub async fn search_types(&self, _query: &str) -> Result<Vec<TypeSearchResult>> {
        // TODO: Implement when TypeResolutionService is available
        Ok(vec![])
    }

    /// Получить детальную информацию о типе для веб UI (временная заглушка)
    pub async fn get_type_details(&self, _expression: &str) -> Result<Option<TypeResolution>> {
        // TODO: Implement when TypeResolutionService is available
        Ok(None)
    }

    /// Построить иерархию типов
    pub async fn build_type_hierarchy(&self) -> Result<TypeHierarchy> {
        // TODO: Implement type hierarchy building
        Ok(TypeHierarchy::default())
    }

    /// Расширенный поиск типов
    pub async fn advanced_search(
        &self,
        _query: &str,
        _filters: SearchFilters,
    ) -> Result<SearchResults> {
        // TODO: Implement advanced search
        Ok(SearchResults::default())
    }

    /// Получить метрики производительности
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        PerformanceMetrics {
            average_response_time: 0.0,
            cache_hit_rate: 0.0,
            active_connections: 0,
        }
    }

    /// Получить статистику типов для dashboard
    pub async fn get_type_statistics(&self) -> Result<TypeStatistics> {
        // TODO: Implement statistics collection
        Ok(TypeStatistics::default())
    }
}
