#[allow(clippy::await_holding_lock)]
async fn wait_lsp_publish_diagnostics(
    receiver: &mut UnboundedReceiver<PublishDiagnosticsParams>,
    uri: &Url,
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
    let mut last_for_uri: Option<Vec<tower_lsp::lsp_types::Diagnostic>> = None;
    loop {
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }
        match tokio::time::timeout_at(deadline, receiver.recv()).await {
            Ok(Some(params)) if params.uri == *uri => {
                let diagnostics = params.diagnostics;
                if !diagnostics.is_empty() {
                    return diagnostics;
                }
                last_for_uri = Some(diagnostics);
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => break,
        }
    }
    last_for_uri.unwrap_or_default()
}

async fn wait_any_lsp_publish_diagnostics(
    receiver: &mut UnboundedReceiver<PublishDiagnosticsParams>,
    uri: &Url,
    timeout: tokio::time::Duration,
) -> Option<Vec<tower_lsp::lsp_types::Diagnostic>> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match tokio::time::timeout_at(deadline, receiver.recv()).await {
            Ok(Some(params)) if params.uri == *uri => return Some(params.diagnostics),
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => return None,
        }
    }
}

fn build_web_test_state() -> AppState {
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
            rules_config_path: None,
            cache_enabled: true,
            strict_fingerprint: false,
        })),
    }
}

async fn wait_mcp_startup(job_manager: &JobManager, startup_job_id: Option<&str>) {
    let job_id = startup_job_id.expect("startup_job_id missing");
    loop {
        let status = job_manager.wait(job_id, 60_000).await.expect("job_wait");
        match status.state {
            JobStateDto::Succeeded => break,
            JobStateDto::Queued | JobStateDto::Running => continue,
            other => panic!("startup job ended unexpectedly: {}", other.as_str()),
        }
    }
}

