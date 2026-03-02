//! IntelliSense performance harness for completion latency regression checks.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::application::get_completion_with_semantic_program_snapshot;
use bsl_backend::perf_gate_evaluator::{
    evaluate_intellisense_perf_profile, PerfGateSample, PerfGateThresholds,
};
use bsl_backend::system::{build_deps_bundle_v2, SystemCoordinator};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::DetailLevel;

struct CountingAllocator;

static ALLOCATION_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

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

#[derive(Debug, Deserialize)]
struct ScenarioCase {
    file: PathBuf,
    marker: String,
    #[allow(dead_code)]
    #[serde(default)]
    label: Option<String>,
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

#[derive(Debug)]
struct PreparedCase {
    file_id: V2FileId,
    file_uri: String,
    content: Arc<str>,
    line: u32,
    column: u32,
}

#[derive(Debug, Clone)]
struct ChurnPlan {
    every: usize,
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

    fn should_apply(&self, iteration: usize) -> bool {
        iteration % self.plan.every == 0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfReport {
    scenario: String,
    cases: usize,
    iterations: usize,
    warmup: usize,
    metrics: PerfMetrics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thresholds: Option<PerfThresholds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comparison: Option<PerfComparison>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfMetrics {
    total_requests: usize,
    count: usize,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    error_rate: f64,
    incomplete_rate: f64,
    #[serde(default)]
    allocations_per_completion: f64,
    #[serde(default)]
    allocated_bytes_per_completion: f64,
    #[serde(default)]
    lock_wait_ms_per_completion: f64,
    #[serde(default)]
    lock_contention_events_per_completion: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfComparison {
    baseline_p95_ms: f64,
    baseline_p99_ms: f64,
    ratio_p95: f64,
    ratio_p99: f64,
    threshold_p95: f64,
    threshold_p99: f64,
    threshold_resource: f64,
    max_error_rate: f64,
    max_incomplete_rate: f64,
    error_rate: f64,
    incomplete_rate: f64,
    contract_version: String,
    verdict: String,
    reason_codes: Vec<String>,
    pass: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfThresholds {
    max_error_rate: f64,
    max_incomplete_rate: f64,
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
            .join("v1")
            .join("contract.json")
    });
    let contract = read_json_value(&contract_path)?;
    let scenario = read_scenario(&scenario_path)?;

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
    let coordinator = SystemCoordinator::new();
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

    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("intellisense-perf"),
        diagnostics_detail_level: DetailLevel::Full,
    });

    for case in &prepared {
        host.apply_change(ChangeV2::SetFile {
            file_id: case.file_id,
            text: case.content.clone(),
            version: 0,
            path: Arc::from(case.file_uri.clone()),
        });
    }

    let mut churn_state = build_churn_state(&scenario, &prepared)?;
    let mut content_by_file = build_content_by_file_map(&prepared);

    if args.warmup > 0 {
        run_iterations(
            &mut host,
            deps_bundle.index_snapshot.as_ref(),
            &metadata_lookup,
            resolver.as_ref(),
            &prepared,
            args.warmup,
            &mut churn_state,
            &mut content_by_file,
            None,
        )
        .await?;
    }

    let mut durations_ms = Vec::new();
    let mut errors = 0usize;
    let mut incomplete = 0usize;
    let mut allocation_count_total = 0_u64;
    let mut allocated_bytes_total = 0_u64;
    let mut lock_wait_ms_total = 0.0_f64;
    let mut lock_contention_events_total = 0_u64;
    run_iterations(
        &mut host,
        deps_bundle.index_snapshot.as_ref(),
        &metadata_lookup,
        resolver.as_ref(),
        &prepared,
        args.iterations,
        &mut churn_state,
        &mut content_by_file,
        Some(OutputTargets {
            durations: &mut durations_ms,
            errors: &mut errors,
            incomplete: &mut incomplete,
            allocation_count_total: &mut allocation_count_total,
            allocated_bytes_total: &mut allocated_bytes_total,
            lock_wait_ms_total: &mut lock_wait_ms_total,
            lock_contention_events_total: &mut lock_contention_events_total,
        }),
    )
    .await?;

    let total_requests = prepared.len() * args.iterations;
    let metrics = build_metrics(
        total_requests,
        &durations_ms,
        errors,
        incomplete,
        allocation_count_total,
        allocated_bytes_total,
        lock_wait_ms_total,
        lock_contention_events_total,
    );
    let thresholds = PerfThresholds {
        max_error_rate: args.max_error_rate,
        max_incomplete_rate: args.max_incomplete_rate,
    };
    let mut report = PerfReport {
        scenario: scenario.name.clone(),
        cases: prepared.len(),
        iterations: args.iterations,
        warmup: args.warmup,
        metrics,
        thresholds: Some(thresholds.clone()),
        comparison: None,
    };

    let rate_pass = report.metrics.error_rate <= args.max_error_rate
        && report.metrics.incomplete_rate <= args.max_incomplete_rate;

    let comparison = if let Some(baseline_path) = args.baseline.as_ref() {
        if baseline_path.exists() {
            let baseline = read_report(baseline_path)?;
            Some(compare_reports(
                &contract,
                &scenario.name,
                &report,
                &baseline,
                args.threshold_p95,
                args.threshold_p99,
                args.threshold_resource,
                args.max_error_rate,
                args.max_incomplete_rate,
                args.blocking_mode,
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
                "Regression detected: verdict={}, reason_codes={:?}, ratio_p95={:.3} (<= {:.2}), ratio_p99={:.3} (<= {:.2}), error_rate={:.3} (<= {:.3}), incomplete_rate={:.3} (<= {:.3})",
                comparison.verdict,
                comparison.reason_codes,
                comparison.ratio_p95,
                comparison.threshold_p95,
                comparison.ratio_p99,
                comparison.threshold_p99,
                comparison.error_rate,
                comparison.max_error_rate,
                comparison.incomplete_rate,
                comparison.max_incomplete_rate
            );
        }
    } else if !rate_pass {
        bail!(
            "Rates exceeded: error_rate={:.3} (<= {:.3}), incomplete_rate={:.3} (<= {:.3})",
            report.metrics.error_rate,
            args.max_error_rate,
            report.metrics.incomplete_rate,
            args.max_incomplete_rate
        );
    }

    Ok(())
}

fn read_scenario(path: &Path) -> Result<Scenario> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read scenario file: {}", path.to_string_lossy()))?;
    let scenario: Scenario = serde_json::from_str(&data).context("Invalid scenario JSON")?;
    if scenario.cases.is_empty() {
        bail!("Scenario must contain at least one case");
    }
    Ok(scenario)
}

fn resolve_override(
    override_path: Option<&PathBuf>,
    scenario_path: Option<&PathBuf>,
    base_dir: &Path,
) -> Option<PathBuf> {
    let path = override_path.or(scenario_path)?;
    Some(resolve_relative(base_dir, path))
}

fn resolve_relative(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or(manifest_dir)
}

fn prepare_cases(cases: &[ScenarioCase], base_dir: &Path) -> Result<Vec<PreparedCase>> {
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
        });
    }

    Ok(prepared)
}

