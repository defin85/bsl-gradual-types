use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct ParseMetricsConfig {
    pub(crate) slow_threshold: Duration,
    pub(crate) top_n: usize,
    pub(crate) log_each: bool,
}

pub(crate) fn parse_metrics_config() -> &'static ParseMetricsConfig {
    static CONFIG: OnceLock<ParseMetricsConfig> = OnceLock::new();
    CONFIG.get_or_init(|| ParseMetricsConfig {
        slow_threshold: Duration::from_millis(env_u64("BSL_SLOW_MODULE_THRESHOLD_MS", 3000)),
        top_n: env_usize("BSL_SLOW_MODULE_TOP_N", 5).max(1),
        log_each: env_bool("BSL_MODULE_PARSE_LOG_EACH", false),
    })
}

pub(crate) fn report_slow_modules(slow_modules: &mut Vec<(Duration, PathBuf)>) {
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

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => default,
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
