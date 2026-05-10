//! Get All Types command handler
//!
//! Handles bsl.getAllTypes custom command for TreeView.

use std::sync::Arc;
use tracing::info;

use bsl_backend::system::DomainBundle;
use bsl_shared::api::dtos::{
    AnalysisResultDto, CategoryDto, MethodDto, MetricsDto, PaginationDto, ParamDto, TypeDto,
};
use bsl_shared::domain::types::{RawDataSource, RawTypeData};

/// Request for bsl.getAllTypes
#[derive(Debug, serde::Deserialize)]
pub struct GetAllTypesRequest {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    pub category: Option<String>,
}

fn default_limit() -> usize {
    1000
}

/// Handle bsl.getAllTypes command
pub fn handle_get_all_types(
    params: GetAllTypesRequest,
    domain: Option<Arc<DomainBundle>>,
) -> AnalysisResultDto {
    info!(
        "Custom command: bsl.getAllTypes - limit: {}, offset: {}, category: {:?}",
        params.limit, params.offset, params.category
    );

    match domain {
        Some(bundle) => {
            let repository = bundle.repository.clone();
            let all_types: Vec<RawTypeData> = repository.get_all_types();

            let mut dtos: Vec<TypeDto> = all_types.iter().map(raw_type_to_dto).collect();

            if let Some(ref category) = params.category {
                dtos.retain(|dto| dto.category == *category);
            }

            let total_items = dtos.len();
            let limit = params.limit.max(1);
            let offset = params.offset.min(total_items);

            let paged: Vec<TypeDto> = dtos.iter().skip(offset).take(limit).cloned().collect();

            let metrics = MetricsDto {
                total_types: total_items,
                certainty_high: paged.len(),
                certainty_medium: 0,
                certainty_low: 0,
                flow_sensitive: 0,
                cache_hit_rate: "0%".to_string(),
                analysis_speed: "N/A".to_string(),
            };

            let pagination = Some(PaginationDto {
                current_page: (offset / limit) + 1,
                page_size: limit,
                total_items,
                total_pages: total_items.div_ceil(limit),
                has_prev: offset > 0,
                has_next: offset + limit < total_items,
            });

            let mut categories = std::collections::HashMap::new();
            let platform_count = paged.iter().filter(|t| t.source == "Platform").count();
            let config_count = paged.iter().filter(|t| t.source == "Configuration").count();

            if platform_count > 0 {
                categories.insert(
                    "Platform".to_string(),
                    CategoryDto {
                        color: "#3498db".to_string(),
                        icon: "🔧".to_string(),
                        count: platform_count,
                    },
                );
            }

            if config_count > 0 {
                categories.insert(
                    "Configuration".to_string(),
                    CategoryDto {
                        color: "#e74c3c".to_string(),
                        icon: "⚙️".to_string(),
                        count: config_count,
                    },
                );
            }

            AnalysisResultDto {
                types: paged,
                categories,
                metrics,
                connections: Vec::new(),
                pagination,
            }
        }
        None => {
            tracing::warn!("AnalysisEngine not available");
            AnalysisResultDto {
                types: vec![],
                categories: std::collections::HashMap::new(),
                metrics: MetricsDto {
                    total_types: 0,
                    certainty_high: 0,
                    certainty_medium: 0,
                    certainty_low: 0,
                    flow_sensitive: 0,
                    cache_hit_rate: "0%".to_string(),
                    analysis_speed: "N/A".to_string(),
                },
                connections: vec![],
                pagination: None,
            }
        }
    }
}

