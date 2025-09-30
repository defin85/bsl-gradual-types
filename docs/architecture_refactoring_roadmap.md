# 📋 Детальный план рефакторинга архитектуры BSL Type System

## 🎯 Цель рефакторинга
Привести фактическую реализацию в соответствие с архитектурной диаграммой из `simplified_architecture.md`, следуя принципу **Right-Sized Architecture**.

**Дата создания**: 2025-09-30
**Статус**: В планировании
**Приоритет**: Критический

---

## 🔴 Выявленные проблемы

### Критические архитектурные нарушения

1. **TypeResolver в Domain Layer, но выполняет Application Logic**
   - ❌ Содержит async методы
   - ❌ Оркестрация с HashMap
   - ❌ Фильтрация по query
   - ✅ Должен содержать только чистую бизнес-логику

2. **SyntaxHelperParser в shared, но это Infrastructure**
   - ❌ I/O операции (чтение файлов)
   - ❌ HTML парсинг (Infrastructure concern)
   - ❌ Многопоточность (rayon)
   - ✅ Должен быть в `backend/src/data/loaders/`

3. **Converters в shared, но это Adapters**
   - ❌ Адаптеры между слоями
   - ✅ Должны быть в `backend/src/data/adapters/`

4. **AnalysisEngine создает Infrastructure компоненты**
   - ❌ Управляет парсингом файлов
   - ❌ Выполняет I/O операции
   - ✅ Должен получать готовые зависимости

5. **TypeSystemService - тонкая обёртка без добавленной стоимости**
   - ❌ Просто делегирует вызовы
   - ✅ Должен содержать Application логику

6. **Web Handlers преобразуют Domain → DTO**
   - ❌ Presentation Layer содержит бизнес-логику
   - ✅ Должно быть в Application Layer

---

## 📦 Phase 1: Перемещение Infrastructure компонентов (Критично)

### 1.1 Переместить SyntaxHelperParser

**Текущее положение**: `shared/src/loaders/syntax_helper_parser.rs`
**Целевое положение**: `backend/src/data/loaders/syntax_helper_parser.rs`

**Действия**:
```bash
# 1. Создать структуру директорий
mkdir -p backend/src/data/loaders
mkdir -p backend/src/data/adapters

# 2. Переместить файл
git mv shared/src/loaders/syntax_helper_parser.rs backend/src/data/loaders/

# 3. Обновить mod.rs
# backend/src/data/loaders/mod.rs
```

**Изменения в коде**:
```rust
// backend/src/data/loaders/mod.rs
pub mod syntax_helper_parser;

pub use syntax_helper_parser::{
    SyntaxHelperParser,
    SyntaxHelperDatabase,
    TypeInfo,
    // ... остальные экспорты
};
```

**Обновить импорты в**:
- `backend/src/application/type_system_service.rs`
- `shared/src/engine.rs` (временно, пока не переместим инициализацию)
- Все тесты, использующие SyntaxHelperParser

**Критерии успеха**:
- ✅ `cargo build -p bsl-backend` компилируется
- ✅ `shared` крейт больше не содержит I/O код
- ✅ Все тесты парсинга проходят

**Приоритет**: 🔴 Критический
**Оценка времени**: 2-3 часа

---

### 1.2 Переместить ConfigParser

**Текущее положение**: `shared/src/loaders/config_parser_guided_discovery.rs`
**Целевое положение**: `backend/src/data/loaders/config_parser.rs`

**Действия**:
```bash
git mv shared/src/loaders/config_parser_guided_discovery.rs \
       backend/src/data/loaders/config_parser.rs
```

**Изменения в коде**:
```rust
// backend/src/data/loaders/config_parser.rs
// Переименовать структуры для единообразия:
// ConfigurationGuidedParser → ConfigParser
// DiscoveredMetadata → ConfigMetadata
```

**Критерии успеха**:
- ✅ `cargo build -p bsl-backend` компилируется
- ✅ Парсинг конфигураций работает корректно

**Приоритет**: 🔴 Критический
**Оценка времени**: 1-2 часа

---

### 1.3 Переместить Converters (Adapters)