fn find_position(content: &str, marker: &str) -> Option<(u32, u32)> {
    let byte_index = content.find(marker)?;
    let before = &content[..byte_index + marker.len()];
    let line = before.lines().count().saturating_sub(1) as u32;
    let last_line = before.lines().last().unwrap_or("");
    let character = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    Some((line, character))
}

async fn run_iterations(
    host: &mut AnalysisHostV2,
    index_snapshot: &bsl_backend::system::IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
    resolver: &TypeResolver,
    cases: &[PreparedCase],
    iterations: usize,
    churn_state: &mut Option<ChurnRuntimeState>,
    content_by_file: &mut HashMap<String, Arc<str>>,
    mut output: Option<OutputTargets<'_>>,
) -> Result<()> {
    for iteration in 0..iterations {
        maybe_apply_churn(host, churn_state, content_by_file, iteration)?;
        let analysis = host.analysis();
        for case in cases {
            let started = Instant::now();
            let alloc_before = allocation_snapshot();
            let case_content = content_by_file
                .get(case.file_uri.as_str())
                .cloned()
                .unwrap_or_else(|| case.content.clone());

            let ir_started = Instant::now();
            let ir_program = analysis
                .ir(case.file_id)
                .map_err(|_| anyhow::anyhow!("ir query cancelled"))?
                .context("ir unavailable")?;
            let ir_elapsed_ms = ir_started.elapsed().as_secs_f64() * 1000.0;

            let result = get_completion_with_semantic_program_snapshot(
                case_content.as_ref(),
                case.line,
                case.column,
                Some(case.file_uri.as_str()),
                index_snapshot,
                metadata_lookup,
                case.file_uri.as_str(),
                resolver,
                ir_program,
                None,
                false,
            )
            .await;
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            let alloc_after = allocation_snapshot();
            let alloc_delta = alloc_after.count.saturating_sub(alloc_before.count);
            let bytes_delta = alloc_after.bytes.saturating_sub(alloc_before.bytes);
            let lock_contention_event = if ir_elapsed_ms > 0.0 { 1_u64 } else { 0_u64 };

            if let Some(targets) = output.as_mut() {
                match result {
                    Ok(response) => {
                        targets.durations.push(elapsed_ms);
                        if response.is_incomplete {
                            *targets.incomplete += 1;
                        }
                    }
                    Err(_) => {
                        *targets.errors += 1;
                    }
                }
                *targets.allocation_count_total += alloc_delta;
                *targets.allocated_bytes_total += bytes_delta;
                *targets.lock_wait_ms_total += ir_elapsed_ms;
                *targets.lock_contention_events_total += lock_contention_event;
            }
        }
    }
    Ok(())
}

