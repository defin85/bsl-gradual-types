//! Dashboard component with metrics and overview

use crate::api::*;
use leptos::prelude::*;

/// Dashboard with overview metrics
#[component]
#[allow(non_snake_case)]
pub fn Dashboard(
    /// Type metrics signal
    metrics: Signal<Option<TypeMetrics>>,
    /// Search result signal
    search_result: Signal<Option<AnalysisResultDto>>,
) -> impl IntoView {
    view! {
        <div class="dashboard-grid">
            // Total types metric
            <div class="metric-card">
                <div class="metric-card__icon">"📈"</div>
                <div class="metric-card__content">
                    <h3 class="metric-card__title">"Всего типов"</h3>
                    <div class="metric-card__value">
                        {move || {
                            if let Some(result) = search_result.get() {
                                result.metrics.total_types.to_string()
                            } else if let Some(metrics_data) = metrics.get() {
                                metrics_data.total_types.to_string()
                            } else {
                                "0".to_string()
                            }
                        }}
                    </div>
                    <div class="metric-card__subtitle">"активных в системе"</div>
                </div>
            </div>

            // Known types metric
            <div class="metric-card metric-card--success">
                <div class="metric-card__icon">"✅"</div>
                <div class="metric-card__content">
                    <h3 class="metric-card__title">"Известные типы"</h3>
                    <div class="metric-card__value">
                        {move || {
                            if let Some(result) = search_result.get() {
                                result.metrics.certainty_high.to_string()
                            } else if let Some(metrics_data) = metrics.get() {
                                metrics_data.known_types().to_string()
                            } else {
                                "0".to_string()
                            }
                        }}
                    </div>
                    <div class="metric-card__subtitle">"100% определенность"</div>
                </div>
            </div>

            // Inferred types metric
            <div class="metric-card metric-card--warning">
                <div class="metric-card__icon">"🔍"</div>
                <div class="metric-card__content">
                    <h3 class="metric-card__title">"Выведенные типы"</h3>
                    <div class="metric-card__value">
                        {move || {
                            if let Some(result) = search_result.get() {
                                result.metrics.certainty_medium.to_string()
                            } else if let Some(metrics_data) = metrics.get() {
                                metrics_data.inferred_types().to_string()
                            } else {
                                "0".to_string()
                            }
                        }}
                    </div>
                    <div class="metric-card__subtitle">"анализ потоков"</div>
                </div>
            </div>

            // Unknown types metric
            <div class="metric-card metric-card--danger">
                <div class="metric-card__icon">"❓"</div>
                <div class="metric-card__content">
                    <h3 class="metric-card__title">"Неизвестные типы"</h3>
                    <div class="metric-card__value">
                        {move || {
                            if let Some(result) = search_result.get() {
                                result.metrics.certainty_low.to_string()
                            } else if let Some(metrics_data) = metrics.get() {
                                metrics_data.unknown_types().to_string()
                            } else {
                                "0".to_string()
                            }
                        }}
                    </div>
                    <div class="metric-card__subtitle">"требуют анализа"</div>
                </div>
            </div>

            // Flow-sensitive types metric
            <div class="metric-card metric-card--info">
                <div class="metric-card__icon">"🔄"</div>
                <div class="metric-card__content">
                    <h3 class="metric-card__title">"Flow-sensitive"</h3>
                    <div class="metric-card__value">
                        {move || {
                            if let Some(result) = search_result.get() {
                                result.metrics.flow_sensitive.to_string()
                            } else if let Some(metrics_data) = metrics.get() {
                                metrics_data.flow_sensitive_types().to_string()
                            } else {
                                "0".to_string()
                            }
                        }}
                    </div>
                    <div class="metric-card__subtitle">"динамических типов"</div>
                </div>
            </div>

            // Performance metrics
            <div class="metric-card metric-card--performance">
                <div class="metric-card__icon">"⚡"</div>
                <div class="metric-card__content">
                    <h3 class="metric-card__title">"Производительность"</h3>
                    <div class="metric-card__value">
                        {move || {
                            if let Some(result) = search_result.get() {
                                result.metrics.cache_hit_rate.clone()
                            } else if let Some(metrics_data) = metrics.get() {
                                metrics_data.cache_hit_rate.clone()
                            } else {
                                "N/A".to_string()
                            }
                        }}
                    </div>
                    <div class="metric-card__subtitle">"попаданий в кеш"</div>
                    <div class="metric-card__extra">
                        {move || {
                            if let Some(metrics_data) = metrics.get() {
                                format!("Скорость: {:.0}ms", metrics_data.analysis_speed_ms())
                            } else {
                                "".to_string()
                            }
                        }}
                    </div>
                </div>
            </div>

            // Categories overview
            <div class="metric-card metric-card--categories">
                <div class="metric-card__icon">"📊"</div>
                <div class="metric-card__content">
                    <h3 class="metric-card__title">"Категории"</h3>
                    <div class="category-distribution">
                        {move || {
                            if let Some(result) = search_result.get() {
                                result.categories.iter().map(|(category, info)| {
                                    view! {
                                        <div class="category-item">
                                            <span class="category-icon">{info.icon.clone()}</span>
                                            <span class="category-name">{category.clone()}</span>
                                            <span class="category-count">{info.count.to_string()}</span>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            } else {
                                view! {
                                    <div class="category-item">
                                        <span class="category-icon">"🔧"</span>
                                        <span class="category-name">"Platform"</span>
                                        <span class="category-count">"5"</span>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>

            // Analysis speed
            <div class="metric-card metric-card--speed">
                <div class="metric-card__icon">"🚀"</div>
                <div class="metric-card__content">
                    <h3 class="metric-card__title">"Скорость анализа"</h3>
                    <div class="metric-card__value">
                        {move || {
                            if let Some(result) = search_result.get() {
                                result.metrics.analysis_speed.clone()
                            } else if let Some(metrics_data) = metrics.get() {
                                format!("{:.0}ms", metrics_data.analysis_speed_ms())
                            } else {
                                "125ms".to_string()
                            }
                        }}
                    </div>
                    <div class="metric-card__subtitle">"среднее время"</div>
                </div>
            </div>
        </div>
    }
}
