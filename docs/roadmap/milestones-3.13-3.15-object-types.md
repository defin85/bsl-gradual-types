# Milestones 3.13-3.15: Object-Based Type System

**Дата создания:** 2025-11-25
**Дата ревизии:** 2025-11-25 (после архитектурного анализа)
**Статус:** Планирование
**Автор:** Architect Agent + Orchestrator Review

---

## Обзор

Серия Milestones для перехода от строкового сравнения типов к объектному, с поддержкой Go To Definition.

### Требования пользователя

1. **Объектное сравнение типов** — вместо текущего строкового (`is_type_compatible()`)
2. **Резолвинг параметров и return types в объекты** — методы/функции должны резолвить типы в объекты
3. **Go To Definition** — из hover должна быть возможность перейти к определению типа

### Существующая архитектура (НЕ дублировать!)

| Компонент | Файл | Назначение |
|-----------|------|------------|
| **TypeResolution** | `shared/src/domain/types.rs` | Результат резолвинга типа (Concrete, Union, Generic, Nullable) |
| **TypeRepository** | `shared/src/domain/repository.rs` | Хранилище RawTypeData, `find_type()` |
| **TypeMetadataLookup** | `shared/src/domain/metadata_lookup.rs` | Мост TypeResolution ↔ RawTypeData |
| **SignatureIndex** | `shared/src/domain/signature_index.rs` | Индекс сигнатур методов |
| **TypeResolver** | `shared/src/domain/resolver.rs` | `is_type_compatible()`, `resolve_expression_sync()` |
| **TypeSystemService** | `backend/src/application/type_system_service.rs` | Application-level сервис (кэширование, оркестрация) |
| **CodeLocation** | `shared/src/domain/code_location.rs` | Контекст выполнения кода (Server/Client) |

### Архитектурные слои (по диаграмме)

```
┌─────────────────────────────────────────────────────────────┐
│ Application Layer: backend/src/application/                │
│   - TypeSystemService (сервисы с кэшированием)              │
│   - SemanticValidationVisitor                               │
├─────────────────────────────────────────────────────────────┤
│ Domain Layer: shared/src/domain/                           │
│   - TypeResolution, TypeResolver (чистая логика)            │
│   - TypeRepository, SignatureIndex                          │
│   - TypeDefinitionLocation (НОВОЕ)                          │
├─────────────────────────────────────────────────────────────┤
│ Data Layer: backend/src/data/                              │
│   - SyntaxHelperParser, PlatformTypes                       │
└─────────────────────────────────────────────────────────────┘
```

---

## 📦 Milestone 3.13: Object-Based Type Comparison

**Приоритет:** 🔴 ВЫСОКИЙ — фундамент для качественной валидации типов
**Оценка времени:** 1.5-2 недели

### Проблема

Текущая валидация типов основана на строковом сравнении в `is_type_compatible()`:

```rust
// shared/src/domain/resolver.rs:1389-1425
fn is_type_compatible(expected: &str, actual: &str) -> bool {
    // Строковое сравнение — не учитывает фасеты, generic, иерархию
    if expected.contains(" | ") {
        return expected.split(" | ").any(|v| names_equal_ignore_case(v.trim(), actual));
    }
    names_equal_ignore_case(expected, actual)
}
```

**Это приводит к:**
- Невозможности учитывать фасеты при сравнении (СправочникОбъект vs СправочникСсылка)
- Отсутствию поддержки иерархии типов
- Неточной валидации Generic типов
- Невозможности реализовать Go To Definition

### Цель

**Расширить существующий TypeResolution** для объектного сравнения типов (НЕ создавать новый ResolvedType!).

---

### Phase 1: Расширение TypeResolution (3-4 дня)

**Файл:** `shared/src/domain/types.rs` — расширение существующего

#### Задача 1.1: Добавить TypeRef для lazy lookup

