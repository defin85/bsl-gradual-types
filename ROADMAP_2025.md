# 🗺️ BSL Gradual Types — Roadmap 2025

**Проект:** BSL Gradual Type System для 1С:Предприятие
**Философия:** Right-Sized Architecture — начинаем просто, масштабируем по необходимости
**Версия:** 1.0 → 2.0 → 3.0
**Дата:** 2025-10-05

---

## 📋 Содержание

1. [Текущее состояние проекта](#-текущее-состояние-проекта-версия-10)
2. [✅ Завершённые Milestones](#-завершённые-milestones-компактный-формат) — **Детали:** [ROADMAP_ARCHIVE_2025.md](ROADMAP_ARCHIVE_2025.md)
3. [🎯 Планируемые Milestones](#-milestone-217-configuration-metadata-parser-3-4-дня)
4. [🚀 Версия 3.0 — Advanced Features](#-версия-30--advanced-features-q2-2025-3-месяца)
5. [🌐 Версия 4.0 — Collaboration & Ecosystem](#-версия-40--collaboration--ecosystem-q3-q4-2025-6-месяцев)
6. [📅 Timeline Summary](#-timeline-summary)

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

**Детальные описания доступны в архиве:** [ROADMAP_ARCHIVE_2025.md](ROADMAP_ARCHIVE_2025.md)

| Milestone | Статус | Дата завершения | Ключевой результат | Подробности |
|-----------|--------|-----------------|-------------------|-------------|
| 2.1 Tree-sitter Integration | ✅ | 2025-09-XX | Подключена tree-sitter-bsl v0.1.5, TreeSitterAdapter, инкрементальный парсинг < 10ms | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-21-tree-sitter-integration-) |
| 2.2 VSCode Extension Optimization | ✅ | 2025-10-13 | VSIX 3.2 MB (было 30 MB), 1 бинарник, все команды через LSP, 6 test suites | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-22-vscode-extension-optimization--2025-10-13) |
| 2.3 Advanced Type System | ✅ | 2025-10-13 | Union/Intersection/Generic/Nullable Types реализованы, 50 unit-тестов проходят | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-23-advanced-type-system--2025-10-13) |
| 2.5 Унификация визуализации типов | ✅ | 2025-10-XX | Крейт `type-visualization`, HtmlRenderer/JsonRenderer, LSP custom request | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-25-унификация-визуализации-типов-) |
| 2.7 TreeSitterAdapter | ✅ | 2025-10-XX | Полная конвертация tree-sitter AST → BSL IR, обработка ошибок с fallback | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-27-treesitteradapter-) |
| 2.8 Semantic IR Layer | ✅ | 2025-10-XX | `SemanticProgram`, `SymbolTable`, `Parser trait` для DI, `AstToIrConverter` | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-28-semantic-ir-layer-) |
| 2.9 Inline Scope Analysis | ✅ | 2025-10-08 | Анализ типов локальных переменных "на лету", `find_variable_at_position()` | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-29-inline-scope-analysis--2025-10-08) |
| 2.10 LSP Configuration + Type Index | ✅ | 2025-10-08 | LSP initialization options, custom requests `bsl/renderTypeHtml`, `bsl/extractPlatformDocs` | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-210-lsp-configuration--type-index--2025-10-08) |
| 2.11 Tree-Sitter Span Extraction | ✅ | 2025-10-13 | Реальные координаты из tree-sitter, `find_node_at_position()` работает, 10 тестов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-211-tree-sitter-span-extraction--2025-10-13) |
| 2.13 IR Caching & Performance | ✅ | 2025-11-01 | 37× ускорение hover (<5ms). IR Cache с LRU, xxHash64, LSP invalidation | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-213-ir-caching--performance-optimization--2025-11-01) |
| 2.14 Hash Unification | ✅ | 2025-11-01 | Централизация hash_content в shared/utils/hash.rs, устранение 4 дублирований | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-214-hash-unification--централизация-hash_content--2025-11-01) |
| 2.16 Semantic Tree Visualization | ✅ | 2025-10-17 | VSCode webview, LSP custom request `bsl.getSemanticHtml`, HTML/CSS expand/collapse | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-216-semantic-tree-visualization--2025-10-17) |
| 2.18 LSP Syntax Error Diagnostics | ✅ | 2025-10-18 | Синтаксические ошибки в LSP Diagnostics, UTF-16 координаты, ~300× ускорение парсинга, 40 тестов | [Архив](ROADMAP_ARCHIVE_2025.md#-milestone-218-lsp-syntax-error-diagnostics--2025-10-18) |

**Итого завершено:** 13 Milestones
**Прогресс Версии 2.0:** ~65% завершено

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

### 📊 Milestone 2.20: Enhanced Status Bar (2-3 дня)

**Приоритет:** 🟡 СРЕДНИЙ — улучшает user experience, даёт визуальную обратную связь

**Проблема:**
Текущая строка статуса в VSCode Extension минималистична и не даёт пользователю обратной связи о:
- Прогрессе загрузки LSP сервера (user не знает, что происходит)
- Прогрессе индексации конфигурации (на больших проектах может занимать минуты)
- Текущем контексте редактора (в какой функции/процедуре находится курсор)
- Количестве загруженных типов в TypeRepository (3927 типов платформы + конфигурация)

**Референс:**
На скриншоте показана строка статуса другого LSP расширения с индикаторами:
```
main ⟳ | ⊘ 0 △ 0 | Downloading BSL Language Server v0.24.2 - 0.34 % | ПричтениеНаСервере | Обновляем кэш файла № 4900 из 17689
```

**Цель:**
Добавить расширенные индикаторы статуса для улучшения user experience:
1. **LSP Server Status** — прогресс загрузки/инициализации сервера
2. **Configuration Indexing** — прогресс парсинга метаданных конфигурации
3. **Current Context** — отображение текущей функции/процедуры в редакторе
4. **Type Repository Stats** — количество загруженных типов (платформа + конфигурация)

#### Задачи:

**Task 1: LSP Server Status Indicator** (0.5 дня)

**Проблема:**
Пользователь не видит, что LSP сервер стартует — статус бар показывает "BSL Analyzer: Starting..." без деталей.

**Добавить в `lsp/client.ts`:**
```typescript
// vscode-extension/src/lsp/client.ts

export interface LspServerStatus {
    state: 'initializing' | 'connecting' | 'ready' | 'error';
    progress?: number;  // 0-100
    message?: string;
}

// Event emitter для обновления статуса LSP
export const lspStatusEmitter = new vscode.EventEmitter<LspServerStatus>();

async function startLanguageClient(context: vscode.ExtensionContext) {
    // ... существующий код ...

    // ✅ НОВОЕ: Уведомляем о начале инициализации
    lspStatusEmitter.fire({
        state: 'initializing',
        progress: 0,
        message: 'Starting LSP Server...'
    });

    try {
        // Запуск сервера (как сейчас)
        await client.start();

        // ✅ НОВОЕ: Уведомляем о подключении
        lspStatusEmitter.fire({
            state: 'connecting',
            progress: 50,
            message: 'Connecting to LSP Server...'
        });

        // ✅ НОВОЕ: Уведомляем о готовности
        lspStatusEmitter.fire({
            state: 'ready',
            progress: 100,
            message: 'LSP Server Ready'
        });

    } catch (error) {
        lspStatusEmitter.fire({
            state: 'error',
            message: `LSP Server failed: ${error}`
        });
    }
}
```

**Обновить `extension.ts`:**
```typescript
// vscode-extension/src/extension.ts

import { lspStatusEmitter } from './lsp/client';

export async function activate(context: vscode.ExtensionContext) {
    // ... существующий код ...

    // ✅ НОВОЕ: Подписываемся на обновления LSP статуса
    context.subscriptions.push(
        lspStatusEmitter.event((status) => {
            if (status.state === 'initializing' || status.state === 'connecting') {
                const icon = '$(sync~spin)';
                statusBarItem.text = `${icon} BSL Analyzer: ${status.message} (${status.progress}%)`;
                statusBarItem.tooltip = `LSP Server: ${status.state}\n${status.message}`;
            } else if (status.state === 'ready') {
                statusBarItem.text = '$(database) BSL Analyzer: Ready';
                statusBarItem.tooltip = 'BSL Type Safety Analyzer\nLSP Server активен';
            } else if (status.state === 'error') {
                statusBarItem.text = '$(error) BSL Analyzer: Error';
                statusBarItem.tooltip = status.message;
            }
        })
    );
}
```

**Task 2: Platform & Configuration Indexing Progress** (1 день)

**Проблема:**
При парсинге больших конфигураций (17000+ файлов) или платформенных типов из Syntax Helper (3927 типов) пользователь не видит прогресс — статус бар показывает только "BSL Index: Parsing configuration... (35%)" без деталей.

**Требуется индикация для ДВУХ типов парсинга:**
1. **Platform Types** — парсинг Syntax Helper файлов (контекстная справка 1С:Предприятие)
   - `rebuilt.shcntx_ru` — объекты, методы, свойства (~3927 типов)
   - `rebuilt.shlang_ru` — справка по языку (примитивные типы, операторы)
2. **Configuration Types** — парсинг метаданных конфигурации (Configuration.xml)
   - Справочники, Документы, Регистры, Перечисления (~100-500 типов обычно)

**Добавить в `lsp/progress.ts`:**
```typescript
// vscode-extension/src/lsp/progress.ts

export type IndexingType = 'platform' | 'configuration';

export interface DetailedIndexingProgress {
    indexingType: IndexingType;     // ✅ НОВОЕ: Тип индексации
    currentItem: number;
    totalItems: number;
    currentItemName?: string;        // Имя текущего файла или типа
}

// ✅ НОВОЕ: Расширяем IndexingProgress
export interface IndexingProgress {
    isIndexing: boolean;
    currentStep: string;
    progress: number;        // 0-100
    totalSteps: number;
    currentStepNumber: number;
    startTime?: Date;
    estimatedTimeRemaining?: string;

    // ✅ НОВОЕ: Детали индексации (платформа или конфигурация)
    detailedProgress?: DetailedIndexingProgress;
}

export function updateIndexingProgress(
    stepNumber: number,
    stepName: string,
    progress: number,
    detailedProgress?: DetailedIndexingProgress  // ✅ ИЗМЕНЕНО: переименован параметр
) {
    // ... существующий код ...

    globalIndexingProgress = {
        ...globalIndexingProgress,
        currentStep: stepName,
        progress: Math.min(progress, 100),
        currentStepNumber: stepNumber,
        estimatedTimeRemaining: eta ? `${eta}s` : 'calculating...',
        detailedProgress  // ✅ НОВОЕ: универсальный прогресс для платформы и конфигурации
    };

    updateStatusBar(undefined, globalIndexingProgress);
}

export function updateStatusBar(text?: string, progress?: IndexingProgress) {
    // ... существующий код ...

    if (progress && progress.isIndexing) {
        const icon = '$(sync~spin)';
        const percent = Math.round(progress.progress);

        // ✅ НОВОЕ: Если есть детали — показываем их с учётом типа индексации
        let detailsText = '';
        if (progress.detailedProgress) {
            const { indexingType, currentItem, totalItems } = progress.detailedProgress;

            // Разный текст для платформы и конфигурации
            if (indexingType === 'platform') {
                detailsText = ` | Тип ${currentItem}/${totalItems}`;
            } else if (indexingType === 'configuration') {
                detailsText = ` | Файл ${currentItem}/${totalItems}`;
            }
        }

        const eta = progress.estimatedTimeRemaining ? ` - ETA: ${progress.estimatedTimeRemaining}` : '';
        statusBarItem.text = `${icon} BSL Index: ${progress.currentStep} (${percent}%${eta})${detailsText}`;

        // Tooltip с деталями
        let tooltipDetails = '';
        if (progress.detailedProgress) {
            const { indexingType, currentItem, totalItems, currentItemName } = progress.detailedProgress;
            const typeLabel = indexingType === 'platform' ? 'Platform Types' : 'Configuration Types';
            tooltipDetails = `\n${typeLabel}: ${currentItem}/${totalItems}`;
            if (currentItemName) {
                tooltipDetails += `\nCurrent: ${currentItemName}`;
            }
        }

        statusBarItem.tooltip = `Step ${progress.currentStepNumber}/${progress.totalSteps}\n` +
            `Progress: ${percent}%\n${progress.currentStep}${tooltipDetails}`;
        statusBarItem.show();
    }
}
```

**Примеры использования для разных типов индексации:**

**Сценарий A: Парсинг платформенных типов (Syntax Helper)**
```typescript
// При старте LSP сервера с параметром --syntax-helper-path

// Начало парсинга Syntax Helper
updateIndexingProgress(1, 'Парсинг Syntax Helper...', 10, {
    indexingType: 'platform',
    currentItem: 150,
    totalItems: 3927,
    currentItemName: 'Массив'
});

// Статус-бар показывает:
// "$(sync~spin) BSL Index: Парсинг Syntax Helper... (10%) - ETA: 15s | Тип 150/3927"
// Tooltip:
// "Step 1/4
//  Progress: 10%
//  Парсинг Syntax Helper...
//  Platform Types: 150/3927
//  Current: Массив"
```

**Сценарий B: Парсинг конфигурации**
```typescript
// При выполнении команды "Parse Configuration"

// Начало парсинга Configuration.xml
updateIndexingProgress(2, 'Парсинг конфигурации...', 35, {
    indexingType: 'configuration',
    currentItem: 4900,
    totalItems: 17689,
    currentItemName: 'Справочники/Номенклатура/Catalog.xml'
});

// Статус-бар показывает:
// "$(sync~spin) BSL Index: Парсинг конфигурации... (35%) - ETA: 45s | Файл 4900/17689"
// Tooltip:
// "Step 2/4
//  Progress: 35%
//  Парсинг конфигурации...
//  Configuration Types: 4900/17689
//  Current: Справочники/Номенклатура/Catalog.xml"
```

**Интеграция с LSP Server (Rust backend):**

**Добавить в `backend/src/data/loaders/syntax_helper_parser.rs`:**
```rust
// backend/src/data/loaders/syntax_helper_parser.rs

impl SyntaxHelperParser {
    pub async fn parse_with_progress<F>(&self, progress_callback: F) -> Result<Vec<PlatformType>>
    where
        F: Fn(usize, usize, &str) + Send + Sync,
    {
        let entries = self.discover_syntax_helper_entries()?;
        let total = entries.len();

        let mut types = Vec::new();

        for (index, entry) in entries.iter().enumerate() {
            // ✅ НОВОЕ: Отправляем прогресс в Extension
            progress_callback(index + 1, total, &entry.type_name);

            let parsed_type = self.parse_entry(entry)?;
            types.push(parsed_type);
        }

        Ok(types)
    }
}
```

**Добавить LSP Custom Notification `bsl/indexingProgress`:**
```rust
// backend/src/bin/lsp_server.rs

#[derive(Debug, Serialize)]
struct IndexingProgressNotification {
    indexing_type: String,  // "platform" | "configuration"
    current_item: usize,
    total_items: usize,
    current_item_name: Option<String>,
}

async fn load_platform_types_with_progress(&self) {
    let parser = SyntaxHelperParser::new(&self.syntax_helper_path);

    let progress_callback = |current: usize, total: usize, name: &str| {
        // Отправляем прогресс в Extension
        let notification = IndexingProgressNotification {
            indexing_type: "platform".to_string(),
            current_item: current,
            total_items: total,
            current_item_name: Some(name.to_string()),
        };

        // ✅ НОВОЕ: Custom LSP Notification
        self.client.send_notification::<notification::Custom>(
            "bsl/indexingProgress",
            serde_json::to_value(notification).unwrap()
        );
    };

    let types = parser.parse_with_progress(progress_callback).await?;
    self.type_repository.register_types(types);
}
```

**Подписка на прогресс в Extension:**
```typescript
// vscode-extension/src/lsp/client.ts

client.onNotification('bsl/indexingProgress', (params: any) => {
    const { indexing_type, current_item, total_items, current_item_name } = params;

    // Вычисляем процент прогресса
    const percent = Math.round((current_item / total_items) * 100);

    // Обновляем статус-бар
    updateIndexingProgress(
        1,  // stepNumber
        indexing_type === 'platform' ? 'Парсинг Syntax Helper...' : 'Парсинг конфигурации...',
        percent,
        {
            indexingType: indexing_type,
            currentItem: current_item,
            totalItems: total_items,
            currentItemName: current_item_name
        }
    );
});
```

**Task 3: Current Context Indicator (Function/Procedure Name)** (1 день)

**Проблема:**
Пользователь не видит, в какой функции/процедуре находится курсор — нужно прокручивать код вверх для определения контекста.

**Добавить новый модуль `vscode-extension/src/lsp/contextProvider.ts`:**
```typescript
// vscode-extension/src/lsp/contextProvider.ts

import * as vscode from 'vscode';
import { getLanguageClient } from './client';

export interface CurrentContext {
    functionName?: string;
    procedureName?: string;
    moduleName?: string;
    line: number;
    column: number;
}

// Event emitter для обновления контекста
export const contextEmitter = new vscode.EventEmitter<CurrentContext>();

/**
 * Инициализирует отслеживание текущего контекста в редакторе
 */
export function initializeContextProvider(context: vscode.ExtensionContext) {
    // Обновляем контекст при изменении позиции курсора
    context.subscriptions.push(
        vscode.window.onDidChangeTextEditorSelection(async (event) => {
            await updateCurrentContext(event.textEditor);
        })
    );

    // Обновляем контекст при переключении редактора
    context.subscriptions.push(
        vscode.window.onDidChangeActiveTextEditor(async (editor) => {
            if (editor) {
                await updateCurrentContext(editor);
            }
        })
    );

    // Начальное обновление контекста
    if (vscode.window.activeTextEditor) {
        updateCurrentContext(vscode.window.activeTextEditor);
    }
}

async function updateCurrentContext(editor: vscode.TextEditor) {
    // Проверяем, что это BSL файл
    if (editor.document.languageId !== 'bsl') {
        contextEmitter.fire({
            line: editor.selection.active.line,
            column: editor.selection.active.character
        });
        return;
    }

    const position = editor.selection.active;
    const client = getLanguageClient();

    if (!client) {
        return;
    }

    try {
        // ✅ ИСПОЛЬЗУЕМ LSP для получения текущего символа (Milestone 2.8 IR)
        const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
            'vscode.executeDocumentSymbolProvider',
            editor.document.uri
        );

        if (!symbols || symbols.length === 0) {
            contextEmitter.fire({ line: position.line, column: position.character });
            return;
        }

        // Ищем функцию/процедуру, которая содержит текущую позицию
        const currentSymbol = findContainingSymbol(symbols, position);

        if (currentSymbol) {
            const context: CurrentContext = {
                line: position.line,
                column: position.character
            };

            if (currentSymbol.kind === vscode.SymbolKind.Function) {
                context.functionName = currentSymbol.name;
            } else if (currentSymbol.kind === vscode.SymbolKind.Method) {
                context.procedureName = currentSymbol.name;
            }

            contextEmitter.fire(context);
        }
    } catch (error) {
        console.error('Failed to get current context:', error);
    }
}

function findContainingSymbol(
    symbols: vscode.DocumentSymbol[],
    position: vscode.Position
): vscode.DocumentSymbol | undefined {
    for (const symbol of symbols) {
        // Проверяем, содержит ли символ текущую позицию
        if (symbol.range.contains(position)) {
            // Рекурсивно проверяем дочерние символы
            if (symbol.children && symbol.children.length > 0) {
                const childSymbol = findContainingSymbol(symbol.children, position);
                if (childSymbol) {
                    return childSymbol;
                }
            }
            return symbol;
        }
    }
    return undefined;
}
```

**Обновить `extension.ts` для отображения контекста:**
```typescript
// vscode-extension/src/extension.ts

import { initializeContextProvider, contextEmitter } from './lsp/contextProvider';

export async function activate(context: vscode.ExtensionContext) {
    // ... существующий код ...

    // ✅ НОВОЕ: Инициализируем контекст-провайдер
    initializeContextProvider(context);

    // ✅ НОВОЕ: Подписываемся на обновления контекста
    context.subscriptions.push(
        contextEmitter.event((ctx) => {
            // Обновляем tooltip статус-бара с информацией о текущем контексте
            let contextText = '';
            if (ctx.functionName) {
                contextText = `\nТекущая функция: ${ctx.functionName}`;
            } else if (ctx.procedureName) {
                contextText = `\nТекущая процедура: ${ctx.procedureName}`;
            }

            if (contextText) {
                statusBarItem.tooltip = statusBarItem.tooltip + contextText;
            }
        })
    );
}
```

**Task 4: Type Repository Statistics** (0.5 дня)

**Проблема:**
Пользователь не знает, сколько типов загружено в TypeRepository — не понятно, работает ли парсинг платформенной документации.

**Добавить LSP Custom Request для статистики TypeRepository:**
```typescript
// vscode-extension/src/lsp/customRequests.ts

export interface TypeRepositoryStats {
    totalTypes: number;
    platformTypes: number;
    configurationTypes: number;
    lastUpdateTime?: string;
}

/**
 * Получить статистику TypeRepository из LSP Server
 */
export async function getTypeRepositoryStats(): Promise<TypeRepositoryStats | null> {
    const client = getLanguageClient();
    if (!client) {
        return null;
    }

    try {
        const stats = await client.sendRequest<TypeRepositoryStats>(
            'bsl/getTypeRepositoryStats',
            {}
        );
        return stats;
    } catch (error) {
        console.error('Failed to get type repository stats:', error);
        return null;
    }
}
```

**Обновить `extension.ts` для отображения статистики:**
```typescript
// vscode-extension/src/extension.ts

import { getTypeRepositoryStats } from './lsp/customRequests';

export async function activate(context: vscode.ExtensionContext) {
    // ... существующий код ...

    // ✅ НОВОЕ: Периодически обновляем статистику TypeRepository
    const updateTypeStatsInterval = setInterval(async () => {
        const stats = await getTypeRepositoryStats();
        if (stats) {
            const tooltip = statusBarItem.tooltip as string;
            const statsText = `\n\nTypeRepository: ${stats.totalTypes} типов` +
                `\n- Платформа: ${stats.platformTypes}` +
                `\n- Конфигурация: ${stats.configurationTypes}`;

            // Добавляем статистику в tooltip (если её там ещё нет)
            if (!tooltip.includes('TypeRepository:')) {
                statusBarItem.tooltip = tooltip + statsText;
            }
        }
    }, 5000);  // Обновляем каждые 5 секунд

    context.subscriptions.push({
        dispose: () => clearInterval(updateTypeStatsInterval)
    });
}
```

**Результат Milestone 2.20:**
- ✅ Прогресс загрузки LSP сервера отображается в статус-баре (0-100%)
- ✅ Прогресс парсинга платформенных типов из Syntax Helper (например, "Тип 150/3927")
- ✅ Прогресс индексации конфигурации с количеством файлов (например, "Файл 4900/17689")
- ✅ Отображение текущей функции/процедуры в tooltip статус-бара
- ✅ Статистика TypeRepository (количество типов платформы и конфигурации)
- ✅ Визуальная обратная связь для всех длительных операций
- ✅ Пользователь понимает, что происходит в расширении в любой момент времени

**Тестирование:**

**Сценарий 1: Загрузка LSP сервера**
1. Перезапустить VSCode
2. **Ожидаемый результат:**
   - Статус-бар показывает: "$(sync~spin) BSL Analyzer: Starting LSP Server... (0%)"
   - Через 1-2 секунды: "$(sync~spin) BSL Analyzer: Connecting to LSP Server... (50%)"
   - Через 2-3 секунды: "$(database) BSL Analyzer: Ready"

**Сценарий 2: Парсинг платформенных типов (Syntax Helper)**
1. Запустить LSP сервер с параметром `--syntax-helper-path examples/syntax_helper`
2. **Ожидаемый результат:**
   - Статус-бар показывает: "$(sync~spin) BSL Index: Парсинг Syntax Helper... (15%) - ETA: 12s | Тип 580/3927"
   - Прогресс обновляется в реальном времени
   - Tooltip показывает:
     ```
     Step 1/4
     Progress: 15%
     Парсинг Syntax Helper...
     Platform Types: 580/3927
     Current: Массив
     ```
3. **После завершения парсинга:**
   - Статус-бар показывает: "$(database) BSL Analyzer: Ready"
   - Tooltip показывает:
     ```
     BSL Type Safety Analyzer
     LSP Server активен

     TypeRepository: 3927 типов
     - Платформа: 3927
     - Конфигурация: 0
     ```

**Сценарий 3: Индексация конфигурации**
1. Выполнить команду "BSL Analyzer: Build Index"
2. **Ожидаемый результат:**
   - Статус-бар показывает: "$(sync~spin) BSL Index: Парсинг конфигурации... (35%) - ETA: 45s | Файл 4900/17689"
   - Прогресс обновляется в реальном времени
   - Tooltip показывает:
     ```
     Step 2/4
     Progress: 35%
     Парсинг конфигурации...
     Configuration Types: 4900/17689
     Current: Справочники/Номенклатура/Catalog.xml
     ```
3. **После завершения парсинга:**
   - Статус-бар показывает: "$(database) BSL Analyzer: Ready"
   - Tooltip показывает обновлённую статистику:
     ```
     TypeRepository: 4150 типов
     - Платформа: 3927
     - Конфигурация: 223
     ```

**Сценарий 4: Текущий контекст**
1. Открыть `.bsl` файл с функцией
2. Навести курсор внутрь функции
3. **Ожидаемый результат:**
   - Tooltip статус-бара показывает: "Текущая функция: ПолучитьДанные"
4. Переместить курсор в процедуру
5. **Ожидаемый результат:**
   - Tooltip обновляется: "Текущая процедура: ОбработатьДанные"

**Сценарий 5: Статистика типов**
1. Навести мышь на статус-бар
2. **Ожидаемый результат:**
   - Tooltip показывает:
     ```
     BSL Type Safety Analyzer
     LSP Server активен

     TypeRepository: 3927 типов
     - Платформа: 3927
     - Конфигурация: 0
     ```
3. Выполнить команду "Parse Configuration"
4. **Ожидаемый результат:**
   - Tooltip обновляется:
     ```
     TypeRepository: 4150 типов
     - Платформа: 3927
     - Конфигурация: 223
     ```

**Интеграционные тесты:**
```typescript
// vscode-extension/src/test/suite/statusBar.test.ts

import * as assert from 'assert';
import * as vscode from 'vscode';
import { lspStatusEmitter } from '../../lsp/client';

suite('Status Bar Integration Tests', () => {
    test('LSP Server status updates correctly', async () => {
        // Эмулируем события LSP
        lspStatusEmitter.fire({
            state: 'initializing',
            progress: 0,
            message: 'Starting LSP Server...'
        });

        await new Promise(resolve => setTimeout(resolve, 100));

        // Проверяем, что статус-бар обновился
        const statusBarItems = vscode.window.visibleTextEditors;
        // TODO: Проверить текст статус-бара (требует доступа к statusBarItem из extension.ts)
    });
});
```

**LSP Server Changes (Rust Backend):**

**Добавить custom request `bsl/getTypeRepositoryStats` в `backend/src/bin/lsp_server.rs`:**
```rust
// backend/src/bin/lsp_server.rs

#[derive(Debug, Serialize, Deserialize)]
struct TypeRepositoryStats {
    total_types: usize,
    platform_types: usize,
    configuration_types: usize,
    last_update_time: Option<String>,
}

async fn handle_custom_request(&self, method: &str, params: Value) -> Result<Value> {
    match method {
        // ... существующие handlers ...

        // ✅ НОВОЕ: Статистика TypeRepository
        "bsl/getTypeRepositoryStats" => {
            let type_service = self.get_type_service();
            let repository = type_service.repository().read().await;

            let stats = TypeRepositoryStats {
                total_types: repository.all_types().len(),
                platform_types: repository.platform_types_count(),
                configuration_types: repository.configuration_types_count(),
                last_update_time: Some(chrono::Utc::now().to_rfc3339()),
            };

            Ok(serde_json::to_value(stats)?)
        }

        _ => Err(anyhow!("Unknown custom request: {}", method))
    }
}
```

**Добавить методы в `TypeRepository`:**
```rust
// shared/src/domain/repository.rs

impl TypeRepository {
    /// Количество типов платформы (из Syntax Helper)
    pub fn platform_types_count(&self) -> usize {
        self.types
            .values()
            .filter(|t| t.source == TypeSource::Platform)
            .count()
    }

    /// Количество типов конфигурации (из Configuration.xml)
    pub fn configuration_types_count(&self) -> usize {
        self.types
            .values()
            .filter(|t| t.source == TypeSource::Configuration)
            .count()
    }
}
```

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
ПЛАНИРУЕТСЯ:  📊 Milestone 2.20 - Enhanced Status Bar (🟡 СРЕДНИЙ)
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
