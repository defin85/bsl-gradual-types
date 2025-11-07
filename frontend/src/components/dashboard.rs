//! Dashboard component with metrics and overview
//! Uses BSL design system colors from front_template

use crate::api::*;
use leptos::prelude::*;

/// Helper function for metric card styling variants (BSL color scheme)
fn metric_card_classes(variant: &str) -> String {
    // Base: cream surface with gradient background
    let base = "rounded-xl p-6 shadow-sm hover:shadow-md transition-all duration-200";
    let bg_and_border = match variant {
        "teal" => "bg-gradient-to-br from-bsl-cream-100 to-bsl-bg-1 border border-bsl-brown-600/12 dark:from-bsl-charcoal-800 dark:to-bsl-bg-1",
        "yellow" => "bg-gradient-to-br from-bsl-cream-100 to-bsl-bg-2 border border-bsl-brown-600/12 dark:from-bsl-charcoal-800 dark:to-bsl-bg-2",
        "orange" => "bg-gradient-to-br from-bsl-cream-100 to-bsl-bg-6 border border-bsl-brown-600/12 dark:from-bsl-charcoal-800 dark:to-bsl-bg-6",
        "red" => "bg-gradient-to-br from-bsl-cream-100 to-bsl-bg-4 border border-bsl-brown-600/12 dark:from-bsl-charcoal-800 dark:to-bsl-bg-4",
        "purple" => "bg-gradient-to-br from-bsl-cream-100 to-bsl-bg-5 border border-bsl-brown-600/12 dark:from-bsl-charcoal-800 dark:to-bsl-bg-5",
        "cyan" => "bg-gradient-to-br from-bsl-cream-100 to-bsl-bg-8 border border-bsl-brown-600/12 dark:from-bsl-charcoal-800 dark:to-bsl-bg-8",
        _ => "bg-gradient-to-br from-bsl-cream-100 to-bsl-bg-1 border border-bsl-brown-600/12 dark:from-bsl-charcoal-800 dark:to-bsl-bg-1",
    };
    format!("{} {}", base, bg_and_border)
}

