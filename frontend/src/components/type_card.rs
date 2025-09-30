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
        match info.get_category() {
            TypeCategory::Platform => "type-card card-known",
            TypeCategory::Configuration => "type-card card-known",
            TypeCategory::Union => "type-card card-inferred",
            TypeCategory::Dynamic => "type-card card-unknown",
        }
    };

    let certainty_badge_class = move || {
        let info = type_info.get();
        match info.get_certainty() {
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
                <div class="type-name">{move || type_info.get().name.clone()}</div>
                <div class=certainty_badge_class>
                    {move || type_info.get().get_certainty().as_percentage()}
                </div>
            </div>

            <div class="type-details">
                <div class="detail-row">
                    <span class="detail-label">"Категория:"</span>
                    <span class="detail-value">{move || type_info.get().category.clone()}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">"Источник:"</span>
                    <span class="detail-value">{move || type_info.get().source.clone()}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">"Уверенность:"</span>
                    <span class="detail-value">{move || type_info.get().certainty_text.clone()}</span>
                </div>
            </div>

            <div class="facets-section">
                <strong>"Доступные фасеты:"</strong><br/>
                {move || {
                    type_info.get().facets.into_iter().map(|facet| {
                        let facet_class = format!("facet-tag facet-{}", facet.to_lowercase());
                        view! {
                            <span class=facet_class>
                                {facet.clone()}
                            </span>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            {move || {
                let info = type_info.get();
                if info.is_flow_sensitive() {
                    view! {
                        <div class="flow-sensitive">
                            <strong>"🔄 Flow-Sensitive Analysis"</strong><br/>
                            <span class="flow-step">"Тип может изменяться в ходе выполнения"</span>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            <div class="description-section">
                <div style="background: #f8f9fa; padding: 10px; border-radius: 6px; border-left: 3px solid #007bff;">
                    <small><strong>"Описание:"</strong>" " {move || type_info.get().description.clone()}</small>
                </div>
            </div>
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