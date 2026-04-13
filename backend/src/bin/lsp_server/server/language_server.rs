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
use tower_lsp::LanguageServer;
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::*;
use tracing::{debug, error, info, warn};

use bsl_backend::data::loaders::progress::{IndexingPhase, ProgressUpdate};
use bsl_backend::system::{StartupInputs, startup_v2};
use bsl_shared::api::semantic_dtos::{GetSemanticHtmlRequest, GetSemanticTreeRequest};
use bsl_shared::utils::hash::hash_content;

use crate::commands::{
    CacheCommandParams, CacheToggleParams, GetAllTypesRequest, ParseConfigurationParams,
    QueryTypeParams, SearchTypesRequest, handle_cache_clear, handle_cache_set_enabled,
    handle_cache_stats, handle_get_all_types, handle_get_type_repository_stats,
    handle_parse_configuration, handle_query_type, handle_search_types, semantic_html_from_tree,
    semantic_tree_from_ir,
};
use crate::config::{BslSettings, LspConfig};
use crate::handlers::{
    RenameError, apply_text_edit, build_document_symbols, build_workspace_symbols,
    format_bsl_range_to_edits, format_bsl_to_edits, handle_code_actions_v2,
    handle_completion_resolve, handle_goto_definition_v2, handle_hover_v2, handle_inlay_hints_v2,
    handle_prepare_rename, handle_references, handle_rename, handle_signature_help_v2,
};
use crate::progress::log_progress_to_file;
use crate::progress_bridge::{LspWorkDoneReporter, ProgressReporter};
use crate::types::{GetCurrentContextParams, ServerStatus, ServerStatusParams};

use super::BslLanguageServer;

#[path = "language_server/helpers.rs"]
mod helpers;
#[path = "language_server/impl_completion.rs"]
mod impl_completion;
#[path = "language_server/impl_completion_helpers.rs"]
mod impl_completion_helpers;
#[path = "language_server/impl_document_sync.rs"]
mod impl_document_sync;
#[path = "language_server/impl_features_a.rs"]
mod impl_features_a;
#[path = "language_server/impl_features_b.rs"]
mod impl_features_b;
#[path = "language_server/impl_features_c.rs"]
mod impl_features_c;
#[path = "language_server/impl_init_config.rs"]
mod impl_init_config;

use self::helpers::*;

#[cfg(test)]
pub(crate) fn did_change_inline_parse_delay_active_for_test() -> bool {
    impl_document_sync::did_change_inline_parse_delay_active_for_test()
}

#[cfg(test)]
pub(crate) fn did_save_inline_parse_delay_active_for_test() -> bool {
    impl_document_sync::did_save_inline_parse_delay_active_for_test()
}

#[cfg(test)]
pub(crate) fn reset_completion_checkpoint_hits_for_test() {
    impl_completion::reset_completion_checkpoint_hits_for_test()
}

#[cfg(test)]
pub(crate) fn completion_checkpoint_hits_for_test(checkpoint: &'static str) -> u64 {
    impl_completion::completion_checkpoint_hits_for_test(checkpoint)
}

#[tower_lsp::async_trait]
impl LanguageServer for BslLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> JsonRpcResult<InitializeResult> {
        self.lsp_initialize(params).await
    }

    async fn initialized(&self, params: InitializedParams) {
        self.lsp_initialized(params).await
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        self.lsp_shutdown().await
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.lsp_did_change_configuration(params).await
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.lsp_did_open(params).await
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.lsp_did_change(params).await
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.lsp_did_save(params).await
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.lsp_did_close(params).await
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> JsonRpcResult<Option<Vec<TextEdit>>> {
        self.lsp_formatting(params).await
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> JsonRpcResult<Option<Vec<TextEdit>>> {
        self.lsp_range_formatting(params).await
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> JsonRpcResult<Option<DocumentSymbolResponse>> {
        self.lsp_document_symbol(params).await
    }

    async fn references(&self, params: ReferenceParams) -> JsonRpcResult<Option<Vec<Location>>> {
        self.lsp_references(params).await
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> JsonRpcResult<Option<PrepareRenameResponse>> {
        self.lsp_prepare_rename(params).await
    }

    async fn rename(&self, params: RenameParams) -> JsonRpcResult<Option<WorkspaceEdit>> {
        self.lsp_rename(params).await
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> JsonRpcResult<Option<Vec<SymbolInformation>>> {
        self.lsp_symbol(params).await
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> JsonRpcResult<Option<CompletionResponse>> {
        self.lsp_completion(params).await
    }

    async fn completion_resolve(&self, item: CompletionItem) -> JsonRpcResult<CompletionItem> {
        self.lsp_completion_resolve(item).await
    }

    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        self.lsp_hover(params).await
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> JsonRpcResult<Option<Vec<InlayHint>>> {
        self.lsp_inlay_hint(params).await
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> JsonRpcResult<Option<CodeActionResponse>> {
        self.lsp_code_action(params).await
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> JsonRpcResult<Option<GotoDefinitionResponse>> {
        self.lsp_goto_definition(params).await
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> JsonRpcResult<Option<SignatureHelp>> {
        self.lsp_signature_help(params).await
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> JsonRpcResult<Option<serde_json::Value>> {
        self.lsp_execute_command(params).await
    }
}

#[cfg(test)]
#[path = "language_server/tests.rs"]
mod tests;
