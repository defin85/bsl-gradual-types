//! Интеграционные тесты: Web API contract для flow-sensitive режима (opt-in).

use axum::http::{header, Request, StatusCode};
use bsl_backend::presentation::web::{create_router, AppState};
use bsl_backend::system::{build_deps_bundle_v2, EffectiveStartupInputs, SystemCoordinator};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

fn test_state() -> AppState {
    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator
        .start_with_paths_blocking(None, None, None, None)
        .expect("startup");

    let deps_bundle_v2 =
        build_deps_bundle_v2(coordinator.as_ref(), None, None).expect("deps bundle v2");

    AppState {
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
    }
}

#[tokio::test]
async fn semantic_tree_rejects_legacy_include_flow_sensitive() {
    let app = create_router(test_state(), "backend/static", true);

    let resp = app
        .oneshot(
            Request::post("/api/semantic-tree")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": "Процедура T()\nКонецПроцедуры\n",
                        "include_flow_sensitive": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn semantic_tree_accepts_include_flow_sensitive_camel_case() {
    let app = create_router(test_state(), "backend/static", true);

    let resp = app
        .oneshot(
            Request::post("/api/semantic-tree")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": "Процедура T()\nКонецПроцедуры\n",
                        "includeFlowSensitive": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(resp.status().is_success());
}

#[tokio::test]
async fn diagnostics_includes_null_safety_only_when_enabled() {
    let app = create_router(test_state(), "backend/static", true);

    let code = "Процедура T()\n    x = Null;\n    x.Добавить(1);\nКонецПроцедуры\n";

    let resp_base = app
        .clone()
        .oneshot(
            Request::post("/api/diagnostics")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(json!({ "code": code }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp_base.status().is_success());
    let body_base = axum::body::to_bytes(resp_base.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_base: serde_json::Value = serde_json::from_slice(&body_base).expect("valid json");
    let base_has_null_warning = json_base
        .get("semanticErrors")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
        .any(|m| m.contains("может быть Null"));
    assert!(
        !base_has_null_warning,
        "expected base diagnostics to not include null-safety warnings, json={}",
        json_base
    );

    let resp_flow = app
        .oneshot(
            Request::post("/api/diagnostics")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({ "code": code, "includeFlowSensitive": true }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp_flow.status().is_success());
    let body_flow = axum::body::to_bytes(resp_flow.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_flow: serde_json::Value = serde_json::from_slice(&body_flow).expect("valid json");
    let flow_has_null_warning = json_flow
        .get("semanticErrors")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
        .any(|m| m.contains("может быть Null"));
    assert!(
        flow_has_null_warning,
        "expected flow-sensitive diagnostics to include null-safety warnings, json={}",
        json_flow
    );
}