**Текущее положение**: `shared/src/loaders/converters.rs`
**Целевое положение**: `backend/src/data/adapters/converters.rs`

**Действия**:
```bash
mkdir -p backend/src/data/adapters
git mv shared/src/loaders/converters.rs backend/src/data/adapters/
```

**Изменения в коде**:
```rust
// backend/src/data/adapters/converters.rs
use crate::data::loaders::{SyntaxHelperDatabase, ConfigMetadata};
use bsl_shared::domain::types::RawTypeData;

pub fn convert_syntax_helper_to_raw(db: &SyntaxHelperDatabase) -> Vec<RawTypeData> {
    // Существующая логика
}

pub fn convert_config_metadata_to_raw(metadata: &[ConfigMetadata]) -> Vec<RawTypeData> {
    // Существующая логика
}
```

**Критерии успеха**:
- ✅ Адаптеры находятся в правильном слое
- ✅ Чистое разделение между Infrastructure и Domain

**Приоритет**: 🔴 Критический
**Оценка времени**: 1 час

---

## 🧠 Phase 2: Рефакторинг Domain Layer (Критично)

### 2.1 Очистить TypeResolver от Application логики

**Текущее положение**: `shared/src/domain/resolver.rs` (содержит Application логику)
**Целевое состояние**: Чистая Domain логика без async, HashMap, фильтрации

**Создать новый чистый TypeResolver**:
```rust
// shared/src/domain/resolver.rs
use crate::domain::repository::TypeRepository;
use crate::domain::types::{TypeResolution, ConcreteType, ResolutionContext};

/// Чистый Domain resolver - только бизнес-логика типизации
pub struct TypeResolver {
    repository: Arc<dyn TypeRepository>,
}

impl TypeResolver {
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        Self { repository }
    }

    /// Основной метод резолвинга - чистая бизнес-логика
    pub fn resolve_type(
        &self,
        type_name: &str,
        context: &ResolutionContext
    ) -> TypeResolution {
        // 1. Поиск в repository
        if let Some(raw_type) = self.repository.find_type(type_name) {
            return self.create_resolution_from_raw(&raw_type, context);
        }

        // 2. Анализ составных типов (Справочники.Контрагенты)
        if let Some(resolution) = self.resolve_qualified_name(type_name, context) {
            return resolution;
        }

        // 3. Union type анализ
        if type_name.contains(',') {
            return self.resolve_union_type(type_name, context);
        }

        TypeResolution::unknown()
    }

    /// Сужение типа на основе условия (flow-sensitive анализ)
    pub fn narrow_type(
        &self,
        current: &TypeResolution,
        condition: &TypeCondition,
    ) -> TypeResolution {
        // Чистая логика сужения типов
        match condition {
            TypeCondition::TypeCheck(expected) => {
                // Если ТипЗнч(x) = Тип("Строка")
                self.narrow_to_concrete(current, expected)
            }
            TypeCondition::NotUndefined => {
                // Если x <> Неопределено
                self.remove_undefined(current)
            }
            // ... другие условия
        }
    }

    /// Преобразование RawTypeData в TypeResolution (чистая логика)
    fn create_resolution_from_raw(
        &self,
        raw: &RawTypeData,
        context: &ResolutionContext,
    ) -> TypeResolution {
        let concrete_type = match raw.source {
            RawDataSource::Platform => {
                ConcreteType::Platform(PlatformType {
                    name: raw.name.clone()
                })
            }
            RawDataSource::Configuration => {
                ConcreteType::Configuration(ConfigurationType {
                    kind: raw.kind.unwrap_or_default(),
                    name: raw.name.clone(),
                })
            }
        };

        TypeResolution::known(concrete_type)
            .with_facets(raw.facets.clone())
            .with_active_facet(self.infer_facet_from_context(context))
    }

    // ... остальные ЧИСТЫЕ методы
}

/// Контекст для резолвинга типов
pub struct ResolutionContext {
    pub in_assignment: bool,
    pub expected_type: Option<String>,
    pub scope_variables: HashMap<String, TypeResolution>,
}

/// Условие для сужения типов
pub enum TypeCondition {
    TypeCheck(String),
    NotUndefined,
    NotNull,
    IsEmpty,
    IsNotEmpty,
}
```

