//! Интеграционный тест: Web API отдаёт метрики через `/api/metrics`.

use axum::http::Request;
use bsl_backend::presentation::web::{create_router, AppState};
use bsl_backend::system::{build_deps_bundle_v2, EffectiveStartupInputs, SystemCoordinator};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn metrics_endpoint_returns_observability_payload() {
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
        .oneshot(
            Request::get("/api/metrics")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();

    let json: serde_json::Value = serde_json::from_slice(&body).expect("valid json");
    let types = json.get("types").expect("expected field `types`");
    let observability = json
        .get("observability")
        .expect("expected field `observability`");

    assert!(
        types
            .get("total_types")
            .and_then(|value| value.as_u64())
            .is_some(),
        "expected numeric `types.total_types`"
    );
    assert!(
        observability.get("counters").is_some(),
        "expected field `observability.counters`"
    );
    assert!(
        observability.get("gauges").is_some(),
        "expected field `observability.gauges`"
    );
    assert!(
        observability.get("histograms").is_some(),
        "expected field `observability.histograms`"
    );
    assert!(
        observability.get("rates").is_some(),
        "expected field `observability.rates`"
    );
    assert!(
        observability
            .get("uptime_seconds")
            .and_then(|value| value.as_u64())
            .is_some(),
        "expected numeric `observability.uptime_seconds`"
    );
}
