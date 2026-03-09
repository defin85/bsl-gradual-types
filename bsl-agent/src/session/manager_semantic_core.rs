impl SessionManager {
    pub async fn bsl_diagnostics(
        &self,
        params: BslDiagnosticsParams,
    ) -> Result<BslDiagnosticsResponse, rmcp::ErrorData> {
        self.bsl_diagnostics_with_progress(params, None).await
    }

    pub(crate) async fn bsl_diagnostics_with_progress(
        &self,
        params: BslDiagnosticsParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<BslDiagnosticsResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let flow_sensitive_enabled = params.include_flow_sensitive;
        tracing::debug!(
            session_id = %params.session_id,
            include_flow_sensitive = flow_sensitive_enabled,
            limit = params.limit,
            scope = ?params.scope,
            "bsl_diagnostics entered"
        );
        report_job_stage(progress.as_ref(), "resolving_scope", 5).await;
        let (
            roots,
            hot_set,
            overlays,
            analysis_revision,
            deps_id,
            deps,
            index_snapshot,
            coordinator,
        ) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            (
                session.roots.clone(),
                session.documents.hot_set.clone(),
                session.documents.overlays.clone(),
                session.analysis_revision,
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
                startup.deps_bundle_v2.index_snapshot.clone(),
                startup.coordinator.clone(),
            )
        };

        let scope = normalize_workspace_scope(params.scope)?;
        let profile = match &scope {
            WorkspaceScopeTagged::File { .. } => DiagnosticsProfile::Fast,
            _ => DiagnosticsProfile::DebouncedFull,
        };
        let worker_class =
            bsl_runtime::application::diagnostics_execution_plan(profile, flow_sensitive_enabled)
                .cpu_class;
        let files = collect_scope_files(&roots, &hot_set, scope)?;
        let total_files = files.len();
        tracing::debug!(
            session_id = %params.session_id,
            files = total_files,
            profile = profile.as_str(),
            worker_class = ?worker_class,
            "bsl_diagnostics resolved scope files"
        );
        report_batch_progress(progress.as_ref(), "analyzing_files", 0, total_files, 15, 85).await;
        let facade = SemanticFacade;
        let mut diagnostics = Vec::new();
        let mut truncated = false;
        let mut total_read_bytes = 0u64;

        for (index, file) in files.into_iter().enumerate() {
            let doc_snapshot = match load_document_snapshot(&file, &overlays)? {
                Some(snapshot) => snapshot,
                None => continue,
            };

            total_read_bytes = total_read_bytes.saturating_add(doc_snapshot.text.len() as u64);
            if total_read_bytes > MAX_TOTAL_READ_BYTES {
                truncated = true;
                break;
            }

            let remaining_limit = params.limit as usize - diagnostics.len();
            if remaining_limit == 0 {
                truncated = true;
                break;
            }

            let deps_local = deps.clone();
            let index_snapshot_local = index_snapshot.clone();
            let coordinator_local = coordinator.clone();
            let deps_id_local = deps_id.clone();
            let doc_path_for_error = doc_snapshot.abs_path.display().to_string();
            tracing::debug!(
                session_id = %params.session_id,
                file = %doc_path_for_error,
                bytes = total_read_bytes,
                remaining_limit,
                "bsl_diagnostics dispatching file worker"
            );

            let file_result =
                bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                    worker_class,
                    ObservabilityOrigin::Agent.as_str(),
                    Some(coordinator.as_ref()),
                    move || {
                        collect_file_diagnostics(FileDiagnosticsRequest {
                            flow_sensitive_enabled,
                            analysis_revision,
                            deps_id: deps_id_local,
                            deps: deps_local,
                            index_snapshot: index_snapshot_local,
                            coordinator: coordinator_local,
                            doc_snapshot,
                            remaining_limit,
                        })
                    },
                )
                .await
                .map_err(|err| {
                    rmcp::ErrorData::internal_error(
                        format!(
                            "diagnostics worker task join failed for {}: {err}",
                            doc_path_for_error
                        ),
                        None,
                    )
                })??;

            let Some(file_result) = file_result else {
                continue;
            };

            tracing::debug!(
                session_id = %params.session_id,
                file = %file.abs_path.display(),
                diagnostics = file_result.diagnostics.len(),
                hit_limit = file_result.hit_limit,
                "bsl_diagnostics file worker completed"
            );
            diagnostics.extend(file_result.diagnostics);
            report_batch_progress(
                progress.as_ref(),
                "analyzing_files",
                index + 1,
                total_files,
                15,
                85,
            )
            .await;
            if file_result.hit_limit {
                truncated = true;
                break;
            }
        }

        report_job_stage(progress.as_ref(), "finalizing", 95).await;
        facade.sort_diagnostics(&mut diagnostics);
        if diagnostics.len() > params.limit as usize {
            diagnostics.truncate(params.limit as usize);
            truncated = true;
        }

        Ok(BslDiagnosticsResponse {
            analysis_revision,
            flow_sensitive_enabled,
            diagnostics,
            truncated,
        })
    }

    pub async fn bsl_symbol_search(
        &self,
        params: BslSymbolSearchParams,
    ) -> Result<BslSymbolSearchResponse, rmcp::ErrorData> {
        self.bsl_symbol_search_with_progress(params, None).await
    }

    pub(crate) async fn bsl_symbol_search_with_progress(
        &self,
        params: BslSymbolSearchParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<BslSymbolSearchResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let query = params.query.trim();
        if query.is_empty() {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            return Ok(BslSymbolSearchResponse {
                analysis_revision: session.analysis_revision,
                symbols: Vec::new(),
                truncated: false,
            });
        }

        report_job_stage(progress.as_ref(), "collecting_files", 5).await;
        let (roots, analysis_revision, deps_id, deps, index_snapshot, coordinator) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            (
                session.roots.clone(),
                session.analysis_revision,
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
                startup.deps_bundle_v2.index_snapshot.clone(),
                startup.coordinator.clone(),
            )
        };

        let files = collect_project_files(&roots)?;
        let total_files = files.len();
        let query_lower = query.to_lowercase();
        let mut symbols = Vec::new();
        let mut truncated = false;
        let mut total_read_bytes = 0u64;
        report_batch_progress(progress.as_ref(), "scanning_files", 0, total_files, 15, 85).await;

        for (index, file) in files.into_iter().enumerate() {
            let text = match load_disk_text_with_limits(&file.root_path, &file.abs_path)? {
                Some(text) => text,
                None => continue,
            };
            total_read_bytes = total_read_bytes.saturating_add(text.len() as u64);
            if total_read_bytes > MAX_TOTAL_READ_BYTES {
                truncated = true;
                break;
            }

            let remaining = params.limit as usize - symbols.len();
            if remaining == 0 {
                truncated = true;
                break;
            }

            let deps_local = deps.clone();
            let index_snapshot_local = index_snapshot.clone();
            let coordinator_local = coordinator.clone();
            let deps_id_local = deps_id.clone();
            let file_root_id = file.root_id.clone();
            let file_rel_path = file.rel_path.clone();
            let file_abs_path = file.abs_path.to_string_lossy().to_string();
            let query_lower = query_lower.clone();
            let analysis_revision_local = analysis_revision;

            let file_symbols =
                bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                    bsl_runtime::application::CpuWorkClass::Background,
                    ObservabilityOrigin::Agent.as_str(),
                    Some(coordinator.as_ref()),
                    move || -> Result<Vec<SymbolDto>, rmcp::ErrorData> {
                        let (context, prepared) = match prepare_ephemeral_mcp_operation(
                            SemanticOperation::SymbolSearch,
                            false,
                            deps_id_local,
                            deps_local,
                            index_snapshot_local,
                            Arc::from(text),
                            0,
                            Arc::from(file_abs_path),
                            DetailLevel::Full,
                            coordinator_local.as_ref(),
                        ) {
                            Ok(values) => values,
                            Err(_) => return Ok(Vec::new()),
                        };

                        let analysis = prepared.snapshot.analysis;
                        let program_query = IntellisenseV2Facade::run_optional_query(
                            &context,
                            ObservabilityStage::IrQuery,
                            &analysis,
                            Some(coordinator_local.as_ref()),
                            |analysis| analysis.ir(FileId(1)),
                        );
                        let Some(program) = program_query.ok().flatten() else {
                            return Ok(Vec::new());
                        };
                        let Some(code) = analysis.file_text(FileId(1)).ok().flatten() else {
                            return Ok(Vec::new());
                        };
                        let Some(line_index) = analysis.line_index(FileId(1)).ok().flatten() else {
                            return Ok(Vec::new());
                        };

                        let mut out = Vec::new();
                        for node in program.nodes.iter() {
                            let (kind, name) = match &node.kind {
                                bsl_shared::ir::SemanticNodeKind::FunctionDeclaration {
                                    name,
                                    ..
                                } => ("function", name.as_str()),
                                bsl_shared::ir::SemanticNodeKind::ProcedureDeclaration {
                                    name,
                                    ..
                                } => ("procedure", name.as_str()),
                                _ => continue,
                            };

                            if !name.to_lowercase().contains(&query_lower) {
                                continue;
                            }

                            if out.len() >= remaining {
                                break;
                            }

                            let range = span_to_range_with_index(
                                code.as_ref(),
                                line_index.as_ref(),
                                node.span,
                            );
                            let file_ref = DocumentRefDto {
                                root_id: file_root_id.clone(),
                                path: file_rel_path.clone(),
                            };
                            let document_id = ids::document_id(&file_ref.root_id, &file_ref.path);
                            let symbol_id = ids::stable_id_hex(&[
                                ids::IdPart::U64(analysis_revision_local),
                                ids::IdPart::Str(&document_id),
                                ids::IdPart::Str(kind),
                                ids::IdPart::U32(range.start.line),
                                ids::IdPart::U32(range.start.character),
                                ids::IdPart::U32(range.end.line),
                                ids::IdPart::U32(range.end.character),
                                ids::IdPart::Str(name),
                            ]);

                            out.push(SymbolDto {
                                symbol_id,
                                name: name.to_string(),
                                kind: kind.to_string(),
                                file: file_ref,
                                range,
                            });
                        }
                        Ok(out)
                    },
                )
                .await
                .map_err(|err| {
                    rmcp::ErrorData::internal_error(
                        format!("symbol_search worker task join failed: {err}"),
                        None,
                    )
                })??;

            symbols.extend(file_symbols);
            report_batch_progress(
                progress.as_ref(),
                "scanning_files",
                index + 1,
                total_files,
                15,
                85,
            )
            .await;
            if symbols.len() >= params.limit as usize {
                truncated = true;
                break;
            }
        }

        report_job_stage(progress.as_ref(), "finalizing", 95).await;
        sort_symbols(&mut symbols);
        if symbols.len() > params.limit as usize {
            symbols.truncate(params.limit as usize);
            truncated = true;
        }

        {
            let mut sessions = self.sessions.write().await;
            let Some(session) = sessions.get_mut(&uuid) else {
                return Err(rmcp::ErrorData::invalid_params("session not found", None));
            };
            if session.analysis_revision == analysis_revision {
                session.id_map.reset(analysis_revision);
                for symbol in &symbols {
                    session.id_map.symbols.insert(
                        symbol.symbol_id.clone(),
                        StoredSymbol {
                            name: symbol.name.clone(),
                            kind: symbol.kind.clone(),
                            file: symbol.file.clone(),
                            range: symbol.range,
                        },
                    );
                }
            }
        }

        Ok(BslSymbolSearchResponse {
            analysis_revision,
            symbols,
            truncated,
        })
    }

    pub async fn bsl_type_at_position(
        &self,
        params: BslTypeAtPositionParams,
    ) -> Result<BslTypeAtPositionResponse, rmcp::ErrorData> {
        self.bsl_type_at_position_with_progress(params, None).await
    }

    pub(crate) async fn bsl_type_at_position_with_progress(
        &self,
        params: BslTypeAtPositionParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<BslTypeAtPositionResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let flow_sensitive_enabled = params.include_flow_sensitive;
        report_job_stage(progress.as_ref(), "resolving_document", 10).await;
        let (analysis_revision, roots, overlays, deps_id, deps, index_snapshot, coordinator) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            (
                session.analysis_revision,
                session.roots.clone(),
                session.documents.overlays.clone(),
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
                startup.deps_bundle_v2.index_snapshot.clone(),
                startup.coordinator.clone(),
            )
        };

        let file_key = document_key_from_ref(&roots, &params.file.doc)?;
        let (root_path, abs_path, _file_ref) = resolve_doc_path(&roots, &file_key)?;
        let text =
            select_effective_text(&params.file, &file_key, &overlays, &root_path, &abs_path)?;
        let version = select_effective_version(&params.file, &file_key, &overlays);
        let abs_path_display = abs_path.to_string_lossy().to_string();

        report_job_stage(progress.as_ref(), "preparing_snapshot", 35).await;
        let coordinator_for_worker = coordinator.clone();
        let response = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
            bsl_runtime::application::cpu_work_class_for_operation(
                SemanticOperation::TypeAtPosition,
            ),
            ObservabilityOrigin::Agent.as_str(),
            Some(coordinator.as_ref()),
            move || {
                collect_type_at_position(TypeAtPositionRequest {
                    analysis_revision,
                    flow_sensitive_enabled,
                    deps_id,
                    deps,
                    index_snapshot,
                    coordinator: coordinator_for_worker,
                    text,
                    version,
                    abs_path: abs_path_display,
                    position: params.position,
                })
            },
        )
        .await
        .map_err(|err| {
            rmcp::ErrorData::internal_error(
                format!("type_at_position worker task join failed: {err}"),
                None,
            )
        })??;

        report_job_stage(progress.as_ref(), "finalizing", 95).await;
        Ok(response)
    }

    pub async fn bsl_members(
        &self,
        params: BslMembersParams,
    ) -> Result<BslMembersResponse, rmcp::ErrorData> {
        self.bsl_members_with_progress(params, None).await
    }

    pub(crate) async fn bsl_members_with_progress(
        &self,
        params: BslMembersParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<BslMembersResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        let flow_sensitive_enabled = params.include_flow_sensitive;
        report_job_stage(progress.as_ref(), "resolving_document", 10).await;
        let (analysis_revision, roots, overlays, deps_id, deps, index_snapshot, coordinator) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            (
                session.analysis_revision,
                session.roots.clone(),
                session.documents.overlays.clone(),
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
                startup.deps_bundle_v2.index_snapshot.clone(),
                startup.coordinator.clone(),
            )
        };

        let file_key = document_key_from_ref(&roots, &params.file.doc)?;
        let (root_path, abs_path, _file_ref) = resolve_doc_path(&roots, &file_key)?;
        let text =
            select_effective_text(&params.file, &file_key, &overlays, &root_path, &abs_path)?;
        let version = select_effective_version(&params.file, &file_key, &overlays);
        let abs_path_display = abs_path.to_string_lossy().to_string();
        let completion_runtime = tokio::runtime::Handle::current();

        report_job_stage(progress.as_ref(), "preparing_snapshot", 30).await;
        let coordinator_for_worker = coordinator.clone();
        let response = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
            bsl_runtime::application::cpu_work_class_for_operation(SemanticOperation::Members),
            ObservabilityOrigin::Agent.as_str(),
            Some(coordinator.as_ref()),
            move || {
                collect_members(MembersRequest {
                    analysis_revision,
                    flow_sensitive_enabled,
                    deps_id,
                    deps,
                    index_snapshot,
                    coordinator: coordinator_for_worker,
                    text,
                    version,
                    abs_path: abs_path_display,
                    position: params.position,
                    limit: params.limit,
                    completion_runtime,
                })
            },
        )
        .await
        .map_err(|err| {
            rmcp::ErrorData::internal_error(format!("members worker task join failed: {err}"), None)
        })??;

        report_job_stage(progress.as_ref(), "finalizing", 95).await;
        Ok(response)
    }
}

