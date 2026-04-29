//! Web API handlers
//!
//! HTTP handlers для REST API endpoints

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use bsl_analysis_v2::{FileId as V2FileId, SettingsId};
use bsl_line_index::LineIndex;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::application::get_hover_info_with_semantic_program;
use crate::application::type_system::web_api_service;
use crate::application::{
    CancellationPolicy, ExecutionContext, ExecutionSettings, IntellisenseV2Facade,
    ObservabilityOrigin, ObservabilityStage, PreparedOperationSnapshot, SemanticOperation,
};
use crate::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter, HoverOutputFormat};
use crate::system::{
    startup_v2, DepsBundleV2, EffectiveStartupInputs, StartupInputs, SystemCoordinator,
};
use bsl_shared::api::ValidationErrorDto;
use bsl_shared::api::{
    AstNodeDto, DebugAstResponseDto, DiagnosticsResponseDto, EnhancedHoverResponse,
    GlobalContextDocsStatusDto, McpBackendModeDto, McpStatusDto, SemanticErrorDto,
    SnapshotInputsDto, SnapshotMetaDto, StartupProgressDto, SyntaxErrorDto,
};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{DiagnosticSeverity, TypeDiagnostic};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::{normalize_user_facing_type_name, DetailLevel};

#[path = "handlers/debug.rs"]
mod debug;
#[path = "handlers/semantic.rs"]
mod semantic;

pub use debug::{get_debug_ast, get_diagnostics_debug};
pub use semantic::{get_enhanced_hover, get_semantic_tree};

// --- СТАРЫЕ DTO УДАЛЕНЫ ---

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub page: Option<usize>, // 1-based page number
    pub limit: Option<usize>,
    #[serde(default)]
    pub category: Vec<String>,
    #[serde(default)]
    pub certainty_level: Vec<String>,
    pub flow_sensitive_only: Option<bool>,
}

#[derive(Deserialize)]
pub struct HoverRequest {
    pub code: String,
    pub line: u32,
    pub column: u32,
    #[serde(default, rename = "filePath")]
    pub file_path: Option<String>,
    #[serde(default = "default_detail_level")]
    pub detail_level: String,
    /// Enable flow-sensitive analysis (opt-in). Default: false.
    #[serde(default, rename = "includeFlowSensitive")]
    pub include_flow_sensitive: bool,
    /// Legacy field: `include_flow_sensitive` is rejected.
    #[serde(default, rename = "include_flow_sensitive")]
    pub legacy_include_flow_sensitive: Option<bool>,
}

fn default_detail_level() -> String {
    "detailed".to_string()
}

pub(super) fn inline_web_path(file_path: Option<&str>, fallback: &'static str) -> Arc<str> {
    file_path
        .filter(|value| !value.trim().is_empty())
        .map(Arc::<str>::from)
        .unwrap_or_else(|| Arc::from(fallback))
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

fn prepare_ephemeral_web_operation(
    deps_bundle: &DepsBundleV2,
    coordinator: &SystemCoordinator,
    operation: SemanticOperation,
    diagnostics_detail_level: DetailLevel,
    include_flow_sensitive: bool,
    code: Arc<str>,
    path: Arc<str>,
) -> anyhow::Result<(ExecutionContext, PreparedOperationSnapshot)> {
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Web,
        operation,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: V2FileId(1),
        min_file_version: Some(0),
        expected_deps_id: Some(deps_bundle.deps_id.clone()),
        flow_sensitive: include_flow_sensitive,
        settings: ExecutionSettings {
            settings_id: compute_settings_id_v2(diagnostics_detail_level),
            diagnostics_detail_level,
        },
        cancellation: CancellationPolicy::BestEffort,
    };
    let prepared = IntellisenseV2Facade::prepare_ephemeral_operation(
        &context,
        deps_bundle.deps_id.clone(),
        deps_bundle.semantic_deps.clone(),
        deps_bundle.index_snapshot.clone(),
        code,
        0,
        path,
        Some(coordinator),
    )
    .map_err(|outcome| anyhow::anyhow!("ephemeral operation failed: {}", outcome.as_str()))?;
    Ok((context, prepared))
}

