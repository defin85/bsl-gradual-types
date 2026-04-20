use super::*;
use crate::server::ReadyParseSnapshotStateV2;
use crate::server::SnapshotBuildFailureStateV2;
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
use bsl_analysis_v2::{LineIndex, ParseChangedRange, ParseSnapshot};
use bsl_backend::perf_gate_evaluator::{
    get_report_u64, validate_parity_cutover_evidence, PARITY_DRIFT_RATE_MAX_FOR_CUTOVER,
    PARITY_PAIRS_TOTAL_MIN_FOR_CUTOVER,
};
use bsl_backend::presentation::web::{create_router, AppState};
use bsl_backend::system::{
    build_deps_bundle_v2, EffectiveStartupInputs, IndexItem, IndexItemKind, IndexKind,
    IndexSnapshot, IndexSnapshotId, TypeKind,
};
use bsl_shared::api::dtos::{
    SnapshotPhaseDto, SnapshotReadinessDto, SnapshotReadinessStateDto, SnapshotTaskStateDto,
};
use bsl_syntax::ParseOptions;
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
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    DocumentFormattingParams, DocumentRangeFormattingParams, DocumentSymbolParams,
    DocumentSymbolResponse, FormattingOptions, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, InitializeParams, InitializedParams, InlayHint, InlayHintLabel,
    InlayHintParams, Location, MarkedString, PartialResultParams, Position, PrepareRenameResponse,
    PublishDiagnosticsParams, Range, ReferenceContext, ReferenceParams, RenameParams,
    SymbolInformation, SymbolKind, TextDocumentContentChangeEvent, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams, Url, VersionedTextDocumentIdentifier,
    WorkDoneProgressParams, WorkspaceEdit, WorkspaceSymbolParams,
};
use tower_lsp::LanguageServer;
use tower_lsp::LspService;
use tree_sitter::{Parser as TreeSitterParser, Tree};

mod support;
use support::*;

mod current_context_and_scale;
mod current_revision_head;
mod diagnostics_save_timeline;
mod did_save_followup;
mod interactive_completion;
mod live_reports;
mod lsp_features_and_observability;
mod snapshot_status_and_perf;
mod startup_and_fastlane;

include!("root/root_helpers.rs");
include!("root/live_transport_completion_timeline.rs");
include!("root/scale_aware_support.rs");
include!("root/completion_supersession_and_parse_gap.rs");
include!("root/document_symbol_gap_and_head.rs");
include!("root/current_context_and_metrics.rs");
include!("root/scale_aware_profiles.rs");
