//! Web API handlers
//!
//! HTTP handlers для REST API endpoints

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::application::TypeInferenceService;
use crate::application::get_hover_info_with_semantic_program;
use crate::application::type_system::web_api_service;
use crate::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter, HoverOutputFormat};
use crate::system::{DepsBundleV2, EffectiveStartupInputs, StartupInputs, SystemCoordinator, startup_v2};
use bsl_shared::api::{
    AstNodeDto, DebugAstResponseDto, DiagnosticsResponseDto, EnhancedHoverResponse,
    SemanticErrorDto, SnapshotInputsDto, SnapshotMetaDto, StartupProgressDto, SyntaxErrorDto,
};
use bsl_shared::api::ValidationErrorDto;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{DiagnosticSeverity, TypeDiagnostic};
use bsl_shared::formatting::DetailLevel;

// --- СТАРЫЕ DTO УДАЛЕНЫ ---

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub page: Option<usize>, // 1-based page number
    pub limit: Option<usize>,
    pub category: Option<String>,
    pub certainty_level: Option<String>,
    pub flow_sensitive_only: Option<bool>,
}

#[derive(Deserialize)]
pub struct HoverRequest {
    pub code: String,
    pub line: u32,
    pub column: u32,
    #[serde(default = "default_detail_level")]
    pub detail_level: String,
}

fn default_detail_level() -> String {
    "detailed".to_string()
}

#[derive(Clone)]
pub struct AppState {
    pub deps_bundle_v2: Arc<RwLock<Arc<DepsBundleV2>>>,
    pub system_coordinator: Arc<SystemCoordinator>,
    pub syntax_helper_path: Option<PathBuf>,
    pub startup_inputs: Arc<RwLock<EffectiveStartupInputs>>,
}

fn compute_settings_id_v2(diagnostics_detail_level: DetailLevel) -> SettingsId {
    let payload = format!(
        "schema={};diagnostics.detail_level={:?}",
        bsl_analysis_v2::SETTINGS_SCHEMA_VERSION,
        diagnostics_detail_level
    );
    SettingsId::from_hash(blake3::hash(payload.as_bytes()).to_hex().to_string())
}

fn validation_error_type(message: &str) -> &'static str {
    if message.contains("не существует") {
        "NonExistentMethod"
    } else if message.contains("параметр") {
        "ParameterError"
    } else {
        "SemanticError"
    }
}

fn type_diagnostics_to_validation_errors(diagnostics: &[TypeDiagnostic]) -> Vec<ValidationErrorDto> {
    diagnostics
        .iter()
        .map(|d| ValidationErrorDto {
            message: d.message.clone(),
            severity: match d.severity {
                DiagnosticSeverity::Error => "error".to_string(),
                DiagnosticSeverity::Warning => "warning".to_string(),
                DiagnosticSeverity::Info | DiagnosticSeverity::Hint => "info".to_string(),
            },
            line: d.line,
            column: d.column,
            end_line: d.end_line,
            end_column: d.end_column,
            error_type: validation_error_type(&d.message).to_string(),
        })
        .collect()
}

fn deps_resolver(deps: &Arc<bsl_analysis_v2::SemanticDeps>) -> Arc<TypeResolver> {
    deps.resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())))
}

fn build_inference_v2(
    deps: &Arc<bsl_analysis_v2::SemanticDeps>,
) -> (TypeInferenceService, TypeMetadataLookup) {
    let resolver = deps_resolver(deps);
    let inference_service = TypeInferenceService::new(resolver, deps.repository.clone());
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
    (inference_service, metadata_lookup)
}

/// Get system metrics
pub async fn get_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let (inference_service, _metadata_lookup) = build_inference_v2(&deps_bundle.semantic_deps);
    let types = web_api_service::get_metrics_summary(&inference_service);
    let observability = state.system_coordinator.observability_metrics();
    Json(json!({
        "types": types,
        "observability": observability
    }))
}

