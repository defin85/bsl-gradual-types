//! Frontend-specific extensions for shared DTOs
//!
//! This module provides UI-specific functionality (colors, formatting, filters)
//! that extends the core DTOs from bsl-shared without duplicating them.

use bsl_shared::api::dtos::{CategoryDto, MethodDto, MetricsDto, PaginationDto, ParamDto, TypeDto};

// Type aliases for backward compatibility during transition
pub type TypeInfo = TypeDto;
pub type TypeMetrics = MetricsDto;
pub type PaginationInfo = PaginationDto;
pub type MethodInfo = MethodDto;
pub type ParamInfo = ParamDto;

/// Frontend-specific methods for TypeDto
pub trait TypeDtoExt {
    fn display_name(&self) -> &str;
    fn certainty_color(&self) -> &'static str;
    fn certainty_percentage(&self) -> String;
    fn facet_colors(&self) -> Vec<(&str, &'static str)>;
    fn get_category(&self) -> &str;
    fn get_certainty(&self) -> u8;
    fn is_flow_sensitive(&self) -> bool;
}

impl TypeDtoExt for TypeDto {
    fn display_name(&self) -> &str {
        &self.name
    }

    fn certainty_color(&self) -> &'static str {
        match self.certainty {
            90..=100 => "#28a745", // Green - Known
            50..=89 => "#ffc107",  // Yellow - Inferred
            _ => "#dc3545",        // Red - Unknown
        }
    }

    fn certainty_percentage(&self) -> String {
        format!("{}%", self.certainty)
    }

    fn facet_colors(&self) -> Vec<(&str, &'static str)> {
        self.facets
            .iter()
            .map(|f| {
                let color = match f.as_str() {
                    "Manager" => "#007bff",    // Blue
                    "Object" => "#28a745",     // Green
                    "Reference" => "#ffc107",  // Yellow
                    "Collection" => "#17a2b8", // Cyan
                    "Selection" => "#e83e8c",  // Pink
                    "List" => "#6f42c1",       // Purple
                    "Metadata" => "#6c757d",   // Gray
                    _ => "#6c757d",            // Default gray
                };
                (f.as_str(), color)
            })
            .collect()
    }

    fn get_category(&self) -> &str {
        &self.category
    }

    fn get_certainty(&self) -> u8 {
        self.certainty
    }

    fn is_flow_sensitive(&self) -> bool {
        self.flow_sensitive
    }
}

/// Category colors for UI
pub trait CategoryDtoExt {
    fn get_color(&self, category: &str) -> &'static str;
}

impl CategoryDtoExt for CategoryDto {
    fn get_color(&self, category: &str) -> &'static str {
        match category {
            "Platform" => "#007bff",      // Blue
            "Configuration" => "#28a745", // Green
            "Union" => "#ffc107",         // Yellow
            "Dynamic" => "#dc3545",       // Red
            "Примитивный" => "#4CAF50",   // Green
            "Платформа" => "#2196F3",     // Blue
            "Справочник" => "#FF9800",    // Orange
            "Документ" => "#9C27B0",      // Purple
            "Перечисление" => "#00BCD4",  // Cyan
            _ => "#6c757d",               // Gray
        }
    }
}

/// Фильтры для поиска типов (Frontend-specific)
#[derive(Debug, Clone)]
pub struct TypeFilters {
    pub search_query: Option<String>,
    // Категории - отдельные флаги вместо Option<TypeCategory>
    pub show_platform: bool,
    pub show_configuration: bool,
    pub show_union: bool,
    pub show_dynamic: bool,
    // Уровни определённости - отдельные флаги
    pub show_high_certainty: bool,
    pub show_medium_certainty: bool,
    pub show_low_certainty: bool,
    pub facet: Option<String>,
    pub flow_sensitive_only: bool,
    pub page: usize,
    pub page_size: usize,
}

impl Default for TypeFilters {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeFilters {
    pub fn new() -> Self {
        Self {
            search_query: None,
            // По умолчанию все флаги false = показать всё
            show_platform: false,
            show_configuration: false,
            show_union: false,
            show_dynamic: false,
            // По умолчанию все уровни false = показать всё
            show_high_certainty: false,
            show_medium_certainty: false,
            show_low_certainty: false,
            facet: None,
            flow_sensitive_only: false,
            page: 1,
            page_size: 50,
        }
    }