fn raw_type_to_dto(raw: &RawTypeData) -> TypeDto {
    let methods: Vec<MethodDto> = raw
        .methods
        .iter()
        .map(|method| MethodDto {
            name: method.name.clone(),
            english_name: if method.english_name.is_empty() {
                None
            } else {
                Some(method.english_name.clone())
            },
            return_type: if method.return_type.is_empty() {
                None
            } else {
                Some(method.return_type.clone())
            },
            params: method
                .params
                .iter()
                .map(|param| ParamDto {
                    name: param.name.clone(),
                    param_type: if param.param_type.is_empty() {
                        "Произвольный".to_string()
                    } else {
                        param.param_type.clone()
                    },
                    is_optional: param.is_optional,
                    default_value: param.default_value.clone(),
                })
                .collect(),
            description: method.description.clone(),
            is_deprecated: method.is_deprecated,
            is_constructor: method.is_constructor,
        })
        .collect();

    let properties: Vec<String> = raw
        .properties
        .iter()
        .map(|prop| prop.name.clone())
        .collect();

    let tabular_sections = raw
        .tabular_sections
        .iter()
        .map(|section| bsl_shared::api::dtos::TabularSectionDto {
            name: section.name.clone(),
            attributes: section
                .attributes
                .iter()
                .map(|attr| bsl_shared::api::dtos::TabularSectionAttributeDto {
                    name: attr.name.clone(),
                    attr_type: Some(attr.attr_type.clone()),
                })
                .collect(),
        })
        .collect();

    let source = match raw.source {
        RawDataSource::Platform => "Platform",
        RawDataSource::Configuration => "Configuration",
        RawDataSource::UserDefined => "UserDefined",
    };

    let category = if matches!(raw.source, RawDataSource::Configuration) {
        raw.kind
            .map(|kind| format!("{:?}", kind))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| {
                if raw.category.is_empty() {
                    "Other".to_string()
                } else {
                    raw.category.clone()
                }
            })
    } else if raw.category.is_empty() {
        "Other".to_string()
    } else {
        raw.category.clone()
    };

    let methods_count = if methods.is_empty() {
        None
    } else {
        Some(methods.len())
    };
    let attributes_count = if raw.attributes.is_empty() {
        None
    } else {
        Some(raw.attributes.len())
    };

    TypeDto {
        id: raw.name.clone(),
        name: raw.name.clone(),
        category,
        certainty: 100,
        certainty_text: "Known 100%".to_string(),
        facets: raw
            .facets
            .iter()
            .map(|facet| facet.display_name().to_string())
            .collect(),
        methods_count,
        methods,
        attributes_count,
        properties,
        enum_values: if raw.enum_values.is_empty() {
            None
        } else {
            Some(raw.enum_values.clone())
        },
        tabular_sections,
        source: source.to_string(),
        flow_sensitive: false,
        description: raw.description.clone(),
        union_types: None,
        flow_analysis: None,
        connections: None,
        warning: None,
        recommendation: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_backend::system::DomainBundle;
    use bsl_shared::domain::{
        repository::InMemoryTypeRepository,
        types::{MetadataKind, RawDataSource, RawTypeData},
        GlobalContextIndex, TypeRepository, TypeResolver,
    };

    fn domain_bundle_with_types(types: Vec<RawTypeData>) -> Arc<DomainBundle> {
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository.load_types(types).expect("load test types");

        Arc::new(DomainBundle {
            repository: repository.clone(),
            resolver: Arc::new(TypeResolver::new(repository)),
            global_context_index: Arc::new(GlobalContextIndex::unavailable()),
        })
    }

    fn platform_type(name: &str, category: &str) -> RawTypeData {
        RawTypeData {
            name: name.to_string(),
            category: category.to_string(),
            source: RawDataSource::Platform,
            description: format!("{name} description"),
            ..RawTypeData::default()
        }
    }

    fn configuration_type(name: &str, kind: MetadataKind) -> RawTypeData {
        RawTypeData {
            name: name.to_string(),
            category: "IgnoredFallback".to_string(),
            source: RawDataSource::Configuration,
            kind: Some(kind),
            description: format!("{name} description"),
            ..RawTypeData::default()
        }
    }

    #[test]
    fn get_all_types_returns_paged_item_level_group_fields() {
        let domain = domain_bundle_with_types(vec![
            platform_type("PlatformOne", "Collections"),
            platform_type("PlatformTwo", "Collections"),
            configuration_type("Catalog.Products", MetadataKind::Catalog),
        ]);

        let result = handle_get_all_types(
            GetAllTypesRequest {
                limit: 2,
                offset: 0,
                category: None,
            },
            Some(domain),
        );

        assert_eq!(result.types.len(), 2);
        assert_eq!(result.metrics.total_types, 3);

        let first = &result.types[0];
        assert_eq!(first.name, "PlatformOne");
        assert_eq!(first.source, "Platform");
        assert_eq!(first.category, "Collections");

        let pagination = result.pagination.expect("paged response metadata");
        assert_eq!(pagination.page_size, 2);
        assert_eq!(pagination.total_items, 3);
        assert_eq!(pagination.total_pages, 2);
        assert!(pagination.has_next);
    }

    #[test]
    fn get_all_types_category_filter_uses_item_category_and_totals() {
        let domain = domain_bundle_with_types(vec![
            platform_type("Array", "Collections"),
            platform_type("ValueTable", "Collections"),
            platform_type("String", "Primitives"),
        ]);

        let result = handle_get_all_types(
            GetAllTypesRequest {
                limit: 100,
                offset: 0,
                category: Some("Collections".to_string()),
            },
            Some(domain),
        );

        assert_eq!(result.metrics.total_types, 2);
        assert_eq!(
            result
                .types
                .iter()
                .map(|ty| (ty.name.as_str(), ty.source.as_str(), ty.category.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("Array", "Platform", "Collections"),
                ("ValueTable", "Platform", "Collections"),
            ]
        );
        assert_eq!(
            result.pagination.expect("filtered pagination").total_items,
            2
        );
    }

    #[test]
    fn get_all_types_empty_domain_is_well_formed_empty_result() {
        let result = handle_get_all_types(
            GetAllTypesRequest {
                limit: 100,
                offset: 0,
                category: None,
            },
            None,
        );

        assert!(result.types.is_empty());
        assert!(result.categories.is_empty());
        assert_eq!(result.metrics.total_types, 0);
        assert!(result.pagination.is_none());
    }
}
