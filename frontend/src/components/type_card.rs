//! Type card component for displaying type information

use crate::api::types::*;
use leptos::prelude::*;

/// Компонент карточки типа
#[component]
#[allow(non_snake_case)]
pub fn TypeCard(
    /// Информация о типе
    #[prop(into)] type_info: Signal<TypeInfo>,
    /// Обработчик клика по карточке
    #[prop(optional)] on_click: Option<Callback<TypeInfo>>,
) -> impl IntoView {
    let card_class = move || {
        let info = type_info.get();
        match info.category {
            TypeCategory::Platform => "type-card card-known",
            TypeCategory::Configuration => "type-card card-known", 
            TypeCategory::Union => "type-card card-inferred",
            TypeCategory::Dynamic => "type-card card-unknown",
        }
    };

    let certainty_badge_class = move || {
        let info = type_info.get();
        match info.certainty {
            Certainty::Known => "certainty-badge badge-known",
            Certainty::Inferred(_) => "certainty-badge badge-inferred",
            Certainty::Unknown => "certainty-badge badge-unknown",
        }
    };

    let handle_click = move |_| {
        if let Some(handler) = on_click {
            handler.run(type_info.get());
        }
    };

    view! {
        <div class=card_class on:click=handle_click>
            <div class="type-header">
                <div class="type-name">{move || type_info.get().display_name}</div>
                <div class=certainty_badge_class>
                    {move || type_info.get().certainty.as_percentage()}
                </div>
            </div>

            <div class="type-details">
                <div class="detail-row">
                    <span class="detail-label">"Категория:"</span>
                    <span class="detail-value">{move || type_info.get().category.as_str()}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">"Источник:"</span>
                    <span class="detail-value">{move || type_info.get().source.clone()}</span>
                </div>
                {move || {
                    let info = type_info.get();
                    if let Some(methods) = info.methods_count {
                        view! {
                            <div class="detail-row">
                                <span class="detail-label">"Методы:"</span>
                                <span class="detail-value">{format!("{} методов", methods)}</span>
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }
                }}
            </div>

            <div class="facets-section">
                <strong>"Доступные фасеты:"</strong><br/>
                {move || {
                    type_info.get().facets.into_iter().map(|facet| {
                        let facet_class = format!("facet-tag facet-{}", facet.as_str().to_lowercase());
                        view! {
                            <span class=facet_class style=move || format!("background: {}; color: white;", facet.color())>
                                {facet.as_str()}
                            </span>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            {move || {
                let info = type_info.get();
                if let Some(union_types) = info.union_types {
                    view! {
                        <div class="union-types">
                            <strong>"Возможные типы:"</strong>
                            {union_types.into_iter().map(|weighted_type| {
                                view! {
                                    <div class="union-type">
                                        <span>{weighted_type.type_name}</span>
                                        <div class="weight-bar">
                                            <div class="weight-fill" style=move || format!("width: {}%", weighted_type.weight * 100.0)></div>
                                        </div>
                                        <span>{format!("{:.0}%", weighted_type.weight * 100.0)}</span>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            {move || {
                let info = type_info.get();
                if info.is_flow_sensitive {
                    view! {
                        <div class="flow-sensitive">
                            <strong>"🔄 Flow-Sensitive Analysis"</strong><br/>
                            <span class="flow-step">"Init: Неопределено"</span>
                            <span class="flow-step">"Check: " {info.category.as_str()}</span>
                            <span class="flow-step">"Final: " {info.category.as_str()}</span>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            {move || {
                let info = type_info.get();
                if let Some(description) = info.description {
                    let style_class = match info.category {
                        TypeCategory::Platform => "background: #e8f5e8; padding: 10px; border-radius: 6px;",
                        TypeCategory::Configuration => "background: #fff3cd; padding: 10px; border-radius: 6px; border-left: 3px solid #ffc107;",
                        TypeCategory::Union => "background: #d1ecf1; padding: 10px; border-radius: 6px; margin-top: 10px;",
                        TypeCategory::Dynamic => "background: #f8d7da; padding: 10px; border-radius: 6px; border-left: 3px solid #dc3545;",
                    };
                    
                    let prefix = match info.category {
                        TypeCategory::Platform => "Популярные методы:",
                        TypeCategory::Configuration => "1C Специфика:",
                        TypeCategory::Union => "💡 Рекомендация:",
                        TypeCategory::Dynamic => "⚠️ Требует runtime проверки:",
                    };
                    
                    view! {
                        <div style=style_class>
                            <small><strong>{prefix}</strong>" " {description}</small>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
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
    #[prop(into)] types: Signal<Vec<TypeInfo>>,
    /// Обработчик клика по карточке
    #[prop(optional)] on_card_click: Option<Callback<TypeInfo>>,
) -> impl IntoView {
    view! {
        <div class="cards-grid">
            {move || {
                types.get().into_iter().map(|type_info| {
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
                }).collect::<Vec<_>>()
            }}
        </div>
    }
}