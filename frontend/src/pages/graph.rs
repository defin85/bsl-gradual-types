//! Graph page for network visualization of type relationships

use crate::api::*;
use crate::components::GraphView;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use std::collections::HashSet;

#[derive(Clone, Debug)]
struct SimpleGraph {
    nodes: Vec<SimpleNode>,
    connections: Vec<ConnectionDto>,
}

#[derive(Clone, Debug)]
struct SimpleNode {
    id: String,
    type_info: TypeDto,
    connections: Vec<String>,
}

/// Страница графового представления типов
#[component]
#[allow(non_snake_case)]
pub fn GraphPage(
    /// Поисковый запрос для фильтрации
    #[prop(optional)] _search_query: Option<RwSignal<String>>,
) -> impl IntoView {
    let graph = RwSignal::new(SimpleGraph { nodes: vec![], connections: vec![] });
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let selected_node = RwSignal::new(None::<SimpleNode>);
    let display_mode = RwSignal::new("all".to_string());
    let connection_type_filter = RwSignal::new("all".to_string());
    let connection_depth = RwSignal::new("all".to_string());
    let search_query = RwSignal::new("".to_string());

    // Загружаем граф при монтировании компонента
    let load_graph = move || {
        loading.set(true);
        error.set(None);

        spawn_local(async move {
            match fetch_type_graph().await {
                Ok(connections) => {
                    // Создаем упрощенный граф из списка связей
                    let nodes = vec![]; // TODO: построить узлы из connections
                    graph.set(SimpleGraph { nodes, connections });
                    loading.set(false);
                },
                Err(err) => {
                    error.set(Some(err));
                    loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        load_graph();
    });

    let _handle_node_click = move |type_info: TypeDto| {
        web_sys::console::log_1(&format!("Clicked node: {}", type_info.name).into());
        // Найдем узел в графе
        if let Some(node) = graph.get().nodes.iter().find(|n| n.type_info.name == type_info.name) {
            selected_node.set(Some(node.clone()));
        }
    };

    let _handle_connection_click = move |connection: ConnectionDto| {
        web_sys::console::log_1(&format!("Clicked connection: {} -> {}", connection.source, connection.target).into());
    };

    let handle_display_mode_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let select = target.dyn_into::<web_sys::HtmlSelectElement>().unwrap();
        display_mode.set(select.value());
    };

    let handle_connection_type_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let select = target.dyn_into::<web_sys::HtmlSelectElement>().unwrap();
        connection_type_filter.set(select.value());
    };

    let handle_depth_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let select = target.dyn_into::<web_sys::HtmlSelectElement>().unwrap();
        connection_depth.set(select.value());
    };

    let handle_search_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input = target.dyn_into::<web_sys::HtmlInputElement>().unwrap();
        search_query.set(input.value());
    };

    // Фильтруем граф на основе настроек
    let filtered_graph = Signal::derive(move || {
        let mut current_graph = graph.get();
        let mode = display_mode.get();
        let query = search_query.get();
        
        // Фильтруем узлы по режиму отображения
        current_graph.nodes = current_graph.nodes.into_iter().filter(|node| {
            match mode.as_str() {
                "known" => node.type_info.certainty >= 90,
                "union" => node.type_info.category == "Union",
                "flow" => node.type_info.flow_sensitive,
                "config" => node.type_info.category == "Configuration",
                _ => true, // "all"
            }
        }).collect();
        
        // Фильтруем по поисковому запросу
        if !query.is_empty() {
            current_graph.nodes = current_graph.nodes.into_iter().filter(|node| {
                node.type_info.name.to_lowercase().contains(&query.to_lowercase()) ||
                node.type_info.name.to_lowercase().contains(&query.to_lowercase())
            }).collect();
        }
        
        // Фильтруем связи - оставляем только те, у которых есть оба узла
        let node_ids: HashSet<String> = current_graph.nodes.iter().map(|n| n.id.clone()).collect();
        current_graph.connections = current_graph.connections.into_iter().filter(|conn| {
            node_ids.contains(&conn.source) && node_ids.contains(&conn.target)
        }).collect();
        
        current_graph
    });

    view! {
        <main class="main-content" style="padding: 0; height: 100vh; display: flex; flex-direction: column;">
            <div class="graph-header" style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px; box-shadow: 0 2px 20px rgba(0, 0, 0, 0.3);">
                <h1>"🕸️ BSL Type Network"</h1>
                <p>"Interactive graph visualization of type relationships, dependencies, and flow-sensitive analysis"</p>

                <div class="graph-controls" style="display: flex; gap: 20px; align-items: center; margin-top: 15px;">
                    <div class="control-group">
                        <label style="font-size: 0.9em; color: rgba(255, 255, 255, 0.9);">"Режим отображения:"</label>
                        <select 
                            style="padding: 8px 12px; border: none; border-radius: 6px; background: rgba(255, 255, 255, 0.1); color: white; outline: none;"
                            on:change=handle_display_mode_change
                        >
                            <option value="all">"All Types"</option>
                            <option value="known">"Known Types Only"</option>
                            <option value="union">"Union Types Focus"</option>
                            <option value="flow">"Flow-Sensitive Only"</option>
                            <option value="config">"Configuration Types"</option>
                        </select>
                    </div>

                    <div class="control-group">
                        <label style="font-size: 0.9em; color: rgba(255, 255, 255, 0.9);">"Связи:"</label>
                        <select 
                            style="padding: 8px 12px; border: none; border-radius: 6px; background: rgba(255, 255, 255, 0.1); color: white; outline: none;"
                            on:change=handle_connection_type_change
                        >
                            <option value="all">"All Connections"</option>
                            <option value="inheritance">"Inheritance"</option>
                            <option value="composition">"Composition"</option>
                            <option value="dependencies">"Dependencies"</option>
                            <option value="flow">"Flow Transitions"</option>
                        </select>
                    </div>

                    <div class="control-group">
                        <label style="font-size: 0.9em; color: rgba(255, 255, 255, 0.9);">"Поиск узла:"</label>
                        <input 
                            type="text"
                            placeholder="Название типа..."
                            style="padding: 8px 12px; border: none; border-radius: 6px; background: rgba(255, 255, 255, 0.1); color: white; outline: none;"
                            on:input=handle_search_change
                        />
                    </div>

                    <div class="control-group">
                        <label style="font-size: 0.9em; color: rgba(255, 255, 255, 0.9);">"Глубина связей:"</label>
                        <select 
                            style="padding: 8px 12px; border: none; border-radius: 6px; background: rgba(255, 255, 255, 0.1); color: white; outline: none;"
                            on:change=handle_depth_change
                        >
                            <option value="1">"1 уровень"</option>
                            <option value="2">"2 уровня"</option>
                            <option value="3">"3 уровня"</option>
                            <option value="all">"Все уровни"</option>
                        </select>
                    </div>
                </div>
            </div>

            {move || {
                if loading.get() {
                    view! {
                        <div class="loading" style="flex: 1; display: flex; align-items: center; justify-content: center; background: #1a1a2e; color: white;">
                            <p>"🔄 Загрузка графа типов..."</p>
                        </div>
                    }.into_any()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="error" style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; background: #1a1a2e; color: white;">
                            <p>"❌ Ошибка загрузки: " {err}</p>
                            <button 
                                style="margin-top: 1rem; padding: 0.5rem 1rem; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer;"
                                on:click=move |_| load_graph()
                            >
                                "Повторить"
                            </button>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div style="flex: 1; position: relative; background: #1a1a2e;">
                            <GraphView
                                types=Signal::derive(move || {
                                    filtered_graph.get().nodes.into_iter().map(|node| node.type_info).collect()
                                })
                            />
                            
                            <div class="graph-stats" style="position: absolute; top: 20px; left: 20px; background: rgba(0, 0, 0, 0.8); padding: 15px; border-radius: 8px; color: white; border: 1px solid rgba(255, 255, 255, 0.2);">
                                <h4>"📊 Статистика графа"</h4>
                                <p>"Узлов: " {move || filtered_graph.get().nodes.len()}</p>
                                <p>"Связей: " {move || filtered_graph.get().connections.len()}</p>
                                {move || {
                                    if let Some(node) = selected_node.get() {
                                        view! {
                                            <div style="margin-top: 10px; padding-top: 10px; border-top: 1px solid rgba(255,255,255,0.2);">
                                                <p><strong>"Выбран: "</strong> {node.type_info.name.clone()}</p>
                                                <p>"Связей: " {node.connections.len()}</p>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }
                                }}
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </main>
    }
}