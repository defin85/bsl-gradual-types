//! LanguageServer trait implementation for BslLanguageServer
//!
//! This module contains the complete implementation of the tower_lsp::LanguageServer trait.
//! All LSP protocol methods are implemented here:
//! - Lifecycle: initialize, initialized, shutdown
//! - Configuration: did_change_configuration
//! - File management: did_open, did_change, did_close
//! - Features: completion, hover, goto_definition, signature_help
//! - Commands: execute_command

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::LanguageServer;
use tracing::{debug, error, info, warn};

use bsl_backend::data::loaders::progress::{IndexingPhase, ProgressUpdate};
use bsl_backend::system::{startup_v2, StartupInputs};
use bsl_shared::api::semantic_dtos::{GetSemanticHtmlRequest, GetSemanticTreeRequest};
use bsl_shared::utils::hash::hash_content;

use crate::commands::{
    handle_cache_clear, handle_cache_set_enabled, handle_cache_stats, handle_get_all_types,
    handle_get_type_repository_stats, handle_parse_configuration, handle_query_type,
    handle_search_types, semantic_html_from_tree, semantic_tree_from_ir, CacheCommandParams,
    CacheToggleParams, GetAllTypesRequest, ParseConfigurationParams, QueryTypeParams,
    SearchTypesRequest,
};
use crate::config::{BslSettings, LspConfig};
use crate::handlers::{
    apply_text_edit, build_document_symbols, build_workspace_symbols, format_bsl_range_to_edits,
    format_bsl_to_edits, handle_code_actions_v2, handle_completion_resolve,
    handle_goto_definition_v2, handle_hover_v2, handle_inlay_hints_v2, handle_prepare_rename,
    handle_references, handle_rename, handle_signature_help_v2, RenameError,
};
use crate::progress::log_progress_to_file;
use crate::progress_bridge::{LspWorkDoneReporter, ProgressReporter};
use crate::types::{GetCurrentContextParams, ServerStatus, ServerStatusParams};

use super::{BslLanguageServer, CompletionStaleFallbackCacheEntryV2};

fn effective_include_flow_sensitive(
    request_override: Option<bool>,
    enable_flow_sensitive_setting: bool,
) -> bool {
    request_override.unwrap_or(enable_flow_sensitive_setting)
}

fn should_schedule_profile(
    trigger: bsl_runtime::application::DiagnosticsTrigger,
    profile: bsl_runtime::application::DiagnosticsProfile,
    flow_sensitive_enabled: bool,
) -> bool {
    if matches!(
        profile,
        bsl_runtime::application::DiagnosticsProfile::IdleHeavy
    ) && !flow_sensitive_enabled
    {
        return matches!(
            trigger,
            bsl_runtime::application::DiagnosticsTrigger::DidSave
                | bsl_runtime::application::DiagnosticsTrigger::Idle
        );
    }
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LargeChurnTransition {
    None,
    Entered,
    Exited,
}

fn should_defer_heavy_diagnostics_for_large_churn(
    trigger: bsl_runtime::application::DiagnosticsTrigger,
    profile: bsl_runtime::application::DiagnosticsProfile,
    large_churn_active: bool,
) -> bool {
    large_churn_active
        && matches!(
            trigger,
            bsl_runtime::application::DiagnosticsTrigger::DidChange
        )
        && !matches!(profile, bsl_runtime::application::DiagnosticsProfile::Fast)
}

fn lsp_range_change_to_parser_edit(
    change: &TextDocumentContentChangeEvent,
) -> Option<bsl_runtime::system::parser_coordinator::TextEdit> {
    let range = change.range?;
    Some(bsl_runtime::system::parser_coordinator::TextEdit {
        start_line: range.start.line,
        start_utf16_column: range.start.character,
        old_end_line: range.end.line,
        old_end_utf16_column: range.end.character,
        new_text: change.text.clone(),
    })
}

fn unix_time_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn changed_range_footprint_bytes(
    range: &bsl_runtime::system::parser_coordinator::ParseChangedRange,
) -> usize {
    let old_span =
        usize::try_from(range.old_end_byte.saturating_sub(range.start_byte)).unwrap_or(0);
    let new_span =
        usize::try_from(range.new_end_byte.saturating_sub(range.start_byte)).unwrap_or(0);
    old_span.max(new_span)
}

fn advance_large_churn_state(
    state: &mut super::ScaleAwareChurnStateV2,
    now: Instant,
    is_large_document: bool,
    knobs: bsl_runtime::application::ScaleAwareDiagnosticsKnobs,
) -> LargeChurnTransition {
    if now.duration_since(state.window_started_at) > knobs.churn_window {
        state.window_started_at = now;
        state.changes_in_window = 0;
    }

    state.changes_in_window = state.changes_in_window.saturating_add(1);
    let was_active = state.large_churn_active;
    let is_churn = state.changes_in_window >= knobs.churn_min_changes;
    state.large_churn_active = knobs.enabled && is_large_document && is_churn;

    match (was_active, state.large_churn_active) {
        (false, true) => LargeChurnTransition::Entered,
        (true, false) => LargeChurnTransition::Exited,
        _ => LargeChurnTransition::None,
    }
}

fn completion_trigger_mode_label(context: Option<&CompletionContext>) -> &'static str {
    match context.map(|ctx| ctx.trigger_kind) {
        Some(CompletionTriggerKind::TRIGGER_CHARACTER) => "trigger_character",
        Some(CompletionTriggerKind::INVOKED) => "invoked",
        Some(CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS) => "trigger_for_incomplete",
        Some(_) => "other",
        None => "none",
    }
}

const COMPLETION_SHADOW_INTERNAL_TRIGGER_MARKER: &str = "__bsl_shadow_internal__";

fn completion_shadow_internal_trigger_payload(value: &str) -> Option<Option<char>> {
    let payload = value.strip_prefix(COMPLETION_SHADOW_INTERNAL_TRIGGER_MARKER)?;
    let payload = payload.strip_prefix(':')?;
    let codepoint = payload.parse::<u32>().ok()?;
    if codepoint == 0 {
        Some(None)
    } else {
        char::from_u32(codepoint).map(Some)
    }
}

fn completion_shadow_internal_trigger_value(trigger_char_hint: Option<char>) -> String {
    format!(
        "{}:{}",
        COMPLETION_SHADOW_INTERNAL_TRIGGER_MARKER,
        trigger_char_hint.map(u32::from).unwrap_or(0),
    )
}

fn completion_is_shadow_internal_request(context: Option<&CompletionContext>) -> bool {
    context
        .and_then(|ctx| ctx.trigger_character.as_deref())
        .is_some_and(|value| completion_shadow_internal_trigger_payload(value).is_some())
}

fn completion_trigger_character(context: Option<&CompletionContext>) -> Option<char> {
    context
        .and_then(|ctx| ctx.trigger_character.as_deref())
        .and_then(|value| {
            completion_shadow_internal_trigger_payload(value)
                .unwrap_or_else(|| value.chars().next())
        })
}

fn is_completion_identifier_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn completion_request_targets_member_access(
    text: &str,
    position: Position,
    trigger_char_hint: Option<char>,
) -> bool {
    if trigger_char_hint == Some('.') {
        return true;
    }

    let Some(line_text) = text.lines().nth(position.line as usize) else {
        return false;
    };
    let column_index =
        bsl_backend::system::positioning::utf16_to_byte_offset(line_text, position.character);
    let line_prefix = line_text.get(..column_index).unwrap_or(line_text);
    let line_prefix = if line_text
        .get(column_index..)
        .and_then(|tail| tail.chars().next())
        == Some('.')
    {
        format!("{line_prefix}.")
    } else {
        line_prefix.to_string()
    };

    let trimmed = line_prefix.trim_end();
    let Some(dot_pos) = trimmed.rfind('.') else {
        return false;
    };
    let after_dot = trimmed[dot_pos + 1..].trim_start();
    after_dot.is_empty() || after_dot.chars().all(is_completion_identifier_char)
}

fn completion_labels_fingerprint(response: &CompletionResponse) -> Vec<String> {
    const PARITY_LABELS_LIMIT: usize = 64;

    let mut labels = BTreeSet::new();
    let push_label = |set: &mut BTreeSet<String>, label: &str| {
        if set.len() >= PARITY_LABELS_LIMIT {
            return;
        }
        let normalized = label.trim().to_lowercase();
        if normalized.is_empty() {
            return;
        }
        set.insert(normalized);
    };

    match response {
        CompletionResponse::List(list) => {
            for item in &list.items {
                push_label(&mut labels, &item.label);
            }
        }
        CompletionResponse::Array(items) => {
            for item in items {
                push_label(&mut labels, &item.label);
            }
        }
    }

    labels.into_iter().collect()
}

fn completion_labels_overlap_ratio(lhs: &[String], rhs: &[String]) -> f64 {
    if lhs.is_empty() || rhs.is_empty() {
        return 0.0;
    }

    let left: BTreeSet<&str> = lhs.iter().map(String::as_str).collect();
    let right: BTreeSet<&str> = rhs.iter().map(String::as_str).collect();
    let intersection = left.intersection(&right).count();
    let union = left.union(&right).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn completion_parity_overlap_bucket(overlap_ratio: f64) -> &'static str {
    if overlap_ratio <= 0.0 {
        "none"
    } else if overlap_ratio < 0.3 {
        "low"
    } else {
        "high"
    }
}

fn completion_publish_allowed(request_epoch: u64, latest_request_epoch: Option<u64>) -> bool {
    match latest_request_epoch {
        Some(latest_epoch) => latest_epoch == request_epoch,
        None => true,
    }
}

fn completion_queue_enqueue_failed(
    outcome: super::completion_dispatcher::QueueEnqueueOutcome,
) -> bool {
    matches!(
        outcome,
        super::completion_dispatcher::QueueEnqueueOutcome::Full
            | super::completion_dispatcher::QueueEnqueueOutcome::Closed
    )
}

fn completion_empty_response(is_incomplete: bool) -> crate::handlers::CompletionResponseWithStats {
    crate::handlers::CompletionResponseWithStats {
        response: CompletionResponse::List(CompletionList {
            is_incomplete,
            items: Vec::new(),
        }),
        stats: None,
        had_error: false,
    }
}

fn completion_incomplete_empty_response() -> crate::handlers::CompletionResponseWithStats {
    completion_empty_response(true)
}

fn completion_response_with_cached_items(
    items: Vec<CompletionItem>,
) -> crate::handlers::CompletionResponseWithStats {
    crate::handlers::CompletionResponseWithStats {
        response: CompletionResponse::List(CompletionList {
            is_incomplete: true,
            items,
        }),
        stats: None,
        had_error: false,
    }
}

fn spawn_completion_refresh_after_stale_fastpath(
    server: BslLanguageServer,
    mut params: CompletionParams,
    trigger_char_hint: Option<char>,
) {
    let shadow_trigger = completion_shadow_internal_trigger_value(trigger_char_hint);
    if let Some(context) = params.context.as_mut() {
        context.trigger_character = Some(shadow_trigger);
    } else {
        params.context = Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: Some(shadow_trigger),
        });
    }
    tokio::spawn(async move {
        let _ = server.completion(params).await;
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionResponseRoute {
    Legacy,
    EventDriven,
}

impl CompletionResponseRoute {
    fn event_driven_guards_enabled(self) -> bool {
        matches!(self, Self::EventDriven)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompletionRoutingPlan {
    response_route: CompletionResponseRoute,
    run_shadow_event_driven: bool,
}

fn completion_dispatch_enabled_for_mode(mode: bsl_runtime::application::CompletionMode) -> bool {
    !matches!(mode, bsl_runtime::application::CompletionMode::Off)
}

fn completion_canary_routing_key(
    uri: &Url,
    position: Position,
    trigger_mode: &str,
    trigger_char_hint: Option<char>,
    version_hint: Option<i32>,
) -> String {
    let trigger_char_code = trigger_char_hint.map(u32::from).unwrap_or(0);
    format!(
        "{}:{}:{}:{}:{}:{}",
        uri,
        position.line,
        position.character,
        trigger_mode,
        trigger_char_code,
        version_hint.unwrap_or(i32::MIN),
    )
}

fn completion_route_canary_event_driven(routing_key: &str, canary_percent: u8) -> bool {
    if canary_percent == 0 {
        return false;
    }
    if canary_percent >= 100 {
        return true;
    }
    (hash_content(routing_key) % 100) < u64::from(canary_percent)
}

fn completion_routing_plan(
    mode: bsl_runtime::application::CompletionMode,
    canary_percent: u8,
    routing_key: &str,
) -> CompletionRoutingPlan {
    match mode {
        bsl_runtime::application::CompletionMode::Off => CompletionRoutingPlan {
            response_route: CompletionResponseRoute::Legacy,
            run_shadow_event_driven: false,
        },
        bsl_runtime::application::CompletionMode::Shadow => CompletionRoutingPlan {
            response_route: CompletionResponseRoute::Legacy,
            run_shadow_event_driven: true,
        },
        bsl_runtime::application::CompletionMode::Canary => CompletionRoutingPlan {
            response_route: if completion_route_canary_event_driven(routing_key, canary_percent) {
                CompletionResponseRoute::EventDriven
            } else {
                CompletionResponseRoute::Legacy
            },
            run_shadow_event_driven: false,
        },
        bsl_runtime::application::CompletionMode::On => CompletionRoutingPlan {
            response_route: CompletionResponseRoute::EventDriven,
            run_shadow_event_driven: false,
        },
    }
}

fn completion_observability_mode_label(
    response_route: CompletionResponseRoute,
    shadow_internal_request: bool,
) -> &'static str {
    if shadow_internal_request {
        "shadow"
    } else if response_route.event_driven_guards_enabled() {
        "event_driven"
    } else {
        "legacy"
    }
}

struct CompletionRequestDropCancelGuard {
    request_id: Option<String>,
    cancellation_registry: Arc<super::completion_cancellation::CompletionCancellationRegistry>,
    dispatcher: Arc<super::completion_dispatcher::CompletionDispatcherRegistry>,
    disarmed: bool,
}

impl CompletionRequestDropCancelGuard {
    fn new(
        request_id: Option<String>,
        cancellation_registry: Arc<super::completion_cancellation::CompletionCancellationRegistry>,
        dispatcher: Arc<super::completion_dispatcher::CompletionDispatcherRegistry>,
    ) -> Self {
        Self {
            request_id,
            cancellation_registry,
            dispatcher,
            disarmed: false,
        }
    }

    fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for CompletionRequestDropCancelGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }
        let Some(request_id) = self.request_id.clone() else {
            return;
        };
        let Some(entry) = self.cancellation_registry.cancel_request(&request_id) else {
            return;
        };
        let dispatcher = Arc::clone(&self.dispatcher);
        tokio::spawn(async move {
            let _ = dispatcher.emit_cancel(entry.file_id, request_id).await;
        });
    }
}

async fn completion_checkpoint_outcome(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    request_id: Option<&str>,
    request_epoch: u64,
    cancellation_token: Option<&super::completion_cancellation::CompletionCancellationToken>,
    checkpoint: &'static str,
    cancel_event_emitted: &mut bool,
) -> Option<&'static str> {
    if cancellation_token.is_some_and(|token| token.is_cancelled()) {
        if let Some(request_id) = request_id {
            if !*cancel_event_emitted {
                let cancel_ticket = server
                    .completion_dispatcher_v2
                    .emit_cancel(file_id, request_id.to_string())
                    .await;
                *cancel_event_emitted = true;
                if completion_queue_enqueue_failed(cancel_ticket.queue_outcome) {
                    debug!(
                        file_id = file_id.0,
                        file_seq = cancel_ticket.file_seq,
                        request_epoch = cancel_ticket.request_epoch,
                        request_id = request_id,
                        queue_outcome = ?cancel_ticket.queue_outcome,
                        checkpoint,
                        "completion dispatcher dropped cancel checkpoint event"
                    );
                }
            }
        }
        return Some("cancelled");
    }

    let latest_request_epoch = server
        .completion_dispatcher_v2
        .latest_request_epoch(file_id)
        .await;
    if !completion_publish_allowed(request_epoch, latest_request_epoch) {
        return Some("superseded_epoch");
    }

    None
}

#[allow(clippy::too_many_arguments)]
async fn completion_checkpoint_outcome_if_enabled(
    event_driven_guards_enabled: bool,
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    request_id: Option<&str>,
    request_epoch: u64,
    cancellation_token: Option<&super::completion_cancellation::CompletionCancellationToken>,
    checkpoint: &'static str,
    cancel_event_emitted: &mut bool,
) -> Option<&'static str> {
    if !event_driven_guards_enabled {
        return None;
    }
    completion_checkpoint_outcome(
        server,
        file_id,
        request_id,
        request_epoch,
        cancellation_token,
        checkpoint,
        cancel_event_emitted,
    )
    .await
}

async fn completion_cached_stale_items(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    observed_deps_id: &bsl_analysis_v2::DepsSnapshotId,
    observed_settings_id: Option<&bsl_analysis_v2::SettingsId>,
    observed_file_version: Option<i32>,
) -> (Option<Vec<CompletionItem>>, Option<Vec<CompletionItem>>) {
    let cache = server.completion_stale_fallback_cache_v2.read().await;
    let strict = match (observed_settings_id, observed_file_version) {
        (Some(settings_id), Some(file_version)) => cache.get(&file_id).and_then(|entry| {
            let compatible = entry.deps_id == *observed_deps_id
                && entry.settings_id == *settings_id
                && entry.file_version == file_version
                && !entry.items.is_empty();
            if compatible {
                Some(entry.items.clone())
            } else {
                None
            }
        }),
        _ => None,
    };
    let relaxed = cache.get(&file_id).and_then(|entry| {
        if entry.items.is_empty() {
            return None;
        }
        let deps_compatible = entry.deps_id == *observed_deps_id;
        let settings_compatible = observed_settings_id
            .map(|settings_id| entry.settings_id == *settings_id)
            .unwrap_or(true);
        let file_version_compatible = observed_file_version
            .map(|file_version| entry.file_version == file_version)
            .unwrap_or(true);
        if deps_compatible && settings_compatible && file_version_compatible {
            Some(entry.items.clone())
        } else {
            None
        }
    });
    (strict, relaxed)
}

