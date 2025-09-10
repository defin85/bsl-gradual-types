//! Metric card component

use leptos::prelude::*;

fn metric_card_view(
    value: Signal<String>,
    title: Signal<String>,
    color: Signal<String>,
) -> impl IntoView {
    view! {
        <div class="metric-card">
            <div class="metric-value" style=move || format!("color: {};", color.get())>
                {value}
            </div>
            <div class="metric-title">{title}</div>
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
