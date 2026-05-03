## 1. Структура расширения
- [x] 1.1 Создать `zed-extension/extension.toml` с метаданными расширения (id, name, version, authors, repository)
- [x] 1.2 Зарегистрировать Tree-sitter грамматику BSL в `[grammars]` секции `extension.toml`
- [x] 1.3 Зарегистрировать Language Server в `[language_servers]` секции `extension.toml`

## 2. Языковая поддержка BSL
- [x] 2.1 Создать `zed-extension/languages/bsl/config.toml` с метаданными языка (name, grammar, path_suffixes, line_comments)
- [x] 2.2 Создать `zed-extension/languages/bsl/highlights.scm` для подсветки синтаксиса
- [x] 2.3 Создать `zed-extension/languages/bsl/brackets.scm` для парных скобок
- [x] 2.4 Создать `zed-extension/languages/bsl/indents.scm` для авто-отступов
- [x] 2.5 Создать `zed-extension/languages/bsl/outline.scm` для структуры документа

## 3. Wasm-загрузчик
- [x] 3.1 Создать `zed-extension/Cargo.toml` с зависимостью `zed_extension_api` и целью `wasm32-wasip1`
- [x] 3.2 Реализовать `zed-extension/src/lib.rs`: структура `BslExtension`, impl `Extension`, макрос `register_extension!`
- [x] 3.3 Реализовать `language_server_command()` — запуск `bsl-lsp-server` из bundled бинарника
- [x] 3.4 Реализовать `language_server_initialization_options()` — проксирование настроек в LSP (default impl достаточен; настройки передаются в LSP через env)

## 4. Бандлинг LSP-бинарника
- [x] 4.1 Скопировать `bsl-lsp-server` (release) в `zed-extension/`
- [x] 4.2 Бинарник доступен для `language_server_command()` (fallback-путь через `current_dir()`)

## 5. Верификация
- [ ] 5.1 Установить расширение как dev-extension в Zed (`zed: install dev extension`)
- [ ] 5.2 Проверить подсветку синтаксиса на примерах `.bsl` файлов
- [ ] 5.3 Проверить автодополнение (completion) через LSP
- [ ] 5.4 Проверить hover (информация о типе)
- [ ] 5.5 Проверить диагностику (ошибки типов)
- [ ] 5.6 Проверить inlay hints (подсказки типов)
- [ ] 5.7 Проверить document symbols (outline)
- [ ] 5.8 Проверить didChange/didSave → перестройку индекса типов
- [x] 5.9 Зафиксировать результат smoke-проверки в `zed-extension/README.md`

## 6. Согласование
- [x] 6.1 `openspec validate add-zed-extension --strict --no-interactive`
- [ ] 6.2 Review proposal с ключевыми контрибьюторами
