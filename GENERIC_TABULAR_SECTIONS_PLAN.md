# План реализации Generic типов для табличных частей

**Milestone:** 2.19-2.21
**Общее время:** 10 дней (6-7 при параллельной работе)
**Статус:** Готов к реализации
**Дата создания:** 2025-01-18

---

## 📊 Контекст

### ✅ Критическое открытие

Generic инфраструктура **УЖЕ ПОЛНОСТЬЮ РЕАЛИЗОВАНА** в Milestone 2.3:
- ✅ `GenericType` структура работает (`shared/src/domain/types.rs:190`)
- ✅ `ResolutionResult::Generic` поддерживается
- ✅ `resolve_generic()` создаёт правильную TypeResolution
- ✅ `metadata_lookup` извлекает базовый тип из Generic (строка 282)
- ✅ Обработка в validators, flow_analysis
- ✅ 5 тестов Generic типов проходят успешно

### Цель

Добавить поддержку табличных частей как `ТабличнаяЧасть<СтрокаРаботы>` используя существующую Generic инфраструктуру.

### Результат

```bsl
// Generic работает:
Документ = Документы.ЗаказНаряды.СоздатьДокумент();
Работы = Документ.Работы;
// Тип: ТабличнаяЧасть<СтрокаРаботы>

НоваяСтрока = Работы.Добавить();
// Тип: СтрокаРаботы (из Generic параметра!)

НоваяСтрока.ВидРаботы = "Монтаж";
// Hover: атрибут ВидРаботы (тип известен из TabularRowType)
```

---

## 🎯 Задачи

### Task 1: TabularRowType в ConcreteType (2 дня)

**Зависимости:** Нет (базовый task)

#### Изменения

##### Файл: `shared/src/domain/types.rs`

**Добавить enum вариант (после строки 265):**
```rust
// БЫЛО (строка 260-266):
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConcreteType {
    Platform(PlatformType),
    Configuration(ConfigurationType),
    Primitive(PrimitiveType),
    Special(SpecialType),
    GlobalFunction(GlobalFunctionInfo),
}

// ПОСЛЕ:
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConcreteType {
    Platform(PlatformType),
    Configuration(ConfigurationType),
    Primitive(PrimitiveType),
    Special(SpecialType),
    GlobalFunction(GlobalFunctionInfo),
    TabularRow(TabularRowType),  // ← НОВОЕ
}
```

**Добавить структуру TabularRowType (после строки 294):**
```rust
/// Тип строки табличной части для Generic типов
///
/// Пример: ТабличнаяЧасть<СтрокаРаботы>, где СтрокаРаботы - это TabularRowType
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularRowType {
    /// Полное имя родительского типа (например "Документы.ЗаказПокупателя")
    pub parent_type: String,

    /// Имя табличной части (например "Работы")
    pub tabular_section_name: String,

    /// Атрибуты строки табличной части
    pub attributes: Vec<RawAttributeData>,
}

impl TabularRowType {
    /// Создать новый тип строки табличной части
    pub fn new(parent_type: String, section_name: String, attributes: Vec<RawAttributeData>) -> Self {
        Self {
            parent_type,
            tabular_section_name: section_name,
            attributes,
        }
    }

    /// Получить полное имя типа (например "СтрокаРаботы")
    pub fn get_full_name(&self) -> String {
        format!("Строка{}", self.tabular_section_name)
    }

    /// Получить display name для UI
    pub fn display_name(&self) -> String {
        format!("{}.{}", self.parent_type, self.tabular_section_name)
    }
}
```

**Обновить Display trait (строка 428-440):**
```rust
impl fmt::Display for ConcreteType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConcreteType::Platform(platform) => write!(f, "{}", platform.name),
            ConcreteType::Configuration(config) => {
                write!(f, "{}.{}", config.kind.display_name(), config.name)
            }
            ConcreteType::Primitive(primitive) => write!(f, "{}", primitive.display_name()),
            ConcreteType::Special(special) => write!(f, "{}", special.display_name()),
            ConcreteType::GlobalFunction(func) => write!(f, "{}()", func.name),
            ConcreteType::TabularRow(row) => write!(f, "{}", row.get_full_name()),  // ← НОВОЕ
        }
    }
}
```