/// Get all types with pagination support
pub async fn get_types(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> impl IntoResponse {
    // Валидация и значения по умолчанию
    let page = params.page.unwrap_or(1).max(1); // Минимум страница 1
    let limit = params.limit.unwrap_or(50).clamp(1, 1000); // 1-1000 элементов

    // Конвертация page → offset для внутреннего использования
    let offset = (page - 1) * limit;

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let (inference_service, metadata_lookup) = build_inference_v2(&deps_bundle.semantic_deps);
    let result = web_api_service::get_all_types_as_dto(
        &inference_service,
        &metadata_lookup,
        limit,
        offset,
        params.category,
        params.certainty_level,
        params.flow_sensitive_only.unwrap_or(false),
    );
    Json(result)
}

/// Search types by query
pub async fn search_types(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let (inference_service, metadata_lookup) = build_inference_v2(&deps_bundle.semantic_deps);
    match web_api_service::search_types_as_dto(&inference_service, &metadata_lookup, &query.q).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "bsl-gradual-types",
        "version": env!("CARGO_PKG_VERSION"),
        "build": env!("BUILD_TIMESTAMP"),
        "git": env!("GIT_HASH")
    }))
}

/// Version information endpoint
pub async fn get_version() -> impl IntoResponse {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "build_timestamp": env!("BUILD_TIMESTAMP"),
        "git_hash": env!("GIT_HASH"),
        "rust_version": env!("CARGO_PKG_RUST_VERSION", "unknown"),
        "name": "BSL Gradual Types"
    }))
}

pub async fn get_snapshot_meta(State(state): State<AppState>) -> impl IntoResponse {
    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let inputs = state.startup_inputs.read().await.clone();
    Json(snapshot_meta_dto(deps_bundle.as_ref(), &inputs))
}

