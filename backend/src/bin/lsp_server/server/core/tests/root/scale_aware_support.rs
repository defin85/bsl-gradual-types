#[derive(Clone, Copy)]
struct ScaleAwarePhase {
    name: &'static str,
    warmup: u64,
    iterations: u64,
}

#[derive(Debug, Clone)]
struct ScaleAwareWorkspaceSetup {
    platform_docs_archive: std::path::PathBuf,
    configuration_path: std::path::PathBuf,
    platform_version: String,
}

#[derive(Debug, Clone, Copy)]
struct ScaleAwareObservabilityProbe {
    every: u64,
    timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScaleAwareObservabilityProbeOutcome {
    Ok,
    TimedOut,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScaleAwareChurnMode {
    Off,
    LargeWarm,
    WarmAll,
    All,
}

impl ScaleAwareChurnMode {
    fn as_str(self) -> &'static str {
        match self {
            ScaleAwareChurnMode::Off => "off",
            ScaleAwareChurnMode::LargeWarm => "large_warm",
            ScaleAwareChurnMode::WarmAll => "warm_all",
            ScaleAwareChurnMode::All => "all",
        }
    }
}

fn scale_aware_churn_mode_from_env() -> ScaleAwareChurnMode {
    let raw =
        std::env::var("BSL_V2_SCALE_AWARE_CHURN_MODE").unwrap_or_else(|_| "large_warm".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "off" => ScaleAwareChurnMode::Off,
        "warm_all" => ScaleAwareChurnMode::WarmAll,
        "all" => ScaleAwareChurnMode::All,
        "large_warm" => ScaleAwareChurnMode::LargeWarm,
        _ => ScaleAwareChurnMode::LargeWarm,
    }
}

fn scale_aware_churn_every_from_env() -> u64 {
    std::env::var("BSL_V2_SCALE_AWARE_CHURN_EVERY")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1)
        .clamp(1, 1024)
}

fn scale_aware_non_zero_u64_from_env(name: &str, default: u64, max: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
        .clamp(1, max)
}

fn scale_aware_u64_from_env(name: &str, default: u64, max: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
        .min(max)
}

fn scale_aware_phase_plan_from_env() -> [ScaleAwarePhase; 3] {
    [
        ScaleAwarePhase {
            name: "start",
            warmup: 0,
            iterations: scale_aware_non_zero_u64_from_env(
                "BSL_V2_SCALE_AWARE_START_ITERATIONS",
                1,
                10_000,
            ),
        },
        ScaleAwarePhase {
            name: "cold",
            warmup: 0,
            iterations: scale_aware_non_zero_u64_from_env(
                "BSL_V2_SCALE_AWARE_COLD_ITERATIONS",
                5,
                10_000,
            ),
        },
        ScaleAwarePhase {
            name: "warm",
            warmup: scale_aware_u64_from_env("BSL_V2_SCALE_AWARE_WARM_WARMUP", 5, 10_000),
            iterations: scale_aware_non_zero_u64_from_env(
                "BSL_V2_SCALE_AWARE_WARM_ITERATIONS",
                50,
                10_000,
            ),
        },
    ]
}

fn scale_aware_required_warm_samples_from_env() -> u64 {
    scale_aware_non_zero_u64_from_env("BSL_V2_SCALE_AWARE_REQUIRED_WARM_SAMPLES", 50, 10_000)
}

fn real_module_phase_plan_from_env() -> [ScaleAwarePhase; 3] {
    [
        ScaleAwarePhase {
            name: "start",
            warmup: 0,
            iterations: scale_aware_non_zero_u64_from_env(
                "BSL_V2_REAL_MODULE_START_ITERATIONS",
                1,
                10_000,
            ),
        },
        ScaleAwarePhase {
            name: "cold",
            warmup: 0,
            iterations: scale_aware_non_zero_u64_from_env(
                "BSL_V2_REAL_MODULE_COLD_ITERATIONS",
                2,
                10_000,
            ),
        },
        ScaleAwarePhase {
            name: "warm",
            warmup: scale_aware_u64_from_env("BSL_V2_REAL_MODULE_WARM_WARMUP", 1, 10_000),
            iterations: scale_aware_non_zero_u64_from_env(
                "BSL_V2_REAL_MODULE_WARM_ITERATIONS",
                10,
                10_000,
            ),
        },
    ]
}

fn real_module_required_warm_samples_from_env() -> u64 {
    scale_aware_non_zero_u64_from_env("BSL_V2_REAL_MODULE_REQUIRED_WARM_SAMPLES", 10, 10_000)
}

fn real_module_observability_probe_from_env() -> ScaleAwareObservabilityProbe {
    let every =
        scale_aware_non_zero_u64_from_env("BSL_V2_REAL_MODULE_OBSERVABILITY_PROBE_EVERY", 1, 1024);
    let timeout_ms = scale_aware_non_zero_u64_from_env(
        "BSL_V2_REAL_MODULE_OBSERVABILITY_TIMEOUT_MS",
        1_500,
        30_000,
    );
    ScaleAwareObservabilityProbe {
        every,
        timeout: Duration::from_millis(timeout_ms),
    }
}

fn should_apply_scale_aware_churn(
    mode: ScaleAwareChurnMode,
    profile_name: &str,
    phase: ScaleAwarePhase,
    request_index: u64,
    churn_every: u64,
) -> bool {
    let measured = request_index >= phase.warmup;
    if !measured {
        return false;
    }
    let measured_index = request_index - phase.warmup;
    if !measured_index.is_multiple_of(churn_every) {
        return false;
    }

    match mode {
        ScaleAwareChurnMode::Off => false,
        ScaleAwareChurnMode::LargeWarm => profile_name == "large" && phase.name == "warm",
        ScaleAwareChurnMode::WarmAll => phase.name == "warm",
        ScaleAwareChurnMode::All => true,
    }
}

fn should_probe_scale_aware_observability(
    phase: ScaleAwarePhase,
    request_index: u64,
    every: u64,
) -> bool {
    let measured = request_index >= phase.warmup;
    if !measured {
        return false;
    }
    let measured_index = request_index - phase.warmup;
    measured_index.is_multiple_of(every)
}

