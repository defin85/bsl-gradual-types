//! Web Type Service - сервис для веб-интерфейса с богатыми данными

use anyhow::Result;

// ✅ УБИРАЕМ неправильный импорт - теперь resolvers приватен
// use crate::domain::resolvers::platform::PlatformTypeResolver; // ❌ НЕ КОМПИЛИРУЕТСЯ
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
    /// Центральный сервис разрешения типов (обязательный!)
    resolution_service: std::sync::Arc<crate::domain::repository::TypeResolutionService>,
}

impl WebTypeService {
    /// Создать новый веб сервис с обязательным resolution_service
    pub fn new(
        resolution_service: std::sync::Arc<crate::domain::repository::TypeResolutionService>,
    ) -> Self {
        Self { resolution_service }
    }

    /// Поиск типов для веб-интерфейса
    pub async fn search_types(&self, query: &str) -> Result<Vec<String>> {
        // ✅ ИСПОЛЬЗУЕМ только resolution_service - никаких fallback'ов!
        let results = self.resolution_service.search_types(query);
        Ok(results)
    }

    /// Получить детали типа
    pub async fn get_type_details(&self, type_name: &str) -> Result<Option<TypeResolution>> {
        // ✅ ИСПОЛЬЗУЕМ только resolution_service - никаких fallback'ов!
        let platform_globals = self.resolution_service.get_all_platform_globals();
        Ok(platform_globals.get(type_name).cloned())
    }

    /// Получить детальную информацию о типе для веб UI
    pub async fn get_type_completions(
        &self,
        expression: &str,
    ) -> Result<Vec<crate::domain::CompletionItem>> {
        // ✅ ИСПОЛЬЗУЕМ только resolution_service - никаких fallback'ов!
        let completions = self.resolution_service.get_completions(expression);
        Ok(completions)
    }
    /// Построить иерархию типов
    /// Построить иерархию типов с использованием переданных данных
    pub async fn build_type_hierarchy_with_types(
        &self,
        all_types: &std::collections::HashMap<String, TypeResolution>,
    ) -> Result<TypeHierarchy> {
        // Группируем типы по категориям
        let mut hierarchy = TypeHierarchy::default();
        let mut platform_types = Vec::new();

        for (name, _resolution) in all_types {
            platform_types.push(name.clone());

            let category_name = if name.starts_with("Справочники") || name.starts_with("Catalogs")
            {
                "Справочники"
            } else if name.starts_with("Документы") || name.starts_with("Documents") {
                "Документы"
            } else if name.starts_with("Перечисления") || name.starts_with("Enums") {
                "Перечисления"
            } else if name.starts_with("Регистры") || name.contains("Registers") {
                "Регистры"
            } else {
                "Глобальные объекты"
            };

            // Добавляем в соответствующую категорию
            if !hierarchy
                .categories
                .iter()
                .any(|c| &c.name == category_name)
            {
                hierarchy.categories.push(super::services::TypeCategory {
                    id: category_name.to_lowercase().replace(" ", "_"),
                    name: category_name.to_string(),
                    description: format!("Категория {}", category_name),
                    types: Vec::new(),
                    subcategories: Vec::new(),
                });
            }

            // Находим категорию и добавляем тип
            if let Some(cat) = hierarchy
                .categories
                .iter_mut()
                .find(|c| &c.name == category_name)
            {
                cat.types.push(name.clone());
            }
        }

        // Обновляем статистику
        hierarchy.total_types = platform_types.len();
        hierarchy.root_types = platform_types;
        hierarchy.statistics.total_categories = hierarchy.categories.len();
        hierarchy.statistics.total_types = hierarchy.total_types;
        hierarchy.statistics.platform_types = hierarchy.total_types;
        hierarchy.statistics.configuration_types = 0;

        Ok(hierarchy)
    }

    pub async fn build_type_hierarchy(&self) -> Result<TypeHierarchy> {
        // ✅ ИСПОЛЬЗУЕМ только resolution_service - никаких fallback'ов!
        let platform_globals = self.resolution_service.get_all_platform_globals();
        self.build_type_hierarchy_with_types(platform_globals).await
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
