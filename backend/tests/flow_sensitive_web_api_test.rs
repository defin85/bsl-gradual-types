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

fn test_state_with_empty_deps_bundle() -> AppState {
    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator
        .start_with_paths_blocking(None, None, None, None)
        .expect("startup");

    let empty_deps_coordinator = SystemCoordinator::new();
    let empty_deps_bundle =
        build_deps_bundle_v2(&empty_deps_coordinator, None, None).expect("empty deps bundle v2");
    assert_eq!(
        empty_deps_bundle
            .semantic_deps
            .repository
            .get_stats()
            .total_types,
        0,
        "test precondition: empty deps snapshot must have no semantic types"
    );

    AppState {
        deps_bundle_v2: Arc::new(tokio::sync::RwLock::new(Arc::new(empty_deps_bundle))),
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

fn type_index_reason_total(metrics: &serde_json::Value) -> u64 {
    metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .map(|counters| {
            counters
                .iter()
                .filter(|(key, _)| {
                    key.starts_with("intellisense_v2_type_index_reason_total_reason_")
                })
                .map(|(_, value)| value.as_u64().unwrap_or(0))
                .sum()
        })
        .unwrap_or(0)
}

fn interactive_fail_closed_hover_total(metrics: &serde_json::Value) -> u64 {
    metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .map(|counters| {
            counters
                .iter()
                .filter(|(key, _)| {
                    key.starts_with(
                        "intellisense_v2_fail_closed_reason_total_origin_web_operation_hover_reason_",
                    )
                })
                .map(|(_, value)| value.as_u64().unwrap_or(0))
                .sum()
        })
        .unwrap_or(0)
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

#[tokio::test]
async fn hover_endpoints_emit_type_index_reason_metrics() {
    let state = test_state();
    let coordinator = state.system_coordinator.clone();
    let app = create_router(state, "backend/static", true);

    let code = "Процедура T()\n    Arr = Новый Массив;\n    ДляHover = Arr;\nКонецПроцедуры\n";
    let baseline_total = type_index_reason_total(&coordinator.observability_metrics());

    let hover_resp = app
        .clone()
        .oneshot(
            Request::post("/api/hover")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "line": 2,
                        "column": 15
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(hover_resp.status().is_success());

    let after_hover_total = type_index_reason_total(&coordinator.observability_metrics());
    assert!(
        after_hover_total > baseline_total,
        "web hover must emit type-index reasons: before={baseline_total}, after={after_hover_total}"
    );

    let enhanced_hover_resp = app
        .oneshot(
            Request::post("/api/hover/enhanced")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "line": 2,
                        "column": 15
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(enhanced_hover_resp.status().is_success());

    let after_enhanced_total = type_index_reason_total(&coordinator.observability_metrics());
    assert!(
        after_enhanced_total > after_hover_total,
        "web enhanced hover must emit type-index reasons: before={after_hover_total}, after={after_enhanced_total}"
    );
}

#[tokio::test]
async fn hover_endpoints_fail_closed_on_missing_canonical_artifacts() {
    let state = test_state_with_empty_deps_bundle();
    let coordinator = state.system_coordinator.clone();
    let app = create_router(state, "backend/static", true);
    let code = "Процедура T()\n    Arr = Новый Массив;\n    ДляHover = Arr;\nКонецПроцедуры\n";
    let baseline_total = interactive_fail_closed_hover_total(&coordinator.observability_metrics());

    let hover_resp = app
        .clone()
        .oneshot(
            Request::post("/api/hover")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "line": 2,
                        "column": 15
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        hover_resp.status().is_success(),
        "web hover must fail closed instead of returning transport error on missing semantic deps: {}",
        hover_resp.status()
    );
    let hover_body = axum::body::to_bytes(hover_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let hover_json: serde_json::Value = serde_json::from_slice(&hover_body).expect("valid json");
    assert!(
        hover_json.get("hover").is_some(),
        "hover fail-closed response must keep transport shape: {hover_json}"
    );
    assert!(
        hover_json.get("hover").unwrap().is_null(),
        "hover fail-closed response must return null semantic payload: {hover_json}"
    );

    let enhanced_resp = app
        .oneshot(
            Request::post("/api/hover/enhanced")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "line": 2,
                        "column": 15
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        enhanced_resp.status().is_success(),
        "web enhanced hover must fail closed instead of returning transport error on missing semantic deps: {}",
        enhanced_resp.status()
    );
    let enhanced_body = axum::body::to_bytes(enhanced_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let enhanced_json: serde_json::Value =
        serde_json::from_slice(&enhanced_body).expect("valid json");
    assert_eq!(
        enhanced_json
            .get("hoverText")
            .and_then(|value| value.as_str())
            .unwrap_or_default(),
        "No information available",
        "enhanced hover fail-closed response must keep public unavailable payload: {enhanced_json}"
    );

    let after_total = interactive_fail_closed_hover_total(&coordinator.observability_metrics());
    assert!(
        after_total >= baseline_total + 2,
        "web hover and enhanced hover must emit shared fail-closed reasons on missing semantic deps: before={baseline_total}, after={after_total}"
    );
}
