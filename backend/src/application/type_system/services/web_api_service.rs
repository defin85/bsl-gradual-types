//! Web API Service - search and type details operations for Web API
//!
//! Functions for searching types and retrieving type details,
//! used by the Web API endpoints.

use anyhow::Result;
use tracing::info;

use bsl_shared::api::dtos::{
    AnalysisResultDto, CategoryDto, MetricsDto, PaginationDto, TabularSectionAttributeDto,
    TabularSectionDto, TypeDto, UnionComponentDto,
};
use bsl_shared::api::{MethodDto, ParamDto};
use bsl_shared::domain::types::{Certainty, ResolutionResult, TypeResolution};
use bsl_shared::domain::{CompletionItem, TypeMetadataLookup};

use crate::application::TypeInferenceService;
use crate::system::AnalysisCache;

/// Searches types by query string
///
/// # Arguments
/// * `inference_service` - TypeInferenceService for type operations
/// * `query` - Search query string
///
/// # Returns
/// List of matching type names
pub async fn search_types(
    inference_service: &TypeInferenceService,
    query: &str,
) -> Result<Vec<String>> {
    info!("🌐 Web search types: {}", query);
    let results = inference_service.search_types(query);
    Ok(results)
}

/// Phase 5: Search types with DTO transformation (Web API)
///
/// # Arguments
/// * `inference_service` - TypeInferenceService for type operations
/// * `metadata_lookup` - Lookup for type metadata
/// * `cache` - Analysis cache for metrics
/// * `query` - Search query string
///
/// # Returns
/// AnalysisResultDto with filtered types
pub async fn search_types_as_dto(
    inference_service: &TypeInferenceService,
    metadata_lookup: &TypeMetadataLookup,
    cache: &AnalysisCache,
    query: &str,
) -> Result<AnalysisResultDto> {
    info!("🌐 Web search types with DTO: {}", query);

    // 1. Get all types and filter by query
    let all_types = inference_service.get_all_platform_globals();
    let query_lower = query.to_lowercase();

    let filtered_types: Vec<(&String, &TypeResolution)> = all_types
        .iter()
        .filter(|(name, _)| name.to_lowercase().contains(&query_lower))
        .collect();

    // 2. Transform to DTO
    let type_dtos: Vec<TypeDto> = filtered_types
        .iter()
        .map(|(name, res)| {
            type_resolution_to_dto(name, res, metadata_lookup, |_| {
                // Generate type description for search results
                generate_type_description(res)
            })
        })
        .collect();

    // 3. Generate metrics
    let metrics = MetricsDto {
        total_types: type_dtos.len(),
        certainty_high: type_dtos.iter().filter(|t| t.certainty > 80).count(),
        certainty_medium: type_dtos
            .iter()
            .filter(|t| t.certainty > 40 && t.certainty <= 80)
            .count(),
        certainty_low: type_dtos.iter().filter(|t| t.certainty <= 40).count(),
        flow_sensitive: type_dtos.iter().filter(|t| t.flow_sensitive).count(),
        cache_hit_rate: format!("{:.1}%", cache.get_hit_rate()),
        analysis_speed: "125ms".to_string(),
    };

    // 4. Generate categories
    let mut categories = std::collections::HashMap::new();
    categories.insert(
        "Platform".to_string(),
        CategoryDto {
            color: "#3498db".to_string(),
            icon: "🔧".to_string(),
            count: type_dtos
                .iter()
                .filter(|t| t.category == "Platform")
                .count(),
        },
    );
    categories.insert(
        "Configuration".to_string(),
        CategoryDto {
            color: "#e74c3c".to_string(),
            icon: "⚙️".to_string(),
            count: type_dtos
                .iter()
                .filter(|t| t.category == "Configuration")
                .count(),
        },
    );

    Ok(AnalysisResultDto {
        types: type_dtos,
        categories,
        metrics,
        connections: vec![],
        pagination: None, // Search without pagination
    })
}

/// Get type details by name
///
/// # Arguments
/// * `inference_service` - TypeInferenceService for type operations
/// * `type_name` - Name of the type to retrieve
///
/// # Returns
/// TypeResolution if found
pub async fn get_type_details(
    inference_service: &TypeInferenceService,
    type_name: &str,
) -> Result<Option<TypeResolution>> {
    info!("🌐 Web type details: {}", type_name);
    let platform_globals = inference_service.get_all_platform_globals();
    Ok(platform_globals.get(type_name).cloned())
}

