use super::*;

const SIDEBAR_HISTOGRAM_KEYS: &[&str] = &[
    "intellisense_v2_wait_for_file_version_completion_ms",
    "intellisense_v2_snapshot_completion_ms",
    "intellisense_v2_ir_query_completion_ms",
    "intellisense_v2_wait_for_file_version_hover_ms",
    "intellisense_v2_snapshot_hover_ms",
    "intellisense_v2_ir_query_hover_ms",
    "intellisense_v2_wait_for_file_version_diagnostics_ms",
    "intellisense_v2_syntax_diagnostics_query_ms",
    "intellisense_v2_semantic_diagnostics_query_ms",
];

impl StructuredLogger {
    pub(super) fn new() -> Self {
        Self {}
    }

    pub(super) fn info(&self, event: &str, data: serde_json::Value) {
        info!(event = event, data = %data, "Structured log entry");
    }

    #[allow(dead_code)]
    pub(super) fn warn(&self, event: &str, data: serde_json::Value) {
        warn!(event = event, data = %data, "Structured warning");
    }

    #[allow(dead_code)]
    pub(super) fn error(&self, event: &str, data: serde_json::Value) {
        tracing::error!(event = event, data = %data, "Structured error");
    }
}

impl SimpleMetrics {
    pub(super) fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            gauges: Arc::new(Mutex::new(HashMap::new())),
            histograms: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    pub(super) fn increment(&self, metric: &str) {
        if let Ok(mut counters) = self.counters.lock() {
            *counters.entry(metric.to_string()).or_insert(0) += 1;
        }
    }

    pub(super) fn register_counter(&self, metric: &str) {
        if let Ok(mut counters) = self.counters.lock() {
            counters.entry(metric.to_string()).or_insert(0);
        }
    }

    pub(super) fn register_gauge(&self, metric: &str) {
        if let Ok(mut gauges) = self.gauges.lock() {
            gauges.entry(metric.to_string()).or_insert(0.0);
        }
    }

    pub(super) fn observe(&self, metric: &str, value: f64) {
        if let Ok(mut gauges) = self.gauges.lock() {
            gauges.insert(metric.to_string(), value);
        }
    }

    pub(super) fn observe_histogram(&self, metric: &str, value: f64) {
        const MAX_SAMPLES: usize = 2000;

        if let Ok(mut histograms) = self.histograms.lock() {
            let values = histograms.entry(metric.to_string()).or_default();
            values.push(value);
            if values.len() > MAX_SAMPLES {
                let overflow = values.len() - MAX_SAMPLES;
                values.drain(0..overflow);
            }
        }
    }

    pub(super) fn register_histogram(&self, metric: &str) {
        if let Ok(mut histograms) = self.histograms.lock() {
            histograms.entry(metric.to_string()).or_default();
        }
    }

    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn get_counter(&self, metric: &str) -> u64 {
        self.counters
            .lock()
            .ok()
            .and_then(|counters| counters.get(metric).copied())
            .unwrap_or(0)
    }

    pub fn get_gauge(&self, metric: &str) -> f64 {
        self.gauges
            .lock()
            .ok()
            .and_then(|gauges| gauges.get(metric).copied())
            .unwrap_or(0.0)
    }

    pub(super) fn add_counter(&self, metric: &str, value: u64) {
        if let Ok(mut counters) = self.counters.lock() {
            *counters.entry(metric.to_string()).or_insert(0) += value;
        }
    }

    fn clone_counters(&self) -> HashMap<String, u64> {
        self.counters
            .lock()
            .map(|c| c.clone())
            .unwrap_or_else(|_| HashMap::new())
    }