fn scale_aware_progress_enabled() -> bool {
    std::env::var("BSL_V2_SCALE_AWARE_PROGRESS")
        .map(|raw| {
            let normalized = raw.trim().to_ascii_lowercase();
            !matches!(normalized.as_str(), "0" | "false" | "off" | "no")
        })
        .unwrap_or(true)
}

fn scale_aware_progress_every() -> u64 {
    std::env::var("BSL_V2_SCALE_AWARE_PROGRESS_EVERY")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(10)
        .clamp(1, 10_000)
}

fn should_emit_scale_aware_progress(
    request_index: u64,
    total_requests: u64,
    progress_every: u64,
) -> bool {
    if total_requests == 0 {
        return false;
    }
    let completed = request_index.saturating_add(1);
    completed == 1 || completed == total_requests || completed.is_multiple_of(progress_every)
}

fn scale_aware_progress_percent(completed: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    ((completed as f64) * 100.0) / (total as f64)
}

fn scale_aware_progress_eta_ms(elapsed: Duration, completed: u64, total: u64) -> u128 {
    if completed == 0 || completed >= total {
        return 0;
    }
    let avg_ms_per_request = elapsed.as_millis() / completed as u128;
    let remaining = (total - completed) as u128;
    avg_ms_per_request.saturating_mul(remaining)
}

fn emit_scale_aware_progress_line(line: &str, last_line_width: &mut usize) {
    use std::io::Write as _;

    let line_len = line.len();
    let trailing_padding = last_line_width.saturating_sub(line_len);
    if trailing_padding > 0 {
        print!("\r{line}{:trailing_padding$}", "");
    } else {
        print!("\r{line}");
    }
    let _ = std::io::stdout().flush();
    *last_line_width = line_len;
}

fn read_numeric_metric(value: Option<&serde_json::Value>) -> f64 {
    value
        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|n| n as f64)))
        .unwrap_or(0.0)
}

fn read_u64_metric(value: Option<&serde_json::Value>) -> u64 {
    value.and_then(|v| v.as_u64()).unwrap_or(0)
}

fn assert_optional_u64_budget(
    trace: &serde_json::Value,
    label: &str,
    metric_name: &str,
    observed_ms: Option<u64>,
    budget_ms: u64,
) {
    if let Some(observed_ms) = observed_ms {
        assert!(
            observed_ms <= budget_ms,
            "{label} {metric_name} regression: observed={}ms > budget={}ms, trace={trace:?}",
            observed_ms,
            budget_ms
        );
    }
}

fn percentile_from_sorted_samples(sorted: &[f64], quantile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let last_index = sorted.len().saturating_sub(1);
    let rank = ((last_index as f64) * quantile).round() as usize;
    sorted[rank.min(last_index)]
}

fn sample_histogram_value(samples_ms: &[f64]) -> serde_json::Value {
    if samples_ms.is_empty() {
        return serde_json::json!({
            "count": 0,
            "p50": 0.0,
            "p95": 0.0,
            "p99": 0.0,
        });
    }

    let mut sorted = samples_ms.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

    serde_json::json!({
        "count": sorted.len(),
        "p50": percentile_from_sorted_samples(&sorted, 0.50),
        "p95": percentile_from_sorted_samples(&sorted, 0.95),
        "p99": percentile_from_sorted_samples(&sorted, 0.99),
    })
}

fn histogram_metric_value(
    histograms: &serde_json::Map<String, serde_json::Value>,
    primary_key: &str,
    fallback_key: Option<&str>,
) -> serde_json::Value {
    let histogram = histograms
        .get(primary_key)
        .or_else(|| fallback_key.and_then(|key| histograms.get(key)))
        .and_then(|value| value.as_object())
        .unwrap_or_else(|| panic!("missing histogram key {primary_key}"));

    serde_json::json!({
        "count": read_u64_metric(histogram.get("count")),
        "p50": read_numeric_metric(histogram.get("p50")),
        "p95": read_numeric_metric(histogram.get("p95")),
        "p99": read_numeric_metric(histogram.get("p99")),
    })
}

fn histogram_metric_value_or_zero(
    histograms: &serde_json::Map<String, serde_json::Value>,
    primary_key: &str,
    fallback_key: Option<&str>,
) -> serde_json::Value {
    let Some(histogram) = histograms
        .get(primary_key)
        .or_else(|| fallback_key.and_then(|key| histograms.get(key)))
        .and_then(|value| value.as_object())
    else {
        return serde_json::json!({
            "count": 0,
            "p50": 0.0,
            "p95": 0.0,
            "p99": 0.0,
        });
    };

    serde_json::json!({
        "count": read_u64_metric(histogram.get("count")),
        "p50": read_numeric_metric(histogram.get("p50")),
        "p95": read_numeric_metric(histogram.get("p95")),
        "p99": read_numeric_metric(histogram.get("p99")),
    })
}

fn find_utf16_position_after_marker(source: &str, marker: &str) -> Position {
    let byte_index = source
        .find(marker)
        .unwrap_or_else(|| panic!("marker not found: {marker}"));
    let prefix = &source[..byte_index + marker.len()];
    let line = prefix.lines().count().saturating_sub(1) as u32;
    let last_line = prefix.lines().last().unwrap_or("");
    let character = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    Position::new(line, character)
}

fn find_utf16_position_at_marker_tail(source: &str, marker: &str) -> Position {
    let start_byte = source
        .find(marker)
        .unwrap_or_else(|| panic!("marker not found: {marker}"));
    let (tail_start_in_marker, tail_utf16) = marker
        .char_indices()
        .last()
        .map(|(idx, ch)| (idx, ch.len_utf16()))
        .unwrap_or_else(|| panic!("marker must not be empty"));
    let tail_start = start_byte + tail_start_in_marker;
    let prefix = &source[..tail_start];
    let line = prefix.lines().count().saturating_sub(1) as u32;
    let last_line = prefix.lines().last().unwrap_or("");
    let character = last_line
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>()
        .saturating_add(tail_utf16) as u32;
    Position::new(line, character)
}

