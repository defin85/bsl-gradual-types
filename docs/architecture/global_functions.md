# Архитектура поддержки глобальных функций

## Обзор

Глобальные функции в BSL - это **встроенные платформенные функции**, доступные в любом месте кода без привязки к конкретному объекту. Они являются важной частью языка 1С:Предприятие и требуют особой обработки в системе типов.

**Важно**: Не путать с пользовательскими функциями, которые определены в коде BSL. См. [Function Types Hierarchy](function_types_hierarchy.md) для полной картины.

## Проблема

В текущей реализации глобальные функции неправильно классифицируются:
- Парсятся как `SyntaxNode::Type` вместо функций
- Хранятся в категории "Global context" как типы
- LSP предлагает их с неправильным `CompletionItemKind`

## Архитектурное решение

### 1. Расширение Core Layer

В `src/core/types.rs` добавляется новый вариант `ConcreteType`:

```rust
pub enum ConcreteType {
    Platform(PlatformType),
    Configuration(ConfigurationType),
    Primitive(PrimitiveType),
    Special(SpecialType),
    GlobalFunction(GlobalFunction), // Новый вариант
}

pub struct GlobalFunction {
    pub name: String,
    pub english_name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Box<TypeResolution>,
    pub pure: bool,              // Чистая функция без побочных эффектов
    pub polymorphic: bool,        // Полиморфная функция
    pub context_required: Vec<ExecutionContext>, // Где доступна
}
```

### 2. Обновление Adapter Layer

В `src/adapters/syntax_helper_parser.rs`:

```rust
pub enum SyntaxNode {
    Category(CategoryInfo),
    Type(TypeInfo),
    Method(MethodInfo),
    Property(PropertyInfo),
    Constructor(ConstructorInfo),
    GlobalFunction(GlobalFunctionInfo), // Новый вариант
}

pub struct GlobalFunctionInfo {
    pub name: String,
    pub english_name: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub return_description: Option<String>,
    pub polymorphic_variants: Vec<PolymorphicVariant>,
}
```

### 3. Классификация глобальных функций

Глобальные функции делятся на категории:

#### Полиморфные функции
- `Мин(Min)`, `Макс(Max)` - работают с числами, строками, датами
- `Строка(String)`, `Число(Number)` - преобразование типов
- Требуют анализа первого аргумента для определения варианта

#### Системные функции
- `Сообщить(Message)` - вывод сообщений
- `ТекущаяДата(CurrentDate)` - получение даты
- `Тип(Type)`, `ТипЗнч(TypeOf)` - работа с типами

#### Математические функции
- `Окр(Round)`, `Цел(Int)`, `Лог(Log)` - математические операции
- Всегда возвращают числовой тип

#### Строковые функции
- `СтрДлина(StrLen)`, `Лев(Left)`, `Прав(Right)` - работа со строками
- Принимают строки, возвращают строки или числа

## Вывод типов для полиморфных функций

Для функции `Мин`:

```rust
impl GlobalFunction {
    pub fn resolve_return_type(&self, args: &[TypeResolution]) -> TypeResolution {
        match self.name.as_str() {
            "Мин" | "Min" => {
                if args.is_empty() {
                    return TypeResolution::unknown();
                }
                
                // Тип возврата определяется первым аргументом
                match &args[0].result {
                    ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)) => {
                        TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number))
                    }
                    ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)) => {
                        TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String))
                    }
                    ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Date)) => {
                        TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Date))
                    }
                    _ => TypeResolution::inferred(0.5, args[0].result.clone())
                }
            }
            _ => TypeResolution::unknown()
        }
    }
}
```

## Интеграция с LSP

В `src/bin/lsp_server.rs`:

```rust
pub enum CompletionKind {
    Global,
    Catalog,
    Document,
    Enum,
    Method,
    Property,
    GlobalFunction, // Новый вариант
}

// При конвертации в LSP
CompletionKind::GlobalFunction => CompletionItemKind::FUNCTION,
```

## Преимущества решения

1. **Корректная семантика** - глобальные функции отличаются от типов и методов
2. **Правильный вывод типов** - учитывается полиморфизм
3. **Улучшенный UX в IDE** - правильные иконки и подсказки
4. **Градуальная типизация** - поддержка частичной информации о типах
5. **Расширяемость** - легко добавлять новые глобальные функции

## План реализации

1. ✅ Обновить документацию (CLAUDE.md)
2. ⏳ Добавить `GlobalFunction` в `core/types.rs`
3. ⏳ Обновить парсер для правильной классификации
4. ⏳ Добавить вывод типов для полиморфных функций
5. ⏳ Обновить LSP для корректной обработки
6. ⏳ Написать тесты
7. ⏳ Обновить визуализацию

## Тестирование

Ключевые тест-кейсы:

1. Парсинг глобальных функций из синтакс-помощника
2. Вывод типов для полиморфных функций
3. Автодополнение в LSP
4. Hover информация для глобальных функций
5. Различение глобальных функций и методов объектов