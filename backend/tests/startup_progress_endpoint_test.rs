//! Интеграционный тест: Web API отдаёт прогресс старта через `/api/startup/progress`.

use axum::http::Request;
use bsl_backend::presentation::web::{create_router, AppState};
use bsl_backend::system::{EffectiveStartupInputs, SystemCoordinator, build_deps_bundle_v2};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn startup_progress_endpoint_returns_json() {
    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator
        .start_with_paths_blocking(None, None, None, None)
        .expect("startup");

    let deps_bundle_v2 =
        build_deps_bundle_v2(coordinator.as_ref(), None, None).expect("deps bundle v2");

    let state = AppState {
        deps_bundle_v2: Arc::new(tokio::sync::RwLock::new(Arc::new(deps_bundle_v2))),
        system_coordinator: coordinator,
        syntax_helper_path: None,
        startup_inputs: Arc::new(tokio::sync::RwLock::new(EffectiveStartupInputs {
            syntax_helper_path: None,
            configuration_path: None,
            platform_version: None,
            cache_enabled: true,
            strict_fingerprint: false,
        })),
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
