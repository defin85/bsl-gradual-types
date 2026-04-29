impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSession {
    fn document_key(&self, doc: &DocumentRef) -> Result<DocumentKey, rmcp::ErrorData> {
        document_key_from_ref(&self.roots, doc)
    }
}

impl DocumentStore {
    fn set_overlay(&mut self, key: DocumentKey, overlay: DocumentOverlay) -> bool {
        match self.overlays.get(&key) {
            Some(existing) if *existing == overlay => false,
            _ => {
                self.overlays.insert(key, overlay);
                true
            }
        }
    }

    fn clear_overlay(&mut self, key: &DocumentKey) -> bool {
        self.overlays.remove(key).is_some()
    }

    fn mark_hot(&mut self, key: DocumentKey) -> bool {
        self.hot_set.insert(key)
    }

    fn clear_hot(&mut self, key: &DocumentKey) -> bool {
        self.hot_set.remove(key)
    }
}

impl IdMap {
    fn reset(&mut self, analysis_revision: u64) {
        self.analysis_revision = analysis_revision;
        self.symbols.clear();
    }

    fn get_symbol(&self, analysis_revision: u64, symbol_id: &str) -> Option<&StoredSymbol> {
        if self.analysis_revision != analysis_revision {
            return None;
        }
        self.symbols.get(symbol_id)
    }
}

impl PackStore {
    fn reset(&mut self, analysis_revision: u64) {
        self.analysis_revision = analysis_revision;
        self.packs.clear();
    }

    fn insert_pack(&mut self, analysis_revision: u64, pack_id: String, pack: StoredPack) {
        if self.analysis_revision != analysis_revision {
            self.reset(analysis_revision);
        }
        self.packs.insert(pack_id, pack);
    }

    fn get_item(
        &self,
        analysis_revision: u64,
        pack_id: &str,
        item_id: &str,
    ) -> Option<&StoredPackItem> {
        if self.analysis_revision != analysis_revision {
            return None;
        }
        self.packs.get(pack_id)?.items.get(item_id)
    }
}

fn parse_session_id(session_id: &str) -> Result<Uuid, rmcp::ErrorData> {
    Uuid::parse_str(session_id)
        .map_err(|_| rmcp::ErrorData::invalid_params("invalid session_id", None))
}

fn root_id(path: &Path) -> String {
    blake3::hash(path.to_string_lossy().as_bytes())
        .to_hex()
        .to_string()
}

fn normalize_relative_path(path: &str) -> Result<String, rmcp::ErrorData> {
    if path.is_empty() {
        return Err(rmcp::ErrorData::invalid_params(
            "path must be non-empty",
            None,
        ));
    }

    let mut components = Vec::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(value) => components.push(value.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(rmcp::ErrorData::invalid_params(
                    "path must not contain '..'",
                    None,
                ))
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(rmcp::ErrorData::invalid_params(
                    "path must be relative",
                    None,
                ))
            }
        }
    }

    if components.is_empty() {
        return Err(rmcp::ErrorData::invalid_params(
            "path must be non-empty",
            None,
        ));
    }

    Ok(components.join("/"))
}

fn workspace_missing_inputs(settings: &WorkspaceSettings) -> Vec<String> {
    let mut missing = Vec::new();
    if settings.configuration_path.is_some() && settings.platform_version.is_none() {
        missing.push("platform_version".to_string());
    }
    missing
}

fn normalize_mode(mode: Option<String>) -> Option<String> {
    let mode = mode?;
    let trimmed = mode.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") {
        return None;
    }
    Some(trimmed.to_string())
}

fn normalize_workspace_scope(
    scope: WorkspaceScope,
) -> Result<WorkspaceScopeTagged, rmcp::ErrorData> {
    match scope {
        WorkspaceScope::Tagged(value) => Ok(value),
        WorkspaceScope::Simple(value) => {
            let trimmed = value.trim();
            if trimmed.eq_ignore_ascii_case("project") {
                Ok(WorkspaceScopeTagged::Project)
            } else if trimmed.eq_ignore_ascii_case("hot") {
                Ok(WorkspaceScopeTagged::Hot)
            } else if trimmed.eq_ignore_ascii_case("file") {
                Err(rmcp::ErrorData::invalid_params(
                    "scope=\"file\" is not supported as a string; use tagged file scope: {\"kind\":\"file\",\"document\":...}",
                    None,
                ))
            } else {
                Err(rmcp::ErrorData::invalid_params(
                    format!("unknown scope: {trimmed}"),
                    None,
                ))
            }
        }
    }
}

