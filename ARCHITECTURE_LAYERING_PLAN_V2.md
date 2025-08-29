# План исправления архитектуры BSL Gradual Types (v2.0)

## Исправления после анализа

**Проблема v1.0:** Неверное понимание роли `CentralTypeSystem` как нарушителя архитектуры.  
**Реальность:** `CentralTypeSystem` - это архитектурно необходимый координатор и IoC контейнер.

## Цель (исправленная)

Исправить **способ доступа** к Domain Layer, а не запретить доступ координатору системы. Обеспечить принудительную изоляцию Application/Presentation слоёв через систему типов Rust.

## Реальные архитектурные проблемы

### 1. CentralTypeSystem дублирует логику вместо делегирования
```rust
// ❌ ПРОБЛЕМА: обходит собственные зависимости
impl CentralTypeSystem {
    pub async fn get_all_types_with_resolutions(&self) -> HashMap<String, TypeResolution> {
        let platform_resolver = PlatformTypeResolver::instance(); // Игнорирует self.resolution_service
        platform_resolver.get_platform_globals().clone()
    }
    
    pub async fn resolve_expression(&self, expression: &str) -> TypeResolution {
        let mut platform_resolver = PlatformTypeResolver::new(); // Создаёт новый экземпляр!
        platform_resolver.resolve_expression(expression)
    }
}
```

### 2. Application Layer обходит dependency injection
```rust
// ❌ ПРОБЛЕМА: Application Layer обращается к синглтону напрямую
impl WebTypeService {
    pub async fn search_types(&self, query: &str) -> Result<Vec<String>> {
        if self.resolution_service.is_none() {
            let resolver = PlatformTypeResolver::instance(); // Fallback нарушает архитектуру
            return Ok(resolver.get_completions(query).into_iter().map(|c| c.label).collect());
        }
        // ...
    }
}
```

### 3. TypeResolutionService неполноценен
```rust
// ❌ ПРОБЛЕМА: TypeResolutionService не покрывает все случаи использования
// Отсутствуют методы:
// - get_all_platform_globals()
// - get_completions()  
// - resolve_expression() (async версия)
// Из-за этого CentralTypeSystem и WebTypeService вынуждены обращаться к синглтону
```

## Правильная целевая архитектура

### Роли координатора системы:
```
CentralTypeSystem (System Coordinator)
├── 🏭 IoC Container: создаёт и связывает зависимости
├── 🎛️ System Coordinator: управляет инициализацией слоёв  
├── 🏥 Health Monitor: проверяет состояние компонентов
├── 📊 Performance Monitor: собирает системные метрики
├── 🔄 Lifecycle Manager: управляет перезагрузкой данных
└── 🏗️ Facade Provider: предоставляет интерфейсы (LSP/Web/CLI)
```

### Правила доступа к слоям:
```
System Layer (CentralTypeSystem)
    ↓ РАЗРЕШЕНО: владеет и координирует все слои
    ↓ ЧЕРЕЗ: self.resolution_service (НЕ через синглтон)
    
Application Layer (WebTypeService, LspTypeService)  
    ↓ ЗАПРЕЩЕНО: прямой доступ к Domain resolvers
    ↓ ТОЛЬКО: через TypeResolutionService (обязательная зависимость)
    
Domain Layer (TypeResolutionService - единственный фасад)
    ↓ РАЗРЕШЕНО: доступ к Data Layer
    ↓ ИНКАПСУЛЯЦИЯ: все resolvers приватны
    
Data Layer (TypeRepository, Loaders)
```

## План исправления (v2.0)

### Этап 1: Усилить TypeResolutionService

