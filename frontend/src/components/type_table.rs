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
        <div class="bg-white dark:bg-gray-900 rounded-lg overflow-hidden shadow-sm border border-gray-200 dark:border-gray-800">
            <div class="overflow-x-auto">
                <table class="w-full border-collapse">
                    <thead>
                        <tr class="border-b border-gray-200 dark:border-gray-700">
                            <th
                                class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer select-none hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                                on:click=move |_| handle_sort(SortColumn::Name)
                            >
                                <div class="flex items-center gap-2">
                                    <span>"Тип"</span>
                                    <span class="opacity-50 hover:opacity-100">{move || get_sort_indicator(SortColumn::Name)}</span>
                                </div>
                            </th>
                            <th
                                class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer select-none hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                                on:click=move |_| handle_sort(SortColumn::Category)
                            >
                                <div class="flex items-center gap-2">
                                    <span>"Категория"</span>
                                    <span class="opacity-50 hover:opacity-100">{move || get_sort_indicator(SortColumn::Category)}</span>
                                </div>
                            </th>
                            <th
                                class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer select-none hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                                on:click=move |_| handle_sort(SortColumn::Certainty)
                            >
                                <div class="flex items-center gap-2">
                                    <span>"Определённость"</span>
                                    <span class="opacity-50 hover:opacity-100">{move || get_sort_indicator(SortColumn::Certainty)}</span>
                                </div>
                            </th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">"Фасеты"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">"Union Types"</th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">"Flow"</th>
                            <th
                                class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer select-none hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                                on:click=move |_| handle_sort(SortColumn::Source)
                            >
                                <div class="flex items-center gap-2">
                                    <span>"Источник"</span>
                                    <span class="opacity-50 hover:opacity-100">{move || get_sort_indicator(SortColumn::Source)}</span>
                                </div>
                            </th>
                            <th
                                class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer select-none hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                                on:click=move |_| handle_sort(SortColumn::Methods)
                            >
                                <div class="flex items-center gap-2">
                                    <span>"Методы/Свойства"</span>
                                    <span class="opacity-50 hover:opacity-100">{move || get_sort_indicator(SortColumn::Methods)}</span>
                                </div>
                            </th>
                            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">"Действия"</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
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

    // Helper для получения цвета категории
    let get_category_class = move || {
        let category = type_info.get().category;
        match category.as_str() {
            "Platform" => "bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200",
            "Configuration" => "bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200",
            "Union" => "bg-orange-100 dark:bg-orange-900 text-orange-800 dark:text-orange-200",
            "Dynamic" => "bg-pink-100 dark:bg-pink-900 text-pink-800 dark:text-pink-200",
            _ => "bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200"
        }
    };

    view! {
        <tr
            class="hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors cursor-pointer"
            on:click=handle_row_click
        >
            <td class="px-4 py-3">
                <div class="font-medium text-gray-900 dark:text-white">{move || type_info.get().name.clone()}</div>
                <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">{move || type_info.get().id.clone()}</div>
            </td>
            <td class="px-4 py-3">
                <span class=move || format!("inline-flex items-center px-2.5 py-1 rounded text-xs font-medium {}", get_category_class())>
                    {move || type_info.get().category.clone()}
                </span>
            </td>
            <td class="px-4 py-3">
                <div class="flex items-center gap-2">
                    <div class="flex-1 h-1.5 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
                        <div
                            class="h-full rounded transition-all duration-300"
                            style=move || {
                                let info = type_info.get();
                                let width = info.certainty as f32;
                                format!("width: {}%; background: {};", width, info.certainty_color())
                            }
                        ></div>
                    </div>
                    <div class="text-xs font-medium text-gray-600 dark:text-gray-400 whitespace-nowrap">
                        {move || type_info.get().certainty_percentage()}
                    </div>
                </div>
            </td>
            <td class="px-4 py-3">
                <div class="flex flex-wrap gap-1">
                    {move || {
                        type_info.get().facets.into_iter().map(|facet| {
                            view! {
                                <span class="inline-block px-2 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200 text-xs rounded">
                                    {facet}
                                </span>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>
            </td>
            <td class="px-4 py-3 text-gray-500 dark:text-gray-400">
                <span>"-"</span>
            </td>
            <td class="px-4 py-3">
                {move || {
                    if type_info.get().is_flow_sensitive() {
                        view! {
                            <span class="inline-flex items-center px-2 py-1 bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200 text-xs rounded font-medium">
                                "✓"
                            </span>
                        }.into_any()
                    } else {
                        view! {
                            <span class="inline-flex items-center px-2 py-1 bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 text-xs rounded">
                                "✗"
                            </span>
                        }.into_any()
                    }
                }}
            </td>
            <td class="px-4 py-3 text-sm text-gray-700 dark:text-gray-300">{move || type_info.get().source}</td>
            <td class="px-4 py-3 text-sm text-gray-700 dark:text-gray-300">
                "Runtime Validation"
            </td>
            <td class="px-4 py-3">
                <div class="flex items-center gap-1">
                    <button
                        class="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
                        on:click=handle_action("view")
                        title="Просмотр"
                    >"👁️"</button>
                    <button
                        class="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
                        on:click=handle_action("copy")
                        title="Копировать"
                    >"📋"</button>
                    <button
                        class="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
                        on:click=handle_action("link")
                        title="Ссылка"
                    >"🔗"</button>
                </div>
            </td>
        </tr>
    }
}
