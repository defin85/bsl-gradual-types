//! Web API smoke: flow-sensitive results are gated by request flag.

use axum::body::Body;
use axum::http::Request;
use bsl_backend::presentation::web::{create_router, AppState};
use bsl_backend::system::{build_deps_bundle_v2, EffectiveStartupInputs, SystemCoordinator};
use std::sync::Arc;
use tower::ServiceExt;

fn build_app() -> axum::Router {
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

    create_router(state, "backend/static", true)
}

#[tokio::test]
async fn diagnostics_endpoint_flow_sensitive_null_safety_is_gated_by_flag() {
    let app = build_app();
    let code = "Procedure Test()\n\
                x = Null;\n\
                x.Method();\n\
                EndProcedure\n";

    let disabled_body = serde_json::json!({ "code": code });
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/diagnostics")
                .header("content-type", "application/json")
                .body(Body::from(disabled_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    let semantic = json
        .get("semanticErrors")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        semantic.iter().all(|entry| !entry
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .contains("может быть Null")),
        "unexpected null-safety diagnostics when disabled: {}",
        json
    );

    let enabled_body = serde_json::json!({ "code": code, "includeFlowSensitive": true });
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/diagnostics")
                .header("content-type", "application/json")
                .body(Body::from(enabled_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    let semantic = json
        .get("semanticErrors")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        semantic.iter().any(|entry| entry
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .contains("может быть Null")),
        "expected null-safety diagnostics when enabled: {}",
        json
    );

    // Breaking: legacy snake_case flag must be rejected with 400.
    let legacy_body = serde_json::json!({ "code": code, "include_flow_sensitive": true });
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/diagnostics")
                .header("content-type", "application/json")
                .body(Body::from(legacy_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn semantic_tree_flow_variants_are_gated_by_flag_and_default_off() {
    let app = build_app();
    let code = "Процедура Test()\n\
                x = 0;\n\
                Если ТипЗнч(x) = Тип(\"Массив\") Тогда\n\
                    y = x;\n\
                КонецЕсли;\n\
                КонецПроцедуры\n";

    let disabled_body = serde_json::json!({
        "code": code,
        "file_path": "inline.bsl",
        "compact": false,
        "include_call_graph": false
    });
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/semantic-tree")
                .header("content-type", "application/json")
                .body(Body::from(disabled_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    let symbol_table = json
        .get("symbol_table")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let has_flow_variants = symbol_table.values().any(|value| {
        value
            .get("flow_variants")
            .and_then(|fv| fv.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false)
    });
    assert!(
        !has_flow_variants,
        "expected semantic-tree to omit flow_variants by default: {}",
        json
    );

    let enabled_body = serde_json::json!({
        "code": code,
        "file_path": "inline.bsl",
        "compact": false,
        "include_call_graph": false,
        "includeFlowSensitive": true
    });
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/semantic-tree")
                .header("content-type", "application/json")
                .body(Body::from(enabled_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    let symbol_table = json
        .get("symbol_table")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let has_flow_variants = symbol_table.values().any(|value| {
        value
            .get("flow_variants")
            .and_then(|fv| fv.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false)
    });
    assert!(
        has_flow_variants,
        "expected semantic-tree to include flow_variants when enabled: {}",
        json
    );

    // Breaking: legacy snake_case flag must be rejected with 400.
    let legacy_body = serde_json::json!({
        "code": code,
        "file_path": "inline.bsl",
        "compact": false,
        "include_call_graph": false,
        "include_flow_sensitive": true
    });
    let resp = app
        .clone()
        .oneshot(
            Request::post("/api/semantic-tree")
                .header("content-type", "application/json")
                .body(Body::from(legacy_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::BAD_REQUEST);
}
