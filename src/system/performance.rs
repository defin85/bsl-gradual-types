//! Модуль профилирования и оптимизации производительности
//!
//! Предоставляет инструменты для измерения и оптимизации производительности
//! различных компонентов системы типов.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Метрики производительности для компонента
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    /// Название компонента
    pub name: String,
    /// Общее время выполнения
    pub total_time: Duration,
    /// Количество вызовов
    pub call_count: usize,
    /// Среднее время выполнения
    pub avg_time: Duration,
    /// Минимальное время
    pub min_time: Duration,
    /// Максимальное время
    pub max_time: Duration,
    /// Последние 10 измерений
    pub recent_times: Vec<Duration>,
}

impl ComponentMetrics {
    pub fn new(name: String) -> Self {
        Self {
            name,
            total_time: Duration::ZERO,
            call_count: 0,
            avg_time: Duration::ZERO,
            min_time: Duration::MAX,
            max_time: Duration::ZERO,
            recent_times: Vec::new(),
        }
    }

    /// Добавить новое измерение
    pub fn add_measurement(&mut self, time: Duration) {
        self.total_time += time;
        self.call_count += 1;

        // Обновляем min/max
        if time < self.min_time {
            self.min_time = time;
        }
        if time > self.max_time {
            self.max_time = time;
        }

        // Пересчитываем среднее
        self.avg_time = self.total_time / self.call_count as u32;

        // Обновляем последние измерения
        self.recent_times.push(time);
        if self.recent_times.len() > 10 {
            self.recent_times.remove(0);
        }
    }

    /// Получить процентиль времени выполнения
    pub fn get_percentile(&self, percentile: f64) -> Duration {
        if self.recent_times.is_empty() {
            return Duration::ZERO;
        }

        let mut sorted_times = self.recent_times.clone();
        sorted_times.sort();

        let index = ((sorted_times.len() - 1) as f64 * percentile / 100.0) as usize;
        sorted_times[index]
    }
}

/// Глобальный профайлер производительности
pub struct PerformanceProfiler {
    /// Метрики для различных компонентов
    metrics: HashMap<String, ComponentMetrics>,
    /// Включен ли профайлер
    enabled: bool,
    /// Время начала сессии профилирования
    session_start: Instant,
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceProfiler {
    /// Создать новый профайлер
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            enabled: false,
            session_start: Instant::now(),
        }
    }

    /// Включить профилирование
    pub fn enable(&mut self) {
        self.enabled = true;
        self.session_start = Instant::now();
    }

    /// Выключить профилирование
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Измерить время выполнения блока кода
    pub fn measure<F, R>(&mut self, component: &str, func: F) -> R
    where
        F: FnOnce() -> R,
    {
        if !self.enabled {
            return func();
        }

        let start = Instant::now();
        let result = func();
        let elapsed = start.elapsed();

        // Добавляем измерение
        let metrics = self
            .metrics
            .entry(component.to_string())
            .or_insert_with(|| ComponentMetrics::new(component.to_string()));
        metrics.add_measurement(elapsed);

        result
    }

    /// Создать таймер для ручного измерения
    pub fn start_timer(&self, component: &str) -> PerformanceTimer {
        PerformanceTimer {
            component: component.to_string(),
            start_time: Instant::now(),
            enabled: self.enabled,
        }
    }

    /// Получить метрики для компонента
    pub fn get_metrics(&self, component: &str) -> Option<&ComponentMetrics> {
        self.metrics.get(component)
    }

    /// Получить все метрики
    pub fn get_all_metrics(&self) -> &HashMap<String, ComponentMetrics> {
        &self.metrics
    }

    /// Сгенерировать отчет о производительности
    pub fn generate_report(&self) -> PerformanceReport {
        let mut total_time = Duration::ZERO;
        let mut total_calls = 0;

        for metrics in self.metrics.values() {
            total_time += metrics.total_time;
            total_calls += metrics.call_count;
        }

        PerformanceReport {
            session_duration: self.session_start.elapsed(),
            total_analysis_time: total_time,
            total_calls,
            avg_call_time: if total_calls > 0 {
                total_time / total_calls as u32
            } else {
                Duration::ZERO
            },
            components: self.metrics.clone(),
            slowest_components: self.get_slowest_components(5),
            most_called_components: self.get_most_called_components(5),
        }
    }

    /// Получить самые медленные компоненты
    fn get_slowest_components(&self, limit: usize) -> Vec<(String, Duration)> {
        let mut components: Vec<_> = self
            .metrics
            .iter()
            .map(|(name, metrics)| (name.clone(), metrics.avg_time))
            .collect();

        components.sort_by(|a, b| b.1.cmp(&a.1));
        components.truncate(limit);
        components
    }

    /// Получить самые вызываемые компоненты
    fn get_most_called_components(&self, limit: usize) -> Vec<(String, usize)> {
        let mut components: Vec<_> = self
            .metrics
            .iter()
            .map(|(name, metrics)| (name.clone(), metrics.call_count))
            .collect();

        components.sort_by(|a, b| b.1.cmp(&a.1));
        components.truncate(limit);
        components
    }

    /// Очистить все метрики
    pub fn reset(&mut self) {
        self.metrics.clear();
        self.session_start = Instant::now();
    }

    /// Экспортировать метрики в JSON
    pub fn export_json(&self) -> anyhow::Result<String> {
        let report = self.generate_report();
        Ok(serde_json::to_string_pretty(&report)?)
    }
}

