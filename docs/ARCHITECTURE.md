# Архитектура BSL Gradual Type System

## 📊 Обзор

BSL Gradual Type System - это современная система типов для языка 1С:Предприятие BSL, объединяющая статический анализ с runtime контрактами через градуальную типизацию.

## 🎯 Ключевые принципы

1. **Градуальная типизация** - плавный переход от динамической к статической типизации
2. **Эволюционная архитектура** - начинаем с MVP, расширяем без переделок
3. **Фасетная система** - поддержка множественных представлений типов
4. **Честная работа с неопределённостью** - уровни уверенности вместо догадок

## 🏗️ Архитектура системы

### Центральные абстракции

#### TypeResolution - не тип, а разрешение типа

```rust
pub struct TypeResolution {
    certainty: Certainty,        // Known | Inferred(0.0-1.0) | Unknown
    result: ResolutionResult,     // Concrete | Union | Conditional | Dynamic
    source: ResolutionSource,     // Static | Inferred | Runtime | Predicted
    metadata: ResolutionMetadata, // Debug info
}
```

#### UnifiedBslType - объединённое представление

```rust
pub struct UnifiedBslType {
    // Идентификация
    core_name: String,
    metadata_kind: Option<MetadataKind>,
    
    // Фасеты для разных представлений
    facets: HashMap<FacetKind, Facet>,
    
    // Разрешение типа
    resolution: TypeResolution,
    
    // Градуальная информация
    gradual_info: GradualInfo {
        static_type: Option<StaticType>,
        dynamic_contract: Option<Contract>,
        confidence: f32,
    },
}
```

### Слоистая архитектура

```
┌─────────────────────────────────────────┐
│         Application Layer               │
│   (LSP Server, CLI Tools, Extensions)   │
├─────────────────────────────────────────┤
│         Resolution Layer                │
│   (Type Resolver, Context Resolver)     │
├─────────────────────────────────────────┤
│           Core Layer                    │
│   (Types, Facets, Contracts)           │
├─────────────────────────────────────────┤
│         Adapter Layer                   │
│   (Platform Docs, Config Parser)        │
└─────────────────────────────────────────┘
```

## 🚀 Эволюционный план развития

### Phase 1: MVP (недели 1-2) ✅
**Статус**: Реализовано

- [x] Базовые структуры данных
- [x] TypeResolution с уровнями уверенности
- [x] Простой резолвер типов
- [x] Контракты для неопределённых типов
- [ ] Загрузка платформенных типов
- [ ] Парсинг Configuration.xml

### Phase 2: Фасеты и контекст (недели 3-4)
**Статус**: Планируется

- [ ] Полная поддержка фасетов (Manager, Object, Reference, Metadata)
- [ ] Контекстное разрешение типов
- [ ] Переходы между фасетами
- [ ] Динамический доступ по имени

### Phase 3: Вывод типов (недели 5-7)
**Статус**: Будущее

- [ ] Constraint collector
- [ ] Constraint solver (Hindley-Milner style)
- [ ] Inference cache
- [ ] Цепочки вызовов

### Phase 4: Runtime контракты (недели 8-9)
**Статус**: Будущее

- [ ] Генерация контрактов
- [ ] Инъекция runtime проверок
- [ ] Настраиваемая строгость
- [ ] Отчёты о нарушениях

### Phase 5: Продвинутые возможности (недели 10-13)
**Статус**: Экспериментально

- [ ] Abstract interpretation
- [ ] ML предсказания
- [ ] Flow analysis
- [ ] Оптимизации производительности

## 🔌 Точки расширения

### 1. Type Sources
```rust
pub trait TypeSource {
    fn can_resolve(&self, expr: &Expression) -> bool;
    fn resolve(&self, expr: &Expression) -> Option<TypeResolution>;
}
```

### 2. Type Analyzers
```rust
pub trait TypeAnalyzer {
    fn analyze(&self, resolution: TypeResolution) -> TypeResolution;
}
```

### 3. Extension Hooks
```rust
pub trait ExtensionHook {
    fn on_type_resolved(&mut self, type_: &mut UnifiedBslType);
    fn on_constraint_added(&self, constraint: &Constraint);
}
```

## 🎨 Фасетная система

Фасеты позволяют одному типу иметь разные представления в зависимости от контекста:

```bsl
// Manager фасет
Справочники.Контрагенты.СоздатьЭлемент()

// Object фасет (изменяемый)
НовыйКонтрагент.ИНН = "1234567890"
НовыйКонтрагент.Записать()

// Reference фасет (ссылка)
Контрагент = Справочники.Контрагенты.НайтиПоКоду("123")

// Metadata фасет
Метаданные.Справочники.Контрагенты
```

## 🔧 Градуальная типизация

### Уровни уверенности

1. **Known** - тип точно известен из кода
2. **Inferred(0.8)** - тип выведен с уверенностью 80%
3. **Unknown** - тип невозможно определить статически

### Runtime контракты

Для типов с низкой уверенностью генерируются runtime проверки:

```bsl
// Автоматически генерируемый контракт
Если ТипЗнч(Переменная) <> Тип("СправочникСсылка.Контрагенты") Тогда
    ВызватьИсключение "Type mismatch: expected Контрагенты";
КонецЕсли;
```

## 📚 Дополнительная документация

- [Эволюционная архитектура](architecture/EVOLUTIONARY_TYPE_SYSTEM_ARCHITECTURE.md)
- [Дизайн фасетной системы](design/FACET_SYSTEM_DESIGN.md)
- [Альтернативные подходы](design/ALTERNATIVE_TYPE_SYSTEM_APPROACHES.md)
- [Обзор решений](decisions/UNIFIED_TYPE_SYSTEM_COMPILED_REVIEW.md)

## 🎯 Ключевые преимущества подхода

1. **Инкрементальная разработка** - каждая фаза даёт работающий функционал
2. **Обратная совместимость** - новые слои не ломают старые
3. **Модульность** - компоненты слабо связаны
4. **Расширяемость** - легко добавлять новые анализаторы
5. **Практичность** - MVP работает уже через 2 недели

## 🚦 Текущий статус

- **Версия**: 0.1.0 (MVP)
- **Фаза**: 1 из 5 завершена
- **Готовность**: Базовая инфраструктура работает
- **Следующий шаг**: Загрузка платформенных типов и парсинг конфигурации