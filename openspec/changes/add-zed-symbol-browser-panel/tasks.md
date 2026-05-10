## 1. Исследование GPUI async-паттернов
- [x] 1.1 Найти рабочий паттерн панели, вызывающей `project.symbols("", cx)` из async-контекста
- [x] 1.2 Понять правильную сигнатуру: `Entity::update` vs `Entity::update_in`, `Context` vs `AsyncWindowContext`
- [x] 1.3 Задокументировать рабочий паттерн в `design.md`

## 2. Крейт symbol_browser
- [x] 2.1 Создать `crates/symbol_browser/` с Cargo.toml
- [x] 2.2 Реализовать `SymbolBrowserPanel` с `Panel`, `Focusable`, `Render` trait'ами
- [x] 2.3 Добавить `SymbolBrowserSettings` через `RegisterSetting`
- [x] 2.4 Реализовать `fetch_symbols()` — запрос `workspace/symbol` через `project.symbols()`
- [x] 2.5 Реализовать `group_symbols()` — группировка по `SymbolKind`
- [x] 2.6 Довести `render()` до acceptance: отдельные состояния loading / loaded-empty / error, без вечного `Loading symbols...` при пустом результате

## 3. Регистрация в Zed
- [x] 3.1 Добавить `symbol_browser` в workspace members и зависимости
- [x] 3.2 Добавить `SymbolBrowserSettingsContent` в `settings_content`
- [x] 3.3 Добавить `symbol_browser` в `assets/settings/default.json`
- [x] 3.4 Добавить `add_panel_when_ready(symbol_browser_panel, ...)` в `crates/zed/src/zed.rs`
- [x] 3.5 Вызвать `symbol_browser::init(cx)` в `crates/zed/src/main.rs`

## 4. Верификация
- [x] 4.1 `cargo check -p symbol_browser` без ошибок
- [x] 4.2 `cargo build --release -p zed` без ошибок
- [x] 4.3 Запустить Zed → открыть панель → проверить список символов
- [x] 4.4 Проверить на BSL-проекте (группировка по SymbolKind)
- [x] 4.5 Проверить на Rust-проекте

## 5. Согласование
- [x] 5.1 `openspec validate add-zed-symbol-browser-panel --strict --no-interactive`
- [ ] 5.2 Review с ключевыми контрибьюторами

## 6. Остаточные обязательные gap'ы после factual review
- [x] 6.1 Закрыть контракт `workspace/symbol` для BSL: пустой query должен возвращать доступные workspace symbols, потому что Zed panel вызывает `project.symbols("", cx)`
- [x] 6.2 Добавить refresh при открытии/фокусе панели, чтобы не зависеть только от первого запуска до готовности LSP
- [x] 6.3 Добавить focused tests для группировки, UI state helpers и настроек/дефолтов
- [x] 6.4 Прогнать `cargo check -p zed` и focused tests после правок в форке
- [x] 6.5 Прогнать Zed fork integration/live smoke с установленным BSL dev-extension и repo-root worktree: панель показывает `Functions (4)` для `examples/test_lsp.bsl`; Rust live-smoke показывает непустой список Symbol Browser (`5 groups`)
- [x] 6.6 Закрыть review blocker: ошибки фактического `workspace/symbol` запроса не должны превращаться в пустой success; панель должна получать `Err` и показывать `Symbols unavailable`
- [x] 6.7 Убрать scope drift: текущий change остаётся generic `workspace/symbol`, а BSL-specific `bsl/getAllTypes` режим вынесен в отдельную follow-up задачу `bsl-gradual-types-zkxt`
