## Context

Панель Symbol Browser добавляется в ядро Zed через форк `defin85/zed`. Основная техническая сложность — GPUI async-паттерны для вызова `project.symbols()` из панели и честное состояние UI, когда LSP ещё не готов, вернул пустой список или вернул ошибку.

## Goals / Non-Goals

- **Goals**: статическая панель с группировкой символов по `SymbolKind`, запрос через LSP `workspace/symbol`
- **Non-Goals**: интерактив (клик, сворачивание), поиск, BSL-specific группировка

## Decisions

### Decision 1: Паттерн вызова `project.symbols()` из панели

**Проблема**: `project.symbols("", cx)` возвращает `Task<Result<Vec<Symbol>>>`. Чтобы получить результат, нужно:
1. Вызвать `project.update(cx, |project, cx| project.symbols("", cx))` — возвращает `Result<Task<...>>` или `Task<...>`
2. Если возвращается `Task`, сохранить её и позже `await`
3. После `await` — обновить состояние панели через `cx.notify()`

**Рабочий паттерн в текущем форке**:
```rust
fn fetch_symbols(
    &mut self,
    project: Entity<Project>,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    let symbols_task = project.update(cx, |project, cx| project.symbols("", cx));

    self._fetch_task = cx.spawn_in(window, async move |this, cx| {
        let symbols = symbols_task.await;

        this.update_in(cx, |this, _window, cx| {
            match symbols {
                Ok(symbols) => {
                    this.groups = group_symbols(symbols);
                    this.status = SymbolBrowserStatus::Loaded;
                }
                Err(error) => {
                    log::warn!("failed to fetch Symbol Browser workspace symbols: {error:?}");
                    this.groups.clear();
                    this.status = SymbolBrowserStatus::Error;
                }
            }
            cx.notify();
        }).log_err();
    });
}
```

**Альтернативы**:
- `cx.spawn` + `Entity::update` — не подходит для текущей реализации панели, потому что в актуальном Zed API безопасный update панели идёт через window-aware context.
- `cx.spawn_in(window, ...)` + `Entity::update_in` — выбранный вариант; требует `Window`, поэтому `fetch_symbols()` вызывается из `new()`/`set_active()` там, где window доступен.

**Решение**: использовать `cx.spawn_in(window, ...)` и `Entity::update_in`. Ошибки нельзя превращать в вечный пустой success: для acceptance нужно логировать ошибку и переводить панель в error state.

### Decision 2: Группировка символов

Используем `BTreeMap<String, Vec<SymbolEntry>>` с ключом — имя категории из `kind_name(kind)`. Порядок групп — алфавитный на первой итерации. Это generic grouping по LSP `SymbolKind`; BSL-specific дерево метаданных остаётся non-goal.

### Decision 3: Рендеринг

GPUI `Render` trait возвращает `impl IntoElement`. Первая итерация: список групп с заголовками, без сворачивания. Каждый символ — `Label`.

Acceptance rendering states:
- `Loading symbols...` только пока идёт активный fetch.
- `No symbols available` после successful loaded state с пустым списком.
- `Symbols unavailable` или аналогичный error placeholder после ошибки `workspace/symbol`, с логированием причины.
- Непустой successful result отображается группами и символами.

### Decision 4: Refresh behavior

Панель не должна зависеть только от initial fetch в `new()`, потому что при старте Zed LSP может ещё не быть готов. Обязательный минимум: повторный `fetch_symbols()` при открытии/активации панели (`set_active(true)`), чтобы пользователь мог получить символы после готовности language server без перезапуска Zed.

### Decision 5: Error semantics for `workspace/symbol`

Zed `Project::symbols` должен различать три случая:
- нет активного language server с `workspace/symbol` support -> successful empty result, панель показывает `No symbols available`;
- language server вернул successful empty response -> successful empty result, панель показывает `No symbols available`;
- все фактически отправленные `workspace/symbol` запросы завершились ошибкой -> error result, панель показывает `Symbols unavailable` и логирует причину.

Это закрывает UI-контракт без BSL-specific API: current change остаётся generic-only. BSL-specific `bsl/getAllTypes` режим вынесен в follow-up задачу `bsl-gradual-types-zkxt`.

## Risks / Trade-offs

| Риск | Mitigation |
|------|-----------|
| `project.symbols("")` возвращает пустой список | Показывать `No symbols available` после loaded state, не вечный loading |
| GPUI API меняется между версиями | Привязаться к конкретному коммиту Zed |
| Большое количество символов тормозит рендер | Виртуализация в будущем |
| BSL LSP возвращает пустой список на empty query | Исправить `bsl-lsp-server` contract: empty query возвращает все доступные workspace symbols |

## Open Questions

- Resolved: текущий форк использует `AsyncWindowContext` через `cx.spawn_in(window, ...)` и обновляет entity через `update_in`.
- Resolved: `bsl-lsp-server` empty query теперь возвращает доступные open-document workspace symbols для generic Symbol Browser.

## Verification Evidence

- `cargo test -p bsl-backend p12_workspace_symbol_searches_open_documents` passed after adding an empty-query regression.
- `cargo test -p project workspace_symbols` passed after adding coverage that `workspace/symbol` request errors are not converted to empty success, while `None` responses remain empty success.
- `cargo test -p symbol_browser` passed with 5 focused tests for grouping, empty/error state helpers, and settings/defaults.
- `cargo check -p zed` passed.
- `cargo build --release -p zed` passed in 19m 40s.
- Direct JSON-RPC smoke against rebuilt `target/release/bsl-lsp-server` returned 4 symbols for `workspace/symbol` with empty query on `examples/test_lsp.bsl`: `ТестHover`, `ТестАнглийскихИмен`, `ТестМассива`, `ТестТаблицыЗначений`.
- X11 temp smoke with installed BSL dev-extension and repo-root worktree opened `/home/egor/code/bsl-gradual-types` plus `examples/test_lsp.bsl`, toggled Symbol Browser through a temporary `symbol_browser::ToggleFocus` keybinding, and captured the right-dock non-empty BSL list: `1 groups`, `Functions (4)`.
- X11 temp smoke against the Rust `/home/egor/code/zed` worktree opened `crates/symbol_browser/src/symbol_browser.rs`, selected `symbol browser: toggle` through Command Palette after `rust-analyzer` startup, and captured the right-dock non-empty Rust list: `5 groups`, including `Enums (21)`, `Interfaces (2)`, `Other (5)`, and `Structs (97)`.

## Residual Acceptance Gaps

- The earlier single-file X11 smoke is not an acceptance contour for this workspace panel: it opened only `examples/test_lsp.bsl` and produced an empty Zed `Project::symbols` result even though direct backend JSON-RPC returned symbols. Acceptance evidence now uses a repo-root worktree with the BSL dev-extension installed.
- BSL-specific `bsl/getAllTypes` grouping is intentionally not part of this change and is tracked separately as `bsl-gradual-types-zkxt`.
- Contributor review remains unchecked.
