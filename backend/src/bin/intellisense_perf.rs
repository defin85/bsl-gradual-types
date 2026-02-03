//! IntelliSense performance harness for completion latency regression checks.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::application::get_completion_with_semantic_program_snapshot;
use bsl_backend::system::{build_deps_bundle_v2, SystemCoordinator};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::DetailLevel;

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

#[derive(Debug)]
struct PreparedCase {
    file_id: V2FileId,
    file_uri: String,
    content: Arc<str>,
    line: u32,
    column: u32,
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerfComparison {
    baseline_p95_ms: f64,
    baseline_p99_ms: f64,
    ratio_p95: f64,
    ratio_p99: f64,
    threshold_p95: f64,
    threshold_p99: f64,
    max_error_rate: f64,
    max_incomplete_rate: f64,
    error_rate: f64,
    incomplete_rate: f64,
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

    let analysis = host.analysis();

    if args.warmup > 0 {
        run_iterations(
            &analysis,
            deps_bundle.index_snapshot.as_ref(),
            &metadata_lookup,
            resolver.as_ref(),
            &prepared,
            args.warmup,
            None,
        )
        .await?;
    }

    let mut durations_ms = Vec::new();
    let mut errors = 0usize;
    let mut incomplete = 0usize;
    run_iterations(
        &analysis,
        deps_bundle.index_snapshot.as_ref(),
        &metadata_lookup,
        resolver.as_ref(),
        &prepared,
        args.iterations,
        Some(OutputTargets {
            durations: &mut durations_ms,
            errors: &mut errors,
            incomplete: &mut incomplete,
        }),
    )
    .await?;

    let total_requests = prepared.len() * args.iterations;
    let metrics = build_metrics(total_requests, &durations_ms, errors, incomplete);
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
                &report,
                &baseline,
                args.threshold_p95,
                args.threshold_p99,
                args.max_error_rate,
                args.max_incomplete_rate,
                rate_pass,
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
                "Regression detected: ratio_p95={:.3} (<= {:.2}), ratio_p99={:.3} (<= {:.2}), error_rate={:.3} (<= {:.3}), incomplete_rate={:.3} (<= {:.3})",
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
    analysis: &bsl_analysis_v2::AnalysisV2,
    index_snapshot: &bsl_backend::system::IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
    resolver: &TypeResolver,
    cases: &[PreparedCase],
    iterations: usize,
    mut output: Option<OutputTargets<'_>>,
) -> Result<()> {
    for _ in 0..iterations {
        for case in cases {
            let started = Instant::now();

            let ir_program = analysis
                .ir(case.file_id)
                .map_err(|_| anyhow::anyhow!("ir query cancelled"))?
                .context("ir unavailable")?;

	            let result = get_completion_with_semantic_program_snapshot(
	                case.content.as_ref(),
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
            }
        }
    }
    Ok(())
}

struct OutputTargets<'a> {
    durations: &'a mut Vec<f64>,
    errors: &'a mut usize,
    incomplete: &'a mut usize,
}

fn build_metrics(
    total_requests: usize,
    durations: &[f64],
    errors: usize,
    incomplete: usize,
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

    PerfMetrics {
        total_requests,
        count,
        p50_ms: p50,
        p95_ms: p95,
        p99_ms: p99,
        error_rate,
        incomplete_rate,
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
    current: &PerfReport,
    baseline: &PerfReport,
    threshold_p95: f64,
    threshold_p99: f64,
    max_error_rate: f64,
    max_incomplete_rate: f64,
    rate_pass: bool,
) -> PerfComparison {
    let baseline_p95 = baseline.metrics.p95_ms.max(0.000_001);
    let baseline_p99 = baseline.metrics.p99_ms.max(0.000_001);
    let ratio_p95 = current.metrics.p95_ms / baseline_p95;
    let ratio_p99 = current.metrics.p99_ms / baseline_p99;
    let pass = ratio_p95 <= threshold_p95 && ratio_p99 <= threshold_p99 && rate_pass;

    PerfComparison {
        baseline_p95_ms: baseline.metrics.p95_ms,
        baseline_p99_ms: baseline.metrics.p99_ms,
        ratio_p95,
        ratio_p99,
        threshold_p95,
        threshold_p99,
        max_error_rate,
        max_incomplete_rate,
        error_rate: current.metrics.error_rate,
        incomplete_rate: current.metrics.incomplete_rate,
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
