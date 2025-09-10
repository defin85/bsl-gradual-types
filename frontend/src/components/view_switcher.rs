//! View switcher component for switching between different type views

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
    #[prop(into)] current_view: RwSignal<ViewType>,
    /// Обработчик изменения представления
    #[prop(optional)] on_view_change: Option<std::sync::Arc<dyn Fn(ViewType) + Send + Sync>>,
    /// Компактный режим (только иконки)
    #[prop(optional, default = false)] compact: bool,
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
        <div class="view-switcher">
            {views.into_iter().map(|view| {
                let view_clone = view.clone();
                let view_clone2 = view.clone();
                let view_clone3 = view.clone();
                let _is_active = move || current_view.get() == view_clone;
                let button_class = move || {
                    if current_view.get() == view_clone2 {
                        "view-btn active"
                    } else {
                        "view-btn"
                    }
                };
                let handler = handle_view_change.clone();

                view! {
                    <button 
                        class=button_class
                        on:click=move |_| handler(view_clone3.clone())
                        title=view.description()
                    >
                        <span class="view-icon">{view.icon()}</span>
                        {if !compact {
                            view! {
                                <span class="view-label">{view.display_name()}</span>
                            }.into_any()
                        } else {
                            view! {}.into_any()
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
    #[prop(into)] current_view: RwSignal<ViewType>,
    /// Обработчик изменения представления
    #[prop(optional)] on_view_change: Option<std::sync::Arc<dyn Fn(ViewType) + Send + Sync>>,
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
        <div class="extended-view-switcher">
            <h3>"Выберите представление"</h3>
            <div class="view-options">
                {views.into_iter().map(|view| {
                    let view_clone = view.clone();
                let view_clone2 = view.clone();
                let view_clone3 = view.clone();
                let view_clone4 = view.clone();
                let card_class = move || {
                    if current_view.get() == view_clone {
                        "view-option-card active"
                    } else {
                        "view-option-card"
                    }
                };
                let handler = handle_view_change.clone();

                view! {
                    <div 
                        class=card_class
                        on:click=move |_| handler(view_clone2.clone())
                    >
                        <div class="view-option-icon">{view_clone3.icon()}</div>
                        <div class="view-option-title">{view_clone3.display_name()}</div>
                        <div class="view-option-description">{view_clone3.description()}</div>
                        {if current_view.get() == view_clone4 {
                            view! {
                                <div class="view-option-badge">"Активно"</div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
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
    #[prop(into)] current_view: RwSignal<ViewType>,
    /// Обработчик изменения представления
    #[prop(optional)] on_view_change: Option<std::sync::Arc<dyn Fn(ViewType) + Send + Sync>>,
) -> impl IntoView {
    let views = vec![
        ViewType::Dashboard,
        ViewType::Cards,
        ViewType::Table,
        ViewType::Graph,
    ];

    view! {
        <div class="view-tabs">
            {views.into_iter().map(|view| {
                let view_for_click = view.clone();
                let view_for_class = view.clone();
                let view_for_icon = view.clone();
                let view_for_label = view.clone();
                let on_view_change_clone = on_view_change.clone();
                
                let tab_class = move || {
                    if current_view.get() == view_for_class {
                        "view-tab active"
                    } else {
                        "view-tab"
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
                        <span class="tab-icon">{view_for_icon.icon()}</span>
                        <span class="tab-label">{view_for_label.display_name()}</span>
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
    #[prop(into)] current_view: RwSignal<ViewType>,
    /// Обработчик изменения представления
    #[prop(optional)] on_view_change: Option<std::sync::Arc<dyn Fn(ViewType) + Send + Sync>>,
) -> impl IntoView {
    let is_open = RwSignal::new(false);
    
    let toggle_dropdown = move |_| {
        is_open.update(|open| *open = !*open);
    };

    view! {
        <div class="view-dropdown">
            <button class="dropdown-trigger" on:click=toggle_dropdown>
                <span class="current-view-icon">{move || current_view.get().icon()}</span>
                <span class="current-view-label">{move || current_view.get().display_name()}</span>
                <span class="dropdown-arrow">{move || if is_open.get() { "▲" } else { "▼" }}</span>
            </button>
            
            {move || {
                let on_view_change_clone: Option<std::sync::Arc<dyn Fn(ViewType) + Send + Sync>> = on_view_change.as_ref().map(|arc| arc.clone());
                if is_open.get() {
                    view! {
                        <div class="dropdown-menu">
                            <button 
                                class=move || if current_view.get() == ViewType::Dashboard { "dropdown-item current" } else { "dropdown-item" }
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
                                <span class="item-icon">{ViewType::Dashboard.icon()}</span>
                                <div class="item-content">
                                    <div class="item-title">{ViewType::Dashboard.display_name()}</div>
                                    <div class="item-description">{ViewType::Dashboard.description()}</div>
                                </div>
                                {move || if current_view.get() == ViewType::Dashboard {
                                    view! { <span class="item-check">"✓"</span> }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                            </button>
                            
                            <button 
                                class=move || if current_view.get() == ViewType::Cards { "dropdown-item current" } else { "dropdown-item" }
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
                                <span class="item-icon">{ViewType::Cards.icon()}</span>
                                <div class="item-content">
                                    <div class="item-title">{ViewType::Cards.display_name()}</div>
                                    <div class="item-description">{ViewType::Cards.description()}</div>
                                </div>
                                {move || if current_view.get() == ViewType::Cards {
                                    view! { <span class="item-check">"✓"</span> }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                            </button>
                            
                            <button 
                                class=move || if current_view.get() == ViewType::Table { "dropdown-item current" } else { "dropdown-item" }
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
                                <span class="item-icon">{ViewType::Table.icon()}</span>
                                <div class="item-content">
                                    <div class="item-title">{ViewType::Table.display_name()}</div>
                                    <div class="item-description">{ViewType::Table.description()}</div>
                                </div>
                                {move || if current_view.get() == ViewType::Table {
                                    view! { <span class="item-check">"✓"</span> }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                            </button>
                            
                            <button 
                                class=move || if current_view.get() == ViewType::Graph { "dropdown-item current" } else { "dropdown-item" }
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
                                <span class="item-icon">{ViewType::Graph.icon()}</span>
                                <div class="item-content">
                                    <div class="item-title">{ViewType::Graph.display_name()}</div>
                                    <div class="item-description">{ViewType::Graph.description()}</div>
                                </div>
                                {move || if current_view.get() == ViewType::Graph {
                                    view! { <span class="item-check">"✓"</span> }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                            </button>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </div>
    }
}