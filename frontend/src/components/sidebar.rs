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

    // Handle category filter changes
    let handle_platform_change = move |checked: bool| {
        let mut new_filters = filters.get();
        new_filters.show_platform = checked;
        filters.set(new_filters.clone());
        on_filters_change.run(new_filters);
    };

    let handle_configuration_change = move |checked: bool| {
        let mut new_filters = filters.get();
        new_filters.show_configuration = checked;
        filters.set(new_filters.clone());
        on_filters_change.run(new_filters);
    };

    let handle_union_change = move |checked: bool| {
        let mut new_filters = filters.get();
        new_filters.show_union = checked;
        filters.set(new_filters.clone());
        on_filters_change.run(new_filters);
    };

    let handle_dynamic_change = move |checked: bool| {
        let mut new_filters = filters.get();
        new_filters.show_dynamic = checked;
        filters.set(new_filters.clone());
        on_filters_change.run(new_filters);
    };

    // Handle certainty filter changes
    let handle_high_certainty_change = move |checked: bool| {
        let mut new_filters = filters.get();
        new_filters.show_high_certainty = checked;
        filters.set(new_filters.clone());
        on_filters_change.run(new_filters);
    };

    let handle_medium_certainty_change = move |checked: bool| {
        let mut new_filters = filters.get();
        new_filters.show_medium_certainty = checked;
        filters.set(new_filters.clone());
        on_filters_change.run(new_filters);
    };

    let handle_low_certainty_change = move |checked: bool| {
        let mut new_filters = filters.get();
        new_filters.show_low_certainty = checked;
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
                                prop:checked=move || filters.get().show_platform
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_platform_change(checked);
                                }
                            />
                            <span>"🔧 Platform"</span>
                        </label>
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                prop:checked=move || filters.get().show_configuration
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_configuration_change(checked);
                                }
                            />
                            <span>"⚙️ Configuration"</span>
                        </label>
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                prop:checked=move || filters.get().show_union
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_union_change(checked);
                                }
                            />
                            <span>"🔗 Union"</span>
                        </label>
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                prop:checked=move || filters.get().show_dynamic
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_dynamic_change(checked);
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
                                prop:checked=move || filters.get().show_high_certainty
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_high_certainty_change(checked);
                                }
                            />
                            <span>"Высокая (≥80%)"</span>
                        </label>
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                prop:checked=move || filters.get().show_medium_certainty
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_medium_certainty_change(checked);
                                }
                            />
                            <span>"Средняя (30-79%)"</span>
                        </label>
                        <label class="filter-item">
                            <input
                                type="checkbox"
                                prop:checked=move || filters.get().show_low_certainty
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_low_certainty_change(checked);
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
                                prop:checked=move || filters.get().flow_sensitive_only
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
