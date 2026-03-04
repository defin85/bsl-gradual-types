#[salsa::db]
#[derive(Clone)]
pub struct AnalysisDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for AnalysisDatabase {}

impl Default for AnalysisDatabase {
    fn default() -> Self {
        let storage = salsa::Storage::<Self>::builder()
            .event_callback(Box::new(record_analysis_database_salsa_event))
            .build();
        Self { storage }
    }
}

pub struct AnalysisHostV2 {
    db: AnalysisDatabase,
    files: HashMap<FileId, SourceFile>,
    parse_snapshots: HashMap<FileId, ParseSnapshot>,
    derived_cache: Arc<std::sync::Mutex<DerivedArtifactsCache>>,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
}

impl Default for AnalysisHostV2 {
    fn default() -> Self {
        let db = AnalysisDatabase::default();
        let repository = Arc::new(InMemoryTypeRepository::new()) as Arc<dyn TypeRepository>;
        let platform_signatures_loaded = repository.platform_docs_loaded();
        let deps_data = Arc::new(SemanticDeps {
            signature_index: repository.get_signature_index_clone(),
            resolver: Some(Arc::new(TypeResolver::new(repository.clone()))),
            repository,
            platform_signatures_loaded,
        });
        let deps = DepsSnapshot::new(
            &db,
            DepsSnapshotId::from_hash(""),
            DepsDataSnapshot(deps_data),
        );
        let settings = SettingsSnapshot::new(&db, SettingsId::from_hash(""), DetailLevel::Full);
        Self {
            db,
            files: HashMap::new(),
            parse_snapshots: HashMap::new(),
            derived_cache: Arc::new(std::sync::Mutex::new(DerivedArtifactsCache::default())),
            deps,
            settings,
        }
    }
}

impl AnalysisHostV2 {
    pub fn apply_change(&mut self, change: Change) -> TypeIndexCacheChangeEffects {
        let mut effects = TypeIndexCacheChangeEffects::default();
        match change {
            Change::SetFile {
                file_id,
                text,
                version,
                path,
            } => {
                let outcome = self.set_file_with_outcome(file_id, text, version, path);
                effects.evicted_per_file_window_total = outcome.evicted_per_file_window_total;
            }
            Change::SetFileWithSnapshot {
                file_id,
                text,
                version,
                path,
                parse_snapshot,
            } => {
                let outcome = self.set_file_with_snapshot_with_outcome(
                    file_id,
                    text,
                    version,
                    path,
                    parse_snapshot,
                );
                effects.evicted_per_file_window_total = outcome.evicted_per_file_window_total;
            }
            Change::RemoveFile { file_id } => {
                self.files.remove(&file_id);
                self.parse_snapshots.remove(&file_id);
                self.derived_cache
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clear_file(file_id);
            }
            Change::SetDepsSnapshot { deps_id, deps } => {
                self.deps.set_id(&mut self.db).to(deps_id.clone());
                self.deps.set_data(&mut self.db).to(DepsDataSnapshot(deps));
                effects.invalidated_deps_total = self
                    .derived_cache
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .invalidate_type_index_for_deps(&deps_id);
            }
            Change::SetSettingsSnapshot {
                settings_id,
                diagnostics_detail_level,
            } => {
                self.settings.set_id(&mut self.db).to(settings_id.clone());
                self.settings
                    .set_diagnostics_detail_level(&mut self.db)
                    .to(diagnostics_detail_level);
                effects.invalidated_settings_total = self
                    .derived_cache
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .invalidate_type_index_for_settings(&settings_id);
            }
        }
        effects
    }

    fn set_file_with_outcome(
        &mut self,
        file_id: FileId,
        text: Arc<str>,
        version: i32,
        path: Arc<str>,
    ) -> TypeIndexStoreOutcome {
        match self.files.get(&file_id).copied() {
            Some(file) => {
                file.set_text(&mut self.db).to(text);
                file.set_version(&mut self.db).to(version);
            }
            None => {
                let file = SourceFile::new(&self.db, file_id.0, text, version, path);
                self.files.insert(file_id, file);
            }
        }
        self.parse_snapshots.remove(&file_id);
        self.derived_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain_versions_for_file(file_id, version)
    }

    pub fn set_file(&mut self, file_id: FileId, text: Arc<str>, version: i32, path: Arc<str>) {
        let _ = self.set_file_with_outcome(file_id, text, version, path);
    }

    fn set_file_with_snapshot_with_outcome(
        &mut self,
        file_id: FileId,
        text: Arc<str>,
        version: i32,
        path: Arc<str>,
        parse_snapshot: ParseSnapshot,
    ) -> TypeIndexStoreOutcome {
        let outcome = self.set_file_with_outcome(file_id, text, version, path);
        if parse_snapshot.file_id != file_id || parse_snapshot.file_version != version {
            return outcome;
        }
        self.parse_snapshots.insert(file_id, parse_snapshot);
        outcome
    }

    pub fn set_file_with_snapshot(
        &mut self,
        file_id: FileId,
        text: Arc<str>,
        version: i32,
        path: Arc<str>,
        parse_snapshot: ParseSnapshot,
    ) {
        let _ =
            self.set_file_with_snapshot_with_outcome(file_id, text, version, path, parse_snapshot);
    }

    pub fn has_file(&self, file_id: FileId) -> bool {
        self.files.contains_key(&file_id)
    }

    pub fn deps_id(&self) -> DepsSnapshotId {
        self.deps.id(&self.db).clone()
    }

    pub fn settings_id(&self) -> SettingsId {
        self.settings.id(&self.db).clone()
    }

    pub fn snapshot(&self) -> AnalysisV2 {
        AnalysisV2 {
            db: self.db.clone(),
            files: self.files.clone(),
            parse_snapshots: self.parse_snapshots.clone(),
            derived_cache: self.derived_cache.clone(),
            deps: self.deps,
            settings: self.settings,
        }
    }

    pub fn analysis(&self) -> AnalysisV2 {
        self.snapshot()
    }
}

