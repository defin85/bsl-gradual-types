use std::path::PathBuf;
use std::time::Duration;

use crate::system::runtime_config::{global_runtime_config, RuntimeKey};

#[derive(Debug, Clone)]
pub(crate) struct ParseMetricsConfig {
    pub(crate) slow_threshold: Duration,
    pub(crate) top_n: usize,
    pub(crate) log_each: bool,
}

pub(crate) fn parse_metrics_config() -> ParseMetricsConfig {
    ParseMetricsConfig {
        slow_threshold: Duration::from_millis(
            global_runtime_config()
                .get_u64(RuntimeKey::SlowModuleThresholdMs)
                .unwrap_or(3000),
        ),
        top_n: global_runtime_config()
            .get_usize(RuntimeKey::SlowModuleTopN)
            .unwrap_or(5)
            .max(1),
        log_each: global_runtime_config()
            .get_bool(RuntimeKey::ModuleParseLogEach)
            .unwrap_or(false),
    }
}

pub(crate) fn report_slow_modules(slow_modules: &mut [(Duration, PathBuf)]) {
    let metrics = parse_metrics_config();
    if slow_modules.is_empty() {
        return;
    }
    slow_modules.sort_by(|a, b| b.0.cmp(&a.0));
    let top = slow_modules.iter().take(metrics.top_n);
    let table = format_slow_modules_table(top);
    tracing::info!(
        "Медленный парсинг модулей: всего={}\n{}",
        slow_modules.len(),
        table
    );
}

pub(crate) fn human_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    if secs > 0 {
        format!("{}.{:03}s", secs, millis)
    } else {
        format!("{}ms", millis)
    }
}

fn format_slow_modules_table<'a>(rows: impl Iterator<Item = &'a (Duration, PathBuf)>) -> String {
    let mut collected: Vec<(String, String)> = Vec::new();
    for (idx, (elapsed, path)) in rows.enumerate() {
        let rank = format!("{}", idx + 1);
        let duration = human_duration(*elapsed);
        let path_str = path.to_string_lossy().to_string();
        collected.push((rank, format!("{} | {}", duration, path_str)));
    }

    let rank_width = collected
        .iter()
        .map(|(rank, _)| rank.len())
        .max()
        .unwrap_or(1)
        .max(1);

    let detail_width = collected
        .iter()
        .map(|(_, detail)| detail.len())
        .max()
        .unwrap_or(1)
        .max(1);

    let mut out = String::new();
    let border = format!(
        "+-{:-<rank$}-+-{:-<detail$}-+",
        "",
        "",
        rank = rank_width,
        detail = detail_width
    );
    out.push_str(&border);
    out.push('\n');
    out.push_str(&format!(
        "| {:<rank$} | {:<detail$} |",
        "N",
        "duration | module_path",
        rank = rank_width,
        detail = detail_width
    ));
    out.push('\n');
    out.push_str(&border);
    out.push('\n');

    for (rank, detail) in collected {
        out.push_str(&format!(
            "| {:<rank$} | {:<detail$} |",
            rank,
            detail,
            rank = rank_width,
            detail = detail_width
        ));
        out.push('\n');
    }

    out.push_str(&border);
    out
}