fn record_type_index_reason_at_utf16_position(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: V2FileId,
    line: u32,
    column: u32,
    coordinator: &SystemCoordinator,
) {
    let Some(byte_offset) = analysis
        .utf16_position_to_byte_offset(file_id, line, column)
        .ok()
        .flatten()
    else {
        return;
    };
    let byte_offset = byte_offset.min(u32::MAX as usize) as u32;
    let Ok(profiled) = analysis.type_at_byte_offset_serve_only_profiled(file_id, byte_offset)
    else {
        return;
    };
    coordinator.record_intellisense_v2_type_index_reason(profiled.serve_reason_code.as_str());
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

fn type_diagnostics_to_validation_errors(
    diagnostics: &[TypeDiagnostic],
    source: &str,
    line_index: &LineIndex,
) -> Vec<ValidationErrorDto> {
    diagnostics
        .iter()
        .map(|d| {
            let (line, column) =
                line_index.byte_offset_to_utf16_position(source, d.span.start as usize);
            let (end_line, end_column) =
                line_index.byte_offset_to_utf16_position(source, d.span.end as usize);
            ValidationErrorDto {
                message: d.message.clone(),
                severity: match d.severity {
                    DiagnosticSeverity::Error => "error".to_string(),
                    DiagnosticSeverity::Warning => "warning".to_string(),
                    DiagnosticSeverity::Info | DiagnosticSeverity::Hint => "info".to_string(),
                },
                line,
                column,
                end_line,
                end_column,
                error_type: validation_error_type(&d.message).to_string(),
            }
        })
        .collect()
}

fn deps_resolver(deps: &Arc<bsl_analysis_v2::SemanticDeps>) -> Arc<TypeResolver> {
    deps.resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())))
}

fn build_metadata_lookup_v2(deps: &Arc<bsl_analysis_v2::SemanticDeps>) -> TypeMetadataLookup {
    TypeMetadataLookup::new(deps.repository.clone())
}

fn web_semantic_artifacts_unavailable(deps_bundle: &DepsBundleV2) -> bool {
    deps_bundle.semantic_deps.repository.get_stats().total_types == 0
}