#[allow(clippy::too_many_arguments)]
async fn resolve_completion_without_ir(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    observed_deps_id: bsl_analysis_v2::DepsSnapshotId,
    observed_settings_id: Option<bsl_analysis_v2::SettingsId>,
    observed_file_version: Option<i32>,
    member_access_context: bool,
    file_content: Arc<str>,
    file_path: Arc<str>,
    parse_result: Option<Arc<bsl_syntax::ast::ParseResult>>,
    member_access_owner_type_hint: Option<bsl_shared::domain::types::TypeResolution>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    position: Position,
    uri: &Url,
    index_snapshot: &bsl_backend::system::IndexSnapshot,
    snippet_support: bool,
    include_flow_sensitive: bool,
    trigger_char_hint: Option<char>,
) -> (
    &'static str,
    Option<crate::handlers::CompletionResponseWithStats>,
) {
    let (strict_stale_cached_items, relaxed_stale_cached_items) = completion_cached_stale_items(
        server,
        file_id,
        &observed_deps_id,
        observed_settings_id.as_ref(),
        observed_file_version,
    )
    .await;

    let mut degraded = if strict_stale_cached_items.is_none() && member_access_context {
        crate::handlers::handle_completion_v2_degraded(
            file_content,
            file_path,
            parse_result,
            member_access_owner_type_hint,
            deps,
            position,
            uri,
            index_snapshot,
            snippet_support,
            include_flow_sensitive,
            trigger_char_hint,
        )
        .await
    } else {
        None
    };

    if let Some(response) = degraded.as_mut() {
        if let CompletionResponse::List(list) = &mut response.response {
            list.is_incomplete = true;
        }
    }

    let decision = bsl_runtime::application::completion_missing_ir_policy_decision(
        strict_stale_cached_items.is_some(),
        member_access_context,
        degraded.is_some(),
        relaxed_stale_cached_items.is_some(),
    );

    match decision {
        bsl_runtime::application::CompletionMissingIrPolicyDecision::StrictCacheIncomplete => (
            "degraded_incomplete",
            Some(completion_response_with_cached_items(
                strict_stale_cached_items.expect("strict cache decision requires strict items"),
            )),
        ),
        bsl_runtime::application::CompletionMissingIrPolicyDecision::EmptyForNonMemberAccess => {
            ("missing_ir", Some(completion_empty_response(false)))
        }
        bsl_runtime::application::CompletionMissingIrPolicyDecision::DegradedIncomplete => {
            ("degraded_incomplete", degraded)
        }
        bsl_runtime::application::CompletionMissingIrPolicyDecision::RelaxedCacheIncomplete => {
            server
                .coordinator
                .record_intellisense_v2_completion_stale_fallback();
            (
                "degraded_incomplete",
                Some(completion_response_with_cached_items(
                    relaxed_stale_cached_items
                        .expect("relaxed cache decision requires relaxed items"),
                )),
            )
        }
        bsl_runtime::application::CompletionMissingIrPolicyDecision::KeywordFallbackUnavailable => {
            server
                .coordinator
                .record_intellisense_v2_completion_fallback_unavailable();
            (
                "fallback_unavailable",
                Some(crate::handlers::build_keyword_degraded_completion(
                    snippet_support,
                )),
            )
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BslLanguageServer {
    // ========================================================================
    // LIFECYCLE METHODS
    // ========================================================================

    async fn initialize(&self, params: InitializeParams) -> JsonRpcResult<InitializeResult> {
        info!("Initializing BSL Language Server");

        // DEBUG: Log ClientCapabilities
        debug!(
            "[JSON-RPC] initialize: ClientCapabilities.window.workDoneProgress = {:?}",
            params
                .capabilities
                .window
                .as_ref()
                .and_then(|w| w.work_done_progress)
        );

        // MILESTONE 2.10: Read initializationOptions from Extension
        if let Some(options) = params.initialization_options {
            match serde_json::from_value::<LspConfig>(options.clone()) {
                Ok(config) => {
                    info!("LSP Config received: {:?}", config);
                    *self.config.write().await = Some(config.clone());
                    info!(
                        "Feature flags: enableTypeHints={:?}, enableCodeActions={:?}",
                        config.enable_type_hints, config.enable_code_actions
                    );
                    if let Some(cache_enabled) = config.cache_enabled {
                        let result = self.coordinator.set_cache_enabled(cache_enabled).await;
                        info!(
                            "Cache enabled updated: requested={}, effective={}, env_disabled={}",
                            result.requested, result.effective, result.env_disabled
                        );
                    }
                    if let Some(strict_fingerprint) = config.strict_fingerprint {
                        self.coordinator.set_strict_fingerprint(strict_fingerprint);
                        info!("Strict fingerprint updated: {}", strict_fingerprint);
                    }
                    info!("Configuration saved, will reload types in initialized()");
                }
                Err(e) => {
                    error!("Failed to parse LSP config: {}", e);
                    error!("Raw options: {:?}", options);
                }
            }
        } else {
            info!("No initializationOptions provided - using defaults (4 basic types only)");
        }

        let snippet_support = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.completion.as_ref())
            .and_then(|completion| completion.completion_item.as_ref())
            .and_then(|item| item.snippet_support)
            .unwrap_or(false);
        *self.completion_snippet_support.write().await = snippet_support;
        info!("Client snippet support: {}", snippet_support);

        let dynamic_document_formatting = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.formatting.as_ref())
            .and_then(|cap| cap.dynamic_registration)
            .unwrap_or(false);
        let dynamic_range_formatting = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.range_formatting.as_ref())
            .and_then(|cap| cap.dynamic_registration)
            .unwrap_or(false);

        let dynamic_inlay_hints = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.inlay_hint.as_ref())
            .and_then(|cap| cap.dynamic_registration)
            .unwrap_or(false);

        let dynamic_code_actions = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|td| td.code_action.as_ref())
            .and_then(|cap| cap.dynamic_registration)
            .unwrap_or(false);

        {
            let mut state = self.formatting_capability.write().await;
            state.dynamic_document_formatting = dynamic_document_formatting;
            state.dynamic_range_formatting = dynamic_range_formatting;
        }
        {
            let mut state = self.inlay_hints_capability.write().await;
            state.dynamic_registration = dynamic_inlay_hints;
        }
        {
            let mut state = self.code_actions_capability.write().await;
            state.dynamic_registration = dynamic_code_actions;
        }
        info!(
            "Client dynamicRegistration: formatting={}, rangeFormatting={}",
            dynamic_document_formatting, dynamic_range_formatting
        );
        info!(
            "Client dynamicRegistration: inlayHints={}, codeActions={}",
            dynamic_inlay_hints, dynamic_code_actions
        );

        // Version info for LSP Protocol
        let version = env!("CARGO_PKG_VERSION");
        let build_timestamp = env!("BUILD_TIMESTAMP");
        let git_hash = env!("GIT_HASH");

        let (enable_type_hints, enable_code_actions) = {
            let cfg = self.config.read().await;
            let enable_type_hints = cfg
                .as_ref()
                .and_then(|cfg| cfg.enable_type_hints)
                .unwrap_or(false);
            let enable_code_actions = cfg
                .as_ref()
                .and_then(|cfg| cfg.enable_code_actions)
                .unwrap_or(false);
            (enable_type_hints, enable_code_actions)
        };

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(true),
                    trigger_characters: Some(vec![".".to_string(), "(".to_string()]),
                    ..Default::default()
                }),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                diagnostic_provider: None,
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "bsl.getAllTypes".to_string(),
                        "bsl.getSemanticHtml".to_string(),
                        "bsl.getSemanticTree".to_string(),
                        "bsl.searchTypes".to_string(),
                        "bsl.getCurrentContext".to_string(),
                        "bsl.getTypeRepositoryStats".to_string(),
                        "bsl.getWorkspaceStats".to_string(),
                        "bsl.getObservabilityMetrics".to_string(),
                        "bsl.getRuntimeConfig".to_string(),
                        "bsl.parseConfiguration".to_string(),
                        "bsl.cache.getStats".to_string(),
                        "bsl.cache.clear".to_string(),
                        "bsl.cache.setEnabled".to_string(),
                    ],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: Some(vec![")".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                }),
                // Formatting is registered dynamically based on workspace settings.
                // This prevents VSCode formatOnSave from calling formatting when it's disabled.
                document_formatting_provider: None,
                document_range_formatting_provider: None,
                inlay_hint_provider: if dynamic_inlay_hints {
                    None
                } else {
                    enable_type_hints.then_some(OneOf::Left(true))
                },
                code_action_provider: if dynamic_code_actions {
                    None
                } else {
                    enable_code_actions.then_some(CodeActionProviderCapability::Simple(true))
                },
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "BSL Language Server".to_string(),
                version: Some(format!(
                    "{} (build: {}, git: {})",
                    version, build_timestamp, git_hash
                )),
            }),
        })
    }

    // TODO: Consider splitting initialized() into smaller functions in future refactoring
    // This method is 278 lines but handles complex async progress reporting that's hard to split
    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "BSL Language Server initialized!")
            .await;

        self.sync_formatting_capability_registration().await;
        self.sync_inlay_hints_capability_registration().await;
        self.sync_code_actions_capability_registration().await;

        // MILESTONE 2.10: Reload types with config from initializationOptions
        let config = self.config.read().await;
        if let Some(ref cfg) = *config {
            if let Some(ref platform_docs) = cfg.platform_docs_archive {
                info!(
                    "Reloading types with platformDocsArchive: {}",
                    platform_docs
                );

                // Create channels for progress and result
                let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();
                let (result_tx, result_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

                // Send bsl/serverStatus (loading: true)
                info!("[LSP->Extension] Sending bsl/serverStatus: loading=true");
                let _ = self
                    .client
                    .send_notification::<ServerStatus>(ServerStatusParams::loading(
                        "Loading types...",
                    ))
                    .await;

                // Send WorkDoneProgressBegin (единый progress bridge)
                let title = if cfg.configuration_path.is_some() {
                    "Loading platform and configuration types".to_string()
                } else {
                    "Loading platform types".to_string()
                };

                let mut reporter =
                    LspWorkDoneReporter::create(self.client.clone(), "bsl-load-types").await;
                reporter.set_throttle_interval(std::time::Duration::from_millis(150));
                reporter
                    .begin(title, Some("Initializing...".to_string()))
                    .await;

                log_progress_to_file("[LSP->Extension] SEND WorkDoneProgressBegin");

                // Spawn task to handle progress
                let client_clone = self.client.clone();
                let start_time = std::time::Instant::now();
                let self_clone = self.clone();

                tokio::spawn(async move {
                    let mut reporter = reporter;

                    // PHASE 1: Process progress updates
                    while let Some(update) = progress_rx.recv().await {
                        debug!(
                            "[RECV] {:?} {:.1}% ({}/{}) - {}",
                            update.phase,
                            update.percentage,
                            update.current,
                            update.total,
                            update.message.as_deref().unwrap_or("")
                        );

                        // Calculate ETA
                        let elapsed = start_time.elapsed().as_secs_f32();
                        let eta = if update.percentage > 5.0 {
                            Some(((elapsed * 100.0 / update.percentage) - elapsed) as u32)
                        } else {
                            None
                        };

                        // Format message
                        let message = match update.phase {
                            IndexingPhase::ParsingFiles => {
                                format!(
                                    "Type {}/{}{}",
                                    update.current,
                                    update.total,
                                    update
                                        .message
                                        .as_ref()
                                        .map(|m| format!(" - {}", m))
                                        .unwrap_or_default()
                                )
                            }
                            IndexingPhase::ConfigurationParsing => {
                                update.message.clone().unwrap_or_else(|| {
                                    format!(
                                        "{} | {}/{}",
                                        update.phase.display_name(),
                                        update.current,
                                        update.total
                                    )
                                })
                            }
                            _ => update.message.clone().unwrap_or_else(|| {
                                format!(
                                    "{} | {}/{}",
                                    update.phase.display_name(),
                                    update.current,
                                    update.total
                                )
                            }),
                        };

                        let message_with_eta = if let Some(eta_secs) = eta {
                            format!("{} - ETA: {}s", message, eta_secs)
                        } else {
                            message
                        };

                        reporter
                            .report(update.percentage as u32, Some(message_with_eta))
                            .await;
                    }

                    // PHASE 2: Channel closed, wait for result
                    match result_rx.await {
                        Ok(Ok(())) => {
                            // SUCCESS: Send WorkDoneProgressEnd
                            reporter
                                .end(Some("Platform types loaded successfully".to_string()))
                                .await;

                            let _ = client_clone
                                .send_notification::<ServerStatus>(ServerStatusParams::ready())
                                .await;

                            // Reschedule diagnostics for open documents so they are recomputed
                            // against the latest deps snapshot.
                            info!("Rescheduling v2 diagnostics for open documents after deps update...");
                            let open_versions: Vec<(bsl_analysis_v2::FileId, i32)> = {
                                self_clone
                                    .latest_received_file_versions_v2
                                    .read()
                                    .await
                                    .iter()
                                    .map(|(file_id, version)| (*file_id, *version))
                                    .collect()
                            };
                            let keys = self_clone.file_key_to_file_id_v2.read().await.clone();

                            for (file_id, version) in open_versions {
                                let uri = keys.iter().find_map(|(key, mapped)| {
                                    if *mapped != file_id {
                                        return None;
                                    }
                                    match key {
                                        super::V2FileKey::Path(path) => {
                                            Url::from_file_path(path).ok()
                                        }
                                        super::V2FileKey::Url(raw) => Url::parse(raw).ok(),
                                    }
                                });

                                if let Some(uri) = uri {
                                    let diagnostics_generation =
                                        self_clone.bump_diagnostics_generation_v2(file_id).await;
                                    for profile in
                                        bsl_runtime::application::diagnostics_profiles_for_trigger(
                                            bsl_runtime::application::DiagnosticsTrigger::DidOpen,
                                        )
                                    {
                                        self_clone
                                            .schedule_diagnostics_profile_v2(
                                                uri.clone(),
                                                file_id,
                                                version,
                                                diagnostics_generation,
                                                bsl_runtime::application::DiagnosticsTrigger::DidOpen,
                                                *profile,
                                                true,
                                            )
                                            .await;
                                    }
                                }
                            }
                        }
                        Ok(Err(error_msg)) => {
                            // ERROR: Send WorkDoneProgressEnd with error
                            reporter.end(Some(format!("Error: {}", error_msg))).await;

                            let _ = client_clone
                                .send_notification::<ServerStatus>(ServerStatusParams::ready())
                                .await;
                        }
                        Err(_) => {
                            warn!("Result channel closed unexpectedly");
                        }
                    }
                });

                // Load types
                let inputs = StartupInputs::from_lsp_settings(
                    Some(platform_docs),
                    cfg.configuration_path.as_deref(),
                    cfg.platform_version.as_deref(),
                    cfg.cache_enabled,
                    cfg.strict_fingerprint,
                );

                let result = startup_v2(self.coordinator.clone(), inputs, Some(progress_tx)).await;

                match result {
                    Ok(startup) => {
                        info!("Platform types loaded successfully");
                        self.apply_deps_bundle_v2("start_with_paths", startup.deps_bundle_v2)
                            .await;
                        self.sync_v2_globals().await;
                        let _ = result_tx.send(Ok(()));
                        self.client
                            .log_message(
                                MessageType::INFO,
                                format!("Platform documentation loaded from: {}", platform_docs),
                            )
                            .await;
                    }
                    Err(e) => {
                        error!("Failed to load platform types: {}", e);
                        let _ = result_tx.send(Err(e.to_string()));
                        self.client
                            .log_message(
                                MessageType::ERROR,
                                format!("Failed to load platform documentation: {}", e),
                            )
                            .await;
                    }
                }
            } else {
                info!("platformDocsArchive not provided - using basic types only");
            }
        }
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        info!("Shutting down BSL Language Server");
        Ok(())
    }

    // ========================================================================
    // CONFIGURATION
    // ========================================================================

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        info!("Received didChangeConfiguration");

        if let Some(settings_value) = params.settings.as_object() {
            if let Some(bsl_analyzer_value) = settings_value.get("bslAnalyzer") {
                match serde_json::from_value::<LspConfig>(bsl_analyzer_value.clone()) {
                    Ok(mut new_config) => {
                        normalize_lsp_config(&mut new_config);
                        let mut guard = self.config.write().await;
                        let mut merged = guard.clone().unwrap_or(LspConfig {
                            platform_docs_archive: None,
                            configuration_path: None,
                            platform_version: None,
                            cache_enabled: None,
                            strict_fingerprint: None,
                            enable_type_hints: None,
                            enable_code_actions: None,
                        });
                        if new_config.platform_docs_archive.is_some() {
                            merged.platform_docs_archive = new_config.platform_docs_archive;
                        }
                        if new_config.configuration_path.is_some() {
                            merged.configuration_path = new_config.configuration_path;
                        }
                        if new_config.platform_version.is_some() {
                            merged.platform_version = new_config.platform_version;
                        }
                        if new_config.cache_enabled.is_some() {
                            merged.cache_enabled = new_config.cache_enabled;
                        }
                        if new_config.strict_fingerprint.is_some() {
                            merged.strict_fingerprint = new_config.strict_fingerprint;
                        }
                        if new_config.enable_type_hints.is_some() {
                            merged.enable_type_hints = new_config.enable_type_hints;
                        }
                        if new_config.enable_code_actions.is_some() {
                            merged.enable_code_actions = new_config.enable_code_actions;
                        }
                        *guard = Some(merged.clone());
                        if let Some(cache_enabled) = merged.cache_enabled {
                            let result = self.coordinator.set_cache_enabled(cache_enabled).await;
                            info!(
                                "Cache enabled updated via settings: requested={}, effective={}, env_disabled={}",
                                result.requested, result.effective, result.env_disabled
                            );
                        }
                        if let Some(strict_fingerprint) = merged.strict_fingerprint {
                            self.coordinator.set_strict_fingerprint(strict_fingerprint);
                            info!(
                                "Strict fingerprint updated via settings: {}",
                                strict_fingerprint
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse BslAnalyzer settings: {}", e);
                    }
                }
            }
            if let Some(bsl_value) = settings_value.get("bsl") {
                match serde_json::from_value::<BslSettings>(bsl_value.clone()) {
                    Ok(new_settings) => {
                        info!(
                            "Parsed BslSettings: hover.detailLevel={}, diagnostics.detailLevel={}, formatting.enabled={}, formatting.indentSize={}, typeHints.enabled={}, codeActions.enabled={}, enableFlowSensitive={}",
                            new_settings.hover.detail_level,
                            new_settings.diagnostics.detail_level,
                            new_settings.formatting.enabled,
                            new_settings.formatting.indent_size,
                            new_settings.type_hints.enabled,
                            new_settings.code_actions.enabled,
                            new_settings.enable_flow_sensitive,
                        );

                        // Apply runtime `BSL_*` overrides (stable + dev-only) without restarting the server.
                        // Stable overrides are always accepted; dev-only overrides require explicit opt-in.
                        {
                            let store = bsl_runtime::system::global_runtime_config();
                            let stable_report =
                                store.replace_stable_overrides(&new_settings.env_overrides);
                            if !stable_report.ignored_unknown_keys.is_empty()
                                || !stable_report.ignored_invalid_values.is_empty()
                                || !stable_report.ignored_wrong_tier_keys.is_empty()
                            {
                                warn!(
                                    "RuntimeConfig stable overrides: unknown={:?}, invalid={:?}, wrong_tier={:?}",
                                    stable_report.ignored_unknown_keys,
                                    stable_report.ignored_invalid_values,
                                    stable_report.ignored_wrong_tier_keys,
                                );
                            }

                            let dev_report = store.replace_dev_overrides(
                                &new_settings.dev_env_overrides,
                                new_settings.enable_dev_env_overrides(),
                            );
                            if dev_report.dev_overrides_ignored {
                                warn!(
                                    "RuntimeConfig dev-only overrides ignored (set bsl.allowDevOverrides=true or legacy bsl.dev.enableDevEnvOverrides=true to apply)."
                                );
                            } else if !dev_report.ignored_unknown_keys.is_empty()
                                || !dev_report.ignored_invalid_values.is_empty()
                                || !dev_report.ignored_wrong_tier_keys.is_empty()
                            {
                                warn!(
                                    "RuntimeConfig dev overrides: unknown={:?}, invalid={:?}, wrong_tier={:?}",
                                    dev_report.ignored_unknown_keys,
                                    dev_report.ignored_invalid_values,
                                    dev_report.ignored_wrong_tier_keys,
                                );
                            }
                        }

                        *self.settings.write().await = new_settings;

                        // Keep feature gates (initializationOptions.*) aligned with runtime settings to
                        // avoid "enabled in settings but server refuses" situations.
                        {
                            let mut guard = self.config.write().await;
                            let mut merged = guard.clone().unwrap_or(LspConfig {
                                platform_docs_archive: None,
                                configuration_path: None,
                                platform_version: None,
                                cache_enabled: None,
                                strict_fingerprint: None,
                                enable_type_hints: None,
                                enable_code_actions: None,
                            });
                            let settings = self.settings.read().await;
                            merged.enable_type_hints = Some(settings.type_hints.enabled);
                            merged.enable_code_actions = Some(settings.code_actions.enabled);
                            *guard = Some(merged);
                        }

                        self.sync_formatting_capability_registration().await;
                        self.sync_inlay_hints_capability_registration().await;
                        self.sync_code_actions_capability_registration().await;

                        // Re-sync cache/strict-fingerprint toggles via coordinator to reflect runtime-config
                        // changes (e.g., `BSL_CACHE_DISABLE`, `BSL_CACHE_STRICT_FINGERPRINT`) without restart.
                        {
                            let cache_disable = bsl_runtime::system::global_runtime_config()
                                .get_bool(bsl_runtime::system::RuntimeKey::CacheDisable)
                                .unwrap_or(false);
                            let requested_cache_enabled = self
                                .config
                                .read()
                                .await
                                .as_ref()
                                .and_then(|cfg| cfg.cache_enabled)
                                .unwrap_or(true);
                            let _ = self
                                .coordinator
                                .set_cache_enabled(requested_cache_enabled && !cache_disable)
                                .await;

                            let strict = bsl_runtime::system::global_runtime_config()
                                .get_bool(bsl_runtime::system::RuntimeKey::CacheStrictFingerprint)
                                .unwrap_or(false);
                            self.coordinator.set_strict_fingerprint(strict);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse BslSettings: {}", e);
                    }
                }
            }
        }

        self.sync_v2_globals().await;
    }

    // ========================================================================
    // FILE MANAGEMENT
    // ========================================================================

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;

        let _sync_guard = self.text_sync_v2.lock().await;

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let completion_knobs =
            bsl_runtime::application::CompletionPipelineKnobs::from_runtime_config();
        self.completion_dispatcher_v2
            .set_queue_capacity(completion_knobs.queue_capacity)
            .await;
        let completion_mode = completion_knobs.mode;
        if completion_dispatch_enabled_for_mode(completion_mode) {
            let open_ticket = self
                .completion_dispatcher_v2
                .emit_did_open(file_id, version)
                .await;
            if completion_queue_enqueue_failed(open_ticket.queue_outcome) {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    file_seq = open_ticket.file_seq,
                    request_epoch = open_ticket.request_epoch,
                    queue_outcome = ?open_ticket.queue_outcome,
                    "completion dispatcher dropped didOpen event"
                );
            }
        }
        let path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => uri.to_string(),
        };
        let text: Arc<str> = Arc::from(text);
        let path: Arc<str> = Arc::from(path);
        let parse_snapshot = self
            .coordinator
            .parser_coordinator()
            .and_then(|parser| {
                let parse_started = Instant::now();
                let report = parser
                    .parse_incremental_with_report(
                        PathBuf::from(path.as_ref()),
                        text.to_string(),
                        Vec::new(),
                    )
                    .ok()?;
                Some((report, parse_started.elapsed()))
            })
            .map(|(report, parse_elapsed)| {
                let mode = if report.incremental {
                    if report.changed_ranges.is_empty() {
                        "reused"
                    } else {
                        "incremental"
                    }
                } else {
                    "full"
                };
                let changed_ranges_count = report.changed_ranges.len();
                let changed_ranges_bytes: usize = report
                    .changed_ranges
                    .iter()
                    .map(changed_range_footprint_bytes)
                    .sum();
                self.coordinator.record_intellisense_v2_parse_snapshot(
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    mode,
                    changed_ranges_count,
                    changed_ranges_bytes,
                    report.fallback_reason.as_deref(),
                    parse_elapsed,
                );
                bsl_analysis_v2::ParseSnapshot {
                    file_id,
                    file_version: version,
                    parse_result: Arc::new(report.parse_result),
                    line_index: report.line_index,
                    backend_tree: report.backend_tree,
                    changed_ranges: Arc::new(
                        report
                            .changed_ranges
                            .into_iter()
                            .map(|range| bsl_analysis_v2::ParseChangedRange {
                                start_byte: range.start_byte,
                                old_end_byte: range.old_end_byte,
                                new_end_byte: range.new_end_byte,
                            })
                            .collect(),
                    ),
                    produced_at_millis: unix_time_millis(),
                    backend_tree_hash: report.backend_tree_hash,
                    incremental: report.incremental,
                    fallback_reason: report.fallback_reason.map(Arc::from),
                }
            });

        self.latest_received_file_versions_v2
            .write()
            .await
            .insert(file_id, version);
        self.latest_document_shadow_state_v2.write().await.insert(
            file_id,
            super::DocumentShadowStateV2 {
                version,
                text: text.clone(),
            },
        );

        self.analysis_v2
            .apply_changes(vec![if let Some(parse_snapshot) = parse_snapshot {
                bsl_analysis_v2::Change::SetFileWithSnapshot {
                    file_id,
                    text,
                    version,
                    path,
                    parse_snapshot,
                }
            } else {
                bsl_analysis_v2::Change::SetFile {
                    file_id,
                    text,
                    version,
                    path,
                }
            }]);

        let diagnostics_generation = self.bump_diagnostics_generation_v2(file_id).await;
        for profile in bsl_runtime::application::diagnostics_profiles_for_trigger(
            bsl_runtime::application::DiagnosticsTrigger::DidOpen,
        ) {
            self.schedule_diagnostics_profile_v2(
                uri.clone(),
                file_id,
                version,
                diagnostics_generation,
                bsl_runtime::application::DiagnosticsTrigger::DidOpen,
                *profile,
                false,
            )
            .await;
        }

        self.client
            .log_message(
                MessageType::INFO,
                format!("Opened document (v2 diagnostics scheduled): {}", uri),
            )
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let changes = params.content_changes;

        let _sync_guard = self.text_sync_v2.lock().await;

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let completion_knobs =
            bsl_runtime::application::CompletionPipelineKnobs::from_runtime_config();
        self.completion_dispatcher_v2
            .set_queue_capacity(completion_knobs.queue_capacity)
            .await;
        let completion_mode = completion_knobs.mode;
        if completion_dispatch_enabled_for_mode(completion_mode) {
            let change_ticket = self
                .completion_dispatcher_v2
                .emit_did_change(file_id, version)
                .await;
            if completion_queue_enqueue_failed(change_ticket.queue_outcome) {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    file_seq = change_ticket.file_seq,
                    request_epoch = change_ticket.request_epoch,
                    queue_outcome = ?change_ticket.queue_outcome,
                    "completion dispatcher dropped didChange event"
                );
            }
        }
        let path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => uri.to_string(),
        };

        // Apply changes
        let (updated_text, parser_edits) =
            if let Some(full_change) = changes.iter().find(|c| c.range.is_none()) {
                (full_change.text.clone(), Vec::new())
            } else {
                let shadow_state = {
                    let shadow = self.latest_document_shadow_state_v2.read().await;
                    shadow.get(&file_id).cloned()
                };
                if let Some(state) = shadow_state.as_ref() {
                    if version < state.version {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            requested_version = version,
                            shadow_version = state.version,
                            "Skipping out-of-order didChange for older version"
                        );
                        return;
                    }
                }
                let base_text = if let Some(state) = shadow_state {
                    state.text.to_string()
                } else {
                    self.analysis_v2
                        .snapshot()
                        .await
                        .file_text(file_id)
                        .ok()
                        .flatten()
                        .map(|text| text.to_string())
                        .unwrap_or_default()
                };

                let mut current_text = base_text;
                let mut parser_edits = Vec::new();
                for change in &changes {
                    if let Some(range) = change.range {
                        if let Some(edit) = lsp_range_change_to_parser_edit(change) {
                            parser_edits.push(edit);
                        }
                        current_text = apply_text_edit(&current_text, range, &change.text);
                    }
                }
                (current_text, parser_edits)
            };

        let scale_aware_knobs =
            bsl_runtime::application::ScaleAwareDiagnosticsKnobs::from_runtime_config();
        let mut large_churn_active = false;
        if scale_aware_knobs.enabled {
            let is_large_document = bsl_runtime::application::scale_aware_document_is_large(
                &updated_text,
                scale_aware_knobs,
            );
            let now = Instant::now();
            let transition = {
                let mut churn_state = self.scale_aware_churn_state_v2.write().await;
                let state = churn_state
                    .entry(file_id)
                    .or_insert(super::ScaleAwareChurnStateV2 {
                        window_started_at: now,
                        changes_in_window: 0,
                        large_churn_active: false,
                    });
                let transition =
                    advance_large_churn_state(state, now, is_large_document, scale_aware_knobs);
                large_churn_active = state.large_churn_active;
                transition
            };
            match transition {
                LargeChurnTransition::Entered => self
                    .coordinator
                    .record_intellisense_v2_large_churn_transition(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "enter",
                    ),
                LargeChurnTransition::Exited => self
                    .coordinator
                    .record_intellisense_v2_large_churn_transition(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "exit",
                    ),
                LargeChurnTransition::None => {}
            }
        } else {
            let was_active = self
                .scale_aware_churn_state_v2
                .write()
                .await
                .remove(&file_id)
                .is_some_and(|state| state.large_churn_active);
            if was_active {
                self.coordinator
                    .record_intellisense_v2_large_churn_transition(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "exit",
                    );
            }
        }

        self.latest_received_file_versions_v2
            .write()
            .await
            .insert(file_id, version);
        let updated_text: Arc<str> = Arc::from(updated_text);
        let path: Arc<str> = Arc::from(path);
        self.latest_document_shadow_state_v2.write().await.insert(
            file_id,
            super::DocumentShadowStateV2 {
                version,
                text: updated_text.clone(),
            },
        );
        let parse_snapshot = self
            .coordinator
            .parser_coordinator()
            .and_then(|parser| {
                let parse_started = Instant::now();
                let report = parser
                    .parse_incremental_with_report(
                        PathBuf::from(path.as_ref()),
                        updated_text.to_string(),
                        parser_edits,
                    )
                    .ok()?;
                Some((report, parse_started.elapsed()))
            })
            .map(|(report, parse_elapsed)| {
                let mode = if report.incremental {
                    if report.changed_ranges.is_empty() {
                        "reused"
                    } else {
                        "incremental"
                    }
                } else {
                    "full"
                };
                let changed_ranges_count = report.changed_ranges.len();
                let changed_ranges_bytes: usize = report
                    .changed_ranges
                    .iter()
                    .map(changed_range_footprint_bytes)
                    .sum();
                self.coordinator.record_intellisense_v2_parse_snapshot(
                    bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                    mode,
                    changed_ranges_count,
                    changed_ranges_bytes,
                    report.fallback_reason.as_deref(),
                    parse_elapsed,
                );
                bsl_analysis_v2::ParseSnapshot {
                    file_id,
                    file_version: version,
                    parse_result: Arc::new(report.parse_result),
                    line_index: report.line_index,
                    backend_tree: report.backend_tree,
                    changed_ranges: Arc::new(
                        report
                            .changed_ranges
                            .into_iter()
                            .map(|range| bsl_analysis_v2::ParseChangedRange {
                                start_byte: range.start_byte,
                                old_end_byte: range.old_end_byte,
                                new_end_byte: range.new_end_byte,
                            })
                            .collect(),
                    ),
                    produced_at_millis: unix_time_millis(),
                    backend_tree_hash: report.backend_tree_hash,
                    incremental: report.incremental,
                    fallback_reason: report.fallback_reason.map(Arc::from),
                }
            });
        self.analysis_v2
            .apply_changes(vec![if let Some(parse_snapshot) = parse_snapshot {
                bsl_analysis_v2::Change::SetFileWithSnapshot {
                    file_id,
                    text: updated_text,
                    version,
                    path,
                    parse_snapshot,
                }
            } else {
                bsl_analysis_v2::Change::SetFile {
                    file_id,
                    text: updated_text,
                    version,
                    path,
                }
            }]);

        let flow_sensitive_enabled = {
            let settings = self.settings.read().await;
            settings.enable_flow_sensitive
        };
        let diagnostics_generation = self.bump_diagnostics_generation_v2(file_id).await;
        for profile in bsl_runtime::application::diagnostics_profiles_for_trigger(
            bsl_runtime::application::DiagnosticsTrigger::DidChange,
        ) {
            if !should_schedule_profile(
                bsl_runtime::application::DiagnosticsTrigger::DidChange,
                *profile,
                flow_sensitive_enabled,
            ) {
                continue;
            }
            if should_defer_heavy_diagnostics_for_large_churn(
                bsl_runtime::application::DiagnosticsTrigger::DidChange,
                *profile,
                large_churn_active,
            ) {
                self.coordinator
                    .record_intellisense_v2_heavy_diagnostics_deferred(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        profile.as_str(),
                        bsl_runtime::application::DeferredHeavyDiagnosticsReason::LargeAndChurn
                            .as_str(),
                    );
                self.schedule_diagnostics_profile_v2(
                    uri.clone(),
                    file_id,
                    version,
                    diagnostics_generation,
                    bsl_runtime::application::DiagnosticsTrigger::Idle,
                    *profile,
                    true,
                )
                .await;
                continue;
            }
            match profile {
                bsl_runtime::application::DiagnosticsProfile::Fast => {
                    self.run_diagnostics_profile_immediate_v2(
                        uri.clone(),
                        file_id,
                        version,
                        diagnostics_generation,
                        bsl_runtime::application::DiagnosticsTrigger::DidChange,
                        *profile,
                    )
                    .await;
                }
                _ => {
                    let trigger = match profile {
                        bsl_runtime::application::DiagnosticsProfile::IdleHeavy => {
                            bsl_runtime::application::DiagnosticsTrigger::Idle
                        }
                        _ => bsl_runtime::application::DiagnosticsTrigger::DidChange,
                    };
                    self.schedule_diagnostics_profile_v2(
                        uri.clone(),
                        file_id,
                        version,
                        diagnostics_generation,
                        trigger,
                        *profile,
                        true,
                    )
                    .await;
                }
            }
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let Some(file_id) = self.get_file_id_v2(&uri).await else {
            return;
        };
        let Some(version) = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
        else {
            return;
        };

        let flow_sensitive_enabled = {
            let settings = self.settings.read().await;
            settings.enable_flow_sensitive
        };
        let diagnostics_generation = self.bump_diagnostics_generation_v2(file_id).await;
        for profile in bsl_runtime::application::diagnostics_profiles_for_trigger(
            bsl_runtime::application::DiagnosticsTrigger::DidSave,
        ) {
            if !should_schedule_profile(
                bsl_runtime::application::DiagnosticsTrigger::DidSave,
                *profile,
                flow_sensitive_enabled,
            ) {
                continue;
            }
            self.schedule_diagnostics_profile_v2(
                uri.clone(),
                file_id,
                version,
                diagnostics_generation,
                bsl_runtime::application::DiagnosticsTrigger::DidSave,
                *profile,
                false,
            )
            .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        let _sync_guard = self.text_sync_v2.lock().await;

        if let Some(file_id) = self.get_file_id_v2(&uri).await {
            let close_ticket = self
                .completion_dispatcher_v2
                .close_file_dispatcher(file_id)
                .await;
            if close_ticket
                .map(|ticket| completion_queue_enqueue_failed(ticket.queue_outcome))
                .unwrap_or(false)
            {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    file_seq = ?close_ticket.map(|ticket| ticket.file_seq),
                    request_epoch = ?close_ticket.map(|ticket| ticket.request_epoch),
                    queue_outcome = ?close_ticket.map(|ticket| ticket.queue_outcome),
                    "completion dispatcher dropped didClose event"
                );
            }
            let removed_completion_cancellations = self
                .completion_cancellation_registry_v2
                .remove_file(file_id);
            if removed_completion_cancellations > 0 {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    removed_completion_cancellations,
                    "completion cancellation registry cleanup on didClose"
                );
            }
            self.cancel_diagnostics_v2(file_id).await;
            self.latest_received_file_versions_v2
                .write()
                .await
                .remove(&file_id);
            self.latest_document_shadow_state_v2
                .write()
                .await
                .remove(&file_id);
            self.completion_stale_fallback_cache_v2
                .write()
                .await
                .remove(&file_id);
            self.completion_parity_state_v2
                .write()
                .await
                .retain(|(tracked_file_id, _, _, _), _| *tracked_file_id != file_id);
            self.diagnostics_generation_v2
                .write()
                .await
                .remove(&file_id);
            let had_large_churn = self
                .scale_aware_churn_state_v2
                .write()
                .await
                .remove(&file_id)
                .is_some_and(|state| state.large_churn_active);
            if had_large_churn {
                self.coordinator
                    .record_intellisense_v2_large_churn_transition(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "exit",
                    );
            }
            self.analysis_v2
                .apply_changes(vec![bsl_analysis_v2::Change::RemoveFile { file_id }]);
        }

        // Clear diagnostics
        self.client
            .publish_diagnostics(uri.clone(), vec![], None)
            .await;
        self.update_diagnostics_count(&uri, 0).await;

        self.client
            .log_message(MessageType::INFO, format!("Closed document: {}", uri))
            .await;
    }

    // ========================================================================
    // LSP FEATURES
    // ========================================================================

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> JsonRpcResult<Option<Vec<TextEdit>>> {
        let settings = self.settings.read().await.clone();
        if !settings.formatting.enabled {
            return Ok(None);
        }

        self.sync_v2_globals().await;
        let uri = params.text_document.uri;
        let file_id = self.get_or_create_file_id_v2(&uri).await;

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();

        if let Some(expected_version) = expected_version {
            let ok = self
                .analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await;
            if !ok {
                return Ok(None);
            }
        }

        let analysis = self.analysis_v2.snapshot().await;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };

        let edits = format_bsl_to_edits(&file_content, settings.formatting.indent_size)
            .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
        Ok(edits)
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> JsonRpcResult<Option<Vec<TextEdit>>> {
        let settings = self.settings.read().await.clone();
        if !settings.formatting.enabled {
            return Ok(None);
        }

        self.sync_v2_globals().await;
        let uri = params.text_document.uri;
        let file_id = self.get_or_create_file_id_v2(&uri).await;

        let expected_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();

        if let Some(expected_version) = expected_version {
            let ok = self
                .analysis_v2
                .wait_for_file_version(file_id, expected_version)
                .await;
            if !ok {
                return Ok(None);
            }
        }

        let analysis = self.analysis_v2.snapshot().await;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };

        let edits =
            format_bsl_range_to_edits(&file_content, settings.formatting.indent_size, params.range)
                .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
        Ok(edits)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> JsonRpcResult<Option<DocumentSymbolResponse>> {
        self.sync_v2_globals().await;

        let uri = params.text_document.uri;
        let Some(file_id) = self.get_file_id_v2(&uri).await else {
            return Ok(None);
        };

        let prepared = self
            .prepare_lsp_stateful_operation_v2(
                &uri,
                file_id,
                bsl_runtime::application::SemanticOperation::DocumentSymbol,
                false,
            )
            .await;
        let (context, prepared, _expected_version) = match prepared {
            Ok(values) => values,
            Err(_) => return Ok(None),
        };
        let analysis = prepared.snapshot.analysis;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let parse_result_query =
            bsl_runtime::application::IntellisenseV2Facade::run_parse_result_query_singleflight(
                &context,
                &analysis,
                true,
                Some(self.coordinator.as_ref()),
                file_id,
            );
        let Some(parse_result) = parse_result_query.ok().flatten() else {
            return Ok(None);
        };

        let response = build_document_symbols(&uri, &file_content, &parse_result)
            .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
        Ok(Some(response))
    }

    async fn references(&self, params: ReferenceParams) -> JsonRpcResult<Option<Vec<Location>>> {
        self.sync_v2_globals().await;

        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        let Some(file_id) = self.get_file_id_v2(&uri).await else {
            return Ok(None);
        };

        let prepared = self
            .prepare_lsp_stateful_operation_v2(
                &uri,
                file_id,
                bsl_runtime::application::SemanticOperation::References,
                false,
            )
            .await;
        let (context, prepared, _expected_version) = match prepared {
            Ok(values) => values,
            Err(_) => return Ok(None),
        };
        let analysis = prepared.snapshot.analysis;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let parse_result_query =
            bsl_runtime::application::IntellisenseV2Facade::run_parse_result_query_singleflight(
                &context,
                &analysis,
                true,
                Some(self.coordinator.as_ref()),
                file_id,
            );
        let Some(parse_result) = parse_result_query.ok().flatten() else {
            return Ok(None);
        };

        Ok(handle_references(
            &file_content,
            &parse_result,
            &uri,
            position,
            include_declaration,
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> JsonRpcResult<Option<PrepareRenameResponse>> {
        self.sync_v2_globals().await;

        let uri = params.text_document.uri.clone();
        let Some(file_id) = self.get_file_id_v2(&uri).await else {
            return Ok(None);
        };

        let prepared = self
            .prepare_lsp_stateful_operation_v2(
                &uri,
                file_id,
                bsl_runtime::application::SemanticOperation::Rename,
                false,
            )
            .await;
        let (context, prepared, _expected_version) = match prepared {
            Ok(values) => values,
            Err(_) => return Ok(None),
        };
        let analysis = prepared.snapshot.analysis;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let parse_result_query =
            bsl_runtime::application::IntellisenseV2Facade::run_parse_result_query_singleflight(
                &context,
                &analysis,
                true,
                Some(self.coordinator.as_ref()),
                file_id,
            );
        let Some(parse_result) = parse_result_query.ok().flatten() else {
            return Ok(None);
        };

        Ok(handle_prepare_rename(&file_content, &parse_result, params))
    }

    async fn rename(&self, params: RenameParams) -> JsonRpcResult<Option<WorkspaceEdit>> {
        self.sync_v2_globals().await;

        let uri = params.text_document_position.text_document.uri.clone();
        let Some(file_id) = self.get_file_id_v2(&uri).await else {
            return Ok(None);
        };

        let prepared = self
            .prepare_lsp_stateful_operation_v2(
                &uri,
                file_id,
                bsl_runtime::application::SemanticOperation::Rename,
                false,
            )
            .await;
        let (context, prepared, _expected_version) = match prepared {
            Ok(values) => values,
            Err(_) => return Ok(None),
        };
        let analysis = prepared.snapshot.analysis;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let parse_result_query =
            bsl_runtime::application::IntellisenseV2Facade::run_parse_result_query_singleflight(
                &context,
                &analysis,
                true,
                Some(self.coordinator.as_ref()),
                file_id,
            );
        let Some(parse_result) = parse_result_query.ok().flatten() else {
            return Ok(None);
        };

        match handle_rename(&file_content, &parse_result, params) {
            Ok(edit) => Ok(Some(edit)),
            Err(RenameError::InvalidNewName) => Err(tower_lsp::jsonrpc::Error::invalid_params(
                "Invalid new name",
            )),
            Err(RenameError::Unsupported) => Err(tower_lsp::jsonrpc::Error::invalid_params(
                "Rename is not supported for this symbol",
            )),
        }
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> JsonRpcResult<Option<Vec<SymbolInformation>>> {
        let query = params.query;
        if query.trim().is_empty() {
            return Ok(Some(Vec::new()));
        }

        self.sync_v2_globals().await;

        let open_file_ids: Vec<bsl_analysis_v2::FileId> = self
            .latest_received_file_versions_v2
            .read()
            .await
            .keys()
            .copied()
            .collect();

        if open_file_ids.is_empty() {
            return Ok(Some(Vec::new()));
        }

        let keys = self.file_key_to_file_id_v2.read().await.clone();
        let mut file_id_to_uri: std::collections::HashMap<bsl_analysis_v2::FileId, Url> =
            std::collections::HashMap::new();
        for (key, file_id) in keys {
            let uri = match key {
                super::V2FileKey::Path(path) => Url::from_file_path(path).ok(),
                super::V2FileKey::Url(raw) => Url::parse(&raw).ok(),
            };
            if let Some(uri) = uri {
                file_id_to_uri.insert(file_id, uri);
            }
        }

        let mut out: Vec<SymbolInformation> = Vec::new();
        for file_id in open_file_ids {
            let Some(uri) = file_id_to_uri.get(&file_id).cloned() else {
                continue;
            };
            let prepared = self
                .prepare_lsp_stateful_operation_v2(
                    &uri,
                    file_id,
                    bsl_runtime::application::SemanticOperation::SymbolSearch,
                    false,
                )
                .await;
            let (context, prepared, _expected_version) = match prepared {
                Ok(values) => values,
                Err(_) => continue,
            };
            let analysis = prepared.snapshot.analysis;
            let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
                continue;
            };
            let parse_result_query =
                bsl_runtime::application::IntellisenseV2Facade::run_parse_result_query_singleflight(
                    &context,
                    &analysis,
                    true,
                    Some(self.coordinator.as_ref()),
                    file_id,
                );
            let Some(parse_result) = parse_result_query.ok().flatten() else {
                continue;
            };
            out.extend(build_workspace_symbols(
                &query,
                &uri,
                &file_content,
                &parse_result,
            ));
        }

        out.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.location.uri.as_str().cmp(b.location.uri.as_str()))
                .then_with(|| {
                    a.location
                        .range
                        .start
                        .line
                        .cmp(&b.location.range.start.line)
                })
                .then_with(|| {
                    a.location
                        .range
                        .start
                        .character
                        .cmp(&b.location.range.start.character)
                })
        });

        const WORKSPACE_SYMBOL_LIMIT: usize = 200;
        if out.len() > WORKSPACE_SYMBOL_LIMIT {
            out.truncate(WORKSPACE_SYMBOL_LIMIT);
        }

        Ok(Some(out))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> JsonRpcResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;
        let trigger_mode = completion_trigger_mode_label(params.context.as_ref());
        let trigger_char_hint = completion_trigger_character(params.context.as_ref());
        let shadow_internal_request =
            completion_is_shadow_internal_request(params.context.as_ref());
        let completion_request_id = super::request_context::current_request_id()
            .or_else(|| super::request_context::take_completion_request_id(&uri, position));
        if !shadow_internal_request {
            self.coordinator
                .record_intellisense_v2_completion_trigger_mode(trigger_mode);
        }

        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let version_hint = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        let completion_knobs =
            bsl_runtime::application::CompletionPipelineKnobs::from_runtime_config();
        self.completion_dispatcher_v2
            .set_queue_capacity(completion_knobs.queue_capacity)
            .await;
        let routing_key = completion_canary_routing_key(
            &uri,
            position,
            trigger_mode,
            trigger_char_hint,
            version_hint,
        );
        let routing_plan = if shadow_internal_request {
            CompletionRoutingPlan {
                response_route: CompletionResponseRoute::EventDriven,
                run_shadow_event_driven: false,
            }
        } else {
            completion_routing_plan(
                completion_knobs.mode,
                completion_knobs.canary_percent,
                &routing_key,
            )
        };

        if routing_plan.run_shadow_event_driven {
            let mut shadow_params = params.clone();
            let shadow_trigger = completion_shadow_internal_trigger_value(trigger_char_hint);
            if let Some(context) = shadow_params.context.as_mut() {
                context.trigger_character = Some(shadow_trigger);
            } else {
                shadow_params.context = Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: Some(shadow_trigger),
                });
            }
            let shadow_server = self.clone();
            tokio::spawn(async move {
                let _ = shadow_server.completion(shadow_params).await;
            });
        }

        let event_driven_guards_enabled = routing_plan.response_route.event_driven_guards_enabled();
        let completion_observability_mode = completion_observability_mode_label(
            routing_plan.response_route,
            shadow_internal_request,
        );
        let started = Instant::now();
        let (
            completion_ticket,
            completion_turn_outcome,
            _completion_request_registration,
            completion_cancellation_token,
            mut completion_drop_guard,
        ) = if event_driven_guards_enabled {
            let completion_dispatch = self
                .completion_dispatcher_v2
                .emit_completion_request_with_turn(
                    file_id,
                    completion_request_id.clone(),
                    version_hint,
                    trigger_mode.to_string(),
                )
                .await;
            let completion_ticket = completion_dispatch.ticket;
            let completion_request_registration = completion_request_id.clone().map(|request_id| {
                self.completion_cancellation_registry_v2.register_request(
                    request_id,
                    file_id,
                    completion_ticket.request_epoch,
                )
            });
            let completion_cancellation_token = completion_request_registration
                .as_ref()
                .map(|registration| registration.token());
            let completion_drop_guard = Some(CompletionRequestDropCancelGuard::new(
                completion_request_id.clone(),
                Arc::clone(&self.completion_cancellation_registry_v2),
                Arc::clone(&self.completion_dispatcher_v2),
            ));
            if completion_queue_enqueue_failed(completion_ticket.queue_outcome) {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    file_seq = completion_ticket.file_seq,
                    request_epoch = completion_ticket.request_epoch,
                    request_id = ?completion_request_id,
                    queue_outcome = ?completion_ticket.queue_outcome,
                    "completion dispatcher dropped completion event"
                );
            }
            let completion_turn_outcome =
                if completion_queue_enqueue_failed(completion_ticket.queue_outcome) {
                    super::completion_dispatcher::CompletionTurnOutcome::QueueRejected
                } else if let Some(turn_waiter) = completion_dispatch.turn_waiter {
                    let turn_wait_started = Instant::now();
                    let turn_outcome = turn_waiter.wait().await;
                    self.coordinator
                        .record_completion_stage_latency("turn_wait", turn_wait_started.elapsed());
                    turn_outcome
                } else {
                    super::completion_dispatcher::CompletionTurnOutcome::QueueRejected
                };
            (
                completion_ticket,
                Some(completion_turn_outcome),
                completion_request_registration,
                completion_cancellation_token,
                completion_drop_guard,
            )
        } else {
            (
                super::completion_dispatcher::DispatchTicket {
                    file_seq: 0,
                    request_epoch: 0,
                    queue_outcome: super::completion_dispatcher::QueueEnqueueOutcome::Enqueued,
                },
                None,
                None,
                None,
                None,
            )
        };
        let snippet_support = *self.completion_snippet_support.read().await;
        #[cfg(test)]
        if let Some(delay_ms) = std::env::var("BSL_TEST_COMPLETION_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
        {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
        let mut completion_outcome: Option<&'static str> = None;
        let mut observed_file_version_for_completion: Option<i32> = None;
        let mut member_access_observed = false;
        let mut cancel_event_emitted = false;
        let mut completion = 'completion_flow: {
            if let Some(turn_outcome) = completion_turn_outcome {
                match turn_outcome {
                    super::completion_dispatcher::CompletionTurnOutcome::Ready => {}
                    super::completion_dispatcher::CompletionTurnOutcome::SupersededBeforeStart => {
                        completion_outcome = Some("superseded_epoch");
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                    super::completion_dispatcher::CompletionTurnOutcome::QueueRejected => {
                        completion_outcome = Some("queue_rejected");
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                }
            }
            let first_completion_for_file = {
                let mut seen = self.completion_seen_files_v2.write().await;
                seen.insert(file_id)
            };
            self.coordinator
                .record_intellisense_v2_completion_temperature(if first_completion_for_file {
                    "first"
                } else {
                    "warm"
                });
            let sync_globals_started = Instant::now();
            self.sync_v2_globals().await;
            self.coordinator
                .record_completion_stage_latency("sync_globals", sync_globals_started.elapsed());

            let empty = || Some(completion_empty_response(false));
            let extract_non_empty_items =
                |response: &crate::handlers::CompletionResponseWithStats| match &response.response {
                    CompletionResponse::List(list) if !list.items.is_empty() => {
                        Some(list.items.clone())
                    }
                    CompletionResponse::Array(items) if !items.is_empty() => Some(items.clone()),
                    _ => None,
                };
            let mut member_access_request = trigger_char_hint == Some('.');
            if !member_access_request {
                let shadow_text = {
                    let shadow = self.latest_document_shadow_state_v2.read().await;
                    shadow.get(&file_id).map(|state| state.text.clone())
                };
                if let Some(text) = shadow_text {
                    member_access_request = completion_request_targets_member_access(
                        text.as_ref(),
                        position,
                        trigger_char_hint,
                    );
                }
            }

            let include_flow_sensitive = {
                let settings = self.settings.read().await;
                settings.enable_flow_sensitive
            };

            let prepare_started = Instant::now();
            let prepared = self
                .prepare_lsp_stateful_operation_v2_with_completion_mode(
                    &uri,
                    file_id,
                    bsl_runtime::application::SemanticOperation::Completion,
                    include_flow_sensitive,
                    Some(completion_observability_mode),
                )
                .await;
            self.coordinator
                .record_completion_stage_latency("prepare_stateful", prepare_started.elapsed());

            match prepared {
                Ok((context, prepared, expected_version)) => {
                    let force_incomplete_due_stale = prepared.stale_served;
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "wait",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                    let (snapshot_file_bytes, snapshot_file_lines) = prepared
                        .snapshot
                        .analysis
                        .file_text(file_id)
                        .ok()
                        .flatten()
                        .map(|text| (text.len(), text.lines().count()))
                        .unwrap_or((0, 0));
                    self.coordinator
                        .record_intellisense_v2_payload_shape_with_origin(
                            context.origin.as_str(),
                            context.operation.as_str(),
                            bsl_runtime::application::ObservabilityStage::RuntimeSnapshotWithDeps
                                .as_str(),
                            snapshot_file_bytes,
                            snapshot_file_lines,
                        );
                    if let Some(wait_elapsed) = prepared.wait_elapsed {
                        if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                            if wait_elapsed >= threshold {
                                warn!(
                                    uri = %uri,
                                    file_id = file_id.0,
                                    expected_version,
                                    wait_ms = wait_elapsed.as_millis(),
                                    threshold_ms = threshold.as_millis(),
                                    "Completion v2: wait_for_file_version is slow"
                                );
                            }
                        }
                    }
                    if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
                        if prepared.snapshot_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                snapshot_ms = prepared.snapshot_elapsed.as_millis(),
                                file_bytes = snapshot_file_bytes,
                                file_lines = snapshot_file_lines,
                                threshold_ms = threshold.as_millis(),
                                "Completion v2: snapshot acquisition is slow"
                            );
                        }
                    }
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "snapshot",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }

                    if prepared.completion_churn_fastpath_active
                        && prepared.wait_budget_exhausted
                        && prepared.stale_served
                    {
                        let observed_deps_id = prepared.snapshot.deps_id.clone();
                        let observed_settings_id = prepared.snapshot.analysis.settings_id().ok();
                        let observed_file_version = prepared
                            .snapshot
                            .analysis
                            .file_version(file_id)
                            .ok()
                            .flatten();
                        observed_file_version_for_completion = observed_file_version;

                        let (strict_stale_cached_items, relaxed_stale_cached_items) =
                            completion_cached_stale_items(
                                self,
                                file_id,
                                &observed_deps_id,
                                observed_settings_id.as_ref(),
                                observed_file_version,
                            )
                            .await;
                        if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                            event_driven_guards_enabled,
                            self,
                            file_id,
                            completion_request_id.as_deref(),
                            completion_ticket.request_epoch,
                            completion_cancellation_token.as_ref(),
                            "collect",
                            &mut cancel_event_emitted,
                        )
                        .await
                        {
                            completion_outcome = Some(outcome);
                            break 'completion_flow Some(completion_incomplete_empty_response());
                        }

                        if let Some(items) =
                            strict_stale_cached_items.or(relaxed_stale_cached_items)
                        {
                            completion_outcome.get_or_insert("degraded_incomplete");
                            if !shadow_internal_request {
                                spawn_completion_refresh_after_stale_fastpath(
                                    self.clone(),
                                    params.clone(),
                                    trigger_char_hint,
                                );
                            }
                            break 'completion_flow Some(completion_response_with_cached_items(
                                items,
                            ));
                        }

                        if !shadow_internal_request {
                            spawn_completion_refresh_after_stale_fastpath(
                                self.clone(),
                                params.clone(),
                                trigger_char_hint,
                            );
                        }
                        self.coordinator
                            .record_intellisense_v2_completion_fallback_unavailable();
                        completion_outcome.get_or_insert("fallback_unavailable");
                        break 'completion_flow Some(
                            crate::handlers::build_keyword_degraded_completion(snippet_support),
                        );
                    }

                    let query_bundle_started = Instant::now();
                    let (
                        file_content,
                        file_path,
                        parse_result,
                        member_access_owner_type_hint,
                        deps,
                        ir_program,
                        index_snapshot,
                        observed_deps_id,
                        observed_settings_id,
                        observed_file_version,
                    ) = {
                        let analysis = prepared.snapshot.analysis;
                        let index_snapshot = prepared.snapshot.index_snapshot;
                        let parse_result_without_ir = member_access_request;
                        let member_access_request_for_query = member_access_request;

                        let observed_file_version = analysis.file_version(file_id).ok().flatten();
                        let observed_deps_id = prepared.snapshot.deps_id;
                        let observed_settings_id = analysis.settings_id().ok();
                        debug!(
                        "Completion v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
                            uri,
                            file_id.0,
                            observed_file_version,
                            Some(observed_deps_id.as_str()),
                            observed_settings_id.as_ref().map(|v| v.as_str()),
                            index_snapshot.id.as_str(),
                    );
                        match analysis.file_text_len(file_id) {
                            Ok(Some(len)) => debug!(
                                "Completion v2 (salsa) active: uri={}, file_id={}, text_len={}",
                                uri, file_id.0, len
                            ),
                            Ok(None) => debug!(
                                "Completion v2 (salsa) active: uri={}, file_id={} (file not found)",
                                uri, file_id.0
                            ),
                            Err(_) => debug!(
                                "Completion v2 (salsa) cancelled: uri={}, file_id={}",
                                uri, file_id.0
                            ),
                        }

                        let observed_byte_offset = analysis
                            .utf16_position_to_byte_offset(
                                file_id,
                                position.line,
                                position.character,
                            )
                            .ok()
                            .flatten();
                        let observed_point = analysis
                            .utf16_position_to_point(file_id, position.line, position.character)
                            .ok()
                            .flatten();
                        debug!(
                        "Completion v2 positioning: uri={}, file_id={}, lsp=({}:{}) -> byte_offset={:?}, point={:?}",
                        uri,
                        file_id.0,
                        position.line,
                        position.character,
                        observed_byte_offset,
                        observed_point,
                    );

                        let context_for_query = context.clone();
                        let coordinator_for_query = self.coordinator.clone();
                        let uri_for_query = uri.clone();
                        let observed_deps_id_for_query = observed_deps_id.clone();
                        let cancellation_token_for_query = completion_cancellation_token.clone();
                        let query_result =
                            bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                                bsl_runtime::application::CpuWorkClass::Interactive,
                                context_for_query.origin.as_str(),
                                Some(self.coordinator.as_ref()),
                                move || {
                                    let deps_and_file_snapshot_started = Instant::now();
                                    let file_content = analysis.file_text(file_id).ok().flatten();
                                    let file_path = analysis.file_path(file_id).ok().flatten();
                                    let deps = analysis.deps_data().ok();
                                    coordinator_for_query.record_completion_stage_latency(
                                        "query_bundle_deps_and_file_snapshot",
                                        deps_and_file_snapshot_started.elapsed(),
                                    );
                                    if cancellation_token_for_query
                                        .as_ref()
                                        .is_some_and(|token| token.is_cancelled())
                                    {
                                        coordinator_for_query
                                            .record_intellisense_v2_completion_owner_hint_result(
                                                "cancelled",
                                            );
                                        return (
                                            file_content,
                                            file_path,
                                            None,
                                            None,
                                            deps,
                                            None,
                                            false,
                                            true,
                                        );
                                    }

                                    let ir_started = Instant::now();
                                    let ir_query =
                                        bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                                            &context_for_query,
                                            &analysis,
                                            Some(coordinator_for_query.as_ref()),
                                            file_id,
                                        );
                                    let ir_elapsed = ir_started.elapsed();
                                    let ir_outcome =
                                        bsl_runtime::application::classify_optional_query(&ir_query);
                                    if let Some(threshold) =
                                        super::intellisense_v2_slow_query_warn_threshold()
                                    {
                                        if ir_elapsed >= threshold {
                                            warn!(
                                                uri = %uri_for_query,
                                                file_id = file_id.0,
                                                ir_ms = ir_elapsed.as_millis(),
                                                threshold_ms = threshold.as_millis(),
                                                "Completion v2: ir query is slow"
                                            );
                                        }
                                    }

                                    let (ir_program, ir_cancelled_after_retry) = match ir_query {
                                        Ok(program) => (program, false),
                                        Err(first_cancelled) => {
                                            // One fast retry mitigates transient cancellation races between
                                            // rapid didChange updates and completion query execution.
                                            let retry_started = Instant::now();
                                            let ir_retry =
                                                bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                                                    &context_for_query,
                                                    &analysis,
                                                    Some(coordinator_for_query.as_ref()),
                                                    file_id,
                                                );
                                            let retry_elapsed = retry_started.elapsed();
                                            if let Some(threshold) =
                                                super::intellisense_v2_slow_query_warn_threshold()
                                            {
                                                if retry_elapsed >= threshold {
                                                    warn!(
                                                        uri = %uri_for_query,
                                                        file_id = file_id.0,
                                                        ir_retry_ms = retry_elapsed.as_millis(),
                                                        threshold_ms = threshold.as_millis(),
                                                        "Completion v2: ir retry query is slow"
                                                    );
                                                }
                                            }
                                            match ir_retry {
                                                Ok(program) => {
                                                    debug!(
                                                        uri = %uri_for_query,
                                                        file_id = file_id.0,
                                                        "Completion v2: recovered from transient ir cancellation via retry"
                                                    );
                                                    (program, false)
                                                }
                                                Err(retry_cancelled) => {
                                                    debug!(
                                                        uri = %uri_for_query,
                                                        file_id = file_id.0,
                                                        first_error = ?first_cancelled,
                                                        retry_error = ?retry_cancelled,
                                                        ir_outcome = ir_outcome.as_str(),
                                                        "Completion v2: ir query cancelled after retry"
                                                    );
                                                    (None, true)
                                                }
                                            }
                                        }
                                    };
                                    if cancellation_token_for_query
                                        .as_ref()
                                        .is_some_and(|token| token.is_cancelled())
                                    {
                                        coordinator_for_query
                                            .record_intellisense_v2_completion_owner_hint_result(
                                                "cancelled",
                                            );
                                        return (
                                            file_content,
                                            file_path,
                                            None,
                                            None,
                                            deps,
                                            ir_program,
                                            ir_cancelled_after_retry,
                                            true,
                                        );
                                    }
                                    let parse_result =
                                        bsl_runtime::application::IntellisenseV2Facade::run_parse_result_query_singleflight(
                                            &context_for_query,
                                            &analysis,
                                            ir_program.is_some() || parse_result_without_ir,
                                            Some(coordinator_for_query.as_ref()),
                                            file_id,
                                        )
                                        .ok()
                                        .flatten();

                                    if bsl_runtime::system::global_runtime_config()
                                        .get_bool(
                                            bsl_runtime::system::RuntimeKey::IntellisenseV2P4Smoke,
                                        )
                                        .unwrap_or(false)
                                    {
                                        match ir_program.as_ref() {
                                            Some(program) => debug!(
                                                "Completion v2 ir: uri={}, file_id={}, deps_id={:?}, nodes={}",
                                                uri_for_query,
                                                file_id.0,
                                                Some(observed_deps_id_for_query.as_str()),
                                                program.nodes.len()
                                            ),
                                            None => debug!(
                                                "Completion v2 ir: uri={}, file_id={} (unavailable)",
                                                uri_for_query, file_id.0
                                            ),
                                        }
                                    }

                                    if bsl_runtime::system::global_runtime_config()
                                        .get_bool(
                                            bsl_runtime::system::RuntimeKey::IntellisenseV2P3Smoke,
                                        )
                                        .unwrap_or(false)
                                    {
                                        match parse_result.as_ref() {
                                            Some(parsed) => debug!(
                                                "Completion v2 parse_result: uri={}, file_id={}, syntax_errors={}",
                                                uri_for_query,
                                                file_id.0,
                                                parsed.syntax_errors.len()
                                            ),
                                            None => debug!(
                                                "Completion v2 parse_result: uri={}, file_id={} (unavailable)",
                                                uri_for_query, file_id.0
                                            ),
                                        }
                                    }
                                    if cancellation_token_for_query
                                        .as_ref()
                                        .is_some_and(|token| token.is_cancelled())
                                    {
                                        coordinator_for_query
                                            .record_intellisense_v2_completion_owner_hint_result(
                                                "cancelled",
                                            );
                                        return (
                                            file_content,
                                            file_path,
                                            parse_result,
                                            None,
                                            deps,
                                            ir_program,
                                            ir_cancelled_after_retry,
                                            true,
                                        );
                                    }

                                    let owner_hint_started = Instant::now();
                                    let mut owner_hint_reason = "not_member_access";
                                    let mut owner_hint_line_len_chars: Option<usize> = None;
                                    let mut owner_hint_receiver_len_chars: Option<usize> = None;
                                    let mut owner_hint_lookup_path: Option<&'static str> = None;
                                    let mut owner_hint_lookup_result: Option<&'static str> = None;
                                    let ms_to_duration = |value_ms: u128| {
                                        std::time::Duration::from_millis(
                                            value_ms.min(u64::MAX as u128) as u64,
                                        )
                                    };
                                    let record_type_lookup_profile =
                                        |profile: &bsl_analysis_v2::TypeAtByteOffsetProfile| {
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch",
                                            ms_to_duration(profile.index_fetch_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_wait",
                                            ms_to_duration(profile.index_fetch_wait_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_unattributed",
                                            ms_to_duration(profile.index_fetch_unattributed_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_pre_first_salsa_event_wait",
                                            ms_to_duration(
                                                profile.index_fetch_pre_first_salsa_event_wait_ms,
                                            ),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_post_last_salsa_event_tail",
                                            ms_to_duration(
                                                profile.index_fetch_post_last_salsa_event_tail_ms,
                                            ),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_inside_salsa_window",
                                            ms_to_duration(
                                                profile.index_fetch_inside_salsa_window_ms,
                                            ),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_type_index",
                                            ms_to_duration(
                                                profile.index_fetch_first_will_execute_type_index_ms,
                                            ),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_type_index",
                                            ms_to_duration(
                                                profile.index_fetch_last_will_execute_type_index_ms,
                                            ),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_first_will_execute_parse_result",
                                            ms_to_duration(
                                                profile
                                                    .index_fetch_first_will_execute_parse_result_ms,
                                            ),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_execute_parse_result",
                                            ms_to_duration(
                                                profile.index_fetch_last_will_execute_parse_result_ms,
                                            ),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_first_will_check_cancellation",
                                            ms_to_duration(
                                                profile
                                                    .index_fetch_first_will_check_cancellation_ms,
                                            ),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_fetch_last_will_check_cancellation",
                                            ms_to_duration(
                                                profile
                                                    .index_fetch_last_will_check_cancellation_ms,
                                            ),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_query_total",
                                            ms_to_duration(profile.index_query_total_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_query_inputs",
                                            ms_to_duration(profile.index_query_inputs_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_query_parse_result_query",
                                            ms_to_duration(profile.index_query_parse_result_query_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_query_build",
                                            ms_to_duration(profile.index_query_build_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_parse_result",
                                            ms_to_duration(profile.index_parse_result_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_build_total",
                                            ms_to_duration(profile.index_build_total_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_build_seed_context",
                                            ms_to_duration(profile.index_build_seed_module_context_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_build_local_function_summaries",
                                            ms_to_duration(
                                                profile
                                                    .index_build_local_function_summaries_ms,
                                            ),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_build_visit_statements",
                                            ms_to_duration(profile.index_build_visit_statements_ms),
                                        );
                                        coordinator_for_query.record_completion_stage_latency(
                                            "query_bundle_owner_hint_type_lookup_index_scan",
                                            ms_to_duration(profile.index_scan_ms),
                                        );
                                        coordinator_for_query.record_intellisense_v2_completion_owner_hint_index_fetch_salsa_counters(
                                            bsl_runtime::system::basic_observability::CompletionOwnerHintIndexFetchSalsaCounters {
                                                block_on_total: profile.index_fetch_will_block_on_total,
                                                block_on_type_index_total: profile.index_fetch_will_block_on_type_index_total,
                                                block_on_parse_result_total: profile.index_fetch_will_block_on_parse_result_total,
                                                block_on_other_total: profile.index_fetch_will_block_on_other_total,
                                                will_execute_total: profile.index_fetch_will_execute_total,
                                                will_execute_type_index_total: profile.index_fetch_will_execute_type_index_total,
                                                will_execute_parse_result_total: profile.index_fetch_will_execute_parse_result_total,
                                                will_execute_other_total: profile.index_fetch_will_execute_other_total,
                                                did_validate_memoized_total: profile.index_fetch_did_validate_memoized_total,
                                                did_validate_memoized_type_index_total: profile.index_fetch_did_validate_memoized_type_index_total,
                                                did_validate_memoized_parse_result_total: profile.index_fetch_did_validate_memoized_parse_result_total,
                                                did_validate_memoized_other_total: profile.index_fetch_did_validate_memoized_other_total,
                                                will_check_cancellation_total: profile.index_fetch_will_check_cancellation_total,
                                            },
                                        );
                                        coordinator_for_query
                                            .record_intellisense_v2_completion_owner_hint_index_fetch_active_gauge(
                                                profile.index_fetch_active_at_entry,
                                            );
                                        };
                                    let member_access_owner_type_hint =
                                        if member_access_request_for_query {
                                            let owner_hint_extract_started = Instant::now();
                                            let extracted_owner_hint = (|| {
                                                let text = match file_content.as_deref() {
                                                    Some(text) => text,
                                                    None => {
                                                        owner_hint_reason = "no_file_content";
                                                        return None;
                                                    }
                                                };
                                                let line_text = match text.lines().nth(position.line as usize) {
                                                    Some(line_text) => line_text,
                                                    None => {
                                                        owner_hint_reason = "no_line";
                                                        return None;
                                                    }
                                                };
                                                owner_hint_line_len_chars =
                                                    Some(line_text.chars().count());
                                                let cursor_byte =
                                                    bsl_backend::system::positioning::utf16_to_byte_offset(
                                                        line_text,
                                                        position.character,
                                                    );
                                                let line_prefix = match line_text.get(..cursor_byte) {
                                                    Some(line_prefix) => line_prefix,
                                                    None => {
                                                        owner_hint_reason = "no_dot";
                                                        return None;
                                                    }
                                                };
                                                // Some clients place cursor exactly on '.' when requesting completion.
                                                // Include that char into prefix so owner hint can still be resolved.
                                                let line_prefix = if line_text
                                                    .get(cursor_byte..)
                                                    .and_then(|tail| tail.chars().next())
                                                    == Some('.')
                                                {
                                                    line_text
                                                        .get(..cursor_byte + 1)
                                                        .unwrap_or(line_prefix)
                                                } else {
                                                    line_prefix
                                                };
                                                let dot_in_line = match line_prefix.rfind('.') {
                                                    Some(dot_in_line) => dot_in_line,
                                                    None => {
                                                        owner_hint_reason = "no_dot";
                                                        return None;
                                                    }
                                                };
                                                let receiver = match line_prefix.get(..dot_in_line) {
                                                    Some(receiver) => receiver.trim_end(),
                                                    None => {
                                                        owner_hint_reason = "no_receiver";
                                                        return None;
                                                    }
                                                };
                                                owner_hint_receiver_len_chars =
                                                    Some(receiver.chars().count());
                                                let (probe_byte, _) = match receiver
                                                    .char_indices()
                                                    .rev()
                                                    .find(|(_, ch)| !ch.is_whitespace())
                                                {
                                                    Some(probe) => probe,
                                                    None => {
                                                        owner_hint_reason = "no_receiver";
                                                        return None;
                                                    }
                                                };
                                                Some(
                                                    bsl_backend::system::positioning::byte_offset_to_utf16(
                                                        line_text, probe_byte,
                                                    ),
                                                )
                                            })();
                                            coordinator_for_query.record_completion_stage_latency(
                                                "query_bundle_owner_hint_extract",
                                                owner_hint_extract_started.elapsed(),
                                            );
                                            match extracted_owner_hint {
                                                Some(probe_utf16) => {
                                                    let owner_hint_offset_started = Instant::now();
                                                    let offset = analysis
                                                        .utf16_position_to_byte_offset(
                                                            file_id,
                                                            position.line,
                                                            probe_utf16,
                                                        )
                                                        .ok()
                                                        .flatten();
                                                    coordinator_for_query
                                                        .record_completion_stage_latency(
                                                            "query_bundle_owner_hint_offset",
                                                            owner_hint_offset_started.elapsed(),
                                                        );
                                                    match offset {
                                                        Some(offset) => {
                                                            let offset =
                                                                offset.min(u32::MAX as usize) as u32;
                                                            let owner_hint_type_lookup_started =
                                                                Instant::now();
                                                            let hint = if include_flow_sensitive {
                                                                owner_hint_lookup_path =
                                                                    Some("flow_only");
                                                                let flow_lookup_started =
                                                                    Instant::now();
                                                                let flow_hint_result = analysis
                                                                    .flow_type_at_byte_offset(
                                                                        file_id, offset,
                                                                    );
                                                                coordinator_for_query.record_completion_stage_latency(
                                                                    "query_bundle_owner_hint_flow_lookup",
                                                                    flow_lookup_started.elapsed(),
                                                                );
                                                                match flow_hint_result {
                                                                    Ok(Some(flow_hint)) => {
                                                                        owner_hint_reason =
                                                                            "flow_type_hit";
                                                                        owner_hint_lookup_result =
                                                                            Some("hit");
                                                                        Some(flow_hint)
                                                                    }
                                                                    Ok(None) => {
                                                                        owner_hint_lookup_path = Some(
                                                                            "flow_plus_fallback",
                                                                        );
                                                                        let fallback_started =
                                                                            Instant::now();
                                                                        let fallback_hint_result =
                                                                            analysis
                                                                                .type_at_byte_offset_profiled(
                                                                                    file_id, offset,
                                                                                );
                                                                        coordinator_for_query.record_completion_stage_latency(
                                                                            "query_bundle_owner_hint_type_lookup_fallback",
                                                                            fallback_started.elapsed(),
                                                                        );
                                                                        match fallback_hint_result {
                                                                            Ok(profiled) => {
                                                                                record_type_lookup_profile(
                                                                                    &profiled.profile,
                                                                                );
                                                                                if let Some(type_hint) = profiled.resolution {
                                                                                    owner_hint_reason =
                                                                                        "type_hit";
                                                                                    owner_hint_lookup_result = Some("hit");
                                                                                    Some(type_hint)
                                                                                } else {
                                                                                    owner_hint_reason =
                                                                                        "type_miss";
                                                                                    owner_hint_lookup_result = Some("miss");
                                                                                    None
                                                                                }
                                                                            }
                                                                            Err(_) => {
                                                                                owner_hint_reason =
                                                                                    "cancelled";
                                                                                owner_hint_lookup_result = Some("cancelled");
                                                                                None
                                                                            }
                                                                        }
                                                                    }
                                                                    Err(_) => {
                                                                        owner_hint_reason =
                                                                            "cancelled";
                                                                        owner_hint_lookup_result =
                                                                            Some("cancelled");
                                                                        None
                                                                    }
                                                                }
                                                            } else {
                                                                owner_hint_lookup_path =
                                                                    Some("direct");
                                                                let direct_started = Instant::now();
                                                                let type_hint_result = analysis
                                                                    .type_at_byte_offset_profiled(
                                                                        file_id, offset,
                                                                    )
                                                                ;
                                                                coordinator_for_query
                                                                    .record_completion_stage_latency(
                                                                        "query_bundle_owner_hint_type_lookup_direct",
                                                                        direct_started.elapsed(),
                                                                );
                                                                match type_hint_result {
                                                                    Ok(profiled) => {
                                                                        record_type_lookup_profile(
                                                                            &profiled.profile,
                                                                        );
                                                                        if let Some(type_hint) = profiled.resolution {
                                                                            owner_hint_reason =
                                                                                "type_hit";
                                                                            owner_hint_lookup_result =
                                                                                Some("hit");
                                                                            Some(type_hint)
                                                                        } else {
                                                                            owner_hint_reason =
                                                                                "type_miss";
                                                                            owner_hint_lookup_result =
                                                                                Some("miss");
                                                                            None
                                                                        }
                                                                    }
                                                                    Err(_) => {
                                                                        owner_hint_reason =
                                                                            "cancelled";
                                                                        owner_hint_lookup_result =
                                                                            Some("cancelled");
                                                                        None
                                                                    }
                                                                }
                                                            };
                                                            coordinator_for_query
                                                                .record_completion_stage_latency(
                                                                    "query_bundle_owner_hint_type_lookup",
                                                                    owner_hint_type_lookup_started
                                                                        .elapsed(),
                                                                );
                                                            hint
                                                        }
                                                        None => {
                                                            owner_hint_reason = "offset_unresolved";
                                                            None
                                                        }
                                                    }
                                                }
                                                None => None,
                                            }
                                    } else {
                                        None
                                    };
                                    coordinator_for_query
                                        .record_intellisense_v2_completion_owner_hint_result(
                                            owner_hint_reason,
                                        );
                                    if let Some(path) = owner_hint_lookup_path {
                                        coordinator_for_query
                                            .record_intellisense_v2_completion_owner_hint_lookup_path(
                                                path,
                                            );
                                    }
                                    if let Some(result) = owner_hint_lookup_result {
                                        coordinator_for_query
                                            .record_intellisense_v2_completion_owner_hint_lookup_result(
                                                result,
                                            );
                                    }
                                    if let (Some(line_len_chars), Some(receiver_len_chars)) = (
                                        owner_hint_line_len_chars,
                                        owner_hint_receiver_len_chars,
                                    ) {
                                        coordinator_for_query
                                            .record_intellisense_v2_completion_owner_hint_context(
                                                line_len_chars,
                                                receiver_len_chars,
                                            );
                                    }
                                    coordinator_for_query.record_completion_stage_latency(
                                        "query_bundle_owner_hint",
                                        owner_hint_started.elapsed(),
                                    );

                                    (
                                        file_content,
                                        file_path,
                                        parse_result,
                                        member_access_owner_type_hint,
                                        deps,
                                        ir_program,
                                        ir_cancelled_after_retry,
                                        false,
                                    )
                                },
                            )
                            .await;

                        let (
                            file_content,
                            file_path,
                            parse_result,
                            member_access_owner_type_hint,
                            deps,
                            ir_program,
                            ir_cancelled_after_retry,
                            query_checkpoint_cancelled,
                        ) = match query_result {
                            Ok(result) => result,
                            Err(join_error) => {
                                warn!(
                                    uri = %uri,
                                    file_id = file_id.0,
                                    error = %join_error,
                                    "Completion v2: interactive query task failed"
                                );
                                (None, None, None, None, None, None, true, true)
                            }
                        };
                        if (ir_cancelled_after_retry || query_checkpoint_cancelled)
                            && completion_outcome.is_none()
                        {
                            completion_outcome = Some("cancelled");
                        }
                        if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                            event_driven_guards_enabled,
                            self,
                            file_id,
                            completion_request_id.as_deref(),
                            completion_ticket.request_epoch,
                            completion_cancellation_token.as_ref(),
                            "ir",
                            &mut cancel_event_emitted,
                        )
                        .await
                        {
                            completion_outcome = Some(outcome);
                            break 'completion_flow Some(completion_incomplete_empty_response());
                        }

                        (
                            file_content,
                            file_path,
                            parse_result,
                            member_access_owner_type_hint,
                            deps,
                            ir_program,
                            index_snapshot,
                            observed_deps_id,
                            observed_settings_id,
                            observed_file_version,
                        )
                    };
                    self.coordinator.record_completion_stage_latency(
                        "query_bundle",
                        query_bundle_started.elapsed(),
                    );
                    observed_file_version_for_completion = observed_file_version;
                    let member_access_context = file_content
                        .as_deref()
                        .map(|text| {
                            completion_request_targets_member_access(
                                text,
                                position,
                                trigger_char_hint,
                            )
                        })
                        .unwrap_or(member_access_request);
                    member_access_observed = member_access_context;
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "collect",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }

                    let response_build_started = Instant::now();
                    let mut completion_response = match (file_content, file_path, deps, ir_program)
                    {
                        (Some(file_content), Some(file_path), Some(deps), Some(ir_program)) => {
                            crate::handlers::handle_completion_v2_with_trigger_hint(
                                file_content,
                                file_path,
                                ir_program,
                                parse_result,
                                member_access_owner_type_hint,
                                deps,
                                position,
                                &uri,
                                index_snapshot.as_ref(),
                                snippet_support,
                                include_flow_sensitive,
                                trigger_char_hint,
                            )
                            .await
                        }
                        (None, _, _, _) => {
                            completion_outcome.get_or_insert("missing_file_content");
                            empty()
                        }
                        (Some(_), None, _, _) => {
                            completion_outcome.get_or_insert("missing_file_path");
                            empty()
                        }
                        (Some(_), Some(_), None, _) => {
                            completion_outcome.get_or_insert("missing_deps");
                            empty()
                        }
                        (Some(file_content), Some(file_path), Some(deps), None) => {
                            let (fallback_outcome, response) = resolve_completion_without_ir(
                                self,
                                file_id,
                                observed_deps_id.clone(),
                                observed_settings_id.clone(),
                                observed_file_version,
                                member_access_context,
                                file_content,
                                file_path,
                                parse_result,
                                member_access_owner_type_hint,
                                deps,
                                position,
                                &uri,
                                index_snapshot.as_ref(),
                                snippet_support,
                                include_flow_sensitive,
                                trigger_char_hint,
                            )
                            .await;
                            completion_outcome.get_or_insert(fallback_outcome);
                            response
                        }
                    };
                    self.coordinator.record_completion_stage_latency(
                        "response_build",
                        response_build_started.elapsed(),
                    );
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "rank",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "format",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                    if force_incomplete_due_stale {
                        if let Some(response) = completion_response.as_mut() {
                            if let CompletionResponse::List(list) = &mut response.response {
                                list.is_incomplete = true;
                            }
                        }
                    }
                    if !matches!(completion_outcome, Some("cancelled" | "superseded_epoch")) {
                        let cache_store_started = Instant::now();
                        if let (Some(settings_id), Some(file_version), Some(response_items)) = (
                            observed_settings_id.clone(),
                            observed_file_version,
                            completion_response
                                .as_ref()
                                .and_then(extract_non_empty_items),
                        ) {
                            self.completion_stale_fallback_cache_v2
                                .write()
                                .await
                                .insert(
                                    file_id,
                                    CompletionStaleFallbackCacheEntryV2 {
                                        deps_id: observed_deps_id,
                                        settings_id,
                                        file_version,
                                        items: response_items,
                                    },
                                );
                        }
                        self.coordinator.record_completion_stage_latency(
                            "cache_store",
                            cache_store_started.elapsed(),
                        );
                    }
                    completion_response
                }
                Err(outcome) => {
                    completion_outcome = Some("wait_not_ready");
                    debug!(
                        uri = %uri,
                        file_id = file_id.0,
                        outcome = outcome.as_str(),
                        "Completion v2: stateful operation not ready"
                    );
                    empty()
                }
            }
        };
        let elapsed = started.elapsed();
        self.coordinator.record_completion_latency(elapsed);
        if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
            event_driven_guards_enabled,
            self,
            file_id,
            completion_request_id.as_deref(),
            completion_ticket.request_epoch,
            completion_cancellation_token.as_ref(),
            "publish",
            &mut cancel_event_emitted,
        )
        .await
        {
            completion_outcome = Some(outcome);
            completion = Some(completion_incomplete_empty_response());
        }

        if let Some(result) = &completion {
            if result.had_error {
                self.coordinator.record_completion_error();
            }

            let items_count = match &result.response {
                CompletionResponse::List(list) => {
                    if list.is_incomplete {
                        self.coordinator.record_completion_incomplete();
                    }
                    list.items.len()
                }
                CompletionResponse::Array(items) => items.len(),
            };
            self.coordinator
                .record_intellisense_v2_completion_items_count(items_count);

            if let Some(stats) = &result.stats {
                self.coordinator
                    .record_completion_stage_latency("snapshot_read", stats.stage_snapshot_read);
                self.coordinator
                    .record_completion_stage_latency("collect", stats.stage_collect);
                self.coordinator
                    .record_completion_stage_latency("rank", stats.stage_rank);
                self.coordinator
                    .record_completion_stage_latency("format", stats.stage_format);
            }

            if bsl_runtime::system::global_runtime_config()
                .get_bool(bsl_runtime::system::RuntimeKey::CompletionQuality)
                .unwrap_or(false)
            {
                if let Some(stats) = &result.stats {
                    self.coordinator.record_completion_quality(
                        stats.total_candidates,
                        stats.dedup_removed,
                        &stats.score_samples,
                        stats.prefix_exact,
                        stats.prefix_starts,
                        stats.prefix_contains,
                        stats.prefix_none,
                        stats.member_access,
                        stats.has_owner,
                    );
                }
            }

            if completion_outcome.is_none() {
                completion_outcome = Some(if result.had_error {
                    "handler_error"
                } else if items_count == 0 {
                    "ok_empty"
                } else {
                    "ok_non_empty"
                });
            }

            if member_access_observed && !result.had_error && items_count == 0 {
                self.coordinator
                    .record_intellisense_v2_completion_member_access_terminal_empty(
                        trigger_mode,
                        completion_outcome.unwrap_or("ok_empty"),
                    );
            }

            if member_access_observed && matches!(trigger_mode, "trigger_character" | "invoked") {
                if let Some(observed_file_version) = observed_file_version_for_completion {
                    let key = (
                        file_id,
                        observed_file_version,
                        position.line,
                        position.character,
                    );
                    let non_empty = items_count > 0;
                    let labels = completion_labels_fingerprint(&result.response);
                    let parity_result = {
                        let mut parity = self.completion_parity_state_v2.write().await;
                        let entry = parity.entry(key).or_default();
                        if trigger_mode == "trigger_character" {
                            entry.trigger_character_non_empty = Some(non_empty);
                            entry.trigger_character_labels = Some(labels.clone());
                        } else {
                            entry.invoked_non_empty = Some(non_empty);
                            entry.invoked_labels = Some(labels.clone());
                        }
                        match (
                            entry.trigger_character_non_empty,
                            entry.invoked_non_empty,
                            entry.trigger_character_labels.as_ref(),
                            entry.invoked_labels.as_ref(),
                        ) {
                            (
                                Some(trigger_non_empty),
                                Some(invoked_non_empty),
                                Some(trigger_labels),
                                Some(invoked_labels),
                            ) => {
                                let overlap_ratio =
                                    completion_labels_overlap_ratio(trigger_labels, invoked_labels);
                                let mismatch = trigger_non_empty != invoked_non_empty
                                    || (trigger_non_empty
                                        && invoked_non_empty
                                        && overlap_ratio <= 0.0);
                                parity.remove(&key);
                                Some((mismatch, overlap_ratio))
                            }
                            _ => None,
                        }
                    };
                    if let Some((parity_drift, overlap_ratio)) = parity_result {
                        self.coordinator
                            .record_intellisense_v2_completion_parity_overlap_bucket(
                                trigger_mode,
                                completion_parity_overlap_bucket(overlap_ratio),
                            );
                        if parity_drift {
                            self.coordinator
                                .record_intellisense_v2_completion_parity_drift(trigger_mode);
                        }
                    }
                }
            }
        }

        if let Some(outcome) = completion_outcome {
            self.coordinator
                .record_intellisense_v2_completion_outcome(outcome);
        }

        if let Some(drop_guard) = completion_drop_guard.as_mut() {
            drop_guard.disarm();
        }
        Ok(completion.map(|result| result.response))
    }

    async fn completion_resolve(&self, item: CompletionItem) -> JsonRpcResult<CompletionItem> {
        let snippet_support = *self.completion_snippet_support.read().await;
        let started = Instant::now();
        let deps = self.analysis_v2.snapshot().await.deps_data().ok();
        let resolved = handle_completion_resolve(item, deps, snippet_support).await;
        let elapsed = started.elapsed();
        self.coordinator.record_completion_resolve_latency(elapsed);
        Ok(resolved)
    }

    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        info!(
            "Hover requested at {}:{}",
            position.line, position.character
        );

        {
            let file_id = self.get_or_create_file_id_v2(&uri).await;
            self.sync_v2_globals().await;

            let include_flow_sensitive = {
                let settings = self.settings.read().await;
                settings.enable_flow_sensitive
            };
            let prepared = self
                .prepare_lsp_stateful_operation_v2(
                    &uri,
                    file_id,
                    bsl_runtime::application::SemanticOperation::Hover,
                    include_flow_sensitive,
                )
                .await;
            let (context, prepared, expected_version) = match prepared {
                Ok(values) => values,
                Err(outcome) => {
                    debug!(
                        uri = %uri,
                        file_id = file_id.0,
                        outcome = outcome.as_str(),
                        "Hover v2: stateful operation not ready"
                    );
                    return Ok(None);
                }
            };

            if let Some(wait_elapsed) = prepared.wait_elapsed {
                if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                    if wait_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            expected_version,
                            wait_ms = wait_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "Hover v2: wait_for_file_version is slow"
                        );
                    }
                }
            }
            if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
                if prepared.snapshot_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        snapshot_ms = prepared.snapshot_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "Hover v2: snapshot acquisition is slow"
                    );
                }
            }

            let (analysis, file_content, file_path, deps, ir_program) = {
                let analysis = prepared.snapshot.analysis;
                let index_snapshot = prepared.snapshot.index_snapshot;
                let observed_deps_id = Some(prepared.snapshot.deps_id);
                let observed_file_version = analysis.file_version(file_id).ok().flatten();
                let observed_settings_id = analysis.settings_id().ok();
                debug!(
                    "Hover v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
                    uri,
                    file_id.0,
                    observed_file_version,
                    observed_deps_id.as_ref().map(|v| v.as_str()),
                    observed_settings_id.as_ref().map(|v| v.as_str()),
                    index_snapshot.id.as_str(),
                );

                let observed_byte_offset = analysis
                    .utf16_position_to_byte_offset(file_id, position.line, position.character)
                    .ok()
                    .flatten();
                let observed_point = analysis
                    .utf16_position_to_point(file_id, position.line, position.character)
                    .ok()
                    .flatten();
                debug!(
                    "Hover v2 positioning: uri={}, file_id={}, lsp=({}:{}) -> byte_offset={:?}, point={:?}",
                    uri,
                    file_id.0,
                    position.line,
                    position.character,
                    observed_byte_offset,
                    observed_point,
                );

                let file_content = analysis.file_text(file_id).ok().flatten();
                let file_path = analysis.file_path(file_id).ok().flatten();
                let deps = analysis.deps_data().ok();
                let ir_started = Instant::now();
                let ir_program =
                    bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                        &context,
                        &analysis,
                        Some(self.coordinator.as_ref()),
                        file_id,
                    )
                    .ok()
                    .flatten();
                let ir_elapsed = ir_started.elapsed();
                if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                    if ir_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            ir_ms = ir_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "Hover v2: ir query is slow"
                        );
                    }
                }

                (analysis, file_content, file_path, deps, ir_program)
            };

            let settings = self.settings.read().await;
            let result = match (file_content, file_path, deps, ir_program) {
                (Some(file_content), Some(file_path), Some(deps), Some(ir_program)) => {
                    handle_hover_v2(
                        &analysis,
                        file_id,
                        file_content,
                        file_path,
                        ir_program,
                        deps,
                        position,
                        &uri,
                        &settings.hover,
                        include_flow_sensitive,
                    )
                }
                _ => None,
            };

            return Ok(result);
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> JsonRpcResult<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;

        let (feature_enabled, settings) = {
            let cfg = self.config.read().await;
            let feature_enabled = cfg
                .as_ref()
                .and_then(|cfg| cfg.enable_type_hints)
                .unwrap_or(false);
            let settings = self.settings.read().await.type_hints.clone();
            (feature_enabled, settings)
        };
        if !feature_enabled || !settings.enabled {
            return Ok(None);
        }

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let include_flow_sensitive = {
            let guard = self.settings.read().await;
            guard.enable_flow_sensitive
        };
        let prepared = self
            .prepare_lsp_stateful_operation_v2(
                &uri,
                file_id,
                bsl_runtime::application::SemanticOperation::TypeAtPosition,
                include_flow_sensitive,
            )
            .await;
        let (context, prepared, _expected_version) = match prepared {
            Ok(values) => values,
            Err(outcome) => {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    outcome = outcome.as_str(),
                    "Inlay hints v2: stateful operation not ready"
                );
                return Ok(None);
            }
        };
        let analysis = prepared.snapshot.analysis;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let ir_program = bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
            &context,
            &analysis,
            Some(self.coordinator.as_ref()),
            file_id,
        )
        .ok()
        .flatten();
        let Some(ir_program) = ir_program else {
            return Ok(None);
        };

        let range = params.range;
        let computed = timeout(std::time::Duration::from_millis(80), async move {
            handle_inlay_hints_v2(
                &analysis,
                file_id,
                file_content,
                ir_program,
                range,
                &settings,
            )
        })
        .await;

        match computed {
            Ok(hints) => Ok(Some(hints)),
            Err(_) => {
                warn!(uri = %uri, "Inlay hints: timed out");
                Ok(Some(Vec::new()))
            }
        }
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> JsonRpcResult<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;

        let (feature_enabled, code_actions_settings, type_hints_settings) = {
            let cfg = self.config.read().await;
            let feature_enabled = cfg
                .as_ref()
                .and_then(|cfg| cfg.enable_code_actions)
                .unwrap_or(false);
            let settings = self.settings.read().await;
            (
                feature_enabled,
                settings.code_actions.clone(),
                settings.type_hints.clone(),
            )
        };
        if !feature_enabled || !code_actions_settings.enabled {
            return Ok(None);
        }

        self.sync_v2_globals().await;
        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let include_flow_sensitive = {
            let guard = self.settings.read().await;
            guard.enable_flow_sensitive
        };
        let prepared = self
            .prepare_lsp_stateful_operation_v2(
                &uri,
                file_id,
                bsl_runtime::application::SemanticOperation::TypeAtPosition,
                include_flow_sensitive,
            )
            .await;
        let (context, prepared, _expected_version) = match prepared {
            Ok(values) => values,
            Err(outcome) => {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    outcome = outcome.as_str(),
                    "Code actions v2: stateful operation not ready"
                );
                return Ok(None);
            }
        };
        let analysis = prepared.snapshot.analysis;
        let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
            return Ok(None);
        };
        let ir_program = bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
            &context,
            &analysis,
            Some(self.coordinator.as_ref()),
            file_id,
        )
        .ok()
        .flatten();
        let Some(ir_program) = ir_program else {
            return Ok(None);
        };

        let range = params.range;
        let uri_for_action = uri.clone();
        let computed = timeout(std::time::Duration::from_millis(120), async move {
            handle_code_actions_v2(
                &analysis,
                file_id,
                file_content,
                ir_program,
                &uri_for_action,
                range,
                &code_actions_settings,
                &type_hints_settings,
            )
        })
        .await;

        match computed {
            Ok(actions) => Ok(Some(actions)),
            Err(_) => {
                warn!(uri = %uri, "Code actions: timed out");
                Ok(Some(Vec::new()))
            }
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> JsonRpcResult<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        {
            let file_id = self.get_or_create_file_id_v2(&uri).await;
            self.sync_v2_globals().await;

            let include_flow_sensitive = {
                let settings = self.settings.read().await;
                settings.enable_flow_sensitive
            };
            let prepared = self
                .prepare_lsp_stateful_operation_v2(
                    &uri,
                    file_id,
                    bsl_runtime::application::SemanticOperation::Definition,
                    include_flow_sensitive,
                )
                .await;
            let (context, prepared, expected_version) = match prepared {
                Ok(values) => values,
                Err(outcome) => {
                    debug!(
                        uri = %uri,
                        file_id = file_id.0,
                        outcome = outcome.as_str(),
                        "Definition v2: stateful operation not ready"
                    );
                    return Ok(None);
                }
            };
            if let Some(wait_elapsed) = prepared.wait_elapsed {
                if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                    if wait_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            expected_version,
                            wait_ms = wait_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "Definition v2: wait_for_file_version is slow"
                        );
                    }
                }
            }
            if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
                if prepared.snapshot_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        snapshot_ms = prepared.snapshot_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "Definition v2: snapshot acquisition is slow"
                    );
                }
            }

            let (
                file_content,
                file_path,
                type_at_position_hint,
                receiver_type_hint,
                deps,
                ir_program,
            ) = {
                let analysis = prepared.snapshot.analysis;
                let index_snapshot = prepared.snapshot.index_snapshot;

                let observed_file_version = analysis.file_version(file_id).ok().flatten();
                let observed_deps_id = Some(prepared.snapshot.deps_id);
                let observed_settings_id = analysis.settings_id().ok();
                debug!(
                    "Definition v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
                    uri,
                    file_id.0,
                    observed_file_version,
                    observed_deps_id.as_ref().map(|v| v.as_str()),
                    observed_settings_id.as_ref().map(|v| v.as_str()),
                    index_snapshot.id.as_str(),
                );

                let file_content = analysis.file_text(file_id).ok().flatten();
                let file_path = analysis.file_path(file_id).ok().flatten();
                let deps = analysis.deps_data().ok();
                let ir_started = Instant::now();
                let ir_program =
                    bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                        &context,
                        &analysis,
                        Some(self.coordinator.as_ref()),
                        file_id,
                    )
                    .ok()
                    .flatten();
                let ir_elapsed = ir_started.elapsed();
                if let Some(threshold) = super::intellisense_v2_slow_query_warn_threshold() {
                    if ir_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            ir_ms = ir_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "Definition v2: ir query is slow"
                        );
                    }
                }

                let type_at_position_hint = {
                    let offset = analysis
                        .utf16_position_to_byte_offset(file_id, position.line, position.character)
                        .ok()
                        .flatten();
                    offset.and_then(|offset| {
                        let offset = offset.min(u32::MAX as usize) as u32;
                        if include_flow_sensitive {
                            analysis
                                .flow_type_at_byte_offset(file_id, offset)
                                .ok()
                                .flatten()
                                .or_else(|| {
                                    analysis.type_at_byte_offset(file_id, offset).ok().flatten()
                                })
                        } else {
                            analysis.type_at_byte_offset(file_id, offset).ok().flatten()
                        }
                    })
                };
                let receiver_type_hint = ir_program.as_ref().and_then(|program| {
                    let offset = analysis
                        .utf16_position_to_byte_offset(file_id, position.line, position.character)
                        .ok()
                        .flatten()
                        .map(|offset| offset.min(u32::MAX as usize) as u32)?;

                    let node = program.find_node_at_byte_offset(offset)?;

                    let object_span = match &node.kind {
                        bsl_shared::ir::SemanticNodeKind::MemberAccess { object_node, .. } => {
                            object_node.and_then(|idx| program.nodes.get(idx).map(|n| n.span))
                        }
                        bsl_shared::ir::SemanticNodeKind::FunctionCall { object_node, .. } => {
                            object_node.and_then(|idx| program.nodes.get(idx).map(|n| n.span))
                        }
                        _ => None,
                    }?;

                    if include_flow_sensitive {
                        analysis
                            .flow_type_at_byte_offset(file_id, object_span.start)
                            .ok()
                            .flatten()
                            .or_else(|| {
                                analysis
                                    .type_at_byte_offset(file_id, object_span.start)
                                    .ok()
                                    .flatten()
                            })
                    } else {
                        analysis
                            .type_at_byte_offset(file_id, object_span.start)
                            .ok()
                            .flatten()
                    }
                });

                (
                    file_content,
                    file_path,
                    type_at_position_hint,
                    receiver_type_hint,
                    deps,
                    ir_program,
                )
            };

            let result = match (file_content, file_path, deps, ir_program) {
                (Some(file_content), Some(file_path), Some(deps), Some(ir_program)) => {
                    handle_goto_definition_v2(
                        file_path,
                        file_content,
                        ir_program,
                        type_at_position_hint,
                        receiver_type_hint,
                        deps,
                        position,
                        &uri,
                    )
                    .await
                }
                _ => None,
            };

            return Ok(result);
        }
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> JsonRpcResult<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        {
            let file_id = self.get_or_create_file_id_v2(&uri).await;
            self.sync_v2_globals().await;

            let include_flow_sensitive = {
                let settings = self.settings.read().await;
                settings.enable_flow_sensitive
            };
            let prepared = self
                .prepare_lsp_stateful_operation_v2(
                    &uri,
                    file_id,
                    bsl_runtime::application::SemanticOperation::SignatureHelp,
                    include_flow_sensitive,
                )
                .await;
            let (_context, prepared, expected_version) = match prepared {
                Ok(values) => values,
                Err(outcome) => {
                    debug!(
                        uri = %uri,
                        file_id = file_id.0,
                        outcome = outcome.as_str(),
                        "SignatureHelp v2: stateful operation not ready"
                    );
                    return Ok(None);
                }
            };
            if let Some(wait_elapsed) = prepared.wait_elapsed {
                if let Some(threshold) = super::intellisense_v2_slow_wait_warn_threshold() {
                    if wait_elapsed >= threshold {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            expected_version,
                            wait_ms = wait_elapsed.as_millis(),
                            threshold_ms = threshold.as_millis(),
                            "SignatureHelp v2: wait_for_file_version is slow"
                        );
                    }
                }
            }
            if let Some(threshold) = super::intellisense_v2_slow_snapshot_warn_threshold() {
                if prepared.snapshot_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        snapshot_ms = prepared.snapshot_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "SignatureHelp v2: snapshot acquisition is slow"
                    );
                }
            }

            let (file_content, deps, receiver_type_hint) = {
                let analysis = prepared.snapshot.analysis;
                let index_snapshot = prepared.snapshot.index_snapshot;
                let observed_file_version = analysis.file_version(file_id).ok().flatten();
                let observed_deps_id = Some(prepared.snapshot.deps_id);
                let observed_settings_id = analysis.settings_id().ok();
                debug!(
                    "SignatureHelp v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
                    uri,
                    file_id.0,
                    observed_file_version,
                    observed_deps_id.as_ref().map(|v| v.as_str()),
                    observed_settings_id.as_ref().map(|v| v.as_str()),
                    index_snapshot.id.as_str(),
                );

                let observed_byte_offset = analysis
                    .utf16_position_to_byte_offset(file_id, position.line, position.character)
                    .ok()
                    .flatten();
                let observed_point = analysis
                    .utf16_position_to_point(file_id, position.line, position.character)
                    .ok()
                    .flatten();
                debug!(
                    "SignatureHelp v2 positioning: uri={}, file_id={}, lsp=({}:{}) -> byte_offset={:?}, point={:?}",
                    uri,
                    file_id.0,
                    position.line,
                    position.character,
                    observed_byte_offset,
                    observed_point,
                );

                let file_content = analysis.file_text(file_id).ok().flatten();
                let deps = analysis.deps_data().ok();

                let receiver_type_hint = file_content.as_ref().and_then(|text| {
                    let query = bsl_backend::application::type_system::signature_help_query(
                        text.as_ref(),
                        position.line,
                        position.character,
                    )?;
                    let receiver_end_character = query.receiver_end_character?;
                    let offset = analysis
                        .utf16_position_to_byte_offset(
                            file_id,
                            query.call_start_line,
                            receiver_end_character,
                        )
                        .ok()
                        .flatten()?;
                    let offset = offset.min(u32::MAX as usize) as u32;
                    if include_flow_sensitive {
                        analysis
                            .flow_type_at_byte_offset(file_id, offset)
                            .ok()
                            .flatten()
                            .or_else(|| {
                                analysis.type_at_byte_offset(file_id, offset).ok().flatten()
                            })
                    } else {
                        analysis.type_at_byte_offset(file_id, offset).ok().flatten()
                    }
                });

                (file_content, deps, receiver_type_hint)
            };

            let started = Instant::now();
            let result = match (file_content, deps) {
                (Some(file_content), Some(deps)) => {
                    handle_signature_help_v2(file_content, position, receiver_type_hint, deps).await
                }
                _ => None,
            };
            let elapsed = started.elapsed();
            self.coordinator.record_signature_help_latency(elapsed);
            if result.is_none() {
                self.coordinator.record_signature_help_empty();
            }
            return Ok(result);
        }
    }

    // ========================================================================
    // COMMAND EXECUTION
    // ========================================================================

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> JsonRpcResult<Option<serde_json::Value>> {
        info!(
            "Execute command: {} with {} arguments",
            params.command,
            params.arguments.len()
        );

        match params.command.as_str() {
            "bsl.getSemanticHtml" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing request parameters",
                    ));
                }

                let request: GetSemanticHtmlRequest =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let uri = Url::parse(&request.uri).map_err(|e| {
                    tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid URI: {}", e))
                })?;

                self.sync_v2_globals().await;
                let file_id = self.get_or_create_file_id_v2(&uri).await;
                let prepared = self
                    .prepare_lsp_stateful_operation_v2(
                        &uri,
                        file_id,
                        bsl_runtime::application::SemanticOperation::SymbolSearch,
                        false,
                    )
                    .await;
                let (context, prepared, _expected_version) = match prepared {
                    Ok(values) => values,
                    Err(outcome) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            outcome = outcome.as_str(),
                            "getSemanticHtml: stateful operation not ready"
                        );
                        return Err(tower_lsp::jsonrpc::Error::internal_error());
                    }
                };
                let analysis = prepared.snapshot.analysis;
                let file_text = analysis
                    .file_text(file_id)
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;
                let line_index = analysis
                    .line_index(file_id)
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;
                let ir_program =
                    bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                        &context,
                        &analysis,
                        Some(self.coordinator.as_ref()),
                        file_id,
                    )
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;

                let semantic_tree = semantic_tree_from_ir(
                    ir_program.as_ref(),
                    true,
                    true,
                    file_text.as_ref(),
                    line_index.as_ref(),
                );
                let result = semantic_html_from_tree(
                    &semantic_tree,
                    request.theme.as_deref(),
                    request.compact,
                );

                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getSemanticTree" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing request parameters",
                    ));
                }

                let request: GetSemanticTreeRequest =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let uri = Url::parse(&request.uri).map_err(|e| {
                    tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid URI: {}", e))
                })?;

                self.sync_v2_globals().await;
                let file_id = self.get_or_create_file_id_v2(&uri).await;
                let enable_flow_sensitive = {
                    let settings = self.settings.read().await;
                    settings.enable_flow_sensitive
                };
                let include_flow_sensitive = effective_include_flow_sensitive(
                    request.include_flow_sensitive,
                    enable_flow_sensitive,
                );
                let prepared = self
                    .prepare_lsp_stateful_operation_v2(
                        &uri,
                        file_id,
                        bsl_runtime::application::SemanticOperation::SymbolSearch,
                        include_flow_sensitive,
                    )
                    .await;
                let (context, prepared, _expected_version) = match prepared {
                    Ok(values) => values,
                    Err(outcome) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            outcome = outcome.as_str(),
                            "getSemanticTree: stateful operation not ready"
                        );
                        return Err(tower_lsp::jsonrpc::Error::internal_error());
                    }
                };
                let analysis = prepared.snapshot.analysis;
                let file_text = analysis
                    .file_text(file_id)
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;
                let line_index = analysis
                    .line_index(file_id)
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;
                let ir_program =
                    bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                        &context,
                        &analysis,
                        Some(self.coordinator.as_ref()),
                        file_id,
                    )
                    .ok()
                    .flatten()
                    .ok_or_else(tower_lsp::jsonrpc::Error::internal_error)?;

                let result = semantic_tree_from_ir(
                    ir_program.as_ref(),
                    request.include_call_graph,
                    include_flow_sensitive,
                    file_text.as_ref(),
                    line_index.as_ref(),
                );

                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.searchTypes" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing search query",
                    ));
                }

                let request: SearchTypesRequest =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = handle_search_types(request, self.coordinator.get_domain_bundle());
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getAllTypes" => {
                // Parameters are optional - use defaults if not provided
                let request: GetAllTypesRequest = if params.arguments.is_empty() {
                    GetAllTypesRequest {
                        limit: 1000,
                        offset: 0,
                        category: None,
                    }
                } else {
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?
                };

                let result = handle_get_all_types(request, self.coordinator.get_domain_bundle());
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getCurrentContext" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing parameters",
                    ));
                }

                let request: GetCurrentContextParams =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = self.handle_get_current_context(request).await?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.queryType" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing type name",
                    ));
                }

                let request: QueryTypeParams = serde_json::from_value(params.arguments[0].clone())
                    .map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let result = handle_query_type(request, self.coordinator.get_domain_bundle());
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getTypeRepositoryStats" => {
                let result = handle_get_type_repository_stats(self.coordinator.clone());
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getWorkspaceStats" => {
                let result = self.handle_get_workspace_stats().await?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getObservabilityMetrics" => {
                let result = self.handle_get_observability_metrics().await?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.getRuntimeConfig" => {
                let snapshot = bsl_runtime::system::global_runtime_config().snapshot();
                Ok(Some(serde_json::to_value(snapshot).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.parseConfiguration" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing configuration path",
                    ));
                }

                let request: ParseConfigurationParams =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;

                let platform_docs_root = {
                    let config = self.config.read().await;
                    config
                        .as_ref()
                        .and_then(|cfg| cfg.platform_docs_archive.as_deref())
                        .map(PathBuf::from)
                };
                let config_root = PathBuf::from(&request.config_path);

                let result = handle_parse_configuration(
                    request,
                    self.coordinator.get_domain_bundle(),
                    self.client.clone(),
                    "parse-config",
                    "Parsing configuration",
                    Some(self.coordinator.clone()),
                )
                .await;

                if result.success {
                    self.deps_update_v2(
                        "parseConfiguration",
                        platform_docs_root,
                        Some(config_root),
                    )
                    .await;
                    self.sync_v2_globals().await;
                }

                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.cache.getStats" => {
                let config_path = resolve_cache_config_path(&params, &self.config).await?;
                let scope = self
                    .coordinator
                    .cache_scope_for_config_path(Path::new(&config_path))
                    .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(e.to_string()))?;
                let result = handle_cache_stats(self.coordinator.clone(), scope)
                    .await
                    .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.cache.clear" => {
                let config_path = resolve_cache_config_path(&params, &self.config).await?;
                let scope = self
                    .coordinator
                    .cache_scope_for_config_path(Path::new(&config_path))
                    .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(e.to_string()))?;
                let result = handle_cache_clear(self.coordinator.clone(), scope)
                    .await
                    .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            "bsl.cache.setEnabled" => {
                if params.arguments.is_empty() {
                    return Err(tower_lsp::jsonrpc::Error::invalid_params(
                        "Missing enabled flag",
                    ));
                }

                let request: CacheToggleParams =
                    serde_json::from_value(params.arguments[0].clone()).map_err(|e| {
                        tower_lsp::jsonrpc::Error::invalid_params(format!(
                            "Invalid parameters: {}",
                            e
                        ))
                    })?;
                let result = handle_cache_set_enabled(self.coordinator.clone(), request.enabled)
                    .await
                    .map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
                Ok(Some(serde_json::to_value(result).map_err(|_| {
                    tower_lsp::jsonrpc::Error::internal_error()
                })?))
            }
            _ => {
                warn!("Unknown command: {}", params.command);
                Err(tower_lsp::jsonrpc::Error::method_not_found())
            }
        }
    }
}