```rust
// shared/src/domain/types.rs — РАСШИРЕНИЕ (не новый файл!)

/// Ссылка на тип в TypeRepository (lazy lookup)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeRef {
    /// Имя типа для поиска в repository
    pub lookup_key: String,
    /// Кешированный hash для быстрого сравнения
    pub type_hash: u64,
}

impl TypeRef {
    pub fn new(lookup_key: &str) -> Self {
        Self {
            lookup_key: lookup_key.to_string(),
            type_hash: Self::hash_type_name(lookup_key),
        }
    }

    fn hash_type_name(name: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.to_lowercase().hash(&mut hasher);
        hasher.finish()
    }
}

// Расширение существующего TypeResolution
impl TypeResolution {
    /// Ссылка на RawTypeData для lazy lookup
    pub fn raw_type_ref(&self) -> Option<TypeRef> {
        match &self.result {
            ResolutionResult::Concrete(ConcreteType::Platform(pt)) => {
                Some(TypeRef::new(&pt.name))
            }
            ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => {
                Some(TypeRef::new(&format!("{}.{}", cfg.kind.to_prefix(), cfg.name)))
            }
            _ => None,
        }
    }
}
```

#### Задача 1.2: Добавить TypeCompatibility enum

```rust
// shared/src/domain/types.rs — добавить в конец файла

/// Результат сравнения совместимости типов
#[derive(Debug, Clone, PartialEq)]
pub enum TypeCompatibility {
    /// Полностью совместимы
    Compatible,
    /// Несовместимы с причиной
    Incompatible { reason: String },
    /// Частично совместимы (gradual typing)
    PartiallyCompatible { certainty: f32, reason: String },
}

impl TypeCompatibility {
    pub fn is_compatible(&self) -> bool {
        matches!(self, TypeCompatibility::Compatible | TypeCompatibility::PartiallyCompatible { .. })
    }

    pub fn reason(&self) -> String {
        match self {
            TypeCompatibility::Compatible => String::new(),
            TypeCompatibility::Incompatible { reason } => reason.clone(),
            TypeCompatibility::PartiallyCompatible { reason, .. } => reason.clone(),
        }
    }
}
```

#### Задача 1.3: Добавить метод is_compatible_with в TypeResolution

