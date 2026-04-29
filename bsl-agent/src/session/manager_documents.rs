impl SessionManager {
    async fn set_startup_job_id(
        &self,
        session_id: Uuid,
        startup_job_id: String,
    ) -> Result<(), rmcp::ErrorData> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        session.startup_job_id = Some(startup_job_id);
        drop(sessions);

        self.persist_session(session_id).await;
        Ok(())
    }

    async fn set_startup_progress(&self, session_id: Uuid, phase: String, percent: u8) {
        let mut sessions = self.sessions.write().await;
        let Some(session) = sessions.get_mut(&session_id) else {
            return;
        };
        if session.startup.is_some() {
            return;
        }
        session.startup_phase = phase;
        let percent = percent.min(99);
        session.startup_progress = session.startup_progress.max(percent);
    }

    async fn set_startup_result(
        &self,
        session_id: Uuid,
        startup: bsl_runtime::system::StartupResultV2,
    ) -> Result<(), rmcp::ErrorData> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        session.startup = Some(startup);
        session.startup_phase = "idle".to_string();
        session.startup_progress = 100;
        session.startup_error = None;
        drop(sessions);

        self.persist_session(session_id).await;
        Ok(())
    }

    async fn set_startup_error(&self, session_id: Uuid, error: String) {
        let mut sessions = self.sessions.write().await;
        let Some(session) = sessions.get_mut(&session_id) else {
            return;
        };
        if session.startup.is_some() {
            return;
        }
        session.startup_phase = "startup/failed".to_string();
        session.startup_progress = 100;
        session.startup_error = Some(error);
    }

    async fn persist_session(&self, session_id: Uuid) {
        let Some(store) = self.store.as_ref() else {
            return;
        };
        let sessions = self.sessions.read().await;
        let Some(session) = sessions.get(&session_id) else {
            return;
        };

        let mut persisted = PersistedSession {
            session_id: session_id.to_string(),
            roots: session
                .roots
                .iter()
                .map(|root| root.path.to_string_lossy().to_string())
                .collect(),
            platform_docs_archive: session
                .settings
                .platform_docs_archive
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            platform_version: session.settings.platform_version.clone(),
            configuration_path: session
                .settings
                .configuration_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            rules_config_path: session
                .settings
                .rules_config_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            mode: session.settings.mode.clone(),
            analysis_revision: session.analysis_revision,
            created_at: session.created_at,
            updated_at: crate::state::now_unix_secs(),
            startup_job_id: session.startup_job_id.clone(),
            env_overrides: session.settings.env_overrides.clone(),
            dev_env_overrides: session.settings.dev_env_overrides.clone(),
            allow_dev_overrides: session.settings.allow_dev_overrides,
        };
        store.upsert(&mut persisted);
    }

    pub async fn settings_get(
        &self,
        session_id: &str,
    ) -> Result<WorkspaceGetSettingsResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;

        let snapshot = global_runtime_config().snapshot();
        let runtime_config = serde_json::to_value(snapshot)
            .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?;

        Ok(WorkspaceGetSettingsResponse {
            session_id: session_id.to_string(),
            allow_dev_overrides: session.settings.allow_dev_overrides,
            env_overrides: session.settings.env_overrides.clone(),
            dev_env_overrides: session.settings.dev_env_overrides.clone(),
            runtime_config,
        })
    }

    pub async fn settings_update(
        &self,
        session_id: &str,
        env_patch: Option<&HashMap<String, serde_json::Value>>,
        dev_patch: Option<&HashMap<String, serde_json::Value>>,
        allow_dev_overrides: Option<bool>,
    ) -> Result<WorkspaceUpdateSettingsResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;

        fn apply_patch(
            target: &mut HashMap<String, serde_json::Value>,
            patch: &HashMap<String, serde_json::Value>,
        ) {
            for (k, v) in patch {
                if v.is_null() {
                    target.remove(k);
                } else {
                    target.insert(k.clone(), v.clone());
                }
            }
        }

        let (stable_overrides, dev_overrides, allow_dev, has_startup) = {
            let mut sessions = self.sessions.write().await;
            let session = sessions
                .get_mut(&uuid)
                .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;

            if let Some(patch) = env_patch {
                apply_patch(&mut session.settings.env_overrides, patch);
            }
            if let Some(patch) = dev_patch {
                apply_patch(&mut session.settings.dev_env_overrides, patch);
            }
            if let Some(value) = allow_dev_overrides {
                session.settings.allow_dev_overrides = value;
            }

            (
                session.settings.env_overrides.clone(),
                session.settings.dev_env_overrides.clone(),
                session.settings.allow_dev_overrides,
                session.startup.is_some(),
            )
        };

        let store = global_runtime_config();
        let stable_report = store.replace_stable_overrides(&stable_overrides);

        // Ensure dev-only layer is cleared when opt-in is disabled, even if the client keeps values around.
        let dev_report = if allow_dev {
            store.replace_dev_overrides(&dev_overrides, true)
        } else {
            let empty: HashMap<String, serde_json::Value> = HashMap::new();
            let mut report = store.replace_dev_overrides(&empty, true);
            if !dev_overrides.is_empty() {
                report.dev_overrides_ignored = true;
            }
            report
        };

        let mut report = stable_report;
        report
            .requires_restart_keys
            .extend(dev_report.requires_restart_keys.iter().cloned());
        report.requires_restart_keys.sort();
        report.requires_restart_keys.dedup();
        report
            .ignored_unknown_keys
            .extend(dev_report.ignored_unknown_keys);
        report
            .ignored_invalid_values
            .extend(dev_report.ignored_invalid_values);
        report
            .ignored_wrong_tier_keys
            .extend(dev_report.ignored_wrong_tier_keys);
        report.dev_overrides_ignored |= dev_report.dev_overrides_ignored;

        // Apply a minimal runtime sync for coordinator-level settings.
        if has_startup {
            let coordinator = {
                let sessions = self.sessions.read().await;
                sessions.get(&uuid).and_then(|session| {
                    session.startup.as_ref().map(|s| Arc::clone(&s.coordinator))
                })
            };

            if let Some(coordinator) = coordinator {
                let cache_disable = store.get_bool(RuntimeKey::CacheDisable).unwrap_or(false);
                let desired_cache_enabled = !cache_disable;
                // NOTE: changing cache root dir at runtime is not supported yet (startup-only).
                let effective_cache_enabled = coordinator
                    .set_cache_enabled(desired_cache_enabled)
                    .await
                    .effective;

                let strict = store
                    .get_bool(RuntimeKey::CacheStrictFingerprint)
                    .unwrap_or(false);
                coordinator.set_strict_fingerprint(strict);

                let mut sessions = self.sessions.write().await;
                if let Some(session) = sessions.get_mut(&uuid) {
                    if let Some(startup) = session.startup.as_mut() {
                        startup.inputs.cache_enabled = effective_cache_enabled;
                        startup.inputs.strict_fingerprint = strict;
                    }
                }
            }
        }

        self.persist_session(uuid).await;

        let snapshot = store.snapshot();
        let runtime_config = serde_json::to_value(snapshot)
            .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?;

        Ok(WorkspaceUpdateSettingsResponse {
            ok: true,
            session_id: session_id.to_string(),
            allow_dev_overrides: allow_dev,
            env_overrides: stable_overrides,
            dev_env_overrides: dev_overrides,
            report: RuntimeSettingsReportDto::from(report),
            runtime_config,
        })
    }

    pub async fn observability_metrics_get(
        &self,
        session_id: &str,
    ) -> Result<WorkspaceObservabilityMetricsResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        let startup = session.startup.as_ref().ok_or_else(|| {
            rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
        })?;
        Ok(WorkspaceObservabilityMetricsResponse {
            metrics: startup.coordinator.observability_metrics(),
        })
    }

    pub async fn analysis_revision(&self, session_id: &str) -> Result<u64, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        Ok(session.analysis_revision)
    }

    pub async fn record_diagnostics_pipeline_event(
        &self,
        session_id: &str,
        trigger: DiagnosticsTrigger,
        profile: DiagnosticsProfile,
        reason: DiagnosticsDisposition,
    ) -> Result<(), rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;
        let startup = session.startup.as_ref().ok_or_else(|| {
            rmcp::ErrorData::invalid_params("workspace not ready (startup in progress)", None)
        })?;
        startup
            .coordinator
            .record_intellisense_v2_diagnostics_pipeline_event(
                ObservabilityOrigin::Agent.as_str(),
                trigger.as_str(),
                profile.as_str(),
                reason.as_str(),
            );
        Ok(())
    }

    pub async fn documents_set(
        &self,
        session_id: &str,
        files: &[WorkspaceDocumentsSetFile],
        mark_hot: bool,
    ) -> Result<WorkspaceDocumentsSetResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;
        let mut did_change_revision = false;
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;

        let mut changed = false;
        for file in files {
            let mut apply = |doc: &DocumentRef,
                             text: Option<&String>,
                             version: Option<u64>|
             -> Result<(), rmcp::ErrorData> {
                let key = session.document_key(doc)?;
                if let Some(text) = text {
                    let Some(version) = version else {
                        return Err(rmcp::ErrorData::invalid_params(
                            "version is required when text is provided",
                            None,
                        ));
                    };
                    if text.len() > MAX_OVERLAY_BYTES {
                        return Err(rmcp::ErrorData::invalid_params(
                            format!("overlay text exceeds MAX_OVERLAY_BYTES={MAX_OVERLAY_BYTES}"),
                            None,
                        ));
                    }
                    let overlay = DocumentOverlay {
                        text: text.clone(),
                        version,
                    };
                    changed |= session.documents.set_overlay(key.clone(), overlay);
                }

                if mark_hot {
                    changed |= session.documents.mark_hot(key);
                }
                Ok(())
            };

            match file {
                WorkspaceDocumentsSetFile::File(file) => {
                    apply(&file.doc, file.text.as_ref(), file.version)?
                }
                WorkspaceDocumentsSetFile::Document(doc) => apply(doc, None, None)?,
                WorkspaceDocumentsSetFile::Path(path) => {
                    let doc = DocumentRef::Path(path.clone());
                    apply(&doc, None, None)?
                }
            }
        }

        if changed {
            session.analysis_revision = session.analysis_revision.saturating_add(1);
            session.id_map.reset(session.analysis_revision);
            session.pack_store.reset(session.analysis_revision);
            did_change_revision = true;
        }

        let analysis_revision = session.analysis_revision;
        drop(sessions);
        if did_change_revision {
            self.persist_session(uuid).await;
        }

        Ok(WorkspaceDocumentsSetResponse {
            ok: true,
            analysis_revision,
        })
    }

    pub async fn documents_clear(
        &self,
        session_id: &str,
        documents: &[DocumentRef],
        clear_hot: bool,
    ) -> Result<WorkspaceDocumentsClearResponse, rmcp::ErrorData> {
        let uuid = parse_session_id(session_id)?;
        let mut did_change_revision = false;
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&uuid)
            .ok_or_else(|| rmcp::ErrorData::invalid_params("session not found", None))?;

        let mut changed = false;
        for doc in documents {
            let key = session.document_key(doc)?;
            changed |= session.documents.clear_overlay(&key);
            if clear_hot {
                changed |= session.documents.clear_hot(&key);
            }
        }

        if changed {
            session.analysis_revision = session.analysis_revision.saturating_add(1);
            session.id_map.reset(session.analysis_revision);
            session.pack_store.reset(session.analysis_revision);
            did_change_revision = true;
        }

        let analysis_revision = session.analysis_revision;
        drop(sessions);
        if did_change_revision {
            self.persist_session(uuid).await;
        }

        Ok(WorkspaceDocumentsClearResponse {
            ok: true,
            analysis_revision,
        })
    }
}
