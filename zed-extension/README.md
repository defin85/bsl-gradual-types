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

```bash
rustup target add wasm32-wasip1
cargo build --manifest-path zed-extension/Cargo.toml --target wasm32-wasip1 --release
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

## Требования

- Zed ≥ 0.150 (с поддержкой расширений)
- Rust + rustup (для сборки)
- bsl-lsp-server собран под linux-x86_64

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
