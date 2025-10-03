//! Enhanced search bar component for filtering types

use crate::api::types::*;
use leptos::prelude::*;
use leptos::html;
use web_sys::Event;
use wasm_bindgen::JsCast;

/// Компонент строки поиска с фильтрами
#[component]
#[allow(non_snake_case)]
pub fn SearchBar(
    /// Текущие фильтры
    #[prop(into)] filters: RwSignal<TypeFilters>,
    /// Обработчик изменения фильтров
    #[prop(optional)] on_filters_change: Option<Callback<TypeFilters>>,
    /// Плейсхолдер для поля поиска
    #[prop(optional, into)] placeholder: Option<String>,
) -> impl IntoView {
    let search_input_ref = NodeRef::<html::Input>::new();
    
    let placeholder_text = placeholder.unwrap_or_else(|| "Поиск типов... (например: Массив, Справочники, Строка)".to_string());

    let handle_search_input = move |_| {
        if let Some(input) = search_input_ref.get() {
            let value = input.value();
            filters.update(|f| {
                f.search_query = if value.is_empty() { None } else { Some(value) };
            });
            
            if let Some(callback) = on_filters_change {
                callback.run(filters.get());
            }
        }
    };

    // Примечание: Фильтры по категориям и certainty теперь управляются через Sidebar
    // с булевыми флагами. Эти обработчики оставлены для обратной совместимости,
    // но в текущей версии интерфейса используются чекбоксы в Sidebar.

    let handle_facet_change = move |ev: Event| {
        let target = ev.target().unwrap();
        let select = target.dyn_into::<web_sys::HtmlSelectElement>().unwrap();
        let value = select.value();
        
        filters.update(|f| {
            f.facet = match value.as_str() {
                "Manager" => Some(FacetKind::Manager),
                "Object" => Some(FacetKind::Object),
                "Reference" => Some(FacetKind::Reference),
                "Collection" => Some(FacetKind::Collection),
                "Metadata" => Some(FacetKind::Metadata),
                _ => None,
            };
        });
        
        if let Some(callback) = on_filters_change {
            callback.run(filters.get());
        }
    };

    let handle_flow_sensitive_change = move |ev: Event| {
        let target = ev.target().unwrap();
        let checkbox = target.dyn_into::<web_sys::HtmlInputElement>().unwrap();
        let checked = checkbox.checked();
        
        filters.update(|f| {
            f.flow_sensitive_only = checked;
        });
        
        if let Some(callback) = on_filters_change {
            callback.run(filters.get());
        }
    };

    view! {
        <div class="search-bar">
            <input
                type="text"
                placeholder=placeholder_text
                node_ref=search_input_ref
                on:input=handle_search_input
            />
        </div>

        // Примечание: Фильтры категорий и certainty теперь находятся в Sidebar
        // Этот компонент SearchBar остался для обратной совместимости,
        // но в текущей версии используется только поле поиска
        <div class="filters-toolbar">
            <div class="filter-group">
                <label>"Фасеты:"</label>
                <select on:change=handle_facet_change>
                    <option value="all">"Все фасеты"</option>
                    <option value="Manager">"Manager"</option>
                    <option value="Object">"Object"</option>
                    <option value="Reference">"Reference"</option>
                    <option value="Collection">"Collection"</option>
                    <option value="Metadata">"Metadata"</option>
                </select>
            </div>

            <div class="filter-group">
                <label>
                    <input
                        type="checkbox"
                        on:change=handle_flow_sensitive_change
                    />
                    " Flow-Sensitive только"
                </label>
            </div>

            <div class="filter-group">
                <button
                    class="filter-reset-btn"
                    on:click=move |_| {
                        filters.set(TypeFilters::default());
                        if let Some(input) = search_input_ref.get() {
                            input.set_value("");
                        }
                        if let Some(callback) = on_filters_change {
                            callback.run(filters.get());
                        }
                    }
                >
                    "Сбросить фильтры"
                </button>
            </div>
        </div>
    }
}

/// Компонент простой строки поиска без дополнительных фильтров
#[component]
#[allow(non_snake_case)]
pub fn SimpleSearchBar(
    /// Текущее значение поиска
    #[prop(into)] search_value: RwSignal<String>,
    /// Обработчик изменения значения поиска
    #[prop(optional)] on_search_change: Option<Callback<String>>,
    /// Плейсхолдер для поля поиска
    #[prop(optional, into)] placeholder: Option<String>,
) -> impl IntoView {
    let search_input_ref = NodeRef::<html::Input>::new();
    
    let placeholder_text = placeholder.unwrap_or_else(|| "Поиск...".to_string());

    let handle_input = move |_| {
        if let Some(input) = search_input_ref.get() {
            let value = input.value();
            search_value.set(value.clone());
            
            if let Some(callback) = on_search_change {
                callback.run(value);
            }
        }
    };

    view! {
        <div class="search-bar">
            <input 
                type="text"
                placeholder=placeholder_text
                node_ref=search_input_ref
                value=move || search_value.get()
                on:input=handle_input
            />
        </div>
    }
}

/// Компонент поиска для header'а
#[component]
#[allow(non_snake_case)]
pub fn HeaderSearchBar(
    /// RwSignal для текста поиска
    #[prop(into)] search_query: RwSignal<String>,
    /// Колбэк для обработки поиска
    on_search: Callback<String>,
) -> impl IntoView {
    let input_ref = NodeRef::<html::Input>::new();

    let handle_input = move |_| {
        if let Some(input) = input_ref.get() {
            let value = input.value();
            search_query.set(value.clone());
            on_search.run(value);
        }
    };

    let on_search_clone = on_search;
    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" {
            if let Some(input) = input_ref.get() {
                input.set_value("");
                search_query.set(String::new());
                on_search_clone.run(String::new());
            }
        }
    };

    view! {
        <input
            type="text"
            placeholder="Поиск типов..."
            node_ref=input_ref
            value=move || search_query.get()
            on:input=handle_input
            on:keydown=handle_keydown
            class="form-control"
        />
    }
}