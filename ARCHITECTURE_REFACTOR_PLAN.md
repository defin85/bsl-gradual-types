# План рефакторинга архитектуры BSL Gradual Types

**Дата создания**: 26 августа 2025 г.  
**Цель**: Привести проект к целевой архитектуре согласно документации в `docs/reference/target_architecture/`

## Анализ текущего состояния архитектуры

### ✅ **Что уже соответствует целевой архитектуре:**

#### **1. Плоская структура слоёв (правильная)**
```
src/data/        ✅ TypeSource, loaders/ (8 файлов) 
src/parsing/     ✅ bsl/, query/ 
src/domain/      ✅ types, resolvers, analysis, repository, resolution_service
src/application/ ✅ lsp_service, web_service, analysis_service 
src/presentation/✅ adapters, interfaces
src/system/      ✅ coordination (CentralTypeSystem), performance
```

#### **2. Правильно реализованные компоненты:**
- `CentralTypeSystem` в `src/system/coordination.rs` - ✅ есть координатор
- `TypeRepository` trait в `src/domain/repository.rs` - ✅ единый интерфейс данных
- `TypeResolutionService` в `src/domain/resolution_service.rs` - ✅ центральный сервис
- Специализированные сервисы в `src/application/` - ✅ разделение по назначению
- Загрузчики данных в `src/data/loaders/` - ✅ правильное место

### 🔴 **Проблемы требующие исправления:**

## **1. Legacy структуры (12 файлов на удаление)**
```bash
src/documentation/          # 🔴 УДАЛИТЬ - 12 файлов
├── core/                   # Перенести в application/documentation_service.rs  
├── platform/              # Интегрировать в domain/resolvers/platform.rs
├── search/                 # Перенести в application/search_service.rs
└── render/                 # Перенести в presentation/html_renderer.rs

src/adapters/mod.rs         # 🔴 УДАЛИТЬ - только re-export, дублирует data/loaders/
```

## **2. Дублирование типов между модулями**

#### **CompletionItem** (найдено 4+ определения):
- `src/domain/resolvers/platform.rs:14` ✅ **основная версия** 
- `src/presentation/adapters.rs:42` - дублирует с дополнительными полями
- `src/architecture/domain/mod.rs:630` - legacy версия
- `src/core/platform_resolver.rs:12` - legacy версия

#### **TypeHierarchy** (найдено 3+ определения):
- `src/application/services.rs:144` ✅ **основная версия**
- `src/application/web_service.rs:20` - дублирует 
- `src/documentation/core/hierarchy.rs:11` - legacy сложная версия
- `src/domain/search.rs:139` - ещё одна версия

## **3. Семантические несоответствия в интерфейсах**
- Методы возвращают `Result<Vec<T>>` но презентация ожидает `Vec<T>`
- `CompletionItem` не имеет полей `insert_text`, `filter_text`, `sort_text`
- `TypeHierarchy.categories` имеет тип `Vec<String>` но код ожидает `Vec<TypeCategory>`

## Обобщённый план доработок

### **ЭТАП 1: Очистка legacy (30 мин)**

#### **1.1 Удаление legacy папок**
```bash
# Удалить legacy папки
rm -rf src/documentation/         # 12 файлов
rm -rf src/adapters/             # 1 файл (только re-export)
```

#### **1.2 Обновление импортов (8 файлов)**
- `src/bin/web_server.rs` - заменить `use bsl_gradual_types::documentation::` на `use bsl_gradual_types::application::`
- `src/application/documentation_service.rs` - раскомментировать и исправить импорты 
- `src/domain/type_system_service.rs` - обновить закомментированные импорты

### **ЭТАП 2: Унификация типов (45 мин)**

#### **2.1 Единый CompletionItem**
**Цель**: Оставить одно определение в `src/domain/resolvers/platform.rs:14`

**Действия**:
```rust
// Расширить основную версию недостающими полями:
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    // Добавить недостающие поля для LSP
    pub insert_text: Option<String>,
    pub filter_text: Option<String>, 
    pub sort_text: Option<String>,
}
```

**Файлы на изменение**:
- ✅ Расширить `src/domain/resolvers/platform.rs:14`
- 🔴 Удалить дубликат в `src/presentation/adapters.rs:42`
- 🔴 Удалить legacy версии в `src/architecture/domain/mod.rs:630`, `src/core/platform_resolver.rs:12`
- 🔧 Обновить импорты во всех использующих файлах

#### **2.2 Единая TypeHierarchy**
**Цель**: Оставить одно определение в `src/application/services.rs:144`

**Действия**:
```rust
// Убедиться что основная версия полная:
#[derive(Debug, Default)]
pub struct TypeHierarchy {
    pub root_types: Vec<String>,
    pub categories: Vec<TypeCategory>,  // Не Vec<String>!
    pub statistics: HashMap<String, u32>,
    pub total_types: usize,
}

#[derive(Debug, Clone)]
pub struct TypeCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub types: Vec<String>,
    pub subcategories: Vec<TypeCategory>,
}
```