async fn resolve_cache_config_path(
    params: &ExecuteCommandParams,
    config: &tokio::sync::RwLock<Option<LspConfig>>,
) -> JsonRpcResult<String> {
    if !params.arguments.is_empty() {
        let request: CacheCommandParams = serde_json::from_value(params.arguments[0].clone())
            .map_err(|e| {
                tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid parameters: {}", e))
            })?;
        if let Some(path) = request.configuration_path {
            return Ok(path);
        }
    }

    let config_guard = config.read().await;
    if let Some(cfg) = config_guard.as_ref() {
        if let Some(path) = cfg.configuration_path.clone() {
            return Ok(path);
        }
    }

    Err(tower_lsp::jsonrpc::Error::invalid_params(
        "Missing configuration path",
    ))
}

fn normalize_lsp_config(config: &mut LspConfig) {
    config.platform_docs_archive = normalize_optional_string(config.platform_docs_archive.clone());
    config.configuration_path = normalize_optional_string(config.configuration_path.clone());
    config.platform_version = normalize_optional_string(config.platform_version.clone());
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{
        advance_large_churn_state, completion_canary_routing_key,
        completion_dispatch_enabled_for_mode, completion_publish_allowed,
        completion_route_canary_event_driven, completion_routing_plan,
        completion_shadow_internal_trigger_payload, completion_shadow_internal_trigger_value,
        should_defer_heavy_diagnostics_for_large_churn, should_schedule_profile,
        CompletionResponseRoute, LargeChurnTransition,
    };
    use bsl_runtime::application::{
        CompletionMode, DiagnosticsProfile, DiagnosticsTrigger, ScaleAwareDiagnosticsKnobs,
    };
    use std::time::{Duration, Instant};
    use tower_lsp::lsp_types::{Position, Url};

    #[test]
    fn idle_heavy_runs_for_save_trigger_even_when_flow_sensitive_disabled() {
        assert!(!should_schedule_profile(
            DiagnosticsTrigger::DidChange,
            DiagnosticsProfile::IdleHeavy,
            false
        ));
        assert!(should_schedule_profile(
            DiagnosticsTrigger::DidSave,
            DiagnosticsProfile::IdleHeavy,
            false
        ));
        assert!(should_schedule_profile(
            DiagnosticsTrigger::Idle,
            DiagnosticsProfile::IdleHeavy,
            false
        ));
        assert!(should_schedule_profile(
            DiagnosticsTrigger::DidChange,
            DiagnosticsProfile::IdleHeavy,
            true
        ));
        assert!(should_schedule_profile(
            DiagnosticsTrigger::DidChange,
            DiagnosticsProfile::Fast,
            false
        ));
        assert!(should_schedule_profile(
            DiagnosticsTrigger::DidChange,
            DiagnosticsProfile::DebouncedFull,
            false
        ));
    }

    #[test]
    fn large_churn_defers_heavy_profiles_for_did_change_only() {
        assert!(should_defer_heavy_diagnostics_for_large_churn(
            DiagnosticsTrigger::DidChange,
            DiagnosticsProfile::DebouncedFull,
            true
        ));
        assert!(should_defer_heavy_diagnostics_for_large_churn(
            DiagnosticsTrigger::DidChange,
            DiagnosticsProfile::IdleHeavy,
            true
        ));
        assert!(!should_defer_heavy_diagnostics_for_large_churn(
            DiagnosticsTrigger::DidChange,
            DiagnosticsProfile::Fast,
            true
        ));
        assert!(!should_defer_heavy_diagnostics_for_large_churn(
            DiagnosticsTrigger::DidSave,
            DiagnosticsProfile::DebouncedFull,
            true
        ));
        assert!(!should_defer_heavy_diagnostics_for_large_churn(
            DiagnosticsTrigger::DidChange,
            DiagnosticsProfile::DebouncedFull,
            false
        ));
    }

    #[test]
    fn large_churn_state_enters_on_threshold_and_exits_after_window_reset() {
        let knobs = ScaleAwareDiagnosticsKnobs {
            enabled: true,
            large_doc_bytes: 64 * 1024,
            large_doc_lines: 2_000,
            churn_window: Duration::from_millis(150),
            churn_min_changes: 3,
        };
        let start = Instant::now();
        let mut state = crate::server::ScaleAwareChurnStateV2 {
            window_started_at: start,
            changes_in_window: 0,
            large_churn_active: false,
        };

        assert_eq!(
            advance_large_churn_state(&mut state, start, true, knobs),
            LargeChurnTransition::None
        );
        assert_eq!(
            advance_large_churn_state(&mut state, start + Duration::from_millis(40), true, knobs),
            LargeChurnTransition::None
        );
        assert_eq!(
            advance_large_churn_state(&mut state, start + Duration::from_millis(80), true, knobs),
            LargeChurnTransition::Entered
        );
        assert!(state.large_churn_active);
        assert_eq!(state.changes_in_window, 3);

        assert_eq!(
            advance_large_churn_state(&mut state, start + Duration::from_millis(300), true, knobs),
            LargeChurnTransition::Exited
        );
        assert!(!state.large_churn_active);
        assert_eq!(state.changes_in_window, 1);
    }

    #[test]
    fn completion_publish_guard_requires_latest_epoch() {
        assert!(completion_publish_allowed(3, Some(3)));
        assert!(completion_publish_allowed(1, None));
        assert!(!completion_publish_allowed(3, Some(4)));
    }

    #[test]
    fn completion_publish_guard_rejects_superseded_epochs_in_burst() {
        let latest_epoch = Some(11);
        assert!(!completion_publish_allowed(9, latest_epoch));
        assert!(!completion_publish_allowed(10, latest_epoch));
        assert!(completion_publish_allowed(11, latest_epoch));
    }

    #[test]
    fn completion_dispatch_disabled_only_for_off_mode() {
        assert!(!completion_dispatch_enabled_for_mode(CompletionMode::Off));
        assert!(completion_dispatch_enabled_for_mode(CompletionMode::Shadow));
        assert!(completion_dispatch_enabled_for_mode(CompletionMode::Canary));
        assert!(completion_dispatch_enabled_for_mode(CompletionMode::On));
    }

    #[test]
    fn completion_canary_routing_is_deterministic_for_same_key() {
        let key = "file:///test.bsl:10:5:invoked:0:3";
        let first = completion_route_canary_event_driven(key, 37);
        for _ in 0..16 {
            assert_eq!(completion_route_canary_event_driven(key, 37), first);
        }
    }

    #[test]
    fn completion_canary_routing_uses_threshold_bucket() {
        let key = "file:///test.bsl:1:2:trigger_character:46:9";
        let bucket = (bsl_shared::utils::hash::hash_content(key) % 100) as u8;
        assert!(!completion_route_canary_event_driven(key, bucket));
        let next_threshold = bucket.saturating_add(1).max(1);
        assert!(completion_route_canary_event_driven(key, next_threshold));
    }

    #[test]
    fn completion_routing_plan_follows_mode_contract() {
        let key = "file:///test.bsl:2:4:invoked:0:-1";

        let off = completion_routing_plan(CompletionMode::Off, 100, key);
        assert_eq!(off.response_route, CompletionResponseRoute::Legacy);
        assert!(!off.run_shadow_event_driven);

        let shadow = completion_routing_plan(CompletionMode::Shadow, 100, key);
        assert_eq!(shadow.response_route, CompletionResponseRoute::Legacy);
        assert!(shadow.run_shadow_event_driven);

        let canary_zero = completion_routing_plan(CompletionMode::Canary, 0, key);
        assert_eq!(canary_zero.response_route, CompletionResponseRoute::Legacy);
        assert!(!canary_zero.run_shadow_event_driven);

        let canary_hundred = completion_routing_plan(CompletionMode::Canary, 100, key);
        assert_eq!(
            canary_hundred.response_route,
            CompletionResponseRoute::EventDriven
        );
        assert!(!canary_hundred.run_shadow_event_driven);

        let on = completion_routing_plan(CompletionMode::On, 0, key);
        assert_eq!(on.response_route, CompletionResponseRoute::EventDriven);
        assert!(!on.run_shadow_event_driven);
    }

    #[test]
    fn completion_mode_parity_groups_are_stable_for_fixed_revision() {
        let key = "file:///test_fixed_revision.bsl:15:9:trigger_character:46:42";

        let off = completion_routing_plan(CompletionMode::Off, 50, key).response_route;
        let shadow = completion_routing_plan(CompletionMode::Shadow, 50, key).response_route;
        let canary_zero = completion_routing_plan(CompletionMode::Canary, 0, key).response_route;
        let canary_hundred =
            completion_routing_plan(CompletionMode::Canary, 100, key).response_route;
        let on = completion_routing_plan(CompletionMode::On, 50, key).response_route;

        assert_eq!(off, CompletionResponseRoute::Legacy);
        assert_eq!(shadow, CompletionResponseRoute::Legacy);
        assert_eq!(canary_zero, CompletionResponseRoute::Legacy);
        assert_eq!(canary_hundred, CompletionResponseRoute::EventDriven);
        assert_eq!(on, CompletionResponseRoute::EventDriven);
    }

    #[test]
    fn completion_shadow_internal_trigger_roundtrip_keeps_original_char() {
        let dot_encoded = completion_shadow_internal_trigger_value(Some('.'));
        assert_eq!(
            completion_shadow_internal_trigger_payload(&dot_encoded),
            Some(Some('.'))
        );

        let none_encoded = completion_shadow_internal_trigger_value(None);
        assert_eq!(
            completion_shadow_internal_trigger_payload(&none_encoded),
            Some(None)
        );
    }

    #[test]
    fn completion_canary_routing_key_is_stable_for_same_inputs() {
        let uri = Url::parse("file:///test.bsl").expect("url");
        let first = completion_canary_routing_key(
            &uri,
            Position::new(10, 4),
            "invoked",
            Some('.'),
            Some(7),
        );
        let second = completion_canary_routing_key(
            &uri,
            Position::new(10, 4),
            "invoked",
            Some('.'),
            Some(7),
        );
        assert_eq!(first, second);
    }
}
