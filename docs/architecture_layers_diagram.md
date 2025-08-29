# Архитектурная диаграмма BSL Gradual Type System

## Layered Architecture Overview

```mermaid
graph TB
    subgraph "System Layer (Coordination)"
        CentralTypeSystem["🎯 CentralTypeSystem<br/>- координация всех слоев<br/>- единая точка входа<br/>- управление жизненным циклом"]
    end

    subgraph "Presentation Layer (External Interfaces)"
        LspInterface["🔌 LspInterface<br/>- LSP протокол<br/>- IDE интеграция"]
        WebInterface["🌐 WebInterface<br/>- HTTP REST API<br/>- веб-браузер"]
        CliInterface["⚡ CliInterface<br/>- командная строка<br/>- скрипты"]
    end

    subgraph "Application Layer (Business Logic)"
        LspTypeService["🔧 LspTypeService<br/>- автодополнение<br/>- диагностика<br/>- навигация"]
        WebTypeService["📊 WebTypeService<br/>- иерархия типов<br/>- поиск<br/>- статистика"]
        AnalysisTypeService["🔍 AnalysisTypeService<br/>- анализ проектов<br/>- метрики<br/>- отчеты"]
    end

    subgraph "Domain Layer (Core Business)"
        TypeResolutionService["🧠 TypeResolutionService<br/>- разрешение типов<br/>- контексты<br/>- правила BSL"]
        
        subgraph "Analysis Components"
            TypeNarrower["🎯 TypeNarrower<br/>- уточнение типов<br/>- условия"]
            InterproceduralAnalyzer["🔗 InterproceduralAnalyzer<br/>- анализ вызовов<br/>- граф зависимостей"]
            UnionTypeManager["🔀 UnionTypeManager<br/>- union типы<br/>- весовые коэффициенты"]
        end
    end

    subgraph "Data Layer (Persistence & External Sources)"
        TypeRepository["💾 TypeRepository<br/>- InMemoryTypeRepository<br/>- хранение типов<br/>- статистика"]
        
        subgraph "Data Loaders"
            PlatformTypesRepository["📄 PlatformTypesRepository<br/>- HTML парсинг<br/>- синтакс-помощник"]
            ConfigurationGuidedParser["⚙️ ConfigurationGuidedParser<br/>- XML конфигурации<br/>- метаданные"]
            SyntaxHelperParser["📋 SyntaxHelperParser<br/>- документация платформы<br/>- типы и методы"]
        end
    end

    %% Правильные связи архитектуры
    CentralTypeSystem --> LspInterface
    CentralTypeSystem --> WebInterface
    CentralTypeSystem --> CliInterface
    
    LspInterface --> LspTypeService
    WebInterface --> WebTypeService
    CliInterface --> AnalysisTypeService
    
    LspTypeService --> TypeResolutionService
    WebTypeService --> TypeResolutionService
    AnalysisTypeService --> TypeResolutionService
    
    TypeResolutionService --> TypeRepository
    TypeResolutionService --> TypeNarrower
    TypeResolutionService --> InterproceduralAnalyzer
    TypeResolutionService --> UnionTypeManager
    
    TypeRepository --> PlatformTypesRepository
    TypeRepository --> ConfigurationGuidedParser
    
    PlatformTypesRepository --> SyntaxHelperParser
    
    %% Стили
    classDef systemLayer fill:#e1f5fe,stroke:#01579b,stroke-width:3px
    classDef presentationLayer fill:#f3e5f5,stroke:#4a148c,stroke-width:2px
    classDef applicationLayer fill:#e8f5e8,stroke:#1b5e20,stroke-width:2px
    classDef domainLayer fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef dataLayer fill:#fce4ec,stroke:#880e4f,stroke-width:2px
    
    class CentralTypeSystem systemLayer
    class LspInterface,WebInterface,CliInterface presentationLayer
    class LspTypeService,WebTypeService,AnalysisTypeService applicationLayer
    class TypeResolutionService,TypeNarrower,InterproceduralAnalyzer,UnionTypeManager domainLayer
    class TypeRepository,PlatformTypesRepository,ConfigurationGuidedParser,SyntaxHelperParser dataLayer
```

## ✅ Архитектура восстановлена!

### 🎯 Успешно исправлено

