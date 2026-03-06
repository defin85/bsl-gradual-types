impl SessionManager {
    pub async fn context_pack(
        &self,
        params: ContextPackParams,
    ) -> Result<ContextPackResponse, rmcp::ErrorData> {
        self.context_pack_with_progress(params, None).await
    }

    pub(crate) async fn context_pack_with_progress(
        &self,
        params: ContextPackParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<ContextPackResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        report_job_stage(progress.as_ref(), "collecting_context", 5).await;
        let (
            analysis_revision,
            roots,
            overlays,
            _hot_set,
            settings,
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
                session.analysis_revision,
                session.roots.clone(),
                session.documents.overlays.clone(),
                session.documents.hot_set.clone(),
                session.settings.clone(),
                startup.deps_bundle_v2.deps_id.clone(),
                startup.deps_bundle_v2.semantic_deps.clone(),
                startup.deps_bundle_v2.index_snapshot.clone(),
                startup.coordinator.clone(),
            )
        };

        let budget_chars = compute_budget_chars(params.budget_chars, params.budget_tokens);
        let budget_chars_u32 = if budget_chars > u32::MAX as usize {
            u32::MAX
        } else {
            budget_chars as u32
        };

        let goal = params.goal.unwrap_or_default();
        let scope = normalize_workspace_scope(
            params
                .scope
                .unwrap_or(WorkspaceScope::Tagged(WorkspaceScopeTagged::Hot)),
        )?;
        let include_key = include_fingerprint(&params.include);
        let scope_key = scope_key_for_pack(&roots, &scope)?;
        let focus_key = match params.focus.as_ref() {
            Some(focus) => focus_key_for_pack(&roots, focus)?,
            None => "none".to_string(),
        };

        let pack_id = ids::pack_id(
            analysis_revision,
            goal.as_str(),
            focus_key.as_str(),
            scope_key.as_str(),
            include_key.as_str(),
            budget_chars_u32,
        );

        let missing_inputs = workspace_missing_inputs(&settings);
        let completeness = if missing_inputs.is_empty() {
            CompletenessDto::Full
        } else {
            CompletenessDto::Partial
        };

        let mut text = TextBudget::new(budget_chars);
        text.push_line("bsl-agent context_pack");
        if !goal.is_empty() {
            text.push_line(&format!("goal: {goal}"));
        }
        text.push_line(&format!("analysis_revision: {analysis_revision}"));
        text.push_line(&format!(
            "completeness: {}",
            match completeness {
                CompletenessDto::Full => "full",
                CompletenessDto::Partial => "partial",
            }
        ));
        if !missing_inputs.is_empty() {
            text.push_line(&format!("missing_inputs: {}", missing_inputs.join(", ")));
        }
        text.push_line("");

        let mut items: Vec<ContextPackItemDto> = Vec::new();
        let mut stored_items: HashMap<String, StoredPackItem> = HashMap::new();
        let mut truncated = false;

        match params.focus {
            Some(ContextFocus::Position { file, position }) => {
                report_job_stage(progress.as_ref(), "focus_position", 25).await;
                let file_key = document_key_from_ref(&roots, &file.doc)?;
                let (root_path, abs_path, file_ref) = resolve_doc_path(&roots, &file_key)?;
                let source_text =
                    select_effective_text(&file, &file_key, &overlays, &root_path, &abs_path)?;

                text.push_line(&format!(
                    "focus: position {}:{}:{}",
                    file_ref.path,
                    position.line + 1,
                    position.character + 1
                ));
                text.push_line("");

                if params.include.snippets {
                    report_job_stage(progress.as_ref(), "rendering_snippet", 45).await;
                    let center_line = position.line;
                    let snippet =
                        render_snippet(&source_text, center_line, PACK_SNIPPET_CONTEXT_LINES);
                    let primary = format!(
                        "{}:{}:{}",
                        ids::document_id(&file_ref.root_id, &file_ref.path),
                        center_line,
                        PACK_SNIPPET_CONTEXT_LINES
                    );
                    let item_id = ids::pack_item_id(&pack_id, "snippet", &primary);
                    stored_items.insert(
                        item_id.clone(),
                        StoredPackItem::Snippet {
                            file: file_ref.clone(),
                            center_line,
                        },
                    );
                    items.push(ContextPackItemDto {
                        item_id: item_id.clone(),
                        kind: "snippet".to_string(),
                        file: Some(file_ref.clone()),
                        range: Some(snippet.range),
                        summary: format!("Snippet around {}:{}", file_ref.path, center_line + 1),
                    });
                    text.push_line("```bsl");
                    text.push_str(&snippet.text);
                    text.push_line("```");
                    text.push_line("");
                    truncated |= snippet.truncated;
                }

                if params.include.types {
                    report_job_stage(progress.as_ref(), "resolving_type", 70).await;
                    let version = select_effective_version(&file, &file_key, &overlays);
                    if let Ok((context, prepared)) = prepare_ephemeral_mcp_operation(
                        SemanticOperation::TypeAtPosition,
                        false,
                        deps_id.clone(),
                        deps.clone(),
                        index_snapshot.clone(),
                        Arc::from(source_text.clone()),
                        version,
                        Arc::from(abs_path.to_string_lossy().to_string()),
                        DetailLevel::Full,
                        coordinator.as_ref(),
                    ) {
                        let analysis = prepared.snapshot.analysis;
                        let program = IntellisenseV2Facade::run_optional_query(
                            &context,
                            ObservabilityStage::IrQuery,
                            &analysis,
                            Some(coordinator.as_ref()),
                            |analysis| analysis.ir(FileId(1)),
                        )
                        .ok()
                        .flatten();
                        if let Some(program) = program {
                            let _ = program;
                            if let Some(type_info) = type_at_utf16_position(
                                &analysis,
                                FileId(1),
                                position.line,
                                position.character,
                                false,
                            ) {
                                text.push_line(&format!(
                                    "type_at_position: {}",
                                    user_facing_resolution_type_name(&type_info)
                                ));
                                text.push_line("");
                            }
                        }
                    }
                }

                let _ = index_snapshot.id.as_str();
            }
            Some(ContextFocus::Diagnostic { diagnostic_id }) => {
                report_job_stage(progress.as_ref(), "loading_diagnostics", 35).await;
                text.push_line(&format!("focus: diagnostic {diagnostic_id}"));
                text.push_line("");

                let diagnostics = self
                    .bsl_diagnostics(BslDiagnosticsParams {
                        session_id: params.session_id.clone(),
                        scope: WorkspaceScope::Tagged(scope.clone()),
                        limit: 500,
                        include_impact: false,
                        include_coverage: false,
                        include_flow_sensitive: false,
                    })
                    .await?;
                let diagnostic = diagnostics
                    .diagnostics
                    .iter()
                    .find(|diag| diag.diagnostic_id == diagnostic_id)
                    .ok_or_else(|| {
                        rmcp::ErrorData::invalid_params("stale or unknown diagnostic_id", None)
                    })?;

                if params.include.snippets {
                    report_job_stage(progress.as_ref(), "rendering_snippet", 70).await;
                    let doc = DocumentRef::Canonical(CanonicalDocumentRef {
                        root_id: diagnostic.file.root_id.clone(),
                        path: diagnostic.file.path.clone(),
                    });
                    let file_key = document_key_from_ref(&roots, &doc)?;
                    let (root_path, abs_path, file_ref) = resolve_doc_path(&roots, &file_key)?;
                    let file = FileRef {
                        doc,
                        text: None,
                        version: None,
                    };
                    let source_text =
                        select_effective_text(&file, &file_key, &overlays, &root_path, &abs_path)?;

                    let center_line = diagnostic.range.start.line;
                    let snippet =
                        render_snippet(&source_text, center_line, PACK_SNIPPET_CONTEXT_LINES);
                    let primary = format!(
                        "{}:{}:{}",
                        ids::document_id(&file_ref.root_id, &file_ref.path),
                        center_line,
                        PACK_SNIPPET_CONTEXT_LINES
                    );
                    let item_id = ids::pack_item_id(&pack_id, "snippet", &primary);
                    stored_items.insert(
                        item_id.clone(),
                        StoredPackItem::Snippet {
                            file: file_ref.clone(),
                            center_line,
                        },
                    );
                    items.push(ContextPackItemDto {
                        item_id: item_id.clone(),
                        kind: "snippet".to_string(),
                        file: Some(file_ref.clone()),
                        range: Some(snippet.range),
                        summary: format!(
                            "Snippet for diagnostic in {}:{}",
                            file_ref.path,
                            center_line + 1
                        ),
                    });
                    text.push_line(&format!("diagnostic: {}", diagnostic.message));
                    text.push_line("```bsl");
                    text.push_str(&snippet.text);
                    text.push_line("```");
                    text.push_line("");
                    truncated |= snippet.truncated;
                }
                truncated |= diagnostics.truncated;
            }
            Some(ContextFocus::Symbol { symbol_id }) => {
                report_job_stage(progress.as_ref(), "resolving_symbol", 35).await;
                let symbol = {
                    let sessions = self.sessions.read().await;
                    let session = sessions.get(&uuid).ok_or_else(|| {
                        rmcp::ErrorData::invalid_params("session not found", None)
                    })?;
                    session
                        .id_map
                        .get_symbol(session.analysis_revision, &symbol_id)
                        .ok_or_else(|| {
                            rmcp::ErrorData::invalid_params("stale or unknown symbol_id", None)
                        })?
                        .clone()
                };

                text.push_line(&format!(
                    "focus: symbol {} {} ({})",
                    symbol.kind, symbol.name, symbol_id
                ));
                text.push_line(&format!(
                    "definition: {}:{}",
                    symbol.file.path,
                    symbol.range.start.line + 1
                ));
                text.push_line("");

                if params.include.snippets {
                    report_job_stage(progress.as_ref(), "rendering_snippet", 55).await;
                    let doc = DocumentRef::Canonical(CanonicalDocumentRef {
                        root_id: symbol.file.root_id.clone(),
                        path: symbol.file.path.clone(),
                    });
                    let file_key = document_key_from_ref(&roots, &doc)?;
                    let (root_path, abs_path, file_ref) = resolve_doc_path(&roots, &file_key)?;
                    let file = FileRef {
                        doc,
                        text: None,
                        version: None,
                    };
                    let source_text =
                        select_effective_text(&file, &file_key, &overlays, &root_path, &abs_path)?;
                    let center_line = symbol.range.start.line;
                    let snippet =
                        render_snippet(&source_text, center_line, PACK_SNIPPET_CONTEXT_LINES);
                    let primary = format!(
                        "{}:{}:{}",
                        ids::document_id(&file_ref.root_id, &file_ref.path),
                        center_line,
                        PACK_SNIPPET_CONTEXT_LINES
                    );
                    let item_id = ids::pack_item_id(&pack_id, "snippet", &primary);
                    stored_items.insert(
                        item_id.clone(),
                        StoredPackItem::Snippet {
                            file: file_ref.clone(),
                            center_line,
                        },
                    );
                    items.push(ContextPackItemDto {
                        item_id: item_id.clone(),
                        kind: "snippet".to_string(),
                        file: Some(file_ref.clone()),
                        range: Some(snippet.range),
                        summary: format!("Snippet for symbol {} in {}", symbol.name, file_ref.path),
                    });
                    text.push_line("```bsl");
                    text.push_str(&snippet.text);
                    text.push_line("```");
                    text.push_line("");
                    truncated |= snippet.truncated;
                }

                if params.include.references {
                    report_job_stage(progress.as_ref(), "loading_references", 75).await;
                    let refs = self
                        .bsl_references(BslReferencesParams {
                            session_id: params.session_id.clone(),
                            symbol_id,
                            limit: 50,
                            include_snippets: false,
                        })
                        .await?;
                    text.push_line(&format!("references: {}", refs.count));
                    text.push_line("");
                    truncated |= refs.truncated;
                }
            }
            Some(ContextFocus::Query { query }) => {
                report_job_stage(progress.as_ref(), "searching_symbols", 60).await;
                text.push_line(&format!("focus: query {query:?}"));
                text.push_line("");

                if params.include.symbols {
                    let response = self
                        .bsl_symbol_search(BslSymbolSearchParams {
                            session_id: params.session_id.clone(),
                            query,
                            limit: 20,
                        })
                        .await?;
                    if !response.symbols.is_empty() {
                        text.push_line("symbols:");
                        for symbol in &response.symbols {
                            text.push_line(&format!(
                                "- {} {} ({}:{})",
                                symbol.kind,
                                symbol.name,
                                symbol.file.path,
                                symbol.range.start.line + 1
                            ));
                        }
                        text.push_line("");
                    }
                    truncated |= response.truncated;
                }
            }
            None => {
                report_job_stage(progress.as_ref(), "assembling_pack", 60).await;
                text.push_line("focus: none");
                text.push_line("");
            }
        }

        report_job_stage(progress.as_ref(), "finalizing", 95).await;
        sort_pack_items(&mut items);

        let text_truncated = text.truncated;
        let pack_truncated = text_truncated || truncated;

        let stored_pack = StoredPack {
            items: stored_items,
        };
        {
            let mut sessions = self.sessions.write().await;
            let session = sessions
                .get_mut(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            if session.analysis_revision != analysis_revision {
                return Err(rmcp::ErrorData::invalid_params(
                    "analysis_revision changed; retry",
                    None,
                ));
            }
            session
                .pack_store
                .insert_pack(analysis_revision, pack_id.clone(), stored_pack);
        }

        Ok(ContextPackResponse {
            analysis_revision,
            pack_id,
            text: text.finish(),
            items,
            truncated: pack_truncated,
            completeness,
            missing_inputs,
        })
    }

    pub async fn context_expand(
        &self,
        params: ContextExpandParams,
    ) -> Result<ContextExpandResponse, rmcp::ErrorData> {
        self.context_expand_with_progress(params, None).await
    }

    pub(crate) async fn context_expand_with_progress(
        &self,
        params: ContextExpandParams,
        progress: Option<SemanticJobProgress>,
    ) -> Result<ContextExpandResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(&params.session_id)?;
        report_job_stage(progress.as_ref(), "resolving_item", 15).await;
        let (analysis_revision, roots, overlays, item) = {
            let sessions = self.sessions.read().await;
            let session = sessions
                .get(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
            let item = session
                .pack_store
                .get_item(session.analysis_revision, &params.pack_id, &params.item_id)
                .ok_or_else(|| {
                    rmcp::ErrorData::invalid_params("stale or unknown pack_id/item_id", None)
                })?
                .clone();
            (
                session.analysis_revision,
                session.roots.clone(),
                session.documents.overlays.clone(),
                item,
            )
        };

        let budget_chars = compute_budget_chars(params.budget_chars, params.budget_tokens);
        let mut text = TextBudget::new(budget_chars);

        match item {
            StoredPackItem::Snippet { file, center_line } => {
                report_job_stage(progress.as_ref(), "rendering_snippet", 80).await;
                let doc = DocumentRef::Canonical(CanonicalDocumentRef {
                    root_id: file.root_id.clone(),
                    path: file.path.clone(),
                });
                let file_key = document_key_from_ref(&roots, &doc)?;
                let (root_path, abs_path, file_ref) = resolve_doc_path(&roots, &file_key)?;
                let file = FileRef {
                    doc,
                    text: None,
                    version: None,
                };
                let source_text =
                    select_effective_text(&file, &file_key, &overlays, &root_path, &abs_path)?;
                let snippet =
                    render_snippet(&source_text, center_line, EXPAND_SNIPPET_CONTEXT_LINES);
                text.push_line(&format!(
                    "snippet {}:{} (+/-{} lines)",
                    file_ref.path,
                    center_line + 1,
                    EXPAND_SNIPPET_CONTEXT_LINES
                ));
                text.push_line("```bsl");
                text.push_str(&snippet.text);
                text.push_line("```");
                let _ = snippet.truncated;
            }
        }

        let truncated = text.truncated;
        report_job_stage(progress.as_ref(), "finalizing", 95).await;
        Ok(ContextExpandResponse {
            analysis_revision,
            text: text.finish(),
            truncated,
        })
    }
}
