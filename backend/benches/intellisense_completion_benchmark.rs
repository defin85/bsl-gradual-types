//! Benchmark IntelliSense completion latency.
//!
//! Usage:
//!   cargo bench --bench intellisense_completion_benchmark

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde::Deserialize;

use bsl_backend::system::SystemCoordinator;

#[derive(Debug, Deserialize)]
struct Scenario {
    name: String,
    #[serde(default)]
    syntax_helper_path: Option<PathBuf>,
    #[serde(default)]
    config_path: Option<PathBuf>,
    cases: Vec<ScenarioCase>,
}

#[derive(Debug, Deserialize)]
struct ScenarioCase {
    file: PathBuf,
    marker: String,
}

#[derive(Debug)]
struct PreparedCase {
    content: String,
    line: u32,
    column: u32,
    file_uri: String,
}

fn completion_benchmark(c: &mut Criterion) {
    let scenario_path = default_scenario_path();
    let scenario = match read_scenario(&scenario_path) {
        Ok(value) => value,
        Err(err) => {
            eprintln!(
                "WARN: Scenario not available ({}): {}",
                scenario_path.display(),
                err
            );
            return;
        }
    };

    let workspace_root = workspace_root();
    let syntax_helper_path = scenario
        .syntax_helper_path
        .as_ref()
        .map(|path| resolve_relative(&workspace_root, path));
    let config_path = scenario
        .config_path
        .as_ref()
        .map(|path| resolve_relative(&workspace_root, path));

    let prepared = match prepare_cases(&scenario.cases, &workspace_root) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("WARN: Failed to prepare cases: {}", err);
            return;
        }
    };

    let coordinator = SystemCoordinator::new();
    if let Err(err) = coordinator.start_with_paths_blocking(
        syntax_helper_path.as_deref(),
        config_path.as_deref(),
        None,
        None,
    ) {
        eprintln!("WARN: Startup failed: {}", err);
        return;
    }

    let type_service = match coordinator.type_service() {
        Some(service) => service,
        None => {
            eprintln!("WARN: TypeSystemService unavailable after startup");
            return;
        }
    };

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let bench_name = format!("intellisense_completion_{}", scenario.name);

    c.bench_function(&bench_name, |b| {
        b.iter(|| {
            runtime.block_on(async {
                for case in &prepared {
                    let result = type_service
                        .get_completion(
                            &case.content,
                            case.line,
                            case.column,
                            Some(case.file_uri.as_str()),
                        )
                        .await;
                    black_box(result).ok();
                }
            });
        })
    });
}

fn default_scenario_path() -> PathBuf {
    let workspace_root = workspace_root();
    workspace_root.join("backend/tests/perf/scenarios/intellisense_medium.json")
}

fn read_scenario(path: &Path) -> Result<Scenario> {
    let data = fs::read_to_string(path).with_context(|| {
        format!("Failed to read scenario file: {}", path.to_string_lossy())
    })?;
    let scenario: Scenario = serde_json::from_str(&data).context("Invalid scenario JSON")?;
    Ok(scenario)
}

fn prepare_cases(cases: &[ScenarioCase], base_dir: &Path) -> Result<Vec<PreparedCase>> {
    let mut prepared = Vec::with_capacity(cases.len());
    for case in cases {
        let file = resolve_relative(base_dir, &case.file);
        let content = fs::read_to_string(&file).with_context(|| {
            format!("Failed to read case file: {}", file.to_string_lossy())
        })?;
        let (line, column) = find_position(&content, &case.marker).with_context(|| {
            format!(
                "Marker '{}' not found in {}",
                case.marker,
                file.to_string_lossy()
            )
        })?;
        prepared.push(PreparedCase {
            content,
            line,
            column,
            file_uri: file.to_string_lossy().into_owned(),
        });
    }
    Ok(prepared)
}

fn find_position(content: &str, marker: &str) -> Option<(u32, u32)> {
    let byte_index = content.find(marker)?;
    let before = &content[..byte_index + marker.len()];
    let line = before.lines().count().saturating_sub(1) as u32;
    let last_line = before.lines().last().unwrap_or("");
    let character = last_line
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;
    Some((line, character))
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

criterion_group!(benches, completion_benchmark);
criterion_main!(benches);