fn build_content_by_file_map(cases: &[PreparedCase]) -> HashMap<String, Arc<str>> {
    let mut map = HashMap::new();
    for case in cases {
        map.entry(case.file_uri.clone())
            .or_insert_with(|| case.content.clone());
    }
    map
}

fn build_churn_state(
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
        target_file_uri,
        target_file_path,
        target_file_ids,
        base_content: target_case_ref.content.clone(),
    };
    Ok(Some(ChurnRuntimeState::new(plan)))
}

fn maybe_apply_churn(
    host: &mut AnalysisHostV2,
    churn_state: &mut Option<ChurnRuntimeState>,
    content_by_file: &mut HashMap<String, Arc<str>>,
    iteration: usize,
) -> Result<()> {
    let Some(state) = churn_state.as_mut() else {
        return Ok(());
    };
    if !state.should_apply(iteration) {
        return Ok(());
    }

    state.revision = state.revision.saturating_add(1);
    let churned_content = build_churned_content(state.plan.base_content.as_ref(), state.revision);
    let churned_content_arc: Arc<str> = Arc::from(churned_content);

    for file_id in &state.plan.target_file_ids {
        host.apply_change(ChangeV2::SetFile {
            file_id: *file_id,
            text: churned_content_arc.clone(),
            version: state.next_version,
            path: state.plan.target_file_path.clone(),
        });
    }
    content_by_file.insert(state.plan.target_file_uri.clone(), churned_content_arc);
    state.next_version = state.next_version.saturating_add(1);

    Ok(())
}

fn build_churned_content(base_content: &str, revision: u64) -> String {
    let mut content = String::with_capacity(base_content.len() + 64);
    content.push_str(base_content);
    if !content.ends_with('\n') {
        content.push('\n');
    }
    let marker = if revision % 2 == 0 { "B" } else { "A" };
    content.push_str("// __intellisense_perf_churn_marker__ ");
    content.push_str(marker);
    content.push('\n');
    content
}

struct OutputTargets<'a> {
    durations: &'a mut Vec<f64>,
    errors: &'a mut usize,
    incomplete: &'a mut usize,
    allocation_count_total: &'a mut u64,
    allocated_bytes_total: &'a mut u64,
    lock_wait_ms_total: &'a mut f64,
    lock_contention_events_total: &'a mut u64,
}

fn build_metrics(
    total_requests: usize,
    durations: &[f64],
    errors: usize,
    incomplete: usize,
    allocation_count_total: u64,
    allocated_bytes_total: u64,
    lock_wait_ms_total: f64,
    lock_contention_events_total: u64,
) -> PerfMetrics {
    let count = durations.len();
    let (p50, p95, p99) = if count == 0 {
        (0.0, 0.0, 0.0)
    } else {
        let mut sorted = durations.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        (
            percentile_sorted(&sorted, 0.50),
            percentile_sorted(&sorted, 0.95),
            percentile_sorted(&sorted, 0.99),
        )
    };

    let total_requests_f = total_requests as f64;
    let error_rate = if total_requests == 0 {
        0.0
    } else {
        errors as f64 / total_requests_f
    };
    let incomplete_rate = if total_requests == 0 {
        0.0
    } else {
        incomplete as f64 / total_requests_f
    };
    let allocations_per_completion = if total_requests == 0 {
        0.0
    } else {
        allocation_count_total as f64 / total_requests_f
    };
    let allocated_bytes_per_completion = if total_requests == 0 {
        0.0
    } else {
        allocated_bytes_total as f64 / total_requests_f
    };
    let lock_wait_ms_per_completion = if total_requests == 0 {
        0.0
    } else {
        lock_wait_ms_total / total_requests_f
    };
    let lock_contention_events_per_completion = if total_requests == 0 {
        0.0
    } else {
        lock_contention_events_total as f64 / total_requests_f
    };

    PerfMetrics {
        total_requests,
        count,
        p50_ms: p50,
        p95_ms: p95,
        p99_ms: p99,
        error_rate,
        incomplete_rate,
        allocations_per_completion,
        allocated_bytes_per_completion,
        lock_wait_ms_per_completion,
        lock_contention_events_per_completion,
    }
}