struct TypeAtPositionRequest {
    analysis_revision: u64,
    flow_sensitive_enabled: bool,
    deps_id: bsl_analysis_v2::DepsSnapshotId,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    index_snapshot: Arc<bsl_runtime::system::IndexSnapshot>,
    coordinator: Arc<bsl_runtime::system::SystemCoordinator>,
    text: String,
    version: i32,
    abs_path: String,
    position: crate::server::types::Position,
}

fn collect_type_at_position(
    request: TypeAtPositionRequest,
) -> Result<BslTypeAtPositionResponse, rmcp::ErrorData> {
    let TypeAtPositionRequest {
        analysis_revision,
        flow_sensitive_enabled,
        deps_id,
        deps,
        index_snapshot,
        coordinator,
        text,
        version,
        abs_path,
        position,
    } = request;

    let (context, prepared) = match prepare_ephemeral_mcp_operation(
        SemanticOperation::TypeAtPosition,
        flow_sensitive_enabled,
        deps_id,
        deps,
        index_snapshot,
        Arc::from(text),
        version,
        Arc::from(abs_path),
        DetailLevel::Full,
        coordinator.as_ref(),
    ) {
        Ok(values) => values,
        Err(_) => {
            return Ok(BslTypeAtPositionResponse {
                analysis_revision,
                flow_sensitive_enabled,
                type_info: None,
                node: None,
                warnings: vec!["IR not available".to_string()],
            });
        }
    };

    let analysis = prepared.snapshot.analysis;
    let program_query = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(coordinator.as_ref()),
        |analysis| analysis.ir(FileId(1)),
    );
    let Some(program) = program_query.ok().flatten() else {
        return Ok(BslTypeAtPositionResponse {
            analysis_revision,
            flow_sensitive_enabled,
            type_info: None,
            node: None,
            warnings: vec!["IR not available".to_string()],
        });
    };

    let type_info = type_at_utf16_position(
        &analysis,
        FileId(1),
        position.line,
        position.character,
        flow_sensitive_enabled,
    )
    .map(|resolution| TypeInfoDto {
        name: user_facing_resolution_type_name(&resolution),
        certainty: format!("{:?}", resolution.certainty).to_lowercase(),
        active_facet: resolution
            .active_facet
            .as_ref()
            .map(|facet| format!("{:?}", facet)),
    });

    let node = node_at_utf16_position(
        &analysis,
        program.as_ref(),
        FileId(1),
        position.line,
        position.character,
    )
    .map(|node| NodeInfoDto {
        kind: format!("{:?}", node.kind),
        range: span_to_range_with_analysis(&analysis, FileId(1), node.span),
    });

    Ok(BslTypeAtPositionResponse {
        analysis_revision,
        flow_sensitive_enabled,
        type_info,
        node,
        warnings: Vec::new(),
    })
}

