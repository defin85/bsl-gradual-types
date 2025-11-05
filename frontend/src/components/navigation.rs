//! Navigation component

use crate::components::ViewType;
use leptos::prelude::*;

/// Компонент навигации
#[component]
#[allow(non_snake_case)]
pub fn Navigation(
    /// Текущее активное представление
    #[prop(into)]
    current_view: RwSignal<ViewType>,
) -> impl IntoView {
    view! {
        <nav class="bg-bsl-cream-100 dark:bg-bsl-charcoal-800 border-b border-bsl-brown-600/20 dark:border-bsl-gray-400/30 px-6 py-4 flex items-center justify-between">
            <div class="flex flex-col gap-1">
                <h1 class="text-2xl font-bold text-bsl-slate-900 dark:text-bsl-gray-200">"BSL Gradual Type System"</h1>
                <div class="text-sm text-bsl-slate-500/70 dark:text-bsl-gray-300/70">
                    {move || {
                        match current_view.get() {
                            ViewType::Dashboard => "Executive Dashboard - Обзор метрик системы",
                            ViewType::Cards => "Card Explorer - Карточное представление типов",
                            ViewType::Table => "Analytical Table - Табличный анализ типов",
                            ViewType::Graph => "Network Graph - Сетевая визуализация связей",
                        }
                    }}
                </div>
            </div>
            <div class="flex items-center gap-6">
                <div class="flex items-center gap-2">
                    <span class="text-lg">{move || current_view.get().icon()}</span>
                    <span class="text-sm font-medium text-bsl-slate-900 dark:text-bsl-gray-200">{move || current_view.get().display_name()}</span>
                </div>
                <div class="flex items-center gap-1">
                    <span class="text-bsl-teal-500 text-xs">"●"</span>
                    <span class="text-xs text-bsl-slate-500/70 dark:text-bsl-gray-300/70">"Online"</span>
                </div>
            </div>
        </nav>
    }
}
