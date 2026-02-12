//! Quick Actions Panel component for VSCode webview
//!
//! Provides search functionality and quick action buttons for BSL projects.
//! This replaces the React implementation from quickActions.tsx.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::common::{send_to_vscode, VsCodeMessage};

/// Search result from LSP server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub facet: String,
    pub certainty: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuickActionsTypeRepositorySnapshot {
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuickActionsDiagnosticsSnapshot {
    pub total: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub hints: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuickActionsSidebarSnapshot {
    pub type_repository: QuickActionsTypeRepositorySnapshot,
    pub diagnostics: QuickActionsDiagnosticsSnapshot,
}

/// Action button configuration
#[derive(Clone, Debug)]
struct ActionButton {
    icon: &'static str,
    title: &'static str,
    subtitle: &'static str,
    action: &'static str,
}

impl ActionButton {
    const ACTIONS: [ActionButton; 4] = [
        ActionButton {
            icon: "📊",
            title: "Анализ проекта",
            subtitle: "Полный анализ типов",
            action: "analyzeProject",
        },
        ActionButton {
            icon: "🔍",
            title: "Типы платформы",
            subtitle: "Live counts from TypeRepository",
            action: "buildIndex",
        },
        ActionButton {
            icon: "⚙️",
            title: "Настройки",
            subtitle: "Конфигурация LSP",
            action: "openSettings",
        },
        ActionButton {
            icon: "📚",
            title: "Документация",
            subtitle: "Открыть CLAUDE.md",
            action: "showDocs",
        },
    ];
}

/// Quick Actions Panel component
#[component]
pub fn QuickActionsPanel(
    /// Search results from LSP server
    search_results: ReadSignal<Vec<SearchResult>>,
    /// Searching state
    is_searching: ReadSignal<bool>,
    /// Unified sidebar snapshot from extension
    sidebar_snapshot: ReadSignal<Option<QuickActionsSidebarSnapshot>>,
) -> impl IntoView {
    let (search_query, set_search_query) = signal(String::new());

    // Handle search input
    let handle_search = move |query: String| {
        set_search_query.set(query.clone());

        if query.len() >= 2 {
            // Send search request to VSCode
            let _ = send_to_vscode(VsCodeMessage::new("searchTypes", query));
        } else {
            // Clear results on short/empty query
            let _ = send_to_vscode(VsCodeMessage::new("searchTypes", String::new()));
        }
    };

    // Handle action button click
    let handle_action = move |action: &'static str| {
        let _ = send_to_vscode(VsCodeMessage::new("executeAction", action.to_string()));
    };

    // Handle type click from search results
    let handle_type_click = move |type_name: String| {
        let _ = send_to_vscode(VsCodeMessage::new("showTypeDetails", type_name));
    };

    view! {
        <div class="bg-vscode-bg text-vscode-fg p-4 min-h-screen">
            <h1 class="text-xl font-bold mb-4">"BSL Quick Actions"</h1>

            <Show when=move || sidebar_snapshot.get().is_some()>
                <div class="mb-4 p-3 rounded bg-vscode-input-bg border border-vscode-fg/10 text-sm">
                    {move || {
                        if let Some(snapshot) = sidebar_snapshot.get() {
                            let repo = &snapshot.type_repository;
                            let diag = &snapshot.diagnostics;
                            if repo.status == "live" {
                                format!(
                                    "TypeRepository: {} (Platform {}, Config {}) | Diagnostics: {} (E{} W{} I{} H{})",
                                    repo.total_types,
                                    repo.platform_types,
                                    repo.configuration_types,
                                    diag.total,
                                    diag.errors,
                                    diag.warnings,
                                    diag.infos,
                                    diag.hints
                                )
                            } else {
                                format!(
                                    "TypeRepository: n/a | Diagnostics: {} (E{} W{} I{} H{})",
                                    diag.total,
                                    diag.errors,
                                    diag.warnings,
                                    diag.infos,
                                    diag.hints
                                )
                            }
                        } else {
                            String::new()
                        }
                    }}
                </div>
            </Show>

            // Search input section
            <div class="mb-6 relative">
                <input
                    type="text"
                    placeholder="Поиск типов (Справочники, Документы, Массив...)"
                    prop:value=move || search_query.get()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        handle_search(value);
                    }
                    class="w-full px-4 py-2 bg-vscode-input-bg text-vscode-input-fg rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
                />

                // Loading spinner
                <Show when=move || is_searching.get()>
                    <div class="absolute right-3 top-2.5">
                        <span class="animate-spin text-lg">"⏳"</span>
                    </div>
                </Show>

                // Search results dropdown
                <Show when=move || !search_results.get().is_empty()>
                    <div class="absolute z-10 w-full mt-1 bg-vscode-input-bg border border-vscode-fg/20 rounded-md shadow-lg max-h-64 overflow-y-auto">
                        <For
                            each=move || search_results.get()
                            key=|result| result.name.clone()
                            children=move |result: SearchResult| {
                                let name = result.name.clone();
                                view! {
                                    <div
                                        on:click=move |_| handle_type_click(name.clone())
                                        class="p-3 hover:bg-vscode-list-hover cursor-pointer border-b border-vscode-fg/10 last:border-b-0"
                                    >
                                        <div class="flex items-center justify-between">
                                            <code class="text-sm font-medium">{result.name.clone()}</code>
                                            <span class="text-xs px-2 py-0.5 bg-purple-600 text-white rounded">
                                                {result.facet.clone()}
                                            </span>
                                        </div>
                                        <div class="text-xs text-vscode-fg/60 mt-1">
                                            {result.certainty.clone()}
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </div>
                </Show>
            </div>

            // Action buttons grid (2x2)
            <div class="grid grid-cols-2 gap-4">
                <For
                    each=|| ActionButton::ACTIONS
                    key=|action| action.action
                    children=move |action: ActionButton| {
                        let action_id = action.action.to_string();
                        let action_command = action.action;
                        let static_subtitle = action.subtitle.to_string();
                        view! {
                            <button
                                on:click=move |_| handle_action(action_command)
                                class="p-4 bg-vscode-button-bg hover:bg-vscode-button-hover text-vscode-button-fg rounded-lg transition-colors"
                            >
                                <span class="text-2xl mb-2 block">{action.icon}</span>
                                <p class="font-medium">{action.title}</p>
                                <p class="text-xs text-vscode-fg/60 mt-1">
                                    {move || {
                                        if action_id == "buildIndex" {
                                            if let Some(snapshot) = sidebar_snapshot.get() {
                                                let repo = snapshot.type_repository;
                                                if repo.status == "live" {
                                                    format!(
                                                        "Platform {} / Config {} / Total {}",
                                                        repo.platform_types,
                                                        repo.configuration_types,
                                                        repo.total_types
                                                    )
                                                } else {
                                                    "TypeRepository: n/a".to_string()
                                                }
                                            } else {
                                                "TypeRepository: n/a".to_string()
                                            }
                                        } else {
                                            static_subtitle.clone()
                                        }
                                    }}
                                </p>
                            </button>
                        }
                    }
                />
            </div>
        </div>
    }
}
