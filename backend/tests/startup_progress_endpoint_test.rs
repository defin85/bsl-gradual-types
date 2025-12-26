//! Интеграционный тест: Web API отдаёт прогресс старта через `/api/startup/progress`.

use axum::http::Request;
use bsl_backend::presentation::web::{create_router, AppState};
use bsl_backend::system::SystemCoordinator;
use tower::ServiceExt;

#[tokio::test]
async fn startup_progress_endpoint_returns_json() {
    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths_blocking(None, None, None, None)
        .expect("startup");

    let type_service = coordinator
        .type_service()
        .expect("TypeSystemService should be initialized after start");

    let state = AppState {
        type_service,
        system_coordinator: std::sync::Arc::new(coordinator),
    };

    let app = create_router(state, "backend/static", true);
    let resp = app
        .oneshot(Request::get("/api/startup/progress").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&body).expect("valid json");
    assert!(json.get("phase").is_some(), "expected field `phase`");
    assert!(json.get("percentage").is_some(), "expected field `percentage`");
    assert!(json.get("done").is_some(), "expected field `done`");
}