#### 1.1 Добавить недостающие методы
```rust
// src/domain/repository.rs
impl TypeResolutionService {
    /// Получить все платформенные типы (для CentralTypeSystem)
    pub fn get_all_platform_globals(&self) -> &HashMap<String, TypeResolution> {
        let resolver = PlatformTypeResolver::instance();
        resolver.get_platform_globals()
    }
    
    /// Получить автодополнения для запроса (для WebTypeService)
    pub fn get_completions(&self, query: &str) -> Vec<CompletionItem> {
        let resolver = PlatformTypeResolver::instance();
        resolver.get_completions(query)
    }
    
    /// Асинхронное разрешение выражений (для CentralTypeSystem)
    pub async fn resolve_expression_async(&self, expression: &str) -> TypeResolution {
        // Используем thread-safe версию или токио task
        let expr = expression.to_string();
        tokio::task::spawn_blocking(move || {
            let mut resolver = PlatformTypeResolver::new();
            resolver.resolve_expression(&expr)
        }).await.unwrap_or_else(|_| TypeResolution::unknown())
    }
    
    /// Поиск типов по запросу (для WebTypeService)
    pub fn search_types(&self, query: &str) -> Vec<String> {
        let completions = self.get_completions(query);
        completions.into_iter().map(|c| c.label).collect()
    }
    
    /// Получить информацию о конкретном типе (для CentralTypeSystem)
    pub fn get_type_info(&self, type_name: &str) -> Option<crate::system::coordination::TypeInfo> {
        let platform_globals = self.get_all_platform_globals();
        let resolution = platform_globals.get(type_name)?;
        
        if !matches!(resolution.certainty, crate::domain::types::Certainty::Unknown) {
            Some(crate::system::coordination::TypeInfo {
                name: type_name.to_string(),
                category: format!("{:?}", resolution.result),
                description: resolution.metadata.notes.first().cloned(),
                methods: Vec::new(), // TODO: реализовать через анализ metadata
                properties: Vec::new(), // TODO: реализовать через анализ metadata
                constructors: Vec::new(), // TODO: добавить когда будет готово
            })
        } else {
            None
        }
    }
}
```

#### 1.2 Исправить проблему мутабельности  
```rust
// Добавить immutable методы в PlatformTypeResolver для использования из синглтона
impl PlatformTypeResolver {
    /// Версия resolve_expression без записи в кэш (для синглтона)
    pub(crate) fn resolve_expression_immutable(&self, expression: &str) -> TypeResolution {
        // Логика разрешения без изменения self.cache
        // Только чтение из существующего кэша + fallback на новые вычисления
    }
}
```

### Этап 2: Исправить CentralTypeSystem (использовать зависимости)

#### 2.1 Заменить прямые вызовы синглтона
```rust
// src/system/coordination.rs
impl CentralTypeSystem {
    /// Использовать собственный resolution_service вместо синглтона
    pub async fn get_all_types_with_resolutions(&self) -> HashMap<String, TypeResolution> {
        // ✅ ПРАВИЛЬНО: через свою зависимость
        self.resolution_service.get_all_platform_globals().clone()
    }
    
    /// Делегировать разрешение выражений своему сервису
    pub async fn resolve_expression(&self, expression: &str) -> TypeResolution {
        // ✅ ПРАВИЛЬНО: через свою зависимость  
        self.resolution_service.resolve_expression_async(expression).await
    }
    
    /// Делегировать получение информации о типе
    pub async fn get_type_info(&self, type_name: &str) -> Option<TypeInfo> {
        // ✅ ПРАВИЛЬНО: через свою зависимость
        self.resolution_service.get_type_info(type_name)
    }
    
    /// Делегировать поиск типов
    pub async fn search_types(&self, query: &str) -> Vec<String> {
        // ✅ ПРАВИЛЬНО: через свою зависимость
        self.resolution_service.search_types(query)
    }
    
    /// Делегировать разрешение переменных (пока упрощённо)
    pub async fn get_variable_type(&self, variable_name: &str, _context: &str) -> TypeResolution {
        // ✅ ПРАВИЛЬНО: через свою зависимость
        self.resolution_service.resolve_expression_async(variable_name).await
    }
}
```

