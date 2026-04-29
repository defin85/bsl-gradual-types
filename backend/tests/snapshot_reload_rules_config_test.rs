use axum::http::Request;
use bsl_backend::presentation::web::{create_router, AppState};
use bsl_backend::system::{build_deps_bundle_v2, EffectiveStartupInputs, SystemCoordinator};
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;

#[tokio::test]
async fn snapshot_reload_reuses_effective_rules_config_path() {
    let temp = TempDir::new().expect("tempdir");
    let rules_path = temp.path().join("bsl-rules.toml");
    std::fs::write(
        &rules_path,
        "[semantic.common_module_factories]\nbuiltin_bsp = false\n",
    )
    .expect("write rules config");

    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator
        .start_with_paths_blocking(None, None, None, None)
        .expect("startup");
    let initial_deps =
        build_deps_bundle_v2(coordinator.as_ref(), None, None).expect("deps bundle v2");
    let initial_deps_id = initial_deps.deps_id.as_str().to_string();

    let state = AppState {
        deps_bundle_v2: Arc::new(tokio::sync::RwLock::new(Arc::new(initial_deps))),
        system_coordinator: coordinator,
        syntax_helper_path: None,
        startup_inputs: Arc::new(tokio::sync::RwLock::new(EffectiveStartupInputs {
            syntax_helper_path: None,
            configuration_path: None,
            platform_version: None,
            rules_config_path: Some(rules_path.clone()),
            cache_enabled: true,
            strict_fingerprint: false,
        })),
    };

    let app = create_router(state, "backend/static", true);
    let resp = app
        .oneshot(
            Request::post("/api/snapshot/reload")
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

    assert_ne!(
        json.get("depsId").and_then(|value| value.as_str()),
        Some(initial_deps_id.as_str()),
        "disabled builtin BSP rule must participate in web reload deps identity"
    );
    assert_eq!(
        json.pointer("/inputs/rulesConfigPath")
            .and_then(|value| value.as_str()),
        Some(rules_path.to_string_lossy().as_ref())
    );
}
