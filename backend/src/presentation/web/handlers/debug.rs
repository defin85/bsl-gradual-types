use super::*;

/// Debug diagnostics endpoint - returns extended debug info
pub async fn get_diagnostics_debug(
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
    let file_path = inline_web_path(payload.file_path.as_deref(), "<debug_validation>");
    let include_flow_sensitive = payload.include_flow_sensitive;

    let diagnostics_result =
        crate::application::spawn_bounded_blocking(move || -> anyhow::Result<serde_json::Value> {
            let mut debug_info = serde_json::json!({
                "steps": [],
                "resolver_available": false,
                "property_accesses": []
            });

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

            let ir = IntellisenseV2Facade::run_optional_query(
                &context,
                ObservabilityStage::IrQuery,
                &analysis,
                Some(coordinator.as_ref()),
                |analysis| analysis.ir(V2FileId(1)),
            )
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

/// Debug AST endpoint - shows parsed structure for debugging.
/// Milestone 2.16: Semantic visualization.
pub async fn get_debug_ast(
    State(_state): State<AppState>,
    Json(_payload): Json<bsl_shared::api::ValidateCodeRequest>,
) -> impl IntoResponse {
    let start = Instant::now();
    let duration_ms = start.elapsed().as_millis();

    // Stub implementation - returns minimal AST for testing.
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
