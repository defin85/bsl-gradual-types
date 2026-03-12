pub struct AnalysisV2 {
    db: AnalysisDatabase,
    files: HashMap<FileId, SourceFile>,
    parse_snapshots: HashMap<FileId, ParseSnapshot>,
    derived_cache: Arc<std::sync::Mutex<DerivedArtifactsCache>>,
    deps: DepsSnapshot,
    settings: SettingsSnapshot,
}

impl AnalysisV2 {
    fn parse_snapshot_for_file(&self, file_id: FileId, file: SourceFile) -> Option<&ParseSnapshot> {
        let snapshot = self.parse_snapshots.get(&file_id)?;
        let file_version = file.version(&self.db);
        (snapshot.file_version == file_version).then_some(snapshot)
    }

    fn parse_snapshot_observability_mode(snapshot: &ParseSnapshot) -> &'static str {
        if snapshot.incremental {
            if snapshot.changed_ranges.is_empty() {
                "reused"
            } else {
                "incremental"
            }
        } else {
            "full"
        }
    }

    fn parse_snapshot_can_reuse_previous_ir(snapshot: &ParseSnapshot, current_text: &str) -> bool {
        if !snapshot.incremental || snapshot.fallback_reason.is_some() {
            return false;
        }
        if snapshot.changed_ranges.is_empty() {
            return true;
        }
        if snapshot.changed_ranges.len() != 1 {
            return false;
        }
        let range = &snapshot.changed_ranges[0];
        if range.start_byte != range.old_end_byte {
            return false;
        }
        let start = range.start_byte as usize;
        let new_end = range.new_end_byte as usize;
        if new_end != current_text.len() || start > new_end {
            return false;
        }
        let Some(inserted) = current_text.get(start..new_end) else {
            return false;
        };
        !inserted.is_empty() && inserted.chars().all(char::is_whitespace)
    }

    fn try_reuse_ir_from_previous_version(
        &self,
        file_id: FileId,
        file_version: i32,
        snapshot: &ParseSnapshot,
        current_text: &str,
        deps_id: &DepsSnapshotId,
    ) -> Option<Arc<SemanticProgram>> {
        if file_version <= 0 || !Self::parse_snapshot_can_reuse_previous_ir(snapshot, current_text)
        {
            return None;
        }
        let previous_version = file_version.saturating_sub(1);
        let cache = self
            .derived_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.get_ir(file_id, previous_version, deps_id)
    }

    fn remember_ir_artifact(
        &self,
        file_id: FileId,
        file_version: i32,
        deps_id: DepsSnapshotId,
        program: Arc<SemanticProgram>,
    ) {
        self.derived_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .store_ir(file_id, file_version, deps_id, program);
    }

    fn make_type_index_artifact_key(
        &self,
        file_id: FileId,
        file_version: i32,
    ) -> TypeIndexArtifactKey {
        TypeIndexArtifactKey::new(
            file_id,
            file_version,
            self.deps.id(&self.db).clone(),
            self.settings.id(&self.db).clone(),
        )
    }

    pub fn precompute_type_index_for_file(
        &self,
        file_id: FileId,
        expected_version: Option<i32>,
        queue_wait_ms: u128,
    ) -> Cancellable<TypeIndexPrecomputeResult> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(TypeIndexPrecomputeResult::with_reason(
                TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeMissingFile,
            ));
        };

        let initial_version = file.version(&self.db);
        if expected_version.is_some_and(|version| version != initial_version) {
            return Ok(TypeIndexPrecomputeResult {
                reason_code: TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeSuperseded,
                file_version: Some(initial_version),
                stats: TypeIndexPrecomputeStats {
                    queue_wait_ms,
                    ..TypeIndexPrecomputeStats::default()
                },
            });
        }

        let parse_snapshot_meta = self
            .parse_snapshot_for_file(file_id, file)
            .map(|snapshot| TypeIndexParseSnapshotMeta::from_snapshot(Some(snapshot)))
            .unwrap_or_default();
        let key = self.make_type_index_artifact_key(file_id, initial_version);
        let exec_started = Instant::now();
        let (type_index, build_profile) = if let Some(_snapshot) = self.parse_snapshot_for_file(file_id, file) {
            let deps_data = self.deps.data(&self.db).0.clone();
            let file_path = file.path(&self.db).clone();
            let Some(program) = self.ir(file_id)? else {
                return Ok(TypeIndexPrecomputeResult::with_reason(
                    TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeMissingFile,
                ));
            };
            let profiled = cancellable(|| {
                type_inference_v2::build_type_index_from_semantic_program_with_path_profiled(
                    program.as_ref(),
                    file_path.as_ref(),
                    deps_data,
                )
            })?;
            (Arc::new(profiled.index), profiled.profile)
        } else {
            let index_snapshot = cancellable(|| type_index(&self.db, file, self.deps, self.settings))?;
            (index_snapshot.index(), index_snapshot.build_profile())
        };
        let exec_ms = exec_started.elapsed().as_millis();
        let latest_version = file.version(&self.db);
        if expected_version.is_some_and(|version| version != latest_version) {
            return Ok(TypeIndexPrecomputeResult {
                reason_code: TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeSuperseded,
                file_version: Some(latest_version),
                stats: TypeIndexPrecomputeStats {
                    queue_wait_ms,
                    exec_ms,
                    build_ms: build_profile.total_ms,
                    ..TypeIndexPrecomputeStats::default()
                },
            });
        }

        let artifact = Arc::new(TypeIndexArtifact {
            type_index,
            build_profile,
            parse_snapshot_meta,
            produced_at_millis: unix_time_millis(),
        });
        let store_outcome = self
            .derived_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .store_type_index(key, artifact);

        Ok(TypeIndexPrecomputeResult {
            reason_code: TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeExactStored,
            file_version: Some(latest_version),
            stats: TypeIndexPrecomputeStats {
                queue_wait_ms,
                exec_ms,
                build_ms: build_profile.total_ms,
                evicted_per_file_window_total: store_outcome.evicted_per_file_window_total,
                evicted_global_guard_total: store_outcome.evicted_global_guard_total,
            },
        })
    }

    fn type_at_byte_offset_profiled_empty(
        reason_code: TypeIndexServeReasonCode,
    ) -> TypeAtByteOffsetProfiledResult {
        TypeAtByteOffsetProfiledResult {
            resolution: None,
            profile: TypeAtByteOffsetProfile::default(),
            serve_reason_code: reason_code,
        }
    }

    fn fallback_wrapped_tail_probe(source_text: &str, byte_offset: u32) -> Option<u32> {
        let bytes = source_text.as_bytes();
        if bytes.is_empty() {
            return None;
        }

        let mut idx = (byte_offset as usize).min(bytes.len().saturating_sub(1));
        let mut skipped = false;
        while let Some(byte) = bytes.get(idx).copied() {
            let is_wrapper_tail = matches!(byte, b')' | b'(' | b']' | b'[')
                || byte.is_ascii_whitespace();
            if !is_wrapper_tail {
                break;
            }
            skipped = true;
            if idx == 0 {
                return None;
            }
            idx = idx.saturating_sub(1);
        }

        skipped.then_some(idx.min(u32::MAX as usize) as u32)
    }

    fn resolve_type_index_at_offset(
        source_text: &str,
        index: &type_inference_v2::TypeIndex,
        byte_offset: u32,
    ) -> Option<TypeResolution> {
        index.type_at_byte_offset(byte_offset).or_else(|| {
            Self::fallback_wrapped_tail_probe(source_text, byte_offset)
                .and_then(|fallback| index.type_at_byte_offset(fallback))
        })
    }

    pub fn type_at_byte_offset_serve_only(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<TypeResolution>> {
        self.type_at_byte_offset_serve_only_profiled(file_id, byte_offset)
            .map(|profiled| profiled.resolution)
    }

    pub fn type_at_byte_offset_serve_only_profiled(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<TypeAtByteOffsetProfiledResult> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(Self::type_at_byte_offset_profiled_empty(
                TypeIndexServeReasonCode::TypeIndexFallbackUnavailable,
            ));
        };

        let file_version = file.version(&self.db);
        let key = self.make_type_index_artifact_key(file_id, file_version);
        let lookup_started = Instant::now();
        let (artifact, reason_code) = {
            let cache = self
                .derived_cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(artifact) = cache.get_type_index_exact(&key) {
                if artifact.parse_snapshot_meta.serve_only_blocked {
                    (
                        None,
                        TypeIndexServeReasonCode::TypeIndexFallbackUnavailable,
                    )
                } else {
                    (Some(artifact), TypeIndexServeReasonCode::TypeIndexExactHit)
                }
            } else if cache.get_type_index_stale(&key).is_some() {
                (None, TypeIndexServeReasonCode::TypeIndexFallbackUnavailable)
            } else {
                (None, TypeIndexServeReasonCode::TypeIndexFallbackUnavailable)
            }
        };

        let Some(artifact) = artifact else {
            return Ok(Self::type_at_byte_offset_profiled_empty(reason_code));
        };
        let scan_started = Instant::now();
        let source_text = file.text(&self.db);
        let resolution =
            Self::resolve_type_index_at_offset(source_text.as_ref(), artifact.type_index.as_ref(), byte_offset);
        let index_scan_ms = scan_started.elapsed().as_millis();
        let total_ms = lookup_started.elapsed().as_millis();
        Ok(TypeAtByteOffsetProfiledResult {
            resolution,
            profile: TypeAtByteOffsetProfile {
                index_fetch_ms: total_ms,
                index_build_total_ms: artifact.build_profile.total_ms,
                index_scan_ms,
                total_ms,
                ..TypeAtByteOffsetProfile::default()
            },
            serve_reason_code: reason_code,
        })
    }

    pub fn current_type_index_serve_only_ready(&self, file_id: FileId) -> Cancellable<bool> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(false);
        };

        let file_version = file.version(&self.db);
        let key = self.make_type_index_artifact_key(file_id, file_version);
        let cache = self
            .derived_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        Ok(cache
            .get_type_index_exact(&key)
            .is_some_and(|artifact| !artifact.parse_snapshot_meta.serve_only_blocked))
    }

    fn current_type_index_exact(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<Arc<type_inference_v2::TypeIndex>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };

        let file_version = file.version(&self.db);
        let key = self.make_type_index_artifact_key(file_id, file_version);
        let cache = self
            .derived_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        Ok(cache
            .get_type_index_exact(&key)
            .filter(|artifact| !artifact.parse_snapshot_meta.serve_only_blocked)
            .map(|artifact| artifact.type_index.clone()))
    }

    pub fn type_for_span_serve_only(
        &self,
        file_id: FileId,
        span: Span,
    ) -> Cancellable<Option<TypeResolution>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.type_resolution_for_span(span)))
    }

    pub fn definition_location_at_byte_offset_serve_only(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<TypeDefinitionLocation>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.definition_location_at_byte_offset(byte_offset)))
    }

    pub fn definition_location_for_span_serve_only(
        &self,
        file_id: FileId,
        span: Span,
    ) -> Cancellable<Option<TypeDefinitionLocation>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.definition_location_for_span(span)))
    }

    pub fn call_receiver_type_for_span_serve_only(
        &self,
        file_id: FileId,
        span: Span,
    ) -> Cancellable<Option<TypeResolution>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.call_receiver_type_for_span(span)))
    }

    pub fn call_receiver_type_at_byte_offset_serve_only(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<TypeResolution>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.call_receiver_type_at_byte_offset(byte_offset)))
    }

    pub fn member_access_object_type_for_span_serve_only(
        &self,
        file_id: FileId,
        span: Span,
    ) -> Cancellable<Option<TypeResolution>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.member_access_object_type_for_span(span)))
    }

    pub fn member_access_object_type_at_byte_offset_serve_only(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<TypeResolution>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.member_access_object_type_at_byte_offset(byte_offset)))
    }

    pub fn call_method_target_for_span_serve_only(
        &self,
        file_id: FileId,
        span: Span,
    ) -> Cancellable<Option<SemanticMethodTarget>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.call_method_target_for_span(span)))
    }

    pub fn call_method_target_at_byte_offset_serve_only(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<SemanticMethodTarget>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.call_method_target_at_byte_offset(byte_offset)))
    }

    pub fn member_method_target_for_span_serve_only(
        &self,
        file_id: FileId,
        span: Span,
    ) -> Cancellable<Option<SemanticMethodTarget>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.member_method_target_for_span(span)))
    }

    pub fn member_method_target_at_byte_offset_serve_only(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<SemanticMethodTarget>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.member_method_target_at_byte_offset(byte_offset)))
    }

    pub fn constructor_target_for_span_serve_only(
        &self,
        file_id: FileId,
        span: Span,
    ) -> Cancellable<Option<SemanticConstructorTarget>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.constructor_target_for_span(span)))
    }

    pub fn constructor_target_at_byte_offset_serve_only(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<SemanticConstructorTarget>> {
        Ok(self
            .current_type_index_exact(file_id)?
            .and_then(|index| index.constructor_target_at_byte_offset(byte_offset)))
    }

    pub fn file_text(&self, file_id: FileId) -> Cancellable<Option<Arc<str>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file.text(&self.db).clone()).map(Some)
    }

    pub fn file_version(&self, file_id: FileId) -> Cancellable<Option<i32>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file.version(&self.db)).map(Some)
    }

    pub fn file_path(&self, file_id: FileId) -> Cancellable<Option<Arc<str>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file.path(&self.db).clone()).map(Some)
    }

    pub fn file_text_len(&self, file_id: FileId) -> Cancellable<Option<usize>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| file_text_len(&self.db, file)).map(Some)
    }

    pub fn line_index(&self, file_id: FileId) -> Cancellable<Option<Arc<LineIndex>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            return Ok(Some(snapshot.line_index.clone()));
        }
        cancellable(|| line_index(&self.db, file)).map(Some)
    }

    pub fn parse_result(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<Arc<bsl_syntax::ast::ParseResult>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            return Ok(Some(snapshot.parse_result.clone()));
        }
        cancellable(|| parse_result(&self.db, file, self.settings).0).map(Some)
    }

    pub fn ir(&self, file_id: FileId) -> Cancellable<Option<Arc<SemanticProgram>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        let file_version = file.version(&self.db);
        let deps_id = self.deps.id(&self.db).clone();
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            let source = file.text(&self.db);
            if let Some(reused) = self.try_reuse_ir_from_previous_version(
                file_id,
                file_version,
                snapshot,
                source.as_ref(),
                &deps_id,
            ) {
                self.remember_ir_artifact(file_id, file_version, deps_id, reused.clone());
                return Ok(Some(reused));
            }
            let deps_data = self.deps.data(&self.db).0.clone();
            let parsed = parse_result(&self.db, file, self.settings).0;
            let file_path = file.path(&self.db);
            let program = build_ir_from_parsed(
                parsed,
                source.as_ref(),
                file_path.as_ref(),
                deps_data,
            );
            self.remember_ir_artifact(file_id, file_version, deps_id, program.clone());
            return Ok(Some(program));
        }
        let program = cancellable(|| ir(&self.db, file, self.deps, self.settings).0)?;
        self.remember_ir_artifact(file_id, file_version, deps_id, program.clone());
        Ok(Some(program))
    }

    pub fn syntax_diagnostics(&self, file_id: FileId) -> Cancellable<Option<Arc<Vec<ParseError>>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            return Ok(Some(Arc::new(snapshot.parse_result.syntax_errors.clone())));
        }
        cancellable(|| syntax_diagnostics(&self.db, file, self.settings).0).map(Some)
    }

    pub fn syntax_diagnostics_observability_mode(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<&'static str>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        if let Some(snapshot) = self.parse_snapshot_for_file(file_id, file) {
            return Ok(Some(Self::parse_snapshot_observability_mode(snapshot)));
        }
        Ok(Some("other"))
    }

    pub fn semantic_diagnostics(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<Arc<Vec<TypeDiagnostic>>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| semantic_diagnostics(&self.db, file, self.deps, self.settings).0).map(Some)
    }

    pub fn semantic_diagnostics_flow_sensitive(
        &self,
        file_id: FileId,
    ) -> Cancellable<Option<Arc<Vec<TypeDiagnostic>>>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| {
            semantic_diagnostics_flow_sensitive(&self.db, file, self.deps, self.settings).0
        })
        .map(Some)
    }

    pub fn utf16_position_to_byte_offset(
        &self,
        file_id: FileId,
        line: u32,
        character: u32,
    ) -> Cancellable<Option<usize>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| {
            let text = file.text(&self.db);
            let index = line_index(&self.db, file);
            index.utf16_position_to_byte_offset(text, line, character)
        })
        .map(Some)
    }

    pub fn utf16_position_to_point(
        &self,
        file_id: FileId,
        line: u32,
        character: u32,
    ) -> Cancellable<Option<(usize, usize)>> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(None);
        };
        cancellable(|| {
            let text = file.text(&self.db);
            let index = line_index(&self.db, file);
            index.utf16_position_to_point(text, line, character)
        })
        .map(Some)
    }

    pub fn deps_id(&self) -> Cancellable<DepsSnapshotId> {
        cancellable(|| self.deps.id(&self.db).clone())
    }

    pub fn deps_data(&self) -> Cancellable<Arc<SemanticDeps>> {
        cancellable(|| self.deps.data(&self.db).0.clone())
    }

    pub fn settings_id(&self) -> Cancellable<SettingsId> {
        cancellable(|| self.settings.id(&self.db).clone())
    }

    pub fn completion(&self, _file_id: FileId, _line: u32, _character: u32) -> Cancellable<()> {
        Ok(())
    }

    pub fn hover(&self, _file_id: FileId, _line: u32, _character: u32) -> Cancellable<()> {
        Ok(())
    }

    pub fn signature_help(&self, _file_id: FileId, _line: u32, _character: u32) -> Cancellable<()> {
        Ok(())
    }

    pub fn type_at_byte_offset(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<TypeResolution>> {
        self.type_at_byte_offset_profiled(file_id, byte_offset)
            .map(|profiled| profiled.resolution)
    }

    pub fn type_at_byte_offset_profiled(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<TypeAtByteOffsetProfiledResult> {
        let Some(&file) = self.files.get(&file_id) else {
            return Ok(TypeAtByteOffsetProfiledResult {
                resolution: None,
                profile: TypeAtByteOffsetProfile::default(),
                serve_reason_code: TypeIndexServeReasonCode::TypeIndexFallbackUnavailable,
            });
        };
        let started = Instant::now();
        let index_fetch_started = Instant::now();
        let index_fetch_active_at_entry = ActiveTypeIndexFetchGuard::enter();
        let _index_fetch_active_guard = ActiveTypeIndexFetchGuard;
        let index_fetch_revision_start = current_revision_u64(&self.db);
        let index_fetch_global_did_set_cancellation_flag_before =
            ANALYSIS_V2_GLOBAL_DID_SET_CANCELLATION_FLAG_TOTAL.load(Ordering::Relaxed);
        begin_salsa_event_timeline();
        let event_counters_before = salsa_event_counters_snapshot();
        let index_snapshot_result =
            cancellable(|| type_index(&self.db, file, self.deps, self.settings));
        let event_timeline = finish_salsa_event_timeline();
        let index_snapshot = index_snapshot_result?;
        let index_fetch_revision_end = current_revision_u64(&self.db);
        let index_fetch_revision_delta =
            index_fetch_revision_end.saturating_sub(index_fetch_revision_start);
        let index_fetch_global_did_set_cancellation_flag_after =
            ANALYSIS_V2_GLOBAL_DID_SET_CANCELLATION_FLAG_TOTAL.load(Ordering::Relaxed);
        let event_counters_after = salsa_event_counters_snapshot();
        let index_fetch_ms = index_fetch_started.elapsed().as_millis();
        let index_fetch_will_block_on_total = event_counters_after
            .will_block_on_total
            .saturating_sub(event_counters_before.will_block_on_total);
        let index_fetch_will_block_on_type_index_total = event_counters_after
            .will_block_on_type_index
            .saturating_sub(event_counters_before.will_block_on_type_index);
        let index_fetch_will_block_on_parse_result_total = event_counters_after
            .will_block_on_parse_result
            .saturating_sub(event_counters_before.will_block_on_parse_result);
        let index_fetch_will_block_on_other_total = compute_index_fetch_key_kind_other_total(
            index_fetch_will_block_on_total,
            index_fetch_will_block_on_type_index_total,
            index_fetch_will_block_on_parse_result_total,
        );
        let index_fetch_will_execute_total = event_counters_after
            .will_execute_total
            .saturating_sub(event_counters_before.will_execute_total);
        let index_fetch_will_execute_type_index_total = event_counters_after
            .will_execute_type_index
            .saturating_sub(event_counters_before.will_execute_type_index);
        let index_fetch_will_execute_parse_result_total = event_counters_after
            .will_execute_parse_result
            .saturating_sub(event_counters_before.will_execute_parse_result);
        let index_fetch_will_execute_other_total = compute_index_fetch_key_kind_other_total(
            index_fetch_will_execute_total,
            index_fetch_will_execute_type_index_total,
            index_fetch_will_execute_parse_result_total,
        );
        let index_fetch_will_iterate_cycle_total = event_counters_after
            .will_iterate_cycle_total
            .saturating_sub(event_counters_before.will_iterate_cycle_total);
        let index_fetch_did_validate_memoized_total = event_counters_after
            .did_validate_memoized_total
            .saturating_sub(event_counters_before.did_validate_memoized_total);
        let index_fetch_did_validate_memoized_type_index_total = event_counters_after
            .did_validate_memoized_type_index
            .saturating_sub(event_counters_before.did_validate_memoized_type_index);
        let index_fetch_did_validate_memoized_parse_result_total = event_counters_after
            .did_validate_memoized_parse_result
            .saturating_sub(event_counters_before.did_validate_memoized_parse_result);
        let index_fetch_did_validate_memoized_other_total =
            compute_index_fetch_key_kind_other_total(
                index_fetch_did_validate_memoized_total,
                index_fetch_did_validate_memoized_type_index_total,
                index_fetch_did_validate_memoized_parse_result_total,
            );
        let index_fetch_will_check_cancellation_total = event_counters_after
            .will_check_cancellation_total
            .saturating_sub(event_counters_before.will_check_cancellation_total);
        let index_fetch_did_set_cancellation_flag_total = event_counters_after
            .did_set_cancellation_flag_total
            .saturating_sub(event_counters_before.did_set_cancellation_flag_total);
        let index_fetch_global_did_set_cancellation_flag_total =
            index_fetch_global_did_set_cancellation_flag_after
                .saturating_sub(index_fetch_global_did_set_cancellation_flag_before);
        let index_fetch_did_discard_total = event_counters_after
            .did_discard_total
            .saturating_sub(event_counters_before.did_discard_total);
        let index_fetch_did_discard_accumulated_total = event_counters_after
            .did_discard_accumulated_total
            .saturating_sub(event_counters_before.did_discard_accumulated_total);
        let index = index_snapshot.index();
        let index_build_profile = index_snapshot.build_profile();
        let index_query_profile = index_snapshot.query_profile();
        let clip_to_index_fetch = |value_ms: u128| value_ms.min(index_fetch_ms);
        let index_query_total_ms = clip_to_index_fetch(index_query_profile.total_ms);
        let index_query_inputs_ms = clip_to_index_fetch(index_query_profile.inputs_ms);
        let index_query_parse_result_query_ms =
            clip_to_index_fetch(index_query_profile.parse_result_query_ms);
        let index_query_build_ms = clip_to_index_fetch(index_query_profile.build_ms);
        let index_fetch_unattributed_ms = index_fetch_ms.saturating_sub(index_query_total_ms);
        let (index_fetch_pre_first_salsa_event_wait_ms, index_fetch_post_last_salsa_event_tail_ms) =
            compute_index_fetch_salsa_event_edges_ms(
                index_fetch_ms,
                event_timeline.first_event_elapsed_ms,
                event_timeline.last_event_elapsed_ms,
            );
        let index_fetch_inside_salsa_window_ms = compute_index_fetch_inside_salsa_window_ms(
            index_fetch_ms,
            index_fetch_pre_first_salsa_event_wait_ms,
            index_fetch_post_last_salsa_event_tail_ms,
        );
        let clip_event_elapsed_ms = |value: Option<u128>| value.unwrap_or(0).min(index_fetch_ms);
        let index_fetch_first_will_execute_type_index_ms =
            clip_event_elapsed_ms(event_timeline.first_will_execute_type_index_elapsed_ms);
        let index_fetch_last_will_execute_type_index_ms =
            clip_event_elapsed_ms(event_timeline.last_will_execute_type_index_elapsed_ms);
        let index_fetch_first_will_execute_parse_result_ms =
            clip_event_elapsed_ms(event_timeline.first_will_execute_parse_result_elapsed_ms);
        let index_fetch_last_will_execute_parse_result_ms =
            clip_event_elapsed_ms(event_timeline.last_will_execute_parse_result_elapsed_ms);
        let index_fetch_first_will_execute_other_ms =
            clip_event_elapsed_ms(event_timeline.first_will_execute_other_elapsed_ms);
        let index_fetch_last_will_execute_other_ms =
            clip_event_elapsed_ms(event_timeline.last_will_execute_other_elapsed_ms);
        let index_fetch_first_will_iterate_cycle_ms =
            clip_event_elapsed_ms(event_timeline.first_will_iterate_cycle_elapsed_ms);
        let index_fetch_last_will_iterate_cycle_ms =
            clip_event_elapsed_ms(event_timeline.last_will_iterate_cycle_elapsed_ms);
        let index_fetch_first_will_check_cancellation_ms =
            clip_event_elapsed_ms(event_timeline.first_will_check_cancellation_elapsed_ms);
        let index_fetch_last_will_check_cancellation_ms =
            clip_event_elapsed_ms(event_timeline.last_will_check_cancellation_elapsed_ms);
        let index_fetch_first_will_check_to_first_will_execute_type_index_ms =
            compute_index_fetch_event_delta_ms(
                index_fetch_ms,
                event_timeline.first_will_check_cancellation_elapsed_ms,
                event_timeline.first_will_execute_type_index_elapsed_ms,
            );
        let first_type_index_timeline =
            compute_first_type_index_timeline_snapshot(&event_timeline.events, index_fetch_ms);
        let index_fetch_last_will_check_to_first_will_execute_type_index_ms =
            compute_index_fetch_event_delta_ms(
                index_fetch_ms,
                first_type_index_timeline
                    .last_will_check_before_first_will_execute_type_index_elapsed_ms,
                first_type_index_timeline.first_will_execute_type_index_elapsed_ms,
            );
        let index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms =
            compute_index_fetch_event_delta_ms(
                index_fetch_ms,
                first_type_index_timeline
                    .last_will_execute_parse_result_before_first_will_execute_type_index_elapsed_ms,
                first_type_index_timeline.first_will_execute_type_index_elapsed_ms,
            );
        let index_fetch_idle_before_first_will_execute_type_index_ms =
            compute_index_fetch_event_delta_ms(
                index_fetch_ms,
                first_type_index_timeline
                    .last_event_before_first_will_execute_type_index_elapsed_ms,
                first_type_index_timeline.first_will_execute_type_index_elapsed_ms,
            );
        let index_fetch_events_before_first_will_execute_type_index_total =
            first_type_index_timeline.events_before_first_will_execute_type_index_total;
        let index_fetch_will_check_before_first_will_execute_type_index_total =
            first_type_index_timeline.will_check_before_first_will_execute_type_index_total;
        let index_fetch_will_execute_parse_result_before_first_will_execute_type_index_total =
            first_type_index_timeline
                .will_execute_parse_result_before_first_will_execute_type_index_total;
        let index_fetch_first_will_execute_type_index_seen_total =
            first_type_index_timeline.first_will_execute_type_index_seen_total;
        let index_fetch_timeline_capture_limit = event_timeline.event_capture_limit as u64;
        let index_fetch_timeline_total_events = event_timeline.event_total;
        let index_fetch_timeline_stored_events = event_timeline.events.len() as u64;
        let index_fetch_timeline_truncated = event_timeline.event_truncated;
        let index_fetch_timeline = format_salsa_event_timeline(&event_timeline);
        let index_parse_result_ms = clip_to_index_fetch(index_snapshot.parse_result_ms());
        let index_build_total_ms = clip_to_index_fetch(index_build_profile.total_ms);
        let index_fetch_wait_ms = compute_index_fetch_wait_ms(
            index_fetch_ms,
            index_parse_result_ms,
            index_build_total_ms,
        );
        let index_build_seed_module_context_ms =
            clip_to_index_fetch(index_build_profile.seed_module_context_ms);
        let index_build_local_function_summaries_ms =
            clip_to_index_fetch(index_build_profile.local_function_summaries_ms);
        let index_build_visit_statements_ms =
            clip_to_index_fetch(index_build_profile.visit_statements_ms);

        let index_scan_started = Instant::now();
        let source_text = file.text(&self.db);
        let resolution = Self::resolve_type_index_at_offset(source_text.as_ref(), index.as_ref(), byte_offset);
        let index_scan_ms = index_scan_started.elapsed().as_millis();
        let total_ms = started.elapsed().as_millis();

        if slow_index_fetch_log_enabled() {
            if let Some(threshold_ms) = slow_index_fetch_log_threshold_ms() {
                if index_fetch_ms >= threshold_ms {
                    tracing::warn!(
                        target: "bsl_backend::analysis_v2",
                        file_id = file_id.0,
                        byte_offset,
                        file_path = %file.path(&self.db),
                        index_fetch_ms,
                        index_fetch_wait_ms,
                        index_fetch_unattributed_ms,
                        index_fetch_pre_first_salsa_event_wait_ms,
                        index_fetch_post_last_salsa_event_tail_ms,
                        index_fetch_inside_salsa_window_ms,
                        index_fetch_first_will_execute_type_index_ms,
                        index_fetch_last_will_execute_type_index_ms,
                        index_fetch_first_will_execute_parse_result_ms,
                        index_fetch_last_will_execute_parse_result_ms,
                        index_fetch_first_will_execute_other_ms,
                        index_fetch_last_will_execute_other_ms,
                        index_fetch_first_will_iterate_cycle_ms,
                        index_fetch_last_will_iterate_cycle_ms,
                        index_fetch_first_will_check_cancellation_ms,
                        index_fetch_last_will_check_cancellation_ms,
                        index_fetch_first_will_check_to_first_will_execute_type_index_ms,
                        index_fetch_last_will_check_to_first_will_execute_type_index_ms,
                        index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms,
                        index_fetch_idle_before_first_will_execute_type_index_ms,
                        index_fetch_active_at_entry,
                        index_fetch_events_before_first_will_execute_type_index_total,
                        index_fetch_will_check_before_first_will_execute_type_index_total,
                        index_fetch_will_execute_parse_result_before_first_will_execute_type_index_total,
                        index_fetch_first_will_execute_type_index_seen_total,
                        index_query_total_ms,
                        index_query_inputs_ms,
                        index_query_parse_result_query_ms,
                        index_query_build_ms,
                        index_build_total_ms,
                        index_parse_result_ms,
                        index_fetch_will_execute_total,
                        index_fetch_will_execute_type_index_total,
                        index_fetch_will_execute_parse_result_total,
                        index_fetch_will_iterate_cycle_total,
                        index_fetch_did_validate_memoized_total,
                        index_fetch_will_block_on_total,
                        index_fetch_will_check_cancellation_total,
                        index_fetch_did_set_cancellation_flag_total,
                        index_fetch_global_did_set_cancellation_flag_total,
                        index_fetch_did_discard_total,
                        index_fetch_did_discard_accumulated_total,
                        index_fetch_revision_start,
                        index_fetch_revision_end,
                        index_fetch_revision_delta,
                        index_fetch_timeline_capture_limit,
                        index_fetch_timeline_total_events,
                        index_fetch_timeline_stored_events,
                        index_fetch_timeline_truncated,
                        index_fetch_timeline = %index_fetch_timeline,
                        "analysis_v2: slow type_index fetch in type_at_byte_offset_profiled"
                    );
                }
            }
        }

        Ok(TypeAtByteOffsetProfiledResult {
            resolution,
            profile: TypeAtByteOffsetProfile {
                index_fetch_ms,
                index_fetch_wait_ms,
                index_fetch_unattributed_ms,
                index_fetch_pre_first_salsa_event_wait_ms,
                index_fetch_post_last_salsa_event_tail_ms,
                index_fetch_inside_salsa_window_ms,
                index_fetch_first_will_execute_type_index_ms,
                index_fetch_last_will_execute_type_index_ms,
                index_fetch_first_will_execute_parse_result_ms,
                index_fetch_last_will_execute_parse_result_ms,
                index_fetch_first_will_execute_other_ms,
                index_fetch_last_will_execute_other_ms,
                index_fetch_first_will_iterate_cycle_ms,
                index_fetch_last_will_iterate_cycle_ms,
                index_fetch_first_will_check_cancellation_ms,
                index_fetch_last_will_check_cancellation_ms,
                index_fetch_first_will_check_to_first_will_execute_type_index_ms,
                index_fetch_last_will_check_to_first_will_execute_type_index_ms,
                index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms,
                index_fetch_idle_before_first_will_execute_type_index_ms,
                index_fetch_active_at_entry,
                index_fetch_events_before_first_will_execute_type_index_total,
                index_fetch_will_check_before_first_will_execute_type_index_total,
                index_fetch_will_execute_parse_result_before_first_will_execute_type_index_total,
                index_fetch_first_will_execute_type_index_seen_total,
                index_fetch_will_block_on_total,
                index_fetch_will_block_on_type_index_total,
                index_fetch_will_block_on_parse_result_total,
                index_fetch_will_block_on_other_total,
                index_fetch_will_execute_total,
                index_fetch_will_execute_type_index_total,
                index_fetch_will_execute_parse_result_total,
                index_fetch_will_execute_other_total,
                index_fetch_will_iterate_cycle_total,
                index_fetch_did_validate_memoized_total,
                index_fetch_did_validate_memoized_type_index_total,
                index_fetch_did_validate_memoized_parse_result_total,
                index_fetch_did_validate_memoized_other_total,
                index_fetch_will_check_cancellation_total,
                index_fetch_did_set_cancellation_flag_total,
                index_fetch_global_did_set_cancellation_flag_total,
                index_fetch_did_discard_total,
                index_fetch_did_discard_accumulated_total,
                index_fetch_revision_start,
                index_fetch_revision_end,
                index_fetch_revision_delta,
                index_query_total_ms,
                index_query_inputs_ms,
                index_query_parse_result_query_ms,
                index_query_build_ms,
                index_parse_result_ms,
                index_build_total_ms,
                index_build_seed_module_context_ms,
                index_build_local_function_summaries_ms,
                index_build_visit_statements_ms,
                index_scan_ms,
                total_ms,
            },
            serve_reason_code: TypeIndexServeReasonCode::TypeIndexExactHit,
        })
    }

    pub fn flow_type_at_byte_offset(
        &self,
        file_id: FileId,
        byte_offset: u32,
    ) -> Cancellable<Option<TypeResolution>> {
        let Some(base) = self.type_at_byte_offset_serve_only(file_id, byte_offset)? else {
            return Ok(None);
        };
        let Some(program) = self.ir(file_id)? else {
            return Ok(None);
        };
        cancellable(|| flow_type_at_byte_offset_impl(program.as_ref(), byte_offset, base))
    }
}