```rust
// shared/src/domain/types.rs — расширение impl TypeResolution

impl TypeResolution {
    /// Объектное сравнение типов с учётом семантики
    pub fn is_compatible_with(&self, other: &TypeResolution) -> TypeCompatibility {
        // Dynamic/Unknown совместимы со всем (gradual typing)
        if matches!(self.result, ResolutionResult::Dynamic)
            || matches!(other.result, ResolutionResult::Dynamic) {
            return TypeCompatibility::Compatible;
        }

        match (&self.result, &other.result) {
            // Concrete типы
            (ResolutionResult::Concrete(a), ResolutionResult::Concrete(b)) => {
                Self::check_concrete_compatibility(a, b, self.active_facet, other.active_facet)
            }

            // Union — actual должен быть совместим с хотя бы одним членом
            (_, ResolutionResult::Union(variants)) => {
                for variant in variants {
                    let variant_resolution = TypeResolution::known(variant.type_.clone());
                    if self.is_compatible_with(&variant_resolution).is_compatible() {
                        return TypeCompatibility::Compatible;
                    }
                }
                TypeCompatibility::Incompatible {
                    reason: "Не совместим ни с одним вариантом Union".to_string(),
                }
            }

            // Generic — проверяем base type и параметры
            (ResolutionResult::Generic(g1), ResolutionResult::Generic(g2)) => {
                Self::check_generic_compatibility(g1, g2)
            }

            // Nullable
            (ResolutionResult::Nullable(inner), other_result) => {
                let inner_resolution = TypeResolution::known(*inner.clone());
                inner_resolution.is_compatible_with(&TypeResolution {
                    result: other_result.clone(),
                    ..TypeResolution::unknown()
                })
            }

            _ => TypeCompatibility::Incompatible {
                reason: format!("Типы {:?} и {:?} несовместимы", self.result, other.result),
            },
        }
    }

    /// Проверка совместимости конкретных типов
    fn check_concrete_compatibility(
        from: &ConcreteType,
        to: &ConcreteType,
        from_facet: Option<FacetKind>,
        to_facet: Option<FacetKind>,
    ) -> TypeCompatibility {
        match (from, to) {
            // Примитивы — точное совпадение
            (ConcreteType::Primitive(a), ConcreteType::Primitive(b)) => {
                if a == b {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Примитивы {:?} и {:?} несовместимы", a, b),
                    }
                }
            }

            // Configuration типы — учитываем фасеты!
            (ConcreteType::Configuration(cfg1), ConcreteType::Configuration(cfg2)) => {
                if cfg1.kind != cfg2.kind || !Self::names_equal(&cfg1.name, &cfg2.name) {
                    return TypeCompatibility::Incompatible {
                        reason: format!("Разные типы конфигурации: {}.{} vs {}.{}",
                            cfg1.kind.to_prefix(), cfg1.name,
                            cfg2.kind.to_prefix(), cfg2.name),
                    };
                }
                // Проверяем фасетную совместимость
                Self::check_facet_compatibility(
                    from_facet.or(cfg1.facet),
                    to_facet.or(cfg2.facet),
                )
            }

            // Platform типы — case-insensitive сравнение имён
            (ConcreteType::Platform(pt1), ConcreteType::Platform(pt2)) => {
                if Self::names_equal(&pt1.name, &pt2.name) {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Платформенные типы {} и {} несовместимы", pt1.name, pt2.name),
                    }
                }
            }

            // Special типы
            (ConcreteType::Special(s1), ConcreteType::Special(s2)) => {
                if s1 == s2 {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Специальные типы {:?} и {:?} несовместимы", s1, s2),
                    }
                }
            }

            _ => TypeCompatibility::Incompatible {
                reason: "Несовместимые категории типов".to_string(),
            },
        }
    }

    /// Проверка фасетной совместимости
    fn check_facet_compatibility(
        from: Option<FacetKind>,
        to: Option<FacetKind>,
    ) -> TypeCompatibility {
        match (from, to) {
            (None, _) | (_, None) => TypeCompatibility::Compatible,
            (Some(f1), Some(f2)) if f1 == f2 => TypeCompatibility::Compatible,
            // Object → Reference: допустимо (неявная конвертация)
            (Some(FacetKind::Object), Some(FacetKind::Reference)) => TypeCompatibility::Compatible,
            // Reference → Object: НЕ допустимо (нужен ПолучитьОбъект())
            (Some(FacetKind::Reference), Some(FacetKind::Object)) => {
                TypeCompatibility::Incompatible {
                    reason: "Ссылка не может быть неявно преобразована в Объект (используйте ПолучитьОбъект())".to_string(),
                }
            }
            // Manager → любой другой: НЕ допустимо
            (Some(FacetKind::Manager), Some(other)) => {
                TypeCompatibility::Incompatible {
                    reason: format!("Менеджер несовместим с фасетом {:?}", other),
                }
            }
            (Some(f1), Some(f2)) => {
                TypeCompatibility::Incompatible {
                    reason: format!("Фасет {:?} несовместим с {:?}", f1, f2),
                }
            }
        }
    }

    /// Проверка совместимости Generic типов
    fn check_generic_compatibility(g1: &GenericType, g2: &GenericType) -> TypeCompatibility {
        // Проверяем базовый тип
        if !Self::names_equal(&g1.base_type, &g2.base_type) {
            return TypeCompatibility::Incompatible {
                reason: format!("Несовместимые базовые типы: {} vs {}", g1.base_type, g2.base_type),
            };
        }

        // Проверяем параметры
        if g1.type_params.len() != g2.type_params.len() {
            return TypeCompatibility::Incompatible {
                reason: "Разное количество параметров Generic".to_string(),
            };
        }

        for (p1, p2) in g1.type_params.iter().zip(g2.type_params.iter()) {
            let r1 = TypeResolution::known(p1.clone());
            let r2 = TypeResolution::known(p2.clone());
            if !r1.is_compatible_with(&r2).is_compatible() {
                return TypeCompatibility::Incompatible {
                    reason: format!("Параметры Generic {:?} и {:?} несовместимы", p1, p2),
                };
            }
        }

        TypeCompatibility::Compatible
    }

    fn names_equal(a: &str, b: &str) -> bool {
        a.to_lowercase() == b.to_lowercase()
    }
}
```

---

### Phase 2: Обновление TypeResolver (2-3 дня)

**Файл:** `shared/src/domain/resolver.rs` — замена существующего метода

#### Задача 2.1: Заменить is_type_compatible на объектную версию