pub(super) enum WebHoverQueryOutcome {
    Ready(Option<String>),
    FailClosed(&'static str),
}

pub(super) fn record_web_interactive_fail_closed_reason(
    coordinator: &SystemCoordinator,
    operation: &str,
    reason: &'static str,
) {
    coordinator.record_intellisense_v2_interactive_fail_closed_reason("web", operation, reason);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_web_hover_query(
    deps_bundle: &DepsBundleV2,
    coordinator: &SystemCoordinator,
    code: Arc<str>,
    file_path: Arc<str>,
    line: u32,
    column: u32,
    syntax_helper_path: Option<PathBuf>,
    include_flow_sensitive: bool,
    hover_config: Option<HoverFormatConfig>,
) -> anyhow::Result<WebHoverQueryOutcome> {
    if web_semantic_artifacts_unavailable(deps_bundle) {
        return Ok(WebHoverQueryOutcome::FailClosed("unavailable_by_contract"));
    }

    let (context, prepared) = match prepare_ephemeral_web_operation(
        deps_bundle,
        coordinator,
        SemanticOperation::Hover,
        DetailLevel::Full,
        include_flow_sensitive,
        code,
        file_path,
    ) {
        Ok(values) => values,
        Err(_) => return Ok(WebHoverQueryOutcome::FailClosed("missing_canonical_ir")),
    };
    let analysis = prepared.snapshot.analysis;
    record_type_index_reason_at_utf16_position(&analysis, V2FileId(1), line, column, coordinator);
    let file_content = match analysis.file_text(V2FileId(1)) {
        Ok(Some(file_content)) => file_content,
        Ok(None) | Err(_) => {
            return Ok(WebHoverQueryOutcome::FailClosed("unavailable_by_contract"))
        }
    };
    let ir_program = match IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(coordinator),
        |analysis| analysis.ir(V2FileId(1)),
    ) {
        Ok(Some(ir_program)) => ir_program,
        Ok(None) | Err(_) => return Ok(WebHoverQueryOutcome::FailClosed("missing_canonical_ir")),
    };

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
    let exact_type_index_available =
        bsl_runtime::application::type_system::hover_exact_type_index_available_at_position(
            &analysis,
            V2FileId(1),
            file_content.as_ref(),
            line,
            column,
            ir_program.as_ref(),
        );
    let hover = get_hover_info_with_semantic_program(
        &analysis,
        V2FileId(1),
        file_content.as_ref(),
        line,
        column,
        include_flow_sensitive,
        &metadata_lookup,
        &hover_formatter,
        hover_config,
        resolver.as_ref(),
        ir_program,
    );
    if hover.is_none() && !exact_type_index_available {
        return Ok(WebHoverQueryOutcome::FailClosed("missing_semantic_index"));
    }

    Ok(WebHoverQueryOutcome::Ready(hover))
}

/// Get system metrics
pub async fn get_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let types = web_api_service::get_metrics_summary(deps_bundle.semantic_deps.as_ref());
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
    let metadata_lookup = build_metadata_lookup_v2(&deps_bundle.semantic_deps);
    let result = web_api_service::get_all_types_as_dto(
        deps_bundle.semantic_deps.as_ref(),
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
    let metadata_lookup = build_metadata_lookup_v2(&deps_bundle.semantic_deps);
    match web_api_service::search_types_as_dto(
        deps_bundle.semantic_deps.as_ref(),
        &metadata_lookup,
        &query.q,
    )
    .await
    {
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

/// Capability detection endpoint for unified SPA.
///
/// In `bsl-web-server` mode, MCP dashboard endpoints are not supported.
pub async fn get_mcp_status() -> impl IntoResponse {
    Json(McpStatusDto {
        mode: McpBackendModeDto::WebServer,
        supported: false,
        read_only: false,
        instance_id: None,
        ui_url: None,
        cache_dir: None,
    })
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
        current_inputs.rules_config_path.clone(),
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

fn snapshot_meta_dto(
    deps_bundle: &DepsBundleV2,
    inputs: &EffectiveStartupInputs,
) -> SnapshotMetaDto {
    SnapshotMetaDto {
        deps_id: deps_bundle.deps_id.as_str().to_string(),
        index_snapshot_id: deps_bundle.meta.index_snapshot_id.clone(),
        platform_version: deps_bundle.meta.platform_version.clone(),
        platform_fingerprint: deps_bundle.meta.platform_fingerprint.clone(),
        config_fingerprint: deps_bundle.meta.config_fingerprint.clone(),
        strict_fingerprint: deps_bundle.meta.strict_fingerprint,
        global_context: GlobalContextDocsStatusDto {
            state: deps_bundle.meta.global_context_state.clone(),
            property_count: deps_bundle.meta.global_context_property_count,
            fingerprint: deps_bundle.meta.global_context_fingerprint.clone(),
            degraded_reason: deps_bundle.meta.global_context_degraded_reason.clone(),
        },
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
            rules_config_path: inputs
                .rules_config_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
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

    if payload.legacy_include_flow_sensitive.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Unsupported field: include_flow_sensitive. Use includeFlowSensitive (camelCase)."
            })),
        )
            .into_response();
    }

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let coordinator = state.system_coordinator.clone();
    let code = payload.code.clone();
    let file_path = inline_web_path(payload.file_path.as_deref(), "<semantic_validation>");
    let include_flow_sensitive = payload.include_flow_sensitive;
    let validation_result = crate::application::spawn_bounded_blocking(
        move || -> anyhow::Result<Vec<ValidationErrorDto>> {
            let code_arc: Arc<str> = Arc::from(code);
            let line_index = LineIndex::new(code_arc.as_ref());
            let (context, prepared) = prepare_ephemeral_web_operation(
                deps_bundle.as_ref(),
                coordinator.as_ref(),
                SemanticOperation::Diagnostics,
                DetailLevel::Full,
                include_flow_sensitive,
                code_arc.clone(),
                file_path,
            )?;
            let analysis = prepared.snapshot.analysis;
            let diagnostics = if include_flow_sensitive {
                IntellisenseV2Facade::run_optional_query(
                    &context,
                    ObservabilityStage::SemanticDiagnosticsQuery,
                    &analysis,
                    Some(coordinator.as_ref()),
                    |analysis| analysis.semantic_diagnostics_flow_sensitive(V2FileId(1)),
                )
                .map_err(|_| anyhow::anyhow!("semantic diagnostics cancelled"))?
                .unwrap_or_else(|| Arc::new(Vec::new()))
            } else {
                IntellisenseV2Facade::run_optional_query(
                    &context,
                    ObservabilityStage::SemanticDiagnosticsQuery,
                    &analysis,
                    Some(coordinator.as_ref()),
                    |analysis| analysis.semantic_diagnostics(V2FileId(1)),
                )
                .map_err(|_| anyhow::anyhow!("semantic diagnostics cancelled"))?
                .unwrap_or_else(|| Arc::new(Vec::new()))
            };

            Ok(type_diagnostics_to_validation_errors(
                diagnostics.as_ref(),
                code_arc.as_ref(),
                &line_index,
            ))
        },
    )
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