/// Get type completions for an expression
///
/// # Arguments
/// * `inference_service` - TypeInferenceService for type operations
/// * `expression` - Expression to get completions for
///
/// # Returns
/// List of completion items
pub async fn get_type_completions(
    inference_service: &TypeInferenceService,
    expression: &str,
) -> Result<Vec<CompletionItem>> {
    info!("🌐 Web completions for: {}", expression);
    let completions = inference_service.get_completions(expression);
    Ok(completions)
}

/// Phase 5: Get all types as DTO (Web API)
///
/// # Arguments
/// * `inference_service` - TypeInferenceService for type operations
/// * `metadata_lookup` - Lookup for type metadata
/// * `cache` - Analysis cache for metrics
/// * `limit` - Page size
/// * `offset` - Page offset
/// * `category_filter` - Optional category filter
/// * `certainty_filter` - Optional certainty filter (high/medium/low)
/// * `flow_sensitive_only` - Filter to flow-sensitive types only
///
/// # Returns
/// AnalysisResultDto with paginated types
#[allow(clippy::too_many_arguments)]
pub fn get_all_types_as_dto(
    inference_service: &TypeInferenceService,
    metadata_lookup: &TypeMetadataLookup,
    cache: &AnalysisCache,
    limit: usize,
    offset: usize,
    category_filter: Option<String>,
    certainty_filter: Option<String>,
    flow_sensitive_only: bool,
) -> AnalysisResultDto {
    // 1. Get all types from Domain
    let all_types = inference_service.get_all_platform_globals();

    // 2. First transform to DTO (to know category), then filter, then paginate
    let all_type_dtos: Vec<TypeDto> = all_types
        .iter()
        .map(|(name, res)| {
            type_resolution_to_dto(name, res, metadata_lookup, |r| {
                generate_type_description(r)
            })
        })
        .collect();

    // 3. Apply filters
    let filtered_types: Vec<TypeDto> = all_type_dtos
        .into_iter()
        .filter(|t| {
            // Filter by category
            if let Some(ref cat) = category_filter {
                if &t.category != cat {
                    return false;
                }
            }

            // Filter by certainty
            if let Some(ref cert) = certainty_filter {
                let passes = match cert.as_str() {
                    "high" => t.certainty >= 80,
                    "medium" => t.certainty >= 30 && t.certainty < 80,
                    "low" => t.certainty < 30,
                    _ => true,
                };
                if !passes {
                    return false;
                }
            }

            // Filter by flow-sensitive
            if flow_sensitive_only && !t.flow_sensitive {
                return false;
            }

            true
        })
        .collect();

    // 4. Apply pagination
    let type_dtos: Vec<TypeDto> = filtered_types
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    // 5. Generate metrics (using filtered data)
    let metrics = MetricsDto {
        total_types: filtered_types.len(),
        certainty_high: type_dtos.iter().filter(|t| t.certainty > 80).count(),
        certainty_medium: type_dtos
            .iter()
            .filter(|t| t.certainty > 40 && t.certainty <= 80)
            .count(),
        certainty_low: type_dtos.iter().filter(|t| t.certainty <= 40).count(),
        flow_sensitive: type_dtos.iter().filter(|t| t.flow_sensitive).count(),
        cache_hit_rate: format!("{:.1}%", cache.get_hit_rate()),
        analysis_speed: "125ms".to_string(), // TODO: real metric
    };

    // 6. Generate categories
    let mut categories = std::collections::HashMap::new();
    categories.insert(
        "Platform".to_string(),
        CategoryDto {
            color: "#3498db".to_string(),
            icon: "🔧".to_string(),
            count: type_dtos
                .iter()
                .filter(|t| t.category == "Platform")
                .count(),
        },
    );
    categories.insert(
        "Configuration".to_string(),
        CategoryDto {
            color: "#e74c3c".to_string(),
            icon: "⚙️".to_string(),
            count: type_dtos
                .iter()
                .filter(|t| t.category == "Configuration")
                .count(),
        },
    );

    // Count Union types
    let union_count = type_dtos.iter().filter(|t| t.category == "Union").count();
    categories.insert(
        "Union".to_string(),
        CategoryDto {
            color: "#9b59b6".to_string(),
            icon: "🎯".to_string(),
            count: union_count,
        },
    );

    // Count Dynamic types
    let dynamic_count = type_dtos.iter().filter(|t| t.category == "Dynamic").count();
    categories.insert(
        "Dynamic".to_string(),
        CategoryDto {
            color: "#f39c12".to_string(),
            icon: "🔄".to_string(),
            count: dynamic_count,
        },
    );

    // 7. Generate pagination info (using filtered data)
    let total_items = filtered_types.len();
    let current_page = (offset / limit) + 1;
    let total_pages = total_items.div_ceil(limit);
    let has_prev = current_page > 1;
    let has_next = current_page < total_pages;

    let pagination = Some(PaginationDto {
        current_page,
        page_size: limit,
        total_items,
        total_pages,
        has_prev,
        has_next,
    });

    // 8. Return full structure
    AnalysisResultDto {
        types: type_dtos,
        categories,
        metrics,
        connections: Vec::new(),
        pagination,
    }
}