#### Затронутые файлы (требуют обновления match expressions)

Все файлы с `match ConcreteType`:
1. `shared/src/domain/metadata_lookup.rs` (строка 253-270)
2. `shared/src/domain/resolver.rs` (множество мест)
3. `shared/src/domain/validators.rs`
4. `backend/src/application/type_system_service.rs`

**Для каждого добавить:**
```rust
ConcreteType::TabularRow(_) => {
    // Обработка или игнорирование в зависимости от контекста
}
```

#### Критерии готовности

- ✅ `cargo check` проходит без ошибок
- ✅ Unit тест `test_tabular_row_type_creation()` проходит
- ✅ Serde сериализация/десериализация работает
- ✅ Все match expressions обновлены (нет compiler warnings)

#### Unit тест

```rust
// shared/src/domain/types.rs (добавить в конец)
#[cfg(test)]
mod tabular_row_tests {
    use super::*;

    #[test]
    fn test_tabular_row_type_creation() {
        let row = TabularRowType::new(
            "Документы.ЗаказПокупателя".to_string(),
            "Работы".to_string(),
            vec![
                RawAttributeData {
                    name: "Услуга".to_string(),
                    attr_type: "СправочникСсылка.Номенклатура".to_string()
                },
            ]
        );

        assert_eq!(row.get_full_name(), "СтрокаРаботы");
        assert_eq!(row.display_name(), "Документы.ЗаказПокупателя.Работы");
        assert_eq!(row.attributes.len(), 1);
    }

    #[test]
    fn test_tabular_row_serde() {
        let row = TabularRowType::new(
            "Документы.ЗаказПокупателя".to_string(),
            "Товары".to_string(),
            vec![]
        );

        let json = serde_json::to_string(&row).unwrap();
        let deserialized: TabularRowType = serde_json::from_str(&json).unwrap();

        assert_eq!(row, deserialized);
    }
}
```

#### Риски

- **Breaking changes:** Все файлы с `match ConcreteType` потребуют обновления
- **Edge case:** TabularRow без атрибутов (пустая табличная часть) - допустимо

---

### Task 2: Резолюция через GenericType (2 дня)

**Зависимости:** Task 1 (нужен TabularRowType)

#### Изменения

##### Файл: `shared/src/domain/resolver.rs`

**Обновить `resolve_member_access()` (добавить после строки 101):**
```rust
fn resolve_member_access(&self, base: &str, member: &str) -> TypeResolution {
    use crate::domain::types::{
        Certainty, ConcreteType, ConfigurationType, GenericType, MetadataKind,
        ResolutionMetadata, ResolutionResult, ResolutionSource, TabularRowType
    };

    // 1. Сначала резолвим базовый тип (например "документ" → "Документы.ЗаказПокупателя")
    let base_resolution = self.resolve_expression_sync(base);

    // 2. Проверяем, является ли base конфигурационным типом с табличными частями
    if let ResolutionResult::Concrete(ConcreteType::Configuration(config)) = &base_resolution.result {
        // Получаем полное имя типа для поиска в репозитории
        let full_type_name = format!("{}.{}",
            self.metadata_kind_to_prefix(&config.kind),
            config.name
        );

        // Ищем метаданные конфигурационного типа
        if let Some(raw_type) = self.repository.find_type(&full_type_name) {
            // Проверяем табличные части
            if let Some(tabular_section) = raw_type.tabular_sections.iter()
                .find(|ts| ts.name == member)
            {
                // Создаём TabularRowType из метаданных
                let row_type = TabularRowType::new(
                    full_type_name.clone(),
                    tabular_section.name.clone(),
                    tabular_section.attributes.clone()
                );

                // Используем СУЩЕСТВУЮЩИЙ GenericType!
                let generic_type = GenericType {
                    base_type: "ТабличнаяЧасть".to_string(),
                    type_params: vec![ConcreteType::TabularRow(row_type)],
                };

                return TypeResolution {
                    certainty: Certainty::Known,
                    result: ResolutionResult::Generic(generic_type),
                    source: ResolutionSource::Static,
                    metadata: ResolutionMetadata {
                        file: Some(format!("{}.{}", full_type_name, member)),
                        line: None,
                        column: None,
                        notes: vec![format!("Табличная часть '{}' документа '{}'", member, config.name)],
                    },
                    active_facet: Some(FacetKind::Collection),
                    available_facets: vec![FacetKind::Collection],
                };
            }
        }
    }

    // Существующая логика для других member access...
    // (строки 108-187 остаются без изменений)
}
```

