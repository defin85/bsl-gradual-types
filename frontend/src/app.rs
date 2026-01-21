//! Unified App component based on front_template

use crate::api::*; // Uses shared DTOs + frontend extensions
use crate::components::{CardsView, Dashboard, GraphView, Sidebar, TableView};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendMode {
    Detecting,
    WebServer,
    McpAgent,
}

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
    let backend_mode = RwSignal::new(BackendMode::Detecting);

    // MCP dashboard state (read-only)
    let mcp_status = RwSignal::new(None::<McpStatusDto>);
    let mcp_sessions = RwSignal::new(None::<McpSessionsResponseDto>);
    let mcp_jobs = RwSignal::new(None::<McpJobsResponseDto>);
    let filters = RwSignal::new(TypeFilters::new());
    let search_query = RwSignal::new(String::new());

    // Load initial data
    let load_web_data = move || {
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
                    load_web_data();
                }
                Err(err) => {
                    snapshot_error.set(Some(format!("Ошибка snapshot/reload: {}", err)));
                    snapshot_reload.set(false);
                }
            }
        });
    };

    let load_mcp_data = move || {
        loading.set(true);
        error.set(None);
        snapshot_error.set(None);

        spawn_local(async move {
            match fetch_mcp_sessions().await {
                Ok(sessions) => {
                    let ready_session_id = sessions
                        .sessions
                        .iter()
                        .find(|session| session.ready)
                        .map(|session| session.session_id.clone());
                    let has_sessions = !sessions.sessions.is_empty();
                    mcp_sessions.set(Some(sessions));

                    if has_sessions {
                        if let Some(session_id) = ready_session_id {
                            match fetch_mcp_deps_meta(Some(&session_id)).await {
                                Ok(meta) => snapshot_meta.set(Some(meta)),
                                Err(err) => snapshot_error
                                    .set(Some(format!("Ошибка mcp/deps/meta: {}", err))),
                            }
                        } else {
                            snapshot_meta.set(None);
                        }
                    } else {
                        snapshot_meta.set(None);
                    }
                }
                Err(err) => {
                    error.set(Some(format!("Ошибка mcp/sessions: {}", err)));
                    loading.set(false);
                    return;
                }
            }

            match fetch_mcp_jobs().await {
                Ok(jobs) => {
                    mcp_jobs.set(Some(jobs));
                    loading.set(false);
                }
                Err(err) => {
                    error.set(Some(format!("Ошибка mcp/jobs: {}", err)));
                    loading.set(false);
                }
            }
        });
    };

    // Detect backend mode on mount and load corresponding data.
    Effect::new(move |_| {
        spawn_local(async move {
            match fetch_mcp_status().await {
                Ok(status) if status.supported && status.mode == McpBackendModeDto::McpAgent => {
                    backend_mode.set(BackendMode::McpAgent);
                    mcp_status.set(Some(status));
                    current_mode.set("mcp".to_string());
                    load_mcp_data();
                }
                Ok(_status) => {
                    backend_mode.set(BackendMode::WebServer);
                    load_web_data();
                }
                Err(_) => {
                    backend_mode.set(BackendMode::WebServer);
                    load_web_data();
                }
            }
        });
    });

    // Handle mode switching
    let switch_mode = move |mode: String| {
        current_mode.set(mode);
    };

    // Handle filter changes
    let handle_filters_change = move |new_filters: TypeFilters| {
        filters.set(new_filters);
        load_web_data();
    };

    // Handle search
    let handle_search = move |query: String| {
        search_query.set(query.clone());
        let mut new_filters = filters.get();
        new_filters.search_query = if query.is_empty() { None } else { Some(query) };
        new_filters.page = 1; // Reset to first page on search
        filters.set(new_filters);
        load_web_data();
    };

    // Handle page changes
    let handle_page_change = move |page: usize| {
        let mut new_filters = filters.get();
        new_filters.page = page;
        filters.set(new_filters);
        load_web_data();
    };

    view! {
        <div class="min-h-screen flex flex-col bg-bsl-cream-50 dark:bg-bsl-charcoal-700">
            // Header with navigation (sticky at top)
            <header class="sticky top-0 z-50 bg-gradient-to-r from-bsl-cream-100 to-bsl-bg-1 dark:from-bsl-charcoal-800 dark:to-bsl-bg-1 border-b border-bsl-brown-600/20 dark:border-bsl-gray-400/30 shadow-sm">
                <div class="container max-w-[1280px] mx-auto px-6">
                    <div class="header__content">
                        <h1 class="header__title">"Система типизации BSL"</h1>
                        {move || {
                            if backend_mode.get() == BackendMode::WebServer {
                                view! {
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
                                }.into_any()
                            } else if backend_mode.get() == BackendMode::McpAgent {
                                view! {
                                    <div class="flex items-center gap-2 text-sm">
                                        <span class="font-semibold">"MCP Dashboard"</span>
                                        {move || {
                                            mcp_status.get().and_then(|st| st.instance_id).map(|id| {
                                                view! {
                                                    <code class="px-2 py-1 rounded bg-bsl-cream-200/60 dark:bg-bsl-charcoal-700/40 font-mono text-[11px] select-all">
                                                        {id}
                                                    </code>
                                                }
                                            })
                                        }}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div class="text-sm opacity-70">"Определение режима..."</div> }.into_any()
                            }
                        }}
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
                                        {move || {
                                            if backend_mode.get() == BackendMode::WebServer {
                                                view! {
                                                    <button
                                                        class="btn btn--sm"
                                                        disabled=move || snapshot_reload.get()
                                                        on:click=reload_deps
                                                    >
                                                        {move || if snapshot_reload.get() { "Reloading..." } else { "Reload deps" }}
                                                    </button>
                                                }.into_any()
                                            } else {
                                                ().into_any()
                                            }
                                        }}
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
                    {move || {
                        match backend_mode.get() {
                            BackendMode::WebServer => view! {
                                <div class="grid grid-cols-[280px_1fr] gap-6 py-6">
                                    <Sidebar
                                        filters=filters
                                        on_filters_change=Callback::new(handle_filters_change)
                                    />

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
                                                        <button class="btn btn--sm" on:click=move |_| load_web_data()>"Повторить"</button>
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
                            }.into_any(),

                            BackendMode::McpAgent => view! {
                                <div class="py-6">
                                    {move || {
                                        if loading.get() {
                                            view! {
                                                <div class="loading">
                                                    <p>"🔄 Загрузка MCP данных..."</p>
                                                </div>
                                            }.into_any()
                                        } else if let Some(err) = error.get() {
                                            view! {
                                                <div class="error">
                                                    <p>"❌ " {err}</p>
                                                    <button class="btn btn--sm" on:click=move |_| load_mcp_data()>"Повторить"</button>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="space-y-6">
                                                    <div class="card">
                                                        <div class="flex items-center justify-between">
                                                            <h2 class="text-lg font-semibold mb-2">"Сессии"</h2>
                                                            <button class="btn btn--sm" on:click=move |_| load_mcp_data()>"Обновить"</button>
                                                        </div>
                                                        {move || {
                                                            mcp_sessions.get().map(|resp| {
                                                                view! {
                                                                    <div class="space-y-2">
                                                                        {resp.sessions.into_iter().map(|s| {
                                                                            let missing_inputs = s.missing_inputs.clone();
                                                                            view! {
                                                                                <div class="p-3 rounded border border-bsl-brown-600/15 dark:border-bsl-gray-400/20">
                                                                                    <div class="flex flex-wrap items-center gap-2 text-sm">
                                                                                        <span class="font-semibold">{s.session_id.clone()}</span>
                                                                                        <span class="opacity-70">{format!("ready={}", s.ready)}</span>
                                                                                        <span class="opacity-70">{format!("rev={}", s.analysis_revision)}</span>
                                                                                        <span class="opacity-70">{format!("{} {}%", s.phase, s.progress_percent)}</span>
                                                                                    </div>
                                                                                    <div class="mt-2 text-xs">
                                                                                        <div class="opacity-70">"roots:"</div>
                                                                                        <ul class="list-disc ml-5">
                                                                                            {s.roots.into_iter().map(|r| view! { <li><code class="font-mono text-[11px] select-all">{format!("{}:{}", r.root_id, r.path)}</code></li> }).collect_view()}
                                                                                        </ul>
                                                                                    </div>
                                                                                    {if !missing_inputs.is_empty() {
                                                                                        view! {
                                                                                            <div class="mt-2 text-xs text-yellow-700 dark:text-yellow-300">
                                                                                                <span class="font-semibold">"missing_inputs: "</span>
                                                                                                {missing_inputs.join(", ")}
                                                                                            </div>
                                                                                        }
                                                                                        .into_any()
                                                                                    } else {
                                                                                        ().into_any()
                                                                                    }}
                                                                                </div>
                                                                            }
                                                                        }).collect_view()}
                                                                    </div>
                                                                }.into_any()
                                                            })
                                                        }}
                                                    </div>

                                                    <div class="card">
                                                        <h2 class="text-lg font-semibold mb-2">"Jobs"</h2>
                                                        {move || {
                                                            mcp_jobs.get().map(|resp| {
                                                                view! {
                                                                    <div class="space-y-2">
                                                                        {resp.jobs.into_iter().map(|j| {
                                                                            view! {
                                                                                <div class="p-3 rounded border border-bsl-brown-600/15 dark:border-bsl-gray-400/20 text-sm">
                                                                                    <div class="flex flex-wrap items-center gap-2">
                                                                                        <code class="font-mono text-[11px] select-all">{j.job_id}</code>
                                                                                        <span class="opacity-70">{format!("{} {}%", j.phase, j.progress_percent)}</span>
                                                                                        <span class="opacity-70">{j.state}</span>
                                                                                    </div>
                                                                                    {j.error.map(|e| view! { <div class="mt-1 text-xs text-red-600 dark:text-red-400">{e}</div> })}
                                                                                </div>
                                                                            }
                                                                        }).collect_view()}
                                                                    </div>
                                                                }.into_any()
                                                            })
                                                        }}
                                                    </div>
                                                </div>
                                            }.into_any()
                                        }
                                    }}
                                </div>
                            }.into_any(),

                            BackendMode::Detecting => view! {
                                <div class="py-6">
                                    <div class="loading">
                                        <p>"🔄 Определение режима..."</p>
                                    </div>
                                </div>
                            }.into_any(),
                        }
                    }}
                </div>
            </main>
        </div>
    }
}
