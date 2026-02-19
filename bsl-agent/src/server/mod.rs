use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use bsl_runtime::application::{
    cpu_work_class_for_operation, diagnostics_execution_plan, DiagnosticsDisposition,
    DiagnosticsProfile, DiagnosticsTrigger, SemanticOperation,
};

use crate::jobs::JobManager;
use crate::session::SessionManager;
use crate::types::{
    BslAgentError, BuildInfoResponse, JobStartResponse, JobStateDto, McpHelpResponse, UiUrlResponse,
};

pub mod types;

use types::{
    BslDefinitionParams, BslDiagnosticsParams, BslMembersParams, BslReferencesParams,
    BslSymbolSearchParams, BslTypeAtPositionParams, BslTypeGetParams, BslTypesListParams,
    BslTypesSearchParams, BuildInfoParams, ContextExpandParams, ContextPackParams, JobCancelParams,
    JobResultParams, JobStatusParams, JobWaitParams, McpHelpParams, UiUrlParams,
    WorkspaceCloseParams, WorkspaceGetObservabilityMetricsParams, WorkspaceGetSettingsParams,
    WorkspaceListParams, WorkspaceOpenParams, WorkspaceResumeParams, WorkspaceStatusParams,
    WorkspaceUpdateSettingsParams,
};
use types::{WorkspaceDocumentsClearParams, WorkspaceDocumentsSetParams};

#[derive(Debug, Clone)]
struct TrackedBatchJob {
    job_id: String,
    analysis_revision: u64,
    profile: DiagnosticsProfile,
}

