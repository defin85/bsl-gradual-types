# VSCode Extension Tests

Полный набор тестов для BSL Gradual Types VSCode Extension.

## Структура тестов

### 1. Extension Activation Tests (`extension.test.ts`)
- ✅ Проверка наличия расширения
- ✅ Активация расширения
- ✅ Регистрация всех команд (21 команда)
- ✅ Parser utilities (parseMethodCall, extractTypeName)
- ✅ Configuration tests

**Покрытие:** Extension lifecycle, Commands registration, Parser utils

### 2. LSP Integration Tests (`lsp.test.ts`)
- ✅ Защита от дублирования команд
- ✅ Регистрация команд до запуска LSP
- ✅ Обработка duplicate command registration
- ✅ LSP сервер не регистрирует extension команды
- ✅ Перезапуск сервера без дублирования команд
- ✅ Обработка ошибок инициализации LSP
- ✅ Изоляция команд расширения от LSP
- ✅ Защита от множественной активации

**Покрытие:** LSP lifecycle, Command conflicts prevention, Multiple activation handling

### 3. Commands Tests (`commands.test.ts`)
- ✅ Существование всех команд
- ✅ Refresh команды (4 команды)
- ✅ Обработка отсутствующей конфигурации
- ✅ Валидация формата версии платформы
- ✅ Performance tests (activation < 5s, commands < 1s)
- ✅ TreeDataProvider регистрация
- ✅ Configuration management (8 настроек)
- ✅ Изменение конфигурации

**Покрытие:** Commands execution, Error handling, Performance, Configuration

### 4. Integration Tests (`integration.test.ts`)
- ✅ Полный жизненный цикл расширения
- ✅ Работа с BSL файлами
- ✅ Взаимодействие с Language Server
- ✅ Обработка конфигурации 1С
- ✅ Механизм кеширования
- ✅ Diagnostics provider
- ✅ Output channel
- ✅ WebView команды
- ✅ Совместимость с VSCode >= 1.75.0
- ✅ Доступность всех API

**Покрытие:** End-to-end workflows, File handling, LSP communication, Compatibility

### 5. LSP Custom Requests Tests (`customRequests.test.ts`) ✨ NEW
- ✅ `queryType()` - замена CLI query_type
- ✅ `buildIndex()` - замена CLI build_unified_index
- ✅ `validateMethod()` - замена CLI для методов
- ✅ `checkTypeCompatibility()` - замена CLI check_type_compatibility
- ✅ `incrementalUpdate()` - замена CLI incremental_update
- ✅ `extractPlatformDocs()` - замена CLI extract_platform_docs
- ✅ Обработка ошибок LSP сервера
- ✅ Performance тесты (< 2s)
- ✅ Параллельные вызовы custom requests

**Покрытие:** Task 3 из Milestone 2.2 - все 6 custom requests

### 6. Provider Tests (`providers.test.ts`) ✨ NEW
- ✅ BslOverviewProvider регистрация и refresh
- ✅ BslDiagnosticsProvider регистрация и refresh
- ✅ HierarchicalTypeIndexProvider регистрация и refresh
- ✅ BslPlatformDocsProvider регистрация и CRUD операции
- ✅ BslActionsWebviewProvider регистрация
- ✅ Provider LSP интеграция
- ✅ Обработка перезапуска LSP сервера
- ✅ Performance провайдеров (< 2s)
- ✅ Параллельное обновление провайдеров (< 5s)

**Покрытие:** All Tree Data Providers, WebView providers, LSP integration

## Статистика

- **Всего тестовых файлов:** 6
- **Всего test cases:** 67+
- **Всего test suites:** 20+

### Покрытие по категориям

| Категория | Тесты | Статус |
|-----------|-------|--------|
| Extension Activation | 12 | ✅ |
| LSP Integration | 8 | ✅ |
| Commands | 14 | ✅ |
| Integration | 10 | ✅ |
| Custom Requests | 10 | ✅ NEW |
| Providers | 13 | ✅ NEW |
| **ИТОГО** | **67** | **✅** |

## Запуск тестов

### Обычный режим
```bash
npm test
```

### Фокусный smoke для Completion Timeline / Client Probe Feed
```bash
npm run compile:fast
BSL_TEST_GREP='Completion Probe (Schema|Recorder|Runtime|Store) Test Suite|Completion Timeline (Clipboard|Model|Webview Provider) Test Suite|Client Options Test Suite|getCompletionTimeline should work via executeCommand|getCompletionTimeline should fail-closed on Method not found' node ./out/test/runTest.js
```

Этот путь повторяет extension-side slice из `./scripts/run-intellisense-tests.sh smoke` и проверяет:
- bounded/redacted probe schema и eviction;
- runtime transport hook и selection observer для client probes;
- wiring default `LanguageClient` path;
- dual-view `Server Timeline` / `Client Probe Feed`;
- rendering/export для `Server Timeline` с `response.version=3` и bounded `server_edge_details`, плюс backward-compatible чтение legacy `version=2` payload;
- fail-closed/executeCommand поведение `bsl.getCompletionTimeline`.

### С coverage (цель: 80%)
```bash
npm run test:coverage
```

Coverage отчёт генерируется в `coverage/index.html`.

### Только линтинг
```bash
npm run lint
```

## Coverage конфигурация

Файл `.c8rc.json`:
- **Lines:** 80%
- **Functions:** 80%
- **Branches:** 70%
- **Statements:** 80%

## Тестируемые компоненты

✅ Extension activation lifecycle
✅ LSP Client initialization
✅ Command registration (21 команда)
✅ LSP Custom Requests (6 requests) — Task 3 Milestone 2.2
✅ Tree Data Providers (5 providers)
✅ Configuration management
✅ Parser utilities
✅ Error handling
✅ Performance constraints
✅ Compatibility with VSCode API

## CI/CD Integration

Тесты готовы для интеграции в CI pipeline:
```yaml
# .github/workflows/test.yml
- name: Run tests
  run: npm test

- name: Check coverage
  run: npm run test:coverage
```

## Milestone 2.2: Task 4 Status ✅

**Цель:** Написать тесты с покрытием 80%

**Выполнено:**
- ✅ 67+ test cases
- ✅ 20+ test suites
- ✅ Покрытие LSP Custom Requests (6/6)
- ✅ Покрытие Providers (5/5)
- ✅ Coverage reporting настроен (c8)
- ✅ Performance constraints проверены
- ✅ Error handling протестирован

**Результат:** Task 4 завершён на 100%