/// Get metrics summary for Web API
///
/// # Arguments
/// * `inference_service` - TypeInferenceService for type operations
///
/// # Returns
/// JSON value with metrics summary
pub fn get_metrics_summary(inference_service: &TypeInferenceService) -> serde_json::Value {
    let all_types = inference_service.get_all_platform_globals();

    let mut known = 0;
    let mut inferred = 0;
    let mut unknown = 0;

    for res in all_types.values() {
        match res.certainty {
            Certainty::Known => known += 1,
            Certainty::Inferred | Certainty::InferredWeak => inferred += 1,
            Certainty::Unknown => unknown += 1,
        }
    }

    serde_json::json!({
        "total_types": all_types.len(),
        "known_types": known,
        "inferred_types": inferred,
        "unknown_types": unknown,
    })
}

// === Helper functions ===

/// Determines category and source for a type based on TypeResolution
fn determine_category_and_source(res: &TypeResolution) -> (String, String) {
    use bsl_shared::domain::types::ConcreteType;

    // 1. Union types (highest priority)
    if matches!(res.result, ResolutionResult::Union(_)) {
        return ("Union".to_string(), "Flow Analysis".to_string());
    }

    // 2. Dynamic types (low certainty)
    if matches!(res.certainty, Certainty::Unknown) {
        return ("Dynamic".to_string(), "Runtime".to_string());
    }

    // 3. Determine category by ConcreteType
    match &res.result {
        ResolutionResult::Concrete(ConcreteType::Platform(_)) => {
            ("Platform".to_string(), "Static Analysis".to_string())
        }
        ResolutionResult::Concrete(ConcreteType::Configuration(_)) => {
            ("Configuration".to_string(), "Configuration".to_string())
        }
        ResolutionResult::Concrete(ConcreteType::Primitive(_)) => {
            ("Platform".to_string(), "Primitive".to_string())
        }
        _ => ("Platform".to_string(), "Static Analysis".to_string()),
    }
}

/// Determines if flow-sensitive analysis is needed for a type
fn determine_flow_sensitivity(res: &TypeResolution) -> bool {
    // Flow-sensitive is needed if:

    // 1. Has union types (requires flow analysis)
    if matches!(res.result, ResolutionResult::Union(_)) {
        return true;
    }

    // 2. Certainty is inferred (not static)
    if matches!(res.certainty, Certainty::Inferred | Certainty::InferredWeak) {
        return true;
    }

    false
}

