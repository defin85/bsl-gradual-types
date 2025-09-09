# BSL Type System — Leptos Frontend (CSR Only)

Этот документ описывает реализацию веб‑интерфейса на Leptos в режиме CSR (Client‑Side Rendering). SSR полностью убран: сервер не рендерит HTML, а только отдаёт статические ассеты (WASM/JS/CSS) и обслуживает REST API.

## Обзор

- Архитектура: SPA на Leptos (WASM), данные через REST API.
- Сервер: Axum — только API и раздача статических файлов; без `leptos_axum`, без SSR‑роутов.
- Сборка фронта: Trunk компилирует Rust → WASM и кладёт файлы в `target/site`.
- Роутинг: клиентский (`leptos_router`), SPA‑fallback на `index.html`.

## Структура проекта (Workspace в `src/presentation`)

```
bsl-gradual-types/
├── Cargo.toml                   # [workspace] members = [
│                                #   "src/presentation/shared",
│                                #   "src/presentation/frontend",
│                                #   "src/presentation/backend"
│                                # ]
└── src/
    └── presentation/
        ├── shared/              # crate: общие типы (DTO/модели)
        │   └── Cargo.toml
        ├── frontend/            # crate: Leptos (WASM, CSR)
        │   ├── Cargo.toml
        │   ├── Trunk.toml       # dist = ../backend/target/site
        │   ├── index.html       # <link data-trunk rel="rust" />
        │   └── src/
        │       └── web/
        │           ├── mod.rs
        │           ├── client.rs
        │           └── components/
        │               ├── mod.rs
        │               ├── app.rs
        │               ├── dashboard.rs
        │               ├── type_cards.rs
        │               ├── type_table.rs
        │               ├── type_graph.rs
        │               └── common/
        │                   ├── header.rs
        │                   ├── search_bar.rs
        │                   └── metric_card.rs
        └── backend/             # crate: Axum (API + статика)
            ├── Cargo.toml
            └── src/bin/web_server.rs
```

## Зависимости и фичи (Cargo.toml)

```toml
[features]
web-ui = [
  "leptos", "leptos_meta", "leptos_router",
  "gloo-net", "wasm-bindgen", "console_error_panic_hook", "log"
]
web-server = ["axum", "tower", "tower-http"]

[dependencies]
leptos        = { version = "0.6", features = ["csr"], optional = true }
leptos_meta   = { version = "0.6", features = ["csr"], optional = true }
leptos_router = { version = "0.6", features = ["csr"], optional = true }
gloo-net      = { version = "0.5", optional = true }
wasm-bindgen  = { version = "0.2", optional = true }
console_error_panic_hook = { version = "0.1", optional = true }
log           = { version = "0.4", optional = true }

axum       = { version = "0.7", optional = true }
tower      = { version = "0.4", optional = true }
tower-http = { version = "0.5", features = ["fs"], optional = true }
```

SSR‑зависимости (`leptos_axum`, `leptos/ssr`, `leptos_router/ssr`) не используются.

## Главный компонент (App)

```rust
// frontend/src/web/components/app.rs
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::web::components::{Dashboard, TypeCards, TypeTable, TypeGraph};
use crate::web::components::common::Header;

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

## WASM entrypoint (CSR)

```rust
// frontend/src/web/client.rs
#[cfg(all(feature = "web-ui", target_arch = "wasm32"))]
use wasm_bindgen::prelude::*;

#[cfg(all(feature = "web-ui", target_arch = "wasm32"))]
#[wasm_bindgen(start)]
pub fn main_js() {
    use leptos::*;
    use crate::web::components::app::App;

    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> });
}
```

## API‑клиент (браузер)

```rust
// src/presentation/web/components/api/client.rs
// (сокращённо) — использует gloo-net::http::Request
const BASE_URL: &str = "http://localhost:8080/api";

// Примеры методов:
// GET {BASE_URL}/metrics -> DashboardMetrics
// GET {BASE_URL}/types   -> { "types": Vec<TypeResolution> }
// GET {BASE_URL}/search?q=... -> { "results": Vec<TypeResolution> }
```

## Сервер (без SSR): API + статика

```rust
// Идея: Axum отдаёт /api/* и статические файлы из target/site
use axum::{routing::get, Router};
use tower_http::services::{ServeDir, ServeFile};

let static_dir = ServeDir::new("target/site")
    .not_found_service(ServeFile::new("target/site/index.html"));

let app = Router::new()
    .route("/api/metrics", get(get_metrics))
    .route("/api/types", get(get_types))
    .route("/api/search", get(search_types))
    .fallback_service(static_dir);
```

SPA‑fallback обеспечивает работу клиентских роутов (`/cards`, `/table`, `/graph`) при прямом открытии URL.

## index.html (Trunk)

```html
<!DOCTYPE html>
<html lang="ru">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>BSL Type System</title>
    <base href="/" />
    <link data-trunk rel="rust" data-cargo-features="web-ui" />
  </head>
  <body>
    <noscript>Для работы требуется включенный JavaScript.</noscript>
  </body>
  </html>
```

## Команды сборки и запуска

```bash
# 1) Сборка фронта (WASM) — кладёт файлы в backend/target/site
cd src/presentation/frontend
trunk build --features web-ui

# 2) Запуск сервера (API + статика) из backend
cd ../backend
cargo run --bin web-server --features "web-ui,web-server"

# Dev-режим с HMR (по желанию):
cd src/presentation/frontend && trunk serve --features web-ui --open
# и отдельно запустить API (cd ../backend && cargo run); настроить прокси /api в Trunk
```

Примечание (Windows): некоторые зависимости для wasm могут требовать C‑toolchain (LLVM/Clang). Установите LLVM, если сборка жалуется на `clang`.

## Зависимости между crate’ами (workspace в `src/presentation`)

- `src/presentation/frontend/Cargo.toml`:
  `shared = { path = "../shared" }`
- `src/presentation/backend/Cargo.toml`:
  `shared = { path = "../shared" }`

Плюсы такого расположения: всё, что касается презентационного слоя (UI/API), хранится в `src/presentation`, при этом сохраняется чистая изоляция зависимостей между фронтом (wasm) и беком (native).

## Почему CSR

- SEO не критичен, интерфейс — внутренний дашборд.
- Простой сервер: меньше зависимостей, без SSR‑пайплайна.
- Быстрый цикл разработки: Trunk + клиентский роутинг.

## Что изменилось по сравнению с SSR‑версией

- Полностью убран `leptos_axum`, `generate_route_list`, `LeptosRoutes`.
- Нет `ssr`‑фич и SSR‑маршрутов.
- Сервер не рендерит HTML, только отдаёт SPA‑ассеты и API.

## Контракты API (сводно)

- `GET /api/metrics` → `DashboardMetrics`
- `GET /api/types`   → `{ "types": Vec<TypeResolution> }`
- `GET /api/search?q=...` → `{ "results": Vec<TypeResolution> }`

Рекомендуется держать эти контракты синхронизированными между сервером и клиентом.

---

Эта версия документа отражает только CSR‑подход. Если в будущем потребуется SSR (SEO, быстрый первый рендер, метаданные) — можно вернуть `leptos_axum` и гидрацию, не меняя структуру компонентов.