fn syntax_helper_path_for_tests() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let syntax_helper_path = manifest_dir
        .join("..")
        .join("examples")
        .join("syntax_helper");
    assert!(
        syntax_helper_path.exists(),
        "syntax helper path does not exist: {}",
        syntax_helper_path.display()
    );
    syntax_helper_path
}

fn conf_big_candidate_roots_for_tests(
    workspace_root: &std::path::Path,
    explicit_root: Option<std::path::PathBuf>,
) -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::new();
    if let Some(root) = explicit_root.filter(|path| !path.as_os_str().is_empty()) {
        candidates.push(root);
    }
    candidates.push(workspace_root.join("examples").join("conf_big"));
    candidates.push(std::path::PathBuf::from("examples/conf_big"));
    candidates.push(std::path::PathBuf::from("../examples/conf_big"));
    candidates
}

fn conf_big_root_from_candidates(
    candidates: impl IntoIterator<Item = std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    candidates
        .into_iter()
        .find(|path| path.join("Configuration.xml").exists())
}

fn conf_big_root_for_tests() -> Option<std::path::PathBuf> {
    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let explicit_root = std::env::var_os("BSL_TEST_CONF_BIG_ROOT").map(std::path::PathBuf::from);
    conf_big_root_from_candidates(conf_big_candidate_roots_for_tests(
        &workspace_root,
        explicit_root,
    ))
}

fn conf_big_large_module_path_for_tests(root: &std::path::Path) -> std::path::PathBuf {
    root.join("Documents")
        .join("РеализацияТоваровУслуг")
        .join("Forms")
        .join("ФормаДокументаОбщая")
        .join("Ext")
        .join("Form")
        .join("Module.bsl")
}

fn conf_big_waiter_mixed_load_module_path_for_tests(root: &std::path::Path) -> std::path::PathBuf {
    root.join("Catalogs")
        .join("СпособыВыплатыЗарплаты")
        .join("Forms")
        .join("ФормаЭлемента")
        .join("Ext")
        .join("Form")
        .join("Module.bsl")
}

async fn prime_server_with_syntax_helper_deps(server: &BslLanguageServer) {
    let syntax_helper_path = syntax_helper_path_for_tests();
    let coordinator = server.coordinator.clone();
    let startup_path = syntax_helper_path.clone();
    tokio::task::spawn_blocking(move || {
        coordinator.start_with_paths_blocking(Some(startup_path.as_path()), None, None, None)
    })
    .await
    .expect("syntax helper startup join")
    .expect("syntax helper startup");
    server
        .deps_update_v2(
            "p7_universal_collection_exact_acceptance_setup",
            Some(syntax_helper_path),
            None,
        )
        .await;
    server.sync_v2_globals().await;
}

async fn prime_server_with_workspace_setup(
    server: &BslLanguageServer,
    setup: &ScaleAwareWorkspaceSetup,
    operation_id: &str,
) {
    *server.config.write().await = Some(crate::config::LspConfig {
        platform_docs_archive: Some(setup.platform_docs_archive.to_string_lossy().to_string()),
        configuration_path: Some(setup.configuration_path.to_string_lossy().to_string()),
        rules_config: None,
        platform_version: Some(setup.platform_version.clone()),
        cache_enabled: Some(true),
        strict_fingerprint: Some(false),
        enable_type_hints: Some(false),
        enable_code_actions: Some(false),
    });

    let coordinator = server.coordinator.clone();
    let startup_docs = setup.platform_docs_archive.clone();
    let startup_root = setup.configuration_path.clone();
    let startup_version = setup.platform_version.clone();
    tokio::task::spawn_blocking(move || {
        coordinator.start_with_paths_blocking(
            Some(startup_docs.as_path()),
            Some(&startup_root),
            Some(startup_version.as_str()),
            None,
        )
    })
    .await
    .expect("workspace startup join")
    .expect("workspace startup");
    server
        .deps_update_v2(
            operation_id,
            Some(setup.platform_docs_archive.clone()),
            Some(setup.configuration_path.clone()),
        )
        .await;
    server.sync_v2_globals().await;
}

async fn probe_observability_sidebar_latency(
    service: &mut LspService<BslLanguageServer>,
    request_id: i64,
    timeout: Duration,
) -> (ScaleAwareObservabilityProbeOutcome, Option<f64>) {
    let execute = Request::build("workspace/executeCommand")
        .id(request_id)
        .params(serde_json::json!({
            "command": "bsl.getObservabilityMetrics",
            "arguments": [{
                "shape": "sidebar",
            }],
        }))
        .finish();
    let started = Instant::now();
    let response = tokio::time::timeout(timeout, async {
        service.ready().await.unwrap().call(execute).await
    })
    .await;
    match response {
        Ok(Ok(Some(_))) => (
            ScaleAwareObservabilityProbeOutcome::Ok,
            Some(started.elapsed().as_millis() as f64),
        ),
        Ok(Ok(None)) | Ok(Err(_)) => (ScaleAwareObservabilityProbeOutcome::Error, None),
        Err(_) => (ScaleAwareObservabilityProbeOutcome::TimedOut, None),
    }
}

async fn open_lsp_fixture_with_snapshot(
    fixture: &str,
    uri_str: &str,
) -> (
    LspService<BslLanguageServer>,
    tokio::task::JoinHandle<()>,
    BslLanguageServer,
    Url,
    bsl_analysis_v2::FileId,
) {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;
    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse(uri_str).expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.to_string(),
        },
    };
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didOpen")
                .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let expected_version = server
        .latest_received_file_versions_v2
        .read()
        .await
        .get(&file_id)
        .copied()
        .expect("latest received version for opened file");
    assert_eq!(
        expected_version, 1,
        "opened fixture must start at version 1"
    );
    assert!(
        server
            .analysis_v2
            .wait_for_file_version(file_id, expected_version)
            .await,
        "analysis runtime must catch up to opened file version"
    );
    wait_for_type_index_precompute_completion(&server, file_id).await;

    (service, drain_task, server, uri, file_id)
}