/// Generates type description (human-readable format)
fn generate_type_description(resolution: &TypeResolution) -> String {
    match &resolution.result {
        ResolutionResult::Concrete(concrete) => {
            // Use Display instead of Debug for readable format
            format!("Конкретный тип: {}", concrete)
        }
        ResolutionResult::Union(types) => {
            // Show union type variants
            let type_names: Vec<String> = types
                .iter()
                .map(|wt| format!("{}", wt.type_))
                .collect();
            format!("Union тип: {}", type_names.join(" | "))
        }
        ResolutionResult::Intersection(types) => {
            let type_names: Vec<String> = types
                .iter()
                .map(|t| format!("{}", t))
                .collect();
            format!("Intersection тип: {}", type_names.join(" & "))
        }
        ResolutionResult::Generic(gen) => {
            let params: Vec<String> = gen
                .type_params
                .iter()
                .map(|t| format!("{}", t))
                .collect();
            if params.is_empty() {
                format!("Generic тип: {}", gen.base_type)
            } else {
                format!("Generic тип: {}<{}>", gen.base_type, params.join(", "))
            }
        }
        ResolutionResult::Nullable(inner) => {
            format!("Nullable тип: {} | Неопределено", inner)
        }
        ResolutionResult::Dynamic => "Динамический тип (Произвольный)".to_string(),
    }
}

/// Converts TypeResolution to TypeDto
fn type_resolution_to_dto<F>(
    name: &str,
    res: &TypeResolution,
    metadata_lookup: &TypeMetadataLookup,
    description_generator: F,
) -> TypeDto
where
    F: Fn(&TypeResolution) -> String,
{
    // Determine category and source
    let (category, source) = determine_category_and_source(res);

    // Calculate certainty
    let certainty_val = match res.certainty {
        Certainty::Known => 100,
        Certainty::Inferred => 80,
        Certainty::InferredWeak => 50,
        Certainty::Unknown => 0,
    };

    // Extract union types
    let union_types = if let ResolutionResult::Union(types) = &res.result {
        Some(
            types
                .iter()
                .map(|wt| UnionComponentDto {
                    // Use Display instead of Debug for readable format
                    type_name: format!("{}", wt.type_),
                    probability: (wt.weight * 100.0) as u8,
                })
                .collect(),
        )
    } else {
        None
    };

    // Get methods and properties via TypeMetadataLookup
    let methods = metadata_lookup.get_methods(res);
    let properties = metadata_lookup.get_properties(res);
    let raw_type = metadata_lookup.get_raw_type(res);

    // Extract real description from RawTypeData
    let description = raw_type
        .as_ref()
        .map(|rt| rt.description.clone())
        .unwrap_or_else(|| description_generator(res));

    // Extract enum values for platform enumerations
    let enum_values = raw_type.as_ref().and_then(|rt| {
        if rt.enum_values.is_empty() {
            None
        } else {
            Some(rt.enum_values.clone())
        }
    });

    TypeDto {
        id: name.to_string(),
        name: name.to_string(),
        category,
        certainty: certainty_val,
        certainty_text: format!("{:?} {}%", res.certainty, certainty_val),
        // Use display_name() for readable facet format
        facets: res
            .available_facets
            .iter()
            .map(|f| f.display_name().to_string())
            .collect(),
        methods_count: Some(methods.len()),
        methods: methods
            .iter()
            .map(|m| MethodDto {
                is_deprecated: false,
                is_constructor: false,
                name: m.name.clone(),
                english_name: Some(m.english_name.clone()),
                return_type: Some(m.return_type.clone()),
                params: m
                    .params
                    .iter()
                    .map(|p| ParamDto {
                        name: p.name.clone(),
                        param_type: p.param_type.clone(),
                        is_optional: p.is_optional,
                        default_value: None,
                    })
                    .collect(),
                description: None,
            })
            .collect(),
        attributes_count: raw_type.as_ref().map(|rt| rt.attributes.len()),
        properties: properties.iter().map(|p| p.name.clone()).collect(),
        enum_values,
        // Convert tabular sections from RawTypeData to DTO
        tabular_sections: raw_type
            .as_ref()
            .map(|rt| {
                rt.tabular_sections
                    .iter()
                    .map(|ts| TabularSectionDto {
                        name: ts.name.clone(),
                        attributes: ts
                            .attributes
                            .iter()
                            .map(|attr| TabularSectionAttributeDto {
                                name: attr.name.clone(),
                                attr_type: Some(attr.attr_type.clone()),
                            })
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        source,
        flow_sensitive: determine_flow_sensitivity(res),
        description,
        union_types,
        flow_analysis: None,
        connections: None,
        warning: None,
        recommendation: None,
    }
}
