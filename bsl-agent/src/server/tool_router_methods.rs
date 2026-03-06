use super::*;

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

    pub(super) async fn register_batch_job(
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

    pub(super) async fn cancel_stale_batch_jobs(&self, session_id: &str, min_revision: u64) {
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
        Content::json(help::build_mcp_help_response(tool_name)?)
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
    pub(super) async fn workspace_documents_set(
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
        tracing::info!(
            session_id = %params.session_id,
            scope = ?params.scope,
            limit = params.limit,
            include_flow_sensitive = params.include_flow_sensitive,
            profile = profile.as_str(),
            cpu_class = ?job_class,
            "starting diagnostics job"
        );

        let session_manager = Arc::clone(&self.session_manager);
        let session_id = params.session_id.clone();
        let session_id_for_job = session_id.clone();
        let job_id = self
            .job_manager
            .spawn_with_class("bsl_diagnostics", job_class, move |ctx| async move {
                let progress = SemanticJobProgress::new(ctx.clone(), "bsl_diagnostics");
                tracing::debug!(
                    job_id = %ctx.job_id(),
                    session_id = %params.session_id,
                    "diagnostics job entered async closure"
                );
                let response = session_manager
                    .bsl_diagnostics_with_progress(params, Some(progress))
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
                tracing::info!(
                    job_id = %ctx.job_id(),
                    session_id = %session_id_for_job,
                    diagnostics = response.diagnostics.len(),
                    truncated = response.truncated,
                    "diagnostics job produced response"
                );
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
                let progress = SemanticJobProgress::new(ctx, "bsl_symbol_search");
                let response = session_manager
                    .bsl_symbol_search_with_progress(params, Some(progress))
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
                let progress = SemanticJobProgress::new(ctx, "bsl_type_at_position");
                let response = session_manager
                    .bsl_type_at_position_with_progress(params, Some(progress))
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
                let progress = SemanticJobProgress::new(ctx, "bsl_members");
                let response = session_manager
                    .bsl_members_with_progress(params, Some(progress))
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
                let progress = SemanticJobProgress::new(ctx, "bsl_definition");
                let response = session_manager
                    .bsl_definition_with_progress(params, Some(progress))
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
                let progress = SemanticJobProgress::new(ctx, "bsl_references");
                let response = session_manager
                    .bsl_references_with_progress(params, Some(progress))
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
                let progress = SemanticJobProgress::new(ctx, "context_pack");
                let response = session_manager
                    .context_pack_with_progress(params, Some(progress))
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
                let progress = SemanticJobProgress::new(ctx, "context_expand");
                let response = session_manager
                    .context_expand_with_progress(params, Some(progress))
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
    pub(super) async fn job_cancel(
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
                    DiagnosticsDisposition::ClientCancel,
                )
                .await;
            }
        }
        Content::json(response)
    }
}
