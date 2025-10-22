//! Type table component for tabular display of types

use crate::api::*;
use leptos::prelude::*;

/// Колонка для сортировки
#[derive(Debug, Clone, PartialEq)]
pub enum SortColumn {
    Name,
    Category,
    Certainty,
    Source,
    Methods,
}

/// Направление сортировки
#[derive(Debug, Clone, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Компонент таблицы типов
#[component]
#[allow(non_snake_case)]
pub fn TypeTable(
    /// Список типов для отображения
    #[prop(into)]
    types: Signal<Vec<TypeInfo>>,
    /// Обработчик клика по строке
    #[prop(optional)]
    on_row_click: Option<Callback<TypeInfo>>,
    /// Обработчик действий
    #[prop(optional)]
    on_action: Option<Callback<(String, TypeInfo)>>,
) -> impl IntoView {
    let sort_column = RwSignal::new(SortColumn::Name);
    let sort_direction = RwSignal::new(SortDirection::Asc);

    let sorted_types = Signal::derive(move || {
        let mut types_list = types.get();
        let col = sort_column.get();
        let dir = sort_direction.get();

        types_list.sort_by(|a, b| {
            let comparison = match col {
                SortColumn::Name => a.name.cmp(&b.name),
                SortColumn::Category => a.category.cmp(&b.category),
                SortColumn::Certainty => a.certainty.cmp(&b.certainty),
                SortColumn::Source => a.source.cmp(&b.source),
                SortColumn::Methods => {
                    // Since we don't have methods_count in new structure, use 0
                    std::cmp::Ordering::Equal
                }
            };

            match dir {
                SortDirection::Asc => comparison,
                SortDirection::Desc => comparison.reverse(),
            }
        });

        types_list
    });

    let handle_sort = move |column: SortColumn| {
        if sort_column.get() == column {
            // Переключаем направление сортировки
            sort_direction.update(|dir| {
                *dir = match *dir {
                    SortDirection::Asc => SortDirection::Desc,
                    SortDirection::Desc => SortDirection::Asc,
                };
            });
        } else {
            // Новая колонка, сортируем по возрастанию
            sort_column.set(column);
            sort_direction.set(SortDirection::Asc);
        }
    };

    let get_sort_indicator = move |column: SortColumn| {
        if sort_column.get() == column {
            match sort_direction.get() {
                SortDirection::Asc => " ↑",
                SortDirection::Desc => " ↓",
            }
        } else {
            ""
        }
    };

    view! {
        <div class="table-container">
            <div class="table-wrapper">
                <table class="types-table">
                    <thead>
                        <tr>
                            <th class="sortable" on:click=move |_| handle_sort(SortColumn::Name)>
                                "Тип" {move || get_sort_indicator(SortColumn::Name)}
                            </th>
                            <th class="sortable" on:click=move |_| handle_sort(SortColumn::Category)>
                                "Категория" {move || get_sort_indicator(SortColumn::Category)}
                            </th>
                            <th class="sortable" on:click=move |_| handle_sort(SortColumn::Certainty)>
                                "Определённость" {move || get_sort_indicator(SortColumn::Certainty)}
                            </th>
                            <th>"Фасеты"</th>
                            <th>"Union Types"</th>
                            <th>"Flow"</th>
                            <th class="sortable" on:click=move |_| handle_sort(SortColumn::Source)>
                                "Источник" {move || get_sort_indicator(SortColumn::Source)}
                            </th>
                            <th class="sortable" on:click=move |_| handle_sort(SortColumn::Methods)>
                                "Методы/Свойства" {move || get_sort_indicator(SortColumn::Methods)}
                            </th>
                            <th>"Действия"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            sorted_types.get().into_iter().map(|type_info| {
                                let type_info_clone = type_info.clone();
                                let on_row_click_clone = on_row_click;
                                let on_action_clone = on_action;

                                match (on_row_click_clone, on_action_clone) {
                                    (Some(row_handler), Some(action_handler)) => view! {
                                        <TypeTableRow
                                            type_info=Signal::derive(move || type_info_clone.clone())
                                            on_row_click=row_handler
                                            on_action=action_handler
                                        />
                                    }.into_view(),
                                    (Some(row_handler), None) => view! {
                                        <TypeTableRow
                                            type_info=Signal::derive(move || type_info_clone.clone())
                                            on_row_click=row_handler
                                        />
                                    }.into_view(),
                                    (None, Some(action_handler)) => view! {
                                        <TypeTableRow
                                            type_info=Signal::derive(move || type_info_clone.clone())
                                            on_action=action_handler
                                        />
                                    }.into_view(),
                                    (None, None) => view! {
                                        <TypeTableRow
                                            type_info=Signal::derive(move || type_info_clone.clone())
                                        />
                                    }.into_view(),
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

/// Компонент строки таблицы типов
#[component]
#[allow(non_snake_case)]
fn TypeTableRow(
    /// Информация о типе
    #[prop(into)]
    type_info: Signal<TypeInfo>,
    /// Обработчик клика по строке
    #[prop(optional)]
    on_row_click: Option<Callback<TypeInfo>>,
    /// Обработчик действий
    #[prop(optional)]
    on_action: Option<Callback<(String, TypeInfo)>>,
) -> impl IntoView {
    let handle_row_click = move |_| {
        if let Some(ref handler) = on_row_click {
            handler.run(type_info.get());
        }
    };

    let handle_action = move |action: &str| {
        let action = action.to_string();
        let on_action_clone = on_action;
        let type_info_clone = type_info;
        move |_| {
            if let Some(ref handler) = on_action_clone {
                handler.run((action.clone(), type_info_clone.get()));
            }
        }
    };

    view! {
        <tr on:click=handle_row_click>
            <td>
                <div class="type-name">{move || type_info.get().name.clone()}</div>
                <small style="color: #6c757d;">{move || type_info.get().id.clone()}</small>
            </td>
            <td>
                <span
                    class="category-badge"
                    style=move || format!("background: {}; color: white;", CategoryDto::get_color(&CategoryDto{ color: String::new(), icon: String::new(), count: 0 }, &type_info.get().category))
                >
                    {move || type_info.get().category.clone()}
                </span>
            </td>
            <td>
                <div class="certainty-bar">
                    <div
                        class="certainty-fill"
                        style=move || {
                            let info = type_info.get();
                            let width = info.certainty as f32;
                            format!("width: {}%; background: {};", width, info.certainty_color())
                        }
                    ></div>
                    <div class="certainty-text">{move || type_info.get().certainty_percentage()}</div>
                </div>
            </td>
            <td class="facets-cell">
                {move || {
                    type_info.get().facets.into_iter().map(|facet| {
                        view! {
                            <span
                                class="facet-tag"
                                style="background: #007bff; color: white; margin: 2px;"
                            >
                                {facet}
                            </span>
                        }
                    }).collect::<Vec<_>>()
                }}
            </td>
            <td>
                <span>"-"</span>
            </td>
            <td class="flow-indicator">
                {move || {
                    if type_info.get().is_flow_sensitive() {
                        view! { <span class="flow-yes">"✓"</span> }.into_any()
                    } else {
                        view! { <span class="flow-no">"✗"</span> }.into_any()
                    }
                }}
            </td>
            <td>{move || type_info.get().source}</td>
            <td>
                "Runtime Validation"
            </td>
            <td class="actions-cell">
                <button class="action-btn" on:click=handle_action("view")>"👁️"</button>
                <button class="action-btn" on:click=handle_action("copy")>"📋"</button>
                <button class="action-btn" on:click=handle_action("link")>"🔗"</button>
            </td>
        </tr>
    }
}
