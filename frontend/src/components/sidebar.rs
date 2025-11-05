//! Sidebar component with filters

use crate::api::*;
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
        <aside
            class=move || format!(
                "bg-bsl-cream-100 dark:bg-bsl-charcoal-900 border border-bsl-teal-200 dark:border-gray-700 rounded-lg p-6 h-fit sticky transition-all duration-300 {} {}",
                "top-[calc(1.5rem+80px)]",
                if sidebar_open.get() { "w-72" } else { "w-0 overflow-hidden p-0" }
            )
            aria-expanded=move || if sidebar_open.get() { "true" } else { "false" }
        >
            <div class="flex justify-between items-center mb-5">
                <h3 class="m-0 text-xl font-semibold text-bsl-charcoal-900 dark:text-bsl-cream-100">"Фильтры"</h3>
                <button
                    class="hidden md:hidden bg-transparent border-0 text-xl cursor-pointer text-bsl-charcoal-700 dark:text-bsl-cream-300 hover:text-bsl-charcoal-900 dark:hover:text-bsl-cream-100 transition-colors"
                    on:click=move |_| sidebar_open.set(!sidebar_open.get())
                    aria-label=move || if sidebar_open.get() { "Закрыть меню" } else { "Открыть меню" }
                >
                    {move || if sidebar_open.get() { "×" } else { "☰" }}
                </button>
            </div>

            // Categories filter
                <div class="mb-6">
                    <h4 class="m-0 mb-3 text-base font-semibold text-bsl-charcoal-900 dark:text-bsl-cream-100">"Категории"</h4>
                    <div class="flex flex-col gap-2">
                        <label class="flex items-center gap-2 cursor-pointer p-2 rounded hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-800 transition-colors">
                            <input
                                type="checkbox"
                                class="m-0 cursor-pointer focus:outline-none focus:ring-2 focus:ring-bsl-teal-500 focus:ring-offset-2 dark:focus:ring-offset-bsl-charcoal-900"
                                prop:checked=move || filters.get().show_platform
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_platform_change(checked);
                                }
                            />
                            <span class="text-sm text-bsl-charcoal-900 dark:text-bsl-cream-100">"🔧 Platform"</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer p-2 rounded hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-800 transition-colors">
                            <input
                                type="checkbox"
                                class="m-0 cursor-pointer focus:outline-none focus:ring-2 focus:ring-bsl-teal-500 focus:ring-offset-2 dark:focus:ring-offset-bsl-charcoal-900"
                                prop:checked=move || filters.get().show_configuration
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_configuration_change(checked);
                                }
                            />
                            <span class="text-sm text-bsl-charcoal-900 dark:text-bsl-cream-100">"⚙️ Configuration"</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer p-2 rounded hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-800 transition-colors">
                            <input
                                type="checkbox"
                                class="m-0 cursor-pointer focus:outline-none focus:ring-2 focus:ring-bsl-teal-500 focus:ring-offset-2 dark:focus:ring-offset-bsl-charcoal-900"
                                prop:checked=move || filters.get().show_union
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_union_change(checked);
                                }
                            />
                            <span class="text-sm text-bsl-charcoal-900 dark:text-bsl-cream-100">"🔗 Union"</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer p-2 rounded hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-800 transition-colors">
                            <input
                                type="checkbox"
                                class="m-0 cursor-pointer focus:outline-none focus:ring-2 focus:ring-bsl-teal-500 focus:ring-offset-2 dark:focus:ring-offset-bsl-charcoal-900"
                                prop:checked=move || filters.get().show_dynamic
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_dynamic_change(checked);
                                }
                            />
                            <span class="text-sm text-bsl-charcoal-900 dark:text-bsl-cream-100">"🌟 Dynamic"</span>
                        </label>
                    </div>
                </div>

                // Certainty filter
                <div class="mb-6">
                    <h4 class="m-0 mb-3 text-base font-semibold text-bsl-charcoal-900 dark:text-bsl-cream-100">"Определенность"</h4>
                    <div class="flex flex-col gap-2">
                        <label class="flex items-center gap-2 cursor-pointer p-2 rounded hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-800 transition-colors">
                            <input
                                type="checkbox"
                                class="m-0 cursor-pointer focus:outline-none focus:ring-2 focus:ring-bsl-teal-500 focus:ring-offset-2 dark:focus:ring-offset-bsl-charcoal-900"
                                prop:checked=move || filters.get().show_high_certainty
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_high_certainty_change(checked);
                                }
                            />
                            <span class="text-sm text-bsl-charcoal-900 dark:text-bsl-cream-100">"Высокая (≥80%)"</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer p-2 rounded hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-800 transition-colors">
                            <input
                                type="checkbox"
                                class="m-0 cursor-pointer focus:outline-none focus:ring-2 focus:ring-bsl-teal-500 focus:ring-offset-2 dark:focus:ring-offset-bsl-charcoal-900"
                                prop:checked=move || filters.get().show_medium_certainty
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_medium_certainty_change(checked);
                                }
                            />
                            <span class="text-sm text-bsl-charcoal-900 dark:text-bsl-cream-100">"Средняя (30-79%)"</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer p-2 rounded hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-800 transition-colors">
                            <input
                                type="checkbox"
                                class="m-0 cursor-pointer focus:outline-none focus:ring-2 focus:ring-bsl-teal-500 focus:ring-offset-2 dark:focus:ring-offset-bsl-charcoal-900"
                                prop:checked=move || filters.get().show_low_certainty
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_low_certainty_change(checked);
                                }
                            />
                            <span class="text-sm text-bsl-charcoal-900 dark:text-bsl-cream-100">"Низкая (<30%)"</span>
                        </label>
                    </div>
                </div>

                // Additional filters
                <div class="mb-6">
                    <h4 class="m-0 mb-3 text-base font-semibold text-bsl-charcoal-900 dark:text-bsl-cream-100">"Дополнительно"</h4>
                    <div class="flex flex-col gap-2">
                        <label class="flex items-center gap-2 cursor-pointer p-2 rounded hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-800 transition-colors">
                            <input
                                type="checkbox"
                                class="m-0 cursor-pointer focus:outline-none focus:ring-2 focus:ring-bsl-teal-500 focus:ring-offset-2 dark:focus:ring-offset-bsl-charcoal-900"
                                prop:checked=move || filters.get().flow_sensitive_only
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    handle_flow_sensitive_change(checked);
                                }
                            />
                            <span class="text-sm text-bsl-charcoal-900 dark:text-bsl-cream-100">"Flow-sensitive"</span>
                        </label>
                    </div>
                </div>

            // Clear filters button
            <button
                class="inline-flex items-center justify-center gap-2 px-3 py-1.5 rounded border border-bsl-teal-600 dark:border-bsl-teal-500 bg-transparent text-bsl-teal-700 dark:text-bsl-teal-400 text-sm font-medium cursor-pointer transition-all duration-200 hover:bg-bsl-teal-600 hover:text-white dark:hover:bg-bsl-teal-500 dark:hover:text-white hover:-translate-y-0.5 active:translate-y-0 focus:outline-none focus:ring-2 focus:ring-bsl-teal-500 focus:ring-offset-2 dark:focus:ring-offset-bsl-charcoal-900"
                on:click=clear_filters
            >
                "Сбросить фильтры"
            </button>
        </aside>
    }
}
