//! Type Guard Detection
//!
//! Обнаружение паттернов проверки типов в коде 1С:
//! - ТипЗнч(x) = Тип("Строка")
//! - x <> Неопределено
//! - ЗначениеЗаполнено(x)
//!
//! Milestone 3.7: Advanced Type Narrowing

use crate::domain::types::{ConcreteType, PlatformType, TypeResolution};

/// Типы type guards, которые можно обнаружить в коде
#[derive(Debug, Clone, PartialEq)]
pub enum TypeGuard {
    /// Проверка типа через ТипЗнч(): ТипЗнч(x) = Тип("Строка")
    TypeCheck {
        variable: String,
        expected_type: String,
    },

    /// Проверка на Неопределено: x <> Неопределено
    NotUndefined { variable: String },

    /// Проверка на заполненность: ЗначениеЗаполнено(x)
    ValueFilled { variable: String },

    /// Проверка на Null: x = Null
    IsNull { variable: String },

    /// Проверка на пустую строку: x <> ""
    NotEmptyString { variable: String },

    /// Проверка на 0: x <> 0
    NotZero { variable: String },

    /// Проверка на истинность: x = Истина
    IsTrue { variable: String },

    /// Проверка на ложность: x = Ложь
    IsFalse { variable: String },
}

impl TypeGuard {
    /// Применить type guard для сужения типа
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// let guard = TypeGuard::TypeCheck {
    ///     variable: "x".to_string(),
    ///     expected_type: "Строка".to_string(),
    /// };
    /// let narrowed = guard.apply_narrowing(&current_type);
    /// // narrowed теперь имеет тип Строка вместо Any
    /// ```
    pub fn apply_narrowing(&self, current: &TypeResolution) -> TypeResolution {
        match self {
            TypeGuard::TypeCheck {
                variable: _,
                expected_type,
            } => {
                // Сужаем до конкретного типа
                TypeResolution::known(ConcreteType::Platform(PlatformType {
                    name: expected_type.clone(),
                }))
            }

            TypeGuard::NotUndefined { variable: _ } => {
                // Удаляем Undefined из union type
                use crate::domain::types::{Certainty, ResolutionResult, ResolutionSource};

                match &current.result {
                    ResolutionResult::Union(types) => {
                        // Фильтруем Неопределено из union
                        let filtered: Vec<_> = types
                            .iter()
                            .filter(|wt| {
                                if let ConcreteType::Platform(pt) = &wt.type_ {
                                    pt.name != "Неопределено"
                                } else {
                                    true
                                }
                            })
                            .cloned()
                            .collect();

                        if filtered.len() == 1 {
                            // Остался один тип
                            TypeResolution::known(filtered[0].type_.clone())
                        } else if !filtered.is_empty() {
                            // Остался union без Неопределено
                            TypeResolution {
                                certainty: Certainty::Inferred(0.8),
                                result: ResolutionResult::Union(filtered),
                                source: ResolutionSource::Inferred,
                                metadata: current.metadata.clone(),
                                active_facet: None,
                                available_facets: vec![],
                            }
                        } else {
                            current.clone()
                        }
                    }
                    ResolutionResult::Nullable(inner) => {
                        // Убираем nullable обёртку
                        TypeResolution::known(inner.as_ref().clone())
                    }
                    _ => current.clone(),
                }
            }

            TypeGuard::ValueFilled { variable: _ } => {
                // ЗначениеЗаполнено() исключает: Неопределено, Null, "", 0, Ложь
                use crate::domain::types::{Certainty, ResolutionResult, ResolutionSource};

                match &current.result {
                    ResolutionResult::Union(types) => {
                        let filtered: Vec<_> = types
                            .iter()
                            .filter(|wt| {
                                if let ConcreteType::Platform(pt) = &wt.type_ {
                                    !matches!(pt.name.as_str(), "Неопределено" | "Null" | "Ложь")
                                } else {
                                    true
                                }
                            })
                            .cloned()
                            .collect();

                        if !filtered.is_empty() {
                            TypeResolution {
                                certainty: Certainty::Inferred(0.8),
                                result: ResolutionResult::Union(filtered),
                                source: ResolutionSource::Inferred,
                                metadata: current.metadata.clone(),
                                active_facet: None,
                                available_facets: vec![],
                            }
                        } else {
                            current.clone()
                        }
                    }
                    _ => current.clone(),
                }
            }

            TypeGuard::IsNull { variable: _ } => {
                TypeResolution::known(ConcreteType::Platform(PlatformType {
                    name: "Null".to_string(),
                }))
            }

            TypeGuard::NotEmptyString { variable: _ } => {
                // Гарантируем непустую строку
                TypeResolution::known(ConcreteType::Platform(PlatformType {
                    name: "Строка".to_string(),
                }))
            }

            TypeGuard::NotZero { variable: _ } => {
                // Гарантируем ненулевое число
                TypeResolution::known(ConcreteType::Platform(PlatformType {
                    name: "Число".to_string(),
                }))
            }

            TypeGuard::IsTrue { variable: _ } => {
                TypeResolution::known(ConcreteType::Platform(PlatformType {
                    name: "Булево".to_string(),
                }))
            }

            TypeGuard::IsFalse { variable: _ } => {
                TypeResolution::known(ConcreteType::Platform(PlatformType {
                    name: "Булево".to_string(),
                }))
            }
        }
    }

