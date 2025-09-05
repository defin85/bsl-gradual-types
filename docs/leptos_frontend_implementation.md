# BSL Type System - Leptos Frontend Implementation

## 🚀 Leptos: Современный Rust Frontend для BSL Type System

### 📦 Интегрированная структура проекта

```
bsl-gradual-types/
├── Cargo.toml                    # Основной проект с условными features
├── src/
│   ├── presentation/
│   │   ├── web/                  # 🌐 Web UI компоненты
│   │   │   ├── mod.rs
│   │   │   ├── ui.rs             # Базовый HTML UI (существующий)
│   │   │   └── components/       # Leptos компоненты (новое)
│   │   │       ├── mod.rs
│   │   │       ├── app.rs        # Main App component
│   │   │       ├── dashboard.rs  # Dashboard view
│   │   │       ├── type_cards.rs # Card-based view
│   │   │       ├── type_table.rs # Table view
│   │   │       ├── type_graph.rs # Graph view
│   │   │       ├── api/
│   │   │       │   └── client.rs # API client
│   │   │       └── common/
│   │   │           ├── header.rs
│   │   │           ├── search_bar.rs
│   │   │           └── metric_card.rs
│   │   ├── lsp/                  # 🔧 LSP компоненты
│   │   └── cli/                  # 💻 CLI компоненты
│   ├── application/              # 🎯 Бизнес-логика
│   ├── domain/                   # 📝 Доменные модели
│   │   └── models/               # Общие типы для API
│   ├── data/                     # 💾 Данные
│   └── bin/
│       └── web_server.rs         # Расширенный web сервер
└── static/                       # Статические файлы для Leptos
    ├── index.html
    ├── style.css
    └── pkg/                      # WebAssembly output
```

### 📋 Dependencies (интегрированные в основной Cargo.toml)

```toml
# Уже в Cargo.toml добавлено:
[features]
default = ["lsp", "mcp"]
lsp = []
mcp = []
ml-predictions = []
web-ui = ["leptos", "leptos_meta", "leptos_router", "gloo-net", "wasm-bindgen", "console_error_panic_hook"]

[dependencies]
# Leptos web UI (условные зависимости)
leptos = { version = "0.6", features = ["csr", "nightly"], optional = true }
leptos_meta = { version = "0.6", features = ["csr", "nightly"], optional = true }
leptos_router = { version = "0.6", features = ["csr", "nightly"], optional = true }
gloo-net = { version = "0.5", optional = true }
wasm-bindgen = { version = "0.2", optional = true }
console_error_panic_hook = { version = "0.1", optional = true }

# Уже существующие:
# warp = "0.3"  - для backend API
# serde = { version = "1.0", features = ["derive"] }
# serde_json = "1.0"
```

### 🎨 Main App Component

```rust
// src/presentation/web/components/app.rs
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::presentation::web::components::{Dashboard, TypeCards, TypeTable, TypeGraph};
use crate::presentation::web::components::common::Header;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    
    view! {
        <Html lang="ru"/>
        <Title text="BSL Type System"/>
        <Meta charset="utf-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>
        
        <Router>
            <div class="app">
                <Header />
                
                <main class="main-content">
                    <Routes>
                        <Route path="/" view=Dashboard/>
                        <Route path="/cards" view=TypeCards/>
                        <Route path="/table" view=TypeTable/>
                        <Route path="/graph" view=TypeGraph/>
                    </Routes>
                </main>
            </div>
        </Router>
    }
}
```

### 📊 Dashboard Component

