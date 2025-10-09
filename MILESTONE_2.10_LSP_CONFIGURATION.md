# Milestone 2.10: LSP Configuration + Type Index Integration

**Дата начала:** TBD
**Статус:** 📋 Планирование
**Цель:** Полноценная интеграция Extension ↔ LSP через configuration и custom requests

---

## 📊 Контекст и проблемы

### Проблема 1: LSP не получает конфигурацию из Extension

**Текущая ситуация (после Milestone 2.9):**

```
VSCode Extension (TypeScript):
├── Настройки: platformDocsArchive = "C:/path/to/syntax_helper"
└── Запускает LSP Server через STDIO

LSP Server (Rust):
├── Стартует с пустыми настройками
├── Загружает только 4 примитивных типа (fallback)
└── ❌ НЕ получает platformDocsArchive из Extension!
```

**Результат:**
- ❌ TypeRepository пустой (4 типа вместо 3927)
- ❌ Hover показывает "Unknown type" для платформенных типов
- ❌ Методы и свойства не отображаются

**Причина:**
LSP не читает `initializationOptions` при запуске, не обрабатывает `workspace/didChangeConfiguration`.

---

### Проблема 2: Type Index UI не показывает типы

**Текущая ситуация:**

```
Extension UI:
├── Type Repository panel показывает заглушку
├── Закомментирован код загрузки JSONL (~4.5 MB)
└── ❌ НЕ запрашивает типы из LSP!

LSP Server:
├── TypeRepository содержит типы в памяти
└── ❌ НЕ предоставляет Custom Requests для UI!
```

**Результат:**
- ❌ Пользователь не видит доступные типы платформы
- ❌ Нет поиска по типам
- ❌ Нет категоризации (Справочники, Документы, и т.д.)

**Причина:**
Не реализованы LSP Custom Requests (`bsl/getAllTypes`, `bsl/searchTypes`).

---

## 🎯 Целевая архитектура

### Архитектура 1: LSP Configuration через initializationOptions

```typescript
// Extension передаёт настройки при запуске LSP
const initializationOptions = {
  platformDocsArchive: "C:/path/to/syntax_helper",
  configurationPath: "C:/path/to/Configuration.xml",
  platformVersion: "8.3.25"
};

client.start(initializationOptions);
```

```rust
// LSP читает настройки при initialize()
async fn initialize(&self, params: InitializeParams) -> JsonRpcResult<InitializeResult> {
    if let Some(options) = params.initialization_options {
        let config: LspConfig = serde_json::from_value(options)?;

        // Передаём в SystemCoordinator
        let syntax_path = config.platform_docs_archive.as_ref().map(Path::new);
        self.coordinator.start_with_paths(syntax_path, None).await?;
    }

    // Возвращаем capabilities...
}
```

**Преимущества:**
- ✅ LSP получает конфигурацию при старте
- ✅ TypeRepository загружает документацию синтаксис-помощника
- ✅ Hover работает с платформенными типами
- ✅ Методы и свойства отображаются корректно

---

### Архитектура 2: LSP Custom Requests для Type Index

```rust
// LSP предоставляет Custom Requests
client.on_request("bsl/getAllTypes", |_params| async {
    let types = type_service.get_all_types_as_dto().await?;
    Ok(types)
});

client.on_request("bsl/searchTypes", |params: SearchParams| async {
    let results = type_service.search_types(&params.query).await?;
    Ok(results)
});

client.on_request("bsl/getTypesByCategory", |params: CategoryParams| async {
    let types = type_service.get_types_by_category(&params.category).await?;
    Ok(types)
});
```

```typescript
// Extension запрашивает типы из LSP
const types = await client.sendRequest('bsl/getAllTypes', {});
typeIndexProvider.updateTypes(types);

const searchResults = await client.sendRequest('bsl/searchTypes', {
  query: 'Массив'
});
```

**Преимущества:**
- ✅ Единый источник данных (TypeRepository в LSP)
- ✅ UI всегда показывает актуальные типы
- ✅ Нет дублирования кеша (~4.5 MB экономии памяти)
- ✅ Автоматическая синхронизация при обновлении типов

