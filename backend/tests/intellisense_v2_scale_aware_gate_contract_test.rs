use serde_json::{json, Value};

const LARGE_WAIT_RATIO_MAX: f64 = 0.60;
const LARGE_COMPLETION_RATIO_MAX: f64 = 0.75;
const SMALL_COMPLETION_RATIO_MAX: f64 = 1.25;
const MAX_CANCELLED_RATE: f64 = 0.10;
const MIN_COMPLETION_TOTAL: u64 = 50;

const REQUIRED_PHASES: &[&str] = &["start", "cold", "warm"];
const REQUIRED_METRICS: &[&str] = &[
    "completion_duration_ms",
    "intellisense_v2_wait_for_file_version_completion_ms",
    "intellisense_v2_snapshot_completion_ms",
    "intellisense_v2_ir_query_completion_ms",
];

#[derive(Debug, Clone, Copy)]
struct WarmMetrics {
    completion_p95_ms: f64,
    wait_p95_ms: f64,
    completion_total: u64,
    completion_cancelled_total: u64,
}

#[derive(Debug)]
struct GateVerdict {
    large_wait_ratio: f64,
    large_completion_ratio: f64,
    small_completion_ratio: f64,
    large_cancelled_rate: f64,
    small_cancelled_rate: f64,
    pass: bool,
}

fn get_value<'a>(root: &'a Value, path: &[&str]) -> Result<&'a Value, String> {
    let mut current = root;
    for segment in path {
        current = current
            .get(*segment)
            .ok_or_else(|| format!("missing field '{}'", path.join(".")))?;
    }
    Ok(current)
}

fn read_f64(root: &Value, path: &[&str]) -> Result<f64, String> {
    let value = get_value(root, path)?;
    value
        .as_f64()
        .or_else(|| value.as_u64().map(|n| n as f64))
        .ok_or_else(|| format!("field '{}' must be numeric", path.join(".")))
}

fn read_u64(root: &Value, path: &[&str]) -> Result<u64, String> {
    get_value(root, path)?
        .as_u64()
        .ok_or_else(|| format!("field '{}' must be u64", path.join(".")))
}

fn validate_profile_shape(report: &Value, profile: &str) -> Result<(), String> {
    for phase in REQUIRED_PHASES {
        for metric in REQUIRED_METRICS {
            let p95_path = ["profiles", profile, phase, "metrics", metric, "p95"];
            let count_path = ["profiles", profile, phase, "metrics", metric, "count"];
            let _ = read_f64(report, &p95_path)?;
            let _ = read_u64(report, &count_path)?;
        }
    }
    Ok(())
}

fn read_warm_metrics(report: &Value, profile: &str) -> Result<WarmMetrics, String> {
    validate_profile_shape(report, profile)?;

    Ok(WarmMetrics {
        completion_p95_ms: read_f64(
            report,
            &[
                "profiles",
                profile,
                "warm",
                "metrics",
                "completion_duration_ms",
                "p95",
            ],
        )?,
        wait_p95_ms: read_f64(
            report,
            &[
                "profiles",
                profile,
                "warm",
                "metrics",
                "intellisense_v2_wait_for_file_version_completion_ms",
                "p95",
            ],
        )?,
        completion_total: read_u64(report, &["profiles", profile, "warm", "completion_total"])?,
        completion_cancelled_total: read_u64(
            report,
            &["profiles", profile, "warm", "completion_cancelled_total"],
        )?,
    })
}

fn evaluate_scale_aware_gate(current: &Value, baseline: &Value) -> Result<GateVerdict, String> {
    let large_current = read_warm_metrics(current, "large")?;
    let small_current = read_warm_metrics(current, "small")?;
    let large_baseline = read_warm_metrics(baseline, "large")?;
    let small_baseline = read_warm_metrics(baseline, "small")?;

    let large_wait_ratio = large_current.wait_p95_ms / large_baseline.wait_p95_ms.max(0.000_001);
    let large_completion_ratio =
        large_current.completion_p95_ms / large_baseline.completion_p95_ms.max(0.000_001);
    let small_completion_ratio =
        small_current.completion_p95_ms / small_baseline.completion_p95_ms.max(0.000_001);

    let large_cancelled_rate = large_current.completion_cancelled_total as f64
        / large_current.completion_total.max(1) as f64;
    let small_cancelled_rate = small_current.completion_cancelled_total as f64
        / small_current.completion_total.max(1) as f64;

    let pass = large_wait_ratio <= LARGE_WAIT_RATIO_MAX
        && large_completion_ratio <= LARGE_COMPLETION_RATIO_MAX
        && small_completion_ratio <= SMALL_COMPLETION_RATIO_MAX
        && large_cancelled_rate <= MAX_CANCELLED_RATE
        && small_cancelled_rate <= MAX_CANCELLED_RATE
        && large_current.completion_total >= MIN_COMPLETION_TOTAL
        && small_current.completion_total >= MIN_COMPLETION_TOTAL;

    Ok(GateVerdict {
        large_wait_ratio,
        large_completion_ratio,
        small_completion_ratio,
        large_cancelled_rate,
        small_cancelled_rate,
        pass,
    })
}

fn phase(completion_p95: f64, wait_p95: f64, snapshot_p95: f64, ir_p95: f64, count: u64) -> Value {
    json!({
        "metrics": {
            "completion_duration_ms": { "p95": completion_p95, "count": count },
            "intellisense_v2_wait_for_file_version_completion_ms": { "p95": wait_p95, "count": count },
            "intellisense_v2_snapshot_completion_ms": { "p95": snapshot_p95, "count": count },
            "intellisense_v2_ir_query_completion_ms": { "p95": ir_p95, "count": count }
        }
    })
}

