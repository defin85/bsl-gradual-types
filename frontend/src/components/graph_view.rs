//! Graph view component for type relationships visualization

use crate::api::*;
use leptos::prelude::*;

/// Graph view for visualizing type relationships
#[component]
#[allow(non_snake_case)]
pub fn GraphView(
    /// Types signal
    types: Signal<Vec<TypeInfo>>,
) -> impl IntoView {
    let selected_node = RwSignal::new(None::<TypeInfo>);

    let handle_node_click = move |type_info: TypeInfo| {
        selected_node.set(Some(type_info.clone()));
        web_sys::console::log_1(&format!("Selected node: {}", type_info.name).into());
    };

    view! {
        <div class="graph-view">
            // Graph canvas (placeholder for future SVG implementation)
            <div class="graph-container">
                <div class="graph-canvas">
                    <div class="graph-placeholder">
                        <div class="graph-placeholder__icon">"🕸️"</div>
                        <h3 class="graph-placeholder__title">"Граф типов"</h3>
                        <p class="graph-placeholder__description">
                            "Интерактивная визуализация связей между типами будет реализована в следующих версиях"
                        </p>

                        // Simple node representation for now
                        <div class="simple-nodes">
                            {move || {
                                types.get().into_iter().take(10).enumerate().map(|(index, type_info)| {
                                    let x = 50 + (index % 5) * 150;
                                    let y = 100 + (index / 5) * 100;
                                    let category_class = get_category_class(&type_info.category);

                                    view! {
                                        <div
                                            class=format!("graph-node {}", category_class)
                                            style=format!("left: {}px; top: {}px", x, y)
                                            on:click={
                                                let type_info = type_info.clone();
                                                move |_| handle_node_click(type_info.clone())
                                            }
                                            title=type_info.description.clone()
                                        >
                                            <div class="node-icon">{get_category_icon(&type_info.category)}</div>
                                            <div class="node-label">{type_info.name.clone()}</div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()
                            }}
                        </div>
                    </div>
                </div>

                // Graph controls
                <div class="graph-controls">
                    <button class="btn btn--sm" title="Приблизить">
                        "🔍+"
                    </button>
                    <button class="btn btn--sm" title="Отдалить">
                        "🔍-"
                    </button>
                    <button class="btn btn--sm" title="Сбросить масштаб">
                        "🔄"
                    </button>
                </div>
            </div>

            // Node details panel
            <div class="graph-details">
                {move || {
                    if let Some(node) = selected_node.get() {
                        view! {
                            <div class="node-details">
                                <h3 class="node-details__title">"Детали узла"</h3>
                                <div class="node-details__content">
                                    <div class="detail-item">
                                        <span class="detail-label">"Название:"</span>
                                        <span class="detail-value">{node.name.clone()}</span>
                                    </div>
                                    <div class="detail-item">
                                        <span class="detail-label">"Категория:"</span>
                                        <span class="detail-value">
                                            {get_category_icon(&node.category)} " " {node.category.clone()}
                                        </span>
                                    </div>
                                    <div class="detail-item">
                                        <span class="detail-label">"Определенность:"</span>
                                        <span class="detail-value">{node.certainty}"%"</span>
                                    </div>
                                    <div class="detail-item">
                                        <span class="detail-label">"Фасеты:"</span>
                                        <div class="detail-facets">
                                            {node.facets.iter().map(|facet| {
                                                view! {
                                                    <span class="facet-tag">{facet.clone()}</span>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                    <div class="detail-item">
                                        <span class="detail-label">"Описание:"</span>
                                        <p class="detail-description">{node.description.clone()}</p>
                                    </div>
                                    <div class="detail-item">
                                        <span class="detail-label">"Flow-sensitive:"</span>
                                        <span class="detail-value">
                                            {if node.flow_sensitive { "Да 🔄" } else { "Нет 📊" }}
                                        </span>
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="node-details-placeholder">
                                <h3>"Детали узла"</h3>
                                <p>"Выберите узел для просмотра деталей"</p>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
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