struct MembersRequest {
    analysis_revision: u64,
    flow_sensitive_enabled: bool,
    deps_id: bsl_analysis_v2::DepsSnapshotId,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    index_snapshot: Arc<bsl_runtime::system::IndexSnapshot>,
    coordinator: Arc<bsl_runtime::system::SystemCoordinator>,
    text: String,
    version: i32,
    abs_path: String,
    position: crate::server::types::Position,
    limit: u32,
    completion_runtime: tokio::runtime::Handle,
}

fn collect_members(request: MembersRequest) -> Result<BslMembersResponse, rmcp::ErrorData> {
    let MembersRequest {
        analysis_revision,
        flow_sensitive_enabled,
        deps_id,
        deps,
        index_snapshot,
        coordinator,
        text,
        version,
        abs_path,
        position,
        limit,
        completion_runtime,
    } = request;

    let (context, prepared) = match prepare_ephemeral_mcp_operation(
        SemanticOperation::Members,
        flow_sensitive_enabled,
        deps_id,
        deps.clone(),
        index_snapshot,
        Arc::from(text.clone()),
        version,
        Arc::from(abs_path.clone()),
        DetailLevel::Full,
        coordinator.as_ref(),
    ) {
        Ok(values) => values,
        Err(_) => {
            return Ok(BslMembersResponse {
                analysis_revision,
                flow_sensitive_enabled,
                members: Vec::new(),
                truncated: false,
            });
        }
    };

    let bsl_runtime::application::SemanticSnapshot {
        analysis,
        index_snapshot,
        ..
    } = prepared.snapshot;
    let program = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(coordinator.as_ref()),
        |analysis| analysis.ir(FileId(1)),
    )
    .ok()
    .flatten();
    let Some(program) = program else {
        return Ok(BslMembersResponse {
            analysis_revision,
            flow_sensitive_enabled,
            members: Vec::new(),
            truncated: false,
        });
    };

    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());

    let member_access_owner_type_hint = member_access_owner_type_hint_at_position(
        &analysis,
        FileId(1),
        text.as_str(),
        position.line,
        position.character,
        flow_sensitive_enabled,
    );

    let result = completion_runtime
        .block_on(
            bsl_runtime::application::type_system::get_completion_with_semantic_program_snapshot(
                text.as_str(),
                position.line,
                position.character,
                None,
                index_snapshot.as_ref(),
                &metadata_lookup,
                abs_path.as_str(),
                resolver.as_ref(),
                program,
                member_access_owner_type_hint,
                flow_sensitive_enabled,
            ),
        )
        .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?;

    let mut members = result
        .items
        .into_iter()
        .filter_map(|candidate| {
            let kind = match candidate.item.kind {
                bsl_shared::domain::CompletionKind::Method => "method",
                bsl_shared::domain::CompletionKind::Property => "property",
                bsl_shared::domain::CompletionKind::Field => "field",
                bsl_shared::domain::CompletionKind::Function => "function",
                bsl_shared::domain::CompletionKind::Constructor => "constructor",
                _ => return None,
            };

            Some(MemberDto {
                name: candidate.item.label,
                kind: kind.to_string(),
                detail: candidate.item.detail,
                member_identity: candidate.member_identity,
            })
        })
        .collect::<Vec<_>>();

    members.sort_by(|a, b| (a.kind.as_str(), a.name.as_str()).cmp(&(b.kind.as_str(), b.name.as_str())));
    let truncated = members.len() > limit as usize || result.is_incomplete;
    if members.len() > limit as usize {
        members.truncate(limit as usize);
    }

    Ok(BslMembersResponse {
        analysis_revision,
        flow_sensitive_enabled,
        members,
        truncated,
    })
}