**Добавить helper метод:**
```rust
/// Конвертирует MetadataKind в префикс для имени типа
fn metadata_kind_to_prefix(&self, kind: &MetadataKind) -> &'static str {
    match kind {
        MetadataKind::Catalog => "Справочники",
        MetadataKind::Document => "Документы",
        MetadataKind::Enum => "Перечисления",
        MetadataKind::InformationRegister => "РегистрыСведений",
        MetadataKind::AccumulationRegister => "РегистрыНакопления",
        MetadataKind::AccountingRegister => "РегистрыБухгалтерии",
        MetadataKind::CalculationRegister => "РегистрыРасчета",
        MetadataKind::ChartOfAccounts => "ПланыСчетов",
        MetadataKind::ChartOfCharacteristicTypes => "ПланыВидовХарактеристик",
        MetadataKind::ChartOfCalculationTypes => "ПланыВидовРасчета",
        MetadataKind::Report => "Отчеты",
        MetadataKind::DataProcessor => "Обработки",
        MetadataKind::BusinessProcess => "БизнесПроцессы",
        MetadataKind::Task => "Задачи",
    }
}
```

#### Критерии готовности

- ✅ `cargo test -p bsl-shared resolver` проходит
- ✅ Unit тест `test_resolve_tabular_section()` проходит
- ✅ Резолюция `документ.Работы` возвращает `Generic(ТабличнаяЧасть<СтрокаРаботы>)`

#### Unit тест

```rust
// shared/src/domain/resolver/resolver_tabular_tests.rs (новый файл)
#[cfg(test)]
mod tabular_section_tests {
    use super::*;
    use std::sync::Arc;
    use crate::domain::repository::InMemoryTypeRepository;
    use crate::domain::types::*;

    #[test]
    fn test_resolve_tabular_section() {
        // Настройка репозитория с метаданными документа
        let repo = Arc::new(InMemoryTypeRepository::new());

        // Загружаем тестовый документ с табличной частью
        let doc_type = RawTypeData {
            name: "Документы.ЗаказПокупателя".to_string(),
            english_name: "Documents.SalesOrder".to_string(),
            description: "Заказ покупателя".to_string(),
            category: "Документы".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![],
            kind: Some(MetadataKind::Document),
            attributes: vec![],
            tabular_sections: vec![
                RawTabularSectionData {
                    name: "Работы".to_string(),
                    attributes: vec![
                        RawAttributeData {
                            name: "Услуга".to_string(),
                            attr_type: "СправочникСсылка.Номенклатура".to_string(),
                        },
                    ],
                },
            ],
            enum_values: vec![],
        };

        repo.load_types(vec![doc_type]).unwrap();

        let resolver = TypeResolver::new(repo);

        // Создаём базовую резолюцию для документа
        // (в реальности это будет из IR/AST)
        let resolution = resolver.resolve_member_access("документ", "Работы");

        match resolution.result {
            ResolutionResult::Generic(generic) => {
                assert_eq!(generic.base_type, "ТабличнаяЧасть");
                assert_eq!(generic.type_params.len(), 1);

                match &generic.type_params[0] {
                    ConcreteType::TabularRow(row) => {
                        assert_eq!(row.tabular_section_name, "Работы");
                        assert_eq!(row.parent_type, "Документы.ЗаказПокупателя");
                        assert_eq!(row.attributes.len(), 1);
                    },
                    _ => panic!("Expected TabularRow type parameter")
                }
            },
            _ => panic!("Expected Generic type for tabular section, got {:?}", resolution.result)
        }
    }
}
```

#### Риски

- **Проблема:** Если метаданные конфигурации не загружены
- **Решение:** Возвращать `Certainty::Inferred(0.5)` с заглушкой
- **Edge case:** Табличная часть с пустым списком атрибутов - допустимо

---

### Task 3: Базовый тип "ТабличнаяЧасть" (2 дня)