```rust
// shared/src/domain/resolver.rs — ЗАМЕНА существующего метода

impl TypeResolver {
    /// Объектное сравнение типов (ЗАМЕНЯЕТ старый is_type_compatible)
    pub fn is_type_compatible_v2(
        &self,
        expected: &str,
        actual: &str,
    ) -> TypeCompatibility {
        let expected_resolution = self.resolve_expression_sync(expected);
        let actual_resolution = self.resolve_expression_sync(actual);

        actual_resolution.is_compatible_with(&expected_resolution)
    }

    /// Валидация вызова с объектным сравнением типов
    pub fn validate_call_v2(
        &self,
        type_name: Option<&str>,
        method_name: &str,
        arg_types: &[String],
        signature_index: &SignatureIndex,
    ) -> ValidationResultV2 {
        let signature = if let Some(type_name) = type_name {
            signature_index.find_method(type_name, method_name)
        } else {
            signature_index.find_global_function(method_name)
        };

        let signature = match signature {
            Some(sig) => sig,
            None => return ValidationResultV2::NotFound,
        };

        // Проверка количества параметров
        let required_count = signature.params.iter().filter(|p| !p.is_optional).count();
        if arg_types.len() < required_count {
            return ValidationResultV2::MissingRequiredParam {
                param_name: signature.params[arg_types.len()].name.clone(),
                param_index: arg_types.len(),
            };
        }
        if arg_types.len() > signature.params.len() {
            return ValidationResultV2::TooManyArgs {
                expected: signature.params.len(),
                actual: arg_types.len(),
            };
        }

        // Проверяем типы параметров с объектным сравнением
        for (i, (param, arg_type)) in signature.params.iter().zip(arg_types.iter()).enumerate() {
            if let Some(expected_type) = &param.type_name {
                let compat = self.is_type_compatible_v2(expected_type, arg_type);
                if !compat.is_compatible() {
                    return ValidationResultV2::TypeMismatch {
                        param_name: param.name.clone(),
                        param_index: i,
                        expected: expected_type.clone(),
                        actual: arg_type.clone(),
                        reason: compat.reason(),
                    };
                }
            }
        }

        ValidationResultV2::Ok(signature.return_type.clone())
    }
}

/// Результат валидации вызова v2
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResultV2 {
    Ok(Option<String>),
    NotFound,
    MissingRequiredParam { param_name: String, param_index: usize },
    TooManyArgs { expected: usize, actual: usize },
    TypeMismatch {
        param_name: String,
        param_index: usize,
        expected: String,
        actual: String,
        reason: String,
    },
}
```

---

### Phase 3: Интеграция в SemanticValidationVisitor (2-3 дня)

**Файл:** `backend/src/application/semantic_validation_visitor.rs` — обновление

#### Задача 3.1: Использовать validate_call_v2

```rust
// backend/src/application/semantic_validation_visitor.rs

impl<'a> SemanticValidationVisitor<'a> {
    fn validate_method_call_with_types(
        &mut self,
        obj_type: &str,
        method_name: &str,
        arg_types: &[String],
        span: Span,
    ) {
        // Используем новую версию с объектным сравнением
        let result = self.resolver.validate_call_v2(
            Some(obj_type),
            method_name,
            arg_types,
            self.signature_index,
        );

        match result {
            ValidationResultV2::TypeMismatch { param_name, expected, actual, reason, .. } => {
                self.errors.push(TypeDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: format!(
                        "Параметр '{}': ожидается {}, получено {}{}",
                        param_name,
                        expected,
                        actual,
                        if reason.is_empty() { String::new() } else { format!(" ({})", reason) }
                    ),
                    line: span.start_line,
                    column: span.start_column,
                    end_line: span.end_line,
                    end_column: span.end_column,
                });
            }
            ValidationResultV2::MissingRequiredParam { param_name, .. } => {
                self.errors.push(TypeDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: format!("Отсутствует обязательный параметр '{}'", param_name),
                    line: span.start_line,
                    column: span.start_column,
                    end_line: span.end_line,
                    end_column: span.end_column,
                });
            }
            _ => {}
        }
    }
}
```

---

### Тесты Phase 1-3

