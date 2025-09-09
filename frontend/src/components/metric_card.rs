//! Metric card component

use leptos::prelude::*;

#[component]
#[allow(non_snake_case)]
pub fn MetricCard(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] title: Signal<String>,
    #[prop(into)] color: Signal<String>,
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