/// Helper function for metric card icon styling (BSL color scheme)
fn metric_icon_classes(variant: &str) -> String {
    let base = "text-4xl mb-3 flex items-center justify-center w-16 h-16 rounded-full";
    let color = match variant {
        "teal" => "bg-bsl-teal-500/10 dark:bg-bsl-teal-500/20 text-bsl-teal-600 dark:text-bsl-teal-400",
        "orange" => "bg-bsl-orange-500/10 dark:bg-bsl-orange-500/20 text-bsl-orange-500 dark:text-bsl-orange-400",
        "red" => "bg-bsl-red-500/10 dark:bg-bsl-red-500/20 text-bsl-red-500 dark:text-bsl-red-400",
        "slate" => "bg-bsl-slate-500/10 dark:bg-bsl-slate-500/20 text-bsl-slate-500 dark:text-bsl-gray-400",
        _ => "bg-bsl-brown-600/12 dark:bg-bsl-brown-600/20 text-bsl-slate-900 dark:text-bsl-gray-300",
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
                    <h3 class="text-sm text-bsl-slate-500 dark:text-bsl-gray-400 mb-2">"Всего типов"</h3>
                    <div class="text-3xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100 mb-1">
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
                    <div class="text-xs text-bsl-slate-500 dark:text-bsl-gray-400">"активных в системе"</div>
                </div>
            </div>

            // Known types metric (certainty_high → success/teal)
            <div class={metric_card_classes("teal")}>
                <div class={metric_icon_classes("teal")}>"✅"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-bsl-slate-500 dark:text-bsl-gray-400 mb-2">"Известные типы"</h3>
                    <div class="text-3xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100 mb-1">
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
                    <div class="text-xs text-bsl-slate-500 dark:text-bsl-gray-400">"100% определенность"</div>
                </div>
            </div>

            // Inferred types metric (certainty_medium → warning/orange)
            <div class={metric_card_classes("orange")}>
                <div class={metric_icon_classes("orange")}>"🔍"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-bsl-slate-500 dark:text-bsl-gray-400 mb-2">"Выведенные типы"</h3>
                    <div class="text-3xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100 mb-1">
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
                    <div class="text-xs text-bsl-slate-500 dark:text-bsl-gray-400">"анализ потоков"</div>
                </div>
            </div>

            // Unknown types metric (certainty_low → danger/red)
            <div class={metric_card_classes("red")}>
                <div class={metric_icon_classes("red")}>"❓"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-bsl-slate-500 dark:text-bsl-gray-400 mb-2">"Неизвестные типы"</h3>
                    <div class="text-3xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100 mb-1">
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
                    <div class="text-xs text-bsl-slate-500 dark:text-bsl-gray-400">"требуют анализа"</div>
                </div>
            </div>

            // Flow-sensitive types metric (info/slate)
            <div class={metric_card_classes("cyan")}>
                <div class={metric_icon_classes("slate")}>"🔄"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-bsl-slate-500 dark:text-bsl-gray-400 mb-2">"Flow-sensitive"</h3>
                    <div class="text-3xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100 mb-1">
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
                    <div class="text-xs text-bsl-slate-500 dark:text-bsl-gray-400">"динамических типов"</div>
                </div>
            </div>

            // Performance metrics (purple background)
            <div class={metric_card_classes("purple")}>
                <div class={metric_icon_classes("default")}>"⚡"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-bsl-slate-500 dark:text-bsl-gray-400 mb-2">"Производительность"</h3>
                    <div class="text-3xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100 mb-1">
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
                    <div class="text-xs text-bsl-slate-500 dark:text-bsl-gray-400">"попаданий в кеш"</div>
                    <div class="text-xs text-bsl-teal-600 dark:text-bsl-teal-400 mt-1">
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

            // Categories overview (yellow background)
            <div class={metric_card_classes("yellow")}>
                <div class={metric_icon_classes("default")}>"📊"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-bsl-slate-500 dark:text-bsl-gray-400 mb-2">"Категории"</h3>
                    <div class="flex flex-col gap-2">
                        {move || {
                            if let Some(result) = search_result.get() {
                                result.categories.iter().map(|(category, info)| {
                                    view! {
                                        <div class="flex items-center gap-3 p-2 bg-bsl-brown-600/5 dark:bg-bsl-gray-400/10 rounded">
                                            <span class="text-lg">{info.icon.clone()}</span>
                                            <span class="flex-1 font-medium text-bsl-slate-900 dark:text-bsl-gray-300">{category.clone()}</span>
                                            <span class="font-bold text-bsl-teal-600 dark:text-bsl-teal-400">{info.count.to_string()}</span>
                                        </div>
                                    }
                                }).collect::<Vec<_>>().into_any()
                            } else {
                                view! {
                                    <div class="flex items-center gap-3 p-2 bg-bsl-brown-600/5 dark:bg-bsl-gray-400/10 rounded">
                                        <span class="text-lg">"🔧"</span>
                                        <span class="flex-1 font-medium text-bsl-slate-900 dark:text-bsl-gray-300">"Platform"</span>
                                        <span class="font-bold text-bsl-teal-600 dark:text-bsl-teal-400">"5"</span>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>
            </div>

            // Analysis speed (cyan background)
            <div class={metric_card_classes("cyan")}>
                <div class={metric_icon_classes("teal")}>"🚀"</div>
                <div class="flex flex-col">
                    <h3 class="text-sm text-bsl-slate-500 dark:text-bsl-gray-400 mb-2">"Скорость анализа"</h3>
                    <div class="text-3xl font-bold text-bsl-slate-900 dark:text-bsl-cream-100 mb-1">
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
                    <div class="text-xs text-bsl-slate-500 dark:text-bsl-gray-400">"среднее время"</div>
                </div>
            </div>
        </div>
    }
}