**Критерии успеха**:
- ✅ Нет async методов
- ✅ Нет HashMap в публичном API (только внутри логики)
- ✅ Нет фильтрации и поиска (это Application Layer)
- ✅ Только чистые функции преобразования типов

**Приоритет**: 🔴 Критический
**Оценка времени**: 4-6 часов

---

### 2.2 Создать Application Service для оркестрации

**Новый файл**: `backend/src/application/type_resolution_service.rs`

```rust
// backend/src/application/type_resolution_service.rs
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::TypeResolution;
use std::collections::HashMap;
use std::sync::Arc;

/// Application Layer сервис для работы с типами
/// Содержит оркестрацию, кэширование, async операции
pub struct TypeResolutionService {
    resolver: Arc<TypeResolver>,
    cache: Arc<AnalysisCache>,
}

impl TypeResolutionService {
    pub fn new(resolver: Arc<TypeResolver>, cache: Arc<AnalysisCache>) -> Self {
        Self { resolver, cache }
    }

    /// Получить все платформенные типы (оркестрация + преобразование)
    pub fn get_all_platform_types(&self) -> HashMap<String, TypeResolution> {
        // 1. Получаем сырые данные из repository
        let raw_types = self.resolver.repository().get_all_types();

        // 2. Преобразуем через resolver
        let mut result = HashMap::new();
        let context = ResolutionContext::default();

        for raw_type in raw_types {
            let resolution = self.resolver.resolve_type(&raw_type.name, &context);
            result.insert(raw_type.name, resolution);
        }

        result
    }

    /// Поиск типов с фильтрацией (Application логика)
    pub fn search_types(&self, query: &str) -> Vec<TypeResolution> {
        let all_types = self.get_all_platform_types();

        all_types
            .into_iter()
            .filter(|(name, _)| {
                name.to_lowercase().contains(&query.to_lowercase())
            })
            .map(|(_, resolution)| resolution)
            .collect()
    }

    /// Async резолвинг для web/LSP контекстов
    pub async fn resolve_expression_async(
        &self,
        expression: &str,
        context: ResolutionContext,
    ) -> TypeResolution {
        // Проверка кэша
        if let Some(cached) = self.cache.get_resolution(expression) {
            return cached;
        }

        // Резолвинг через Domain Layer
        let resolution = self.resolver.resolve_type(expression, &context);

        // Кэширование результата
        self.cache.store_resolution(expression, &resolution);

        resolution
    }

    /// Получить автодополнения (Application логика)
    pub fn get_completions(&self, query: &str) -> Vec<CompletionItem> {
        let types = self.search_types(query);

        types
            .into_iter()
            .map(|resolution| self.resolution_to_completion_item(&resolution))
            .collect()
    }

    // Преобразование Domain → DTO (Application логика)
    fn resolution_to_completion_item(&self, resolution: &TypeResolution) -> CompletionItem {
        CompletionItem {
            label: format!("{:?}", resolution.result),
            kind: self.determine_completion_kind(resolution),
            detail: Some(format!("Certainty: {:?}", resolution.certainty)),
            documentation: resolution.metadata.notes.first().cloned(),
        }
    }

    fn determine_completion_kind(&self, resolution: &TypeResolution) -> CompletionKind {
        // Логика определения типа автодополнения
        match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Platform(_)) => CompletionKind::Global,
            ResolutionResult::Concrete(ConcreteType::Configuration(config)) => {
                match config.kind {
                    MetadataKind::Catalog => CompletionKind::Catalog,
                    MetadataKind::Document => CompletionKind::Document,
                    _ => CompletionKind::Global,
                }
            }
            _ => CompletionKind::Global,
        }
    }
}
```

**Критерии успеха**:
- ✅ Application логика (фильтрация, поиск, async) вынесена из Domain
- ✅ TypeResolver остается чистым
- ✅ Четкое разделение ответственности

**Приоритет**: 🔴 Критический
**Оценка времени**: 3-4 часа

---

