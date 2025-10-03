//! Unified App component based on front_template

use crate::api::{fetch_types, fetch_metrics, types::*};
use crate::components::{Dashboard, CardsView, TableView, GraphView, Sidebar};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Main application component with unified interface
#[component]
#[allow(non_snake_case)]
pub fn App() -> impl IntoView {
    // Application state
    let current_mode = RwSignal::new("dashboard".to_string());
    let types = RwSignal::new(Vec::<TypeInfo>::new());
    let search_result = RwSignal::new(None::<TypeSearchResult>);
    let metrics = RwSignal::new(None::<TypeMetrics>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let filters = RwSignal::new(TypeFilters::new());
    let search_query = RwSignal::new(String::new());

    // Load initial data
    let load_data = move || {
        loading.set(true);
        error.set(None);

        spawn_local(async move {
            // Load metrics
            match fetch_metrics().await {
                Ok(metrics_data) => metrics.set(Some(metrics_data)),
                Err(err) => error.set(Some(format!("Ошибка загрузки метрик: {}", err))),
            }

            // Load types
            match fetch_types(filters.get()).await {
                Ok(result) => {
                    types.set(result.types.clone());
                    search_result.set(Some(result));
                    loading.set(false);
                },
                Err(err) => {
                    error.set(Some(format!("Ошибка загрузки типов: {}", err)));
                    loading.set(false);
                }
            }
        });
    };

    // Load data on mount
    Effect::new(move |_| {
        load_data();
    });

    // Handle mode switching
    let switch_mode = move |mode: String| {
        current_mode.set(mode);
    };

    // Handle filter changes
    let handle_filters_change = move |new_filters: TypeFilters| {
        filters.set(new_filters);
        load_data();
    };

    // Handle search
    let handle_search = move |query: String| {
        search_query.set(query.clone());
        let mut new_filters = filters.get();
        new_filters.search_query = if query.is_empty() { None } else { Some(query) };
        new_filters.page = 1; // Reset to first page on search
        filters.set(new_filters);
        load_data();
    };

    // Handle page changes
    let handle_page_change = move |page: usize| {
        let mut new_filters = filters.get();
        new_filters.page = page;
        filters.set(new_filters);
        load_data();
    };

    view! {
        <div class="app">
            // Header with navigation
            <header class="header">
                <div class="container">
                    <div class="header__content">
                        <h1 class="header__title">"Система типизации BSL"</h1>
                        <nav class="mode-tabs">
                            <button
                                class=move || format!("mode-tab {}", if current_mode.get() == "dashboard" { "active" } else { "" })
                                on:click=move |_| switch_mode("dashboard".to_string())
                            >
                                "📊 Dashboard"
                            </button>
                            <button
                                class=move || format!("mode-tab {}", if current_mode.get() == "cards" { "active" } else { "" })
                                on:click=move |_| switch_mode("cards".to_string())
                            >
                                "🃏 Карточки"
                            </button>
                            <button
                                class=move || format!("mode-tab {}", if current_mode.get() == "table" { "active" } else { "" })
                                on:click=move |_| switch_mode("table".to_string())
                            >
                                "📋 Таблица"
                            </button>
                            <button
                                class=move || format!("mode-tab {}", if current_mode.get() == "graph" { "active" } else { "" })
                                on:click=move |_| switch_mode("graph".to_string())
                            >
                                "🕸️ Граф"
                            </button>
                        </nav>
                        <div class="search-box">
                            <input
                                type="text"
                                placeholder="Поиск типов..."
                                class="form-control"
                                prop:value=move || search_query.get()
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    handle_search(value);
                                }
                            />
                        </div>
                    </div>
                </div>
            </header>

            // Main content
            <main class="main">
                <div class="container">
                    <div class="layout">
                        // Sidebar with filters
                        <Sidebar
                            filters=filters
                            on_filters_change=Callback::new(handle_filters_change)
                        />

                        // Content area
                        <div class="content">
                            {move || {
                                if loading.get() {
                                    view! {
                                        <div class="loading">
                                            <p>"🔄 Загрузка данных..."</p>
                                        </div>
                                    }.into_any()
                                } else if let Some(err) = error.get() {
                                    view! {
                                        <div class="error">
                                            <p>"❌ " {err}</p>
                                            <button class="btn btn--sm" on:click=move |_| load_data()>"Повторить"</button>
                                        </div>
                                    }.into_any()
                                } else {
                                    match current_mode.get().as_str() {
                                        "dashboard" => view! {
                                            <div class="mode-content active">
                                                <Dashboard
                                                    metrics=Signal::derive(move || metrics.get())
                                                    search_result=Signal::derive(move || search_result.get())
                                                />
                                            </div>
                                        }.into_any(),
                                        "cards" => view! {
                                            <div class="mode-content active">
                                                <CardsView
                                                    types=Signal::derive(move || types.get())
                                                    search_result=Signal::derive(move || search_result.get())
                                                    on_page_change=Callback::new(handle_page_change)
                                                />
                                            </div>
                                        }.into_any(),
                                        "table" => view! {
                                            <div class="mode-content active">
                                                <TableView
                                                    types=Signal::derive(move || types.get())
                                                    search_result=Signal::derive(move || search_result.get())
                                                    on_page_change=Callback::new(handle_page_change)
                                                />
                                            </div>
                                        }.into_any(),
                                        "graph" => view! {
                                            <div class="mode-content active">
                                                <GraphView
                                                    types=Signal::derive(move || types.get())
                                                />
                                            </div>
                                        }.into_any(),
                                        _ => view! { <div>"Неизвестный режим"</div> }.into_any()
                                    }
                                }
                            }}
                        </div>
                    </div>
                </div>
            </main>
        </div>
    }
}