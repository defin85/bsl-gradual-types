//! IntelliSense performance harness for completion latency regression checks.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::perf_gate_evaluator::{
    validate_cutover_evidence_authority, validate_perf_report_provenance, PerfGateSample,
    PerfGateThresholds,
};
use bsl_backend::system::{build_deps_bundle_v2, SystemCoordinator};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::DetailLevel;

#[path = "intellisense_perf/reporting.rs"]
mod reporting;
#[path = "intellisense_perf/run_helpers.rs"]
mod run_helpers;

use reporting::{
    build_coverage, build_provenance_failure_comparison, build_results, compare_reports,
    contract_version_from_contract, read_json_value, write_report, write_summary,
};
use run_helpers::*;

#[cfg(test)]
#[path = "intellisense_perf/tests.rs"]
mod tests;

struct CountingAllocator;

static ALLOCATION_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

fn evaluate_intellisense_perf_profile_for_harness(
    contract: &serde_json::Value,
    profile: &str,
    current: &[PerfGateSample],
    baseline: Option<&[PerfGateSample]>,
    thresholds: PerfGateThresholds,
) -> serde_json::Value {
    bsl_backend::perf_gate_evaluator::evaluate_intellisense_perf_profile(
        contract, profile, current, baseline, thresholds,
    )
}

// SAFETY: The wrapper delegates all allocation behavior to the standard system allocator
// while only updating lock-free atomics for measurement.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        }
        new_ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[derive(Debug, Clone, Copy)]
struct AllocationSnapshot {
    count: u64,
    bytes: u64,
}

fn allocation_snapshot() -> AllocationSnapshot {
    AllocationSnapshot {
        count: ALLOCATION_COUNT.load(Ordering::Relaxed),
        bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
    }
}

#[derive(Debug, Parser)]
#[command(name = "intellisense-perf")]
#[command(about = "IntelliSense completion performance harness")]
struct Args {
    /// Scenario JSON path.
    #[arg(long)]
    scenario: PathBuf,

    /// Number of warmup iterations per case.
    #[arg(long, default_value_t = 20)]
    warmup: usize,

    /// Number of measured iterations per case.
    #[arg(long, default_value_t = 200)]
    iterations: usize,

    /// Override scenario syntax helper path.
    #[arg(long)]
    syntax_helper_path: Option<PathBuf>,

    /// Override scenario configuration path (Configuration.xml).
    #[arg(long)]
    config_path: Option<PathBuf>,

    /// Override platform version (e.g., "8.3.25").
    #[arg(long)]
    platform_version: Option<String>,

    /// Baseline JSON path for regression checks.
    #[arg(long)]
    baseline: Option<PathBuf>,

    /// Update baseline file with current results.
    #[arg(long)]
    update_baseline: bool,

    /// Regression threshold for P95 (ratio vs baseline).
    #[arg(long, default_value_t = 1.10)]
    threshold_p95: f64,

    /// Regression threshold for P99 (ratio vs baseline).
    #[arg(long, default_value_t = 1.15)]
    threshold_p99: f64,

    /// Regression threshold for resource metrics (ratio vs baseline).
    #[arg(long, default_value_t = 1.15)]
    threshold_resource: f64,

    /// Maximum allowed error rate (0.0..1.0).
    #[arg(long, default_value_t = 0.0)]
    max_error_rate: f64,

    /// Maximum allowed incomplete rate (0.0..1.0).
    #[arg(long, default_value_t = 0.0)]
    max_incomplete_rate: f64,

    /// Output report JSON path.
    #[arg(long)]
    output: Option<PathBuf>,

    /// Output summary markdown path.
    #[arg(long)]
    summary: Option<PathBuf>,

    /// Versioned contract path for intellisense perf gate.
    #[arg(long)]
    contract_path: Option<PathBuf>,

    /// Enable fail-closed blocking mode for missing baseline budgets.
    #[arg(long, default_value_t = false)]
    blocking_mode: bool,

