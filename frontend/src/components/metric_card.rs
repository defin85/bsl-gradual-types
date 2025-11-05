//! Metric card component

use leptos::prelude::*;

fn metric_card_view(
    value: Signal<String>,
    title: Signal<String>,
    color: Signal<String>,
) -> impl IntoView {
    view! {
        <div class="bg-bsl-cream-100 dark:bg-bsl-charcoal-800 border border-bsl-brown-600/12 dark:border-bsl-gray-400/15 rounded-lg p-6 shadow-sm transition-all duration-normal ease-smooth hover:-translate-y-0.5 hover:shadow-lg">
            <div class="text-2xl font-bold" style=move || format!("color: {};", color.get())>
                {value}
            </div>
            <div class="text-sm text-bsl-slate-500/70 dark:text-bsl-gray-300/70 mt-1">{title}</div>
        </div>
    }
}

#[component]
#[allow(non_snake_case)]
pub fn MetricCard(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] title: Signal<String>,
    #[prop(into)] color: Signal<String>,
) -> impl IntoView {
    metric_card_view(value, title, color)
}