async fn replace_lsp_fixture_and_wait(
    service: &mut LspService<BslLanguageServer>,
    server: &BslLanguageServer,
    uri: &Url,
    file_id: bsl_analysis_v2::FileId,
    version: i32,
    fixture: &str,
) {
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: fixture.to_string(),
        }],
    };
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    server.sync_v2_globals().await;
    let expected_version = server
        .latest_received_file_versions_v2
        .read()
        .await
        .get(&file_id)
        .copied()
        .expect("latest received version for changed file");
    assert_eq!(
        expected_version, version,
        "latest received version must track didChange"
    );
    assert!(
        server
            .analysis_v2
            .wait_for_file_version(file_id, expected_version)
            .await,
        "analysis runtime must catch up after didChange"
    );
    wait_for_type_index_precompute_completion(server, file_id).await;
}

async fn lsp_completion_labels_at<S>(service: &mut S, uri: &Url, position: Position) -> Vec<String>
where
    S: Service<Request, Response = Option<JsonRpcResponse>> + Send,
    S::Future: Send,
    S::Error: std::fmt::Debug,
{
    lsp_completion_labels_with_request(
        service,
        12001,
        uri,
        position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    )
    .await
}

async fn lsp_completion_labels_with_request<S>(
    service: &mut S,
    request_id: i64,
    uri: &Url,
    position: Position,
    context: Option<CompletionContext>,
) -> Vec<String>
where
    S: Service<Request, Response = Option<JsonRpcResponse>> + Send,
    S::Future: Send,
    S::Error: std::fmt::Debug,
{
    crate::server::request_context::record_completion_request_id_for_testing(
        uri,
        position,
        &request_id.to_string(),
    );
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/completion")
                .id(request_id)
                .params(
                    serde_json::to_value(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                        context,
                    })
                    .expect("CompletionParams"),
                )
                .finish(),
        )
        .await
        .expect("completion request")
        .expect("completion response");
    let completion_value =
        serde_json::to_value(&completion_response).expect("serialize completion response");
    let completion_result = completion_value
        .get("result")
        .cloned()
        .expect("completion result field");
    let completion: Option<CompletionResponse> =
        serde_json::from_value(completion_result).expect("parse completion result");

    normalize_lsp_member_labels(&completion.expect("completion result present"))
}

async fn lsp_completion_items_with_request<S>(
    service: &mut S,
    request_id: i64,
    uri: &Url,
    position: Position,
    context: Option<CompletionContext>,
) -> Vec<tower_lsp::lsp_types::CompletionItem>
where
    S: Service<Request, Response = Option<JsonRpcResponse>> + Send,
    S::Future: Send,
    S::Error: std::fmt::Debug,
{
    crate::server::request_context::record_completion_request_id_for_testing(
        uri,
        position,
        &request_id.to_string(),
    );
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/completion")
                .id(request_id)
                .params(
                    serde_json::to_value(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                        context,
                    })
                    .expect("CompletionParams"),
                )
                .finish(),
        )
        .await
        .expect("completion request")
        .expect("completion response");
    let completion_value =
        serde_json::to_value(&completion_response).expect("serialize completion response");
    let completion_result = completion_value
        .get("result")
        .cloned()
        .expect("completion result field");
    let completion: Option<CompletionResponse> =
        serde_json::from_value(completion_result).expect("parse completion result");

    match completion.expect("completion result present") {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(items) => items,
    }
}

async fn lsp_document_symbol_with_request<S>(
    service: &mut S,
    request_id: i64,
    uri: &Url,
) -> Option<DocumentSymbolResponse>
where
    S: Service<Request, Response = Option<JsonRpcResponse>> + Send,
    S::Future: Send,
    S::Error: std::fmt::Debug,
{
    let response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/documentSymbol")
                .id(request_id)
                .params(
                    serde_json::to_value(DocumentSymbolParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    })
                    .expect("DocumentSymbolParams"),
                )
                .finish(),
        )
        .await
        .expect("documentSymbol request")
        .expect("documentSymbol response");
    let value = serde_json::to_value(&response).expect("serialize documentSymbol response");
    let result = value
        .get("result")
        .cloned()
        .expect("documentSymbol result field");
    serde_json::from_value(result).expect("parse documentSymbol response")
}

fn document_symbol_response_from_jsonrpc_response(
    response: &serde_json::Value,
) -> Option<DocumentSymbolResponse> {
    let result = response
        .get("result")
        .cloned()
        .expect("documentSymbol result field");
    serde_json::from_value(result).expect("parse documentSymbol response")
}

fn document_symbol_names(response: &DocumentSymbolResponse) -> Vec<String> {
    match response {
        DocumentSymbolResponse::Flat(items) => items.iter().map(|item| item.name.clone()).collect(),
        DocumentSymbolResponse::Nested(items) => {
            fn collect(items: &[tower_lsp::lsp_types::DocumentSymbol], out: &mut Vec<String>) {
                for item in items {
                    out.push(item.name.clone());
                    if let Some(children) = item.children.as_ref() {
                        collect(children, out);
                    }
                }
            }

            let mut out = Vec::new();
            collect(items, &mut out);
            out
        }
    }
}

fn completion_labels_from_jsonrpc_response(response: &serde_json::Value) -> Vec<String> {
    let completion_result = response
        .get("result")
        .cloned()
        .expect("completion result field");
    let completion: Option<CompletionResponse> =
        serde_json::from_value(completion_result).expect("parse completion result");
    normalize_lsp_member_labels(&completion.expect("completion result present"))
}

fn completion_item_labels_from_jsonrpc_response(response: &serde_json::Value) -> Vec<String> {
    let completion_result = response
        .get("result")
        .cloned()
        .expect("completion result field");
    let completion: Option<CompletionResponse> =
        serde_json::from_value(completion_result).expect("parse completion result");
    completion_item_labels(&completion.expect("completion result present"))
}

fn hover_text_from_jsonrpc_response(response: &serde_json::Value) -> Option<String> {
    let hover_result = response.get("result").cloned().expect("hover result field");
    let hover: Option<Hover> = serde_json::from_value(hover_result).expect("parse hover result");
    hover.and_then(extract_hover_text)
}