    /// Проверка, соответствует ли тип фильтрам
    pub fn matches(&self, type_dto: &TypeDto) -> bool {
        // Search query
        if let Some(ref query) = self.search_query {
            if !type_dto.name.to_lowercase().contains(&query.to_lowercase()) {
                return false;
            }
        }

        // Category filters (если все false - показать всё)
        let has_category_filter =
            self.show_platform || self.show_configuration || self.show_union || self.show_dynamic;
        if has_category_filter {
            let matches_category = match type_dto.category.as_str() {
                "Platform" => self.show_platform,
                "Configuration" => self.show_configuration,
                "Union" => self.show_union,
                "Dynamic" => self.show_dynamic,
                _ => false,
            };
            if !matches_category {
                return false;
            }
        }

        // Certainty filters (если все false - показать всё)
        let has_certainty_filter =
            self.show_high_certainty || self.show_medium_certainty || self.show_low_certainty;
        if has_certainty_filter {
            let matches_certainty = match type_dto.certainty {
                90..=100 => self.show_high_certainty,
                50..=89 => self.show_medium_certainty,
                _ => self.show_low_certainty,
            };
            if !matches_certainty {
                return false;
            }
        }

        // Facet filter
        if let Some(ref facet) = self.facet {
            if !type_dto.facets.contains(facet) {
                return false;
            }
        }

        // Flow-sensitive filter
        if self.flow_sensitive_only && !type_dto.flow_sensitive {
            return false;
        }

        true
    }
}

/// Connection type colors for graph visualization
pub fn connection_type_color(connection_type: &str) -> &'static str {
    match connection_type {
        "dependency" => "rgba(255,255,255,0.3)",
        "inheritance" => "#007bff",
        "composition" => "#28a745",
        "flow" => "#f59e0b",
        "reference" => "#6f42c1",
        _ => "rgba(255,255,255,0.2)",
    }
}

/// Helper function to format metrics
pub trait MetricsDtoExt {
    fn format_cache_hit_rate(&self) -> String;
    fn format_analysis_speed(&self) -> String;
    // Backward compatibility fields
    fn known_types(&self) -> usize;
    fn inferred_types(&self) -> usize;
    fn unknown_types(&self) -> usize;
    fn flow_sensitive_types(&self) -> usize;
    fn analysis_speed_ms(&self) -> f32;
}

impl MetricsDtoExt for MetricsDto {
    fn format_cache_hit_rate(&self) -> String {
        self.cache_hit_rate.clone()
    }

    fn format_analysis_speed(&self) -> String {
        self.analysis_speed.clone()
    }

    fn known_types(&self) -> usize {
        self.certainty_high
    }

    fn inferred_types(&self) -> usize {
        self.certainty_medium
    }

    fn unknown_types(&self) -> usize {
        self.certainty_low
    }

    fn flow_sensitive_types(&self) -> usize {
        self.flow_sensitive
    }

    fn analysis_speed_ms(&self) -> f32 {
        // Parse "125ms" -> 125.0
        self.analysis_speed
            .trim_end_matches("ms")
            .parse()
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certainty_color() {
        let type_dto = TypeDto {
            id: "test".to_string(),
            name: "TestType".to_string(),
            category: "Platform".to_string(),
            certainty: 100,
            certainty_text: "Known 100%".to_string(),
            facets: vec![],
            methods_count: None,
            methods: vec![],
            attributes_count: None,
            properties: vec![],
            enum_values: None,
            tabular_sections: vec![],
            source: "Test".to_string(),
            flow_sensitive: false,
            description: "Test".to_string(),
            union_types: None,
            flow_analysis: None,
            connections: None,
            warning: None,
            recommendation: None,
        };

        assert_eq!(type_dto.certainty_color(), "#28a745"); // Green for Known
    }

    #[test]
    fn test_filters_matches() {
        let type_dto = TypeDto {
            id: "test".to_string(),
            name: "Массив".to_string(),
            category: "Platform".to_string(),
            certainty: 100,
            certainty_text: "Known 100%".to_string(),
            facets: vec!["Collection".to_string()],
            methods_count: None,
            methods: vec![],
            attributes_count: None,
            properties: vec![],
            enum_values: None,
            tabular_sections: vec![],
            source: "Platform".to_string(),
            flow_sensitive: false,
            description: "Array".to_string(),
            union_types: None,
            flow_analysis: None,
            connections: None,
            warning: None,
            recommendation: None,
        };

        let mut filters = TypeFilters::new();
        assert!(filters.matches(&type_dto)); // Default matches all

        filters.search_query = Some("Масс".to_string());
        assert!(filters.matches(&type_dto));

        filters.search_query = Some("Строка".to_string());
        assert!(!filters.matches(&type_dto));
    }
}
