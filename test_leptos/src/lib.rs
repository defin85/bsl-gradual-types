use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <div>
            <h1>"Test Leptos Callbacks"</h1>
            <CallbackTest />
            <TableTest />
        </div>
    }
}

#[derive(Clone, Debug)]
struct FilterData {
    name: String,
    value: String,
}

#[component]
fn CallbackTest() -> impl IntoView {
    let (filters, set_filters) = signal(FilterData {
        name: "test".to_string(),
        value: "initial".to_string(),
    });

    // Паттерн 1: Простой callback с Callback<T>
    let handle_change = Callback::new(move |new_filters: FilterData| {
        set_filters.set(new_filters);
    });

    view! {
        <div>
            <p>"Current filters: " {move || format!("{:?}", filters.get())}</p>
            <ChildWithCallback on_change=handle_change />
            <ChildWithOptionalCallback on_change=handle_change />
            <ChildWithOptionalCallback />
        </div>
    }
}

#[component]
fn ChildWithCallback(
    on_change: Callback<FilterData>,
) -> impl IntoView {
    let handle_click = move |_| {
        on_change.run(FilterData {
            name: "updated".to_string(),
            value: "from_child".to_string(),
        });
    };

    view! {
        <button on:click=handle_click>
            "Update via required callback"
        </button>
    }
}

#[component]
fn ChildWithOptionalCallback(
    #[prop(optional)] on_change: Option<Callback<FilterData>>,
) -> impl IntoView {
    let handle_click = move |_| {
        if let Some(callback) = on_change {
            callback.run(FilterData {
                name: "optional".to_string(),
                value: "from_optional_child".to_string(),
            });
        }
    };

    view! {
        <button on:click=handle_click>
            "Update via optional callback"
        </button>
    }
}

#[derive(Clone, Debug)]
struct TableItem {
    id: String,
    name: String,
    children: Vec<TableItem>,
}

#[component]
fn TableTest() -> impl IntoView {
    let test_data = vec![
        TableItem {
            id: "1".to_string(),
            name: "Parent 1".to_string(),
            children: vec![
                TableItem {
                    id: "1.1".to_string(),
                    name: "Child 1.1".to_string(),
                    children: vec![],
                },
                TableItem {
                    id: "1.2".to_string(),
                    name: "Child 1.2".to_string(),
                    children: vec![],
                },
            ],
        },
        TableItem {
            id: "2".to_string(),
            name: "Parent 2".to_string(),
            children: vec![],
        },
    ];

    let handle_row_click = Callback::new(move |item_id: String| {
        web_sys::console::log_1(&format!("Row clicked: {}", item_id).into());
    });

    let handle_action = Callback::new(move |item_id: String| {
        web_sys::console::log_1(&format!("Action on: {}", item_id).into());
    });

    view! {
        <div>
            <h2>"Table Test"</h2>
            <TestTable 
                items=test_data
                on_row_click=handle_row_click
                on_action=handle_action
            />
        </div>
    }
}

#[component]
fn TestTable(
    items: Vec<TableItem>,
    on_row_click: Callback<String>,
    on_action: Callback<String>,
) -> impl IntoView {
    view! {
        <table>
            <tbody>
                {items.into_iter().map(|item| {
                    view! {
                        <TestTableRow 
                            item=item
                            on_row_click=on_row_click
                            on_action=on_action
                        />
                    }
                }).collect_view()}
            </tbody>
        </table>
    }
}

#[component]
fn TestTableRow(
    item: TableItem,
    on_row_click: Callback<String>,
    on_action: Callback<String>,
) -> impl IntoView {
    let item_id = item.id.clone();
    let item_id_action = item.id.clone();
    
    // Создаем замыкания для обработки событий
    let handle_row_click = move |_| {
        on_row_click.run(item_id.clone());
    };
    
    let handle_action = move |_| {
        on_action.run(item_id_action.clone());
    };

    view! {
        <tr>
            <td on:click=handle_row_click style="cursor: pointer; padding: 8px;">
                {item.name.clone()}
            </td>
            <td>
                <button on:click=handle_action>"Action"</button>
            </td>
        </tr>
        // Рекурсивно отображаем дочерние элементы
        {if !item.children.is_empty() {
            view! {
                <TestTable 
                    items=item.children
                    on_row_click=on_row_click
                    on_action=on_action
                />
            }.into_any()
        } else {
            view! {}.into_any()
        }}
    }
}