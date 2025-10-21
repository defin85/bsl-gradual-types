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
        <div class="cards-view">
            // Top pagination (sticky)
            {move || {
                if let Some(callback) = on_page_change {
                    view! {
                        <div class="pagination-top-sticky">
                            <Pagination
                                pagination=Signal::derive(move || search_result.get().and_then(|r| r.pagination))
                                on_page_change=callback
                            />
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            // Summary information
            <div class="cards-summary">
                <p class="results-info">
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
            <div class="cards-grid">
                {move || {
                    types.get().into_iter().map(|type_info| {
                        let category_class = get_category_class(&type_info.category);

                        view! {
                            <div
                                class=format!("type-card {}", category_class)
                                data-type-id=type_info.id.clone()
                                on:click={
                                    let type_info = type_info.clone();
                                    move |_| handle_card_click(type_info.clone())
                                }
                            >
                                // Card header
                                <div class="type-card__header">
                                    <h3 class="type-card__title">{type_info.name.clone()}</h3>
                                    <span class="type-card__category">
                                        {get_category_icon(&type_info.category)} " " {type_info.category.clone()}
                                    </span>
                                </div>

                                // Certainty indicator
                                <div class="type-card__certainty">
                                    <div class="certainty-bar">
                                        <div
                                            class="certainty-fill"
                                            style=format!("width: {}%", type_info.certainty)
                                        ></div>
                                    </div>
                                    <span class="certainty-text">{type_info.certainty}"%"</span>
                                </div>

                                // Description
                                <p class="type-card__description">{type_info.description.clone()}</p>

                                // Facets
                                <div class="type-card__facets">
                                    {type_info.facets.iter().map(|facet| {
                                        view! {
                                            <span class="facet-badge">{facet.clone()}</span>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>

                                // Flow-sensitive indicator and source
                                <div class="type-card__meta">
                                    <span class="source-tag">{type_info.source.clone()}</span>
                                    <span class=format!("flow-indicator {}", if type_info.flow_sensitive { "active" } else { "" })>
                                        {if type_info.flow_sensitive { "🔄 Flow-sensitive" } else { "📊 Static" }}
                                    </span>
                                </div>

                                // Actions
                                <div class="type-card__actions">
                                    <button
                                        class="action-btn"
                                        title="Подробнее"
                                        on:click={
                                            let type_info = type_info.clone();
                                            move |_| handle_card_click(type_info.clone())
                                        }
                                    >
                                        "👁️"
                                    </button>
                                    <button class="action-btn" title="Скопировать">
                                        "📋"
                                    </button>
                                    <button class="action-btn" title="Связи">
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
                        <div class="empty-state">
                            <div class="empty-state__icon">"🔍"</div>
                            <h3 class="empty-state__title">"Типы не найдены"</h3>
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
        _ => "category-unknown",
    }
    .to_string()
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
