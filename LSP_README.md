# BSL LSP Server

Минимальный Language Server Protocol сервер для BSL с поддержкой градуальной типизации.

## Возможности

- ✅ **Автодополнение** для платформенных типов (Справочники, Документы, Перечисления)
- ✅ **Hover** - показ информации о типах при наведении
- ✅ **Загрузка конфигурации** из XML файлов 1С
- ✅ **Кэширование** разрешённых типов

## Запуск

```bash
cargo run --bin lsp-server
```

## Интеграция с редакторами

### VS Code

1. Установите расширение для BSL (например, [BSL Language](https://marketplace.visualstudio.com/items?itemName=1c-syntax.language-1c-bsl))
2. В настройках расширения укажите путь к LSP серверу:
```json
{
    "bsl.languageServer.path": "path/to/bsl-gradual-types/target/debug/lsp-server"
}
```

### Neovim

Добавьте в конфигурацию:
```lua
local lspconfig = require('lspconfig')
lspconfig.bsl_lsp = {
    cmd = { 'path/to/lsp-server' },
    filetypes = { 'bsl', 'os' },
    root_dir = lspconfig.util.root_pattern('.git', 'src/cf'),
}
```

## Поддерживаемые LSP методы

- `initialize` - инициализация сервера
- `textDocument/didOpen` - открытие документа
- `textDocument/didChange` - изменение документа
- `textDocument/didClose` - закрытие документа
- `textDocument/completion` - автодополнение
- `textDocument/hover` - информация при наведении

## Автодополнение

Сервер предоставляет автодополнение для:

- **Глобальные объекты**: `Справочники`, `Документы`, `Перечисления`, `РегистрыСведений`
- **Конфигурационные объекты**: После точки показываются доступные объекты из конфигурации
- **Примеры** (если конфигурация не загружена): `Контрагенты`, `Номенклатура`, `Организации`

## Конфигурация

Сервер автоматически ищет конфигурацию 1С по пути `{workspace_root}/src/cf/`.

## Логирование

Для отладки можно включить подробное логирование:
```bash
RUST_LOG=debug cargo run --bin lsp-server
```

## Архитектура

```
LSP Client (VS Code, Neovim, etc.)
    ↓ JSON-RPC over stdio
BSL Language Server
    ├── Document Manager (хранение открытых файлов)
    ├── Platform Type Resolver (разрешение типов)
    └── Config Parser (парсинг XML конфигурации)
```

## Планы развития

- [ ] Диагностика типов (подсветка ошибок)
- [ ] Go to definition
- [ ] Find references
- [ ] Rename
- [ ] Code actions (quick fixes)
- [ ] Поддержка методов объектов
- [ ] Интеграция с runtime контрактами