## 🚀 Phase 3: Упрощение AnalysisEngine (Критично)

### 3.1 Убрать Infrastructure инициализацию из AnalysisEngine

**Текущая проблема**: `AnalysisEngine::new_with_init` создает парсеры и выполняет I/O

**Новая архитектура**:

```rust
// shared/src/engine.rs
use crate::domain::resolver::{TypeResolver, ResolutionContext};
use crate::domain::repository::TypeRepository;
use crate::domain::types::{TypeResolution, RawTypeData};
use std::sync::Arc;

/// Чистый оркестратор анализа файлов
/// НЕ зависит от Infrastructure - только от готовых компонентов
pub struct AnalysisEngine {
    resolver: Arc<TypeResolver>,
}

impl AnalysisEngine {
    /// Создание с готовым resolver (без I/O)
    pub fn new(resolver: Arc<TypeResolver>) -> Self {
        Self { resolver }
    }

    /// Анализ содержимого файла (получает готовые данные)
    pub async fn analyze_content(
        &self,
        content: &str,
        ast: Option<&ParseTree>, // Получает готовый AST
    ) -> AnalysisResult {
        let start = std::time::Instant::now();
        let mut resolutions = HashMap::new();

        // Use Case: "Analyze File"
        // 1. Извлекаем переменные из AST (если есть)
        let variables = self.extract_variables_from_ast(ast, content);

        // 2. Резолвим типы для каждой переменной
        let context = ResolutionContext::default();
        for (var_name, type_hint) in variables {
            let resolution = self.resolver.resolve_type(&type_hint, &context);
            resolutions.insert(var_name, resolution);
        }

        AnalysisResult {
            type_resolutions: resolutions,
            analysis_duration_ms: start.elapsed().as_millis(),
        }
    }

    /// Получить resolver для прямого использования
    pub fn resolver(&self) -> &Arc<TypeResolver> {
        &self.resolver
    }

    // Вспомогательные методы для извлечения данных из AST
    fn extract_variables_from_ast(
        &self,
        ast: Option<&ParseTree>,
        content: &str,
    ) -> Vec<(String, String)> {
        // Простая эвристика для извлечения переменных
        // TODO: Использовать настоящий AST парсинг
        vec![]
    }
}

/// Результат анализа (Domain модель)
pub struct AnalysisResult {
    pub type_resolutions: HashMap<String, TypeResolution>,
    pub analysis_duration_ms: u128,
}
```

**Критерии успеха**:
- ✅ `AnalysisEngine` не создает Infrastructure компоненты
- ✅ Получает готовые данные (AST, resolver)
- ✅ Может работать в любом окружении (CLI, Web, LSP)

**Приоритет**: 🔴 Критический
**Оценка времени**: 3-4 часа

---

### 3.2 Переместить инициализацию в SystemCoordinator

**Обновить**: `backend/src/system/system_coordinator.rs`

