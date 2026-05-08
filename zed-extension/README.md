# BSL Extension for Zed Editor

Поддержка языка 1С:Предприятие (BSL) в редакторе [Zed](https://zed.dev):
- Подсветка синтаксиса (Tree-sitter)
- Автодополнение с учётом типов
- Информация о типе (hover)
- Диагностика ошибок типов
- Inlay hints (подсказки типов)
- Переход к определению
- Структура документа (outline)

## Установка (dev-extension)

### 1. Собрать LSP-сервер

```bash
cd bsl-gradual-types
cargo build -p bsl-backend --bin bsl-lsp-server --release
cp target/release/bsl-lsp-server zed-extension/
```

### 2. Собрать Wasm-загрузчик

`bsl-lsp-server` должен уже лежать в `zed-extension/`: Wasm-загрузчик встраивает его через
`include_bytes!`, а при запуске Zed материализует бинарник в extension work dir и делает его
исполняемым.

```bash
rustup target add wasm32-wasip2
cargo build --manifest-path zed-extension/Cargo.toml --target wasm32-wasip2 --release
cp zed-extension/target/wasm32-wasip2/release/zed_bsl.wasm zed-extension/extension.wasm
```

### 3. Установить в Zed

1. Открыть Zed
2. `Cmd+Shift+P` → `zed: extensions`
3. В правом верхнем углу → `Install Dev Extension`
4. Выбрать директорию `bsl-gradual-types/zed-extension/`

### 4. Проверить

- Открыть любой `.bsl` файл
- Должна работать подсветка синтаксиса
- Должен запуститься LSP-сервер (проверить: `zed: open log`)
- В логе Zed ожидаемый путь LSP: `./bsl-lsp-server` или развернутый extension-owned путь, а не пользовательский `PATH`

## Требования

- Zed ≥ 0.150 (с поддержкой расширений)
- Rust + rustup (для сборки)
- `bsl-lsp-server` собран под linux-x86_64 до сборки `extension.wasm`
- `extension.wasm` и `bsl-lsp-server` являются generated dev-extension artifacts; `extension.wasm` embed-ит `bsl-lsp-server` и при запуске разворачивает его в extension work dir

## Структура

```
zed-extension/
├── extension.toml     # Манифест расширения
├── Cargo.toml         # Rust-крепйт для Wasm-загрузчика
├── src/
│   └── lib.rs         # Wasm-загрузчик: запуск bsl-lsp-server
├── bsl-lsp-server     # Бинарник LSP (собирается отдельно)
└── languages/
    └── bsl/
        ├── config.toml     # Метаданные языка
        ├── highlights.scm  # Подсветка синтаксиса
        ├── brackets.scm    # Парные скобки
        ├── indents.scm     # Авто-отступы
        └── outline.scm     # Структура документа
```

## Smoke-проверка

Минимальный локальный smoke без GUI:

```bash
cargo build -p bsl-backend --bin bsl-lsp-server --release
cp target/release/bsl-lsp-server zed-extension/
cargo test --manifest-path zed-extension/Cargo.toml --lib --locked
cargo check --manifest-path zed-extension/Cargo.toml --target wasm32-wasip2 --locked
cargo build --manifest-path zed-extension/Cargo.toml --target wasm32-wasip2 --release --locked
cp zed-extension/target/wasm32-wasip2/release/zed_bsl.wasm zed-extension/extension.wasm
zed-extension/bsl-lsp-server --help
```

Live smoke через Zed:

1. Установить dev-extension из `zed-extension/`.
2. Открыть `.bsl` файл в Zed.
3. Проверить в `zed: open log`, что `bsl` language server стартует и путь указывает на extension-owned `bsl-lsp-server`.
4. Проверить в файле подсветку, completion, hover, diagnostics, inlay hints, document symbols и обновление после `didChange`/`didSave`.