```rust
// src/presentation/web/components/dashboard.rs
use leptos::*;
use serde::{Deserialize, Serialize};

use crate::presentation::web::components::api::ApiClient;
use crate::presentation::web::components::common::MetricCard;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub total_types: u32,
    pub known_types: u32,
    pub inferred_types: u32,
    pub unknown_types: u32,
    pub flow_sensitive_types: u32,
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let (metrics, set_metrics) = create_signal(None::<DashboardMetrics>);
    let (loading, set_loading) = create_signal(true);
    
    // Загружаем данные при монтировании
    create_effect(move |_| {
        spawn_local(async move {
            match ApiClient::get_dashboard_metrics().await {
                Ok(data) => {
                    set_metrics(Some(data));
                    set_loading(false);
                },
                Err(e) => {
                    log::error!("Failed to load metrics: {}", e);
                    set_loading(false);
                }
            }
        });
    });
    
    view! {
        <div class="dashboard">
            <div class="dashboard-header">
                <h1>"🚀 BSL Type System Dashboard"</h1>
                <p>"Gradual Typing with Facets & Flow-Sensitive Analysis"</p>
            </div>
            
            {move || {
                if loading() {
                    view! {
                        <div class="loading">
                            <div class="spinner"></div>
                            <p>"Загрузка метрик..."</p>
                        </div>
                    }.into_view()
                } else if let Some(m) = metrics() {
                    view! {
                        <div class="metrics-grid">
                            <MetricCard
                                title="Total Types"
                                value=m.total_types
                                icon="📊"
                                color="blue"
                            />
                            <MetricCard
                                title="Known Types"
                                value=m.known_types
                                icon="✅"
                                color="green"
                            />
                            <MetricCard
                                title="Inferred Types"
                                value=m.inferred_types
                                icon="🔍"
                                color="yellow"
                            />
                            <MetricCard
                                title="Unknown Types"
                                value=m.unknown_types
                                icon="❓"
                                color="red"
                            />
                            <MetricCard
                                title="Flow-Sensitive"
                                value=m.flow_sensitive_types
                                icon="🔄"
                                color="purple"
                            />
                        </div>
                        
                        <TypeOverview />
                        <RecentActivity />
                    }.into_view()
                } else {
                    view! {
                        <div class="error">
                            <p>"Ошибка загрузки данных"</p>
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}

#[component]
fn TypeOverview() -> impl IntoView {
    view! {
        <div class="type-overview">
            <h2>"Type Distribution"</h2>
            // Здесь будет chart с помощью plotters или canvas
            <div class="chart-container" id="type-chart">
                // Chart будет рендериться через web-sys
            </div>
        </div>
    }
}

#[component]
fn RecentActivity() -> impl IntoView {
    view! {
        <div class="recent-activity">
            <h2>"Recent Type Changes"</h2>
            <div class="activity-list">
                // WebSocket updates здесь
            </div>
        </div>
    }
}
```

### 🃏 Type Cards Component

```rust
// src/presentation/web/components/type_cards.rs
use leptos::*;
use crate::domain::models::TypeInfo; // Используем существующие domain models

use crate::presentation::web::components::api::ApiClient;
use crate::presentation::web::components::common::SearchBar;

#[component]
pub fn TypeCards() -> impl IntoView {
    let (types, set_types) = create_signal(Vec::<TypeInfo>::new());
    let (search_query, set_search_query) = create_signal(String::new());
    let (loading, set_loading) = create_signal(true);
    
    // Загружаем типы
    create_effect(move |_| {
        spawn_local(async move {
            match ApiClient::get_types().await {
                Ok(data) => {
                    set_types(data);
                    set_loading(false);
                },
                Err(e) => {
                    log::error!("Failed to load types: {}", e);
                    set_loading(false);
                }
            }
        });
    });
    
    // Фильтрация по поиску
    let filtered_types = move || {
        let query = search_query().to_lowercase();
        if query.is_empty() {
            types()
        } else {
            types().into_iter()
                .filter(|t| t.name.to_lowercase().contains(&query))
                .collect()
        }
    };
    
    view! {
        <div class="type-cards-view">
            <div class="cards-header">
                <h1>"🃏 BSL Type Explorer"</h1>
                <SearchBar 
                    value=search_query
                    on_input=move |query| set_search_query(query)
                    placeholder="Поиск типов... (например: Массив, Справочники)"
                />
            </div>
            
            {move || {
                if loading() {
                    view! { <div class="loading">"Загрузка типов..."</div> }.into_view()
                } else {
                    view! {
                        <div class="cards-grid">
                            <For
                                each=filtered_types
                                key=|type_info| type_info.id.clone()
                                children=move |type_info| {
                                    view! { <TypeCard type_info=type_info /> }
                                }
                            />
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}

#[component]
fn TypeCard(type_info: TypeInfo) -> impl IntoView {
    let css_class = match type_info.certainty {
        100 => "type-card card-known",
        50..=99 => "type-card card-inferred",
        _ => "type-card card-unknown",
    };
    
    let certainty_badge_class = match type_info.certainty {
        100 => "certainty-badge badge-known",
        50..=99 => "certainty-badge badge-inferred", 
        _ => "certainty-badge badge-unknown",
    };
    
    view! {
        <div class=css_class>
            <div class="type-header">
                <div class="type-name">{&type_info.name}</div>
                <div class=certainty_badge_class>
                    {format!("{}%", type_info.certainty)}
                </div>
            </div>
            
            <div class="type-details">
                <div class="detail-row">
                    <span class="detail-label">"Категория:"</span>
                    <span class="detail-value">{&type_info.category}</span>
                </div>
                <div class="detail-row">
                    <span class="detail-label">"Источник:"</span>
                    <span class="detail-value">{&type_info.source}</span>
                </div>
            </div>
            
            <div class="facets-section">
                <strong>"Фасеты:"</strong>
                <div class="facets-container">
                    <For
                        each=move || type_info.facets.clone()
                        key=|facet| facet.clone()
                        children=move |facet| {
                            let facet_class = format!("facet-tag facet-{}", facet.to_lowercase());
                            view! {
                                <span class=facet_class>{facet}</span>
                            }
                        }
                    />
                </div>
            </div>
            
            {if let Some(ref union_types) = type_info.union_types {
                view! {
                    <div class="union-types">
                        <strong>"Возможные типы:"</strong>
                        <For
                            each=move || union_types.clone()
                            key=|ut| ut.type_name.clone()
                            children=move |union_type| {
                                view! {
                                    <div class="union-type">
                                        <span>{&union_type.type_name}</span>
                                        <div class="weight-bar">
                                            <div 
                                                class="weight-fill" 
                                                style=format!("width: {}%", union_type.weight)
                                            ></div>
                                        </div>
                                        <span>{format!("{}%", union_type.weight)}</span>
                                    </div>
                                }
                            }
                        />
                    </div>
                }.into_view()
            } else {
                view! { <div></div> }.into_view()
            }}
        </div>
    }
}
```

