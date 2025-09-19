//! API client functions

use crate::api::types::*;
use crate::config::get_config;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};


/// Получить метрики системы типизации
pub async fn fetch_metrics() -> Result<TypeMetrics, String> {
    let config = get_config();
    let url = config.api_url("metrics");
    
    match fetch_json::<TypeMetrics>(&url).await {
        Ok(metrics) => Ok(metrics),
        Err(_) => {
            // Fallback to test data if API is not available
            Ok(TypeMetrics {
                total_types: 87,
                known_types: 76,
                inferred_types: 8,
                unknown_types: 3,
                flow_sensitive_types: 23,
                cache_hit_rate: 0.94,
                analysis_speed_ms: 125.0,
            })
        }
    }
}

/// Получить список типов с фильтрацией
pub async fn fetch_types(filters: TypeFilters) -> Result<TypeSearchResult, String> {
    let config = get_config();
    let url = config.api_url("types");
    
    match fetch_json::<TypeSearchResult>(&url).await {
        Ok(result) => Ok(result),
        Err(_) => {
            // Fallback to test data if API is not available
            get_test_types(filters)
        }
    }
}

/// Get test data for types
fn get_test_types(filters: TypeFilters) -> Result<TypeSearchResult, String> {
    // Временные тестовые данные
    let test_types = vec![
        TypeInfo {
            name: "Array".to_string(),
            display_name: "Массив".to_string(),
            category: TypeCategory::Platform,
            certainty: Certainty::Known,
            facets: vec![FacetKind::Object, FacetKind::Collection],
            active_facet: Some(FacetKind::Object),
            union_types: None,
            is_flow_sensitive: false,
            source: "Static Analysis".to_string(),
            methods_count: Some(15),
            properties_count: None,
            description: Some("Коллекция элементов с индексным доступом".to_string()),
        },
        TypeInfo {
            name: "Catalogs.Items".to_string(),
            display_name: "Справочники.Номенклатура".to_string(),
            category: TypeCategory::Configuration,
            certainty: Certainty::Known,
            facets: vec![FacetKind::Manager, FacetKind::Reference, FacetKind::Object, FacetKind::Metadata],
            active_facet: Some(FacetKind::Reference),
            union_types: None,
            is_flow_sensitive: false,
            source: "Configuration".to_string(),
            methods_count: Some(12),
            properties_count: Some(8),
            description: Some("Иерархический справочник с поддержкой групп".to_string()),
        },
        TypeInfo {
            name: "OperationResult".to_string(),
            display_name: "РезультатОперации".to_string(),
            category: TypeCategory::Union,
            certainty: Certainty::Inferred(0.85),
            facets: vec![FacetKind::Object],
            active_facet: Some(FacetKind::Object),
            union_types: Some(vec![
                WeightedType { type_name: "Булево".to_string(), weight: 0.7 },
                WeightedType { type_name: "Неопределено".to_string(), weight: 0.3 },
            ]),
            is_flow_sensitive: true,
            source: "Flow Analysis".to_string(),
            methods_count: None,
            properties_count: None,
            description: Some("Результат операции с возможными значениями".to_string()),
        },
        TypeInfo {
            name: "DynamicObject".to_string(),
            display_name: "ДинамическийОбъект".to_string(),
            category: TypeCategory::Dynamic,
            certainty: Certainty::Inferred(0.3),
            facets: vec![FacetKind::Object],
            active_facet: Some(FacetKind::Object),
            union_types: Some(vec![
                WeightedType { type_name: "Структура".to_string(), weight: 0.6 },
                WeightedType { type_name: "ДанныеФормы".to_string(), weight: 0.4 },
            ]),
            is_flow_sensitive: true,
            source: "Runtime".to_string(),
            methods_count: None,
            properties_count: None,
            description: Some("Динамический объект с runtime определением типа".to_string()),
        },
        TypeInfo {
            name: "String".to_string(),
            display_name: "Строка".to_string(),
            category: TypeCategory::Platform,
            certainty: Certainty::Known,
            facets: vec![FacetKind::Object],
            active_facet: Some(FacetKind::Object),
            union_types: None,
            is_flow_sensitive: false,
            source: "Static Analysis".to_string(),
            methods_count: Some(20),
            properties_count: None,
            description: Some("Строковый тип данных".to_string()),
        },
    ];

    // Применяем фильтры
    let filtered_types: Vec<TypeInfo> = test_types
        .into_iter()
        .filter(|t| {
            if let Some(ref query) = filters.search_query {
                if !query.is_empty() {
                    return t.name.to_lowercase().contains(&query.to_lowercase()) ||
                           t.display_name.to_lowercase().contains(&query.to_lowercase());
                }
            }
            if let Some(ref category) = filters.category {
                if t.category != *category {
                    return false;
                }
            }
            if filters.flow_sensitive_only && !t.is_flow_sensitive {
                return false;
            }
            true
        })
        .collect();

    Ok(TypeSearchResult {
        filtered_count: filtered_types.len() as u32,
        total_count: 5,
        types: filtered_types,
    })
}

