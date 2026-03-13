//! Интеграционные тесты: Web API contract для flow-sensitive режима (opt-in).

use axum::http::{header, Request, StatusCode};
use bsl_backend::presentation::web::{create_router, AppState};
use bsl_backend::system::{
    build_deps_bundle_v2, EffectiveStartupInputs, IndexItem, IndexItemKind, IndexKind,
    IndexSnapshot, IndexSnapshotId, SystemCoordinator, TypeKind,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceExt;

const OBJECT_MODULE_FILE_PATH: &str = "Documents/Док1/Ext/ObjectModule.bsl";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn syntax_helper_root() -> PathBuf {
    let path = workspace_root().join("examples").join("syntax_helper");
    assert!(
        path.exists(),
        "syntax helper path does not exist: {}",
        path.display()
    );
    path
}

fn conf_fixture_root() -> PathBuf {
    let path = workspace_root()
        .join("examples")
        .join("conf")
        .join("conf_test");
    assert!(
        path.exists(),
        "conf fixture path does not exist: {}",
        path.display()
    );
    path
}

fn startup_inputs(
    syntax_helper_path: Option<PathBuf>,
    configuration_path: Option<PathBuf>,
    platform_version: Option<&str>,
) -> EffectiveStartupInputs {
    EffectiveStartupInputs {
        syntax_helper_path,
        configuration_path,
        platform_version: platform_version.map(str::to_string),
        cache_enabled: true,
        strict_fingerprint: false,
    }
}

fn test_state_with_paths(
    syntax_helper_path: Option<PathBuf>,
    configuration_path: Option<PathBuf>,
    platform_version: Option<&str>,
) -> AppState {
    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator
        .start_with_paths_blocking(
            syntax_helper_path.as_deref(),
            configuration_path.as_deref(),
            platform_version,
            None,
        )
        .expect("startup");

    let deps_bundle_v2 = build_deps_bundle_v2(
        coordinator.as_ref(),
        syntax_helper_path.as_deref(),
        configuration_path.as_deref(),
    )
    .expect("deps bundle v2");

    AppState {
        deps_bundle_v2: Arc::new(tokio::sync::RwLock::new(Arc::new(deps_bundle_v2))),
        system_coordinator: coordinator,
        syntax_helper_path: syntax_helper_path.clone(),
        startup_inputs: Arc::new(tokio::sync::RwLock::new(startup_inputs(
            syntax_helper_path,
            configuration_path,
            platform_version,
        ))),
    }
}

fn test_state() -> AppState {
    test_state_with_paths(None, None, None)
}

fn test_state_with_conf_fixture() -> AppState {
    test_state_with_paths(
        Some(syntax_helper_root()),
        Some(conf_fixture_root()),
        Some("8.3.25"),
    )
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

fn test_state_with_empty_deps_bundle_and_polluted_search_index() -> AppState {
    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator
        .start_with_paths_blocking(None, None, None, None)
        .expect("startup");

    let empty_deps_coordinator = SystemCoordinator::new();
    let mut polluted_snapshot =
        IndexSnapshot::empty(IndexSnapshotId::from_hash("web-hover-search-only-snapshot"));
    Arc::make_mut(&mut polluted_snapshot.type_index).insert(
        "SearchOnlyType".to_string(),
        Arc::new(IndexItem::new(
            "SearchOnlyType".to_string(),
            IndexItemKind::Type(TypeKind::Generic),
            IndexKind::Type,
        )),
    );
    empty_deps_coordinator
        .intellisense_index()
        .replace_snapshot(polluted_snapshot);

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
    assert!(
        empty_deps_bundle
            .index_snapshot
            .type_index
            .contains_key("SearchOnlyType"),
        "test precondition: polluted search snapshot must stay visible to web adapter"
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

#[tokio::test]
async fn hover_endpoints_do_not_backfill_from_polluted_search_index() {
    let state = test_state_with_empty_deps_bundle_and_polluted_search_index();
    let coordinator = state.system_coordinator.clone();
    let app = create_router(state, "backend/static", true);
    let code = "Процедура T()\n    SearchOnly.\nКонецПроцедуры\n";
    let baseline_total = interactive_fail_closed_hover_total(&coordinator.observability_metrics());

    let hover_resp = app
        .clone()
        .oneshot(
            Request::post("/api/hover")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "line": 1,
                        "column": 12
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(hover_resp.status().is_success());
    let hover_body = axum::body::to_bytes(hover_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let hover_json: serde_json::Value = serde_json::from_slice(&hover_body).expect("valid json");
    assert_eq!(
        hover_json.get("hover"),
        Some(&serde_json::Value::Null),
        "web hover must stay fail-closed when only polluted search index is available: {hover_json}"
    );
    assert!(
        !String::from_utf8_lossy(&hover_body).contains("SearchOnlyType"),
        "web hover must not leak polluted search/index payload: {hover_json}"
    );

    let enhanced_resp = app
        .oneshot(
            Request::post("/api/hover/enhanced")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "line": 1,
                        "column": 12
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(enhanced_resp.status().is_success());
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
        "web enhanced hover must keep fail-closed payload when only polluted search index is available: {enhanced_json}"
    );
    assert!(
        !String::from_utf8_lossy(&enhanced_body).contains("SearchOnlyType"),
        "web enhanced hover must not leak polluted search/index payload: {enhanced_json}"
    );

    let after_total = interactive_fail_closed_hover_total(&coordinator.observability_metrics());
    assert!(
        after_total >= baseline_total + 2,
        "web hover endpoints must emit shared fail-closed reasons instead of search rescue: before={baseline_total}, after={after_total}"
    );
}

#[tokio::test]
async fn diagnostics_and_validate_do_not_backfill_from_polluted_search_index() {
    let app = create_router(
        test_state_with_empty_deps_bundle_and_polluted_search_index(),
        "backend/static",
        true,
    );
    let code = "Процедура T()\n    Проверка = SearchOnlyType.Unknown;\nКонецПроцедуры\n";

    let diagnostics_resp = app
        .clone()
        .oneshot(
            Request::post("/api/diagnostics")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(json!({ "code": code }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(diagnostics_resp.status().is_success());
    let diagnostics_body = axum::body::to_bytes(diagnostics_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let diagnostics_json: serde_json::Value =
        serde_json::from_slice(&diagnostics_body).expect("valid json");
    let diagnostics_messages = diagnostics_json
        .get("semanticErrors")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.get("message").and_then(|message| message.as_str()))
        .collect::<Vec<_>>();
    assert!(
        diagnostics_json
            .get("syntaxErrors")
            .and_then(|value| value.as_array())
            .is_some_and(|errors| errors.is_empty()),
        "diagnostics regression must exercise semantic path rather than syntax rejection: {diagnostics_json}"
    );
    assert!(
        diagnostics_messages
            .iter()
            .any(|message| {
                message.contains("Необъявленная переменная")
                    && message.contains("SearchOnlyType")
            }),
        "diagnostics must stay on unresolved-variable path when only polluted search index is available: {diagnostics_json}"
    );

    let validate_resp = app
        .oneshot(
            Request::post("/api/validate")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(json!({ "code": code }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(validate_resp.status().is_success());
    let validate_body = axum::body::to_bytes(validate_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let validate_json: serde_json::Value =
        serde_json::from_slice(&validate_body).expect("valid json");
    assert_eq!(
        validate_json
            .get("isValid")
            .and_then(|value| value.as_bool()),
        Some(false),
        "validate must remain invalid when only polluted search index is available: {validate_json}"
    );
    let validate_messages = validate_json
        .get("errors")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.get("message").and_then(|message| message.as_str()))
        .collect::<Vec<_>>();
    assert!(
        validate_messages
            .iter()
            .any(|message| {
                message.contains("Необъявленная переменная")
                    && message.contains("SearchOnlyType")
            }),
        "validate must stay on unresolved-variable path instead of using polluted search index: {validate_json}"
    );
}

#[tokio::test]
async fn hover_endpoints_use_file_path_for_module_context_bindings() {
    let app = create_router(test_state_with_conf_fixture(), "backend/static", true);
    let code = "Процедура Тест()\n    x = Объект;\nКонецПроцедуры\n";

    let hover_without_path = app
        .clone()
        .oneshot(
            Request::post("/api/hover")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "line": 1,
                        "column": 8
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(hover_without_path.status().is_success());
    let hover_without_path_body = axum::body::to_bytes(hover_without_path.into_body(), usize::MAX)
        .await
        .unwrap();
    let hover_without_path_json: serde_json::Value =
        serde_json::from_slice(&hover_without_path_body).expect("valid json");
    let hover_without_path_text = hover_without_path_json
        .get("hover")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert!(
        !hover_without_path_text.contains("ДокументОбъект.Док1"),
        "synthetic inline path must not masquerade as object-module semantic context: {hover_without_path_json}"
    );

    let hover_with_path = app
        .clone()
        .oneshot(
            Request::post("/api/hover")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "line": 1,
                        "column": 8,
                        "filePath": OBJECT_MODULE_FILE_PATH
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(hover_with_path.status().is_success());
    let hover_with_path_body = axum::body::to_bytes(hover_with_path.into_body(), usize::MAX)
        .await
        .unwrap();
    let hover_with_path_json: serde_json::Value =
        serde_json::from_slice(&hover_with_path_body).expect("valid json");
    let hover_with_path_text = hover_with_path_json
        .get("hover")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert!(
        hover_with_path_text.contains("ДокументОбъект.Док1"),
        "hover must honor filePath for object-module canonical binding: {hover_with_path_json}"
    );

    let enhanced_without_path = app
        .clone()
        .oneshot(
            Request::post("/api/hover/enhanced")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "line": 1,
                        "column": 8
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(enhanced_without_path.status().is_success());
    let enhanced_without_path_body =
        axum::body::to_bytes(enhanced_without_path.into_body(), usize::MAX)
            .await
            .unwrap();
    let enhanced_without_path_json: serde_json::Value =
        serde_json::from_slice(&enhanced_without_path_body).expect("valid json");
    let enhanced_without_path_text = enhanced_without_path_json
        .get("hoverText")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert!(
        !enhanced_without_path_text.contains("ДокументОбъект.Док1"),
        "enhanced hover without filePath must not fabricate object-module context: {enhanced_without_path_json}"
    );

    let enhanced_with_path = app
        .oneshot(
            Request::post("/api/hover/enhanced")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "line": 1,
                        "column": 8,
                        "filePath": OBJECT_MODULE_FILE_PATH
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(enhanced_with_path.status().is_success());
    let enhanced_with_path_body = axum::body::to_bytes(enhanced_with_path.into_body(), usize::MAX)
        .await
        .unwrap();
    let enhanced_with_path_json: serde_json::Value =
        serde_json::from_slice(&enhanced_with_path_body).expect("valid json");
    let enhanced_with_path_text = enhanced_with_path_json
        .get("hoverText")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert!(
        enhanced_with_path_text.contains("ДокументОбъект.Док1"),
        "enhanced hover must honor filePath for object-module canonical binding: {enhanced_with_path_json}"
    );
}

#[tokio::test]
async fn diagnostics_and_validate_use_file_path_for_module_context_bindings() {
    let app = create_router(test_state_with_conf_fixture(), "backend/static", true);
    let code = "Процедура Тест()\n    Проверка = Объект.Ссылка;\nКонецПроцедуры\n";

    let diagnostics_without_path = app
        .clone()
        .oneshot(
            Request::post("/api/diagnostics")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(json!({ "code": code }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(diagnostics_without_path.status().is_success());
    let diagnostics_without_path_body =
        axum::body::to_bytes(diagnostics_without_path.into_body(), usize::MAX)
            .await
            .unwrap();
    let diagnostics_without_path_json: serde_json::Value =
        serde_json::from_slice(&diagnostics_without_path_body).expect("valid json");
    let diagnostics_without_messages = diagnostics_without_path_json
        .get("semanticErrors")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.get("message").and_then(|message| message.as_str()))
        .collect::<Vec<_>>();
    assert!(
        diagnostics_without_messages
            .iter()
            .any(|message| message.contains("Необъявленная переменная") && message.contains("Объект")),
        "diagnostics without filePath must stay unresolved for object-module-only binding: {diagnostics_without_path_json}"
    );

    let diagnostics_with_path = app
        .clone()
        .oneshot(
            Request::post("/api/diagnostics")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "filePath": OBJECT_MODULE_FILE_PATH
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(diagnostics_with_path.status().is_success());
    let diagnostics_with_path_body =
        axum::body::to_bytes(diagnostics_with_path.into_body(), usize::MAX)
            .await
            .unwrap();
    let diagnostics_with_path_json: serde_json::Value =
        serde_json::from_slice(&diagnostics_with_path_body).expect("valid json");
    let diagnostics_with_messages = diagnostics_with_path_json
        .get("semanticErrors")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.get("message").and_then(|message| message.as_str()))
        .collect::<Vec<_>>();
    assert!(
        diagnostics_with_messages.iter().all(|message| {
            !message.contains("Необъявленная переменная") || !message.contains("Объект")
        }),
        "diagnostics with filePath must stop failing at unresolved object-module binding: {diagnostics_with_path_json}"
    );
    assert!(
        diagnostics_with_messages.iter().any(|message| {
            message.contains("Ссылка") && message.contains("ДокументОбъект.Док1")
        }),
        "diagnostics with filePath must use resolved object-module type in semantic error: {diagnostics_with_path_json}"
    );

    let validate_with_path = app
        .oneshot(
            Request::post("/api/validate")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "code": code,
                        "filePath": OBJECT_MODULE_FILE_PATH
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(validate_with_path.status().is_success());
    let validate_with_path_body = axum::body::to_bytes(validate_with_path.into_body(), usize::MAX)
        .await
        .unwrap();
    let validate_with_path_json: serde_json::Value =
        serde_json::from_slice(&validate_with_path_body).expect("valid json");
    assert_eq!(
        validate_with_path_json
            .get("isValid")
            .and_then(|value| value.as_bool()),
        Some(false),
        "validate must surface the real type-aware semantic error for the resolved object-module binding: {validate_with_path_json}"
    );
    let validate_messages = validate_with_path_json
        .get("errors")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.get("message").and_then(|message| message.as_str()))
        .collect::<Vec<_>>();
    assert!(
        validate_messages.iter().all(|message| {
            !message.contains("Необъявленная переменная") || !message.contains("Объект")
        }),
        "validate with filePath must stop failing at unresolved object-module binding: {validate_with_path_json}"
    );
    assert!(
        validate_messages.iter().any(|message| {
            message.contains("Ссылка") && message.contains("ДокументОбъект.Док1")
        }),
        "validate with filePath must use resolved object-module type in semantic error: {validate_with_path_json}"
    );
}