```rust
// backend/src/system/system_coordinator.rs
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use crate::data::loaders::{SyntaxHelperParser, ConfigParser};
use crate::data::adapters::converters;

impl SystemCoordinator {
    /// Инициализация с полным контролем Infrastructure
    pub async fn start_with_paths(
        &self,
        syntax_helper_path: Option<&Path>,
        config_path: Option<&Path>,
    ) -> Result<(), StartupError> {
        info!("🎯 SystemCoordinator: инициализация системы...");

        // === INFRASTRUCTURE LAYER ===
        // 1. Парсинг синтаксис-помощника
        let mut syntax_parser = SyntaxHelperParser::new();
        if let Some(path) = syntax_helper_path {
            syntax_parser.parse_syntax_helper(path)?;
        }
        let syntax_db = syntax_parser.export_database();

        // 2. Парсинг конфигурации
        let mut config_parser = ConfigParser::new();
        if let Some(path) = config_path {
            config_parser.parse_configuration(path)?;
        }
        let config_metadata = config_parser.export_metadata();

        // === DATA LAYER ===
        // 3. Адаптеры - преобразование в Domain формат
        let platform_types = converters::convert_syntax_helper_to_raw(&syntax_db);
        let config_types = converters::convert_config_metadata_to_raw(&config_metadata);

        // 4. Загрузка в Repository
        let repository = Arc::new(InMemoryTypeRepository::new());
        repository.load_types(platform_types)?;
        repository.load_types(config_types)?;

        info!("📊 Загружено {} типов", repository.get_stats().total_types);

        // === DOMAIN LAYER ===
        // 5. Создание TypeResolver с готовым repository
        let resolver = Arc::new(TypeResolver::new(repository));

        // === APPLICATION LAYER ===
        // 6. Создание AnalysisEngine с готовым resolver
        let analysis_engine = Arc::new(AnalysisEngine::new(resolver.clone()));

        // 7. Создание TypeResolutionService
        let resolution_service = Arc::new(TypeResolutionService::new(
            resolver,
            self.cache.clone(),
        ));

        // 8. Создание TypeSystemService (высокоуровневый API)
        let type_service = Arc::new(TypeSystemService::new(
            analysis_engine,
            resolution_service,
            self.cache.clone(),
            self.parser.clone(),
        ));

        // Кэшируем созданные сервисы
        {
            let mut cache = self.type_service_cache.lock().unwrap();
            *cache = Some(type_service);
        }

        info!("💾 Прогрев кеша...");
        self.cache.warm_cache()?;

        info!("✅ SystemCoordinator: система готова!");
        Ok(())
    }
}
```

**Критерии успеха**:
- ✅ Все Infrastructure компоненты создаются в SystemCoordinator
- ✅ Четкий поток данных: Infrastructure → Data → Domain → Application
- ✅ AnalysisEngine получает готовые зависимости

**Приоритет**: 🔴 Критический
**Оценка времени**: 4-5 часов

---

## 🎭 Phase 4: Расширение TypeSystemService (Важно)

### 4.1 Добавить реальную бизнес-логику

**Обновить**: `backend/src/application/type_system_service.rs`