    fn clone_gauges(&self) -> HashMap<String, f64> {
        self.gauges
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| HashMap::new())
    }

    fn clone_histograms(&self) -> HashMap<String, Vec<f64>> {
        self.histograms
            .lock()
            .map(|h| h.clone())
            .unwrap_or_else(|_| HashMap::new())
    }

    fn clone_histograms_by_name(&self, names: &[&str]) -> HashMap<String, Vec<f64>> {
        self.histograms
            .lock()
            .map(|histograms| {
                names.iter()
                    .filter_map(|name| {
                        histograms
                            .get(*name)
                            .cloned()
                            .map(|values| ((*name).to_string(), values))
                    })
                    .collect()
            })
            .unwrap_or_else(|_| HashMap::new())
    }

    fn build_histogram_stats(
        histograms: HashMap<String, Vec<f64>>,
    ) -> HashMap<String, serde_json::Value> {
        let mut histogram_stats = HashMap::new();
        for (name, mut values) in histograms {
            if values.is_empty() {
                histogram_stats.insert(
                    name,
                    json!({
                        "count": 0,
                        "p50": 0.0,
                        "p95": 0.0,
                        "p99": 0.0
                    }),
                );
                continue;
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let count = values.len();
            let p50 = percentile_sorted(&values, 0.50);
            let p95 = percentile_sorted(&values, 0.95);
            let p99 = percentile_sorted(&values, 0.99);
            histogram_stats.insert(
                name,
                json!({
                    "count": count,
                    "p50": p50,
                    "p95": p95,
                    "p99": p99
                }),
            );
        }
        histogram_stats
    }

    fn build_rates(counters: &HashMap<String, u64>) -> HashMap<String, f64> {
        let mut rates = HashMap::new();
        if let Some(rate) =
            compute_rate(counters, "completion_incomplete_total", "completion_total")
        {
            rates.insert("completion_incomplete_rate".to_string(), rate);
        }
        if let Some(rate) = compute_rate(counters, "completion_error_total", "completion_total") {
            rates.insert("completion_error_rate".to_string(), rate);
        }
        if let Some(rate) = compute_rate(
            counters,
            "signature_help_empty_total",
            "signature_help_total",
        ) {
            rates.insert("signature_help_empty_rate".to_string(), rate);
        }
        let parse_result_leader = sum_counters_with_all_substrings(
            counters,
            &[
                "intellisense_v2_drilldown_singleflight_effectiveness_total",
                "_outcome_leader_",
                "_query_kind_parse_result",
            ],
        );
        let parse_result_shared = sum_counters_with_all_substrings(
            counters,
            &[
                "intellisense_v2_drilldown_singleflight_effectiveness_total",
                "_outcome_shared_",
                "_query_kind_parse_result",
            ],
        );
        let parse_result_singleflight_total = parse_result_leader + parse_result_shared;
        if parse_result_singleflight_total > 0 {
            rates.insert(
                "intellisense_v2_parse_result_singleflight_shared_rate".to_string(),
                parse_result_shared as f64 / parse_result_singleflight_total as f64,
            );
        }
        let parse_result_cancelled = sum_counters_with_all_substrings(
            counters,
            &[
                "intellisense_v2_drilldown_stage_reason_total",
                "_stage_parse_result_query_",
                "_reason_other",
            ],
        );
        if let Some(parse_total) = counters.get("intellisense_v2_parse_result_query_total") {
            if *parse_total > 0 {
                rates.insert(
                    "intellisense_v2_parse_result_query_cancel_rate".to_string(),
                    parse_result_cancelled as f64 / *parse_total as f64,
                );
            }
        }
        rates
    }

    fn build_metrics_payload(
        &self,
        counters: HashMap<String, u64>,
        gauges: HashMap<String, f64>,
        histogram_stats: HashMap<String, serde_json::Value>,
    ) -> serde_json::Value {
        let rates = Self::build_rates(&counters);
        json!({
            "counters": counters,
            "gauges": gauges,
            "histograms": histogram_stats,
            "rates": rates,
            "uptime_seconds": self.uptime().as_secs()
        })
    }

    /// Экспорт всех метрик (для health endpoints)
    pub fn export_metrics(&self) -> serde_json::Value {
        let counters = self.clone_counters();
        let gauges = self.clone_gauges();
        let histograms = self.clone_histograms();
        let histogram_stats = Self::build_histogram_stats(histograms);
        self.build_metrics_payload(counters, gauges, histogram_stats)
    }

    /// Экспорт lightweight snapshot для sidebar polling.
    pub fn export_metrics_sidebar(&self) -> serde_json::Value {
        let counters = self.clone_counters();
        let gauges = self.clone_gauges();
        let histograms = self.clone_histograms_by_name(SIDEBAR_HISTOGRAM_KEYS);
        let histogram_stats = Self::build_histogram_stats(histograms);
        self.build_metrics_payload(counters, gauges, histogram_stats)
    }
}
