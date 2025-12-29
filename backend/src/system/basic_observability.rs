//! Basic Observability - простая замена сложного observability stack
//!
//! Structured logging + basic metrics вместо полного enterprise стека

use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Простая система наблюдения
pub struct BasicObservability {
    logger: StructuredLogger,
    metrics: SimpleMetrics,
    start_time: Instant,
}

/// Структурированные логи
pub struct StructuredLogger {
    // Wrapper для tracing с JSON форматированием
}

/// Простые метрики
pub struct SimpleMetrics {
    counters: Arc<Mutex<HashMap<String, u64>>>,
    gauges: Arc<Mutex<HashMap<String, f64>>>,
    histograms: Arc<Mutex<HashMap<String, Vec<f64>>>>,
    start_time: Instant,
}

/// Статус здоровья системы
pub struct HealthStatus {
    pub status: String,
    pub uptime: Duration,
    pub components: Vec<ComponentHealth>,
}

/// Здоровье отдельного компонента
pub struct ComponentHealth {
    pub name: String,
    pub status: String,
    pub details: Option<String>,
}

impl Default for BasicObservability {
    fn default() -> Self {
        Self {
            logger: StructuredLogger::new(),
            metrics: SimpleMetrics::new(),
            start_time: Instant::now(),
        }
    }
}

impl BasicObservability {
    /// Логирование запуска системы
    pub fn log_startup(&self) {
        self.logger.info(
            "system_startup",
            json!({
                "timestamp": Utc::now().to_rfc3339(),
                "version": env!("CARGO_PKG_VERSION"),
                "architecture": "simplified"
            }),
        );
    }

