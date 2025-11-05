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
        <div class="flex gap-6">
            // Graph canvas (placeholder for future SVG implementation)
            <div class="flex-1 relative">
                <div class="bg-bsl-cream-50 dark:bg-bsl-charcoal-900 rounded-lg p-8 min-h-[500px]">
                    <div class="flex flex-col items-center justify-center h-full text-center">
                        <div class="text-6xl mb-4">"🕸️"</div>
                        <h3 class="text-2xl font-bold text-bsl-charcoal-800 dark:text-bsl-cream-100 mb-2">"Граф типов"</h3>
                        <p class="text-bsl-charcoal-600 dark:text-bsl-cream-300 max-w-md">
                            "Интерактивная визуализация связей между типами будет реализована в следующих версиях"
                        </p>

                        // Simple node representation for now
                        <div class="relative w-full h-96">
                            {move || {
                                types.get().into_iter().take(10).enumerate().map(|(index, type_info)| {
                                    let x = 50 + (index % 5) * 150;
                                    let y = 100 + (index / 5) * 100;
                                    let node_classes = get_node_classes(&type_info.category);

                                    view! {
                                        <div
                                            class=node_classes
                                            style=format!("left: {}px; top: {}px", x, y)
                                            on:click={
                                                let type_info = type_info.clone();
                                                move |_| handle_node_click(type_info.clone())
                                            }
                                            title=type_info.description.clone()
                                        >
                                            <div class="text-2xl mb-1">{get_category_icon(&type_info.category)}</div>
                                            <div class="text-sm font-medium">{type_info.name.clone()}</div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()
                            }}
                        </div>
                    </div>
                </div>

                // Graph controls
                <div class="flex gap-2 mt-4">
                    <button
                        class="px-4 py-2 bg-bsl-cream-100 dark:bg-bsl-charcoal-700 text-bsl-charcoal-800 dark:text-bsl-cream-100 rounded-lg hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-600 transition-colors focus:outline-none focus:ring-2 focus:ring-bsl-teal-400"
                        title="Приблизить"
                    >
                        "🔍+"
                    </button>
                    <button
                        class="px-4 py-2 bg-bsl-cream-100 dark:bg-bsl-charcoal-700 text-bsl-charcoal-800 dark:text-bsl-cream-100 rounded-lg hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-600 transition-colors focus:outline-none focus:ring-2 focus:ring-bsl-teal-400"
                        title="Отдалить"
                    >
                        "🔍-"
                    </button>
                    <button
                        class="px-4 py-2 bg-bsl-cream-100 dark:bg-bsl-charcoal-700 text-bsl-charcoal-800 dark:text-bsl-cream-100 rounded-lg hover:bg-bsl-cream-200 dark:hover:bg-bsl-charcoal-600 transition-colors focus:outline-none focus:ring-2 focus:ring-bsl-teal-400"
                        title="Сбросить масштаб"
                    >
                        "🔄"
                    </button>
                </div>
            </div>

            // Node details panel
            <div class="w-80 bg-bsl-cream-50 dark:bg-bsl-charcoal-900 rounded-lg p-6 shadow-md">
                {move || {
                    if let Some(node) = selected_node.get() {
                        view! {
                            <div class="space-y-4">
                                <h3 class="text-xl font-bold text-bsl-charcoal-800 dark:text-bsl-cream-100 mb-4">"Детали узла"</h3>
                                <div class="space-y-3">
                                    <div class="flex flex-col gap-1">
                                        <span class="text-sm font-semibold text-bsl-charcoal-600 dark:text-bsl-cream-300">"Название:"</span>
                                        <span class="text-base text-bsl-charcoal-800 dark:text-bsl-cream-100">{node.name.clone()}</span>
                                    </div>
                                    <div class="flex flex-col gap-1">
                                        <span class="text-sm font-semibold text-bsl-charcoal-600 dark:text-bsl-cream-300">"Категория:"</span>
                                        <span class="text-base text-bsl-charcoal-800 dark:text-bsl-cream-100">
                                            {get_category_icon(&node.category)} " " {node.category.clone()}
                                        </span>
                                    </div>
                                    <div class="flex flex-col gap-1">
                                        <span class="text-sm font-semibold text-bsl-charcoal-600 dark:text-bsl-cream-300">"Определенность:"</span>
                                        <span class="text-base text-bsl-charcoal-800 dark:text-bsl-cream-100">{node.certainty}"%"</span>
                                    </div>
                                    <div class="flex flex-col gap-1">
                                        <span class="text-sm font-semibold text-bsl-charcoal-600 dark:text-bsl-cream-300">"Фасеты:"</span>
                                        <div class="flex flex-wrap gap-2">
                                            {node.facets.iter().map(|facet| {
                                                view! {
                                                    <span class="px-2 py-1 text-xs bg-bsl-teal-100 dark:bg-bsl-teal-900 text-bsl-teal-800 dark:text-bsl-teal-100 rounded-md">{facet.clone()}</span>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                    <div class="flex flex-col gap-1">
                                        <span class="text-sm font-semibold text-bsl-charcoal-600 dark:text-bsl-cream-300">"Описание:"</span>
                                        <p class="text-sm text-bsl-charcoal-700 dark:text-bsl-cream-200">{node.description.clone()}</p>
                                    </div>
                                    <div class="flex flex-col gap-1">
                                        <span class="text-sm font-semibold text-bsl-charcoal-600 dark:text-bsl-cream-300">"Flow-sensitive:"</span>
                                        <span class="text-base text-bsl-charcoal-800 dark:text-bsl-cream-100">
                                            {if node.flow_sensitive { "Да 🔄" } else { "Нет 📊" }}
                                        </span>
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="flex flex-col items-center justify-center h-full text-center">
                                <h3 class="text-xl font-bold text-bsl-charcoal-800 dark:text-bsl-cream-100 mb-2">"Детали узла"</h3>
                                <p class="text-bsl-charcoal-600 dark:text-bsl-cream-300">"Выберите узел для просмотра деталей"</p>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Get Tailwind classes for node with category-specific styling
fn get_node_classes(category: &str) -> String {
    let base_classes = "absolute p-3 rounded-lg shadow-md hover:shadow-xl transition-all duration-200 cursor-pointer border-2 focus:outline-none focus:ring-2 focus:ring-bsl-teal-400";

    let category_classes = match category {
        "Platform" => "bg-blue-50 dark:bg-blue-900 border-blue-300 dark:border-blue-700 text-blue-800 dark:text-blue-100 hover:border-blue-500 dark:hover:border-blue-500",
        "Configuration" => "bg-green-50 dark:bg-green-900 border-green-300 dark:border-green-700 text-green-800 dark:text-green-100 hover:border-green-500 dark:hover:border-green-500",
        "Union" => "bg-purple-50 dark:bg-purple-900 border-purple-300 dark:border-purple-700 text-purple-800 dark:text-purple-100 hover:border-purple-500 dark:hover:border-purple-500",
        "Dynamic" => "bg-yellow-50 dark:bg-yellow-900 border-yellow-300 dark:border-yellow-700 text-yellow-800 dark:text-yellow-100 hover:border-yellow-500 dark:hover:border-yellow-500",
        _ => "bg-gray-50 dark:bg-gray-800 border-gray-300 dark:border-gray-600 text-gray-800 dark:text-gray-100 hover:border-gray-500 dark:hover:border-gray-400",
    };

    format!("{} {}", base_classes, category_classes)
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
