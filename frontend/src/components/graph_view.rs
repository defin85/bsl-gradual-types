//! Graph view component for network visualization of types

use crate::api::types::*;
use leptos::prelude::*;
use leptos::svg;


/// Компонент графового представления типов
#[component]
#[allow(non_snake_case)]
pub fn GraphView(
    /// Граф типов для отображения
    #[prop(into)] graph: Signal<TypeGraph>,
    /// Обработчик клика по узлу
    #[prop(optional)] on_node_click: Option<std::sync::Arc<dyn Fn(TypeInfo) + Send + Sync + 'static>>,
    /// Обработчик клика по связи
    #[prop(optional)] on_connection_click: Option<std::sync::Arc<dyn Fn(TypeConnection) + Send + Sync + 'static>>,
) -> impl IntoView {
    let selected_node = RwSignal::new(None::<TypeGraphNode>);
    let zoom_level = RwSignal::new(1.0);
    let pan_x = RwSignal::new(0.0);
    let pan_y = RwSignal::new(0.0);

    let svg_ref = NodeRef::<svg::Svg>::new();

    let handle_node_click = std::sync::Arc::new(move |node: TypeGraphNode| {
        selected_node.set(Some(node.clone()));
        if let Some(ref handler) = on_node_click {
            handler(node.type_info);
        }
    });

    let handle_zoom_in = move |_| {
        zoom_level.update(|z| *z = (*z * 1.2_f64).min(3.0));
    };

    let handle_zoom_out = move |_| {
        zoom_level.update(|z| *z = (*z / 1.2_f64).max(0.3));
    };

    let handle_reset_zoom = move |_| {
        zoom_level.set(1.0);
        pan_x.set(0.0);
        pan_y.set(0.0);
    };

    let svg_transform = move || {
        format!(
            "translate({}, {}) scale({})",
            pan_x.get(),
            pan_y.get(),
            zoom_level.get()
        )
    };

    view! {
        <div class="graph-container">
            <svg class="graph-svg" node_ref=svg_ref viewBox="0 0 800 600">
                <defs>
                    // Определяем маркеры для стрелок
                    <marker id="arrowhead" markerWidth="10" markerHeight="7" 
                            refX="9" refY="3.5" orient="auto">
                        <polygon points="0 0, 10 3.5, 0 7" fill="#666" />
                    </marker>
                </defs>
                
                <g transform=svg_transform>
                    // Рендерим связи
                    {move || {
                        graph.get().connections.into_iter().map(|connection| {
                            let from_node = graph.get().nodes.iter()
                                .find(|n| n.id == connection.from)
                                .cloned();
                            let to_node = graph.get().nodes.iter()
                                .find(|n| n.id == connection.to)
                                .cloned();
                            
                            if let (Some(from), Some(to)) = (from_node, to_node) {
                                let connection_clone = connection.clone();
                                view! {
                                    <GraphConnection 
                                        connection=Signal::derive(move || connection_clone.clone())
                                        from_pos=(from.x, from.y)
                                        to_pos=(to.x, to.y)
                                        on_click=on_connection_click.as_ref().map(|handler| {
                                            let handler = handler.clone();
                                            let conn = connection.clone();
                                            Box::new(move || handler(conn.clone())) as Box<dyn Fn() + 'static>
                                        }).unwrap_or_else(|| Box::new(|| {}) as Box<dyn Fn() + 'static>)
                                    />
                                }.into_any()
                            } else {
                                view! {}.into_any()
                            }
                        }).collect::<Vec<_>>()
                    }}
                    
                    // Рендерим узлы
                    {move || {
                        graph.get().nodes.into_iter().map(|node| {
                            let node_clone = node.clone();
                            let node_id = node.id.clone();
                            let is_selected = move || {
                                selected_node.get().as_ref().map(|n| n.id == node_id).unwrap_or(false)
                            };
                            
                            view! {
                                <GraphNode 
                                    node=Signal::derive(move || node_clone.clone())
                                    is_selected=Signal::derive(is_selected)
                                    on_click={
                         let node = node.clone();
                         let handle_click = handle_node_click.clone();
                         Box::new(move || handle_click(node.clone())) as Box<dyn Fn() + 'static>
                     }
                                />
                            }
                        }).collect::<Vec<_>>()
                    }}
                </g>
            </svg>

            // Элементы управления зумом
            <div class="zoom-controls">
                <button class="zoom-btn" on:click=handle_zoom_in>"+"</button>
                <button class="zoom-btn" on:click=handle_zoom_out>"-"</button>
                <button class="zoom-btn" on:click=handle_reset_zoom>"⌂"</button>
            </div>

            // Боковая панель с информацией о выбранном узле
            <div class="graph-sidebar">
                {move || {
                    if let Some(node) = selected_node.get() {
                        view! {
                            <NodeInfoPanel node=Signal::derive(move || node.clone()) />
                        }.into_any()
                    } else {
                        view! {
                            <div class="node-info">
                                <div class="node-name">"📋 Выберите узел"</div>
                                <p>"Кликните на узел графа для просмотра детальной информации о типе."</p>
                            </div>
                        }.into_any()
                    }
                }}
                
                <GraphLegend />
            </div>
        </div>
    }
}

/// Компонент узла графа
#[component]
#[allow(non_snake_case)]
fn GraphNode(
    /// Узел графа
    #[prop(into)] node: Signal<TypeGraphNode>,
    /// Выбран ли узел
    #[prop(into)] is_selected: Signal<bool>,
    /// Обработчик клика
    on_click: Box<dyn Fn() + 'static>,
) -> impl IntoView {
    let node_radius = 35.0;
    
    let node_color = move || {
        node.get().type_info.category.color()
    };
    
    let certainty_color = move || {
        node.get().type_info.certainty.color()
    };
    
    let stroke_width = move || {
        if is_selected.get() { 4 } else { 2 }
    };

    view! {
        <g class="node" on:click=move |_| on_click()>
            // Основной круг узла
            <circle 
                cx=move || node.get().x
                cy=move || node.get().y
                r=node_radius
                fill=node_color
                stroke=certainty_color
                stroke-width=stroke_width
                class="node-circle"
            />
            
            // Текст с названием типа
            <text 
                x=move || node.get().x
                y=move || node.get().y + 5.0
                class="node-label"
                text-anchor="middle"
                fill="white"
            >
                {move || {
                    let display_name = node.get().type_info.display_name;
                    if display_name.len() > 10 {
                        format!("{}...", &display_name[..7])
                    } else {
                        display_name
                    }
                }}
            </text>
            
            // Индикаторы фасетов
            <text 
                x=move || node.get().x
                y=move || node.get().y - 15.0
                class="facet-indicator"
                text-anchor="middle"
                fill="white"
                font-size="8"
            >
                {move || {
                    let facets = &node.get().type_info.facets;
                    facets.iter().take(3).map(|f| match f {
                        FacetKind::Manager => "MGR",
                        FacetKind::Object => "OBJ",
                        FacetKind::Reference => "REF",
                        FacetKind::Collection => "COL",
                        FacetKind::Metadata => "META",
                    }).collect::<Vec<_>>().join(" ")
                }}
            </text>
            
            // Индикатор уверенности для Union/Dynamic типов
            {move || {
                let info = node.get().type_info;
                match info.certainty {
                    Certainty::Inferred(val) => {
                        view! {
                            <text 
                                x=move || node.get().x
                                y=move || node.get().y + 25.0
                                class="certainty-indicator"
                                text-anchor="middle"
                                fill="white"
                                font-size="10"
                            >
                                {format!("{:.0}%", val * 100.0)}
                            </text>
                        }.into_any()
                    },
                    Certainty::Unknown => {
                        view! {
                            <text 
                                x=move || node.get().x
                                y=move || node.get().y + 25.0
                                class="certainty-indicator"
                                text-anchor="middle"
                                fill="white"
                                font-size="10"
                            >
                                "RUNTIME"
                            </text>
                        }.into_any()
                    },
                    _ => view! {}.into_any()
                }
            }}
        </g>
    }
}

/// Компонент связи между узлами
#[component]
#[allow(non_snake_case)]
fn GraphConnection(
    /// Связь
    #[prop(into)] connection: Signal<TypeConnection>,
    /// Позиция начального узла
    from_pos: (f32, f32),
    /// Позиция конечного узла
    to_pos: (f32, f32),
    /// Обработчик клика
    #[prop(optional)] on_click: Option<Box<dyn Fn() + 'static>>,
) -> impl IntoView {
    let stroke_color = move || {
        connection.get().connection_type.color()
    };
    
    let is_flow_connection = move || {
        matches!(connection.get().connection_type, ConnectionType::FlowTransition)
    };

    view! {
        <g class="edge">
            <line 
                x1=from_pos.0
                y1=from_pos.1
                x2=to_pos.0
                y2=to_pos.1
                stroke=stroke_color
                stroke-width=move || if is_flow_connection() { 3 } else { 2 }
                stroke-dasharray=move || {
                    match connection.get().connection_type {
                        ConnectionType::Dependency => Some("5,5"),
                        _ => None,
                    }
                }
                marker-end="url(#arrowhead)"
                on:click=move |_| {
                    if let Some(ref handler) = on_click {
                        handler();
                    }
                }
            />
            
            {move || {
                if let Some(label) = connection.get().label {
                    let mid_x = (from_pos.0 + to_pos.0) / 2.0;
                    let mid_y = (from_pos.1 + to_pos.1) / 2.0;
                    
                    view! {
                        <text 
                            x=mid_x
                            y=mid_y - 5.0
                            class="edge-label"
                            text-anchor="middle"
                            fill="rgba(255,255,255,0.7)"
                            font-size="10"
                        >
                            {label}
                        </text>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </g>
    }
}

/// Компонент панели информации о узле
#[component]
#[allow(non_snake_case)]
fn NodeInfoPanel(
    /// Узел для отображения информации
    #[prop(into)] node: Signal<TypeGraphNode>,
) -> impl IntoView {
    view! {
        <div class="node-info">
            <div class="node-name">"📋 " {move || node.get().type_info.display_name}</div>
            <div class="node-details">
                <div class="detail-item">
                    <span>"Тип:"</span>
                    <span>{move || node.get().type_info.name}</span>
                </div>
                <div class="detail-item">
                    <span>"Категория:"</span>
                    <span>{move || node.get().type_info.category.as_str()}</span>
                </div>
                <div class="detail-item">
                    <span>"Определённость:"</span>
                    <span>{move || node.get().type_info.certainty.as_percentage()}</span>
                </div>
                <div class="detail-item">
                    <span>"Фасеты:"</span>
                    <span>{move || {
                        node.get().type_info.facets.iter()
                            .map(|f| f.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }}</span>
                </div>
                <div class="detail-item">
                    <span>"Связи:"</span>
                    <span>{move || format!("{} связей", node.get().connections.len())}</span>
                </div>
                <div class="detail-item">
                    <span>"Flow-Sensitive:"</span>
                    <span>{move || if node.get().type_info.is_flow_sensitive { "Да" } else { "Нет" }}</span>
                </div>
            </div>
            
            {move || {
                let info = node.get().type_info;
                let style_class = match info.category {
                    TypeCategory::Platform => "background: rgba(16, 185, 129, 0.1); padding: 10px; border-radius: 6px; border-left: 3px solid #10b981;",
                    TypeCategory::Configuration => "background: rgba(16, 185, 129, 0.1); padding: 10px; border-radius: 6px; border-left: 3px solid #10b981;",
                    TypeCategory::Union => "background: rgba(245, 158, 11, 0.1); padding: 10px; border-radius: 6px; border-left: 3px solid #f59e0b;",
                    TypeCategory::Dynamic => "background: rgba(239, 68, 68, 0.1); padding: 10px; border-radius: 6px; border-left: 3px solid #ef4444;",
                };
                
                let description = match info.category {
                    TypeCategory::Platform => "Базовый тип платформы 1С",
                    TypeCategory::Configuration => "Тип из конфигурации 1С с поддержкой фасетов",
                    TypeCategory::Union => "Объединенный тип с градуальной типизацией",
                    TypeCategory::Dynamic => "Динамический тип с runtime определением",
                };
                
                view! {
                    <div style=style_class>
                        <small><strong>"Описание: "</strong> {description}</small>
                    </div>
                }
            }}
        </div>
    }
}

/// Компонент легенды графа
#[component]
#[allow(non_snake_case)]
fn GraphLegend() -> impl IntoView {
    view! {
        <div class="legend">
            <div class="legend-title">"🎨 Категории типов"</div>
            
            <div class="legend-item">
                <div class="legend-color" style="background: #007bff;"></div>
                <span>"Platform Types"</span>
            </div>
            
            <div class="legend-item">
                <div class="legend-color" style="background: #28a745;"></div>
                <span>"Configuration Types"</span>
            </div>
            
            <div class="legend-item">
                <div class="legend-color" style="background: #ffc107;"></div>
                <span>"Union Types"</span>
            </div>
            
            <div class="legend-item">
                <div class="legend-color" style="background: #dc3545;"></div>
                <span>"Dynamic Types"</span>
            </div>
            
            <div style="margin-top: 20px; padding-top: 15px; border-top: 1px solid rgba(255,255,255,0.1);">
                <div class="legend-title">"🔗 Типы связей"</div>
                
                <div style="display: flex; align-items: center; margin-bottom: 8px;">
                    <div style="width: 30px; height: 2px; background: rgba(255,255,255,0.3); margin-right: 10px;"></div>
                    <span style="font-size: 0.9em;">"Dependencies"</span>
                </div>
                
                <div style="display: flex; align-items: center; margin-bottom: 8px;">
                    <div style="width: 30px; height: 2px; background: #f59e0b; margin-right: 10px;"></div>
                    <span style="font-size: 0.9em;">"Flow Transitions"</span>
                </div>
                
                <div style="display: flex; align-items: center; margin-bottom: 8px;">
                    <div style="width: 30px; height: 2px; background: #6f42c1; margin-right: 10px;"></div>
                    <span style="font-size: 0.9em;">"References"</span>
                </div>
            </div>
        </div>
    }
}