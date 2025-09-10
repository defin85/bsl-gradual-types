//! Main App component

use crate::components::{Navigation, ViewSwitcher, ViewType};
use crate::pages::{Dashboard, CardsPage, TablePage, GraphPage};
use leptos::prelude::*;

/// Главный компонент приложения
#[component]
#[allow(non_snake_case)]
pub fn App() -> impl IntoView {
    let current_view = RwSignal::new(ViewType::Dashboard);

    let handle_view_change = move |new_view: ViewType| {
        current_view.set(new_view);
    };

    view! {
        <div class="app">
            <Navigation current_view=current_view />
            
            <div class="view-switcher-container" style="background: white; padding: 1rem; box-shadow: 0 2px 4px rgba(0,0,0,0.1);">
                <ViewSwitcher 
                    current_view=current_view
                    on_view_change=std::sync::Arc::new(handle_view_change)
                />
            </div>
            
            {move || {
                match current_view.get() {
                    ViewType::Dashboard => view! { <Dashboard /> }.into_any(),
                    ViewType::Cards => view! { <CardsPage /> }.into_any(),
                    ViewType::Table => view! { <TablePage /> }.into_any(),
                    ViewType::Graph => view! { <GraphPage /> }.into_any(),
                }
            }}
        </div>
    }
}