pub async fn reload_snapshot(State(state): State<AppState>) -> impl IntoResponse {
    let current_inputs = state.startup_inputs.read().await.clone();
    let coordinator = state.system_coordinator.clone();

    let inputs = StartupInputs::from_web_flags(
        current_inputs.syntax_helper_path.clone(),
        current_inputs.configuration_path.clone(),
        current_inputs.platform_version.clone(),
        Some(current_inputs.cache_enabled),
        Some(current_inputs.strict_fingerprint),
    );

    let result = startup_v2(coordinator, inputs, None).await;
    match result {
        Ok(startup) => {
            let deps_bundle = Arc::new(startup.deps_bundle_v2);

            {
                let mut guard = state.deps_bundle_v2.write().await;
                *guard = deps_bundle.clone();
            }
            {
                let mut guard = state.startup_inputs.write().await;
                *guard = startup.inputs.clone();
            }

            Json(snapshot_meta_dto(deps_bundle.as_ref(), &startup.inputs)).into_response()
        }
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

fn snapshot_meta_dto(deps_bundle: &DepsBundleV2, inputs: &EffectiveStartupInputs) -> SnapshotMetaDto {
    SnapshotMetaDto {
        deps_id: deps_bundle.deps_id.as_str().to_string(),
        index_snapshot_id: deps_bundle.meta.index_snapshot_id.clone(),
        platform_version: deps_bundle.meta.platform_version.clone(),
        platform_fingerprint: deps_bundle.meta.platform_fingerprint.clone(),
        config_fingerprint: deps_bundle.meta.config_fingerprint.clone(),
        strict_fingerprint: deps_bundle.meta.strict_fingerprint,
        repository_stats: deps_bundle.semantic_deps.repository.get_stats(),
        inputs: SnapshotInputsDto {
            syntax_helper_path: inputs
                .syntax_helper_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            configuration_path: inputs
                .configuration_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            platform_version: inputs.platform_version.clone(),
            cache_enabled: inputs.cache_enabled,
            strict_fingerprint: inputs.strict_fingerprint,
        },
    }
}

/// Startup progress endpoint (polling).
///
/// Возвращает прогресс инициализации системы, включая загрузку конфигурации и индексацию модулей.
pub async fn get_startup_progress(State(state): State<AppState>) -> impl IntoResponse {
    let snapshot: StartupProgressDto = state.system_coordinator.startup_progress();
    Json(snapshot)
}

/// Validate code fragment
/// Phase 4: TypeValidator integration - проверяет методы и свойства
pub async fn validate_code(
    State(state): State<AppState>,
    Json(payload): Json<bsl_shared::api::ValidateCodeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let code = payload.code.clone();
    let validation_result = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<ValidationErrorDto>> {
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: deps_bundle.deps_id.clone(),
            deps: deps_bundle.semantic_deps.clone(),
        });
        let diagnostics_detail_level = DetailLevel::Full;
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: compute_settings_id_v2(diagnostics_detail_level),
            diagnostics_detail_level,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(code),
            version: 0,
            path: Arc::from("<semantic_validation>"),
        });

        let analysis = host.analysis();
        let diagnostics = analysis
            .semantic_diagnostics(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("semantic diagnostics cancelled"))?
            .unwrap_or_else(|| Arc::new(Vec::new()));

        Ok(type_diagnostics_to_validation_errors(diagnostics.as_ref()))
    })
    .await;

    match validation_result {
        Ok(Ok(errors)) => {
            let is_valid = errors.is_empty();
            let duration_ms = start.elapsed().as_millis() as u64;

            let response = bsl_shared::api::ValidateCodeResponse {
                is_valid,
                errors,
                metadata: Some(bsl_shared::api::ValidationMetadataDto {
                    expressions_analyzed: 1,
                    types_resolved: if is_valid { 1 } else { 0 },
                    duration_ms,
                }),
            };

            Json(response).into_response()
        }
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get hover information for code position
/// Phase 5: LSP Hover integration - returns type information at position
pub async fn get_hover(
    State(state): State<AppState>,
    Json(req): Json<HoverRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let code = req.code.clone();
    let line = req.line;
    let column = req.column;
    let syntax_helper_path = state.syntax_helper_path.clone();

    let hover_result = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<String>> {
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: deps_bundle.deps_id.clone(),
            deps: deps_bundle.semantic_deps.clone(),
        });
        let diagnostics_detail_level = DetailLevel::Full;
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: compute_settings_id_v2(diagnostics_detail_level),
            diagnostics_detail_level,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(code),
            version: 0,
            path: Arc::from("hover_request.bsl"),
        });

        let analysis = host.analysis();
        let file_content = analysis
            .file_text(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("file_text cancelled"))?
            .ok_or_else(|| anyhow::anyhow!("file_text unavailable"))?;
        let ir_program = analysis
            .ir(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("ir query cancelled"))?
            .ok_or_else(|| anyhow::anyhow!("ir unavailable"))?;

        let deps = deps_bundle.semantic_deps.clone();
        let resolver = deps_resolver(&deps);
        let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
        let hover_formatter = HoverFormatter::new(
            HoverFormatConfig {
                syntax_helper_path,
                output_format: HoverOutputFormat::Markdown,
                ..Default::default()
            },
            metadata_lookup.clone(),
        );

        Ok(get_hover_info_with_semantic_program(
            file_content.as_ref(),
            line,
            column,
            &metadata_lookup,
            &hover_formatter,
            None,
            resolver.as_ref(),
            ir_program,
        ))
    })
    .await;

    match hover_result {
        Ok(Ok(hover_text)) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            let response = serde_json::json!({
                "hover": hover_text,
                "line": req.line,
                "column": req.column,
                "duration_ms": duration_ms
            });

            Json(response).into_response()
        }
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Enhanced diagnostics endpoint - separates syntax and semantic errors
/// Milestone 2.18: Comprehensive diagnostics
/// UPDATED Phase 5: Now uses validate_semantics for full semantic validation
pub async fn get_diagnostics(
    State(state): State<AppState>,
    Json(payload): Json<bsl_shared::api::ValidateCodeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let code = payload.code.clone();

    let diagnostics_result = tokio::task::spawn_blocking(move || -> anyhow::Result<(Vec<SyntaxErrorDto>, Vec<SemanticErrorDto>)> {
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: deps_bundle.deps_id.clone(),
            deps: deps_bundle.semantic_deps.clone(),
        });
        let diagnostics_detail_level = DetailLevel::Full;
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: compute_settings_id_v2(diagnostics_detail_level),
            diagnostics_detail_level,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(code),
            version: 0,
            path: Arc::from("<semantic_validation>"),
        });

        let analysis = host.analysis();
        let syntax = analysis
            .syntax_diagnostics(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("syntax diagnostics cancelled"))?
            .unwrap_or_else(|| Arc::new(Vec::new()));

        let syntax_errors: Vec<SyntaxErrorDto> = syntax
            .iter()
            .map(|e| SyntaxErrorDto {
                message: e.message.clone(),
                line: e.span.start_line,
                column: e.span.start_column,
            })
            .collect();

        if !syntax_errors.is_empty() {
            return Ok((syntax_errors, Vec::new()));
        }

        let diagnostics = analysis
            .semantic_diagnostics(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("semantic diagnostics cancelled"))?
            .unwrap_or_else(|| Arc::new(Vec::new()));

        let semantic_errors: Vec<SemanticErrorDto> = diagnostics
            .iter()
            .map(|d| SemanticErrorDto {
                message: d.message.clone(),
                line: d.line,
                column: d.column,
                end_line: d.end_line,
                end_column: d.end_column,
                severity: format!("{:?}", d.severity).to_lowercase(),
            })
            .collect();

        Ok((syntax_errors, semantic_errors))
    })
    .await;

    match diagnostics_result {
        Ok(Ok((syntax_errors, semantic_errors))) => {
            let duration_ms = start.elapsed().as_millis();
            let response = DiagnosticsResponseDto {
                total_errors: syntax_errors.len() + semantic_errors.len(),
                syntax_errors,
                semantic_errors,
                duration_ms,
            };
            Json(response).into_response()
        }
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Debug diagnostics endpoint - returns extended debug info
pub async fn get_diagnostics_debug(
    State(state): State<AppState>,
    Json(payload): Json<bsl_shared::api::ValidateCodeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let code = payload.code.clone();

    let diagnostics_result = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let mut debug_info = serde_json::json!({
            "steps": [],
            "resolver_available": false,
            "property_accesses": []
        });

        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: deps_bundle.deps_id.clone(),
            deps: deps_bundle.semantic_deps.clone(),
        });
        let diagnostics_detail_level = DetailLevel::Full;
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: compute_settings_id_v2(diagnostics_detail_level),
            diagnostics_detail_level,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(code),
            version: 0,
            path: Arc::from("<debug_validation>"),
        });

        let analysis = host.analysis();
        let syntax = analysis
            .syntax_diagnostics(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("syntax diagnostics cancelled"))?
            .unwrap_or_else(|| Arc::new(Vec::new()));

        let steps = debug_info["steps"]
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("debug_info.steps is not array"))?;
        steps.push(serde_json::json!({
            "step": "parse",
            "success": true,
            "syntax_errors": syntax.len()
        }));

        let syntax_errors: Vec<SyntaxErrorDto> = syntax
            .iter()
            .map(|e| SyntaxErrorDto {
                message: e.message.clone(),
                line: e.span.start_line,
                column: e.span.start_column,
            })
            .collect();

        if !syntax_errors.is_empty() {
            let duration_ms = start.elapsed().as_millis();
            return Ok(serde_json::json!({
                "syntaxErrors": syntax_errors,
                "semanticErrors": [],
                "totalErrors": syntax_errors.len(),
                "durationMs": duration_ms,
                "debug": debug_info
            }));
        }

        debug_info["resolver_available"] = serde_json::json!(true);

        let ir = analysis
            .ir(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("ir query cancelled"))?
            .ok_or_else(|| anyhow::anyhow!("ir unavailable"))?;

        let steps = debug_info["steps"]
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("debug_info.steps is not array"))?;
        steps.push(serde_json::json!({
            "step": "ast_to_ir",
            "success": true,
            "ir_nodes": ir.nodes.len()
        }));

        debug_info["ir_info"] = serde_json::json!({
            "nodes_count": ir.nodes.len(),
            "has_cfg": ir.cfg.is_some()
        });

        let diagnostics = analysis
            .semantic_diagnostics(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("semantic diagnostics cancelled"))?
            .unwrap_or_else(|| Arc::new(Vec::new()));

        let errors = diagnostics.as_ref();

        let steps = debug_info["steps"]
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("debug_info.steps is not array"))?;
        steps.push(serde_json::json!({
            "step": "semantic_validation",
            "success": true,
            "errors_found": errors.len()
        }));

        let semantic_errors: Vec<SemanticErrorDto> = errors
            .iter()
            .map(|d| SemanticErrorDto {
                message: d.message.clone(),
                line: d.line,
                column: d.column,
                end_line: d.end_line,
                end_column: d.end_column,
                severity: format!("{:?}", d.severity).to_lowercase(),
            })
            .collect();

        let duration_ms = start.elapsed().as_millis();
        Ok(serde_json::json!({
            "syntaxErrors": [],
            "semanticErrors": semantic_errors,
            "totalErrors": semantic_errors.len(),
            "durationMs": duration_ms,
            "debug": debug_info
        }))
    })
    .await;

    match diagnostics_result {
        Ok(Ok(json)) => Json(json).into_response(),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Debug AST endpoint - shows parsed structure for debugging
/// Milestone 2.16: Semantic visualization
pub async fn get_debug_ast(
    State(_state): State<AppState>,
    Json(_payload): Json<bsl_shared::api::ValidateCodeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();
    let duration_ms = start.elapsed().as_millis();

    // Stub implementation - returns minimal AST for testing
    let response = DebugAstResponseDto {
        nodes: vec![AstNodeDto {
            kind: "Program".to_string(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
            text: None,
        }],
        symbol_table: vec![],
        parse_errors: 0,
        duration_ms,
    };

    Json(response).into_response()
}

/// Enhanced hover endpoint - provides detailed variable information
/// Milestone 2.13: Enhanced hover with variable details
pub async fn get_enhanced_hover(
    State(state): State<AppState>,
    Json(req): Json<HoverRequest>,
) -> impl IntoResponse {
    use bsl_shared::formatting::DetailLevel;

    let start = Instant::now();

    // Parse detail_level from request
    let detail_level = DetailLevel::parse(&req.detail_level);

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let code = req.code.clone();
    let line = req.line;
    let column = req.column;
    let syntax_helper_path = state.syntax_helper_path.clone();

    let hover_result = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<String>> {
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: deps_bundle.deps_id.clone(),
            deps: deps_bundle.semantic_deps.clone(),
        });
        let diagnostics_detail_level = DetailLevel::Full;
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: compute_settings_id_v2(diagnostics_detail_level),
            diagnostics_detail_level,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(code),
            version: 0,
            path: Arc::from("hover_request.bsl"),
        });

        let analysis = host.analysis();
        let file_content = analysis
            .file_text(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("file_text cancelled"))?
            .ok_or_else(|| anyhow::anyhow!("file_text unavailable"))?;
        let ir_program = analysis
            .ir(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("ir query cancelled"))?
            .ok_or_else(|| anyhow::anyhow!("ir unavailable"))?;

        let deps = deps_bundle.semantic_deps.clone();
        let resolver = deps_resolver(&deps);
        let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
        let hover_formatter = HoverFormatter::new(
            HoverFormatConfig {
                syntax_helper_path: syntax_helper_path.clone(),
                output_format: HoverOutputFormat::Markdown,
                ..Default::default()
            },
            metadata_lookup.clone(),
        );

        let hover_config = HoverFormatConfig {
            detail_level,
            syntax_helper_path,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        Ok(get_hover_info_with_semantic_program(
            file_content.as_ref(),
            line,
            column,
            &metadata_lookup,
            &hover_formatter,
            Some(hover_config),
            resolver.as_ref(),
            ir_program,
        ))
    })
    .await;

    match hover_result {
        Ok(Ok(hover_text)) => {
            let duration_ms = start.elapsed().as_millis();

            let hover_text_str = hover_text.unwrap_or_else(|| "No information available".to_string());

            let response = EnhancedHoverResponse {
                hover_text: hover_text_str,
                variable_name: None,
                variable_type: None,
                type_hint: None,
                found_in_scope: false,
                line: req.line,
                column: req.column,
                duration_ms,
            };

            Json(response).into_response()
        }
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Request для получения семантического дерева
#[derive(Deserialize)]
pub struct SemanticTreeRequest {
    pub code: String,
    #[serde(default = "default_file_path")]
    pub file_path: String,
    /// Compact режим: исключает symbol_table и call_graph для уменьшения размера ответа
    #[serde(default)]
    pub compact: bool,
    /// Включить граф вызовов (по умолчанию: true)
    #[serde(default = "default_true")]
    pub include_call_graph: bool,
    /// Включить flow-sensitive информацию (по умолчанию: true)
    #[serde(default = "default_true")]
    pub include_flow_sensitive: bool,
}

fn default_file_path() -> String {
    "inline.bsl".to_string()
}

fn default_true() -> bool {
    true
}

/// Get semantic tree for code - показывает семантическое представление кода
/// Milestone 5.3: Web API endpoint для семантического дерева
pub async fn get_semantic_tree(
    State(state): State<AppState>,
    Json(req): Json<SemanticTreeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let code = req.code.clone();
    let file_path = req.file_path.clone();
    let compact = req.compact;
    let include_call_graph = req.include_call_graph;
    let include_flow_sensitive = req.include_flow_sensitive;

    let tree_result = tokio::task::spawn_blocking(move || -> anyhow::Result<bsl_shared::api::semantic_dtos::SemanticTreeDto> {
        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: deps_bundle.deps_id.clone(),
            deps: deps_bundle.semantic_deps.clone(),
        });
        let diagnostics_detail_level = DetailLevel::Full;
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: compute_settings_id_v2(diagnostics_detail_level),
            diagnostics_detail_level,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(code),
            version: 0,
            path: Arc::from(file_path.clone()),
        });

        let analysis = host.analysis();
        let ir_program = analysis
            .ir(V2FileId(1))
            .map_err(|_| anyhow::anyhow!("ir query cancelled"))?
            .ok_or_else(|| anyhow::anyhow!("ir unavailable"))?;

        let dto = if compact {
            ir_program.to_compact_dto()
        } else {
            ir_program.to_dto(include_call_graph, include_flow_sensitive)
        };

        Ok(dto)
    })
    .await;

    match tree_result {
        Ok(Ok(tree)) => {
            let duration_ms = start.elapsed().as_millis();

            let response = serde_json::json!({
                "file_path": tree.file_path,
                "root_nodes": tree.root_nodes,
                "symbol_table": tree.symbol_table,
                "metrics": {
                    "node_count": tree.metrics.node_count,
                    "procedure_count": tree.metrics.procedure_count,
                    "function_count": tree.metrics.function_count,
                    "variable_count": tree.metrics.variable_count,
                    "known_types": tree.metrics.known_types,
                    "inferred_types": tree.metrics.inferred_types,
                    "unknown_types": tree.metrics.unknown_types,
                    "analysis_time_ms": tree.metrics.analysis_time_ms,
                    "request_duration_ms": duration_ms,
                }
            });

            Json(response).into_response()
        }
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
