//! Type card component for displaying type information

use crate::api::*;
use leptos::prelude::*;
use leptos::For;

/// Helper function for type card classes based on category
fn type_card_classes(category: &str) -> String {
    let base = "rounded-xl shadow-md hover:shadow-xl hover:-translate-y-1 transition-all duration-300 cursor-pointer p-6 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-900";
    let variant = match category {
        "Platform" | "Configuration" => "bg-green-50 dark:bg-green-950 border-2 border-green-200 dark:border-green-800 hover:border-green-300 dark:hover:border-green-700",
        "Union" => "bg-amber-50 dark:bg-amber-950 border-2 border-amber-200 dark:border-amber-800 hover:border-amber-300 dark:hover:border-amber-700",
        "Dynamic" => "bg-red-50 dark:bg-red-950 border-2 border-red-200 dark:border-red-800 hover:border-red-300 dark:hover:border-red-700",
        _ => "bg-gray-50 dark:bg-gray-950 border-2 border-gray-200 dark:border-gray-800 hover:border-gray-300 dark:hover:border-gray-700",
    };
    format!("{} {}", base, variant)
}

/// Helper function for certainty badge classes
fn certainty_badge_classes(certainty: u8) -> String {
    let base = "px-3 py-1 rounded-full text-xs font-semibold";
    let variant = match certainty {
        90..=100 => "bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200",
        50..=89 => "bg-amber-100 dark:bg-amber-900 text-amber-800 dark:text-amber-200",
        _ => "bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-200",
    };
    format!("{} {}", base, variant)
}

/// Helper function for facet tag classes
fn facet_tag_classes(facet: &str) -> &'static str {
    match facet {
        "Manager" => "inline-block px-2 py-1 mr-2 mb-2 text-xs font-medium rounded bg-indigo-100 dark:bg-indigo-900 text-indigo-800 dark:text-indigo-200",
        "Object" => "inline-block px-2 py-1 mr-2 mb-2 text-xs font-medium rounded bg-purple-100 dark:bg-purple-900 text-purple-800 dark:text-purple-200",
        "Reference" => "inline-block px-2 py-1 mr-2 mb-2 text-xs font-medium rounded bg-pink-100 dark:bg-pink-900 text-pink-800 dark:text-pink-200",
        "Selection" => "inline-block px-2 py-1 mr-2 mb-2 text-xs font-medium rounded bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200",
        "List" => "inline-block px-2 py-1 mr-2 mb-2 text-xs font-medium rounded bg-teal-100 dark:bg-teal-900 text-teal-800 dark:text-teal-200",
        _ => "inline-block px-2 py-1 mr-2 mb-2 text-xs font-medium rounded bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200",
    }
}

/// Helper function for section container classes
fn section_container_classes(section_type: &str) -> &'static str {
    match section_type {
        "methods" => "bg-blue-50/50 dark:bg-blue-950/20 p-3 rounded-lg border-l-4 border-blue-500 mb-4",
        "tabular" => "bg-cyan-50/50 dark:bg-cyan-950/20 p-3 rounded-lg border-l-4 border-cyan-500 mb-4",
        "flow" => "bg-purple-50 dark:bg-purple-950/30 p-4 rounded-lg border-l-4 border-purple-500 mb-4",
        "facets" => "bg-blue-50 dark:bg-blue-950/30 p-4 rounded-lg mb-4",
        _ => "p-3 rounded-lg mb-4",
    }
}

/// Helper function for remaining items badge
fn remaining_badge_classes() -> &'static str {
    "inline-block px-3 py-1 mr-2 mb-2 bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-300 text-xs font-medium rounded shadow-sm"
}

/// Helper function for cards grid container classes
fn cards_grid_classes() -> &'static str {
    "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 dark:gap-4"
}