/// Таймер для ручного измерения производительности
pub struct PerformanceTimer {
    component: String,
    start_time: Instant,
    enabled: bool,
}

impl PerformanceTimer {
    /// Завершить измерение и записать результат
    pub fn finish(self, profiler: &mut PerformanceProfiler) {
        if !self.enabled {
            return;
        }

        let elapsed = self.start_time.elapsed();
        let metrics = profiler
            .metrics
            .entry(self.component.clone())
            .or_insert_with(|| ComponentMetrics::new(self.component));
        metrics.add_measurement(elapsed);
    }
}

/// Отчет о производительности
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Длительность сессии профилирования
    pub session_duration: Duration,
    /// Общее время анализа
    pub total_analysis_time: Duration,
    /// Общее количество вызовов
    pub total_calls: usize,
    /// Среднее время вызова
    pub avg_call_time: Duration,
    /// Метрики по компонентам
    pub components: HashMap<String, ComponentMetrics>,
    /// Самые медленные компоненты
    pub slowest_components: Vec<(String, Duration)>,
    /// Самые вызываемые компоненты
    pub most_called_components: Vec<(String, usize)>,
}

impl PerformanceReport {
    /// Форматировать отчет в человекочитаемый вид
    pub fn format_human_readable(&self) -> String {
        let mut report = String::new();

        report.push_str("🔍 Отчет о производительности BSL Type System\n");
        report.push_str(&format!("📊 Сессия: {:.2?}\n", self.session_duration));
        report.push_str(&format!(
            "⏱️  Общее время анализа: {:.2?}\n",
            self.total_analysis_time
        ));
        report.push_str(&format!(
            "🔢 Общее количество вызовов: {}\n",
            self.total_calls
        ));
        report.push_str(&format!(
            "📈 Среднее время вызова: {:.2?}\n\n",
            self.avg_call_time
        ));

        report.push_str("🐌 Самые медленные компоненты:\n");
        for (i, (name, time)) in self.slowest_components.iter().enumerate() {
            report.push_str(&format!("  {}. {} - {:.2?}\n", i + 1, name, time));
        }

        report.push_str("\n🔥 Самые вызываемые компоненты:\n");
        for (i, (name, count)) in self.most_called_components.iter().enumerate() {
            report.push_str(&format!("  {}. {} - {} вызовов\n", i + 1, name, count));
        }

        report.push_str("\n📋 Детальные метрики:\n");
        for (name, metrics) in &self.components {
            report.push_str(&format!(
                "  • {}: {:.2?} avg, {} calls, {:.2?}-{:.2?} range\n",
                name, metrics.avg_time, metrics.call_count, metrics.min_time, metrics.max_time
            ));
        }

        report
    }
}

/// Глобальный экземпляр профайлера
static mut GLOBAL_PROFILER: Option<PerformanceProfiler> = None;
static PROFILER_INIT: std::sync::Once = std::sync::Once::new();

/// Получить глобальный профайлер
#[allow(static_mut_refs)]
pub fn global_profiler() -> &'static mut PerformanceProfiler {
    unsafe {
        PROFILER_INIT.call_once(|| {
            GLOBAL_PROFILER = Some(PerformanceProfiler::new());
        });
        GLOBAL_PROFILER.as_mut().unwrap()
    }
}

/// Макрос для удобного профилирования
#[macro_export]
macro_rules! profile {
    ($component:expr, $block:block) => {{
        let mut profiler = $crate::system::performance::global_profiler();
        profiler.measure($component, || $block)
    }};
}

/// Макрос для профилирования async блоков
#[macro_export]
macro_rules! profile_async {
    ($component:expr, $block:expr) => {{
        let timer = $crate::system::performance::global_profiler().start_timer($component);
        let result = $block.await;
        timer.finish($crate::system::performance::global_profiler());
        result
    }};
}

/// Оптимизатор производительности
pub struct PerformanceOptimizer;

