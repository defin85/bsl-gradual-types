# План рефакторинга extension.ts

## Текущая проблема
- Файл extension.ts содержит 2622 строки кода
- Сложно поддерживать и находить нужные функции
- Смешана логика разных компонентов

## Предлагаемая структура модулей

```
src/
├── extension.ts (основной файл активации, ~200 строк)
├── config/
│   └── configHelper.ts (уже есть)
├── lsp/
│   ├── client.ts (LSP клиент и управление)
│   └── statusBar.ts (статус бар и прогресс)
├── commands/
│   ├── index.ts (регистрация всех команд)
│   ├── analysis.ts (команды анализа)
│   ├── indexing.ts (команды индексации)
│   └── validation.ts (команды валидации)
├── providers/
│   ├── overview.ts (BslOverviewProvider)
│   ├── diagnostics.ts (BslDiagnosticsProvider)
│   ├── typeIndex.ts (BslTypeIndexProvider)
│   ├── platformDocs.ts (BslPlatformDocsProvider)
│   └── actions.ts (BslActionsWebviewProvider)
├── webviews/
│   ├── metrics.ts (отображение метрик)
│   ├── typeInfo.ts (информация о типах)
│   └── templates.ts (HTML шаблоны)
└── utils/
    ├── binaryPath.ts (работа с путями к бинарникам)
    ├── executor.ts (выполнение команд)
    └── parser.ts (парсинг методов и типов)
```

## Распределение кода по модулям

### 1. **extension.ts** (главный файл)
- Функция `activate()` - упрощенная
- Функция `deactivate()`
- Импорты и инициализация модулей

### 2. **lsp/client.ts** (~300 строк)
- `startLanguageClient()`
- Конфигурация LSP клиента
- Управление жизненным циклом клиента

### 3. **lsp/statusBar.ts** (~150 строк)
- `registerStatusBar()`
- `updateStatusBar()`
- Управление прогрессом индексации

### 4. **commands/index.ts** (~100 строк)
- `registerCommands()` - главная функция
- Импорт и регистрация команд из других модулей

### 5. **commands/analysis.ts** (~200 строк)
- `analyzeFile`
- `analyzeWorkspace`
- `generateReports`
- `showMetrics`

### 6. **commands/indexing.ts** (~400 строк)
- `buildIndex`
- `incrementalUpdate`
- `showIndexStats`
- Вся логика работы с индексом

### 7. **commands/validation.ts** (~200 строк)
- `validateMethodCall`
- `checkTypeCompatibility`
- `searchType`
- `exploreType`

### 8. **providers/** (~500 строк всего)
- Каждый провайдер в отдельном файле
- Классы TreeItem остаются с провайдерами

### 9. **webviews/** (~600 строк всего)
- Разделение по типам webview
- Общие шаблоны в `templates.ts`

### 10. **utils/** (~200 строк всего)
- `getBinaryPath()`
- `executeBslCommand()`
- `parseMethodCall()`

## Преимущества рефакторинга

1. **Модульность** - каждый компонент в своем файле
2. **Читаемость** - легко найти нужную функцию
3. **Тестируемость** - можно тестировать модули отдельно
4. **Поддержка** - проще добавлять новые функции
5. **Производительность** - быстрее загрузка и компиляция

## Порядок рефакторинга

1. Создать структуру папок
2. Создать модули с экспортами
3. Перенести код по модулям
4. Обновить импорты в extension.ts
5. Протестировать работоспособность

## Риски

- Возможны проблемы с циклическими зависимостями
- Нужно аккуратно работать с глобальными переменными
- Требуется тщательное тестирование после рефакторинга