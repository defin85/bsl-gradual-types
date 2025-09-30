//! Sidebar component with filters

use crate::api::types::*;
use leptos::prelude::*;

/// Sidebar with filtering options
#[component]
#[allow(non_snake_case)]
pub fn Sidebar(
    /// Current filters
    filters: RwSignal<TypeFilters>,
    /// Callback when filters change
    on_filters_change: Callback<TypeFilters>,
) -> impl IntoView {
    let sidebar_open = RwSignal::new(true);

    // Handle category filter change
    let handle_category_change = move |category: Option<TypeCategory>| {
        let mut new_filters = filters.get();
        new_filters.category = category;
        filters.set(new_filters.clone());
        on_filters_change.run(new_filters);
    };

    // Handle certainty filter change
    let handle_certainty_change = move |certainty_level: Option<String>| {
        let mut new_filters = filters.get();
        new_filters.certainty_level = certainty_level;
        filters.set(new_filters.clone());
        on_filters_change.run(new_filters);
    };

    // Handle flow-sensitive filter change
    let handle_flow_sensitive_change = move |flow_sensitive_only: bool| {
        let mut new_filters = filters.get();
        new_filters.flow_sensitive_only = flow_sensitive_only;
        filters.set(new_filters.clone());
        on_filters_change.run(new_filters);
    };

    // Clear all filters
    let clear_filters = move |_| {
        let new_filters = TypeFilters::default();
        filters.set(new_filters.clone());
        on_filters_change.run(new_filters);
    };

    view! {
        <aside class=move || format!("sidebar {}", if sidebar_open.get() { "open" } else { "closed" })>
            <div class="sidebar__header">
                <h3>"Фильтры"</h3>
                <button
                    class="sidebar__toggle"
                    on:click=move |_| sidebar_open.set(!sidebar_open.get())
                >
                    {move || if sidebar_open.get() { "×" } else { "☰" }}
                </button>
            </div>

            // Categories filter
                <div class="filter-section">
                    <h4>"Категории"</h4>
                    <div class="filter-group">
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                checked=move || matches!(filters.get().category, Some(TypeCategory::Platform))
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_category_change(if checked { Some(TypeCategory::Platform) } else { None });
                                }
                            />
                            <span>"🔧 Platform"</span>
                        </label>
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                checked=move || matches!(filters.get().category, Some(TypeCategory::Configuration))
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_category_change(if checked { Some(TypeCategory::Configuration) } else { None });
                                }
                            />
                            <span>"⚙️ Configuration"</span>
                        </label>
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                checked=move || matches!(filters.get().category, Some(TypeCategory::Union))
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_category_change(if checked { Some(TypeCategory::Union) } else { None });
                                }
                            />
                            <span>"🔗 Union"</span>
                        </label>
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                checked=move || matches!(filters.get().category, Some(TypeCategory::Dynamic))
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_category_change(if checked { Some(TypeCategory::Dynamic) } else { None });
                                }
                            />
                            <span>"🌟 Dynamic"</span>
                        </label>
                    </div>
                </div>

                // Certainty filter
                <div class="filter-section">
                    <h4>"Определенность"</h4>
                    <div class="filter-group">
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                checked=move || filters.get().certainty_level.as_ref().map_or(true, |level| level == "high")
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_certainty_change(if checked { Some("high".to_string()) } else { None });
                                }
                            />
                            <span>"Высокая (≥80%)"</span>
                        </label>
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                checked=move || filters.get().certainty_level.as_ref().map_or(true, |level| level == "medium")
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_certainty_change(if checked { Some("medium".to_string()) } else { None });
                                }
                            />
                            <span>"Средняя (30-79%)"</span>
                        </label>
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                checked=move || filters.get().certainty_level.as_ref().map_or(true, |level| level == "low")
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_certainty_change(if checked { Some("low".to_string()) } else { None });
                                }
                            />
                            <span>"Низкая (<30%)"</span>
                        </label>
                    </div>
                </div>

                // Additional filters
                <div class="filter-section">
                    <h4>"Дополнительно"</h4>
                    <div class="filter-group">
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                checked=move || filters.get().flow_sensitive_only
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_flow_sensitive_change(checked);
                                }
                            />
                            <span>"Flow-sensitive"</span>
                        </label>
                    </div>
                </div>

            // Clear filters button
            <button
                class="btn btn--outline btn--sm"
                on:click=clear_filters
            >
                "Сбросить фильтры"
            </button>
        </aside>
    }
}