impl PerformanceOptimizer {
    /// Проанализировать метрики и дать рекомендации
    pub fn analyze_and_suggest(report: &PerformanceReport) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Проверяем медленные компоненты
        for (component, avg_time) in &report.slowest_components {
            if avg_time.as_millis() > 100 {
                suggestions.push(OptimizationSuggestion {
                    component: component.clone(),
                    issue: "Медленное выполнение".to_string(),
                    suggestion: format!(
                        "Компонент {} выполняется {:.2?} в среднем. Рассмотрите оптимизацию алгоритма или кеширование.",
                        component, avg_time
                    ),
                    priority: OptimizationPriority::High,
                });
            }
        }

        // Проверяем часто вызываемые компоненты
        for (component, call_count) in &report.most_called_components {
            if *call_count > 1000 {
                if let Some(metrics) = report.components.get(component) {
                    if metrics.avg_time.as_millis() > 10 {
                        suggestions.push(OptimizationSuggestion {
                            component: component.clone(),
                            issue: "Частые вызовы медленного компонента".to_string(),
                            suggestion: format!(
                                "Компонент {} вызывается {} раз со средним временем {:.2?}. Добавьте кеширование.",
                                component, call_count, metrics.avg_time
                            ),
                            priority: OptimizationPriority::Medium,
                        });
                    }
                }
            }
        }

        // Проверяем общую производительность
        if report.avg_call_time.as_millis() > 50 {
            suggestions.push(OptimizationSuggestion {
                component: "Общая система".to_string(),
                issue: "Общая медленность системы".to_string(),
                suggestion: format!(
                    "Среднее время вызова {:.2?} превышает рекомендуемое (50ms). Рассмотрите общую оптимизацию.",
                    report.avg_call_time
                ),
                priority: OptimizationPriority::Medium,
            });
        }

        suggestions
    }

    /// Получить рекомендации по кешированию
    pub fn cache_recommendations(report: &PerformanceReport) -> Vec<CacheRecommendation> {
        let mut recommendations = Vec::new();

        for (component, metrics) in &report.components {
            // Рекомендуем кеширование для медленных и часто вызываемых компонентов
            if metrics.call_count > 100 && metrics.avg_time.as_millis() > 20 {
                recommendations.push(CacheRecommendation {
                    component: component.clone(),
                    strategy: CacheStrategy::LRU,
                    estimated_speedup: format!("{}x", metrics.call_count / 10),
                    reason: format!(
                        "{} вызовов со временем {:.2?}",
                        metrics.call_count, metrics.avg_time
                    ),
                });
            }
        }

        recommendations
    }
}

/// Рекомендация по оптимизации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub component: String,
    pub issue: String,
    pub suggestion: String,
    pub priority: OptimizationPriority,
}

/// Приоритет оптимизации
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Рекомендация по кешированию
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheRecommendation {
    pub component: String,
    pub strategy: CacheStrategy,
    pub estimated_speedup: String,
    pub reason: String,
}

/// Стратегия кеширования
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheStrategy {
    LRU,          // Least Recently Used
    TTL,          // Time To Live
    WriteThrough, // Write-through cache
    WriteBack,    // Write-back cache
}

/// Бенчмарки для различных компонентов
pub struct BenchmarkSuite;

impl BenchmarkSuite {
    /// Запустить бенчмарк парсинга
    pub fn benchmark_parsing(source_code: &str, iterations: usize) -> ComponentMetrics {
        let mut metrics = ComponentMetrics::new("parsing".to_string());

        for _ in 0..iterations {
            let start = Instant::now();

            let mut parser = crate::parsing::bsl::common::ParserFactory::create();
            let _result = parser.parse(source_code);

            metrics.add_measurement(start.elapsed());
        }

        metrics
    }

    /// Запустить бенчмарк type checking
    pub fn benchmark_type_checking(
        program: &crate::parsing::bsl::ast::Program,
        iterations: usize,
    ) -> ComponentMetrics {
        let mut metrics = ComponentMetrics::new("type_checking".to_string());

        for _ in 0..iterations {
            let start = Instant::now();

            let type_checker =
                crate::domain::analysis::type_checker::TypeChecker::new("benchmark.bsl".to_string());
            let _result = type_checker.check(program);

            metrics.add_measurement(start.elapsed());
        }

        metrics
    }

