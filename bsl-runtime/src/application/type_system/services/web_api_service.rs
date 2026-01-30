//! Web API Service - search and type details operations for Web API
//!
//! Functions for searching types and retrieving type details,
//! used by the Web API endpoints.

use anyhow::Result;
use tracing::info;

use bsl_analysis_v2::SemanticDeps;
use bsl_shared::api::dtos::{
    AnalysisResultDto, CategoryDto, MetricsDto, PaginationDto, TabularSectionAttributeDto,
    TabularSectionDto, TypeDto, UnionComponentDto,
};
use bsl_shared::api::{MethodDto, ParamDto};
use bsl_shared::domain::signature_index::{MethodSignature, SignatureSource};
use bsl_shared::domain::types::{
    Attribute, Certainty, ConcreteType, ConfigurationType, MetadataKind, RawAttributeData,
    RawDataSource, RawTabularSectionData, ResolutionResult, TabularSection, TypeResolution,
};
use bsl_shared::domain::{CompletionItem, CompletionKind, TypeMetadataLookup};

/// Searches types by query string
///
/// # Arguments
/// * `deps` - v2 deps snapshot (`SemanticDeps`)
/// * `query` - Search query string
///
/// # Returns
/// List of matching type names
pub async fn search_types(deps: &SemanticDeps, query: &str) -> Result<Vec<String>> {
    info!("🌐 Web search types: {}", query);
    let results = search_types_from_deps(deps, query);
    Ok(results)
}

/// Phase 5: Search types with DTO transformation (Web API)
///
/// # Arguments
/// * `deps` - v2 deps snapshot (`SemanticDeps`)
/// * `metadata_lookup` - Lookup for type metadata
/// * `cache` - Analysis cache for metrics
/// * `query` - Search query string
///
/// # Returns
/// AnalysisResultDto with filtered types
pub async fn search_types_as_dto(
    deps: &SemanticDeps,
    metadata_lookup: &TypeMetadataLookup,
    query: &str,
) -> Result<AnalysisResultDto> {
    info!("🌐 Web search types with DTO: {}", query);

    // 1. Get all types and filter by query
    let all_types = get_all_platform_globals(deps);
    let query_lower = query.to_lowercase();

    let filtered_types: Vec<(&String, &TypeResolution)> = all_types
        .iter()
        .filter(|(name, _)| name.to_lowercase().contains(&query_lower))
        .collect();

    // 2. Transform to DTO
    let mut type_dtos: Vec<TypeDto> = filtered_types
        .iter()
        .map(|(name, res)| {
            type_resolution_to_dto(name, res, metadata_lookup, |_| {
                // Generate type description for search results
                generate_type_description(res)
            })
        })
        .collect();

    let mut global_function_dtos = search_global_functions_as_dto(deps, query);

    let mut all_dtos = Vec::with_capacity(type_dtos.len() + global_function_dtos.len());
    all_dtos.append(&mut type_dtos);
    all_dtos.append(&mut global_function_dtos);

    // Детерминированная сортировка + приоритет точного совпадения.
    // Это важно, когда query — полное имя типа (например, "Документы.ЗаказНаряды"),
    // но в репозитории присутствуют дополнительные синтетические типы, содержащие query как подстроку.
    let rank = |name: &str, name_lower: &str| -> u8 {
        if name == query {
            0
        } else if name_lower == query_lower {
            1
        } else if name.starts_with(query) {
            2
        } else if name_lower.starts_with(&query_lower) {
            3
        } else if name_lower.ends_with(&format!(".{}", query_lower))
            || name_lower.ends_with(&query_lower)
        {
            4
        } else {
            5
        }
    };

    all_dtos.sort_by(|a, b| {
        let a_lower = a.name.to_lowercase();
        let b_lower = b.name.to_lowercase();

        rank(a.name.as_str(), &a_lower)
            .cmp(&rank(b.name.as_str(), &b_lower))
            .then_with(|| a_lower.len().cmp(&b_lower.len()))
            .then_with(|| a_lower.cmp(&b_lower))
    });

    // 3. Generate metrics
    let metrics = MetricsDto {
        total_types: all_dtos.len(),
        certainty_high: all_dtos.iter().filter(|t| t.certainty > 80).count(),
        certainty_medium: all_dtos
            .iter()
            .filter(|t| t.certainty > 40 && t.certainty <= 80)
            .count(),
        certainty_low: all_dtos.iter().filter(|t| t.certainty <= 40).count(),
        flow_sensitive: all_dtos.iter().filter(|t| t.flow_sensitive).count(),
        cache_hit_rate: "n/a".to_string(),
        analysis_speed: "125ms".to_string(),
    };

    // 4. Generate categories
    let mut categories = std::collections::HashMap::new();
    categories.insert(
        "Platform".to_string(),
        CategoryDto {
            color: "#3498db".to_string(),
            icon: "🔧".to_string(),
            count: all_dtos.iter().filter(|t| t.category == "Platform").count(),
        },
    );
    categories.insert(
        "Configuration".to_string(),
        CategoryDto {
            color: "#e74c3c".to_string(),
            icon: "⚙️".to_string(),
            count: all_dtos
                .iter()
                .filter(|t| t.category == "Configuration")
                .count(),
        },
    );
    Ok(AnalysisResultDto {
        types: all_dtos,
        categories,
        metrics,
        connections: vec![],
        pagination: None, // Search without pagination
    })
}

