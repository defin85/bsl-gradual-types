use super::*;
use axum::http::{header, Request as AxumRequest};
use bsl_agent::jobs::JobManager;
use bsl_agent::server::types::{
    BslDefinitionParams, BslDiagnosticsParams, BslMembersParams, BslReferencesParams,
    BslSymbolSearchParams, BslTypeAtPositionParams, DocumentRef as McpDocumentRef,
    FileRef as McpFileRef, Position as McpPosition, WorkspaceOpenParams, WorkspaceScope,
    WorkspaceScopeTagged,
};
use bsl_agent::session::SessionManager;
use bsl_agent::types::JobStateDto;
use bsl_backend::perf_gate_evaluator::{
    get_report_u64, validate_parity_cutover_evidence, PARITY_DRIFT_RATE_MAX_FOR_CUTOVER,
    PARITY_PAIRS_TOTAL_MIN_FOR_CUTOVER,
};
use bsl_backend::presentation::web::{create_router, AppState};
use bsl_backend::system::{
    build_deps_bundle_v2, EffectiveStartupInputs, IndexItem, IndexItemKind, IndexKind,
    IndexSnapshot, IndexSnapshotId, TypeKind,
};
use futures::StreamExt;
use std::collections::BTreeSet;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc::UnboundedReceiver;
use tower::Service;
use tower::ServiceExt;
use tower_lsp::jsonrpc::{Request, Response as JsonRpcResponse};
use tower_lsp::lsp_types::{
    ClientCapabilities, CodeActionContext, CodeActionOrCommand, CodeActionParams,
    CompletionContext, CompletionItemKind, CompletionParams, CompletionResponse,
    CompletionTriggerKind, DidChangeConfigurationParams, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentFormattingParams,
    DocumentRangeFormattingParams, DocumentSymbolParams, DocumentSymbolResponse, FormattingOptions,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
    InitializeParams, InitializedParams, InlayHint, InlayHintLabel, InlayHintParams, Location,
    MarkedString, PartialResultParams, Position, PrepareRenameResponse, PublishDiagnosticsParams,
    Range, ReferenceContext, ReferenceParams, RenameParams, SymbolInformation, SymbolKind,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Url, VersionedTextDocumentIdentifier, WorkDoneProgressParams,
    WorkspaceEdit, WorkspaceSymbolParams,
};
use tower_lsp::LanguageServer;
use tower_lsp::LspService;
use tower_lsp::Server;

fn init_test_tracing() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    });
}

static PRECOMPUTE_DELAY_ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> =
    std::sync::OnceLock::new();

fn lock_test_env_mutex(
    mutex: &'static std::sync::OnceLock<std::sync::Mutex<()>>,
) -> std::sync::MutexGuard<'static, ()> {
    mutex
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

const UNIFIED_STAGE_COUNTER_KEYS: &[&str] = &[
    "intellisense_v2_runtime_wait_for_file_version_queue_wait_total",
    "intellisense_v2_runtime_wait_for_file_version_exec_total",
    "intellisense_v2_runtime_snapshot_with_deps_queue_wait_total",
    "intellisense_v2_runtime_snapshot_with_deps_exec_total",
    "intellisense_v2_runtime_apply_changes_queue_wait_total",
    "intellisense_v2_runtime_apply_changes_exec_total",
    "intellisense_v2_runtime_apply_change_set_file_exec_total",
    "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_total",
    "intellisense_v2_runtime_apply_change_remove_file_exec_total",
    "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_total",
    "intellisense_v2_runtime_type_index_precompute_queue_wait_total",
    "intellisense_v2_runtime_type_index_precompute_exec_total",
    "intellisense_v2_runtime_type_index_precompute_build_exec_total",
    "intellisense_v2_runtime_type_index_precompute_ir_exec_total",
    "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_total",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_total",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_total",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_total",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_total",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_incremental",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_reused",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_full",
    "intellisense_v2_parse_snapshot_total_origin_lsp_mode_other",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_incremental_failed",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_no_previous_tree",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_no_edits_provided",
    "intellisense_v2_parse_snapshot_fallback_total_origin_lsp_reason_other",
    "intellisense_v2_wait_for_file_version_diagnostics_total",
    "intellisense_v2_snapshot_diagnostics_total",
    "intellisense_v2_ir_query_other_total",
    "intellisense_v2_syntax_diagnostics_query_total",
    "intellisense_v2_semantic_diagnostics_query_total",
    "intellisense_v2_parse_result_query_total",
    "intellisense_v2_ir_query_cancelled_total_other",
    "intellisense_v2_query_cancelled_total_syntax",
    "intellisense_v2_query_cancelled_total_semantic",
    "intellisense_v2_interactive_wait_budget_exhausted_total",
    "intellisense_v2_interactive_stale_served_total",
    "intellisense_v2_interactive_knob_clamped_total",
    "intellisense_v2_singleflight_leader_total",
    "intellisense_v2_singleflight_shared_total",
    "intellisense_v2_singleflight_key_unavailable_total",
    "intellisense_v2_runtime_queue_wait_interactive_total",
    "intellisense_v2_runtime_queue_wait_background_total",
    "intellisense_v2_runtime_exec_interactive_total",
    "intellisense_v2_runtime_exec_background_total",
    "intellisense_v2_completion_stale_fallback_total",
    "intellisense_v2_completion_fallback_unavailable_total",
    "intellisense_v2_completion_owner_hint_index_fetch_block_on_total",
    "intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total",
    "intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total",
    "intellisense_v2_completion_owner_hint_index_fetch_block_on_other_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_other_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_type_index_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_parse_result_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_other_total",
    "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_check_cancellation_total",
    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready",
    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline",
    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task",
    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_task_present_wrong_version",
    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_observed_version_mismatch",
    "intellisense_v2_completion_exact_type_index_wait_promotion_total",
    "intellisense_v2_completion_exact_type_index_wait_join_total",
    "intellisense_v2_completion_exact_type_index_wait_ready_after_wait_total",
    "intellisense_v2_revision_lag_sample_total",
    "intellisense_v2_observability_contract_violation_total",
    "intellisense_v2_projection_missing_total",
    "intellisense_v2_runtime_saturation_sample_total",
];

const UNIFIED_STAGE_HISTOGRAM_KEYS: &[&str] = &[
    "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms",
    "intellisense_v2_runtime_wait_for_file_version_exec_ms",
    "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms",
    "intellisense_v2_runtime_snapshot_with_deps_exec_ms",
    "intellisense_v2_runtime_apply_changes_queue_wait_ms",
    "intellisense_v2_runtime_apply_changes_exec_ms",
    "intellisense_v2_runtime_apply_change_set_file_exec_ms",
    "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_ms",
    "intellisense_v2_runtime_apply_change_remove_file_exec_ms",
    "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_queue_wait_ms",
    "intellisense_v2_runtime_type_index_precompute_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_build_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
    "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms",
    "intellisense_v2_runtime_apply_changes_batch_size",
    "intellisense_v2_runtime_apply_changes_changed_files_count",
    "intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch",
    "intellisense_v2_completion_owner_hint_index_fetch_revision_start",
    "intellisense_v2_completion_owner_hint_index_fetch_revision_end",
    "intellisense_v2_completion_owner_hint_index_fetch_revision_delta",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_incremental",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_reused",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_full",
    "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_other",
    "intellisense_v2_parse_snapshot_changed_ranges_count_origin_lsp",
    "intellisense_v2_parse_snapshot_changed_ranges_bytes_origin_lsp",
    "intellisense_v2_wait_for_file_version_diagnostics_ms",
    "intellisense_v2_snapshot_diagnostics_ms",
    "intellisense_v2_ir_query_other_ms",
    "intellisense_v2_syntax_diagnostics_query_ms",
    "intellisense_v2_semantic_diagnostics_query_ms",
    "intellisense_v2_parse_result_query_ms",
    "intellisense_v2_singleflight_wait_ms",
    "intellisense_v2_runtime_queue_wait_interactive_ms",
    "intellisense_v2_runtime_queue_wait_background_ms",
    "intellisense_v2_runtime_exec_interactive_ms",
    "intellisense_v2_runtime_exec_background_ms",
    "completion_stage_prepare_apply_age_at_start_ms",
    "completion_stage_prepare_apply_age_at_terminal_ms",
    "completion_stage_exact_wait_apply_age_at_start_ms",
    "completion_stage_exact_wait_apply_age_at_terminal_ms",
    "intellisense_v2_semantic_diagnostics_query_inputs_ms",
    "intellisense_v2_semantic_diagnostics_query_parse_result_ms",
    "intellisense_v2_semantic_diagnostics_query_ir_ms",
    "intellisense_v2_semantic_diagnostics_query_collect_ms",
    "intellisense_v2_semantic_diagnostics_query_flow_sensitive_ms",
    "intellisense_v2_revision_lag_versions",
];

#[test]
fn diagnostics_debounce_floor_prevents_zero_ms_tight_loops() {
    assert_eq!(clamp_diagnostics_debounce_ms(0), 25);
    assert_eq!(clamp_diagnostics_debounce_ms(1), 25);
    assert_eq!(clamp_diagnostics_debounce_ms(25), 25);
    assert_eq!(clamp_diagnostics_debounce_ms(250), 250);
}

const UNIFIED_STAGE_GAUGE_KEYS: &[&str] = &[
    "intellisense_v2_runtime_saturation_waiters_interactive",
    "intellisense_v2_runtime_saturation_waiters_background",
    "intellisense_v2_runtime_saturation_permits_interactive",
    "intellisense_v2_runtime_saturation_permits_background",
    "intellisense_v2_runtime_saturation_permits_shared",
    "intellisense_v2_runtime_saturation_queue_depth_total",
    "intellisense_v2_completion_owner_hint_index_fetch_active",
];

fn assert_unified_intellisense_v2_stage_contract(payload: &serde_json::Value) {
    let metrics = payload.get("metrics").expect("metrics field");
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let gauges = metrics
        .get("gauges")
        .and_then(|value| value.as_object())
        .expect("metrics.gauges object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    for key in UNIFIED_STAGE_COUNTER_KEYS {
        assert!(
            counters.contains_key(*key),
            "missing counter key {key}, got keys={:?}",
            counters.keys().collect::<Vec<_>>()
        );
    }

    for key in UNIFIED_STAGE_HISTOGRAM_KEYS {
        assert!(
            histograms.contains_key(*key),
            "missing histogram key {key}, got keys={:?}",
            histograms.keys().collect::<Vec<_>>()
        );
    }

    for key in UNIFIED_STAGE_GAUGE_KEYS {
        assert!(
            gauges.contains_key(*key),
            "missing gauge key {key}, got keys={:?}",
            gauges.keys().collect::<Vec<_>>()
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NormalizedSemanticDiagnostic {
    message: String,
    severity: String,
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
}

async fn initialize_lsp_service(service: &mut LspService<BslLanguageServer>) {
    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let initialize_response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(
        initialize_response.is_some(),
        "initialize should return a response"
    );

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );
}

struct LiveLspTransportHarness {
    reader: BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    writer: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    server_task: tokio::task::JoinHandle<()>,
}

impl LiveLspTransportHarness {
    async fn send_notification<P>(&mut self, method: &str, params: P)
    where
        P: serde::Serialize,
    {
        self.write_message(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
        .await;
    }

    async fn send_request<P>(&mut self, id: i64, method: &str, params: P) -> serde_json::Value
    where
        P: serde::Serialize,
    {
        self.write_message(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await;
        self.wait_for_response(id).await
    }

    async fn shutdown(mut self) {
        let shutdown_response = self
            .send_request(9_999_991, "shutdown", serde_json::Value::Null)
            .await;
        assert!(
            shutdown_response.get("result").is_some(),
            "shutdown should return a response"
        );
        self.send_notification("exit", serde_json::Value::Null).await;
        drop(self.writer);
        if tokio::time::timeout(Duration::from_secs(5), &mut self.server_task)
            .await
            .is_err()
        {
            self.server_task.abort();
            let _ = self.server_task.await;
        }
    }

    async fn write_message(&mut self, message: &serde_json::Value) {
        let body = serde_json::to_vec(message).expect("serialize LSP transport message");
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.writer
            .write_all(header.as_bytes())
            .await
            .expect("write LSP Content-Length header");
        self.writer
            .write_all(&body)
            .await
            .expect("write LSP message body");
        self.writer.flush().await.expect("flush LSP client stream");
    }

    async fn wait_for_response(&mut self, expected_id: i64) -> serde_json::Value {
        tokio::time::timeout(Duration::from_secs(60), async {
            loop {
                let message = self.read_message().await;
                if message.get("method").is_some() {
                    continue;
                }
                if message
                    .get("id")
                    .and_then(|value| value.as_i64())
                    == Some(expected_id)
                {
                    return message;
                }
            }
        })
        .await
        .expect("timed out waiting for LSP transport response")
    }

    async fn read_message(&mut self) -> serde_json::Value {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            let bytes = self
                .reader
                .read_line(&mut line)
                .await
                .expect("read LSP header line");
            assert!(bytes > 0, "unexpected EOF while reading LSP header");
            if line == "\r\n" {
                break;
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if let Some(raw_len) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(
                    raw_len
                        .trim()
                        .parse::<usize>()
                        .expect("parse Content-Length header"),
                );
            }
        }
        let body_len = content_length.expect("Content-Length header must be present");
        let mut body = vec![0; body_len];
        self.reader
            .read_exact(&mut body)
            .await
            .expect("read LSP message body");
        serde_json::from_slice(&body).expect("parse framed LSP JSON message")
    }
}

async fn spawn_live_lsp_transport_harness(
    coordinator: Arc<SystemCoordinator>,
) -> (LiveLspTransportHarness, BslLanguageServer) {
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (service, socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");
    let service = crate::server::request_context::DispatchContextService::new(
        crate::server::request_context::RequestContextService::new(service),
    );
    let (client_stream, server_stream) = tokio::io::duplex(1024 * 1024);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let (server_read, server_write) = tokio::io::split(server_stream);
    let server_task = tokio::spawn(async move {
        Server::new(server_read, server_write, socket)
            .serve(service)
            .await;
    });
    (
        LiveLspTransportHarness {
            reader: BufReader::new(client_read),
            writer: client_write,
            server_task,
        },
        server,
    )
}

async fn initialize_live_lsp_transport(harness: &mut LiveLspTransportHarness) {
    let initialize_response = harness
        .send_request(
            1,
            "initialize",
            InitializeParams {
                capabilities: ClientCapabilities::default(),
                ..Default::default()
            },
        )
        .await;
    assert!(
        initialize_response.get("result").is_some(),
        "initialize should return a response"
    );
    harness
        .send_notification("initialized", InitializedParams {})
        .await;
}

async fn live_transport_append_text_change(
    harness: &mut LiveLspTransportHarness,
    uri: &Url,
    current_text: &str,
    version: i32,
    appended_text: &str,
) {
    let end_position = utf16_end_position(current_text);
    harness
        .send_notification(
            "textDocument/didChange",
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: end_position,
                        end: end_position,
                    }),
                    range_length: None,
                    text: appended_text.to_string(),
                }],
            },
        )
        .await;
}

async fn live_transport_completion_labels_with_request(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
    uri: &Url,
    position: Position,
    context: Option<CompletionContext>,
) -> Vec<String> {
    let completion_response = harness
        .send_request(
            request_id,
            "textDocument/completion",
            CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context,
            },
        )
        .await;
    let completion_result = completion_response
        .get("result")
        .cloned()
        .expect("completion result field");
    let completion: Option<CompletionResponse> =
        serde_json::from_value(completion_result).expect("parse completion result");

    normalize_lsp_member_labels(&completion.expect("completion result present"))
}

async fn live_transport_get_completion_timeline(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
    limit: usize,
) -> serde_json::Value {
    let execute_response = harness
        .send_request(
            request_id,
            "workspace/executeCommand",
            serde_json::json!({
                "command": "bsl.getCompletionTimeline",
                "arguments": [{ "limit": limit }],
            }),
        )
        .await;
    execute_response
        .get("result")
        .cloned()
        .expect("result field")
}

async fn live_transport_get_observability_metrics(
    harness: &mut LiveLspTransportHarness,
    request_id: i64,
) -> serde_json::Value {
    let execute_response = harness
        .send_request(
            request_id,
            "workspace/executeCommand",
            serde_json::json!({
                "command": "bsl.getObservabilityMetrics",
                "arguments": [],
            }),
        )
        .await;
    execute_response
        .get("result")
        .and_then(|result| result.get("metrics"))
        .cloned()
        .expect("result.metrics field")
}

async fn shutdown_lsp_service(
    service: &mut LspService<BslLanguageServer>,
    close_uri: Option<&Url>,
) {
    if let Some(uri) = close_uri {
        let did_close = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        };
        let did_close_req = Request::build("textDocument/didClose")
            .params(serde_json::to_value(did_close).expect("DidCloseTextDocumentParams"))
            .finish();
        let did_close_response = service
            .ready()
            .await
            .unwrap()
            .call(did_close_req)
            .await
            .expect("didClose notification");
        assert!(did_close_response.is_none(), "didClose is a notification");
    }

    let shutdown_req = Request::build("shutdown").id(2).finish();
    let shutdown_response = service
        .ready()
        .await
        .unwrap()
        .call(shutdown_req)
        .await
        .expect("shutdown request");
    assert!(
        shutdown_response.is_some(),
        "shutdown should return a response"
    );

    let exit_req = Request::build("exit").finish();
    let exit_response = service
        .ready()
        .await
        .unwrap()
        .call(exit_req)
        .await
        .expect("exit notification");
    assert!(exit_response.is_none(), "exit is a notification");
}

#[tokio::test]
async fn p34_initialized_with_startup_config_returns_without_waiting_for_startup() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) =
        LspService::build(move |client| BslLanguageServer::new(client, coordinator.clone()))
            .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let syntax_helper_path = syntax_helper_path_for_tests();
    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities {
            window: Some(tower_lsp::lsp_types::WindowClientCapabilities {
                work_done_progress: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        },
        initialization_options: Some(serde_json::json!({
            "platformDocsArchive": syntax_helper_path.to_string_lossy(),
            "platformVersion": "8.3.25",
            "cacheEnabled": true,
            "enableTypeHints": false,
            "enableCodeActions": false
        })),
        ..Default::default()
    };

    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let initialize_response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(
        initialize_response.is_some(),
        "initialize should return a response"
    );

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = tokio::time::timeout(std::time::Duration::from_millis(200), async {
        service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .expect("initialized notification")
    })
    .await
    .expect("initialized must return without waiting for startup");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let stats_request = Request::build("workspace/executeCommand")
        .id(2)
        .params(serde_json::json!({
            "command": "bsl.getTypeRepositoryStats",
            "arguments": [{}]
        }))
        .finish();
    let stats_response = tokio::time::timeout(std::time::Duration::from_millis(200), async {
        service
            .ready()
            .await
            .unwrap()
            .call(stats_request)
            .await
            .expect("bsl.getTypeRepositoryStats request")
    })
    .await
    .expect("interactive command must stay responsive during startup");
    assert!(
        stats_response.is_some(),
        "executeCommand should return a response"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p35_large_conf_big_did_open_returns_promptly() {
    let conf_big_module = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join("conf_big")
        .join("Documents")
        .join("РеализацияТоваровУслуг")
        .join("Forms")
        .join("ФормаДокументаОбщая")
        .join("Ext")
        .join("Form")
        .join("Module.bsl");
    if !conf_big_module.exists() {
        eprintln!(
            "skipping p35_large_conf_big_did_open_returns_promptly: missing fixture {}",
            conf_big_module.display()
        );
        return;
    }

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) =
        LspService::build(move |client| BslLanguageServer::new(client, coordinator.clone()))
            .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;
    let mut service = crate::server::request_context::DispatchContextService::new(
        crate::server::request_context::RequestContextService::new(service),
    );

    let uri = Url::from_file_path(&conf_big_module).expect("conf_big module uri");
    let text = std::fs::read_to_string(&conf_big_module).expect("read conf_big module");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri,
            language_id: "bsl".to_string(),
            version: 1,
            text,
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification")
    })
    .await
    .expect("didOpen on large conf_big module must return promptly");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_did_open_returns_before_blocking_parse_snapshot_finishes() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    const FIXTURE: &str = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK);
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_OPEN_BLOCKING_PARSE_DELAY_MS", "1500");

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_did_open_prompt_return.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
        },
    };
    let started = Instant::now();
    let did_open_response = tokio::time::timeout(Duration::from_millis(250), async {
        service
            .ready()
            .await
            .unwrap()
            .call(
                Request::build("textDocument/didOpen")
                    .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
                    .finish(),
            )
            .await
            .expect("didOpen notification")
    })
    .await
    .expect("didOpen must return before blocking parse snapshot completes");
    let elapsed = started.elapsed();
    assert!(did_open_response.is_none(), "didOpen is a notification");
    assert!(
        elapsed < Duration::from_millis(250),
        "didOpen must stay short-lived under blocking parse snapshot delay (elapsed={elapsed:?})"
    );

    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    tokio::time::timeout(Duration::from_millis(100), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(1)
                && server
                    .analysis_v2
                    .file_revision_state(file_id)
                    .await
                    .map(|state| state.version)
                    == Some(1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didOpen must complete after current-revision handoff");

    drain_task.abort();
}

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

#[tokio::test]
async fn p6_fast_did_change_series_publish_diagnostics_is_monotonic() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<tower_lsp::lsp_types::PublishDiagnosticsParams>();

    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) =
                serde_json::from_value::<tower_lsp::lsp_types::PublishDiagnosticsParams>(params)
            else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    // LSP initialize handshake is required, otherwise client notifications are suppressed.
    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///test.bsl").expect("test uri");

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Procedure Test()\nEndProcedure".to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    // Two fast didChange events with different versions. We want to ensure that the server
    // never publishes diagnostics for an older version after a newer one is published.
    let did_change_v2 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test(\nEndProcedure".to_string(),
        }],
    };
    let did_change_req_v2 = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change_v2).expect("DidChangeTextDocumentParams v2"))
        .finish();
    let did_change_response_v2 = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req_v2)
        .await
        .expect("didChange v2 notification");
    assert!(
        did_change_response_v2.is_none(),
        "didChange is a notification"
    );

    let did_change_v3 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 3,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test()\nEndProcedure".to_string(),
        }],
    };
    let did_change_req_v3 = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change_v3).expect("DidChangeTextDocumentParams v3"))
        .finish();
    let did_change_response_v3 = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req_v3)
        .await
        .expect("didChange v3 notification");
    assert!(
        did_change_response_v3.is_none(),
        "didChange is a notification"
    );

    let mut versions = Vec::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2);

    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let Ok(next) = tokio::time::timeout(remaining, published_rx.recv()).await else {
            break;
        };
        let Some(params) = next else {
            break;
        };
        if params.uri != uri {
            continue;
        }
        let Some(version) = params.version else {
            continue;
        };

        versions.push(version);
        if version == 3 {
            break;
        }
    }

    assert!(
        versions.contains(&3),
        "expected diagnostics for version 3 to be published, got {:?}",
        versions
    );

    for pair in versions.windows(2) {
        assert!(
            pair[1] >= pair[0],
            "publishDiagnostics versions must not go backwards: {:?}",
            versions
        );
    }

    // After observing version 3, ensure we don't later publish version 1/2 (no jump-back).
    let after_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(300);
    while tokio::time::Instant::now() < after_deadline {
        let remaining = after_deadline.saturating_duration_since(tokio::time::Instant::now());
        let Ok(next) = tokio::time::timeout(remaining, published_rx.recv()).await else {
            break;
        };
        let Some(params) = next else {
            break;
        };
        if params.uri != uri {
            continue;
        }
        let Some(version) = params.version else {
            continue;
        };
        assert!(
            version >= 3,
            "unexpected jump-back diagnostics: v{}",
            version
        );
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let saw_stale_or_cancelled = counters.iter().any(|(key, value)| {
        key.starts_with("intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_")
            && (key.contains("reason_superseded_version")
                || key.contains("reason_superseded_generation"))
            && metric_number(value) > 0.0
    });
    assert!(
        saw_stale_or_cancelled
            || counters.iter().any(|(key, value)| {
                key.starts_with("intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_")
                    && key.contains("reason_other_cancel")
                    && metric_number(value) > 0.0
        }),
        "expected diagnostics pipeline metrics to record stale/cancelled runs after rapid didChange series"
    );
    let saw_did_change_fast_profile = counters.iter().any(|(key, value)| {
        key.starts_with(
            "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_change_profile_fast_",
        ) && metric_number(value) > 0.0
    });
    assert!(
        saw_did_change_fast_profile,
        "expected didChange traffic to execute fast diagnostics profile"
    );
    let saw_did_change_idle_heavy_profile = counters.iter().any(|(key, value)| {
        key.starts_with(
            "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_did_change_profile_idle_heavy_",
        ) && metric_number(value) > 0.0
    });
    assert!(
        !saw_did_change_idle_heavy_profile,
        "idle_heavy diagnostics must not execute under trigger_did_change"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p6_type_index_precompute_slot_tracks_latest_version_and_clears_on_did_close() {
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

    let uri = Url::parse("file:///type-index-precompute-slot-v2.bsl").expect("test uri");
    let base_text =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: base_text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let latest_version = 8_i32;
    for version in 2..=latest_version {
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: base_text.to_string(),
            }],
        };
        let did_change_req = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
            .finish();
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req)
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");
    }

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    let file_id = server.get_or_create_file_id_v2(&uri).await;

    let observed_latest = server
        .latest_received_file_versions_v2
        .read()
        .await
        .get(&file_id)
        .copied();
    assert_eq!(
        observed_latest,
        Some(latest_version),
        "latest received version must track the newest didChange"
    );

    tokio::time::sleep(Duration::from_millis(30)).await;
    {
        let tasks = server.type_index_precompute_tasks_v2.lock().await;
        assert!(
            tasks.len() <= 1,
            "precompute scheduler must keep at most one task slot per file"
        );
        if let Some(task) = tasks.get(&file_id) {
            assert_eq!(
                task.supersession_key.requested_version, latest_version,
                "active precompute slot must target latest requested version"
            );
        }
    }

    let did_close = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
    };
    let did_close_req = Request::build("textDocument/didClose")
        .params(serde_json::to_value(did_close).expect("DidCloseTextDocumentParams"))
        .finish();
    let did_close_response = service
        .ready()
        .await
        .unwrap()
        .call(did_close_req)
        .await
        .expect("didClose notification");
    assert!(did_close_response.is_none(), "didClose is a notification");

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if !server
                .type_index_precompute_tasks_v2
                .lock()
                .await
                .contains_key(&file_id)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("type_index precompute slot must be cleared after didClose");

    drain_task.abort();
}

#[tokio::test]
async fn p6_did_close_records_client_cancel_for_inflight_diagnostics() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///did-close-cancel.bsl").expect("test uri");

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Procedure Test()\nEndProcedure".to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test(\nEndProcedure".to_string(),
        }],
    };
    let did_change_req = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
        .finish();
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req)
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let did_close = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
    };
    let did_close_req = Request::build("textDocument/didClose")
        .params(serde_json::to_value(did_close).expect("DidCloseTextDocumentParams"))
        .finish();
    let did_close_response = service
        .ready()
        .await
        .unwrap()
        .call(did_close_req)
        .await
        .expect("didClose notification");
    assert!(did_close_response.is_none(), "didClose is a notification");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let saw_client_cancel = counters.iter().any(|(key, value)| {
        key.starts_with("intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_")
            && key.contains("reason_client_cancel")
            && metric_number(value) > 0.0
    });
    assert!(
        saw_client_cancel,
        "didClose must record diagnostics pipeline client_cancel for removed in-flight tasks"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p6_idle_heavy_supersession_is_reported_for_burst_did_change() {
    struct DiagnosticsDebounceEnvGuard {
        previous_debounce_ms: Option<String>,
    }

    impl DiagnosticsDebounceEnvGuard {
        fn new() -> Self {
            Self {
                previous_debounce_ms: std::env::var("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS").ok(),
            }
        }

        fn apply(&self, debounce_ms: u64) {
            std::env::set_var("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", debounce_ms.to_string());
            bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
        }
    }

    impl Drop for DiagnosticsDebounceEnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.previous_debounce_ms {
                std::env::set_var("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", value);
            } else {
                std::env::remove_var("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS");
            }
            bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
        }
    }

    let debounce_env_guard = DiagnosticsDebounceEnvGuard::new();
    debounce_env_guard.apply(500);

    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let settings = DidChangeConfigurationParams {
        settings: serde_json::json!({
            "bsl": {
                "hover": {
                    "detailLevel": "full",
                    "maxMethods": 10,
                    "maxProperties": 5,
                    "showCertainty": true
                },
                "diagnostics": {
                    "detailLevel": "standard",
                    "showHints": true
                },
                "formatting": {
                    "enabled": false,
                    "indentSize": 4
                },
                "typeHints": {
                    "enabled": true,
                    "showVariableTypes": true,
                    "showReturnTypes": true,
                    "showUnionDetails": true,
                    "minCertainty": 0.5
                },
                "codeActions": {
                    "enabled": false
                },
                "enableFlowSensitive": true
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_response = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_response.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///idle-heavy-supersession.bsl").expect("test uri");

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Procedure Test()\nEndProcedure".to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let did_change_v2 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test(\nEndProcedure".to_string(),
        }],
    };
    let did_change_req_v2 = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change_v2).expect("DidChangeTextDocumentParams"))
        .finish();
    let did_change_response_v2 = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req_v2)
        .await
        .expect("didChange v2 notification");
    assert!(
        did_change_response_v2.is_none(),
        "didChange is a notification"
    );

    tokio::time::sleep(Duration::from_millis(20)).await;

    let did_change_v3 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 3,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test()\nEndProcedure".to_string(),
        }],
    };
    let did_change_req_v3 = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change_v3).expect("DidChangeTextDocumentParams"))
        .finish();
    let did_change_response_v3 = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req_v3)
        .await
        .expect("didChange v3 notification");
    assert!(
        did_change_response_v3.is_none(),
        "didChange is a notification"
    );

    tokio::time::sleep(Duration::from_millis(800)).await;

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let saw_idle_heavy_superseded = counters.iter().any(|(key, value)| {
        key.starts_with(
            "intellisense_v2_diagnostics_pipeline_total_origin_lsp_trigger_idle_profile_idle_heavy_reason_superseded_",
        ) && metric_number(value) > 0.0
    });
    assert!(
        saw_idle_heavy_superseded,
        "burst didChange must produce superseded cancellation in idle_heavy profile"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_completion_after_did_change_does_not_hang() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///test_p7.bsl").expect("test uri");

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Procedure Test()\nEndProcedure".to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "Procedure Test()\n\t// p7\nEndProcedure".to_string(),
        }],
    };
    let did_change_req = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
        .finish();
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req)
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 0,
                character: 0,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };
    let completion_req = Request::build("textDocument/completion")
        .id(2)
        .params(serde_json::to_value(completion_params).expect("CompletionParams"))
        .finish();

    let completion_response = tokio::time::timeout(
        tokio::time::Duration::from_secs(2),
        service.ready().await.unwrap().call(completion_req),
    )
    .await
    .expect("completion request timeout")
    .expect("completion request");

    assert!(
        completion_response.is_some(),
        "completion should return a response"
    );

    drain_task.abort();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn p28_cancel_request_stops_completion_and_prevents_late_publish() {
    fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK);
    let _delay_guard = EnvVarGuard::set("BSL_TEST_COMPLETION_DELAY_MS", "40");

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
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let uri = Url::parse("file:///test_p28_cancel_request.bsl").expect("test uri");
    let mut base_text = String::from("Процедура Тест()\n    ЛокМассив = Новый Массив;\n");
    for value in 0..800 {
        base_text.push_str(&format!("    ЛокМассив.Добавить({value});\n"));
    }
    base_text.push_str("    ЛокМассив.\nКонецПроцедуры\n");
    let completion_line = 802_u32;
    let completion_character = "    ЛокМассив.".encode_utf16().count() as u32;

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: base_text.clone(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .as_ref()
        .cloned()
        .expect("server instance");
    let file_id = server.get_or_create_file_id_v2(&uri).await;

    let mut observed_cancelled_completion = false;
    for attempt in 0..8_i32 {
        let version = attempt + 2;
        let changed_text = format!("{base_text}// attempt {attempt}\n");
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: changed_text,
            }],
        };
        let did_change_req = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
            .finish();
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req)
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");

        let request_id = 100_i64 + i64::from(attempt);
        let completion_params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(completion_line, completion_character),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: Some("__bsl_shadow_internal__:46".to_string()),
            }),
        };
        let completion_req = Request::build("textDocument/completion")
            .id(request_id)
            .params(serde_json::to_value(completion_params).expect("CompletionParams"))
            .finish();
        let completion_future = service.ready().await.unwrap().call(completion_req);
        let completion_task = tokio::spawn(completion_future);
        let expected_epoch = u64::try_from(attempt + 1).expect("positive epoch");
        let mut before_state = None;
        for _ in 0..100 {
            if let Some((file_seq, epoch)) =
                server.completion_dispatcher_v2.debug_state(file_id).await
            {
                if epoch >= expected_epoch {
                    before_state = Some((file_seq, epoch));
                    break;
                }
            }
            tokio::task::yield_now().await;
        }
        let (before_file_seq, before_epoch) = before_state.expect("dispatcher state before cancel");
        let request_id_string = request_id.to_string();
        let mut registration_present = false;
        for _ in 0..20 {
            if server
                .completion_cancellation_registry_v2
                .get(&request_id_string)
                .is_some()
            {
                registration_present = true;
                break;
            }
            tokio::task::yield_now().await;
        }

        let cancel_req = Request::build("$/cancelRequest")
            .params(serde_json::json!({ "id": request_id }))
            .finish();
        let cancel_response = service
            .call(cancel_req)
            .await
            .expect("cancel request notification");
        assert!(cancel_response.is_none(), "cancel is a notification");

        let mut cancel_event_observed = false;
        for _ in 0..20 {
            if let Some((after_file_seq, after_epoch)) =
                server.completion_dispatcher_v2.debug_state(file_id).await
            {
                if after_file_seq > before_file_seq && after_epoch >= before_epoch {
                    cancel_event_observed = true;
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        let completion_response =
            tokio::time::timeout(tokio::time::Duration::from_secs(5), completion_task)
                .await
                .expect("completion request timeout")
                .expect("completion task join")
                .expect("completion request")
                .expect("completion response");
        let completion_value =
            serde_json::to_value(&completion_response).expect("serialize completion");
        let completion_is_safe =
            if let Some(completion_result) = completion_value.get("result").cloned() {
                let completion_lsp: Option<CompletionResponse> =
                    serde_json::from_value(completion_result).expect("parse completion result");
                completion_lsp
                    .as_ref()
                    .is_some_and(completion_response_incomplete_empty)
            } else if let Some(error) = completion_value.get("error") {
                let error_code = error
                    .get("code")
                    .and_then(|value| value.as_i64())
                    .unwrap_or_default();
                let error_message = error
                    .get("message")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                error_code == -32800 || error_message.contains("cancel")
            } else {
                false
            };
        if registration_present && cancel_event_observed && completion_is_safe {
            observed_cancelled_completion = true;
            break;
        }
    }

    assert!(
        observed_cancelled_completion,
        "expected $/cancelRequest to enqueue Cancel(request_id) and avoid late completion publish"
    );

    drain_task.abort();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn p28_cancel_request_before_first_poll_honors_cancellation() {
    fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }

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
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let uri =
        Url::parse("file:///test_p28_cancel_request_before_first_poll.bsl").expect("test uri");
    let text = "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(2, "    ЛокМассив.".encode_utf16().count() as u32),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: Some("__bsl_shadow_internal__:46".to_string()),
        }),
    };
    let completion_req = Request::build("textDocument/completion")
        .id(701_i64)
        .params(serde_json::to_value(completion_params).expect("CompletionParams"))
        .finish();
    let completion_future = service.ready().await.unwrap().call(completion_req);

    let cancel_req = Request::build("$/cancelRequest")
        .params(serde_json::json!({ "id": 701_i64 }))
        .finish();
    let cancel_response = service
        .ready()
        .await
        .unwrap()
        .call(cancel_req)
        .await
        .expect("cancel request notification");
    assert!(cancel_response.is_none(), "cancel is a notification");

    let completion_response =
        tokio::time::timeout(tokio::time::Duration::from_secs(5), completion_future)
            .await
            .expect("completion request timeout")
            .expect("completion request")
            .expect("completion response");
    let completion_value =
        serde_json::to_value(&completion_response).expect("serialize completion");
    let completion_is_safe =
        if let Some(completion_result) = completion_value.get("result").cloned() {
            let completion_lsp: Option<CompletionResponse> =
                serde_json::from_value(completion_result).expect("parse completion result");
            completion_lsp
                .as_ref()
                .is_some_and(completion_response_incomplete_empty)
        } else if let Some(error) = completion_value.get("error") {
            let error_code = error
                .get("code")
                .and_then(|value| value.as_i64())
                .unwrap_or_default();
            let error_message = error
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            error_code == -32800 || error_message.contains("cancel")
        } else {
            false
        };

    assert!(
        completion_is_safe,
        "expected $/cancelRequest before first poll to prevent late completion publish"
    );

    drain_task.abort();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn p28_newer_completion_proactively_cancels_older_active_completion_on_same_file() {
    fn completion_response_incomplete_empty(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::List(list) => list.is_incomplete && list.items.is_empty(),
            CompletionResponse::Array(items) => items.is_empty(),
        }
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _delay_guard = EnvVarGuard::set("BSL_TEST_COMPLETION_DELAY_MS", "80");

    let fixture =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let (service, drain_task, server, uri, _file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///test_p28_active_completion_supersession.bsl",
    )
    .await;
    let mut service = crate::server::request_context::RequestContextService::new(service);
    let position = find_utf16_position_after_marker(fixture, "ЛокМассив.");

    let first_req = Request::build("textDocument/completion")
        .id(28001)
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
        .finish();
    let first_future = service.ready().await.unwrap().call(first_req);
    let first_task = tokio::spawn(first_future);

    for _ in 0..40 {
        if server
            .completion_cancellation_registry_v2
            .get("28001")
            .is_some()
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        server
            .completion_cancellation_registry_v2
            .get("28001")
            .is_some(),
        "first completion request must register cancellation token before newer request arrives"
    );

    let second_req = Request::build("textDocument/completion")
        .id(28002)
        .params(
            serde_json::to_value(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: None,
                }),
            })
            .expect("CompletionParams"),
        )
        .finish();
    let second_response = service
        .ready()
        .await
        .unwrap()
        .call(second_req)
        .await
        .expect("second completion request")
        .expect("second completion response");

    for _ in 0..40 {
        if server
            .completion_cancellation_registry_v2
            .get("28001")
            .is_none()
        {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        server
            .completion_cancellation_registry_v2
            .get("28001")
            .is_none(),
        "newer completion must proactively cancel the older active completion request on the same file"
    );

    let first_response = first_task
        .await
        .expect("first completion join")
        .expect("first completion request")
        .expect("first completion response");
    let first_value = serde_json::to_value(&first_response).expect("serialize first response");
    let first_result = first_value
        .get("result")
        .cloned()
        .expect("first completion result field");
    let first_completion: Option<CompletionResponse> =
        serde_json::from_value(first_result).expect("parse first completion result");
    assert!(
        first_completion
            .as_ref()
            .is_some_and(completion_response_incomplete_empty),
        "older superseded completion must resolve to bounded incomplete empty response, response={first_completion:?}"
    );

    let second_value = serde_json::to_value(&second_response).expect("serialize second response");
    let second_result = second_value
        .get("result")
        .cloned()
        .expect("second completion result field");
    let second_completion: Option<CompletionResponse> =
        serde_json::from_value(second_result).expect("parse second completion result");
    assert!(
        second_completion.is_some(),
        "newer completion request must still produce a response"
    );

    drain_task.abort();
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn p29_completion_mode_matrix_parity_on_fixed_revision() {
    const CHANGE_ID: &str = "refactor-v2-completion-event-driven-pipeline";
    const ITERATIONS: usize = 40;
    const MAX_USER_FACING_DRIFT_RATE: f64 = 0.01;
    const MAX_SHADOW_PARITY_DRIFT_RATE: f64 = 0.01;
    const MIN_FIRST_TRIGGER_SUCCESS_RATE: f64 = 0.99;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct CompletionFingerprint {
        is_incomplete: bool,
        labels: Vec<String>,
    }

    #[derive(Debug, Clone, Copy)]
    struct ModeScenario {
        name: &'static str,
        completion_mode: &'static str,
        canary_percent: u8,
    }

    #[derive(Debug)]
    struct ModeOutcome {
        name: String,
        completion_p95_ms: f64,
        completion_p99_ms: f64,
        completion_total: u64,
        first_trigger_success_rate: f64,
        parity_drift_rate: f64,
        legacy_stage_total: u64,
        shadow_stage_total: u64,
        event_driven_stage_total: u64,
        dot_fingerprints: Vec<CompletionFingerprint>,
        invoked_fingerprints: Vec<CompletionFingerprint>,
    }

    struct CompletionModeEnvGuard {
        previous_mode: Option<String>,
        previous_canary_percent: Option<String>,
    }

    impl CompletionModeEnvGuard {
        fn new() -> Self {
            Self {
                previous_mode: std::env::var("BSL_INTELLISENSE_V2_COMPLETION_MODE").ok(),
                previous_canary_percent: std::env::var(
                    "BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT",
                )
                .ok(),
            }
        }

        fn apply(&self, completion_mode: &str, canary_percent: u8) {
            std::env::set_var("BSL_INTELLISENSE_V2_COMPLETION_MODE", completion_mode);
            std::env::set_var(
                "BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT",
                canary_percent.to_string(),
            );
            bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
        }
    }

    impl Drop for CompletionModeEnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.previous_mode {
                std::env::set_var("BSL_INTELLISENSE_V2_COMPLETION_MODE", value);
            } else {
                std::env::remove_var("BSL_INTELLISENSE_V2_COMPLETION_MODE");
            }
            if let Some(value) = &self.previous_canary_percent {
                std::env::set_var("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT", value);
            } else {
                std::env::remove_var("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT");
            }
            bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
        }
    }

    fn metric_as_f64(value: Option<&serde_json::Value>) -> f64 {
        value
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .unwrap_or(0.0)
    }

    fn completion_items_count(response: &CompletionResponse) -> usize {
        match response {
            CompletionResponse::Array(items) => items.len(),
            CompletionResponse::List(list) => list.items.len(),
        }
    }

    fn completion_fingerprint(response: &CompletionResponse) -> CompletionFingerprint {
        let (is_incomplete, labels) = match response {
            CompletionResponse::Array(items) => (
                false,
                items
                    .iter()
                    .map(|item| item.label.clone())
                    .collect::<BTreeSet<_>>(),
            ),
            CompletionResponse::List(list) => (
                list.is_incomplete,
                list.items
                    .iter()
                    .map(|item| item.label.clone())
                    .collect::<BTreeSet<_>>(),
            ),
        };
        CompletionFingerprint {
            is_incomplete,
            labels: labels.into_iter().collect(),
        }
    }

    fn sum_counters_by_prefix(
        counters: &serde_json::Map<String, serde_json::Value>,
        prefix: &str,
    ) -> u64 {
        counters
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(_, value)| value.as_u64().unwrap_or(0))
            .sum()
    }

    fn completion_stage_mode_total(
        counters: &serde_json::Map<String, serde_json::Value>,
        mode: &str,
    ) -> u64 {
        counters
            .iter()
            .filter(|(key, _)| {
                key.starts_with("intellisense_v2_drilldown_stage_total_")
                    && key.contains("_origin_lsp_")
                    && key.contains("_operation_completion_")
                    && key.contains(&format!("_mode_{mode}"))
            })
            .map(|(_, value)| value.as_u64().unwrap_or(0))
            .sum()
    }

    async fn run_mode_scenario(scenario: ModeScenario, iterations: usize) -> ModeOutcome {
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
            .as_ref()
            .cloned()
            .expect("server instance");
        prime_server_with_syntax_helper_deps(&server).await;

        let uri =
            Url::parse(&format!("file:///test_p29_mode_{}.bsl", scenario.name)).expect("test uri");
        let text = concat!(
            "Процедура Тест()\n",
            "    ЛокМассив = Новый Массив;\n",
            "    ЛокМассив.\n",
            "КонецПроцедуры\n"
        );
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.to_string(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");
        let member_character = "    ЛокМассив."
            .chars()
            .map(|ch| ch.len_utf16())
            .sum::<usize>() as u32;

        let mut dot_fingerprints = Vec::with_capacity(iterations);
        let mut invoked_fingerprints = Vec::with_capacity(iterations);
        let mut first_trigger_success_total = 0_u64;

        for _ in 0..iterations {
            let dot_completion = server
                .completion(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position: Position::new(2, member_character),
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: Some(CompletionContext {
                        trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                        trigger_character: Some(".".to_string()),
                    }),
                })
                .await
                .expect("dot completion request")
                .expect("dot completion response");
            if completion_items_count(&dot_completion) > 0 {
                first_trigger_success_total += 1;
            }
            dot_fingerprints.push(completion_fingerprint(&dot_completion));

            let invoked_completion = server
                .completion(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position: Position::new(2, member_character),
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: Some(CompletionContext {
                        trigger_kind: CompletionTriggerKind::INVOKED,
                        trigger_character: None,
                    }),
                })
                .await
                .expect("invoked completion request")
                .expect("invoked completion response");
            invoked_fingerprints.push(completion_fingerprint(&invoked_completion));
        }

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");
        let completion_hist = histograms
            .get("completion_duration_ms")
            .and_then(|value| value.as_object())
            .expect("completion duration histogram");
        let completion_p95_ms = metric_as_f64(completion_hist.get("p95"));
        let completion_p99_ms = metric_as_f64(completion_hist.get("p99"));
        let completion_total = counters
            .get("completion_total")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let parity_pairs_total = (iterations as u64) * 2;
        let parity_drift_total = sum_counters_by_prefix(
            counters,
            "intellisense_v2_completion_parity_drift_total_mode_",
        );
        let parity_drift_rate = parity_drift_total as f64 / parity_pairs_total.max(1) as f64;
        let first_trigger_success_rate =
            first_trigger_success_total as f64 / iterations.max(1) as f64;
        let legacy_stage_total = completion_stage_mode_total(counters, "legacy");
        let shadow_stage_total = completion_stage_mode_total(counters, "shadow");
        let event_driven_stage_total = completion_stage_mode_total(counters, "event_driven");

        drain_task.abort();

        ModeOutcome {
            name: scenario.name.to_string(),
            completion_p95_ms,
            completion_p99_ms,
            completion_total,
            first_trigger_success_rate,
            parity_drift_rate,
            legacy_stage_total,
            shadow_stage_total,
            event_driven_stage_total,
            dot_fingerprints,
            invoked_fingerprints,
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let env_guard = CompletionModeEnvGuard::new();

    let scenarios = [
        ModeScenario {
            name: "off",
            completion_mode: "off",
            canary_percent: 0,
        },
        ModeScenario {
            name: "shadow",
            completion_mode: "shadow",
            canary_percent: 0,
        },
        ModeScenario {
            name: "canary",
            completion_mode: "canary",
            canary_percent: 100,
        },
        ModeScenario {
            name: "on",
            completion_mode: "on",
            canary_percent: 0,
        },
    ];

    let mut outcomes = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        env_guard.apply(scenario.completion_mode, scenario.canary_percent);
        let outcome = run_mode_scenario(scenario, ITERATIONS).await;
        assert!(
            outcome.first_trigger_success_rate >= MIN_FIRST_TRIGGER_SUCCESS_RATE,
            "mode={} first-trigger success rate={:.4} < {:.4}",
            outcome.name,
            outcome.first_trigger_success_rate,
            MIN_FIRST_TRIGGER_SUCCESS_RATE
        );
        outcomes.push(outcome);
    }

    let off_outcome = outcomes
        .iter()
        .find(|outcome| outcome.name == "off")
        .expect("off mode outcome");
    let mut drift_by_mode = serde_json::Map::new();
    for outcome in outcomes.iter().filter(|outcome| outcome.name != "off") {
        let dot_mismatch_total = outcome
            .dot_fingerprints
            .iter()
            .zip(off_outcome.dot_fingerprints.iter())
            .filter(|(actual, expected)| actual != expected)
            .count() as u64;
        let invoked_mismatch_total = outcome
            .invoked_fingerprints
            .iter()
            .zip(off_outcome.invoked_fingerprints.iter())
            .filter(|(actual, expected)| actual != expected)
            .count() as u64;
        let mismatch_total = dot_mismatch_total + invoked_mismatch_total;
        let mismatch_rate = mismatch_total as f64 / ((ITERATIONS * 2) as f64);

        drift_by_mode.insert(
            outcome.name.clone(),
            serde_json::json!({
                "mismatch_total": mismatch_total,
                "mismatch_rate": mismatch_rate,
                "dot_mismatch_total": dot_mismatch_total,
                "invoked_mismatch_total": invoked_mismatch_total,
            }),
        );
        assert!(
            mismatch_rate <= MAX_USER_FACING_DRIFT_RATE,
            "mode={} user-facing completion drift rate={:.4} > {:.4}",
            outcome.name,
            mismatch_rate,
            MAX_USER_FACING_DRIFT_RATE
        );
    }

    let shadow_outcome = outcomes
        .iter()
        .find(|outcome| outcome.name == "shadow")
        .expect("shadow mode outcome");
    let canary_outcome = outcomes
        .iter()
        .find(|outcome| outcome.name == "canary")
        .expect("canary mode outcome");
    let on_outcome = outcomes
        .iter()
        .find(|outcome| outcome.name == "on")
        .expect("on mode outcome");

    assert!(
        off_outcome.legacy_stage_total > 0
            && off_outcome.shadow_stage_total == 0
            && off_outcome.event_driven_stage_total == 0,
        "off mode stage routing must be strictly legacy: {:?}",
        (
            off_outcome.legacy_stage_total,
            off_outcome.shadow_stage_total,
            off_outcome.event_driven_stage_total
        )
    );
    assert!(
        shadow_outcome.legacy_stage_total > 0
            && shadow_outcome.shadow_stage_total > 0
            && shadow_outcome.event_driven_stage_total == 0,
        "shadow mode must route user-facing via legacy and run shadow pipeline: {:?}",
        (
            shadow_outcome.legacy_stage_total,
            shadow_outcome.shadow_stage_total,
            shadow_outcome.event_driven_stage_total
        )
    );
    assert!(
        shadow_outcome.parity_drift_rate <= MAX_SHADOW_PARITY_DRIFT_RATE,
        "shadow mode parity drift rate={:.4} > {:.4}",
        shadow_outcome.parity_drift_rate,
        MAX_SHADOW_PARITY_DRIFT_RATE
    );
    assert!(
        canary_outcome.event_driven_stage_total > 0
            && canary_outcome.legacy_stage_total == 0
            && canary_outcome.shadow_stage_total == 0,
        "canary(100) mode must route completion via event-driven only: {:?}",
        (
            canary_outcome.legacy_stage_total,
            canary_outcome.shadow_stage_total,
            canary_outcome.event_driven_stage_total
        )
    );
    assert!(
        on_outcome.event_driven_stage_total > 0
            && on_outcome.legacy_stage_total == 0
            && on_outcome.shadow_stage_total == 0,
        "on mode must route completion via event-driven only: {:?}",
        (
            on_outcome.legacy_stage_total,
            on_outcome.shadow_stage_total,
            on_outcome.event_driven_stage_total
        )
    );

    let mut modes_report = serde_json::Map::new();
    for outcome in &outcomes {
        modes_report.insert(
            outcome.name.clone(),
            serde_json::json!({
                "completion_total": outcome.completion_total,
                "completion_p95_ms": outcome.completion_p95_ms,
                "completion_p99_ms": outcome.completion_p99_ms,
                "first_trigger_success_rate": outcome.first_trigger_success_rate,
                "parity_drift_rate": outcome.parity_drift_rate,
                "stage_totals": {
                    "legacy": outcome.legacy_stage_total,
                    "shadow": outcome.shadow_stage_total,
                    "event_driven": outcome.event_driven_stage_total
                }
            }),
        );
    }
    let report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": "p29_completion_mode_matrix_parity_on_fixed_revision",
        "iterations": ITERATIONS,
        "thresholds": {
            "max_user_facing_drift_rate": MAX_USER_FACING_DRIFT_RATE,
            "max_shadow_parity_drift_rate": MAX_SHADOW_PARITY_DRIFT_RATE,
            "min_first_trigger_success_rate": MIN_FIRST_TRIGGER_SUCCESS_RATE
        },
        "mode_user_facing_drift_vs_off": drift_by_mode,
        "modes": serde_json::Value::Object(modes_report),
    });
    let report_path = std::env::var("BSL_V2_COMPLETION_MODE_MATRIX_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!("{CHANGE_ID}-mode-parity-matrix.json"))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for completion mode matrix report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report)
            .expect("failed to serialize completion mode matrix report"),
    )
    .expect("failed to write completion mode matrix report");
    println!("v2_completion_mode_matrix_report={}", report_path.display());
}

#[tokio::test]
async fn p30_backpressure_fairness_interactive_vs_background_no_starvation() {
    const CHANGE_ID: &str = "refactor-v2-completion-event-driven-pipeline";
    const INTERACTIVE_PROBE_TOTAL: usize = 24;
    const BACKGROUND_BURST_TOTAL: usize = 24;
    const INTERACTIVE_BURST_TOTAL: usize = 32;
    const BACKGROUND_PROBE_TOTAL: usize = 16;
    const ROUND_TIMEOUT_SECS: u64 = 30;
    const MAX_REQUEST_LATENCY_MS: f64 = 10_000.0;

    async fn run_hover_requests(
        server: BslLanguageServer,
        uri: Url,
        position: Position,
        total: usize,
    ) -> (u64, f64) {
        let mut success_total = 0_u64;
        let mut max_latency_ms = 0.0_f64;
        for _ in 0..total {
            let started = Instant::now();
            let response = server
                .hover(HoverParams {
                    text_document_position_params: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position,
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await;
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            max_latency_ms = max_latency_ms.max(elapsed_ms);
            if response.is_ok() {
                success_total += 1;
            }
        }
        (success_total, max_latency_ms)
    }

    async fn run_hover_burst(
        server: BslLanguageServer,
        uri: Url,
        position: Position,
        total: usize,
    ) -> (u64, f64) {
        let mut handles = Vec::with_capacity(total);
        for _ in 0..total {
            let server = server.clone();
            let uri = uri.clone();
            handles.push(tokio::spawn(async move {
                let started = Instant::now();
                let response = server
                    .hover(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri },
                            position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .await;
                (response.is_ok(), started.elapsed().as_secs_f64() * 1000.0)
            }));
        }
        let mut success_total = 0_u64;
        let mut max_latency_ms = 0.0_f64;
        for handle in handles {
            let (ok, latency_ms) = handle.await.expect("hover burst task join");
            if ok {
                success_total += 1;
            }
            max_latency_ms = max_latency_ms.max(latency_ms);
        }
        (success_total, max_latency_ms)
    }

    async fn run_workspace_symbol_requests(
        server: BslLanguageServer,
        query: String,
        total: usize,
    ) -> (u64, f64) {
        let mut success_total = 0_u64;
        let mut max_latency_ms = 0.0_f64;
        for _ in 0..total {
            let started = Instant::now();
            let response = server
                .symbol(WorkspaceSymbolParams {
                    query: query.clone(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await;
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            max_latency_ms = max_latency_ms.max(elapsed_ms);
            if response.is_ok() {
                success_total += 1;
            }
        }
        (success_total, max_latency_ms)
    }

    async fn run_workspace_symbol_burst(
        server: BslLanguageServer,
        query: String,
        total: usize,
    ) -> (u64, f64) {
        let mut handles = Vec::with_capacity(total);
        for _ in 0..total {
            let server = server.clone();
            let query = query.clone();
            handles.push(tokio::spawn(async move {
                let started = Instant::now();
                let response = server
                    .symbol(WorkspaceSymbolParams {
                        query,
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    })
                    .await;
                (response.is_ok(), started.elapsed().as_secs_f64() * 1000.0)
            }));
        }
        let mut success_total = 0_u64;
        let mut max_latency_ms = 0.0_f64;
        for handle in handles {
            let (ok, latency_ms) = handle.await.expect("workspace_symbol burst task join");
            if ok {
                success_total += 1;
            }
            max_latency_ms = max_latency_ms.max(latency_ms);
        }
        (success_total, max_latency_ms)
    }

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

    let mut primary_uri: Option<Url> = None;
    for index in 0..8_u32 {
        let uri = Url::parse(&format!("file:///test_p30_fairness_{index}.bsl")).expect("uri");
        if primary_uri.is_none() {
            primary_uri = Some(uri.clone());
        }
        let mut text = format!("Процедура Тест{index}()\n    ЛокПерем = Новый Массив;\n");
        for value in 0..120_u32 {
            text.push_str(&format!("    ЛокПерем.Добавить({value});\n"));
        }
        text.push_str("    Возврат ЛокПерем.Количество();\nКонецПроцедуры\n");
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text,
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");
    }
    let primary_uri = primary_uri.expect("primary uri");
    let hover_position = Position::new(2, 8);

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .as_ref()
        .cloned()
        .expect("server instance");

    let (warm_interactive_success, _) =
        run_hover_requests(server.clone(), primary_uri.clone(), hover_position, 2).await;
    assert!(
        warm_interactive_success > 0,
        "warm-up interactive requests should succeed"
    );
    let (warm_background_success, _) =
        run_workspace_symbol_requests(server.clone(), "Тест".to_string(), 2).await;
    assert!(
        warm_background_success > 0,
        "warm-up background requests should succeed"
    );

    let round_a_background = tokio::spawn(run_workspace_symbol_burst(
        server.clone(),
        "Тест".to_string(),
        BACKGROUND_BURST_TOTAL,
    ));
    let round_a_interactive = tokio::spawn(run_hover_requests(
        server.clone(),
        primary_uri.clone(),
        hover_position,
        INTERACTIVE_PROBE_TOTAL,
    ));
    let (round_a_background_success, round_a_background_max_ms) =
        tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_a_background)
            .await
            .expect("background burst timeout in round A")
            .expect("background burst join in round A");
    let (round_a_interactive_success, round_a_interactive_max_ms) =
        tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_a_interactive)
            .await
            .expect("interactive probe timeout in round A")
            .expect("interactive probe join in round A");

    let round_b_interactive = tokio::spawn(run_hover_burst(
        server.clone(),
        primary_uri.clone(),
        hover_position,
        INTERACTIVE_BURST_TOTAL,
    ));
    let round_b_background = tokio::spawn(run_workspace_symbol_requests(
        server.clone(),
        "Тест".to_string(),
        BACKGROUND_PROBE_TOTAL,
    ));
    let (round_b_interactive_success, round_b_interactive_max_ms) =
        tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_b_interactive)
            .await
            .expect("interactive burst timeout in round B")
            .expect("interactive burst join in round B");
    let (round_b_background_success, round_b_background_max_ms) =
        tokio::time::timeout(Duration::from_secs(ROUND_TIMEOUT_SECS), round_b_background)
            .await
            .expect("background probe timeout in round B")
            .expect("background probe join in round B");

    assert_eq!(
        round_a_interactive_success, INTERACTIVE_PROBE_TOTAL as u64,
        "interactive requests must progress under background burst"
    );
    assert_eq!(
        round_a_background_success, BACKGROUND_BURST_TOTAL as u64,
        "background burst must complete without starvation"
    );
    assert_eq!(
        round_b_interactive_success, INTERACTIVE_BURST_TOTAL as u64,
        "interactive burst must complete under mixed load"
    );
    assert_eq!(
        round_b_background_success, BACKGROUND_PROBE_TOTAL as u64,
        "background probe must progress under interactive burst"
    );
    for (name, value) in [
        ("round_a_background_max_ms", round_a_background_max_ms),
        ("round_a_interactive_max_ms", round_a_interactive_max_ms),
        ("round_b_background_max_ms", round_b_background_max_ms),
        ("round_b_interactive_max_ms", round_b_interactive_max_ms),
    ] {
        assert!(
            value <= MAX_REQUEST_LATENCY_MS,
            "{name} exceeded bounded latency: {value:.2}ms > {MAX_REQUEST_LATENCY_MS:.2}ms"
        );
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let interactive_queue_wait_total = counters
        .get("intellisense_v2_runtime_queue_wait_interactive_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let background_queue_wait_total = counters
        .get("intellisense_v2_runtime_queue_wait_background_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let interactive_exec_total = counters
        .get("intellisense_v2_runtime_exec_interactive_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let background_exec_total = counters
        .get("intellisense_v2_runtime_exec_background_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    assert!(
        interactive_queue_wait_total > 0,
        "interactive queue-wait counter must be present under mixed load"
    );
    assert!(
        background_queue_wait_total > 0,
        "background queue-wait counter must be present under mixed load"
    );
    assert!(
        interactive_exec_total > 0,
        "interactive exec counter must be present under mixed load"
    );
    assert!(
        background_exec_total > 0,
        "background exec counter must be present under mixed load"
    );

    let report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": "p30_backpressure_fairness_interactive_vs_background_no_starvation",
        "thresholds": {
            "round_timeout_secs": ROUND_TIMEOUT_SECS,
            "max_request_latency_ms": MAX_REQUEST_LATENCY_MS,
        },
        "rounds": {
            "background_burst_vs_interactive_probe": {
                "interactive_total": INTERACTIVE_PROBE_TOTAL,
                "interactive_success": round_a_interactive_success,
                "interactive_max_latency_ms": round_a_interactive_max_ms,
                "background_total": BACKGROUND_BURST_TOTAL,
                "background_success": round_a_background_success,
                "background_max_latency_ms": round_a_background_max_ms,
            },
            "interactive_burst_vs_background_probe": {
                "interactive_total": INTERACTIVE_BURST_TOTAL,
                "interactive_success": round_b_interactive_success,
                "interactive_max_latency_ms": round_b_interactive_max_ms,
                "background_total": BACKGROUND_PROBE_TOTAL,
                "background_success": round_b_background_success,
                "background_max_latency_ms": round_b_background_max_ms,
            }
        },
        "metrics": {
            "intellisense_v2_runtime_queue_wait_interactive_total": interactive_queue_wait_total,
            "intellisense_v2_runtime_queue_wait_background_total": background_queue_wait_total,
            "intellisense_v2_runtime_exec_interactive_total": interactive_exec_total,
            "intellisense_v2_runtime_exec_background_total": background_exec_total,
        },
        "pass": true
    });
    let report_path = std::env::var("BSL_V2_COMPLETION_FAIRNESS_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!(
                    "{CHANGE_ID}-fairness-interactive-vs-background.json"
                ))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for completion fairness report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report)
            .expect("failed to serialize completion fairness report"),
    )
    .expect("failed to write completion fairness report");
    println!("v2_completion_fairness_report={}", report_path.display());

    drain_task.abort();
}

#[tokio::test]
async fn p30_cross_file_did_change_parallel_completion_no_global_lock_bottleneck() {
    const CHANGE_ID: &str = "add-performance-first-ai-engineering-guardrails";
    const DID_CHANGE_BURST_PER_FILE: u32 = 8;
    const COMPLETION_BURST_PER_FILE: usize = 20;
    const REQUEST_TIMEOUT_SECS: u64 = 30;
    const MAX_COMPLETION_LATENCY_MS: f64 = 10_000.0;
    const MAX_QUEUE_WAIT_P95_MS: f64 = 2_000.0;
    const MIN_CONCURRENCY_GAIN: f64 = 1.10;
    const MIN_AVG_LATENCY_FOR_GAIN_CHECK_MS: f64 = 2.0;

    fn completion_items_count(response: &CompletionResponse) -> usize {
        match response {
            CompletionResponse::Array(items) => items.len(),
            CompletionResponse::List(list) => list.items.len(),
        }
    }

    fn completion_is_incomplete(response: &CompletionResponse) -> bool {
        match response {
            CompletionResponse::Array(_) => false,
            CompletionResponse::List(list) => list.is_incomplete,
        }
    }

    fn metric_as_f64(value: Option<&serde_json::Value>) -> f64 {
        value
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .unwrap_or(0.0)
    }

    fn build_document_text(function_name: &str) -> String {
        format!(
            "Процедура {function_name}()\n    ДляCompletion = (Новый Массив()).\nКонецПроцедуры\n"
        )
    }

    struct DocumentState {
        uri: Url,
        version: i32,
        text: String,
    }

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
    {
        let server = server_holder
            .lock()
            .expect("server holder lock")
            .as_ref()
            .cloned()
            .expect("server instance");
        prime_server_with_syntax_helper_deps(&server).await;
    }

    let mut documents = vec![
        DocumentState {
            uri: Url::parse("file:///test_p30_cross_file_a.bsl").expect("document uri A"),
            version: 1,
            text: build_document_text("ТестA"),
        },
        DocumentState {
            uri: Url::parse("file:///test_p30_cross_file_b.bsl").expect("document uri B"),
            version: 1,
            text: build_document_text("ТестB"),
        },
    ];

    for document in &documents {
        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: document.uri.clone(),
                language_id: "bsl".to_string(),
                version: document.version,
                text: document.text.clone(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");
    }

    for burst_idx in 0..DID_CHANGE_BURST_PER_FILE {
        for (doc_idx, document) in documents.iter_mut().enumerate() {
            document.version += 1;
            document
                .text
                .push_str(&format!("// churn doc={doc_idx} step={burst_idx}\n"));
            let did_change = DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: document.uri.clone(),
                    version: document.version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: document.text.clone(),
                }],
            };
            let did_change_req = Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
                .finish();
            let did_change_response = service
                .ready()
                .await
                .unwrap()
                .call(did_change_req)
                .await
                .expect("didChange notification");
            assert!(did_change_response.is_none(), "didChange is a notification");
        }
    }

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .as_ref()
        .cloned()
        .expect("server instance");
    for document in &documents {
        let file_id = server.get_or_create_file_id_v2(&document.uri).await;
        assert!(
            server
                .analysis_v2
                .wait_for_file_version(file_id, document.version)
                .await,
            "analysis runtime must catch up to didChange burst for {}",
            document.uri
        );
        wait_for_type_index_precompute_completion(&server, file_id).await;
    }
    let mut owner_hint_type_names = Vec::with_capacity(documents.len());
    let completion_position =
        find_utf16_position_after_marker(&documents[0].text, "(Новый Массив()).");
    for document in &documents {
        let file_id = server.get_or_create_file_id_v2(&document.uri).await;
        let analysis = server.analysis_v2.snapshot().await;
        owner_hint_type_names.push((
            document.uri.to_string(),
            bsl_runtime::application::completion_member_access_owner_type_hints_from_analysis(
                &analysis,
                file_id,
                &document.text,
                completion_position.line,
                completion_position.character,
            )
            .into_iter()
            .map(|hint| hint.type_name())
            .collect::<Vec<_>>(),
        ));
    }

    let mut handles = Vec::with_capacity(documents.len().saturating_mul(COMPLETION_BURST_PER_FILE));
    let wall_started = Instant::now();
    for document in &documents {
        for _ in 0..COMPLETION_BURST_PER_FILE {
            let server = server.clone();
            let uri = document.uri.clone();
            handles.push(tokio::spawn(async move {
                let started = Instant::now();
                let response = server
                    .completion(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri },
                            position: completion_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                        context: Some(CompletionContext {
                            trigger_kind: CompletionTriggerKind::INVOKED,
                            trigger_character: None,
                        }),
                    })
                    .await;
                (response, started.elapsed().as_secs_f64() * 1000.0)
            }));
        }
    }
    let completion_outcomes =
        tokio::time::timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS), async move {
            let mut outcomes = Vec::with_capacity(handles.len());
            for handle in handles {
                outcomes.push(handle.await.expect("parallel completion task join"));
            }
            outcomes
        })
        .await
        .expect("parallel completion burst timed out");
    let wall_time_ms = wall_started.elapsed().as_secs_f64() * 1000.0;

    let mut success_total = 0_u64;
    let mut non_empty_total = 0_u64;
    let mut empty_incomplete_total = 0_u64;
    let mut empty_complete_total = 0_u64;
    let mut sum_latency_ms = 0.0_f64;
    let mut max_latency_ms = 0.0_f64;
    for (response, latency_ms) in completion_outcomes {
        sum_latency_ms += latency_ms;
        max_latency_ms = max_latency_ms.max(latency_ms);
        if let Ok(Some(completion)) = response {
            success_total += 1;
            if completion_items_count(&completion) > 0 {
                non_empty_total += 1;
            } else if completion_is_incomplete(&completion) {
                empty_incomplete_total += 1;
            } else {
                empty_complete_total += 1;
            }
        }
    }

    let total_requests = (documents.len() * COMPLETION_BURST_PER_FILE) as u64;
    let average_latency_ms = sum_latency_ms / total_requests.max(1) as f64;
    let concurrency_gain = if wall_time_ms > 0.0 {
        sum_latency_ms / wall_time_ms
    } else {
        1.0
    };

    assert_eq!(
        success_total, total_requests,
        "parallel completion burst must complete successfully for all cross-file requests"
    );
    assert!(
        non_empty_total > 0,
        "parallel completion burst produced only empty completion payloads after didChange burst: empty_incomplete_total={empty_incomplete_total}, empty_complete_total={empty_complete_total}, owner_hint_type_names={owner_hint_type_names:?}"
    );
    assert!(
        max_latency_ms <= MAX_COMPLETION_LATENCY_MS,
        "cross-file completion max latency exceeded: {max_latency_ms:.2}ms > {MAX_COMPLETION_LATENCY_MS:.2}ms"
    );
    if average_latency_ms >= MIN_AVG_LATENCY_FOR_GAIN_CHECK_MS {
        assert!(
            concurrency_gain >= MIN_CONCURRENCY_GAIN,
            "parallel completion behaved as serialized workload after didChange burst: gain={concurrency_gain:.2} (sum={sum_latency_ms:.2}ms wall={wall_time_ms:.2}ms)"
        );
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let queue_wait_interactive_p95 = histograms
        .get("intellisense_v2_runtime_queue_wait_interactive_ms")
        .and_then(|value| value.as_object())
        .map(|hist| metric_as_f64(hist.get("p95")))
        .unwrap_or(0.0);
    let completion_total_counter = counters
        .get("completion_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let interactive_exec_total = counters
        .get("intellisense_v2_runtime_exec_interactive_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    assert!(
        completion_total_counter > 0,
        "completion_total counter must be populated for cross-file parallel burst"
    );
    assert!(
        interactive_exec_total > 0,
        "interactive exec counter must be populated for cross-file parallel burst"
    );
    assert!(
        queue_wait_interactive_p95 <= MAX_QUEUE_WAIT_P95_MS,
        "interactive queue-wait p95 regression after didChange burst: {queue_wait_interactive_p95:.2}ms > {MAX_QUEUE_WAIT_P95_MS:.2}ms"
    );

    let report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": "p30_cross_file_did_change_parallel_completion_no_global_lock_bottleneck",
        "inputs": {
            "documents_total": documents.len(),
            "did_change_burst_per_file": DID_CHANGE_BURST_PER_FILE,
            "parallel_completion_burst_per_file": COMPLETION_BURST_PER_FILE,
        },
        "thresholds": {
            "request_timeout_secs": REQUEST_TIMEOUT_SECS,
            "max_completion_latency_ms": MAX_COMPLETION_LATENCY_MS,
            "max_queue_wait_interactive_p95_ms": MAX_QUEUE_WAIT_P95_MS,
            "min_concurrency_gain": MIN_CONCURRENCY_GAIN,
            "min_avg_latency_for_gain_check_ms": MIN_AVG_LATENCY_FOR_GAIN_CHECK_MS,
        },
        "results": {
            "total_requests": total_requests,
            "success_total": success_total,
            "non_empty_total": non_empty_total,
            "sum_latency_ms": sum_latency_ms,
            "wall_time_ms": wall_time_ms,
            "average_latency_ms": average_latency_ms,
            "max_latency_ms": max_latency_ms,
            "concurrency_gain": concurrency_gain,
            "queue_wait_interactive_p95_ms": queue_wait_interactive_p95,
            "completion_total_counter": completion_total_counter,
            "interactive_exec_total": interactive_exec_total,
        },
        "pass": true
    });
    let report_path = std::env::var("BSL_V2_COMPLETION_CROSS_FILE_DID_CHANGE_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!(
                    "{CHANGE_ID}-didchange-parallel-completion-cross-file.json"
                ))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for cross-file completion report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report)
            .expect("failed to serialize cross-file completion report"),
    )
    .expect("failed to write cross-file completion report");
    println!(
        "v2_completion_cross_file_did_change_report={}",
        report_path.display()
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_trigger_character_and_invoked_member_access_keep_semantic_parity() {
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

    let uri = Url::parse("file:///test_p7_trigger_parity.bsl").expect("test uri");
    let text = concat!(
        "Процедура Тест()\n",
        "    ДляCompletion = (Новый Массив()).\n",
        "КонецПроцедуры\n"
    );
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: text.to_string(),
        }],
    };
    let did_change_req = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
        .finish();
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req)
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");
    let completion_position = find_utf16_position_after_marker(text, "(Новый Массив()).");
    let dot_response = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: completion_position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
        })
        .await
        .expect("dot completion request")
        .expect("dot completion response");

    let invoked_response = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: completion_position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        })
        .await
        .expect("invoked completion request")
        .expect("invoked completion response");

    let extract_labels = |response: &CompletionResponse| -> Vec<String> {
        match response {
            CompletionResponse::Array(items) => {
                items.iter().map(|item| item.label.clone()).collect()
            }
            CompletionResponse::List(list) => {
                list.items.iter().map(|item| item.label.clone()).collect()
            }
        }
    };
    let dot_members = extract_labels(&dot_response);
    let invoked_members = extract_labels(&invoked_response);
    let metrics = coordinator.observability_metrics();
    assert!(
        !dot_members.is_empty(),
        "trigger-character completion must return candidates"
    );
    assert!(
        !invoked_members.is_empty(),
        "invoked completion must return candidates"
    );
    assert!(
        dot_members.iter().any(|label| invoked_members.contains(label)),
        "trigger-character and invoked completion must have semantic overlap: dot={:?} invoked={:?}",
        dot_members,
        invoked_members
    );

    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let trigger_char_total = counters
        .get("intellisense_v2_completion_trigger_mode_total_mode_trigger_character")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let invoked_total = counters
        .get("intellisense_v2_completion_trigger_mode_total_mode_invoked")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let overlap_metric_recorded = counters.iter().any(|(key, value)| {
        key.starts_with("intellisense_v2_completion_parity_overlap_total_mode_invoked_bucket_")
            && value.as_u64().unwrap_or(0) > 0
    });
    assert!(
        trigger_char_total > 0,
        "trigger-character completion metric must be recorded"
    );
    assert!(
        invoked_total > 0,
        "invoked completion metric must be recorded"
    );
    assert!(
        overlap_metric_recorded,
        "semantic-overlap parity metric must be recorded"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_completion_context_modes_are_supported() {
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

    let uri = Url::parse("file:///test_p7_completion_context_modes.bsl").expect("test uri");
    let text = "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    let member_character = "    ЛокМассив."
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;
    let base_params = TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position: Position::new(2, member_character),
    };

    let contexts = [
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS,
            trigger_character: None,
        }),
        None,
    ];

    for context in contexts {
        let response = server
            .completion(CompletionParams {
                text_document_position: base_params.clone(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context,
            })
            .await
            .expect("completion request");
        assert!(response.is_some(), "completion response must be present");
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        counters
            .get("intellisense_v2_completion_trigger_mode_total_mode_trigger_character")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        counters
            .get("intellisense_v2_completion_trigger_mode_total_mode_invoked")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        counters
            .get("intellisense_v2_completion_trigger_mode_total_mode_trigger_for_incomplete")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        counters
            .get("intellisense_v2_completion_trigger_mode_total_mode_none")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_first_completion_after_received_advance_fails_closed_empty() {
    const STALE_FIXTURE: &str =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";

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

    let uri = Url::parse("file:///test_p7_stale_completion.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: STALE_FIXTURE.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: STALE_FIXTURE.to_string(),
        }],
    };
    let did_change_req = Request::build("textDocument/didChange")
        .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
        .finish();
    let did_change_response = service
        .ready()
        .await
        .unwrap()
        .call(did_change_req)
        .await
        .expect("didChange notification");
    assert!(did_change_response.is_none(), "didChange is a notification");

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    server
        .deps_update_v2("p7_stale_completion_setup", None, None)
        .await;
    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    {
        let mut versions = server.latest_received_file_versions_v2.write().await;
        // Simulate the window right after the next didChange was received but before runtime apply.
        versions.insert(file_id, 3);
    }

    let completion = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(2, 13),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        })
        .await
        .expect("completion request")
        .expect("completion response");

    match completion {
        CompletionResponse::List(list) => {
            assert!(
                list.items.is_empty(),
                "first completion after received-version advance must fail closed with empty payload"
            );
        }
        CompletionResponse::Array(items) => {
            assert!(
                items.is_empty(),
                "first completion after received-version advance must fail closed with empty payload"
            );
        }
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let stale_fallback_total = counters
        .get("intellisense_v2_completion_stale_fallback_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let fallback_unavailable_total = counters
        .get("intellisense_v2_completion_fallback_unavailable_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let fail_closed_reason_total = counters
        .iter()
        .filter(|(key, _)| {
            key.starts_with(
                "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_completion_",
            )
        })
        .map(|(_, value)| value.as_u64().unwrap_or(0))
        .sum::<u64>();
    assert!(
        stale_fallback_total == 0,
        "stale fallback counter must stay zero under fail-closed contract"
    );
    assert!(
        fallback_unavailable_total > 0,
        "fail-closed completion miss must record fallback_unavailable"
    );
    assert!(
        fail_closed_reason_total > 0,
        "fail-closed completion miss must emit bounded public reason metrics"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_large_churn_budget_timeout_returns_fail_closed_empty_response() {
    const FILLER_LINES: usize = 2200;
    let mut fixture = String::new();
    fixture.push_str("Процедура Тест()\n");
    fixture.push_str("    ЛокМассив = Новый Массив;\n");
    for idx in 0..FILLER_LINES {
        fixture.push_str(&format!("    // filler {idx}\n"));
    }
    fixture.push_str("    ЛокМассив.\n");
    fixture.push_str("КонецПроцедуры\n");

    let completion_line = (2 + FILLER_LINES) as u32;
    let completion_character = "    ЛокМассив."
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;

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

    let uri = Url::parse("file:///test_p7_large_churn_stale_fastpath.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.clone(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    for version in 2..=7_i32 {
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: fixture.clone(),
            }],
        };
        let did_change_req = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
            .finish();
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req)
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");
    }

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    server
        .deps_update_v2("p7_large_churn_stale_fastpath_setup", None, None)
        .await;
    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let large_churn_active = server
        .scale_aware_churn_state_v2
        .read()
        .await
        .get(&file_id)
        .is_some_and(|state| state.large_churn_active);
    assert!(
        large_churn_active,
        "expected large+churn state to be active after burst didChange on large document"
    );

    let warm_completion = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(completion_line, completion_character),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
        })
        .await
        .expect("warm completion request")
        .expect("warm completion response");
    let _ = warm_completion;

    {
        let mut versions = server.latest_received_file_versions_v2.write().await;
        versions.insert(file_id, 8);
    }

    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let started = Instant::now();
    let stale_completion = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(completion_line, completion_character),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
        })
        .await
        .expect("stale completion request")
        .expect("stale completion response");
    let elapsed = started.elapsed();

    match stale_completion {
        CompletionResponse::List(list) => {
            assert!(
                list.items.is_empty(),
                "large-churn timeout must fail closed with empty payload even when cache is warm"
            );
        }
        CompletionResponse::Array(items) => {
            assert!(
                items.is_empty(),
                "large-churn timeout must fail closed with empty payload even when cache is warm"
            );
        }
    }
    assert!(
        elapsed <= Duration::from_millis(wait_budget_ms.saturating_add(300)),
        "fail-closed completion should remain bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let stale_fallback_total = counters
        .get("intellisense_v2_completion_stale_fallback_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let fallback_unavailable_total = counters
        .get("intellisense_v2_completion_fallback_unavailable_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let fail_closed_reason_total = counters
        .iter()
        .filter(|(key, _)| {
            key.starts_with(
                "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_completion_",
            )
        })
        .map(|(_, value)| value.as_u64().unwrap_or(0))
        .sum::<u64>();
    assert!(
        stale_fallback_total == 0,
        "stale fallback counter must stay zero under fail-closed churn path"
    );
    assert!(
        fallback_unavailable_total > 0,
        "fail-closed churn path must record fallback_unavailable"
    );
    assert!(
        fail_closed_reason_total > 0,
        "fail-closed churn path must emit bounded public reason metrics"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_member_access_completion_does_not_backfill_from_runtime_index_snapshot() {
    const FAIL_CLOSED_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_completion_reason_missing_semantic_index";

    fn make_index_snapshot(id: &str, type_name: &str) -> IndexSnapshot {
        let mut snapshot = IndexSnapshot::empty(IndexSnapshotId::from_hash(id.to_string()));
        Arc::make_mut(&mut snapshot.type_index).insert(
            type_name.to_string(),
            Arc::new(IndexItem::new(
                type_name.to_string(),
                IndexItemKind::Type(TypeKind::Generic),
                IndexKind::Type,
            )),
        );
        snapshot
    }

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
ЛокМассив = Новый Массив;\n\
ЛокМассив.\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_completion_no_search_rescue.bsl").expect("uri");
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

    coordinator
        .intellisense_index()
        .replace_snapshot(make_index_snapshot(
            "p7_completion_no_search_rescue",
            "SearchOnlyType",
        ));
    coordinator.intellisense_index().replace_symbols_for_uri(
        uri.as_str(),
        vec![IndexItem::new(
            "SearchOnlySymbol".to_string(),
            IndexItemKind::Symbol(bsl_backend::system::SymbolKind::Function),
            IndexKind::Symbol,
        )],
    );
    coordinator.intellisense_index().replace_modules_for_key(
        "p7_completion_no_search_rescue_module",
        vec![IndexItem::new(
            "SearchOnlyModule".to_string(),
            IndexItemKind::Symbol(bsl_backend::system::SymbolKind::Procedure),
            IndexKind::Module,
        )],
    );

    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let completion_position = find_utf16_position_after_marker(fixture, "ЛокМассив.");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        completion_labels.is_empty(),
        "member-access cache miss must stay fail-closed when only runtime discovery/search index is populated, labels={completion_labels:?}"
    );
    assert!(
        completion_labels
            .iter()
            .all(|label| label != "SearchOnlyType" && label != "SearchOnlySymbol" && label != "SearchOnlyModule"),
        "member-access completion must not backfill from runtime discovery/search index, labels={completion_labels:?}"
    );
    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let fallback_unavailable_total =
        read_u64_metric(counters.get("intellisense_v2_completion_fallback_unavailable_total"));
    assert!(
        fallback_unavailable_total > 0,
        "member-access cache miss must record fallback_unavailable before public reason emission, counters={counters:?}"
    );
    let exact_wait_no_matching_task_total = read_u64_metric(counters.get(
        "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task",
    ));
    assert!(
        exact_wait_no_matching_task_total > 0,
        "member-access exact wait must expose no_matching_task outcome, counters={counters:?}"
    );
    let fail_closed_reason_total = read_u64_metric(counters.get(FAIL_CLOSED_REASON_KEY));
    assert!(
        fail_closed_reason_total > 0,
        "member-access cache miss must emit missing_semantic_index bounded public reason metrics, counters={counters:?}"
    );
    let ir_query_count = histograms
        .get("intellisense_v2_ir_query_completion_ms")
        .and_then(|value| value.as_object())
        .map(|histogram| read_u64_metric(histogram.get("count")))
        .unwrap_or(0);
    assert_eq!(
        ir_query_count, 0,
        "member-access fail-closed path must not build canonical IR when exact type index is unavailable, histograms={histograms:?}"
    );
    let query_bundle_count = histograms
        .get("completion_stage_query_bundle_ms")
        .and_then(|value| value.as_object())
        .map(|histogram| read_u64_metric(histogram.get("count")))
        .unwrap_or(0);
    let exact_wait_apply_age_start_count = histograms
        .get("completion_stage_exact_wait_apply_age_at_start_ms")
        .and_then(|value| value.as_object())
        .map(|histogram| read_u64_metric(histogram.get("count")))
        .unwrap_or(0);
    let exact_wait_apply_age_terminal_count = histograms
        .get("completion_stage_exact_wait_apply_age_at_terminal_ms")
        .and_then(|value| value.as_object())
        .map(|histogram| read_u64_metric(histogram.get("count")))
        .unwrap_or(0);
    assert_eq!(
        query_bundle_count, 0,
        "member-access fail-closed path must short-circuit before query_bundle, histograms={histograms:?}"
    );
    assert!(
        exact_wait_apply_age_start_count > 0 && exact_wait_apply_age_terminal_count > 0,
        "member-access exact wait must expose apply-age histograms before fail-closed return, histograms={histograms:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_type_index_precompute_slot_coalesces_rapid_versions_without_respawn_or_cancel_fanout() {
    const CANCELLED_REASON_KEY: &str =
        "intellisense_v2_type_index_reason_total_reason_type_index_precompute_cancelled";

    let cancelled_reason_total = |coordinator: &Arc<SystemCoordinator>| -> u64 {
        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        read_u64_metric(counters.get(CANCELLED_REASON_KEY))
    };

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (_service, socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move {
        let mut socket = socket;
        while let Some(_req) = socket.next().await {}
    });

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let before_cancelled = cancelled_reason_total(&coordinator);
    let file_id = bsl_analysis_v2::FileId(777);
    let initial_version = 4;
    let latest_version = 8_i32;
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, latest_version);

    let initial_task_id = 321_u64;
    let initial_handle = tokio::spawn(std::future::pending::<()>());
    let initial_phase = Arc::new(std::sync::atomic::AtomicU8::new(
        super::deps_and_precompute::TypeIndexPrecomputePhaseV2::WaitingCpuPermit.as_u8(),
    ));
    {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        tasks.insert(
            file_id,
            crate::server::TypeIndexPrecomputeTaskV2 {
                task_id: initial_task_id,
                supersession_key: crate::server::TypeIndexPrecomputeSupersessionKeyV2 {
                    file_id,
                    requested_version: initial_version,
                },
                work_class: bsl_runtime::application::CpuWorkClass::Background,
                phase: Arc::clone(&initial_phase),
                active_requested_version: Arc::new(std::sync::atomic::AtomicI32::new(0)),
                scheduled_at: Instant::now(),
                handle: initial_handle,
            },
        );
    }

    for version in (initial_version + 1)..=latest_version {
        server
            .schedule_type_index_precompute_v2(file_id, version)
            .await;
    }

    {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        let task = tasks
            .get(&file_id)
            .expect("coalesced type-index precompute task must remain registered");
        assert_eq!(
            task.task_id, initial_task_id,
            "rapid reschedule burst must keep the same coordinated precompute slot instead of abort+respawn"
        );
        assert_eq!(
            task.supersession_key.requested_version, latest_version,
            "coalesced precompute slot must track the latest requested version"
        );
        assert_eq!(
            task.work_class,
            bsl_runtime::application::CpuWorkClass::Background,
            "didChange reschedule burst must keep background work class until an interactive waiter explicitly promotes it"
        );
        task.handle.abort();
        let _ = tasks.remove(&file_id);
    }
    let after_cancelled = cancelled_reason_total(&coordinator);
    assert_eq!(
        after_cancelled, before_cancelled,
        "rapid reschedule burst must not emit type_index_precompute_cancelled fanout when the slot can coalesce in place"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_waiting_completion_promotes_matching_type_index_precompute_to_interactive() {
    const FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";

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
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let (background_started_tx, background_started_rx) = tokio::sync::oneshot::channel::<()>();
    let background_blocker = tokio::spawn(async move {
        bsl_runtime::application::spawn_bounded_blocking_with_class(
            bsl_runtime::application::CpuWorkClass::Background,
            move || {
                let _ = background_started_tx.send(());
                std::thread::sleep(Duration::from_millis(400));
            },
        )
        .await
        .expect("background blocker join");
    });
    background_started_rx
        .await
        .expect("background blocker should start");

    let uri = Url::parse("file:///test_p7_waiter_promotes_exact_precompute.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
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

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = S.");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let promotion_total = read_u64_metric(
        counters.get("intellisense_v2_completion_exact_type_index_wait_promotion_total"),
    );
    let join_total = read_u64_metric(
        counters.get("intellisense_v2_completion_exact_type_index_wait_join_total"),
    );
    assert!(
        completion_labels.iter().any(|label| label == "Количество"),
        "waiting completion should recover typed-structure members once matching exact precompute is promoted or joined, labels={completion_labels:?}, counters={counters:?}"
    );
    assert!(
        promotion_total + join_total > 0,
        "waiting completion must observe waiter-aware exact precompute orchestration (join or promotion), counters={counters:?}"
    );
    assert!(
        read_u64_metric(
            counters.get("intellisense_v2_completion_exact_type_index_wait_ready_after_wait_total")
        ) > 0,
        "waiting completion must report ready_after_wait after joining or promoting the matching precompute, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline")
        ),
        0,
        "promotion path must avoid exact wait deadline miss in this scenario, counters={counters:?}"
    );

    background_blocker.await.expect("background blocker task");
    drain_task.abort();
}

#[tokio::test]
async fn p7_promote_type_index_precompute_replaces_matching_background_waiter() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (_service, socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().expect("server holder lock") = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move {
        let mut socket = socket;
        while let Some(_req) = socket.next().await {}
    });

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be captured");

    let file_id = bsl_analysis_v2::FileId(4242);
    let requested_version = 7;
    server
        .latest_received_file_versions_v2
        .write()
        .await
        .insert(file_id, requested_version);

    let previous_task_id = 999_u64;
    let previous_handle = tokio::spawn(std::future::pending::<()>());
    let previous_phase = Arc::new(std::sync::atomic::AtomicU8::new(
        super::deps_and_precompute::TypeIndexPrecomputePhaseV2::WaitingCpuPermit.as_u8(),
    ));
    {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        tasks.insert(
            file_id,
            crate::server::TypeIndexPrecomputeTaskV2 {
                task_id: previous_task_id,
                supersession_key: crate::server::TypeIndexPrecomputeSupersessionKeyV2 {
                    file_id,
                    requested_version,
                },
                work_class: bsl_runtime::application::CpuWorkClass::Background,
                phase: Arc::clone(&previous_phase),
                active_requested_version: Arc::new(std::sync::atomic::AtomicI32::new(0)),
                scheduled_at: Instant::now(),
                handle: previous_handle,
            },
        );
    }

    let action = server
        .promote_type_index_precompute_for_waiter_v2(file_id, Some(requested_version))
        .await;
    assert_eq!(
        action,
        super::deps_and_precompute::TypeIndexPrecomputeWaiterActionV2::Promoted,
        "matching background task in pre-compute wait phase must be promoted for interactive waiter"
    );

    let promoted_task = {
        let mut tasks = server.type_index_precompute_tasks_v2.lock().await;
        let task = tasks
            .remove(&file_id)
            .expect("promoted task must remain registered");
        task.handle.abort();
        task
    };
    assert_eq!(
        promoted_task.supersession_key.requested_version, requested_version,
        "promotion must preserve requested version for the matching waiter"
    );
    assert_eq!(
        promoted_task.work_class,
        bsl_runtime::application::CpuWorkClass::Interactive,
        "promotion must respawn the task in interactive work class"
    );
    assert_ne!(
        promoted_task.task_id, previous_task_id,
        "promotion must replace the previous task instance instead of mutating it in place"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_hover_and_definition_do_not_backfill_from_runtime_index_snapshot() {
    const HOVER_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_hover_reason_missing_semantic_index";
    const DEFINITION_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_definition_reason_missing_semantic_index";

    fn make_index_snapshot(id: &str, type_name: &str) -> IndexSnapshot {
        let mut snapshot = IndexSnapshot::empty(IndexSnapshotId::from_hash(id.to_string()));
        Arc::make_mut(&mut snapshot.type_index).insert(
            type_name.to_string(),
            Arc::new(IndexItem::new(
                type_name.to_string(),
                IndexItemKind::Type(TypeKind::Generic),
                IndexKind::Type,
            )),
        );
        snapshot
    }

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура МойМетод() Экспорт\n\
КонецПроцедуры\n\
\n\
Процедура Тест()\n\
    ЭтотОбъект.МойМетод();\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_hover_definition_no_search_rescue.bsl").expect("uri");

    coordinator
        .intellisense_index()
        .replace_snapshot(make_index_snapshot(
            "p7_hover_definition_no_search_rescue",
            "SearchOnlyType",
        ));
    coordinator.intellisense_index().replace_symbols_for_uri(
        uri.as_str(),
        vec![IndexItem::new(
            "SearchOnlyMethod".to_string(),
            IndexItemKind::Symbol(bsl_backend::system::SymbolKind::Procedure),
            IndexKind::Symbol,
        )],
    );
    coordinator.intellisense_index().replace_modules_for_key(
        "p7_hover_definition_no_search_rescue_module",
        vec![IndexItem::new(
            "SearchOnlyModule".to_string(),
            IndexItemKind::Symbol(bsl_backend::system::SymbolKind::Procedure),
            IndexKind::Module,
        )],
    );

    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 1).await;

    let member_position = find_utf16_position_after_marker(fixture, "ЭтотОбъект.МойМетод");

    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(12012)
                .params(
                    serde_json::to_value(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: member_position,
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
    assert!(
        hover.is_none(),
        "hover must stay fail-closed when only runtime discovery/search index is populated: {hover_value:?}"
    );

    let definition_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/definition")
                .id(12013)
                .params(
                    serde_json::to_value(GotoDefinitionParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: member_position,
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
    assert!(
        normalize_lsp_definition(definition).is_empty(),
        "definition must stay fail-closed when only runtime discovery/search index is populated: {definition_value:?}"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get(HOVER_REASON_KEY)) > 0,
        "hover cache miss must emit missing_semantic_index bounded public reason metrics, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get(DEFINITION_REASON_KEY)) > 0,
        "definition cache miss must emit missing_semantic_index bounded public reason metrics, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_large_churn_budget_timeout_without_prior_completion_returns_fail_closed_empty_response()
{
    const FILLER_LINES: usize = 2200;
    let mut fixture = String::new();
    fixture.push_str("Процедура Тест()\n");
    fixture.push_str("    ЛокМассив = Новый Массив;\n");
    for idx in 0..FILLER_LINES {
        fixture.push_str(&format!("    // filler {idx}\n"));
    }
    fixture.push_str("    ЛокМассив.\n");
    fixture.push_str("КонецПроцедуры\n");

    let completion_line = (2 + FILLER_LINES) as u32;
    let completion_character = "    ЛокМассив."
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;

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

    let uri = Url::parse("file:///test_p7_large_churn_stale_cache_miss.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.clone(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    for version in 2..=7_i32 {
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: fixture.clone(),
            }],
        };
        let did_change_req = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
            .finish();
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req)
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");
    }

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    server
        .deps_update_v2("p7_large_churn_stale_cache_miss_setup", None, None)
        .await;
    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let large_churn_active = server
        .scale_aware_churn_state_v2
        .read()
        .await
        .get(&file_id)
        .is_some_and(|state| state.large_churn_active);
    assert!(
        large_churn_active,
        "expected large+churn state to be active after burst didChange on large document"
    );

    {
        let mut versions = server.latest_received_file_versions_v2.write().await;
        versions.insert(file_id, 8);
    }

    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let started = Instant::now();
    let stale_completion = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(completion_line, completion_character),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
        })
        .await
        .expect("stale completion request")
        .expect("stale completion response");
    let elapsed = started.elapsed();

    match stale_completion {
        CompletionResponse::List(list) => {
            assert!(
                list.items.is_empty(),
                "cache miss under large-churn timeout must fail closed with empty payload"
            );
        }
        CompletionResponse::Array(items) => {
            assert!(
                items.is_empty(),
                "cache miss under large-churn timeout must fail closed with empty payload"
            );
        }
    }
    assert!(
        elapsed <= Duration::from_millis(wait_budget_ms.saturating_add(300)),
        "fail-closed cache miss should remain bounded near wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let stale_fallback_total = counters
        .get("intellisense_v2_completion_stale_fallback_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        stale_fallback_total == 0,
        "stale fallback counter must stay zero on fail-closed cache miss"
    );
    let fallback_unavailable_total = counters
        .get("intellisense_v2_completion_fallback_unavailable_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        fallback_unavailable_total > 0,
        "expected fallback_unavailable counter to increment for stale cache-miss"
    );
    let prepare_apply_age_start_count = histograms
        .get("completion_stage_prepare_apply_age_at_start_ms")
        .and_then(|value| value.as_object())
        .and_then(|histogram| histogram.get("count"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let prepare_apply_age_terminal_count = histograms
        .get("completion_stage_prepare_apply_age_at_terminal_ms")
        .and_then(|value| value.as_object())
        .and_then(|histogram| histogram.get("count"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        prepare_apply_age_start_count > 0 && prepare_apply_age_terminal_count > 0,
        "prepare fail-closed path must expose apply-age histograms, histograms={histograms:?}"
    );

    drain_task.abort();
}

#[cfg(unix)]
#[tokio::test]
async fn p7_completion_prepare_timeout_stays_bounded_when_disk_fallback_blocks() {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

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

    let temp = tempfile::TempDir::new().expect("tempdir");
    let fifo_path = temp.path().join("blocking_prepare_fallback.bsl");
    let mkfifo_status = std::process::Command::new("mkfifo")
        .arg(&fifo_path)
        .status()
        .expect("mkfifo status");
    assert!(mkfifo_status.success(), "mkfifo must succeed");

    let writer_path = fifo_path.clone();
    let writer = std::thread::spawn(move || {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .mode(0o644)
            .open(&writer_path)
            .expect("open fifo writer");
        std::thread::sleep(Duration::from_millis(350));
        let fixture =
            "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
        file.write_all(fixture.as_bytes())
            .expect("write fifo contents");
    });

    let uri = Url::from_file_path(&fifo_path).expect("fifo uri");
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);

    let started = Instant::now();
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/completion")
                .id(17701)
                .params(
                    serde_json::to_value(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: Position::new(2, 13),
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
    let elapsed = started.elapsed();

    let completion_value =
        serde_json::to_value(&completion_response).expect("serialize completion response");
    let result = completion_value
        .get("result")
        .cloned()
        .expect("result field");
    let response: Option<CompletionResponse> =
        serde_json::from_value(result).expect("parse completion result");
    match response.expect("completion payload") {
        CompletionResponse::List(list) => {
            assert!(
                list.items.is_empty(),
                "blocking disk fallback must still fail closed under prepare timeout"
            );
        }
        CompletionResponse::Array(items) => {
            assert!(
                items.is_empty(),
                "blocking disk fallback must still fail closed under prepare timeout"
            );
        }
    }
    assert!(
        elapsed <= Duration::from_millis(wait_budget_ms.saturating_add(200)),
        "blocking disk fallback must stay bounded by prepare wait budget (elapsed={elapsed:?}, budget_ms={wait_budget_ms})"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"
            )
        ) > 0,
        "blocking disk fallback must attribute fail-closed completion to prepare-timeout, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline"
            )
        ),
        0,
        "blocking disk fallback must not attribute fail-closed completion to exact-deadline, counters={counters:?}"
    );

    writer.join().expect("writer thread");
    drain_task.abort();
}

#[tokio::test]
async fn p7_large_churn_did_change_skips_inline_parse_snapshot() {
    const FILLER_LINES: usize = 2200;
    const LATEST_VERSION: i32 = 7;
    let mut fixture = String::new();
    fixture.push_str("Процедура Тест()\n");
    fixture.push_str("    ЛокМассив = Новый Массив;\n");
    for idx in 0..FILLER_LINES {
        fixture.push_str(&format!("    // filler {idx}\n"));
    }
    fixture.push_str("    ЛокМассив.\n");
    fixture.push_str("КонецПроцедуры\n");

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

    let uri = Url::parse("file:///test_p7_large_churn_skip_parse_snapshot.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.clone(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    for version in 2..=LATEST_VERSION {
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: fixture.clone(),
            }],
        };
        let did_change_req = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
            .finish();
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req)
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");
    }

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    server.sync_v2_globals().await;

    let file_id = server.get_or_create_file_id_v2(&uri).await;
    let large_churn_active = server
        .scale_aware_churn_state_v2
        .read()
        .await
        .get(&file_id)
        .is_some_and(|state| state.large_churn_active);
    assert!(
        large_churn_active,
        "expected large+churn state to be active after burst didChange on large document"
    );
    assert!(
        server
            .analysis_v2
            .wait_for_file_version(file_id, LATEST_VERSION)
            .await,
        "analysis runtime must catch up to the latest didChange version"
    );

    let analysis = server.analysis_v2.snapshot().await;
    let parse_mode = analysis
        .syntax_diagnostics_observability_mode(file_id)
        .expect("syntax diagnostics mode query")
        .expect("syntax diagnostics mode for active file");
    assert_eq!(
        parse_mode, "other",
        "large+churn didChange must skip inline parse snapshot and leave syntax diagnostics on fallback mode"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert!(
        read_u64_metric(counters.get("intellisense_v2_parse_snapshot_total_origin_lsp_mode_other"))
            > 0,
        "skipped large+churn parse snapshot must be visible via mode=other counters"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_hover_cache_miss_emits_bounded_fail_closed_reason() {
    const FALLBACK_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_hover_reason_missing_semantic_index";

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
МассивДляHover = Новый Массив;\n\
ЗначДляHover = МассивДляHover.Количество();\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_hover_type_index_fallback_unavailable.bsl").expect("uri");
    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 1).await;

    let fallback_reason_total = |coordinator: &Arc<SystemCoordinator>| -> u64 {
        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        read_u64_metric(counters.get(FALLBACK_REASON_KEY))
    };

    let before_hover = fallback_reason_total(&coordinator);
    let hover_position = find_utf16_position_after_marker(fixture, "ЗначДляHover = МассивДляHover");
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(9107)
                .params(
                    serde_json::to_value(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: hover_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .expect("HoverParams"),
                )
                .finish(),
        )
        .await
        .expect("hover request");
    assert!(
        hover_response.is_some(),
        "hover should return response envelope"
    );
    let hover_value = serde_json::to_value(&hover_response).expect("serialize response");
    let hover_result = hover_value.get("result").cloned().expect("result field");
    let hover: Option<Hover> = serde_json::from_value(hover_result).expect("parse hover");
    assert!(
        hover.is_none(),
        "hover cache miss must stay fail-closed and return empty semantic payload: {hover_value:?}"
    );
    let after_hover = fallback_reason_total(&coordinator);
    assert!(
        after_hover > before_hover,
        "hover cache miss must emit bounded public fail-closed reason"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_hover_cache_miss_on_map_index_access_does_not_use_legacy_word_fallback() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
Map = Новый Соответствие;\n\
Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
ЗначДляHover = Map[\"k\"];\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_hover_map_index_no_legacy_fallback.bsl").expect("uri");
    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be captured");
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 1).await;

    let hover_position = find_utf16_position_after_marker(fixture, "ЗначДляHover = ");
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(9108)
                .params(
                    serde_json::to_value(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: hover_position,
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
    let hover_value = serde_json::to_value(&hover_response).expect("serialize response");
    let hover_result = hover_value.get("result").cloned().expect("result field");
    let hover: Option<Hover> = serde_json::from_value(hover_result).expect("parse hover");
    assert!(
        hover.is_none(),
        "hover cache miss on map index access must not synthesize legacy fallback payload: {hover_value:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_legitimate_empty_interactive_results_do_not_emit_fail_closed_reasons() {
    const HOVER_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_hover_reason_missing_semantic_index";
    const SIGNATURE_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_signature_help_reason_missing_semantic_index";
    const DEFINITION_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_definition_reason_missing_semantic_index";

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
    МойМассив = Новый Массив;\n\
    МойМассив.Несуществующий(1);\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_legitimate_empty_interactive_results.bsl").expect("uri");
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

    let metric_total = |metric_key: &str| -> u64 {
        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        read_u64_metric(counters.get(metric_key))
    };

    let before_hover = metric_total(HOVER_REASON_KEY);
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(9111)
                .params(
                    serde_json::to_value(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: Position::new(1, 0),
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .expect("HoverParams"),
                )
                .finish(),
        )
        .await
        .expect("hover request");
    assert!(
        hover_response.is_some(),
        "hover must return a response envelope even when result is empty"
    );
    let after_hover = metric_total(HOVER_REASON_KEY);
    assert_eq!(
        after_hover, before_hover,
        "legitimate empty hover result must not emit fail-closed reason"
    );

    let before_signature = metric_total(SIGNATURE_REASON_KEY);
    let signature_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/signatureHelp")
                .id(9112)
                .params(
                    serde_json::to_value(tower_lsp::lsp_types::SignatureHelpParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: find_utf16_position_after_marker(
                                fixture,
                                "МойМассив.Несуществующий(",
                            ),
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        context: None,
                    })
                    .expect("SignatureHelpParams"),
                )
                .finish(),
        )
        .await
        .expect("signatureHelp request");
    assert!(
        signature_response.is_some(),
        "signatureHelp must return a response envelope even when result is empty"
    );
    let after_signature = metric_total(SIGNATURE_REASON_KEY);
    assert_eq!(
        after_signature, before_signature,
        "unknown method signatureHelp result must not emit fail-closed reason"
    );

    let before_definition = metric_total(DEFINITION_REASON_KEY);
    let definition_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/definition")
                .id(9113)
                .params(
                    serde_json::to_value(GotoDefinitionParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: find_utf16_position_after_marker(fixture, "МойМассив."),
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    })
                    .expect("GotoDefinitionParams"),
                )
                .finish(),
        )
        .await
        .expect("definition request");
    assert!(
        definition_response.is_some(),
        "definition must return a response envelope even when result is empty"
    );
    let after_definition = metric_total(DEFINITION_REASON_KEY);
    assert_eq!(
        after_definition, before_definition,
        "unknown method definition result must not emit fail-closed reason"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_constructor_signature_help_without_canonical_fact_stays_empty_without_fail_closed_reason(
) {
    const SIGNATURE_REASON_KEY: &str =
        "intellisense_v2_fail_closed_reason_total_origin_lsp_operation_signature_help_reason_missing_semantic_index";

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;
    let server = server_holder
        .lock()
        .expect("server holder lock")
        .as_ref()
        .cloned()
        .expect("server instance");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура Тест()\n\
    Новый Массив(1, )\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_constructor_signature_help_without_canonical_fact.bsl")
        .expect("uri");
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

    let metric_total = |metric_key: &str| -> u64 {
        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        read_u64_metric(counters.get(metric_key))
    };

    let before_signature = metric_total(SIGNATURE_REASON_KEY);
    let signature_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/signatureHelp")
                .id(9114)
                .params(
                    serde_json::to_value(tower_lsp::lsp_types::SignatureHelpParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: find_utf16_position_after_marker(fixture, "Новый Массив("),
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
    let signature_help: Option<tower_lsp::lsp_types::SignatureHelp> =
        serde_json::from_value(signature_result).expect("parse signatureHelp result");
    assert!(
        signature_help.is_none(),
        "constructor signatureHelp without canonical fact must stay empty on the default LSP path: {signature_value:?}"
    );
    let after_signature = metric_total(SIGNATURE_REASON_KEY);
    assert_eq!(
        after_signature, before_signature,
        "constructor signatureHelp without canonical fact must remain a legitimate empty result"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_map_index_access_exact_cross_consumer_acceptance_uses_snapshot_owner_without_manual_hint(
) {
    let completion_fixture = "Процедура Тест()\n\
    Map = Новый Соответствие;\n\
    Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
    ДляCompletion = Map[\"k\"].\n\
КонецПроцедуры\n";
    let resolved_fixture = "Процедура Тест()\n\
    Map = Новый Соответствие;\n\
    Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
    ДляHover = Map[\"k\"];\n\
    Проверка = Map[\"k\"].Колонки;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        completion_fixture,
        "file:///test_p7_universal_map_exact_acceptance.bsl",
    )
    .await;

    let legacy_type_name = snapshot_type_name_at_marker(
        &server,
        file_id,
        completion_fixture,
        "ДляCompletion = Map[\"k\"]",
    )
    .await;
    assert_eq!(
        legacy_type_name, "ТаблицаЗначений",
        "legacy type-at-position must already know map value type before completion"
    );

    let serve_only_type_name = snapshot_serve_only_type_name_at_marker(
        &server,
        file_id,
        completion_fixture,
        "ДляCompletion = Map[\"k\"]",
    )
    .await;
    assert_eq!(
        serve_only_type_name.as_deref(),
        Some("ТаблицаЗначений"),
        "serve-only snapshot contract must already know map value type before completion"
    );

    let completion_labels = lsp_completion_labels_at(
        &mut service,
        &uri,
        find_utf16_position_after_marker(completion_fixture, "ДляCompletion = Map[\"k\"]."),
    )
    .await;
    assert!(
        completion_labels.iter().any(|label| label == "Колонки"),
        "completion must expose map value members, labels={completion_labels:?}"
    );
    assert!(
        !completion_labels
            .iter()
            .any(|label| label == "Ключ" || label == "Значение"),
        "completion must not fall back to key/value pair members, labels={completion_labels:?}"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, resolved_fixture).await;

    let serve_only_hover_type_name = snapshot_serve_only_type_name_at_marker(
        &server,
        file_id,
        resolved_fixture,
        "ДляHover = Map[\"k\"]",
    )
    .await;
    assert_eq!(
        serve_only_hover_type_name.as_deref(),
        Some("ТаблицаЗначений"),
        "resolved map index must already have exact serve-only type before hover"
    );

    let hover_text = lsp_hover_text_at(
        &mut service,
        &uri,
        find_utf16_position_at_marker_tail(resolved_fixture, "ДляHover = Map[\"k\"]"),
    )
    .await;
    assert!(
        hover_text.contains("ТаблицаЗначений"),
        "hover must expose resolved map value type, hover={hover_text}"
    );

    let type_name =
        snapshot_type_name_at_marker(&server, file_id, resolved_fixture, "ДляHover = Map[\"k\"]")
            .await;
    assert_eq!(
        type_name, "ТаблицаЗначений",
        "type-at-position must match the resolved map value type"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .all(|message| !message_has_unknown_member(message, "Колонки")),
        "diagnostics must not drift for known map value member, diagnostics={diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_dynamic_map_key_exact_cross_consumer_acceptance_uses_safe_policy_without_unknown_key() {
    let completion_fixture = "Процедура Тест()\n\
    Map = Новый Соответствие;\n\
    Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
    Ключ = \"k\";\n\
    ДляCompletion = Map[Ключ].\n\
КонецПроцедуры\n";
    let resolved_fixture = "Процедура Тест()\n\
    Map = Новый Соответствие;\n\
    Map.Вставить(\"k\", Новый ТаблицаЗначений);\n\
    Ключ = \"k\";\n\
    ДляHover = Map[Ключ];\n\
    Проверка = Map[Ключ].Колонки;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        completion_fixture,
        "file:///test_p7_universal_map_dynamic_exact_acceptance.bsl",
    )
    .await;

    let completion_labels = lsp_completion_labels_at(
        &mut service,
        &uri,
        find_utf16_position_after_marker(completion_fixture, "ДляCompletion = Map[Ключ]."),
    )
    .await;
    assert!(
        completion_labels.iter().any(|label| label == "Колонки"),
        "completion must expose generic map value members for dynamic key, labels={completion_labels:?}"
    );
    assert!(
        !completion_labels
            .iter()
            .any(|label| label == "Ключ" || label == "Значение"),
        "dynamic map key must not complete as key/value pair, labels={completion_labels:?}"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, resolved_fixture).await;

    let serve_only_hover_type_name = snapshot_serve_only_type_name_at_marker(
        &server,
        file_id,
        resolved_fixture,
        "ДляHover = Map[Ключ]",
    )
    .await;
    assert_eq!(
        serve_only_hover_type_name.as_deref(),
        Some("ТаблицаЗначений"),
        "resolved dynamic map index must already have generic serve-only value type before hover"
    );

    let hover_text = lsp_hover_text_at(
        &mut service,
        &uri,
        find_utf16_position_at_marker_tail(resolved_fixture, "ДляHover = Map[Ключ]"),
    )
    .await;
    assert!(
        hover_text.contains("ТаблицаЗначений"),
        "hover must expose generic map value type for dynamic key, hover={hover_text}"
    );

    let type_name =
        snapshot_type_name_at_marker(&server, file_id, resolved_fixture, "ДляHover = Map[Ключ]")
            .await;
    assert_eq!(
        type_name, "ТаблицаЗначений",
        "dynamic key type-at-position must follow generic value contract"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .all(|message| !message_has_unknown_member(message, "Колонки")),
        "diagnostics must not drift for known dynamic map value member, diagnostics={diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|message| !message_has_unknown_key(message)),
        "dynamic map key must not emit unknown-key diagnostics, diagnostics={diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_typed_structure_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics(
) {
    let completion_fixture = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    S.Вставить(\"Количество\", 10);\n\
    ДляCompletion = S.\n\
КонецПроцедуры\n";
    let resolved_fixture = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    S.Вставить(\"Количество\", 10);\n\
    ДляHover = S.Идентификатор;\n\
    Ошибка = S.Идентифкатор;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        completion_fixture,
        "file:///test_p7_typed_structure_exact_acceptance.bsl",
    )
    .await;

    let completion_position =
        find_utf16_position_after_marker(completion_fixture, "ДляCompletion = S.");
    let completion_members =
        lsp_completion_members_at(&mut service, &uri, completion_position).await;
    let completion_labels = completion_members
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    assert!(
        completion_labels
            .iter()
            .any(|label| label == "Идентификатор"),
        "completion must include typed structure field Идентификатор, labels={completion_labels:?}"
    );
    assert!(
        completion_labels.iter().any(|label| label == "Количество"),
        "completion must include typed structure field Количество, labels={completion_labels:?}"
    );

    let runtime_resolution = snapshot_type_resolution_at_marker(
        &server,
        file_id,
        completion_fixture,
        "ДляCompletion = S",
    )
    .await;
    let runtime_identifier_identity = runtime_resolution
        .find_structural_member("Идентификатор")
        .map(|member| member.member_id.key.clone())
        .expect("runtime structural identity for Идентификатор");
    let runtime_quantity_identity = runtime_resolution
        .find_structural_member("Количество")
        .map(|member| member.member_id.key.clone())
        .expect("runtime structural identity for Количество");

    let lsp_identifier = completion_members
        .iter()
        .find(|entry| {
            entry.name == "Идентификатор"
                && entry.member_identity.as_deref() == Some(runtime_identifier_identity.as_str())
        })
        .expect("lsp completion entry Идентификатор");
    let lsp_quantity = completion_members
        .iter()
        .find(|entry| {
            entry.name == "Количество"
                && entry.member_identity.as_deref() == Some(runtime_quantity_identity.as_str())
        })
        .expect("lsp completion entry Количество");
    assert_eq!(
        lsp_identifier.member_identity.as_deref(),
        Some(runtime_identifier_identity.as_str()),
        "LSP completion must expose the same structural member identity as runtime"
    );
    assert_eq!(
        lsp_quantity.member_identity.as_deref(),
        Some(runtime_quantity_identity.as_str()),
        "LSP completion must expose the same quantity member identity as runtime"
    );

    let mcp_members = mcp_member_entries_at_code(completion_fixture, completion_position).await;
    let mcp_identifier = mcp_members
        .iter()
        .find(|entry| {
            entry.name == "Идентификатор"
                && entry.member_identity.as_deref() == Some(runtime_identifier_identity.as_str())
        })
        .expect("mcp members entry Идентификатор");
    let mcp_quantity = mcp_members
        .iter()
        .find(|entry| {
            entry.name == "Количество"
                && entry.member_identity.as_deref() == Some(runtime_quantity_identity.as_str())
        })
        .expect("mcp members entry Количество");
    assert_eq!(
        mcp_identifier.member_identity.as_deref(),
        Some(runtime_identifier_identity.as_str()),
        "MCP members must expose the same structural member identity as runtime"
    );
    assert_eq!(
        mcp_quantity.member_identity.as_deref(),
        Some(runtime_quantity_identity.as_str()),
        "MCP members must expose the same quantity member identity as runtime"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, resolved_fixture).await;

    let resolved_position =
        find_utf16_position_at_marker_tail(resolved_fixture, "ДляHover = S.Идентификатор");
    let hover_text = lsp_hover_text_at(&mut service, &uri, resolved_position).await;
    assert!(
        hover_text.contains("Идентификатор") && hover_text.contains("Строка"),
        "hover must expose structure field name and type, hover={hover_text}"
    );

    let type_name = snapshot_type_name_at_marker(
        &server,
        file_id,
        resolved_fixture,
        "ДляHover = S.Идентификатор",
    )
    .await;
    assert_eq!(
        type_name, "Строка",
        "typed structure type-at-position must expose field type"
    );

    let mcp_type_name = mcp_type_name_at_code(resolved_fixture, resolved_position).await;
    assert_eq!(
        mcp_type_name, type_name,
        "MCP type_at_position must agree with shared runtime type for typed structure field"
    );

    let web_hover_text = web_hover_text_for_code(resolved_fixture, resolved_position).await;
    assert!(
        web_hover_text.contains(&type_name),
        "Web hover must agree with shared runtime type for typed structure field, hover={web_hover_text}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентифкатор")),
        "typed structure typo must produce unknown-member diagnostic, diagnostics={diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|message| !message_has_unknown_member(message, "Идентификатор")),
        "typed structure exact field must not regress to unknown-member diagnostic, diagnostics={diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_typed_value_table_row_exact_cross_consumer_acceptance_keeps_same_contract_for_completion_hover_type_and_diagnostics(
) {
    let completion_fixture = "Процедура Тест()\n\
    ТЗ = Новый ТаблицаЗначений;\n\
    ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n\
    ТЗ.Колонки.Добавить(\"Количество\", Новый ОписаниеТипов(\"Число\"));\n\
    Стр = ТЗ.Добавить();\n\
    ДляCompletion = Стр.\n\
КонецПроцедуры\n";
    let resolved_fixture = "Процедура Тест()\n\
    ТЗ = Новый ТаблицаЗначений;\n\
    ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n\
    ТЗ.Колонки.Добавить(\"Количество\", Новый ОписаниеТипов(\"Число\"));\n\
    Стр = ТЗ.Добавить();\n\
    ДляHover = Стр.Идентификатор;\n\
    Ошибка = Стр.Идентифкатор;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        completion_fixture,
        "file:///test_p7_typed_value_table_row_exact_acceptance.bsl",
    )
    .await;

    let completion_position =
        find_utf16_position_after_marker(completion_fixture, "ДляCompletion = Стр.");
    let completion_members =
        lsp_completion_members_at(&mut service, &uri, completion_position).await;
    let completion_labels = completion_members
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    assert!(
        completion_labels
            .iter()
            .any(|label| label == "Идентификатор"),
        "completion must include typed-row column Идентификатор, labels={completion_labels:?}"
    );
    assert!(
        completion_labels.iter().any(|label| label == "Количество"),
        "completion must include typed-row column Количество, labels={completion_labels:?}"
    );

    let runtime_resolution = snapshot_type_resolution_at_marker(
        &server,
        file_id,
        completion_fixture,
        "ДляCompletion = Стр",
    )
    .await;
    let runtime_identifier_identity = runtime_resolution
        .find_structural_member("Идентификатор")
        .map(|member| member.member_id.key.clone())
        .expect("runtime typed-row identity for Идентификатор");
    let runtime_quantity_identity = runtime_resolution
        .find_structural_member("Количество")
        .map(|member| member.member_id.key.clone())
        .expect("runtime typed-row identity for Количество");

    let lsp_identifier = completion_members
        .iter()
        .find(|entry| {
            entry.name == "Идентификатор"
                && entry.member_identity.as_deref() == Some(runtime_identifier_identity.as_str())
        })
        .expect("lsp completion entry Идентификатор");
    let lsp_quantity = completion_members
        .iter()
        .find(|entry| {
            entry.name == "Количество"
                && entry.member_identity.as_deref() == Some(runtime_quantity_identity.as_str())
        })
        .expect("lsp completion entry Количество");
    assert_eq!(
        lsp_identifier.member_identity.as_deref(),
        Some(runtime_identifier_identity.as_str()),
        "LSP completion must expose the same typed-row member identity as runtime"
    );
    assert_eq!(
        lsp_quantity.member_identity.as_deref(),
        Some(runtime_quantity_identity.as_str()),
        "LSP completion must expose the same typed-row quantity identity as runtime"
    );

    let mcp_members = mcp_member_entries_at_code(completion_fixture, completion_position).await;
    let mcp_identifier = mcp_members
        .iter()
        .find(|entry| {
            entry.name == "Идентификатор"
                && entry.member_identity.as_deref() == Some(runtime_identifier_identity.as_str())
        })
        .expect("mcp members entry Идентификатор");
    let mcp_quantity = mcp_members
        .iter()
        .find(|entry| {
            entry.name == "Количество"
                && entry.member_identity.as_deref() == Some(runtime_quantity_identity.as_str())
        })
        .expect("mcp members entry Количество");
    assert_eq!(
        mcp_identifier.member_identity.as_deref(),
        Some(runtime_identifier_identity.as_str()),
        "MCP members must expose the same typed-row member identity as runtime"
    );
    assert_eq!(
        mcp_quantity.member_identity.as_deref(),
        Some(runtime_quantity_identity.as_str()),
        "MCP members must expose the same typed-row quantity identity as runtime"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, resolved_fixture).await;

    let resolved_position =
        find_utf16_position_at_marker_tail(resolved_fixture, "ДляHover = Стр.Идентификатор");
    let hover_text = lsp_hover_text_at(&mut service, &uri, resolved_position).await;
    assert!(
        hover_text.contains("Идентификатор") && hover_text.contains("Строка"),
        "hover must expose typed-row column name and type, hover={hover_text}"
    );

    let type_name = snapshot_type_name_at_marker(
        &server,
        file_id,
        resolved_fixture,
        "ДляHover = Стр.Идентификатор",
    )
    .await;
    assert_eq!(
        type_name, "Строка",
        "typed-row type-at-position must expose column type"
    );

    let mcp_type_name = mcp_type_name_at_code(resolved_fixture, resolved_position).await;
    assert_eq!(
        mcp_type_name, type_name,
        "MCP type_at_position must agree with shared runtime type for typed-row field"
    );

    let web_hover_text = web_hover_text_for_code(resolved_fixture, resolved_position).await;
    assert!(
        web_hover_text.contains(&type_name),
        "Web hover must agree with shared runtime type for typed-row field, hover={web_hover_text}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентифкатор")),
        "typed-row typo must produce unknown-member diagnostic, diagnostics={diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|message| !message_has_unknown_member(message, "Идентификатор")),
        "typed-row exact column must not regress to unknown-member diagnostic, diagnostics={diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_form_module_object_completion_uses_default_lsp_owner_hint_path() {
    let fixture = "Процедура Тест()\n\
    ДляCompletion = Объект.\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture,
        "file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl",
    )
    .await;

    let completion_position = find_utf16_position_after_marker(fixture, "ДляCompletion = Объект.");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        completion_labels.iter().any(|label| label == "Ссылка"),
        "default LSP completion for FormModule.Объект must include form-data property Ссылка, labels={completion_labels:?}"
    );
    assert!(
        completion_labels
            .iter()
            .any(|label| label == "ПометкаУдаления"),
        "default LSP completion for FormModule.Объект must include form-data property ПометкаУдаления, labels={completion_labels:?}"
    );
    assert!(
        !completion_labels
            .iter()
            .any(|label| label == "ПолучитьСсылкуНового"),
        "default LSP completion for FormModule.Объект must not expose applied object-facet method ПолучитьСсылкуНового, labels={completion_labels:?}"
    );

    let type_name =
        snapshot_type_name_at_marker(&server, file_id, fixture, "ДляCompletion = Объект").await;
    assert_eq!(
        type_name, "ДанныеФормыСтруктура",
        "default LSP path must keep shared form-data type at implicit Объект"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_typed_structure_revision_switch_does_not_leak_stale_structural_members_across_interfaces(
) {
    let fixture_v1 = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    ДляCompletion = S.\n\
КонецПроцедуры\n";
    let fixture_v2 = "Процедура Тест()\n\
    S = Новый Структура;\n\
    ДляCompletion = S.\n\
    Ошибка = S.Идентификатор;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture_v1,
        "file:///test_p7_typed_structure_revision_switch.bsl",
    )
    .await;

    let v1_completion_position = find_utf16_position_after_marker(fixture_v1, "ДляCompletion = S.");
    let v1_completion_members =
        lsp_completion_members_at(&mut service, &uri, v1_completion_position).await;
    assert!(
        v1_completion_members.iter().any(|entry| {
            entry.name == "Идентификатор" && entry.member_identity.is_some()
        }),
        "v1 completion must expose structural member identity before revision switch"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, fixture_v2).await;

    let runtime_resolution =
        snapshot_type_resolution_at_marker(&server, file_id, fixture_v2, "ДляCompletion = S").await;
    assert!(
        runtime_resolution
            .find_structural_member("идентификатор")
            .is_none(),
        "runtime snapshot after revision switch must not leak stale structure field"
    );

    let v2_completion_position = find_utf16_position_after_marker(fixture_v2, "ДляCompletion = S.");
    let v2_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, v2_completion_position).await;
    assert!(
        !v2_completion_labels
            .iter()
            .any(|label| label == "Идентификатор"),
        "LSP completion must fail closed after revision switch, labels={v2_completion_labels:?}"
    );

    let mcp_members = mcp_member_entries_at_code(fixture_v2, v2_completion_position).await;
    assert!(
        !mcp_members.iter().any(|entry| entry.name == "Идентификатор"),
        "MCP members must not leak stale structure field after revision switch, members={mcp_members:?}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентификатор")),
        "runtime/LSP diagnostics must surface stale structure field as unknown-member, diagnostics={diagnostics:?}"
    );

    let web_diagnostics = web_semantic_diagnostic_messages_for_code(fixture_v2).await;
    assert!(
        web_diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентификатор")),
        "Web diagnostics must surface stale structure field as unknown-member, diagnostics={web_diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_typed_value_table_row_revision_switch_does_not_leak_stale_structural_members_across_interfaces(
) {
    let fixture_v1 = "Процедура Тест()\n\
    ТЗ = Новый ТаблицаЗначений;\n\
    ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n\
    Стр = ТЗ.Добавить();\n\
    ДляCompletion = Стр.\n\
КонецПроцедуры\n";
    let fixture_v2 = "Процедура Тест()\n\
    ТЗ = Новый ТаблицаЗначений;\n\
    Стр = ТЗ.Добавить();\n\
    ДляCompletion = Стр.\n\
    Ошибка = Стр.Идентификатор;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture_v1,
        "file:///test_p7_typed_value_table_row_revision_switch.bsl",
    )
    .await;

    let v1_completion_position =
        find_utf16_position_after_marker(fixture_v1, "ДляCompletion = Стр.");
    let v1_completion_members =
        lsp_completion_members_at(&mut service, &uri, v1_completion_position).await;
    assert!(
        v1_completion_members.iter().any(|entry| {
            entry.name == "Идентификатор" && entry.member_identity.is_some()
        }),
        "v1 typed-row completion must expose structural member identity before revision switch"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, fixture_v2).await;

    let runtime_resolution =
        snapshot_type_resolution_at_marker(&server, file_id, fixture_v2, "ДляCompletion = Стр")
            .await;
    assert!(
        runtime_resolution
            .find_structural_member("идентификатор")
            .is_none(),
        "runtime snapshot after revision switch must not leak stale typed-row column"
    );

    let v2_completion_position =
        find_utf16_position_after_marker(fixture_v2, "ДляCompletion = Стр.");
    let v2_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, v2_completion_position).await;
    assert!(
        !v2_completion_labels.iter().any(|label| label == "Идентификатор"),
        "LSP completion must fail closed after typed-row revision switch, labels={v2_completion_labels:?}"
    );

    let mcp_members = mcp_member_entries_at_code(fixture_v2, v2_completion_position).await;
    assert!(
        !mcp_members.iter().any(|entry| entry.name == "Идентификатор"),
        "MCP members must not leak stale typed-row column after revision switch, members={mcp_members:?}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентификатор")),
        "runtime/LSP diagnostics must surface stale typed-row column as unknown-member, diagnostics={diagnostics:?}"
    );

    let web_diagnostics = web_semantic_diagnostic_messages_for_code(fixture_v2).await;
    assert!(
        web_diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентификатор")),
        "Web diagnostics must surface stale typed-row column as unknown-member, diagnostics={web_diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_hover_and_type_at_position_revision_switch_do_not_report_stale_typed_structure_member()
{
    let fixture_v1 = "Процедура Тест()\n\
    S = Новый Структура;\n\
    S.Вставить(\"Идентификатор\", \"A-01\");\n\
    ДляHover = S.Идентификатор;\n\
КонецПроцедуры\n";
    let fixture_v2 = "Процедура Тест()\n\
    S = Новый Структура;\n\
    ДляHover = S.Идентификатор;\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture_v1,
        "file:///test_p7_hover_type_revision_switch_structure.bsl",
    )
    .await;

    let v1_position = find_utf16_position_at_marker_tail(fixture_v1, "ДляHover = S.Идентификатор");
    let v1_hover_text = lsp_hover_text_optional_at(&mut service, &uri, v1_position)
        .await
        .expect("v1 hover text");
    assert!(
        v1_hover_text.contains("Идентификатор") && v1_hover_text.contains("Строка"),
        "v1 hover must expose the exact typed structure field before revision switch, hover={v1_hover_text}"
    );
    let v1_type_name = snapshot_type_name_at_marker_optional(
        &server,
        file_id,
        fixture_v1,
        "ДляHover = S.Идентификатор",
    )
    .await
    .expect("v1 type_at_position");
    assert_eq!(
        v1_type_name, "Строка",
        "v1 type_at_position must expose the exact typed structure field before revision switch"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, fixture_v2).await;

    let v2_position = find_utf16_position_at_marker_tail(fixture_v2, "ДляHover = S.Идентификатор");
    let v2_hover_text = lsp_hover_text_optional_at(&mut service, &uri, v2_position).await;
    if let Some(text) = &v2_hover_text {
        assert!(
            !text.contains("Строка"),
            "LSP hover must not leak stale previous-revision field type after revision switch, hover={text}"
        );
        assert!(
            text.contains("Неопределено") || text.contains("Тип не распознан системой"),
            "non-empty LSP hover after revision switch must describe the current unresolved state instead of stale field semantics, hover={text}"
        );
    }

    let v2_type_name = snapshot_type_name_at_marker_optional(
        &server,
        file_id,
        fixture_v2,
        "ДляHover = S.Идентификатор",
    )
    .await;
    assert_ne!(
        v2_type_name.as_deref(),
        Some("Строка"),
        "runtime type_at_position must not leak stale previous-revision field type after revision switch, type={v2_type_name:?}"
    );

    let web_hover_text = web_hover_text_for_code(fixture_v2, v2_position).await;
    assert!(
        !web_hover_text.contains("Строка"),
        "Web hover must not leak stale previous-revision field type after revision switch, hover={web_hover_text}"
    );
    assert!(
        web_hover_text.is_empty()
            || web_hover_text.contains("Неопределено")
            || web_hover_text.contains("Тип не распознан системой"),
        "non-empty Web hover after revision switch must describe the current unresolved state instead of stale field semantics, hover={web_hover_text}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        diagnostics
            .iter()
            .any(|message| message_has_unknown_member(message, "Идентификатор")),
        "revision-switched typed structure access must produce unknown-member diagnostics, diagnostics={diagnostics:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_definition_revision_switch_does_not_return_stale_previous_revision_location_across_lsp_and_mcp(
) {
    let fixture_v1 = "Процедура Целевой()\n\
КонецПроцедуры\n\
\n\
Процедура Тест()\n\
    Целевой();\n\
КонецПроцедуры\n";
    let fixture_v2 = "Процедура Тест()\n\
    Целевой();\n\
КонецПроцедуры\n";

    let (mut service, drain_task, server, uri, file_id) = open_lsp_fixture_with_snapshot(
        fixture_v1,
        "file:///test_p7_definition_revision_switch_removed_local_target.bsl",
    )
    .await;

    let v1_position = find_utf16_position_after_marker(fixture_v1, "Процедура Тест()\nЦелевой");
    let v1_lsp_definition = lsp_definition_points_at(&mut service, &uri, v1_position).await;
    assert!(
        !v1_lsp_definition.is_empty(),
        "v1 definition must resolve before revision switch, definition={v1_lsp_definition:?}"
    );
    let v1_mcp_definition = mcp_definition_points_at_code(fixture_v1, v1_position).await;
    assert!(
        !v1_mcp_definition.is_empty(),
        "v1 MCP definition must resolve before revision switch, definition={v1_mcp_definition:?}"
    );

    replace_lsp_fixture_and_wait(&mut service, &server, &uri, file_id, 2, fixture_v2).await;

    let v2_position = find_utf16_position_after_marker(fixture_v2, "Процедура Тест()\nЦелевой");
    let v2_lsp_definition = lsp_definition_points_at(&mut service, &uri, v2_position).await;
    assert!(
        v2_lsp_definition.is_empty(),
        "LSP definition must not leak stale previous-revision target location after revision switch, definition={v2_lsp_definition:?}"
    );

    let v2_mcp_definition = mcp_definition_points_at_code(fixture_v2, v2_position).await;
    assert!(
        v2_mcp_definition.is_empty(),
        "MCP definition must not leak stale previous-revision target location for the current code, definition={v2_mcp_definition:?}"
    );

    let diagnostics = snapshot_semantic_diagnostic_messages(&server, file_id).await;
    assert!(
        !diagnostics.is_empty(),
        "removed local target must surface current-revision diagnostics instead of silently reusing stale semantics"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_completion_owner_hint_type_lookup_is_serve_only_even_when_flow_sensitive_enabled() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let settings = DidChangeConfigurationParams {
        settings: serde_json::json!({
            "bsl": {
                "hover": {
                    "detailLevel": "full",
                    "maxMethods": 10,
                    "maxProperties": 5,
                    "showCertainty": true
                },
                "diagnostics": {
                    "detailLevel": "standard",
                    "showHints": true
                },
                "formatting": {
                    "enabled": false,
                    "indentSize": 4
                },
                "typeHints": {
                    "enabled": true,
                    "showVariableTypes": true,
                    "showReturnTypes": true,
                    "showUnionDetails": true,
                    "minCertainty": 0.5
                },
                "codeActions": {
                    "enabled": false
                },
                "enableFlowSensitive": true
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_response = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_response.is_none(),
        "didChangeConfiguration is a notification"
    );

    let fixture =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_owner_hint_serve_only.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let completion_character = "    ЛокМассив."
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;
    let completion = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/completion")
                .id(9001)
                .params(
                    serde_json::to_value(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: Position::new(2, completion_character),
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
        .expect("completion request");
    assert!(completion.is_some(), "completion must return a response");

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");

    let lookup_path_direct_total = read_u64_metric(
        counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_direct"),
    );
    let lookup_path_flow_only_total = read_u64_metric(
        counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_flow_only"),
    );
    let lookup_path_flow_plus_fallback_total = read_u64_metric(
        counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_flow_plus_fallback"),
    );
    assert!(
        lookup_path_direct_total > 0,
        "owner-hint type lookup must stay on direct serve-only path under flow-sensitive mode"
    );
    assert_eq!(
        lookup_path_flow_only_total, 0,
        "flow-only owner-hint lookup path must not run in strict serve-only mode"
    );
    assert_eq!(
        lookup_path_flow_plus_fallback_total, 0,
        "flow+fallback owner-hint lookup path must not run in strict serve-only mode"
    );
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_drilldown_stage_total_origin_lsp_mode_event_driven_operation_completion_stage_parse_result_query",
        )),
        0,
        "strict serve-only completion path must not execute parse_result query stage"
    );

    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total",
        ),),
        0,
        "serve-only owner-hint path must not execute type_index compute in request path"
    );
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total",
            ),
        ),
        0,
        "serve-only owner-hint path must not execute parse_result compute in request path"
    );
    assert_eq!(
        read_u64_metric(
            counters
                .get("intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total"),
        ),
        0,
        "serve-only owner-hint path must not block on type_index compute"
    );
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total",
            ),
        ),
        0,
        "serve-only owner-hint path must not block on parse_result compute"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p7_interactive_default_paths_emit_shared_type_index_reasons_on_exact_lookups() {
    const TYPE_INDEX_REASON_PREFIX: &str = "intellisense_v2_type_index_reason_total_reason_";
    const INTERACTIVE_REASON_COUNTER_KEYS: &[&str] = &[
        "intellisense_v2_type_index_reason_total_reason_type_index_exact_hit",
        "intellisense_v2_type_index_reason_total_reason_type_index_fallback_unavailable",
        "intellisense_v2_type_index_reason_total_reason_other",
    ];
    const ALLOWED_TYPE_INDEX_REASON_SUFFIXES: &[&str] = &[
        "type_index_exact_hit",
        "type_index_fallback_unavailable",
        "type_index_precompute_exact_stored",
        "type_index_precompute_superseded",
        "type_index_precompute_cancelled",
        "type_index_precompute_missing_file",
        "type_index_precompute_queue_saturated",
        "type_index_artifact_invalidated_deps",
        "type_index_artifact_invalidated_settings",
        "type_index_artifact_evicted_global_guard",
        "type_index_artifact_evicted_per_file_window",
        "other",
    ];

    let interactive_reason_total = |coordinator: &Arc<SystemCoordinator>| -> u64 {
        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        INTERACTIVE_REASON_COUNTER_KEYS
            .iter()
            .map(|key| read_u64_metric(counters.get(*key)))
            .sum()
    };

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let fixture = "Процедура Тест()\n\
МассивДляCompletion = Новый Массив;\n\
МассивДляCompletion.\n\
\n\
МассивДляHover = Новый Массив;\n\
ЗначДляHover = МассивДляHover.Количество();\n\
\n\
МассивДляSignature = Новый Массив;\n\
МассивДляSignature.Количество();\n\
\n\
МассивДляDefinition = Новый Массив;\n\
МассивДляDefinition.Количество();\n\
КонецПроцедуры\n";
    let uri = Url::parse("file:///test_p7_interactive_type_index_reasons.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: fixture.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let completion_position = find_utf16_position_after_marker(fixture, "МассивДляCompletion.");
    let hover_position = find_utf16_position_after_marker(fixture, "ЗначДляHover = МассивДляHover");
    let signature_position =
        find_utf16_position_after_marker(fixture, "МассивДляSignature.Количество(");
    let definition_position =
        find_utf16_position_after_marker(fixture, "МассивДляDefinition.Количество");

    let before_completion = interactive_reason_total(&coordinator);
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/completion")
                .id(9101)
                .params(
                    serde_json::to_value(CompletionParams {
                        text_document_position: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: completion_position,
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
        .expect("completion request");
    assert!(
        completion_response.is_some(),
        "completion must return a response"
    );
    let after_completion = interactive_reason_total(&coordinator);
    assert!(
        after_completion == before_completion,
        "completion must reuse current semantic state without extra type_index serve reason"
    );

    let before_hover = interactive_reason_total(&coordinator);
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/hover")
                .id(9102)
                .params(
                    serde_json::to_value(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: hover_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .expect("HoverParams"),
                )
                .finish(),
        )
        .await
        .expect("hover request");
    assert!(
        hover_response.is_some(),
        "hover must return a response envelope"
    );
    let after_hover = interactive_reason_total(&coordinator);
    assert!(
        after_hover > before_hover,
        "hover must emit at least one type_index serve reason"
    );

    let before_signature = interactive_reason_total(&coordinator);
    let signature_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/signatureHelp")
                .id(9103)
                .params(
                    serde_json::to_value(tower_lsp::lsp_types::SignatureHelpParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: signature_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        context: None,
                    })
                    .expect("SignatureHelpParams"),
                )
                .finish(),
        )
        .await
        .expect("signatureHelp request");
    assert!(
        signature_response.is_some(),
        "signatureHelp must return a response envelope"
    );
    let after_signature = interactive_reason_total(&coordinator);
    assert!(
        after_signature > before_signature,
        "signatureHelp must emit at least one type_index serve reason on the shared default path"
    );

    let before_definition = interactive_reason_total(&coordinator);
    let definition_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/definition")
                .id(9104)
                .params(
                    serde_json::to_value(GotoDefinitionParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri: uri.clone() },
                            position: definition_position,
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                        partial_result_params: PartialResultParams::default(),
                    })
                    .expect("GotoDefinitionParams"),
                )
                .finish(),
        )
        .await
        .expect("definition request");
    assert!(
        definition_response.is_some(),
        "definition must return a response envelope"
    );
    let after_definition = interactive_reason_total(&coordinator);
    assert!(
        after_definition > before_definition,
        "definition must emit at least one type_index serve reason on the shared default path"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let allowed_suffixes: BTreeSet<&str> =
        ALLOWED_TYPE_INDEX_REASON_SUFFIXES.iter().copied().collect();
    for key in counters.keys() {
        if let Some(suffix) = key.strip_prefix(TYPE_INDEX_REASON_PREFIX) {
            assert!(
                allowed_suffixes.contains(suffix),
                "type_index reason escaped bounded taxonomy: key={key}, suffix={suffix}"
            );
        }
    }
    assert_eq!(
        read_u64_metric(counters.get(
            "intellisense_v2_observability_contract_violation_reason_unknown_type_index_reason",
        )),
        0,
        "interactive serve reasons must stay in known taxonomy and not trigger unknown reason violation"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p8_deps_update_is_atomic_and_completion_uses_runtime_index_snapshot() {
    fn make_index_snapshot(id: &str, type_name: &str) -> IndexSnapshot {
        let mut snapshot = IndexSnapshot::empty(IndexSnapshotId::from_hash(id.to_string()));
        Arc::make_mut(&mut snapshot.type_index).insert(
            type_name.to_string(),
            Arc::new(IndexItem::new(
                type_name.to_string(),
                IndexItemKind::Type(TypeKind::Generic),
                IndexKind::Type,
            )),
        );
        snapshot
    }

    fn extract_completion_labels(
        response: tower_lsp::lsp_types::CompletionResponse,
    ) -> Vec<String> {
        match response {
            tower_lsp::lsp_types::CompletionResponse::Array(items) => {
                items.into_iter().map(|item| item.label).collect()
            }
            tower_lsp::lsp_types::CompletionResponse::List(list) => {
                list.items.into_iter().map(|item| item.label).collect()
            }
        }
    }

    let coordinator = Arc::new(SystemCoordinator::new());
    let server_holder: Arc<std::sync::Mutex<Option<BslLanguageServer>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        let server_holder = server_holder.clone();
        move |client| {
            let server = BslLanguageServer::new(client, coordinator.clone());
            *server_holder.lock().unwrap() = Some(server.clone());
            server
        }
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let server = server_holder
        .lock()
        .unwrap()
        .clone()
        .expect("server must be created");

    let snapshot_a = make_index_snapshot("p8_a", "P8TypeA");
    let snapshot_b = make_index_snapshot("p8_b", "P8TypeB");

    coordinator
        .intellisense_index()
        .replace_snapshot(snapshot_a.clone());
    let expected_deps_id_a = build_deps_bundle_v2(coordinator.as_ref(), None, None)
        .expect("bundle A")
        .deps_id;

    coordinator
        .intellisense_index()
        .replace_snapshot(snapshot_b.clone());
    let expected_deps_id_b = build_deps_bundle_v2(coordinator.as_ref(), None, None)
        .expect("bundle B")
        .deps_id;

    coordinator
        .intellisense_index()
        .replace_snapshot(snapshot_a.clone());
    server.deps_update_v2("p8_test_initial", None, None).await;

    let uri = Url::parse("file:///test_p8.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Procedure Test()\n\t// P8\nEndProcedure".to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 1,
                character: 6,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };

    let completion_a = server
        .completion(completion_params.clone())
        .await
        .expect("completion")
        .expect("completion response");
    let labels_a = extract_completion_labels(completion_a);
    assert!(
        labels_a.iter().any(|label| label == "P8TypeA"),
        "expected completion to contain P8TypeA, got {:?}",
        labels_a
    );
    assert!(
        labels_a.iter().all(|label| label != "P8TypeB"),
        "unexpected P8TypeB in completion A: {:?}",
        labels_a
    );

    let update_task = tokio::spawn({
        let coordinator = coordinator.clone();
        let server = server.clone();
        let snapshot_b = snapshot_b.clone();
        async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            coordinator
                .intellisense_index()
                .replace_snapshot(snapshot_b);
            server.deps_update_v2("p8_test_update", None, None).await;
        }
    });

    for _ in 0..200 {
        let (_analysis, index_snapshot, deps_id) = server.analysis_v2.snapshot_with_deps().await;
        match index_snapshot.id.as_str() {
            "p8_a" => assert_eq!(deps_id.as_str(), expected_deps_id_a.as_str()),
            "p8_b" => assert_eq!(deps_id.as_str(), expected_deps_id_b.as_str()),
            other => panic!("unexpected index snapshot id: {}", other),
        }
    }

    update_task.await.expect("update task join");

    let completion_b = server
        .completion(completion_params)
        .await
        .expect("completion")
        .expect("completion response");
    let labels_b = extract_completion_labels(completion_b);
    assert!(
        labels_b.iter().any(|label| label == "P8TypeB"),
        "expected completion to contain P8TypeB, got {:?}",
        labels_b
    );
    assert!(
        labels_b.iter().all(|label| label != "P8TypeA"),
        "unexpected P8TypeA in completion B: {:?}",
        labels_b
    );

    drain_task.abort();
}

#[tokio::test]
async fn p9a_formatting_disabled_does_not_advertise_capability_and_returns_null() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    let response = response.expect("initialize should return a response");

    let response_value = serde_json::to_value(&response).expect("serialize initialize response");
    let capabilities = response_value
        .get("result")
        .and_then(|v| v.get("capabilities"))
        .expect("initialize capabilities");

    match capabilities.get("documentFormattingProvider") {
        None => {}
        Some(v) => assert!(
            v.is_null(),
            "documentFormattingProvider must be absent/null"
        ),
    }
    match capabilities.get("documentRangeFormattingProvider") {
        None => {}
        Some(v) => assert!(
            v.is_null(),
            "documentRangeFormattingProvider must be absent/null"
        ),
    }

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///test_p9a_formatting_disabled.bsl").expect("test uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Процедура Тест()\nКонецПроцедуры\n".to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let formatting_params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        options: FormattingOptions::default(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let formatting_req = Request::build("textDocument/formatting")
        .id(2)
        .params(serde_json::to_value(formatting_params).expect("DocumentFormattingParams"))
        .finish();
    let formatting_response = service
        .ready()
        .await
        .unwrap()
        .call(formatting_req)
        .await
        .expect("formatting request");
    let formatting_response = formatting_response.expect("formatting should return a response");

    let response_value =
        serde_json::to_value(&formatting_response).expect("serialize formatting response");
    match response_value.get("error") {
        None => {}
        Some(v) => assert!(v.is_null(), "formatting must not return an error"),
    }
    let result = response_value
        .get("result")
        .cloned()
        .expect("formatting result field");
    assert!(
        result.is_null(),
        "disabled formatting should return null edits"
    );

    let range_formatting_params = DocumentRangeFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        options: FormattingOptions::default(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let range_req = Request::build("textDocument/rangeFormatting")
        .id(3)
        .params(
            serde_json::to_value(range_formatting_params).expect("DocumentRangeFormattingParams"),
        )
        .finish();
    let range_response = service
        .ready()
        .await
        .unwrap()
        .call(range_req)
        .await
        .expect("rangeFormatting request");
    let range_response = range_response.expect("rangeFormatting should return a response");

    let range_value =
        serde_json::to_value(&range_response).expect("serialize rangeFormatting response");
    match range_value.get("error") {
        None => {}
        Some(v) => assert!(v.is_null(), "rangeFormatting must not return an error"),
    }
    let range_result = range_value
        .get("result")
        .cloned()
        .expect("rangeFormatting result field");
    assert!(
        range_result.is_null(),
        "disabled rangeFormatting should return null edits"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p9_formatting_reindents_and_trims_when_enabled() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    // LSP initialize handshake is required, otherwise client notifications are suppressed.
    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    // Enable formatting through didChangeConfiguration (section `bsl`).
    let settings = DidChangeConfigurationParams {
        settings: serde_json::json!({
            "bsl": {
                "hover": {
                    "detailLevel": "full",
                    "maxMethods": 10,
                    "maxProperties": 5,
                    "showCertainty": true
                },
                "diagnostics": {
                    "detailLevel": "standard",
                    "showHints": true
                },
                "formatting": {
                    "enabled": true,
                    "indentSize": 4
                }
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_resp = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_resp.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///test_p9_formatting.bsl").expect("test uri");
    let text = "Процедура Тест()\nЕсли Истина Тогда  \nСообщить(1);\nИначе\nСообщить(2);   \nКонецЕсли;\nКонецПроцедуры\n";

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let formatting_params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        options: FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            ..Default::default()
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let formatting_req = Request::build("textDocument/formatting")
        .id(2)
        .params(serde_json::to_value(formatting_params).expect("DocumentFormattingParams"))
        .finish();
    let formatting_response = service
        .ready()
        .await
        .unwrap()
        .call(formatting_req)
        .await
        .expect("formatting request");
    let formatting_response = formatting_response.expect("formatting should return a response");

    let response_value =
        serde_json::to_value(&formatting_response).expect("serialize formatting response");
    let edits_value = response_value
        .get("result")
        .cloned()
        .expect("formatting result field");
    let edits: Option<Vec<tower_lsp::lsp_types::TextEdit>> =
        serde_json::from_value(edits_value).expect("parse edits");
    let edits = edits.expect("edits present");
    assert!(!edits.is_empty(), "formatting must return edits");

    // Apply per-line edits (formatter emits full-line replacements).
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    for edit in edits {
        let line = edit.range.start.line as usize;
        lines[line] = edit.new_text;
    }
    let formatted = lines.join("\n");

    let expected = "Процедура Тест()\n    Если Истина Тогда\n        Сообщить(1);\n    Иначе\n        Сообщить(2);\n    КонецЕсли;\nКонецПроцедуры\n";
    assert_eq!(formatted, expected);

    drain_task.abort();
}

#[tokio::test]
async fn p10_range_formatting_only_updates_selected_lines() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let settings = DidChangeConfigurationParams {
        settings: serde_json::json!({
            "bsl": {
                "hover": {
                    "detailLevel": "full",
                    "maxMethods": 10,
                    "maxProperties": 5,
                    "showCertainty": true
                },
                "diagnostics": {
                    "detailLevel": "standard",
                    "showHints": true
                },
                "formatting": {
                    "enabled": true,
                    "indentSize": 4
                }
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_resp = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_resp.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///test_p10_range_formatting.bsl").expect("test uri");
    let text = concat!(
        "Процедура Тест()\n",
        "    Сообщить(\"a\");\n",
        "Если Истина Тогда\n",
        "Сообщить(1);\n",
        "КонецЕсли;\n",
        "    Сообщить(\"b\");\n",
        "КонецПроцедуры\n",
    );

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let range_formatting_params = DocumentRangeFormattingParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range {
            start: Position {
                line: 2,
                character: 0,
            },
            end: Position {
                line: 5,
                character: 0,
            },
        },
        options: FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            ..Default::default()
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let range_req = Request::build("textDocument/rangeFormatting")
        .id(2)
        .params(
            serde_json::to_value(range_formatting_params).expect("DocumentRangeFormattingParams"),
        )
        .finish();

    let response_a = service
        .ready()
        .await
        .unwrap()
        .call(range_req)
        .await
        .expect("rangeFormatting request");
    let response_a = response_a.expect("rangeFormatting should return a response");

    let response_value =
        serde_json::to_value(&response_a).expect("serialize rangeFormatting response");
    let edits_value = response_value
        .get("result")
        .cloned()
        .expect("rangeFormatting result field");
    let edits: Option<Vec<tower_lsp::lsp_types::TextEdit>> =
        serde_json::from_value(edits_value).expect("parse edits");
    let edits = edits.expect("edits present");

    assert_eq!(edits.len(), 3, "expected 3 line edits inside the range");
    for edit in &edits {
        assert!(
            (2..=4).contains(&edit.range.start.line),
            "unexpected edit line {:?}",
            edit.range.start.line
        );
    }
    let projected_a: Vec<(u32, String)> = edits
        .iter()
        .map(|edit| (edit.range.start.line, edit.new_text.clone()))
        .collect();

    // Apply per-line edits.
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    for edit in edits {
        let line = edit.range.start.line as usize;
        lines[line] = edit.new_text;
    }
    let formatted = lines.join("\n");

    let expected = concat!(
        "Процедура Тест()\n",
        "    Сообщить(\"a\");\n",
        "    Если Истина Тогда\n",
        "        Сообщить(1);\n",
        "    КонецЕсли;\n",
        "    Сообщить(\"b\");\n",
        "КонецПроцедуры\n",
    );
    assert_eq!(formatted, expected);

    // Determinism: second request returns identical edits.
    let range_req_2 = Request::build("textDocument/rangeFormatting")
        .id(3)
        .params(
            serde_json::to_value(DocumentRangeFormattingParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                range: Range {
                    start: Position {
                        line: 2,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 0,
                    },
                },
                options: FormattingOptions {
                    tab_size: 4,
                    insert_spaces: true,
                    ..Default::default()
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .expect("DocumentRangeFormattingParams"),
        )
        .finish();

    let response_b = service
        .ready()
        .await
        .unwrap()
        .call(range_req_2)
        .await
        .expect("rangeFormatting request (2)");
    let response_b = response_b.expect("rangeFormatting (2) should return a response");

    let value_b = serde_json::to_value(&response_b).expect("serialize response");
    let edits_b_value = value_b.get("result").cloned().expect("result field");
    let edits_b: Option<Vec<tower_lsp::lsp_types::TextEdit>> =
        serde_json::from_value(edits_b_value).expect("parse edits");
    let edits_b = edits_b.expect("edits present");
    let projected_b: Vec<(u32, String)> = edits_b
        .iter()
        .map(|edit| (edit.range.start.line, edit.new_text.clone()))
        .collect();
    assert_eq!(
        projected_b, projected_a,
        "range formatting must be deterministic"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p11_document_symbol_groups_routines_by_region() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///test_p11_document_symbol.bsl").expect("test uri");
    let text = concat!(
        "#Область Public\n",
        "Процедура Inside() Экспорт\n",
        "КонецПроцедуры\n",
        "#КонецОбласти\n",
        "Функция Outside() Экспорт\n",
        "КонецФункции\n",
    );

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    let req = Request::build("textDocument/documentSymbol")
        .id(2)
        .params(serde_json::to_value(params.clone()).expect("DocumentSymbolParams"))
        .finish();
    let response_a = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("documentSymbol request");
    let response_a = response_a.expect("documentSymbol should return a response");

    let value_a = serde_json::to_value(&response_a).expect("serialize response");
    let result_a_value = value_a.get("result").cloned().expect("result field");

    let parsed_a: Option<DocumentSymbolResponse> =
        serde_json::from_value(result_a_value.clone()).expect("parse result");
    let parsed_a = parsed_a.expect("result present");

    let DocumentSymbolResponse::Nested(top_level) = parsed_a else {
        panic!("expected nested document symbols");
    };

    let region = top_level
        .iter()
        .find(|sym| sym.name == "Public")
        .expect("expected region Public");
    assert_eq!(region.kind, SymbolKind::NAMESPACE);

    let children = region.children.as_ref().expect("region must have children");
    let inside = children
        .iter()
        .find(|sym| sym.name == "Inside")
        .expect("expected Inside");
    assert_eq!(inside.kind, SymbolKind::METHOD);
    assert_eq!(inside.detail.as_deref(), Some("export"));
    assert_eq!(inside.range.start.line, 1);
    assert_eq!(inside.selection_range.start.line, 1);
    assert_eq!(inside.selection_range.start.character, 10);
    assert_eq!(inside.selection_range.end.character, 16);

    let outside = top_level
        .iter()
        .find(|sym| sym.name == "Outside")
        .expect("expected Outside");
    assert_eq!(outside.kind, SymbolKind::FUNCTION);
    assert_eq!(outside.detail.as_deref(), Some("export"));
    assert_eq!(outside.selection_range.start.line, 4);
    assert_eq!(outside.selection_range.start.character, 8);
    assert_eq!(outside.selection_range.end.character, 15);

    // Determinism: second request returns identical JSON result.
    let req_2 = Request::build("textDocument/documentSymbol")
        .id(3)
        .params(serde_json::to_value(params).expect("DocumentSymbolParams"))
        .finish();
    let response_b = service
        .ready()
        .await
        .unwrap()
        .call(req_2)
        .await
        .expect("documentSymbol request (2)");
    let response_b = response_b.expect("documentSymbol (2) should return a response");
    let value_b = serde_json::to_value(&response_b).expect("serialize response");
    let result_b_value = value_b.get("result").cloned().expect("result field");
    assert_eq!(
        result_a_value, result_b_value,
        "documentSymbol must be deterministic"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p12_workspace_symbol_searches_open_documents() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri_a = Url::parse("file:///test_p12_a.bsl").expect("test uri a");
    let uri_b = Url::parse("file:///test_p12_b.bsl").expect("test uri b");

    let did_open_a = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri_a.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Процедура FooOne() Экспорт\nКонецПроцедуры\n".to_string(),
        },
    };
    let did_open_b = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri_b.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Функция FooTwo() Экспорт\nКонецФункции\n".to_string(),
        },
    };

    for did_open in [did_open_a, did_open_b] {
        let req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let resp = service
            .ready()
            .await
            .unwrap()
            .call(req)
            .await
            .expect("didOpen notification");
        assert!(resp.is_none(), "didOpen is a notification");
    }

    let params = WorkspaceSymbolParams {
        query: "Foo".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let req = Request::build("workspace/symbol")
        .id(2)
        .params(serde_json::to_value(params).expect("WorkspaceSymbolParams"))
        .finish();

    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("workspace/symbol request");
    let response = response.expect("workspace/symbol should return a response");

    let value = serde_json::to_value(&response).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<Vec<SymbolInformation>> =
        serde_json::from_value(result_value).expect("parse result");
    let parsed = parsed.expect("result present");

    assert!(
        parsed
            .iter()
            .any(|sym| sym.name == "FooOne" && sym.location.uri == uri_a),
        "expected FooOne in uri_a, got {:?}",
        parsed
            .iter()
            .map(|s| (s.name.clone(), s.location.uri.clone()))
            .collect::<Vec<_>>()
    );
    assert!(
        parsed
            .iter()
            .any(|sym| sym.name == "FooTwo" && sym.location.uri == uri_b),
        "expected FooTwo in uri_b, got {:?}",
        parsed
            .iter()
            .map(|s| (s.name.clone(), s.location.uri.clone()))
            .collect::<Vec<_>>()
    );

    drain_task.abort();
}

#[tokio::test]
async fn p13_unclosed_region_is_closed_at_eof() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///test_p13_unclosed_region.bsl").expect("test uri");
    let text = concat!(
        "#Область Unclosed\n",
        "Процедура Inside() Экспорт\n",
        "КонецПроцедуры\n",
    );

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let req = Request::build("textDocument/documentSymbol")
        .id(2)
        .params(
            serde_json::to_value(DocumentSymbolParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .expect("DocumentSymbolParams"),
        )
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("documentSymbol request");
    let response = response.expect("documentSymbol should return a response");

    let value = serde_json::to_value(&response).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<DocumentSymbolResponse> =
        serde_json::from_value(result_value).expect("parse result");
    let parsed = parsed.expect("result present");

    let DocumentSymbolResponse::Nested(top_level) = parsed else {
        panic!("expected nested document symbols");
    };

    let region = top_level
        .iter()
        .find(|sym| sym.name == "Unclosed")
        .expect("expected region Unclosed");
    assert_eq!(region.kind, SymbolKind::NAMESPACE);
    assert_eq!(
        region.range.end,
        Position {
            line: 3,
            character: 0,
        },
        "unclosed region should be closed at EOF"
    );

    let children = region.children.as_ref().expect("region must have children");
    assert!(
        children.iter().any(|sym| sym.name == "Inside"),
        "expected Inside inside Unclosed region"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p14_references_returns_local_var_locations_and_respects_include_declaration() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///test_p14_references.bsl").expect("test uri");
    let text = concat!(
        "Процедура T()\n",
        "    Перем X;\n",
        "    X = 1;\n",
        "    Сообщить(X);\n",
        "КонецПроцедуры\n",
    );

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let params_with_decl = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 2,
                character: 4,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration: true,
        },
    };

    let req_with_decl = Request::build("textDocument/references")
        .id(2)
        .params(serde_json::to_value(params_with_decl).expect("ReferenceParams"))
        .finish();
    let response_with_decl = service
        .ready()
        .await
        .unwrap()
        .call(req_with_decl)
        .await
        .expect("references request");
    let response_with_decl = response_with_decl.expect("references should return a response");

    let value = serde_json::to_value(&response_with_decl).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<Vec<Location>> = serde_json::from_value(result_value).expect("parse result");
    let parsed = parsed.expect("result present");

    assert_eq!(parsed.len(), 3, "expected declaration + 2 usages");
    assert!(
        parsed.iter().any(|loc| loc.range
            == Range {
                start: Position {
                    line: 1,
                    character: 10
                },
                end: Position {
                    line: 1,
                    character: 11
                }
            }),
        "expected declaration location for X"
    );
    assert!(
        parsed.iter().any(|loc| loc.range
            == Range {
                start: Position {
                    line: 2,
                    character: 4
                },
                end: Position {
                    line: 2,
                    character: 5
                }
            }),
        "expected assignment target usage for X"
    );
    assert!(
        parsed.iter().any(|loc| loc.range
            == Range {
                start: Position {
                    line: 3,
                    character: 13
                },
                end: Position {
                    line: 3,
                    character: 14
                }
            }),
        "expected call argument usage for X"
    );

    let params_no_decl = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 2,
                character: 4,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration: false,
        },
    };

    let req_no_decl = Request::build("textDocument/references")
        .id(3)
        .params(serde_json::to_value(params_no_decl).expect("ReferenceParams"))
        .finish();
    let response_no_decl = service
        .ready()
        .await
        .unwrap()
        .call(req_no_decl)
        .await
        .expect("references request (no decl)");
    let response_no_decl = response_no_decl.expect("references (no decl) should return a response");
    let value = serde_json::to_value(&response_no_decl).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<Vec<Location>> = serde_json::from_value(result_value).expect("parse result");
    let parsed = parsed.expect("result present");
    assert_eq!(parsed.len(), 2, "expected 2 usages without declaration");

    drain_task.abort();
}

#[tokio::test]
async fn p15_rename_updates_only_target_symbol_and_prepare_rename_is_supported() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///test_p15_rename.bsl").expect("test uri");
    let text = concat!(
        "Процедура T()\n",
        "    Перем X;\n",
        "    Перем XX;\n",
        "    X = 1;\n",
        "    XX = 2;\n",
        "    Сообщить(X);\n",
        "    Сообщить(XX);\n",
        "КонецПроцедуры\n",
    );

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let prepare_req = Request::build("textDocument/prepareRename")
        .id(2)
        .params(
            serde_json::to_value(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 5,
                    character: 13,
                },
            })
            .expect("TextDocumentPositionParams"),
        )
        .finish();
    let prepare_resp = service
        .ready()
        .await
        .unwrap()
        .call(prepare_req)
        .await
        .expect("prepareRename request");
    let prepare_resp = prepare_resp.expect("prepareRename should return a response");
    let value = serde_json::to_value(&prepare_resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<PrepareRenameResponse> =
        serde_json::from_value(result_value).expect("parse prepareRename");
    let parsed = parsed.expect("result present");
    match parsed {
        PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } => {
            assert_eq!(placeholder, "X");
            assert_eq!(
                range,
                Range {
                    start: Position {
                        line: 5,
                        character: 13
                    },
                    end: Position {
                        line: 5,
                        character: 14
                    }
                }
            );
        }
        other => panic!("unexpected prepareRename response: {:?}", other),
    }

    let rename_params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 5,
                character: 13,
            },
        },
        new_name: "Y".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let rename_req = Request::build("textDocument/rename")
        .id(3)
        .params(serde_json::to_value(rename_params).expect("RenameParams"))
        .finish();

    let rename_resp = service
        .ready()
        .await
        .unwrap()
        .call(rename_req)
        .await
        .expect("rename request");
    let rename_resp = rename_resp.expect("rename should return a response");

    let value = serde_json::to_value(&rename_resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<WorkspaceEdit> =
        serde_json::from_value(result_value).expect("parse workspace edit");
    let parsed = parsed.expect("result present");
    let changes = parsed.changes.expect("changes present");
    let edits = changes.get(&uri).expect("edits for uri");
    assert_eq!(edits.len(), 3, "expected declaration + 2 usages for X");
    assert!(
        edits.iter().all(|e| e.new_text == "Y"),
        "all edits must rename to Y"
    );
    assert!(
        edits.iter().all(|e| e.range.start.line != 2),
        "must not touch XX declaration line"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p16_references_returns_routine_declaration_and_calls() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///test_p16_routine_references.bsl").expect("test uri");
    let text = concat!(
        "Процедура Foo() Экспорт\n",
        "КонецПроцедуры\n",
        "\n",
        "Процедура Bar()\n",
        "    Foo();\n",
        "    Foo();\n",
        "КонецПроцедуры\n",
    );

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 4,
                character: 4,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration: true,
        },
    };

    let req = Request::build("textDocument/references")
        .id(2)
        .params(serde_json::to_value(params).expect("ReferenceParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("references request");
    let response = response.expect("references should return a response");

    let value = serde_json::to_value(&response).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<Vec<Location>> = serde_json::from_value(result_value).expect("parse result");
    let parsed = parsed.expect("result present");
    assert_eq!(parsed.len(), 3, "expected declaration + 2 call sites");

    drain_task.abort();
}

#[tokio::test]
async fn p17_rename_routine_updates_declaration_and_calls_only() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let uri = Url::parse("file:///test_p17_routine_rename.bsl").expect("test uri");
    let text = concat!(
        "Процедура Foo() Экспорт\n",
        "КонецПроцедуры\n",
        "Процедура FooX() Экспорт\n",
        "КонецПроцедуры\n",
        "Процедура Bar()\n",
        "    Foo();\n",
        "    FooX();\n",
        "    Foo();\n",
        "КонецПроцедуры\n",
    );

    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let rename_params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 5,
                character: 4,
            },
        },
        new_name: "Baz".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let rename_req = Request::build("textDocument/rename")
        .id(2)
        .params(serde_json::to_value(rename_params).expect("RenameParams"))
        .finish();

    let rename_resp = service
        .ready()
        .await
        .unwrap()
        .call(rename_req)
        .await
        .expect("rename request");
    let rename_resp = rename_resp.expect("rename should return a response");

    let value = serde_json::to_value(&rename_resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let parsed: Option<WorkspaceEdit> =
        serde_json::from_value(result_value).expect("parse workspace edit");
    let parsed = parsed.expect("result present");
    let changes = parsed.changes.expect("changes present");
    let edits = changes.get(&uri).expect("edits for uri");

    assert!(
        edits.iter().all(|e| e.new_text == "Baz"),
        "all edits must rename to Baz"
    );
    assert_eq!(
        edits.len(),
        3,
        "expected declaration + 2 call sites for Foo"
    );
    assert!(
        edits.iter().all(|e| e.range.start.line != 6),
        "must not touch FooX() call"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p18_capabilities_gate_inlay_hints_and_code_actions() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        initialization_options: Some(serde_json::json!({
            "enableTypeHints": true,
            "enableCodeActions": false
        })),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();

    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request")
        .expect("initialize response");

    let response_value = serde_json::to_value(&response).expect("serialize initialize response");
    let caps = response_value
        .get("result")
        .and_then(|v| v.get("capabilities"))
        .expect("initialize result.capabilities");

    assert!(
        caps.get("inlayHintProvider").is_some(),
        "inlayHintProvider must be present when enableTypeHints=true"
    );
    let code_actions = caps.get("codeActionProvider");
    assert!(
        code_actions.is_none() || code_actions.is_some_and(|v| v.is_null()),
        "codeActionProvider must be absent/null when enableCodeActions=false"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p19_inlay_hints_returns_type_hints_when_enabled() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        initialization_options: Some(serde_json::json!({
            "enableTypeHints": true,
            "enableCodeActions": false
        })),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let settings = DidChangeConfigurationParams {
        settings: serde_json::json!({
            "bsl": {
                "hover": {
                    "detailLevel": "full",
                    "maxMethods": 10,
                    "maxProperties": 5,
                    "showCertainty": true
                },
                "diagnostics": {
                    "detailLevel": "standard",
                    "showHints": true
                },
                "formatting": {
                    "enabled": false,
                    "indentSize": 4
                },
                "typeHints": {
                    "enabled": true,
                    "showVariableTypes": true,
                    "showReturnTypes": false,
                    "showUnionDetails": true,
                    "minCertainty": 0.7
                },
                "codeActions": {
                    "enabled": false
                }
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_resp = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_resp.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///test_p19_inlay_hints.bsl").expect("test uri");
    let text = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let params = InlayHintParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range::new(Position::new(0, 0), Position::new(10, 0)),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };
    let req = Request::build("textDocument/inlayHint")
        .id(2)
        .params(serde_json::to_value(params).expect("InlayHintParams"))
        .finish();
    let resp = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("inlayHint request")
        .expect("inlayHint response");

    let value = serde_json::to_value(&resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let hints: Option<Vec<InlayHint>> = serde_json::from_value(result_value).expect("parse hints");
    let hints = hints.expect("hints present");

    assert!(!hints.is_empty(), "expected at least one hint");
    assert!(
        hints.iter().any(
            |hint| matches!(&hint.label, InlayHintLabel::String(text) if text.contains(": Число"))
        ),
        "expected ': Число' hint"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p20_code_actions_return_quickfix_add_type_annotation() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        initialization_options: Some(serde_json::json!({
            "enableTypeHints": true,
            "enableCodeActions": true
        })),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let settings = DidChangeConfigurationParams {
        settings: serde_json::json!({
            "bsl": {
                "hover": {
                    "detailLevel": "full",
                    "maxMethods": 10,
                    "maxProperties": 5,
                    "showCertainty": true
                },
                "diagnostics": {
                    "detailLevel": "standard",
                    "showHints": true
                },
                "formatting": {
                    "enabled": false,
                    "indentSize": 4
                },
                "typeHints": {
                    "enabled": true,
                    "showVariableTypes": true,
                    "showReturnTypes": false,
                    "showUnionDetails": true,
                    "minCertainty": 0.7
                },
                "codeActions": {
                    "enabled": true
                }
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_resp = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_resp.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///test_p20_code_actions.bsl").expect("test uri");
    let text = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let params = CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range::new(Position::new(2, 0), Position::new(2, 5)),
        context: CodeActionContext {
            diagnostics: Vec::new(),
            only: None,
            trigger_kind: None,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let req = Request::build("textDocument/codeAction")
        .id(2)
        .params(serde_json::to_value(params).expect("CodeActionParams"))
        .finish();
    let resp = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("codeAction request")
        .expect("codeAction response");

    let value = serde_json::to_value(&resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let actions: Option<Vec<CodeActionOrCommand>> =
        serde_json::from_value(result_value).expect("parse actions");
    let actions = actions.expect("actions present");

    assert!(
        actions.iter().any(|action| matches!(action, CodeActionOrCommand::CodeAction(action) if action.kind.as_ref() == Some(&tower_lsp::lsp_types::CodeActionKind::QUICKFIX))),
        "expected at least one quickfix action"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p21_code_actions_return_extract_refactor_on_selection() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        initialization_options: Some(serde_json::json!({
            "enableTypeHints": true,
            "enableCodeActions": true
        })),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(response.is_some(), "initialize should return a response");

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let settings = DidChangeConfigurationParams {
        settings: serde_json::json!({
            "bsl": {
                "hover": {
                    "detailLevel": "full",
                    "maxMethods": 10,
                    "maxProperties": 5,
                    "showCertainty": true
                },
                "diagnostics": {
                    "detailLevel": "standard",
                    "showHints": true
                },
                "formatting": {
                    "enabled": false,
                    "indentSize": 4
                },
                "typeHints": {
                    "enabled": true,
                    "showVariableTypes": true,
                    "showReturnTypes": false,
                    "showUnionDetails": true,
                    "minCertainty": 0.7
                },
                "codeActions": {
                    "enabled": true
                }
            }
        }),
    };
    let settings_req = Request::build("workspace/didChangeConfiguration")
        .params(serde_json::to_value(settings).expect("DidChangeConfigurationParams"))
        .finish();
    let settings_resp = service
        .ready()
        .await
        .unwrap()
        .call(settings_req)
        .await
        .expect("didChangeConfiguration notification");
    assert!(
        settings_resp.is_none(),
        "didChangeConfiguration is a notification"
    );

    let uri = Url::parse("file:///test_p21_code_actions.bsl").expect("test uri");
    let text = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let params = CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: Range::new(Position::new(2, 4), Position::new(2, 5)),
        context: CodeActionContext {
            diagnostics: Vec::new(),
            only: None,
            trigger_kind: None,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let req = Request::build("textDocument/codeAction")
        .id(2)
        .params(serde_json::to_value(params).expect("CodeActionParams"))
        .finish();
    let resp = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .expect("codeAction request")
        .expect("codeAction response");

    let value = serde_json::to_value(&resp).expect("serialize response");
    let result_value = value.get("result").cloned().expect("result field");
    let actions: Option<Vec<CodeActionOrCommand>> =
        serde_json::from_value(result_value).expect("parse actions");
    let actions = actions.expect("actions present");

    assert!(
        actions.iter().any(|action| matches!(action, CodeActionOrCommand::CodeAction(action) if action.kind.as_ref() == Some(&tower_lsp::lsp_types::CodeActionKind::REFACTOR_EXTRACT))),
        "expected refactor.extract action"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_unified_stage_contract() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    let initialize_params = InitializeParams {
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .id(1)
        .params(serde_json::to_value(initialize_params).expect("InitializeParams"))
        .finish();
    let initialize_response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .expect("initialize request");
    assert!(
        initialize_response.is_some(),
        "initialize should return a response"
    );

    let initialized = Request::build("initialized")
        .params(serde_json::to_value(InitializedParams {}).expect("InitializedParams"))
        .finish();
    let initialized_response = service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .expect("initialized notification");
    assert!(
        initialized_response.is_none(),
        "initialized is a notification"
    );

    let execute = Request::build("workspace/executeCommand")
        .id(2)
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
    let result = value.get("result").cloned().expect("result field");
    assert_unified_intellisense_v2_stage_contract(&result);

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_syntax_diagnostics_mode_drilldown() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) =
                serde_json::from_value::<tower_lsp::lsp_types::PublishDiagnosticsParams>(params)
            else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///syntax_mode_metrics_fixture.bsl").expect("fixture uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: "Процедура Тест(\nКонецПроцедуры\n".to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");
    let diagnostics = wait_lsp_publish_diagnostics(&mut published_rx, &uri).await;
    assert!(
        !diagnostics.is_empty(),
        "syntax fixture must publish diagnostics before metrics snapshot"
    );

    let execute = Request::build("workspace/executeCommand")
        .id(2)
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
    let result = value.get("result").cloned().expect("result field");
    let metrics = result.get("metrics").expect("metrics field");
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    let drilldown_counter = counters
        .get(
            "intellisense_v2_drilldown_stage_total_origin_lsp_mode_full_operation_diagnostics_stage_syntax_diagnostics_query",
        )
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let aggregate_counter = counters
        .get("intellisense_v2_syntax_diagnostics_query_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let drilldown_hist_count = histograms
        .get(
            "intellisense_v2_drilldown_stage_latency_ms_origin_lsp_mode_full_operation_diagnostics_stage_syntax_diagnostics_query",
        )
        .and_then(|value| value.get("count"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let aggregate_hist_count = histograms
        .get("intellisense_v2_syntax_diagnostics_query_ms")
        .and_then(|value| value.get("count"))
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    assert!(
        drilldown_counter > 0,
        "metrics endpoint must expose syntax_diagnostics stage drilldown by parse mode"
    );
    assert_eq!(
        aggregate_counter, drilldown_counter,
        "legacy aggregate total must remain in parity with syntax_diagnostics mode drilldown"
    );
    assert!(
        drilldown_hist_count > 0,
        "metrics endpoint must expose syntax_diagnostics latency histogram by parse mode"
    );
    assert_eq!(
        aggregate_hist_count, drilldown_hist_count,
        "legacy aggregate latency must remain in parity with syntax_diagnostics mode drilldown"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_sidebar_shape_filters_unrelated_histograms() {
    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator.record_completion_latency(Duration::from_millis(33));
    coordinator.record_intellisense_v2_ir_query_latency("completion", Duration::from_millis(21));

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let execute = Request::build("workspace/executeCommand")
        .id(2203)
        .params(serde_json::json!({
            "command": "bsl.getObservabilityMetrics",
            "arguments": [{
                "shape": "sidebar",
            }],
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
    let result = value.get("result").cloned().expect("result field");
    let metrics = result.get("metrics").expect("metrics field");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    assert!(
        histograms.contains_key("intellisense_v2_ir_query_completion_ms"),
        "sidebar shape must keep key completion IR histogram"
    );
    assert!(
        !histograms.contains_key("completion_duration_ms"),
        "sidebar shape must omit unrelated full histograms"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_semantic_diagnostics_breakdown() {
    const SEMANTIC_FIXTURE: &str = "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.НесуществующийМетод();\nКонецПроцедуры\n";

    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) =
                serde_json::from_value::<tower_lsp::lsp_types::PublishDiagnosticsParams>(params)
            else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///semantic_breakdown_fixture.bsl").expect("fixture uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: SEMANTIC_FIXTURE.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");
    let diagnostics = wait_lsp_publish_diagnostics(&mut published_rx, &uri).await;
    assert!(
        !diagnostics.is_empty(),
        "semantic fixture must publish diagnostics before metrics snapshot"
    );

    let execute = Request::build("workspace/executeCommand")
        .id(2204)
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
    let result = value.get("result").cloned().expect("result field");
    let metrics = result.get("metrics").expect("metrics field");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    for key in [
        "intellisense_v2_semantic_diagnostics_query_inputs_ms",
        "intellisense_v2_semantic_diagnostics_query_parse_result_ms",
        "intellisense_v2_semantic_diagnostics_query_ir_ms",
        "intellisense_v2_semantic_diagnostics_query_collect_ms",
    ] {
        let count = histograms
            .get(key)
            .and_then(|value| value.get("count"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert!(
            count > 0,
            "metrics endpoint must expose semantic diagnostics breakdown histogram {key}"
        );
    }

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_type_index_precompute_breakdown() {
    let coordinator = Arc::new(SystemCoordinator::new());
    for (stage, millis) in [
        ("type_index_precompute", 4800_u64),
        ("type_index_precompute_build", 14_u64),
        ("type_index_precompute_ir", 4700_u64),
        ("type_index_precompute_ast_to_ir", 1900_u64),
        ("type_index_precompute_semantic_facts", 2600_u64),
        (
            "type_index_precompute_semantic_facts_seed_module_context",
            120_u64,
        ),
        (
            "type_index_precompute_semantic_facts_local_function_summaries",
            2300_u64,
        ),
        (
            "type_index_precompute_semantic_facts_visit_statements",
            180_u64,
        ),
    ] {
        coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
            stage,
            Duration::from_millis(millis),
        );
    }

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let execute = Request::build("workspace/executeCommand")
        .id(2205)
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
    let result = value.get("result").cloned().expect("result field");
    let metrics = result.get("metrics").expect("metrics field");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    for key in [
        "intellisense_v2_runtime_type_index_precompute_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_build_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms",
    ] {
        let count = histograms
            .get(key)
            .and_then(|value| value.get("count"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert!(
            count > 0,
            "metrics endpoint must expose type-index precompute breakdown histogram {key}"
        );
    }

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_completion_timeline_exposes_versioned_contract() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let execute = Request::build("workspace/executeCommand")
        .id(2)
        .params(serde_json::json!({
            "command": "bsl.getCompletionTimeline",
            "arguments": [{}],
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
    let result = value.get("result").cloned().expect("result field");
    assert_eq!(
        result
            .get("version")
            .and_then(|value| value.as_u64())
            .expect("version"),
        11
    );
    assert!(
        result
            .get("traces")
            .and_then(|value| value.as_array())
            .is_some(),
        "result must contain traces array, got {result}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_completion_timeline_contains_completion_trace() {
    const FIXTURE: &str =
        "Процедура Тест()\n    ДляCompletion = (Новый Массив()).\nКонецПроцедуры\n";

    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///completion_timeline_fixture.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let completion = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(2, 13),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    };
    let completion_req = Request::build("textDocument/completion")
        .id(2)
        .params(serde_json::to_value(completion).expect("CompletionParams"))
        .finish();
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(completion_req)
        .await
        .expect("completion request");
    assert!(
        completion_response.is_some(),
        "completion should return a response"
    );

    let execute = Request::build("workspace/executeCommand")
        .id(3)
        .params(serde_json::json!({
            "command": "bsl.getCompletionTimeline",
            "arguments": [{ "limit": 10 }],
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
    let result = value.get("result").cloned().expect("result field");
    let traces = result
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("traces array");
    assert!(
        !traces.is_empty(),
        "expected non-empty completion timeline traces after completion request"
    );
    let trace = traces
        .last()
        .and_then(|value| value.as_object())
        .expect("trace");
    for field in [
        "trace_id",
        "request_id",
        "uri",
        "trigger_mode",
        "outcome",
        "started_at_ms",
        "total_duration_ms",
        "dominant_stage",
        "prepare_details",
        "server_edge_details",
        "turn_attribution",
        "stages",
    ] {
        assert!(
            trace.contains_key(field),
            "missing field `{field}` in trace"
        );
    }
    let prepare_details = trace
        .get("prepare_details")
        .and_then(|value| value.as_object())
        .expect("prepare_details object");
    let server_edge_details = trace
        .get("server_edge_details")
        .and_then(|value| value.as_object())
        .expect("server_edge_details object");
    assert!(
        prepare_details.contains_key("wait_budget_ms"),
        "missing field `wait_budget_ms` in prepare_details"
    );
    assert!(
        prepare_details.contains_key("route"),
        "prepare_details must expose bounded route field even when absent"
    );
    assert!(
        prepare_details.contains_key("fail_closed_cause"),
        "prepare_details must expose split fail-closed cause field even when absent"
    );
    assert!(
        prepare_details.contains_key("guard_outcome"),
        "missing field `guard_outcome` in prepare_details"
    );
    assert!(
        prepare_details.contains_key("min_file_version"),
        "missing field `min_file_version` in prepare_details"
    );
    assert!(
        prepare_details.contains_key("progress"),
        "prepare_details must expose prepare progress details in v5 contract"
    );
    assert!(
        prepare_details.contains_key("wait_for_file_version_runtime"),
        "prepare_details must expose wait_for_file_version runtime drilldown in v5 contract"
    );
    assert!(
        prepare_details.contains_key("snapshot_with_deps_runtime"),
        "prepare_details must expose snapshot_with_deps runtime drilldown in v5 contract"
    );
    assert!(
        prepare_details.contains_key("exact_wait"),
        "prepare_details must expose exact wait details in v5 contract"
    );
    assert!(
        prepare_details.contains_key("timeout_attribution"),
        "prepare_details must expose timeout attribution details in v6 contract"
    );
    for field in [
        "transport_received_at_ms",
        "transport_received_at_ms_provenance",
        "pre_method_attribution_provenance",
        "handler_entered_at_ms",
        "response_sent_at_ms",
        "transport_to_method_wait_ms",
        "method_prelude_exec_ms",
        "transport_to_handler_wait_ms",
        "server_handler_exec_ms",
    ] {
        assert!(
            server_edge_details.contains_key(field),
            "missing field `{field}` in server_edge_details"
        );
    }
    let transport_received_at_ms = server_edge_details
        .get("transport_received_at_ms")
        .and_then(|value| value.as_u64())
        .expect("transport_received_at_ms");
    let transport_received_at_ms_provenance = server_edge_details
        .get("transport_received_at_ms_provenance")
        .and_then(|value| value.as_str())
        .expect("transport_received_at_ms_provenance");
    let pre_method_attribution_provenance = server_edge_details
        .get("pre_method_attribution_provenance")
        .and_then(|value| value.as_str())
        .expect("pre_method_attribution_provenance");
    let handler_entered_at_ms = server_edge_details
        .get("handler_entered_at_ms")
        .and_then(|value| value.as_u64())
        .expect("handler_entered_at_ms");
    let method_entered_at_ms = server_edge_details
        .get("method_entered_at_ms")
        .and_then(|value| value.as_u64());
    let response_sent_at_ms = server_edge_details
        .get("response_sent_at_ms")
        .and_then(|value| value.as_u64())
        .expect("response_sent_at_ms");
    let transport_to_method_wait_ms = server_edge_details
        .get("transport_to_method_wait_ms")
        .and_then(|value| value.as_u64())
        .expect("transport_to_method_wait_ms");
    let method_prelude_exec_ms = server_edge_details
        .get("method_prelude_exec_ms")
        .and_then(|value| value.as_u64())
        .expect("method_prelude_exec_ms");
    let transport_to_handler_wait_ms = server_edge_details
        .get("transport_to_handler_wait_ms")
        .and_then(|value| value.as_u64())
        .expect("transport_to_handler_wait_ms");
    let server_handler_exec_ms = server_edge_details
        .get("server_handler_exec_ms")
        .and_then(|value| value.as_u64())
        .expect("server_handler_exec_ms");
    assert!(
        matches!(
            transport_received_at_ms_provenance,
            "request_context_call_entry" | "jsonrpc_dispatch_received"
        ),
        "unexpected transport_received_at_ms_provenance={transport_received_at_ms_provenance}"
    );
    assert!(
        matches!(
            pre_method_attribution_provenance,
            "same_request_authoritative" | "best_effort_fallback" | "unavailable"
        ),
        "unexpected pre_method_attribution_provenance={pre_method_attribution_provenance}"
    );
    assert!(
        transport_received_at_ms <= handler_entered_at_ms,
        "transport_received_at_ms must not exceed handler_entered_at_ms"
    );
    if let Some(method_entered_at_ms) = method_entered_at_ms {
        assert!(
            transport_received_at_ms <= method_entered_at_ms,
            "transport_received_at_ms must not exceed method_entered_at_ms"
        );
        assert!(
            method_entered_at_ms <= handler_entered_at_ms,
            "method_entered_at_ms must not exceed handler_entered_at_ms"
        );
        assert_eq!(
            transport_to_method_wait_ms,
            method_entered_at_ms.saturating_sub(transport_received_at_ms),
            "transport_to_method_wait_ms must match timestamp delta"
        );
        if let Some(jsonrpc_dispatch_received_at_ms) = server_edge_details
            .get("jsonrpc_dispatch_received_at_ms")
            .and_then(|value| value.as_u64())
        {
            let dispatch_to_request_context_wait_ms = server_edge_details
                .get("dispatch_to_request_context_wait_ms")
                .and_then(|value| value.as_u64())
                .expect("dispatch_to_request_context_wait_ms");
            assert_eq!(
                transport_received_at_ms_provenance, "jsonrpc_dispatch_received",
                "jsonrpc dispatch timestamp must align with provenance"
            );
            assert_eq!(
                transport_received_at_ms,
                jsonrpc_dispatch_received_at_ms,
                "transport_received_at_ms must equal jsonrpc dispatch timestamp when provenance is jsonrpc_dispatch_received"
            );
            assert!(
                transport_received_at_ms <= method_entered_at_ms,
                "jsonrpc dispatch timestamp must not exceed method_entered_at_ms"
            );
            assert_eq!(
                dispatch_to_request_context_wait_ms <= transport_to_method_wait_ms,
                true,
                "dispatch_to_request_context_wait_ms must not exceed transport_to_method_wait_ms"
            );
        } else {
            assert_eq!(
                transport_received_at_ms_provenance,
                "request_context_call_entry",
                "missing jsonrpc dispatch timestamp must fall back to request_context_call_entry provenance"
            );
            assert!(
                !server_edge_details.contains_key("dispatch_to_request_context_wait_ms"),
                "dispatch_to_request_context_wait_ms must not be fabricated when jsonrpc_dispatch_received_at_ms is absent"
            );
        }
        assert_eq!(
            method_prelude_exec_ms,
            handler_entered_at_ms.saturating_sub(method_entered_at_ms),
            "method_prelude_exec_ms must match timestamp delta"
        );
        if let Some(service_scope_entered_at_ms) = server_edge_details
            .get("service_scope_entered_at_ms")
            .and_then(|value| value.as_u64())
        {
            let service_future_created_at_ms = server_edge_details
                .get("service_future_created_at_ms")
                .and_then(|value| value.as_u64())
                .expect("service_future_created_at_ms");
            let transport_to_service_future_wait_ms = server_edge_details
                .get("transport_to_service_future_wait_ms")
                .and_then(|value| value.as_u64())
                .expect("transport_to_service_future_wait_ms");
            let service_future_to_scope_wait_ms = server_edge_details
                .get("service_future_to_scope_wait_ms")
                .and_then(|value| value.as_u64())
                .expect("service_future_to_scope_wait_ms");
            let service_future_first_poll_entered_at_ms = server_edge_details
                .get("service_future_first_poll_entered_at_ms")
                .and_then(|value| value.as_u64());
            let transport_to_service_scope_wait_ms = server_edge_details
                .get("transport_to_service_scope_wait_ms")
                .and_then(|value| value.as_u64())
                .expect("transport_to_service_scope_wait_ms");
            let service_scope_to_method_wait_ms = server_edge_details
                .get("service_scope_to_method_wait_ms")
                .and_then(|value| value.as_u64())
                .expect("service_scope_to_method_wait_ms");
            assert!(
                transport_received_at_ms <= service_scope_entered_at_ms,
                "transport_received_at_ms must not exceed service_scope_entered_at_ms"
            );
            assert!(
                transport_received_at_ms <= service_future_created_at_ms,
                "transport_received_at_ms must not exceed service_future_created_at_ms"
            );
            assert!(
                service_future_created_at_ms <= service_scope_entered_at_ms,
                "service_future_created_at_ms must not exceed service_scope_entered_at_ms"
            );
            assert!(
                service_scope_entered_at_ms <= method_entered_at_ms,
                "service_scope_entered_at_ms must not exceed method_entered_at_ms"
            );
            assert_eq!(
                transport_to_service_future_wait_ms,
                service_future_created_at_ms.saturating_sub(transport_received_at_ms),
                "transport_to_service_future_wait_ms must match timestamp delta"
            );
            assert_eq!(
                service_future_to_scope_wait_ms,
                service_scope_entered_at_ms.saturating_sub(service_future_created_at_ms),
                "service_future_to_scope_wait_ms must match timestamp delta"
            );
            if let Some(service_future_first_poll_entered_at_ms) =
                service_future_first_poll_entered_at_ms
            {
                let service_future_to_first_poll_wait_ms = server_edge_details
                    .get("service_future_to_first_poll_wait_ms")
                    .and_then(|value| value.as_u64())
                    .expect("service_future_to_first_poll_wait_ms");
                let service_future_first_poll_outcome = server_edge_details
                    .get("service_future_first_poll_outcome")
                    .and_then(|value| value.as_str())
                    .expect("service_future_first_poll_outcome");
                assert!(
                    service_scope_entered_at_ms <= service_future_first_poll_entered_at_ms,
                    "service_scope_entered_at_ms must not exceed service_future_first_poll_entered_at_ms"
                );
                assert!(
                    service_future_created_at_ms <= service_future_first_poll_entered_at_ms,
                    "service_future_created_at_ms must not exceed service_future_first_poll_entered_at_ms"
                );
                assert!(
                    matches!(service_future_first_poll_outcome, "ready" | "pending"),
                    "unexpected service_future_first_poll_outcome={service_future_first_poll_outcome}"
                );
                assert_eq!(
                    service_future_to_first_poll_wait_ms,
                    service_future_first_poll_entered_at_ms
                        .saturating_sub(service_future_created_at_ms),
                    "service_future_to_first_poll_wait_ms must match timestamp delta"
                );
                if let Some(service_future_first_wake_scheduled_at_ms) = server_edge_details
                    .get("service_future_first_wake_scheduled_at_ms")
                    .and_then(|value| value.as_u64())
                {
                    let first_poll_to_first_wake_wait_ms = server_edge_details
                        .get("first_poll_to_first_wake_wait_ms")
                        .and_then(|value| value.as_u64())
                        .expect("first_poll_to_first_wake_wait_ms");
                    assert_eq!(
                        service_future_first_poll_outcome, "pending",
                        "first wake split must only exist for pending-first-poll traces"
                    );
                    assert!(
                        service_future_first_poll_entered_at_ms
                            <= service_future_first_wake_scheduled_at_ms,
                        "service_future_first_poll_entered_at_ms must not exceed service_future_first_wake_scheduled_at_ms"
                    );
                    assert_eq!(
                        first_poll_to_first_wake_wait_ms,
                        service_future_first_wake_scheduled_at_ms
                            .saturating_sub(service_future_first_poll_entered_at_ms),
                        "first_poll_to_first_wake_wait_ms must match timestamp delta"
                    );
                } else {
                    assert!(
                        !server_edge_details.contains_key("first_poll_to_first_wake_wait_ms"),
                        "first_poll_to_first_wake_wait_ms must not be fabricated when service_future_first_wake_scheduled_at_ms is absent"
                    );
                }
            } else {
                assert!(
                    !server_edge_details.contains_key("service_future_to_first_poll_wait_ms"),
                    "service_future_to_first_poll_wait_ms must not be fabricated when service_future_first_poll_entered_at_ms is absent"
                );
                assert!(
                    !server_edge_details.contains_key("service_future_first_poll_outcome"),
                    "service_future_first_poll_outcome must not be fabricated when service_future_first_poll_entered_at_ms is absent"
                );
                assert!(
                    !server_edge_details.contains_key("service_future_first_wake_scheduled_at_ms"),
                    "service_future_first_wake_scheduled_at_ms must not be fabricated when service_future_first_poll_entered_at_ms is absent"
                );
                assert!(
                    !server_edge_details.contains_key("first_poll_to_first_wake_wait_ms"),
                    "first_poll_to_first_wake_wait_ms must not be fabricated when service_future_first_poll_entered_at_ms is absent"
                );
            }
            assert_eq!(
                transport_to_service_scope_wait_ms,
                service_scope_entered_at_ms.saturating_sub(transport_received_at_ms),
                "transport_to_service_scope_wait_ms must match timestamp delta"
            );
            assert_eq!(
                service_scope_to_method_wait_ms,
                method_entered_at_ms.saturating_sub(service_scope_entered_at_ms),
                "service_scope_to_method_wait_ms must match timestamp delta"
            );
        } else {
            assert!(
                !server_edge_details.contains_key("service_future_created_at_ms"),
                "service_future_created_at_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("transport_to_service_future_wait_ms"),
                "transport_to_service_future_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_future_to_scope_wait_ms"),
                "service_future_to_scope_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("transport_to_service_scope_wait_ms"),
                "transport_to_service_scope_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_scope_to_method_wait_ms"),
                "service_scope_to_method_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_future_first_poll_entered_at_ms"),
                "service_future_first_poll_entered_at_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_future_to_first_poll_wait_ms"),
                "service_future_to_first_poll_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_future_first_poll_outcome"),
                "service_future_first_poll_outcome must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("service_future_first_wake_scheduled_at_ms"),
                "service_future_first_wake_scheduled_at_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
            assert!(
                !server_edge_details.contains_key("first_poll_to_first_wake_wait_ms"),
                "first_poll_to_first_wake_wait_ms must not be fabricated when service_scope_entered_at_ms is absent"
            );
        }
    } else {
        assert!(
            !server_edge_details.contains_key("service_future_created_at_ms"),
            "service_future_created_at_ms must not be present when method_entered_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("transport_to_service_future_wait_ms"),
            "transport_to_service_future_wait_ms must not be present when method_entered_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("service_future_to_scope_wait_ms"),
            "service_future_to_scope_wait_ms must not be present when method_entered_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("service_scope_entered_at_ms"),
            "service_scope_entered_at_ms must not be present when method_entered_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("transport_to_service_scope_wait_ms"),
            "transport_to_service_scope_wait_ms must not be present when method_entered_at_ms is absent"
        );
        assert!(
            !server_edge_details.contains_key("service_scope_to_method_wait_ms"),
            "service_scope_to_method_wait_ms must not be present when method_entered_at_ms is absent"
        );
    }
    assert!(
        handler_entered_at_ms <= response_sent_at_ms,
        "handler_entered_at_ms must not exceed response_sent_at_ms"
    );
    assert_eq!(
        transport_to_handler_wait_ms,
        handler_entered_at_ms.saturating_sub(transport_received_at_ms),
        "transport_to_handler_wait_ms must match timestamp delta"
    );
    assert_eq!(
        server_handler_exec_ms,
        response_sent_at_ms.saturating_sub(handler_entered_at_ms),
        "server_handler_exec_ms must match timestamp delta"
    );
    let stages = trace
        .get("stages")
        .and_then(|value| value.as_array())
        .expect("trace stages array");
    assert!(!stages.is_empty(), "trace stages must not be empty");
    for stage in stages {
        let stage = stage.as_object().expect("stage object");
        for field in ["name", "status", "started_offset_ms", "duration_ms"] {
            assert!(
                stage.contains_key(field),
                "missing field `{field}` in stage"
            );
        }
    }
    let turn_attribution = trace
        .get("turn_attribution")
        .and_then(|value| value.as_object())
        .expect("turn_attribution object");
    for field in [
        "request_file_seq",
        "request_epoch",
        "queue_outcome",
        "queue_capacity",
        "queue_depth_before_enqueue",
        "queue_depth_after_enqueue",
        "queued_completion_ahead_count",
        "did_change_ahead_count",
        "active_completion_count",
        "dropped_completion_file_seq",
    ] {
        assert!(
            turn_attribution.contains_key(field),
            "missing field `{field}` in turn_attribution"
        );
    }
    assert!(
        turn_attribution.contains_key("dispatcher_resolution_latency_ms"),
        "missing field `dispatcher_resolution_latency_ms` in turn_attribution"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p22_get_observability_metrics_exposes_runtime_knobs_config() {
    let coordinator = Arc::new(SystemCoordinator::new());

    let (mut service, mut socket) = LspService::build({
        let coordinator = coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();

    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let execute = Request::build("workspace/executeCommand")
        .id(2)
        .params(serde_json::json!({
            "command": "bsl.getObservabilityMetrics",
            "arguments": [{}],
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
    let result = value.get("result").cloned().expect("result field");
    let metrics = result
        .get("metrics")
        .and_then(|value| value.as_object())
        .expect("metrics object");
    let config = metrics
        .get("config")
        .and_then(|value| value.as_object())
        .expect("config object");
    let interactive_wait_budget = config
        .get("BSL_INTELLISENSE_V2_INTERACTIVE_WAIT_BUDGET_MS")
        .and_then(|value| value.as_object())
        .expect("interactive wait budget config entry");
    assert_eq!(
        interactive_wait_budget
            .get("effective")
            .and_then(|value| value.as_u64()),
        Some(120),
        "metrics config must expose effective interactive wait budget"
    );
    assert!(
        interactive_wait_budget
            .get("source")
            .and_then(|value| value.as_str())
            .is_some(),
        "metrics config must expose value source"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p23_cross_interface_semantic_parity_lsp_web_mcp_diagnostics() {
    const PARITY_FIXTURE: &str = "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.НесуществующийМетод();\nКонецПроцедуры\n";

    let lsp_coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = lsp_coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) =
                serde_json::from_value::<tower_lsp::lsp_types::PublishDiagnosticsParams>(params)
            else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;
    let lsp_uri = Url::parse("file:///parity_fixture.bsl").expect("lsp uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: lsp_uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: PARITY_FIXTURE.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");
    let lsp_diagnostics = wait_lsp_publish_diagnostics(&mut published_rx, &lsp_uri).await;
    let lsp_normalized = normalize_lsp_semantic_diagnostics(&lsp_diagnostics);
    assert!(
        !lsp_normalized.is_empty(),
        "expected non-empty LSP diagnostics"
    );
    drain_task.abort();

    let app = create_router(build_web_test_state(), "backend/static", true);
    let web_response = app
        .oneshot(
            AxumRequest::post("/api/diagnostics")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::json!({ "code": PARITY_FIXTURE }).to_string(),
                ))
                .expect("web diagnostics request"),
        )
        .await
        .expect("web diagnostics response");
    assert!(
        web_response.status().is_success(),
        "unexpected web status: {}",
        web_response.status()
    );
    let web_body = axum::body::to_bytes(web_response.into_body(), usize::MAX)
        .await
        .expect("web body");
    let web_payload: serde_json::Value =
        serde_json::from_slice(&web_body).expect("web diagnostics payload");
    let web_normalized = normalize_web_semantic_diagnostics(&web_payload);
    assert!(
        !web_normalized.is_empty(),
        "expected non-empty Web diagnostics, payload={web_payload}"
    );

    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(&module_path, PARITY_FIXTURE).expect("write module");
    let mcp_manager = Arc::new(SessionManager::new());
    let mcp_job_manager = Arc::new(JobManager::new());
    let open = mcp_manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            mcp_job_manager.clone(),
        )
        .await
        .expect("mcp workspace open");
    wait_mcp_startup(mcp_job_manager.as_ref(), open.startup_job_id.as_deref()).await;
    let mcp_diagnostics = mcp_manager
        .bsl_diagnostics(BslDiagnosticsParams {
            session_id: open.session_id,
            scope: WorkspaceScope::Tagged(WorkspaceScopeTagged::Project),
            limit: 200,
            include_impact: false,
            include_coverage: false,
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp diagnostics");
    let mcp_normalized = normalize_mcp_semantic_diagnostics(&mcp_diagnostics.diagnostics);
    assert!(
        !mcp_normalized.is_empty(),
        "expected non-empty MCP diagnostics"
    );

    assert_eq!(
        lsp_normalized, web_normalized,
        "LSP/Web semantic diagnostics drift detected"
    );
    assert_eq!(
        lsp_normalized, mcp_normalized,
        "LSP/MCP semantic diagnostics drift detected"
    );
}

#[tokio::test]
async fn p24_real_scenario_observability_stage_parity_lsp_vs_mcp() {
    const OBSERVABILITY_FIXTURE: &str =
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n";

    let lsp_coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = lsp_coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) =
                serde_json::from_value::<tower_lsp::lsp_types::PublishDiagnosticsParams>(params)
            else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;

    let uri = Url::parse("file:///observability_fixture.bsl").expect("lsp uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: OBSERVABILITY_FIXTURE.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");
    let _ = wait_lsp_publish_diagnostics(&mut published_rx, &uri).await;

    let completion = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(2, 13),
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };
    let completion_req = Request::build("textDocument/completion")
        .id(2)
        .params(serde_json::to_value(completion).expect("CompletionParams"))
        .finish();
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(completion_req)
        .await
        .expect("completion request");
    assert!(
        completion_response.is_some(),
        "completion should return a response"
    );

    let execute = Request::build("workspace/executeCommand")
        .id(3)
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
    let lsp_metrics_payload =
        serde_json::to_value(&execute_response).expect("serialize execute response");
    let lsp_metrics_payload = lsp_metrics_payload
        .get("result")
        .cloned()
        .expect("execute result field");
    drain_task.abort();

    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(&module_path, OBSERVABILITY_FIXTURE).expect("write module");
    let mcp_manager = Arc::new(SessionManager::new());
    let mcp_job_manager = Arc::new(JobManager::new());
    let open = mcp_manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            mcp_job_manager.clone(),
        )
        .await
        .expect("mcp workspace open");
    wait_mcp_startup(mcp_job_manager.as_ref(), open.startup_job_id.as_deref()).await;

    let _diagnostics = mcp_manager
        .bsl_diagnostics(BslDiagnosticsParams {
            session_id: open.session_id.clone(),
            scope: WorkspaceScope::Tagged(WorkspaceScopeTagged::Project),
            limit: 200,
            include_impact: false,
            include_coverage: false,
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp diagnostics");
    let _members = mcp_manager
        .bsl_members(BslMembersParams {
            session_id: open.session_id.clone(),
            file: McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            },
            position: McpPosition {
                line: 2,
                character: 13,
            },
            limit: 50,
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp members");
    let mcp_metrics_payload = mcp_manager
        .observability_metrics_get(&open.session_id)
        .await
        .expect("mcp observability")
        .metrics;

    let lsp_stages = collect_observed_stages(&lsp_metrics_payload);
    let mcp_stages = collect_observed_stages(&mcp_metrics_payload);
    let required_shared_stages = [
        "runtime_snapshot_with_deps",
        "semantic_diagnostics_query",
        "ir_query",
    ];

    for stage in required_shared_stages {
        assert!(
            lsp_stages.contains(stage),
            "LSP metrics missing required stage {stage}, stages={lsp_stages:?}"
        );
        assert!(
            mcp_stages.contains(stage),
            "MCP metrics missing required stage {stage}, stages={mcp_stages:?}"
        );
        assert!(
            has_positive_counter_for_stage(&lsp_metrics_payload, stage),
            "LSP stage {stage} has no positive counters"
        );
        assert!(
            has_positive_counter_for_stage(&mcp_metrics_payload, stage),
            "MCP stage {stage} has no positive counters"
        );
    }
    assert!(
        mcp_stages.contains("parse_result_query"),
        "MCP metrics missing parse_result_query stage, stages={mcp_stages:?}"
    );
    assert!(
        !has_positive_counter_for_stage(&mcp_metrics_payload, "parse_result_query"),
        "MCP semantic scenario must not execute parse_result_query stage"
    );

    assert_drilldown_stage_metrics_for_origin(&lsp_metrics_payload, "lsp");
    assert_drilldown_stage_metrics_for_origin(&mcp_metrics_payload, "agent");
}

#[tokio::test]
async fn p25_web_hover_endpoints_use_shared_ephemeral_exact_artifact() {
    let code = "Процедура Тест()\n\
    Arr = Новый Массив;\n\
    Arr.Добавить(1);\n\
    ДляHover = Arr;\n\
КонецПроцедуры\n";
    let position = find_utf16_position_at_marker_tail(code, "ДляHover = Arr");

    let hover_text = web_hover_text_for_code(code, position).await;
    assert!(
        hover_text.contains("Массив"),
        "web hover must expose warmed exact type for ephemeral semantic snapshot, hover={hover_text}"
    );

    let enhanced_hover_text = web_enhanced_hover_text_for_code(code, position).await;
    assert!(
        enhanced_hover_text.contains("Массив"),
        "web enhanced hover must expose warmed exact type for ephemeral semantic snapshot, hover={enhanced_hover_text}"
    );
}

#[tokio::test]
async fn p25_cross_interface_semantic_parity_lsp_web_mcp_core_tools() {
    const TOOLS_FIXTURE: &str = "Процедура Foo() Экспорт\nКонецПроцедуры\n\nПроцедура Bar()\n    Arr = Новый Массив;\n    Arr.Добавить(1);\n    Foo();\nКонецПроцедуры\n";
    const TARGET_SYMBOL: &str = "Foo";
    const TYPE_LINE: u32 = 5;
    const TYPE_CHARACTER: u32 = 5;
    const MEMBERS_LINE: u32 = 5;
    const MEMBERS_CHARACTER: u32 = 7;
    const SYMBOL_CALL_LINE: u32 = 6;
    const SYMBOL_CALL_CHARACTER: u32 = 5;

    let lsp_coordinator = Arc::new(SystemCoordinator::new());
    let (mut service, mut socket) = LspService::build({
        let coordinator = lsp_coordinator.clone();
        move |client| BslLanguageServer::new(client, coordinator.clone())
    })
    .finish();
    let drain_task = tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

    initialize_lsp_service(&mut service).await;

    let lsp_uri = Url::parse("file:///Module.bsl").expect("lsp uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: lsp_uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: TOOLS_FIXTURE.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: lsp_uri.clone(),
            },
            position: Position {
                line: MEMBERS_LINE,
                character: MEMBERS_CHARACTER,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };
    let completion_req = Request::build("textDocument/completion")
        .id(2)
        .params(serde_json::to_value(completion_params).expect("CompletionParams"))
        .finish();
    let completion_response = service
        .ready()
        .await
        .unwrap()
        .call(completion_req)
        .await
        .expect("completion request")
        .expect("completion response");
    let completion_value = serde_json::to_value(&completion_response).expect("serialize response");
    let completion_result = completion_value
        .get("result")
        .cloned()
        .expect("result field");
    let lsp_completion: Option<CompletionResponse> =
        serde_json::from_value(completion_result).expect("parse completion result");
    let lsp_completion = lsp_completion.expect("completion result present");
    let lsp_members = normalize_lsp_member_labels(&lsp_completion);

    let symbol_req = Request::build("workspace/symbol")
        .id(3)
        .params(
            serde_json::to_value(WorkspaceSymbolParams {
                query: TARGET_SYMBOL.to_string(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .expect("WorkspaceSymbolParams"),
        )
        .finish();
    let symbol_response = service
        .ready()
        .await
        .unwrap()
        .call(symbol_req)
        .await
        .expect("workspace/symbol request")
        .expect("workspace/symbol response");
    let symbol_value = serde_json::to_value(&symbol_response).expect("serialize response");
    let symbol_result = symbol_value.get("result").cloned().expect("result field");
    let lsp_symbols: Option<Vec<SymbolInformation>> =
        serde_json::from_value(symbol_result).expect("parse symbol result");
    let lsp_symbols = lsp_symbols.expect("symbol result present");
    let lsp_symbols = normalize_lsp_workspace_symbols(&lsp_symbols);
    assert!(
        !lsp_symbols.is_empty(),
        "expected non-empty LSP symbol_search result"
    );

    let definition_req = Request::build("textDocument/definition")
        .id(4)
        .params(
            serde_json::to_value(GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: lsp_uri.clone(),
                    },
                    position: Position {
                        line: SYMBOL_CALL_LINE,
                        character: SYMBOL_CALL_CHARACTER,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .expect("GotoDefinitionParams"),
        )
        .finish();
    let definition_response = service
        .ready()
        .await
        .unwrap()
        .call(definition_req)
        .await
        .expect("textDocument/definition request")
        .expect("textDocument/definition response");
    let definition_value = serde_json::to_value(&definition_response).expect("serialize response");
    let definition_result = definition_value
        .get("result")
        .cloned()
        .expect("result field");
    let lsp_definition: Option<GotoDefinitionResponse> =
        serde_json::from_value(definition_result).expect("parse definition result");
    let lsp_definition = normalize_lsp_definition(lsp_definition);
    assert!(
        !lsp_definition.is_empty(),
        "expected non-empty LSP definition result"
    );

    let references_req = Request::build("textDocument/references")
        .id(5)
        .params(
            serde_json::to_value(ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: lsp_uri.clone(),
                    },
                    position: Position {
                        line: SYMBOL_CALL_LINE,
                        character: SYMBOL_CALL_CHARACTER,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: ReferenceContext {
                    include_declaration: false,
                },
            })
            .expect("ReferenceParams"),
        )
        .finish();
    let references_response = service
        .ready()
        .await
        .unwrap()
        .call(references_req)
        .await
        .expect("textDocument/references request")
        .expect("textDocument/references response");
    let references_value = serde_json::to_value(&references_response).expect("serialize response");
    let references_result = references_value
        .get("result")
        .cloned()
        .expect("result field");
    let lsp_references: Option<Vec<Location>> =
        serde_json::from_value(references_result).expect("parse references result");
    let lsp_references = normalize_lsp_locations(&lsp_references.unwrap_or_default());
    assert!(
        !lsp_references.is_empty(),
        "expected non-empty LSP references result"
    );

    let hover_req = Request::build("textDocument/hover")
        .id(6)
        .params(
            serde_json::to_value(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: lsp_uri.clone(),
                    },
                    position: Position {
                        line: TYPE_LINE,
                        character: TYPE_CHARACTER,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .expect("HoverParams"),
        )
        .finish();
    let hover_response = service
        .ready()
        .await
        .unwrap()
        .call(hover_req)
        .await
        .expect("textDocument/hover request")
        .expect("textDocument/hover response");
    let hover_value = serde_json::to_value(&hover_response).expect("serialize response");
    let hover_result = hover_value.get("result").cloned().expect("result field");
    let lsp_hover: Option<Hover> = serde_json::from_value(hover_result).expect("parse hover");
    let lsp_hover_text = lsp_hover
        .and_then(extract_hover_text)
        .unwrap_or_else(|| String::from(""));
    assert!(
        !lsp_hover_text.is_empty(),
        "expected non-empty LSP hover response at type position"
    );
    drain_task.abort();

    // Web currently exposes hover/diagnostics for semantic parity, while MCP-only tools below
    // are validated via LSP/MCP pairs.
    let app = create_router(build_web_test_state(), "backend/static", true);
    let web_hover_response = app
        .clone()
        .oneshot(
            AxumRequest::post("/api/hover")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::json!({
                        "code": TOOLS_FIXTURE,
                        "line": TYPE_LINE,
                        "column": TYPE_CHARACTER
                    })
                    .to_string(),
                ))
                .expect("web hover request"),
        )
        .await
        .expect("web hover response");
    assert!(
        web_hover_response.status().is_success(),
        "unexpected web hover status: {}",
        web_hover_response.status()
    );
    let web_hover_body = axum::body::to_bytes(web_hover_response.into_body(), usize::MAX)
        .await
        .expect("web hover body");
    let web_hover_payload: serde_json::Value =
        serde_json::from_slice(&web_hover_body).expect("web hover payload");
    let web_hover_text = web_hover_payload
        .get("hover")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    assert!(
        !web_hover_text.is_empty(),
        "expected non-empty Web hover text, payload={web_hover_payload}"
    );
    let web_enhanced_hover_response = app
        .oneshot(
            AxumRequest::post("/api/hover/enhanced")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::json!({
                        "code": TOOLS_FIXTURE,
                        "line": TYPE_LINE,
                        "column": TYPE_CHARACTER
                    })
                    .to_string(),
                ))
                .expect("web enhanced hover request"),
        )
        .await
        .expect("web enhanced hover response");
    assert!(
        web_enhanced_hover_response.status().is_success(),
        "unexpected web enhanced hover status: {}",
        web_enhanced_hover_response.status()
    );
    let web_enhanced_hover_body =
        axum::body::to_bytes(web_enhanced_hover_response.into_body(), usize::MAX)
            .await
            .expect("web enhanced hover body");
    let web_enhanced_hover_payload: serde_json::Value =
        serde_json::from_slice(&web_enhanced_hover_body).expect("web enhanced hover payload");
    let web_enhanced_hover_text = web_enhanced_hover_payload
        .get("hoverText")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    assert!(
        !web_enhanced_hover_text.is_empty()
            && web_enhanced_hover_text != "No information available",
        "expected non-empty Web enhanced hover text, payload={web_enhanced_hover_payload}"
    );

    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(&module_path, TOOLS_FIXTURE).expect("write module");
    let mcp_manager = Arc::new(SessionManager::new());
    let mcp_job_manager = Arc::new(JobManager::new());
    let open = mcp_manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            mcp_job_manager.clone(),
        )
        .await
        .expect("mcp workspace open");
    wait_mcp_startup(mcp_job_manager.as_ref(), open.startup_job_id.as_deref()).await;

    let mcp_type = mcp_manager
        .bsl_type_at_position(BslTypeAtPositionParams {
            session_id: open.session_id.clone(),
            file: McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            },
            position: McpPosition {
                line: TYPE_LINE,
                character: TYPE_CHARACTER,
            },
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp type_at_position");
    assert!(
        mcp_type.warnings.is_empty(),
        "mcp type_at_position returned warnings: {:?}",
        mcp_type.warnings
    );
    let mcp_type_name = mcp_type
        .type_info
        .as_ref()
        .map(|type_info| type_info.name.clone())
        .expect("mcp type_at_position type_info");

    let mcp_members = mcp_manager
        .bsl_members(BslMembersParams {
            session_id: open.session_id.clone(),
            file: McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            },
            position: McpPosition {
                line: MEMBERS_LINE,
                character: MEMBERS_CHARACTER,
            },
            limit: 100,
            include_flow_sensitive: false,
        })
        .await
        .expect("mcp members");
    let mcp_members = normalize_mcp_member_labels(&mcp_members.members);

    let mcp_symbol_search = mcp_manager
        .bsl_symbol_search(BslSymbolSearchParams {
            session_id: open.session_id.clone(),
            query: TARGET_SYMBOL.to_string(),
            limit: 20,
        })
        .await
        .expect("mcp symbol_search");
    let mcp_symbols = normalize_mcp_workspace_symbols(&mcp_symbol_search.symbols);
    assert!(
        !mcp_symbols.is_empty(),
        "expected non-empty MCP symbol_search result"
    );
    let mcp_target_symbol_id = mcp_symbol_search
        .symbols
        .iter()
        .find(|symbol| symbol.name == TARGET_SYMBOL)
        .map(|symbol| symbol.symbol_id.clone())
        .expect("mcp target symbol id");

    let mcp_references = mcp_manager
        .bsl_references(BslReferencesParams {
            session_id: open.session_id.clone(),
            symbol_id: mcp_target_symbol_id,
            limit: 50,
            include_snippets: false,
        })
        .await
        .expect("mcp references");
    let mcp_references = normalize_mcp_references(&mcp_references.references);
    assert!(
        !mcp_references.is_empty(),
        "expected non-empty MCP references result"
    );

    let mcp_definition = mcp_manager
        .bsl_definition(BslDefinitionParams {
            session_id: open.session_id,
            symbol_id: None,
            file: Some(McpFileRef {
                doc: McpDocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            }),
            position: Some(McpPosition {
                line: SYMBOL_CALL_LINE,
                character: SYMBOL_CALL_CHARACTER,
            }),
        })
        .await
        .expect("mcp definition");
    let mcp_definition = normalize_mcp_definition(mcp_definition.location.as_ref());
    assert!(
        !mcp_definition.is_empty(),
        "expected non-empty MCP definition result"
    );

    assert_eq!(lsp_members, mcp_members, "LSP/MCP members drift detected");
    assert_eq!(
        lsp_symbols, mcp_symbols,
        "LSP/MCP symbol_search drift detected"
    );
    assert_eq!(
        lsp_references, mcp_references,
        "LSP/MCP references drift detected"
    );
    assert_eq!(
        lsp_definition, mcp_definition,
        "LSP/MCP definition drift detected"
    );

    assert!(
        lsp_hover_text.contains(&mcp_type_name),
        "LSP hover/type_at_position drift detected: expected '{mcp_type_name}' in hover text, got '{lsp_hover_text}'"
    );
    assert!(
        web_hover_text.contains(&mcp_type_name),
        "Web hover/type_at_position drift detected: expected '{mcp_type_name}' in hover text, got '{web_hover_text}'"
    );
}

#[tokio::test]
async fn p26_interactive_warm_path_completion_slo_smoke_conf_big() {
    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();

    let Some(root) = conf_big_root_for_tests() else {
        if allow_fixture_skip {
            eprintln!(
                "skipping p26 warm-path SLO smoke: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
            );
            return;
        }
        panic!(
            "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip this test explicitly"
        );
    };

    let module_path = conf_big_large_module_path_for_tests(&root);
    if !module_path.exists() {
        if allow_fixture_skip {
            eprintln!(
                "skipping p26 warm-path SLO smoke: module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                module_path.display()
            );
            return;
        }
        panic!(
            "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip this test explicitly",
            module_path.display()
        );
    }

    let module_text = std::fs::read_to_string(&module_path).expect("read conf_big module");
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

    let uri = Url::parse("file:///conf_big_perf_module.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: module_text,
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    for _ in 0..50_u64 {
        let completion = server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(0, 0),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            })
            .await
            .expect("completion request");
        assert!(completion.is_some(), "completion response expected");
    }

    // Dedicated concurrent parse burst to exercise parse_result singleflight sharing
    // without polluting completion duration SLO samples.
    let file_id = server
        .get_file_id_v2(&uri)
        .await
        .expect("file_id must be available after didOpen");
    let parse_context = Arc::new(
        server
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Diagnostics,
                file_id,
                None,
                false,
            )
            .await,
    );
    let parse_barrier = Arc::new(std::sync::Barrier::new(9));
    std::thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..8_u32 {
            let runtime = server.analysis_v2.clone();
            let parse_context = parse_context.clone();
            let parse_barrier = parse_barrier.clone();
            let coordinator = coordinator.clone();
            workers.push(scope.spawn(move || {
                parse_barrier.wait();
                let analysis = futures::executor::block_on(runtime.snapshot());
                bsl_runtime::application::IntellisenseV2Facade::run_parse_result_query_singleflight(
                    parse_context.as_ref(),
                    &analysis,
                    true,
                    Some(coordinator.as_ref()),
                    file_id,
                )
            }));
        }
        parse_barrier.wait();
        for worker in workers {
            let result = worker.join().expect("parse burst worker should not panic");
            assert!(
                result.is_ok(),
                "parse burst worker should complete without hard cancellation"
            );
        }
    });

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let rates = metrics
        .get("rates")
        .and_then(|value| value.as_object())
        .expect("metrics.rates object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    let wait_hist = histograms
        .get("intellisense_v2_wait_for_file_version_completion_ms")
        .or_else(|| histograms.get("intellisense_v2_wait_for_file_version_other_ms"))
        .and_then(|value| value.as_object());
    let completion_hist = histograms
        .get("completion_duration_ms")
        .and_then(|value| value.as_object())
        .expect("completion duration histogram");

    let completion_count = completion_hist
        .get("count")
        .and_then(|value| value.as_u64())
        .expect("completion count");
    assert!(
        completion_count >= 50,
        "expected at least 50 completion duration samples, got {completion_count}"
    );

    let wait_p95 = wait_hist
        .and_then(|hist| hist.get("p95"))
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
        .unwrap_or(0.0);
    let queue_wait_interactive_p95 = histograms
        .get("intellisense_v2_runtime_queue_wait_interactive_ms")
        .and_then(|value| value.as_object())
        .and_then(|hist| hist.get("p95"))
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
        .unwrap_or(0.0);
    let completion_p95 = completion_hist
        .get("p95")
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
        .expect("completion p95");
    let parse_result_shared_rate = rates
        .get("intellisense_v2_parse_result_singleflight_shared_rate")
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
        .unwrap_or(0.0);
    let parse_result_cancel_rate = rates
        .get("intellisense_v2_parse_result_query_cancel_rate")
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
        .unwrap_or(0.0);
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120)
        .clamp(10, 2000) as f64;

    assert!(
        wait_p95 <= wait_budget_ms + 20.0,
        "warm-path wait p95 regression: wait_p95={}ms budget={}ms",
        wait_p95,
        wait_budget_ms
    );
    assert!(
        completion_p95 <= 1500.0,
        "warm-path completion p95 regression: completion_p95={}ms > 1500ms",
        completion_p95
    );
    assert!(
        queue_wait_interactive_p95 <= wait_budget_ms + 250.0,
        "warm-path interactive queue-wait p95 regression: queue_wait_interactive_p95={}ms budget={}ms",
        queue_wait_interactive_p95,
        wait_budget_ms
    );
    assert!(
        parse_result_shared_rate >= 0.01,
        "parse_result singleflight shared-rate regression: shared_rate={:.3}",
        parse_result_shared_rate
    );
    assert!(
        parse_result_cancel_rate <= 0.30,
        "parse_result cancel-rate regression: cancel_rate={:.3}",
        parse_result_cancel_rate
    );
    let completion_total = counters
        .get("completion_total")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    assert!(
        completion_total >= 50,
        "expected completion_total >= 50, got {completion_total}"
    );
    let completion_cancelled_total = counters
        .get("intellisense_v2_completion_result_total_cancelled")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let completion_cancelled_rate =
        completion_cancelled_total as f64 / completion_total.max(1) as f64;
    assert!(
        completion_cancelled_rate <= 0.10,
        "warm-path completion cancel-rate regression: cancelled={} total={} rate={:.3}",
        completion_cancelled_total,
        completion_total,
        completion_cancelled_rate
    );

    drain_task.abort();
}

#[tokio::test]
async fn p32_foreign_document_skips_config_semantic_diagnostics() {
    fn write_configuration_xml(root: &std::path::Path, name: &str, common_modules: &[&str]) {
        let mut child_objects = String::new();
        for module in common_modules {
            child_objects.push_str(&format!("<CommonModule>{module}</CommonModule>"));
        }
        std::fs::write(
            root.join("Configuration.xml"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>{name}</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>{child_objects}</ChildObjects>
  </Configuration>
</MetaDataObject>
"#
            ),
        )
        .expect("write Configuration.xml");
    }

    fn write_common_module(root: &std::path::Path, name: &str) {
        std::fs::create_dir_all(root.join("CommonModules")).expect("create CommonModules");
        std::fs::write(
            root.join("CommonModules").join(format!("{name}.xml")),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses" xmlns:v8="http://v8.1c.ru/8.1/data/core">
  <CommonModule uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>{name}</Name>
      <Global>false</Global>
      <ClientManagedApplication>false</ClientManagedApplication>
      <ClientOrdinaryApplication>false</ClientOrdinaryApplication>
      <Server>true</Server>
      <ExternalConnection>false</ExternalConnection>
      <ServerCall>false</ServerCall>
      <Privileged>false</Privileged>
      <ReturnValuesReuse>DontUse</ReturnValuesReuse>
    </Properties>
  </CommonModule>
</MetaDataObject>
"#
            ),
        )
        .expect("write CommonModule xml");
        std::fs::create_dir_all(root.join("CommonModules").join(name).join("Ext"))
            .expect("create common module dir");
        std::fs::write(
            root.join("CommonModules")
                .join(name)
                .join("Ext")
                .join("Module.bsl"),
            "Процедура Ф() Экспорт\nКонецПроцедуры\n",
        )
        .expect("write common module source");
    }

    fn write_form_module(
        root: &std::path::Path,
        document_name: &str,
        form_name: &str,
        code: &str,
    ) -> std::path::PathBuf {
        let path = root
            .join("Documents")
            .join(document_name)
            .join("Forms")
            .join(form_name)
            .join("Ext")
            .join("Form")
            .join("Module.bsl");
        std::fs::create_dir_all(path.parent().expect("form module parent"))
            .expect("create form module dir");
        std::fs::write(&path, code).expect("write form module source");
        path
    }

    let tmp = tempfile::TempDir::new().expect("temp dir");
    let configured_root = tmp.path().join("configured");
    let foreign_root = tmp.path().join("foreign");
    std::fs::create_dir_all(&configured_root).expect("create configured root");
    std::fs::create_dir_all(&foreign_root).expect("create foreign root");

    write_configuration_xml(&configured_root, "ConfiguredConfig", &[]);
    write_configuration_xml(&foreign_root, "ForeignConfig", &["МойМодуль"]);
    write_common_module(&foreign_root, "МойМодуль");

    let configured_form_code = "Процедура Тест()\n    НесуществующийМодуль.Ф();\nКонецПроцедуры\n";
    let configured_form_path =
        write_form_module(&configured_root, "Док1", "Форма1", configured_form_code);
    let foreign_form_code = "Процедура Тест()\n    МойМодуль.Ф();\nКонецПроцедуры\n";
    let foreign_form_path = write_form_module(&foreign_root, "Док2", "Форма2", foreign_form_code);

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

    let (published_tx, mut published_rx) =
        tokio::sync::mpsc::unbounded_channel::<PublishDiagnosticsParams>();
    let drain_task = tokio::spawn(async move {
        while let Some(req) = socket.next().await {
            if req.method() != "textDocument/publishDiagnostics" {
                continue;
            }
            let Some(params) = req.params().cloned() else {
                continue;
            };
            let Ok(parsed) = serde_json::from_value::<PublishDiagnosticsParams>(params) else {
                continue;
            };
            let _ = published_tx.send(parsed);
        }
    });

    initialize_lsp_service(&mut service).await;

    let server = server_holder
        .lock()
        .expect("server holder lock")
        .clone()
        .expect("server must be created");
    *server.config.write().await = Some(crate::config::LspConfig {
        platform_docs_archive: None,
        configuration_path: Some(configured_root.to_string_lossy().to_string()),
        platform_version: Some("8.3.25".to_string()),
        cache_enabled: Some(true),
        strict_fingerprint: Some(false),
        enable_type_hints: Some(false),
        enable_code_actions: Some(false),
    });

    let startup_coordinator = coordinator.clone();
    let startup_root = configured_root.clone();
    tokio::task::spawn_blocking(move || {
        startup_coordinator.start_with_paths_blocking(
            None,
            Some(&startup_root),
            Some("8.3.25"),
            None,
        )
    })
    .await
    .expect("config startup join")
    .expect("config startup");
    server
        .deps_update_v2(
            "p32_foreign_document_skips_config_semantic_diagnostics",
            None,
            Some(configured_root.clone()),
        )
        .await;

    let configured_uri =
        Url::from_file_path(&configured_form_path).expect("configured form module uri");
    let configured_did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: configured_uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: configured_form_code.to_string(),
        },
    };
    let configured_open_req = Request::build("textDocument/didOpen")
        .params(
            serde_json::to_value(configured_did_open)
                .expect("configured DidOpenTextDocumentParams"),
        )
        .finish();
    let configured_open_response = service
        .ready()
        .await
        .unwrap()
        .call(configured_open_req)
        .await
        .expect("configured didOpen notification");
    assert!(
        configured_open_response.is_none(),
        "didOpen is a notification"
    );

    let configured_diagnostics =
        wait_lsp_publish_diagnostics(&mut published_rx, &configured_uri).await;
    assert!(
        configured_diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("Необъявленная переменная")
                && diagnostic.message.contains("НесуществующийМодуль")
        }),
        "expected semantic diagnostic for file inside configured root, got {:?}",
        configured_diagnostics
    );

    let foreign_uri = Url::from_file_path(&foreign_form_path).expect("foreign form module uri");
    let foreign_did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: foreign_uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: foreign_form_code.to_string(),
        },
    };
    let foreign_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(foreign_did_open).expect("foreign DidOpenTextDocumentParams"))
        .finish();
    let foreign_open_response = service
        .ready()
        .await
        .unwrap()
        .call(foreign_open_req)
        .await
        .expect("foreign didOpen notification");
    assert!(foreign_open_response.is_none(), "didOpen is a notification");

    let foreign_diagnostics = wait_any_lsp_publish_diagnostics(
        &mut published_rx,
        &foreign_uri,
        tokio::time::Duration::from_secs(5),
    )
    .await
    .expect("foreign diagnostics publish");
    assert!(
        !foreign_diagnostics.iter().any(|diagnostic| {
            diagnostic.message.contains("Необъявленная переменная")
                && diagnostic.message.contains("МойМодуль")
        }),
        "expected semantic stage to be skipped for file outside configured root, got {:?}",
        foreign_diagnostics
    );

    drain_task.abort();
}

#[tokio::test]
async fn p27_interactive_completion_acceptance_gates_emit_artifact() {
    const CHANGE_ID: &str = "refactor-ir-canonical-semantic-pipeline";
    const ITERATIONS: u64 = 120;
    const MAX_P95_MS: f64 = 300.0;
    const MAX_P99_MS: f64 = 800.0;
    const MIN_FIRST_TRIGGER_SUCCESS_RATE: f64 = 0.99;
    const MAX_TERMINAL_EMPTY_RATE: f64 = 0.005;
    const MAX_PARITY_MISMATCH_RATE: f64 = 0.01;

    fn completion_items_count(response: &CompletionResponse) -> usize {
        match response {
            CompletionResponse::Array(items) => items.len(),
            CompletionResponse::List(list) => list.items.len(),
        }
    }

    fn metric_as_f64(value: Option<&serde_json::Value>) -> f64 {
        value
            .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|v| v as f64)))
            .unwrap_or(0.0)
    }

    fn sum_counters_by_prefix(
        counters: &serde_json::Map<String, serde_json::Value>,
        prefix: &str,
    ) -> u64 {
        counters
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(_, value)| value.as_u64().unwrap_or(0))
            .sum()
    }

    fn sum_counters_by_prefix_and_substring(
        counters: &serde_json::Map<String, serde_json::Value>,
        prefix: &str,
        needle: &str,
    ) -> u64 {
        counters
            .iter()
            .filter(|(key, _)| key.starts_with(prefix) && key.contains(needle))
            .map(|(_, value)| value.as_u64().unwrap_or(0))
            .sum()
    }

    fn stage_mode_counter_total(
        counters: &serde_json::Map<String, serde_json::Value>,
        stage: &str,
        mode: &str,
    ) -> u64 {
        let stage_token = format!("_stage_{stage}");
        counters
            .iter()
            .filter(|(key, _)| {
                key.starts_with("intellisense_v2_drilldown_stage_total_")
                    && key.contains("_origin_lsp_")
                    && key.contains("_operation_completion_")
                    && (key.contains(&format!("{stage_token}_")) || key.ends_with(&stage_token))
                    && key.contains(&format!("_mode_{mode}"))
            })
            .map(|(_, value)| value.as_u64().unwrap_or(0))
            .sum()
    }

    fn stage_mode_latency_p95(
        histograms: &serde_json::Map<String, serde_json::Value>,
        stage: &str,
        mode: &str,
    ) -> f64 {
        let stage_token = format!("_stage_{stage}");
        histograms
            .iter()
            .filter(|(key, _)| {
                key.starts_with("intellisense_v2_drilldown_stage_latency_ms_")
                    && key.contains("_origin_lsp_")
                    && key.contains("_operation_completion_")
                    && (key.contains(&format!("{stage_token}_")) || key.ends_with(&stage_token))
                    && key.contains(&format!("_mode_{mode}"))
            })
            .filter_map(|(_, value)| value.as_object())
            .map(|hist| metric_as_f64(hist.get("p95")))
            .fold(0.0, f64::max)
    }

    fn collect_mode_split_stage_metrics(
        counters: &serde_json::Map<String, serde_json::Value>,
        histograms: &serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        const STAGES: &[&str] = &[
            "runtime_wait_for_file_version",
            "runtime_snapshot_with_deps",
            "ir_query",
            "parse_result_query",
        ];
        const MODES: &[&str] = &["legacy", "event_driven", "shadow"];

        let mut by_mode = serde_json::Map::new();
        for mode in MODES {
            let mut by_stage = serde_json::Map::new();
            for stage in STAGES {
                by_stage.insert(
                    (*stage).to_string(),
                    serde_json::json!({
                        "total": stage_mode_counter_total(counters, stage, mode),
                        "p95_ms": stage_mode_latency_p95(histograms, stage, mode),
                    }),
                );
            }
            by_mode.insert((*mode).to_string(), serde_json::Value::Object(by_stage));
        }
        serde_json::Value::Object(by_mode)
    }

    fn parse_snapshot_mode_counter_total(
        counters: &serde_json::Map<String, serde_json::Value>,
        mode: &str,
    ) -> u64 {
        counters
            .get(&format!(
                "intellisense_v2_parse_snapshot_total_origin_lsp_mode_{mode}"
            ))
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
    }

    fn parse_snapshot_mode_latency_p95(
        histograms: &serde_json::Map<String, serde_json::Value>,
        mode: &str,
    ) -> f64 {
        histograms
            .get(&format!(
                "intellisense_v2_parse_snapshot_build_ms_origin_lsp_mode_{mode}"
            ))
            .and_then(|value| value.as_object())
            .map(|hist| metric_as_f64(hist.get("p95")))
            .unwrap_or(0.0)
    }

    fn collect_parse_snapshot_mode_metrics(
        counters: &serde_json::Map<String, serde_json::Value>,
        histograms: &serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        const PARSE_MODES: &[&str] = &["incremental", "reused", "full", "other"];

        let changed_ranges_count_p95 = histograms
            .get("intellisense_v2_parse_snapshot_changed_ranges_count_origin_lsp")
            .and_then(|value| value.as_object())
            .map(|hist| metric_as_f64(hist.get("p95")))
            .unwrap_or(0.0);
        let changed_ranges_bytes_p95 = histograms
            .get("intellisense_v2_parse_snapshot_changed_ranges_bytes_origin_lsp")
            .and_then(|value| value.as_object())
            .map(|hist| metric_as_f64(hist.get("p95")))
            .unwrap_or(0.0);

        let mut by_mode = serde_json::Map::new();
        for mode in PARSE_MODES {
            by_mode.insert(
                (*mode).to_string(),
                serde_json::json!({
                    "total": parse_snapshot_mode_counter_total(counters, mode),
                    "p95_ms": parse_snapshot_mode_latency_p95(histograms, mode),
                }),
            );
        }

        serde_json::json!({
            "by_mode": by_mode,
            "changed_ranges_count_p95": changed_ranges_count_p95,
            "changed_ranges_bytes_p95": changed_ranges_bytes_p95,
        })
    }

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

    let uri = Url::parse("file:///test_p27_interactive_acceptance_gate.bsl").expect("test uri");
    let text = concat!(
        "Процедура Тест()\n",
        "    ЛокМассив = Новый Массив;\n",
        "    ЛокМассив.\n",
        "КонецПроцедуры\n"
    );
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    let did_open_req = Request::build("textDocument/didOpen")
        .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
        .finish();
    let did_open_response = service
        .ready()
        .await
        .unwrap()
        .call(did_open_req)
        .await
        .expect("didOpen notification");
    assert!(did_open_response.is_none(), "didOpen is a notification");
    let member_character = "    ЛокМассив."
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;

    let mut first_trigger_success_total = 0_u64;
    let mut first_trigger_total = 0_u64;
    let mut parity_pairs_total = 0_u64;

    for iteration in 0..ITERATIONS {
        let version = (iteration + 2) as i32;
        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.to_string(),
            }],
        };
        let did_change_req = Request::build("textDocument/didChange")
            .params(serde_json::to_value(did_change).expect("DidChangeTextDocumentParams"))
            .finish();
        let did_change_response = service
            .ready()
            .await
            .unwrap()
            .call(did_change_req)
            .await
            .expect("didChange notification");
        assert!(did_change_response.is_none(), "didChange is a notification");

        let dot_completion = server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(2, member_character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                    trigger_character: Some(".".to_string()),
                }),
            })
            .await
            .expect("dot completion request")
            .expect("dot completion response");
        first_trigger_total += 1;
        if completion_items_count(&dot_completion) > 0 {
            first_trigger_success_total += 1;
        }

        let invoked_completion = server
            .completion(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: uri.clone() },
                    position: Position::new(2, member_character),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: None,
                }),
            })
            .await
            .expect("invoked completion request")
            .expect("invoked completion response");
        assert!(
            completion_items_count(&invoked_completion) > 0,
            "invoked completion must return non-empty candidates in acceptance gate loop"
        );
        parity_pairs_total += 1;
    }

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let completion_hist = histograms
        .get("completion_duration_ms")
        .and_then(|value| value.as_object())
        .expect("completion duration histogram");
    let completion_p95 = metric_as_f64(completion_hist.get("p95"));
    let completion_p99 = metric_as_f64(completion_hist.get("p99"));
    let mode_split_stage_metrics = collect_mode_split_stage_metrics(counters, histograms);
    let parse_snapshot_mode_metrics = collect_parse_snapshot_mode_metrics(counters, histograms);

    let first_trigger_success_rate =
        first_trigger_success_total as f64 / first_trigger_total.max(1) as f64;
    let terminal_empty_fail_closed_total = [
        "missing_canonical_ir",
        "missing_semantic_index",
        "superseded_revision",
        "cancelled",
        "unavailable_by_contract",
    ]
    .into_iter()
    .map(|reason| {
        sum_counters_by_prefix_and_substring(
            counters,
            "intellisense_v2_completion_member_access_terminal_empty_total_",
            &format!("_reason_{reason}"),
        )
    })
    .sum::<u64>();
    let terminal_empty_rate =
        terminal_empty_fail_closed_total as f64 / first_trigger_total.max(1) as f64;
    let parity_drift_total = sum_counters_by_prefix(
        counters,
        "intellisense_v2_completion_parity_drift_total_mode_",
    );
    let parity_mismatch_rate = parity_drift_total as f64 / parity_pairs_total.max(1) as f64;
    let parity_evidence = serde_json::json!({
        "results": {
            "parity_pairs_total": parity_pairs_total,
            "parity_mismatch_rate": parity_mismatch_rate
        }
    });
    let parity_evidence_verdict = validate_parity_cutover_evidence(&parity_evidence);
    let completion_mode = bsl_runtime::system::global_runtime_config()
        .get_string(bsl_runtime::system::RuntimeKey::IntellisenseV2CompletionMode)
        .unwrap_or_else(|| "on".to_string())
        .to_ascii_lowercase();
    let canary_percent = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2CompletionCanaryPercent)
        .unwrap_or(0)
        .clamp(0, 100) as u8;
    let report_mode_suffix = if completion_mode == "canary" {
        format!("canary-{canary_percent}")
    } else {
        completion_mode.clone()
    };

    let pass = completion_p95 <= MAX_P95_MS
        && completion_p99 <= MAX_P99_MS
        && first_trigger_success_rate >= MIN_FIRST_TRIGGER_SUCCESS_RATE
        && terminal_empty_rate <= MAX_TERMINAL_EMPTY_RATE
        && parity_mismatch_rate <= MAX_PARITY_MISMATCH_RATE
        && parity_evidence_verdict.is_ok();

    let report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": "p27_interactive_completion_acceptance_gates",
        "mode": completion_mode,
        "canary_percent": canary_percent,
        "iterations": ITERATIONS,
        "thresholds": {
            "completion_p95_ms_max": MAX_P95_MS,
            "completion_p99_ms_max": MAX_P99_MS,
            "first_trigger_success_rate_min": MIN_FIRST_TRIGGER_SUCCESS_RATE,
            "terminal_empty_fail_closed_rate_max": MAX_TERMINAL_EMPTY_RATE,
            "parity_mismatch_rate_max": MAX_PARITY_MISMATCH_RATE,
            "parity_pairs_total_min": PARITY_PAIRS_TOTAL_MIN_FOR_CUTOVER,
            "parity_drift_rate_max": PARITY_DRIFT_RATE_MAX_FOR_CUTOVER
        },
        "results": {
            "completion_p95_ms": completion_p95,
            "completion_p99_ms": completion_p99,
            "first_trigger_success_total": first_trigger_success_total,
            "first_trigger_total": first_trigger_total,
            "first_trigger_success_rate": first_trigger_success_rate,
            "terminal_empty_fail_closed_total": terminal_empty_fail_closed_total,
            "terminal_empty_fail_closed_rate": terminal_empty_rate,
            "parity_drift_total": parity_drift_total,
            "parity_pairs_total": parity_pairs_total,
            "parity_mismatch_rate": parity_mismatch_rate,
            "mode_split_stage_metrics": mode_split_stage_metrics,
            "parse_snapshot_mode_metrics": parse_snapshot_mode_metrics
        },
        "pass": pass
    });

    let report_path = std::env::var("BSL_V2_COMPLETION_GATE_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!("{CHANGE_ID}-gate-{report_mode_suffix}.json"))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for v2 completion gate report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report)
            .expect("failed to serialize v2 completion gate report"),
    )
    .expect("failed to write v2 completion gate report");
    println!("v2_completion_gate_report={}", report_path.display());

    assert!(
        completion_p95 <= MAX_P95_MS,
        "acceptance gate failed: completion p95={}ms > {}ms",
        completion_p95,
        MAX_P95_MS
    );
    assert!(
        completion_p99 <= MAX_P99_MS,
        "acceptance gate failed: completion p99={}ms > {}ms",
        completion_p99,
        MAX_P99_MS
    );
    assert!(
        first_trigger_success_rate >= MIN_FIRST_TRIGGER_SUCCESS_RATE,
        "acceptance gate failed: first-trigger success rate={:.4} < {:.4}",
        first_trigger_success_rate,
        MIN_FIRST_TRIGGER_SUCCESS_RATE
    );
    assert!(
        terminal_empty_rate <= MAX_TERMINAL_EMPTY_RATE,
        "acceptance gate failed: terminal-empty(fail_closed) rate={:.4} > {:.4}",
        terminal_empty_rate,
        MAX_TERMINAL_EMPTY_RATE
    );
    assert!(
        parity_mismatch_rate <= MAX_PARITY_MISMATCH_RATE,
        "acceptance gate failed: parity mismatch rate={:.4} > {:.4}",
        parity_mismatch_rate,
        MAX_PARITY_MISMATCH_RATE
    );
    assert!(
        parity_pairs_total >= PARITY_PAIRS_TOTAL_MIN_FOR_CUTOVER,
        "acceptance gate failed: parity pairs total={} < {}",
        parity_pairs_total,
        PARITY_PAIRS_TOTAL_MIN_FOR_CUTOVER
    );
    assert!(
        parity_evidence_verdict.is_ok(),
        "acceptance gate failed: parity evidence invalid: {}",
        parity_evidence_verdict
            .err()
            .unwrap_or_else(|| "unknown".to_string())
    );

    drain_task.abort();
}

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

fn conf_big_root_for_tests() -> Option<std::path::PathBuf> {
    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let candidates = [
        workspace_root.join("examples").join("conf_big"),
        std::path::PathBuf::from("examples/conf_big"),
        std::path::PathBuf::from("../examples/conf_big"),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("Configuration.xml").exists())
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
            panic!(
                "type-index precompute did not yield current exact serve-only artifact for file_id={} (has_task={}, exact_ready={}, observed_version={observed_version:?}, manual_precompute={manual_precompute:?}, exact_ready_after_manual={exact_ready_after_manual})",
                file_id.0,
                has_task,
                exact_ready
            );
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

async fn seed_exact_type_index_for_current_file_version(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
) -> serde_json::Value {
    let analysis = server.analysis_v2.snapshot().await;
    let observed_version = analysis
        .file_version(file_id)
        .expect("file_version for exact type index seed")
        .expect("observed file version for exact type index seed");
    let manual_precompute = analysis
        .precompute_type_index_for_file(file_id, Some(observed_version), 0)
        .expect("manual exact type index precompute for current file version");
    let exact_ready = analysis
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready after exact type index seed");
    assert!(
        exact_ready,
        "manual exact type index seed must produce current serve-only artifact for file_id={}",
        file_id.0
    );
    serde_json::json!({
        "observed_version": observed_version,
        "manual_precompute": format!("{manual_precompute:?}"),
        "exact_ready": exact_ready,
    })
}

fn completion_timeline_trace_stage_duration_ms(
    trace: &serde_json::Value,
    stage_name: &str,
) -> Option<u64> {
    trace
        .get("stages")
        .and_then(|value| value.as_array())
        .and_then(|stages| {
            stages.iter().find_map(|stage| {
                let stage = stage.as_object()?;
                let name = stage.get("name")?.as_str()?;
                if name != stage_name {
                    return None;
                }
                stage.get("duration_ms").and_then(|value| value.as_u64())
            })
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

fn completion_timeline_server_edge_u64(trace: &serde_json::Value, field: &str) -> Option<u64> {
    trace
        .get("server_edge_details")
        .and_then(|value| value.as_object())
        .and_then(|details| details.get(field))
        .and_then(|value| value.as_u64())
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
    server.analysis_v2.apply_changes(vec![
        bsl_analysis_v2::Change::RemoveFile { file_id },
        bsl_analysis_v2::Change::SetFile {
            file_id,
            text: Arc::from(content.to_string()),
            version,
            path: Arc::from(path),
        },
    ]);
    {
        let mut versions = server.latest_received_file_versions_v2.write().await;
        versions.insert(file_id, version);
    }
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

#[tokio::test]
async fn p33_completion_uses_current_revision_head_path_without_exact_artifact() {
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура Тест()\n    Результат = (Новый Массив()).\nКонецПроцедуры\n";
    let uri =
        Url::parse("file:///test_p33_completion_head_without_exact_artifact.bsl").expect("uri");
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
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let completion_position = find_utf16_position_after_marker(fixture, "(Новый Массив()).");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        !completion_labels.is_empty(),
        "member-access completion should use current-revision head path even when exact artifact is missing, labels={completion_labels:?}"
    );
    assert!(
        completion_labels.iter().any(|label| label == "Количество"),
        "head-path completion should surface canonical members for current-revision explicit receiver, labels={completion_labels:?}"
    );
    let timeline = lsp_get_completion_timeline(&mut service, 401, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces.last().expect("head-path completion timeline trace");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "head-path completion trace must expose bounded route in prepare_details, trace={trace:?}"
    );
    assert!(
        trace.get("prepare_details")
            .and_then(|value| value.as_object())
            .is_some_and(|details| details.contains_key("fail_closed_cause")),
        "head-path completion trace must keep fail_closed_cause field present even when route succeeds, trace={trace:?}"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"
            )
        ),
        0,
        "head-path completion should not rely on exact wait deadline, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get("intellisense_v2_completion_fallback_unavailable_total")),
        0,
        "head-path completion should not record fallback_unavailable for explicit current-revision receiver, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit")) > 0,
        "head-path completion must record head-hit route, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_exact_hit")),
        0,
        "head-path completion must not record exact-hit route, counters={counters:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_form_module_object_completion_uses_current_revision_head_path_without_exact_artifact()
{
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";
    let uri = Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl")
        .expect("form module uri");
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
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let completion_position = find_utf16_position_after_marker(fixture, "ДляCompletion = Объект.");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        completion_labels.iter().any(|label| label == "Ссылка"),
        "head-path completion for FormModule.Объект must include form-data property Ссылка, labels={completion_labels:?}"
    );
    assert!(
        completion_labels
            .iter()
            .any(|label| label == "ПометкаУдаления"),
        "head-path completion for FormModule.Объект must include form-data property ПометкаУдаления, labels={completion_labels:?}"
    );
    assert!(
        !completion_labels
            .iter()
            .any(|label| label == "ПолучитьСсылкуНового"),
        "head-path completion for FormModule.Объект must not leak object-facet method ПолучитьСсылкуНового, labels={completion_labels:?}"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 403, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("form-module head-path completion timeline trace");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "form-module head-path completion trace must expose bounded route in prepare_details, trace={trace:?}"
    );
    assert!(
        trace.get("prepare_details")
            .and_then(|value| value.as_object())
            .is_some_and(|details| details.contains_key("fail_closed_cause")),
        "form-module head-path completion trace must keep fail_closed_cause field present even when route succeeds, trace={trace:?}"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"
            )
        ),
        0,
        "form-module head-path completion should not rely on exact wait deadline, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get("intellisense_v2_completion_fallback_unavailable_total")),
        0,
        "form-module head-path completion should not record fallback_unavailable, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit")) > 0,
        "form-module head-path completion must record head-hit route, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_exact_hit")),
        0,
        "form-module head-path completion must not record exact-hit route, counters={counters:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_form_module_head_path_skips_ir_query_delay_when_owner_hints_are_ready() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _ir_delay_guard = EnvVarGuard::set("BSL_TEST_COMPLETION_IR_QUERY_DELAY_MS", "400");

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let fixture = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";
    let uri = Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl")
        .expect("form module uri");
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
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let metrics_before = coordinator.observability_metrics();
    let counters_before = metrics_before
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_before.counters object");

    let completion_position = find_utf16_position_after_marker(fixture, "ДляCompletion = Объект.");
    let started = Instant::now();
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    let elapsed = started.elapsed();
    assert!(
        completion_labels.iter().any(|label| label == "Ссылка"),
        "head-path completion for FormModule.Объект must remain non-empty even when IR query delay is injected, labels={completion_labels:?}"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "head-path completion must stay bounded and skip IR delay (elapsed={elapsed:?})"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 404, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("form-module head-path completion timeline trace with ir delay");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "form-module head-path completion with injected IR delay must still expose head route, trace={trace:?}"
    );
    assert!(
        completion_timeline_trace_stage_duration_ms(trace, "query_bundle").unwrap_or(u64::MAX)
            < 250,
        "head-path query_bundle must not inherit injected IR delay, trace={trace:?}"
    );

    let metrics_after = coordinator.observability_metrics();
    let counters_after = metrics_after
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_after.counters object");
    let ir_query_delta =
        read_u64_metric(counters_after.get("intellisense_v2_ir_query_completion_total"))
            .saturating_sub(read_u64_metric(
                counters_before.get("intellisense_v2_ir_query_completion_total"),
            ));
    assert_eq!(
        ir_query_delta, 0,
        "head-path completion must not execute completion IR query when owner hints are already ready, counters_before={counters_before:?}, counters_after={counters_after:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn p33_completion_current_revision_head_ignores_did_change_inline_parse_delay() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    fn extract_completion_labels(
        response: tower_lsp::lsp_types::CompletionResponse,
    ) -> Vec<String> {
        match response {
            tower_lsp::lsp_types::CompletionResponse::Array(items) => {
                items.into_iter().map(|item| item.label).collect()
            }
            tower_lsp::lsp_types::CompletionResponse::List(list) => {
                list.items.into_iter().map(|item| item.label).collect()
            }
        }
    }

    const FIXTURE: &str = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _parse_delay_guard = EnvVarGuard::set("BSL_TEST_DID_CHANGE_PARSE_DELAY_MS", "1500");

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl")
        .expect("form module uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
        }],
    };
    let did_change_server = server.clone();
    let did_change_handle = tokio::spawn(async move {
        did_change_server.did_change(did_change).await;
    });

    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if super::super::language_server::did_change_inline_parse_delay_active_for_test()
                && server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange must enter delayed inline parse window");
    assert_eq!(
        server
            .analysis_v2
            .file_revision_state(file_id)
            .await
            .map(|state| state.version),
        Some(2),
        "current-revision apply must reach analysis runtime before delayed inline parse completes"
    );

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = Объект.");
    let started = Instant::now();
    let completion_response = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: completion_position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        })
        .await
        .expect("completion request")
        .expect("completion response");
    let elapsed = started.elapsed();
    let completion_labels = extract_completion_labels(completion_response);
    assert!(
        completion_labels.iter().any(|label| label == "Ссылка"),
        "current-revision head path must stay available while didChange inline parse is delayed, labels={completion_labels:?}"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "head-path completion must not wait for delayed didChange inline parse to finish (elapsed={elapsed:?})"
    );
    assert!(
        did_change_handle.is_finished(),
        "didChange must already return while delayed parse continues in background"
    );

    did_change_handle.await.expect("didChange join");

    let timeline = lsp_get_completion_timeline(&mut service, 405, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace after delayed didChange inline parse");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "current-revision completion must still expose head_hit route while didChange parse remains in-flight, trace={trace:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_completion_service_first_poll_ignores_blocking_did_change_parse_delay() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    const FIXTURE: &str = "Процедура Тест()\n    ДляCompletion = Объект.\nКонецПроцедуры\n";

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;
    let mut service = crate::server::request_context::RequestContextService::new(service);

    let uri = Url::parse("file:///test_p33_completion_service_first_poll_blocking_parse.bsl")
        .expect("form module uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
        }],
    };
    let did_change_server = server.clone();
    let did_change_handle = tokio::spawn(async move {
        did_change_server.did_change(did_change).await;
    });

    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(2)
                && server
                    .analysis_v2
                    .file_revision_state(file_id)
                    .await
                    .map(|state| state.version)
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didChange must publish current revision before blocking parse");

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = Объект.");
    let started = Instant::now();
    let completion_labels = lsp_completion_labels_with_request(
        &mut service,
        406,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    )
    .await;
    let elapsed = started.elapsed();
    let timeline = lsp_get_completion_timeline(&mut service, 4060, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace after blocking didChange parse");
    assert!(
        elapsed < Duration::from_millis(250),
        "completion must not inherit blocking didChange parse delay before first poll (elapsed={elapsed:?}, labels={completion_labels:?}, trace={trace:?})"
    );
    assert!(
        did_change_handle.is_finished(),
        "didChange must already return while blocking parse continues in background"
    );

    did_change_handle.await.expect("didChange join");

    let service_future_to_first_poll_wait_ms = trace
        .get("server_edge_details")
        .and_then(|value| value.as_object())
        .and_then(|details| details.get("service_future_to_first_poll_wait_ms"))
        .and_then(|value| value.as_u64())
        .expect("service_future_to_first_poll_wait_ms");
    assert!(
        service_future_to_first_poll_wait_ms < 250,
        "service future first poll must not inherit blocking didChange parse delay, trace={trace:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_changed_text_current_revision_head_stays_available_while_parse_snapshot_builds() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    const V1_FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";
    const V2_FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    S.Вставить(\"Описание\", \"x\");\n    ДляCompletion = S.\nКонецПроцедуры\n";

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_changed_text_current_revision_head.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: V1_FIXTURE.to_string(),
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
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: V2_FIXTURE.to_string(),
        }],
    };
    let did_change_server = server.clone();
    let did_change_handle = tokio::spawn(async move {
        did_change_server.did_change(did_change).await;
    });

    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(2)
                && server
                    .analysis_v2
                    .file_revision_state(file_id)
                    .await
                    .map(|state| state.version)
                    == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect(
        "didChange must publish changed current revision before blocking parse snapshot completes",
    );

    let completion_position = find_utf16_position_after_marker(V2_FIXTURE, "ДляCompletion = S.");
    let started = Instant::now();
    let completion_response = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: completion_position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        })
        .await
        .expect("completion request")
        .expect("completion response");
    let elapsed = started.elapsed();
    let completion_labels: Vec<String> = match completion_response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    };
    assert!(
        completion_labels.iter().any(|label| label == "Описание"),
        "changed-text current-revision head must expose new member before parse snapshot completes, labels={completion_labels:?}"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "changed-text current-revision head must stay bounded while parse snapshot builds in background (elapsed={elapsed:?})"
    );
    assert!(
        did_change_handle.is_finished(),
        "didChange must already return while changed-text parse snapshot build continues in background"
    );

    did_change_handle.await.expect("didChange join");

    let timeline = lsp_get_completion_timeline(&mut service, 4061, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace after changed-text didChange");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "changed-text completion must resolve through current-revision head route, trace={trace:?}"
    );
    assert_ne!(
        completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
        Some("exact_deadline"),
        "changed-text current-revision head must not regress into exact_deadline while parse snapshot is still building, trace={trace:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_changed_text_current_revision_head_waits_for_delayed_runtime_apply() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    const V1_FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";
    const V2_FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    S.Вставить(\"Описание\", \"x\");\n    ДляCompletion = S.\nКонецПроцедуры\n";

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "300");
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_changed_text_runtime_apply_delay.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: V1_FIXTURE.to_string(),
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
    let did_change_started = Instant::now();
    server
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: V2_FIXTURE.to_string(),
            }],
        })
        .await;
    let did_change_elapsed = did_change_started.elapsed();
    assert!(
        did_change_elapsed < Duration::from_millis(250),
        "didChange must return before delayed runtime apply completes (elapsed={did_change_elapsed:?})"
    );

    server.cancel_type_index_precompute_v2(file_id).await;

    tokio::time::timeout(Duration::from_millis(1200), async {
        loop {
            if server
                .analysis_v2
                .file_revision_state(file_id)
                .await
                .map(|state| state.version)
                == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("delayed runtime apply must eventually publish version 2");

    let completion_position = find_utf16_position_after_marker(V2_FIXTURE, "ДляCompletion = S.");
    let completion_response = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: completion_position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        })
        .await
        .expect("completion request")
        .expect("completion response");
    let completion_labels: Vec<String> = match completion_response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    };
    assert!(
        completion_labels.iter().any(|label| label == "Описание"),
        "current-revision head must survive delayed runtime apply and expose latest member on first response, labels={completion_labels:?}"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 4062, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace after delayed runtime apply");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "delayed runtime apply must still resolve through current-revision head route, trace={trace:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_changed_text_burst_supersedes_obsolete_current_revision_head_precompute() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");

    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let head_precompute_delay_ms = (wait_budget_ms / 3).max(40);
    let _current_head_delay_guard = EnvVarGuard::set(
        "BSL_TEST_CURRENT_REVISION_HEAD_PRECOMPUTE_DELAY_MS",
        &head_precompute_delay_ms.to_string(),
    );
    let _async_parse_delay_guard = EnvVarGuard::set("BSL_TEST_DID_CHANGE_PARSE_DELAY_MS", "500");

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_changed_text_burst_current_revision_supersession.bsl")
        .expect("uri");
    let mut current_text =
        "Процедура Тест()\n    S = Новый Структура;\n    ДляCompletion = S.\nКонецПроцедуры\n"
            .to_string();
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: current_text.clone(),
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

    let latest_version = 8_i32;
    for version in 2..=latest_version {
        let insert_line = format!("    S.Вставить(\"Поле{version}\", {version});\n");
        current_text = current_text.replacen(
            "    ДляCompletion = S.\n",
            &(insert_line + "    ДляCompletion = S.\n"),
            1,
        );
        server
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: current_text.clone(),
                }],
            })
            .await;
        server.cancel_type_index_precompute_v2(file_id).await;
    }

    let completion_position = find_utf16_position_after_marker(&current_text, "ДляCompletion = S.");
    let completion_started = Instant::now();
    let completion_response = server
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: completion_position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::INVOKED,
                trigger_character: None,
            }),
        })
        .await
        .expect("completion request")
        .expect("completion response");
    let completion_elapsed = completion_started.elapsed();
    let completion_labels: Vec<String> = match completion_response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    };
    assert!(
        completion_labels
            .iter()
            .any(|label| label == &format!("Поле{latest_version}")),
        "burst changed-text path must preserve latest current-revision head instead of burning CPU on obsolete versions, labels={completion_labels:?}"
    );
    assert!(
        completion_elapsed < Duration::from_millis(wait_budget_ms.saturating_mul(2)),
        "burst changed-text completion must stay bounded after current-revision head supersession (elapsed={completion_elapsed:?}, latest_version={latest_version}, labels={completion_labels:?})"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 4063, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace after changed-text burst supersession");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "burst changed-text completion must resolve through latest current-revision head route, trace={trace:?}"
    );
    assert_ne!(
        completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
        Some("exact_deadline"),
        "burst changed-text completion must not regress into exact_deadline when obsolete head precompute work is superseded, trace={trace:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_completion_head_hit_then_upgrade_after_precompute_finish() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    const FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK);
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = wait_budget_ms.saturating_add(500).max(400);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_completion_exact_wait_recovery.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
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
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        super::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = S.");
    let first_started = Instant::now();
    let first_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    let first_elapsed = first_started.elapsed();
    assert!(
        first_completion_labels.iter().any(|label| label == "Количество"),
        "first member-access completion must serve typed-structure members from current-revision head while matching exact precompute is still computing, labels={first_completion_labels:?}"
    );
    assert!(
        first_elapsed < Duration::from_millis(250),
        "first head-path completion should stay bounded while exact precompute runs in background (elapsed={first_elapsed:?}, budget_ms={wait_budget_ms})"
    );

    wait_for_type_index_precompute_completion(&server, file_id).await;

    let second_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        second_completion_labels.iter().any(|label| label == "Количество"),
        "member-access completion must keep typed-structure members available after exact precompute finishes, labels={second_completion_labels:?}"
    );
    let timeline = lsp_get_completion_timeline(&mut service, 402, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let first_trace = traces
        .get(traces.len().saturating_sub(2))
        .expect("first head-hit completion trace");
    let second_trace = traces
        .last()
        .expect("second completion trace after exact precompute");
    assert_eq!(
        completion_timeline_prepare_detail_str(first_trace, "route"),
        Some("head_hit"),
        "first completion trace must expose current-revision head route while exact precompute is still computing, trace={first_trace:?}"
    );
    assert!(
        matches!(
            completion_timeline_prepare_detail_str(second_trace, "route"),
            Some("head_hit" | "exact_hit")
        ),
        "completion after exact precompute must stay on canonical head/exact route, trace={second_trace:?}"
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    assert_eq!(
        read_u64_metric(
            counters.get(
                "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"
            )
        ),
        0,
        "typed-structure head path must not regress into exact-deadline fail-closed, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task")
        ),
        0,
        "typed-structure head path must not degrade to no_matching_task while exact precompute is present, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_task_present_wrong_version")
        ),
        0,
        "typed-structure head path must not report wrong_version while serving current revision, counters={counters:?}"
    );
    assert_eq!(
        read_u64_metric(
            counters.get("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
        ),
        0,
        "typed-structure head path must not attribute completion to exact-deadline once head artifact is available, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit")) > 0,
        "typed-structure head path must record head-hit route before exact upgrade, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_head_to_exact_upgrade_total")) > 0,
        "background exact precompute must still record head-to-exact upgrade for the same revision, counters={counters:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p33_current_revision_head_precompute_stays_available_under_background_cpu_saturation() {
    const V1_FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    ДляCompletion = S.\nКонецПроцедуры\n";
    const V2_FIXTURE: &str = "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";

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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let total_cpu_permits = std::thread::available_parallelism()
        .map(|parallelism| parallelism.get().max(2))
        .unwrap_or(4);
    let interactive_reserved = if total_cpu_permits >= 4 { 2 } else { 1 };
    let background_blocker_count = total_cpu_permits
        .saturating_sub(interactive_reserved)
        .max(1);
    let mut blocker_handles = Vec::new();
    let mut blocker_started = Vec::new();
    for _ in 0..background_blocker_count {
        let (started_tx, started_rx) = tokio::sync::oneshot::channel::<()>();
        blocker_handles.push(tokio::spawn(async move {
            bsl_runtime::application::spawn_bounded_blocking_with_class(
                bsl_runtime::application::CpuWorkClass::Background,
                move || {
                    let _ = started_tx.send(());
                    std::thread::sleep(Duration::from_millis(400));
                },
            )
            .await
            .expect("background blocker join");
        }));
        blocker_started.push(started_rx);
    }
    for started_rx in blocker_started {
        started_rx
            .await
            .expect("background blocker should acquire non-interactive CPU permit");
    }

    let uri = Url::parse("file:///test_p33_current_revision_head_under_background_saturation.bsl")
        .expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: V1_FIXTURE.to_string(),
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

    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: V2_FIXTURE.to_string(),
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
    let file_id = server.get_or_create_file_id_v2(&uri).await;

    tokio::time::timeout(Duration::from_millis(1200), async {
        loop {
            if server
                .analysis_v2
                .file_revision_state(file_id)
                .await
                .map(|state| state.version)
                == Some(2)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("current revision apply must reach version 2 before head-fast-lane measurement");

    let completion_position = find_utf16_position_after_marker(V2_FIXTURE, "ДляCompletion = S.");
    let head_started = Instant::now();
    let head_owner_hints = tokio::time::timeout(Duration::from_millis(250), async {
        loop {
            let analysis = server.analysis_v2.snapshot().await;
            let head_ready = analysis
                .current_completion_head_ready(file_id)
                .ok()
                .unwrap_or(false);
            let Some(file_text) = analysis.file_text(file_id).ok().flatten() else {
                tokio::task::yield_now().await;
                continue;
            };
            let owner_hints = bsl_runtime::application::completion_member_access_owner_type_hints_from_completion_head(
                &analysis,
                file_id,
                file_text.as_ref(),
                completion_position.line,
                completion_position.character,
            );
            if head_ready && !owner_hints.is_empty() {
                break owner_hints;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("current-revision completion head must stay available under background CPU saturation");
    let head_elapsed = head_started.elapsed();
    assert!(
        head_elapsed < Duration::from_millis(250),
        "current-revision completion head must stay bounded under background CPU saturation (elapsed={head_elapsed:?}, owner_hints={head_owner_hints:?}, blockers={background_blocker_count}, total_permits={total_cpu_permits})"
    );

    let started = Instant::now();
    let completion_response = tokio::time::timeout(
        Duration::from_millis(1200),
        server.completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: completion_position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some(".".to_string()),
            }),
        }),
    )
    .await
    .expect("completion should eventually finish after head fast-lane availability under background CPU saturation")
    .expect("completion request")
    .expect("completion response");
    let elapsed = started.elapsed();
    let completion_labels: Vec<String> = match completion_response {
        CompletionResponse::Array(items) => items.into_iter().map(|item| item.label).collect(),
        CompletionResponse::List(list) => list.items.into_iter().map(|item| item.label).collect(),
    };
    assert!(
        completion_labels.iter().any(|label| label == "Количество"),
        "current-revision head must remain available under background CPU saturation, labels={completion_labels:?}"
    );
    assert!(
        elapsed < Duration::from_millis(1200),
        "completion should eventually resolve after current-revision head becomes available (elapsed={elapsed:?}, blockers={background_blocker_count}, total_permits={total_cpu_permits})"
    );

    let timeline = lsp_get_completion_timeline(&mut service, 4064, 10).await;
    let traces = timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let trace = traces
        .last()
        .expect("completion trace under background CPU saturation");
    assert_eq!(
        completion_timeline_prepare_detail_str(trace, "route"),
        Some("head_hit"),
        "background CPU saturation must still resolve completion through current-revision head route, trace={trace:?}"
    );
    assert_ne!(
        completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
        Some("exact_deadline"),
        "background CPU saturation must not regress into exact_deadline when current-revision head should take the fast lane, trace={trace:?}"
    );

    for blocker_handle in blocker_handles {
        blocker_handle.await.expect("background blocker task");
    }

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_completion_head_hit_emits_exact_upgrade_when_background_exact_finishes() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    const FIXTURE: &str = "Процедура Тест()\n    Результат = (Новый Массив()).\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK);
    let _precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS", "200");
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_completion_head_to_exact_upgrade.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
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
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        super::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "(Новый Массив()).");
    let completion_labels = lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    assert!(
        !completion_labels.is_empty(),
        "head route must still provide current-revision completion while exact precompute computes in background, labels={completion_labels:?}"
    );

    wait_for_type_index_precompute_completion(&server, file_id).await;

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit")) > 0,
        "head route must be recorded before upgrade, counters={counters:?}"
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_head_to_exact_upgrade_total")) > 0,
        "background exact precompute must record head-to-exact upgrade for same revision, counters={counters:?}"
    );
    assert!(
        histograms
            .get("intellisense_v2_completion_head_to_exact_upgrade_ms")
            .and_then(|value| value.get("count"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "head-to-exact upgrade latency histogram must be emitted, histograms={histograms:?}"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_completion_head_and_exact_resolve_keep_candidate_id_stable_for_same_revision() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    const FIXTURE: &str = "Процедура Тест()\n    Результат = (Новый Массив()).\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK);
    let _precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS", "200");
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_completion_resolve_candidate_parity.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
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
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        super::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "(Новый Массив()).");
    let head_items = lsp_completion_items_with_request(
        &mut service,
        14001,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    )
    .await;
    let head_item = head_items
        .into_iter()
        .find(|item| item.label == "Добавить")
        .expect("head completion item for Добавить");
    let head_candidate_id = head_item
        .data
        .as_ref()
        .and_then(|value| value.get("candidate_id"))
        .cloned()
        .expect("head completion candidate_id");

    wait_for_type_index_precompute_completion(&server, file_id).await;

    let resolved_head_item =
        lsp_completion_resolve_item_with_request(&mut service, 14002, head_item.clone()).await;
    assert!(
        resolved_head_item.detail != head_item.detail
            || resolved_head_item.documentation != head_item.documentation,
        "head item must gain exact resolve enrichment once the same revision exact path is ready, head_item={head_item:?}, resolved_head_item={resolved_head_item:?}"
    );

    let exact_items = lsp_completion_items_with_request(
        &mut service,
        14003,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    )
    .await;
    let exact_item = exact_items
        .into_iter()
        .find(|item| item.label == "Добавить")
        .expect("exact completion item for Добавить");
    let exact_candidate_id = exact_item
        .data
        .as_ref()
        .and_then(|value| value.get("candidate_id"))
        .cloned()
        .expect("exact completion candidate_id");
    assert_eq!(
        exact_candidate_id, head_candidate_id,
        "same-revision head response and exact response must preserve stable candidate_id"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_completion_resolve_stays_bound_to_origin_revision() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    const FIXTURE: &str = "Процедура Тест()\n    Результат = (Новый Массив()).\nКонецПроцедуры\n";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK);
    let _precompute_delay_guard =
        EnvVarGuard::set("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS", "200");
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_completion_resolve_revision_binding.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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
    let did_change_v2 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
        }],
    };
    let did_change_v2_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change_v2).expect("DidChangeTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(
        did_change_v2_response.is_none(),
        "didChange is a notification"
    );
    server.sync_v2_globals().await;
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        super::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "(Новый Массив()).");
    let head_items = lsp_completion_items_with_request(
        &mut service,
        14011,
        &uri,
        completion_position,
        Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
            trigger_character: Some(".".to_string()),
        }),
    )
    .await;
    let head_item = head_items
        .into_iter()
        .find(|item| item.label == "Добавить")
        .expect("head completion item for Добавить");

    let did_change_v3 = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 3,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
        }],
    };
    let did_change_v3_response = service
        .ready()
        .await
        .unwrap()
        .call(
            Request::build("textDocument/didChange")
                .params(serde_json::to_value(did_change_v3).expect("DidChangeTextDocumentParams"))
                .finish(),
        )
        .await
        .expect("didChange notification");
    assert!(
        did_change_v3_response.is_none(),
        "didChange is a notification"
    );
    server.sync_v2_globals().await;

    let resolved_stale_item =
        lsp_completion_resolve_item_with_request(&mut service, 14012, head_item.clone()).await;
    assert_eq!(
        resolved_stale_item.detail, head_item.detail,
        "resolve must stay bound to the origin revision and fail closed once a newer revision supersedes the item"
    );
    assert_eq!(
        resolved_stale_item.documentation, head_item.documentation,
        "stale resolve must not enrich documentation from a newer revision"
    );

    drain_task.abort();
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn p33_completion_head_upgrade_perf_report() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    const FIXTURE: &str =
        "Процедура Тест()\n    S = Новый Структура;\n    S.Вставить(\"Количество\", 10);\n    ДляCompletion = S.\nКонецПроцедуры\n";
    const PROFILE_NAME: &str = "p33_completion_head_upgrade_perf_report";

    let _env_lock = lock_test_env_mutex(&PRECOMPUTE_DELAY_ENV_LOCK);
    let wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);
    let precompute_delay_ms = wait_budget_ms.saturating_add(500).max(400);
    let _precompute_delay_guard = EnvVarGuard::set(
        "BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS",
        &precompute_delay_ms.to_string(),
    );
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
        .expect("server must be captured");
    prime_server_with_syntax_helper_deps(&server).await;

    let uri = Url::parse("file:///test_p33_completion_exact_wait_recovery_perf.bsl").expect("uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: FIXTURE.to_string(),
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
    let did_change = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: FIXTURE.to_string(),
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
    wait_for_type_index_precompute_phase(
        &server,
        file_id,
        super::deps_and_precompute::TypeIndexPrecomputePhaseV2::Computing,
    )
    .await;

    let completion_position = find_utf16_position_after_marker(FIXTURE, "ДляCompletion = S.");
    let first_started = Instant::now();
    let first_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    let first_elapsed_ms = first_started.elapsed().as_millis() as u64;
    assert!(
        first_completion_labels.iter().any(|label| label == "Количество"),
        "first member-access completion must serve typed-structure members from current-revision head while matching exact precompute is still computing, labels={first_completion_labels:?}"
    );
    assert!(
        first_elapsed_ms < 250,
        "first head-path completion must stay bounded while exact precompute runs in background, elapsed_ms={first_elapsed_ms}"
    );

    wait_for_type_index_precompute_completion(&server, file_id).await;

    let second_started = Instant::now();
    let second_completion_labels =
        lsp_completion_labels_at(&mut service, &uri, completion_position).await;
    let second_elapsed_ms = second_started.elapsed().as_millis() as u64;
    assert!(
        second_completion_labels.iter().any(|label| label == "Количество"),
        "member-access completion must keep typed-structure members available after exact precompute finishes, labels={second_completion_labels:?}"
    );
    assert!(
        second_elapsed_ms < 250,
        "second completion must stay bounded after exact precompute finishes, elapsed_ms={second_elapsed_ms}"
    );

    let completion_timeline = lsp_get_completion_timeline(&mut service, 13321, 10).await;
    let observability_metrics = lsp_get_observability_metrics(&mut service, 13322).await;

    let timeline_traces = completion_timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let filtered_traces: Vec<serde_json::Value> = timeline_traces
        .iter()
        .filter(|trace| trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str()))
        .cloned()
        .collect();
    assert!(
        filtered_traces.len() >= 2,
        "expected at least two completion traces for perf report, filtered_traces={filtered_traces:?}"
    );
    let mut selected_traces: Vec<serde_json::Value> =
        filtered_traces.iter().rev().take(2).cloned().collect();
    selected_traces.reverse();
    let first_trace = &selected_traces[0];
    let second_trace = &selected_traces[1];

    let counters = observability_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let histograms = observability_metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    let first_trace_id = first_trace
        .get("trace_id")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let first_outcome = first_trace
        .get("outcome")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let first_prepare_guard_outcome =
        completion_timeline_prepare_detail_str(first_trace, "guard_outcome").map(str::to_string);
    let first_prepare_outcome =
        completion_timeline_prepare_detail_str(first_trace, "outcome").map(str::to_string);
    let first_wait_exact_type_index_ms =
        completion_timeline_trace_stage_duration_ms(first_trace, "wait_exact_type_index")
            .unwrap_or(0);
    let first_prepare_stateful_ms =
        completion_timeline_trace_stage_duration_ms(first_trace, "prepare_stateful").unwrap_or(0);

    let second_trace_id = second_trace
        .get("trace_id")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let second_outcome = second_trace
        .get("outcome")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let second_prepare_guard_outcome =
        completion_timeline_prepare_detail_str(second_trace, "guard_outcome").map(str::to_string);
    let second_prepare_outcome =
        completion_timeline_prepare_detail_str(second_trace, "outcome").map(str::to_string);
    let second_wait_exact_type_index_ms =
        completion_timeline_trace_stage_duration_ms(second_trace, "wait_exact_type_index")
            .unwrap_or(0);
    let second_query_bundle_ms =
        completion_timeline_trace_stage_duration_ms(second_trace, "query_bundle").unwrap_or(0);
    let second_collect_ms =
        completion_timeline_trace_stage_duration_ms(second_trace, "collect").unwrap_or(0);

    let deadline_total = read_u64_metric(
        counters
            .get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"),
    );
    let ready_total = read_u64_metric(
        counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready"),
    );
    let no_matching_task_total = read_u64_metric(counters.get(
        "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task",
    ));
    let fail_closed_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_fail_closed"));
    let ok_non_empty_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_ok_non_empty"));
    let head_hit_total =
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit"));
    let exact_hit_total =
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_exact_hit"));
    let head_to_exact_upgrade_total =
        read_u64_metric(counters.get("intellisense_v2_completion_head_to_exact_upgrade_total"));

    let selected_observability = serde_json::json!({
        "completion_duration_ms": histogram_metric_value(histograms, "completion_duration_ms", None),
        "intellisense_v2_wait_for_file_version_completion_ms": histogram_metric_value_or_zero(
            histograms,
            "intellisense_v2_wait_for_file_version_completion_ms",
            None
        ),
        "intellisense_v2_ir_query_completion_ms": histogram_metric_value_or_zero(
            histograms,
            "intellisense_v2_ir_query_completion_ms",
            None
        ),
        "completion_stage_prepare_stateful_ms": histogram_metric_value_or_zero(
            histograms,
            "completion_stage_prepare_stateful_ms",
            None
        ),
        "completion_stage_query_bundle_ms": histogram_metric_value_or_zero(
            histograms,
            "completion_stage_query_bundle_ms",
            None
        ),
        "completion_stage_collect_ms": histogram_metric_value_or_zero(
            histograms,
            "completion_stage_collect_ms",
            None
        ),
        "completion_stage_response_build_ms": histogram_metric_value_or_zero(
            histograms,
            "completion_stage_response_build_ms",
            None
        ),
        "intellisense_v2_completion_route_total_route_head_hit": head_hit_total,
        "intellisense_v2_completion_route_total_route_exact_hit": exact_hit_total,
        "intellisense_v2_completion_head_to_exact_upgrade_total": head_to_exact_upgrade_total,
        "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline": deadline_total,
        "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready": ready_total,
        "intellisense_v2_completion_fallback_unavailable_total": read_u64_metric(
            counters.get("intellisense_v2_completion_fallback_unavailable_total")
        ),
        "intellisense_v2_interactive_wait_budget_exhausted_total": read_u64_metric(
            counters.get("intellisense_v2_interactive_wait_budget_exhausted_total")
        ),
    });

    let report = serde_json::json!({
        "change_id": "refactor-v2-completion-dual-artifact-path",
        "profile": PROFILE_NAME,
        "schema_version": 1,
        "fixture": {
            "uri": uri.as_str(),
            "file_id": file_id.0,
            "marker": "ДляCompletion = S.",
            "precompute_delay_ms": precompute_delay_ms,
            "wait_budget_ms": wait_budget_ms,
        },
        "requests": {
            "first": {
                "expected_behavior": "ok_non_empty_head_hit_while_exact_computes",
                "elapsed_ms": first_elapsed_ms,
                "label_count": first_completion_labels.len(),
                "labels": first_completion_labels,
            },
            "second": {
                "expected_behavior": "ok_non_empty_after_exact_upgrade",
                "elapsed_ms": second_elapsed_ms,
                "label_count": second_completion_labels.len(),
                "labels": second_completion_labels,
            }
        },
        "summary": {
            "first_trace_id": first_trace_id,
            "first_outcome": first_outcome,
            "first_prepare_guard_outcome": first_prepare_guard_outcome,
            "first_prepare_outcome": first_prepare_outcome,
            "first_wait_exact_type_index_ms": first_wait_exact_type_index_ms,
            "first_prepare_stateful_ms": first_prepare_stateful_ms,
            "second_trace_id": second_trace_id,
            "second_outcome": second_outcome,
            "second_prepare_guard_outcome": second_prepare_guard_outcome,
            "second_prepare_outcome": second_prepare_outcome,
            "second_wait_exact_type_index_ms": second_wait_exact_type_index_ms,
            "second_query_bundle_ms": second_query_bundle_ms,
            "second_collect_ms": second_collect_ms,
            "head_hit_total": head_hit_total,
            "exact_hit_total": exact_hit_total,
            "head_to_exact_upgrade_total": head_to_exact_upgrade_total,
            "deadline_total": deadline_total,
            "ready_total": ready_total,
            "no_matching_task_total": no_matching_task_total,
            "fail_closed_total": fail_closed_total,
            "ok_non_empty_total": ok_non_empty_total,
        },
        "completion_timeline": {
            "trace_count": filtered_traces.len(),
            "selected_traces": selected_traces,
            "raw": completion_timeline,
        },
        "observability": {
            "selected": selected_observability,
            "raw": observability_metrics,
        }
    });

    let report_path = std::env::var("BSL_V2_COMPLETION_DEADLINE_RECOVERY_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join("completion-head-upgrade-perf.json")
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for p33 completion perf report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize p33 completion perf report"),
    )
    .expect("write p33 completion perf report");
    println!("{}_path={}", PROFILE_NAME, report_path.display());

    assert_eq!(
        first_trace.get("outcome").and_then(|value| value.as_str()),
        Some("ok_non_empty"),
        "first perf trace must capture ok_non_empty head response, trace={first_trace:?}"
    );
    assert_eq!(
        second_trace.get("outcome").and_then(|value| value.as_str()),
        Some("ok_non_empty"),
        "second perf trace must capture post-upgrade completion outcome, trace={second_trace:?}"
    );
    assert_eq!(
        completion_timeline_prepare_detail_str(first_trace, "route"),
        Some("head_hit"),
        "first perf trace must capture head-hit route while exact precompute is still computing, trace={first_trace:?}"
    );
    assert!(
        matches!(
            completion_timeline_prepare_detail_str(second_trace, "route"),
            Some("head_hit" | "exact_hit")
        ),
        "second perf trace must stay on canonical head/exact route after upgrade, trace={second_trace:?}"
    );

    drain_task.abort();
}

#[tokio::test]
async fn p33_get_current_context_uses_parse_snapshot_without_warming_exact_type_index() {
    let fixture = concat!(
        "Процедура Тест(ПервыйПараметр, ВторойПараметр)\n",
        "    Если Истина Тогда\n",
        "        Сообщить(ПервыйПараметр);\n",
        "    КонецЕсли;\n",
        "КонецПроцедуры\n",
    );

    let (mut service, drain_task, server, uri, file_id) =
        open_lsp_fixture_with_snapshot(fixture, "file:///current_context_fixture.bsl").await;
    force_current_revision_without_exact_type_index(&server, file_id, &uri, fixture, 2).await;

    let execute = Request::build("workspace/executeCommand")
        .id(13301)
        .params(serde_json::json!({
            "command": "bsl.getCurrentContext",
            "arguments": [{
                "uri": uri.to_string(),
                "line": 2,
                "character": 18,
            }],
        }))
        .finish();
    let execute_response = tokio::time::timeout(Duration::from_secs(2), async {
        service
            .ready()
            .await
            .unwrap()
            .call(execute)
            .await
            .expect("workspace/executeCommand request")
    })
    .await
    .expect("bsl.getCurrentContext timeout")
    .expect("workspace/executeCommand response");

    let value = serde_json::to_value(&execute_response).expect("serialize response");
    let result = value.get("result").cloned().expect("result field");
    assert_eq!(
        result.get("functionName").and_then(|value| value.as_str()),
        Some("Тест"),
        "current context must resolve enclosing procedure name from parse snapshot"
    );
    assert_eq!(
        result.get("functionKind").and_then(|value| value.as_str()),
        Some("procedure"),
        "current context must resolve enclosing routine kind from parse snapshot"
    );
    let params = result
        .get("params")
        .and_then(|value| value.as_array())
        .expect("current context params array");
    let params = params
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        params,
        vec!["ПервыйПараметр", "ВторойПараметр"],
        "current context must surface routine parameters without exact type index"
    );

    let exact_ready = server
        .analysis_v2
        .snapshot()
        .await
        .current_type_index_serve_only_ready(file_id)
        .expect("current_type_index_serve_only_ready after getCurrentContext");
    assert!(
        !exact_ready,
        "getCurrentContext must not eagerly warm exact type index on the request path"
    );

    drain_task.abort();
}

fn message_has_unknown_member(message: &str, member_name: &str) -> bool {
    let lower_message = message.to_lowercase();
    lower_message.contains(&member_name.to_lowercase())
        && (lower_message.contains("не существует") || lower_message.contains("не найден"))
}

fn message_has_unknown_key(message: &str) -> bool {
    let lower_message = message.to_lowercase();
    lower_message.contains("ключ") && lower_message.contains("не найден")
}

fn utf16_end_position(source: &str) -> Position {
    let mut line = 0u32;
    let mut last_line = "";
    for (idx, segment) in source.split('\n').enumerate() {
        line = idx as u32;
        last_line = segment;
    }
    let character = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    Position::new(line, character)
}

fn histogram_p95(metrics: &serde_json::Value, key: &str) -> f64 {
    metrics
        .get(key)
        .and_then(|value| value.get("p95"))
        .and_then(|value| value.as_f64().or_else(|| value.as_u64().map(|n| n as f64)))
        .unwrap_or(0.0)
}

fn dominant_stage_from_metrics(metrics: &serde_json::Value) -> serde_json::Value {
    let stage_keys = [
        (
            "wait_for_file_version_completion",
            "intellisense_v2_wait_for_file_version_completion_ms",
        ),
        (
            "snapshot_completion",
            "intellisense_v2_snapshot_completion_ms",
        ),
        (
            "ir_query_completion",
            "intellisense_v2_ir_query_completion_ms",
        ),
        ("parse_result_query", "intellisense_v2_parse_result_query_ms"),
        ("singleflight_wait", "intellisense_v2_singleflight_wait_ms"),
        (
            "runtime_exec_interactive",
            "intellisense_v2_runtime_exec_interactive_ms",
        ),
        (
            "runtime_wait_for_file_version_queue_wait",
            "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms",
        ),
        (
            "runtime_snapshot_with_deps_queue_wait",
            "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms",
        ),
        (
            "runtime_apply_changes_queue_wait",
            "intellisense_v2_runtime_apply_changes_queue_wait_ms",
        ),
        (
            "runtime_apply_changes_exec",
            "intellisense_v2_runtime_apply_changes_exec_ms",
        ),
        (
            "runtime_apply_change_set_file_exec",
            "intellisense_v2_runtime_apply_change_set_file_exec_ms",
        ),
        (
            "runtime_apply_change_set_file_with_snapshot_exec",
            "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_ms",
        ),
        (
            "runtime_apply_change_remove_file_exec",
            "intellisense_v2_runtime_apply_change_remove_file_exec_ms",
        ),
        (
            "runtime_apply_change_set_settings_snapshot_exec",
            "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_ms",
        ),
        (
            "runtime_type_index_precompute_queue_wait",
            "intellisense_v2_runtime_type_index_precompute_queue_wait_ms",
        ),
        (
            "runtime_type_index_precompute_exec",
            "intellisense_v2_runtime_type_index_precompute_exec_ms",
        ),
        (
            "runtime_type_index_precompute_build_exec",
            "intellisense_v2_runtime_type_index_precompute_build_exec_ms",
        ),
        (
            "runtime_type_index_precompute_ir_exec",
            "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
        ),
        (
            "runtime_type_index_precompute_ast_to_ir_exec",
            "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms",
        ),
        (
            "runtime_type_index_precompute_semantic_facts_exec",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms",
        ),
        (
            "runtime_type_index_precompute_semantic_facts_seed_module_context_exec",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms",
        ),
        (
            "runtime_type_index_precompute_semantic_facts_local_function_summaries_exec",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
        ),
        (
            "runtime_type_index_precompute_semantic_facts_visit_statements_exec",
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms",
        ),
        ("completion_stage_turn_wait", "completion_stage_turn_wait_ms"),
        (
            "completion_stage_prepare_stateful",
            "completion_stage_prepare_stateful_ms",
        ),
        (
            "completion_stage_prepare_apply_age_at_start",
            "completion_stage_prepare_apply_age_at_start_ms",
        ),
        (
            "completion_stage_prepare_apply_age_at_terminal",
            "completion_stage_prepare_apply_age_at_terminal_ms",
        ),
        (
            "completion_stage_sync_globals",
            "completion_stage_sync_globals_ms",
        ),
        (
            "completion_stage_exact_wait_apply_age_at_start",
            "completion_stage_exact_wait_apply_age_at_start_ms",
        ),
        (
            "completion_stage_exact_wait_apply_age_at_terminal",
            "completion_stage_exact_wait_apply_age_at_terminal_ms",
        ),
        (
            "completion_stage_query_bundle",
            "completion_stage_query_bundle_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint",
            "completion_stage_query_bundle_owner_hint_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_extract",
            "completion_stage_query_bundle_owner_hint_extract_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_offset",
            "completion_stage_query_bundle_owner_hint_offset_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_flow_lookup",
            "completion_stage_query_bundle_owner_hint_flow_lookup_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_direct",
            "completion_stage_query_bundle_owner_hint_type_lookup_direct_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_fallback",
            "completion_stage_query_bundle_owner_hint_type_lookup_fallback_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_unattributed",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_unattributed_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_total",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_total_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_inputs",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_inputs_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_parse_result_query",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_parse_result_query_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_build",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_build_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_parse_result",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_parse_result_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_total",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_total_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_seed_context",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_seed_context_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_local_function_summaries",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_local_function_summaries_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_visit_statements",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_visit_statements_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup_index_scan",
            "completion_stage_query_bundle_owner_hint_type_lookup_index_scan_ms",
        ),
        (
            "completion_stage_query_bundle_owner_hint_type_lookup",
            "completion_stage_query_bundle_owner_hint_type_lookup_ms",
        ),
        (
            "completion_stage_query_bundle_deps_and_file_snapshot",
            "completion_stage_query_bundle_deps_and_file_snapshot_ms",
        ),
        (
            "completion_stage_response_build",
            "completion_stage_response_build_ms",
        ),
        ("completion_stage_cache_store", "completion_stage_cache_store_ms"),
        (
            "completion_stage_snapshot_read",
            "completion_stage_snapshot_read_ms",
        ),
        ("completion_stage_collect", "completion_stage_collect_ms"),
        ("completion_stage_rank", "completion_stage_rank_ms"),
        ("completion_stage_format", "completion_stage_format_ms"),
        (
            "runtime_queue_wait_interactive",
            "intellisense_v2_runtime_queue_wait_interactive_ms",
        ),
        (
            "syntax_diagnostics_query",
            "intellisense_v2_syntax_diagnostics_query_ms",
        ),
        (
            "semantic_diagnostics_query",
            "intellisense_v2_semantic_diagnostics_query_ms",
        ),
        (
            "semantic_diagnostics_query_inputs",
            "intellisense_v2_semantic_diagnostics_query_inputs_ms",
        ),
        (
            "semantic_diagnostics_query_parse_result",
            "intellisense_v2_semantic_diagnostics_query_parse_result_ms",
        ),
        (
            "semantic_diagnostics_query_ir",
            "intellisense_v2_semantic_diagnostics_query_ir_ms",
        ),
        (
            "semantic_diagnostics_query_collect",
            "intellisense_v2_semantic_diagnostics_query_collect_ms",
        ),
        (
            "semantic_diagnostics_query_flow_sensitive",
            "intellisense_v2_semantic_diagnostics_query_flow_sensitive_ms",
        ),
    ];

    let mut candidates = serde_json::Map::new();
    let mut dominant: Option<(&'static str, f64)> = None;
    for (name, key) in stage_keys {
        let p95 = histogram_p95(metrics, key);
        candidates.insert(name.to_string(), serde_json::json!(p95));
        if p95 > 0.0 && dominant.is_none_or(|(_, value)| p95 > value) {
            dominant = Some((name, p95));
        }
    }

    let (stage, p95_ms) = dominant.unwrap_or(("none", 0.0));
    serde_json::json!({
        "stage": stage,
        "p95_ms": p95_ms,
        "candidates_p95_ms": candidates
    })
}

#[allow(clippy::too_many_arguments)]
async fn run_scale_aware_profile(
    profile_name: &str,
    uri: Url,
    text: String,
    position: Position,
    phases: &[ScaleAwarePhase],
    churn_mode: ScaleAwareChurnMode,
    churn_every: u64,
    workspace_setup: Option<&ScaleAwareWorkspaceSetup>,
    observability_probe: Option<ScaleAwareObservabilityProbe>,
) -> serde_json::Value {
    let mut profile_report = serde_json::Map::new();
    let progress_enabled = scale_aware_progress_enabled();
    let progress_every = scale_aware_progress_every();

    for (phase_index, phase) in phases.iter().enumerate() {
        let phase_started = Instant::now();
        let mut progress_line_width = 0usize;
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
        let mut drain_task =
            tokio::spawn(async move { while let Some(_req) = socket.next().await {} });

        initialize_lsp_service(&mut service).await;

        let server = server_holder
            .lock()
            .expect("server holder lock")
            .clone()
            .expect("server must be created");
        if let Some(setup) = workspace_setup {
            prime_server_with_workspace_setup(
                &server,
                setup,
                "p31_scale_aware_real_workspace_setup",
            )
            .await;
        }

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "bsl".to_string(),
                version: 1,
                text: text.clone(),
            },
        };
        let did_open_req = Request::build("textDocument/didOpen")
            .params(serde_json::to_value(did_open).expect("DidOpenTextDocumentParams"))
            .finish();
        let did_open_response = service
            .ready()
            .await
            .unwrap()
            .call(did_open_req)
            .await
            .expect("didOpen notification");
        assert!(did_open_response.is_none(), "didOpen is a notification");

        let mut current_text = text.clone();
        let mut current_version: i32 = 1;
        let mut churn_edits_applied = 0u64;
        let mut observability_ok_latencies_ms = Vec::new();
        let mut observability_timeouts_total = 0u64;
        let mut observability_errors_total = 0u64;

        let total_requests = phase.warmup + phase.iterations;
        if progress_enabled {
            emit_scale_aware_progress_line(
                &format!(
                    "[p31] profile={} phase={} progress=0/{} (0.0%) elapsed_ms=0 eta_ms=0 churn_edits=0 warmup={} iterations={} churn_mode={} churn_every={} progress_every={}",
                    profile_name,
                    phase.name,
                    total_requests,
                    phase.warmup,
                    phase.iterations,
                    churn_mode.as_str(),
                    churn_every,
                    progress_every
                ),
                &mut progress_line_width,
            );
        }
        for request_index in 0..total_requests {
            if should_apply_scale_aware_churn(
                churn_mode,
                profile_name,
                *phase,
                request_index,
                churn_every,
            ) {
                let end_position = utf16_end_position(&current_text);
                let churn_payload = if churn_edits_applied.is_multiple_of(2) {
                    " "
                } else {
                    "\n"
                };
                let next_version = current_version
                    .checked_add(1)
                    .expect("scale-aware churn version overflow");
                let did_change = DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier {
                        uri: uri.clone(),
                        version: next_version,
                    },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: Some(Range {
                            start: end_position,
                            end: end_position,
                        }),
                        range_length: None,
                        text: churn_payload.to_string(),
                    }],
                };
                let did_change_req = Request::build("textDocument/didChange")
                    .params(
                        serde_json::to_value(did_change)
                            .expect("scale-aware churn didChange params"),
                    )
                    .finish();
                let did_change_response = service
                    .ready()
                    .await
                    .unwrap()
                    .call(did_change_req)
                    .await
                    .expect("scale-aware churn didChange notification");
                assert!(did_change_response.is_none(), "didChange is a notification");
                current_version = next_version;
                current_text.push_str(churn_payload);
                churn_edits_applied += 1;
            }

            let completion = server
                .completion(CompletionParams {
                    text_document_position: TextDocumentPositionParams {
                        text_document: TextDocumentIdentifier { uri: uri.clone() },
                        position,
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: Some(CompletionContext {
                        trigger_kind: CompletionTriggerKind::INVOKED,
                        trigger_character: None,
                    }),
                })
                .await
                .expect("completion request");
            assert!(
                completion.is_some(),
                "completion response expected for profile={profile_name}, phase={}",
                phase.name
            );

            if progress_enabled
                && should_emit_scale_aware_progress(request_index, total_requests, progress_every)
            {
                let completed = request_index + 1;
                let elapsed = phase_started.elapsed();
                let progress_percent = scale_aware_progress_percent(completed, total_requests);
                let eta_ms = scale_aware_progress_eta_ms(elapsed, completed, total_requests);
                emit_scale_aware_progress_line(
                    &format!(
                        "[p31] profile={} phase={} progress={}/{} ({:.1}%) elapsed_ms={} eta_ms={} churn_edits={}",
                        profile_name,
                        phase.name,
                        completed,
                        total_requests,
                        progress_percent,
                        elapsed.as_millis(),
                        eta_ms,
                        churn_edits_applied
                    ),
                    &mut progress_line_width,
                );
            }

            if let Some(probe) = observability_probe {
                if should_probe_scale_aware_observability(*phase, request_index, probe.every) {
                    let request_id = 31_000_000_i64
                        .saturating_add((phase_index as i64) * 100_000)
                        .saturating_add(request_index as i64);
                    let (outcome, latency_ms) = probe_observability_sidebar_latency(
                        &mut service,
                        request_id,
                        probe.timeout,
                    )
                    .await;
                    match outcome {
                        ScaleAwareObservabilityProbeOutcome::Ok => {
                            if let Some(latency_ms) = latency_ms {
                                observability_ok_latencies_ms.push(latency_ms);
                            }
                        }
                        ScaleAwareObservabilityProbeOutcome::TimedOut => {
                            observability_timeouts_total += 1;
                        }
                        ScaleAwareObservabilityProbeOutcome::Error => {
                            observability_errors_total += 1;
                        }
                    }
                }
            }
        }

        let metrics = coordinator.observability_metrics();
        let counters = metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let histograms = metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("metrics.histograms object");
        let gauges = metrics
            .get("gauges")
            .and_then(|value| value.as_object())
            .expect("metrics.gauges object");

        let completion_total = read_u64_metric(counters.get("completion_total"));
        let completion_cancelled_total =
            read_u64_metric(counters.get("intellisense_v2_completion_result_total_cancelled"));
        let completion_cancelled_rate =
            completion_cancelled_total as f64 / completion_total.max(1) as f64;
        let completion_ok_non_empty_total =
            read_u64_metric(counters.get("intellisense_v2_completion_result_total_ok_non_empty"));
        let completion_ok_empty_total =
            read_u64_metric(counters.get("intellisense_v2_completion_result_total_ok_empty"));
        let completion_fail_closed_total =
            read_u64_metric(counters.get("intellisense_v2_completion_result_total_fail_closed"));

        let mut phase_metrics = serde_json::json!({
            "completion_duration_ms": histogram_metric_value(histograms, "completion_duration_ms", None),
            "intellisense_v2_wait_for_file_version_completion_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_wait_for_file_version_completion_ms",
                Some("intellisense_v2_wait_for_file_version_other_ms")
            ),
            "intellisense_v2_snapshot_completion_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_snapshot_completion_ms",
                Some("intellisense_v2_snapshot_other_ms")
            ),
            "intellisense_v2_ir_query_completion_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_ir_query_completion_ms",
                Some("intellisense_v2_ir_query_other_ms")
            ),
            "intellisense_v2_parse_result_query_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_parse_result_query_ms",
                None
            ),
            "intellisense_v2_singleflight_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_singleflight_wait_ms",
                None
            ),
            "intellisense_v2_runtime_exec_interactive_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_exec_interactive_ms",
                None
            ),
            "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_wait_for_file_version_queue_wait_ms",
                None
            ),
            "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_snapshot_with_deps_queue_wait_ms",
                None
            ),
            "intellisense_v2_runtime_apply_changes_queue_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_changes_queue_wait_ms",
                None
            ),
            "intellisense_v2_runtime_apply_changes_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_changes_exec_ms",
                None
            ),
            "intellisense_v2_runtime_apply_change_set_file_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_change_set_file_exec_ms",
                None
            ),
            "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_change_set_file_with_snapshot_exec_ms",
                None
            ),
            "intellisense_v2_runtime_apply_change_remove_file_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_change_remove_file_exec_ms",
                None
            ),
            "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_change_set_settings_snapshot_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_queue_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_queue_wait_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_build_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_build_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_ir_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
                None
            ),
            "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms",
                None
            ),
            "intellisense_v2_runtime_apply_changes_batch_size": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_changes_batch_size",
                None
            ),
            "intellisense_v2_runtime_apply_changes_changed_files_count": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_runtime_apply_changes_changed_files_count",
                None
            ),
            "completion_stage_turn_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_turn_wait_ms",
                None
            ),
            "completion_stage_prepare_stateful_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_prepare_stateful_ms",
                None
            ),
            "completion_stage_prepare_apply_age_at_start_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_prepare_apply_age_at_start_ms",
                None
            ),
            "completion_stage_prepare_apply_age_at_terminal_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_prepare_apply_age_at_terminal_ms",
                None
            ),
            "completion_stage_sync_globals_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_sync_globals_ms",
                None
            ),
            "completion_stage_exact_wait_apply_age_at_start_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_exact_wait_apply_age_at_start_ms",
                None
            ),
            "completion_stage_exact_wait_apply_age_at_terminal_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_exact_wait_apply_age_at_terminal_ms",
                None
            ),
            "completion_stage_query_bundle_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_extract_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_extract_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_offset_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_offset_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_flow_lookup_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_flow_lookup_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_direct_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_direct_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_fallback_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_fallback_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_unattributed_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_unattributed_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_other_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_to_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_idle_before_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_age_at_query_start_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_first_will_execute_type_index_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_apply_to_fetch_end_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_total_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_total_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_inputs_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_inputs_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_parse_result_query_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_parse_result_query_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_query_build_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_query_build_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_parse_result_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_parse_result_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_total_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_total_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_seed_context_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_seed_context_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_local_function_summaries_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_local_function_summaries_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_build_visit_statements_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_build_visit_statements_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_index_scan_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_index_scan_ms",
                None
            ),
            "completion_stage_query_bundle_owner_hint_type_lookup_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_owner_hint_type_lookup_ms",
                None
            ),
            "completion_stage_query_bundle_deps_and_file_snapshot_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_query_bundle_deps_and_file_snapshot_ms",
                None
            ),
            "completion_stage_response_build_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_response_build_ms",
                None
            ),
            "completion_stage_cache_store_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_cache_store_ms",
                None
            ),
            "completion_stage_snapshot_read_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_snapshot_read_ms",
                None
            ),
            "completion_stage_collect_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_collect_ms",
                None
            ),
            "completion_stage_rank_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_rank_ms",
                None
            ),
            "completion_stage_format_ms": histogram_metric_value_or_zero(
                histograms,
                "completion_stage_format_ms",
                None
            ),
            "intellisense_v2_completion_owner_hint_line_len_chars": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_line_len_chars",
                None
            ),
            "intellisense_v2_completion_owner_hint_receiver_len_chars": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_receiver_len_chars",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_active": read_numeric_metric(
                gauges.get("intellisense_v2_completion_owner_hint_index_fetch_active")
            ),
            "intellisense_v2_runtime_queue_wait_interactive_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_runtime_queue_wait_interactive_ms",
                None
            ),
            "intellisense_v2_syntax_diagnostics_query_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_syntax_diagnostics_query_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_ms": histogram_metric_value(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_inputs_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_inputs_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_parse_result_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_parse_result_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_ir_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_ir_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_collect_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_collect_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query_flow_sensitive_ms": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_flow_sensitive_ms",
                None
            ),
            "intellisense_v2_interactive_wait_budget_exhausted_total": read_u64_metric(
                counters.get("intellisense_v2_interactive_wait_budget_exhausted_total")
            ),
            "intellisense_v2_interactive_stale_served_total": read_u64_metric(
                counters.get("intellisense_v2_interactive_stale_served_total")
            ),
            "intellisense_v2_completion_stale_fallback_total": read_u64_metric(
                counters.get("intellisense_v2_completion_stale_fallback_total")
            ),
            "intellisense_v2_completion_fallback_unavailable_total": read_u64_metric(
                counters.get("intellisense_v2_completion_fallback_unavailable_total")
            ),
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready": read_u64_metric(
                counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready")
            ),
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline": read_u64_metric(
                counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline")
            ),
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task": read_u64_metric(
                counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task")
            ),
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_task_present_wrong_version": read_u64_metric(
                counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_task_present_wrong_version")
            ),
            "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_observed_version_mismatch": read_u64_metric(
                counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_observed_version_mismatch")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_block_on_other_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_other_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_other_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_other_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_type_index_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_type_index_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_parse_result_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_parse_result_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_other_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_other_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_salsa_will_check_cancellation_total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_check_cancellation_total")
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_check_cancellation_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_execute_other_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_iterate_cycle_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_did_set_cancellation_flag_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_global_did_set_cancellation_flag_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_did_discard_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_did_discard_accumulated_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_events_before_first_will_execute_type_index_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_check_before_first_will_execute_type_index_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_will_execute_parse_result_before_first_will_execute_type_index_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_first_will_execute_type_index_seen_per_fetch",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_revision_start": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_revision_start",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_revision_end": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_revision_end",
                None
            ),
            "intellisense_v2_completion_owner_hint_index_fetch_revision_delta": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_completion_owner_hint_index_fetch_revision_delta",
                None
            ),
        });
        phase_metrics["intellisense_v2_completion_owner_hint_result_total"] = serde_json::json!({
            "not_member_access": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_not_member_access")
            ),
            "no_file_content": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_no_file_content")
            ),
            "no_line": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_no_line")
            ),
            "no_dot": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_no_dot")
            ),
            "no_receiver": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_no_receiver")
            ),
            "offset_unresolved": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_offset_unresolved")
            ),
            "flow_type_hit": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_flow_type_hit")
            ),
            "flow_type_miss": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_flow_type_miss")
            ),
            "type_hit": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_type_hit")
            ),
            "type_miss": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_type_miss")
            ),
            "cancelled": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_cancelled")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_result_total_reason_other")
            )
        });
        phase_metrics["intellisense_v2_completion_owner_hint_lookup_path_total"] = serde_json::json!({
            "direct": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_direct")
            ),
            "flow_only": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_flow_only")
            ),
            "flow_plus_fallback": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_flow_plus_fallback")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_path_total_other")
            )
        });
        phase_metrics["intellisense_v2_completion_owner_hint_lookup_result_total"] = serde_json::json!({
            "hit": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_result_total_hit")
            ),
            "miss": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_result_total_miss")
            ),
            "cancelled": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_result_total_cancelled")
            ),
            "error": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_result_total_error")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_lookup_result_total_other")
            )
        });
        phase_metrics["intellisense_v2_completion_owner_hint_index_fetch_block_on_total_by_kind"] = serde_json::json!({
            "total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_total")
            ),
            "type_index": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_type_index_total")
            ),
            "parse_result": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_parse_result_total")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_block_on_other_total")
            )
        });
        phase_metrics
            ["intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total_by_kind"] = serde_json::json!({
            "total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_total")
            ),
            "type_index": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_type_index_total")
            ),
            "parse_result": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_parse_result_total")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_will_execute_other_total")
            )
        });
        phase_metrics
            ["intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total_by_kind"] = serde_json::json!({
            "total": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_total")
            ),
            "type_index": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_type_index_total")
            ),
            "parse_result": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_parse_result_total")
            ),
            "other": read_u64_metric(
                counters.get("intellisense_v2_completion_owner_hint_index_fetch_salsa_did_validate_memoized_other_total")
            )
        });
        let dominant_stage = dominant_stage_from_metrics(&phase_metrics);
        let phase_report = serde_json::json!({
            "warmup": phase.warmup,
            "iterations": phase.iterations,
            "profile_size": profile_name,
            "churn_mode": churn_mode.as_str(),
            "completion_total": completion_total,
            "completion_outcomes": {
                "ok_non_empty": completion_ok_non_empty_total,
                "ok_empty": completion_ok_empty_total,
                "fail_closed": completion_fail_closed_total,
                "cancelled": completion_cancelled_total,
            },
            "completion_cancelled_total": completion_cancelled_total,
            "completion_cancelled_rate": completion_cancelled_rate,
            "observability_sidebar_probe": {
                "enabled": observability_probe.is_some(),
                "every": observability_probe.map(|probe| probe.every).unwrap_or(0),
                "timeout_ms": observability_probe
                    .map(|probe| probe.timeout.as_millis().min(u64::MAX as u128) as u64)
                    .unwrap_or(0),
                "ok_total": observability_ok_latencies_ms.len(),
                "timeout_total": observability_timeouts_total,
                "error_total": observability_errors_total,
                "request_ms": sample_histogram_value(&observability_ok_latencies_ms),
            },
            "churn_edits_applied": churn_edits_applied,
            "metrics": phase_metrics,
            "dominant_stage": dominant_stage
        });
        if progress_enabled {
            emit_scale_aware_progress_line(
                &format!(
                    "[p31] profile={} phase={} done progress={}/{} (100.0%) elapsed_ms={} eta_ms=0 completion_total={} cancelled_total={} cancelled_rate={:.4} churn_edits={}",
                    profile_name,
                    phase.name,
                    total_requests,
                    total_requests,
                    phase_started.elapsed().as_millis(),
                    completion_total,
                    completion_cancelled_total,
                    completion_cancelled_rate,
                    churn_edits_applied
                ),
                &mut progress_line_width,
            );
            println!();
        }
        profile_report.insert(phase.name.to_string(), phase_report);

        shutdown_lsp_service(&mut service, Some(&uri)).await;
        drop(server);
        drop(service);

        if tokio::time::timeout(Duration::from_millis(500), &mut drain_task)
            .await
            .is_err()
        {
            drain_task.abort();
        }
    }

    serde_json::Value::Object(profile_report)
}

#[test]
fn scale_aware_progress_emits_start_step_and_finish() {
    assert!(should_emit_scale_aware_progress(0, 55, 10));
    assert!(should_emit_scale_aware_progress(9, 55, 10));
    assert!(should_emit_scale_aware_progress(54, 55, 10));
}

#[test]
fn scale_aware_phase_plan_defaults_match_acceptance_contract() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn unset(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, previous }
        }

        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _guards = [
        EnvVarGuard::unset("BSL_V2_SCALE_AWARE_START_ITERATIONS"),
        EnvVarGuard::unset("BSL_V2_SCALE_AWARE_COLD_ITERATIONS"),
        EnvVarGuard::unset("BSL_V2_SCALE_AWARE_WARM_WARMUP"),
        EnvVarGuard::unset("BSL_V2_SCALE_AWARE_WARM_ITERATIONS"),
        EnvVarGuard::unset("BSL_V2_SCALE_AWARE_REQUIRED_WARM_SAMPLES"),
    ];

    let phases = scale_aware_phase_plan_from_env();
    assert_eq!(phases[0].name, "start");
    assert_eq!(phases[0].warmup, 0);
    assert_eq!(phases[0].iterations, 1);
    assert_eq!(phases[1].name, "cold");
    assert_eq!(phases[1].warmup, 0);
    assert_eq!(phases[1].iterations, 5);
    assert_eq!(phases[2].name, "warm");
    assert_eq!(phases[2].warmup, 5);
    assert_eq!(phases[2].iterations, 50);
    assert_eq!(scale_aware_required_warm_samples_from_env(), 50);

    let _override_guard = EnvVarGuard::set("BSL_V2_SCALE_AWARE_START_ITERATIONS", "7");
    assert_eq!(scale_aware_phase_plan_from_env()[0].iterations, 7);
}

#[test]
fn scale_aware_phase_plan_accepts_local_debug_overrides() {
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _guards = [
        EnvVarGuard::set("BSL_V2_SCALE_AWARE_START_ITERATIONS", "2"),
        EnvVarGuard::set("BSL_V2_SCALE_AWARE_COLD_ITERATIONS", "1"),
        EnvVarGuard::set("BSL_V2_SCALE_AWARE_WARM_WARMUP", "1"),
        EnvVarGuard::set("BSL_V2_SCALE_AWARE_WARM_ITERATIONS", "4"),
        EnvVarGuard::set("BSL_V2_SCALE_AWARE_REQUIRED_WARM_SAMPLES", "4"),
    ];

    let phases = scale_aware_phase_plan_from_env();
    assert_eq!(phases[0].iterations, 2);
    assert_eq!(phases[1].iterations, 1);
    assert_eq!(phases[2].warmup, 1);
    assert_eq!(phases[2].iterations, 4);
    assert_eq!(scale_aware_required_warm_samples_from_env(), 4);
}

#[test]
fn scale_aware_progress_skips_intermediate_non_step_points() {
    assert!(!should_emit_scale_aware_progress(1, 55, 10));
    assert!(!should_emit_scale_aware_progress(8, 55, 10));
    assert!(!should_emit_scale_aware_progress(53, 55, 10));
    assert!(!should_emit_scale_aware_progress(0, 0, 10));
}

#[test]
fn scale_aware_progress_percent_and_eta_are_stable() {
    let elapsed = Duration::from_millis(2_500);
    let completed = 5;
    let total = 10;
    let percent = scale_aware_progress_percent(completed, total);
    let eta_ms = scale_aware_progress_eta_ms(elapsed, completed, total);
    assert!((percent - 50.0).abs() < f64::EPSILON);
    assert_eq!(eta_ms, 2_500);
    assert_eq!(scale_aware_progress_eta_ms(elapsed, 0, total), 0);
    assert_eq!(scale_aware_progress_eta_ms(elapsed, total, total), 0);
}

#[test]
fn scale_aware_dominant_stage_includes_completion_pipeline_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_wait_for_file_version_completion_ms": {"p95": 2.0},
        "intellisense_v2_snapshot_completion_ms": {"p95": 1.0},
        "intellisense_v2_ir_query_completion_ms": {"p95": 3.0},
        "intellisense_v2_runtime_queue_wait_interactive_ms": {"p95": 2.0},
        "intellisense_v2_syntax_diagnostics_query_ms": {"p95": 0.0},
        "intellisense_v2_semantic_diagnostics_query_ms": {"p95": 10.0},
        "completion_stage_snapshot_read_ms": {"p95": 15.0},
        "completion_stage_collect_ms": {"p95": 120.0},
        "completion_stage_rank_ms": {"p95": 8.0},
        "completion_stage_format_ms": {"p95": 6.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_collect"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        120.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_completion_turn_wait_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_wait_for_file_version_completion_ms": {"p95": 2.0},
        "intellisense_v2_snapshot_completion_ms": {"p95": 1.0},
        "intellisense_v2_ir_query_completion_ms": {"p95": 3.0},
        "intellisense_v2_runtime_queue_wait_interactive_ms": {"p95": 2.0},
        "intellisense_v2_syntax_diagnostics_query_ms": {"p95": 0.0},
        "intellisense_v2_semantic_diagnostics_query_ms": {"p95": 280.0},
        "completion_stage_turn_wait_ms": {"p95": 1500.0},
        "completion_stage_prepare_stateful_ms": {"p95": 20.0},
        "completion_stage_sync_globals_ms": {"p95": 5.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_turn_wait"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        1500.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_completion_query_bundle_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_wait_for_file_version_completion_ms": {"p95": 2.0},
        "intellisense_v2_snapshot_completion_ms": {"p95": 1.0},
        "intellisense_v2_ir_query_completion_ms": {"p95": 9.0},
        "intellisense_v2_runtime_queue_wait_interactive_ms": {"p95": 2.0},
        "intellisense_v2_semantic_diagnostics_query_ms": {"p95": 320.0},
        "intellisense_v2_parse_result_query_ms": {"p95": 120.0},
        "intellisense_v2_singleflight_wait_ms": {"p95": 40.0},
        "intellisense_v2_runtime_exec_interactive_ms": {"p95": 25.0},
        "completion_stage_query_bundle_ms": {"p95": 2400.0},
        "completion_stage_response_build_ms": {"p95": 50.0},
        "completion_stage_cache_store_ms": {"p95": 30.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        2400.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_completion_query_bundle_owner_hint_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_wait_for_file_version_completion_ms": {"p95": 2.0},
        "intellisense_v2_snapshot_completion_ms": {"p95": 1.0},
        "intellisense_v2_ir_query_completion_ms": {"p95": 9.0},
        "intellisense_v2_runtime_queue_wait_interactive_ms": {"p95": 2.0},
        "intellisense_v2_semantic_diagnostics_query_ms": {"p95": 320.0},
        "completion_stage_query_bundle_ms": {"p95": 2400.0},
        "completion_stage_query_bundle_owner_hint_ms": {"p95": 3500.0},
        "completion_stage_query_bundle_deps_and_file_snapshot_ms": {"p95": 100.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3500.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_owner_hint_index_fetch_wait_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_wait_for_file_version_completion_ms": {"p95": 2.0},
        "intellisense_v2_snapshot_completion_ms": {"p95": 1.0},
        "intellisense_v2_ir_query_completion_ms": {"p95": 9.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms": {"p95": 2800.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_ms": {"p95": 2700.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_build_total_ms": {"p95": 100.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        2800.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_owner_hint_index_fetch_inside_salsa_window_breakdown() {
    let metrics = serde_json::json!({
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms": {"p95": 2000.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window_ms": {"p95": 3100.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait_ms": {"p95": 100.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail_ms": {"p95": 50.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3100.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_owner_hint_first_will_check_to_first_will_execute_breakdown()
{
    let metrics = serde_json::json!({
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms": {"p95": 2000.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation_ms": {"p95": 1200.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms": {"p95": 1500.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index_ms": {"p95": 3300.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_to_first_will_execute_type_index"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3300.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_owner_hint_first_will_execute_other_breakdown() {
    let metrics = serde_json::json!({
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_wait_ms": {"p95": 2000.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms": {"p95": 1500.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result_ms": {"p95": 1700.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other_ms": {"p95": 3400.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_other"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3400.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_owner_hint_will_iterate_cycle_breakdown() {
    let metrics = serde_json::json!({
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index_ms": {"p95": 1500.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle_ms": {"p95": 3600.0},
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_last_will_iterate_cycle_ms": {"p95": 3400.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "completion_stage_query_bundle_owner_hint_type_lookup_index_fetch_first_will_iterate_cycle"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3600.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_runtime_apply_changes_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_runtime_apply_changes_queue_wait_ms": {"p95": 3500.0},
        "intellisense_v2_runtime_apply_changes_exec_ms": {"p95": 3200.0},
        "intellisense_v2_runtime_apply_change_set_file_exec_ms": {"p95": 2800.0},
        "completion_stage_query_bundle_ms": {"p95": 1200.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "runtime_apply_changes_queue_wait"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        3500.0
    );
}

#[test]
fn scale_aware_dominant_stage_includes_type_index_precompute_breakdown() {
    let metrics = serde_json::json!({
        "intellisense_v2_runtime_type_index_precompute_queue_wait_ms": {"p95": 120.0},
        "intellisense_v2_runtime_type_index_precompute_exec_ms": {"p95": 4800.0},
        "intellisense_v2_runtime_type_index_precompute_build_exec_ms": {"p95": 14.0},
        "intellisense_v2_runtime_type_index_precompute_ir_exec_ms": {"p95": 4700.0},
        "intellisense_v2_runtime_type_index_precompute_ast_to_ir_exec_ms": {"p95": 1900.0},
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_exec_ms": {"p95": 2600.0},
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_seed_module_context_exec_ms": {"p95": 120.0},
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms": {"p95": 4100.0},
        "intellisense_v2_runtime_type_index_precompute_semantic_facts_visit_statements_exec_ms": {"p95": 320.0},
        "completion_stage_prepare_stateful_ms": {"p95": 121.0}
    });

    let dominant = dominant_stage_from_metrics(&metrics);
    assert_eq!(
        dominant
            .get("stage")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        "runtime_type_index_precompute_exec"
    );
    assert_eq!(
        dominant
            .get("p95_ms")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        4800.0
    );
    assert_eq!(
        dominant
            .get("candidates_p95_ms")
            .and_then(|value| value
                .get("runtime_type_index_precompute_semantic_facts_local_function_summaries_exec"))
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        4100.0
    );
}

fn synthetic_scale_aware_profile(
    completion_p95: f64,
    wait_p95: f64,
    completion_total: u64,
    completion_cancelled_total: u64,
) -> serde_json::Value {
    let phase = |completion_count: u64, completion_p95_value: f64, wait_p95_value: f64| {
        serde_json::json!({
            "completion_total": completion_count,
            "completion_cancelled_total": 0,
            "metrics": {
                "completion_duration_ms": {
                    "count": completion_count,
                    "p50": completion_p95_value,
                    "p95": completion_p95_value,
                    "p99": completion_p95_value
                },
                "intellisense_v2_wait_for_file_version_completion_ms": {
                    "count": completion_count,
                    "p50": wait_p95_value,
                    "p95": wait_p95_value,
                    "p99": wait_p95_value
                },
                "intellisense_v2_snapshot_completion_ms": {
                    "count": completion_count,
                    "p50": 0.0,
                    "p95": 0.0,
                    "p99": 0.0
                },
                "intellisense_v2_ir_query_completion_ms": {
                    "count": completion_count,
                    "p50": 0.0,
                    "p95": 0.0,
                    "p99": 0.0
                },
                "intellisense_v2_completion_stale_fallback_total": 0,
                "intellisense_v2_interactive_wait_budget_exhausted_total": 0,
                "intellisense_v2_completion_fallback_unavailable_total": 0,
                "intellisense_v2_interactive_stale_served_total": 0
            }
        })
    };
    serde_json::json!({
        "start": phase(1, completion_p95, wait_p95),
        "cold": phase(5, completion_p95, wait_p95),
        "warm": {
            "completion_total": completion_total,
            "completion_cancelled_total": completion_cancelled_total,
            "metrics": {
                "completion_duration_ms": {
                    "count": completion_total,
                    "p50": completion_p95,
                    "p95": completion_p95,
                    "p99": completion_p95
                },
                "intellisense_v2_wait_for_file_version_completion_ms": {
                    "count": completion_total,
                    "p50": wait_p95,
                    "p95": wait_p95,
                    "p99": wait_p95
                },
                "intellisense_v2_snapshot_completion_ms": {
                    "count": completion_total,
                    "p50": 0.0,
                    "p95": 0.0,
                    "p99": 0.0
                },
                "intellisense_v2_ir_query_completion_ms": {
                    "count": completion_total,
                    "p50": 0.0,
                    "p95": 0.0,
                    "p99": 0.0
                },
                "intellisense_v2_completion_stale_fallback_total": 0,
                "intellisense_v2_interactive_wait_budget_exhausted_total": 0,
                "intellisense_v2_completion_fallback_unavailable_total": 0,
                "intellisense_v2_interactive_stale_served_total": 0
            }
        }
    })
}

fn synthetic_scale_aware_report(
    change_id: &str,
    large_completion_p95: f64,
    large_wait_p95: f64,
    small_completion_p95: f64,
    small_wait_p95: f64,
) -> serde_json::Value {
    serde_json::json!({
        "change_id": change_id,
        "profile": "p31_scale_aware_large_small_completion_gate_live",
        "schema_version": 1,
        "profiles": {
            "large": synthetic_scale_aware_profile(large_completion_p95, large_wait_p95, 60, 0),
            "small": synthetic_scale_aware_profile(small_completion_p95, small_wait_p95, 60, 0)
        }
    })
}

#[test]
fn scale_aware_baseline_schema_requires_explicit_pass_fail_summary() {
    let baseline = synthetic_scale_aware_report("baseline", 100.0, 100.0, 100.0, 0.0);
    let err = validate_scale_aware_baseline_schema_for_acceptance(&baseline)
        .expect_err("baseline without gate.pass must be rejected");
    assert!(
        err.contains("gate.pass"),
        "expected error mentioning gate.pass, got: {err}"
    );
}

#[test]
fn scale_aware_baseline_schema_accepts_required_shape() {
    let mut baseline = synthetic_scale_aware_report("baseline", 100.0, 100.0, 100.0, 0.0);
    baseline["gate"] = serde_json::json!({
        "pass": true
    });
    validate_scale_aware_baseline_schema_for_acceptance(&baseline)
        .expect("baseline with required gate summary and metrics should validate");
}

#[tokio::test]
async fn p31_scale_aware_large_small_completion_gate_live() {
    init_test_tracing();
    const CHANGE_ID: &str = "add-bounded-stale-completion-fastpath";
    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();

    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let Some(conf_big_root) = conf_big_root_for_tests() else {
        if allow_fixture_skip {
            eprintln!(
                "skipping p31 scale-aware gate: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
            );
            return;
        }
        panic!(
            "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly"
        );
    };

    let large_module_path = conf_big_large_module_path_for_tests(&conf_big_root);
    if !large_module_path.exists() {
        if allow_fixture_skip {
            eprintln!(
                "skipping p31 scale-aware gate: conf_big module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                large_module_path.display()
            );
            return;
        }
        panic!(
            "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly",
            large_module_path.display()
        );
    }

    let small_module_path = workspace_root.join("examples").join("test_lsp.bsl");
    assert!(
        small_module_path.exists(),
        "small module fixture not found: {}",
        small_module_path.display()
    );

    let large_text = std::fs::read_to_string(&large_module_path)
        .expect("read conf_big module text for p31 scale-aware gate");
    let small_text = std::fs::read_to_string(&small_module_path)
        .expect("read small module text for p31 scale-aware gate");

    let large_position = find_utf16_position_after_marker(&large_text, "Объект.");
    let small_position = find_utf16_position_after_marker(&small_text, "Arr.");
    let phases = scale_aware_phase_plan_from_env();
    let required_warm_samples = scale_aware_required_warm_samples_from_env();
    let churn_mode = scale_aware_churn_mode_from_env();
    let churn_every = scale_aware_churn_every_from_env();

    let large_profile = run_scale_aware_profile(
        "large",
        Url::parse("file:///p31_scale_large_module.bsl").expect("large uri"),
        large_text,
        large_position,
        &phases,
        churn_mode,
        churn_every,
        None,
        None,
    )
    .await;
    let small_profile = run_scale_aware_profile(
        "small",
        Url::parse("file:///p31_scale_small_module.bsl").expect("small uri"),
        small_text,
        small_position,
        &phases,
        churn_mode,
        churn_every,
        None,
        None,
    )
    .await;

    let mut report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": "p31_scale_aware_large_small_completion_gate_live",
        "schema_version": 1,
        "phases": phases.iter().map(|phase| {
            serde_json::json!({
                "name": phase.name,
                "warmup": phase.warmup,
                "iterations": phase.iterations
            })
        }).collect::<Vec<_>>(),
        "churn": {
            "mode": churn_mode.as_str(),
            "every": churn_every
        },
        "requirements": {
            "required_warm_samples": required_warm_samples
        },
        "profiles": {
            "large": large_profile,
            "small": small_profile
        }
    });

    let baseline_path = std::env::var("BSL_V2_SCALE_AWARE_GATE_BASELINE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("baselines")
                .join(format!("{CHANGE_ID}.json"))
        });
    let enforce_gate = std::env::var("BSL_V2_SCALE_AWARE_GATE_ENFORCE")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !baseline_path.exists() {
        panic!(
            "scale-aware baseline is required but missing: {}",
            baseline_path.display()
        );
    }

    let baseline_raw =
        std::fs::read_to_string(&baseline_path).expect("read scale-aware baseline file");
    let baseline_report: serde_json::Value =
        serde_json::from_str(&baseline_raw).expect("parse scale-aware baseline json");
    validate_scale_aware_baseline_schema_for_acceptance(&baseline_report)
        .expect("validate scale-aware baseline schema");
    let gate = evaluate_scale_aware_gate_for_acceptance(&report, &baseline_report)
        .expect("evaluate scale-aware large/small gate");
    report["baseline"] = serde_json::json!({
        "path": baseline_path,
        "present": true
    });
    report["gate"] = gate.clone();

    if enforce_gate {
        let pass = gate
            .get("pass")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        assert!(
            pass,
            "p31 scale-aware gate failed in enforce mode: {}",
            serde_json::to_string_pretty(&gate).unwrap_or_else(|_| "<gate json>".to_string())
        );
    }

    let report_path = std::env::var("BSL_V2_SCALE_AWARE_GATE_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!("{CHANGE_ID}-live.json"))
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for p31 scale-aware report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize p31 scale-aware report"),
    )
    .expect("write p31 scale-aware report");
    println!("p31_scale_aware_gate_report={}", report_path.display());

    let large_warm_total =
        get_report_u64(&report, &["profiles", "large", "warm", "completion_total"])
            .expect("large warm completion_total");
    let small_warm_total =
        get_report_u64(&report, &["profiles", "small", "warm", "completion_total"])
            .expect("small warm completion_total");
    assert!(
        large_warm_total >= required_warm_samples && small_warm_total >= required_warm_samples,
        "expected >={required_warm_samples} warm completion samples for both profiles, got large={} small={}",
        large_warm_total,
        small_warm_total
    );
}

#[tokio::test]
async fn p36_real_conf_big_completion_and_observability_gate_live() {
    init_test_tracing();
    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();

    let Some(conf_big_root) = conf_big_root_for_tests() else {
        if allow_fixture_skip {
            eprintln!(
                "skipping p36 real conf_big gate: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
            );
            return;
        }
        panic!(
            "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly"
        );
    };

    let module_path = conf_big_large_module_path_for_tests(&conf_big_root);
    if !module_path.exists() {
        if allow_fixture_skip {
            eprintln!(
                "skipping p36 real conf_big gate: module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                module_path.display()
            );
            return;
        }
        panic!(
            "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly",
            module_path.display()
        );
    }

    let module_text =
        std::fs::read_to_string(&module_path).expect("read conf_big module text for p36 gate");
    let phases = real_module_phase_plan_from_env();
    let required_warm_samples = real_module_required_warm_samples_from_env();
    let churn_mode = scale_aware_churn_mode_from_env();
    let churn_every = scale_aware_churn_every_from_env();
    let observability_probe = real_module_observability_probe_from_env();
    let workspace_setup = ScaleAwareWorkspaceSetup {
        platform_docs_archive: syntax_helper_path_for_tests(),
        configuration_path: conf_big_root.clone(),
        platform_version: "8.3.25".to_string(),
    };
    let profile = run_scale_aware_profile(
        "large",
        Url::from_file_path(&module_path).expect("real conf_big module uri"),
        module_text.clone(),
        find_utf16_position_after_marker(&module_text, "Объект."),
        &phases,
        churn_mode,
        churn_every,
        Some(&workspace_setup),
        Some(observability_probe),
    )
    .await;

    let mut report = serde_json::json!({
        "change_id": "refactor-ir-canonical-semantic-pipeline",
        "profile": "p36_real_conf_big_completion_and_observability_gate_live",
        "schema_version": 1,
        "configuration_path": conf_big_root,
        "module_path": module_path,
        "phases": phases.iter().map(|phase| {
            serde_json::json!({
                "name": phase.name,
                "warmup": phase.warmup,
                "iterations": phase.iterations
            })
        }).collect::<Vec<_>>(),
        "churn": {
            "mode": churn_mode.as_str(),
            "every": churn_every
        },
        "requirements": {
            "required_warm_samples": required_warm_samples
        },
        "observability_probe": {
            "every": observability_probe.every,
            "timeout_ms": observability_probe.timeout.as_millis(),
        },
        "profile_report": profile,
    });

    let report_path = std::env::var("BSL_V2_REAL_MODULE_GATE_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join("real-conf-big-completion-observability-live.json")
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for p36 real-module report");
    }

    let warm_completion_total =
        get_report_u64(&report, &["profile_report", "warm", "completion_total"])
            .expect("real-module warm completion_total");
    let warm_fail_closed_total = get_report_u64(
        &report,
        &[
            "profile_report",
            "warm",
            "completion_outcomes",
            "fail_closed",
        ],
    )
    .expect("real-module warm fail_closed");
    let warm_ok_non_empty_total = get_report_u64(
        &report,
        &[
            "profile_report",
            "warm",
            "completion_outcomes",
            "ok_non_empty",
        ],
    )
    .expect("real-module warm ok_non_empty");
    let warm_observability_timeout_total = get_report_u64(
        &report,
        &[
            "profile_report",
            "warm",
            "observability_sidebar_probe",
            "timeout_total",
        ],
    )
    .expect("real-module warm observability timeout_total");
    report["summary"] = serde_json::json!({
        "warm_completion_total": warm_completion_total,
        "warm_fail_closed_total": warm_fail_closed_total,
        "warm_ok_non_empty_total": warm_ok_non_empty_total,
        "warm_observability_timeout_total": warm_observability_timeout_total,
    });

    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize p36 real-module report"),
    )
    .expect("write p36 real-module report");
    println!("p36_real_module_gate_report={}", report_path.display());

    assert!(
        warm_completion_total >= required_warm_samples,
        "expected >={required_warm_samples} warm completion samples for real module, got {}",
        warm_completion_total
    );
}

#[tokio::test]
async fn p37_real_conf_big_warm_cache_completion_perf_report_live() {
    init_test_tracing();
    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
    const PROFILE_NAME: &str = "p37_real_conf_big_warm_cache_completion_perf_report_live";
    const WARMUP_REQUESTS: usize = 5;
    const MEASURE_REQUESTS: usize = 4;
    const WARM_HEAD_PATH_P95_BUDGET_MS: f64 = 150.0;

    let Some(conf_big_root) = conf_big_root_for_tests() else {
        if allow_fixture_skip {
            eprintln!(
                "skipping {PROFILE_NAME}: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
            );
            return;
        }
        panic!(
            "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly"
        );
    };

    let module_path = conf_big_large_module_path_for_tests(&conf_big_root);
    if !module_path.exists() {
        if allow_fixture_skip {
            eprintln!(
                "skipping {PROFILE_NAME}: module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                module_path.display()
            );
            return;
        }
        panic!(
            "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly",
            module_path.display()
        );
    }

    let module_text =
        std::fs::read_to_string(&module_path).expect("read conf_big module text for p37 report");
    let workspace_setup = ScaleAwareWorkspaceSetup {
        platform_docs_archive: syntax_helper_path_for_tests(),
        configuration_path: conf_big_root.clone(),
        platform_version: "8.3.25".to_string(),
    };
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
        .expect("server must be captured");
    prime_server_with_workspace_setup(&server, &workspace_setup, "p37_real_conf_big_live_setup")
        .await;

    let uri = Url::from_file_path(&module_path).expect("real conf_big module uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: module_text.clone(),
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
    let opened_version = server
        .latest_received_file_versions_v2
        .read()
        .await
        .get(&file_id)
        .copied()
        .expect("latest received version for p37 opened file");
    assert_eq!(
        opened_version, 1,
        "real conf_big fixture must open at version 1"
    );
    assert!(
        server
            .analysis_v2
            .wait_for_file_version(file_id, opened_version)
            .await,
        "analysis runtime must catch up to opened real conf_big file version"
    );
    wait_for_type_index_precompute_completion(&server, file_id).await;
    let exact_type_index_seed =
        seed_exact_type_index_for_current_file_version(&server, file_id).await;
    server.cancel_type_index_precompute_v2(file_id).await;

    let completion_position = find_utf16_position_after_marker(&module_text, "Объект.");
    let completion_context = Some(CompletionContext {
        trigger_kind: CompletionTriggerKind::INVOKED,
        trigger_character: None,
    });

    let mut warmup_samples = Vec::new();
    for index in 0..WARMUP_REQUESTS {
        let request_id = 37_100_000_i64 + index as i64;
        let started = Instant::now();
        let labels = lsp_completion_labels_with_request(
            &mut service,
            request_id,
            &uri,
            completion_position,
            completion_context.clone(),
        )
        .await;
        warmup_samples.push(serde_json::json!({
            "step": format!("warmup_completion_{}", index + 1),
            "request_id": request_id,
            "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            "label_count": labels.len(),
            "labels": labels,
            "version": opened_version,
        }));
    }

    let mut measured_samples = Vec::new();
    for index in 0..MEASURE_REQUESTS {
        let request_id = 37_100_100_i64 + index as i64;
        let started = Instant::now();
        let labels = lsp_completion_labels_with_request(
            &mut service,
            request_id,
            &uri,
            completion_position,
            completion_context.clone(),
        )
        .await;
        measured_samples.push(serde_json::json!({
            "step": format!("measured_warm_completion_{}", index + 1),
            "request_id": request_id,
            "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            "label_count": labels.len(),
            "labels": labels,
            "version": opened_version,
        }));
    }

    let completion_timeline = lsp_get_completion_timeline(&mut service, 37_100_900, 64).await;
    let observability_metrics = lsp_get_observability_metrics(&mut service, 37_100_901).await;
    let timeline_traces = completion_timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let filtered_traces: Vec<serde_json::Value> = timeline_traces
        .iter()
        .filter(|trace| trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str()))
        .cloned()
        .collect();
    assert!(
        !filtered_traces.is_empty(),
        "expected non-empty completion timeline traces for real conf_big module"
    );

    let histograms = observability_metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let counters = observability_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");

    let total_sample_count = WARMUP_REQUESTS + MEASURE_REQUESTS;
    let trace_request_id_present_total = filtered_traces
        .iter()
        .filter(|trace| {
            trace
                .get("request_id")
                .and_then(|value| value.as_str())
                .is_some()
        })
        .count();
    let trace_matching_mode = if trace_request_id_present_total > 0 {
        "request_id"
    } else {
        "ordinal_by_filtered_uri_trace_order"
    };
    let fallback_trace_window: Vec<serde_json::Value> =
        if filtered_traces.len() >= total_sample_count {
            filtered_traces[filtered_traces.len() - total_sample_count..].to_vec()
        } else {
            filtered_traces.clone()
        };

    let enrich_samples = |samples: Vec<serde_json::Value>,
                          sample_offset: usize|
     -> Vec<serde_json::Value> {
        samples
                .into_iter()
                .enumerate()
                .map(|(sample_index, sample)| {
                    let request_id_text = sample
                        .get("request_id")
                        .and_then(|value| value.as_i64())
                        .map(|value| value.to_string());
                    let trace = if trace_request_id_present_total > 0 {
                        request_id_text.as_ref().and_then(|request_id| {
                            filtered_traces.iter().find(|trace| {
                                trace.get("request_id").and_then(|value| value.as_str())
                                    == Some(request_id)
                            })
                        })
                    } else {
                        fallback_trace_window.get(sample_offset + sample_index)
                    };
                let trace_summary = trace.map(|trace| {
                    serde_json::json!({
                        "trace_id": trace.get("trace_id").and_then(|value| value.as_str()),
                        "request_id": trace.get("request_id").and_then(|value| value.as_str()),
                        "trigger_mode": trace.get("trigger_mode").and_then(|value| value.as_str()),
                        "outcome": trace.get("outcome").and_then(|value| value.as_str()),
                        "route": completion_timeline_prepare_detail_str(trace, "route"),
                        "fail_closed_cause": completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
                        "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                        "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                        "queue_outcome": trace.get("queue_outcome").and_then(|value| value.as_str()),
                        "turn_wait_outcome": trace.get("turn_wait_outcome").and_then(|value| value.as_str()),
                        "queued_completion_ahead": trace.get("queued_completion_ahead").and_then(|value| value.as_u64()),
                        "did_change_ahead": trace.get("did_change_ahead").and_then(|value| value.as_u64()),
                        "active_completion_count": trace.get("active_completion_count").and_then(|value| value.as_u64()),
                        "prepare_guard_outcome": completion_timeline_prepare_detail_str(trace, "guard_outcome"),
                        "prepare_outcome": completion_timeline_prepare_detail_str(trace, "outcome"),
                        "prepare_wait_elapsed_ms": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("wait_elapsed_ms"))
                            .and_then(|value| value.as_u64()),
                        "prepare_snapshot_elapsed_ms": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("snapshot_elapsed_ms"))
                            .and_then(|value| value.as_u64()),
                        "turn_wait_ms": completion_timeline_trace_stage_duration_ms(trace, "turn_wait"),
                        "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                        "wait_exact_type_index_ms": completion_timeline_trace_stage_duration_ms(trace, "wait_exact_type_index"),
                        "query_bundle_ms": completion_timeline_trace_stage_duration_ms(trace, "query_bundle"),
                        "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                        "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
                    })
                });
                let mut sample_object = sample
                    .as_object()
                    .cloned()
                    .expect("sample must be json object");
                sample_object.insert(
                    "trace".to_string(),
                    trace_summary.unwrap_or(serde_json::json!(null)),
                );
                serde_json::Value::Object(sample_object)
                })
                .collect::<Vec<_>>()
    };

    let warmup_samples = enrich_samples(warmup_samples, 0);
    let measured_samples = enrich_samples(measured_samples, WARMUP_REQUESTS);

    let latest_trace_summaries = filtered_traces
        .iter()
        .rev()
        .take(16)
        .map(|trace| {
            serde_json::json!({
                "trace_id": trace.get("trace_id").and_then(|value| value.as_str()),
                "request_id": trace.get("request_id").and_then(|value| value.as_str()),
                "trigger_mode": trace.get("trigger_mode").and_then(|value| value.as_str()),
                "outcome": trace.get("outcome").and_then(|value| value.as_str()),
                "route": completion_timeline_prepare_detail_str(trace, "route"),
                "fail_closed_cause": completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
                "started_at_ms": trace.get("started_at_ms").and_then(|value| value.as_u64()),
                "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                "queue_outcome": trace.get("queue_outcome").and_then(|value| value.as_str()),
                "turn_wait_outcome": trace.get("turn_wait_outcome").and_then(|value| value.as_str()),
                "queued_completion_ahead": trace.get("queued_completion_ahead").and_then(|value| value.as_u64()),
                "did_change_ahead": trace.get("did_change_ahead").and_then(|value| value.as_u64()),
                "active_completion_count": trace.get("active_completion_count").and_then(|value| value.as_u64()),
                "prepare_guard_outcome": completion_timeline_prepare_detail_str(trace, "guard_outcome"),
                "prepare_outcome": completion_timeline_prepare_detail_str(trace, "outcome"),
                "prepare_wait_elapsed_ms": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("wait_elapsed_ms"))
                    .and_then(|value| value.as_u64()),
                "prepare_snapshot_elapsed_ms": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("snapshot_elapsed_ms"))
                    .and_then(|value| value.as_u64()),
                "turn_wait_ms": completion_timeline_trace_stage_duration_ms(trace, "turn_wait"),
                "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                "wait_exact_type_index_ms": completion_timeline_trace_stage_duration_ms(trace, "wait_exact_type_index"),
                "query_bundle_ms": completion_timeline_trace_stage_duration_ms(trace, "query_bundle"),
                "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                "rank_ms": completion_timeline_trace_stage_duration_ms(trace, "rank"),
                "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
                "response_build_other_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build_other"),
            })
        })
        .collect::<Vec<_>>();

    let completion_total = read_u64_metric(counters.get("completion_total"));
    let fail_closed_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_fail_closed"));
    let cancelled_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_cancelled"));
    let ok_non_empty_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_ok_non_empty"));
    let ok_empty_total =
        read_u64_metric(counters.get("intellisense_v2_completion_result_total_ok_empty"));
    let deadline_total = read_u64_metric(
        counters
            .get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"),
    );
    let ready_total = read_u64_metric(
        counters.get("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready"),
    );
    let warmup_non_empty_samples = warmup_samples
        .iter()
        .filter(|sample| {
            sample
                .get("label_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0
        })
        .count();
    let measured_non_empty_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("label_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0
        })
        .count();
    let measured_ok_non_empty_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("outcome"))
                .and_then(|value| value.as_str())
                == Some("ok_non_empty")
        })
        .count();
    let measured_head_hit_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("route"))
                .and_then(|value| value.as_str())
                == Some("head_hit")
        })
        .count();
    let measured_exact_hit_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("route"))
                .and_then(|value| value.as_str())
                == Some("exact_hit")
        })
        .count();
    let sample_elapsed_histogram = |samples: &[serde_json::Value]| {
        let values = samples
            .iter()
            .filter_map(|sample| sample.get("elapsed_ms").and_then(|value| value.as_u64()))
            .map(|value| value as f64)
            .collect::<Vec<_>>();
        sample_histogram_value(&values)
    };
    let sample_trace_histogram = |samples: &[serde_json::Value], field: &str| {
        let values = samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get(field))
                    .and_then(|value| value.as_u64())
            })
            .map(|value| value as f64)
            .collect::<Vec<_>>();
        sample_histogram_value(&values)
    };
    let warmup_latency_histogram = sample_elapsed_histogram(&warmup_samples);
    let measured_latency_histogram = sample_elapsed_histogram(&measured_samples);
    let measured_latency_p95_ms = read_numeric_metric(measured_latency_histogram.get("p95"));

    let report = serde_json::json!({
        "change_id": "refactor-v2-completion-dual-artifact-path",
        "profile": PROFILE_NAME,
        "schema_version": 1,
        "configuration_path": conf_big_root,
        "module_path": module_path,
        "marker": "Объект.",
        "request_plan": {
            "cache_mode": "self_warmed_same_process",
            "wait_for_current_revision": true,
            "exact_type_index_seed_mode": "snapshot_precompute_current_revision",
            "warmup_requests": WARMUP_REQUESTS,
            "measured_requests": MEASURE_REQUESTS,
            "completion_trigger_mode": "invoked",
        },
        "warm_cache_seed": exact_type_index_seed,
        "warmup_samples": warmup_samples,
        "measured_samples": measured_samples,
        "summary": {
            "completion_total": completion_total,
            "trace_count_for_uri": filtered_traces.len(),
            "ok_non_empty_total": ok_non_empty_total,
            "ok_empty_total": ok_empty_total,
            "fail_closed_total": fail_closed_total,
            "cancelled_total": cancelled_total,
            "deadline_total": deadline_total,
            "ready_total": ready_total,
            "head_hit_total": read_u64_metric(
                counters.get("intellisense_v2_completion_route_total_route_head_hit")
            ),
            "exact_hit_total": read_u64_metric(
                counters.get("intellisense_v2_completion_route_total_route_exact_hit")
            ),
            "head_to_exact_upgrade_total": read_u64_metric(
                counters.get("intellisense_v2_completion_head_to_exact_upgrade_total")
            ),
            "prepare_timeout_total": read_u64_metric(
                counters.get(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"
                )
            ),
            "exact_deadline_total": read_u64_metric(
                counters.get(
                    "intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline"
                )
            ),
            "fallback_unavailable_total": read_u64_metric(
                counters.get("intellisense_v2_completion_fallback_unavailable_total")
            ),
            "interactive_wait_budget_exhausted_total": read_u64_metric(
                counters.get("intellisense_v2_interactive_wait_budget_exhausted_total")
            ),
            "trace_matching_mode": trace_matching_mode,
            "trace_request_id_present_total": trace_request_id_present_total,
            "warmup_non_empty_samples": warmup_non_empty_samples,
            "measured_non_empty_samples": measured_non_empty_samples,
            "measured_ok_non_empty_traces": measured_ok_non_empty_traces,
            "measured_head_hit_traces": measured_head_hit_traces,
            "measured_exact_hit_traces": measured_exact_hit_traces,
            "warmup_latency_ms": warmup_latency_histogram,
            "measured_latency_ms": measured_latency_histogram,
            "measured_turn_wait_ms": sample_trace_histogram(&measured_samples, "turn_wait_ms"),
            "measured_prepare_stateful_ms": sample_trace_histogram(&measured_samples, "prepare_stateful_ms"),
            "measured_wait_exact_type_index_ms": sample_trace_histogram(&measured_samples, "wait_exact_type_index_ms"),
            "measured_query_bundle_ms": sample_trace_histogram(&measured_samples, "query_bundle_ms"),
            "measured_collect_ms": sample_trace_histogram(&measured_samples, "collect_ms"),
        },
        "extension_like_key_latencies": {
            "intellisense_v2_wait_for_file_version_diagnostics": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_wait_for_file_version_diagnostics_ms",
                None
            ),
            "intellisense_v2_syntax_diagnostics_query": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_syntax_diagnostics_query_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_ms",
                None
            ),
            "intellisense_v2_wait_for_file_version_completion": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_wait_for_file_version_completion_ms",
                None
            ),
            "intellisense_v2_snapshot_completion": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_snapshot_completion_ms",
                None
            ),
            "intellisense_v2_ir_query_completion": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_ir_query_completion_ms",
                None
            ),
        },
        "latest_trace_summaries": latest_trace_summaries,
        "completion_timeline": {
            "trace_count": filtered_traces.len(),
            "selected_traces": filtered_traces,
            "raw": completion_timeline,
        },
        "observability": {
            "raw": observability_metrics,
        }
    });

    let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_WARM_CACHE_COMPLETION_PERF_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join("real-conf-big-warm-cache-completion-perf-live.json")
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for p37 real conf_big perf report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize p37 real conf_big perf report"),
    )
    .expect("write p37 real conf_big perf report");
    println!("{PROFILE_NAME}_path={}", report_path.display());

    assert!(
        trace_matching_mode == "request_id",
        "expected request-context parity to expose JSON-RPC request ids in completion timeline, trace_matching_mode={}, trace_request_id_present_total={}, filtered_traces={filtered_traces:?}",
        trace_matching_mode,
        trace_request_id_present_total
    );
    assert!(
        measured_non_empty_samples == MEASURE_REQUESTS,
        "expected all measured warm-cache samples to be non-empty, measured_non_empty_samples={}, measured_samples={measured_samples:?}",
        measured_non_empty_samples
    );
    assert!(
        measured_ok_non_empty_traces >= MEASURE_REQUESTS.saturating_sub(1),
        "expected nearly all measured warm-cache traces to be ok_non_empty, measured_ok_non_empty_traces={}, measured_samples={measured_samples:?}",
        measured_ok_non_empty_traces
    );
    assert!(
        measured_latency_p95_ms <= WARM_HEAD_PATH_P95_BUDGET_MS,
        "warm-cache head-path p95 regression: measured_latency_p95_ms={}ms > {}ms, measured_samples={measured_samples:?}",
        measured_latency_p95_ms,
        WARM_HEAD_PATH_P95_BUDGET_MS
    );
    assert!(
        read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_head_hit"))
            + read_u64_metric(counters.get("intellisense_v2_completion_route_total_route_exact_hit"))
            > 0,
        "expected warm-cache live report to expose at least one completion route bucket, counters={counters:?}"
    );
    assert!(
        completion_total >= (WARMUP_REQUESTS + MEASURE_REQUESTS) as u64,
        "expected completion_total >= collected request samples, completion_total={}, request_samples={}",
        completion_total,
        WARMUP_REQUESTS + MEASURE_REQUESTS
    );

    drop(server);
    drop(service);
    drain_task.abort();
}

#[tokio::test]
async fn p38_real_conf_big_revision_churn_completion_perf_report_live() {
    init_test_tracing();
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    static ENV_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _env_lock = ENV_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("env lock");
    let _blocking_parse_delay_guard =
        EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "1500");

    let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
    const PROFILE_NAME: &str = "p38_real_conf_big_post_handoff_readiness_completion_perf_report_live";
    const CHANGE_ID: &str = "refactor-current-revision-readiness-fast-lane";
    const WARMUP_REQUESTS: usize = 1;
    const MEASURE_REQUESTS: usize = 10;
    const DID_CHANGE_BURST_NOTIFICATIONS: usize = 4;
    const REVISION_CHURN_HEAD_PATH_P95_BUDGET_MS: f64 = 150.0;
    const SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS: f64 = 250.0;
    const SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS: u64 = 1_000;
    const POST_HANDOFF_QUEUE_WAIT_P95_FACTOR: f64 = 0.50;
    const POST_HANDOFF_QUEUE_WAIT_MAX_FACTOR: u64 = 4;
    let interactive_wait_budget_ms = bsl_runtime::system::global_runtime_config()
        .get_u64(bsl_runtime::system::RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs)
        .unwrap_or(120);

    let Some(conf_big_root) = conf_big_root_for_tests() else {
        if allow_fixture_skip {
            eprintln!(
                "skipping {PROFILE_NAME}: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
            );
            return;
        }
        panic!(
            "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly"
        );
    };

    let module_path = conf_big_large_module_path_for_tests(&conf_big_root);
    if !module_path.exists() {
        if allow_fixture_skip {
            eprintln!(
                "skipping {PROFILE_NAME}: module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                module_path.display()
            );
            return;
        }
        panic!(
            "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly",
            module_path.display()
        );
    }

    let module_text =
        std::fs::read_to_string(&module_path).expect("read conf_big module text for p38 report");
    let workspace_setup = ScaleAwareWorkspaceSetup {
        platform_docs_archive: syntax_helper_path_for_tests(),
        configuration_path: conf_big_root.clone(),
        platform_version: "8.3.25".to_string(),
    };
    let coordinator = Arc::new(SystemCoordinator::new());
    let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
    initialize_live_lsp_transport(&mut harness).await;
    prime_server_with_workspace_setup(&server, &workspace_setup, "p38_real_conf_big_live_setup")
        .await;

    let uri = Url::from_file_path(&module_path).expect("real conf_big module uri");
    let did_open = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "bsl".to_string(),
            version: 1,
            text: module_text.clone(),
        },
    };
    server.did_open(did_open).await;

    server.sync_v2_globals().await;
    let file_id = server.get_or_create_file_id_v2(&uri).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if server
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied()
                == Some(1)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("didOpen must publish latest received version on live transport");
    let opened_version = server
        .latest_received_file_versions_v2
        .read()
        .await
        .get(&file_id)
        .copied()
        .expect("latest received version for p38 opened file");
    assert_eq!(
        opened_version, 1,
        "real conf_big fixture must open at version 1"
    );
    assert!(
        server
            .analysis_v2
            .wait_for_file_version(file_id, opened_version)
            .await,
        "analysis runtime must catch up to opened real conf_big file version"
    );
    let exact_type_index_seed =
        seed_exact_type_index_for_current_file_version(&server, file_id).await;
    server.cancel_type_index_precompute_v2(file_id).await;

    let completion_position = find_utf16_position_after_marker(&module_text, "Объект.");
    let completion_context = Some(CompletionContext {
        trigger_kind: CompletionTriggerKind::INVOKED,
        trigger_character: None,
    });

    let mut current_text = module_text.clone();
    let mut current_version = opened_version;

    let mut warmup_samples = Vec::new();
    for index in 0..WARMUP_REQUESTS {
        let request_id = 38_100_000_i64 + index as i64;
        let started = Instant::now();
        let labels = live_transport_completion_labels_with_request(
            &mut harness,
            request_id,
            &uri,
            completion_position,
            completion_context.clone(),
        )
        .await;
        warmup_samples.push(serde_json::json!({
            "step": format!("warmup_completion_{}", index + 1),
            "request_id": request_id,
            "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            "label_count": labels.len(),
            "labels": labels,
            "version": current_version,
        }));
    }

    let metrics_before_measured = coordinator.observability_metrics();
    let counters_before_measured = metrics_before_measured
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics_before_measured.counters object");

    let mut measured_samples = Vec::new();
    for index in 0..MEASURE_REQUESTS {
        let mut burst_versions = Vec::new();
        let mut burst_appended = String::new();
        for burst_index in 0..DID_CHANGE_BURST_NOTIFICATIONS {
            let appended_text = if (index + burst_index) % 2 == 0 {
                " "
            } else {
                "\n"
            };
            let next_version = current_version
                .checked_add(1)
                .expect("p38 revision churn version overflow");
            live_transport_append_text_change(
                &mut harness,
                &uri,
                &current_text,
                next_version,
                appended_text,
            )
            .await;
            current_text.push_str(appended_text);
            current_version = next_version;
            burst_versions.push(current_version);
            burst_appended.push_str(appended_text);
        }

        let request_id = 38_100_100_i64 + index as i64;
        let started = Instant::now();
        let labels = live_transport_completion_labels_with_request(
            &mut harness,
            request_id,
            &uri,
            completion_position,
            completion_context.clone(),
        )
        .await;
        measured_samples.push(serde_json::json!({
            "step": format!("measured_revision_churn_completion_{}", index + 1),
            "request_id": request_id,
            "elapsed_ms": started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            "label_count": labels.len(),
            "labels": labels,
            "version": current_version,
            "burst_notification_count": DID_CHANGE_BURST_NOTIFICATIONS,
            "burst_versions": burst_versions,
            "appended_text": burst_appended,
        }));
    }

    let completion_timeline =
        live_transport_get_completion_timeline(&mut harness, 38_100_900, 96).await;
    let observability_metrics =
        live_transport_get_observability_metrics(&mut harness, 38_100_901).await;
    let timeline_traces = completion_timeline
        .get("traces")
        .and_then(|value| value.as_array())
        .expect("completion timeline traces array");
    let filtered_traces: Vec<serde_json::Value> = timeline_traces
        .iter()
        .filter(|trace| trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str()))
        .cloned()
        .collect();
    assert!(
        !filtered_traces.is_empty(),
        "expected non-empty completion timeline traces for real conf_big module"
    );

    let histograms = observability_metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");
    let counters = observability_metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");

    let total_sample_count = WARMUP_REQUESTS + MEASURE_REQUESTS;
    let trace_request_id_present_total = filtered_traces
        .iter()
        .filter(|trace| {
            trace
                .get("request_id")
                .and_then(|value| value.as_str())
                .is_some()
        })
        .count();
    let trace_matching_mode = if trace_request_id_present_total > 0 {
        "request_id"
    } else {
        "ordinal_by_filtered_uri_trace_order"
    };
    let fallback_trace_window: Vec<serde_json::Value> =
        if filtered_traces.len() >= total_sample_count {
            filtered_traces[filtered_traces.len() - total_sample_count..].to_vec()
        } else {
            filtered_traces.clone()
        };

    let enrich_samples = |samples: Vec<serde_json::Value>,
                          sample_offset: usize|
     -> Vec<serde_json::Value> {
        samples
            .into_iter()
            .enumerate()
            .map(|(sample_index, sample)| {
                let request_id_text = sample
                    .get("request_id")
                    .and_then(|value| value.as_i64())
                    .map(|value| value.to_string());
                let trace = if trace_request_id_present_total > 0 {
                    request_id_text.as_ref().and_then(|request_id| {
                        filtered_traces.iter().find(|trace| {
                            trace.get("request_id").and_then(|value| value.as_str())
                                == Some(request_id)
                        })
                    })
                } else {
                    fallback_trace_window.get(sample_offset + sample_index)
                };
                let trace_summary = trace.map(|trace| {
                    serde_json::json!({
                        "trace_id": trace.get("trace_id").and_then(|value| value.as_str()),
                        "request_id": trace.get("request_id").and_then(|value| value.as_str()),
                        "trigger_mode": trace.get("trigger_mode").and_then(|value| value.as_str()),
                        "outcome": trace.get("outcome").and_then(|value| value.as_str()),
                        "route": completion_timeline_prepare_detail_str(trace, "route"),
                        "fail_closed_cause": completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
                        "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                        "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                        "queue_outcome": trace.get("queue_outcome").and_then(|value| value.as_str()),
                        "turn_wait_outcome": trace.get("turn_wait_outcome").and_then(|value| value.as_str()),
                        "queued_completion_ahead": trace.get("queued_completion_ahead").and_then(|value| value.as_u64()),
                        "did_change_ahead": trace.get("did_change_ahead").and_then(|value| value.as_u64()),
                        "active_completion_count": trace.get("active_completion_count").and_then(|value| value.as_u64()),
                        "prepare_guard_outcome": completion_timeline_prepare_detail_str(trace, "guard_outcome"),
                        "prepare_outcome": completion_timeline_prepare_detail_str(trace, "outcome"),
                        "prepare_wait_elapsed_ms": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("wait_elapsed_ms"))
                            .and_then(|value| value.as_u64()),
                        "min_file_version": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("min_file_version"))
                            .and_then(|value| value.as_i64()),
                        "observed_file_version": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("observed_file_version"))
                            .and_then(|value| value.as_i64()),
                        "wait_for_file_version_runtime_queue_wait_ms": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("wait_for_file_version_runtime"))
                            .and_then(|value| value.get("queue_wait_ms"))
                            .and_then(|value| value.as_u64()),
                        "timeout_phase": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("timeout_attribution"))
                            .and_then(|value| value.get("phase"))
                            .and_then(|value| value.as_str()),
                        "timeout_source": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("timeout_attribution"))
                            .and_then(|value| value.get("source"))
                            .and_then(|value| value.as_str()),
                        "head_ready_before_wait": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("exact_wait"))
                            .and_then(|value| value.get("head_ready_before_wait"))
                            .and_then(|value| value.as_bool()),
                        "artifact_poll": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("exact_wait"))
                            .and_then(|value| value.get("artifact_poll"))
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        "dispatch_to_request_context_wait_ms": completion_timeline_server_edge_u64(
                            trace,
                            "dispatch_to_request_context_wait_ms",
                        ),
                        "transport_to_service_future_wait_ms": completion_timeline_server_edge_u64(
                            trace,
                            "transport_to_service_future_wait_ms",
                        ),
                        "service_future_to_first_poll_wait_ms": completion_timeline_server_edge_u64(
                            trace,
                            "service_future_to_first_poll_wait_ms",
                        ),
                        "prepare_snapshot_elapsed_ms": trace
                            .get("prepare_details")
                            .and_then(|value| value.get("snapshot_elapsed_ms"))
                            .and_then(|value| value.as_u64()),
                        "turn_wait_ms": completion_timeline_trace_stage_duration_ms(trace, "turn_wait"),
                        "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                        "wait_exact_type_index_ms": completion_timeline_trace_stage_duration_ms(trace, "wait_exact_type_index"),
                        "query_bundle_ms": completion_timeline_trace_stage_duration_ms(trace, "query_bundle"),
                        "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                        "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
                        "response_build_other_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build_other"),
                    })
                });
                let mut sample_object = sample
                    .as_object()
                    .cloned()
                    .expect("sample must be json object");
                sample_object.insert(
                    "trace".to_string(),
                    trace_summary.unwrap_or(serde_json::json!(null)),
                );
                serde_json::Value::Object(sample_object)
            })
            .collect::<Vec<_>>()
    };

    let warmup_samples = enrich_samples(warmup_samples, 0);
    let measured_samples = enrich_samples(measured_samples, WARMUP_REQUESTS);

    let latest_trace_summaries = filtered_traces
        .iter()
        .rev()
        .take(20)
        .map(|trace| {
            serde_json::json!({
                "trace_id": trace.get("trace_id").and_then(|value| value.as_str()),
                "request_id": trace.get("request_id").and_then(|value| value.as_str()),
                "trigger_mode": trace.get("trigger_mode").and_then(|value| value.as_str()),
                "outcome": trace.get("outcome").and_then(|value| value.as_str()),
                "route": completion_timeline_prepare_detail_str(trace, "route"),
                "fail_closed_cause": completion_timeline_prepare_detail_str(trace, "fail_closed_cause"),
                "started_at_ms": trace.get("started_at_ms").and_then(|value| value.as_u64()),
                "total_duration_ms": trace.get("total_duration_ms").and_then(|value| value.as_u64()),
                "dominant_stage": trace.get("dominant_stage").and_then(|value| value.as_str()),
                "queue_outcome": trace.get("queue_outcome").and_then(|value| value.as_str()),
                "turn_wait_outcome": trace.get("turn_wait_outcome").and_then(|value| value.as_str()),
                "queued_completion_ahead": trace.get("queued_completion_ahead").and_then(|value| value.as_u64()),
                "did_change_ahead": trace.get("did_change_ahead").and_then(|value| value.as_u64()),
                "active_completion_count": trace.get("active_completion_count").and_then(|value| value.as_u64()),
                "prepare_guard_outcome": completion_timeline_prepare_detail_str(trace, "guard_outcome"),
                "prepare_outcome": completion_timeline_prepare_detail_str(trace, "outcome"),
                "min_file_version": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("min_file_version"))
                    .and_then(|value| value.as_i64()),
                "observed_file_version": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("observed_file_version"))
                    .and_then(|value| value.as_i64()),
                "wait_for_file_version_runtime_queue_wait_ms": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("wait_for_file_version_runtime"))
                    .and_then(|value| value.get("queue_wait_ms"))
                    .and_then(|value| value.as_u64()),
                "timeout_phase": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("timeout_attribution"))
                    .and_then(|value| value.get("phase"))
                    .and_then(|value| value.as_str()),
                "timeout_source": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("timeout_attribution"))
                    .and_then(|value| value.get("source"))
                    .and_then(|value| value.as_str()),
                "head_ready_before_wait": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("exact_wait"))
                    .and_then(|value| value.get("head_ready_before_wait"))
                    .and_then(|value| value.as_bool()),
                "artifact_poll": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("exact_wait"))
                    .and_then(|value| value.get("artifact_poll"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                "prepare_wait_elapsed_ms": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("wait_elapsed_ms"))
                    .and_then(|value| value.as_u64()),
                "dispatch_to_request_context_wait_ms": completion_timeline_server_edge_u64(
                    trace,
                    "dispatch_to_request_context_wait_ms",
                ),
                "transport_to_service_future_wait_ms": completion_timeline_server_edge_u64(
                    trace,
                    "transport_to_service_future_wait_ms",
                ),
                "service_future_to_first_poll_wait_ms": completion_timeline_server_edge_u64(
                    trace,
                    "service_future_to_first_poll_wait_ms",
                ),
                "prepare_snapshot_elapsed_ms": trace
                    .get("prepare_details")
                    .and_then(|value| value.get("snapshot_elapsed_ms"))
                    .and_then(|value| value.as_u64()),
                "turn_wait_ms": completion_timeline_trace_stage_duration_ms(trace, "turn_wait"),
                "prepare_stateful_ms": completion_timeline_trace_stage_duration_ms(trace, "prepare_stateful"),
                "wait_exact_type_index_ms": completion_timeline_trace_stage_duration_ms(trace, "wait_exact_type_index"),
                "query_bundle_ms": completion_timeline_trace_stage_duration_ms(trace, "query_bundle"),
                "collect_ms": completion_timeline_trace_stage_duration_ms(trace, "collect"),
                "rank_ms": completion_timeline_trace_stage_duration_ms(trace, "rank"),
                "response_build_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build"),
                "response_build_other_ms": completion_timeline_trace_stage_duration_ms(trace, "response_build_other"),
            })
        })
        .collect::<Vec<_>>();

    let counter_delta = |name: &str| -> u64 {
        read_u64_metric(counters.get(name))
            .saturating_sub(read_u64_metric(counters_before_measured.get(name)))
    };

    let measured_non_empty_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("label_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0
        })
        .count();
    let measured_ok_non_empty_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("outcome"))
                .and_then(|value| value.as_str())
                == Some("ok_non_empty")
        })
        .count();
    let measured_fail_closed_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("outcome"))
                .and_then(|value| value.as_str())
                == Some("fail_closed")
        })
        .count();
    let measured_head_hit_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("route"))
                .and_then(|value| value.as_str())
                == Some("head_hit")
        })
        .count();
    let measured_exact_hit_traces = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("route"))
                .and_then(|value| value.as_str())
                == Some("exact_hit")
        })
        .count();
    let measured_trace_linked_samples = measured_samples
        .iter()
        .filter(|sample| sample.get("trace").is_some_and(|trace| !trace.is_null()))
        .count();
    let warmup_non_empty_samples = warmup_samples
        .iter()
        .filter(|sample| {
            sample
                .get("label_count")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0
        })
        .count();
    let sample_elapsed_histogram = |samples: &[serde_json::Value]| {
        let values = samples
            .iter()
            .filter_map(|sample| sample.get("elapsed_ms").and_then(|value| value.as_u64()))
            .map(|value| value as f64)
            .collect::<Vec<_>>();
        sample_histogram_value(&values)
    };
    let sample_trace_histogram = |samples: &[serde_json::Value], field: &str| {
        let values = samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get(field))
                    .and_then(|value| value.as_u64())
            })
            .map(|value| value as f64)
            .collect::<Vec<_>>();
        sample_histogram_value(&values)
    };
    let sample_trace_server_edge_histogram = |samples: &[serde_json::Value], field: &str| {
        let values = samples
            .iter()
            .filter_map(|sample| {
                sample
                    .get("trace")
                    .and_then(|trace| trace.get(field))
                    .and_then(|value| value.as_u64())
            })
            .map(|value| value as f64)
            .collect::<Vec<_>>();
        sample_histogram_value(&values)
    };
    let warmup_latency_histogram = sample_elapsed_histogram(&warmup_samples);
    let measured_latency_histogram = sample_elapsed_histogram(&measured_samples);
    let measured_latency_p95_ms = read_numeric_metric(measured_latency_histogram.get("p95"));
    let measured_wait_for_file_version_runtime_queue_wait_histogram =
        sample_trace_histogram(&measured_samples, "wait_for_file_version_runtime_queue_wait_ms");
    let measured_wait_for_file_version_runtime_queue_wait_p95_ms = read_numeric_metric(
        measured_wait_for_file_version_runtime_queue_wait_histogram.get("p95"),
    );
    let measured_wait_for_file_version_runtime_queue_wait_present_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("wait_for_file_version_runtime_queue_wait_ms"))
                .and_then(|value| value.as_u64())
                .is_some()
        })
        .count();
    let measured_head_ready_before_wait_present_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("head_ready_before_wait"))
                .and_then(|value| value.as_bool())
                .is_some()
        })
        .count();
    let measured_wait_for_file_version_runtime_queue_wait_max_ms = measured_samples
        .iter()
        .filter_map(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("wait_for_file_version_runtime_queue_wait_ms"))
                .and_then(|value| value.as_u64())
        })
        .max()
        .unwrap_or(0);
    let measured_prepare_timeout_wait_for_file_version_samples = measured_samples
        .iter()
        .filter(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("fail_closed_cause"))
                .and_then(|value| value.as_str())
                == Some("prepare_timeout")
                && sample
                    .get("trace")
                    .and_then(|trace| trace.get("timeout_phase"))
                    .and_then(|value| value.as_str())
                    == Some("wait_for_file_version")
        })
        .count();
    let measured_post_apply_head_gap_exact_deadline_samples = measured_samples
        .iter()
        .filter(|sample| {
            let trace = sample.get("trace");
            let artifact_poll = trace
                .and_then(|trace| trace.get("artifact_poll"))
                .filter(|value| !value.is_null());
            trace
                .and_then(|trace| trace.get("fail_closed_cause"))
                .and_then(|value| value.as_str())
                == Some("exact_deadline")
                && trace
                    .and_then(|trace| trace.get("head_ready_before_wait"))
                    .and_then(|value| value.as_bool())
                    == Some(false)
                && trace
                    .and_then(|trace| trace.get("min_file_version"))
                    .and_then(|value| value.as_i64())
                    .zip(
                        artifact_poll
                            .and_then(|poll| poll.get("observed_file_version"))
                            .and_then(|value| value.as_i64()),
                    )
                    .is_some_and(|(min_file_version, observed_file_version)| {
                        min_file_version == observed_file_version
                    })
        })
        .count();
    let post_handoff_queue_wait_p95_budget_ms =
        (interactive_wait_budget_ms as f64) * POST_HANDOFF_QUEUE_WAIT_P95_FACTOR;
    let post_handoff_queue_wait_max_budget_ms =
        interactive_wait_budget_ms.saturating_mul(POST_HANDOFF_QUEUE_WAIT_MAX_FACTOR);
    let measured_service_future_first_poll_histogram =
        sample_trace_server_edge_histogram(&measured_samples, "service_future_to_first_poll_wait_ms");
    let measured_service_future_first_poll_p95_ms =
        read_numeric_metric(measured_service_future_first_poll_histogram.get("p95"));
    let measured_service_future_first_poll_max_ms = measured_samples
        .iter()
        .filter_map(|sample| {
            sample
                .get("trace")
                .and_then(|trace| trace.get("service_future_to_first_poll_wait_ms"))
                .and_then(|value| value.as_u64())
        })
        .max()
        .unwrap_or(0);

    let report = serde_json::json!({
        "change_id": CHANGE_ID,
        "profile": PROFILE_NAME,
        "schema_version": 1,
        "configuration_path": conf_big_root,
        "module_path": module_path,
        "marker": "Объект.",
        "request_plan": {
            "cache_mode": "self_warmed_then_revision_churn_same_process",
            "wait_for_current_revision_before_seed": true,
            "exact_type_index_seed_mode": "snapshot_precompute_current_revision",
            "warmup_requests": WARMUP_REQUESTS,
            "measured_requests": MEASURE_REQUESTS,
            "completion_trigger_mode": "invoked",
            "transport_path": "tower_lsp_server_serve_duplex",
            "churn_profile": "didChange-burst",
            "churn_before_each_measured_completion": true,
            "churn_edit_kind": "append_at_eof_incremental",
            "did_change_notifications_per_measured_completion": DID_CHANGE_BURST_NOTIFICATIONS,
            "did_change_blocking_parse_delay_ms": 1500,
        },
        "warm_cache_seed": exact_type_index_seed,
        "warmup_samples": warmup_samples,
        "measured_samples": measured_samples,
        "summary": {
            "trace_count_for_uri": filtered_traces.len(),
            "trace_matching_mode": trace_matching_mode,
            "trace_request_id_present_total": trace_request_id_present_total,
            "warmup_non_empty_samples": warmup_non_empty_samples,
            "measured_trace_linked_samples": measured_trace_linked_samples,
            "measured_non_empty_samples": measured_non_empty_samples,
            "measured_ok_non_empty_traces": measured_ok_non_empty_traces,
            "measured_fail_closed_traces": measured_fail_closed_traces,
            "measured_head_hit_traces": measured_head_hit_traces,
            "measured_exact_hit_traces": measured_exact_hit_traces,
            "measured_completion_total_delta": counter_delta("completion_total"),
            "measured_ok_non_empty_total_delta": counter_delta("intellisense_v2_completion_result_total_ok_non_empty"),
            "measured_ok_empty_total_delta": counter_delta("intellisense_v2_completion_result_total_ok_empty"),
            "measured_fail_closed_total_delta": counter_delta("intellisense_v2_completion_result_total_fail_closed"),
            "measured_cancelled_total_delta": counter_delta("intellisense_v2_completion_result_total_cancelled"),
            "measured_deadline_total_delta": counter_delta("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"),
            "measured_ready_total_delta": counter_delta("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready"),
            "measured_no_matching_task_total_delta": counter_delta("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_no_matching_task"),
            "measured_task_present_wrong_version_total_delta": counter_delta("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_task_present_wrong_version"),
            "measured_observed_version_mismatch_total_delta": counter_delta("intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_observed_version_mismatch"),
            "measured_head_hit_total_delta": counter_delta("intellisense_v2_completion_route_total_route_head_hit"),
            "measured_exact_hit_total_delta": counter_delta("intellisense_v2_completion_route_total_route_exact_hit"),
            "measured_head_to_exact_upgrade_total_delta": counter_delta("intellisense_v2_completion_head_to_exact_upgrade_total"),
            "measured_prepare_timeout_total_delta": counter_delta(
                "intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"
            ),
            "measured_exact_deadline_total_delta": counter_delta(
                "intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline"
            ),
            "measured_fallback_unavailable_total_delta": counter_delta("intellisense_v2_completion_fallback_unavailable_total"),
            "measured_interactive_wait_budget_exhausted_total_delta": counter_delta("intellisense_v2_interactive_wait_budget_exhausted_total"),
            "warmup_latency_ms": warmup_latency_histogram,
            "measured_latency_ms": measured_latency_histogram,
            "interactive_wait_budget_ms": interactive_wait_budget_ms,
            "measured_wait_for_file_version_runtime_queue_wait_ms": measured_wait_for_file_version_runtime_queue_wait_histogram,
            "measured_wait_for_file_version_runtime_queue_wait_present_samples": measured_wait_for_file_version_runtime_queue_wait_present_samples,
            "measured_head_ready_before_wait_present_samples": measured_head_ready_before_wait_present_samples,
            "measured_wait_for_file_version_runtime_queue_wait_max_ms": measured_wait_for_file_version_runtime_queue_wait_max_ms,
            "measured_prepare_timeout_wait_for_file_version_samples": measured_prepare_timeout_wait_for_file_version_samples,
            "measured_post_apply_head_gap_exact_deadline_samples": measured_post_apply_head_gap_exact_deadline_samples,
            "measured_service_future_to_first_poll_wait_ms": measured_service_future_first_poll_histogram,
            "measured_service_future_to_first_poll_wait_max_ms": measured_service_future_first_poll_max_ms,
            "measured_dispatch_to_request_context_wait_ms": sample_trace_server_edge_histogram(
                &measured_samples,
                "dispatch_to_request_context_wait_ms"
            ),
            "measured_transport_to_service_future_wait_ms": sample_trace_server_edge_histogram(
                &measured_samples,
                "transport_to_service_future_wait_ms"
            ),
            "measured_turn_wait_ms": sample_trace_histogram(&measured_samples, "turn_wait_ms"),
            "measured_prepare_stateful_ms": sample_trace_histogram(&measured_samples, "prepare_stateful_ms"),
            "measured_wait_exact_type_index_ms": sample_trace_histogram(&measured_samples, "wait_exact_type_index_ms"),
            "measured_query_bundle_ms": sample_trace_histogram(&measured_samples, "query_bundle_ms"),
            "measured_collect_ms": sample_trace_histogram(&measured_samples, "collect_ms"),
        },
        "extension_like_key_latencies": {
            "intellisense_v2_wait_for_file_version_diagnostics": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_wait_for_file_version_diagnostics_ms",
                None
            ),
            "intellisense_v2_syntax_diagnostics_query": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_syntax_diagnostics_query_ms",
                None
            ),
            "intellisense_v2_semantic_diagnostics_query": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_semantic_diagnostics_query_ms",
                None
            ),
            "intellisense_v2_wait_for_file_version_completion": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_wait_for_file_version_completion_ms",
                None
            ),
            "intellisense_v2_snapshot_completion": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_snapshot_completion_ms",
                None
            ),
            "intellisense_v2_ir_query_completion": histogram_metric_value_or_zero(
                histograms,
                "intellisense_v2_ir_query_completion_ms",
                None
            ),
        },
        "latest_trace_summaries": latest_trace_summaries,
        "completion_timeline": {
            "trace_count": filtered_traces.len(),
            "selected_traces": filtered_traces,
            "raw": completion_timeline,
        },
        "observability": {
            "raw": observability_metrics,
        }
    });

    let report_path = std::env::var("BSL_V2_REAL_CONF_BIG_REVISION_CHURN_COMPLETION_PERF_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join("real-conf-big-post-handoff-readiness-completion-perf-live.json")
        });
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)
            .expect("failed to create directory for p38 real conf_big perf report");
    }
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("serialize p38 real conf_big perf report"),
    )
    .expect("write p38 real conf_big perf report");
    println!("{PROFILE_NAME}_path={}", report_path.display());

    assert!(
        MEASURE_REQUESTS >= 10,
        "revision-churn representative gate must collect at least 10 measured samples"
    );
    assert!(
        trace_matching_mode == "request_id",
        "expected request-context parity to expose JSON-RPC request ids in completion timeline, trace_matching_mode={}, trace_request_id_present_total={}, filtered_traces={filtered_traces:?}",
        trace_matching_mode,
        trace_request_id_present_total
    );
    assert!(
        warmup_non_empty_samples == WARMUP_REQUESTS,
        "expected baseline warm-cache samples to be non-empty before churn, warmup_non_empty_samples={}, warmup_samples={warmup_samples:?}",
        warmup_non_empty_samples
    );
    assert!(
        measured_trace_linked_samples == MEASURE_REQUESTS,
        "expected every measured churn sample to link to a completion timeline trace, measured_trace_linked_samples={}, measured_samples={measured_samples:?}",
        measured_trace_linked_samples
    );
    assert!(
        measured_wait_for_file_version_runtime_queue_wait_present_samples == MEASURE_REQUESTS,
        "expected every measured post-handoff readiness sample to expose wait_for_file_version_runtime.queue_wait_ms, present_samples={}, measured_samples={measured_samples:?}",
        measured_wait_for_file_version_runtime_queue_wait_present_samples
    );
    assert!(
        measured_head_ready_before_wait_present_samples == MEASURE_REQUESTS,
        "expected every measured post-handoff readiness sample to expose head_ready_before_wait, present_samples={}, measured_samples={measured_samples:?}",
        measured_head_ready_before_wait_present_samples
    );
    assert!(
        counter_delta("completion_total") >= MEASURE_REQUESTS as u64,
        "expected measured completion_total delta >= churn request samples, completion_total_delta={}, measured_requests={}",
        counter_delta("completion_total"),
        MEASURE_REQUESTS
    );
    assert!(
        measured_non_empty_samples == MEASURE_REQUESTS,
        "expected every measured revision-churn sample to return a first-response candidate list, measured_non_empty_samples={}, measured_samples={measured_samples:?}",
        measured_non_empty_samples
    );
    assert!(
        measured_ok_non_empty_traces == MEASURE_REQUESTS,
        "expected every measured revision-churn trace to be ok_non_empty, measured_ok_non_empty_traces={}, measured_samples={measured_samples:?}",
        measured_ok_non_empty_traces
    );
    assert!(
        measured_fail_closed_traces == 0
            && counter_delta("intellisense_v2_completion_result_total_fail_closed") == 0,
        "revision-churn gate must fail on first-response fail_closed regressions, measured_fail_closed_traces={}, fail_closed_total_delta={}, measured_samples={measured_samples:?}",
        measured_fail_closed_traces,
        counter_delta("intellisense_v2_completion_result_total_fail_closed")
    );
    assert!(
        counter_delta("intellisense_v2_completion_fallback_unavailable_total") == 0,
        "revision-churn gate must not degrade to fallback_unavailable, fallback_unavailable_total_delta={}, counters={counters:?}",
        counter_delta("intellisense_v2_completion_fallback_unavailable_total")
    );
    assert!(
        counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout")
            == 0
            && counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
                == 0,
        "revision-churn gate must keep first-response fail-closed cause buckets at zero after head-path rollout, prepare_timeout_total_delta={}, exact_deadline_total_delta={}, counters={counters:?}",
        counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_prepare_timeout"),
        counter_delta("intellisense_v2_completion_fail_closed_cause_total_cause_exact_deadline")
    );
    assert!(
        measured_head_hit_traces + measured_exact_hit_traces == MEASURE_REQUESTS,
        "expected every measured revision-churn trace to expose head/exact route attribution, measured_head_hit_traces={}, measured_exact_hit_traces={}, measured_samples={measured_samples:?}",
        measured_head_hit_traces,
        measured_exact_hit_traces
    );
    assert!(
        measured_latency_p95_ms <= REVISION_CHURN_HEAD_PATH_P95_BUDGET_MS,
        "revision-churn head-path p95 regression: measured_latency_p95_ms={}ms > {}ms, measured_samples={measured_samples:?}",
        measured_latency_p95_ms,
        REVISION_CHURN_HEAD_PATH_P95_BUDGET_MS
    );
    assert!(
        measured_wait_for_file_version_runtime_queue_wait_p95_ms <= post_handoff_queue_wait_p95_budget_ms,
        "post-handoff readiness queue-wait p95 regression: measured_wait_for_file_version_runtime.queue_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
        measured_wait_for_file_version_runtime_queue_wait_p95_ms,
        post_handoff_queue_wait_p95_budget_ms
    );
    assert!(
        measured_wait_for_file_version_runtime_queue_wait_max_ms <= post_handoff_queue_wait_max_budget_ms,
        "post-handoff readiness queue-wait max regression: measured_wait_for_file_version_runtime.queue_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
        measured_wait_for_file_version_runtime_queue_wait_max_ms,
        post_handoff_queue_wait_max_budget_ms
    );
    assert!(
        measured_prepare_timeout_wait_for_file_version_samples == 0,
        "post-handoff readiness gate must fail on prepare_timeout@wait_for_file_version after same-file handoff, prepare_timeout_wait_for_file_version_samples={}, measured_samples={measured_samples:?}",
        measured_prepare_timeout_wait_for_file_version_samples
    );
    assert!(
        measured_post_apply_head_gap_exact_deadline_samples == 0,
        "post-handoff readiness gate must fail on exact_deadline with observed current revision and head_ready_before_wait=false, samples={}, measured_samples={measured_samples:?}",
        measured_post_apply_head_gap_exact_deadline_samples
    );
    assert!(
        measured_service_future_first_poll_p95_ms <= SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS,
        "revision-churn pre-poll p95 regression: measured_service_future_to_first_poll_wait_ms p95={}ms > {}ms, measured_samples={measured_samples:?}",
        measured_service_future_first_poll_p95_ms,
        SERVICE_FUTURE_FIRST_POLL_P95_BUDGET_MS
    );
    assert!(
        measured_service_future_first_poll_max_ms <= SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS,
        "revision-churn pre-poll max regression: measured_service_future_to_first_poll_wait_ms max={}ms > {}ms, measured_samples={measured_samples:?}",
        measured_service_future_first_poll_max_ms,
        SERVICE_FUTURE_FIRST_POLL_MAX_BUDGET_MS
    );
    assert!(
        counter_delta("intellisense_v2_completion_route_total_route_head_hit")
            + counter_delta("intellisense_v2_completion_route_total_route_exact_hit")
            >= MEASURE_REQUESTS as u64,
        "expected measured revision-churn route counters to cover all samples, head_hit_total_delta={}, exact_hit_total_delta={}, counters={counters:?}",
        counter_delta("intellisense_v2_completion_route_total_route_head_hit"),
        counter_delta("intellisense_v2_completion_route_total_route_exact_hit")
    );

    drop(server);
    harness.shutdown().await;
}
