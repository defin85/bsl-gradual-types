//! API client functions
//! Uses shared DTOs from bsl-shared + frontend extensions

use crate::api::*; // Re-exported shared DTOs + extensions
use crate::config::get_config;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// Получить метрики системы типизации
pub async fn fetch_metrics() -> Result<MetricsDto, String> {
    let config = get_config();
    let url = config.api_url("metrics");

    match fetch_json::<MetricsDto>(&url).await {
        Ok(metrics) => Ok(metrics),
        Err(_) => {
            // Fallback to test data if API is not available
            Ok(MetricsDto {
                total_types: 87,
                certainty_high: 76,
                certainty_medium: 8,
                certainty_low: 3,
                flow_sensitive: 23,
                cache_hit_rate: "94%".to_string(),
                analysis_speed: "125ms".to_string(),
            })
        }
    }
}

/// Получить список типов с фильтрацией и пагинацией
pub async fn fetch_types(filters: TypeFilters) -> Result<AnalysisResultDto, String> {
    let config = get_config();

    // Построение URL с параметрами фильтрации
    let base_url = if let Some(ref query) = filters.search_query {
        if !query.is_empty() {
            format!(
                "{}?q={}&page={}&limit={}",
                config.api_url("search"),
                query,
                filters.page,
                filters.page_size
            )
        } else {
            format!(
                "{}?page={}&limit={}",
                config.api_url("types"),
                filters.page,
                filters.page_size
            )
        }
    } else {
        format!(
            "{}?page={}&limit={}",
            config.api_url("types"),
            filters.page,
            filters.page_size
        )
    };

    let mut url = base_url;

    // Фильтры категорий
    let any_category_checked = filters.show_platform
        || filters.show_configuration
        || filters.show_union
        || filters.show_dynamic;

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

    // Фильтры уровня определённости
    let any_certainty_checked =
        filters.show_high_certainty || filters.show_medium_certainty || filters.show_low_certainty;

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

    // Fetch from API
    match fetch_json::<AnalysisResultDto>(&url).await {
        Ok(result) => {
            web_sys::console::log_1(
                &format!("✅ Loaded {} types from API", result.types.len()).into(),
            );
            Ok(result)
        }
        Err(e) => {
            web_sys::console::error_1(&format!("❌ API error: {:?}", e).into());
            // Fallback to test data
            get_test_types(filters)
        }
    }
}

/// Get test data for types
fn get_test_types(filters: TypeFilters) -> Result<AnalysisResultDto, String> {
    let test_types = vec![
        TypeDto {
            id: "array".to_string(),
            name: "Массив".to_string(),
            category: "Platform".to_string(),
            certainty: 100,
            certainty_text: "Known 100%".to_string(),
            facets: vec!["Collection".to_string()],
            methods_count: Some(5),
            methods: vec![
                MethodDto {
                    name: "Добавить".to_string(),
                    english_name: Some("Add".to_string()),
                    return_type: None,
                    params: vec![ParamDto {
                        name: "Значение".to_string(),
                        param_type: "Произвольный".to_string(),
                        is_optional: false,
                        default_value: None,
                    }],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
                MethodDto {
                    name: "Удалить".to_string(),
                    english_name: Some("Delete".to_string()),
                    return_type: None,
                    params: vec![ParamDto {
                        name: "Индекс".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                        default_value: None,
                    }],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
                MethodDto {
                    name: "Очистить".to_string(),
                    english_name: Some("Clear".to_string()),
                    return_type: None,
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
            ],
            attributes_count: None,
            properties: vec!["Количество".to_string()],
            enum_values: None,
            tabular_sections: vec![],
            source: "Platform".to_string(),
            flow_sensitive: false,
            description: "Коллекция элементов с индексным доступом".to_string(),
            union_types: None,
            flow_analysis: None,
            connections: None,
            warning: None,
            recommendation: None,
        },
        TypeDto {
            id: "catalogs_items".to_string(),
            name: "Справочники.Номенклатура".to_string(),
            category: "Configuration".to_string(),
            certainty: 100,
            certainty_text: "Known 100%".to_string(),
            facets: vec!["Manager".to_string(), "Reference".to_string()],
            methods_count: Some(3),
            methods: vec![
                MethodDto {
                    name: "НайтиПоНаименованию".to_string(),
                    english_name: Some("FindByDescription".to_string()),
                    return_type: Some("СправочникСсылка.Номенклатура".to_string()),
                    params: vec![ParamDto {
                        name: "Наименование".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: false,
                        default_value: None,
                    }],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
                MethodDto {
                    name: "СоздатьЭлемент".to_string(),
                    english_name: Some("CreateItem".to_string()),
                    return_type: Some("СправочникОбъект.Номенклатура".to_string()),
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
            ],
            attributes_count: Some(5),
            properties: vec!["Наименование".to_string(), "Код".to_string()],
            enum_values: None,
            tabular_sections: vec![],
            source: "Configuration".to_string(),
            flow_sensitive: false,
            description: "Иерархический справочник с поддержкой групп".to_string(),
            union_types: None,
            flow_analysis: None,
            connections: None,
            warning: None,
            recommendation: None,
        },
        TypeDto {
            id: "string".to_string(),
            name: "Строка".to_string(),
            category: "Platform".to_string(),
            certainty: 100,
            certainty_text: "Known 100%".to_string(),
            facets: vec![],
            methods_count: Some(10),
            methods: vec![
                MethodDto {
                    name: "Длина".to_string(),
                    english_name: Some("StrLen".to_string()),
                    return_type: Some("Число".to_string()),
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
                MethodDto {
                    name: "НРег".to_string(),
                    english_name: Some("Lower".to_string()),
                    return_type: Some("Строка".to_string()),
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
                MethodDto {
                    name: "ВРег".to_string(),
                    english_name: Some("Upper".to_string()),
                    return_type: Some("Строка".to_string()),
                    params: vec![],
                    description: None,
                    is_deprecated: false,
                    is_constructor: false,
                },
            ],
            attributes_count: None,
            properties: vec![],
            enum_values: None,
            tabular_sections: vec![],
            source: "Platform".to_string(),
            flow_sensitive: false,
            description: "Строковый тип данных".to_string(),
            union_types: None,
            flow_analysis: None,
            connections: None,
            warning: None,
            recommendation: None,
        },
    ];

    // Apply filters
    let filtered_types: Vec<TypeDto> = test_types
        .into_iter()
        .filter(|t| filters.matches(t))
        .collect();

    let mut categories = HashMap::new();
    categories.insert(
        "Platform".to_string(),
        CategoryDto {
            color: "#3498db".to_string(),
            icon: "🔧".to_string(),
            count: 2,
        },
    );
    categories.insert(
        "Configuration".to_string(),
        CategoryDto {
            color: "#e74c3c".to_string(),
            icon: "⚙️".to_string(),
            count: 1,
        },
    );

    let pagination = PaginationDto {
        current_page: filters.page,
        page_size: filters.page_size,
        total_items: filtered_types.len(),
        total_pages: filtered_types.len().div_ceil(filters.page_size),
        has_prev: filters.page > 1,
        has_next: filters.page < filtered_types.len().div_ceil(filters.page_size),
    };

    Ok(AnalysisResultDto {
        types: filtered_types,
        categories,
        metrics: MetricsDto {
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

/// Получить граф типов (заглушка)
pub async fn fetch_type_graph() -> Result<Vec<ConnectionDto>, String> {
    // TODO: Implement real graph API call
    // For now return empty connections
    Ok(vec![])
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