fn profile(phases: [Value; 3], warm_total: u64, warm_cancelled: u64) -> Value {
    let mut warm = phases[2].clone();
    warm["completion_total"] = json!(warm_total);
    warm["completion_cancelled_total"] = json!(warm_cancelled);

    json!({
        "start": phases[0],
        "cold": phases[1],
        "warm": warm
    })
}

fn make_report(
    large: [Value; 3],
    small: [Value; 3],
    large_total: u64,
    large_cancelled: u64,
    small_total: u64,
    small_cancelled: u64,
) -> Value {
    json!({
        "profiles": {
            "large": profile(large, large_total, large_cancelled),
            "small": profile(small, small_total, small_cancelled),
        }
    })
}

#[test]
fn scale_aware_gate_passes_with_expected_ratios() {
    let baseline = make_report(
        [
            phase(4200.0, 3200.0, 700.0, 320.0, 60),
            phase(4000.0, 3000.0, 680.0, 300.0, 80),
            phase(4000.0, 3000.0, 650.0, 280.0, 120),
        ],
        [
            phase(300.0, 8.0, 4.0, 180.0, 60),
            phase(280.0, 6.0, 3.0, 170.0, 80),
            phase(250.0, 5.0, 2.0, 160.0, 120),
        ],
        120,
        6,
        120,
        3,
    );
    let current = make_report(
        [
            phase(3100.0, 1800.0, 600.0, 260.0, 60),
            phase(2950.0, 1700.0, 560.0, 240.0, 80),
            phase(2900.0, 1700.0, 540.0, 220.0, 120),
        ],
        [
            phase(300.0, 7.0, 3.0, 180.0, 60),
            phase(290.0, 6.0, 3.0, 170.0, 80),
            phase(300.0, 5.0, 2.0, 165.0, 120),
        ],
        120,
        8,
        120,
        5,
    );

    let verdict = evaluate_scale_aware_gate(&current, &baseline).expect("gate evaluation");
    assert!(
        verdict.pass,
        "gate must pass: large_wait_ratio={:.3}, large_completion_ratio={:.3}, small_completion_ratio={:.3}, large_cancelled_rate={:.3}, small_cancelled_rate={:.3}",
        verdict.large_wait_ratio,
        verdict.large_completion_ratio,
        verdict.small_completion_ratio,
        verdict.large_cancelled_rate,
        verdict.small_cancelled_rate
    );
}

#[test]
fn scale_aware_gate_fails_when_large_ratio_is_worse_than_target() {
    let baseline = make_report(
        [
            phase(4200.0, 3200.0, 700.0, 320.0, 60),
            phase(4000.0, 3000.0, 680.0, 300.0, 80),
            phase(4000.0, 3000.0, 650.0, 280.0, 120),
        ],
        [
            phase(300.0, 8.0, 4.0, 180.0, 60),
            phase(280.0, 6.0, 3.0, 170.0, 80),
            phase(250.0, 5.0, 2.0, 160.0, 120),
        ],
        120,
        6,
        120,
        3,
    );
    let current = make_report(
        [
            phase(3900.0, 2500.0, 640.0, 260.0, 60),
            phase(3900.0, 2500.0, 620.0, 250.0, 80),
            phase(3900.0, 2500.0, 600.0, 240.0, 120),
        ],
        [
            phase(300.0, 7.0, 3.0, 180.0, 60),
            phase(290.0, 6.0, 3.0, 170.0, 80),
            phase(280.0, 5.0, 2.0, 165.0, 120),
        ],
        120,
        8,
        120,
        5,
    );

    let verdict = evaluate_scale_aware_gate(&current, &baseline).expect("gate evaluation");
    assert!(
        !verdict.pass,
        "gate must fail when large profile does not meet ratio target"
    );
    assert!(verdict.large_completion_ratio > LARGE_COMPLETION_RATIO_MAX);
}

#[test]
fn scale_aware_gate_rejects_missing_required_metric() {
    let baseline = make_report(
        [
            phase(4200.0, 3200.0, 700.0, 320.0, 60),
            phase(4000.0, 3000.0, 680.0, 300.0, 80),
            phase(4000.0, 3000.0, 650.0, 280.0, 120),
        ],
        [
            phase(300.0, 8.0, 4.0, 180.0, 60),
            phase(280.0, 6.0, 3.0, 170.0, 80),
            phase(250.0, 5.0, 2.0, 160.0, 120),
        ],
        120,
        6,
        120,
        3,
    );
    let mut broken_current = make_report(
        [
            phase(3100.0, 1800.0, 600.0, 260.0, 60),
            phase(2950.0, 1700.0, 560.0, 240.0, 80),
            phase(2900.0, 1700.0, 540.0, 220.0, 120),
        ],
        [
            phase(300.0, 7.0, 3.0, 180.0, 60),
            phase(290.0, 6.0, 3.0, 170.0, 80),
            phase(300.0, 5.0, 2.0, 165.0, 120),
        ],
        120,
        8,
        120,
        5,
    );

    broken_current["profiles"]["large"]["cold"]["metrics"]
        .as_object_mut()
        .expect("metrics object")
        .remove("intellisense_v2_ir_query_completion_ms");

    let error = evaluate_scale_aware_gate(&broken_current, &baseline)
        .expect_err("gate evaluation must fail on missing metric");
    assert!(
        error.contains("profiles.large.cold.metrics.intellisense_v2_ir_query_completion_ms"),
        "unexpected error: {error}"
    );
}
