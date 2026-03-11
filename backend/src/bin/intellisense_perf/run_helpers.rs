use super::*;

use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter};
use bsl_runtime::application::type_system::{
    definition_exact_type_index_available_at_position, get_hover_info_with_semantic_program,
    goto_definition_v2_with_source_and_analysis, hover_exact_type_index_available_at_position,
};

pub(super) fn read_scenario(path: &Path) -> Result<Scenario> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read scenario file: {}", path.to_string_lossy()))?;
    let scenario: Scenario = serde_json::from_str(&data).context("Invalid scenario JSON")?;
    if scenario.cases.is_empty() {
        bail!("Scenario must contain at least one case");
    }
    Ok(scenario)
}

pub(super) fn resolve_override(
    override_path: Option<&PathBuf>,
    scenario_path: Option<&PathBuf>,
    base_dir: &Path,
) -> Option<PathBuf> {
    let path = override_path.or(scenario_path)?;
    Some(resolve_relative(base_dir, path))
}

pub(super) fn resolve_relative(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

pub(super) fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or(manifest_dir)
}

pub(super) fn prepare_cases(cases: &[ScenarioCase], base_dir: &Path) -> Result<Vec<PreparedCase>> {
    let mut cache: HashMap<PathBuf, String> = HashMap::new();
    let mut prepared = Vec::with_capacity(cases.len());

    for case in cases {
        let file = resolve_relative(base_dir, &case.file);
        let content = if let Some(existing) = cache.get(&file) {
            existing.clone()
        } else {
            let text = fs::read_to_string(&file)
                .with_context(|| format!("Failed to read case file: {}", file.to_string_lossy()))?;
            cache.insert(file.clone(), text.clone());
            text
        };

        let (line, column) = find_position(&content, &case.marker).with_context(|| {
            format!(
                "Marker '{}' not found in {}",
                case.marker,
                file.to_string_lossy()
            )
        })?;

        let file_uri = file.to_string_lossy().into_owned();
        let file_id = V2FileId((prepared.len() + 1) as u32);
        prepared.push(PreparedCase {
            file_id,
            file_uri,
            content: Arc::from(content),
            line,
            column,
            operation: case.operation,
            fixture_family: case.fixture_family,
        });
    }

    Ok(prepared)
}

pub(super) async fn prime_runtime_files(
    facade: &bsl_backend::application::IntellisenseV2Facade,
    cases: &[PreparedCase],
) {
    let changes = cases
        .iter()
        .map(|case| ChangeV2::SetFile {
            file_id: case.file_id,
            text: case.content.clone(),
            version: 0,
            path: Arc::from(case.file_uri.clone()),
        })
        .collect::<Vec<_>>();
    facade.apply_changes(changes);
    let _ = facade.snapshot().await;
}

pub(super) fn build_case_group_counts(cases: &[PreparedCase]) -> HashMap<ResultGroupKey, usize> {
    let mut counts = HashMap::new();
    for case in cases {
        *counts
            .entry(ResultGroupKey {
                fixture_family: case.fixture_family,
                operation: case.operation,
            })
            .or_insert(0) += 1;
    }
    counts
}

fn find_position(content: &str, marker: &str) -> Option<(u32, u32)> {
    let byte_index = content.find(marker)?;
    let before = &content[..byte_index + marker.len()];
    let line = before.lines().count().saturating_sub(1) as u32;
    let last_line = before.lines().last().unwrap_or("");
    let character = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    Some((line, character))
}

pub(super) struct IterationContext<'a> {
    pub(super) facade: &'a bsl_backend::application::IntellisenseV2Facade,
    pub(super) deps_id: &'a bsl_analysis_v2::DepsSnapshotId,
    pub(super) settings: bsl_backend::application::ExecutionSettings,
    pub(super) coordinator: &'a SystemCoordinator,
    pub(super) metadata_lookup: &'a TypeMetadataLookup,
    pub(super) resolver: &'a TypeResolver,
}

#[derive(Debug)]
pub(super) struct CaseExecutionMeasurement {
    pub(super) total_duration_ms: f64,
    pub(super) wait_for_file_version_ms: f64,
    pub(super) snapshot_preparation_ms: f64,
    pub(super) ir_query_ms: f64,
    pub(super) incomplete: bool,
    pub(super) fail_closed: bool,
    pub(super) allocation_count: u64,
    pub(super) allocated_bytes: u64,
    pub(super) lock_wait_ms: f64,
    pub(super) lock_contention_events: u64,
    pub(super) anti_rescue: PerfAntiRescueCounts,
}