    /// Запустить бенчмарк flow-sensitive анализа
    pub fn benchmark_flow_analysis(
        program: &crate::parsing::bsl::ast::Program,
        iterations: usize,
    ) -> ComponentMetrics {
        let mut metrics = ComponentMetrics::new("flow_analysis".to_string());

        for _ in 0..iterations {
            let start = Instant::now();

            // Создаем базовый контекст
            let context = crate::domain::analysis::type_checker::TypeContext {
                variables: HashMap::new(),
                functions: HashMap::new(),
                current_scope: crate::domain::analysis::dependency_graph::Scope::Global,
                scope_stack: vec![],
            };

            let mut analyzer = crate::domain::analysis::flow_sensitive::FlowSensitiveAnalyzer::new(context);

            // Анализируем все statements
            for statement in &program.statements {
                analyzer.analyze_statement(statement);
            }

            metrics.add_measurement(start.elapsed());
        }

        metrics
    }

    /// Запустить полный набор бенчмарков
    pub fn run_full_benchmark_suite() -> PerformanceReport {
        let mut profiler = PerformanceProfiler::new();
        profiler.enable();

        // Тестовый код для бенчмарков
        let test_code = r#"
            Функция ТестоваяФункция(параметр1, параметр2)
                Если параметр1 > 0 Тогда
                    результат = параметр1 + параметр2;
                    Возврат Строка(результат);
                Иначе
                    Возврат "ошибка";
                КонецЕсли;
            КонецФункции
            
            Процедура ТестоваяПроцедура()
                Для сч = 1 По 10 Цикл
                    значение = ТестоваяФункция(сч, сч * 2);
                    Сообщить(значение);
                КонецЦикла;
            КонецПроцедуры
        "#;

        // Парсинг
        let parsing_metrics = Self::benchmark_parsing(test_code, 50);

        // Парсим один раз для получения AST
        let mut parser = crate::parsing::bsl::common::ParserFactory::create();
        if let Ok(program) = parser.parse(test_code) {
            // Type checking
            let type_checking_metrics = Self::benchmark_type_checking(&program, 20);

            // Flow analysis
            let flow_metrics = Self::benchmark_flow_analysis(&program, 20);

            // Добавляем метрики в профайлер
            profiler
                .metrics
                .insert("parsing".to_string(), parsing_metrics);
            profiler
                .metrics
                .insert("type_checking".to_string(), type_checking_metrics);
            profiler
                .metrics
                .insert("flow_analysis".to_string(), flow_metrics);
        }

        profiler.generate_report()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_metrics() {
        let mut metrics = ComponentMetrics::new("test".to_string());

        metrics.add_measurement(Duration::from_millis(10));
        metrics.add_measurement(Duration::from_millis(20));
        metrics.add_measurement(Duration::from_millis(15));

        assert_eq!(metrics.call_count, 3);
        assert_eq!(metrics.avg_time, Duration::from_millis(15));
        assert_eq!(metrics.min_time, Duration::from_millis(10));
        assert_eq!(metrics.max_time, Duration::from_millis(20));
    }

    #[test]
    fn test_performance_profiler() {
        let mut profiler = PerformanceProfiler::new();
        profiler.enable();

        let result = profiler.measure("test_component", || {
            std::thread::sleep(Duration::from_millis(1));
            42
        });

        assert_eq!(result, 42);

        let metrics = profiler.get_metrics("test_component").unwrap();
        assert_eq!(metrics.call_count, 1);
        assert!(metrics.avg_time.as_millis() >= 1);
    }

    #[test]
    fn test_performance_timer() {
        let mut profiler = PerformanceProfiler::new();
        profiler.enable();

        let timer = profiler.start_timer("manual_test");
        std::thread::sleep(Duration::from_millis(1));
        timer.finish(&mut profiler);

        let metrics = profiler.get_metrics("manual_test").unwrap();
        assert_eq!(metrics.call_count, 1);
    }

    #[test]
    fn test_benchmark_suite() {
        let simple_code = "Процедура Тест() КонецПроцедуры";
        let metrics = BenchmarkSuite::benchmark_parsing(simple_code, 5);

        assert_eq!(metrics.call_count, 5);
        assert!(metrics.avg_time.as_nanos() > 0);
    }

    #[test]
    fn test_optimization_suggestions() {
        // Создаем отчет с медленным компонентом
        let mut components = HashMap::new();
        let mut slow_metrics = ComponentMetrics::new("slow_component".to_string());
        slow_metrics.add_measurement(Duration::from_millis(200));
        components.insert("slow_component".to_string(), slow_metrics);

        let report = PerformanceReport {
            session_duration: Duration::from_secs(60),
            total_analysis_time: Duration::from_millis(200),
            total_calls: 1,
            avg_call_time: Duration::from_millis(200),
            components,
            slowest_components: vec![("slow_component".to_string(), Duration::from_millis(200))],
            most_called_components: vec![],
        };

        let suggestions = PerformanceOptimizer::analyze_and_suggest(&report);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].priority, OptimizationPriority::High);
    }
}
