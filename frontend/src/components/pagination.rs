//! Компонент пагинации

use crate::api::PaginationInfo;
use leptos::prelude::*;

/// Компонент пагинации
#[component]
#[allow(non_snake_case)]
pub fn Pagination(
    /// Информация о пагинации
    pagination: Signal<Option<PaginationInfo>>,
    /// Коллбек при изменении страницы
    on_page_change: Callback<usize>,
) -> impl IntoView {
    view! {
        {move || {
            if let Some(info) = pagination.get() {
                let start = info.current_page * info.page_size - info.page_size + 1;
                let end = std::cmp::min(info.current_page * info.page_size, info.total_items);
                let total = info.total_items;

                view! {
                    <div class="flex flex-col sm:flex-row items-center justify-between gap-4 py-3 px-4 bg-bsl-cream-50 dark:bg-bsl-charcoal-800 border-t border-bsl-brown-600/20 dark:border-bsl-gray-400/30">
                        // Info
                        <div class="text-sm text-bsl-slate-500/70 dark:text-bsl-gray-300/70">
                            "Показано " {start} "-" {end} " из " {total} " типов"
                        </div>

                        // Controls
                        <div class="flex items-center gap-1">
                            // Первая страница
                            <button
                                class="px-2 py-1 text-sm rounded hover:bg-bsl-brown-600/8 dark:hover:bg-bsl-gray-400/10 disabled:opacity-40 disabled:cursor-not-allowed transition-colors duration-fast"
                                disabled=!info.has_prev
                                on:click=move |_| on_page_change.run(1)
                                title="Первая страница"
                            >
                                "⏮"
                            </button>

                            // Предыдущая страница
                            <button
                                class="px-2 py-1 text-sm rounded hover:bg-bsl-brown-600/8 dark:hover:bg-bsl-gray-400/10 disabled:opacity-40 disabled:cursor-not-allowed transition-colors duration-fast"
                                disabled=!info.has_prev
                                on:click=move |_| on_page_change.run(info.current_page - 1)
                                title="Предыдущая страница"
                            >
                                "◀"
                            </button>

                            // Номера страниц
                            <div class="flex items-center gap-1 mx-2">
                                {get_page_numbers(info.current_page, info.total_pages).into_iter().map(|page_item| {
                                    match page_item {
                                        PageItem::Page(page) => {
                                            let is_current = page == info.current_page;
                                            view! {
                                                <button
                                                    class=move || {
                                                        let base = "min-w-[32px] px-2 py-1 text-sm rounded transition-colors duration-fast";
                                                        if is_current {
                                                            format!("{} bg-bsl-teal-500 text-white font-medium", base)
                                                        } else {
                                                            format!("{} hover:bg-bsl-brown-600/8 dark:hover:bg-bsl-gray-400/10 text-bsl-slate-900 dark:text-bsl-gray-200", base)
                                                        }
                                                    }
                                                    disabled=is_current
                                                    on:click=move |_| on_page_change.run(page)
                                                >
                                                    {page}
                                                </button>
                                            }.into_any()
                                        },
                                        PageItem::Ellipsis => {
                                            view! {
                                                <span class="px-2 text-bsl-slate-500/50 dark:text-bsl-gray-300/50">
                                                    "..."
                                                </span>
                                            }.into_any()
                                        }
                                    }
                                }).collect::<Vec<_>>()}
                            </div>

                            // Следующая страница
                            <button
                                class="px-2 py-1 text-sm rounded hover:bg-bsl-brown-600/8 dark:hover:bg-bsl-gray-400/10 disabled:opacity-40 disabled:cursor-not-allowed transition-colors duration-fast"
                                disabled=!info.has_next
                                on:click=move |_| on_page_change.run(info.current_page + 1)
                                title="Следующая страница"
                            >
                                "▶"
                            </button>

                            // Последняя страница
                            <button
                                class="px-2 py-1 text-sm rounded hover:bg-bsl-brown-600/8 dark:hover:bg-bsl-gray-400/10 disabled:opacity-40 disabled:cursor-not-allowed transition-colors duration-fast"
                                disabled=!info.has_next
                                on:click=move |_| on_page_change.run(info.total_pages)
                                title="Последняя страница"
                            >
                                "⏭"
                            </button>
                        </div>

                        // Page size selector
                        <div class="flex items-center gap-2">
                            <label for="page-size-select" class="text-sm text-bsl-slate-500/70 dark:text-bsl-gray-300/70">
                                "Показывать по:"
                            </label>
                            <select
                                id="page-size-select"
                                class="px-2 py-1 text-sm bg-bsl-cream-100 dark:bg-bsl-charcoal-700 border border-bsl-brown-600/20 dark:border-bsl-gray-400/30 rounded focus:border-bsl-teal-500 focus:ring-2 focus:ring-bsl-teal-500/20"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    if let Ok(_page_size) = value.parse::<usize>() {
                                        // При изменении размера страницы сбрасываем на первую страницу
                                        on_page_change.run(1);
                                        // TODO: нужен отдельный коллбек для изменения размера страницы
                                    }
                                }
                            >
                                <option value="25" selected=info.page_size == 25>"25"</option>
                                <option value="50" selected=info.page_size == 50>"50"</option>
                                <option value="100" selected=info.page_size == 100>"100"</option>
                                <option value="200" selected=info.page_size == 200>"200"</option>
                            </select>
                        </div>
                    </div>
                }.into_any()
            } else {
                let _: () = view! {};
                ().into_any()
            }
        }}
    }
}

#[derive(Debug, Clone)]
enum PageItem {
    Page(usize),
    Ellipsis,
}

/// Генерирует список номеров страниц для отображения с многоточиями
fn get_page_numbers(current: usize, total: usize) -> Vec<PageItem> {
    if total <= 7 {
        // Если страниц мало, показываем все
        (1..=total).map(PageItem::Page).collect()
    } else if current <= 4 {
        // Начало: 1 2 3 4 5 ... last
        let mut pages = (1..=5).map(PageItem::Page).collect::<Vec<_>>();
        pages.push(PageItem::Ellipsis);
        pages.push(PageItem::Page(total));
        pages
    } else if current >= total - 3 {
        // Конец: 1 ... last-4 last-3 last-2 last-1 last
        let mut pages = vec![PageItem::Page(1), PageItem::Ellipsis];
        pages.extend((total - 4..=total).map(PageItem::Page));
        pages
    } else {
        // Середина: 1 ... current-1 current current+1 ... last
        vec![
            PageItem::Page(1),
            PageItem::Ellipsis,
            PageItem::Page(current - 1),
            PageItem::Page(current),
            PageItem::Page(current + 1),
            PageItem::Ellipsis,
            PageItem::Page(total),
        ]
    }
}