/// Компонент карточки типа
#[component]
#[allow(non_snake_case)]
pub fn TypeCard(
    /// Информация о типе
    #[prop(into)]
    type_info: Signal<TypeInfo>,
    /// Обработчик клика по карточке
    #[prop(optional)]
    on_click: Option<Callback<TypeInfo>>,
) -> impl IntoView {
    let card_class = move || {
        let info = type_info.get();
        type_card_classes(&info.category)
    };

    let certainty_badge_class = move || {
        let info = type_info.get();
        certainty_badge_classes(info.certainty)
    };

    let handle_click = move |_| {
        if let Some(handler) = on_click {
            handler.run(type_info.get());
        }
    };

    view! {
        <div
            class=card_class
            on:click=handle_click
            on:keydown=move |ev| {
                if ev.key() == "Enter" || ev.key() == " " {
                    ev.prevent_default();
                    if let Some(handler) = on_click {
                        handler.run(type_info.get());
                    }
                }
            }
            role="button"
            tabindex="0"
            aria-label=move || format!("Открыть подробности типа {}", type_info.get().name)
        >
            {move || {
                let info = type_info.get();

                view! {
                    <div class="flex items-center justify-between mb-4 pb-3 border-b border-gray-200 dark:border-gray-700">
                        <div class="text-xl font-bold text-gray-900 dark:text-white">{info.name.clone()}</div>
                        <div class={certainty_badge_classes(info.certainty)}>
                            {info.certainty_percentage()}
                        </div>
                    </div>

                    <div class="space-y-2 mb-4">
                        <div class="flex justify-between items-center py-1">
                            <span class="text-sm text-gray-600 dark:text-gray-400 font-medium">"Категория:"</span>
                            <span class="text-sm text-gray-900 dark:text-white">{info.category.clone()}</span>
                        </div>
                        <div class="flex justify-between items-center py-1">
                            <span class="text-sm text-gray-600 dark:text-gray-400 font-medium">"Источник:"</span>
                            <span class="text-sm text-gray-900 dark:text-white">{info.source.clone()}</span>
                        </div>
                        <div class="flex justify-between items-center py-1">
                            <span class="text-sm text-gray-600 dark:text-gray-400 font-medium">"Уверенность:"</span>
                            <span class="text-sm text-gray-900 dark:text-white">{info.certainty_text.clone()}</span>
                        </div>
                    </div>

                    <div class={section_container_classes("facets")}>
                        <strong class="text-gray-900 dark:text-white">"Доступные фасеты:"</strong><br/>
                        {info.facets.into_iter().map(|facet| {
                            let facet_class = facet_tag_classes(&facet);
                            view! {
                                <span class=facet_class>
                                    {facet.clone()}
                                </span>
                            }
                        }).collect::<Vec<_>>()}
                    </div>

                    {if !info.methods.is_empty() {
                        let methods_to_show: Vec<MethodInfo> = info.methods.iter().take(5).cloned().collect();
                        let has_more = info.methods.len() > 5;
                        let remaining_count = if has_more { info.methods.len() - 5 } else { 0 };
                        let methods_len = info.methods.len();

                        view! {
                            <div class={section_container_classes("methods")}>
                                <strong class="text-gray-900 dark:text-white">"📋 Методы (" {methods_len} "):"</strong><br/>
                                <div class="mt-2">
                                    {methods_to_show.into_iter().map(|method| {
                                        view! {
                                            <span class="inline-block px-3 py-1 mr-2 mb-2 bg-blue-600 dark:bg-blue-700 text-white text-xs font-medium rounded shadow-sm hover:bg-blue-700 dark:hover:bg-blue-600 transition-colors">
                                                {method.name.clone()}
                                            </span>
                                        }
                                    }).collect::<Vec<_>>()}
                                    {if has_more {
                                        view! {
                                            <span class={remaining_badge_classes()}>
                                                "+" {remaining_count} " ещё"
                                            </span>
                                        }.into_any()
                                    } else {
                                        let _: () = view! {};
                                        ().into_any()
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        let _: () = view! {};
                        ().into_any()
                    }}

                    {if !info.tabular_sections.is_empty() {
                        let sections_to_show: Vec<String> = info.tabular_sections.iter()
                            .take(3)
                            .map(|ts| ts.name.clone())
                            .collect();
                        let has_more = info.tabular_sections.len() > 3;
                        let remaining_count = if has_more { info.tabular_sections.len() - 3 } else { 0 };
                        let sections_len = info.tabular_sections.len();

                        view! {
                            <div class={section_container_classes("tabular")}>
                                <strong class="text-gray-900 dark:text-white">"📋 Табличные части (" {sections_len} "):"</strong><br/>
                                <div class="mt-2">
                                    {sections_to_show.into_iter().map(|section_name| {
                                        view! {
                                            <span class="inline-block px-3 py-1 mr-2 mb-2 bg-cyan-100 dark:bg-cyan-900 text-cyan-800 dark:text-cyan-200 text-xs font-medium rounded shadow-sm">
                                                {section_name}
                                            </span>
                                        }
                                    }).collect::<Vec<_>>()}
                                    {if has_more {
                                        view! {
                                            <span class={remaining_badge_classes()}>
                                                "+" {remaining_count} " ещё"
                                            </span>
                                        }.into_any()
                                    } else {
                                        let _: () = view! {};
                                        ().into_any()
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        let _: () = view! {};
                        ().into_any()
                    }}

                    {if info.is_flow_sensitive() {
                        view! {
                            <div class={section_container_classes("flow")}>
                                <strong class="text-gray-900 dark:text-white">"🔄 Flow-Sensitive Analysis"</strong><br/>
                                <span class="text-sm text-purple-700 dark:text-purple-300">"Тип может изменяться в ходе выполнения"</span>
                            </div>
                        }.into_any()
                    } else {
                        let _: () = view! {};
                        ().into_any()
                    }}

                    <div class="mt-4">
                        <div class="bg-blue-50 dark:bg-blue-950/30 p-3 rounded-lg border-l-4 border-blue-500">
                            <small class="text-gray-900 dark:text-white"><strong>"Описание:"</strong>" " {info.description.clone()}</small>
                        </div>
                    </div>
                }
            }}
        </div>
    }
}

/// Компонент сетки карточек типов
#[component]
#[allow(non_snake_case)]
pub fn TypeCardsGrid(
    /// Список типов для отображения
    #[prop(into)]
    types: Signal<Vec<TypeInfo>>,
    /// Обработчик клика по карточке
    #[prop(optional)]
    on_card_click: Option<Callback<TypeInfo>>,
) -> impl IntoView {
    view! {
        <div class={cards_grid_classes()}>
            <For
                each=move || types.get()
                key=|type_info| type_info.id.clone()
                children=move |type_info: TypeInfo| {
                    let type_signal = Signal::derive(move || type_info.clone());
                    let click_handler = on_card_click;

                    match click_handler {
                        Some(handler) => view! {
                            <TypeCard
                                type_info=type_signal
                                on_click=handler
                            />
                        }.into_view(),
                        None => view! {
                            <TypeCard
                                type_info=type_signal
                            />
                        }.into_view(),
                    }
                }
            />
        </div>
    }
}