pub(super) async fn run_iterations(
    context: &IterationContext<'_>,
    cases: &[PreparedCase],
    iterations: usize,
    churn_state: &mut Option<ChurnRuntimeState>,
    content_by_file: &mut HashMap<String, Arc<str>>,
    version_by_file: &mut HashMap<String, i32>,
    mut output: Option<&mut HashMap<ResultGroupKey, MeasuredResults>>,
) -> Result<()> {
    for iteration in 0..iterations {
        for (case_index, case) in cases.iter().enumerate() {
            maybe_apply_churn(
                context.facade,
                churn_state,
                content_by_file,
                version_by_file,
                iteration,
                case_index,
            )?;
            let measurement =
                execute_case_iteration(context, case, content_by_file, version_by_file).await;
            if let Some(groups) = output.as_mut() {
                let entry = groups
                    .entry(ResultGroupKey {
                        fixture_family: case.fixture_family,
                        operation: case.operation,
                    })
                    .or_default();
                let Ok(measurement) = measurement else {
                    entry.errors += 1;
                    continue;
                };
                entry.total_duration_ms.push(measurement.total_duration_ms);
                entry
                    .wait_for_file_version_ms
                    .push(measurement.wait_for_file_version_ms);
                entry
                    .snapshot_preparation_ms
                    .push(measurement.snapshot_preparation_ms);
                entry.ir_query_ms.push(measurement.ir_query_ms);
                if measurement.incomplete {
                    entry.incomplete += 1;
                }
                if measurement.fail_closed {
                    entry.fail_closed += 1;
                }
                entry.allocation_count_total += measurement.allocation_count;
                entry.allocated_bytes_total += measurement.allocated_bytes;
                entry.lock_wait_ms_total += measurement.lock_wait_ms;
                entry.lock_contention_events_total += measurement.lock_contention_events;
                entry.anti_rescue.stale_fallback_total +=
                    measurement.anti_rescue.stale_fallback_total;
                entry.anti_rescue.stale_served_total +=
                    measurement.anti_rescue.stale_served_total;
                entry.anti_rescue.degraded_substitute_total +=
                    measurement.anti_rescue.degraded_substitute_total;
                entry.anti_rescue.search_backed_substitute_total +=
                    measurement.anti_rescue.search_backed_substitute_total;
            }
        }
    }
    Ok(())
}