    /// Получить имя переменной, к которой применяется guard
    pub fn variable_name(&self) -> &str {
        match self {
            TypeGuard::TypeCheck { variable, .. } => variable,
            TypeGuard::NotUndefined { variable } => variable,
            TypeGuard::ValueFilled { variable } => variable,
            TypeGuard::IsNull { variable } => variable,
            TypeGuard::NotEmptyString { variable } => variable,
            TypeGuard::NotZero { variable } => variable,
            TypeGuard::IsTrue { variable } => variable,
            TypeGuard::IsFalse { variable } => variable,
        }
    }
}

/// Обнаружение type guards в условном выражении
///
/// # Примеры
///
/// ```ignore
/// let guards = detect_type_guards("ТипЗнч(Параметр) = Тип(\"Число\")");
/// // Вернёт: vec![TypeGuard::TypeCheck { variable: "Параметр", expected_type: "Число" }]
/// ```
pub fn detect_type_guards(condition: &str) -> Vec<TypeGuard> {
    let mut guards = Vec::new();

    // Нормализуем условие (убираем лишние пробелы)
    let normalized = condition.trim();

    // Паттерн 1: ТипЗнч(x) = Тип("...")
    if let Some(guard) = detect_type_check(normalized) {
        guards.push(guard);
    }

    // Паттерн 2: x <> Неопределено
    if let Some(guard) = detect_not_undefined(normalized) {
        guards.push(guard);
    }

    // Паттерн 3: ЗначениеЗаполнено(x)
    if let Some(guard) = detect_value_filled(normalized) {
        guards.push(guard);
    }

    // Паттерн 4: x = Null
    if let Some(guard) = detect_is_null(normalized) {
        guards.push(guard);
    }

    // Паттерн 5: x <> ""
    if let Some(guard) = detect_not_empty_string(normalized) {
        guards.push(guard);
    }

    // Паттерн 6: x <> 0
    if let Some(guard) = detect_not_zero(normalized) {
        guards.push(guard);
    }

    // Паттерн 7: x = Истина / x = Ложь
    if let Some(guard) = detect_boolean_check(normalized) {
        guards.push(guard);
    }

    guards
}