#### 2.2 Убрать создание новых экземпляров PlatformTypeResolver
```rust
// ❌ УДАЛИТЬ такие строки:
let mut platform_resolver = PlatformTypeResolver::new();

// ❌ УДАЛИТЬ такие строки:  
let platform_resolver = PlatformTypeResolver::instance();

// ✅ ЗАМЕНИТЬ на:
self.resolution_service.method_name()
```

### Этап 3: Принудительно изолировать Application Layer

#### 3.1 Сделать resolvers приватными (только для Application слоя)
```rust
// src/domain/mod.rs
pub mod repository;    // Публичный - единственный фасад для Application Layer
pub mod types;         // Публичный - общие типы и структуры  
pub mod standard_types;

mod resolvers;         // ПРИВАТНЫЙ - недоступен для Application Layer
mod search;            // ПРИВАТНЫЙ - недоступен для Application Layer

// Экспортируем ТОЛЬКО официальные фасады
pub use repository::TypeResolutionService;
pub use types::*;
```

#### 3.2 Ограничить видимость PlatformTypeResolver
```rust
// src/domain/resolvers/platform.rs
pub(crate) struct PlatformTypeResolver {
    // Доступен только внутри domain crate
}

impl PlatformTypeResolver {
    pub(super) fn new() -> Self {
        // Только для родительского модуля (resolvers)
    }
    
    pub(crate) fn instance() -> &'static Self {
        // Только внутри domain crate (для TypeResolutionService)
    }
    
    // Все остальные методы pub(crate)
}
```

#### 3.3 Убрать все импорты resolvers из Application Layer  
```rust
// src/application/web_service.rs
// ❌ УДАЛИТЬ (не скомпилируется после приватизации):
use crate::domain::resolvers::platform::PlatformTypeResolver;

// ✅ РАЗРЕШЕНЫ только эти импорты:
use crate::domain::repository::TypeResolutionService;
use crate::domain::types::TypeResolution;
```

### Этап 4: Исправить WebTypeService

#### 4.1 Сделать TypeResolutionService обязательным
```rust
// src/application/web_service.rs
pub struct WebTypeService {
    // Убираем Option - зависимость ВСЕГДА обязательна
    resolution_service: Arc<TypeResolutionService>,
}

impl WebTypeService {
    // Убираем new() без параметров
    
    pub fn new(resolution_service: Arc<TypeResolutionService>) -> Self {
        Self { resolution_service }
    }
    
    // УДАЛЯЕМ with_resolution_service() - теперь только один конструктор
}
```

#### 4.2 Убрать все fallback'и к синглтону
```rust
impl WebTypeService {
    pub async fn search_types(&self, query: &str) -> Result<Vec<String>> {
        // ❌ УДАЛИТЬ весь fallback блок:
        // if self.resolution_service.is_none() { ... }
        
        // ✅ ТОЛЬКО через зависимость:
        let types = self.resolution_service.search_types(query);
        Ok(types)
    }
    
    pub async fn get_completions(&self, query: &str) -> Result<Vec<CompletionItem>> {
        // ✅ ТОЛЬКО через зависимость:
        let completions = self.resolution_service.get_completions(query);
        Ok(completions)
    }
    
    // Аналогично для всех остальных методов - убрать fallback'и
}
```

#### 4.3 Обновить конструктор в CentralTypeSystem
```rust
// src/system/coordination.rs
impl CentralTypeSystem {
    pub fn new(config: CentralSystemConfig) -> Self {
        // ...
        
        // ✅ Обновить создание WebTypeService
        let web_service = Arc::new(WebTypeService::new(resolution_service.clone()));
        
        // ...
    }
}
```

## Этапы внедрения (исправленные)

### Фаза A: Подготовительная (безопасные изменения)
1. **Усилить TypeResolutionService** - добавить все недостающие методы
2. **Решить проблему мутабельности** - добавить immutable версии  
3. **Протестировать новые методы** - убедиться что они работают корректно