fn percentile_sorted(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let clamped = percentile.clamp(0.0, 1.0);
    let rank = ((values.len() - 1) as f64 * clamped).round() as usize;
    values[rank]
}

fn read_report(path: &Path) -> Result<PerfReport> {
    let data =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let report: PerfReport = serde_json::from_str(&data).context("Invalid baseline JSON")?;
    Ok(report)
}

fn read_json_value(path: &Path) -> Result<serde_json::Value> {
    let data =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&data).context("Invalid JSON")?;
    Ok(value)
}

fn write_report(path: &Path, report: &PerfReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create report dir: {}", parent.to_string_lossy())
        })?;
    }
    let json = serde_json::to_string_pretty(report)?;
    fs::write(path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn compare_reports(
    contract: &serde_json::Value,
    profile: &str,
    current: &PerfReport,
    baseline: &PerfReport,
    threshold_p95: f64,
    threshold_p99: f64,
    threshold_resource: f64,
    max_error_rate: f64,
    max_incomplete_rate: f64,
    blocking_mode: bool,
) -> PerfComparison {
    let baseline_p95 = baseline.metrics.p95_ms.max(0.000_001);
    let baseline_p99 = baseline.metrics.p99_ms.max(0.000_001);
    let ratio_p95 = current.metrics.p95_ms / baseline_p95;
    let ratio_p99 = current.metrics.p99_ms / baseline_p99;
    let evaluation = evaluate_intellisense_perf_profile(
        contract,
        profile,
        PerfGateSample {
            p95_ms: current.metrics.p95_ms,
            p99_ms: current.metrics.p99_ms,
            error_rate: current.metrics.error_rate,
            incomplete_rate: current.metrics.incomplete_rate,
            allocations_per_completion: current.metrics.allocations_per_completion,
            allocated_bytes_per_completion: current.metrics.allocated_bytes_per_completion,
            lock_wait_ms_per_completion: current.metrics.lock_wait_ms_per_completion,
            lock_contention_events_per_completion: current
                .metrics
                .lock_contention_events_per_completion,
        },
        Some(PerfGateSample {
            p95_ms: baseline.metrics.p95_ms,
            p99_ms: baseline.metrics.p99_ms,
            error_rate: baseline.metrics.error_rate,
            incomplete_rate: baseline.metrics.incomplete_rate,
            allocations_per_completion: baseline.metrics.allocations_per_completion,
            allocated_bytes_per_completion: baseline.metrics.allocated_bytes_per_completion,
            lock_wait_ms_per_completion: baseline.metrics.lock_wait_ms_per_completion,
            lock_contention_events_per_completion: baseline
                .metrics
                .lock_contention_events_per_completion,
        }),
        PerfGateThresholds {
            latency_ratio_p95_max: threshold_p95,
            latency_ratio_p99_max: threshold_p99,
            resource_ratio_max: threshold_resource,
            max_error_rate,
            max_incomplete_rate,
            blocking_mode,
        },
    );
    let verdict = evaluation
        .get("verdict")
        .and_then(|value| value.as_str())
        .unwrap_or("fail")
        .to_string();
    let reason_codes = evaluation
        .get("reason_codes")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["missing_required_metric_field".to_string()]);
    let pass = verdict == "pass";

    PerfComparison {
        baseline_p95_ms: baseline.metrics.p95_ms,
        baseline_p99_ms: baseline.metrics.p99_ms,
        ratio_p95,
        ratio_p99,
        threshold_p95,
        threshold_p99,
        threshold_resource,
        max_error_rate,
        max_incomplete_rate,
        error_rate: current.metrics.error_rate,
        incomplete_rate: current.metrics.incomplete_rate,
        contract_version: evaluation
            .get("contract_version")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string(),
        verdict,
        reason_codes,
        pass,
    }
}