/// Обнаружение паттерна: ТипЗнч(x) = Тип("...")
fn detect_type_check(condition: &str) -> Option<TypeGuard> {
    // Простой парсинг для ТипЗнч(переменная) = Тип("тип")
    if condition.contains("ТипЗнч") && condition.contains("=") && condition.contains("Тип")
    {
        // Извлекаем переменную из ТипЗнч(...)
        if let Some(var_start) = condition.find("ТипЗнч(") {
            let after_open = &condition[var_start + "ТипЗнч(".len()..];
            if let Some(var_end) = after_open.find(')') {
                let variable = after_open[..var_end].trim().to_string();

                // Извлекаем тип из Тип("...")
                if let Some(type_start) = condition.find("Тип(\"") {
                    let after_type = &condition[type_start + "Тип(\"".len()..];
                    if let Some(type_end) = after_type.find("\"") {
                        let expected_type = after_type[..type_end].trim().to_string();

                        return Some(TypeGuard::TypeCheck {
                            variable,
                            expected_type,
                        });
                    }
                }
            }
        }
    }
    None
}

/// Обнаружение паттерна: x <> Неопределено
fn detect_not_undefined(condition: &str) -> Option<TypeGuard> {
    if condition.contains("<>") && condition.contains("Неопределено") {
        // Извлекаем переменную до <>
        if let Some(pos) = condition.find("<>") {
            let variable = condition[..pos].trim().to_string();
            return Some(TypeGuard::NotUndefined { variable });
        }
    }
    None
}

/// Обнаружение паттерна: ЗначениеЗаполнено(x)
fn detect_value_filled(condition: &str) -> Option<TypeGuard> {
    if condition.contains("ЗначениеЗаполнено") {
        if let Some(var_start) = condition.find("ЗначениеЗаполнено(") {
            let after_open = &condition[var_start + "ЗначениеЗаполнено(".len()..];
            if let Some(var_end) = after_open.find(')') {
                let variable = after_open[..var_end].trim().to_string();
                return Some(TypeGuard::ValueFilled { variable });
            }
        }
    }
    None
}

/// Обнаружение паттерна: x = Null
fn detect_is_null(condition: &str) -> Option<TypeGuard> {
    if condition.contains("=") && condition.contains("Null") {
        if let Some(pos) = condition.find('=') {
            let before = condition[..pos].trim();
            let after = condition[pos + 1..].trim();

            if after == "Null" {
                return Some(TypeGuard::IsNull {
                    variable: before.to_string(),
                });
            }
        }
    }
    None
}

/// Обнаружение паттерна: x <> ""
fn detect_not_empty_string(condition: &str) -> Option<TypeGuard> {
    if condition.contains("<>") && condition.contains("\"\"") {
        if let Some(pos) = condition.find("<>") {
            let variable = condition[..pos].trim().to_string();
            return Some(TypeGuard::NotEmptyString { variable });
        }
    }
    None
}

/// Обнаружение паттерна: x <> 0
fn detect_not_zero(condition: &str) -> Option<TypeGuard> {
    if condition.contains("<>") && condition.contains('0') {
        if let Some(pos) = condition.find("<>") {
            let before = condition[..pos].trim();
            let after = condition[pos + 2..].trim();

            if after == "0" {
                return Some(TypeGuard::NotZero {
                    variable: before.to_string(),
                });
            }
        }
    }
    None
}

