# LSP Client

> Language Server Protocol клиент для VSCode Extension

## Обзор

LSP клиент обеспечивает интеграцию между VSCode и BSL Language Server (Rust backend). Реализует полный lifecycle управления сервером, health checks, progress reporting и custom commands.

**Основная задача:** Связать VSCode UI с backend services (hover, completion, diagnostics, etc.)

## Структура модулей

```
lsp/
├── client/                    # Модульная архитектура LSP клиента
│   ├── index.ts               # Публичный API клиента
│   ├── lifecycle.ts           # Управление жизненным циклом
│   ├── server-options.ts      # Конфигурация сервера
│   ├── client-options.ts      # Конфигурация клиента
│   ├── progress-handler.ts    # Progress notifications
│   └── health-check.ts        # Health check мониторинг
│
├── contextProvider.ts         # Контекст текущего файла
├── statsProvider.ts           # Статистика сервера
├── customRequests.ts          # Custom LSP запросы
├── enhanced-client.ts         # Расширенный клиент (legacy)
├── typeVisualization.ts       # Визуализация типов
├── progress.ts                # Progress reporting (legacy)
├── serverStatus.ts            # Статус сервера (legacy)
├── logger.ts                  # Логирование
└── index.ts                   # Главный entry point
```

## Ключевые компоненты

### Client Module (модульная архитектура)

**Директория:** `client/`

После рефакторинга (Task #8) LSP клиент разделён на фокусированные модули:

#### index.ts

Публичный API модуля. Экспортирует функции для создания и управления клиентом.

```typescript
export function createLanguageClient(
    context: ExtensionContext,
    outputChannel: OutputChannel
): LanguageClient;

export function startClient(client: LanguageClient): Promise<void>;
export function stopClient(client: LanguageClient): Promise<void>;
export function restartClient(client: LanguageClient): Promise<void>;
```

#### lifecycle.ts

Управление жизненным циклом LSP клиента.

**Функции:**
- `startClient()` - запуск клиента и регистрация handlers
- `stopClient()` - graceful shutdown
- `restartClient()` - перезапуск с сохранением state

**Особенности:**
- Регистрация progress handlers
- Регистрация health check
- Error handling и recovery

#### server-options.ts

Конфигурация запуска Language Server (backend).

```typescript
function getServerOptions(context: ExtensionContext): ServerOptions {
    return {
        run: { command: serverPath, args: [] },
        debug: { command: serverPath, args: [] }
    };
}
```

**Настройки:**
- Путь к bsl-lsp-server бинарнику
- Аргументы запуска
- Debug конфигурация

#### client-options.ts

Конфигурация LSP клиента.

```typescript
function getClientOptions(outputChannel: OutputChannel): LanguageClientOptions {
    return {
        documentSelector: [{ scheme: 'file', language: 'bsl' }],
        synchronize: { fileEvents: workspace.createFileSystemWatcher('**/*.bsl') },
        outputChannel: outputChannel
    };
}
```

**Настройки:**
- Document selector (*.bsl файлы)
- File watchers
- Output channel для логов

#### progress-handler.ts

Обработка progress notifications от сервера.

**Функции:**
- `registerProgressHandlers()` - регистрация handlers для $/progress
- Отображение progress в VSCode UI (progress bars, notifications)

**Пример:**

```typescript
// Сервер отправляет:
// $/progress { token: "parse", message: "Parsing files...", percentage: 50 }

// Клиент отображает в UI
```

#### health-check.ts

Мониторинг здоровья сервера.

**Функции:**
- Периодический health check запрос
- Автоматический restart при падении
- Обновление status bar

### ContextProvider

**Файл:** `contextProvider.ts`

Предоставляет контекст текущего документа для сервера.

**Возможности:**
- Отслеживание активного редактора
- Отправка контекста при изменении файла
- Поддержка workspace context

**Пример использования:**

```typescript
const provider = new ContextProvider(client);
provider.activate(context);

// При изменении активного файла:
// → Отправляет workspace/didChangeConfiguration с текущим файлом
```

### StatsProvider

**Файл:** `statsProvider.ts`

Отображение статистики сервера в status bar.

**Метрики:**
- Количество загруженных типов
- Количество распарсенных файлов
- Время последнего анализа
- Статус сервера (running/stopped)

**UI элементы:**
- Status bar item с иконкой
- TreeView с детальной статистикой
- Команды для обновления статистики

### Custom Requests

**Файл:** `customRequests.ts`

Custom LSP команды и запросы.

**Доступные запросы:**

```typescript
// Поиск типов
client.sendRequest('bsl/searchTypes', { query: 'Массив' });

// Информация о типе
client.sendRequest('bsl/queryType', { typeName: 'ТаблицаЗначений' });

// Статистика сервера
client.sendRequest('bsl/stats', {});

// Semantic diagnostics
client.sendRequest('bsl/semanticDiagnostics', { uri: fileUri });
```

**Использование:**

```typescript
import { searchTypes, queryType, getStats } from './customRequests';

const types = await searchTypes(client, 'Массив');
const typeInfo = await queryType(client, 'ТаблицаЗначений');
const stats = await getStats(client);
```

## Конфигурация

### Настройки Extension

**Файл:** `package.json` (корень extension)

```json
{
  "contributes": {
    "configuration": {
      "title": "BSL Language Server",
      "properties": {
        "bslLanguageServer.serverPath": {
          "type": "string",
          "default": "",
          "description": "Путь к bsl-lsp-server бинарнику"
        },
        "bslLanguageServer.trace.server": {
          "type": "string",
          "enum": ["off", "messages", "verbose"],
          "default": "off",
          "description": "Уровень логирования LSP коммуникации"
        }
      }
    }
  }
}
```

### Получение настроек

```typescript
import { workspace } from 'vscode';

const config = workspace.getConfiguration('bslLanguageServer');
const serverPath = config.get<string>('serverPath') || getDefaultServerPath();
const traceLevel = config.get<string>('trace.server', 'off');
```

## Development

### Локальный запуск

**1. Собрать Language Server:**

```bash
cd /home/egor/code/bsl-gradual-types
cargo build --release --bin bsl-lsp-server
```

**2. Скопировать в extension:**

```bash
cp target/release/bsl-lsp-server vscode-extension/bin/
```

**3. Запустить extension в debug режиме:**

```
F5 в VSCode → Extension Development Host
```

### Отладка LSP коммуникации

**Включить trace:**

```json
// settings.json
{
  "bslLanguageServer.trace.server": "verbose"
}
```

**Просмотр логов:**

```
Output Panel → BSL Language Server
```

**Типичные логи:**

```
[Trace - 10:30:15] Sending request 'textDocument/hover'
[Trace - 10:30:15] Received response 'textDocument/hover' in 15ms
```

### Тестирование custom requests

**Пример теста:**

```typescript
import { searchTypes } from './customRequests';

// В test suite
test('searchTypes returns platform types', async () => {
    const client = await createTestClient();
    const results = await searchTypes(client, 'Массив');

    expect(results).toContainEqual(
        expect.objectContaining({ name: 'Массив' })
    );
});
```

## Интеграция с Backend

### Поток данных

```
VSCode UI (user action)
      ↓
LSP Client (vscode-extension/src/lsp/)
      ↓
LSP Protocol (JSON-RPC over stdio)
      ↓
LSP Server (backend/src/bin/lsp_server/)
      ↓
v2 analysis host (salsa)
      ↓
IR + deps snapshot
      ↓
TypeRepository (shared/src/domain/)
      ↓
Response в обратном порядке
      ↓
VSCode UI (результат отображается)
```

### Пример: Hover Request

**Клиент:**

```typescript
// LSP клиент автоматически отправляет при hover
// Реализовано в vscode-languageclient
```

**Протокол:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "textDocument/hover",
  "params": {
    "textDocument": { "uri": "file:///path/to/file.bsl" },
    "position": { "line": 10, "character": 5 }
  }
}
```

**Сервер обрабатывает:**

```rust
// backend/src/bin/lsp_server/handlers/hover.rs
pub fn handle_hover(params: HoverParams) -> Option<Hover> {
    // v2 hover entrypoint
}
```

**Ответ:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "contents": {
      "kind": "markdown",
      "value": "**ТаблицаЗначений**\n\nТип: PlatformType\n..."
    }
  }
}
```

