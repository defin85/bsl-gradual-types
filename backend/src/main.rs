//! CSR server: serves API and static SPA (no SSR)

#[cfg(feature = "web-ui")]
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
#[cfg(feature = "web-ui")]
use bsl_backend::{application::TypeSystemService, system::SystemCoordinator};
#[cfg(feature = "web-ui")]
use bsl_shared::domain::types::{Certainty, ResolutionResult};
#[cfg(feature = "web-ui")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "web-ui")]
use std::sync::Arc;
#[cfg(feature = "web-ui")]
use tower_http::services::{ServeDir, ServeFile};

// --- DTOs for API ---
#[cfg(feature = "web-ui")]
#[derive(Serialize, Clone)]
pub struct ApiMetrics {
    pub total_types: usize,
    pub known_types: usize,
    pub inferred_types: usize,
    pub unknown_types: usize,
}

#[cfg(feature = "web-ui")]
#[derive(Serialize, Clone)]
pub struct ApiType {
    pub id: String,
    pub name: String,
    pub certainty: u8,
    pub category: String,
    pub source: String,
    pub facets: Vec<String>,
    pub union_types: Option<Vec<ApiUnionType>>,
}

#[cfg(feature = "web-ui")]
#[derive(Serialize, Clone)]
pub struct ApiUnionType {
    pub type_name: String,
    pub weight: u8,
}

#[cfg(feature = "web-ui")]
#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

#[cfg(feature = "web-ui")]
#[derive(Clone)]
struct AppState {
    type_service: Arc<TypeSystemService>,
}

#[cfg(feature = "web-ui")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let system_coord = Arc::new(SystemCoordinator::new());
    let type_service = system_coord.get_type_service();

    let app_state = AppState {
        type_service: type_service.clone(),
    };

    // Static SPA from trunk output (see Cargo.toml [package.metadata.leptos])
    let static_dir =
        ServeDir::new("target/site").not_found_service(ServeFile::new("target/site/index.html"));

    let app = Router::new()
        .route("/api/metrics", get(get_metrics))
        .route("/api/types", get(get_types))
        .route("/api/search", get(search_types))
        .fallback_service(static_dir)
        .with_state(app_state);

    let addr = "127.0.0.1:8080";
    println!(
        "\u{1F680} BSL Type System Web UI (CSR) listening on http://{}",
        addr
    );
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

#[cfg(not(feature = "web-ui"))]
fn main() {
    println!("BSL Type System - LSP only mode");
    println!("Web UI disabled. Use --features web-ui to enable.");
    println!("This would start the LSP server...");
}

#[cfg(feature = "web-ui")]
async fn get_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let all_types = state.type_service.get_all_platform_globals();

    let mut known = 0;
    let mut inferred = 0;
    let mut unknown = 0;

    for res in all_types.values() {
        match res.certainty {
            Certainty::Known => known += 1,
            Certainty::Inferred(_) => inferred += 1,
            Certainty::Unknown => unknown += 1,
        }
    }

    Json(ApiMetrics {
        total_types: all_types.len(),
        known_types: known,
        inferred_types: inferred,
        unknown_types: unknown,
    })
}

#[cfg(feature = "web-ui")]
async fn get_types(State(state): State<AppState>) -> impl IntoResponse {
    let all_types = state.type_service.get_all_platform_globals();

    let api_types: Vec<ApiType> = all_types
        .iter()
        .map(|(name, res)| {
            let (category, source) = match &res.source {
                bsl_shared::domain::types::ResolutionSource::Static => {
                    ("Platform".to_string(), "Static".to_string())
                }
                bsl_shared::domain::types::ResolutionSource::Inferred => {
                    ("Inferred".to_string(), "Inferred".to_string())
                }
                bsl_shared::domain::types::ResolutionSource::Annotated => {
                    ("Annotated".to_string(), "Annotated".to_string())
                }
                bsl_shared::domain::types::ResolutionSource::Runtime => {
                    ("Runtime".to_string(), "Runtime".to_string())
                }
                bsl_shared::domain::types::ResolutionSource::Predicted => {
                    ("Predicted".to_string(), "Predicted".to_string())
                }
            };

            let union_types = if let ResolutionResult::Union(types) = &res.result {
                Some(
                    types
                        .iter()
                        .map(|wt| ApiUnionType {
                            type_name: format!("{:?}", wt.type_),
                            weight: (wt.weight * 100.0) as u8,
                        })
                        .collect(),
                )
            } else {
                None
            };

            ApiType {
                id: name.clone(),
                name: name.clone(),
                certainty: match res.certainty {
                    Certainty::Known => 100,
                    Certainty::Inferred(val) => (val * 100.0) as u8,
                    Certainty::Unknown => 0,
                },
                category,
                source,
                facets: res
                    .available_facets
                    .iter()
                    .map(|f| format!("{:?}", f))
                    .collect(),
                union_types,
            }
        })
        .collect();

    Json(api_types)
}

#[cfg(feature = "web-ui")]
async fn search_types(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    match state.type_service.search_types(&query.q).await {
        Ok(results) => Json(results).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