    /// Authoritative OpenSpec change-id for perf artifact provenance.
    #[arg(long)]
    change_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Scenario {
    name: String,
    #[serde(default)]
    syntax_helper_path: Option<PathBuf>,
    #[serde(default)]
    config_path: Option<PathBuf>,
    #[serde(default)]
    platform_version: Option<String>,
    #[serde(default)]
    churn: Option<ScenarioChurn>,
    cases: Vec<ScenarioCase>,
}

#[derive(Debug, Deserialize, Clone)]
struct ScenarioCase {
    file: PathBuf,
    marker: String,
    #[allow(dead_code)]
    #[serde(default)]
    label: Option<String>,
    operation: PerfOperation,
    fixture_family: PerfFixtureFamily,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum PerfOperation {
    Completion,
    Hover,
    Definition,
    TypeAtPosition,
    Members,
}

impl PerfOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completion => "completion",
            Self::Hover => "hover",
            Self::Definition => "definition",
            Self::TypeAtPosition => "type_at_position",
            Self::Members => "members",
        }
    }

    fn semantic_operation(self) -> bsl_backend::application::SemanticOperation {
        match self {
            Self::Completion => bsl_backend::application::SemanticOperation::Completion,
            Self::Hover => bsl_backend::application::SemanticOperation::Hover,
            Self::Definition => bsl_backend::application::SemanticOperation::Definition,
            Self::TypeAtPosition => bsl_backend::application::SemanticOperation::TypeAtPosition,
            Self::Members => bsl_backend::application::SemanticOperation::Members,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum PerfFixtureFamily {
    SteadyMemberChain,
    PostDidChangeCurrentRevision,
    ObjectModuleExplicitContext,
    RecordsetModuleExplicitContext,
    IncompleteSyntaxMemberAccess,
}

impl PerfFixtureFamily {
    fn as_str(self) -> &'static str {
        match self {
            Self::SteadyMemberChain => "steady_member_chain",
            Self::PostDidChangeCurrentRevision => "post_did_change_current_revision",
            Self::ObjectModuleExplicitContext => "object_module_explicit_context",
            Self::RecordsetModuleExplicitContext => "recordset_module_explicit_context",
            Self::IncompleteSyntaxMemberAccess => "incomplete_syntax_member_access",
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
struct ScenarioChurn {
    #[serde(default = "default_churn_every")]
    every: usize,
    #[serde(default)]
    target_case: Option<usize>,
}

fn default_churn_every() -> usize {
    1
}

#[derive(Debug, Clone)]
struct PreparedCase {
    file_id: V2FileId,
    file_uri: String,
    content: Arc<str>,
    line: u32,
    column: u32,
    operation: PerfOperation,
    fixture_family: PerfFixtureFamily,
}

#[derive(Debug, Clone)]
struct ChurnPlan {
    every: usize,
    trigger_case_index: usize,
    target_file_uri: String,
    target_file_path: Arc<str>,
    target_file_ids: Vec<V2FileId>,
    base_content: Arc<str>,
}

#[derive(Debug)]
struct ChurnRuntimeState {
    plan: ChurnPlan,
    next_version: i32,
    revision: u64,
}

impl ChurnRuntimeState {
    fn new(plan: ChurnPlan) -> Self {
        Self {
            plan,
            next_version: 1,
            revision: 0,
        }
    }

    fn should_apply(&self, iteration: usize, case_index: usize) -> bool {
        iteration.is_multiple_of(self.plan.every) && case_index == self.plan.trigger_case_index
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfReport {
    scenario: String,
    profile: String,
    cases: usize,
    iterations: usize,
    warmup: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    change_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provenance: Option<PerfReportProvenance>,
    #[serde(default = "unknown_contract_version")]
    contract_version: String,
    coverage: PerfCoverage,
    results: Vec<PerfResultEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thresholds: Option<PerfThresholds>,
    verdict: String,
    reason_codes: Vec<String>,
    pass: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    comparison: Option<PerfComparison>,
}

fn unknown_contract_version() -> String {
    "unknown".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct PerfReportProvenance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    change_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    generated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    schema_version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    contract_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfMetrics {
    count: usize,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfCoverage {
    operation_coverage_mode: String,
    reported_operations: Vec<String>,
    reported_fixture_families: Vec<String>,
    reported_matrix_entries: usize,
    authoritative_for_cutover_acceptance: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfResultEntry {
    fixture_family: PerfFixtureFamily,
    operation: PerfOperation,
    cases: usize,
    total_requests: usize,
    fail_closed_total: usize,
    fail_closed_rate: f64,
    error_rate: f64,
    incomplete_rate: f64,
    metrics: PerfResultMetrics,
    anti_rescue: PerfAntiRescueCounts,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfResultMetrics {
    total_duration_ms: PerfMetrics,
    wait_for_file_version_ms: PerfMetrics,
    snapshot_preparation_ms: PerfMetrics,
    ir_query_ms: PerfMetrics,
    allocations_per_request: f64,
    allocated_bytes_per_request: f64,
    lock_wait_ms_per_request: f64,
    lock_contention_events_per_request: f64,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
struct PerfAntiRescueCounts {
    stale_fallback_total: u64,
    stale_served_total: u64,
    degraded_substitute_total: u64,
    search_backed_substitute_total: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfComparison {
    contract_version: String,
    verdict: String,
    reason_codes: Vec<String>,
    pass: bool,
    #[serde(default)]
    entries: Vec<serde_json::Value>,
}

fn resolve_expected_change_id_from_sources(
    cli_change_id: Option<&str>,
    env_change_id: Option<&str>,
) -> Option<String> {
    if let Some(value) = cli_change_id {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    env_change_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn resolve_expected_change_id(cli_change_id: Option<&str>) -> Option<String> {
    let env_change_id = std::env::var("OPENSPEC_CHANGE_ID").ok();
    resolve_expected_change_id_from_sources(cli_change_id, env_change_id.as_deref())
}

fn requires_authoritative_evidence_context(
    baseline_present: bool,
    update_baseline: bool,
    blocking_mode: bool,
    expected_change_id_present: bool,
) -> bool {
    expected_change_id_present && (blocking_mode || (baseline_present && !update_baseline))
}

fn should_compare_against_existing_baseline(baseline_present: bool, update_baseline: bool) -> bool {
    baseline_present && !update_baseline
}

fn now_unix_millis_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

fn contract_schema_version(contract: &serde_json::Value) -> Option<u64> {
    contract
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfThresholds {
    max_error_rate: f64,
    max_incomplete_rate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ResultGroupKey {
    fixture_family: PerfFixtureFamily,
    operation: PerfOperation,
}

#[derive(Default, Clone)]
struct MeasuredResults {
    total_duration_ms: Vec<f64>,
    wait_for_file_version_ms: Vec<f64>,
    snapshot_preparation_ms: Vec<f64>,
    ir_query_ms: Vec<f64>,
    errors: usize,
    fail_closed: usize,
    incomplete: usize,
    allocation_count_total: u64,
    allocated_bytes_total: u64,
    lock_wait_ms_total: f64,
    lock_contention_events_total: u64,
    anti_rescue: PerfAntiRescueCounts,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let scenario_path = args
        .scenario
        .canonicalize()
        .with_context(|| format!("Scenario not found: {}", args.scenario.to_string_lossy()))?;
    let workspace_root = workspace_root();
    let contract_path = args.contract_path.clone().unwrap_or_else(|| {
        workspace_root
            .join("contracts")
            .join("intellisense-perf-gate")
            .join("v2")
            .join("contract.json")
    });
    let contract = read_json_value(&contract_path)?;
    let scenario = read_scenario(&scenario_path)?;
    let expected_change_id = resolve_expected_change_id(args.change_id.as_deref());
    let requires_authoritative_evidence = requires_authoritative_evidence_context(
        args.baseline.is_some(),
        args.update_baseline,
        args.blocking_mode,
        expected_change_id.is_some(),
    );

    let syntax_helper_path = resolve_override(
        args.syntax_helper_path.as_ref(),
        scenario.syntax_helper_path.as_ref(),
        &workspace_root,
    );
    let config_path = resolve_override(
        args.config_path.as_ref(),
        scenario.config_path.as_ref(),
        &workspace_root,
    );
    let platform_version = args
        .platform_version
        .as_deref()
        .or(scenario.platform_version.as_deref());

    let prepared = prepare_cases(&scenario.cases, &workspace_root)?;
    let case_counts = build_case_group_counts(&prepared);
    let coordinator = Arc::new(SystemCoordinator::new());
    coordinator
        .start_with_paths_blocking(
            syntax_helper_path.as_deref(),
            config_path.as_deref(),
            platform_version,
            None,
        )
        .context("startup failed")?;

    let deps_bundle = build_deps_bundle_v2(
        &coordinator,
        syntax_helper_path.as_deref(),
        config_path.as_deref(),
    )
    .context("build_deps_bundle_v2")?;

    let deps = deps_bundle.semantic_deps.clone();
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
    let settings = bsl_backend::application::ExecutionSettings {
        settings_id: SettingsId::from_hash("intellisense-perf"),
        diagnostics_detail_level: DetailLevel::Full,
    };

    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: settings.settings_id.clone(),
        diagnostics_detail_level: settings.diagnostics_detail_level,
    });

    let facade = bsl_backend::application::IntellisenseV2Facade::new(
        host,
        deps_bundle.index_snapshot.clone(),
        Some(coordinator.clone()),
    );
    prime_runtime_files(&facade, &prepared).await;

    let mut churn_state = build_churn_state(&scenario, &prepared)?;
    let mut content_by_file = build_content_by_file_map(&prepared);
    let mut version_by_file = build_file_version_map(&prepared);
    let iteration_context = IterationContext {
        facade: &facade,
        deps_id: &deps_bundle.deps_id,
        settings,
        coordinator: coordinator.as_ref(),
        metadata_lookup: &metadata_lookup,
        resolver: resolver.as_ref(),
    };

    if args.warmup > 0 {
        run_iterations(
            &iteration_context,
            &prepared,
            args.warmup,
            &mut churn_state,
            &mut content_by_file,
            &mut version_by_file,
            None,
        )
        .await?;
    }

    let mut measured = HashMap::<ResultGroupKey, MeasuredResults>::new();
    run_iterations(
        &iteration_context,
        &prepared,
        args.iterations,
        &mut churn_state,
        &mut content_by_file,
        &mut version_by_file,
        Some(&mut measured),
    )
    .await?;

    let results = build_results(&contract, args.iterations, &case_counts, &measured);
    let coverage = build_coverage(&contract, &results);
    let thresholds = PerfThresholds {
        max_error_rate: args.max_error_rate,
        max_incomplete_rate: args.max_incomplete_rate,
    };
    let gate_thresholds = PerfGateThresholds {
        latency_ratio_p95_max: args.threshold_p95,
        latency_ratio_p99_max: args.threshold_p99,
        resource_ratio_max: args.threshold_resource,
        max_error_rate: args.max_error_rate,
        max_incomplete_rate: args.max_incomplete_rate,
        blocking_mode: args.blocking_mode,
    };
    let mut report = PerfReport {
        scenario: scenario.name.clone(),
        profile: scenario.name.clone(),
        cases: prepared.len(),
        iterations: args.iterations,
        warmup: args.warmup,
        change_id: expected_change_id.clone(),
        provenance: Some(PerfReportProvenance {
            change_id: expected_change_id.clone(),
            generated_at: Some(now_unix_millis_string()),
            profile: Some(scenario.name.clone()),
            schema_version: contract_schema_version(&contract),
            contract_version: Some(contract_version_from_contract(&contract)),
        }),
        contract_version: contract_version_from_contract(&contract),
        coverage,
        results,
        thresholds: Some(thresholds.clone()),
        verdict: "pass".to_string(),
        reason_codes: Vec::new(),
        pass: true,
        comparison: None,
    };
    let provenance_validation_error = {
        let report_value = serde_json::to_value(&report)
            .context("failed to serialize report for provenance validation")?;
        validate_perf_report_provenance(&report_value, expected_change_id.as_deref()).err()
    };
    let cutover_authority_validation_error = if requires_authoritative_evidence {
        validate_cutover_evidence_authority(expected_change_id.as_deref()).err()
    } else {
        None
    };

    let comparison = if let Some(baseline_path) = args.baseline.as_ref() {
        if let Some(reason_code) = cutover_authority_validation_error.as_ref() {
            Some(build_provenance_failure_comparison(
                &report,
                reason_code,
                gate_thresholds,
            ))
        } else if let Some(reason_code) = provenance_validation_error.as_ref() {
            Some(build_provenance_failure_comparison(
                &report,
                reason_code,
                gate_thresholds,
            ))
        } else if should_compare_against_existing_baseline(
            baseline_path.exists(),
            args.update_baseline,
        ) {
            let baseline_raw = read_json_value(baseline_path)?;
            let baseline: PerfReport =
                serde_json::from_value(baseline_raw).context("Invalid baseline JSON")?;
            Some(compare_reports(
                &contract,
                &scenario.name,
                &report,
                &baseline,
                gate_thresholds,
            ))
        } else if args.update_baseline {
            None
        } else {
            bail!("Baseline not found: {}", baseline_path.display());
        }
    } else {
        None
    };
    report.comparison = comparison.clone();
    if let Some(comparison) = &comparison {
        report.verdict = comparison.verdict.clone();
        report.reason_codes = comparison.reason_codes.clone();
        report.pass = comparison.pass;
    }

    if let Some(output_path) = args.output.as_ref() {
        write_report(output_path, &report)?;
    }
    if let Some(summary_path) = args.summary.as_ref() {
        write_summary(summary_path, &report)?;
    }

    println!("{}", serde_json::to_string_pretty(&report)?);

    if args.update_baseline {
        let baseline_path = args
            .baseline
            .as_ref()
            .context("baseline path required with --update-baseline")?;
        write_report(baseline_path, &report)?;
    }

    if let Some(comparison) = comparison {
        if !comparison.pass {
            bail!(
                "Regression detected: verdict={}, reason_codes={:?}",
                comparison.verdict,
                comparison.reason_codes,
            );
        }
    } else if let Some(reason_code) = cutover_authority_validation_error {
        bail!(
            "Perf report cutover evidence authority validation failed: reason_code={}",
            reason_code
        );
    } else if let Some(reason_code) = provenance_validation_error {
        bail!(
            "Perf report provenance validation failed: reason_code={}",
            reason_code
        );
    }

    Ok(())
}