## Команды Extension

**Регистрация команд:**

```typescript
// extension.ts
context.subscriptions.push(
    commands.registerCommand('bsl.restartServer', async () => {
        await restartClient(client);
    })
);
```

**Доступные команды:**
- `bsl.restartServer` - перезапуск LSP сервера
- `bsl.showStats` - показать статистику
- `bsl.searchTypes` - поиск типов
- `bsl.queryType` - информация о типе

## Troubleshooting

### Сервер не запускается

**Проверка:**

1. Бинарник существует:
   ```bash
   ls -la vscode-extension/bin/bsl-lsp-server
   ```

2. Права на выполнение:
   ```bash
   chmod +x vscode-extension/bin/bsl-lsp-server
   ```

3. Версия совместима:
   ```bash
   ./vscode-extension/bin/bsl-lsp-server --version
   ```

### Hover не работает

**Диагностика:**

1. Включить trace: `"bslLanguageServer.trace.server": "verbose"`
2. Проверить Output Panel на ошибки
3. Проверить что файл *.bsl открыт (не unsaved)
4. Проверить что позиция внутри identifier

### Custom request не возвращает результат

**Проверка:**

1. Сервер поддерживает запрос:
   ```bash
   # Проверить capabilities в Output Panel
   ```

2. Параметры корректны:
   ```typescript
   // Для searchTypes query обязателен
   client.sendRequest('bsl/searchTypes', { query: 'test' });
   ```

## Связанные документы

- [Backend README](../../../backend/src/README.md) - серверная архитектура
- [Application Layer README](../../../backend/src/application/README.md) - бизнес-логика
- [Web API Reference](../../../docs/api/web-api-reference.md) - альтернативный API

## Статус

**LSP Client статус:**
- ✅ Модульная архитектура (Task #8, рефакторинг client/)
- ✅ Lifecycle management (startup, shutdown, restart)
- ✅ Progress handlers (parsing, analysis)
- ✅ Health check monitoring
- ✅ Custom requests (searchTypes, queryType, stats)
- ✅ Context provider (активный файл)
- ✅ Stats provider (метрики сервера)