---

### Архитектура 3: Прогресс парсинга документации

```rust
// LSP отправляет прогресс-нотификации
client.send_notification("bsl/parsingProgress", ParsingProgressParams {
    stage: "parsing_syntax_helper",
    current: 1234,
    total: 3927,
    message: "Парсинг контекстной справки..."
});
```

```typescript
// Extension показывает прогресс пользователю
client.onNotification('bsl/parsingProgress', (params) => {
    vscode.window.withProgress({
        location: vscode.ProgressLocation.Window,
        title: "BSL Analyzer"
    }, (progress) => {
        const percentage = (params.current / params.total) * 100;
        progress.report({
            increment: percentage,
            message: params.message
        });
    });
});
```

**Преимущества:**
- ✅ Пользователь видит что происходит при старте LSP
- ✅ Понятно когда парсинг завершён
- ✅ Нет ощущения "зависания" при долгом парсинге

---

## 📋 Задачи

---

## БЛОК A: LSP Configuration (🔴 КРИТИЧЕСКИЙ ПРИОРИТЕТ)

**Цель:** Передача настроек из Extension в LSP при старте

---

### Task A1: Определить LSP Configuration структуры (Rust)

**Приоритет:** 🔴 Критический
**Оценка:** 1 час

**Что делать:**
1. Создать структуры в `backend/src/bin/lsp_server.rs`:
   ```rust
   #[derive(Debug, Clone, Deserialize)]
   struct LspConfig {
       platform_docs_archive: Option<String>,
       configuration_path: Option<String>,
       platform_version: Option<String>,
   }
   ```

2. Добавить поле в `BslLanguageServer`:
   ```rust
   struct BslLanguageServer {
       config: Arc<RwLock<Option<LspConfig>>>,
       // ... остальные поля
   }
   ```

**Критерий выполнения:**
- ✅ Структура `LspConfig` определена
- ✅ Добавлено поле `config` в `BslLanguageServer`
- ✅ Код компилируется без ошибок

---

### Task A2: Чтение initializationOptions в initialize()

**Приоритет:** 🔴 Критический
**Оценка:** 2 часа

**Что делать:**
1. Обновить `initialize()` метод:
   ```rust
   async fn initialize(&self, params: InitializeParams) -> JsonRpcResult<InitializeResult> {
       info!("Initializing BSL Language Server");

       // Читаем initializationOptions
       if let Some(options) = params.initialization_options {
           match serde_json::from_value::<LspConfig>(options) {
               Ok(config) => {
                   info!("📂 LSP Config received: {:?}", config);

                   // Сохраняем конфигурацию
                   *self.config.write().await = Some(config.clone());

                   // Перезапускаем SystemCoordinator с новыми настройками
                   self.reload_with_config(config).await?;
               }
               Err(e) => {
                   error!("Failed to parse LSP config: {}", e);
               }
           }
       } else {
           warn!("No initializationOptions provided - using defaults");
       }

       // Возвращаем capabilities...
   }
   ```

2. Реализовать `reload_with_config()`:
   ```rust
   async fn reload_with_config(&self, config: LspConfig) -> Result<()> {
       let syntax_path = config.platform_docs_archive
           .as_ref()
           .map(|p| Path::new(p.as_str()));

       // Перезапускаем SystemCoordinator
       self.type_service.reload_types(syntax_path).await?;

       Ok(())
   }
   ```

**Критерий выполнения:**
- ✅ `initialize()` читает `initializationOptions`
- ✅ LSP логирует полученную конфигурацию
- ✅ При ошибке парсинга - fallback на defaults
- ✅ `reload_with_config()` реализован

---

### Task A3: Передача initializationOptions из Extension

**Приоритет:** 🔴 Критический
**Оценка:** 1 час