**Зависимости:** Нет (независимый базовый тип)

#### Изменения

##### Файл: `backend/src/data/loaders/platform_types.rs`

**Добавить функцию `create_tabular_section_type()`:**
```rust
/// Создаёт базовый платформенный тип ТабличнаяЧасть с generic методами
pub fn create_tabular_section_type() -> RawTypeData {
    RawTypeData {
        name: "ТабличнаяЧасть".to_string(),
        english_name: "TabularSection".to_string(),
        description: "Коллекция строк табличной части объектов конфигурации".to_string(),
        category: "Коллекции".to_string(),
        source: RawDataSource::Platform,
        methods: vec![
            RawMethodData {
                name: "Добавить".to_string(),
                english_name: "Add".to_string(),
                return_type: "T".to_string(),  // Generic параметр!
                params: vec![],
            },
            RawMethodData {
                name: "Вставить".to_string(),
                english_name: "Insert".to_string(),
                return_type: "T".to_string(),
                params: vec![
                    RawParamData {
                        name: "Индекс".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                    },
                ],
            },
            RawMethodData {
                name: "Удалить".to_string(),
                english_name: "Delete".to_string(),
                return_type: "".to_string(),
                params: vec![
                    RawParamData {
                        name: "Индекс".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                    },
                ],
            },
            RawMethodData {
                name: "Очистить".to_string(),
                english_name: "Clear".to_string(),
                return_type: "".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "Количество".to_string(),
                english_name: "Count".to_string(),
                return_type: "Число".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "Индекс".to_string(),
                english_name: "IndexOf".to_string(),
                return_type: "Число".to_string(),
                params: vec![
                    RawParamData {
                        name: "Элемент".to_string(),
                        param_type: "T".to_string(),
                        is_optional: false,
                    },
                ],
            },
            RawMethodData {
                name: "Найти".to_string(),
                english_name: "Find".to_string(),
                return_type: "T".to_string(),
                params: vec![
                    RawParamData {
                        name: "Значение".to_string(),
                        param_type: "Произвольный".to_string(),
                        is_optional: false,
                    },
                    RawParamData {
                        name: "Колонки".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                ],
            },
            RawMethodData {
                name: "НайтиСтроки".to_string(),
                english_name: "FindRows".to_string(),
                return_type: "Массив".to_string(),  // Должно быть Массив<T>
                params: vec![
                    RawParamData {
                        name: "Отбор".to_string(),
                        param_type: "Структура".to_string(),
                        is_optional: false,
                    },
                ],
            },
            RawMethodData {
                name: "Итог".to_string(),
                english_name: "Total".to_string(),
                return_type: "Число".to_string(),
                params: vec![
                    RawParamData {
                        name: "ИмяКолонки".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: false,
                    },
                ],
            },
            RawMethodData {
                name: "Сдвинуть".to_string(),
                english_name: "Move".to_string(),
                return_type: "".to_string(),
                params: vec![
                    RawParamData {
                        name: "Строка".to_string(),
                        param_type: "T".to_string(),
                        is_optional: false,
                    },
                    RawParamData {
                        name: "Смещение".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                    },
                ],
            },
            RawMethodData {
                name: "Получить".to_string(),
                english_name: "Get".to_string(),
                return_type: "T".to_string(),
                params: vec![
                    RawParamData {
                        name: "Индекс".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                    },
                ],
            },
            RawMethodData {
                name: "Сортировать".to_string(),
                english_name: "Sort".to_string(),
                return_type: "".to_string(),
                params: vec![
                    RawParamData {
                        name: "Колонки".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                ],
            },
            RawMethodData {
                name: "Свернуть".to_string(),
                english_name: "GroupBy".to_string(),
                return_type: "".to_string(),
                params: vec![
                    RawParamData {
                        name: "Группировка".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                    RawParamData {
                        name: "Суммируемые".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                ],
            },
            RawMethodData {
                name: "Скопировать".to_string(),
                english_name: "Copy".to_string(),
                return_type: "ТабличнаяЧасть".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "Выгрузить".to_string(),
                english_name: "Unload".to_string(),
                return_type: "ТаблицаЗначений".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "Загрузить".to_string(),
                english_name: "Load".to_string(),
                return_type: "".to_string(),
                params: vec![
                    RawParamData {
                        name: "Таблица".to_string(),
                        param_type: "ТаблицаЗначений".to_string(),
                        is_optional: false,
                    },
                ],
            },
        ],
        properties: vec![],
        facets: vec![FacetKind::Collection],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
    }
}
```