```rust
#[cfg(test)]
mod type_compatibility_tests {
    use super::*;

    #[test]
    fn test_primitive_compatibility() {
        let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));
        let number = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number));

        assert!(string.is_compatible_with(&string).is_compatible());
        assert!(!string.is_compatible_with(&number).is_compatible());
    }

    #[test]
    fn test_facet_compatibility_object_to_reference() {
        let obj = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::Object),
            })),
            active_facet: Some(FacetKind::Object),
            ..TypeResolution::unknown()
        };
        let ref_ = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::Reference),
            })),
            active_facet: Some(FacetKind::Reference),
            ..TypeResolution::unknown()
        };

        // Object → Reference: OK
        assert!(obj.is_compatible_with(&ref_).is_compatible());
        // Reference → Object: NOT OK
        assert!(!ref_.is_compatible_with(&obj).is_compatible());
    }

    #[test]
    fn test_union_compatibility() {
        let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));
        let union = TypeResolution {
            result: ResolutionResult::Union(vec![
                WeightedType { type_: ConcreteType::Primitive(PrimitiveType::String), weight: 0.5 },
                WeightedType { type_: ConcreteType::Primitive(PrimitiveType::Number), weight: 0.5 },
            ]),
            ..TypeResolution::unknown()
        };

        assert!(string.is_compatible_with(&union).is_compatible());
    }

    #[test]
    fn test_dynamic_compatible_with_everything() {
        let dynamic = TypeResolution::unknown(); // result = Dynamic
        let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));

        assert!(dynamic.is_compatible_with(&string).is_compatible());
        assert!(string.is_compatible_with(&dynamic).is_compatible());
    }
}
```

### Результат Milestone 3.13

- ✅ **TypeRef** — lazy ссылка на RawTypeData
- ✅ **TypeCompatibility** — результат объектного сравнения
- ✅ **is_compatible_with()** — метод TypeResolution для объектного сравнения
- ✅ Сравнение с учётом фасетов (Object → Reference: OK, Reference → Object: ERROR)
- ✅ Сравнение Generic типов с параметрами
- ✅ **validate_call_v2()** — валидация с объектным сравнением
- ✅ Интеграция в SemanticValidationVisitor
- ✅ 20+ unit тестов

**Зависимости:**
- ✅ Milestone 3.10 (валидация параметров)
- ✅ Milestone 3.11 (фасетная система)

---

## 📦 Milestone 3.14: Go To Definition для типов

**Приоритет:** 🟡 СРЕДНИЙ — улучшение навигации по коду
**Оценка времени:** 1-1.5 недели

### Проблема

При наведении на переменную или метод нет возможности перейти к определению типа.

### Цель

Реализовать LSP `textDocument/definition` для навигации к определениям типов.

---

### Phase 1: TypeDefinitionLocation (2-3 дня)

**Файл:** `shared/src/domain/type_definition_location.rs` — **НОВЫЙ файл**

> **Примечание:** Это НЕ дублирует CodeLocation!
> - `CodeLocation` — где находится **код** (модуль формы, объекта, контекст Server/Client)
> - `TypeDefinitionLocation` — где определён **тип** (платформа, конфигурация, пользовательский)

#### Задача 1.1: Создать TypeDefinitionLocation

