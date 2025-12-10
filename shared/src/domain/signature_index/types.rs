//! Вспомогательные типы для SignatureIndex
//!
//! Содержит типы, не связанные напрямую с методами или индексом:
//! - ConstructorSignature
//! - SignatureSource
//! - SignatureValidationResult
//! - SignatureMismatch

use super::super::types::ParameterInfo;
use serde::{Deserialize, Serialize};

/// Сигнатура конструктора
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorSignature {
    /// Имя типа ("Массив", "ТаблицаЗначений")
    pub type_name: String,

    /// Параметры конструктора
    pub params: Vec<ParameterInfo>,

    /// Результирующий facet (Object, Reference, Manager)
    /// None означает что конструктор возвращает сам тип
    pub facet: Option<String>,

    /// Источник сигнатуры
    pub source: SignatureSource,

    /// Является ли тип коллекцией (для generic inference)
    pub is_collection: bool,

    /// Количество generic параметров для коллекций
    /// Массив -> 1, Соответствие -> 2, СписокЗначений -> 1
    pub generic_params_count: usize,
}

/// Источник сигнатуры
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureSource {
    Platform,
    Configuration,
    UserCode, // Код пользователя (для валидации)
}

/// Результат валидации сигнатуры
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureValidationResult {
    /// Сигнатура валидна
    Valid,
    /// Сигнатура невалидна (список несоответствий)
    Invalid(Vec<SignatureMismatch>),
}

/// Несоответствие в сигнатуре
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureMismatch {
    /// Неправильное количество параметров
    ParameterCount { expected: usize, actual: usize },
    /// Неправильный тип параметра
    ParameterType {
        index: usize,
        param_name: String,
        expected: String,
        actual: String,
    },
    /// Неправильный тип возврата
    ReturnType {
        expected: Option<String>,
        actual: Option<String>,
    },
}
