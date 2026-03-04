use super::*;

impl BslLanguageServer {
    pub(super) async fn lsp_formatting(
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

    pub(super) async fn lsp_range_formatting(
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

    pub(super) async fn lsp_document_symbol(
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

    pub(super) async fn lsp_references(
        &self,
        params: ReferenceParams,
    ) -> JsonRpcResult<Option<Vec<Location>>> {
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

    pub(super) async fn lsp_prepare_rename(
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

    pub(super) async fn lsp_rename(
        &self,
        params: RenameParams,
    ) -> JsonRpcResult<Option<WorkspaceEdit>> {
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

    pub(super) async fn lsp_symbol(
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
                super::super::V2FileKey::Path(path) => Url::from_file_path(path).ok(),
                super::super::V2FileKey::Url(raw) => Url::parse(&raw).ok(),
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
}
