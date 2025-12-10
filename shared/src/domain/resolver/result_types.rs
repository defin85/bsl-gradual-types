//! Domain Layer: Type Resolver Result Types
//!
//! Типы результатов резолюции и валидации типов

/// Результат валидации вызова функции
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Вызов корректен, возвращает тип (если функция возвращает значение)
    Ok(Option<String>),
    /// Отсутствует обязательный параметр
    MissingRequiredParam {
        param_name: String,
        param_index: usize,
    },
    /// Слишком много аргументов
    TooManyArgs { expected: usize, actual: usize },
    /// Несоответствие типов аргумента
    TypeMismatch {
        param_name: String,
        expected: String,
        actual: String,
    },
    /// Функция/метод не найдены
    NotFound,
}

/// Результат резолвинга конструктора (Milestone 2.21)
#[derive(Debug, Clone, PartialEq)]
pub enum ConstructorResolution {
    /// Успешно резолвлен
    Resolved {
        /// Результирующий тип
        type_name: String,

        /// Facet результата (если есть)
        facet: Option<String>,

        /// Generic параметры (для коллекций)
        /// Массив<Число> → Some(vec!["Число"])
        generic_params: Option<Vec<String>>,

        /// Ошибки валидации параметров
        validation_errors: Vec<String>,
    },

    /// Конструктор не найден
    NotFound { type_name: String, hint: String },

    /// Динамический конструктор с неизвестным типом
    Dynamic {
        /// Невозможно определить тип статически
        reason: String,
    },
}

/// Результат валидации вызова v2 (с объектным сравнением типов)
/// Milestone 3.13: Object-Based Type Comparison
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResultV2 {
    /// Вызов корректен, возвращает тип (если функция возвращает значение)
    Ok(Option<String>),
    /// Функция/метод не найдены
    NotFound,
    /// Отсутствует обязательный параметр
    MissingRequiredParam { param_name: String, param_index: usize },
    /// Слишком много аргументов
    TooManyArgs { expected: usize, actual: usize },
    /// Несоответствие типов аргумента (с детальной причиной)
    TypeMismatch {
        param_name: String,
        param_index: usize,
        expected: String,
        actual: String,
        reason: String,
    },
}
