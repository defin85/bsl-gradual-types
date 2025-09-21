//! Main App component with enhanced UI from front_template

use crate::components::{ViewType, HeaderSearchBar};
use crate::pages::{Dashboard, CardsPage, TablePage, GraphPage};
use leptos::prelude::*;

/// Главный компонент приложения с улучшенным дизайном
#[component]
#[allow(non_snake_case)]
pub fn App() -> impl IntoView {
    let current_view = RwSignal::new(ViewType::Dashboard);
    let search_query = RwSignal::new(String::new());

    let handle_view_change = move |new_view: ViewType| {
        current_view.set(new_view);
    };

    let handle_search = move |query: String| {
        search_query.set(query);
    };

    view! {
        <div class="app">
            // Enhanced Header Navigation
            <header class="header">
                <div class="container">
                    <div class="header__content">
                        <div class="nav-brand">
                            <h1 class="header__title">Система типизации</h1>
                            <div class="nav-subtitle">BSL Gradual Type System</div>
                        </div>
                        <nav class="mode-tabs">
                            <button
                                class=move || format!("mode-tab {}", if current_view.get() == ViewType::Dashboard { "active" } else { "" })
                                on:click=move |_| handle_view_change(ViewType::Dashboard)
                            >
                                "📊 Dashboard"
                            </button>
                            <button
                                class=move || format!("mode-tab {}", if current_view.get() == ViewType::Cards { "active" } else { "" })
                                on:click=move |_| handle_view_change(ViewType::Cards)
                            >
                                "🃏 Карточки"
                            </button>
                            <button
                                class=move || format!("mode-tab {}", if current_view.get() == ViewType::Table { "active" } else { "" })
                                on:click=move |_| handle_view_change(ViewType::Table)
                            >
                                "📋 Таблица"
                            </button>
                            <button
                                class=move || format!("mode-tab {}", if current_view.get() == ViewType::Graph { "active" } else { "" })
                                on:click=move |_| handle_view_change(ViewType::Graph)
                            >
                                "🕸️ Граф"
                            </button>
                        </nav>
                        <div class="search-box">
                            <HeaderSearchBar
                                search_query=search_query
                                on_search=Callback::new(handle_search)
                            />
                        </div>
                    </div>
                </div>
            </header>

            // Main Content Area
            <main class="main-content">
                {move || {
                    match current_view.get() {
                        ViewType::Dashboard => view! { <Dashboard _search_query=search_query /> }.into_any(),
                        ViewType::Cards => view! { <CardsPage _search_query=search_query /> }.into_any(),
                        ViewType::Table => view! { <TablePage _search_query=search_query /> }.into_any(),
                        ViewType::Graph => view! { <GraphPage _search_query=search_query /> }.into_any(),
                    }
                }}
            </main>
        </div>
    }
}