    /// Логирование завершения анализа
    pub fn log_analysis(&self, file_path: &str, duration: Duration) {
        self.logger.info(
            "analysis_completed",
            json!({
                "file": file_path,
                "duration_ms": duration.as_millis(),
                "timestamp": Utc::now().to_rfc3339()
            }),
        );

        self.metrics.increment("analyses_total");
        self.metrics
            .observe("analysis_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_index_warmup_hit(&self, duration: Duration) {
        self.metrics.increment("index_warmup_hit_total");
        self.metrics
            .observe("index_warmup_hit_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_index_warmup_miss(&self, duration: Duration) {
        self.metrics.increment("index_warmup_miss_total");
        self.metrics
            .observe("index_warmup_miss_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_index_warmup_skip(&self, reason: &str) {
        self.metrics
            .increment(&format!("index_warmup_skip_total_{}", reason));
    }

    pub fn record_completion_latency(&self, duration: Duration) {
        self.metrics.increment("completion_total");
        self.metrics
            .observe_histogram("completion_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_completion_resolve_latency(&self, duration: Duration) {
        self.metrics.increment("completion_resolve_total");
        self.metrics
            .observe_histogram("completion_resolve_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_signature_help_latency(&self, duration: Duration) {
        self.metrics.increment("signature_help_total");
        self.metrics
            .observe_histogram("signature_help_duration_ms", duration.as_millis() as f64);
    }

    pub fn record_signature_help_empty(&self) {
        self.metrics.increment("signature_help_empty_total");
    }

    pub fn record_completion_incomplete(&self) {
        self.metrics.increment("completion_incomplete_total");
    }

    pub fn record_completion_quality(
        &self,
        total_candidates: usize,
        dedup_removed: usize,
        score_samples: &[f32],
        prefix_exact: usize,
        prefix_starts: usize,
        prefix_contains: usize,
        prefix_none: usize,
        member_access: usize,
        has_owner: usize,
    ) {
        self.metrics
            .add_counter("completion_candidates_total", total_candidates as u64);
        self.metrics.add_counter(
            "completion_dedup_removed_total",
            dedup_removed as u64,
        );
        self.metrics.add_counter("completion_prefix_exact_total", prefix_exact as u64);
        self.metrics.add_counter(
            "completion_prefix_starts_total",
            prefix_starts as u64,
        );
        self.metrics.add_counter(
            "completion_prefix_contains_total",
            prefix_contains as u64,
        );
        self.metrics.add_counter("completion_prefix_none_total", prefix_none as u64);
        self.metrics
            .add_counter("completion_member_access_total", member_access as u64);
        self.metrics
            .add_counter("completion_has_owner_total", has_owner as u64);

        for score in score_samples {
            self.metrics
                .observe_histogram("completion_score", f64::from(*score));
        }
    }

    /// Простая проверка здоровья
    pub fn health_check(&self) -> HealthStatus {
        let uptime = self.start_time.elapsed();

        HealthStatus {
            status: "healthy".to_string(),
            uptime,
            components: vec![
                ComponentHealth {
                    name: "cache".to_string(),
                    status: "operational".to_string(),
                    details: None,
                },
                ComponentHealth {
                    name: "parser".to_string(),
                    status: "operational".to_string(),
                    details: None,
                },
                ComponentHealth {
                    name: "type_resolver".to_string(),
                    status: "operational".to_string(),
                    details: None,
                },
            ],
        }
    }

    /// Получить метрики
    pub fn get_metrics(&self) -> &SimpleMetrics {
        &self.metrics
    }
}

impl StructuredLogger {
    fn new() -> Self {
        Self {}
    }

    fn info(&self, event: &str, data: serde_json::Value) {
        info!(event = event, data = %data, "Structured log entry");
    }

    #[allow(dead_code)]
    fn warn(&self, event: &str, data: serde_json::Value) {
        warn!(event = event, data = %data, "Structured warning");
    }

    #[allow(dead_code)]
    fn error(&self, event: &str, data: serde_json::Value) {
        tracing::error!(event = event, data = %data, "Structured error");
    }
}

impl SimpleMetrics {
    fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            gauges: Arc::new(Mutex::new(HashMap::new())),
            histograms: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    fn increment(&self, metric: &str) {
        if let Ok(mut counters) = self.counters.lock() {
            *counters.entry(metric.to_string()).or_insert(0) += 1;
        }
    }

    fn observe(&self, metric: &str, value: f64) {
        if let Ok(mut gauges) = self.gauges.lock() {
            gauges.insert(metric.to_string(), value);
        }
    }

    fn observe_histogram(&self, metric: &str, value: f64) {
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

    fn add_counter(&self, metric: &str, value: u64) {
        if let Ok(mut counters) = self.counters.lock() {
            *counters.entry(metric.to_string()).or_insert(0) += value;
        }
    }

    /// Экспорт всех метрик (для health endpoints)
    pub fn export_metrics(&self) -> serde_json::Value {
        let counters = self
            .counters
            .lock()
            .map(|c| c.clone())
            .unwrap_or_else(|_| HashMap::new());
        let gauges = self
            .gauges
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| HashMap::new());
        let histograms = self
            .histograms
            .lock()
            .map(|h| h.clone())
            .unwrap_or_else(|_| HashMap::new());

        let mut histogram_stats = HashMap::new();
        for (name, mut values) in histograms {
            if values.is_empty() {
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

        json!({
            "counters": counters,
            "gauges": gauges,
            "histograms": histogram_stats,
            "uptime_seconds": self.uptime().as_secs()
        })
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

// === COMPARISON WITH COMPLEX OBSERVABILITY ===

#[cfg(test)]
mod comparison_notes {
    //! Сравнение: Simple vs Complex observability
    //!
    //! Complex (Full Enterprise Stack):
    //! - LoggingManager + MetricsCollector + HealthChecker
    //! - CircuitBreaker + EventBus + AlertingManager  
    //! - Distributed tracing + APM integration
    //! - Advanced dashboards + SLA monitoring
    //! - ~500+ LOC
    //!
    //! Simple (BasicObservability):
    //! - StructuredLogger + SimpleMetrics + HealthEndpoint
    //! - Basic health check
    //! - ~150 LOC
    //!
    //! Экономия: ~70% сложности, покрывает основные потребности monitoring
}
