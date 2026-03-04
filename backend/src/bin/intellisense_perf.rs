//! IntelliSense performance harness for completion latency regression checks.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
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
use bsl_backend::application::get_completion_with_semantic_program_snapshot;
use bsl_backend::perf_gate_evaluator::{
    evaluate_intellisense_perf_profile, validate_cutover_evidence_authority,
    validate_perf_report_provenance, PerfGateSample, PerfGateThresholds,
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
    build_metrics, build_missing_required_metric_comparison, build_provenance_failure_comparison,
    compare_reports, contract_version_from_contract, missing_resource_metric_keys, read_json_value,
    write_report, write_summary,
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
        iteration.is_multiple_of(self.plan.every)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfReport {
    scenario: String,
    cases: usize,
    iterations: usize,
    warmup: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    change_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provenance: Option<PerfReportProvenance>,
    #[serde(default = "unknown_contract_version")]
    contract_version: String,
    metrics: PerfMetrics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thresholds: Option<PerfThresholds>,
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
    total_requests: usize,
    count: usize,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    error_rate: f64,
    incomplete_rate: f64,
    allocations_per_completion: f64,
    allocated_bytes_per_completion: f64,
    lock_wait_ms_per_completion: f64,
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
) -> bool {
    blocking_mode || (baseline_present && !update_baseline)
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

#[derive(Default)]
struct MeasuredResults {
    durations_ms: Vec<f64>,
    errors: usize,
    incomplete: usize,
    allocation_count_total: u64,
    allocated_bytes_total: u64,
    lock_wait_ms_total: f64,
    lock_contention_events_total: u64,
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
    let expected_change_id = resolve_expected_change_id(args.change_id.as_deref());
    let requires_authoritative_evidence = requires_authoritative_evidence_context(
        args.baseline.is_some(),
        args.update_baseline,
        args.blocking_mode,
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
    let iteration_context = IterationContext {
        index_snapshot: deps_bundle.index_snapshot.as_ref(),
        metadata_lookup: &metadata_lookup,
        resolver: resolver.as_ref(),
        cases: &prepared,
    };

    if args.warmup > 0 {
        run_iterations(
            &mut host,
            &iteration_context,
            args.warmup,
            &mut churn_state,
            &mut content_by_file,
            None,
        )
        .await?;
    }

    let mut measured = MeasuredResults::default();
    run_iterations(
        &mut host,
        &iteration_context,
        args.iterations,
        &mut churn_state,
        &mut content_by_file,
        Some(OutputTargets {
            durations: &mut measured.durations_ms,
            errors: &mut measured.errors,
            incomplete: &mut measured.incomplete,
            allocation_count_total: &mut measured.allocation_count_total,
            allocated_bytes_total: &mut measured.allocated_bytes_total,
            lock_wait_ms_total: &mut measured.lock_wait_ms_total,
            lock_contention_events_total: &mut measured.lock_contention_events_total,
        }),
    )
    .await?;

    let total_requests = prepared.len() * args.iterations;
    let metrics = build_metrics(total_requests, &measured);
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
        metrics,
        thresholds: Some(thresholds.clone()),
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

    let rate_pass = report.metrics.error_rate <= args.max_error_rate
        && report.metrics.incomplete_rate <= args.max_incomplete_rate;

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
        } else if baseline_path.exists() {
            let baseline_raw = read_json_value(baseline_path)?;
            let current_raw =
                serde_json::to_value(&report).context("failed to serialize current report")?;
            let mut missing_fields = Vec::new();
            missing_fields.extend(
                missing_resource_metric_keys(&current_raw)
                    .into_iter()
                    .map(|field| format!("current.metrics.{field}")),
            );
            missing_fields.extend(
                missing_resource_metric_keys(&baseline_raw)
                    .into_iter()
                    .map(|field| format!("baseline.metrics.{field}")),
            );

            if !missing_fields.is_empty() {
                eprintln!(
                    "missing_required_metric_field: {}",
                    missing_fields.join(", ")
                );
                Some(build_missing_required_metric_comparison(
                    &contract,
                    &report,
                    gate_thresholds.latency_ratio_p95_max,
                    gate_thresholds.latency_ratio_p99_max,
                    gate_thresholds.resource_ratio_max,
                    gate_thresholds.max_error_rate,
                    gate_thresholds.max_incomplete_rate,
                ))
            } else {
                let baseline: PerfReport =
                    serde_json::from_value(baseline_raw).context("Invalid baseline JSON")?;
                Some(compare_reports(
                    &contract,
                    &scenario.name,
                    &report,
                    &baseline,
                    gate_thresholds,
                ))
            }
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
