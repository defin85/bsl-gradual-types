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
        <nav class="main-nav">
            <div class="nav-brand">
                <h1>"BSL Gradual Type System"</h1>
                <div class="nav-subtitle">
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
            <div class="nav-info">
                <div class="current-view-indicator">
                    <span class="view-icon">{move || current_view.get().icon()}</span>
                    <span class="view-name">{move || current_view.get().display_name()}</span>
                </div>
                <div class="nav-status">
                    <span class="status-indicator online">"●"</span>
                    <span class="status-text">"Online"</span>
                </div>
            </div>
        </nav>
    }
}