### 🌐 API Client

```rust
// src/presentation/web/components/api/client.rs
use gloo_net::http::Request;
use serde_json::Value;
use crate::domain::models::TypeInfo; // Используем domain models
use crate::presentation::web::components::dashboard::DashboardMetrics;

pub struct ApiClient;

impl ApiClient {
    const BASE_URL: &'static str = "http://localhost:8080/api";
    
    pub async fn get_dashboard_metrics() -> Result<DashboardMetrics, Box<dyn std::error::Error>> {
        let response = Request::get(&format!("{}/metrics", Self::BASE_URL))
            .send()
            .await?;
            
        let metrics: DashboardMetrics = response.json().await?;
        Ok(metrics)
    }
    
    pub async fn get_types() -> Result<Vec<TypeInfo>, Box<dyn std::error::Error>> {
        let response = Request::get(&format!("{}/types", Self::BASE_URL))
            .send()
            .await?;
            
        let data: Value = response.json().await?;
        let types: Vec<TypeInfo> = serde_json::from_value(data["types"].clone())?;
        Ok(types)
    }
    
    pub async fn search_types(query: &str) -> Result<Vec<TypeInfo>, Box<dyn std::error::Error>> {
        let url = format!("{}/search?q={}", Self::BASE_URL, query);
        let response = Request::get(&url).send().await?;
        
        let data: Value = response.json().await?;
        let results: Vec<TypeInfo> = serde_json::from_value(data["results"].clone())?;
        Ok(results)
    }
}
```

## 🚀 Преимущества интегрированной Leptos архитектуры:

1. **🔥 Единый проект:** Все в одном Cargo.toml с условными features
2. **🦀 Type Safety:** Используем domain models из `src/domain/`
3. **📱 Реактивность:** Современная реактивная система Leptos
4. **🔄 Интеграция:** Прямая интеграция с SystemCoordinator и TypeSystemService
5. **📦 Условная компиляция:** Feature "web-ui" включается только при необходимости
6. **🎯 Clean Architecture:** Leptos компоненты в presentation/web/ слое

## 🛠️ Команды для разработки:

```bash
# Сборка только backend (без веб-UI)
cargo build

# Сборка с Leptos веб-интерфейсом
cargo build --features web-ui

# Запуск веб-сервера с Leptos
cargo run --bin web-server --features web-ui

# Разработка (watch mode)
trunk serve --features web-ui
```

## 📋 Следующие шаги реализации:

1. **Создать базовые Leptos компоненты** в `src/presentation/web/components/`
2. **Расширить web_server.rs** для обслуживания Leptos приложения
3. **Добавить WebSocket** для real-time обновлений
4. **Интегрировать с TypeSystemService** для получения реальных данных
5. **Создать статические файлы** (index.html, CSS)

Готов к началу реализации компонентов! 🚀