**Добавить в функцию `load_platform_types()`:**
```rust
pub fn load_platform_types(repository: &Arc<dyn TypeRepository>) -> Result<()> {
    let mut types = vec![
        // ... существующие типы ...
    ];

    // Добавляем ТабличнаяЧасть
    types.push(create_tabular_section_type());

    repository.load_types(types)?;
    Ok(())
}
```

#### Критерии готовности

- ✅ Тип "ТабличнаяЧасть" загружается в TypeRepository при старте
- ✅ 16 методов доступны через metadata_lookup
- ✅ Методы с типом "T" помечены корректно

#### Риски

- **Проблема:** Как обозначить generic параметр "T"?
- **Решение:** Использовать строку "T" и обрабатывать в metadata_lookup (Task 4)

---

### Task 4: Generic handling в metadata_lookup (1 день)

**Зависимости:** Task 1, Task 3

#### Изменения

##### Файл: `shared/src/domain/metadata_lookup.rs`

**Обновить `get_methods()` (после строки 107):**
```rust
pub fn get_methods(&self, resolution: &TypeResolution) -> Vec<RawMethodData> {
    match &resolution.result {
        ResolutionResult::Generic(generic_type) => {
            // Получаем методы базового типа
            let base_methods = self.repository
                .find_type(&generic_type.base_type)
                .map(|raw| raw.methods.clone())
                .unwrap_or_default();

            // Если есть параметр типа, подставляем его вместо "T"
            if let Some(param_type) = generic_type.type_params.first() {
                let param_type_name = self.format_concrete_type(param_type);

                base_methods.into_iter().map(|mut method| {
                    // Заменяем "T" на реальный тип параметра в return_type
                    if method.return_type == "T" {
                        method.return_type = param_type_name.clone();
                    }
                    if method.return_type.contains("<T>") {
                        method.return_type = method.return_type.replace("<T>", &format!("<{}>", param_type_name));
                    }

                    // Заменяем "T" в параметрах методов
                    for param in &mut method.params {
                        if param.param_type == "T" {
                            param.param_type = param_type_name.clone();
                        }
                    }

                    method
                }).collect()
            } else {
                base_methods
            }
        },

        // Существующая логика для других типов
        _ => {
            let type_name = self.extract_type_name(resolution);
            let raw_type = type_name.and_then(|name| self.repository.find_type(&name));
            raw_type.map(|raw| raw.methods.clone()).unwrap_or_default()
        }
    }
}

/// Форматирует ConcreteType в строку для подстановки
fn format_concrete_type(&self, concrete: &ConcreteType) -> String {
    match concrete {
        ConcreteType::TabularRow(row) => row.get_full_name(),
        ConcreteType::Platform(p) => p.name.clone(),
        ConcreteType::Configuration(c) => format!("{}.{}", c.kind.display_name(), c.name),
        ConcreteType::Primitive(p) => p.display_name().to_string(),
        ConcreteType::Special(s) => s.display_name().to_string(),
        ConcreteType::GlobalFunction(f) => f.name.clone(),
    }
}
```

**Добавить метод для получения атрибутов строки:**
```rust
/// Получить атрибуты строки табличной части из Generic типа
pub fn get_tabular_row_attributes(&self, generic_type: &GenericType) -> Vec<RawAttributeData> {
    if generic_type.base_type == "ТабличнаяЧасть" {
        if let Some(ConcreteType::TabularRow(row)) = generic_type.type_params.first() {
            return row.attributes.clone();
        }
    }
    vec![]
}
```

#### Критерии готовности

- ✅ `get_methods()` подставляет тип параметра вместо "T"
- ✅ Метод `Добавить()` возвращает `СтрокаРаботы` вместо "T"
- ✅ Unit тест `test_generic_methods_substitution()` проходит

#### Unit тест