**Что делать:**
1. Обновить `vscode-extension/src/lsp/client.ts`:
   ```typescript
   const clientOptions: LanguageClientOptions = {
       documentSelector: [
           { scheme: 'file', language: 'bsl' },
           { scheme: 'untitled', language: 'bsl' }
       ],
       synchronize: {
           fileEvents: [
               vscode.workspace.createFileSystemWatcher('**/*.bsl'),
               vscode.workspace.createFileSystemWatcher('**/*.os'),
               vscode.workspace.createFileSystemWatcher('**/Configuration.xml')
           ],
           configurationSection: 'bslAnalyzer'
       },
       initializationOptions: {
           platformDocsArchive: BslAnalyzerConfig.platformDocsArchive,
           configurationPath: BslAnalyzerConfig.configurationPath,
           platformVersion: BslAnalyzerConfig.platformVersion
       },
       outputChannel: outputChannel,
       // ... остальное
   };
   ```

2. Добавить логирование:
   ```typescript
   outputChannel.appendLine(`📤 Sending initializationOptions to LSP:`);
   outputChannel.appendLine(`   platformDocsArchive: ${BslAnalyzerConfig.platformDocsArchive}`);
   outputChannel.appendLine(`   configurationPath: ${BslAnalyzerConfig.configurationPath}`);
   ```

**Критерий выполнения:**
- ✅ `initializationOptions` передаются в LSP
- ✅ Логируются переданные значения
- ✅ Extension компилируется без ошибок
- ✅ LSP получает настройки (проверяется в логах)

---

### Task A4: Реализовать TypeSystemService.reload_types()

**Приоритет:** 🔴 Критический
**Оценка:** 2 часа

**Что делать:**
1. Добавить метод в `backend/src/application/type_system_service.rs`:
   ```rust
   pub async fn reload_types(&self, syntax_helper_path: Option<&Path>) -> Result<(), String> {
       info!("🔄 Reloading types with syntax_helper_path: {:?}", syntax_helper_path);

       // 1. Очищаем TypeRepository
       self.analysis_engine.type_repository().clear();

       // 2. Перезапускаем SystemCoordinator с новым путём
       let coordinator = SystemCoordinator::new();
       coordinator.start_with_paths(syntax_helper_path, None).await
           .map_err(|e| format!("Failed to reload types: {}", e))?;

       // 3. Обновляем AnalysisEngine
       let new_repository = coordinator.type_repository()
           .ok_or("TypeRepository not initialized")?;

       self.analysis_engine.update_repository(new_repository);

       let stats = self.analysis_engine.type_repository().get_stats();
       info!("✅ Types reloaded: {} total types", stats.total_types);

       Ok(())
   }
   ```

**Критерий выполнения:**
- ✅ Метод `reload_types()` реализован
- ✅ TypeRepository очищается перед перезагрузкой
- ✅ Новые типы загружаются из синтаксис-помощника
- ✅ Логируется количество загруженных типов

---

### Task A5: Тестирование LSP Configuration

**Приоритет:** 🔴 Критический
**Оценка:** 2 часа

**Что делать:**
1. Запустить Extension Development Host (F5)
2. Проверить логи в Output Channel "BSL Analyzer":
   - ✅ "📤 Sending initializationOptions to LSP: platformDocsArchive: C:/path/..."
   - ✅ "📂 LSP Config received: LspConfig { platform_docs_archive: Some(...) }"
   - ✅ "✅ Types reloaded: 3927 total types"

3. Проверить файл `vscode-extension/rust_lsp_server.log`:
   - ✅ "📂 Загружаем синтаксис-помощник: C:/path/syntax_helper"
   - ✅ "✅ Парсинг синтаксис-помощника завершен успешно"
   - ✅ "📊 Загружено 3927 типов из синтаксис-помощника"

4. Проверить hover на переменную:
   - Открыть файл `cli/test_inline_hover.bsl`
   - Навести на `МассивДанных`
   - **Ожидаемый результат:** Hover показывает тип "Массив" + методы

**Критерий выполнения:**
- ✅ LSP получает конфигурацию из Extension
- ✅ Парсинг документации происходит успешно
- ✅ TypeRepository содержит 3927 типов
- ✅ Hover работает с платформенными типами