/// Получить граф типов
pub async fn fetch_type_graph() -> Result<TypeGraph, String> {
    let config = get_config();
    let url = config.api_url("type-graph");
    
    match fetch_json::<TypeGraph>(&url).await {
        Ok(graph) => Ok(graph),
        Err(_) => {
            // Fallback to test data if API is not available
            get_test_type_graph()
        }
    }
}

/// Get test data for type graph
fn get_test_type_graph() -> Result<TypeGraph, String> {
    // Временные тестовые данные для графа
    let nodes = vec![
        TypeGraphNode {
            id: "array".to_string(),
            type_info: TypeInfo {
                name: "Array".to_string(),
                display_name: "Массив".to_string(),
                category: TypeCategory::Platform,
                certainty: Certainty::Known,
                facets: vec![FacetKind::Object, FacetKind::Collection],
                active_facet: Some(FacetKind::Object),
                union_types: None,
                is_flow_sensitive: false,
                source: "Static Analysis".to_string(),
                methods_count: Some(15),
                properties_count: None,
                description: None,
            },
            x: 200.0,
            y: 150.0,
            connections: vec!["catalogs".to_string()],
        },
        TypeGraphNode {
            id: "catalogs".to_string(),
            type_info: TypeInfo {
                name: "Catalogs.Items".to_string(),
                display_name: "Справочники.Номенклатура".to_string(),
                category: TypeCategory::Configuration,
                certainty: Certainty::Known,
                facets: vec![FacetKind::Manager, FacetKind::Reference, FacetKind::Object],
                active_facet: Some(FacetKind::Reference),
                union_types: None,
                is_flow_sensitive: false,
                source: "Configuration".to_string(),
                methods_count: Some(12),
                properties_count: Some(8),
                description: None,
            },
            x: 500.0,
            y: 180.0,
            connections: vec!["operation_result".to_string()],
        },
    ];

    let connections = vec![
        TypeConnection {
            from: "array".to_string(),
            to: "catalogs".to_string(),
            connection_type: ConnectionType::Dependency,
            label: Some("uses".to_string()),
        },
    ];

    Ok(TypeGraph { nodes, connections })
}

/// Generic function to fetch JSON from API
async fn fetch_json<T>(url: &str) -> Result<T, JsValue>
where
    T: serde::de::DeserializeOwned,
{
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url, &opts)?;
    request.headers().set("Accept", "application/json")?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into().unwrap();

    if !resp.ok() {
        return Err(JsValue::from_str(&format!("HTTP error: {}", resp.status())));
    }

    let json = JsFuture::from(resp.json()?).await?;
    let data: T = serde_wasm_bindgen::from_value(json)?;
    
    Ok(data)
}
