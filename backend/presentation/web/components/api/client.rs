//! API client для взаимодействия с бэкендом

#[cfg(feature = "web-ui")]
use gloo_net::http::Request;
#[cfg(feature = "web-ui")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "web-ui")]
use serde_json::Value;
#[cfg(feature = "web-ui")]
use thiserror::Error;

#[cfg(feature = "web-ui")]
use bsl_shared::domain::types::TypeResolution;

/// Кастомная ошибка для API клиента, чтобы реализовать нужные трейты
#[cfg(feature = "web-ui")]
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ApiError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Parsing error: {0}")]
    Parsing(String),
}

/// Структура для элемента типа в категории
#[cfg(feature = "web-ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryType {
    pub name: String,
    pub description: String,
}

/// Структура для данных о состоянии архитектуры
#[cfg(feature = "web-ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureHealth {
    pub components_active: u32,
    pub components_total: u32,
    pub cache_hit_rate: u32,
    pub analysis_speed_ms: u32,
}

/// Расширенные метрики для дашборда
#[cfg(feature = "web-ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub known_types: u32,
    pub inferred_types: u32,
    pub unknown_types: u32,
    pub platform_types: Vec<CategoryType>,
    pub config_types: Vec<CategoryType>,
    pub union_types: Vec<CategoryType>,
    pub flow_sensitive_types: Vec<CategoryType>,
    pub health: ArchitectureHealth,
}

/// API client для получения данных
#[cfg(feature = "web-ui")]
pub struct ApiClient;

#[cfg(feature = "web-ui")]
impl ApiClient {
    const BASE_URL: &'static str = "http://localhost:8080/api";

    pub async fn get_dashboard_metrics() -> Result<DashboardMetrics, ApiError> {
        Request::get(&format!("{}/metrics", Self::BASE_URL))
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?
            .json()
            .await
            .map_err(|e| ApiError::Parsing(e.to_string()))
    }

    pub async fn get_types() -> Result<Vec<TypeResolution>, ApiError> {
        let response: Value = Request::get(&format!("{}/types", Self::BASE_URL))
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?
            .json()
            .await
            .map_err(|e| ApiError::Parsing(e.to_string()))?;
        
        serde_json::from_value(response["types"].clone())
            .map_err(|e| ApiError::Parsing(e.to_string()))
    }

    pub async fn search_types(query: &str) -> Result<Vec<TypeResolution>, ApiError> {
        let url = format!("{}/search?q={}", Self::BASE_URL, query);
        let response: Value = Request::get(&url)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?
            .json()
            .await
            .map_err(|e| ApiError::Parsing(e.to_string()))?;

        serde_json::from_value(response["results"].clone())
            .map_err(|e| ApiError::Parsing(e.to_string()))
    }
}