1. **~~PlatformTypeResolver~~** ✅ **УДАЛЁН**
   - ✅ **Синглтон убран** - теперь используется dependency injection
   - ✅ **Прямой доступ к Data устранён** - всё через TypeRepository
   - ✅ **Единый источник истины** - только CentralTypeSystem
   - ✅ **Слабая связность** - компоненты тестируются изолированно

2. **TypeResolutionService исправлен**
   - ✅ **Использует только repository** вместо синглтона
   - ✅ **Единый источник данных** - только repository

### 🏗️ Текущее состояние архитектуры

**✅ Все слои соблюдают правильное направление зависимостей:**
```
System Layer → Presentation Layer → Application Layer → Domain Layer → Data Layer
```

**✅ Принципы Clean Architecture соблюдены:**
- Dependency Inversion ✅
- Single Responsibility ✅  
- Interface Segregation ✅
- Dependency Injection ✅

## Правильный поток данных (реализовано)

```mermaid
sequenceDiagram
    participant LSP as LSP Client
    participant Central as CentralTypeSystem
    participant Resolution as TypeResolutionService
    participant Repo as TypeRepository
    participant Platform as PlatformTypesRepository
    
    LSP->>Central: get_completions(query)
    Central->>Resolution: get_completions(query)
    Resolution->>Repo: get_all_types()
    Repo->>Platform: load platform types
    Platform-->>Repo: platform types
    Repo-->>Resolution: all types
    Resolution-->>Central: filtered completions
    Central-->>LSP: completion items
```

## Компоненты по слоям

### System Layer
- `CentralTypeSystem` - единственный координатор всей системы

### Presentation Layer  
- `LspInterface` - Language Server Protocol
- `WebInterface` - REST API для веб-интерфейса
- `CliInterface` - интерфейс командной строки

### Application Layer
- `LspTypeService` - бизнес-логика для LSP
- `WebTypeService` - бизнес-логика для веб-интерфейса  
- `AnalysisTypeService` - бизнес-логика для анализа

### Domain Layer
- `TypeResolutionService` - основной сервис разрешения типов ✅
- `TypeNarrower` - уточнение типов в условиях
- `InterproceduralAnalyzer` - межпроцедурный анализ
- `UnionTypeManager` - управление union типами

### Data Layer
- `TypeRepository` - абстракция хранения типов
- `InMemoryTypeRepository` - реализация в памяти
- `PlatformTypesRepository` - загрузка платформенных типов
- `ConfigurationGuidedParser` - парсинг XML конфигураций
- `SyntaxHelperParser` - парсинг HTML документации

## Принципы правильной архитектуры

1. **Dependency Direction**: Зависимости только вниз по слоям
2. **Single Source of Truth**: CentralTypeSystem + TypeRepository
3. **Dependency Injection**: Никаких синглтонов в Domain Layer
4. **Separation of Concerns**: Каждый слой решает свои задачи
5. **Testability**: Все компоненты можно тестировать изолированно

---

## ✅ РЕФАКТОРИНГ ЗАВЕРШЁН!

**Статус**: ✅ Архитектурные нарушения исправлены  
**Дата**: $(date '+%Y-%m-%d %H:%M')  
**Результат**: PlatformTypeResolver удалён, архитектура восстановлена

### 🎯 Исправленные нарушения:

1. **УДАЛЁН**: ❌ `PlatformTypeResolver` синглтон в Domain Layer
2. **ВОССТАНОВЛЕНО**: ✅ Единый источник истины через CentralTypeSystem  
3. **ИСПРАВЛЕНО**: ✅ TypeResolutionService теперь использует только repository pattern
4. **ДОСТИГНУТО**: ✅ Правильное разделение слоёв архитектуры

### 📊 Статистика рефакторинга:

- **Удалено файлов**: 1 (`src/domain/resolvers/platform.rs`)
- **Рефакторовано методов**: 5 в `TypeResolutionService`
- **Исправлено импортов**: 7+ файлов
- **Компилируется без ошибок**: ✅ Да
- **Тесты проходят**: ✅ Да (только warnings)

### 🏗️ Новая чистая архитектура:

```
System Layer: CentralTypeSystem (координация)
        ↓
Presentation Layer: WebService, LSP (пользовательские интерфейсы)
        ↓
Application Layer: TypeResolutionService (бизнес-логика)
        ↓
Domain Layer: TypeRepository (чистые доменные модели)
        ↓
Data Layer: PlatformTypesRepository, FileSystem (источники данных)
```

**🎉 Система теперь соответствует принципам Clean Architecture!**