fn infer_platform_version_from_config_dump(
    configuration_path: &Path,
) -> Result<String, rmcp::ErrorData> {
    fn inference_error(message: impl std::fmt::Display) -> rmcp::ErrorData {
        rmcp::ErrorData::invalid_params(
            format!(
                "cannot infer platform_version from configuration_path: {message}; provide platform_version explicitly (e.g. 8.3.25)"
            ),
            None,
        )
    }

    fn parse_triplet(raw: &str) -> Option<(u32, u32, u32)> {
        let trimmed = raw.trim();
        let without_prefix = trimmed.strip_prefix("Version").unwrap_or(trimmed);
        let normalized = without_prefix.replace('_', ".");
        let mut parts = normalized.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some((major, minor, patch))
    }

    let discovery = ConfigurationDiscovery::new(configuration_path.to_path_buf(), false);
    let configs = discovery
        .discover_all_configurations()
        .map_err(|err| inference_error(format!("failed to discover configurations: {err}")))?;
    if configs.is_empty() {
        return Err(inference_error(
            "no configurations found (missing Configuration.xml?)",
        ));
    }

    let mut best: Option<(u32, u32, u32)> = None;
    for config in configs {
        let raw_mode = config.compatibility_mode.as_deref().ok_or_else(|| {
            inference_error(format!(
                "CompatibilityMode is missing for configuration {}",
                config.name
            ))
        })?;
        let parsed = parse_triplet(raw_mode).ok_or_else(|| {
            inference_error(format!(
                "invalid CompatibilityMode '{}' for configuration {}",
                raw_mode, config.name
            ))
        })?;
        best = Some(best.map_or(parsed, |current| current.max(parsed)));
    }

    let (major, minor, patch) = best.expect("at least one configuration");
    Ok(format!("{major}.{minor}.{patch}"))
}

fn workspace_warnings(settings: &WorkspaceSettings) -> Vec<String> {
    let mut warnings = Vec::new();
    if settings.platform_docs_archive.is_none()
        && settings.configuration_path.is_none()
        && settings.platform_version.is_some()
    {
        warnings.push("platform_version has no effect without platform_docs_archive".to_string());
    }
    if let Some(mode) = settings.mode.as_deref() {
        if !mode.is_empty() && !mode.eq_ignore_ascii_case("progressive") {
            warnings.push(format!("unknown mode: {mode}"));
        }
    }
    if let Some(path) = settings.rules_config_path.as_deref() {
        if !path.is_file() {
            warnings.push(format!(
                "rules_config_path is not readable; default semantic rules will be used: {}",
                path.display()
            ));
        }
    }
    warnings
}

async fn start_semantic_runtime(
    settings: &WorkspaceSettings,
    progress_tx: Option<mpsc::UnboundedSender<ProgressUpdate>>,
) -> Result<bsl_runtime::system::StartupResultV2, rmcp::ErrorData> {
    // Apply unified runtime overrides before coordinator initialization so bootstrap-only settings
    // (e.g., cache root dir) are consistently picked up.
    let _stable_report = global_runtime_config().replace_stable_overrides(&settings.env_overrides);
    if settings.allow_dev_overrides {
        let _dev_report =
            global_runtime_config().replace_dev_overrides(&settings.dev_env_overrides, true);
    } else {
        let empty: HashMap<String, serde_json::Value> = HashMap::new();
        let _dev_report = global_runtime_config().replace_dev_overrides(&empty, true);
    }

    let coordinator = Arc::new(bsl_runtime::system::SystemCoordinator::new());
    let inputs = bsl_runtime::system::StartupInputs::from_web_flags(
        settings.platform_docs_archive.clone(),
        settings.configuration_path.clone(),
        settings.platform_version.clone(),
        settings.rules_config_path.clone(),
        None,
        None,
    );

    bsl_runtime::system::startup_v2(coordinator, inputs, progress_tx)
        .await
        .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))
}

async fn run_startup_job(
    session_manager: Arc<SessionManager>,
    session_id: Uuid,
    settings: WorkspaceSettings,
    ctx: JobContext,
) -> Result<serde_json::Value, anyhow::Error> {
    ctx.set_progress("startup/starting".to_string(), 0).await;
    session_manager
        .set_startup_progress(session_id, "startup/starting".to_string(), 0)
        .await;

    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();
    let ctx_for_progress = ctx.clone();
    let session_for_progress = Arc::clone(&session_manager);
    let progress_task = tokio::spawn(async move {
        while let Some(update) = progress_rx.recv().await {
            let phase = format!("startup/{:?}", update.phase);
            let percent = update.percentage.round().clamp(0.0, 100.0) as u8;
            ctx_for_progress.set_progress(phase.clone(), percent).await;
            session_for_progress
                .set_startup_progress(session_id, phase, percent)
                .await;
        }
    });

    let startup = match start_semantic_runtime(&settings, Some(progress_tx)).await {
        Ok(value) => value,
        Err(err) => {
            session_manager
                .set_startup_error(session_id, err.to_string())
                .await;
            let _ = progress_task.await;
            return Err(anyhow::anyhow!(err.to_string()));
        }
    };

    if let Err(err) = session_manager
        .set_startup_result(session_id, startup)
        .await
    {
        session_manager
            .set_startup_error(session_id, err.to_string())
            .await;
        let _ = progress_task.await;
        return Err(anyhow::anyhow!(err.to_string()));
    }

    let _ = progress_task.await;
    Ok(serde_json::json!({ "ok": true }))
}

fn restore_roots(roots_raw: &[String]) -> Result<(Vec<RootEntry>, Vec<RootDto>), rmcp::ErrorData> {
    if roots_raw.is_empty() {
        return Err(rmcp::ErrorData::invalid_params(
            "roots must be non-empty",
            None,
        ));
    }

    let mut roots = Vec::new();
    let mut root_dtos = Vec::new();
    let mut seen = HashSet::new();

    for root_raw in roots_raw {
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

    Ok((roots, root_dtos))
}
