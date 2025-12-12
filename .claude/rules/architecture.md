# Архитектура системы типов

## Диаграмма компонентов

```mermaid
graph TB
    subgraph "System Layer (backend/src/system)"
        SystemCoordinator["SystemCoordinator"]
        AnalysisCache["AnalysisCache"]
        ParserCoordinator["ParserCoordinator<br/>- TreeSitter + Regex<br/>- AST → IR через AstToIrConverter"]
        BasicObservability["BasicObservability"]
    end

    subgraph "Presentation Layer (разные процессы)"
        subgraph "LSP Process"
            LSPServer["LSP Server (backend)"]
            VSCode["VSCode Extension (TypeScript)"]
        end

        subgraph "Web Process"
            WebServer["Web Server (backend)"]
            Frontend["Frontend UI (Leptos WASM)"]
            SemanticRoutes["Semantic Routes<br/>- /api/semantic/:file_path<br/>- JSON/HTML visualization"]
        end

        CLITool["CLI Tool (cli)<br/>LightweightParser (~2-3 MB)"]
    end

    subgraph "Helper Layer"
        TypeViz["type-visualization"]
    end

    subgraph "Application Layer"
        subgraph "backend/src/application"
            TypeSystemService["TypeSystemService<br/>LSP hover через AST → IR"]
            AstToIr["AstToIrConverter<br/>AST → IR bridge<br/>- Конвертирует синтаксис в семантику<br/>- Строит SymbolTable"]
        end
        subgraph "shared/src/engine"
            AnalysisEngine["AnalysisEngine<br/>analyze_program(IR)<br/>- Работает с SemanticProgram<br/>- Не зависит от парсеров"]
        end
    end

    subgraph "Semantic Layer (shared/src/ir/)"
        IR["Intermediate Representation<br/>- SemanticProgram<br/>- SemanticNode<br/>- SymbolTable<br/>- FlowSensitiveVisitor<br/>Независим от парсера!"]

        ParserTrait["Parser trait<br/>- parse() → SemanticProgram<br/>- DI для разных парсеров"]
    end

    subgraph "Domain Layer (shared/src/domain)"
        TypeResolver["TypeResolver"]
        TypeMetadataLookup["TypeMetadataLookup"]
        TypeRepository["TypeRepository (3927 типов)"]
    end

    subgraph "Data Layer"
        PlatformTypes["Platform Types<br/>(Syntax Helper)"]
        ConfigData["Configuration"]
        HbkRecovery["HBK Recovery<br/>Восстановление .hbk файлов"]
    end

    subgraph "DTOs"
        DTOs["shared/api/dtos.rs"]
    end

    %% System coordination
    SystemCoordinator --> AnalysisCache
    SystemCoordinator --> ParserCoordinator
    SystemCoordinator --> BasicObservability
    SystemCoordinator --> TypeSystemService

    %% Presentation → Application
    LSPServer --> TypeSystemService
    WebServer --> TypeSystemService
    WebServer --> SemanticRoutes
    LSPServer -.->|"bsl/getSemanticHtml"| SemanticRoutes
    SemanticRoutes --> TypeSystemService
    VSCode --> LSPServer
    Frontend --> WebServer
    CLITool --> AnalysisEngine

    %% Helper layer
    LSPServer --> TypeViz
    TypeViz -.-> DTOs

    %% Application → Semantic Layer
    TypeSystemService --> AnalysisEngine
    TypeSystemService --> AstToIr
    TypeSystemService --> ParserCoordinator

    ParserCoordinator -.->|"converts AST"| AstToIr
    AstToIr -.->|"produces"| IR

    %% CLI использует Parser trait
    CLITool -.->|"uses ParserTrait"| ParserTrait
    ParserTrait -.->|"returns"| IR

    %% ParserCoordinator implements Parser trait
    ParserCoordinator -.->|"implements"| ParserTrait

    %% AnalysisEngine работает с IR
    AnalysisEngine -.->|"analyzes"| IR
    AnalysisEngine --> TypeResolver

    %% Domain layer
    TypeResolver --> TypeRepository
    TypeMetadataLookup --> TypeRepository
    TypeSystemService -.-> TypeMetadataLookup
    AnalysisEngine -.-> TypeMetadataLookup

    TypeRepository --> PlatformTypes
    TypeRepository --> ConfigData

    %% HBK Recovery
    SystemCoordinator -.->|"auto-recovery"| HbkRecovery
    HbkRecovery -.->|".hbk → .zip"| PlatformTypes

    TypeSystemService --> DTOs
    TypeSystemService --> AnalysisCache
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

**Подробнее:** [docs/architecture/type_system_architecture.md](../../docs/architecture/type_system_architecture.md)