```rust
// shared/src/domain/type_definition_location.rs — НОВЫЙ файл

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Местоположение определения типа
///
/// НЕ путать с CodeLocation (контекст выполнения кода)!
/// TypeDefinitionLocation указывает где ОПРЕДЕЛЁН тип, а не где используется.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeDefinitionLocation {
    /// Платформенный тип — определён в платформе 1С
    /// Нет исходного файла, только ссылка на документацию
    Platform {
        type_name: String,
        /// URI для Syntax Helper документации (опционально)
        docs_uri: Option<String>,
    },

    /// Конфигурационный тип — определён в метаданных конфигурации
    Configuration {
        /// Путь к файлу метаданных (.xml)
        metadata_path: PathBuf,
        /// Пути к модулям (если есть)
        module_paths: ModulePaths,
    },

    /// Пользовательский тип — определён в BSL коде
    UserDefined {
        file_path: PathBuf,
        /// Позиция определения
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
    },

    /// Примитивный тип — встроен в язык, нет определения
    Primitive,
}

/// Пути к модулям конфигурационного типа
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModulePaths {
    /// Модуль объекта (Ext/ObjectModule.bsl)
    pub object_module: Option<PathBuf>,
    /// Модуль менеджера (Ext/ManagerModule.bsl)
    pub manager_module: Option<PathBuf>,
    /// Модуль набора записей (Ext/RecordSetModule.bsl)
    pub recordset_module: Option<PathBuf>,
}

impl TypeDefinitionLocation {
    /// Создать location для платформенного типа
    pub fn platform(type_name: &str) -> Self {
        Self::Platform {
            type_name: type_name.to_string(),
            docs_uri: Some(format!("bsl://docs/{}", type_name)),
        }
    }

    /// Создать location для конфигурационного типа
    pub fn configuration(metadata_path: PathBuf) -> Self {
        Self::Configuration {
            metadata_path,
            module_paths: ModulePaths::default(),
        }
    }

    /// Создать location для примитива
    pub fn primitive() -> Self {
        Self::Primitive
    }

    /// Получить основной путь для навигации (если есть)
    pub fn primary_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Configuration { module_paths, metadata_path, .. } => {
                // Приоритет: object_module > manager_module > metadata_path
                module_paths.object_module.as_ref()
                    .or(module_paths.manager_module.as_ref())
                    .or(Some(metadata_path))
            }
            Self::UserDefined { file_path, .. } => Some(file_path),
            _ => None,
        }
    }
}
```

#### Задача 1.2: Добавить метод в TypeResolution

```rust
// shared/src/domain/types.rs — расширение

impl TypeResolution {
    /// Получить местоположение определения типа
    pub fn get_definition_location(&self, repository: &dyn TypeRepository) -> Option<TypeDefinitionLocation> {
        match &self.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(_)) => {
                Some(TypeDefinitionLocation::Primitive)
            }

            ResolutionResult::Concrete(ConcreteType::Platform(pt)) => {
                Some(TypeDefinitionLocation::platform(&pt.name))
            }

            ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => {
                let type_key = format!("{}.{}", cfg.kind.to_prefix(), cfg.name);
                // TODO: Получить реальные пути из repository или config parser
                Some(TypeDefinitionLocation::configuration(PathBuf::from(&type_key)))
            }

            ResolutionResult::Generic(gen) => {
                // Для Generic возвращаем location базового типа
                Some(TypeDefinitionLocation::platform(&gen.base_type))
            }

            _ => None,
        }
    }
}
```

---

### Phase 2: LSP Integration (3-4 дня)

#### Задача 2.1: LSP handler для textDocument/definition

```rust
// backend/src/bin/lsp_server.rs — добавить handler

async fn handle_goto_definition(
    &self,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>, ResponseError> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    // 1. Получаем IR для файла
    let ir = self.get_semantic_program(&uri).await?;

    // 2. Находим узел в позиции
    let node = match ir.find_node_at_position(position.line, position.character) {
        Some(n) => n,
        None => return Ok(None),
    };

    // 3. Определяем тип узла
    let type_resolution = self.resolve_node_type(&node, &ir)?;

    // 4. Получаем location определения
    let location = match type_resolution.get_definition_location(&*self.repository) {
        Some(loc) => loc,
        None => return Ok(None),
    };

    // 5. Конвертируем в LSP Location
    match location {
        TypeDefinitionLocation::Configuration { module_paths, .. } => {
            if let Some(path) = module_paths.object_module.or(module_paths.manager_module) {
                return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: Url::from_file_path(&path).map_err(|_| ResponseError::new(
                        ErrorCode::InvalidParams,
                        format!("Invalid path: {:?}", path),
                    ))?,
                    range: Range::default(),
                })));
            }
        }
        TypeDefinitionLocation::UserDefined { file_path, start_line, start_column, end_line, end_column } => {
            return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                uri: Url::from_file_path(&file_path).map_err(|_| ResponseError::new(
                    ErrorCode::InvalidParams,
                    format!("Invalid path: {:?}", file_path),
                ))?,
                range: Range {
                    start: Position { line: start_line, character: start_column },
                    end: Position { line: end_line, character: end_column },
                },
            })));
        }
        _ => {}
    }

    Ok(None)
}
```