### Фаза B: Рефакторинг CentralTypeSystem (breaking changes для System Layer)
1. **Заменить все прямые вызовы синглтона** на self.resolution_service
2. **Убрать создание новых экземпляров** PlatformTypeResolver  
3. **Протестировать координатор** - убедиться что все функции работают

### Фаза C: Изоляция Application Layer (breaking changes для Application слоя)
1. **Приватизировать resolvers модуль** - сделать его недоступным
2. **Исправить ошибки компиляции** в WebTypeService и LspTypeService
3. **Убрать fallback'и** - оставить только dependency injection

### Фаза D: Финальная проверка
1. **Интеграционное тестирование** - все слои работают корректно
2. **Проверка производительности** - нет деградации  
3. **Проверка веб-сервера** - hierarchy строится правильно

## Ключевые принципы (исправленные)

### ✅ Что разрешено:
```rust
// System Layer может обращаться к Domain Layer (он им владеет)
CentralTypeSystem → self.resolution_service.method() ✅

// Domain Layer может использовать внутренние resolvers  
TypeResolutionService → PlatformTypeResolver::instance() ✅

// Application Layer может использовать только фасады
WebTypeService → self.resolution_service.method() ✅
```

### ❌ Что запрещено:
```rust
// Application Layer НЕ может обращаться к resolvers напрямую
WebTypeService → PlatformTypeResolver::instance() ❌  (не скомпилируется)

// System Layer НЕ должен создавать новые экземпляры или обращаться к синглтону
CentralTypeSystem → PlatformTypeResolver::new() ❌ (неэффективно)
CentralTypeSystem → PlatformTypeResolver::instance() ❌ (обходит DI)

// Никто НЕ должен иметь fallback'и  
if service.is_none() { fallback_to_singleton() } ❌ (нарушает DI)
```

## Ожидаемые результаты

### 🏗️ Архитектурная чистота:
- **CentralTypeSystem сохраняет роль координатора** но использует правильные зависимости
- **Application Layer принудительно изолирован** через систему типов  
- **TypeResolutionService становится полноценным фасадом** со всеми методами
- **Dependency Injection строго соблюдается** - никаких fallback'ов

### 🚀 Производительность:
- **Нет дублирования экземпляров** PlatformTypeResolver
- **Переиспользование данных** через единый resolution_service  
- **Эффективное кэширование** в синглтоне через TypeResolutionService

### 🧪 Тестируемость:
- **CentralTypeSystem тестируется с mock зависимостями**
- **Application сервисы легко изолируются** для unit тестов
- **Domain Layer остаётся чистым** и независимым

## Критерии успеха (исправленные)

1. ✅ **CentralTypeSystem использует self.resolution_service** вместо синглтона
2. ✅ **Application Layer НЕ может импортировать resolvers** (ошибка компиляции)  
3. ✅ **TypeResolutionService покрывает все use cases** - нет нужды в обходах
4. ✅ **Веб-сервер продолжает работать** с правильной иерархией типов
5. ✅ **Производительность не хуже** текущей (возможно лучше)
6. ✅ **Координирующая роль CentralTypeSystem сохранена** и улучшена

## Отличия от v1.0

### ❌ Что было неправильно в v1.0:
- Попытка запретить CentralTypeSystem доступ к Domain Layer
- Непонимание роли координатора как IoC контейнера
- Попытка убрать get_all_types_with_resolutions() как "нарушение"

### ✅ Что исправлено в v2.0:  
- CentralTypeSystem остаётся координатором с доступом к Domain Layer
- Проблема в СПОСОБЕ доступа (через синглтон) а не в самом доступе
- TypeResolutionService усиливается для покрытия всех случаев использования
- Изоляция только для Application/Presentation слоёв, не для System Layer

---

*Документ создан: 28 августа 2025*  
*Статус: Планирование v2.0*  
*Заменяет: ARCHITECTURE_LAYERING_PLAN.md (v1.0)*
