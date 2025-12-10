//! Type error kinds based on Balyuk & Popova (2021) research
//!
//! This module defines the categories of type errors detected by the type checker.

use crate::domain::code_location::CompilerDirective;
use crate::domain::runtime_context::ContextRequirements;
use crate::domain::types::MetadataKind;

/// Категории ошибок типизации из статьи Balyuk & Popova
#[derive(Debug, Clone, PartialEq)]
pub enum TypeErrorKind {
    /// Некорректная передача параметров методу
    IncorrectParameterType {
        method_name: String,
        param_index: usize,
        expected: String,
        actual: String,
        variable_name: Option<String>,      // MILESTONE 3.6 Phase 3: переменная объекта
        param_variable_name: Option<String>, // MILESTONE 3.6 Phase 3: переменная параметра
    },
    /// Обращение к несуществующему свойству объекта
    NonExistentProperty {
        object_type: String,
        property_name: String,
        variable_name: Option<String>,  // MILESTONE 3.6 Phase 3: имя переменной
    },
    /// Обращение к несуществующему методу объекта
    NonExistentMethod {
        object_type: String,
        method_name: String,
        variable_name: Option<String>,  // MILESTONE 3.6 Phase 3: имя переменной
    },
    /// Обработка простого типа как коллекции
    SimpleTypeAsCollection {
        type_name: String,
        operation: String,
        variable_name: Option<String>,  // MILESTONE 3.6 Phase 3: имя переменной
    },
    /// Метод недоступен в текущем контексте выполнения (MILESTONE 3.11 Phase 3)
    MethodNotAvailableInContext {
        method_name: String,
        object_type: String,
        variable_name: Option<String>,
        current_context: CompilerDirective,      // Type-safe context (OnClient, OnServer, etc.)
        required_context: ContextRequirements,   // Type-safe requirements (ServerOnly, Universal, etc.)
    },
    /// Обращение к несуществующему объекту метаданных (MILESTONE 3.16)
    UnknownMetadataObject {
        /// Вид метаданных (Document, Catalog, InformationRegister, etc.)
        kind: MetadataKind,
        /// Имя объекта, который не найден
        name: String,
        /// Список похожих имён (подсказки)
        suggestions: Vec<String>,
        /// Имя переменной (если доступно)
        variable_name: Option<String>,
    },
    /// Доступ к члену у типа Unknown (переменная не была присвоена) - MILESTONE 5.1
    UnknownTypeAccess {
        /// Имя переменной (если известно)
        variable_name: Option<String>,
        /// Имя члена (свойство или метод)
        member_name: String,
    },
    /// Необъявленная переменная
    UndeclaredVariable {
        /// Имя переменной
        variable_name: String,
        /// Имя метода (если в аргументе)
        method_name: Option<String>,
        /// Индекс параметра (1-based)
        param_index: Option<usize>,
    },
    /// Объявление переменной (Перем) после исполняемого кода
    VarDeclarationAfterExecutable {
        /// Имя переменной
        variable_name: String,
        /// Имя функции/процедуры
        function_name: String,
    },
    /// Использование неинициализированной переменной
    UninitializedVariableUsage {
        /// Имя переменной
        variable_name: String,
    },
}
