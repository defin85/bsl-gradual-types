//! API client functions

use crate::api::types::*;
use crate::config::get_config;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use serde::{Deserialize, Serialize};

/// Backend DTO structures to match the shared API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResultDto {
    pub types: Vec<TypeInfo>,
    pub categories: std::collections::HashMap<String, CategoryInfo>,
    pub metrics: BackendMetrics,
    pub connections: Vec<TypeConnection>,
    pub pagination: Option<BackendPagination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendMetrics {
    pub total_types: usize,
    pub certainty_high: usize,
    pub certainty_medium: usize,
    pub certainty_low: usize,
    pub flow_sensitive: usize,
    pub cache_hit_rate: String,
    pub analysis_speed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendPagination {
    pub current_page: usize,
    pub page_size: usize,
    pub total_items: usize,
    pub total_pages: usize,
    pub has_prev: bool,
    pub has_next: bool,
}


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

/// Получить список типов с фильтрацией и пагинацией
pub async fn fetch_types(filters: TypeFilters) -> Result<TypeSearchResult, String> {
    let config = get_config();

    // Построение URL с новыми булевыми флагами
    let base_url = if let Some(ref query) = filters.search_query {
        if !query.is_empty() {
            // Используем search API для конкретных запросов
            format!("{}?q={}&limit={}&offset={}",
                config.api_url("search"),
                query,
                filters.page_size,
                filters.offset())
        } else {
            // Для пустого поиска используем обычный API с пагинацией
            format!("{}?limit={}&offset={}",
                config.api_url("types"),
                filters.page_size,
                filters.offset())
        }
    } else {
        // Загружаем типы с пагинацией
        format!("{}?limit={}&offset={}",
            config.api_url("types"),
            filters.page_size,
            filters.offset())
    };

    // Добавляем параметры фильтрации на основе булевых флагов
    let mut url = base_url;

    // Категории - добавляем параметры для включённых категорий
    // ТОЛЬКО если хотя бы один флаг включён (иначе показываем всё)
    let any_category_checked = filters.show_platform || filters.show_configuration ||
                               filters.show_union || filters.show_dynamic;

    if any_category_checked {
        if filters.show_platform {
            url.push_str("&category=Platform");
        }
        if filters.show_configuration {
            url.push_str("&category=Configuration");
        }
        if filters.show_union {
            url.push_str("&category=Union");
        }
        if filters.show_dynamic {
            url.push_str("&category=Dynamic");
        }
    }

    // Уровни определённости - аналогично
    let any_certainty_checked = filters.show_high_certainty || filters.show_medium_certainty ||
                               filters.show_low_certainty;

    if any_certainty_checked {
        if filters.show_high_certainty {
            url.push_str("&certainty_level=high");
        }
        if filters.show_medium_certainty {
            url.push_str("&certainty_level=medium");
        }
        if filters.show_low_certainty {
            url.push_str("&certainty_level=low");
        }
    }

    // Flow-sensitive фильтр
    if filters.flow_sensitive_only {
        url.push_str("&flow_sensitive_only=true");
    }

    // Backend возвращает AnalysisResultDto, нужно преобразовать в TypeSearchResult
    match fetch_json::<AnalysisResultDto>(&url).await {
        Ok(analysis_result) => {
            // Преобразуем AnalysisResultDto в TypeSearchResult
            let result = TypeSearchResult {
                types: analysis_result.types,
                categories: analysis_result.categories,
                metrics: convert_metrics(analysis_result.metrics),
                connections: analysis_result.connections,
                pagination: analysis_result.pagination.map(|p| PaginationInfo {
                    current_page: p.current_page,
                    page_size: p.page_size,
                    total_items: p.total_items,
                    total_pages: p.total_pages,
                    has_next: p.has_next,
                    has_prev: p.has_prev,
                }),
            };

            // Логируем успешную загрузку для отладки
            web_sys::console::log_1(&format!("✅ Загружено {} типов из API", result.types.len()).into());
            Ok(result)
        },
        Err(e) => {
            // Логируем ошибку для отладки
            web_sys::console::error_1(&format!("❌ Ошибка API: {:?}", e).into());
            // Возвращаем тестовые данные как fallback
            get_test_types(filters)
        }
    }
}

/// Get test data for types
fn get_test_types(filters: TypeFilters) -> Result<TypeSearchResult, String> {
    // Временные тестовые данные в новом формате
    let test_types = vec![
        TypeInfo {
            id: "array".to_string(),
            name: "Массив".to_string(),
            category: "Platform".to_string(),
            certainty: 100,
            certainty_text: "Known 100%".to_string(),
            facets: vec!["Object".to_string(), "Collection".to_string()],
            source: "Static Analysis".to_string(),
            flow_sensitive: false,
            description: "Коллекция элементов с индексным доступом".to_string(),
            methods: vec!["Добавить".to_string(), "Удалить".to_string()],
            methods_count: Some(2),
            attributes_count: None,
            properties: vec![],
            enum_values: vec![],
        },
        TypeInfo {
            id: "catalogs_items".to_string(),
            name: "Справочники.Номенклатура".to_string(),
            category: "Configuration".to_string(),
            certainty: 100,
            certainty_text: "Known 100%".to_string(),
            facets: vec!["Manager".to_string(), "Reference".to_string(), "Object".to_string()],
            source: "Configuration".to_string(),
            flow_sensitive: false,
            description: "Иерархический справочник с поддержкой групп".to_string(),
            methods: vec![],
            methods_count: Some(0),
            attributes_count: Some(5),
            properties: vec![],
            enum_values: vec![],
        },
        TypeInfo {
            id: "string".to_string(),
            name: "Строка".to_string(),
            category: "Platform".to_string(),
            certainty: 100,
            certainty_text: "Known 100%".to_string(),
            facets: vec!["Object".to_string()],
            source: "Static Analysis".to_string(),
            flow_sensitive: false,
            description: "Строковый тип данных".to_string(),
            methods: vec![],
            methods_count: Some(0),
            attributes_count: None,
            properties: vec![],
            enum_values: vec![],
        },
    ];

    // Применяем фильтры с новыми булевыми флагами
    let filtered_types: Vec<TypeInfo> = test_types
        .into_iter()
        .filter(|t| {
            // Поисковый запрос
            if let Some(ref query) = filters.search_query {
                if !query.is_empty() && !t.name.to_lowercase().contains(&query.to_lowercase()) {
                    return false;
                }
            }

            // Фильтр по категориям
            // Если все категории unchecked, показываем всё
            let any_category_checked = filters.show_platform || filters.show_configuration ||
                                       filters.show_union || filters.show_dynamic;
            if any_category_checked {
                let category = t.get_category();
                let category_match = match category {
                    TypeCategory::Platform => filters.show_platform,
                    TypeCategory::Configuration => filters.show_configuration,
                    TypeCategory::Union => filters.show_union,
                    TypeCategory::Dynamic => filters.show_dynamic,
                };
                if !category_match {
                    return false;
                }
            }

            // Фильтр по уровню определённости
            // Если все уровни unchecked, показываем всё
            let any_certainty_checked = filters.show_high_certainty || filters.show_medium_certainty ||
                                       filters.show_low_certainty;
            if any_certainty_checked {
                let certainty_percent = t.certainty;
                let certainty_match =
                    (certainty_percent >= 80 && filters.show_high_certainty) ||
                    (certainty_percent >= 30 && certainty_percent < 80 && filters.show_medium_certainty) ||
                    (certainty_percent < 30 && filters.show_low_certainty);
                if !certainty_match {
                    return false;
                }
            }

            // Flow-sensitive фильтр
            if filters.flow_sensitive_only && !t.is_flow_sensitive() {
                return false;
            }

            true
        })
        .collect();

    // Создаем мок-данные в формате backend API
    let mut categories = std::collections::HashMap::new();
    categories.insert("Platform".to_string(), CategoryInfo {
        color: "#3498db".to_string(),
        icon: "🔧".to_string(),
        count: 2,
    });
    categories.insert("Configuration".to_string(), CategoryInfo {
        color: "#e74c3c".to_string(),
        icon: "⚙️".to_string(),
        count: 1,
    });

    let pagination = PaginationInfo::new(filters.page, filters.page_size, filtered_types.len());

    Ok(TypeSearchResult {
        types: filtered_types,
        categories,
        metrics: TypeSummaryMetrics {
            total_types: 3,
            certainty_high: 3,
            certainty_medium: 0,
            certainty_low: 0,
            flow_sensitive: 0,
            cache_hit_rate: "N/A".to_string(),
            analysis_speed: "N/A".to_string(),
        },
        connections: vec![],
        pagination: Some(pagination),
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
                id: "array".to_string(),
                name: "Массив".to_string(),
                category: "Platform".to_string(),
                certainty: 100,
                certainty_text: "Known 100%".to_string(),
                facets: vec!["Object".to_string(), "Collection".to_string()],
                source: "Static Analysis".to_string(),
                flow_sensitive: false,
                description: "Коллекция элементов с индексным доступом".to_string(),
                methods: vec!["Добавить".to_string(), "Удалить".to_string()],
                methods_count: Some(2),
                attributes_count: None,
                properties: vec![],
                enum_values: vec![],
            },
            x: 200.0,
            y: 150.0,
            connections: vec!["catalogs".to_string()],
        },
        TypeGraphNode {
            id: "catalogs".to_string(),
            type_info: TypeInfo {
                id: "catalogs_items".to_string(),
                name: "Справочники.Номенклатура".to_string(),
                category: "Configuration".to_string(),
                certainty: 100,
                certainty_text: "Known 100%".to_string(),
                facets: vec!["Manager".to_string(), "Reference".to_string(), "Object".to_string()],
                source: "Configuration".to_string(),
                flow_sensitive: false,
                methods: vec![],
                methods_count: Some(0),
                attributes_count: Some(5),
                description: "Иерархический справочник с поддержкой групп".to_string(),
                properties: vec![],
                enum_values: vec![],
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

/// Convert backend metrics to frontend format
fn convert_metrics(backend_metrics: BackendMetrics) -> TypeSummaryMetrics {
    TypeSummaryMetrics {
        total_types: backend_metrics.total_types as u32,
        certainty_high: backend_metrics.certainty_high as u32,
        certainty_medium: backend_metrics.certainty_medium as u32,
        certainty_low: backend_metrics.certainty_low as u32,
        flow_sensitive: backend_metrics.flow_sensitive as u32,
        cache_hit_rate: backend_metrics.cache_hit_rate,
        analysis_speed: backend_metrics.analysis_speed,
    }
}
