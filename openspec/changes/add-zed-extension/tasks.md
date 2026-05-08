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
- [x] 3.1 Создать `zed-extension/Cargo.toml` с зависимостью `zed_extension_api` и целью `wasm32-wasip2`
- [x] 3.2 Реализовать `zed-extension/src/lib.rs`: структура `BslExtension`, impl `Extension`, макрос `register_extension!`
- [x] 3.3 Реализовать `language_server_command()` — запуск `bsl-lsp-server` из bundled бинарника
- [x] 3.4 Реализовать `language_server_initialization_options()` — проксирование настроек в LSP (default impl достаточен; настройки передаются в LSP через env)

## 4. Бандлинг LSP-бинарника
- [x] 4.1 Build/smoke path материализует `bsl-lsp-server` (release) в `zed-extension/`
- [x] 4.2 Wasm-загрузчик embed-ит `bsl-lsp-server`, разворачивает его в Zed extension work dir и возвращает extension-relative `./bsl-lsp-server`

## 5. Верификация
- [x] 5.1 Установить расширение как dev-extension в Zed (`zed: install dev extension`)
- [x] 5.2 Проверить подсветку синтаксиса на примерах `.bsl` файлов
- [x] 5.3 Проверить автодополнение (completion) через LSP
- [x] 5.4 Проверить hover (информация о типе)
- [x] 5.5 Проверить диагностику (ошибки типов)
- [x] 5.6 Проверить inlay hints (подсказки типов)
- [x] 5.7 Проверить document symbols (outline)
- [x] 5.8 Проверить didChange/didSave → перестройку индекса типов
- [x] 5.9 Зафиксировать результат smoke-проверки в `zed-extension/README.md`

## 6. Согласование
- [x] 6.1 `openspec validate add-zed-extension --strict --no-interactive`
- [x] 6.2 Finish-to-100 review закрыл обязательные разрывы review-vs-plan

## Evidence 2026-05-08
- Live Zed WSL smoke: dev-extension source `/mnt/e/Projects/bsl-gradual-types/zed-extension` uploaded to `/home/egor/.local/share/zed/remote_extensions/bsl-gradual-types`; LSP started from `/home/egor/.local/share/zed/remote_extensions/work/bsl-gradual-types/bsl-lsp-server`; `bsl/snapshotStatus` reached `state=ready` for `examples/test_lsp.bsl`.
- Syntax smoke: `tree-sitter parse --grammar-path third_party/tree-sitter-bsl --quiet --stat examples/test_lsp.bsl`; `tree-sitter query --grammar-path third_party/tree-sitter-bsl zed-extension/languages/bsl/highlights.scm examples/test_lsp.bsl --captures`.
- LSP feature smoke: targeted `bsl-lsp-server` tests passed for completion/didChange, hover, diagnostics, inlay hints, document symbols, and didSave current-revision rebuild.
- Residual risk closed: backend timing test `p6_did_save_fastlane_uses_ready_parse_snapshot_when_shadow_is_missing` now passes after save-fastlane was hardened to prefer same-version ready parse snapshots and complete deferred syntax-error assembly before publishing.
