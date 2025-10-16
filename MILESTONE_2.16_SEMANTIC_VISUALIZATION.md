# MILESTONE 2.16: Real-time Semantic Tree Visualization

**Цель:** Создать систему визуализации семантического анализа BSL модулей с поддержкой двух клиентов:
- 📦 VSCode Extension (Webview Panel)
- 🌐 Web Dashboard (Leptos Frontend)

**Статус:** 🚧 В разработке
**Дата начала:** 2025-10-11
**Приоритет:** HIGH (улучшает UX и демонстрирует возможности системы)

---

## 📋 Содержание

1. [Архитектура передачи данных](#архитектура-передачи-данных)
2. [DTO структуры](#dto-структуры)
3. [LSP Custom Requests](#lsp-custom-requests)
4. [Web API Endpoints](#web-api-endpoints)
5. [Клиентская визуализация](#клиентская-визуализация)
6. [Roadmap задач](#roadmap-задач)
7. [Примеры использования](#примеры-использования)

---

## 🏗️ Архитектура передачи данных

### Гибридный подход (рекомендуется)

**Идея:** LSP сервер предоставляет ДВА формата данных — клиент выбирает нужный:

```
                    ┌─────────────────────────┐
                    │   LSP Server (Rust)     │
                    │  TypeSystemService      │
                    └───────────┬─────────────┘
                                │
                ┌───────────────┴───────────────┐
                │                               │
                ▼                               ▼
    ┌──────────────────────┐        ┌──────────────────────┐
    │  SemanticTreeDto     │        │  RenderedHtmlDto     │
    │  (сырые данные)      │        │  (готовый HTML)      │
    └──────────┬───────────┘        └──────────┬───────────┘
               │                               │
    ┌──────────┴──────────┐         ┌─────────┴──────────┐
    │                     │         │                    │
    ▼                     ▼         ▼                    ▼
┌─────────┐      ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│ VSCode  │      │ Web Frontend │  │ VSCode       │  │ Web Browser  │
│ renders │      │ renders      │  │ shows HTML   │  │ shows HTML   │
│ locally │      │ locally      │  │ (fallback)   │  │ (simple)     │
└─────────┘      └──────────────┘  └──────────────┘  └──────────────┘
```

### Преимущества гибридного подхода

**✅ Сырые данные (SemanticTreeDto):**
- Клиент контролирует внешний вид
- Интерактивность (fold/unfold, навигация)
- Кастомизация (темы, фильтры)
- Меньше трафика (JSON компактнее HTML)

**✅ Готовый HTML (RenderedHtmlDto):**
- Простота интеграции (не нужен сложный UI)
- Единообразие визуализации
- Работает везде (браузер, markdown preview, etc.)
- Fallback для клиентов без рендеринга

---

## 📦 DTO структуры

### 1. SemanticTreeDto (сырые данные)

```rust
// shared/src/api/semantic_dtos.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// DTO для передачи семантического дерева клиентам
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTreeDto {
    /// Путь к файлу
    pub file_path: String,

    /// Корневые узлы (procedures, functions, global scope)
    pub root_nodes: Vec<SemanticNodeDto>,

    /// Таблица символов (имя → информация)
    pub symbol_table: HashMap<String, SymbolInfoDto>,

    /// Граф вызовов функций
    pub call_graph: Vec<CallEdgeDto>,

    /// Метрики анализа
    pub metrics: SemanticMetricsDto,
}

/// DTO для узла семантического дерева
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNodeDto {
    /// Тип узла (Procedure, Function, Variable, IfStatement, etc.)
    pub kind: String,

    /// Имя (для именованных узлов)
    pub name: Option<String>,

    /// Позиция в исходном коде
    pub location: SourceLocationDto,

    /// Вложенные узлы
    pub children: Vec<SemanticNodeDto>,

    /// Дополнительные атрибуты (тип переменной, параметры функции, etc.)
    pub attributes: HashMap<String, String>,
}

/// DTO для символа в таблице символов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfoDto {
    /// Имя символа
    pub name: String,

    /// Тип символа (Variable, Parameter, Function, Procedure)
    pub kind: String,

    /// Разрешённый тип (из TypeResolver)
    pub resolved_type: Option<TypeResolutionDto>,

    /// Область видимости (Global, Local, Parameter)
    pub scope: String,

    /// Позиция объявления
    pub declaration_location: SourceLocationDto,

    /// Flow-sensitive информация (если тип меняется в условиях)
    pub flow_variants: Vec<FlowTypeVariantDto>,
}

/// DTO для flow-sensitive типов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTypeVariantDto {
    /// Условие, при котором тип имеет это значение
    pub condition: String,

    /// Тип в этой ветке
    pub type_info: TypeResolutionDto,

    /// Диапазон кода, где действует этот тип
    pub range: SourceRangeDto,
}

/// DTO для рёбра графа вызовов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdgeDto {
    /// Откуда вызов (имя функции/процедуры)
    pub from: String,

    /// Куда вызов (имя функции/процедуры)
    pub to: String,

    /// Позиция вызова в коде
    pub location: SourceLocationDto,
}

/// DTO для метрик семантического анализа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMetricsDto {
    /// Количество процедур
    pub procedure_count: usize,

    /// Количество функций
    pub function_count: usize,

    /// Количество переменных
    pub variable_count: usize,

    /// Количество типов Known
    pub known_types: usize,

    /// Количество типов Inferred
    pub inferred_types: usize,

    /// Количество типов Unknown
    pub unknown_types: usize,

    /// Средняя уверенность типизации (0.0 - 1.0)
    pub average_certainty: f32,

    /// Время анализа (мс)
    pub analysis_time_ms: u64,
}

/// DTO для позиции в исходном коде
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocationDto {
    pub line: u32,
    pub column: u32,
}

/// DTO для диапазона в исходном коде
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRangeDto {
    pub start: SourceLocationDto,
    pub end: SourceLocationDto,
}

/// DTO для TypeResolution (переиспользуем существующий из shared/api/dtos.rs)
use super::dtos::TypeDto; // Уже существует

/// Обёртка для TypeResolution в контексте семантики
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeResolutionDto {
    /// Имя типа
    pub name: String,

    /// Категория (Platform, Configuration, Runtime)
    pub category: String,

    /// Уверенность (Known, Inferred, Unknown)
    pub certainty: String,

    /// Процент уверенности (0-100)
    pub certainty_percent: u8,

    /// Активный фасет (Manager, Object, Reference, Selection, List)
    pub active_facet: Option<String>,

    /// Доступные методы
    pub methods: Vec<String>,

    /// Доступные свойства
    pub properties: Vec<String>,
}
```

### 2. RenderedHtmlDto (готовый HTML)

```rust
// shared/src/api/semantic_dtos.rs

/// DTO для готового HTML рендеринга
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedHtmlDto {
    /// Путь к файлу
    pub file_path: String,

    /// Полный HTML документ (с CSS стилями)
    pub html: String,

    /// Метрики (для отображения в статусбаре/хедере)
    pub metrics: SemanticMetricsDto,

    /// Timestamp генерации
    pub generated_at: String,
}
```

---

## 🔌 LSP Custom Requests

### 1. `bsl/getSemanticTree` — получить сырые данные

**Request:**
```json
{
  "method": "bsl/getSemanticTree",
  "params": {
    "uri": "file:///path/to/module.bsl",
    "includeCallGraph": true,
    "includeFlowSensitive": true
  }
}
```

**Response:**
```json
{
  "file_path": "module.bsl",
  "root_nodes": [...],
  "symbol_table": {...},
  "call_graph": [...],
  "metrics": {...}
}
```

### 2. `bsl/getSemanticHtml` — получить готовый HTML

**Request:**
```json
{
  "method": "bsl/getSemanticHtml",
  "params": {
    "uri": "file:///path/to/module.bsl",
    "theme": "dark",
    "compact": false
  }
}
```

**Response:**
```json
{
  "file_path": "module.bsl",
  "html": "<!DOCTYPE html>...",
  "metrics": {...},
  "generated_at": "2025-10-11T12:00:00Z"
}
```

### 3. `bsl/visualizeSemantics` — команда для открытия панели

**Execute Command (VSCode):**
```typescript
await client.sendRequest('workspace/executeCommand', {
  command: 'bsl.visualizeSemantics',
  arguments: [editor.document.uri.toString()]
});
```

**LSP обрабатывает команду:**
- Генерирует SemanticTreeDto
- Отправляет уведомление клиенту: `window/showDocument` или кастомный `bsl/showSemanticPanel`
- Клиент открывает webview panel и запрашивает данные через `bsl/getSemanticTree`

---

## 🌐 Web API Endpoints

### 1. `GET /api/semantic/:file_id` — получить данные

**Request:**
```bash
curl http://localhost:3002/api/semantic/module.bsl?format=json
```

**Response (JSON):**
```json
{
  "file_path": "module.bsl",
  "root_nodes": [...],
  "symbol_table": {...},
  "metrics": {...}
}
```

**Request (HTML):**
```bash
curl http://localhost:3002/api/semantic/module.bsl?format=html
```

**Response (HTML):**
```html
<!DOCTYPE html>
<html>
  <head>...</head>
  <body>
    <div class="semantic-view">...</div>
  </body>
</html>
```

### 2. `POST /api/semantic/analyze` — анализ произвольного кода

**Request:**
```json
{
  "code": "Процедура Тест()\n  МассивДанных = Новый Массив();\nКонецПроцедуры",
  "format": "json"
}
```

**Response:**
```json
{
  "file_path": "<inline>",
  "root_nodes": [...],
  "symbol_table": {...}
}
```

### 3. `GET /api/semantic/stream/:file_id` — Server-Sent Events (live updates)

**Request:**
```javascript
const eventSource = new EventSource('/api/semantic/stream/module.bsl');
eventSource.onmessage = (event) => {
  const data = JSON.parse(event.data);
  updateVisualization(data);
};
```

**Server отправляет обновления при изменении файла (через file watcher):**
```json
{
  "type": "update",
  "data": { "file_path": "...", "metrics": {...} }
}
```

---

## 🎨 Клиентская визуализация

### VSCode Extension (TypeScript + React/Preact)

**Структура:**
```
vscode-extension/
├── src/
│   ├── webview/
│   │   ├── SemanticPanel.tsx         # Главный компонент webview
│   │   ├── SemanticTreeView.tsx      # Дерево узлов (fold/unfold)
│   │   ├── SymbolTableView.tsx       # Таблица символов
│   │   ├── CallGraphView.tsx         # Граф вызовов (D3.js/Cytoscape)
│   │   ├── MetricsView.tsx           # Метрики анализа
│   │   └── styles.css                # Стили (адаптация тем VSCode)
│   └── extension.ts                  # Регистрация команд
```

**Пример компонента (SemanticPanel.tsx):**
```tsx
import React, { useEffect, useState } from 'react';
import { SemanticTreeDto } from './types';

export const SemanticPanel: React.FC = () => {
  const [data, setData] = useState<SemanticTreeDto | null>(null);

  useEffect(() => {
    // Запрос данных от LSP
    window.acquireVsCodeApi().postMessage({
      command: 'getSemanticTree'
    });

    // Получение данных
    window.addEventListener('message', (event) => {
      if (event.data.type === 'semanticTree') {
        setData(event.data.payload);
      }
    });
  }, []);

  if (!data) return <div>Loading...</div>;

  return (
    <div className="semantic-panel">
      <MetricsView metrics={data.metrics} />
      <SemanticTreeView nodes={data.root_nodes} />
      <SymbolTableView symbols={data.symbol_table} />
      <CallGraphView edges={data.call_graph} />
    </div>
  );
};
```

### Web Frontend (Leptos WASM)

**Структура:**
```
frontend/src/
├── pages/
│   └── semantic.rs                    # Страница семантической визуализации
├── components/
│   ├── semantic_tree.rs               # Компонент дерева
│   ├── symbol_table.rs                # Компонент таблицы символов
│   ├── call_graph.rs                  # Компонент графа (canvas/svg)
│   └── metrics_dashboard.rs           # Метрики
└── api/
    └── semantic_client.rs             # HTTP клиент для API
```

**Пример компонента (semantic_tree.rs):**
```rust
use leptos::*;
use crate::api::SemanticTreeDto;

#[component]
pub fn SemanticTree(
    nodes: ReadSignal<Vec<SemanticNodeDto>>,
) -> impl IntoView {
    view! {
        <div class="semantic-tree">
            <For
                each=move || nodes.get()
                key=|node| node.name.clone().unwrap_or_default()
                children=move |node| {
                    view! { <TreeNode node=node /> }
                }
            />
        </div>
    }
}

#[component]
fn TreeNode(node: SemanticNodeDto) -> impl IntoView {
    let (expanded, set_expanded) = create_signal(false);

    view! {
        <div class="tree-node">
            <div
                class="node-header"
                on:click=move |_| set_expanded.update(|e| *e = !*e)
            >
                <span class="node-kind">{node.kind}</span>
                {node.name.map(|n| view! { <span class="node-name">{n}</span> })}
            </div>

            <Show when=move || expanded.get()>
                <div class="node-children">
                    <For
                        each=move || node.children.clone()
                        key=|child| format!("{:?}", child.location)
                        children=move |child| view! { <TreeNode node=child /> }
                    />
                </div>
            </Show>
        </div>
    }
}
```

---

## 📋 Roadmap задач

### Phase 1: Инфраструктура DTO (2-3 дня)

- [x] **Task 1.1:** Создать `shared/src/api/semantic_dtos.rs`
  - SemanticTreeDto, SemanticNodeDto, SymbolInfoDto
  - RenderedHtmlDto
  - Все вспомогательные DTO

- [ ] **Task 1.2:** Добавить конверторы из IR в DTO
  - `shared/src/ir/mod.rs::to_semantic_dto()`
  - Маппинг SemanticProgram → SemanticTreeDto
  - Маппинг SymbolTable → HashMap<String, SymbolInfoDto>

- [ ] **Task 1.3:** Добавить конверторы TypeResolution → TypeResolutionDto
  - Переиспользовать существующий TypeDto
  - Адаптация для семантического контекста

### Phase 2: LSP Custom Requests (2-3 дня)

- [ ] **Task 2.1:** Реализовать `bsl/getSemanticTree` в `lsp_server.rs`
  - Обработка request
  - Вызов TypeSystemService
  - Конвертация в DTO
  - Отправка response

- [ ] **Task 2.2:** Реализовать `bsl/getSemanticHtml` в `lsp_server.rs`
  - Использование HtmlRenderer
  - Генерация полного HTML документа
  - Кэширование (опционально)

- [ ] **Task 2.3:** Реализовать команду `bsl.visualizeSemantics`
  - Execute command handler
  - Открытие webview panel (через клиента)

### Phase 3: Web API Endpoints (2-3 дня)

- [ ] **Task 3.1:** Создать `backend/src/presentation/semantic_routes.rs`
  - `GET /api/semantic/:file_id?format=json`
  - `GET /api/semantic/:file_id?format=html`
  - `POST /api/semantic/analyze`

- [ ] **Task 3.2:** Добавить SSE endpoint (опционально)
  - `GET /api/semantic/stream/:file_id`
  - File watcher для live updates
  - Debouncing (500ms)

- [ ] **Task 3.3:** Интеграция с SystemCoordinator
  - Переиспользование TypeSystemService
  - Кэширование результатов

### Phase 4: VSCode Extension (3-4 дня)

- [ ] **Task 4.1:** Настроить React/Preact для webview
  - Webpack/Vite config
  - TypeScript types для DTO
  - CSS стили (адаптация тем VSCode)

- [ ] **Task 4.2:** Реализовать компоненты
  - SemanticPanel (main)
  - SemanticTreeView (fold/unfold)
  - SymbolTableView (таблица)
  - MetricsView (статистика)

- [ ] **Task 4.3:** Реализовать CallGraphView (опционально)
  - D3.js или Cytoscape.js
  - Интерактивный граф вызовов

- [ ] **Task 4.4:** Добавить команды и UI
  - Command Palette: "BSL: Visualize Semantics"
  - Context Menu: "Visualize Semantics"
  - Status Bar: метрики анализа

### Phase 5: Web Frontend (3-4 дня)

- [ ] **Task 5.1:** Создать страницу `/semantic` в Leptos
  - Routing
  - HTTP клиент для API

- [ ] **Task 5.2:** Реализовать компоненты Leptos
  - SemanticTree (reactive)
  - SymbolTable (sortable, filterable)
  - MetricsDashboard

- [ ] **Task 5.3:** Добавить CallGraph визуализацию (опционально)
  - Canvas/SVG рендеринг
  - Интерактивность

- [ ] **Task 5.4:** Интеграция с главной страницей
  - Кнопка "Visualize" для каждого типа
  - Deep links (share URL)

### Phase 6: Тестирование и оптимизация (2-3 дня)

- [ ] **Task 6.1:** Написать unit тесты
  - Конверторы IR → DTO
  - LSP request handlers
  - Web API endpoints

- [ ] **Task 6.2:** Написать интеграционные тесты
  - End-to-end: BSL файл → LSP → VSCode webview
  - End-to-end: BSL файл → Web API → Frontend

- [ ] **Task 6.3:** Performance оптимизация
  - Кэширование результатов
  - Debouncing для live updates
  - Lazy loading для больших деревьев

- [ ] **Task 6.4:** UI/UX полировка
  - Темизация (Light/Dark/HighContrast)
  - Accessibility (ARIA labels)
  - Keyboard navigation

---

## 📚 Примеры использования

### Пример 1: VSCode Extension

```typescript
// Пользователь открывает модуль в редакторе
// Нажимает Ctrl+Shift+P → "BSL: Visualize Semantics"

// Extension отправляет запрос LSP серверу
const response = await client.sendRequest('bsl/getSemanticTree', {
  uri: document.uri.toString(),
  includeCallGraph: true,
  includeFlowSensitive: true,
});

// Открывается webview panel с визуализацией
panel.webview.html = generateWebviewHtml(response);

// При изменении документа — автоматически обновляется
workspace.onDidChangeTextDocument((event) => {
  if (event.document.uri === activeDocument.uri) {
    debounce(() => {
      updateSemanticPanel(event.document.uri);
    }, 500);
  }
});
```

### Пример 2: Web Dashboard

```rust
// Пользователь открывает http://localhost:3002/semantic?file=module.bsl

// Leptos компонент загружает данные
#[component]
pub fn SemanticPage() -> impl IntoView {
    let (data, set_data) = create_signal(None);

    create_effect(move |_| {
        spawn_local(async move {
            let response = fetch_semantic_tree("module.bsl").await;
            set_data.set(Some(response));
        });
    });

    view! {
        <SemanticDashboard data=data />
    }
}
```

### Пример 3: CLI экспорт

```bash
# Анализ модуля и экспорт в HTML
cargo run --bin bsl-cli -- analyze module.bsl --visualize --output report.html

# Открытие в браузере
start report.html  # Windows
```

---

## 🎯 Критерии успеха

### MVP (Минимально жизнеспособный продукт)

- ✅ LSP custom request `bsl/getSemanticTree` работает
- ✅ Web API endpoint `/api/semantic/:file_id` возвращает JSON
- ✅ VSCode Extension показывает базовое дерево (без графа)
- ✅ Web Frontend показывает таблицу символов

### Full Release

- ✅ Все endpoints реализованы (JSON + HTML)
- ✅ VSCode Extension: интерактивное дерево + граф вызовов
- ✅ Web Frontend: полная визуализация + live updates
- ✅ Flow-sensitive анализ отображается
- ✅ Темизация работает (Light/Dark/HighContrast)
- ✅ Performance: анализ файла 1000 строк < 100ms

### Nice to Have

- ⭐ Экспорт в PDF/SVG
- ⭐ Diff между версиями модуля
- ⭐ Интеграция с Git (анализ изменений)
- ⭐ AI-powered инсайты ("Эта функция слишком сложная")

---

## 🔧 Технические детали

### Производительность

**Проблема:** Большие файлы (>1000 строк) могут генерировать огромные DTO.

**Решение:**
1. **Пагинация:** Отправлять дерево по кускам (top-level nodes + lazy load children)
2. **Compression:** gzip для HTTP/LSP responses
3. **Incremental updates:** Отправлять только изменённые узлы

### Безопасность

**Проблема:** Web API endpoint может быть использован для DoS атак.

**Решение:**
1. **Rate limiting:** 10 запросов/минуту на IP
2. **Timeout:** Анализ файла максимум 5 секунд
3. **Size limit:** Максимальный размер файла 10 MB

### Кэширование

**Стратегия:**
- LSP: кэш в памяти (AnalysisCache) — TTL 5 минут
- Web API: кэш в Redis (опционально) — TTL 10 минут
- Ключ кэша: `hash(file_path + content + options)`

---

## 📝 Заметки по реализации

### Конвертация SemanticProgram → SemanticTreeDto

```rust
// shared/src/ir/mod.rs

impl SemanticProgram {
    pub fn to_dto(&self) -> SemanticTreeDto {
        SemanticTreeDto {
            file_path: self.source_map.file_path.clone(),
            root_nodes: self.nodes.iter()
                .map(|n| n.to_dto())
                .collect(),
            symbol_table: self.symbol_table.to_dto(),
            call_graph: self.extract_call_graph(),
            metrics: self.calculate_metrics(),
        }
    }
}

impl SemanticNode {
    fn to_dto(&self) -> SemanticNodeDto {
        SemanticNodeDto {
            kind: format!("{:?}", self.kind),
            name: self.get_name(),
            location: SourceLocationDto {
                line: self.span.start.line,
                column: self.span.start.column,
            },
            children: self.children.iter()
                .map(|c| c.to_dto())
                .collect(),
            attributes: self.extract_attributes(),
        }
    }
}
```

### HTML рендеринг через type-visualization

```rust
// backend/src/application/type_system_service.rs

impl TypeSystemService {
    pub async fn get_semantic_html(&self, file_path: &str) -> Result<String> {
        // 1. Анализ файла
        let analysis = self.analyze_file(file_path).await?;

        // 2. Конвертация в SemanticTreeDto
        let semantic_dto = analysis.to_semantic_dto();

        // 3. Рендеринг через HtmlRenderer
        let renderer = HtmlRenderer::new(RenderOptions::default());
        let body = format_semantic_tree(&semantic_dto);

        Ok(renderer.render_document("Semantic Analysis", &body))
    }
}

fn format_semantic_tree(dto: &SemanticTreeDto) -> String {
    let mut html = String::new();

    // Метрики
    html.push_str(&format!("<div class='metrics'>{}</div>",
        format_metrics(&dto.metrics)));

    // Дерево узлов
    html.push_str("<div class='tree'>");
    for node in &dto.root_nodes {
        html.push_str(&format_node(node, 0));
    }
    html.push_str("</div>");

    // Таблица символов
    html.push_str(&format!("<div class='symbols'>{}</div>",
        format_symbol_table(&dto.symbol_table)));

    html
}
```

---

## 🚀 Начало работы

### Команды для разработки

```bash
# 1. Запуск LSP сервера с логированием
RUST_LOG=debug cargo run --bin bsl-lsp-server

# 2. Запуск Web сервера
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper

# 3. Разработка VSCode Extension
cd vscode-extension
npm install
npm run watch  # hot reload

# 4. Разработка Web Frontend
cd frontend
trunk serve --port 8080  # hot reload

# 5. Тестирование
cargo test --package bsl-backend semantic
cargo test --package bsl-shared ir::to_dto
```

### Тестовые файлы

```bsl
// cli/test_semantic.bsl

Процедура ТестоваяПроцедура()
    МассивДанных = Новый Массив();

    Для Каждого Элемент Из МассивДанных Цикл
        Если Элемент <> Неопределено Тогда
            Сообщить(Элемент);
        КонецЕсли;
    КонецЦикла;
КонецПроцедуры

Функция ВычислитьСумму(Число1, Число2)
    Результат = Число1 + Число2;
    Возврат Результат;
КонецФункции
```

**Ожидаемый результат визуализации:**
- 2 корневых узла (Procedure, Function)
- 3 символа в таблице (МассивДанных, Элемент, Результат)
- Flow-sensitive для Элемент (non-null в then, null в else)
- 1 вызов в графе (ТестоваяПроцедура → Сообщить)

---

## 📊 Метрики успешности

**Будем отслеживать:**

- ⏱️ **Performance:** Время анализа файла (target: <100ms для 1000 строк)
- 📦 **Size:** Размер DTO (target: <500KB для файла 1000 строк)
- 🎯 **Accuracy:** Корректность символов в таблице (target: 95%+)
- 🚀 **Responsiveness:** Задержка обновления при редактировании (target: <500ms)
- 🎨 **UX:** Время до первого рендера (target: <1s)

---

## 🎉 Заключение

MILESTONE 2.12 создаёт **мощную систему визуализации**, которая:

✅ Демонстрирует возможности семантического анализа
✅ Упрощает отладку системы типов
✅ Улучшает UX для разработчиков на 1С
✅ Создаёт основу для будущих AI-powered инсайтов

**Приоритет:** HIGH — это killer feature для проекта! 🚀
