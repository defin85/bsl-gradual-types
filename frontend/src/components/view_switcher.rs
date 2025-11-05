//! View switcher component for switching between different type views

#![allow(clippy::unused_unit)]

use leptos::prelude::*;

/// Доступные представления
#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    Dashboard,
    Cards,
    Table,
    Graph,
}

impl ViewType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ViewType::Dashboard => "dashboard",
            ViewType::Cards => "cards",
            ViewType::Table => "table",
            ViewType::Graph => "graph",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ViewType::Dashboard => "Dashboard",
            ViewType::Cards => "Cards",
            ViewType::Table => "Table",
            ViewType::Graph => "Graph",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ViewType::Dashboard => "📊",
            ViewType::Cards => "🃏",
            ViewType::Table => "📋",
            ViewType::Graph => "🕸️",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ViewType::Dashboard => "Executive Dashboard с метриками",
            ViewType::Cards => "Карточное представление типов",
            ViewType::Table => "Табличный анализ типов",
            ViewType::Graph => "Сетевая визуализация связей",
        }
    }
}

/// Компонент переключателя представлений
#[component]
#[allow(non_snake_case)]
pub fn ViewSwitcher(
    /// Текущее активное представление
    #[prop(into)]
    current_view: RwSignal<ViewType>,
    /// Обработчик изменения представления
    #[prop(optional)]
    on_view_change: Option<std::sync::Arc<dyn Fn(ViewType) + Send + Sync>>,
    /// Компактный режим (только иконки)
    #[prop(optional, default = false)]
    compact: bool,
) -> impl IntoView {
    let views = vec![
        ViewType::Dashboard,
        ViewType::Cards,
        ViewType::Table,
        ViewType::Graph,
    ];

    let handle_view_change = move |view: ViewType| {
        current_view.set(view.clone());
        if let Some(ref handler) = on_view_change {
            handler(view);
        }
    };

    let handle_view_change = std::rc::Rc::new(handle_view_change);

    view! {
        <div class="flex gap-2 bg-bsl-brown-600/8 dark:bg-bsl-gray-400/10 rounded-lg p-1">
            {views.into_iter().map(|view| {
                let view_clone2 = view.clone();
                let view_clone3 = view.clone();
                let button_class = move || {
                    let base = "flex items-center gap-2 px-4 py-2 rounded text-sm font-medium transition-colors duration-fast";
                    if current_view.get() == view_clone2 {
                        format!("{} bg-bsl-cream-100 dark:bg-bsl-charcoal-700 text-bsl-teal-500 shadow-sm", base)
                    } else {
                        format!("{} text-bsl-slate-900 dark:text-bsl-gray-200 hover:bg-bsl-cream-100/50 dark:hover:bg-bsl-charcoal-700/50", base)
                    }
                };
                let handler = handle_view_change.clone();

                view! {
                    <button
                        class=button_class
                        on:click=move |_| handler(view_clone3.clone())
                        title=view.description()
                    >
                        <span class="text-lg">{view.icon()}</span>
                        {if !compact {
                            view! {
                                <span>{view.display_name()}</span>
                            }.into_any()
                        } else {
                            let _: () = view! {};
                            ().into_any()
                        }}
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

/// Компонент расширенного переключателя с описаниями
#[component]
#[allow(non_snake_case)]
pub fn ExtendedViewSwitcher(
    /// Текущее активное представление
    #[prop(into)]
    current_view: RwSignal<ViewType>,
    /// Обработчик изменения представления
    #[prop(optional)]
    on_view_change: Option<std::sync::Arc<dyn Fn(ViewType) + Send + Sync>>,
) -> impl IntoView {
    let views = vec![
        ViewType::Dashboard,
        ViewType::Cards,
        ViewType::Table,
        ViewType::Graph,
    ];

    let handle_view_change = move |view: ViewType| {
        current_view.set(view.clone());
        if let Some(ref handler) = on_view_change {
            handler(view);
        }
    };

    view! {
        <div class="p-6">
            <h3 class="text-lg font-semibold mb-4 text-bsl-slate-900 dark:text-bsl-gray-200">
                "Выберите представление"
            </h3>
            <div class="grid grid-cols-2 gap-3">
                {views.into_iter().map(|view| {
                    let view_clone = view.clone();
                let view_clone2 = view.clone();
                let view_clone3 = view.clone();
                let view_clone4 = view.clone();
                let card_class = move || {
                    let base = "p-4 rounded-lg border cursor-pointer transition-all duration-fast";
                    if current_view.get() == view_clone {
                        format!("{} border-bsl-teal-500 bg-bsl-teal-500/10 dark:bg-bsl-teal-500/20", base)
                    } else {
                        format!("{} border-bsl-brown-600/20 dark:border-bsl-gray-400/30 hover:border-bsl-teal-500/50", base)
                    }
                };
                let handler = handle_view_change.clone();

                view! {
                    <div
                        class=card_class
                        on:click=move |_| handler(view_clone2.clone())
                    >
                        <div class="text-2xl mb-2">{view_clone3.icon()}</div>
                        <div class="font-medium text-bsl-slate-900 dark:text-bsl-gray-200">{view_clone3.display_name()}</div>
                        <div class="text-xs text-bsl-slate-500/70 dark:text-bsl-gray-300/70 mt-1">{view_clone3.description()}</div>
                        {if current_view.get() == view_clone4 {
                            view! {
                                <div class="mt-2 inline-block px-2 py-0.5 bg-bsl-teal-500 text-white text-xs rounded">
                                    "Активно"
                                </div>
                            }.into_any()
                        } else {
                            let _: () = view! {};
                            ().into_any()
                        }}
                    </div>
                }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}

/// Компонент табов для переключения представлений
#[component]
#[allow(non_snake_case)]
pub fn ViewTabs(
    /// Текущее активное представление
    #[prop(into)]
    current_view: RwSignal<ViewType>,
    /// Обработчик изменения представления
    #[prop(optional)]
    on_view_change: Option<std::sync::Arc<dyn Fn(ViewType) + Send + Sync>>,
) -> impl IntoView {
    let views = vec![
        ViewType::Dashboard,
        ViewType::Cards,
        ViewType::Table,
        ViewType::Graph,
    ];

    view! {
        <div class="flex border-b border-bsl-brown-600/20 dark:border-bsl-gray-400/30">
            {views.into_iter().map(|view| {
                let view_for_click = view.clone();
                let view_for_class = view.clone();
                let view_for_icon = view.clone();
                let view_for_label = view.clone();
                let on_view_change_clone = on_view_change.clone();

                let tab_class = move || {
                    let base = "flex items-center gap-2 px-4 py-2 font-medium text-sm transition-colors duration-fast";
                    if current_view.get() == view_for_class {
                        format!("{} text-bsl-teal-500 border-b-2 border-bsl-teal-500", base)
                    } else {
                        format!("{} text-bsl-slate-500/70 dark:text-bsl-gray-300/70 hover:text-bsl-slate-900 dark:hover:text-bsl-gray-200", base)
                    }
                };

                view! {
                    <button
                        class=tab_class
                        on:click=move |_| {
                            current_view.set(view_for_click.clone());
                            if let Some(ref handler) = on_view_change_clone {
                                handler(view_for_click.clone());
                            }
                        }
                    >
                        <span>{view_for_icon.icon()}</span>
                        <span>{view_for_label.display_name()}</span>
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

/// Компонент дропдауна для выбора представления
#[component]
#[allow(non_snake_case)]
pub fn ViewDropdown(
    /// Текущее активное представление
    #[prop(into)]
    current_view: RwSignal<ViewType>,
    /// Обработчик изменения представления
    #[prop(optional)]
    on_view_change: Option<std::sync::Arc<dyn Fn(ViewType) + Send + Sync>>,
) -> impl IntoView {
    let is_open = RwSignal::new(false);

    let toggle_dropdown = move |_| {
        is_open.update(|open| *open = !*open);
    };

    view! {
        <div class="relative inline-block">
            <button
                class="flex items-center gap-2 px-4 py-2 bg-bsl-cream-100 dark:bg-bsl-charcoal-700 border border-bsl-brown-600/20 dark:border-bsl-gray-400/30 rounded hover:bg-bsl-cream-50 dark:hover:bg-bsl-charcoal-600 transition-colors"
                on:click=toggle_dropdown
            >
                <span>{move || current_view.get().icon()}</span>
                <span>{move || current_view.get().display_name()}</span>
                <span>{move || if is_open.get() { "▲" } else { "▼" }}</span>
            </button>

            {move || {
                let on_view_change_clone: Option<std::sync::Arc<dyn Fn(ViewType) + Send + Sync>> = on_view_change.as_ref().map(|arc| arc.clone());
                if is_open.get() {
                    view! {
                        <div class="absolute right-0 mt-2 w-64 bg-bsl-cream-100 dark:bg-bsl-charcoal-700 border border-bsl-brown-600/20 dark:border-bsl-gray-400/30 rounded-lg shadow-lg z-10">
                            <button
                                class=move || {
                                    let base = "flex items-center gap-3 w-full text-left px-4 py-3 transition-colors duration-fast";
                                    if current_view.get() == ViewType::Dashboard {
                                        format!("{} bg-bsl-teal-500/10 dark:bg-bsl-teal-500/20 text-bsl-teal-500", base)
                                    } else {
                                        format!("{} hover:bg-bsl-brown-600/8 dark:hover:bg-bsl-gray-400/10", base)
                                    }
                                }
                                on:click={
                                    let handler_clone = on_view_change_clone.clone();
                                    move |_| {
                                        current_view.set(ViewType::Dashboard);
                                        is_open.set(false);
                                        if let Some(ref handler) = handler_clone {
                                            handler(ViewType::Dashboard);
                                        }
                                    }
                                }
                            >
                                <span class="text-xl">{ViewType::Dashboard.icon()}</span>
                                <div class="flex-1">
                                    <div class="font-medium text-bsl-slate-900 dark:text-bsl-gray-200">{ViewType::Dashboard.display_name()}</div>
                                    <div class="text-xs text-bsl-slate-500/70 dark:text-bsl-gray-300/70">{ViewType::Dashboard.description()}</div>
                                </div>
                                {move || if current_view.get() == ViewType::Dashboard {
                                    view! { <span class="text-bsl-teal-500">"✓"</span> }.into_any()
                                } else {
                                    let _: () = view! {};
                                    ().into_any()
                                }}
                            </button>

                            <button
                                class=move || {
                                    let base = "flex items-center gap-3 w-full text-left px-4 py-3 transition-colors duration-fast";
                                    if current_view.get() == ViewType::Cards {
                                        format!("{} bg-bsl-teal-500/10 dark:bg-bsl-teal-500/20 text-bsl-teal-500", base)
                                    } else {
                                        format!("{} hover:bg-bsl-brown-600/8 dark:hover:bg-bsl-gray-400/10", base)
                                    }
                                }
                                on:click={
                                    let handler_clone = on_view_change_clone.clone();
                                    move |_| {
                                        current_view.set(ViewType::Cards);
                                        is_open.set(false);
                                        if let Some(ref handler) = handler_clone {
                                            handler(ViewType::Cards);
                                        }
                                    }
                                }
                            >
                                <span class="text-xl">{ViewType::Cards.icon()}</span>
                                <div class="flex-1">
                                    <div class="font-medium text-bsl-slate-900 dark:text-bsl-gray-200">{ViewType::Cards.display_name()}</div>
                                    <div class="text-xs text-bsl-slate-500/70 dark:text-bsl-gray-300/70">{ViewType::Cards.description()}</div>
                                </div>
                                {move || if current_view.get() == ViewType::Cards {
                                    view! { <span class="text-bsl-teal-500">"✓"</span> }.into_any()
                                } else {
                                    let _: () = view! {};
                                    ().into_any()
                                }}
                            </button>

                            <button
                                class=move || {
                                    let base = "flex items-center gap-3 w-full text-left px-4 py-3 transition-colors duration-fast";
                                    if current_view.get() == ViewType::Table {
                                        format!("{} bg-bsl-teal-500/10 dark:bg-bsl-teal-500/20 text-bsl-teal-500", base)
                                    } else {
                                        format!("{} hover:bg-bsl-brown-600/8 dark:hover:bg-bsl-gray-400/10", base)
                                    }
                                }
                                on:click={
                                    let handler_clone = on_view_change_clone.clone();
                                    move |_| {
                                        current_view.set(ViewType::Table);
                                        is_open.set(false);
                                        if let Some(ref handler) = handler_clone {
                                            handler(ViewType::Table);
                                        }
                                    }
                                }
                            >
                                <span class="text-xl">{ViewType::Table.icon()}</span>
                                <div class="flex-1">
                                    <div class="font-medium text-bsl-slate-900 dark:text-bsl-gray-200">{ViewType::Table.display_name()}</div>
                                    <div class="text-xs text-bsl-slate-500/70 dark:text-bsl-gray-300/70">{ViewType::Table.description()}</div>
                                </div>
                                {move || if current_view.get() == ViewType::Table {
                                    view! { <span class="text-bsl-teal-500">"✓"</span> }.into_any()
                                } else {
                                    let _: () = view! {};
                                    ().into_any()
                                }}
                            </button>

                            <button
                                class=move || {
                                    let base = "flex items-center gap-3 w-full text-left px-4 py-3 transition-colors duration-fast";
                                    if current_view.get() == ViewType::Graph {
                                        format!("{} bg-bsl-teal-500/10 dark:bg-bsl-teal-500/20 text-bsl-teal-500", base)
                                    } else {
                                        format!("{} hover:bg-bsl-brown-600/8 dark:hover:bg-bsl-gray-400/10", base)
                                    }
                                }
                                on:click={
                                    let handler_clone = on_view_change_clone.clone();
                                    move |_| {
                                        current_view.set(ViewType::Graph);
                                        is_open.set(false);
                                        if let Some(ref handler) = handler_clone {
                                            handler(ViewType::Graph);
                                        }
                                    }
                                }
                            >
                                <span class="text-xl">{ViewType::Graph.icon()}</span>
                                <div class="flex-1">
                                    <div class="font-medium text-bsl-slate-900 dark:text-bsl-gray-200">{ViewType::Graph.display_name()}</div>
                                    <div class="text-xs text-bsl-slate-500/70 dark:text-bsl-gray-300/70">{ViewType::Graph.description()}</div>
                                </div>
                                {move || if current_view.get() == ViewType::Graph {
                                    view! { <span class="text-bsl-teal-500">"✓"</span> }.into_any()
                                } else {
                                    let _: () = view! {};
                                    ().into_any()
                                }}
                            </button>
                        </div>
                    }.into_any()
                } else {
                    let _: () = view! {};
                    ().into_any()
                }
            }}
        </div>
    }
}