    if req.legacy_include_flow_sensitive.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Unsupported field: include_flow_sensitive. Use includeFlowSensitive (camelCase)."
            })),
        )
            .into_response();
    }

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let coordinator = state.system_coordinator.clone();
    let code = req.code.clone();
    let file_path = inline_web_path(req.file_path.as_deref(), "hover_request.bsl");
    let line = req.line;
    let column = req.column;
    let syntax_helper_path = state.syntax_helper_path.clone();
    let include_flow_sensitive = req.include_flow_sensitive;
    let worker_coordinator = coordinator.clone();

    let hover_result = crate::application::spawn_bounded_blocking(
        move || -> anyhow::Result<WebHoverQueryOutcome> {
            resolve_web_hover_query(
                deps_bundle.as_ref(),
                worker_coordinator.as_ref(),
                Arc::from(code),
                file_path,
                line,
                column,
                syntax_helper_path,
                include_flow_sensitive,
                None,
            )
        },
    )
    .await;

    match hover_result {
        Ok(Ok(WebHoverQueryOutcome::Ready(hover_text))) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let hover_text = hover_text.map(|value| normalize_user_facing_type_name(&value));

            let response = serde_json::json!({
                "hover": hover_text,
                "line": req.line,
                "column": req.column,
                "duration_ms": duration_ms
            });

            Json(response).into_response()
        }
        Ok(Ok(WebHoverQueryOutcome::FailClosed(reason))) => {
            record_web_interactive_fail_closed_reason(coordinator.as_ref(), "hover", reason);
            let duration_ms = start.elapsed().as_millis() as u64;
            Json(serde_json::json!({
                "hover": serde_json::Value::Null,
                "line": req.line,
                "column": req.column,
                "duration_ms": duration_ms
            }))
            .into_response()
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

    if payload.legacy_include_flow_sensitive.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Unsupported field: include_flow_sensitive. Use includeFlowSensitive (camelCase)."
            })),
        )
            .into_response();
    }

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let coordinator = state.system_coordinator.clone();
    let code = payload.code.clone();
    let file_path = inline_web_path(payload.file_path.as_deref(), "<semantic_validation>");
    let include_flow_sensitive = payload.include_flow_sensitive;

    let diagnostics_result = crate::application::spawn_bounded_blocking(
        move || -> anyhow::Result<(Vec<SyntaxErrorDto>, Vec<SemanticErrorDto>)> {
            let code_arc: Arc<str> = Arc::from(code);
            let line_index = LineIndex::new(code_arc.as_ref());
            let (context, prepared) = prepare_ephemeral_web_operation(
                deps_bundle.as_ref(),
                coordinator.as_ref(),
                SemanticOperation::Diagnostics,
                DetailLevel::Full,
                include_flow_sensitive,
                code_arc.clone(),
                file_path,
            )?;
            let analysis = prepared.snapshot.analysis;
            let syntax = IntellisenseV2Facade::run_optional_query(
                &context,
                ObservabilityStage::SyntaxDiagnosticsQuery,
                &analysis,
                Some(coordinator.as_ref()),
                |analysis| analysis.syntax_diagnostics(V2FileId(1)),
            )
            .map_err(|_| anyhow::anyhow!("syntax diagnostics cancelled"))?
            .unwrap_or_else(|| Arc::new(Vec::new()));

            let syntax_errors: Vec<SyntaxErrorDto> = syntax
                .iter()
                .map(|e| {
                    let (line, column) = line_index
                        .byte_offset_to_utf16_position(code_arc.as_ref(), e.span.start as usize);
                    SyntaxErrorDto {
                        message: e.message.clone(),
                        line,
                        column,
                    }
                })
                .collect();

            if !syntax_errors.is_empty() {
                return Ok((syntax_errors, Vec::new()));
            }

            let diagnostics = if include_flow_sensitive {
                IntellisenseV2Facade::run_optional_query(
                    &context,
                    ObservabilityStage::SemanticDiagnosticsQuery,
                    &analysis,
                    Some(coordinator.as_ref()),
                    |analysis| analysis.semantic_diagnostics_flow_sensitive(V2FileId(1)),
                )
                .map_err(|_| anyhow::anyhow!("semantic diagnostics cancelled"))?
                .unwrap_or_else(|| Arc::new(Vec::new()))
            } else {
                IntellisenseV2Facade::run_optional_query(
                    &context,
                    ObservabilityStage::SemanticDiagnosticsQuery,
                    &analysis,
                    Some(coordinator.as_ref()),
                    |analysis| analysis.semantic_diagnostics(V2FileId(1)),
                )
                .map_err(|_| anyhow::anyhow!("semantic diagnostics cancelled"))?
                .unwrap_or_else(|| Arc::new(Vec::new()))
            };

            let semantic_errors: Vec<SemanticErrorDto> = diagnostics
                .iter()
                .map(|d| {
                    let (line, column) = line_index
                        .byte_offset_to_utf16_position(code_arc.as_ref(), d.span.start as usize);
                    let (end_line, end_column) = line_index
                        .byte_offset_to_utf16_position(code_arc.as_ref(), d.span.end as usize);
                    SemanticErrorDto {
                        message: d.message.clone(),
                        line,
                        column,
                        end_line,
                        end_column,
                        severity: format!("{:?}", d.severity).to_lowercase(),
                    }
                })
                .collect();

            Ok((syntax_errors, semantic_errors))
        },
    )
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