```rust
// backend/src/application/type_system_service.rs
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::api::dtos::{TypeDto, AnalysisResultDto, MetricsDto};
use crate::application::type_resolution_service::TypeResolutionService;

pub struct TypeSystemService {
    analysis_engine: Arc<AnalysisEngine>,
    resolution_service: Arc<TypeResolutionService>,
    cache: Arc<AnalysisCache>,
    parser: Arc<ParserCoordinator>,
}

impl TypeSystemService {
    pub fn new(
        analysis_engine: Arc<AnalysisEngine>,
        resolution_service: Arc<TypeResolutionService>,
        cache: Arc<AnalysisCache>,
        parser: Arc<ParserCoordinator>,
    ) -> Self {
        Self {
            analysis_engine,
            resolution_service,
            cache,
            parser,
        }
    }

    /// HIGH-LEVEL API: Получить все типы с преобразованием в DTO
    pub fn get_all_types_as_dto(
        &self,
        pagination: PaginationParams,
    ) -> AnalysisResultDto {
        // 1. Получаем Domain данные через resolution_service
        let all_types = self.resolution_service.get_all_platform_types();

        // 2. Применяем пагинацию
        let paginated_types: Vec<_> = all_types
            .iter()
            .skip(pagination.offset)
            .take(pagination.limit)
            .collect();

        // 3. Преобразуем Domain → DTO (Application логика)
        let type_dtos: Vec<TypeDto> = paginated_types
            .into_iter()
            .map(|(name, resolution)| self.convert_resolution_to_dto(name, resolution))
            .collect();

        // 4. Собираем метрики
        let metrics = self.calculate_metrics(&all_types);

        // 5. Собираем категории
        let categories = self.extract_categories(&all_types);

        // 6. Создаем pagination info
        let pagination_info = self.create_pagination_info(
            pagination,
            all_types.len(),
        );

        AnalysisResultDto {
            types: type_dtos,
            categories,
            metrics,
            connections: vec![], // TODO: Implement
            pagination: Some(pagination_info),
        }
    }

    /// Преобразование Domain → DTO (Application логика)
    fn convert_resolution_to_dto(
        &self,
        name: &str,
        resolution: &TypeResolution,
    ) -> TypeDto {
        // Определение категории на основе типа
        let category = match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Platform(_)) => "Platform",
            ResolutionResult::Concrete(ConcreteType::Configuration(_)) => "Configuration",
            _ => "Unknown",
        };

        // Расчет certainty
        let certainty = match resolution.certainty {
            Certainty::Known => 100,
            Certainty::Inferred(val) => (val * 100.0) as u8,
            Certainty::Unknown => 30,
        };

        // Извлечение union types
        let union_types = self.extract_union_types(resolution);

        TypeDto {
            id: name.to_string(),
            name: name.to_string(),
            category: category.to_string(),
            certainty,
            certainty_text: format!("{:?} {}%", resolution.certainty, certainty),
            facets: resolution.available_facets
                .iter()
                .map(|f| format!("{:?}", f))
                .collect(),
            source: format!("{:?}", resolution.source),
            flow_sensitive: resolution.metadata.flow_sensitive,
            description: self.generate_type_description(resolution),
            union_types,
            methods_count: None, // TODO
            methods: vec![],     // TODO
            attributes_count: None,
            flow_analysis: None,
            connections: None,
            warning: None,
            recommendation: None,
        }
    }

    /// Расчет метрик (Application логика)
    fn calculate_metrics(
        &self,
        types: &HashMap<String, TypeResolution>,
    ) -> MetricsDto {
        let mut high = 0;
        let mut medium = 0;
        let mut low = 0;
        let mut flow_sensitive = 0;

        for resolution in types.values() {
            let certainty = match resolution.certainty {
                Certainty::Known => 100,
                Certainty::Inferred(val) => (val * 100.0) as u8,
                Certainty::Unknown => 30,
            };

            if certainty > 80 {
                high += 1;
            } else if certainty > 40 {
                medium += 1;
            } else {
                low += 1;
            }

            if resolution.metadata.flow_sensitive {
                flow_sensitive += 1;
            }
        }

        MetricsDto {
            total_types: types.len(),
            certainty_high: high,
            certainty_medium: medium,
            certainty_low: low,
            flow_sensitive,
            cache_hit_rate: self.cache.get_hit_rate(),
            analysis_speed: format!("{}ms", self.get_avg_analysis_speed()),
        }
    }

    // ... остальные Application методы
}
```

**Критерии успеха**:
- ✅ TypeSystemService содержит реальную бизнес-логику
- ✅ Преобразование Domain → DTO происходит в Application Layer
- ✅ Не просто делегирует, а добавляет ценность

**Приоритет**: 🟡 Важно
**Оценка времени**: 4-5 часов

---

## 🌐 Phase 5: Упрощение Presentation Layer (Важно)

### 5.1 Упростить Web Handlers

**Обновить**: `backend/src/presentation/web/handlers.rs`

```rust
// backend/src/presentation/web/handlers.rs
use axum::{extract::{Query, State}, response::Json};
use crate::application::TypeSystemService;

/// ТОНКИЙ handler - только routing и HTTP
pub async fn get_types(
    State(state): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Json<AnalysisResultDto> {
    // 1. Преобразование HTTP параметров
    let pagination = PaginationParams {
        limit: params.limit.unwrap_or(50).min(1000),
        offset: params.offset.unwrap_or(0),
    };

    // 2. Делегирование Application Layer
    let result = state.type_service.get_all_types_as_dto(pagination);

    // 3. Возврат результата
    Json(result)
}

/// Поиск типов
pub async fn search_types(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    match state.type_service.search_types_as_dto(&query.q).await {
        Ok(results) => Json(results).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Health check
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "bsl-gradual-types"
    }))
}
```

**Критерии успеха**:
- ✅ Handler < 10 строк кода
- ✅ Нет бизнес-логики в Presentation Layer
- ✅ Только routing и HTTP конвертация

**Приоритет**: 🟡 Важно
**Оценка времени**: 2-3 часа

---

## 🧪 Phase 6: Тестирование и валидация

### 6.1 Unit тесты для каждого слоя

