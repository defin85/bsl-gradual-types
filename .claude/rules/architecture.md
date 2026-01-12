# Архитектура системы типов

## Диаграмма компонентов

```mermaid
graph TB
    subgraph "🎯 System Layer (в `backend/src/system`)"
        StartupInputs["🧾 StartupInputs<br/>- syntax/config/version/cache/strict<br/>- normalize()"]
        StartupV2["🚀 startup_v2()<br/>StartupInputs → { coordinator, deps_bundle_v2, inputs }"]
        SystemCoordinator["🎯 SystemCoordinator"]
        DiskCache["💾 DiskCache"]
        AstCache["🌳 AstCache"]
        ParserCoordinator["🎨 ParserCoordinator<br/>- Tree-sitter + fallback<br/>- DiskCache-backed"]
        IntellisenseIndex["📚 IntellisenseIndexStore"]
        IndexSnapshot["🧷 IndexSnapshot"]
        BasicObservability["📊 BasicObservability"]
        DepsBundleV2["📦 build_deps_bundle_v2()<br/>DepsBundleV2 { deps_id, semantic_deps, index_snapshot }"]
    end

    subgraph "🧩 v2 Analysis Layer (`analysis-v2`)"
        AnalysisHostV2["🧠 AnalysisHostV2<br/>- salsa DB owner (mutable)"]
        AnalysisV2["📸 AnalysisV2 snapshot (read-only)"]
        V2Queries["queries:<br/>- line_index<br/>- parse_result<br/>- ir<br/>- syntax_diagnostics<br/>- semantic_diagnostics"]
        DepsSnapshot["DepsSnapshot (SemanticDeps + deps_id)"]
        SettingsSnapshot["SettingsSnapshot (DetailLevel + settings_id)"]
    end

    subgraph "🔧 Application Layer (в `backend/src/application/type_system/services`)"
        CompletionSvc["completion_service"]
        HoverSvc["hover_service"]
        SignatureHelpSvc["signature_help_service (v2)"]
        DefinitionSvc["definition_service (v2)"]
        WebApiSvc["web_api_service"]
    end

    subgraph "🌟 Semantic Layer"
        SyntaxAST["ParseResult AST (bsl-syntax)"]
        AstToIr["AstToIrConverter (bsl-semantic)"]
        IR["SemanticProgram (IR)"]
    end

    subgraph "🧠 Domain Layer (в `shared/src/domain`)"
        TypeResolver["🧠 TypeResolver"]
        TypeMetadataLookup["🔍 TypeMetadataLookup"]
        TypeRepository["📚 TypeRepository"]
        SignatureIndex["🧾 SignatureIndex"]
    end

    subgraph "💾 Data Layer"
        PlatformTypes["📄 Platform docs (Syntax Helper / HBK)"]
        ConfigData["⚙️ Configuration types"]
        HbkRecovery["🔧 HBK Recovery<br/>- Поиск ZIP signature<br/>- Извлечение валидного архива"]
    end

    subgraph "🌐 Presentation Layer (Адаптеры - разные процессы)"
        subgraph "LSP Process"
            LSPServer["🔌 LSP Server (backend/bin)"]
            AnalysisRuntime["🧵 AnalysisV2Runtime<br/>writer thread"]
            VSCode["📦 VSCode Extension (TypeScript)"]
        end

        subgraph "Web Process"
            WebServer["🌐 Web Server (backend/bin)"]
            Frontend["🖥️ Frontend UI (Leptos WASM)"]
            SemanticRoutes["📊 Semantic Routes<br/>- /api/semantic/:file_path<br/>- /api/semantic-tree"]
        end

        CLITool["⚙️ CLI Tool (cli)"]
    end

    subgraph "🎨 Helper Layer"
        SemanticHtml["🧾 semantic_html_generator"]
        TypeViz["🎨 bsl-type-visualization"]
    end

    subgraph "📄 DTOs"
        DTOs["shared/api/dtos.rs"]
        SemanticDTOs["shared/api/semantic_dtos.rs"]
    end

    %% System wiring
    StartupInputs --> StartupV2
    StartupV2 --> SystemCoordinator
    StartupV2 --> DepsBundleV2

    SystemCoordinator --> DiskCache
    SystemCoordinator --> AstCache
    SystemCoordinator --> ParserCoordinator
    SystemCoordinator --> IntellisenseIndex
    SystemCoordinator --> BasicObservability
    SystemCoordinator --> DepsBundleV2

    IntellisenseIndex --> IndexSnapshot
    DepsBundleV2 --> IndexSnapshot

    %% Presentation wiring
    VSCode --> LSPServer
    LSPServer --> AnalysisRuntime
    AnalysisRuntime --> AnalysisHostV2
    WebServer --> AnalysisHostV2
    CLITool --> AnalysisHostV2

    WebServer --> SemanticRoutes
    Frontend --> WebServer

    %% v2 analysis (inputs -> queries)
    AnalysisHostV2 --> AnalysisV2
    AnalysisV2 --> V2Queries
    DepsBundleV2 --> DepsSnapshot

    V2Queries --> SyntaxAST
    V2Queries --> AstToIr
    AstToIr --> IR

    %% LSP/Web call services with semantic inputs
    LSPServer --> CompletionSvc
    LSPServer --> HoverSvc
    LSPServer --> SignatureHelpSvc
    LSPServer --> DefinitionSvc

    WebServer --> HoverSvc
    WebServer --> WebApiSvc

    CompletionSvc --> IR
    CompletionSvc --> IndexSnapshot
    CompletionSvc --> TypeMetadataLookup

    HoverSvc --> IR
    HoverSvc --> TypeMetadataLookup

    SignatureHelpSvc --> IR
    SignatureHelpSvc --> TypeMetadataLookup

    DefinitionSvc --> IR
    DefinitionSvc --> TypeMetadataLookup

    WebApiSvc --> DTOs

    SemanticRoutes --> SemanticHtml
    SemanticHtml --> IR

    LSPServer --> TypeViz
    TypeViz --> SemanticDTOs

    %% Domain/Data wiring
    DepsSnapshot --> TypeRepository
    DepsSnapshot --> TypeResolver
    DepsSnapshot --> SignatureIndex

    TypeResolver --> TypeRepository
    TypeMetadataLookup --> TypeRepository

    TypeRepository --> PlatformTypes
    TypeRepository --> ConfigData

    SystemCoordinator -.->|"🔧 auto-recovery<br/>при старте"| HbkRecovery
    HbkRecovery -.->|"восстанавливает<br/>.hbk → .zip"| PlatformTypes

    %% Styling
    classDef systemStyle fill:#e3f2fd,stroke:#1976d2,stroke-width:2px
    classDef presentationStyle fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px
    classDef helperStyle fill:#fff9c4,stroke:#f57f17,stroke-width:2px
    classDef applicationStyle fill:#e8f5e8,stroke:#388e3c,stroke-width:2px
    classDef v2Style fill:#e0f7fa,stroke:#006064,stroke-width:2px
    classDef semanticStyle fill:#ffe0b2,stroke:#e65100,stroke-width:4px,stroke-dasharray: 5 5
    classDef domainStyle fill:#fff3e0,stroke:#f57c00,stroke-width:2px
    classDef dataStyle fill:#fce4ec,stroke:#c2185b,stroke-width:2px
    classDef dtoStyle fill:#e1f5fe,stroke:#0277bd,stroke-width:2px

    class StartupInputs,StartupV2,SystemCoordinator,DiskCache,AstCache,ParserCoordinator,IntellisenseIndex,IndexSnapshot,BasicObservability,DepsBundleV2 systemStyle
    class LSPServer,AnalysisRuntime,WebServer,Frontend,VSCode,CLITool,SemanticRoutes presentationStyle
    class SemanticHtml,TypeViz helperStyle
    class CompletionSvc,HoverSvc,SignatureHelpSvc,DefinitionSvc,WebApiSvc applicationStyle
    class AnalysisHostV2,AnalysisV2,V2Queries,DepsSnapshot,SettingsSnapshot v2Style
    class SyntaxAST,AstToIr,IR semanticStyle
    class TypeResolver,TypeMetadataLookup,TypeRepository,SignatureIndex domainStyle
    class PlatformTypes,ConfigData,HbkRecovery dataStyle
    class DTOs,SemanticDTOs dtoStyle
```