async fn lsp_completion_resolve_item_with_request<S>(
    service: &mut S,
    request_id: i64,
    item: tower_lsp::lsp_types::CompletionItem,
) -> tower_lsp::lsp_types::CompletionItem
where
    S: Service<Request, Response = Option<JsonRpcResponse>> + Send,
    S::Future: Send,
    S::Error: std::fmt::Debug,
{
    let resolve_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("completionItem/resolve")
                .id(request_id)
                .params(serde_json::to_value(item).expect("CompletionItem"))
                .finish(),
        )
        .await
        .expect("completion resolve request")
        .expect("completion resolve response");
    let resolve_value =
        serde_json::to_value(&resolve_response).expect("serialize completion resolve response");
    let resolve_result = resolve_value
        .get("result")
        .cloned()
        .expect("completion resolve result field");

    serde_json::from_value(resolve_result).expect("parse completion resolve result")
}

async fn lsp_completion_members_at(
    service: &mut LspService<BslLanguageServer>,
    uri: &Url,
    position: Position,
) -> Vec<NormalizedMemberEntry> {
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/completion")
                .id(12011)
                .params(
                    serde_json::to_value(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                        context: Some(CompletionContext {
                            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                            trigger_character: Some(".".to_string()),
                        }),
                    })
                    .expect("CompletionParams"),
                )
                .finish(),
        )
        .await
        .expect("completion request")
        .expect("completion response");
    let completion_value =
        serde_json::to_value(&completion_response).expect("serialize completion response");
    let completion_result = completion_value
        .get("result")
        .cloned()
        .expect("completion result field");
    let completion: Option<CompletionResponse> =
        serde_json::from_value(completion_result).expect("parse completion result");

    normalize_lsp_member_entries(&completion.expect("completion result present"))
}

async fn lsp_get_completion_timeline<S>(
    service: &mut S,
    request_id: i64,
    limit: usize,
) -> serde_json::Value
where
    S: Service<Request, Response = Option<JsonRpcResponse>> + Send,
    S::Future: Send,
    S::Error: std::fmt::Debug,
{
    let execute = Request::build("workspace/executeCommand")
        .id(request_id)
        .params(serde_json::json!({
            "command": "bsl.getCompletionTimeline",
            "arguments": [{ "limit": limit }],
        }))
        .finish();
    let execute_response = service
        .ready()
        .await
        .unwrap()
        .call(execute)
        .await
        .expect("workspace/executeCommand request")
        .expect("workspace/executeCommand response");
    let value = serde_json::to_value(&execute_response).expect("serialize response");
    value.get("result").cloned().expect("result field")
}

async fn lsp_get_diagnostics_save_timeline<S>(
    service: &mut S,
    request_id: i64,
    limit: usize,
) -> serde_json::Value
where
    S: Service<Request, Response = Option<JsonRpcResponse>> + Send,
    S::Future: Send,
    S::Error: std::fmt::Debug,
{
    let execute = Request::build("workspace/executeCommand")
        .id(request_id)
        .params(serde_json::json!({
            "command": "bsl.getDiagnosticsSaveTimeline",
            "arguments": [{ "limit": limit }],
        }))
        .finish();
    let execute_response = service
        .ready()
        .await
        .unwrap()
        .call(execute)
        .await
        .expect("workspace/executeCommand request")
        .expect("workspace/executeCommand response");
    let value = serde_json::to_value(&execute_response).expect("serialize response");
    value.get("result").cloned().expect("result field")
}

async fn lsp_get_observability_metrics<S>(service: &mut S, request_id: i64) -> serde_json::Value
where
    S: Service<Request, Response = Option<JsonRpcResponse>> + Send,
    S::Future: Send,
    S::Error: std::fmt::Debug,
{
    let execute = Request::build("workspace/executeCommand")
        .id(request_id)
        .params(serde_json::json!({
            "command": "bsl.getObservabilityMetrics",
            "arguments": [],
        }))
        .finish();
    let execute_response = service
        .ready()
        .await
        .unwrap()
        .call(execute)
        .await
        .expect("workspace/executeCommand request")
        .expect("workspace/executeCommand response");
    let value = serde_json::to_value(&execute_response).expect("serialize response");
    value
        .get("result")
        .and_then(|result| result.get("metrics"))
        .cloned()
        .expect("result.metrics field")
}

async fn lsp_hover_text_at(
    service: &mut LspService<BslLanguageServer>,
    uri: &Url,
    position: Position,
) -> String {
    lsp_hover_text_optional_at(service, uri, position)
        .await
        .expect("hover text")
}

async fn lsp_hover_text_optional_at(
    service: &mut LspService<BslLanguageServer>,
    uri: &Url,
    position: Position,
) -> Option<String> {
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(12002)
                .params(
                    serde_json::to_value(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .expect("HoverParams"),
                )
                .finish(),
        )
        .await
        .expect("hover request")
        .expect("hover response");
    let hover_value = serde_json::to_value(&hover_response).expect("serialize hover response");
    let hover_result = hover_value
        .get("result")
        .cloned()
        .expect("hover result field");
    let hover: Option<Hover> = serde_json::from_value(hover_result).expect("parse hover result");

    hover.and_then(extract_hover_text)
}

async fn lsp_definition_points_at(
    service: &mut LspService<BslLanguageServer>,
    uri: &Url,
    position: Position,
) -> Vec<NormalizedPoint> {
    let definition_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/definition")
                .id(12003)
                .params(
                    serde_json::to_value(GotoDefinitionParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    })
                    .expect("GotoDefinitionParams"),
                )
                .finish(),
        )
        .await
        .expect("definition request")
        .expect("definition response");
    let definition_value =
        serde_json::to_value(&definition_response).expect("serialize definition response");
    let definition_result = definition_value
        .get("result")
        .cloned()
        .expect("definition result field");
    let definition: Option<GotoDefinitionResponse> =
        serde_json::from_value(definition_result).expect("parse definition result");

    normalize_lsp_definition(definition)
}

