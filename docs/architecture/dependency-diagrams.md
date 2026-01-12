# Диаграммы зависимостей проекта

**Дата создания:** 2025-12-12
**Статус:** Актуально

---

## 1. Зависимости между crates

```mermaid
graph TB
    subgraph "Workspace"
        Backend["bsl-backend<br/>LSP Server, Web Server"]
        Shared["bsl-shared<br/>Domain, IR, Engine"]
        Frontend["bsl-frontend<br/>Leptos WASM UI"]
        CLI["bsl-cli<br/>CLI Tool"]
        TypeViz["bsl-type-visualization<br/>Hover Rendering"]
        MCPDebug["mcp-debug-server<br/>Debug Tools"]
    end

    subgraph "External"
        TreeSitter["tree-sitter-bsl<br/>(external path)"]
    end

    %% Dependencies
    Backend --> Shared
    Backend --> TypeViz
    Backend --> TreeSitter
    Frontend --> Shared
    CLI --> Shared
    MCPDebug --> Shared

    %% Styling
    classDef core fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    classDef ui fill:#e3f2fd,stroke:#1565c0,stroke-width:2px
    classDef tool fill:#fff3e0,stroke:#ef6c00,stroke-width:2px
    classDef external fill:#fce4ec,stroke:#c2185b,stroke-width:2px

    class Shared core
    class Backend,Frontend ui
    class CLI,TypeViz,MCPDebug tool
    class TreeSitter external
```

**Ключевые зависимости:**
- `bsl-shared` — ядро системы, используется всеми crates
- `bsl-backend` — зависит от shared, type-visualization, tree-sitter-bsl
- `bsl-frontend` — зависит от shared с feature `wasm`

---

## 2. Слои DDD в Backend

```mermaid
graph TB
    subgraph "Presentation Layer"
        LSP["bin/lsp_server<br/>LSP Protocol"]
        Web["bin/web_server<br/>REST API"]
    end

    subgraph "Application Layer"
        TypeSystemFacade["TypeSystemFacade<br/>Orchestration"]
        AstToIr["AstToIrConverter<br/>AST → IR"]
    end

    subgraph "System Layer"
        SystemCoordinator["SystemCoordinator<br/>Lifecycle"]
        ParserCoordinator["ParserCoordinator<br/>TreeSitter + Regex"]
        DiskCache["DiskCache<br/>Caching"]
    end

    subgraph "Domain Layer (in shared)"
        TypeResolver["TypeResolver"]
        TypeRepository["TypeRepository"]
        TypeMetadataLookup["TypeMetadataLookup"]
    end

    subgraph "Data Layer"
        SyntaxHelper["SyntaxHelperParser<br/>HTML Docs"]
        ConfigParser["ConfigurationParser<br/>XML Metadata"]
    end

    %% Flows
    LSP --> TypeSystemFacade
    Web --> TypeSystemFacade

    TypeSystemFacade --> SystemCoordinator
    TypeSystemFacade --> AstToIr

    SystemCoordinator --> ParserCoordinator
    SystemCoordinator --> DiskCache

    AstToIr --> ParserCoordinator
    TypeSystemFacade --> TypeResolver
    TypeSystemFacade --> TypeMetadataLookup

    TypeResolver --> TypeRepository
    TypeMetadataLookup --> TypeRepository

    TypeRepository --> SyntaxHelper
    TypeRepository --> ConfigParser

    %% Styling
    classDef presentation fill:#e1bee7,stroke:#7b1fa2,stroke-width:2px
    classDef application fill:#c8e6c9,stroke:#388e3c,stroke-width:2px
    classDef system fill:#bbdefb,stroke:#1976d2,stroke-width:2px
    classDef domain fill:#ffe0b2,stroke:#f57c00,stroke-width:2px
    classDef data fill:#f5f5f5,stroke:#616161,stroke-width:2px

    class LSP,Web presentation
    class TypeSystemFacade,AstToIr application
    class SystemCoordinator,ParserCoordinator,DiskCache system
    class TypeResolver,TypeRepository,TypeMetadataLookup domain
    class SyntaxHelper,ConfigParser data
```

**Правила зависимостей:**
- Presentation → Application → System/Domain → Data
- Domain не зависит от Infrastructure (чистая архитектура)

---

## 3. Структура shared crate

```mermaid
graph TB
    subgraph "shared/src"
        subgraph "API Layer"
            API["api/<br/>DTOs, contracts"]
        end

        subgraph "Domain Layer"
            Domain["domain/<br/>TypeResolver, Repository"]
            Types["types/<br/>TypeResolution, RawTypeData"]
        end

        subgraph "Engine Layer"
            Engine["engine.rs<br/>AnalysisEngine"]
            Analysis["analysis/<br/>Flow analysis"]
        end

        subgraph "IR Layer"
            IR["ir/<br/>SemanticProgram, SymbolTable"]
            Parsing["parsing/<br/>Parser trait"]
        end

        subgraph "Utils"
            Formatting["formatting/<br/>Theme, colors"]
            Utils["utils/<br/>Helpers"]
        end
    end

    %% Dependencies
    API --> Domain
    API --> Types

    Engine --> IR
    Engine --> Domain
    Engine --> Analysis

    Domain --> Types
    Domain --> IR

    Analysis --> IR
    Analysis --> Types

    %% Styling
    classDef api fill:#e3f2fd,stroke:#1565c0
    classDef domain fill:#fff3e0,stroke:#ef6c00
    classDef engine fill:#e8f5e9,stroke:#2e7d32
    classDef ir fill:#fce4ec,stroke:#c2185b
    classDef utils fill:#f5f5f5,stroke:#757575

    class API api
    class Domain,Types domain
    class Engine,Analysis engine
    class IR,Parsing ir
    class Formatting,Utils utils
```

**Ключевые модули:**
- `ir/` — Intermediate Representation (независим от парсера)
- `domain/` — бизнес-логика типизации
- `engine.rs` — точка входа для анализа

---

## 4. VSCode Extension структура

```mermaid
graph TB
    subgraph "vscode-extension/src"
        subgraph "Entry"
            Extension["extension.ts<br/>activate/deactivate"]
        end

        subgraph "LSP Client"
            Client["lsp/client/<br/>Lifecycle, options"]
            CustomReq["lsp/customRequests.ts<br/>bsl/getAllTypes"]
        end

        subgraph "Providers"
            TreeProvider["providers/<br/>TypeTreeProvider"]
            TreeBuilder["providers/<br/>TypeTreeBuilder"]
        end

        subgraph "Config"
            ConfigHelper["config/<br/>Settings"]
        end
    end

    subgraph "Backend (separate process)"
        LSPServer["LSP Server<br/>Rust"]
    end

    %% Flows
    Extension --> Client
    Extension --> TreeProvider

    Client -->|"JSON-RPC"| LSPServer
    CustomReq -->|"bsl/getAllTypes"| LSPServer

    TreeProvider --> TreeBuilder
    TreeBuilder --> CustomReq

    Extension --> ConfigHelper

    %% Styling
    classDef entry fill:#e8f5e9,stroke:#2e7d32
    classDef lsp fill:#e3f2fd,stroke:#1565c0
    classDef provider fill:#fff3e0,stroke:#ef6c00
    classDef backend fill:#fce4ec,stroke:#c2185b

    class Extension entry
    class Client,CustomReq lsp
    class TreeProvider,TreeBuilder provider
    class LSPServer backend
```

---

## Навигация

- [Архитектура системы типов](type_system_architecture.md)
- [История Milestones](milestones-history.md)
- [План рефакторинга](../roadmap/refactoring-plan.md)