---

## БЛОК B: LSP Custom Requests для Type Index

**Цель:** Предоставить UI доступ к типам из TypeRepository через LSP

---

### Task B1: Реализовать bsl/getAllTypes Custom Request

**Приоритет:** 🟡 Высокий
**Оценка:** 2 часа

**Что делать:**
1. Добавить в `lsp_server.rs`:
   ```rust
   // Регистрация custom request handler
   client.on_custom_request("bsl/getAllTypes", |_params, state| async move {
       let types = state.type_service.get_all_types_as_dto().await?;
       Ok(serde_json::to_value(types)?)
   });
   ```

2. Использовать существующий метод `TypeSystemService::get_all_types_as_dto()`

**Критерий выполнения:**
- ✅ Custom Request `bsl/getAllTypes` зарегистрирован
- ✅ Возвращает все типы из TypeRepository
- ✅ Тестируется через `client.sendRequest()`

---

### Task B2: Реализовать bsl/searchTypes Custom Request

**Приоритет:** 🟡 Высокий
**Оценка:** 2 часа

**Что делать:**
1. Добавить параметры:
   ```rust
   #[derive(Debug, Deserialize)]
   struct SearchTypesParams {
       query: String,
       category: Option<String>,
       limit: Option<usize>,
   }
   ```

2. Реализовать handler:
   ```rust
   client.on_custom_request("bsl/searchTypes", |params: SearchTypesParams, state| async move {
       let results = state.type_service.search_types(
           &params.query,
           params.category.as_deref(),
           params.limit.unwrap_or(50)
       ).await?;
       Ok(serde_json::to_value(results)?)
   });
   ```

**Критерий выполнения:**
- ✅ Custom Request `bsl/searchTypes` зарегистрирован
- ✅ Поддерживает поиск по имени типа
- ✅ Фильтрация по категории (опционально)
- ✅ Лимит результатов (опционально)

---

### Task B3: Интеграция HierarchicalTypeIndexProvider с LSP

**Приоритет:** 🟡 Высокий
**Оценка:** 3 часа

**Что делать:**
1. Обновить `vscode-extension/src/providers/hierarchicalTypeProvider.ts`:
   ```typescript
   private async loadTypes(): Promise<void> {
       this.platformTypes.clear();
       this.configTypes.clear();
       this.typeCategories.clear();

       try {
           // Запрашиваем типы из LSP через Custom Request
           const types = await this.requestTypesFromLSP();

           if (!types || types.length === 0) {
               this.outputChannel?.appendLine('⚠️ No types received from LSP');
               return;
           }

           this.outputChannel?.appendLine(`✅ Received ${types.length} types from LSP`);

           // Категоризируем типы
           this.categorizeTypes(types);

       } catch (error) {
           this.outputChannel?.appendLine(`❌ Failed to load types from LSP: ${error}`);
       }
   }

   private async requestTypesFromLSP(): Promise<any[]> {
       const client = getLanguageClient();
       if (!client || !isClientRunning()) {
           throw new Error('LSP client not running');
       }

       return await client.sendRequest('bsl/getAllTypes', {});
   }
   ```

2. Раскомментировать вызов `loadTypes()` в `refresh()`

**Критерий выполнения:**
- ✅ Type Repository UI запрашивает типы из LSP
- ✅ Показывает актуальные типы из TypeRepository
- ✅ При ошибке LSP - показывает заглушку
- ✅ UI обновляется при refresh

---

### Task B4: Добавить search в Type Repository UI

**Приоритет:** 🟢 Средний
**Оценка:** 2 часа

**Что делать:**
1. Добавить команду поиска:
   ```typescript
   vscode.commands.registerCommand('bslAnalyzer.searchTypes', async () => {
       const query = await vscode.window.showInputBox({
           prompt: 'Введите имя типа для поиска',
           placeHolder: 'Массив, Справочники, и т.д.'
       });

       if (!query) return;

       const results = await client.sendRequest('bsl/searchTypes', {
           query,
           limit: 100
       });

       // Показать результаты в Quick Pick
       const items = results.map(type => ({
           label: type.name,
           description: type.category,
           detail: type.description
       }));

       const selected = await vscode.window.showQuickPick(items);
       if (selected) {
           // Показать детали типа
       }
   });
   ```

