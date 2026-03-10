impl SessionManager {
    pub async fn bsl_definition(
        &self,
        params: BslDefinitionParams,
    ) -> Result<BslDefinitionResponse, rmcp::ErrorData> {
        self.bsl_definition_with_progress(params, None).await
    }

    pub(crate) async fn bsl_definition_with_progress(
        &self,
        params: BslDefinitionParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<BslDefinitionResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        if let Some(symbol_id) = params.symbol_id.as_deref() {
            report_job_stage(progress.as_ref(), "resolving_symbol_id", 60).await;
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let Some(symbol) = session
                .id_map
                .get_symbol(session.analysis_revision, symbol_id)
            else {
                return Err(rmcp::ErrorData::invalid_params(
                    "stale or unknown symbol_id",
                    None,
                ));
            };

            return Ok(BslDefinitionResponse {
                analysis_revision: session.analysis_revision,
                location: Some(LocationDto {
                    file: symbol.file.clone(),
                    range: symbol.range,
                }),
                snippet: None,
            });
        }

        let Some(file) = params.file else {
            return Err(rmcp::ErrorData::invalid_params(
                "expected symbol_id or file+position",
                None,
            ));
        };
        let Some(position) = params.position else {
            return Err(rmcp::ErrorData::invalid_params(
                "expected symbol_id or file+position",
                None,
            ));
        };

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

        let file_key = document_key_from_ref(&roots, &file.doc)?;
        let (root_path, abs_path, _file_ref) = resolve_doc_path(&roots, &file_key)?;
        let text = select_effective_text(&file, &file_key, &overlays, &root_path, &abs_path)?;
        let version = select_effective_version(&file, &file_key, &overlays);

        report_job_stage(progress.as_ref(), "preparing_snapshot", 35).await;
        let (context, prepared) = match prepare_ephemeral_mcp_operation(
            SemanticOperation::Definition,
            false,
            deps_id.clone(),
            deps.clone(),
            index_snapshot,
            Arc::from(text),
            version,
            Arc::from(abs_path.to_string_lossy().to_string()),
            DetailLevel::Full,
            coordinator.as_ref(),
        ) {
            Ok(values) => values,
            Err(_) => {
                return Ok(BslDefinitionResponse {
                    analysis_revision,
                    location: None,
                    snippet: None,
                });
            }
        };

        let analysis = prepared.snapshot.analysis;
        report_job_stage(progress.as_ref(), "querying_ir", 60).await;
        let program_query = IntellisenseV2Facade::run_optional_query(
            &context,
            ObservabilityStage::IrQuery,
            &analysis,
            Some(coordinator.as_ref()),
            |analysis| analysis.ir(FileId(1)),
        );
        let Some(program) = program_query.ok().flatten() else {
            return Ok(BslDefinitionResponse {
                analysis_revision,
                location: None,
                snippet: None,
            });
        };

        let Some(code) = analysis.file_text(FileId(1)).ok().flatten() else {
            return Ok(BslDefinitionResponse {
                analysis_revision,
                location: None,
                snippet: None,
            });
        };
        report_job_stage(progress.as_ref(), "resolving_definition", 85).await;
        let type_at_position_hint = type_at_utf16_position(
            &analysis,
            FileId(1),
            position.line,
            position.character,
            false,
            Some(coordinator.as_ref()),
        );
        let receiver_type_hint = definition_receiver_type_hint_at_position(
            &analysis,
            program.as_ref(),
            FileId(1),
            code.as_ref(),
            position.line,
            position.character,
            Some(coordinator.as_ref()),
        );
        let target = bsl_runtime::application::type_system::goto_definition_v2_with_source(
            abs_path.to_string_lossy().as_ref(),
            code.as_ref(),
            program,
            deps,
            position.line,
            position.character,
            type_at_position_hint,
            receiver_type_hint,
        );

        let Some(target) = target else {
            return Ok(BslDefinitionResponse {
                analysis_revision,
                location: None,
                snippet: None,
            });
        };

        let location = match map_path_to_root(&roots, &target.file_path) {
            Some((root_id, rel_path)) => {
                let range = match target.span {
                    Some(span) if target.file_path == abs_path => span_to_range_with_index(
                        code.as_ref(),
                        &bsl_analysis_v2::LineIndex::new(code.as_ref()),
                        span,
                    ),
                    _ => RangeDto::default(),
                };
                Some(LocationDto {
                    file: DocumentRefDto {
                        root_id,
                        path: rel_path,
                    },
                    range,
                })
            }
            None => None,
        };

        report_job_stage(progress.as_ref(), "finalizing", 95).await;
        Ok(BslDefinitionResponse {
            analysis_revision,
            location,
            snippet: None,
        })
    }

    pub async fn bsl_references(
        &self,
        params: BslReferencesParams,
    ) -> Result<BslReferencesResponse, rmcp::ErrorData> {
        self.bsl_references_with_progress(params, None).await
    }

    pub(crate) async fn bsl_references_with_progress(
        &self,
        params: BslReferencesParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<BslReferencesResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        report_job_stage(progress.as_ref(), "resolving_symbol", 10).await;
        let (roots, analysis_revision, deps_id, deps, index_snapshot, symbol, coordinator) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let startup = session.startup.as_ref().ok_or_else(|| {
                rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
            })?;
            let symbol = session
                .id_map
                .get_symbol(session.analysis_revision, &params.symbol_id)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("stale or unknown symbol_id", None))?
                .clone();
            (
                session.roots.clone(),
                session.analysis_revision,
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
                startup.deps_bundle_v2.index_snapshot.clone(),
                symbol,
                startup.coordinator.clone(),
            )
        };

        if symbol.kind != "function" && symbol.kind != "procedure" {
            report_job_stage(progress.as_ref(), "finalizing", 95).await;
            return Ok(BslReferencesResponse {
                analysis_revision,
                count: 0,
                references: Vec::new(),
                truncated: false,
            });
        }

        let files = collect_project_files(&roots)?;
        let total_files = files.len();
        let mut references = Vec::new();
        let mut truncated = false;
        let mut total_read_bytes = 0u64;
        report_batch_progress(progress.as_ref(), "scanning_files", 0, total_files, 20, 85).await;

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

            let remaining = params.limit as usize - references.len();
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
            let symbol_name = symbol.name.clone();

            let file_references =
                bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                    bsl_runtime::application::CpuWorkClass::Background,
                    ObservabilityOrigin::Agent.as_str(),
                    Some(coordinator.as_ref()),
                    move || -> Result<Vec<ReferenceDto>, rmcp::ErrorData> {
                        let (context, prepared) = match prepare_ephemeral_mcp_operation(
                            SemanticOperation::References,
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
                            let bsl_shared::ir::SemanticNodeKind::FunctionCall {
                                function_name,
                                object_name,
                                object_node,
                                ..
                            } = &node.kind
                            else {
                                continue;
                            };
                            if object_name.is_some() || object_node.is_some() {
                                continue;
                            }
                            if !function_name.eq_ignore_ascii_case(&symbol_name) {
                                continue;
                            }
                            if out.len() >= remaining {
                                break;
                            }

                            out.push(ReferenceDto {
                                file: DocumentRefDto {
                                    root_id: file_root_id.clone(),
                                    path: file_rel_path.clone(),
                                },
                                range: span_to_range_with_index(
                                    code.as_ref(),
                                    line_index.as_ref(),
                                    node.span,
                                ),
                            });
                        }
                        Ok(out)
                    },
                )
                .await
                .map_err(|err| {
                    rmcp::ErrorData::internal_error(
                        format!("references worker task join failed: {err}"),
                        None,
                    )
                })??;

            references.extend(file_references);
            report_batch_progress(
                progress.as_ref(),
                "scanning_files",
                index + 1,
                total_files,
                20,
                85,
            )
            .await;
            if references.len() >= params.limit as usize {
                truncated = true;
                break;
            }
        }

        report_job_stage(progress.as_ref(), "finalizing", 95).await;
        sort_references(&mut references);
        if references.len() > params.limit as usize {
            references.truncate(params.limit as usize);
            truncated = true;
        }

        Ok(BslReferencesResponse {
            analysis_revision,
            count: references.len() as u64,
            references,
            truncated,
        })
    }
}
