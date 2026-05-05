## 1. Исследование GPUI async-паттернов
- [ ] 1.1 Найти пример панели, вызывающей `project.symbols("", cx)` из async-контекста через `cx.spawn`
- [ ] 1.2 Понять правильную сигнатуру: `Entity::update` vs `Entity::update_in`, `Context` vs `AsyncWindowContext`
- [ ] 1.3 Задокументировать рабочий паттерн в `design.md`

## 2. Крейт symbol_browser
- [ ] 2.1 Создать `crates/symbol_browser/` с Cargo.toml
- [ ] 2.2 Реализовать `SymbolBrowserPanel` с `Panel`, `Focusable`, `Render` trait'ами
- [ ] 2.3 Добавить `SymbolBrowserSettings` через `RegisterSetting`
- [ ] 2.4 Реализовать `fetch_symbols()` — запрос `workspace/symbol` через `project.symbols()`
- [ ] 2.5 Реализовать `group_symbols()` — группировка по `SymbolKind`
- [ ] 2.6 Реализовать `render()` — отображение групп и символов

## 3. Регистрация в Zed
- [ ] 3.1 Добавить `symbol_browser` в workspace members и зависимости
- [ ] 3.2 Добавить `SymbolBrowserSettingsContent` в `settings_content`
- [ ] 3.3 Добавить `symbol_browser` в `assets/settings/default.json`
- [ ] 3.4 Добавить `add_panel_when_ready(symbol_browser_panel, ...)` в `crates/zed/src/zed.rs`
- [ ] 3.5 Вызвать `symbol_browser::init(cx)` в `crates/zed/src/main.rs`

## 4. Верификация
- [ ] 4.1 `cargo check -p symbol_browser` без ошибок
- [ ] 4.2 `cargo build --release -p zed` без ошибок
- [ ] 4.3 Запустить Zed → открыть панель → проверить список символов
- [ ] 4.4 Проверить на BSL-проекте (группировка по SymbolKind)
- [ ] 4.5 Проверить на Rust-проекте

## 5. Согласование
- [ ] 5.1 `openspec validate add-zed-symbol-browser-panel --strict --no-interactive`
- [ ] 5.2 Review с ключевыми контрибьюторами
