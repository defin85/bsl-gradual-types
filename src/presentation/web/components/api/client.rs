//! API client для взаимодействия с бэкендом

#[cfg(feature = "web-ui")]
use gloo_net::http::Request;
#[cfg(feature = "web-ui")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "web-ui")]
use serde_json::Value;

#[cfg(feature = "web-ui")]
use crate::domain::types::TypeResolution;

/// Метрики для дашборда
#[cfg(feature = "web-ui")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub total_types: u32,
    pub known_types: u32,
    pub inferred_types: u32,
    pub unknown_types: u32,
    pub flow_sensitive_types: u32,
}

/// API client для получения данных
#[cfg(feature = "web-ui")]
pub struct ApiClient;

#[cfg(feature = "web-ui")]
impl ApiClient {
    const BASE_URL: &'static str = "http://localhost:8080/api";
    
    pub async fn get_dashboard_metrics() -> Result<DashboardMetrics, Box<dyn std::error::Error>> {
        let response = Request::get(&format!("{}/metrics", Self::BASE_URL))
            .send()
            .await?;
            
        let metrics: DashboardMetrics = response.json().await?;
        Ok(metrics)
    }
    
    pub async fn get_types() -> Result<Vec<TypeResolution>, Box<dyn std::error::Error>> {
        let response = Request::get(&format!("{}/types", Self::BASE_URL))
            .send()
            .await?;
            
        let data: Value = response.json().await?;
        let types: Vec<TypeResolution> = serde_json::from_value(data["types"].clone())?;
        Ok(types)
    }
    
    pub async fn search_types(query: &str) -> Result<Vec<TypeResolution>, Box<dyn std::error::Error>> {
        let url = format!("{}/search?q={}", Self::BASE_URL, query);
        let response = Request::get(&url).send().await?;
        
        let data: Value = response.json().await?;
        let results: Vec<TypeResolution> = serde_json::from_value(data["results"].clone())?;
        Ok(results)
    }
}