#### Задача 2.2: Добавить ссылки в Hover response

```rust
// backend/src/helpers/hover_formatter.rs — расширение

impl HoverFormatter {
    pub fn format_with_definition_link(
        &self,
        type_resolution: &TypeResolution,
        repository: &dyn TypeRepository,
    ) -> String {
        let mut content = self.format_type_info(type_resolution);

        if let Some(location) = type_resolution.get_definition_location(repository) {
            match location {
                TypeDefinitionLocation::Configuration { module_paths, .. } => {
                    if let Some(path) = module_paths.object_module {
                        content.push_str(&format!(
                            "\n\n[Перейти к модулю объекта](file://{})",
                            path.display()
                        ));
                    }
                }
                TypeDefinitionLocation::Platform { type_name, docs_uri } => {
                    if let Some(uri) = docs_uri {
                        content.push_str(&format!(
                            "\n\n[Документация {}]({})",
                            type_name, uri
                        ));
                    }
                }
                _ => {}
            }
        }

        content
    }
}
```

### Результат Milestone 3.14

- ✅ **TypeDefinitionLocation** — местоположение определения типа (отдельно от CodeLocation!)
- ✅ LSP `textDocument/definition` для конфигурационных типов
- ✅ Ссылки на определения в Hover
- ✅ Поддержка навигации к модулям объекта/менеджера
- ✅ 15+ интеграционных тестов

**Зависимости:**
- ✅ Milestone 3.13 (TypeRef, объектное сравнение)
- ✅ Milestone 3.12 (config parser с путями к модулям)

---

## 📦 Milestone 3.15: Lazy Resolution с OnceCell

**Приоритет:** 🟡 СРЕДНИЙ — оптимизация производительности
**Оценка времени:** 0.5-1 неделя

### Проблема

`MethodSignature.return_type` хранится как `Option<String>`, что требует повторного резолвинга при каждом использовании.

### Цель

Добавить lazy resolution через OnceCell для кэширования резолвленных типов.

---

### Задачи

#### Задача 1: Расширить MethodSignature с OnceCell

```rust
// shared/src/domain/signature_index.rs — расширение

use once_cell::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub owner_type: Option<String>,
    pub params: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub source: SignatureSource,
    pub return_facet: Option<FacetKind>,
    pub context_requirements: ContextRequirements,

    // Lazy resolved типы (не сериализуются)
    #[serde(skip)]
    resolved_return: OnceCell<Option<TypeResolution>>,
    #[serde(skip)]
    resolved_params: OnceCell<Vec<(String, TypeResolution)>>,
}

impl MethodSignature {
    /// Получить резолвленный тип возврата (lazy)
    pub fn get_resolved_return_type(&self, resolver: &TypeResolver) -> Option<&TypeResolution> {
        self.resolved_return.get_or_init(|| {
            self.return_type.as_ref().map(|rt| resolver.resolve_expression_sync(rt))
        }).as_ref()
    }

    /// Получить резолвленные типы параметров (lazy)
    pub fn get_resolved_params(&self, resolver: &TypeResolver) -> &[(String, TypeResolution)] {
        self.resolved_params.get_or_init(|| {
            self.params.iter()
                .map(|p| {
                    let resolved = p.type_name.as_ref()
                        .map(|t| resolver.resolve_expression_sync(t))
                        .unwrap_or_else(TypeResolution::unknown);
                    (p.name.clone(), resolved)
                })
                .collect()
        })
    }

    /// Сбросить кэш (при изменении типов)
    pub fn clear_cache(&mut self) {
        self.resolved_return = OnceCell::new();
        self.resolved_params = OnceCell::new();
    }
}
```

#### Задача 2: Pre-warm кэш при загрузке

```rust
// backend/src/application/type_system_service.rs — расширение

impl TypeSystemService {
    /// Предварительно резолвить типы для всех сигнатур
    pub fn prewarm_signature_cache(&self) {
        let signature_index = self.repository.get_signature_index_clone();

        // Pre-resolve return types для часто используемых типов
        for type_name in &["Массив", "ТаблицаЗначений", "Строка", "Число"] {
            if let Some(methods) = signature_index.get_methods_for_type(type_name) {
                for method in methods {
                    let _ = method.get_resolved_return_type(&self.resolver);
                }
            }
        }

        log::info!("Signature cache pre-warmed");
    }
}
```

