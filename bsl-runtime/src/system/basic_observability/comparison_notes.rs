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