/// Get type details by name
///
/// # Arguments
/// * `deps` - v2 deps snapshot (`SemanticDeps`)
/// * `type_name` - Name of the type to retrieve
///
/// # Returns
/// TypeResolution if found
pub async fn get_type_details(
    deps: &SemanticDeps,
    type_name: &str,
) -> Result<Option<TypeResolution>> {
    info!("🌐 Web type details: {}", type_name);
    let platform_globals = get_all_platform_globals(deps);
    Ok(platform_globals.get(type_name).cloned())
}

/// Get type details as TypeDto by exact name.
///
/// Intended for MCP/Web parity use-cases where a caller needs a single type payload
/// without enumerating full lists.
pub fn get_type_details_as_dto(
    deps: &SemanticDeps,
    metadata_lookup: &TypeMetadataLookup,
    type_name: &str,
    include_methods: bool,
) -> Option<TypeDto> {
    let platform_globals = get_all_platform_globals(deps);
    let res = platform_globals.get(type_name)?;
    let mut dto =
        type_resolution_to_dto(type_name, res, metadata_lookup, generate_type_description);
    if !include_methods {
        dto.methods.clear();
    }
    Some(dto)
}

/// Get type completions for an expression
///
/// # Arguments
/// * `deps` - v2 deps snapshot (`SemanticDeps`)
/// * `expression` - Expression to get completions for
///
/// # Returns
/// List of completion items
pub async fn get_type_completions(
    deps: &SemanticDeps,
    expression: &str,
) -> Result<Vec<CompletionItem>> {
    info!("🌐 Web completions for: {}", expression);
    let completions = completions_from_deps(deps, expression);
    Ok(completions)
}