async fn snapshot_definition_points_at(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    uri: &Url,
    position: Position,
) -> Vec<NormalizedPoint> {
    let analysis = server.analysis_v2.snapshot().await;
    let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
        return Vec::new();
    };
    let Some(file_path) = analysis.file_path(file_id).ok().flatten() else {
        return Vec::new();
    };
    let Some(deps) = analysis.deps_data().ok() else {
        return Vec::new();
    };
    let Some(ir_program) = analysis.ir(file_id).ok().flatten() else {
        return Vec::new();
    };

    normalize_lsp_definition(crate::handlers::definition::handle_goto_definition_v2(
        crate::handlers::definition::GotoDefinitionRequest {
            analysis: &analysis,
            file_id,
            file_path,
            file_content,
            ir_program,
            deps,
            position,
            uri,
            coordinator: Some(server.coordinator.as_ref()),
        },
    ))
}

async fn lsp_signature_help_at(
    service: &mut LspService<BslLanguageServer>,
    uri: &Url,
    position: Position,
) -> Option<tower_lsp::lsp_types::SignatureHelp> {
    let signature_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/signatureHelp")
                .id(12004)
                .params(
                    serde_json::to_value(tower_lsp::lsp_types::SignatureHelpParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        context: None,
                    })
                    .expect("SignatureHelpParams"),
                )
                .finish(),
        )
        .await
        .expect("signatureHelp request")
        .expect("signatureHelp response");
    let signature_value =
        serde_json::to_value(&signature_response).expect("serialize signatureHelp response");
    let signature_result = signature_value
        .get("result")
        .cloned()
        .expect("signatureHelp result field");
    serde_json::from_value(signature_result).expect("parse signatureHelp result")
}

async fn snapshot_type_name_at_marker(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    fixture: &str,
    marker: &str,
) -> String {
    snapshot_type_name_at_marker_optional(server, file_id, fixture, marker)
        .await
        .expect("type_at_position result")
}

async fn snapshot_type_name_at_marker_optional(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    fixture: &str,
    marker: &str,
) -> Option<String> {
    snapshot_type_resolution_at_marker_optional(server, file_id, fixture, marker)
        .await
        .map(|resolution| bsl_shared::formatting::user_facing_resolution_type_name(&resolution))
}

async fn snapshot_type_resolution_at_marker(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    fixture: &str,
    marker: &str,
) -> bsl_shared::domain::types::TypeResolution {
    snapshot_type_resolution_at_marker_optional(server, file_id, fixture, marker)
        .await
        .expect("type_at_byte_offset result")
}

async fn snapshot_type_resolution_at_marker_optional(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    fixture: &str,
    marker: &str,
) -> Option<bsl_shared::domain::types::TypeResolution> {
    let position = find_utf16_position_at_marker_tail(fixture, marker);
    let analysis = server.analysis_v2.snapshot().await;
    let byte_offset = analysis
        .utf16_position_to_byte_offset(file_id, position.line, position.character)
        .ok()
        .flatten()
        .expect("utf16_position_to_byte_offset");

    analysis
        .type_at_byte_offset(file_id, byte_offset.min(u32::MAX as usize) as u32)
        .expect("type_at_byte_offset query")
}

async fn mcp_member_entries_at_code(code: &str, position: Position) -> Vec<NormalizedMemberEntry> {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(&module_path, code).expect("write module");
    let manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            job_manager.clone(),
        )
        .await
        .expect("mcp workspace open");
    wait_mcp_startup(job_manager.as_ref(), open.startup_job_id.as_deref()).await;

    let members = manager
        .bsl_members(BslMembersParams {
            session_id: open.session_id,
            file: McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            },
            position: McpPosition {
                line: position.line,
                character: position.character,
            },
            limit: 100,
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp members");

    normalize_mcp_member_entries(&members.members)
}

async fn mcp_type_name_at_code(code: &str, position: Position) -> String {
    mcp_type_name_optional_at_code(code, position)
        .await
        .expect("mcp type_at_position type_info")
}

async fn mcp_type_name_optional_at_code(code: &str, position: Position) -> Option<String> {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(&module_path, code).expect("write module");
    let manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            job_manager.clone(),
        )
        .await
        .expect("mcp workspace open");
    wait_mcp_startup(job_manager.as_ref(), open.startup_job_id.as_deref()).await;

    let response = manager
        .bsl_type_at_position(BslTypeAtPositionParams {
            session_id: open.session_id,
            file: McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            },
            position: McpPosition {
                line: position.line,
                character: position.character,
            },
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp type_at_position");

    response.type_info.map(|info| info.name)
}

async fn mcp_definition_points_at_code(code: &str, position: Position) -> Vec<NormalizedPoint> {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(&module_path, code).expect("write module");
    let manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            job_manager.clone(),
        )
        .await
        .expect("mcp workspace open");
    wait_mcp_startup(job_manager.as_ref(), open.startup_job_id.as_deref()).await;

    let response = manager
        .bsl_definition(BslDefinitionParams {
            session_id: open.session_id,
            symbol_id: None,
            file: Some(McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            }),
            position: Some(McpPosition {
                line: position.line,
                character: position.character,
            }),
        })
        .await
        .expect("mcp definition");

    normalize_mcp_definition(response.location.as_ref())
}

