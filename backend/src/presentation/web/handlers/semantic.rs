use super::*;

/// Enhanced hover endpoint - provides detailed variable information
/// Milestone 2.13: Enhanced hover with variable details
pub async fn get_enhanced_hover(
    State(state): State<AppState>,
    Json(req): Json<HoverRequest>,
) -> impl IntoResponse {
    use bsl_shared::formatting::DetailLevel;

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

    // Parse detail_level from request
    let detail_level = DetailLevel::parse(&req.detail_level);

    let deps_bundle = state.deps_bundle_v2.read().await.clone();
    let coordinator = state.system_coordinator.clone();
    let code = req.code.clone();
    let line = req.line;
    let column = req.column;
    let syntax_helper_path = state.syntax_helper_path.clone();
    let include_flow_sensitive = req.include_flow_sensitive;

    let hover_result =
        crate::application::spawn_bounded_blocking(move || -> anyhow::Result<Option<String>> {
            let (context, prepared) = prepare_ephemeral_web_operation(
                deps_bundle.as_ref(),
                coordinator.as_ref(),
                SemanticOperation::Hover,
                DetailLevel::Full,
                include_flow_sensitive,
                Arc::from(code),
                Arc::from("hover_request.bsl"),
            )?;
            let analysis = prepared.snapshot.analysis;
            let file_content = analysis
                .file_text(V2FileId(1))
                .map_err(|_| anyhow::anyhow!("file_text cancelled"))?
                .ok_or_else(|| anyhow::anyhow!("file_text unavailable"))?;
            let ir_program = IntellisenseV2Facade::run_optional_query(
                &context,
                ObservabilityStage::IrQuery,
                &analysis,
                Some(coordinator.as_ref()),
                |analysis| analysis.ir(V2FileId(1)),
            )
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
                &analysis,
                V2FileId(1),
                file_content.as_ref(),
                line,
                column,
                include_flow_sensitive,
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

            let hover_text_str = hover_text
                .map(|value| normalize_user_facing_type_name(&value))
                .unwrap_or_else(|| "No information available".to_string());

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
    /// Включить flow-sensitive информацию (default: false)
    #[serde(default, rename = "includeFlowSensitive")]
    pub include_flow_sensitive: bool,
    /// Legacy field: `include_flow_sensitive` is rejected (breaking).
    #[serde(default, rename = "include_flow_sensitive")]
    pub legacy_include_flow_sensitive: Option<bool>,
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
    let file_path = req.file_path.clone();
    let compact = req.compact;
    let include_call_graph = req.include_call_graph;
    let include_flow_sensitive = req.include_flow_sensitive;

    let tree_result = crate::application::spawn_bounded_blocking(
        move || -> anyhow::Result<bsl_shared::api::semantic_dtos::SemanticTreeDto> {
            let code_arc: Arc<str> = Arc::from(code);
            let line_index = LineIndex::new(code_arc.as_ref());
            let (context, prepared) = prepare_ephemeral_web_operation(
                deps_bundle.as_ref(),
                coordinator.as_ref(),
                SemanticOperation::SymbolSearch,
                DetailLevel::Full,
                include_flow_sensitive,
                code_arc.clone(),
                Arc::from(file_path.clone()),
            )?;
            let analysis = prepared.snapshot.analysis;
            let ir_program = IntellisenseV2Facade::run_optional_query(
                &context,
                ObservabilityStage::IrQuery,
                &analysis,
                Some(coordinator.as_ref()),
                |analysis| analysis.ir(V2FileId(1)),
            )
            .map_err(|_| anyhow::anyhow!("ir query cancelled"))?
            .ok_or_else(|| anyhow::anyhow!("ir unavailable"))?;

            let dto = if compact {
                ir_program.to_compact_dto(code_arc.as_ref(), &line_index)
            } else {
                ir_program.to_dto(
                    include_call_graph,
                    include_flow_sensitive,
                    code_arc.as_ref(),
                    &line_index,
                )
            };

            Ok(dto)
        },
    )
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