#[derive(Clone)]
pub struct BslAgentHandler {
    session_manager: Arc<SessionManager>,
    job_manager: Arc<JobManager>,
    batch_jobs_by_session: Arc<RwLock<HashMap<String, Vec<TrackedBatchJob>>>>,
    ui_url: Option<String>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BslAgentHandler {
    pub fn new() -> Self {
        Self::with_state(Arc::new(SessionManager::new()), Arc::new(JobManager::new()))
    }

    pub fn with_state(session_manager: Arc<SessionManager>, job_manager: Arc<JobManager>) -> Self {
        Self {
            session_manager,
            job_manager,
            batch_jobs_by_session: Arc::new(RwLock::new(HashMap::new())),
            ui_url: None,
            tool_router: Self::tool_router(),
        }
    }

    pub fn session_manager(&self) -> Arc<SessionManager> {
        Arc::clone(&self.session_manager)
    }

    pub fn job_manager(&self) -> Arc<JobManager> {
        Arc::clone(&self.job_manager)
    }

    pub fn set_ui_url(&mut self, ui_url: String) {
        self.ui_url = Some(ui_url);
    }

    async fn record_diagnostics_pipeline_event_best_effort(
        &self,
        session_id: &str,
        trigger: DiagnosticsTrigger,
        profile: DiagnosticsProfile,
        reason: DiagnosticsDisposition,
    ) {
        let _ = self
            .session_manager
            .record_diagnostics_pipeline_event(session_id, trigger, profile, reason)
            .await;
    }

    async fn register_batch_job(
        &self,
        session_id: &str,
        analysis_revision: u64,
        job_id: &str,
        profile: DiagnosticsProfile,
    ) {
        let mut jobs = self.batch_jobs_by_session.write().await;
        jobs.entry(session_id.to_string())
            .or_default()
            .push(TrackedBatchJob {
                job_id: job_id.to_string(),
                analysis_revision,
                profile,
            });
    }

    async fn take_batch_job_by_id(&self, job_id: &str) -> Option<(String, DiagnosticsProfile)> {
        let mut jobs = self.batch_jobs_by_session.write().await;
        let mut found: Option<(String, DiagnosticsProfile)> = None;
        let mut empty_sessions = Vec::new();

        for (session_id, entries) in jobs.iter_mut() {
            if let Some(index) = entries.iter().position(|entry| entry.job_id == job_id) {
                let removed = entries.swap_remove(index);
                found = Some((session_id.clone(), removed.profile));
            }
            if entries.is_empty() {
                empty_sessions.push(session_id.clone());
            }
            if found.is_some() {
                break;
            }
        }

        for session_id in empty_sessions {
            jobs.remove(&session_id);
        }

        found
    }

    async fn cancel_stale_batch_jobs(&self, session_id: &str, min_revision: u64) {
        let stale_jobs = {
            let mut jobs = self.batch_jobs_by_session.write().await;
            let Some(entries) = jobs.get_mut(session_id) else {
                return;
            };

            let mut stale = Vec::new();
            entries.retain(|entry| {
                if entry.analysis_revision < min_revision {
                    stale.push(entry.clone());
                    false
                } else {
                    true
                }
            });

            if entries.is_empty() {
                jobs.remove(session_id);
            }

            stale
        };

        for job in stale_jobs {
            let is_active = self
                .job_manager
                .status(&job.job_id)
                .await
                .ok()
                .is_some_and(|status| {
                    matches!(status.state, JobStateDto::Queued | JobStateDto::Running)
                });
            if !is_active {
                continue;
            }

            let _ = self.job_manager.cancel(&job.job_id).await;
            self.record_diagnostics_pipeline_event_best_effort(
                session_id,
                DiagnosticsTrigger::DocumentsSet,
                job.profile,
                DiagnosticsDisposition::SupersededGeneration,
            )
            .await;
        }
    }

    #[tool(description = "On-demand help: quickstart + per-tool examples (read-only)")]
    async fn mcp_help(
        &self,
        Parameters(params): Parameters<McpHelpParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let tool_name = params
            .tool_name
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        let mut quickstart = vec![
            "workspace_open(roots[], platform_docs_archive?, configuration_path?, platform_version?, mode?)".to_string(),
            "workspace_status(session_id) poll until ready=true".to_string(),
            "workspace_get_settings/workspace_update_settings use camelCase overrides payload (legacy snake_case accepted)".to_string(),
            "workspace_get_observability_metrics(session_id)".to_string(),
            "workspace_documents_set(session_id, files[], mark_hot=true)".to_string(),
            "bsl_diagnostics_start(...) / bsl_symbol_search_start(...) / context_pack_start(...)".to_string(),
            "bsl_types_list_start(...) / bsl_types_search_start(...) / bsl_type_get_start(...)".to_string(),
            "job_wait(job_id, timeout_ms) until state=succeeded".to_string(),
            "job_result(job_id)".to_string(),
        ];

        let mut notes = vec![
            "Multi-root: prefer absolute paths; server resolves via deterministic longest-prefix match against roots[].".to_string(),
            "If configuration_path is set and platform_version is omitted, bsl-agent tries to infer platform_version from config dump; otherwise INVALID_PARAMS.".to_string(),
            "Async: all semantic tools are *_start and return job_id; fetch result via job_result.".to_string(),
        ];

        let mut examples: Vec<serde_json::Value> = Vec::new();
        if let Some(name) = tool_name.as_deref() {
            match name {
                "workspace_open" => {
                    examples.push(serde_json::json!({
                        "name": "workspace_open",
                        "arguments": { "roots": ["/abs/path/to/workspace"], "mode": "default" }
                    }));
                    examples.push(serde_json::json!({
                        "name": "workspace_open",
                        "arguments": { "roots": ["/ws/config", "/ws/ext1"], "configuration_path": "/ws/config", "platform_version": "8.3.25" }
                    }));
                    notes.push("Single-session: calling workspace_open again with different params requires workspace_close first.".to_string());
                }
                "workspace_documents_set" => {
                    examples.push(serde_json::json!({
                        "name": "workspace_documents_set",
                        "arguments": {
                            "session_id": "<session_id>",
                            "files": ["/ws/ext1/src/CommonModules/Foo/Module.bsl"],
                            "mark_hot": true
                        }
                    }));
                    examples.push(serde_json::json!({
                        "name": "workspace_documents_set",
                        "arguments": {
                            "session_id": "<session_id>",
                            "files": [
                                { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" }, "text": "Procedure P()\nEndProcedure\n", "version": 1 }
                            ],
                            "mark_hot": true
                        }
                    }));
                    notes.push("When text is provided, version is required.".to_string());
                }
                "workspace_documents_clear" => {
                    examples.push(serde_json::json!({
                        "name": "workspace_documents_clear",
                        "arguments": {
                            "session_id": "<session_id>",
                            "documents": [{ "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" }],
                            "clear_hot": true
                        }
                    }));
                }
                "bsl_diagnostics_start" => {
                    examples.push(serde_json::json!({
                        "name": "bsl_diagnostics_start",
                        "arguments": { "session_id": "<session_id>", "scope": "hot", "limit": 200 }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_diagnostics_start",
                        "arguments": { "session_id": "<session_id>", "scope": { "kind": "project" }, "limit": 200 }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_diagnostics_start",
                        "arguments": {
                            "session_id": "<session_id>",
                            "scope": { "kind": "file", "document": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                            "limit": 200
                        }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_diagnostics_start",
                        "arguments": {
                            "session_id": "<session_id>",
                            "scope": "hot",
                            "limit": 200,
                            "include_flow_sensitive": true
                        }
                    }));
                    notes.push("scope string supports only: project|hot. For a single file use tagged: {kind:\"file\",document:...}.".to_string());
                    notes.push("Flow-sensitive is opt-in: pass include_flow_sensitive=true. Responses include flow_sensitive_enabled (bool).".to_string());
                }
                "bsl_type_at_position_start" => {
                    examples.push(serde_json::json!({
                        "name": "bsl_type_at_position_start",
                        "arguments": {
                            "session_id": "<session_id>",
                            "file": { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                            "position": { "line": 10, "character": 15 },
                            "include_flow_sensitive": false
                        }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_type_at_position_start",
                        "arguments": {
                            "session_id": "<session_id>",
                            "file": { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                            "position": { "line": 10, "character": 15 },
                            "include_flow_sensitive": true
                        }
                    }));
                    notes.push("Flow-sensitive is opt-in: include_flow_sensitive defaults to false. Responses include flow_sensitive_enabled (bool).".to_string());
                }
                "bsl_members_start" => {
                    examples.push(serde_json::json!({
                        "name": "bsl_members_start",
                        "arguments": {
                            "session_id": "<session_id>",
                            "file": { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                            "position": { "line": 10, "character": 15 },
                            "limit": 200,
                            "include_flow_sensitive": false
                        }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_members_start",
                        "arguments": {
                            "session_id": "<session_id>",
                            "file": { "doc": { "path": "/ws/ext1/src/CommonModules/Foo/Module.bsl" } },
                            "position": { "line": 10, "character": 15 },
                            "limit": 200,
                            "include_flow_sensitive": true
                        }
                    }));
                    notes.push("Flow-sensitive is opt-in: include_flow_sensitive defaults to false. Responses include flow_sensitive_enabled (bool).".to_string());
                }
                "job_wait" => {
                    examples.push(serde_json::json!({
                        "name": "job_wait",
                        "arguments": { "job_id": "<job_id>", "timeout_ms": 5000 }
                    }));
                }
                "bsl_types_list_start" => {
                    examples.push(serde_json::json!({
                        "name": "bsl_types_list_start",
                        "arguments": { "session_id": "<session_id>", "page": 1, "limit": 50, "view": "names_only" }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_types_list_start",
                        "arguments": { "session_id": "<session_id>", "page": 1, "limit": 50, "source": "configuration", "view": "summary" }
                    }));
                }
                "bsl_types_search_start" => {
                    examples.push(serde_json::json!({
                        "name": "bsl_types_search_start",
                        "arguments": { "session_id": "<session_id>", "query": "Документ", "limit": 200, "view": "summary" }
                    }));
                    examples.push(serde_json::json!({
                        "name": "bsl_types_search_start",
                        "arguments": { "session_id": "<session_id>", "query": "Документы.", "limit": 200, "source": "configuration", "view": "names_only" }
                    }));
                }
                "bsl_type_get_start" => {
                    examples.push(serde_json::json!({
                        "name": "bsl_type_get_start",
                        "arguments": { "session_id": "<session_id>", "type_name": "Документы.ЗаказНаряд", "source": "configuration", "include_methods": false }
                    }));
                    notes.push("bsl_type_get_start returns a TypeDto with properties[] and tabularSections[] for configuration objects.".to_string());
                }
                "workspace_update_settings" => {
                    examples.push(serde_json::json!({
                        "name": "workspace_update_settings",
                        "arguments": {
                            "session_id": "<session_id>",
                            "envOverrides": { "BSL_CACHE_DISABLE": true },
                            "allowDevOverrides": true,
                            "devEnvOverrides": { "BSL_COMPLETION_TRACE": true }
                        }
                    }));
                    examples.push(serde_json::json!({
                        "name": "workspace_update_settings",
                        "arguments": {
                            "session_id": "<session_id>",
                            "env_overrides": { "BSL_CACHE_DISABLE": true }
                        }
                    }));
                }
                "workspace_get_observability_metrics" => {
                    examples.push(serde_json::json!({
                        "name": "workspace_get_observability_metrics",
                        "arguments": { "session_id": "<session_id>" }
                    }));
                }
                other => {
                    return Err(rmcp::ErrorData::invalid_params(
                        format!("unknown tool_name: {other}"),
                        None,
                    ));
                }
            }
        } else {
            quickstart.insert(
                0,
                "mcp_help(tool_name?) for examples (read-only)".to_string(),
            );
            notes.push("Pass tool_name to get examples: workspace_open, workspace_update_settings, workspace_get_observability_metrics, workspace_documents_set, workspace_documents_clear, bsl_diagnostics_start, bsl_type_at_position_start, bsl_members_start, bsl_types_list_start, bsl_types_search_start, bsl_type_get_start, job_wait.".to_string());
        }

        Content::json(McpHelpResponse {
            summary: "bsl-agent MCP help (read-only)".to_string(),
            quickstart,
            tool_name,
            notes,
            examples,
        })
    }

    #[tool(description = "Get local HTTP UI URL (read-only). For usage examples see mcp_help.")]
    async fn ui_url(
        &self,
        Parameters(_params): Parameters<UiUrlParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let url = self.ui_url.clone().or_else(|| {
            crate::ui_discovery::read_registry_record_by_pid(std::process::id())
                .map(|record| record.ui_url)
        });
        Content::json(UiUrlResponse {
            enabled: url.is_some(),
            ui_url: url,
        })
    }

    #[tool(description = "Get build info (read-only). For usage examples see mcp_help.")]
    async fn build_info(
        &self,
        Parameters(_params): Parameters<BuildInfoParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let package = env!("CARGO_PKG_NAME").to_string();
        let version = env!("CARGO_PKG_VERSION").to_string();
        let profile = option_env!("BSL_AGENT_PROFILE")
            .unwrap_or("unknown")
            .to_string();
        let target = option_env!("BSL_AGENT_TARGET")
            .unwrap_or("unknown")
            .to_string();

        let git_sha = option_env!("BSL_AGENT_GIT_SHA")
            .and_then(|value| (!value.trim().is_empty() && value != "unknown").then_some(value))
            .map(str::to_string);
        let git_describe = option_env!("BSL_AGENT_GIT_DESCRIBE")
            .and_then(|value| (!value.trim().is_empty()).then_some(value))
            .map(str::to_string);
        let build_unix_secs =
            option_env!("BSL_AGENT_BUILD_UNIX_SECS").and_then(|value| value.parse::<u64>().ok());

        Content::json(BuildInfoResponse {
            package,
            version,
            profile,
            target,
            git_sha,
            git_describe,
            build_unix_secs,
            pid: std::process::id(),
        })
    }

    #[tool(
        description = "Open workspace session (single-session). Supports multi-root; config may infer platform_version. See mcp_help."
    )]
    async fn workspace_open(
        &self,
        Parameters(params): Parameters<WorkspaceOpenParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .session_manager
            .open(params, Arc::clone(&self.job_manager))
            .await?;
        Content::json(response)
    }

    #[tool(
        description = "Get workspace status/progress. ready=true means startup finished. See mcp_help."
    )]
    async fn workspace_status(
        &self,
        Parameters(params): Parameters<WorkspaceStatusParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.status(&params.session_id).await?;
        Content::json(response)
    }

    #[tool(
        description = "Get unified runtime settings: env overrides, dev overrides (if enabled), and effective snapshot (read-only)."
    )]
    async fn workspace_get_settings(
        &self,
        Parameters(params): Parameters<WorkspaceGetSettingsParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .session_manager
            .settings_get(&params.session_id)
            .await?;
        Content::json(response)
    }

