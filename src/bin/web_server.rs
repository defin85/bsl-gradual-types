'''//! Веб-сервер для BSL Type System с интеграцией Leptos
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use bsl_gradual_types::{
    application::TypeSystemService,
    domain::types::{Certainty, ResolutionResult},
    system::SystemCoordinator,
};
use leptos::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::services::ServeDir;

// --- Структуры для API ответов ---

#[derive(Serialize, Clone)]
pub struct ApiMetrics {
    pub total_types: usize,
    pub known_types: usize,
    pub inferred_types: usize,
    pub unknown_types: usize,
}

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

#[derive(Serialize, Clone)]
pub struct ApiUnionType {
    pub type_name: String,
    pub weight: u8,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

// --- Состояние приложения Axum ---

#[derive(Clone)]
struct AppState {
    type_service: Arc<TypeSystemService>,
    leptos_options: LeptosOptions,
}

// --- Точка входа ---

#[tokio::main]
async fn main() {
    // --- Инициализация системы типов ---
    let coordinator = Arc::new(SystemCoordinator::new());
    let type_service = coordinator.type_service();

    // --- Настройка Leptos ---
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(bsl_gradual_types_frontend::app::App);

    // --- Создание состояния Axum ---
    let app_state = AppState {
        type_service: type_service.clone(),
        leptos_options,
    };

    // --- Создание роутера Axum ---
    let app = Router::new()
        .route("/api/metrics", get(get_metrics))
        .route("/api/types", get(get_types))
        .route("/api/search", get(search_types))
        .leptos_routes(&app_state, routes, bsl_gradual_types_frontend::app::App)
        .fallback(fallback)
        .with_state(app_state);

    // --- Запуск сервера ---
    println!("🚀 BSL Type System Web UI listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

// --- Обработчики API ---

async fn get_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let resolver = state.type_service.resolver();
    let all_types = resolver.get_all_platform_globals();

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

    let metrics = ApiMetrics {
        total_types: all_types.len(),
        known_types: known,
        inferred_types: inferred,
        unknown_types: unknown,
    };

    Json(metrics)
}

async fn get_types(State(state): State<AppState>) -> impl IntoResponse {
    let resolver = state.type_service.resolver();
    let all_types = resolver.get_all_platform_globals();

    let api_types: Vec<ApiType> = all_types
        .iter()
        .map(|(name, res)| {
            let (category, source) = match &res.source {
                bsl_gradual_types::domain::types::ResolutionSource::Static => {
                    ("Platform".to_string(), "Static".to_string())
                }
                bsl_gradual_types::domain::types::ResolutionSource::Inferred => {
                    ("Inferred".to_string(), "Inferred".to_string())
                }
                bsl_gradual_types::domain::types::ResolutionSource::Runtime => {
                    ("Runtime".to_string(), "Runtime".to_string())
                }
            };

            let union_types = if let ResolutionResult::Union(union) = &res.result {
                Some(
                    union
                        .types
                        .iter()
                        .map(|wt| ApiUnionType {
                            type_name: wt.type_.to_string(),
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

async fn search_types(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    match state.type_service.search_types(&query.q).await {
        Ok(results) => Json(results).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
            .into_response(),
    }
}

// --- Обработчик для статических файлов ---

async fn fallback(
    State(state): State<AppState>,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    let root = state.leptos_options.site_root.clone();
    let path = format!("{}{}", root, uri.path());
    let res = ServeDir::new(path).oneshot(axum::extract::Request::new(axum::body::Body::empty())).await.unwrap();
    res.into_response()
}

// --- Пустышка для main, если фича web-ui отключена ---
#[cfg(not(feature = "ssr"))]
pub fn main() {
    // ssr-only function
}
''