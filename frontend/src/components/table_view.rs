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
                SortOrder::None => {},
                SortOrder::Asc => {
                    types_list.sort_by(|a, b| match column.as_str() {
                        "name" => a.name.cmp(&b.name),
                        "category" => a.category.cmp(&b.category),
                        "certainty" => a.certainty.cmp(&b.certainty),
                        "flow_sensitive" => a.flow_sensitive.cmp(&b.flow_sensitive),
                        _ => std::cmp::Ordering::Equal,
                    });
                },
                SortOrder::Desc => {
                    types_list.sort_by(|a, b| match column.as_str() {
                        "name" => b.name.cmp(&a.name),
                        "category" => b.category.cmp(&a.category),
                        "certainty" => b.certainty.cmp(&a.certainty),
                        "flow_sensitive" => b.flow_sensitive.cmp(&a.flow_sensitive),
                        _ => std::cmp::Ordering::Equal,
                    });
                },
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
            },
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
        <div class="table-view">
            // Top pagination (sticky)
            <div class="pagination-top-sticky">
                <Pagination
                    pagination=Signal::derive(move || search_result.get().and_then(|r| r.pagination))
                    on_page_change=on_page_change
                />
            </div>

            // Summary information
            <div class="table-summary">
                <p class="results-info">
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

            // Table container
            <div class="table-container">
                <table class="data-table">
                    <thead>
                        <tr>
                            <th
                                class="sortable"
                                on:click=move |_| handle_sort("name".to_string())
                            >
                                "Название "
                                <span class="sort-indicator">{move || get_sort_indicator("name")}</span>
                            </th>
                            <th
                                class="sortable"
                                on:click=move |_| handle_sort("category".to_string())
                            >
                                "Категория "
                                <span class="sort-indicator">{move || get_sort_indicator("category")}</span>
                            </th>
                            <th
                                class="sortable"
                                on:click=move |_| handle_sort("certainty".to_string())
                            >
                                "Определенность "
                                <span class="sort-indicator">{move || get_sort_indicator("certainty")}</span>
                            </th>
                            <th>"Фасеты"</th>
                            <th
                                class="sortable"
                                on:click=move |_| handle_sort("flow_sensitive".to_string())
                            >
                                "Flow-sensitive "
                                <span class="sort-indicator">{move || get_sort_indicator("flow_sensitive")}</span>
                            </th>
                            <th>"Действия"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            sorted_types.get().into_iter().map(|type_info| {
                                let category_class = get_category_class(&type_info.category);

                                view! {
                                    <tr class=format!("type-row {}", category_class)>
                                        // Name
                                        <td class="name-cell">
                                            <div class="type-name">{type_info.name.clone()}</div>
                                            <div class="type-id">{type_info.id.clone()}</div>
                                        </td>

                                        // Category
                                        <td class="category-cell">
                                            <span class="category-badge">
                                                {get_category_icon(&type_info.category)} " " {type_info.category.clone()}
                                            </span>
                                        </td>

                                        // Certainty
                                        <td class="certainty-cell">
                                            <div class="certainty-container">
                                                <div class="certainty-bar">
                                                    <div
                                                        class="certainty-fill"
                                                        style=format!("width: {}%", type_info.certainty)
                                                    ></div>
                                                </div>
                                                <span class="certainty-text">{type_info.certainty}"%"</span>
                                            </div>
                                        </td>

                                        // Facets
                                        <td class="facets-cell">
                                            <div class="facets-list">
                                                {type_info.facets.iter().take(3).map(|facet| {
                                                    view! {
                                                        <span class="facet-tag">{facet.clone()}</span>
                                                    }
                                                }).collect::<Vec<_>>()}
                                                {if type_info.facets.len() > 3 {
                                                    view! {
                                                        <span class="facet-more">
                                                            "+" {type_info.facets.len() - 3}
                                                        </span>
                                                    }.into_any()
                                                } else {
                                                    view! {}.into_any()
                                                }}
                                            </div>
                                        </td>

                                        // Flow-sensitive
                                        <td class="flow-cell">
                                            <span class=format!("flow-status {}", if type_info.flow_sensitive { "active" } else { "inactive" })>
                                                {if type_info.flow_sensitive { "🔄" } else { "📊" }}
                                            </span>
                                        </td>

                                        // Actions
                                        <td class="actions-cell">
                                            <div class="action-buttons">
                                                <button
                                                    class="action-btn action-btn--view"
                                                    title="Просмотр"
                                                    on:click={
                                                        let type_info = type_info.clone();
                                                        move |_| handle_action("view".to_string(), type_info.clone())
                                                    }
                                                >
                                                    "👁️"
                                                </button>
                                                <button
                                                    class="action-btn action-btn--copy"
                                                    title="Копировать"
                                                    on:click={
                                                        let type_info = type_info.clone();
                                                        move |_| handle_action("copy".to_string(), type_info.clone())
                                                    }
                                                >
                                                    "📋"
                                                </button>
                                                <button
                                                    class="action-btn action-btn--link"
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
                        <div class="empty-state">
                            <div class="empty-state__icon">"📋"</div>
                            <h3 class="empty-state__title">"Таблица пуста"</h3>
                            <p class="empty-state__description">
                                "Попробуйте изменить фильтры поиска или очистить их"
                            </p>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
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

/// Get CSS class for category
fn get_category_class(category: &str) -> String {
    match category {
        "Platform" => "category-platform",
        "Configuration" => "category-configuration",
        "Union" => "category-union",
        "Dynamic" => "category-dynamic",
        _ => "category-unknown"
    }.to_string()
}

/// Get icon for category
fn get_category_icon(category: &str) -> &'static str {
    match category {
        "Platform" => "🔧",
        "Configuration" => "⚙️",
        "Union" => "🔗",
        "Dynamic" => "🌟",
        _ => "❓"
    }
}