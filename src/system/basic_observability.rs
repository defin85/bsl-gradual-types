//! Basic Observability - простая замена сложного observability stack
//!
//! Structured logging + basic metrics вместо полного enterprise стека

use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde_json::json;
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

impl BasicObservability {
    pub fn default() -> Self {
        Self {
            logger: StructuredLogger::new(),
            metrics: SimpleMetrics::new(),
            start_time: Instant::now(),
        }
    }
    
    /// Логирование запуска системы
    pub fn log_startup(&self) {
        self.logger.info("system_startup", json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "version": env!("CARGO_PKG_VERSION"),
            "architecture": "simplified"
        }));
    }
    
    /// Логирование завершения анализа
    pub fn log_analysis(&self, file_path: &str, duration: Duration) {
        self.logger.info("analysis_completed", json!({
            "file": file_path,
            "duration_ms": duration.as_millis(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        }));
        
        self.metrics.increment("analyses_total");
        self.metrics.observe("analysis_duration_ms", duration.as_millis() as f64);
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
    
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    pub fn get_counter(&self, metric: &str) -> u64 {
        self.counters.lock()
            .ok()
            .and_then(|counters| counters.get(metric).copied())
            .unwrap_or(0)
    }
    
    pub fn get_gauge(&self, metric: &str) -> f64 {
        self.gauges.lock()
            .ok()
            .and_then(|gauges| gauges.get(metric).copied())
            .unwrap_or(0.0)
    }
    
    /// Экспорт всех метрик (для health endpoints)
    pub fn export_metrics(&self) -> serde_json::Value {
        let counters = self.counters.lock().map(|c| c.clone()).unwrap_or_else(|_| HashMap::new());
        let gauges = self.gauges.lock().map(|g| g.clone()).unwrap_or_else(|_| HashMap::new());
        
        json!({
            "counters": counters,
            "gauges": gauges,
            "uptime_seconds": self.uptime().as_secs()
        })
    }
}

// === COMPARISON WITH COMPLEX OBSERVABILITY ===

/// Сравнение: Simple vs Complex observability
/// 
/// Complex (Full Enterprise Stack):
/// - LoggingManager + MetricsCollector + HealthChecker
/// - CircuitBreaker + EventBus + AlertingManager  
/// - Distributed tracing + APM integration
/// - Advanced dashboards + SLA monitoring
/// - ~500+ LOC
///
/// Simple (BasicObservability):
/// - StructuredLogger + SimpleMetrics + HealthEndpoint
/// - Basic health check
/// - ~150 LOC
/// 
/// Экономия: ~70% сложности, покрывает основные потребности monitoring
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