```rust
// shared/src/domain/resolver_tests.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_platform_type() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        // Загружаем тестовые типы

        let resolver = TypeResolver::new(repo);
        let context = ResolutionContext::default();

        let resolution = resolver.resolve_type("Строка", &context);

        assert!(matches!(resolution.certainty, Certainty::Known));
        assert!(matches!(resolution.result, ResolutionResult::Concrete(_)));
    }

    #[test]
    fn test_narrow_type_from_union() {
        let resolver = TypeResolver::new(mock_repository());
        let union = TypeResolution::union(vec![
            WeightedType::new(ConcreteType::string(), 0.5),
            WeightedType::new(ConcreteType::number(), 0.5),
        ]);

        let narrowed = resolver.narrow_type(
            &union,
            &TypeCondition::TypeCheck("Строка".to_string()),
        );

        assert!(matches!(narrowed.result, ResolutionResult::Concrete(_)));
    }
}
```

**Приоритет**: 🟡 Важно
**Оценка времени**: 6-8 часов

---

### 6.2 Integration тесты

```rust
// backend/tests/full_pipeline_test.rs
#[tokio::test]
async fn test_full_analysis_pipeline() {
    // 1. Инициализация SystemCoordinator
    let coordinator = SystemCoordinator::new();
    coordinator.start_with_paths(
        Some(Path::new("tests/fixtures/syntax_helper")),
        None,
    ).await.unwrap();

    // 2. Получение TypeSystemService
    let service = coordinator.type_service().unwrap();

    // 3. Запрос всех типов
    let result = service.get_all_types_as_dto(PaginationParams {
        limit: 10,
        offset: 0,
    });

    // 4. Проверки
    assert!(result.types.len() > 0);
    assert!(result.metrics.total_types > 0);
    assert!(result.pagination.is_some());
}
```

**Приоритет**: 🟡 Важно
**Оценка времени**: 4-6 часов

---

### 6.3 E2E тесты веб-интерфейса

```bash
# Запуск backend
cargo run -p bsl-backend --bin bsl-web-server -- --port 3001

# Проверка API
curl "http://localhost:3001/api/types?limit=5&offset=0"
curl "http://localhost:3001/api/search?q=Массив"
curl "http://localhost:3001/api/health"

# Проверка frontend
# Открыть http://localhost:3001 в браузере
```

**Приоритет**: 🟢 Среднее
**Оценка времени**: 3-4 часа

---

## 📊 Итоговая архитектура после рефакторинга

```
bsl-gradual-types/
├── shared/                    # ✅ Только Domain + чистые типы
│   ├── domain/
│   │   ├── resolver.rs       # ✅ Чистая бизнес-логика типизации
│   │   ├── repository.rs     # ✅ Абстракция хранилища
│   │   └── types.rs          # ✅ Domain модели
│   ├── engine.rs              # ✅ Чистый оркестратор анализа
│   └── api/dtos.rs            # ✅ Контракты API
│
├── backend/
│   ├── system/                # ✅ System Layer
│   │   ├── system_coordinator.rs  # ✅ DI + Lifecycle
│   │   ├── simple_cache.rs        # ✅ LRU кэш
│   │   ├── parser_coordinator.rs  # ✅ TreeSitter + fallback
│   │   └── basic_observability.rs # ✅ Логирование
│   │
│   ├── application/           # ✅ Application Layer
│   │   ├── type_system_service.rs      # ✅ High-level API
│   │   └── type_resolution_service.rs  # ✅ Оркестрация + кэш
│   │
│   ├── presentation/          # ✅ Presentation Layer (тонкий)
│   │   ├── web/handlers.rs    # ✅ Только routing
│   │   └── lsp/server.rs      # ✅ LSP protocol
│   │
│   └── data/                  # ✅ Infrastructure Layer
│       ├── loaders/
│       │   ├── syntax_helper_parser.rs  # ✅ HTML parsing
│       │   └── config_parser.rs          # ✅ XML parsing
│       └── adapters/
│           └── converters.rs             # ✅ Адаптеры между слоями
│
├── frontend/                  # ✅ Presentation (WASM)
└── cli/                       # ✅ Presentation (CLI)
```

---

## ✅ Чеклист выполнения