async fn web_hover_text_for_code(code: &str, position: Position) -> String {
    let app = create_router(build_web_test_state(), "backend/static", true);
    let response = app
        .oneshot(
            AxumRequest::post("/api/hover")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::json!({
                        "code": code,
                        "line": position.line,
                        "column": position.character
                    })
                    .to_string(),
                ))
                .expect("web hover request"),
        )
        .await
        .expect("web hover response");
    assert!(
        response.status().is_success(),
        "unexpected web hover status: {}",
        response.status()
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("web hover body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("web hover payload");
    payload
        .get("hover")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

async fn web_enhanced_hover_text_for_code(code: &str, position: Position) -> String {
    let app = create_router(build_web_test_state(), "backend/static", true);
    let response = app
        .oneshot(
            AxumRequest::post("/api/hover/enhanced")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::json!({
                        "code": code,
                        "line": position.line,
                        "column": position.character
                    })
                    .to_string(),
                ))
                .expect("web enhanced hover request"),
        )
        .await
        .expect("web enhanced hover response");
    assert!(
        response.status().is_success(),
        "unexpected web enhanced hover status: {}",
        response.status()
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("web enhanced hover body");
    let payload: serde_json::Value =
        serde_json::from_slice(&body).expect("web enhanced hover payload");
    payload
        .get("hoverText")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

async fn web_semantic_diagnostic_messages_for_code(code: &str) -> Vec<String> {
    let app = create_router(build_web_test_state(), "backend/static", true);
    let response = app
        .oneshot(
            AxumRequest::post("/api/diagnostics")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::json!({ "code": code }).to_string(),
                ))
                .expect("web diagnostics request"),
        )
        .await
        .expect("web diagnostics response");
    assert!(
        response.status().is_success(),
        "unexpected web diagnostics status: {}",
        response.status()
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("web diagnostics body");
    let payload: serde_json::Value =
        serde_json::from_slice(&body).expect("web diagnostics payload");
    normalize_web_semantic_diagnostics(&payload)
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

async fn snapshot_serve_only_type_name_at_marker(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    fixture: &str,
    marker: &str,
) -> Option<String> {
    let position = find_utf16_position_at_marker_tail(fixture, marker);
    let analysis = server.analysis_v2.snapshot().await;
    let byte_offset = analysis
        .utf16_position_to_byte_offset(file_id, position.line, position.character)
        .ok()
        .flatten()
        .expect("utf16_position_to_byte_offset");
    let profiled = analysis
        .type_at_byte_offset_serve_only_profiled(file_id, byte_offset.min(u32::MAX as usize) as u32)
        .expect("type_at_byte_offset_serve_only_profiled");

    profiled
        .resolution
        .map(|resolution| bsl_shared::formatting::user_facing_resolution_type_name(&resolution))
}

async fn snapshot_semantic_diagnostic_messages(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
) -> Vec<String> {
    let analysis = server.analysis_v2.snapshot().await;
    analysis
        .semantic_diagnostics(file_id)
        .ok()
        .flatten()
        .as_deref()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

async fn wait_for_type_index_precompute_completion(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
) {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
    loop {
        let has_task = {
            let tasks = server.type_index_precompute_tasks_v2.lock().await;
            tasks.contains_key(&file_id)
        };
        let analysis = server.analysis_v2.snapshot().await;
        let exact_ready = analysis
            .current_type_index_serve_only_ready(file_id)
            .expect("current_type_index_serve_only_ready");
        if !has_task && exact_ready {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            let observed_version = analysis.file_version(file_id).expect("file_version");
            let manual_precompute = observed_version.and_then(|version| {
                analysis
                    .precompute_type_index_for_file(file_id, Some(version), 0)
                    .ok()
            });
            let exact_ready_after_manual = analysis
                .current_type_index_serve_only_ready(file_id)
                .expect("current_type_index_serve_only_ready after manual precompute");
            if exact_ready_after_manual {
                return;
            }
            panic!(
                "type-index precompute did not yield current exact serve-only artifact for file_id={} (has_task={}, exact_ready={}, observed_version={observed_version:?}, manual_precompute={manual_precompute:?}, exact_ready_after_manual={exact_ready_after_manual})",
                file_id.0, has_task, exact_ready
            );
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

fn completion_timeline_trace_stage_duration_ms(
    trace: &serde_json::Value,
    stage_name: &str,
) -> Option<u64> {
    let stages = trace.get("stages").and_then(|value| value.as_array())?;
    stages.iter().find_map(|stage| {
        let stage = stage.as_object()?;
        let name = stage.get("name")?.as_str()?;
        if name != stage_name {
            return None;
        }
        stage.get("duration_ms").and_then(|value| value.as_u64())
    })
}

fn completion_timeline_max_stage_end_ms(trace: &serde_json::Value) -> Option<u64> {
    let stages = trace.get("stages").and_then(|value| value.as_array())?;
    stages
        .iter()
        .filter_map(|stage| {
            let stage = stage.as_object()?;
            let started_offset_ms = stage.get("started_offset_ms")?.as_u64()?;
            let duration_ms = stage.get("duration_ms")?.as_u64()?;
            Some(started_offset_ms.saturating_add(duration_ms))
        })
        .max()
}

fn completion_timeline_uncovered_gap_ms(trace: &serde_json::Value) -> Option<u64> {
    let total_duration_ms = trace.get("total_duration_ms")?.as_u64()?;
    let max_stage_end_ms = completion_timeline_max_stage_end_ms(trace).unwrap_or(0);
    Some(total_duration_ms.saturating_sub(max_stage_end_ms))
}

const COMPLETION_TIMELINE_QUERY_BUNDLE_STAGE_NAMES: &[&str] = &[
    "query_bundle_pool_wait",
    "query_bundle_deps_and_file_snapshot",
    "query_bundle_owner_hint",
    "query_bundle_ir_query",
    "query_bundle_ir_retry",
    "query_bundle_other",
];

fn completion_timeline_query_bundle_total_ms(trace: &serde_json::Value) -> Option<u64> {
    let stages = trace.get("stages").and_then(|value| value.as_array())?;
    let grouped_total = stages.iter().fold(0_u64, |acc, stage| {
        let Some(stage) = stage.as_object() else {
            return acc;
        };
        let Some(name) = stage.get("name").and_then(|value| value.as_str()) else {
            return acc;
        };
        if !COMPLETION_TIMELINE_QUERY_BUNDLE_STAGE_NAMES.contains(&name) {
            return acc;
        }
        acc.saturating_add(
            stage
                .get("duration_ms")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
        )
    });
    if grouped_total > 0 {
        return Some(grouped_total);
    }

    completion_timeline_trace_stage_duration_ms(trace, "query_bundle")
}

fn completion_timeline_query_bundle_breakdown(trace: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "total_ms": completion_timeline_query_bundle_total_ms(trace),
        "pool_wait_ms": completion_timeline_trace_stage_duration_ms(trace, "query_bundle_pool_wait"),
        "deps_and_file_snapshot_ms": completion_timeline_trace_stage_duration_ms(
            trace,
            "query_bundle_deps_and_file_snapshot",
        ),
        "owner_hint_ms": completion_timeline_trace_stage_duration_ms(trace, "query_bundle_owner_hint"),
        "ir_query_ms": completion_timeline_trace_stage_duration_ms(trace, "query_bundle_ir_query"),
        "ir_retry_ms": completion_timeline_trace_stage_duration_ms(trace, "query_bundle_ir_retry"),
        "other_ms": completion_timeline_trace_stage_duration_ms(trace, "query_bundle_other"),
    })
}

fn completion_timeline_prepare_detail_str<'a>(
    trace: &'a serde_json::Value,
    field: &str,
) -> Option<&'a str> {
    trace
        .get("prepare_details")
        .and_then(|value| value.as_object())
        .and_then(|details| details.get(field))
        .and_then(|value| value.as_str())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WarmCompletionRouteCoverage {
    attributed_samples: usize,
    head_hit_samples: usize,
    exact_hit_samples: usize,
}

fn warm_completion_route_coverage(
    measured_samples: &[serde_json::Value],
) -> WarmCompletionRouteCoverage {
    measured_samples.iter().fold(
        WarmCompletionRouteCoverage {
            attributed_samples: 0,
            head_hit_samples: 0,
            exact_hit_samples: 0,
        },
        |mut coverage, sample| {
            match sample
                .get("trace")
                .and_then(|trace| trace.get("route"))
                .and_then(|value| value.as_str())
            {
                Some("head_hit") => {
                    coverage.attributed_samples += 1;
                    coverage.head_hit_samples += 1;
                }
                Some("exact_hit") => {
                    coverage.attributed_samples += 1;
                    coverage.exact_hit_samples += 1;
                }
                Some(_) | None => {}
            }
            coverage
        },
    )
}

fn assert_warm_completion_head_first_gate(measured_samples: &[serde_json::Value]) {
    let coverage = warm_completion_route_coverage(measured_samples);

    assert!(
        coverage.attributed_samples == measured_samples.len(),
        "expected every measured warm-cache trace to expose explicit route attribution, attributed_samples={}, measured_samples={measured_samples:?}",
        coverage.attributed_samples
    );
    assert!(
        coverage.head_hit_samples == measured_samples.len(),
        "expected measured warm-cache success path to stay on head_hit once current-revision head route is available, head_hit_samples={}, exact_hit_samples={}, measured_samples={measured_samples:?}",
        coverage.head_hit_samples,
        coverage.exact_hit_samples
    );
    assert!(
        coverage.exact_hit_samples == 0,
        "expected warm-cache gate to fail on effectively exact-first completion regressions, exact_hit_samples={}, measured_samples={measured_samples:?}",
        coverage.exact_hit_samples
    );
}

fn completion_timeline_server_edge_u64(trace: &serde_json::Value, field: &str) -> Option<u64> {
    trace
        .get("server_edge_details")
        .and_then(|value| value.as_object())
        .and_then(|details| details.get(field))
        .and_then(|value| value.as_u64())
}

async fn wait_for_live_completion_timeline_trace_with_server_edge_fields(
    harness: &mut LiveLspTransportHarness,
    timeline_request_id: i64,
    limit: usize,
    completion_request_id: i64,
    required_server_edge_fields: &[&str],
) -> serde_json::Value {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let timeline =
                live_transport_get_completion_timeline(harness, timeline_request_id, limit).await;
            let traces = timeline
                .get("traces")
                .and_then(|value| value.as_array())
                .expect("completion timeline traces array");
            if let Some(trace) = traces.iter().find(|trace| {
                trace.get("request_id").and_then(|value| value.as_str())
                    == Some(&completion_request_id.to_string())
                    && required_server_edge_fields
                        .iter()
                        .all(|field| completion_timeline_server_edge_u64(trace, field).is_some())
            }) {
                break trace.clone();
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("completion trace with required server-edge fields must appear in timeline")
}

async fn wait_for_type_index_precompute_phase(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    expected_phase: super::deps_and_precompute::TypeIndexPrecomputePhaseV2,
) {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);
    loop {
        let observed_phase = {
            let tasks = server.type_index_precompute_tasks_v2.lock().await;
            tasks
                .get(&file_id)
                .map(|task| task.phase.load(std::sync::atomic::Ordering::Relaxed))
        };
        if observed_phase == Some(expected_phase.as_u8()) {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!(
                "type-index precompute did not reach expected phase for file_id={} (expected_phase={}, observed_phase={observed_phase:?})",
                file_id.0,
                expected_phase.as_u8(),
            );
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

async fn force_current_revision_without_exact_type_index(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    uri: &Url,
    content: &str,
    version: i32,
) {
    let precompute_task = {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        tasks.remove(&file_id)
    };
    if let Some(task) = precompute_task {
        task.handle.abort();
        let _ = task.handle.await;
    }
    let path = uri
        .to_file_path()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| uri.to_string());
    let text: Arc<str> = Arc::from(content.to_string());
    let path: Arc<str> = Arc::from(path);
    server.analysis_v2.apply_changes_interactive(
        bsl_runtime::application::ObservabilityOrigin::Lsp,
        vec![
            bsl_analysis_v2::Change::RemoveFile { file_id },
            bsl_analysis_v2::Change::SetFile {
                file_id,
                text: text.clone(),
                version,
                path: path.clone(),
            },
        ],
    );
    let handoff_registered_at = Instant::now();
    {
        let mut versions = server.latest_received_file_versions_v2.write().await;
        versions.insert(file_id, version);
    }
    server
        .latest_current_revision_handoff_versions_v2
        .write()
        .await
        .insert(file_id, version);
    server
        .latest_apply_enqueued_at_v2
        .write()
        .await
        .insert(file_id, handoff_registered_at);
    server.latest_document_shadow_state_v2.write().await.insert(
        file_id,
        DocumentShadowStateV2 {
            version,
            text: text.clone(),
        },
    );
    server.cancel_type_index_precompute_v2(file_id).await;
    let exact_ready = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready");
    assert!(
        !exact_ready,
        "test setup must create current-revision semantic-index miss"
    );
    let _ = server.analysis_v2.snapshot().await.ir(file_id);
    assert!(
        server
            .analysis_v2
            .snapshot()
            .await
            .current_completion_head_ready(file_id)
            .expect("current_completion_head_ready"),
        "test setup must publish current-revision completion head artifact"
    );
}