```rust
#[cfg(test)]
mod generic_methods_tests {
    use super::*;

    #[test]
    fn test_generic_methods_substitution() {
        let repo = Arc::new(InMemoryTypeRepository::new());

        // Загружаем базовый тип ТабличнаяЧасть
        repo.load_types(vec![create_tabular_section_type()]).unwrap();

        let lookup = TypeMetadataLookup::new(repo);

        // Создаём Generic тип: ТабличнаяЧасть<СтрокаРаботы>
        let row_type = TabularRowType::new(
            "Документы.ЗаказПокупателя".to_string(),
            "Работы".to_string(),
            vec![]
        );

        let generic_type = GenericType {
            base_type: "ТабличнаяЧасть".to_string(),
            type_params: vec![ConcreteType::TabularRow(row_type)],
        };

        let resolution = TypeResolution {
            result: ResolutionResult::Generic(generic_type),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let methods = lookup.get_methods(&resolution);

        // Проверяем метод Добавить()
        let add_method = methods.iter().find(|m| m.name == "Добавить").unwrap();
        assert_eq!(add_method.return_type, "СтрокаРаботы");

        // Проверяем метод Получить()
        let get_method = methods.iter().find(|m| m.name == "Получить").unwrap();
        assert_eq!(get_method.return_type, "СтрокаРаботы");
    }
}
```

#### Риски

- **Edge case:** Вложенные generic типы `Массив<ТабличнаяЧасть<T>>` - пока не поддерживается

---

### Task 5: LSP hover форматирование (1 день)

**Зависимости:** Task 4

#### Изменения

##### Файл: `backend/src/application/type_system_service.rs`

**Добавить `format_generic_tabular_hover()` (после строки 1630):**
```rust
/// Форматирует hover для табличной части с Generic типом
fn format_generic_tabular_hover(
    &self,
    var_name: &str,
    generic_type: &GenericType,
) -> String {
    let mut output = format!("**Переменная:** `{}`\n", var_name);

    // Показываем generic тип
    output.push_str(&format!("**Тип:** `{}<", generic_type.base_type));

    // Извлекаем тип строки
    if let Some(ConcreteType::TabularRow(row_type)) = generic_type.type_params.first() {
        output.push_str(&row_type.get_full_name());
        output.push_str(">`\n");
        output.push_str(&format!("**Табличная часть:** `{}`\n", row_type.tabular_section_name));
        output.push_str(&format!("**Документ:** `{}`\n\n", row_type.parent_type));

        // Методы коллекции
        output.push_str("### 📋 Методы коллекции:\n");
        let collection_resolution = TypeResolution {
            result: ResolutionResult::Generic(generic_type.clone()),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![],
            },
            active_facet: None,
            available_facets: vec![],
        };

        let methods = self.metadata_lookup.get_methods(&collection_resolution);
        for method in methods.iter().take(5) {
            output.push_str(&format!("- `{}(", method.name));

            // Параметры метода
            let params_str = method.params.iter()
                .map(|p| format!("{}: {}", p.name, p.param_type))
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str(&params_str);
            output.push(')');

            if !method.return_type.is_empty() {
                output.push_str(&format!(" → `{}`", method.return_type));
            }
            output.push('\n');
        }

        if methods.len() > 5 {
            output.push_str(&format!("... и ещё {} методов\n", methods.len() - 5));
        }

        // Атрибуты строки
        output.push_str("\n### 🏷️ Атрибуты строки:\n");
        for attr in row_type.attributes.iter().take(10) {
            output.push_str(&format!("- `{}`: `{}`\n", attr.name, attr.attr_type));
        }

        if row_type.attributes.len() > 10 {
            output.push_str(&format!("... и ещё {} атрибутов\n", row_type.attributes.len() - 10));
        }
    } else {
        output.push_str("?>`\n");
        output.push_str("⚠️ **Не удалось извлечь тип строки**\n");
    }

    output
}
```

**Обновить `format_variable_hover()` (около строки 1630):**
```rust
fn format_variable_hover(
    &self,
    var_name: &str,
    type_hint: &bsl_shared::ir::TypeHint,
) -> String {
    // ... существующий код извлечения type_name ...

    let resolution = self.inference_service.resolve_expression_sync(&type_name);

    // Специальная обработка для Generic табличных частей
    if let ResolutionResult::Generic(ref generic_type) = resolution.result {
        if generic_type.base_type == "ТабличнаяЧасть" {
            return self.format_generic_tabular_hover(var_name, generic_type);
        }
    }

    // ... существующий код для обычных типов ...
}
```

#### Критерии готовности

- ✅ Hover показывает `ТабличнаяЧасть<СтрокаРаботы>`
- ✅ Отображаются методы коллекции с подставленными типами
- ✅ Отображаются атрибуты строки табличной части
- ✅ Integration тест с реальным hover проходит

#### Пример hover output

```markdown
**Переменная:** `работы`
**Тип:** `ТабличнаяЧасть<СтрокаРаботы>`
**Табличная часть:** `Работы`
**Документ:** `Документы.ЗаказПокупателя`