/// Обнаружение паттерна: x = Истина / x = Ложь
fn detect_boolean_check(condition: &str) -> Option<TypeGuard> {
    if condition.contains('=') {
        if let Some(pos) = condition.find('=') {
            let before = condition[..pos].trim();
            let after = condition[pos + 1..].trim();

            if after == "Истина" {
                return Some(TypeGuard::IsTrue {
                    variable: before.to_string(),
                });
            } else if after == "Ложь" {
                return Some(TypeGuard::IsFalse {
                    variable: before.to_string(),
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_type_check() {
        let guards = detect_type_guards("ТипЗнч(Параметр) = Тип(\"Число\")");
        assert_eq!(guards.len(), 1);
        assert!(matches!(guards[0], TypeGuard::TypeCheck { .. }));

        if let TypeGuard::TypeCheck {
            variable,
            expected_type,
        } = &guards[0]
        {
            assert_eq!(variable, "Параметр");
            assert_eq!(expected_type, "Число");
        }
    }

    #[test]
    fn test_detect_not_undefined() {
        let guards = detect_type_guards("Параметр <> Неопределено");
        assert_eq!(guards.len(), 1);
        assert!(matches!(guards[0], TypeGuard::NotUndefined { .. }));

        if let TypeGuard::NotUndefined { variable } = &guards[0] {
            assert_eq!(variable, "Параметр");
        }
    }

    #[test]
    fn test_detect_value_filled() {
        let guards = detect_type_guards("ЗначениеЗаполнено(Объект)");
        assert_eq!(guards.len(), 1);
        assert!(matches!(guards[0], TypeGuard::ValueFilled { .. }));

        if let TypeGuard::ValueFilled { variable } = &guards[0] {
            assert_eq!(variable, "Объект");
        }
    }

    #[test]
    fn test_detect_is_null() {
        let guards = detect_type_guards("x = Null");
        assert_eq!(guards.len(), 1);
        assert!(matches!(guards[0], TypeGuard::IsNull { .. }));
    }

    #[test]
    fn test_detect_not_empty_string() {
        let guards = detect_type_guards("Строка <> \"\"");
        assert_eq!(guards.len(), 1);
        assert!(matches!(guards[0], TypeGuard::NotEmptyString { .. }));
    }

    #[test]
    fn test_detect_not_zero() {
        let guards = detect_type_guards("Число <> 0");
        assert_eq!(guards.len(), 1);
        assert!(matches!(guards[0], TypeGuard::NotZero { .. }));
    }

    #[test]
    fn test_detect_boolean() {
        let guards = detect_type_guards("Флаг = Истина");
        assert_eq!(guards.len(), 1);
        assert!(matches!(guards[0], TypeGuard::IsTrue { .. }));

        let guards2 = detect_type_guards("Флаг = Ложь");
        assert_eq!(guards2.len(), 1);
        assert!(matches!(guards2[0], TypeGuard::IsFalse { .. }));
    }

    #[test]
    fn test_apply_type_check_narrowing() {
        use crate::domain::types::{ConcreteType, TypeResolution};

        let current = TypeResolution::unknown(); // Any

        let guard = TypeGuard::TypeCheck {
            variable: "x".to_string(),
            expected_type: "Строка".to_string(),
        };

        let narrowed = guard.apply_narrowing(&current);

        // Проверяем, что тип сузился до Строка
        if let crate::domain::types::ResolutionResult::Concrete(ConcreteType::Platform(pt)) =
            &narrowed.result
        {
            assert_eq!(pt.name, "Строка");
        } else {
            panic!("Expected Concrete(Platform(Строка))");
        }
    }

    #[test]
    fn test_apply_not_undefined_narrowing() {
        use crate::domain::types::{
            Certainty, ConcreteType, PlatformType, ResolutionResult, TypeResolution, WeightedType,
        };

        // Union: Строка | Неопределено
        let current = TypeResolution {
            certainty: Certainty::Inferred(0.7),
            result: ResolutionResult::Union(vec![
                WeightedType {
                    type_: ConcreteType::Platform(PlatformType {
                        name: "Строка".to_string(),
                    }),
                    weight: 0.5,
                },
                WeightedType {
                    type_: ConcreteType::Platform(PlatformType {
                        name: "Неопределено".to_string(),
                    }),
                    weight: 0.5,
                },
            ]),
            source: crate::domain::types::ResolutionSource::Inferred,
            metadata: Default::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let guard = TypeGuard::NotUndefined {
            variable: "x".to_string(),
        };

        let narrowed = guard.apply_narrowing(&current);

        // Должен остаться только Строка
        if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &narrowed.result {
            assert_eq!(pt.name, "Строка");
        } else {
            panic!("Expected Concrete(Platform(Строка)), got: {:?}", narrowed);
        }
    }

    #[test]
    fn test_variable_name() {
        let guard = TypeGuard::TypeCheck {
            variable: "Параметр".to_string(),
            expected_type: "Число".to_string(),
        };
        assert_eq!(guard.variable_name(), "Параметр");

        let guard2 = TypeGuard::NotUndefined {
            variable: "Объект".to_string(),
        };
        assert_eq!(guard2.variable_name(), "Объект");
    }
}
