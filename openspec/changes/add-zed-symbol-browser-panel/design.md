## Context

Панель Symbol Browser добавляется в ядро Zed через форк `defin85/zed`. Основная техническая сложность — GPUI async-паттерны для вызова `project.symbols()` из панели.

## Goals / Non-Goals

- **Goals**: статическая панель с группировкой символов по `SymbolKind`, запрос через LSP `workspace/symbol`
- **Non-Goals**: интерактив (клик, сворачивание), поиск, BSL-specific группировка

## Decisions

### Decision 1: Паттерн вызова `project.symbols()` из панели

**Проблема**: `project.symbols("", cx)` возвращает `Task<Result<Vec<Symbol>>>`. Чтобы получить результат, нужно:
1. Вызвать `project.update(cx, |project, cx| project.symbols("", cx))` — возвращает `Result<Task<...>>` или `Task<...>`
2. Если возвращается `Task`, сохранить её и позже `await`
3. После `await` — обновить состояние панели через `cx.notify()`

**Рабочий паттерн** (найден в `outline_panel`):
```rust
fn fetch_symbols(&mut self, project: Entity<Project>, cx: &mut Context<Self>) {
    // Шаг 1: получаем Task в синхронном контексте
    let task = project.update(cx, |project, cx| project.symbols("", cx));
    
    // Шаг 2: spawn async future
    self._fetch_task = cx.spawn(async move |this, mut cx| {
        let symbols = match task {
            Ok(fetch) => fetch.await.unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        
        this.update(&mut cx, |this, cx| {
            this.groups = group_symbols(symbols);
            cx.notify();
        }).ok();
    });
}
```

**Альтернативы**:
- `cx.spawn_in(window, ...)` — даёт `AsyncWindowContext`, но требует window handle
- `Entity::update_in` — требует `Window` доступ

**Решение**: использовать `cx.spawn` (даёт `AsyncApp`), предварительно извлекая Task через `project.update(cx, ...)` в синхронном контексте.

### Decision 2: Группировка символов

Используем `BTreeMap<String, Vec<SymbolEntry>>` с ключом — имя категории из `kind_name(kind)`. Порядок групп — алфавитный на первой итерации.

### Decision 3: Рендеринг

GPUI `Render` trait возвращает `impl IntoElement`. Первая итерация: список групп с заголовками, без сворачивания. Каждый символ — `Label`.

## Risks / Trade-offs

| Риск | Mitigation |
|------|-----------|
| `project.symbols("")` возвращает пустой список | Показывать "No symbols" placeholder |
| GPUI API меняется между версиями | Привязаться к конкретному коммиту Zed |
| Большое количество символов тормозит рендер | Виртуализация в будущем |

## Open Questions

- Какой тип контекста (`AsyncApp` vs `AsyncWindowContext`) нужен `this.update()` внутри `cx.spawn`?
- Поддерживает ли `bsl-lsp-server` `workspace/symbol` с пустым query?