### 📋 Методы коллекции:
- `Добавить()` → `СтрокаРаботы`
- `Количество()` → `Число`
- `Удалить(Индекс: Число)`
- `Очистить()`
- `Получить(Индекс: Число)` → `СтрокаРаботы`
... и ещё 11 методов

### 🏷️ Атрибуты строки:
- `Услуга`: `СправочникСсылка.Номенклатура`
- `Количество`: `Число`
- `Цена`: `Число`
- `Сумма`: `Число`
```

#### Риски

- **UI перегрузка:** Слишком много информации в hover
- **Решение:** Ограничить до 5 методов и 10 атрибутов

---

### Task 6: Тесты (2 дня)

**Зависимости:** Все предыдущие tasks

#### Unit тесты

##### `shared/src/domain/types.rs` (уже добавлены в Task 1)

##### `shared/src/domain/resolver/resolver_tabular_tests.rs` (уже добавлены в Task 2)

##### `shared/src/domain/metadata_lookup_tests.rs` (уже добавлены в Task 4)

#### Integration тесты

##### Файл: `backend/tests/tabular_section_hover_test.rs` (новый)

```rust
use bsl_backend::application::type_system_service::TypeSystemService;
use bsl_backend::system::SystemCoordinator;
use std::sync::Arc;

#[tokio::test]
async fn test_tabular_section_hover() {
    let code = r#"
        Процедура ТестТабличнойЧасти()
            документ = Документы.ЗаказПокупателя.СоздатьДокумент();
            работы = документ.Работы;
            новаяСтрока = работы.Добавить();
        КонецПроцедуры
    "#;

    // Настройка системы
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.unwrap();

    let service = coordinator.type_service().unwrap();

    // Тестируем hover на переменной "работы" (строка 4, колонка 12)
    let hover = service.get_hover_info(code, 4, 12).await.unwrap();

    assert!(hover.is_some());
    let hover_text = hover.unwrap();

    // Проверяем содержимое
    assert!(hover_text.contains("ТабличнаяЧасть"));
    assert!(hover_text.contains("СтрокаРаботы") || hover_text.contains("Работы"));
    assert!(hover_text.contains("Методы коллекции"));
    assert!(hover_text.contains("Добавить()"));
    assert!(hover_text.contains("→") || hover_text.contains("СтрокаРаботы"));
}