    #[tool(
        description = "Patch unified runtime settings for this session. Use camelCase payload: envOverrides/devEnvOverrides/allowDevOverrides (legacy snake_case aliases are accepted). null removes a key."
    )]
    async fn workspace_update_settings(
        &self,
        Parameters(params): Parameters<WorkspaceUpdateSettingsParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .session_manager
            .settings_update(
                &params.session_id,
                params.env_overrides.as_ref(),
                params.dev_env_overrides.as_ref(),
                params.allow_dev_overrides,
            )
            .await?;
        Content::json(response)
    }

    #[tool(
        description = "Get observability metrics snapshot for a ready workspace session (read-only)."
    )]
    async fn workspace_get_observability_metrics(
        &self,
        Parameters(params): Parameters<WorkspaceGetObservabilityMetricsParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .session_manager
            .observability_metrics_get(&params.session_id)
            .await?;
        Content::json(response)
    }

    #[tool(
        description = "Close workspace session (releases resources). Required to switch params. See mcp_help."
    )]
    async fn workspace_close(
        &self,
        Parameters(params): Parameters<WorkspaceCloseParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        self.session_manager.close(&params.session_id).await?;
        Content::json(serde_json::json!({ "ok": true }))
    }

    #[tool(
        description = "Resume a persisted session by session_id (single-session rules apply). See mcp_help."
    )]
    async fn workspace_resume(
        &self,
        Parameters(params): Parameters<WorkspaceResumeParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .session_manager
            .resume(&params.session_id, Arc::clone(&self.job_manager))
            .await?;
        Content::json(response)
    }

    #[tool(description = "List persisted sessions available for resume. See mcp_help.")]
    async fn workspace_list(
        &self,
        Parameters(_params): Parameters<WorkspaceListParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.session_manager.list().await?;
        Content::json(response)
    }

    #[tool(
        description = "Set overlays and/or mark docs hot. files accept absolute paths; version required with text. See mcp_help."
    )]
    async fn workspace_documents_set(
        &self,
        Parameters(params): Parameters<WorkspaceDocumentsSetParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let previous_revision = self
            .session_manager
            .analysis_revision(&params.session_id)
            .await?;
        let response = self
            .session_manager
            .documents_set(&params.session_id, &params.files, params.mark_hot)
            .await?;
        if response.analysis_revision > previous_revision {
            self.cancel_stale_batch_jobs(&params.session_id, response.analysis_revision)
                .await;
        }
        Content::json(response)
    }

    #[tool(
        description = "Clear overlays and/or remove docs from hot set. documents accept absolute paths. See mcp_help."
    )]
    async fn workspace_documents_clear(
        &self,
        Parameters(params): Parameters<WorkspaceDocumentsClearParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let previous_revision = self
            .session_manager
            .analysis_revision(&params.session_id)
            .await?;
        let response = self
            .session_manager
            .documents_clear(&params.session_id, &params.documents, params.clear_hot)
            .await?;
        if response.analysis_revision > previous_revision {
            self.cancel_stale_batch_jobs(&params.session_id, response.analysis_revision)
                .await;
        }
        Content::json(response)
    }

    #[tool(
        description = "Start diagnostics job. scope: \"project\"|\"hot\" or {kind:\"file\",document:...}. Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_diagnostics_start(
        &self,
        Parameters(params): Parameters<BslDiagnosticsParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        if let types::WorkspaceScope::Simple(value) = &params.scope {
            let trimmed = value.trim();
            if trimmed.eq_ignore_ascii_case("project") || trimmed.eq_ignore_ascii_case("hot") {
                // ok
            } else if trimmed.eq_ignore_ascii_case("file") {
                return Err(rmcp::ErrorData::invalid_params(
                    "scope=\"file\" is not supported as a string; use tagged file scope: {\"kind\":\"file\",\"document\":...}",
                    None,
                ));
            } else {
                return Err(rmcp::ErrorData::invalid_params(
                    format!("unknown scope: {trimmed}"),
                    None,
                ));
            }
        }

        let analysis_revision = self
            .session_manager
            .analysis_revision(&params.session_id)
            .await?;
        let profile = match &params.scope {
            types::WorkspaceScope::Tagged(types::WorkspaceScopeTagged::File { .. }) => {
                DiagnosticsProfile::Fast
            }
            _ => DiagnosticsProfile::DebouncedFull,
        };
        let job_class = diagnostics_execution_plan(profile, false).cpu_class;

        let session_manager = Arc::clone(&self.session_manager);
        let session_id = params.session_id.clone();
        let job_id = self
            .job_manager
            .spawn_with_class("bsl_diagnostics", job_class, move |ctx| async move {
                ctx.set_progress("bsl_diagnostics/running", 0).await;
                let response = session_manager
                    .bsl_diagnostics(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        if !matches!(profile, DiagnosticsProfile::Fast) {
            self.register_batch_job(&session_id, analysis_revision, &job_id, profile)
                .await;
        }
        self.record_diagnostics_pipeline_event_best_effort(
            &session_id,
            DiagnosticsTrigger::JobStart,
            profile,
            DiagnosticsDisposition::Published,
        )
        .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start symbol search job (deterministic). Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_symbol_search_start(
        &self,
        Parameters(params): Parameters<BslSymbolSearchParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let analysis_revision = self
            .session_manager
            .analysis_revision(&params.session_id)
            .await?;
        let session_id = params.session_id.clone();
        let profile = DiagnosticsProfile::DebouncedFull;
        let job_class = cpu_work_class_for_operation(SemanticOperation::SymbolSearch);

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn_with_class("bsl_symbol_search", job_class, move |ctx| async move {
                ctx.set_progress("bsl_symbol_search/running", 0).await;
                let response = session_manager
                    .bsl_symbol_search(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        self.register_batch_job(&session_id, analysis_revision, &job_id, profile)
            .await;
        self.record_diagnostics_pipeline_event_best_effort(
            &session_id,
            DiagnosticsTrigger::JobStart,
            profile,
            DiagnosticsDisposition::Published,
        )
        .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start types list job (platform/config). Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_types_list_start(
        &self,
        Parameters(params): Parameters<BslTypesListParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let status = self.session_manager.status(&params.session_id).await?;
        if !status.ready {
            return Err(rmcp::ErrorData::invalid_params(
                "workspace not ready (startup in progress)",
                None,
            ));
        }
        if params.page < 1 {
            return Err(rmcp::ErrorData::invalid_params("page must be >= 1", None));
        }
        if params.limit < 1 || params.limit > 1000 {
            return Err(rmcp::ErrorData::invalid_params(
                "limit must be in 1..=1000",
                None,
            ));
        }
        if params
            .certainty_level
            .is_some_and(|certainty_level| certainty_level > 100)
        {
            return Err(rmcp::ErrorData::invalid_params(
                "certainty_level must be in 0..=100",
                None,
            ));
        }

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_types_list", move |ctx| async move {
                ctx.set_progress("bsl_types_list/running", 0).await;
                let response = session_manager
                    .bsl_types_list(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                Ok(response)
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start types search job (deterministic). Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_types_search_start(
        &self,
        Parameters(params): Parameters<BslTypesSearchParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let status = self.session_manager.status(&params.session_id).await?;
        if !status.ready {
            return Err(rmcp::ErrorData::invalid_params(
                "workspace not ready (startup in progress)",
                None,
            ));
        }
        let query = params.query.trim();
        if query.is_empty() {
            return Err(rmcp::ErrorData::invalid_params(
                "query must be non-empty",
                None,
            ));
        }
        if params.limit < 1 || params.limit > 1000 {
            return Err(rmcp::ErrorData::invalid_params(
                "limit must be in 1..=1000",
                None,
            ));
        }

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_types_search", move |ctx| async move {
                ctx.set_progress("bsl_types_search/running", 0).await;
                let response = session_manager
                    .bsl_types_search(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                Ok(response)
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start type details job by exact name. Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_type_get_start(
        &self,
        Parameters(params): Parameters<BslTypeGetParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let status = self.session_manager.status(&params.session_id).await?;
        if !status.ready {
            return Err(rmcp::ErrorData::invalid_params(
                "workspace not ready (startup in progress)",
                None,
            ));
        }
        let type_name = params.type_name.trim();
        if type_name.is_empty() {
            return Err(rmcp::ErrorData::invalid_params(
                "type_name must be non-empty",
                None,
            ));
        }

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("bsl_type_get", move |ctx| async move {
                ctx.set_progress("bsl_type_get/running", 0).await;
                let response = session_manager
                    .bsl_type_get(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                Ok(response)
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start type-at-position job. position is 0-based (LSP); file accepts absolute path. See mcp_help."
    )]
    async fn bsl_type_at_position_start(
        &self,
        Parameters(params): Parameters<BslTypeAtPositionParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_id = params.session_id.clone();
        let profile = DiagnosticsProfile::Fast;
        let job_class = cpu_work_class_for_operation(SemanticOperation::TypeAtPosition);

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn_with_class("bsl_type_at_position", job_class, move |ctx| async move {
                ctx.set_progress("bsl_type_at_position/running", 0).await;
                let response = session_manager
                    .bsl_type_at_position(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        self.record_diagnostics_pipeline_event_best_effort(
            &session_id,
            DiagnosticsTrigger::JobStart,
            profile,
            DiagnosticsDisposition::Published,
        )
        .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start members job at position (completion-like). position is 0-based (LSP). See mcp_help."
    )]
    async fn bsl_members_start(
        &self,
        Parameters(params): Parameters<BslMembersParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_id = params.session_id.clone();
        let profile = DiagnosticsProfile::Fast;
        let job_class = cpu_work_class_for_operation(SemanticOperation::Members);

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn_with_class("bsl_members", job_class, move |ctx| async move {
                ctx.set_progress("bsl_members/running", 0).await;
                let response = session_manager
                    .bsl_members(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        self.record_diagnostics_pipeline_event_best_effort(
            &session_id,
            DiagnosticsTrigger::JobStart,
            profile,
            DiagnosticsDisposition::Published,
        )
        .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start definition job (symbol_id or file+position). Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_definition_start(
        &self,
        Parameters(params): Parameters<BslDefinitionParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_id = params.session_id.clone();
        let profile = DiagnosticsProfile::Fast;
        let job_class = cpu_work_class_for_operation(SemanticOperation::Definition);

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn_with_class("bsl_definition", job_class, move |ctx| async move {
                ctx.set_progress("bsl_definition/running", 0).await;
                let response = session_manager
                    .bsl_definition(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        self.record_diagnostics_pipeline_event_best_effort(
            &session_id,
            DiagnosticsTrigger::JobStart,
            profile,
            DiagnosticsDisposition::Published,
        )
        .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start references job for symbol_id. Use job_wait/job_result. See mcp_help."
    )]
    async fn bsl_references_start(
        &self,
        Parameters(params): Parameters<BslReferencesParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let analysis_revision = self
            .session_manager
            .analysis_revision(&params.session_id)
            .await?;
        let session_id = params.session_id.clone();
        let profile = DiagnosticsProfile::DebouncedFull;
        let job_class = cpu_work_class_for_operation(SemanticOperation::References);

        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn_with_class("bsl_references", job_class, move |ctx| async move {
                ctx.set_progress("bsl_references/running", 0).await;
                let response = session_manager
                    .bsl_references(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        self.register_batch_job(&session_id, analysis_revision, &job_id, profile)
            .await;
        self.record_diagnostics_pipeline_event_best_effort(
            &session_id,
            DiagnosticsTrigger::JobStart,
            profile,
            DiagnosticsDisposition::Published,
        )
        .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start context_pack job (hard budget). Use job_wait/job_result. See mcp_help."
    )]
    async fn context_pack_start(
        &self,
        Parameters(params): Parameters<ContextPackParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("context_pack", move |ctx| async move {
                ctx.set_progress("context_pack/running", 0).await;
                let response = session_manager
                    .context_pack(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Start context_expand job for a context_pack item. Use job_wait/job_result. See mcp_help."
    )]
    async fn context_expand_start(
        &self,
        Parameters(params): Parameters<ContextExpandParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let session_manager = Arc::clone(&self.session_manager);
        let job_id = self
            .job_manager
            .spawn("context_expand", move |ctx| async move {
                ctx.set_progress("context_expand/running", 0).await;
                let response = session_manager
                    .context_expand(params)
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                serde_json::to_value(response).map_err(|err| anyhow::anyhow!(err.to_string()))
            })
            .await;
        Content::json(JobStartResponse {
            job_id,
            recommended_poll_ms: Some(200),
        })
    }

    #[tool(
        description = "Get job status/progress. progress=100 only for terminal state. See mcp_help."
    )]
    async fn job_status(
        &self,
        Parameters(params): Parameters<JobStatusParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.job_manager.status(&params.job_id).await?;
        Content::json(response)
    }

    #[tool(
        description = "Long-poll job status up to timeout_ms (no result). Use job_result after succeeded. See mcp_help."
    )]
    async fn job_wait(
        &self,
        Parameters(params): Parameters<JobWaitParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self
            .job_manager
            .wait(&params.job_id, params.timeout_ms)
            .await?;
        Content::json(response)
    }

    #[tool(
        description = "Get job result (succeeded only). Use job_status/job_wait first. See mcp_help."
    )]
    async fn job_result(
        &self,
        Parameters(params): Parameters<JobResultParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let value = self.job_manager.result(&params.job_id).await?;
        Content::json(value)
    }

    #[tool(description = "Cancel a job (best-effort). See mcp_help.")]
    async fn job_cancel(
        &self,
        Parameters(params): Parameters<JobCancelParams>,
    ) -> Result<Content, rmcp::ErrorData> {
        let response = self.job_manager.cancel(&params.job_id).await?;
        if let Some((session_id, profile)) = self.take_batch_job_by_id(&params.job_id).await {
            if response.state == JobStateDto::Canceled {
                self.record_diagnostics_pipeline_event_best_effort(
                    &session_id,
                    DiagnosticsTrigger::JobStart,
                    profile,
                    DiagnosticsDisposition::Cancelled,
                )
                .await;
            }
        }
        Content::json(response)
    }
}

#[tool_handler]
impl ServerHandler for BslAgentHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "bsl-agent: local MCP server (stdio) providing semantic context for BSL projects"
                    .to_string(),
            ),
        }
    }
}

impl Default for BslAgentHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl From<BslAgentError> for rmcp::ErrorData {
    fn from(err: BslAgentError) -> Self {
        err.into_rmcp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::types::{
        DocumentRef, FileRef, JobCancelParams, WorkspaceDocumentsSetFile,
        WorkspaceDocumentsSetParams, WorkspaceOpenParams,
    };
    use crate::types::{JobStateDto, WorkspaceOpenResponse};
    use rmcp::handler::server::wrapper::Parameters;
    use std::time::Duration;

    async fn wait_workspace_ready(
        session_manager: &Arc<SessionManager>,
        job_manager: &Arc<JobManager>,
        open: &WorkspaceOpenResponse,
    ) {
        let startup_job_id = open
            .startup_job_id
            .as_ref()
            .expect("startup_job_id")
            .clone();

        let startup = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let status = job_manager
                    .wait(&startup_job_id, 200)
                    .await
                    .expect("startup wait");
                if !matches!(status.state, JobStateDto::Queued | JobStateDto::Running) {
                    break status;
                }
            }
        })
        .await
        .expect("startup must reach terminal state");
        assert_eq!(
            startup.state,
            JobStateDto::Succeeded,
            "startup job must succeed"
        );

        let ready = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let status = session_manager
                    .status(&open.session_id)
                    .await
                    .expect("status");
                if status.ready {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
        assert!(ready.is_ok(), "workspace must become ready after startup");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancel_stale_batch_jobs_cancels_running_jobs() {
        let session_manager = Arc::new(SessionManager::new());
        let job_manager = Arc::new(JobManager::new_in_memory());
        let handler = BslAgentHandler::with_state(session_manager, Arc::clone(&job_manager));

        let job_id = job_manager
            .spawn_with_class(
                "batch-test",
                cpu_work_class_for_operation(SemanticOperation::SymbolSearch),
                move |_| async move {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    Ok(serde_json::json!({ "ok": true }))
                },
            )
            .await;

        handler
            .register_batch_job("session-1", 1, &job_id, DiagnosticsProfile::DebouncedFull)
            .await;
        handler.cancel_stale_batch_jobs("session-1", 2).await;

        let status = job_manager.status(&job_id).await.expect("status");
        assert_eq!(
            status.state,
            JobStateDto::Canceled,
            "stale running batch job must be canceled on revision advance"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancel_stale_batch_jobs_does_not_rewrite_terminal_state() {
        let session_manager = Arc::new(SessionManager::new());
        let job_manager = Arc::new(JobManager::new_in_memory());
        let handler = BslAgentHandler::with_state(session_manager, Arc::clone(&job_manager));

        let job_id = job_manager
            .spawn_with_class(
                "batch-test-finished",
                cpu_work_class_for_operation(SemanticOperation::SymbolSearch),
                move |_| async move { Ok(serde_json::json!({ "done": true })) },
            )
            .await;
        let waited = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let status = job_manager.status(&job_id).await.expect("status");
                if !matches!(status.state, JobStateDto::Queued | JobStateDto::Running) {
                    break status;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("job should reach terminal state");
        assert_eq!(
            waited.state,
            JobStateDto::Succeeded,
            "test precondition: batch job must finish before stale cancellation"
        );

        handler
            .register_batch_job("session-1", 1, &job_id, DiagnosticsProfile::DebouncedFull)
            .await;
        handler.cancel_stale_batch_jobs("session-1", 2).await;

        let status = job_manager.status(&job_id).await.expect("status");
        assert_eq!(
            status.state,
            JobStateDto::Succeeded,
            "terminal jobs must stay terminal when cleanup scans stale entries"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn job_cancel_records_cancelled_reason_for_tracked_batch_job() {
        let session_manager = Arc::new(SessionManager::new());
        let job_manager = Arc::new(JobManager::new_in_memory());
        let handler =
            BslAgentHandler::with_state(Arc::clone(&session_manager), Arc::clone(&job_manager));

        let root = tempfile::TempDir::new().expect("tempdir");
        let open = session_manager
            .open(
                WorkspaceOpenParams {
                    roots: vec![root.path().to_string_lossy().to_string()],
                    platform_docs_archive: None,
                    platform_version: None,
                    configuration_path: None,
                    mode: None,
                },
                Arc::clone(&job_manager),
            )
            .await
            .expect("workspace_open");
        wait_workspace_ready(&session_manager, &job_manager, &open).await;

        let job_id = job_manager
            .spawn_with_class(
                "batch-cancel-observability",
                cpu_work_class_for_operation(SemanticOperation::SymbolSearch),
                move |_| async move {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    Ok(serde_json::json!({ "ok": true }))
                },
            )
            .await;

        handler
            .register_batch_job(
                &open.session_id,
                open.analysis_revision,
                &job_id,
                DiagnosticsProfile::DebouncedFull,
            )
            .await;

        let _ = handler
            .job_cancel(Parameters(JobCancelParams {
                job_id: job_id.clone(),
            }))
            .await
            .expect("job_cancel");

        let status = job_manager.status(&job_id).await.expect("job status");
        assert_eq!(
            status.state,
            JobStateDto::Canceled,
            "batch job must be canceled"
        );

        let metrics = session_manager
            .observability_metrics_get(&open.session_id)
            .await
            .expect("metrics");
        let counters = metrics
            .metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let key = "intellisense_v2_diagnostics_pipeline_total_origin_agent_trigger_job_start_profile_debounced_full_reason_cancelled";
        let value = counters
            .get(key)
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert!(
            value > 0,
            "expected diagnostics pipeline cancelled metric key {key} to be incremented"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn workspace_documents_set_records_superseded_generation_for_stale_batch_jobs() {
        let session_manager = Arc::new(SessionManager::new());
        let job_manager = Arc::new(JobManager::new_in_memory());
        let handler =
            BslAgentHandler::with_state(Arc::clone(&session_manager), Arc::clone(&job_manager));

        let root = tempfile::TempDir::new().expect("tempdir");
        let open = session_manager
            .open(
                WorkspaceOpenParams {
                    roots: vec![root.path().to_string_lossy().to_string()],
                    platform_docs_archive: None,
                    platform_version: None,
                    configuration_path: None,
                    mode: None,
                },
                Arc::clone(&job_manager),
            )
            .await
            .expect("workspace_open");
        wait_workspace_ready(&session_manager, &job_manager, &open).await;

        let job_id = job_manager
            .spawn_with_class(
                "batch-documents-set-observability",
                cpu_work_class_for_operation(SemanticOperation::SymbolSearch),
                move |_| async move {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    Ok(serde_json::json!({ "ok": true }))
                },
            )
            .await;
        handler
            .register_batch_job(
                &open.session_id,
                open.analysis_revision,
                &job_id,
                DiagnosticsProfile::DebouncedFull,
            )
            .await;

        let overlay_path = root.path().join("Module.bsl");
        let _ = handler
            .workspace_documents_set(Parameters(WorkspaceDocumentsSetParams {
                session_id: open.session_id.clone(),
                files: vec![WorkspaceDocumentsSetFile::File(FileRef {
                    doc: DocumentRef::Path(overlay_path.to_string_lossy().to_string()),
                    text: Some("Procedure T()\nEndProcedure\n".to_string()),
                    version: Some(1),
                })],
                mark_hot: true,
            }))
            .await
            .expect("workspace_documents_set");

        let status = job_manager.status(&job_id).await.expect("job status");
        assert_eq!(
            status.state,
            JobStateDto::Canceled,
            "stale batch job must be canceled after documents_set revision bump"
        );

        let metrics = session_manager
            .observability_metrics_get(&open.session_id)
            .await
            .expect("metrics");
        let counters = metrics
            .metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("metrics.counters object");
        let key = "intellisense_v2_diagnostics_pipeline_total_origin_agent_trigger_documents_set_profile_debounced_full_reason_superseded_generation";
        let value = counters
            .get(key)
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        assert!(
            value > 0,
            "expected diagnostics pipeline superseded metric key {key} to be incremented"
        );
    }
}