**Критерий выполнения:**
- ✅ Команда `bslAnalyzer.searchTypes` реализована
- ✅ Показывает результаты поиска в Quick Pick
- ✅ Поддерживает фильтрацию по категории

---

## БЛОК C: Прогресс парсинга документации

**Цель:** Показывать пользователю прогресс загрузки типов при старте LSP

---

### Task C1: Отправка прогресс-нотификаций из LSP

**Приоритет:** 🟢 Средний
**Оценка:** 2 часа

**Что делать:**
1. Добавить в `SystemCoordinator::start_with_paths()`:
   ```rust
   // Отправляем прогресс через callback
   if let Some(syntax_path) = syntax_helper_path {
       self.send_progress("parsing_started", 0, 0, "Начинаем парсинг документации...");

       match syntax_parser.parse_syntax_helper_with_progress(syntax_path, |current, total| {
           self.send_progress("parsing", current, total, "Парсинг синтаксис-помощника...");
       }) {
           Ok(()) => {
               self.send_progress("parsing_completed", total, total, "Парсинг завершён");
           }
           Err(e) => {
               self.send_progress("parsing_failed", 0, 0, &format!("Ошибка: {}", e));
           }
       }
   }
   ```

2. Передать LSP client в SystemCoordinator для отправки нотификаций

**Критерий выполнения:**
- ✅ LSP отправляет `bsl/parsingProgress` нотификации
- ✅ Показывает текущий прогресс (current/total)
- ✅ Сообщения на русском языке

---

### Task C2: Обработка прогресс-нотификаций в Extension

**Приоритет:** 🟢 Средний
**Оценка:** 1 час

**Что делать:**
1. Обновить `client.ts`:
   ```typescript
   client.onNotification('bsl/parsingProgress', (params: ParsingProgressParams) => {
       const percentage = params.total > 0
           ? Math.round((params.current / params.total) * 100)
           : 0;

       updateStatusBar(`$(loading~spin) ${params.message} (${percentage}%)`);

       outputChannel.appendLine(
           `📊 Parsing progress: ${params.current}/${params.total} - ${params.message}`
       );
   });
   ```

2. Интегрировать с существующим прогресс-баром в `startLanguageClient()`

**Критерий выполнения:**
- ✅ Extension получает `bsl/parsingProgress` нотификации
- ✅ Обновляет status bar с процентами
- ✅ Логирует прогресс в Output Channel

---

## БЛОК D: Документация и финализация

---

### Task D1: Обновление CLAUDE.md

**Приоритет:** 🟢 Средний
**Оценка:** 1 час

**Что делать:**
1. Добавить раздел "LSP Configuration"
2. Описать `initializationOptions`
3. Документировать Custom Requests (`bsl/getAllTypes`, `bsl/searchTypes`)
4. Обновить примеры использования

**Критерий выполнения:**
- ✅ CLAUDE.md отражает LSP Configuration
- ✅ Документированы Custom Requests
- ✅ Добавлены примеры

---

### Task D2: Обновление ROADMAP_2025.md

**Приоритет:** 🟢 Средний
**Оценка:** 30 минут

**Что делать:**
1. Добавить Milestone 2.10 в Timeline
2. Обновить статистику выполнения
3. Запланировать Milestone 2.11 (Inter-procedural Analysis)

**Критерий выполнения:**
- ✅ ROADMAP_2025.md актуален
- ✅ Milestone 2.10 добавлен
- ✅ Timeline обновлён

---

## 📈 Метрики успеха

### Блок A: LSP Configuration

**До:**
- 🔴 LSP загружает 4 типа (fallback)
- 🔴 platformDocsArchive не используется
- 🔴 Hover показывает "Unknown type"

**После:**
- ✅ LSP получает конфигурацию из Extension
- ✅ Загружает 3927 типов из синтаксис-помощника
- ✅ Hover работает с платформенными типами

