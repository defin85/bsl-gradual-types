# 🗺️ BSL Gradual Types — Roadmap 2025

**Проект:** BSL Gradual Type System для 1С:Предприятие
**Философия:** Right-Sized Architecture — начинаем просто, масштабируем по необходимости
**Версия:** 1.0 → 2.0 → 3.0
**Дата:** 2025-10-05

---

## 📊 Текущее состояние проекта (Версия 1.0)

### ✅ Что работает отлично

#### Backend (Rust)
- **Right-Sized Architecture** — 6-8 компонентов вместо 25-30 ✅
- **SystemCoordinator** — единая точка координации и DI management ✅
- **TypeSystemService** — application layer с бизнес-логикой ✅
- **TypeResolver** — чистая доменная логика без I/O ✅
- **TypeMetadataLookup** — bridge для валидации методов/свойств ✅
- **SyntaxHelperParser** — 3927 типов платформы из документации ✅
- **Web API** — REST endpoints для валидации кода ✅
- **LSP Server** — работающий Language Server Protocol ✅

**Статистика:**
- 🎯 3927 типов платформы 1С
- ⚡ Валидация < 1ms
- 📦 Бинарник LSP сервера: 7.3 MB
- ✅ 0 clippy warnings
- 🧪 Unit-тесты для критичных компонентов

#### VSCode Extension (TypeScript)
- **7,591 строк кода** — модульная архитектура ✅
- **LSP клиент** — STDIO/TCP режимы, health check ✅
- **20+ команд** — BSL Index, Verification, Analyzer ✅
- **5 сайдбар-панелей** — Overview, Diagnostics, Type Index ✅
- **BSL грамматика** — синтаксис-подсветка с кириллицей ✅
- **0 TypeScript ошибок** — strict mode ✅

### ⚠️ Что нужно улучшить

#### VSCode Extension
- 🚨 **Размер 30 MB** — слишком большой (норма < 5 MB)
- 🚨 **10 бинарников** — дублирование функциональности
- 🚨 **CLI вызовы** — вместо LSP requests
- 🚨 **Enhanced Features неактивны** — объявлены, но исключены из сборки
- 🚨 **Отсутствие тестов** — только заглушки

#### Backend
- ⚠️ **Tree-sitter НЕ используется** — BslParser возвращает пустой AST
- ⚠️ **Парсинг кода не работает** — валидация только по именам типов
- ⚠️ **Flow-sensitive analysis** — не реализован
- ⚠️ **Union types** — базовая поддержка без нормализации

---

## 🎯 Версия 2.0 — "Production Ready" (Q1 2025: 8-10 недель)

**Цель:** Превратить MVP в production-ready инструмент для ежедневной работы разработчиков 1С

**Ключевое изменение:** Tree-sitter интеграция — **ОСНОВА** всего остального анализа

---

## ✅ Завершённые Milestones (Компактный формат)

### 🧠 Milestone 2.1: Tree-sitter Integration ✅
Подключена tree-sitter-bsl v0.1.5, TreeSitterAdapter, инкрементальный парсинг < 10ms.

### 📦 Milestone 2.2: VSCode Extension Optimization ✅ (2025-10-13)
VSIX 3.2 MB (было 30 MB), 1 бинарник (было 10), все команды через LSP (0 CLI), 6 test suites.

### 🔧 Milestone 2.3: Advanced Type System ✅ (2025-10-13)
Union/Intersection/Generic/Nullable Types реализованы, 50 unit-тестов проходят, GenericInference готов.

### 🎨 Milestone 2.5: Унификация визуализации типов ✅
Крейт `type-visualization`, HtmlRenderer/JsonRenderer/MarkdownRenderer, LSP custom request `bsl/renderTypeHtml`.

### 🔧 Milestone 2.7: TreeSitterAdapter ✅
Полная конвертация tree-sitter AST → BSL IR, поддержка всех конструкций 1С, обработка ошибок с fallback.

### 🏗️ Milestone 2.8: Semantic IR Layer ✅
`SemanticProgram` с упрощённым набором узлов, `SymbolTable`, `Parser trait` для DI, `AstToIrConverter`.

### ✨ Milestone 2.9: Inline Scope Analysis ✅ (2025-10-08)
Анализ типов локальных переменных "на лету", `find_variable_at_position()`, работает в пределах одной процедуры.

### 📦 Milestone 2.10: LSP Configuration + Type Index ✅ (2025-10-08)
LSP принимает конфигурацию через initialization options, custom requests `bsl/renderTypeHtml`, `bsl/extractPlatformDocs`.

### 🔍 Milestone 2.11: Tree-Sitter Span Extraction ✅ (2025-10-13)
Реальные координаты из tree-sitter, `find_node_at_position()` работает, hover показывает разную информацию для переменных, 10 тестов проходят.

### 🎨 Milestone 2.16: Semantic Tree Visualization ✅ (2025-10-17)
VSCode webview с интерактивной визуализацией семантического дерева, LSP custom request `bsl.getSemanticHtml`, исправлена иерархия узлов (FunctionCall дублирование), исправлена проблема с `activeTextEditor` (Output панель становилась активным редактором), HTML/CSS визуализация с expand/collapse.

### 🚨 Milestone 2.18: LSP Syntax Error Diagnostics ✅ (2025-10-18)
Синтаксические ошибки отображаются в LSP Diagnostics, UTF-16 координаты для кириллицы, оптимизация производительности парсинга (~300× ускорение), 40 интеграционных тестов (33 для функциональности + 7 для performance).

---

### ⚡ Milestone 2.13: IR Caching & Performance Optimization (3-5 дней)

**Приоритет:** 🔴 КРИТИЧНЫЙ — устраняет парсинг файла при КАЖДОМ hover

