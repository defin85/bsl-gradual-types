//! Enhanced Cards view component based on front_template

use crate::api::*;
use crate::components::{Pagination, TypeDetailsModal};
use leptos::prelude::*;

/// Enhanced Cards view with rich type information
#[component]
#[allow(non_snake_case)]
pub fn CardsView(
    /// Types signal
    types: Signal<Vec<TypeInfo>>,
    /// Search result signal
    search_result: Signal<Option<AnalysisResultDto>>,
    /// Page change callback (optional for compatibility)
    #[prop(optional)]
    on_page_change: Option<Callback<usize>>,
) -> impl IntoView {
    // State for modal
    let selected_type = RwSignal::new(None::<TypeInfo>);
    let is_closing = RwSignal::new(false);

    let handle_card_click = move |type_info: TypeInfo| {
        is_closing.set(false); // Reset closing flag when opening
        selected_type.set(Some(type_info));
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
        <div class="flex flex-col gap-6 p-6">
            // Top pagination (sticky)
            {move || {
                if let Some(callback) = on_page_change {
                    view! {
                        <div class="sticky top-20 z-10 bg-white dark:bg-gray-800 shadow-md dark:shadow-gray-900/50 rounded-lg p-4 mb-4">
                            <Pagination
                                pagination=Signal::derive(move || search_result.get().and_then(|r| r.pagination))
                                on_page_change=callback
                            />
                        </div>
                    }.into_any()
                } else {
                    let _: () = view! {};
                    ().into_any()
                }
            }}

            // Summary information
            <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 shadow-sm">
                <p class="text-sm font-medium text-gray-700 dark:text-gray-300">
                    {move || {
                        if let Some(result) = search_result.get() {
                            format!("Найдено типов: {} (показано: {})",
                                   result.metrics.total_types,
                                   types.get().len())
                        } else {
                            format!("Найдено типов: {}", types.get().len())
                        }
                    }}
                </p>
            </div>

            // Cards grid
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-5">
                {move || {
                    types.get().into_iter().map(|type_info| {
                        let category = type_info.category.clone();
                        let certainty = type_info.certainty;

                        view! {
                            <div
                                class=move || {
                                    let base = "bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-5 shadow-sm hover:shadow-lg hover:-translate-y-1 transition-all cursor-pointer relative overflow-hidden";
                                    let category_border = match category.as_str() {
                                        "Platform" => "border-t-4 border-t-bsl-teal-500 dark:border-t-bsl-teal-400",
                                        "Configuration" => "border-t-4 border-t-amber-500 dark:border-t-amber-400",
                                        "Union" => "border-t-4 border-t-blue-500 dark:border-t-blue-400",
                                        "Dynamic" => "border-t-4 border-t-purple-500 dark:border-t-purple-400",
                                        _ => "border-t-4 border-t-gray-500 dark:border-t-gray-400"
                                    };
                                    format!("{} {}", base, category_border)
                                }
                                data-type-id=type_info.id.clone()
                                on:click={
                                    let type_info = type_info.clone();
                                    move |_| handle_card_click(type_info.clone())
                                }
                            >
                                // Card header
                                <div class="flex items-start justify-between mb-3">
                                    <h3 class="text-lg font-bold text-gray-900 dark:text-white truncate flex-1">{type_info.name.clone()}</h3>
                                    <span class="text-xs font-semibold bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 px-2 py-1 rounded ml-2 whitespace-nowrap">
                                        {get_category_icon(&type_info.category)} " " {type_info.category.clone()}
                                    </span>
                                </div>

                                // Certainty indicator
                                <div class="mb-3">
                                    <div class="flex items-center gap-2">
                                        <div class="flex-1 h-1.5 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
                                            <div
                                                class="h-full bg-bsl-primary dark:bg-bsl-accent transition-all duration-300"
                                                style=move || format!("width: {}%", certainty)
                                            ></div>
                                        </div>
                                        <span class="text-xs font-medium text-gray-600 dark:text-gray-400">{type_info.certainty}"%"</span>
                                    </div>
                                </div>

                                // Description
                                <p class="text-sm text-gray-600 dark:text-gray-400 mb-3 line-clamp-2">{type_info.description.clone()}</p>

                                // Facets
                                <div class="flex flex-wrap gap-1.5 mb-3">
                                    {type_info.facets.iter().map(|facet| {
                                        view! {
                                            <span class="text-xs font-medium bg-bsl-teal-100 dark:bg-bsl-teal-900/30 text-bsl-teal-700 dark:text-bsl-teal-300 px-2 py-0.5 rounded">{facet.clone()}</span>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>

                                // Flow-sensitive indicator and source
                                <div class="flex items-center justify-between text-xs mb-3">
                                    <span class="font-medium bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 px-2 py-1 rounded">{type_info.source.clone()}</span>
                                    <span class=move || {
                                        if type_info.flow_sensitive {
                                            "font-medium text-blue-600 dark:text-blue-400"
                                        } else {
                                            "font-medium text-gray-500 dark:text-gray-500"
                                        }
                                    }>
                                        {if type_info.flow_sensitive { "🔄 Flow-sensitive" } else { "📊 Static" }}
                                    </span>
                                </div>

                                // Actions
                                <div class="flex items-center gap-2">
                                    <button
                                        class="flex items-center justify-center w-8 h-8 bg-gray-100 dark:bg-gray-800 hover:bg-bsl-primary dark:hover:bg-bsl-accent hover:text-white dark:hover:text-gray-900 text-gray-700 dark:text-gray-300 rounded transition-colors"
                                        title="Подробнее"
                                        on:click={
                                            let type_info = type_info.clone();
                                            move |_| handle_card_click(type_info.clone())
                                        }
                                    >
                                        "👁️"
                                    </button>
                                    <button class="flex items-center justify-center w-8 h-8 bg-gray-100 dark:bg-gray-800 hover:bg-bsl-primary dark:hover:bg-bsl-accent hover:text-white dark:hover:text-gray-900 text-gray-700 dark:text-gray-300 rounded transition-colors" title="Скопировать">
                                        "📋"
                                    </button>
                                    <button class="flex items-center justify-center w-8 h-8 bg-gray-100 dark:bg-gray-800 hover:bg-bsl-primary dark:hover:bg-bsl-accent hover:text-white dark:hover:text-gray-900 text-gray-700 dark:text-gray-300 rounded transition-colors" title="Связи">
                                        "🔗"
                                    </button>
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            // Empty state
            {move || {
                if types.get().is_empty() {
                    view! {
                        <div class="flex flex-col items-center justify-center py-16 text-center">
                            <div class="text-6xl mb-4">"🔍"</div>
                            <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-2">"Типы не найдены"</h3>
                            <p class="text-gray-600 dark:text-gray-400">
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

/// Get icon for category
fn get_category_icon(category: &str) -> &'static str {
    match category {
        "Platform" => "🔧",
        "Configuration" => "⚙️",
        "Union" => "🔗",
        "Dynamic" => "🌟",
        _ => "❓",
    }
}