### Phase 1: Infrastructure компоненты (🔴 Критично)
- [ ] Переместить SyntaxHelperParser → backend/data/loaders
- [ ] Переместить ConfigParser → backend/data/loaders
- [ ] Переместить Converters → backend/data/adapters
- [ ] Обновить все импорты
- [ ] Проверить компиляцию backend
- [ ] Запустить тесты парсеров

### Phase 2: Domain Layer (🔴 Критично)
- [ ] Очистить TypeResolver от Application логики
- [ ] Создать TypeResolutionService в Application Layer
- [ ] Убрать async методы из TypeResolver
- [ ] Убрать HashMap из публичного API TypeResolver
- [ ] Перенести фильтрацию и поиск в TypeResolutionService
- [ ] Запустить domain layer тесты

### Phase 3: AnalysisEngine (🔴 Критично)
- [ ] Убрать Infrastructure инициализацию из AnalysisEngine
- [ ] Переместить инициализацию в SystemCoordinator
- [ ] Обновить CLI для использования новой архитектуры
- [ ] Запустить integration тесты

### Phase 4: TypeSystemService (🟡 Важно)
- [ ] Добавить метод get_all_types_as_dto
- [ ] Реализовать convert_resolution_to_dto
- [ ] Реализовать calculate_metrics
- [ ] Реализовать extract_categories
- [ ] Добавить методы для работы с DTO

### Phase 5: Presentation Layer (🟡 Важно)
- [ ] Упростить get_types handler
- [ ] Упростить search_types handler
- [ ] Убрать бизнес-логику из handlers
- [ ] Обновить LSP handlers (если нужно)

### Phase 6: Тестирование (🟡 Важно)
- [ ] Написать unit тесты для TypeResolver
- [ ] Написать unit тесты для TypeResolutionService
- [ ] Написать integration тесты SystemCoordinator
- [ ] Написать E2E тесты API endpoints
- [ ] Протестировать веб-интерфейс
- [ ] Протестировать CLI

### Финальная проверка
- [ ] Все тесты проходят
- [ ] Компиляция без warnings
- [ ] Документация обновлена
- [ ] CLAUDE.md обновлен
- [ ] Architecture diagram соответствует реализации

---

## 📈 Метрики успеха

### Архитектурные метрики
- ✅ **Соответствие диаграмме**: 100% соответствие `simplified_architecture.md`
- ✅ **Чистота слоев**: Каждый компонент в правильном слое
- ✅ **Зависимости**: Только однонаправленные зависимости (сверху вниз)

### Качественные метрики
- ✅ **Testability**: Каждый слой тестируется независимо
- ✅ **Maintainability**: Понятная структура для новых разработчиков
- ✅ **Scalability**: Возможность масштабирования по необходимости

### Производительность
- ✅ **Компиляция**: Время компиляции не увеличилось
- ✅ **Тесты**: Все тесты проходят
- ✅ **Runtime**: Производительность не деградировала

---

## 📝 Примечания

### Важные решения
1. **SyntaxHelperParser переносится в backend** - это Infrastructure, а не Domain
2. **TypeResolver очищается от async** - Domain должен быть синхронным
3. **AnalysisEngine упрощается** - не создает Infrastructure компоненты
4. **TypeSystemService расширяется** - добавляется реальная бизнес-логика

### Риски и mitigation
1. **Риск**: Сломать существующие тесты
   - **Mitigation**: Постепенное обновление тестов вместе с кодом

2. **Риск**: Увеличение времени компиляции
   - **Mitigation**: Мониторинг времени компиляции на каждом этапе

3. **Риск**: Регрессия функциональности
   - **Mitigation**: E2E тесты перед и после рефакторинга

### Следующие шаги после рефакторинга
1. Оптимизация производительности
2. Добавление новых features
3. Улучшение документации
4. Расширение тестового покрытия

---

## 📚 Ссылки

- [Simplified Architecture Diagram](./simplified_architecture.md)
- [CLAUDE.md](../CLAUDE.md)
- [README.md](../README.md)

---

**Дата последнего обновления**: 2025-09-30
**Версия документа**: 1.0
**Статус**: В планировании