**Проблема:**
В текущей реализации ([type_system_service.rs:460-519](backend/src/application/type_system_service.rs#L460-L519)) файл **парсится ЗАНОВО при каждом hover**:
```rust
pub async fn get_hover_info(&self, file_content: &str, line: u32, column: u32) -> Result<Option<String>> {
    // ❌ ПРОБЛЕМА: Парсинг КАЖДЫЙ РАЗ!
    let parse_result = self.parser.parse(file_content)?;

    // ❌ ПРОБЛЕМА: Конвертация AST → IR КАЖДЫЙ РАЗ!
    let ir_program = AstToIrConverter::convert(parse_result.program, ...)?;

    // ✅ Только этот шаг быстрый
    if let Some((var_name, type_hint)) = ir_program.find_variable_at_position(line, column) {
        return Ok(Some(self.format_variable_hover(&var_name, &type_hint)));
    }
}
```

**Последствия:**
- ⚠️ Hover на большом файле (1000+ строк) — **50-100ms задержка**
- ⚠️ Быстрое наведение мыши → множественные парсинги → **тормоза**
- ⚠️ CPU usage spike при активной работе с кодом

#### Задачи:

**Task 1: IR Caching по file_hash** (2-3 дня)

**Добавить в TypeSystemService:**
```rust
// backend/src/application/type_system_service.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct TypeSystemService {
    // ... существующие поля

    // ✅ НОВОЕ: Кеш IR программ по хешу содержимого
    ir_cache: Arc<RwLock<HashMap<u64, Arc<SemanticProgram>>>>,
}

impl TypeSystemService {
    pub async fn get_hover_info(&self, file_content: &str, line: u32, column: u32) -> Result<Option<String>> {
        // 1. Хешируем содержимое (уже есть метод hash_content)
        let content_hash = self.hash_content(file_content);

        // 2. ✅ ПРОВЕРЯЕМ IR КЕШ
        let ir_program = if let Some(cached_ir) = self.ir_cache.read().await.get(&content_hash) {
            info!("✅ IR cache HIT for file hash {}", content_hash);
            cached_ir.clone()  // ✅ Кеш попадание - парсинг НЕ нужен!
        } else {
            info!("❌ IR cache MISS for file hash {}, parsing...", content_hash);

            // Парсим + конвертируем (как сейчас)
            let parse_result = self.parser.parse(file_content)?;
            let ir = AstToIrConverter::convert(parse_result.program, ...)?;
            let ir_arc = Arc::new(ir);

            // 3. ✅ СОХРАНЯЕМ В КЕШ
            self.ir_cache.write().await.insert(content_hash, ir_arc.clone());

            ir_arc
        };

        // 4. Используем IR для Inline Scope Analysis (как сейчас)
        if let Some((var_name, type_hint)) = ir_program.find_variable_at_position(line, column) {
            return Ok(Some(self.format_variable_hover(&var_name, &type_hint)));
        }

        Ok(None)
    }
}
```

**Преимущества:**
- ✅ Парсинг только **1 раз** при первом hover
- ✅ Последующие hover **мгновенные** (<5ms)
- ✅ Кеш инвалидируется автоматически при изменении файла (новый hash)

**Task 2: Eagerly Parse при `didOpen`** (1 день)

**Модифицировать LSP Server:**
```rust
// backend/src/bin/lsp_server.rs
async fn did_open(&self, params: DidOpenTextDocumentParams) {
    let text = params.text_document.text.clone();

    // 1. Кешируем текст (как сейчас)
    self.documents.write().await.insert(uri.clone(), text.clone());

    // 2. ✅ НОВОЕ: Предварительно парсим и кешируем IR
    let file_path = uri.to_file_path().unwrap().to_string_lossy().to_string();
    match self.type_service.parse_and_cache_ir(&text, &file_path).await {
        Ok(_) => info!("✅ IR cached for {}", file_path),
        Err(e) => error!("❌ Failed to cache IR for {}: {}", file_path, e),
    }

    // 3. Диагностика (как сейчас)
    self.type_service.analyze_file(&file_path).await;
}
```

**Преимущества:**
- ✅ IR готов **ДО первого hover**
- ✅ Hover **мгновенный** с самого начала
- ✅ Парсинг при открытии файла — **незаметен** для пользователя

**Task 3: Очистка старых кешей (LRU eviction)** (1 день)

**Добавить в IR Cache:**
```rust
use lru::LruCache;

// Ограничиваем размер кеша (например, 100 файлов)
ir_cache: Arc<RwLock<LruCache<u64, Arc<SemanticProgram>>>>

// Или time-based eviction
ir_cache_timestamps: Arc<RwLock<HashMap<u64, Instant>>>
```

**Цель:**
- ✅ Ограничение использования памяти
- ✅ Автоматическая очистка неактуальных IR

**Task 4: Метрики производительности** (1 день)

**Добавить логирование:**
```rust
info!("📊 IR Cache stats: hits={}, misses={}, hit_rate={:.1}%",
    cache_hits, cache_misses, hit_rate);

info!("⏱️ Hover performance: parse={}ms, ir_convert={}ms, lookup={}ms, total={}ms",
    parse_time, ir_time, lookup_time, total_time);
```

**Результат Milestone 2.13:**
- ✅ Hover **мгновенный** (<5ms) после первого парсинга
- ✅ Парсинг только 1 раз при открытии файла
- ✅ Кеш инвалидируется автоматически при изменениях
- ✅ Ограничение памяти через LRU eviction
- ✅ Метрики для мониторинга производительности

**Тестирование:**
1. Открыть большой `.bsl` файл (1000+ строк)
2. Hover на переменной — **первый раз может быть 50ms** (парсинг + кеш)
3. Hover на другой переменной — **<5ms** (кеш попадание)
4. Изменить файл (добавить строку)
5. Hover снова — **50ms** (новый hash → ре-парсинг)
6. Hover повторно — **<5ms** (кеш попадание для нового содержимого)

---

### 📦 Milestone 2.17: Configuration Metadata Parser (3-4 дня)

**Приоритет:** 🔴 КРИТИЧНЫЙ — без типов конфигурации система типов неполная

**Проблема:**
Сейчас в `TypeRepository` только **платформенные типы** (Массив, Строка, Справочники, Документы и т.д. — всего 3927 типов). Но отсутствуют **конкретные типы из конфигурации пользователя**:
- `Справочники.Номенклатура` — нет метаданных!
- `Документы.РеализацияТоваровУслуг` — нет метаданных!
- `РегистрыСведений.ЦеныНоменклатуры` — нет метаданных!

**Последствия:**
- ⚠️ Hover на переменной типа `Справочники.Номенклатура` показывает только базовые методы `СправочникМенеджер`, но не знает реквизитов конкретного справочника
- ⚠️ Autocomplete не предлагает специфичные для конфигурации поля (например, `Номенклатура.Артикул`)
- ⚠️ Type checking не валидирует обращение к несуществующим реквизитам

**Цель:**
Парсить метаданные из `Configuration.xml` и автоматически добавлять типы конфигурации в `TypeRepository`.

#### Задачи:

**Task 1: Configuration.xml Parser** (1 день)

**Добавить модуль `backend/src/data/loaders/config_metadata_parser.rs`:**
```rust
use quick_xml::Reader;
use quick_xml::events::Event;

pub struct ConfigurationMetadataParser;

impl ConfigurationMetadataParser {
    /// Парсинг Configuration.xml из указанного пути
    pub fn parse_configuration(config_path: &str) -> Result<ConfigurationMetadata> {
        let config_xml_path = PathBuf::from(config_path).join("Configuration.xml");
        let xml_content = fs::read_to_string(config_xml_path)?;

        let mut reader = Reader::from_str(&xml_content);
        let mut metadata = ConfigurationMetadata::default();

        // Парсим Catalogs (Справочники)
        metadata.catalogs = Self::parse_catalogs(&xml_content, config_path)?;

        // Парсим Documents (Документы)
        metadata.documents = Self::parse_documents(&xml_content, config_path)?;

        // Парсим InformationRegisters (РегистрыСведений)
        metadata.info_registers = Self::parse_info_registers(&xml_content, config_path)?;

        // Парсим Enums (Перечисления)
        metadata.enums = Self::parse_enums(&xml_content, config_path)?;

        Ok(metadata)
    }

    fn parse_catalogs(xml: &str, config_path: &str) -> Result<Vec<CatalogMetadata>> {
        let mut catalogs = Vec::new();

        // Ищем <Catalog uuid="..."> в Configuration.xml
        // Для каждого справочника читаем Catalogs/<Имя>/Catalog.xml
        // Извлекаем:
        // - Имя справочника
        // - Реквизиты (StandardAttributes + Attributes)
        // - Табличные части

        catalogs
    }
}

#[derive(Debug, Default)]
pub struct ConfigurationMetadata {
    pub catalogs: Vec<CatalogMetadata>,
    pub documents: Vec<DocumentMetadata>,
    pub info_registers: Vec<InfoRegisterMetadata>,
    pub enums: Vec<EnumMetadata>,
}

#[derive(Debug)]
pub struct CatalogMetadata {
    pub name: String,           // "Номенклатура"
    pub uuid: String,
    pub attributes: Vec<AttributeMetadata>,
    pub tabular_sections: Vec<TabularSectionMetadata>,
}

#[derive(Debug)]
pub struct AttributeMetadata {
    pub name: String,           // "Артикул"
    pub type_description: String, // "String(50)"
}
```

**Task 2: Интеграция с TypeRepository** (1 день)

**Добавить в `TypeRepository`:**
```rust
// shared/src/domain/repository.rs

impl TypeRepository {
    /// Загрузить типы из метаданных конфигурации
    pub fn load_configuration_types(&mut self, config_path: &str) -> Result<usize> {
        use crate::data::loaders::config_metadata_parser::ConfigurationMetadataParser;

        let metadata = ConfigurationMetadataParser::parse_configuration(config_path)?;
        let mut count = 0;

        // Создаём типы для каждого справочника
        for catalog in metadata.catalogs {
            // 1. Справочники.Номенклатура (Manager)
            let manager_type = PlatformType {
                name: format!("Справочники.{}", catalog.name),
                facets: vec![
                    self.create_catalog_manager_facet(&catalog),
                ],
                ..Default::default()
            };
            self.register_type(manager_type);
            count += 1;

            // 2. СправочникОбъект.Номенклатура (Object)
            let object_type = PlatformType {
                name: format!("СправочникОбъект.{}", catalog.name),
                facets: vec![
                    self.create_catalog_object_facet(&catalog),
                ],
                ..Default::default()
            };
            self.register_type(object_type);
            count += 1;

            // 3. СправочникСсылка.Номенклатура (Reference)
            let ref_type = PlatformType {
                name: format!("СправочникСсылка.{}", catalog.name),
                facets: vec![
                    self.create_catalog_ref_facet(&catalog),
                ],
                ..Default::default()
            };
            self.register_type(ref_type);
            count += 1;
        }

        // Аналогично для Documents, InfoRegisters, Enums

        Ok(count)
    }

    fn create_catalog_object_facet(&self, catalog: &CatalogMetadata) -> TypeFacet {
        let mut methods = Vec::new();

        // Добавляем методы из реквизитов как get/set
        for attr in &catalog.attributes {
            methods.push(MethodSignature {
                name: attr.name.clone(),  // Геттер: Артикул
                params: vec![],
                return_type: Some(attr.type_description.clone()),
            });
        }

        TypeFacet {
            kind: FacetKind::Object,
            methods,
            properties: catalog.attributes.iter().map(|a| a.name.clone()).collect(),
        }
    }
}
```

**Task 3: LSP Custom Request `bsl/parseConfiguration`** (1 день)

**Добавить в LSP Server:**
```rust
// backend/src/bin/lsp_server.rs

async fn handle_execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
    match params.command.as_str() {
        "bsl.parseConfiguration" => {
            let config_path = params.arguments[0].as_str().unwrap();

            let type_service = self.type_service.clone();
            let count = type_service
                .repository()
                .write()
                .await
                .load_configuration_types(config_path)?;

            info!("✅ Loaded {} configuration types", count);

            Ok(Some(json!({
                "types_loaded": count,
                "message": format!("Successfully loaded {} types from configuration", count)
            })))
        }
        _ => // ...
    }
}
```

**Task 4: VSCode команда для парсинга конфигурации** (1 день)

**Добавить в Extension:**
```typescript
// vscode-extension/src/commands/index.ts

await safeRegisterCommand('bslAnalyzer.parseConfiguration', async () => {
    const configPath = BslAnalyzerConfig.configurationPath;
    if (!configPath) {
        vscode.window.showWarningMessage('Please configure 1C configuration path');
        return;
    }

    try {
        const client = getLanguageClient();
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.parseConfiguration',
            arguments: [configPath]
        });

        vscode.window.showInformationMessage(
            `✅ Configuration parsed: ${result.types_loaded} types loaded`
        );
    } catch (error) {
        vscode.window.showErrorMessage(`Configuration parsing failed: ${error}`);
    }
});
```

**Результат Milestone 2.17:**
- ✅ Парсинг метаданных из Configuration.xml
- ✅ Автоматическое добавление типов конфигурации в TypeRepository
- ✅ Hover показывает реквизиты конкретных справочников/документов
- ✅ Autocomplete предлагает поля из конфигурации
- ✅ Type checking валидирует обращение к реквизитам
- ✅ Поддержка Справочников, Документов, Регистров, Перечислений

**Тестирование:**
1. Настроить путь к конфигурации в Extension settings
2. Выполнить команду "Parse Configuration"
3. Открыть `.bsl` файл с кодом:
   ```bsl
   Перем Номенклатура;
   Номенклатура = Справочники.Номенклатура.НайтиПоНаименованию("Товар");
   МойАртикул = Номенклатура.Артикул;  // ✅ Hover показывает тип "Строка(50)"
   ```
4. Проверить, что hover на `Номенклатура.Артикул` показывает правильный тип из метаданных
5. Проверить autocomplete после `Номенклатура.` — должны быть все реквизиты из Configuration

---

### 🚨 Milestone 2.18: LSP Syntax Error Diagnostics (1-2 дня)

**Приоритет:** 🟠 ВЫСОКИЙ — улучшает user experience, пользователь видит синтаксические ошибки в реальном времени

**Проблема:**
Tree-sitter парсер **УСПЕШНО обнаруживает синтаксические ошибки** (незакрытые конструкции, отсутствующие токены), но эти ошибки:
- ✅ Логируются в файл `vscode-extension/rust_lsp_server.log`
- ✅ Доступны в `ParseResult.syntax_errors`
- ❌ **НЕ передаются** в LSP протокол через `publish_diagnostics`
- ❌ **НЕ видны пользователю** в VSCode (нет красных волнистых линий)

**Текущая ситуация:**
```rust
// backend/src/bin/lsp_server.rs - did_change() (строки 492-493)

// ❌ ПРОБЛЕМА: Диагностики пустые!
let all_diagnostics: Vec<Diagnostic> = Vec::new();
// TODO: интегрировать с analyze_file для получения реальных диагностик
```

**Цель:**
Передавать синтаксические ошибки из `ParseResult` в LSP Diagnostics для отображения пользователю в VSCode.

#### Задачи:

**Task 1: Конвертация ParseError → LSP Diagnostic** (0.5 дня)

**Добавить в LSP Server:**
```rust
// backend/src/bin/lsp_server.rs

impl BslLanguageServer {
    /// Конвертировать синтаксические ошибки в LSP Diagnostics
    fn syntax_errors_to_diagnostics(&self, errors: &[ParseError]) -> Vec<Diagnostic> {
        errors
            .iter()
            .map(|error| {
                let severity = match error.error_type {
                    ErrorType::ParseError | ErrorType::InvalidSyntax => DiagnosticSeverity::ERROR,
                    ErrorType::MissingToken => DiagnosticSeverity::ERROR,
                    ErrorType::UnexpectedToken => DiagnosticSeverity::WARNING,
                };

                Diagnostic {
                    range: Range::new(
                        Position::new(error.span.start_line, error.span.start_column),
                        Position::new(error.span.end_line, error.span.end_column)
                    ),
                    severity: Some(severity),
                    message: error.message.clone(),
                    source: Some("bsl-syntax".to_string()),
                    code: Some(NumberOrString::String(format!("{:?}", error.error_type))),
                    ..Default::default()
                }
            })
            .collect()
    }
}
```

**Task 2: Интеграция в did_open()** (0.5 дня)

**Модифицировать did_open() handler:**
```rust
// backend/src/bin/lsp_server.rs

async fn did_open(&self, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri;
    let text = params.text_document.text.clone();
    let version = params.text_document.version;

    // Кешируем текст
    self.documents.write().await.insert(uri.clone(), text.clone());

    // ✅ НОВОЕ: Парсим и получаем синтаксические ошибки
    let mut diagnostics = Vec::new();

    match self.get_type_service().parse_file(&text, uri.path()).await {
        Ok(parse_result) => {
            // ✅ Конвертируем синтаксические ошибки в LSP Diagnostics
            if parse_result.has_errors() {
                info!("⚠️ Found {} syntax errors in {}", parse_result.syntax_errors.len(), uri);
                diagnostics.extend(self.syntax_errors_to_diagnostics(&parse_result.syntax_errors));
            } else {
                info!("✅ No syntax errors in {}", uri);
            }
        }
        Err(e) => {
            error!("Failed to parse document {}: {}", uri, e);
            // Создаём диагностику об ошибке парсинга
            diagnostics.push(Diagnostic {
                range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!("❌ Ошибка парсинга: {}", e),
                source: Some("bsl-syntax".to_string()),
                ..Default::default()
            });
        }
    }

    // Отправляем диагностики
    self.client.publish_diagnostics(uri.clone(), diagnostics, Some(version)).await;
}
```

**Task 3: Интеграция в did_change()** (0.5 дня)

**Модифицировать did_change() handler:**
```rust
// backend/src/bin/lsp_server.rs

async fn did_change(&self, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;
    let version = params.text_document.version;
    let changes = params.content_changes;

    // Применяем изменения к тексту (как сейчас)
    let updated_text = /* ... существующая логика ... */;

    // Кешируем текст
    self.documents.write().await.insert(uri.clone(), updated_text.clone());

    // ✅ НОВОЕ: Парсим и получаем синтаксические ошибки
    let mut diagnostics = Vec::new();

    match self.get_type_service().parse_file(&updated_text, uri.path()).await {
        Ok(parse_result) => {
            if parse_result.has_errors() {
                info!("⚠️ Found {} syntax errors in {}", parse_result.syntax_errors.len(), uri);
                diagnostics.extend(self.syntax_errors_to_diagnostics(&parse_result.syntax_errors));
            }
        }
        Err(e) => {
            error!("Failed to parse document {}: {}", uri, e);
        }
    }

    // Отправляем обновленные диагностики
    self.client.publish_diagnostics(uri.clone(), diagnostics, Some(version)).await;
}
```

**Task 4: Добавить метод parse_file() в TypeSystemService** (0.5 дня)

**Добавить в TypeSystemService:**
```rust
// backend/src/application/type_system_service.rs

impl TypeSystemService {
    /// Парсинг файла с возвратом ParseResult (включая синтаксические ошибки)
    pub async fn parse_file(&self, content: &str, file_path: &str) -> Result<ParseResult> {
        // Используем существующий ParserCoordinator
        self.parser.parse(content)
    }
}
```

**Результат Milestone 2.18:**
- ✅ Синтаксические ошибки отображаются в VSCode с красными волнистыми линиями
- ✅ Пользователь видит ошибки в реальном времени при вводе кода
- ✅ Diagnostics panel показывает список всех синтаксических ошибок
- ✅ Поддержка разных severity levels (Error, Warning)
- ✅ Информативные сообщения об ошибках

**Тестирование:**
1. Открыть `.bsl` файл с незакрытой конструкцией:
   ```bsl
   Функция Тест()
       Если Истина Тогда
           Сообщить("Привет");
       // Отсутствует КонецЕсли!
       Возврат;
   КонецФункции
   ```
2. **Ожидаемый результат:**
   - ❌ Красная волнистая линия на строке с `Возврат;`
   - 🔴 Error в Diagnostics panel: "Отсутствует обязательный элемент: ENDIF_KEYWORD"
   - 📍 Позиция ошибки точно указана (line, column)

3. Исправить ошибку, добавив `КонецЕсли;`
4. **Ожидаемый результат:**
   - ✅ Диагностика исчезает в реальном времени
   - ✅ Diagnostics panel пуст

**Интеграционные тесты:**
```rust
// backend/tests/lsp_syntax_diagnostics_test.rs

#[tokio::test]
async fn test_lsp_reports_syntax_errors() {
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.expect("Failed to start");

    let type_service = coordinator.type_service().expect("No type service");

    // Код с синтаксической ошибкой
    let source = r#"
Функция Тест()
    Если Истина Тогда
        Сообщить("Привет");
    Возврат;
КонецФункции
"#;

    let parse_result = type_service.parse_file(source, "test.bsl").await.unwrap();

    // Проверяем, что ошибка обнаружена
    assert!(parse_result.has_errors());
    assert_eq!(parse_result.syntax_errors.len(), 1);

    let error = &parse_result.syntax_errors[0];
    assert_eq!(error.error_type, ErrorType::MissingToken);
    assert!(error.message.contains("ENDIF_KEYWORD"));
}
```

---

### 🏗️ Milestone 2.19: Architectural Improvements (2-3 дня)

**Приоритет:** 🟡 СРЕДНИЙ — улучшение качества кода и maintainability

**Контекст:**
После завершения Milestone 2.18, **reviewer агент** провёл код-ревью и выявил несколько архитектурных проблем, которые не критичны для функциональности, но важны для поддерживаемости и соответствия Clean Architecture:

1. **Дублирование ParseError** — два одинаковых типа в `backend` и `shared`
2. **Нарушение Clean Architecture** — LSP напрямую обращается к ParserCoordinator
3. **Молчаливая обработка ошибок** — некорректные координаты от tree-sitter пропускаются без логирования

**Цель:**
Устранить выявленные архитектурные проблемы и улучшить качество кода без изменения функциональности.

#### Задачи:

**Task 1: Устранение дублирования ParseError** (1 день)

**Проблема:**
Сейчас есть ДВА идентичных типа:
- `backend::parsing::bsl::ParseError` — в backend
- `shared::domain::types::ParseError` — в shared

И функция `convert_parse_errors()` для конвертации между ними.

**Решение:**

```rust
// ❌ УДАЛИТЬ backend/src/parsing/bsl/mod.rs
pub struct ParseError {
    pub error_type: ErrorType,
    pub message: String,
    pub span: Span,
}

// ✅ shared/src/domain/types.rs — ЕДИНСТВЕННЫЙ источник истины
pub struct ParseError {
    pub error_type: ErrorType,
    pub message: String,
    pub span: crate::ir::Span,
}

// ✅ backend/src/parsing/bsl/mod.rs — реэкспорт
pub use bsl_shared::domain::types::{ParseError, ErrorType};

// ✅ backend/src/bin/lsp_server.rs — прямое использование shared типа
fn syntax_errors_to_diagnostics(&self, errors: &[bsl_shared::domain::types::ParseError]) -> Vec<Diagnostic> {
    // ❌ УДАЛИТЬ функцию convert_parse_errors() — конвертация больше не нужна!
}
```

**Преимущества:**
- ✅ Устранение ~40 строк дублированного кода
- ✅ Единая точка определения типа ошибки
- ✅ Автоматическая синхронизация при добавлении новых ErrorType
- ✅ Соответствие Clean Architecture

**Task 2: API парсинга через TypeSystemService** (1 день)

**Проблема:**
LSP Server напрямую обращается к ParserCoordinator через `SystemCoordinator.parser_coordinator()`, нарушая Clean Architecture:

```rust
// ❌ НАРУШЕНИЕ: LSP → SystemCoordinator → ParserCoordinator
let backend_errors = match coordinator.parser_coordinator().parse(text.as_str()) { ... }
```

**Решение:**

```rust
// ✅ backend/src/application/type_system_service.rs
impl TypeSystemService {
    /// Распарсить код и получить диагностики (Milestone 2.18)
    ///
    /// Unified API для LSP Server — скрывает детали координации парсеров.
    pub async fn parse_and_validate(&self, source: &str) -> Result<Vec<bsl_shared::domain::types::ParseError>> {
        // Используем ParserCoordinator внутри, но не exposing его LSP напрямую
        let parse_result = self.parser.parse(source)?;

        if parse_result.has_errors() {
            Ok(parse_result.syntax_errors)
        } else {
            Ok(vec![])
        }
    }
}

// ✅ backend/src/bin/lsp_server.rs
async fn did_open(&self, params: DidOpenTextDocumentParams) {
    // Получаем ошибки через TypeSystemService API (Clean Architecture!)
    match self.get_type_service().parse_and_validate(&text).await {
        Ok(errors) => {
            let diagnostics = self.syntax_errors_to_diagnostics(&errors);
            self.client.publish_diagnostics(uri.clone(), diagnostics, Some(version)).await;
        }
        Err(e) => {
            error!("Failed to parse document {}: {}", uri, e);
        }
    }
}
```

**Преимущества:**
- ✅ LSP Server НЕ знает о ParserCoordinator (правильная изоляция слоёв)
- ✅ TypeSystemService — **единая точка входа** для Application Layer
- ✅ Упрощение тестирования (можно мокировать TypeSystemService)

**Task 3: Улучшение обработки некорректных координат** (0.5 дня)

**Проблема:**
Текущий код молчаливо пропускает ошибки tree-sitter:

```rust
// ❌ Молчаливо возвращает пустую строку
let start_line_text = source.lines().nth(start_pos.row).unwrap_or("");
```

**Решение:**

```rust
// ✅ backend/src/system/tree_sitter_adapter.rs
let start_line_text = lines.get(start_pos.row).map(|s| s.as_str()).unwrap_or_else(|| {
    tracing::warn!(
        "⚠️ Tree-sitter returned INVALID start line {} for node '{}' (file has {} lines). \
         This indicates a bug in tree-sitter-bsl grammar or file encoding issue.",
        start_pos.row,
        node.kind(),
        lines.len()
    );
    ""
});
```

**Преимущества:**
- ✅ Облегчает отладку проблем с парсером
- ✅ Выявляет баги в tree-sitter grammar
- ✅ Помогает обнаружить проблемы с кодировкой файлов

**Task 4: Документация архитектурных решений** (0.5 дня)

**Добавить комментарии в ключевые файлы:**

```rust
// backend/src/bin/lsp_server.rs

/// Конвертировать синтаксические ошибки в LSP Diagnostics
///
/// # Milestone 2.18
/// Используется для отображения синтаксических ошибок в VSCode.
///
/// # Milestone 2.19
/// Работает напрямую с shared::domain::types::ParseError (устранено дублирование типов).
fn syntax_errors_to_diagnostics(&self, errors: &[ParseError]) -> Vec<Diagnostic> {
    // ...
}
```

**Результат Milestone 2.19:**
- ✅ Устранено дублирование ParseError (~40 строк)
- ✅ Восстановлена Clean Architecture (LSP → TypeSystemService → Parser)
- ✅ Улучшено логирование некорректных координат от tree-sitter
- ✅ Документированы архитектурные решения
- ✅ Код соответствует принципам Right-Sized Architecture

**Тестирование:**
1. Запустить все существующие тесты — должны проходить без изменений
2. Проверить, что LSP diagnostics работают как раньше
3. Проверить логи при парсинге файлов с некорректной кодировкой

**Метрики качества после Milestone 2.19:**
- **Code Review Rating:** ⭐⭐⭐⭐ (4/5) — было 3/5
- **Architecture Compliance:** 95% — было 80%
- **Code Duplication:** <2% — было ~5%
- **Maintainability Index:** "Excellent" — было "Good"

---

### 📈 Milestone 2.4: Performance & Caching (1.5 недели)

**Приоритет:** 🟠 ВЫСОКИЙ — критично для работы с реальными проектами

**Можно делать параллельно с Milestone 2.5**

#### Задачи:

1. **Межсессионное кеширование** (1 неделя)
   - ✅ Кеш AST деревьев в `.bsl_cache/ast/`
   - ✅ Кеш результатов анализа в `.bsl_cache/analysis/`
   - ✅ Инвалидация при изменении файлов (по hash)
   - ✅ TTL для устаревших кешей
   - 🎯 **Цель:** Загрузка из кеша < 50ms

2. **Параллельный анализ проектов** (1 неделя)
   - ✅ Multi-threaded анализ файлов через `rayon`
   - ✅ Прогресс-бар для больших проектов
   - ✅ Graceful degradation при ошибках
   - 🎯 **Цель:** Анализ 1000 файлов < 30 секунд

**Результат Milestone 2.4:**
- ✅ Кеш работает между запусками
- ✅ Анализ больших проектов быстрый
- ✅ Оптимизация памяти

---

### 🎯 Результаты Версии 2.0 (через 8-10 недель)

**Timeline обновлён (2025-10-18):**
```
ЗАВЕРШЕНО:    🧠 Milestone 2.1 - Tree-sitter Integration (✅ ЗАВЕРШЁН)
ЗАВЕРШЕНО:    📦 Milestone 2.2 - VSCode Extension Optimization (✅ ЗАВЕРШЁН 2025-10-13)
ЗАВЕРШЕНО:    🔧 Milestone 2.3 - Advanced Type System (✅ ЗАВЕРШЁН 2025-10-13)
ЗАВЕРШЕНО:    🎨 Milestone 2.5 - Унификация визуализации (✅ ЗАВЕРШЁН)
ЗАВЕРШЕНО:    🔧 Milestone 2.7 - TreeSitterAdapter + hover (✅ ЗАВЕРШЁН)
ЗАВЕРШЕНО:    🏗️ Milestone 2.8 - Semantic IR Layer (✅ ЗАВЕРШЁН)
ЗАВЕРШЕНО:    ✨ Milestone 2.9 - Inline Scope Analysis (✅ ЗАВЕРШЁН 2025-10-08)
ЗАВЕРШЕНО:    📦 Milestone 2.10 - LSP Configuration + Type Index (✅ ЗАВЕРШЁН 2025-10-08)
ЗАВЕРШЕНО:    🔍 Milestone 2.11 - Tree-Sitter Span Extraction (✅ ЗАВЕРШЁН 2025-10-13)
ЗАВЕРШЕНО:    🎨 Milestone 2.16 - Semantic Tree Visualization (✅ ЗАВЕРШЁН 2025-10-17)
ЗАВЕРШЕНО:    🚨 Milestone 2.18 - LSP Syntax Error Diagnostics (✅ ЗАВЕРШЁН 2025-10-18)
ПЛАНИРУЕТСЯ:  🏗️ Milestone 2.19 - Architectural Improvements (🟡 СРЕДНИЙ)
ПЛАНИРУЕТСЯ:  📊 Milestone 2.12 - Custom LSP Requests (bsl/getAllTypes, bsl/searchTypes) (⏳ СРЕДНИЙ)
ПЛАНИРУЕТСЯ:  ⚡ Milestone 2.13 - IR Caching & Performance Optimization (🔴 КРИТИЧНЫЙ)
ПЛАНИРУЕТСЯ:  🔧 Milestone 2.14 - Inter-procedural Analysis (⏳ НИЗКИЙ)
ПЛАНИРУЕТСЯ:  🔧 Milestone 2.15 - Flow-sensitive Analysis (CFG) (⏳ НИЗКИЙ)
ПЛАНИРУЕТСЯ:  📦 Milestone 2.17 - Configuration Metadata Parser (🔴 КРИТИЧНЫЙ)
ПЛАНИРУЕТСЯ:  📈 Milestone 2.4 - Performance Optimization (⏳ СРЕДНИЙ)
```

**Технические метрики (обновлено 2025-10-16):**
- ✅ **Tree-sitter-bsl v0.1.5 интегрирован** — парсинг работает, TreeSitterAdapter реализован
- ✅ **Инкрементальный парсинг < 10ms** — LSP performance достигнута
- ✅ **VSCode Extension: 3.2 MB** (было 30 MB) — **ЗАВЕРШЕНО Milestone 2.2** 🎉
- ✅ **1 бинарник** (lsp_server.exe) — **ЗАВЕРШЕНО Milestone 2.2** 🎉
- ✅ **Все команды через LSP** — 0 fork процессов — **ЗАВЕРШЕНО Milestone 2.2** 🎉
- ✅ **Unit-тесты готовы** — 6 test suites — **ЗАВЕРШЕНО Milestone 2.2** 🎉
- ✅ **Advanced Type System** — Union/Intersection/Generic/Nullable — **ЗАВЕРШЕНО Milestone 2.3** 🎉
- ✅ **50+ unit-тестов проходят** — resolver + generic_inference — **ЗАВЕРШЕНО Milestone 2.3** 🎉
- ✅ **Span Extraction работает** — реальные координаты из tree-sitter — **ЗАВЕРШЕНО Milestone 2.11** 🎉
- ⚠️ **Hover парсит файл КАЖДЫЙ РАЗ** — требует IR Caching — **КРИТИЧНО Milestone 2.13**
- ⏳ **Custom LSP Requests** — bsl/getAllTypes, bsl/searchTypes — ПЛАНИРУЕТСЯ Milestone 2.12
- ⏳ **Flow-sensitive analysis (CFG)** — структуры готовы, требуется интеграция — ПЛАНИРУЕТСЯ Milestone 2.15
- ⏳ **Кеширование < 50ms** — ПЛАНИРУЕТСЯ Milestone 2.4

**Пользовательские метрики:**
- ✅ **LSP hover работает** — показывает типы переменных через Inline Scope Analysis
- ✅ **Hover показывает разную информацию** для разных переменных (не одинаковую)
- ⚠️ **Hover performance** — 50-100ms на больших файлах (парсинг каждый раз) → **требует Milestone 2.13**
- ⏳ **Автодополнение** — базовое работает, требуется улучшение контекста
- ⏳ **Syntax Error Diagnostics** — tree-sitter обнаруживает ошибки, но не показывает пользователю → **требует Milestone 2.18**
- ⏳ **Type checking** — базовая валидация работает, требуется расширение правил
- ❌ **Semantic highlighting** — НЕ РЕАЛИЗОВАНО (планируется Milestone 3.0)

---

## 🚀 Версия 3.0 — "Advanced Features" (Q2 2025: 3 месяца)

**Цель:** Превратить инструмент в полноценную IDE для 1С разработки

### 📦 Milestone 3.1: Code Intelligence (4 недели)

#### Задачи:

1. **Goto Definition** (1 неделя)
   - ✅ Переход к определению функций/процедур
   - ✅ Переход к определению переменных
   - ✅ Переход к определению типов конфигурации
   - 🎯 **Цель:** Мгновенная навигация

2. **Find References** (1 неделя)
   - ✅ Поиск всех использований символа
   - ✅ Показ в Results panel
   - ✅ Group by file
   - 🎯 **Цель:** Рефакторинг без страха

3. **Rename Symbol** (1 неделя)
   - ✅ Безопасное переименование
   - ✅ Preview изменений
   - ✅ Undo support
   - 🎯 **Цель:** Рефакторинг одним кликом

4. **Signature Help** (1 неделя)
   - ✅ Подсказки параметров функций
   - ✅ Документация параметров
   - ✅ Навигация по параметрам
   - 🎯 **Цель:** Помощь при вызове функций

**Результат Milestone 3.1:**
- ✅ Полная навигация по коду
- ✅ Безопасный рефакторинг
- ✅ Интеллектуальные подсказки

---

### 🔧 Milestone 3.2: Code Actions (3 недели)

#### Задачи:

1. **Quick Fixes** (1 неделя)
   - ✅ Автоисправление типовых ошибок
   - ✅ Добавление недостающих импортов
   - ✅ Конвертация типов
   - 🎯 **Цель:** 1 клик для исправления

2. **Refactorings** (1 неделя)
   - ✅ Extract Method
   - ✅ Extract Variable
   - ✅ Inline Variable
   - 🎯 **Цель:** Улучшение структуры кода

3. **Generate Code** (1 неделя)
   - ✅ Generate Constructor
   - ✅ Generate Getters/Setters
   - ✅ Generate Tests
   - 🎯 **Цель:** Автоматизация рутины

**Результат Milestone 3.2:**
- ✅ 20+ Code Actions
- ✅ Рефакторинг одним кликом
- ✅ Генерация шаблонного кода

---

### 📊 Milestone 3.3: Static Analysis (3 недели)

#### Задачи:

1. **Code Quality Rules** (1 неделя)
   - ✅ Проверка сложности функций (Cyclomatic Complexity)
   - ✅ Проверка длины функций
   - ✅ Проверка дублирования кода
   - 🎯 **Цель:** Метрики качества кода

2. **Security Rules** (1 неделя)
   - ✅ Проверка SQL injection
   - ✅ Проверка XSS уязвимостей
   - ✅ Проверка небезопасного eval
   - 🎯 **Цель:** Безопасный код

3. **Performance Rules** (1 неделя)
   - ✅ Проверка неоптимальных запросов
   - ✅ Проверка циклов внутри циклов
   - ✅ Проверка лишних преобразований
   - 🎯 **Цель:** Оптимальный код

**Результат Milestone 3.3:**
- ✅ 50+ правил статического анализа
- ✅ Code Quality Dashboard
- ✅ Security & Performance отчёты

---

### 🎯 Результаты Версии 3.0 (через 6 месяцев от старта)

**Технические метрики:**
- ✅ Goto Definition, Find References, Rename
- ✅ 20+ Code Actions (Quick Fixes, Refactorings)
- ✅ 50+ Static Analysis Rules
- ✅ Code Quality Dashboard

**Пользовательские метрики:**
- ✅ Навигация как в IntelliJ IDEA
- ✅ Рефакторинг одним кликом
- ✅ Автоматическое улучшение качества кода
- ✅ Предотвращение security & performance проблем

---

## 🌐 Версия 4.0 — "Collaboration & Ecosystem" (Q3-Q4 2025: 6 месяцев)

**Цель:** Создать экосистему для совместной разработки на 1С

### Milestone 4.1: Web Platform (8 недель)

1. **Type Explorer Web App** (4 недели)
   - 📊 Визуализация иерархии типов
   - 🔍 Поиск по методам/свойствам
   - 📈 Граф зависимостей типов
   - 🎯 **Цель:** Интерактивная документация

2. **Code Quality Dashboard** (4 недели)
   - 📊 Метрики по проектам
   - 📈 Тренды качества кода
   - 🚨 Критичные проблемы
   - 🎯 **Цель:** Мониторинг качества

### Milestone 4.2: Team Features (8 недель)

1. **Git Integration** (4 недели)
   - 📝 Code Review с типизацией
   - 🔍 Diff с пониманием типов
   - ✅ PR validation
   - 🎯 **Цель:** Качественный Code Review

2. **Shared Type Definitions** (4 недели)
   - 📚 Библиотека общих типов
   - 🔄 Синхронизация между проектами
   - 📦 Package manager для типов
   - 🎯 **Цель:** Переиспользование типов

### Milestone 4.3: AI Assistant (8 недель)

1. **Type Inference ML Model** (4 недели)
   - 🧠 Обучение на реальном коде
   - 🎯 Предсказание типов с вероятностью
   - 🚀 Улучшение точности до 95%
   - 🎯 **Цель:** AI-powered типизация

2. **Code Generation** (4 недели)
   - 🤖 Генерация кода по комментариям
   - 📝 Автодополнение целых функций
   - 🔧 Рефакторинг на основе AI
   - 🎯 **Цель:** AI помощник

---

## 📅 Timeline Summary

| Версия | Период | Длительность | Ключевые фичи |
|--------|--------|--------------|---------------|
| **1.0** (текущая) | Завершена | - | MVP: LSP, Валидация, VSCode Extension |
| **2.0** | Q1 2025 | 3 месяца | Tree-sitter, Flow-sensitive, Union/Generic Types |
| **3.0** | Q2 2025 | 3 месяца | Code Intelligence, Refactorings, Static Analysis |
| **4.0** | Q3-Q4 2025 | 6 месяцев | Web Platform, Team Features, AI Assistant |

---

## 🎯 Success Metrics по версиям

### Версия 2.0 — Production Ready
- ✅ 1000+ активных пользователей
- ✅ 50+ GitHub stars
- ✅ 80% positive reviews
- ✅ < 5 critical bugs в месяц

### Версия 3.0 — Advanced Features
- ✅ 5000+ активных пользователей
- ✅ 200+ GitHub stars
- ✅ 90% positive reviews
- ✅ Топ-3 в VS Code Marketplace для 1С

### Версия 4.0 — Collaboration & Ecosystem
- ✅ 20000+ активных пользователей
- ✅ 1000+ GitHub stars
- ✅ 95% positive reviews
- ✅ #1 инструмент для разработки 1С

---

## 💡 Заключение

BSL Gradual Types следует философии **Right-Sized Architecture**:

1. **Начали просто** (v1.0) — MVP работает, пользователи есть
2. **Масштабируем по необходимости** (v2.0) — добавляем критичные фичи
3. **Расширяем экосистему** (v3.0-4.0) — создаём полноценную платформу

**Ключевой принцип:** Каждая версия должна приносить **реальную ценность пользователям**, а не просто добавлять фичи.

**Следующий шаг:** Начать работу над Milestone 2.1 — оптимизация VSCode Extension 🚀
