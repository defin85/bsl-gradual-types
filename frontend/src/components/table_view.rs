//! Enhanced Table view component with sorting and pagination

use crate::api::*;
use crate::components::{Pagination, TypeDetailsModal};
use leptos::prelude::*;

#[derive(Debug, Clone)]
enum SortOrder {
    None,
    Asc,
    Desc,
}

/// Enhanced Table view with sorting and pagination capabilities
#[component]
#[allow(non_snake_case)]
pub fn TableView(
    /// Types signal
    types: Signal<Vec<TypeInfo>>,
    /// Search result signal
    search_result: Signal<Option<AnalysisResultDto>>,
    /// Page change callback
    on_page_change: Callback<usize>,
) -> impl IntoView {
    let sort_column = RwSignal::new(None::<String>);
    let sort_order = RwSignal::new(SortOrder::None);

    // State for modal
    let selected_type = RwSignal::new(None::<TypeInfo>);
    let is_closing = RwSignal::new(false);

    // Handle sort column click
    let handle_sort = move |column: String| {
        let current_order = if sort_column.get().as_ref() == Some(&column) {
            match sort_order.get() {
                SortOrder::None => SortOrder::Asc,
                SortOrder::Asc => SortOrder::Desc,
                SortOrder::Desc => SortOrder::None,
            }
        } else {
            SortOrder::Asc
        };

        sort_column.set(Some(column));
        sort_order.set(current_order);
    };

    // Get sorted types
    let sorted_types = Signal::derive(move || {
        let mut types_list = types.get();

        if let (Some(column), order) = (sort_column.get(), sort_order.get()) {
            match order {
                SortOrder::None => {}
                SortOrder::Asc => {
                    types_list.sort_by(|a, b| match column.as_str() {
                        "name" => a.name.cmp(&b.name),
                        "category" => a.category.cmp(&b.category),
                        "certainty" => a.certainty.cmp(&b.certainty),
                        "flow_sensitive" => a.flow_sensitive.cmp(&b.flow_sensitive),
                        _ => std::cmp::Ordering::Equal,
                    });
                }
                SortOrder::Desc => {
                    types_list.sort_by(|a, b| match column.as_str() {
                        "name" => b.name.cmp(&a.name),
                        "category" => b.category.cmp(&a.category),
                        "certainty" => b.certainty.cmp(&a.certainty),
                        "flow_sensitive" => b.flow_sensitive.cmp(&a.flow_sensitive),
                        _ => std::cmp::Ordering::Equal,
                    });
                }
            }
        }

        types_list
    });

    // Get sort indicator
    let get_sort_indicator = move |column: &str| {
        if sort_column.get().as_ref() == Some(&column.to_string()) {
            match sort_order.get() {
                SortOrder::Asc => "↑",
                SortOrder::Desc => "↓",
                SortOrder::None => "↕",
            }
        } else {
            "↕"
        }
    };

    let handle_action = move |action: String, type_info: TypeInfo| {
        match action.as_str() {
            "view" => {
                is_closing.set(false); // Reset closing flag when opening
                selected_type.set(Some(type_info));
            }
            "copy" => web_sys::console::log_1(&format!("Copy type: {}", type_info.name).into()),
            "link" => web_sys::console::log_1(&format!("Link to type: {}", type_info.name).into()),
            _ => {}
        }
    };

    let close_modal = move |_: ()| {
        if !is_closing.get() {
            is_closing.set(true);

            // Defer closing to next tick to allow event handlers to complete
            leptos::task::spawn_local(async move {
                // Small delay to ensure DOM events complete
                gloo_timers::future::TimeoutFuture::new(50).await;
                selected_type.set(None);
                is_closing.set(false);
            });
        }
    };

    view! {
        <div class="flex flex-col gap-4">
            // Top pagination
            <div class="bg-bsl-surface dark:bg-gray-900 py-4">
                <Pagination
                    pagination=Signal::derive(move || search_result.get().and_then(|r| r.pagination))
                    on_page_change=on_page_change
                />
            </div>

            // Summary information
            <div class="px-4 py-3 bg-gray-50 dark:bg-gray-800/50 rounded-lg border border-gray-200 dark:border-gray-700">
                <p class="text-sm text-gray-700 dark:text-gray-300 font-medium">
                    {move || {
                        if let Some(result) = search_result.get() {
                            format!("Найдено типов: {} (показано: {})",
                                   result.metrics.total_types,
                                   sorted_types.get().len())
                        } else {
                            format!("Найдено типов: {}", sorted_types.get().len())
                        }
                    }}
                </p>
            </div>

            // Table container with responsive scroll
            <div class="overflow-x-auto bg-white dark:bg-gray-900 rounded-lg shadow-md border border-gray-200 dark:border-gray-700">
                <table class="min-w-full border-collapse">
                    <thead class="sticky top-0 z-20 bg-gray-50 dark:bg-gray-800">
                        <tr>
                            <th
                                class="px-4 py-3 text-left text-xs font-semibold text-gray-700 dark:text-gray-300 uppercase tracking-wider cursor-pointer select-none hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors border-b border-gray-200 dark:border-gray-700 min-w-[200px]"
                                on:click=move |_| handle_sort("name".to_string())
                            >
                                "Название "
                                <span class="ml-1 opacity-50 hover:opacity-100 transition-opacity">{move || get_sort_indicator("name")}</span>
                            </th>
                            <th
                                class="px-4 py-3 text-left text-xs font-semibold text-gray-700 dark:text-gray-300 uppercase tracking-wider cursor-pointer select-none hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors border-b border-gray-200 dark:border-gray-700 min-w-[140px]"
                                on:click=move |_| handle_sort("category".to_string())
                            >
                                "Категория "
                                <span class="ml-1 opacity-50 hover:opacity-100 transition-opacity">{move || get_sort_indicator("category")}</span>
                            </th>
                            <th
                                class="px-4 py-3 text-left text-xs font-semibold text-gray-700 dark:text-gray-300 uppercase tracking-wider cursor-pointer select-none hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors border-b border-gray-200 dark:border-gray-700 min-w-[160px]"
                                on:click=move |_| handle_sort("certainty".to_string())
                            >
                                "Определенность "
                                <span class="ml-1 opacity-50 hover:opacity-100 transition-opacity">{move || get_sort_indicator("certainty")}</span>
                            </th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-gray-700 dark:text-gray-300 uppercase tracking-wider border-b border-gray-200 dark:border-gray-700 min-w-[180px]">"Фасеты"</th>
                            <th
                                class="px-3 py-3 text-center text-xs font-semibold text-gray-700 dark:text-gray-300 uppercase tracking-wider cursor-pointer select-none hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors border-b border-gray-200 dark:border-gray-700 min-w-[120px]"
                                on:click=move |_| handle_sort("flow_sensitive".to_string())
                            >
                                <div class="flex items-center justify-center gap-1">
                                    "Flow"
                                    <span class="opacity-50 hover:opacity-100 transition-opacity">{move || get_sort_indicator("flow_sensitive")}</span>
                                </div>
                            </th>
                            <th class="px-3 py-3 text-center text-xs font-semibold text-gray-700 dark:text-gray-300 uppercase tracking-wider border-b border-gray-200 dark:border-gray-700 min-w-[110px]">"Действия"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            sorted_types.get().into_iter().enumerate().map(|(idx, type_info)| {
                                let row_bg = if idx % 2 == 0 { "" } else { "bg-gray-50/50 dark:bg-gray-800/50" };

                                view! {
                                    <tr class=format!("{} hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors", row_bg)>
                                        // Name
                                        <td class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
                                            <div class="font-medium text-gray-900 dark:text-white">{type_info.name.clone()}</div>
                                            <div class="text-xs text-gray-500 dark:text-gray-400 font-mono">{type_info.id.clone()}</div>
                                        </td>

                                        // Category
                                        <td class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
                                            <span class=get_category_badge_class(&type_info.category)>
                                                {get_category_icon(&type_info.category)} " " {type_info.category.clone()}
                                            </span>
                                        </td>

                                        // Certainty
                                        <td class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
                                            <div class="flex items-center gap-2">
                                                <div class="flex-1 h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden min-w-[80px]">
                                                    <div
                                                        class="h-full bg-bsl-primary dark:bg-bsl-accent transition-all duration-300"
                                                        style=format!("width: {}%", type_info.certainty)
                                                    ></div>
                                                </div>
                                                <span class="text-xs font-medium text-gray-700 dark:text-gray-300 min-w-[2.5rem] text-right">{type_info.certainty}"%"</span>
                                            </div>
                                        </td>

                                        // Facets
                                        <td class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
                                            <div class="flex flex-wrap gap-1">
                                                {type_info.facets.iter().take(3).map(|facet| {
                                                    view! {
                                                        <span class="inline-block px-2 py-0.5 text-xs font-medium bg-bsl-primary/10 dark:bg-bsl-accent/10 text-bsl-primary dark:text-bsl-accent rounded">{facet.clone()}</span>
                                                    }
                                                }).collect::<Vec<_>>()}
                                                {if type_info.facets.len() > 3 {
                                                    view! {
                                                        <span class="inline-block px-2 py-0.5 text-xs font-medium bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-300 rounded">
                                                            "+" {type_info.facets.len() - 3}
                                                        </span>
                                                    }.into_any()
                                                } else {
                                                    let _: () = view! {};
                                                    ().into_any()
                                                }}
                                            </div>
                                        </td>

                                        // Flow-sensitive
                                        <td class="px-3 py-3 border-b border-gray-200 dark:border-gray-700 text-center">
                                            <span class={
                                                if type_info.flow_sensitive {
                                                    "inline-flex items-center justify-center w-7 h-7 text-base bg-bsl-warning/10 dark:bg-bsl-warning/20 rounded-full"
                                                } else {
                                                    "inline-flex items-center justify-center w-7 h-7 text-base bg-gray-100 dark:bg-gray-800 rounded-full opacity-50"
                                                }
                                            }>
                                                {if type_info.flow_sensitive { "✅" } else { "❌" }}
                                            </span>
                                        </td>

                                        // Actions
                                        <td class="px-3 py-3 border-b border-gray-200 dark:border-gray-700">
                                            <div class="flex gap-1 justify-center">
                                                <button
                                                    class="inline-flex items-center justify-center w-7 h-7 text-xs bg-blue-50 dark:bg-blue-900/20 hover:bg-blue-100 dark:hover:bg-blue-900/40 text-blue-600 dark:text-blue-400 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1"
                                                    title="Просмотр"
                                                    on:click={
                                                        let type_info = type_info.clone();
                                                        move |_| handle_action("view".to_string(), type_info.clone())
                                                    }
                                                >
                                                    "👁"
                                                </button>
                                                <button
                                                    class="inline-flex items-center justify-center w-7 h-7 text-xs bg-green-50 dark:bg-green-900/20 hover:bg-green-100 dark:hover:bg-green-900/40 text-green-600 dark:text-green-400 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-1"
                                                    title="Копировать"
                                                    on:click={
                                                        let type_info = type_info.clone();
                                                        move |_| handle_action("copy".to_string(), type_info.clone())
                                                    }
                                                >
                                                    "📋"
                                                </button>
                                                <button
                                                    class="inline-flex items-center justify-center w-7 h-7 text-xs bg-purple-50 dark:bg-purple-900/20 hover:bg-purple-100 dark:hover:bg-purple-900/40 text-purple-600 dark:text-purple-400 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-1"
                                                    title="Связи"
                                                    on:click={
                                                        let type_info = type_info.clone();
                                                        move |_| handle_action("link".to_string(), type_info.clone())
                                                    }
                                                >
                                                    "🔗"
                                                </button>
                                            </div>
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </tbody>
                </table>
            </div>

            // Empty state
            {move || {
                if sorted_types.get().is_empty() {
                    view! {
                        <div class="flex flex-col items-center justify-center py-16 px-4 bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700">
                            <div class="text-6xl mb-4 opacity-50">"📋"</div>
                            <h3 class="text-xl font-semibold text-gray-900 dark:text-white mb-2">"Таблица пуста"</h3>
                            <p class="text-gray-600 dark:text-gray-400 text-center max-w-md">
                                "Попробуйте изменить фильтры поиска или очистить их"
                            </p>
                        </div>
                    }.into_any()
                } else {
                    let _: () = view! {};
                    ().into_any()
                }
            }}

            // Modal for type details
            <TypeDetailsModal
                type_info=Signal::derive(move || selected_type.get())
                on_close=Callback::new(close_modal)
            />
        </div>
    }
}

/// Get Tailwind classes for category badge
fn get_category_badge_class(category: &str) -> &'static str {
    match category {
        "Platform" => "inline-flex items-center gap-1 px-3 py-1 text-sm font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 rounded-full",
        "Configuration" => "inline-flex items-center gap-1 px-3 py-1 text-sm font-medium bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded-full",
        "Union" => "inline-flex items-center gap-1 px-3 py-1 text-sm font-medium bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300 rounded-full",
        "Dynamic" => "inline-flex items-center gap-1 px-3 py-1 text-sm font-medium bg-pink-100 dark:bg-pink-900/30 text-pink-700 dark:text-pink-300 rounded-full",
        _ => "inline-flex items-center gap-1 px-3 py-1 text-sm font-medium bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 rounded-full",
    }
}

/// Get icon for category
fn get_category_icon(category: &str) -> &'static str {
    match category {
        "Platform" => "🔧",
        "Configuration" => "⚙️",
        "Union" => "🎯",
        "Dynamic" => "🔄",
        _ => "❓",
    }
}