fn normalize_lsp_semantic_diagnostics(
    diagnostics: &[tower_lsp::lsp_types::Diagnostic],
) -> Vec<NormalizedSemanticDiagnostic> {
    let mut normalized: Vec<NormalizedSemanticDiagnostic> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.source.as_deref() == Some("bsl-analysis-v2"))
        .map(|diagnostic| {
            let severity = match diagnostic.severity {
                Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR) => "error",
                Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING) => "warning",
                Some(tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION) => "info",
                Some(tower_lsp::lsp_types::DiagnosticSeverity::HINT) => "hint",
                Some(_) | None => "info",
            };
            NormalizedSemanticDiagnostic {
                message: diagnostic.message.clone(),
                severity: severity.to_string(),
                start_line: diagnostic.range.start.line,
                start_character: diagnostic.range.start.character,
                end_line: diagnostic.range.end.line,
                end_character: diagnostic.range.end.character,
            }
        })
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn normalize_web_semantic_diagnostics(
    payload: &serde_json::Value,
) -> Vec<NormalizedSemanticDiagnostic> {
    fn read_u32(diagnostic: &serde_json::Value, key: &str, fallback: Option<&str>) -> u32 {
        diagnostic
            .get(key)
            .or_else(|| fallback.and_then(|alt| diagnostic.get(alt)))
            .and_then(|value| value.as_u64())
            .unwrap_or_default() as u32
    }

    let mut normalized: Vec<NormalizedSemanticDiagnostic> = payload
        .get("semanticErrors")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .map(|diagnostic| NormalizedSemanticDiagnostic {
            message: diagnostic
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            severity: diagnostic
                .get("severity")
                .and_then(|value| value.as_str())
                .unwrap_or("info")
                .to_lowercase(),
            start_line: read_u32(diagnostic, "line", None),
            start_character: read_u32(diagnostic, "column", None),
            end_line: read_u32(diagnostic, "endLine", Some("end_line")),
            end_character: read_u32(diagnostic, "endColumn", Some("end_column")),
        })
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn normalize_mcp_semantic_diagnostics(
    diagnostics: &[bsl_agent::semantic::dto::DiagnosticDto],
) -> Vec<NormalizedSemanticDiagnostic> {
    let mut normalized: Vec<NormalizedSemanticDiagnostic> = diagnostics
        .iter()
        .map(|diagnostic| {
            let severity = match diagnostic.severity {
                bsl_agent::semantic::dto::DiagnosticSeverityDto::Error => "error",
                bsl_agent::semantic::dto::DiagnosticSeverityDto::Warning => "warning",
                bsl_agent::semantic::dto::DiagnosticSeverityDto::Info => "info",
            };
            NormalizedSemanticDiagnostic {
                message: diagnostic.message.clone(),
                severity: severity.to_string(),
                start_line: diagnostic.range.start.line,
                start_character: diagnostic.range.start.character,
                end_line: diagnostic.range.end.line,
                end_character: diagnostic.range.end.character,
            }
        })
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NormalizedSymbol {
    name: String,
    start_line: u32,
    start_character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NormalizedPoint {
    start_line: u32,
    start_character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NormalizedMemberEntry {
    name: String,
    kind: String,
    member_identity: Option<String>,
}

fn member_kind_name(kind: CompletionItemKind) -> Option<&'static str> {
    match kind {
        CompletionItemKind::METHOD => Some("method"),
        CompletionItemKind::PROPERTY => Some("property"),
        CompletionItemKind::FIELD => Some("field"),
        CompletionItemKind::FUNCTION => Some("function"),
        CompletionItemKind::CONSTRUCTOR => Some("constructor"),
        _ => None,
    }
}

fn completion_item_member_identity(item: &tower_lsp::lsp_types::CompletionItem) -> Option<String> {
    item.data
        .as_ref()
        .and_then(|value| value.get("member_identity"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn normalize_lsp_member_labels(response: &CompletionResponse) -> Vec<String> {
    let items = match response {
        CompletionResponse::Array(items) => items.as_slice(),
        CompletionResponse::List(list) => list.items.as_slice(),
    };
    let mut out: Vec<String> = items
        .iter()
        .filter(|item| {
            matches!(
                item.kind,
                Some(CompletionItemKind::METHOD)
                    | Some(CompletionItemKind::PROPERTY)
                    | Some(CompletionItemKind::FIELD)
                    | Some(CompletionItemKind::FUNCTION)
                    | Some(CompletionItemKind::CONSTRUCTOR)
            )
        })
        .map(|item| item.label.clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn completion_item_labels(response: &CompletionResponse) -> Vec<String> {
    let items = match response {
        CompletionResponse::Array(items) => items.as_slice(),
        CompletionResponse::List(list) => list.items.as_slice(),
    };
    let mut out = items
        .iter()
        .map(|item| item.label.clone())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn normalize_lsp_member_entries(response: &CompletionResponse) -> Vec<NormalizedMemberEntry> {
    let items = match response {
        CompletionResponse::Array(items) => items.as_slice(),
        CompletionResponse::List(list) => list.items.as_slice(),
    };
    let mut out: Vec<NormalizedMemberEntry> = items
        .iter()
        .filter_map(|item| {
            let kind = item.kind.and_then(member_kind_name)?;
            Some(NormalizedMemberEntry {
                name: item.label.clone(),
                kind: kind.to_string(),
                member_identity: completion_item_member_identity(item),
            })
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn normalize_mcp_member_labels(members: &[bsl_agent::types::MemberDto]) -> Vec<String> {
    let mut out: Vec<String> = members.iter().map(|member| member.name.clone()).collect();
    out.sort();
    out.dedup();
    out
}

fn normalize_mcp_member_entries(
    members: &[bsl_agent::types::MemberDto],
) -> Vec<NormalizedMemberEntry> {
    let mut out: Vec<NormalizedMemberEntry> = members
        .iter()
        .map(|member| NormalizedMemberEntry {
            name: member.name.clone(),
            kind: member.kind.clone(),
            member_identity: member.member_identity.clone(),
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn normalize_lsp_workspace_symbols(symbols: &[SymbolInformation]) -> Vec<NormalizedSymbol> {
    let mut out: Vec<NormalizedSymbol> = symbols
        .iter()
        .map(|symbol| NormalizedSymbol {
            name: symbol.name.clone(),
            start_line: symbol.location.range.start.line,
            start_character: symbol.location.range.start.character,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn normalize_mcp_workspace_symbols(
    symbols: &[bsl_agent::types::SymbolDto],
) -> Vec<NormalizedSymbol> {
    let mut out: Vec<NormalizedSymbol> = symbols
        .iter()
        .map(|symbol| NormalizedSymbol {
            name: symbol.name.clone(),
            start_line: symbol.range.start.line,
            start_character: symbol.range.start.character,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn normalize_lsp_locations(locations: &[Location]) -> Vec<NormalizedPoint> {
    let mut out: Vec<NormalizedPoint> = locations
        .iter()
        .map(|location| NormalizedPoint {
            start_line: location.range.start.line,
            start_character: location.range.start.character,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn normalize_mcp_references(references: &[bsl_agent::types::ReferenceDto]) -> Vec<NormalizedPoint> {
    let mut out: Vec<NormalizedPoint> = references
        .iter()
        .map(|reference| NormalizedPoint {
            start_line: reference.range.start.line,
            start_character: reference.range.start.character,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn normalize_lsp_definition(response: Option<GotoDefinitionResponse>) -> Vec<NormalizedPoint> {
    let mut out: Vec<NormalizedPoint> = match response {
        Some(GotoDefinitionResponse::Scalar(location)) => vec![NormalizedPoint {
            start_line: location.range.start.line,
            start_character: location.range.start.character,
        }],
        Some(GotoDefinitionResponse::Array(locations)) => locations
            .into_iter()
            .map(|location| NormalizedPoint {
                start_line: location.range.start.line,
                start_character: location.range.start.character,
            })
            .collect(),
        Some(GotoDefinitionResponse::Link(links)) => links
            .into_iter()
            .map(|link| NormalizedPoint {
                start_line: link.target_range.start.line,
                start_character: link.target_range.start.character,
            })
            .collect(),
        None => Vec::new(),
    };
    out.sort();
    out.dedup();
    out
}

fn normalize_mcp_definition(
    location: Option<&bsl_agent::types::LocationDto>,
) -> Vec<NormalizedPoint> {
    let mut out = location
        .map(|location| {
            vec![NormalizedPoint {
                start_line: location.range.start.line,
                start_character: location.range.start.character,
            }]
        })
        .unwrap_or_default();
    out.sort();
    out.dedup();
    out
}

fn extract_hover_text(hover: Hover) -> Option<String> {
    match hover.contents {
        HoverContents::Scalar(marked) => match marked {
            MarkedString::String(value) => Some(value),
            MarkedString::LanguageString(value) => Some(value.value),
        },
        HoverContents::Array(values) => values
            .into_iter()
            .map(|value| match value {
                MarkedString::String(value) => Some(value),
                MarkedString::LanguageString(value) => Some(value.value),
            })
            .next()
            .flatten(),
        HoverContents::Markup(value) => Some(value.value),
    }
}

fn metrics_root(payload: &serde_json::Value) -> &serde_json::Value {
    payload.get("metrics").unwrap_or(payload)
}

fn stage_from_metric_key(key: &str) -> Option<&'static str> {
    if !key.starts_with("intellisense_v2_") {
        return None;
    }
    if key.contains("parse_snapshot_") {
        return Some("parse_snapshot_build");
    }
    if key.contains("runtime_wait_for_file_version") || key.contains("wait_for_file_version_") {
        return Some("runtime_wait_for_file_version");
    }
    if key.contains("runtime_snapshot_with_deps") || key.contains("snapshot_") {
        return Some("runtime_snapshot_with_deps");
    }
    if key.contains("semantic_diagnostics_query") {
        return Some("semantic_diagnostics_query");
    }
    if key.contains("syntax_diagnostics_query") {
        return Some("syntax_diagnostics_query");
    }
    if key.contains("parse_result_query") {
        return Some("parse_result_query");
    }
    if key.contains("ir_query_") {
        return Some("ir_query");
    }
    None
}

fn collect_observed_stages(payload: &serde_json::Value) -> BTreeSet<&'static str> {
    let metrics = metrics_root(payload);
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    let mut stages = BTreeSet::new();
    for key in counters.keys().chain(histograms.keys()) {
        if let Some(stage) = stage_from_metric_key(key.as_str()) {
            stages.insert(stage);
        }
    }
    stages
}

fn metric_number(value: &serde_json::Value) -> f64 {
    if let Some(number) = value.as_f64() {
        return number;
    }
    if let Some(number) = value.as_u64() {
        return number as f64;
    }
    if let Some(number) = value.as_i64() {
        return number as f64;
    }
    0.0
}

fn has_positive_counter_for_stage(payload: &serde_json::Value, stage: &str) -> bool {
    let metrics = metrics_root(payload);
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    counters.iter().any(|(key, value)| {
        stage_from_metric_key(key.as_str()) == Some(stage) && metric_number(value) > 0.0
    })
}

fn assert_drilldown_stage_metrics_for_origin(payload: &serde_json::Value, origin: &str) {
    let metrics = metrics_root(payload);
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    let stage_prefix = format!("intellisense_v2_drilldown_stage_total_origin_{origin}_");
    let latency_prefix = format!("intellisense_v2_drilldown_stage_latency_ms_origin_{origin}_");

    assert!(
        counters.keys().any(|key| key.starts_with(&stage_prefix)),
        "missing drilldown stage_total counters for origin={origin}"
    );
    assert!(
        histograms
            .keys()
            .any(|key| key.starts_with(&latency_prefix)),
        "missing drilldown stage_latency histograms for origin={origin}"
    );
}