#[tokio::test]
async fn test_tabular_row_hover() {
    let code = r#"
        Процедура ТестСтрокиТабличнойЧасти()
            документ = Документы.ЗаказПокупателя.СоздатьДокумент();
            новаяСтрока = документ.Работы.Добавить();
            новаяСтрока.Услуга = "Что-то";
        КонецПроцедуры
    "#;

    let coordinator = SystemCoordinator::new();
    coordinator.start().await.unwrap();
    let service = coordinator.type_service().unwrap();

    // Тестируем hover на переменной "новаяСтрока" (строка 4)
    let hover = service.get_hover_info(code, 4, 12).await.unwrap();

    assert!(hover.is_some());
    let hover_text = hover.unwrap();

    // Проверяем, что это тип строки
    assert!(hover_text.contains("Строка") || hover_text.contains("СтрокаРаботы"));
    assert!(hover_text.contains("Атрибуты") || hover_text.contains("Услуга"));
}
```

#### E2E тесты для LSP

##### Файл: `backend/tests/lsp_tabular_section_test.rs` (новый)

```rust
#[tokio::test]
async fn test_lsp_hover_on_tabular_section() {
    // Запускаем LSP сервер
    let (client, server) = create_test_lsp_pair().await;

    // Открываем файл
    let uri = "file:///test.bsl";
    let text = r#"
        Процедура Тест()
            документ = Документы.ЗаказПокупателя.СоздатьДокумент();
            работы = документ.Работы;
        КонецПроцедуры
    "#;

    client.open_document(uri, text).await;

    // Запрашиваем hover на "работы"
    let hover_response = client.hover(uri, 4, 20).await;

    assert!(hover_response.is_ok());
    let hover = hover_response.unwrap();

    assert!(hover.is_some());
    let hover_content = hover.unwrap().contents;

    // Проверяем содержимое
    assert!(hover_content.contains("ТабличнаяЧасть"));
}
```

#### Критерии готовности

- ✅ 3 unit теста в `shared` проходят
- ✅ 2 integration теста в `backend` проходят
- ✅ 1 E2E тест для LSP проходит
- ✅ Coverage > 80% для новой функциональности
- ✅ `cargo test` проходит без ошибок
- ✅ `cargo clippy` без warnings

#### Риски

- **Проблема:** Тесты зависят от загруженных метаданных конфигурации
- **Решение:** Использовать mock данные в unit тестах

---

## 🚀 Последовательность реализации

```
День 1-2: Task 1 (TabularRowType) ───┐
                                      ├─→ День 5-6: Task 2 (Резолюция) ──┐
День 3-4: Task 3 (Базовый тип) ──────┤                                   │
                                      └─→ День 7: Task 4 (Metadata) ─────┤
                                                                          ├─→ День 8: Task 5 (Hover) ──┐
                                                                          │                            │
                                                                          └────────────────────────────┴─→ День 9-10: Task 6 (Тесты)
```

**Параллельная работа:**
- Task 1 и Task 3 можно делать одновременно (независимы)
- Task 2 и Task 4 можно делать одновременно (после завершения зависимостей)

**Оптимальный timeline:** 6-7 дней при параллельной работе

---

## 📋 Чеклист готовности

### Task 1: TabularRowType
- [ ] `ConcreteType::TabularRow` добавлен
- [ ] `TabularRowType` структура создана
- [ ] Display trait обновлён
- [ ] Все match expressions обновлены
- [ ] Unit тесты проходят
- [ ] `cargo check` успешен

### Task 2: Резолюция
- [ ] `resolve_member_access()` обновлён
- [ ] Helper метод `metadata_kind_to_prefix()` добавлен
- [ ] Generic создаётся для табличных частей
- [ ] Unit тесты проходят

### Task 3: Базовый тип
- [ ] `create_tabular_section_type()` создана
- [ ] 16 методов с Generic параметрами
- [ ] Тип загружается при старте

### Task 4: Metadata lookup
- [ ] `get_methods()` подставляет Generic параметры
- [ ] `format_concrete_type()` добавлен
- [ ] `get_tabular_row_attributes()` добавлен
- [ ] Unit тесты проходят

### Task 5: LSP hover
- [ ] `format_generic_tabular_hover()` создана
- [ ] `format_variable_hover()` обновлена
- [ ] Hover output читаемый и информативный

### Task 6: Тесты
- [ ] 3 unit теста проходят
- [ ] 2 integration теста проходят
- [ ] 1 E2E тест проходит
- [ ] Coverage > 80%
- [ ] `cargo test` без ошибок
- [ ] `cargo clippy` без warnings

---

## 🎯 Итоговая оценка

- **Общее время:** 10 дней (последовательно) / 6-7 дней (параллельно)
- **Основной риск:** Breaking changes в ConcreteType enum (управляемо через compiler)
- **Основное преимущество:** Generic инфраструктура уже работает - нужно только интегрировать TabularRowType
- **Успех критерий:** LSP hover показывает `ТабличнаяЧасть<СтрокаРаботы>` с методами и атрибутами

---

## 📚 Связанные документы

- ROADMAP_2025.md - общий план проекта
- CLAUDE.md - архитектурная документация
- shared/src/domain/resolver/resolver_generic_tests.rs - существующие Generic тесты

---

**Статус:** ✅ План готов к реализации
**Следующий шаг:** Запустить Coder для Task 1
