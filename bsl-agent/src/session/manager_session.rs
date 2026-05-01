impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            store: SessionStore::new(),
        }
    }

    fn roots_match(a: &[RootEntry], b: &[RootEntry]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut left: Vec<(&str, &Path)> = a
            .iter()
            .map(|r| (r.root_id.as_str(), r.path.as_path()))
            .collect();
        let mut right: Vec<(&str, &Path)> = b
            .iter()
            .map(|r| (r.root_id.as_str(), r.path.as_path()))
            .collect();

        left.sort_by(|(id_a, path_a), (id_b, path_b)| {
            id_a.cmp(id_b)
                .then_with(|| path_a.as_os_str().cmp(path_b.as_os_str()))
        });
        right.sort_by(|(id_a, path_a), (id_b, path_b)| {
            id_a.cmp(id_b)
                .then_with(|| path_a.as_os_str().cmp(path_b.as_os_str()))
        });

        left == right
    }

    fn open_response_from_session(
        session_id: Uuid,
        session: &WorkspaceSession,
    ) -> WorkspaceOpenResponse {
        WorkspaceOpenResponse {
            session_id: session_id.to_string(),
            roots: session
                .roots
                .iter()
                .map(|root| RootDto {
                    root_id: root.root_id.clone(),
                    path: root.path.to_string_lossy().to_string(),
                })
                .collect(),
            analysis_revision: session.analysis_revision,
            ready: session.startup.is_some(),
            startup_job_id: session.startup_job_id.clone(),
            warnings: workspace_warnings(&session.settings),
            missing_inputs: workspace_missing_inputs(&session.settings),
        }
    }

    fn normalize_optional_path(
        raw: Option<String>,
        field: &str,
    ) -> Result<Option<PathBuf>, rmcp::ErrorData> {
        let Some(raw) = raw else {
            return Ok(None);
        };
        let path = PathBuf::from(&raw);
        let canonical = std::fs::canonicalize(&path).map_err(|_| {
            rmcp::ErrorData::invalid_params(
                format!(
                    "{field} does not exist or is not accessible: {}",
                    path.display()
                ),
                None,
            )
        })?;
        Ok(Some(canonical))
    }

    fn normalize_optional_rules_config_path(
        raw: Option<String>,
        default_root: Option<&Path>,
    ) -> Option<PathBuf> {
        let raw = raw?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        let path = PathBuf::from(trimmed);
        let path = if path.is_absolute() {
            path
        } else {
            default_root.map(|root| root.join(&path)).unwrap_or(path)
        };
        Some(std::fs::canonicalize(&path).unwrap_or(path))
    }

    fn discover_workspace_rules_config(
        configuration_path: Option<&Path>,
        roots: &[RootEntry],
    ) -> Option<PathBuf> {
        bsl_runtime::system::discover_semantic_rules_config(configuration_path).or_else(|| {
            roots.iter().find_map(|root| {
                bsl_runtime::system::discover_semantic_rules_config(Some(root.path.as_path()))
            })
        })
    }

    pub async fn open(
        self: &Arc<Self>,
        params: WorkspaceOpenParams,
        job_manager: Arc<JobManager>,
    ) -> Result<WorkspaceOpenResponse, rmcp::ErrorData> {
        if params.roots.is_empty() {
            return Err(rmcp::ErrorData::invalid_params(
                "roots must be non-empty",
                None,
            ));
        }

        let mut roots = Vec::new();
        let mut root_dtos = Vec::new();
        let mut seen = HashSet::new();

        for root_raw in &params.roots {
            let root_path = PathBuf::from(root_raw);
            let canonical = std::fs::canonicalize(&root_path).map_err(|_| {
                rmcp::ErrorData::invalid_params(format!("root does not exist: {root_raw}"), None)
            })?;

            let metadata = std::fs::metadata(&canonical).map_err(|_| {
                rmcp::ErrorData::invalid_params(
                    format!("root is not accessible: {}", canonical.display()),
                    None,
                )
            })?;
            if !metadata.is_dir() {
                return Err(rmcp::ErrorData::invalid_params(
                    format!("root is not a directory: {}", canonical.display()),
                    None,
                ));
            }

            let root_id = root_id(&canonical);
            if !seen.insert(root_id.clone()) {
                continue;
            }
            root_dtos.push(RootDto {
                root_id: root_id.clone(),
                path: canonical.to_string_lossy().to_string(),
            });
            roots.push(RootEntry {
                root_id,
                path: canonical,
            });
        }

        let configuration_path =
            Self::normalize_optional_path(params.configuration_path, "configuration_path")?;
        let rules_config_path = Self::normalize_optional_rules_config_path(
            params.rules_config_path,
            roots.first().map(|root| root.path.as_path()),
        )
        .or_else(|| Self::discover_workspace_rules_config(configuration_path.as_deref(), &roots));

        let settings = WorkspaceSettings {
            platform_docs_archive: Self::normalize_optional_path(
                params.platform_docs_archive,
                "platform_docs_archive",
            )?,
            platform_version: params.platform_version.and_then(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }),
            configuration_path,
            rules_config_path,
            mode: normalize_mode(params.mode),
            env_overrides: HashMap::new(),
            dev_env_overrides: HashMap::new(),
            allow_dev_overrides: false,
        };
        let settings = {
            let mut settings = settings;
            if settings.configuration_path.is_some() && settings.platform_version.is_none() {
                let inferred = infer_platform_version_from_config_dump(
                    settings
                        .configuration_path
                        .as_deref()
                        .expect("configuration_path is_some"),
                )?;
                settings.platform_version = Some(inferred);
            }
            settings
        };
        let missing_inputs = workspace_missing_inputs(&settings);
        let warnings = workspace_warnings(&settings);

        let existing_response = {
            let sessions = self.sessions.write().await;
            if sessions.is_empty() {
                None
            } else if sessions.len() == 1 {
                let (existing_id, existing) = sessions
                    .iter()
                    .next()
                    .ok_or_else(|| rmcp::ErrorData::invalid_params("no sessions", None))?;
                if Self::roots_match(&existing.roots, &roots) && existing.settings == settings {
                    Some(Self::open_response_from_session(*existing_id, existing))
                } else {
                    return Err(rmcp::ErrorData::invalid_params(SINGLE_SESSION_ERROR, None));
                }
            } else {
                return Err(rmcp::ErrorData::invalid_params(SINGLE_SESSION_ERROR, None));
            }
        };
        if let Some(response) = existing_response {
            return Ok(response);
        }

        let session_id = Uuid::new_v4();
        let created_at = crate::state::now_unix_secs();
        let session = WorkspaceSession {
            roots,
            documents: DocumentStore::default(),
            analysis_revision: 0,
            settings,
            startup: None,
            startup_job_id: None,
            startup_phase: "startup/queued".to_string(),
            startup_progress: 0,
            startup_error: None,
            created_at,
            id_map: IdMap::default(),
            pack_store: PackStore::default(),
        };
        self.sessions.write().await.insert(session_id, session);
        self.persist_session(session_id).await;

        let startup_settings = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&session_id)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            session.settings.clone()
        };

        let session_manager = Arc::clone(self);
        let startup_job_id = job_manager
            .spawn("startup", move |ctx| async move {
                run_startup_job(session_manager, session_id, startup_settings, ctx).await
            })
            .await;

        self.set_startup_job_id(session_id, startup_job_id.clone())
            .await?;

        Ok(WorkspaceOpenResponse {
            session_id: session_id.to_string(),
            roots: root_dtos,
            analysis_revision: 0,
            ready: false,
            startup_job_id: Some(startup_job_id),
            warnings,
            missing_inputs,
        })
    }

    pub async fn status(
        &self,
        session_id: &str,
    ) -> Result<WorkspaceStatusResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;

        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        let _ = session.roots.len();

        if session.startup.is_some() {
            return Ok(WorkspaceStatusResponse {
                ready: true,
                analysis_revision: session.analysis_revision,
                phase: "idle".to_string(),
                progress: ProgressDto { percent: 100 },
                warnings: workspace_warnings(&session.settings),
                missing_inputs: workspace_missing_inputs(&session.settings),
                startup_job_id: session.startup_job_id.clone(),
                error: None,
            });
        }

        Ok(WorkspaceStatusResponse {
            ready: false,
            analysis_revision: session.analysis_revision,
            phase: session.startup_phase.clone(),
            progress: ProgressDto {
                percent: session.startup_progress,
            },
            warnings: workspace_warnings(&session.settings),
            missing_inputs: workspace_missing_inputs(&session.settings),
            startup_job_id: session.startup_job_id.clone(),
            error: session.startup_error.clone(),
        })
    }

    pub async fn http_list_sessions(&self) -> Vec<McpSessionDto> {
        let sessions = self.sessions.read().await;
        let mut result = Vec::with_capacity(sessions.len());
        for (session_id, session) in sessions.iter() {
            let roots = session
                .roots
                .iter()
                .map(|root| McpRootDto {
                    root_id: root.root_id.clone(),
                    path: root.path.to_string_lossy().to_string(),
                })
                .collect();

            if session.startup.is_some() {
                result.push(McpSessionDto {
                    session_id: session_id.to_string(),
                    roots,
                    ready: true,
                    analysis_revision: session.analysis_revision,
                    phase: "idle".to_string(),
                    progress_percent: 100,
                    warnings: workspace_warnings(&session.settings),
                    missing_inputs: workspace_missing_inputs(&session.settings),
                    startup_job_id: session.startup_job_id.clone(),
                    error: None,
                });
            } else {
                result.push(McpSessionDto {
                    session_id: session_id.to_string(),
                    roots,
                    ready: false,
                    analysis_revision: session.analysis_revision,
                    phase: session.startup_phase.clone(),
                    progress_percent: session.startup_progress,
                    warnings: workspace_warnings(&session.settings),
                    missing_inputs: workspace_missing_inputs(&session.settings),
                    startup_job_id: session.startup_job_id.clone(),
                    error: session.startup_error.clone(),
                });
            }
        }

        result.sort_by(|a, b| a.session_id.cmp(&b.session_id));
        result
    }

    fn select_ready_session_uuid(
        sessions: &HashMap<Uuid, WorkspaceSession>,
        session_id: Option<&str>,
    ) -> Result<Uuid, rmcp::ErrorData> {
        if let Some(session_id) = session_id {
            let uuid = parse_session_id(session_id)?;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            if session.startup.is_none() {
                return Err(rmcp::ErrorData::invalid_params(
                    "workspace not ready (startup in progress)",
                    None,
                ));
            }
            return Ok(uuid);
        }

        let mut ready = sessions
            .iter()
            .filter_map(|(id, session)| session.startup.as_ref().map(|_| *id));

        let Some(first) = ready.next() else {
            return Err(rmcp::ErrorData::invalid_params("no ready sessions", None));
        };
        if ready.next().is_some() {
            return Err(rmcp::ErrorData::invalid_params(
                "exactly one ready session is required",
                None,
            ));
        }
        Ok(first)
    }

    fn build_metadata_lookup_v2(deps: &Arc<bsl_analysis_v2::SemanticDeps>) -> TypeMetadataLookup {
        TypeMetadataLookup::new(deps.repository.clone())
    }

    async fn ready_startup_for_http(
        &self,
        session_id: Option<&str>,
    ) -> Result<bsl_runtime::system::StartupResultV2, rmcp::ErrorData> {
        let sessions = self.sessions.read().await;
        let uuid = Self::select_ready_session_uuid(&sessions, session_id)?;
        let session = sessions
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        session
            .startup
            .clone()
            .ok_or_else(|| rmcp::ErrorData::invalid_params("no ready sessions", None))
    }

    pub async fn http_snapshot_status(
        &self,
        session_id: Option<&str>,
    ) -> Result<McpSnapshotStatusResponseDto, rmcp::ErrorData> {
        let sessions = self.sessions.read().await;
        let uuid = Self::select_ready_session_uuid(&sessions, session_id)?;
        let session = sessions
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;

        let mut tracked = session
            .documents
            .hot_set
            .iter()
            .cloned()
            .collect::<Vec<DocumentKey>>();
        for key in session.documents.overlays.keys() {
            if !tracked.contains(key) {
                tracked.push(key.clone());
            }
        }
        tracked.sort_by(|left, right| {
            (left.path.as_str(), left.root_id.as_str())
                .cmp(&(right.path.as_str(), right.root_id.as_str()))
        });

        let updated_at_ms = session
            .created_at
            .saturating_mul(1000)
            .saturating_add(session.analysis_revision);
        let entries = tracked
            .into_iter()
            .map(|key| {
                let overlay = session.documents.overlays.get(&key);
                SnapshotReadinessDto {
                    schema_version: SNAPSHOT_READINESS_SCHEMA_VERSION,
                    uri: None,
                    path: Some(key.path.clone()),
                    session_id: Some(uuid.to_string()),
                    requested_version: overlay.map(|value| value.version as i64),
                    ready_version: None,
                    analysis_revision: Some(session.analysis_revision),
                    state: if overlay.is_some() {
                        SnapshotReadinessStateDto::ShadowOnly
                    } else {
                        SnapshotReadinessStateDto::Ready
                    },
                    exact: overlay.is_none(),
                    task_state: SnapshotTaskStateDto::NotApplicable,
                    phase: None,
                    trigger: Some(if overlay.is_some() {
                        SnapshotTriggerDto::DocumentsSet
                    } else {
                        SnapshotTriggerDto::Job
                    }),
                    updated_at_ms,
                    fallback_reason: None,
                    reason: None,
                    artifacts: None,
                    worker: None,
                    last_failure: None,
                    recommendation: None,
                }
            })
            .collect();

        Ok(McpSnapshotStatusResponseDto {
            schema_version: SNAPSHOT_READINESS_SCHEMA_VERSION,
            entries,
        })
    }

    pub async fn http_parity_types(
        &self,
        session_id: Option<&str>,
        limit: usize,
        offset: usize,
        category_filter: Vec<String>,
        certainty_filter: Vec<String>,
        flow_sensitive_only: bool,
    ) -> Result<AnalysisResultDto, rmcp::ErrorData> {
        let startup = self.ready_startup_for_http(session_id).await?;
        let deps = startup.deps_bundle_v2.semantic_deps.clone();
        let metadata_lookup = Self::build_metadata_lookup_v2(&deps);
        Ok(web_api_service::get_all_types_as_dto(
            deps.as_ref(),
            &metadata_lookup,
            limit,
            offset,
            category_filter,
            certainty_filter,
            flow_sensitive_only,
        ))
    }

    pub async fn http_parity_search(
        &self,
        session_id: Option<&str>,
        query: &str,
    ) -> Result<AnalysisResultDto, rmcp::ErrorData> {
        let startup = self.ready_startup_for_http(session_id).await?;
        let deps = startup.deps_bundle_v2.semantic_deps.clone();
        let metadata_lookup = Self::build_metadata_lookup_v2(&deps);

        web_api_service::search_types_as_dto(deps.as_ref(), &metadata_lookup, query)
            .await
            .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))
    }

    pub async fn bsl_types_list(
        &self,
        params: BslTypesListParams,
    ) -> Result<serde_json::Value, rmcp::ErrorData> {
        self.bsl_types_list_with_progress(params, None).await
    }

    pub(crate) async fn bsl_types_list_with_progress(
        &self,
        params: BslTypesListParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<serde_json::Value, rmcp::ErrorData> {
        report_visible_job_stage(progress.as_ref(), "validating_request", 5).await;
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

        let offset = (params.page as usize)
            .checked_sub(1)
            .and_then(|page| page.checked_mul(params.limit as usize))
            .ok_or_else(|| rmcp::ErrorData::invalid_params("page/limit overflow", None))?;

        report_visible_job_stage(progress.as_ref(), "loading_snapshot", 20).await;
        let startup = self
            .ready_startup_for_http(Some(&params.session_id))
            .await?;
        let deps = startup.deps_bundle_v2.semantic_deps.clone();

        report_visible_job_stage(progress.as_ref(), "building_metadata", 40).await;
        let metadata_lookup = Self::build_metadata_lookup_v2(&deps);

        let category_filter = params
            .category
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| vec![value.to_string()])
            .unwrap_or_else(|| match params.source {
                Some(BslTypeSource::Platform) => vec!["Platform".to_string()],
                Some(BslTypeSource::Configuration) => vec!["Configuration".to_string()],
                None => Vec::new(),
            });

        let certainty_filter = match params.certainty_level {
            Some(level) if level >= 80 => vec!["high".to_string()],
            Some(level) if level >= 30 => vec!["high".to_string(), "medium".to_string()],
            _ => Vec::new(),
        };

        report_visible_job_stage(progress.as_ref(), "collecting_types", 65).await;
        let mut dto = web_api_service::get_all_types_as_dto(
            deps.as_ref(),
            &metadata_lookup,
            params.limit as usize,
            offset,
            category_filter,
            certainty_filter,
            params.flow_sensitive_only,
        );

        report_visible_job_stage(progress.as_ref(), "serializing_result", 90).await;
        match params.view {
            BslTypesView::NamesOnly => {
                let names: Vec<String> = dto.types.into_iter().map(|t| t.name).collect();
                Ok(serde_json::to_value(names)
                    .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?)
            }
            BslTypesView::Summary => {
                for type_ in &mut dto.types {
                    type_.methods.clear();
                    type_.tabular_sections.clear();
                }
                Ok(serde_json::to_value(dto)
                    .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?)
            }
            BslTypesView::Full => Ok(serde_json::to_value(dto)
                .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?),
        }
    }

    pub async fn bsl_types_search(
        &self,
        params: BslTypesSearchParams,
    ) -> Result<serde_json::Value, rmcp::ErrorData> {
        self.bsl_types_search_with_progress(params, None).await
    }

    pub(crate) async fn bsl_types_search_with_progress(
        &self,
        params: BslTypesSearchParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<serde_json::Value, rmcp::ErrorData> {
        report_visible_job_stage(progress.as_ref(), "validating_request", 5).await;
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

        report_visible_job_stage(progress.as_ref(), "loading_snapshot", 20).await;
        let startup = self
            .ready_startup_for_http(Some(&params.session_id))
            .await?;
        let deps = startup.deps_bundle_v2.semantic_deps.clone();

        report_visible_job_stage(progress.as_ref(), "building_metadata", 40).await;
        let metadata_lookup = Self::build_metadata_lookup_v2(&deps);

        report_visible_job_stage(progress.as_ref(), "searching_types", 65).await;
        let mut dto = web_api_service::search_types_as_dto(deps.as_ref(), &metadata_lookup, query)
            .await
            .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?;

        report_visible_job_stage(progress.as_ref(), "filtering_results", 80).await;
        if let Some(source) = params.source {
            let expected_category = match source {
                BslTypeSource::Platform => "Platform",
                BslTypeSource::Configuration => "Configuration",
            };
            dto.types
                .retain(|type_| type_.category == expected_category);
        }

        if dto.types.len() > params.limit as usize {
            dto.types.truncate(params.limit as usize);
        }

        report_visible_job_stage(progress.as_ref(), "serializing_result", 92).await;
        match params.view {
            BslTypesView::NamesOnly => {
                let names: Vec<String> = dto.types.into_iter().map(|t| t.name).collect();
                Ok(serde_json::to_value(names)
                    .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?)
            }
            BslTypesView::Summary => {
                for type_ in &mut dto.types {
                    type_.methods.clear();
                    type_.tabular_sections.clear();
                }
                Ok(serde_json::to_value(dto)
                    .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?)
            }
            BslTypesView::Full => Ok(serde_json::to_value(dto)
                .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?),
        }
    }

    pub async fn bsl_type_get(
        &self,
        params: BslTypeGetParams,
    ) -> Result<serde_json::Value, rmcp::ErrorData> {
        self.bsl_type_get_with_progress(params, None).await
    }

    pub(crate) async fn bsl_type_get_with_progress(
        &self,
        params: BslTypeGetParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<serde_json::Value, rmcp::ErrorData> {
        report_visible_job_stage(progress.as_ref(), "validating_request", 5).await;
        let type_name = params.type_name.trim();
        if type_name.is_empty() {
            return Err(rmcp::ErrorData::invalid_params(
                "type_name must be non-empty",
                None,
            ));
        }

        report_visible_job_stage(progress.as_ref(), "loading_snapshot", 20).await;
        let startup = self
            .ready_startup_for_http(Some(&params.session_id))
            .await?;
        let deps = startup.deps_bundle_v2.semantic_deps.clone();

        report_visible_job_stage(progress.as_ref(), "building_metadata", 40).await;
        let metadata_lookup = Self::build_metadata_lookup_v2(&deps);

        report_visible_job_stage(progress.as_ref(), "resolving_type", 70).await;
        let mut dto = web_api_service::get_type_details_as_dto(
            deps.as_ref(),
            &metadata_lookup,
            type_name,
            params.include_methods,
        )
        .ok_or_else(|| rmcp::ErrorData::invalid_params("type not found", None))?;

        if let Some(source) = params.source {
            let expected_source = match source {
                BslTypeSource::Platform => "Platform",
                BslTypeSource::Configuration => "Configuration",
            };
            if dto.source != expected_source {
                return Err(rmcp::ErrorData::invalid_params("type not found", None));
            }
        }

        if !params.include_methods {
            dto.methods.clear();
        }

        report_visible_job_stage(progress.as_ref(), "serializing_result", 92).await;
        serde_json::to_value(dto).map_err(|err| {
            rmcp::ErrorData::internal_error(format!("serialize type dto: {err}"), None)
        })
    }

    fn is_flow_sensitive(res: &bsl_shared::domain::types::TypeResolution) -> bool {
        if matches!(res.result, ResolutionResult::Union(_)) {
            return true;
        }
        matches!(res.certainty, Certainty::Inferred | Certainty::InferredWeak)
    }

    pub async fn http_parity_metrics(
        &self,
        session_id: Option<&str>,
    ) -> Result<MetricsDto, rmcp::ErrorData> {
        let startup = self.ready_startup_for_http(session_id).await?;
        let deps = startup.deps_bundle_v2.semantic_deps.clone();

        let all_types = web_api_service::get_all_platform_globals(deps.as_ref());
        let mut certainty_high = 0;
        let mut certainty_medium = 0;
        let mut certainty_low = 0;
        let mut flow_sensitive = 0;

        for res in all_types.values() {
            let certainty_val = match res.certainty {
                Certainty::Known => 100,
                Certainty::Inferred => 80,
                Certainty::InferredWeak => 50,
                Certainty::Unknown => 0,
            };

            if certainty_val >= 80 {
                certainty_high += 1;
            } else if certainty_val >= 30 {
                certainty_medium += 1;
            } else {
                certainty_low += 1;
            }

            if Self::is_flow_sensitive(res) {
                flow_sensitive += 1;
            }
        }

        Ok(MetricsDto {
            total_types: all_types.len(),
            certainty_high,
            certainty_medium,
            certainty_low,
            flow_sensitive,
            cache_hit_rate: "n/a".to_string(),
            analysis_speed: "125ms".to_string(),
        })
    }

    pub async fn http_deps_meta(
        &self,
        session_id: Option<&str>,
    ) -> Result<SnapshotMetaDto, rmcp::ErrorData> {
        let sessions = self.sessions.read().await;
        let uuid = if let Some(session_id) = session_id {
            parse_session_id(session_id)?
        } else if sessions.is_empty() {
            return Err(rmcp::ErrorData::invalid_params("no sessions", None));
        } else if sessions.len() == 1 {
            *sessions
                .keys()
                .next()
                .ok_or_else(|| rmcp::ErrorData::invalid_params("no sessions", None))?
        } else {
            return Err(rmcp::ErrorData::invalid_params(
                "session_id is required when multiple sessions exist",
                None,
            ));
        };

        let session = sessions
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        let startup = session.startup.as_ref().ok_or_else(|| {
            rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
        })?;

        let deps_bundle = &startup.deps_bundle_v2;
        let inputs = &startup.inputs;
        Ok(SnapshotMetaDto {
            deps_id: deps_bundle.deps_id.as_str().to_string(),
            index_snapshot_id: deps_bundle.meta.index_snapshot_id.clone(),
            platform_version: deps_bundle.meta.platform_version.clone(),
            platform_fingerprint: deps_bundle.meta.platform_fingerprint.clone(),
            config_fingerprint: deps_bundle.meta.config_fingerprint.clone(),
            strict_fingerprint: deps_bundle.meta.strict_fingerprint,
            global_context: GlobalContextDocsStatusDto {
                state: deps_bundle.meta.global_context_state.clone(),
                property_count: deps_bundle.meta.global_context_property_count,
                fingerprint: deps_bundle.meta.global_context_fingerprint.clone(),
                degraded_reason: deps_bundle.meta.global_context_degraded_reason.clone(),
            },
            repository_stats: deps_bundle.semantic_deps.repository.get_stats(),
            inputs: SnapshotInputsDto {
                syntax_helper_path: inputs
                    .syntax_helper_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                configuration_path: inputs
                    .configuration_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                platform_version: inputs.platform_version.clone(),
                rules_config_path: inputs
                    .rules_config_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                cache_enabled: inputs.cache_enabled,
                strict_fingerprint: inputs.strict_fingerprint,
            },
        })
    }

    pub async fn close(&self, session_id: &str) -> Result<(), rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;
        let mut sessions = self.sessions.write().await;
        if sessions.remove(&uuid).is_none() {
            return Err(rmcp::ErrorData::invalid_params("session not found", None));
        }
        Ok(())
    }

    pub async fn resume(
        self: &Arc<Self>,
        session_id: &str,
        job_manager: Arc<JobManager>,
    ) -> Result<WorkspaceOpenResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;

        {
            let sessions = self.sessions.read().await;
            if let Some(existing) = sessions.get(&uuid) {
                return Ok(Self::open_response_from_session(uuid, existing));
            }
            if !sessions.is_empty() {
                return Err(rmcp::ErrorData::invalid_params(SINGLE_SESSION_ERROR, None));
            }
        }

        let store = self.store.as_ref().ok_or_else(|| {
            rmcp::ErrorData::internal_error("persist store is not available", None)
        })?;
        let persisted = store
            .load(session_id)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;

        let (roots, root_dtos) = restore_roots(&persisted.roots)?;
        let configuration_path =
            Self::normalize_optional_path(persisted.configuration_path, "configuration_path")?;
        let rules_config_path = Self::normalize_optional_rules_config_path(
            persisted.rules_config_path,
            roots.first().map(|root| root.path.as_path()),
        )
        .or_else(|| Self::discover_workspace_rules_config(configuration_path.as_deref(), &roots));

        let settings = WorkspaceSettings {
            platform_docs_archive: Self::normalize_optional_path(
                persisted.platform_docs_archive,
                "platform_docs_archive",
            )?,
            platform_version: persisted.platform_version.and_then(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }),
            configuration_path,
            rules_config_path,
            mode: normalize_mode(persisted.mode),
            env_overrides: persisted.env_overrides,
            dev_env_overrides: persisted.dev_env_overrides,
            allow_dev_overrides: persisted.allow_dev_overrides,
        };
        let settings = {
            let mut settings = settings;
            if settings.configuration_path.is_some() && settings.platform_version.is_none() {
                let inferred = infer_platform_version_from_config_dump(
                    settings
                        .configuration_path
                        .as_deref()
                        .expect("configuration_path is_some"),
                )?;
                settings.platform_version = Some(inferred);
            }
            settings
        };

        let missing_inputs = workspace_missing_inputs(&settings);
        let warnings = workspace_warnings(&settings);

        let session = WorkspaceSession {
            roots,
            documents: DocumentStore::default(),
            analysis_revision: persisted.analysis_revision,
            settings,
            startup: None,
            startup_job_id: None,
            startup_phase: "startup/queued".to_string(),
            startup_progress: 0,
            startup_error: None,
            created_at: persisted.created_at,
            id_map: IdMap::default(),
            pack_store: PackStore::default(),
        };
        self.sessions.write().await.insert(uuid, session);
        self.persist_session(uuid).await;

        let startup_settings = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            session.settings.clone()
        };

        let session_manager = Arc::clone(self);
        let startup_job_id = job_manager
            .spawn("startup", move |ctx| async move {
                run_startup_job(session_manager, uuid, startup_settings, ctx).await
            })
            .await;

        self.set_startup_job_id(uuid, startup_job_id.clone())
            .await?;

        Ok(WorkspaceOpenResponse {
            session_id: uuid.to_string(),
            roots: root_dtos,
            analysis_revision: persisted.analysis_revision,
            ready: false,
            startup_job_id: Some(startup_job_id),
            warnings,
            missing_inputs,
        })
    }

    pub async fn list(&self) -> Result<WorkspaceListResponse, rmcp::ErrorData> {
        let Some(store) = self.store.as_ref() else {
            return Ok(WorkspaceListResponse {
                sessions: Vec::new(),
            });
        };
        let sessions = store
            .list()
            .into_iter()
            .map(|session| WorkspaceListItemDto {
                session_id: session.session_id,
                roots: session.roots,
                analysis_revision: session.analysis_revision,
                updated_at: session.updated_at,
            })
            .collect();
        Ok(WorkspaceListResponse { sessions })
    }
}