fn write_summary(path: &Path, report: &PerfReport) -> Result<()> {
    let mut lines = Vec::new();
    lines.push(format!("# IntelliSense perf summary ({})", report.scenario));
    lines.push(String::new());
    lines.push(format!("- cases: {}", report.cases));
    lines.push(format!("- iterations: {}", report.iterations));
    lines.push(format!("- warmup: {}", report.warmup));
    lines.push(format!(
        "- p50/p95/p99 (ms): {:.3} / {:.3} / {:.3}",
        report.metrics.p50_ms, report.metrics.p95_ms, report.metrics.p99_ms
    ));
    lines.push(format!(
        "- resource per completion (allocs/bytes/lock_wait_ms/lock_contention): {:.3} / {:.3} / {:.3} / {:.3}",
        report.metrics.allocations_per_completion,
        report.metrics.allocated_bytes_per_completion,
        report.metrics.lock_wait_ms_per_completion,
        report.metrics.lock_contention_events_per_completion
    ));
    lines.push(format!("- error_rate: {:.3}", report.metrics.error_rate));
    lines.push(format!(
        "- incomplete_rate: {:.3}",
        report.metrics.incomplete_rate
    ));
    if let Some(thresholds) = &report.thresholds {
        lines.push(format!(
            "- max_error_rate: {:.3}",
            thresholds.max_error_rate
        ));
        lines.push(format!(
            "- max_incomplete_rate: {:.3}",
            thresholds.max_incomplete_rate
        ));
    }
    if let Some(comparison) = &report.comparison {
        lines.push(format!(
            "- ratio_p95/ratio_p99: {:.3} / {:.3}",
            comparison.ratio_p95, comparison.ratio_p99
        ));
        lines.push(format!(
            "- thresholds p95/p99: {:.2} / {:.2}",
            comparison.threshold_p95, comparison.threshold_p99
        ));
        lines.push(format!(
            "- resource_ratio_threshold: {:.2}",
            comparison.threshold_resource
        ));
        lines.push(format!(
            "- contract_version: {}",
            comparison.contract_version
        ));
        lines.push(format!("- verdict: {}", comparison.verdict));
        lines.push(format!(
            "- reason_codes: {}",
            comparison.reason_codes.join(", ")
        ));
        lines.push(format!(
            "- pass: {}",
            if comparison.pass { "yes" } else { "no" }
        ));
    }
    lines.push(String::new());

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create summary dir: {}", parent.to_string_lossy())
        })?;
    }
    fs::write(path, lines.join("\n"))
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_churned_content_switches_marker_without_growth() {
        let base = "Процедура Тест()\nКонецПроцедуры";
        let first = build_churned_content(base, 1);
        let second = build_churned_content(base, 2);

        assert!(first.contains("__intellisense_perf_churn_marker__ A"));
        assert!(second.contains("__intellisense_perf_churn_marker__ B"));
        assert_eq!(
            first.matches("__intellisense_perf_churn_marker__").count(),
            1
        );
        assert_eq!(
            second.matches("__intellisense_perf_churn_marker__").count(),
            1
        );
    }

    #[test]
    fn build_churn_state_uses_target_case_file_for_all_matching_ids() {
        let file_uri = "/tmp/test.bsl".to_string();
        let cases = vec![
            PreparedCase {
                file_id: V2FileId(1),
                file_uri: file_uri.clone(),
                content: Arc::from("A"),
                line: 0,
                column: 0,
            },
            PreparedCase {
                file_id: V2FileId(2),
                file_uri: file_uri.clone(),
                content: Arc::from("A"),
                line: 0,
                column: 1,
            },
        ];
        let scenario = Scenario {
            name: "churn".to_string(),
            syntax_helper_path: None,
            config_path: None,
            platform_version: None,
            churn: Some(ScenarioChurn {
                every: 2,
                target_case: Some(1),
            }),
            cases: Vec::new(),
        };

        let state = build_churn_state(&scenario, &cases)
            .expect("churn state")
            .expect("churn enabled");
        assert_eq!(state.plan.every, 2);
        assert_eq!(state.plan.target_file_ids, vec![V2FileId(1), V2FileId(2)]);
    }
}