### Результат Milestone 3.15

- ✅ **OnceCell** для lazy resolution в MethodSignature
- ✅ Кэширование резолвленных return types
- ✅ Кэширование резолвленных типов параметров
- ✅ Pre-warm кэш при загрузке
- ✅ 10+ тестов производительности

**Зависимости:**
- ✅ Milestone 3.13 (TypeResolution.is_compatible_with)

---

## 📊 Summary

| Milestone | Приоритет | Время | Ключевой результат |
|-----------|-----------|-------|-------------------|
| **3.13** Object-Based Type Comparison | 🔴 ВЫСОКИЙ | 1.5-2 нед. | TypeCompatibility, is_compatible_with(), фасеты |
| **3.14** Go To Definition | 🟡 СРЕДНИЙ | 1-1.5 нед. | TypeDefinitionLocation, LSP definition |
| **3.15** Lazy Resolution | 🟡 СРЕДНИЙ | 0.5-1 нед. | OnceCell, кэширование |

**Общее время:** 3-4.5 недели (было 5-6.5)

### Архитектурные принципы

1. ✅ **НЕ создаём ResolvedType** — расширяем существующий TypeResolution
2. ✅ **НЕ создаём TypeResolutionService** — используем TypeResolver + TypeSystemService
3. ✅ **TypeDefinitionLocation ≠ CodeLocation** — разные концепции (где определён тип vs где код)
4. ✅ **Lazy lookup через TypeRef и OnceCell** — не загружаем всё в память
5. ✅ **Обратная совместимость** — старые API (is_type_compatible) продолжают работать

---

## Изменённые файлы

### Milestone 3.13
| Файл | Изменение |
|------|-----------|
| `shared/src/domain/types.rs` | Добавить TypeRef, TypeCompatibility, is_compatible_with() |
| `shared/src/domain/resolver.rs` | Добавить is_type_compatible_v2(), validate_call_v2() |
| `backend/src/application/semantic_validation_visitor.rs` | Использовать validate_call_v2() |

### Milestone 3.14
| Файл | Изменение |
|------|-----------|
| `shared/src/domain/type_definition_location.rs` | **НОВЫЙ** — TypeDefinitionLocation enum |
| `shared/src/domain/mod.rs` | Экспорт type_definition_location |
| `shared/src/domain/types.rs` | Добавить get_definition_location() |
| `backend/src/bin/lsp_server.rs` | Добавить handle_goto_definition() |
| `backend/src/helpers/hover_formatter.rs` | Добавить format_with_definition_link() |

### Milestone 3.15
| Файл | Изменение |
|------|-----------|
| `shared/src/domain/signature_index.rs` | Добавить OnceCell поля, get_resolved_*() методы |
| `backend/src/application/type_system_service.rs` | Добавить prewarm_signature_cache() |

---

## Checklist для реализации

### Milestone 3.13
- [ ] Добавить `TypeRef` в `types.rs`
- [ ] Добавить `TypeCompatibility` enum
- [ ] Реализовать `is_compatible_with()` в TypeResolution
- [ ] Реализовать `check_facet_compatibility()`
- [ ] Добавить `is_type_compatible_v2()` в TypeResolver
- [ ] Добавить `ValidationResultV2` enum
- [ ] Добавить `validate_call_v2()` в TypeResolver
- [ ] Обновить SemanticValidationVisitor
- [ ] Написать 20+ unit тестов

### Milestone 3.14
- [ ] Создать `shared/src/domain/type_definition_location.rs`
- [ ] Реализовать `TypeDefinitionLocation` enum
- [ ] Добавить `get_definition_location()` в TypeResolution
- [ ] Добавить LSP handler `textDocument/definition`
- [ ] Добавить ссылки в Hover response
- [ ] Написать 15+ интеграционных тестов

### Milestone 3.15
- [ ] Добавить `OnceCell` поля в MethodSignature
- [ ] Реализовать `get_resolved_return_type()`
- [ ] Реализовать `get_resolved_params()`
- [ ] Добавить `prewarm_signature_cache()`
- [ ] Написать 10+ тестов производительности