## Потоки данных

### Загрузка данных
```
HTML документация → SyntaxHelperParser → RawTypeData → TypeRepository
```

### Статический анализ
```
Выражение → TypeResolver → RawTypeData → TypeResolution
```

### Web API
```
TypeResolution → TypeMetadataLookup → TypeDto → JSON → Frontend
```

## Ключевые структуры

| Структура | Слой | Назначение |
|-----------|------|------------|
| `RawTypeData` | Data | Хранение всех данных парсера (методы, свойства) |
| `TypeResolution` | Domain | Результат анализа (certainty, facets) |
| `TypeMetadataLookup` | Domain | Мост между Resolution и RawTypeData |
| `SemanticProgram` | Semantic | IR независимый от парсера |

## Фасеты 1С

```
Справочники.Контрагенты      → Manager
СправочникОбъект.Контрагенты → Object
СправочникСсылка.Контрагенты → Reference
```

## Резолвинг типов и свойств

### TypeResolution с active_facet

```rust
TypeResolution {
    type_name: "ДокументСсылка.ЗаказНаряды",
    active_facet: Some(Reference),  // Определяет доступные свойства!
    available_facets: [Manager, Object, Reference],
}
```

### Ключевые enum унификации

| Enum | Назначение | Пример |
|------|------------|--------|
| `FacetKind` | Тип фасета | `Manager`, `Object`, `Reference`, `Selection`, `List` |
| `MetadataKind` | Вид метаданных | `Catalog`, `Document`, `InformationRegister` |

**Методы `FacetKind`:**
- `shows_properties()` — Manager/Selection/List = `false`, Object/Reference = `true`
- `platform_suffix()` — `"Менеджер"`, `"Объект"`, `"Ссылка"`

### Flow резолвинга свойств

```
Код: Ссылка.Работы (где Ссылка: ДокументСсылка.ЗаказНаряды)
              ↓
resolve_member_type(object_type, "Работы")
              ↓
metadata_lookup.get_properties(object_type)
              ↓
active_facet = Reference → shows_properties() = true
              ↓
get_facet_properties() → платформенные + конфигурационные + табличные части
              ↓
Найдено: Работы: ТабличнаяЧасть<Работы>
```

### Хранение типов

| Где | Формат | Пример |
|-----|--------|--------|
| SymbolTable (переменные) | Фасетный | `ДокументСсылка.ЗаказНаряды` |
| TypeRepository | Коллекция | `Документы.ЗаказНаряды` |
| SignatureIndex (методы) | Базовый фасет | `ДокументОбъект`, `ДокументСсылка` |

**Конвертация:** `MetadataKind::Document.to_prefix()` → `"Документы"`

**Подробнее:** [docs/architecture/type_system_architecture.md](../../docs/architecture/type_system_architecture.md)