struct FileDiagnosticsBatch {
    diagnostics: Vec<crate::semantic::dto::DiagnosticDto>,
    hit_limit: bool,
}

struct FileDiagnosticsRequest {
    flow_sensitive_enabled: bool,
    analysis_revision: u64,
    deps_id: bsl_analysis_v2::DepsSnapshotId,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    index_snapshot: Arc<bsl_runtime::system::IndexSnapshot>,
    coordinator: Arc<bsl_runtime::system::SystemCoordinator>,
    doc_snapshot: DocumentSnapshot,
    remaining_limit: usize,
}

fn collect_file_diagnostics(
    request: FileDiagnosticsRequest,
) -> Result<Option<FileDiagnosticsBatch>, rmcp::ErrorData> {
    let FileDiagnosticsRequest {
        flow_sensitive_enabled,
        analysis_revision,
        deps_id,
        deps,
        index_snapshot,
        coordinator,
        doc_snapshot,
        remaining_limit,
    } = request;
    let DocumentSnapshot {
        file,
        abs_path,
        text,
        version,
    } = doc_snapshot;
    let abs_path_display = abs_path.display().to_string();
    tracing::debug!(
        file = %abs_path_display,
        bytes = text.len(),
        remaining_limit,
        "diagnostics file worker entered"
    );

    let (context, prepared) = match prepare_ephemeral_mcp_operation(
        SemanticOperation::Diagnostics,
        flow_sensitive_enabled,
        deps_id,
        deps,
        index_snapshot,
        Arc::from(text),
        version,
        Arc::from(abs_path.to_string_lossy().to_string()),
        DetailLevel::Full,
        coordinator.as_ref(),
    ) {
        Ok(values) => values,
        Err(_) => return Ok(None),
    };
    tracing::debug!(
        file = %abs_path_display,
        "diagnostics file worker prepared semantic snapshot"
    );

    let analysis = prepared.snapshot.analysis;
    let Some(code) = analysis.file_text(FileId(1)).ok().flatten() else {
        return Ok(None);
    };
    let Some(line_index) = analysis.line_index(FileId(1)).ok().flatten() else {
        return Ok(None);
    };
    let file_diags_query = if flow_sensitive_enabled {
        IntellisenseV2Facade::run_optional_query(
            &context,
            ObservabilityStage::SemanticDiagnosticsQuery,
            &analysis,
            Some(coordinator.as_ref()),
            |analysis| analysis.semantic_diagnostics_flow_sensitive(FileId(1)),
        )
    } else {
        IntellisenseV2Facade::run_optional_query(
            &context,
            ObservabilityStage::SemanticDiagnosticsQuery,
            &analysis,
            Some(coordinator.as_ref()),
            |analysis| analysis.semantic_diagnostics(FileId(1)),
        )
    };
    let Some(file_diags) = file_diags_query.ok().flatten() else {
        return Ok(None);
    };
    tracing::debug!(
        file = %abs_path_display,
        diagnostics = file_diags.len(),
        "diagnostics file worker collected semantic diagnostics"
    );

    let facade = SemanticFacade;
    let mut diagnostics = Vec::new();
    let mut hit_limit = false;

    for diag in file_diags.iter() {
        if diagnostics.len() >= remaining_limit {
            hit_limit = true;
            break;
        }

        let (start_line, start_character) =
            line_index.byte_offset_to_utf16_position(code.as_ref(), diag.span.start as usize);
        let (end_line, end_character) =
            line_index.byte_offset_to_utf16_position(code.as_ref(), diag.span.end as usize);
        let range = RangeDto {
            start: PositionDto {
                line: start_line,
                character: start_character,
            },
            end: PositionDto {
                line: end_line,
                character: end_character,
            },
        };

        let severity = match diag.severity {
            bsl_shared::domain::types::DiagnosticSeverity::Error => {
                crate::semantic::dto::DiagnosticSeverityDto::Error
            }
            bsl_shared::domain::types::DiagnosticSeverity::Warning => {
                crate::semantic::dto::DiagnosticSeverityDto::Warning
            }
            bsl_shared::domain::types::DiagnosticSeverity::Info
            | bsl_shared::domain::types::DiagnosticSeverity::Hint => {
                crate::semantic::dto::DiagnosticSeverityDto::Info
            }
        };

        diagnostics.push(facade.diagnostic(
            analysis_revision,
            DocumentRefDto {
                root_id: file.root_id.clone(),
                path: file.path.clone(),
            },
            range,
            severity,
            None,
            diag.message.clone(),
        ));
    }

    Ok(Some(FileDiagnosticsBatch {
        diagnostics,
        hit_limit,
    }))
}
