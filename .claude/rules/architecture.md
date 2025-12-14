# Архитектура системы типов

## Диаграмма компонентов

```mermaid
graph TB
    subgraph "🎯 System Layer (в `backend/src/system`)"
        SystemCoordinator["🎯 SystemCoordinator"]
        AnalysisCache["💾 AnalysisCache"]
        ParserCoordinator["🎨 ParserCoordinator<br/>- TreeSitter + Regex<br/>✅ ПОСЛЕ 2.8: → IR через AstToIrConverter"]
        BasicObservability["📊 BasicObservability"]
    end

    subgraph "🌐 Presentation Layer (Адаптеры - разные процессы)"
        subgraph "LSP Process"
            LSPServer["🔌 LSP Server (backend)"]
            VSCode["📦 VSCode Extension (TypeScript)"]
        end

        subgraph "Web Process"
            WebServer["🌐 Web Server (backend)"]
            Frontend["🖥️ Frontend UI (Leptos WASM)"]
            SemanticRoutes["📊 Semantic Routes<br/>✅ MILESTONE 2.16<br/>- /api/semantic/:file_path<br/>- JSON/HTML visualization"]
        end

        CLITool["⚙️ CLI Tool (cli)<br/>✅ ПОСЛЕ 2.8: LightweightParser (~2-3 MB)"]
    end

    subgraph "🎨 Helper Layer"
        TypeViz["🎨 type-visualization"]
    end

    subgraph "🔧 Application Layer"
        subgraph "`backend/src/application`"
            TypeSystemService["🎭 TypeSystemService<br/>✅ LSP hover через AST → IR"]
            AstToIr["🔄 AstToIrConverter<br/>✅ ПОСЛЕ 2.8: AST → IR bridge<br/>- Конвертирует синтаксис в семантику<br/>- Строит SymbolTable"]
        end
        subgraph "`shared/src/engine`"
            AnalysisEngine["🚀 AnalysisEngine<br/>✅ ПОСЛЕ 2.8: analyze_program(IR)<br/>- Работает с SemanticProgram<br/>- Не зависит от парсеров"]
        end
    end

    subgraph "🌟 Semantic Layer (✅ НОВЫЙ! shared/src/ir/)"
        IR["📄 Intermediate Representation<br/>✅ ПОСЛЕ 2.8: shared/src/ir/<br/>- SemanticProgram<br/>- SemanticNode (упрощённый набор)<br/>- SymbolTable<br/>- FlowSensitiveVisitor<br/>✨ Независим от парсера!"]

        ParserTrait["🔌 Parser trait<br/>✅ ПОСЛЕ 2.8: shared/src/parsing/<br/>- parse() → SemanticProgram<br/>- DI для разных парсеров<br/>- LightweightParser для CLI"]
    end

    subgraph "🧠 Domain Layer (в `shared/src/domain`)"
        TypeResolver["🧠 TypeResolver"]
        TypeMetadataLookup["🔍 TypeMetadataLookup"]
        TypeRepository["📚 TypeRepository (3927 типов)"]
    end

    subgraph "💾 Data Layer"
        PlatformTypes["📄 Platform Types<br/>(Syntax Helper: Строка, Число, etc.)"]
        ConfigData["⚙️ Configuration"]
        HbkRecovery["🔧 HBK Recovery<br/>✅ NEW: Восстановление .hbk файлов<br/>- Поиск ZIP signature<br/>- Извлечение валидного архива<br/>- Auto-recovery при старте"]
    end

    subgraph "📄 DTOs"
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
    LSPServer -.->|"custom request<br/>bsl/getSemanticHtml"| SemanticRoutes
    SemanticRoutes --> TypeSystemService
    VSCode --> LSPServer
    Frontend --> WebServer
    CLITool --> AnalysisEngine

    %% Helper layer
    LSPServer --> TypeViz
    TypeViz -.-> DTOs

    %% Application → Semantic Layer (КЛЮЧЕВОЕ ИЗМЕНЕНИЕ 2.8)
    TypeSystemService --> AnalysisEngine
    TypeSystemService --> AstToIr
    TypeSystemService --> ParserCoordinator

    ParserCoordinator -.->|"✅ ПОСЛЕ 2.8: converts AST"| AstToIr
    AstToIr -.->|"produces"| IR

    %% CLI использует Parser trait (Dependency Injection)
    CLITool -.->|"✅ uses ParserTrait"| ParserTrait
    ParserTrait -.->|"returns"| IR

    %% ParserCoordinator implements Parser trait
    ParserCoordinator -.->|"✅ implements"| ParserTrait

    %% AnalysisEngine работает с IR
    AnalysisEngine -.->|"✅ analyzes"| IR
    AnalysisEngine --> TypeResolver

    %% Domain layer
    TypeResolver --> TypeRepository
    TypeMetadataLookup --> TypeRepository
    TypeSystemService -.-> TypeMetadataLookup
    AnalysisEngine -.-> TypeMetadataLookup

    TypeRepository --> PlatformTypes
    TypeRepository --> ConfigData

    %% HBK Recovery (NEW)
    SystemCoordinator -.->|"🔧 auto-recovery<br/>при старте"| HbkRecovery
    HbkRecovery -.->|"восстанавливает<br/>.hbk → .zip"| PlatformTypes

    TypeSystemService --> DTOs
    TypeSystemService --> AnalysisCache

    %% Styling
    classDef systemStyle fill:#e3f2fd,stroke:#1976d2,stroke-width:2px
    classDef presentationStyle fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px
    classDef helperStyle fill:#fff9c4,stroke:#f57f17,stroke-width:2px
    classDef applicationStyle fill:#e8f5e8,stroke:#388e3c,stroke-width:2px
    classDef semanticStyle fill:#ffe0b2,stroke:#e65100,stroke-width:4px,stroke-dasharray: 5 5
    classDef domainStyle fill:#fff3e0,stroke:#f57c00,stroke-width:2px
    classDef dataStyle fill:#fce4ec,stroke:#c2185b,stroke-width:2px
    classDef dtoStyle fill:#e1f5fe,stroke:#0277bd,stroke-width:2px

    class SystemCoordinator,AnalysisCache,ParserCoordinator,BasicObservability systemStyle
    class LSPServer,WebServer,Frontend,VSCode,CLITool,SemanticRoutes presentationStyle
    class TypeViz helperStyle
    class TypeSystemService,AstToIr,AnalysisEngine applicationStyle
    class IR,ParserTrait semanticStyle
    class TypeResolver,TypeMetadataLookup,TypeRepository domainStyle
    class PlatformTypes,ConfigData,HbkRecovery dataStyle
    class DTOs dtoStyle
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
