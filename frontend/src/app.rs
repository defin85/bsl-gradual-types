//! Unified App component based on front_template

use crate::api::*; // Uses shared DTOs + frontend extensions
use crate::components::{CardsView, Dashboard, GraphView, Sidebar, TableView};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Main application component with unified interface
#[component]
#[allow(non_snake_case)]
pub fn App() -> impl IntoView {
    // Application state
    let current_mode = RwSignal::new("dashboard".to_string());
    let types = RwSignal::new(Vec::<TypeDto>::new());
    let search_result = RwSignal::new(None::<AnalysisResultDto>);
    let metrics = RwSignal::new(None::<MetricsDto>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let snapshot_meta = RwSignal::new(None::<SnapshotMetaDto>);
    let snapshot_reload = RwSignal::new(false);
    let snapshot_error = RwSignal::new(None::<String>);
    let filters = RwSignal::new(TypeFilters::new());
    let search_query = RwSignal::new(String::new());

    // Load initial data
    let load_data = move || {
        loading.set(true);
        error.set(None);

        spawn_local(async move {
            // Load snapshot meta
            match fetch_snapshot_meta().await {
                Ok(meta) => snapshot_meta.set(Some(meta)),
                Err(err) => snapshot_error.set(Some(format!("Ошибка snapshot/meta: {}", err))),
            }

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
                }
                Err(err) => {
                    error.set(Some(format!("Ошибка загрузки типов: {}", err)));
                    loading.set(false);
                }
            }
        });
    };

    let reload_deps = move |_| {
        snapshot_reload.set(true);
        snapshot_error.set(None);

        spawn_local(async move {
            match reload_snapshot().await {
                Ok(meta) => {
                    snapshot_meta.set(Some(meta));
                    snapshot_reload.set(false);
                    load_data();
                }
                Err(err) => {
                    snapshot_error.set(Some(format!("Ошибка snapshot/reload: {}", err)));
                    snapshot_reload.set(false);
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
        <div class="min-h-screen flex flex-col bg-bsl-cream-50 dark:bg-bsl-charcoal-700">
            // Header with navigation (sticky at top)
            <header class="sticky top-0 z-50 bg-gradient-to-r from-bsl-cream-100 to-bsl-bg-1 dark:from-bsl-charcoal-800 dark:to-bsl-bg-1 border-b border-bsl-brown-600/20 dark:border-bsl-gray-400/30 shadow-sm">
                <div class="container max-w-[1280px] mx-auto px-6">
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

                    <div class="py-2 border-t border-bsl-brown-600/15 dark:border-bsl-gray-400/20">
                        {move || {
                            if let Some(meta) = snapshot_meta.get() {
                                view! {
                                    <div class="flex flex-wrap items-center gap-2 text-xs">
                                        <span class="font-semibold">"Snapshot"</span>
                                        <span>"deps_id"</span>
                                        <code class="px-2 py-1 rounded bg-bsl-cream-200/60 dark:bg-bsl-charcoal-700/40 font-mono text-[11px] break-all select-all">
                                            {meta.deps_id}
                                        </code>
                                        <span>"index_snapshot_id"</span>
                                        <code class="px-2 py-1 rounded bg-bsl-cream-200/60 dark:bg-bsl-charcoal-700/40 font-mono text-[11px] break-all select-all">
                                            {meta.index_snapshot_id}
                                        </code>
                                        <span>"platform_version"</span>
                                        <code class="px-2 py-1 rounded bg-bsl-cream-200/60 dark:bg-bsl-charcoal-700/40 font-mono text-[11px] break-all select-all">
                                            {meta.platform_version}
                                        </code>
                                        <span>"platform_fp"</span>
                                        <code class="px-2 py-1 rounded bg-bsl-cream-200/60 dark:bg-bsl-charcoal-700/40 font-mono text-[11px] break-all select-all">
                                            {meta.platform_fingerprint.unwrap_or_else(|| "none".to_string())}
                                        </code>
                                        <span>"config_fp"</span>
                                        <code class="px-2 py-1 rounded bg-bsl-cream-200/60 dark:bg-bsl-charcoal-700/40 font-mono text-[11px] break-all select-all">
                                            {meta.config_fingerprint.unwrap_or_else(|| "none".to_string())}
                                        </code>
                                        <span>"config_path"</span>
                                        <code class="px-2 py-1 rounded bg-bsl-cream-200/60 dark:bg-bsl-charcoal-700/40 font-mono text-[11px] break-all select-all">
                                            {meta.inputs.configuration_path.unwrap_or_else(|| "none".to_string())}
                                        </code>
                                        <span>"strict"</span>
                                        <code class="px-2 py-1 rounded bg-bsl-cream-200/60 dark:bg-bsl-charcoal-700/40 font-mono text-[11px] select-all">
                                            {meta.strict_fingerprint.to_string()}
                                        </code>
                                        <button
                                            class="btn btn--sm"
                                            disabled=move || snapshot_reload.get()
                                            on:click=reload_deps
                                        >
                                            {move || if snapshot_reload.get() { "Reloading..." } else { "Reload deps" }}
                                        </button>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="flex items-center gap-2 text-xs">
                                        <span class="font-semibold">"Snapshot"</span>
                                        <span class="opacity-70">"loading..."</span>
                                    </div>
                                }.into_any()
                            }
                        }}

                        {move || {
                            snapshot_error.get().map(|err| {
                                view! {
                                    <div class="text-xs text-red-600 dark:text-red-400 mt-1">
                                        {err}
                                    </div>
                                }
                            })
                        }}
                    </div>
                </div>
            </header>

            // Main content (scrollable)
            <main class="flex-1 overflow-auto">
                <div class="container max-w-[1280px] mx-auto px-6">
                    <div class="grid grid-cols-[280px_1fr] gap-6 py-6">
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
