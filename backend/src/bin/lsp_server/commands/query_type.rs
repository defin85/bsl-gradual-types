//! Query Type command handler
//!
//! Handles bsl/queryType custom request.

use std::sync::Arc;
use tracing::{info, warn};

use bsl_shared::api::dtos::{MethodDto, ParamDto, PropertyDto};
use bsl_shared::engine::AnalysisEngine;

/// Request parameters for bsl/queryType
#[derive(Debug, serde::Deserialize)]
pub struct QueryTypeParams {
    pub type_name: String,
}

/// Response for bsl/queryType
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryTypeResponse {
    pub type_name: String,
    pub found: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub certainty: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub facet: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<MethodDto>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<PropertyDto>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facets: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl QueryTypeResponse {
    pub fn not_found(type_name: String, reason: &str) -> Self {
        Self {
            type_name: type_name.clone(),
            found: false,
            certainty: Some("Unknown".to_string()),
            facet: None,
            description: Some(reason.to_string()),
            methods: vec![],
            properties: vec![],
            facets: vec![],
            details: Some(format!("Type '{}' not found", type_name)),
        }
    }
}

/// Handle bsl/queryType command
pub fn handle_query_type(
    params: QueryTypeParams,
    analysis_engine: Option<Arc<AnalysisEngine>>,
) -> QueryTypeResponse {
    info!("Custom request: bsl/queryType - {}", params.type_name);

    let engine = match analysis_engine {
        Some(e) => e,
        None => {
            warn!("AnalysisEngine not available");
            return QueryTypeResponse::not_found(
                params.type_name,
                "AnalysisEngine not available",
            );
        }
    };

    let repo = engine.get_repository();

    match repo.find_type(&params.type_name) {
        Some(raw_type) => {
            info!(
                "Type '{}' found with {} methods, {} properties",
                params.type_name,
                raw_type.methods.len(),
                raw_type.properties.len()
            );

            // Convert methods from RawMethodData -> MethodDto
            let mut methods: Vec<MethodDto> = raw_type
                .methods
                .iter()
                .map(|m| MethodDto {
                    name: m.name.clone(),
                    english_name: if m.english_name.is_empty() {
                        None
                    } else {
                        Some(m.english_name.clone())
                    },
                    return_type: if m.return_type.is_empty() {
                        None
                    } else {
                        Some(m.return_type.clone())
                    },
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
                    is_deprecated: false,
                    is_constructor: false,
                })
                .collect();

            // Get constructors from SignatureIndex
            if let Some(constructor) = engine.find_constructor(&params.type_name) {
                let constructor_dto = MethodDto {
                    name: format!("New {}", constructor.type_name),
                    english_name: Some(format!("New {}", constructor.type_name)),
                    return_type: Some(constructor.type_name.clone()),
                    params: constructor
                        .params
                        .iter()
                        .map(|p| ParamDto {
                            name: p.name.clone(),
                            param_type: p
                                .type_name
                                .clone()
                                .unwrap_or_else(|| "Any".to_string()),
                            is_optional: p.is_optional,
                            default_value: p.default_value.clone(),
                        })
                        .collect(),
                    description: Some(format!(
                        "Constructor for type {}{}{}",
                        constructor.type_name,
                        if constructor.is_collection {
                            format!(
                                " (collection, {} generic params)",
                                constructor.generic_params_count
                            )
                        } else {
                            String::new()
                        },
                        constructor
                            .facet
                            .as_ref()
                            .map(|f| format!(", facet: {}", f))
                            .unwrap_or_default()
                    )),
                    is_deprecated: false,
                    is_constructor: true,
                };
                methods.push(constructor_dto);
            }

            // Convert properties
            let properties: Vec<PropertyDto> = raw_type
                .properties
                .iter()
                .map(|p| PropertyDto {
                    name: p.name.clone(),
                    prop_type: p.prop_type.clone(),
                    is_readonly: p.is_readonly,
                    description: None,
                })
                .collect();

            // Format facets as strings
            let facets: Vec<String> = raw_type
                .facets
                .iter()
                .map(|f| format!("{:?}", f))
                .collect();

            let main_facet = facets.first().cloned().or_else(|| Some("Object".to_string()));

            QueryTypeResponse {
                type_name: params.type_name.clone(),
                found: true,
                certainty: Some("Known (100%)".to_string()),
                facet: main_facet,
                description: if raw_type.description.is_empty() {
                    None
                } else {
                    Some(raw_type.description.clone())
                },
                methods,
                properties,
                facets,
                details: Some(format!(
                    "Type '{}' found with {} methods, {} properties",
                    params.type_name,
                    raw_type.methods.len(),
                    raw_type.properties.len()
                )),
            }
        }
        None => {
            warn!("Type '{}' not found in TypeRepository", params.type_name);
            QueryTypeResponse::not_found(params.type_name, "Type not found in TypeRepository")
        }
    }
}