pub(super) async fn execute_case_iteration(
    context: &IterationContext<'_>,
    case: &PreparedCase,
    content_by_file: &mut HashMap<String, Arc<str>>,
    version_by_file: &HashMap<String, i32>,
) -> Result<CaseExecutionMeasurement> {
    let started = Instant::now();
    let alloc_before = allocation_snapshot();
    let expected_version = version_by_file
        .get(case.file_uri.as_str())
        .copied()
        .unwrap_or(0);
    let execution = bsl_backend::application::ExecutionContext {
        origin: bsl_backend::application::ObservabilityOrigin::Runtime,
        operation: case.operation.semantic_operation(),
        completion_mode: matches!(case.operation, PerfOperation::Completion | PerfOperation::Members)
            .then_some("perf_harness"),
        completion_large_churn_active: false,
        file_id: case.file_id,
        min_file_version: Some(expected_version),
        expected_deps_id: Some(context.deps_id.clone()),
        flow_sensitive: false,
        settings: context.settings.clone(),
        cancellation: bsl_backend::application::CancellationPolicy::Ignore,
    };
    let prepared = context
        .facade
        .prepare_stateful_operation(&execution, Some(context.coordinator))
        .await
        .map_err(|outcome| anyhow::anyhow!("prepare_stateful_operation failed: {}", outcome.as_str()))?;
    let analysis = prepared.snapshot.analysis;
    let case_content = analysis
        .file_text(case.file_id)
        .ok()
        .flatten()
        .or_else(|| content_by_file.get(case.file_uri.as_str()).cloned())
        .unwrap_or_else(|| case.content.clone());
    let file_path = analysis
        .file_path(case.file_id)
        .ok()
        .flatten()
        .unwrap_or_else(|| Arc::from(case.file_uri.clone()));
    let deps = analysis.deps_data().ok().context("deps unavailable")?;

    let member_access_owner_type_hint =
        if matches!(case.operation, PerfOperation::Completion | PerfOperation::Members)
            && completion_request_targets_member_access(case_content.as_ref(), case.line, case.column)
        {
            let _ =
                analysis.precompute_type_index_for_file(case.file_id, Some(expected_version), 0);
            completion_owner_hint_at_position(
                &analysis,
                case.file_id,
                case_content.as_ref(),
                case.line,
                case.column,
            )
        } else {
            None
        };

    let ir_started = Instant::now();
    let ir_program = bsl_backend::application::IntellisenseV2Facade::run_ir_query_singleflight(
        &execution,
        &analysis,
        Some(context.coordinator),
        case.file_id,
    )
    .map_err(|_| anyhow::anyhow!("ir query cancelled"))?
    .context("ir unavailable")?;
    let ir_query_ms = ir_started.elapsed().as_secs_f64() * 1000.0;

    let outcome = match case.operation {
        PerfOperation::Completion | PerfOperation::Members => {
            let response =
                bsl_backend::application::get_completion_with_semantic_program_snapshot_v2_with_trigger_hint(
                    case_content.as_ref(),
                    case.line,
                    case.column,
                    Some(case.file_uri.as_str()),
                    prepared.index_snapshot.as_ref(),
                    context.metadata_lookup,
                    file_path.as_ref(),
                    context.resolver,
                    ir_program,
                    member_access_owner_type_hint,
                    false,
                    None,
                )
                .await?;
            (response.is_incomplete, false)
        }
        PerfOperation::Hover => {
            let hover_formatter =
                HoverFormatter::new(HoverFormatConfig::default(), context.metadata_lookup.clone());
            let hover = get_hover_info_with_semantic_program(
                &analysis,
                case.file_id,
                case_content.as_ref(),
                case.line,
                case.column,
                false,
                context.metadata_lookup,
                &hover_formatter,
                None,
                context.resolver,
                ir_program.clone(),
            );
            let fail_closed = hover.is_none()
                && !hover_exact_type_index_available_at_position(
                    &analysis,
                    case.file_id,
                    case_content.as_ref(),
                    case.line,
                    case.column,
                    ir_program.as_ref(),
                );
            (false, fail_closed)
        }
        PerfOperation::Definition => {
            let definition = goto_definition_v2_with_source_and_analysis(
                file_path.as_ref(),
                case_content.as_ref(),
                &analysis,
                case.file_id,
                ir_program,
                deps,
                case.line,
                case.column,
                Some(context.coordinator),
            );
            let fail_closed = definition.is_none()
                && !definition_exact_type_index_available_at_position(
                    &analysis,
                    case.file_id,
                    case.line,
                    case.column,
                );
            (false, fail_closed)
        }
        PerfOperation::TypeAtPosition => {
            let type_resolution = type_at_position(
                &analysis,
                case.file_id,
                case.line,
                case.column,
            );
            (false, type_resolution.is_none())
        }
    };

    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let alloc_after = allocation_snapshot();
    let alloc_delta = alloc_after.count.saturating_sub(alloc_before.count);
    let bytes_delta = alloc_after.bytes.saturating_sub(alloc_before.bytes);

    Ok(CaseExecutionMeasurement {
        total_duration_ms: elapsed_ms,
        wait_for_file_version_ms: prepared
            .wait_elapsed
            .map(|elapsed| elapsed.as_secs_f64() * 1000.0)
            .unwrap_or(0.0),
        snapshot_preparation_ms: prepared.snapshot_elapsed.as_secs_f64() * 1000.0,
        ir_query_ms,
        incomplete: outcome.0,
        fail_closed: outcome.1,
        allocation_count: alloc_delta,
        allocated_bytes: bytes_delta,
        lock_wait_ms: ir_query_ms,
        lock_contention_events: u64::from(ir_query_ms > 0.0),
        anti_rescue: PerfAntiRescueCounts {
            stale_served_total: u64::from(prepared.stale_served),
            ..PerfAntiRescueCounts::default()
        },
    })
}

pub(super) fn build_content_by_file_map(cases: &[PreparedCase]) -> HashMap<String, Arc<str>> {
    let mut map = HashMap::new();
    for case in cases {
        map.entry(case.file_uri.clone())
            .or_insert_with(|| case.content.clone());
    }
    map
}

pub(super) fn build_file_version_map(cases: &[PreparedCase]) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    for case in cases {
        map.entry(case.file_uri.clone()).or_insert(0);
    }
    map
}

pub(super) fn build_churn_state(
    scenario: &Scenario,
    cases: &[PreparedCase],
) -> Result<Option<ChurnRuntimeState>> {
    let Some(churn) = scenario.churn else {
        return Ok(None);
    };
    if churn.every == 0 {
        bail!("Scenario churn.every must be greater than 0");
    }
    if cases.is_empty() {
        bail!("Scenario must contain at least one case");
    }

    let target_case = churn.target_case.unwrap_or(0);
    let target_case_ref = cases
        .get(target_case)
        .with_context(|| format!("Scenario churn.target_case out of range: {}", target_case))?;
    let target_file_uri = target_case_ref.file_uri.clone();
    let target_file_path: Arc<str> = Arc::from(target_file_uri.clone());
    let target_file_ids = cases
        .iter()
        .filter(|case| case.file_uri == target_file_uri)
        .map(|case| case.file_id)
        .collect::<Vec<_>>();

    if target_file_ids.is_empty() {
        bail!(
            "Scenario churn target file is not present in prepared cases: {}",
            target_file_uri
        );
    }

    let plan = ChurnPlan {
        every: churn.every,
        trigger_case_index: target_case,
        target_file_uri,
        target_file_path,
        target_file_ids,
        base_content: target_case_ref.content.clone(),
    };
    Ok(Some(ChurnRuntimeState::new(plan)))
}