/// Phase 5: Get all types as DTO (Web API)
///
/// # Arguments
/// * `deps` - v2 deps snapshot (`SemanticDeps`)
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
    deps: &SemanticDeps,
    metadata_lookup: &TypeMetadataLookup,
    limit: usize,
    offset: usize,
    category_filter: Vec<String>,
    certainty_filter: Vec<String>,
    flow_sensitive_only: bool,
) -> AnalysisResultDto {
    // 1. Get all types from Domain
    let all_types = get_all_platform_globals(deps);

    // 2. First transform to DTO (to know category), then filter, then paginate
    let all_type_dtos: Vec<TypeDto> = all_types
        .iter()
        .map(|(name, res)| {
            type_resolution_to_dto(name, res, metadata_lookup, generate_type_description)
        })
        .collect();

    // 3. Apply filters
    let mut filtered_types: Vec<TypeDto> = all_type_dtos
        .into_iter()
        .filter(|t| {
            // Filter by category
            if !category_filter.is_empty() && !category_filter.iter().any(|cat| cat == &t.category)
            {
                return false;
            }

            // Filter by certainty
            if !certainty_filter.is_empty() {
                let passes = certainty_filter.iter().any(|cert| match cert.as_str() {
                    "high" => t.certainty >= 80,
                    "medium" => t.certainty >= 30 && t.certainty < 80,
                    "low" => t.certainty < 30,
                    _ => true,
                });
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

    // Deterministic ordering before pagination.
    // Without this, HashMap iteration order can affect page boundaries.
    filtered_types.sort_by(|a, b| {
        let a_name = a.name.to_lowercase();
        let b_name = b.name.to_lowercase();
        a.category
            .cmp(&b.category)
            .then_with(|| a_name.cmp(&b_name))
            .then_with(|| a.name.cmp(&b.name))
    });

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
        certainty_high: filtered_types.iter().filter(|t| t.certainty >= 80).count(),
        certainty_medium: filtered_types
            .iter()
            .filter(|t| t.certainty >= 30 && t.certainty < 80)
            .count(),
        certainty_low: filtered_types.iter().filter(|t| t.certainty < 30).count(),
        flow_sensitive: filtered_types.iter().filter(|t| t.flow_sensitive).count(),
        cache_hit_rate: "n/a".to_string(),
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
/// * `deps` - v2 deps snapshot (`SemanticDeps`)
///
/// # Returns
/// JSON value with metrics summary
pub fn get_metrics_summary(deps: &SemanticDeps) -> serde_json::Value {
    let all_types = get_all_platform_globals(deps);

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

pub fn get_all_platform_globals(
    deps: &SemanticDeps,
) -> std::collections::HashMap<String, TypeResolution> {
    let raw_types = deps.repository.get_all_types();
    let mut result = std::collections::HashMap::new();

    for raw_type in raw_types {
        let concrete_type = match raw_type.source {
            RawDataSource::Platform => {
                ConcreteType::Platform(bsl_shared::domain::types::PlatformType {
                    name: raw_type.name.clone(),
                })
            }
            RawDataSource::Configuration => {
                // Конвертируем атрибуты и табличные части
                let attributes = convert_raw_attributes(&raw_type.attributes);
                let tabular_sections = convert_raw_tabular_sections(&raw_type.tabular_sections);

                ConcreteType::Configuration(ConfigurationType {
                    kind: raw_type.kind.unwrap_or(MetadataKind::Unknown),
                    name: raw_type.name.clone(),
                    facet: raw_type.facets.first().copied(),
                    attributes,
                    tabular_sections,
                })
            }
            RawDataSource::UserDefined => {
                // Пользовательские типы пока как Platform
                ConcreteType::Platform(bsl_shared::domain::types::PlatformType {
                    name: raw_type.name.clone(),
                })
            }
        };

        let mut resolution = TypeResolution::known(concrete_type);
        // Копируем фасеты из RawTypeData
        resolution.available_facets = raw_type.facets.clone();

        result.insert(raw_type.name, resolution);
    }

    result
}

fn completions_from_deps(deps: &SemanticDeps, query: &str) -> Vec<CompletionItem> {
    let all_types = deps.repository.get_all_types();
    let mut completions = Vec::new();
    let query_lower = query.to_lowercase();

    for raw_type in all_types {
        if !raw_type.name.to_lowercase().contains(&query_lower) {
            continue;
        }

        let concrete_type = match raw_type.source {
            RawDataSource::Platform => {
                ConcreteType::Platform(bsl_shared::domain::types::PlatformType {
                    name: raw_type.name.clone(),
                })
            }
            RawDataSource::Configuration => {
                let attributes = convert_raw_attributes(&raw_type.attributes);
                let tabular_sections = convert_raw_tabular_sections(&raw_type.tabular_sections);

                ConcreteType::Configuration(ConfigurationType {
                    kind: raw_type.kind.unwrap_or(MetadataKind::Unknown),
                    name: raw_type.name.clone(),
                    facet: raw_type.facets.first().copied(),
                    attributes,
                    tabular_sections,
                })
            }
            RawDataSource::UserDefined => {
                ConcreteType::Platform(bsl_shared::domain::types::PlatformType {
                    name: raw_type.name.clone(),
                })
            }
        };

        let resolution = TypeResolution::known(concrete_type);
        let item = CompletionItem::with_details(
            raw_type.name.clone(),
            determine_completion_kind(&resolution),
            Some(format!("{:?}", resolution.result)),
            resolution.metadata.notes.first().cloned(),
        );
        completions.push(item);
    }

    completions
}

fn search_types_from_deps(deps: &SemanticDeps, query: &str) -> Vec<String> {
    completions_from_deps(deps, query)
        .into_iter()
        .map(|c| c.label)
        .collect()
}

fn search_global_functions_from_deps(
    deps: &SemanticDeps,
    query: &str,
) -> Vec<(String, MethodSignature)> {
    let index = &deps.signature_index;
    let query_lower = query.to_lowercase();

    index
        .get_global_functions()
        .iter()
        .filter(|(name, _)| name.display().to_lowercase().contains(&query_lower))
        .map(|(name, sig)| (name.display().to_string(), sig.clone()))
        .collect()
}

fn convert_raw_attributes(raw_attrs: &[RawAttributeData]) -> Vec<Attribute> {
    raw_attrs
        .iter()
        .map(|ra| Attribute {
            name: ra.name.clone(),
            type_: ra.attr_type.clone(),
            is_composite: false,
            types: vec![ra.attr_type.clone()],
        })
        .collect()
}

fn convert_raw_tabular_sections(raw_ts: &[RawTabularSectionData]) -> Vec<TabularSection> {
    raw_ts
        .iter()
        .map(|rts| TabularSection {
            name: rts.name.clone(),
            synonym: None,
            attributes: convert_raw_attributes(&rts.attributes),
        })
        .collect()
}

fn determine_completion_kind(resolution: &TypeResolution) -> CompletionKind {
    match &resolution.result {
        ResolutionResult::Concrete(ConcreteType::Platform(_)) => CompletionKind::Global,
        ResolutionResult::Concrete(ConcreteType::Configuration(config)) => {
            CompletionKind::from_metadata_kind(config.kind)
        }
        _ => CompletionKind::Global,
    }
}

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
            ("Platform".to_string(), "Platform".to_string())
        }
        ResolutionResult::Concrete(ConcreteType::Configuration(_)) => {
            ("Configuration".to_string(), "Configuration".to_string())
        }
        ResolutionResult::Concrete(ConcreteType::Primitive(_)) => {
            ("Platform".to_string(), "Platform".to_string())
        }
        ResolutionResult::Concrete(ConcreteType::GlobalFunction(_)) => {
            ("Platform".to_string(), "Platform".to_string())
        }
        _ => ("Platform".to_string(), "Platform".to_string()),
    }
}

fn search_global_functions_as_dto(deps: &SemanticDeps, query: &str) -> Vec<TypeDto> {
    search_global_functions_from_deps(deps, query)
        .into_iter()
        .map(|(name, sig)| global_function_signature_to_dto(&name, &sig))
        .collect()
}

fn normalize_signature_type(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

fn global_function_signature_to_dto(name: &str, signature: &MethodSignature) -> TypeDto {
    let (category, source) = match signature.source {
        SignatureSource::Platform => ("Platform".to_string(), "Platform".to_string()),
        SignatureSource::Configuration | SignatureSource::UserCode => {
            ("Configuration".to_string(), "Configuration".to_string())
        }
    };
    let return_type = normalize_signature_type(&signature.return_type);
    let return_hint = return_type
        .clone()
        .unwrap_or_else(|| "Неопределено".to_string());
    let description = format!("Глобальная функция. Возвращает: {}", return_hint);

    let params = signature
        .params
        .iter()
        .map(|p| ParamDto {
            name: p.name.clone(),
            param_type: p
                .type_name
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .unwrap_or("Произвольный")
                .to_string(),
            is_optional: p.is_optional,
            default_value: p.default_value.clone(),
        })
        .collect();

    let method = MethodDto {
        name: name.to_string(),
        english_name: None,
        return_type,
        params,
        description: None,
        is_deprecated: false,
        is_constructor: false,
    };

    TypeDto {
        id: name.to_string(),
        name: name.to_string(),
        category,
        certainty: 100,
        certainty_text: "Known 100%".to_string(),
        facets: vec![],
        methods_count: Some(1),
        methods: vec![method],
        attributes_count: None,
        properties: vec![],
        enum_values: None,
        tabular_sections: vec![],
        source,
        flow_sensitive: false,
        description,
        union_types: None,
        flow_analysis: None,
        connections: None,
        warning: None,
        recommendation: None,
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
            let type_names: Vec<String> = types.iter().map(|wt| format!("{}", wt.type_)).collect();
            format!("Union тип: {}", type_names.join(" | "))
        }
        ResolutionResult::Intersection(types) => {
            let type_names: Vec<String> = types.iter().map(|t| format!("{}", t)).collect();
            format!("Intersection тип: {}", type_names.join(" & "))
        }
        ResolutionResult::Generic(gen) => {
            let params: Vec<String> = gen.type_params.iter().map(|t| format!("{}", t)).collect();
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
