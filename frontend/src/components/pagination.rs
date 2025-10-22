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
                view! {
                    <div class="pagination">
                        <div class="pagination__info">
                            <span class="pagination__text">
                                "Показано " {info.current_page * info.page_size - info.page_size + 1}
                                "-" {std::cmp::min(info.current_page * info.page_size, info.total_items)}
                                " из " {info.total_items} " типов"
                            </span>
                        </div>

                        <div class="pagination__controls">
                            // Первая страница
                            <button
                                class="pagination__btn pagination__btn--first"
                                disabled=!info.has_prev
                                on:click=move |_| on_page_change.run(1)
                                title="Первая страница"
                            >
                                "⏮"
                            </button>

                            // Предыдущая страница
                            <button
                                class="pagination__btn pagination__btn--prev"
                                disabled=!info.has_prev
                                on:click=move |_| on_page_change.run(info.current_page - 1)
                                title="Предыдущая страница"
                            >
                                "◀"
                            </button>

                            // Номера страниц
                            <div class="pagination__pages">
                                {get_page_numbers(info.current_page, info.total_pages).into_iter().map(|page_item| {
                                    match page_item {
                                        PageItem::Page(page) => {
                                            let is_current = page == info.current_page;
                                            view! {
                                                <button
                                                    class=format!(
                                                        "pagination__btn pagination__btn--page {}",
                                                        if is_current { "pagination__btn--current" } else { "" }
                                                    )
                                                    disabled=is_current
                                                    on:click=move |_| on_page_change.run(page)
                                                >
                                                    {page}
                                                </button>
                                            }.into_any()
                                        },
                                        PageItem::Ellipsis => {
                                            view! {
                                                <span class="pagination__ellipsis">"..."</span>
                                            }.into_any()
                                        }
                                    }
                                }).collect::<Vec<_>>()}
                            </div>

                            // Следующая страница
                            <button
                                class="pagination__btn pagination__btn--next"
                                disabled=!info.has_next
                                on:click=move |_| on_page_change.run(info.current_page + 1)
                                title="Следующая страница"
                            >
                                "▶"
                            </button>

                            // Последняя страница
                            <button
                                class="pagination__btn pagination__btn--last"
                                disabled=!info.has_next
                                on:click=move |_| on_page_change.run(info.total_pages)
                                title="Последняя страница"
                            >
                                "⏭"
                            </button>
                        </div>

                        <div class="pagination__page-size">
                            <label for="page-size-select" class="pagination__label">
                                "Показывать по:"
                            </label>
                            <select
                                id="page-size-select"
                                class="pagination__select"
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