**Измеримые показатели:**
1. TypeRepository содержит 3927 типов (вместо 4)
2. Hover на "Массив" показывает методы
3. Парсинг документации занимает < 3 секунд

### Блок B: LSP Custom Requests

**До:**
- 🔴 Type Repository UI показывает заглушку
- 🔴 Extension дублирует кеш (~4.5 MB)
- 🔴 Нет поиска по типам

**После:**
- ✅ Type Repository UI показывает актуальные типы
- ✅ Нет дублирования кеша (-4.5 MB памяти)
- ✅ Поиск работает через LSP

**Измеримые показатели:**
1. Потребление памяти Extension: -40%
2. Latency `bsl/getAllTypes` < 100ms
3. Поиск типов < 50ms

### Блок C: Прогресс парсинга

**До:**
- 🔴 Непонятно что происходит при старте
- 🔴 Кажется что Extension "завис"

**После:**
- ✅ Прогресс-бар показывает парсинг
- ✅ Логирование в Output Channel
- ✅ Понятно когда парсинг завершён

---

## 📦 Зависимости между задачами

**Критический путь:**
```
A1 (LspConfig структуры) → A2 (initialize()) → A3 (Extension options) → A4 (reload_types()) → A5 (тестирование)
```

**Параллельные задачи:**
- B1-B4 (Custom Requests) — после A5
- C1-C2 (прогресс) — параллельно с A1-A5
- D1-D2 (документация) — в конце

**Рекомендуемый порядок:**
1. **День 1:** Task A1-A3 (LSP Configuration setup, 4 часа)
2. **День 2:** Task A4-A5 + B1 (reload_types + тестирование + getAllTypes, 6 часов)
3. **День 3:** Task B2-B3 (searchTypes + UI интеграция, 5 часов)
4. **День 4:** Task B4 + C1-C2 (поиск + прогресс, 5 часов)
5. **День 5:** Task D1-D2 (документация, 1.5 часа)

**Общее время:** 3-5 дней

---

## ✅ Критерии завершения

### Обязательные (Блок A):
1. ✅ LSP получает `initializationOptions` из Extension
2. ✅ SystemCoordinator загружает типы из `platformDocsArchive`
3. ✅ TypeRepository содержит 3927 типов
4. ✅ Hover работает с платформенными типами

### Обязательные (Блок B):
5. ✅ Custom Request `bsl/getAllTypes` реализован
6. ✅ Custom Request `bsl/searchTypes` реализован
7. ✅ Type Repository UI показывает типы из LSP
8. ✅ Поиск по типам работает

### Обязательные (Блок C):
9. ✅ Прогресс-бар показывается при парсинге
10. ✅ Логирование прогресса в Output Channel

### Обязательные (Блок D):
11. ✅ CLAUDE.md обновлён (LSP Configuration)
12. ✅ ROADMAP_2025.md обновлён (Milestone 2.10)

### Желательные:
1. 🎯 Парсинг документации < 3 секунд
2. 🎯 Latency Custom Requests < 100ms
3. 🎯 Потребление памяти Extension: -40%
4. 🎯 Type Repository UI responsive (< 50ms для поиска)

---

## 🚀 Следующие шаги после 2.10

После завершения Milestone 2.10 (LSP Configuration + Type Index):

1. **Milestone 2.11:** Inter-procedural Analysis
   - ✅ Отслеживание return types функций
   - ✅ Анализ параметров процедур
   - ✅ Базовый межмодульный анализ (CommonModules)
   - **Время:** 7-10 дней

2. **Milestone 2.12:** Flow-sensitive Analysis (CFG)
   - ✅ Построение Control Flow Graph
   - ✅ Null safety анализ
   - ✅ Type narrowing через условия
   - **Время:** 10-14 дней

3. **Milestone 2.13:** Configuration Integration
   - ✅ Парсинг Configuration.xml
   - ✅ Загрузка Configuration types в TypeRepository
   - **Время:** 5-7 дней
