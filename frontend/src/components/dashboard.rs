//! Dashboard component with metrics and overview

use crate::api::*;
use leptos::prelude::*;

/// Helper function for metric card styling variants
fn metric_card_classes(variant: &str) -> String {
    let base = "bg-white dark:bg-gray-900 rounded-xl p-6 shadow-sm hover:shadow-md transition-all duration-200";
    let border = match variant {
        "success" => "border border-green-200 dark:border-green-800",
        "warning" => "border border-amber-200 dark:border-amber-800",
        "danger" => "border border-red-200 dark:border-red-800",
        "info" => "border border-blue-200 dark:border-blue-800",
        _ => "border border-gray-200 dark:border-gray-800",
    };
    format!("{} {}", base, border)
}

/// Helper function for metric card icon styling
fn metric_icon_classes(variant: &str) -> String {
    let base = "text-4xl mb-3 flex items-center justify-center w-16 h-16 rounded-lg";
    let color = match variant {
        "success" => "bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400",
        "warning" => "bg-amber-50 dark:bg-amber-900/20 text-amber-600 dark:text-amber-400",
        "danger" => "bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400",
        "info" => "bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400",
        "performance" => "bg-purple-50 dark:bg-purple-900/20 text-purple-600 dark:text-purple-400",
        "categories" => "bg-indigo-50 dark:bg-indigo-900/20 text-indigo-600 dark:text-indigo-400",
        "speed" => "bg-cyan-50 dark:bg-cyan-900/20 text-cyan-600 dark:text-cyan-400",
        _ => "bg-gray-50 dark:bg-gray-800 text-gray-600 dark:text-gray-400",
    };
    format!("{} {}", base, color)
}

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
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5 mb-8">
            // Total types metric
            <div class={metric_card_classes("default")}>
                <div class={metric_icon_classes("default")}>"📈"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-gray-600 dark:text-gray-400 mb-2">"Всего типов"</h3>
                    <div class="text-3xl font-bold text-gray-900 dark:text-white mb-1">
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
                    <div class="text-xs text-gray-500 dark:text-gray-400">"активных в системе"</div>
                </div>
            </div>

            // Known types metric
            <div class={metric_card_classes("success")}>
                <div class={metric_icon_classes("success")}>"✅"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-gray-600 dark:text-gray-400 mb-2">"Известные типы"</h3>
                    <div class="text-3xl font-bold text-gray-900 dark:text-white mb-1">
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
                    <div class="text-xs text-gray-500 dark:text-gray-400">"100% определенность"</div>
                </div>
            </div>

            // Inferred types metric
            <div class={metric_card_classes("warning")}>
                <div class={metric_icon_classes("warning")}>"🔍"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-gray-600 dark:text-gray-400 mb-2">"Выведенные типы"</h3>
                    <div class="text-3xl font-bold text-gray-900 dark:text-white mb-1">
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
                    <div class="text-xs text-gray-500 dark:text-gray-400">"анализ потоков"</div>
                </div>
            </div>

            // Unknown types metric
            <div class={metric_card_classes("danger")}>
                <div class={metric_icon_classes("danger")}>"❓"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-gray-600 dark:text-gray-400 mb-2">"Неизвестные типы"</h3>
                    <div class="text-3xl font-bold text-gray-900 dark:text-white mb-1">
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
                    <div class="text-xs text-gray-500 dark:text-gray-400">"требуют анализа"</div>
                </div>
            </div>

            // Flow-sensitive types metric
            <div class={metric_card_classes("info")}>
                <div class={metric_icon_classes("info")}>"🔄"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-gray-600 dark:text-gray-400 mb-2">"Flow-sensitive"</h3>
                    <div class="text-3xl font-bold text-gray-900 dark:text-white mb-1">
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
                    <div class="text-xs text-gray-500 dark:text-gray-400">"динамических типов"</div>
                </div>
            </div>

            // Performance metrics
            <div class={metric_card_classes("default")}>
                <div class={metric_icon_classes("performance")}>"⚡"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-gray-600 dark:text-gray-400 mb-2">"Производительность"</h3>
                    <div class="text-3xl font-bold text-gray-900 dark:text-white mb-1">
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
                    <div class="text-xs text-gray-500 dark:text-gray-400">"попаданий в кеш"</div>
                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
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
            <div class={metric_card_classes("default")}>
                <div class={metric_icon_classes("categories")}>"📊"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-gray-600 dark:text-gray-400 mb-2">"Категории"</h3>
                    <div class="flex flex-col gap-2">
                        {move || {
                            if let Some(result) = search_result.get() {
                                result.categories.iter().map(|(category, info)| {
                                    view! {
                                        <div class="flex items-center gap-3 p-2 bg-gray-50 dark:bg-gray-800 rounded">
                                            <span class="text-lg">{info.icon.clone()}</span>
                                            <span class="flex-1 font-medium text-gray-700 dark:text-gray-300">{category.clone()}</span>
                                            <span class="font-bold text-blue-600 dark:text-blue-400">{info.count.to_string()}</span>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            } else {
                                view! {
                                    <div class="flex items-center gap-3 p-2 bg-gray-50 dark:bg-gray-800 rounded">
                                        <span class="text-lg">"🔧"</span>
                                        <span class="flex-1 font-medium text-gray-700 dark:text-gray-300">"Platform"</span>
                                        <span class="font-bold text-blue-600 dark:text-blue-400">"5"</span>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>

            // Analysis speed
            <div class={metric_card_classes("default")}>
                <div class={metric_icon_classes("speed")}>"🚀"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-gray-600 dark:text-gray-400 mb-2">"Скорость анализа"</h3>
                    <div class="text-3xl font-bold text-gray-900 dark:text-white mb-1">
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
                    <div class="text-xs text-gray-500 dark:text-gray-400">"среднее время"</div>
                </div>
            </div>
        </div>
    }
}