fn maybe_apply_churn(
    facade: &bsl_backend::application::IntellisenseV2Facade,
    churn_state: &mut Option<ChurnRuntimeState>,
    content_by_file: &mut HashMap<String, Arc<str>>,
    version_by_file: &mut HashMap<String, i32>,
    iteration: usize,
    case_index: usize,
) -> Result<()> {
    let Some(state) = churn_state.as_mut() else {
        return Ok(());
    };
    if !state.should_apply(iteration, case_index) {
        return Ok(());
    }

    state.revision = state.revision.saturating_add(1);
    let churned_content = build_churned_content(state.plan.base_content.as_ref(), state.revision);
    let churned_content_arc: Arc<str> = Arc::from(churned_content);

    let mut changes = Vec::with_capacity(state.plan.target_file_ids.len());
    for file_id in &state.plan.target_file_ids {
        changes.push(ChangeV2::SetFile {
            file_id: *file_id,
            text: churned_content_arc.clone(),
            version: state.next_version,
            path: state.plan.target_file_path.clone(),
        });
    }
    facade.apply_changes(changes);
    content_by_file.insert(state.plan.target_file_uri.clone(), churned_content_arc);
    version_by_file.insert(state.plan.target_file_uri.clone(), state.next_version);
    state.next_version = state.next_version.saturating_add(1);

    Ok(())
}

pub(super) fn build_churned_content(base_content: &str, revision: u64) -> String {
    let mut content = String::with_capacity(base_content.len() + 64);
    content.push_str(base_content);
    if !content.ends_with('\n') {
        content.push('\n');
    }
    let marker = if revision.is_multiple_of(2) { "B" } else { "A" };
    content.push_str("// __intellisense_perf_churn_marker__ ");
    content.push_str(marker);
    content.push('\n');
    content
}

fn completion_request_targets_member_access(text: &str, line: u32, column: u32) -> bool {
    let Some(line_text) = text.lines().nth(line as usize) else {
        return false;
    };
    let column_index =
        bsl_analysis_v2::utf16_to_byte_offset(line_text, column).min(line_text.len());
    let line_prefix = line_text.get(..column_index).unwrap_or(line_text);
    let line_prefix = if line_text
        .get(column_index..)
        .and_then(|tail| tail.chars().next())
        == Some('.')
    {
        format!("{line_prefix}.")
    } else {
        line_prefix.to_string()
    };

    let trimmed = line_prefix.trim_end();
    let Some(dot_pos) = trimmed.rfind('.') else {
        return false;
    };
    let after_dot = trimmed[dot_pos + 1..].trim_start();
    after_dot.is_empty() || after_dot.chars().all(is_completion_identifier_char)
}

fn is_completion_identifier_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn completion_owner_hint_at_position(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: &str,
    line: u32,
    column: u32,
) -> Option<bsl_shared::domain::types::TypeResolution> {
    let line_text = file_content.lines().nth(line as usize)?;
    let cursor_byte = bsl_analysis_v2::utf16_to_byte_offset(line_text, column).min(line_text.len());
    let line_prefix = line_text.get(..cursor_byte)?;
    let dot_idx = line_prefix.rfind('.')?;
    let receiver = line_prefix.get(..dot_idx)?.trim_end();
    if receiver.is_empty() {
        return None;
    }

    let probe_utf16 = bsl_analysis_v2::byte_offset_to_utf16(line_text, receiver.len());
    let probe_offset = analysis
        .utf16_position_to_byte_offset(file_id, line, probe_utf16)
        .ok()
        .flatten()?
        .saturating_sub(1)
        .min(u32::MAX as usize) as u32;
    let profiled = analysis
        .type_at_byte_offset_serve_only_profiled(file_id, probe_offset)
        .ok()?;

    profiled
        .resolution
        .filter(|hint| !hint.is_unknown() && !hint.is_dynamic())
}

fn type_at_position(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    line: u32,
    column: u32,
) -> Option<bsl_shared::domain::types::TypeResolution> {
    let offset = analysis
        .utf16_position_to_byte_offset(file_id, line, column)
        .ok()
        .flatten()? as u32;
    analysis.type_at_byte_offset_serve_only(file_id, offset).ok().flatten()
}