**Файлы на изменение**:
- ✅ Проверить полноту `src/application/services.rs:144`
- 🔴 Удалить дубликат в `src/application/web_service.rs:20`
- 🔴 Удалить legacy версии в `src/documentation/core/hierarchy.rs:11`, `src/domain/search.rs:139`
- 🔧 Обновить импорты и использование

#### **2.3 Унификация интерфейсов сервисов**
**Цель**: Привести сигнатуры методов к единому стилю

**Текущие проблемы**:
```rust
// Проблема: метод возвращает Result но презентация ожидает простой тип
pub async fn get_completions_fast(&self, ...) -> Result<Vec<CompletionItem>>  // 🔴
// Должно быть:
pub async fn get_completions_fast(&self, ...) -> Vec<CompletionItem>          // ✅

// Проблема: неправильное количество параметров  
pub async fn get_hover_info(&self, position: &str) -> Option<String>         // 🔴
// Должно быть:
pub async fn get_hover_info(&self, file_path: &str, line: u32, column: u32, expression: &str) -> Option<String>  // ✅
```

**Файлы на изменение**:
- `src/application/lsp_service.rs` - убрать лишние Result обёртки
- `src/application/web_service.rs` - привести сигнатуры к единому стилю  
- `src/presentation/adapters.rs` - обновить вызовы методов

### **ЭТАП 3: Интеграция функционала documentation (60 мин)**

#### **3.1 Создать DocumentationService**
```rust
// src/application/documentation_service.rs - раскомментировать и реализовать
pub struct DocumentationService {
    search_engine: DocumentationSearchEngine,
    html_renderer: HtmlRenderer,  
    platform_provider: PlatformDocumentationProvider,
    repository: Arc<dyn TypeRepository>,
}

impl DocumentationService {
    pub async fn search_types(&self, query: &str) -> SearchResults { ... }
    pub async fn render_type_page(&self, type_name: &str) -> String { ... }
    pub async fn build_hierarchy(&self) -> TypeHierarchy { ... }
}
```

#### **3.2 Перенести функционал по слоям**

**Из `src/documentation/search/` → `src/application/search_service.rs`**:
- `DocumentationSearchEngine` - основной движок поиска
- `SearchOptions`, `SearchResults` - типы для поиска  
- `FuzzyMatcher` - алгоритм поиска

**Из `src/documentation/render/` → `src/presentation/html_renderer.rs`**:
- `HtmlDocumentationRenderer` - рендеринг в HTML
- Шаблоны и стили - перенести в ресурсы

**Из `src/documentation/platform/` → `src/domain/resolvers/platform.rs`**:
- `PlatformDocumentationProvider` - интегрировать в PlatformTypeResolver
- Логику работы с HTML справкой - добавить в загрузчик

**Из `src/documentation/core/` → `src/application/documentation_service.rs`**:
- `TypeDocumentationFull` - комплексная документация типа
- `DocumentationProvider` trait - интерфейс провайдера документации

### **ЭТАП 4: Финальная очистка (30 мин)**

#### **4.1 Обновить публичный API**
```rust
// src/lib.rs - убрать реэкспорты legacy модулей
// 🔴 Удалить:
// pub mod documentation;
// pub mod adapters;

// ✅ Добавить новые реэкспорты:
pub use application::documentation_service::DocumentationService;
pub use presentation::html_renderer::HtmlRenderer;
```

#### **4.2 Обновить бинарники**
**Файлы**: `src/bin/*.rs`
- Заменить импорты `crate::documentation::` на `crate::application::`
- Заменить импорты `crate::adapters::` на `crate::data::loaders::`
- Обновить инициализацию сервисов

#### **4.3 Финальная проверка**
```bash
# Убедиться что нет сломанных импортов
cargo check

# Запустить тесты
cargo test

# Убедиться что все бинарники компилируются
cargo build --bins
```

## Критерии готовности (DoD)

### ✅ **Архитектурные**
- [ ] Нет папок `src/documentation/`, `src/adapters/`  
- [ ] Единые определения типов без дублирования
- [ ] Все функции сохранены в правильных слоях
- [ ] `CentralTypeSystem` работает как координатор всех слоёв

### ✅ **Технические**
- [ ] `cargo check` - 0 ошибок
- [ ] `cargo test` - все тесты проходят  
- [ ] `cargo build --bins` - все бинарники собираются
- [ ] Нет warnings о неиспользуемых импортах

### ✅ **Функциональные**
- [ ] LSP сервер запускается и отвечает на запросы
- [ ] Web сервер показывает документацию и поиск
- [ ] CLI утилиты работают для анализа проектов

## Ожидаемый результат

**После выполнения плана:**
- ✅ Чистая плоская архитектура без legacy папок (соответствует docs/reference/target_architecture/)
- ✅ Единые определения типов без дублирования  
- ✅ Рабочая компиляция с 0 ошибок
- ✅ Все функции сохранены, просто перенесены в правильные слои
- ✅ Готовность к следующим фазам развития (Flow-sensitive анализ, etc.)

**Общее время**: ~2.5 часа  
**Риск**: Низкий (в основном перенос кода без изменения логики)
