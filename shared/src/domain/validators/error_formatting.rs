//! Error formatting for TypeErrorKind
//!
//! This module implements diagnostic message formatting with different detail levels:
//! - Brief: Type error only (backward compatible)
//! - Standard: Type + variable name
//! - Detailed: Standard + smart hints

use crate::domain::code_location::CompilerDirective;
use crate::domain::runtime_context::ContextRequirements;
use crate::domain::types::{DiagnosticSeverity, TypeDiagnostic};
use crate::formatting::DetailLevel;
use crate::ir::Span;

use super::TypeErrorKind;

impl TypeErrorKind {
    /// MILESTONE 3.6 Phase 3: to_diagnostic uses Brief format by default (backward compatibility)
    pub fn to_diagnostic(&self, span: Span) -> TypeDiagnostic {
        let message = self.format_brief();

        TypeDiagnostic {
            severity: DiagnosticSeverity::Error,
            message,
            line: span.start_line,
            column: span.start_column,
            end_line: span.end_line,
            end_column: span.end_column,
        }
    }

    /// MILESTONE 3.6 Phase 3: to_diagnostic with configurable detail level
    pub fn to_diagnostic_with_detail(
        &self,
        span: Span,
        detail_level: DetailLevel,
    ) -> TypeDiagnostic {
        let message = match detail_level {
            DetailLevel::Compact => self.format_brief(),
            DetailLevel::Full => self.format_standard(),
            DetailLevel::Detailed => self.format_detailed(),
        };

        TypeDiagnostic {
            severity: DiagnosticSeverity::Error,
            message,
            line: span.start_line,
            column: span.start_column,
            end_line: span.end_line,
            end_column: span.end_column,
        }
    }

    /// MILESTONE 3.11 Phase 3: Create diagnostic with custom severity
    /// Used for context warnings (Warning instead of Error)
    pub fn to_diagnostic_with_severity(
        &self,
        span: Span,
        detail_level: DetailLevel,
        severity: DiagnosticSeverity,
    ) -> TypeDiagnostic {
        let message = match detail_level {
            DetailLevel::Compact => self.format_brief(),
            DetailLevel::Full => self.format_standard(),
            DetailLevel::Detailed => self.format_detailed(),
        };

        TypeDiagnostic {
            severity,
            message,
            line: span.start_line,
            column: span.start_column,
            end_line: span.end_line,
            end_column: span.end_column,
        }
    }

    /// MILESTONE 3.6 Phase 3: Brief format - error type only (without variable name)
    pub(crate) fn format_brief(&self) -> String {
        match self {
            TypeErrorKind::IncorrectParameterType {
                method_name,
                param_index,
                expected,
                actual,
                ..
            } => format!(
                "Некорректный параметр #{} для метода '{}': ожидается {}, получено {}",
                param_index + 1,
                method_name,
                expected,
                actual
            ),
            TypeErrorKind::NonExistentProperty {
                object_type,
                property_name,
                ..
            } => format!(
                "Свойство '{}' не существует для типа '{}'",
                property_name, object_type
            ),
            TypeErrorKind::NonExistentMethod {
                object_type,
                method_name,
                ..
            } => format!(
                "Метод '{}' не существует для типа '{}'",
                method_name, object_type
            ),
            TypeErrorKind::SimpleTypeAsCollection {
                type_name,
                operation,
                ..
            } => format!(
                "Тип '{}' является примитивным и не поддерживает операцию '{}'",
                type_name, operation
            ),
            TypeErrorKind::MethodNotAvailableInContext {
                method_name,
                current_context,
                required_context,
                ..
            } => format!(
                "Метод '{}' недоступен в контексте {:?}. Требуется: {:?}",
                method_name, current_context, required_context
            ),
            TypeErrorKind::UnknownMetadataObject { kind, name, .. } => format!(
                "{} \"{}\" не найден в конфигурации",
                kind.to_russian_name(),
                name
            ),
            TypeErrorKind::UnknownTypeAccess {
                variable_name,
                member_name,
            } => {
                if let Some(var_name) = variable_name {
                    format!(
                        "Невозможно определить член '{}' для переменной '{}': тип не определён",
                        member_name, var_name
                    )
                } else {
                    format!(
                        "Невозможно определить член '{}': тип объекта не определён",
                        member_name
                    )
                }
            }
            TypeErrorKind::UndeclaredVariable {
                variable_name,
                method_name,
                param_index,
            } => {
                if let (Some(method), Some(idx)) = (method_name, param_index) {
                    format!(
                        "Необъявленная переменная '{}' в параметре #{} метода '{}'",
                        variable_name, idx, method
                    )
                } else {
                    format!("Необъявленная переменная '{}'", variable_name)
                }
            }
            TypeErrorKind::VarDeclarationAfterExecutable {
                variable_name,
                function_name,
            } => {
                format!(
                    "Объявление переменной '{}' после исполняемого кода в '{}'",
                    variable_name, function_name
                )
            }
            TypeErrorKind::UninitializedVariableUsage { variable_name } => {
                format!(
                    "Использование неинициализированной переменной '{}'",
                    variable_name
                )
            }
            TypeErrorKind::UnknownType { type_name, .. } => {
                format!("Тип '{}' не найден", type_name)
            }
            TypeErrorKind::InvalidStringConcatenation {
                left_type,
                right_type,
            } => format!(
                "Конкатенация строк требует тип 'Строка' для обоих операндов: получено '{}' и '{}'",
                left_type, right_type
            ),
        }
    }

    /// MILESTONE 3.6 Phase 3: Standard format - type + variable name
    pub(crate) fn format_standard(&self) -> String {
        match self {
            TypeErrorKind::NonExistentMethod {
                object_type,
                method_name,
                variable_name,
            } => {
                if let Some(var) = variable_name {
                    format!(
                        "Метод '{}' не существует для переменной '{}' типа '{}'",
                        method_name, var, object_type
                    )
                } else {
                    self.format_brief() // Fallback if variable_name is missing
                }
            }
            TypeErrorKind::IncorrectParameterType {
                method_name,
                param_index,
                expected,
                actual,
                variable_name,
                param_variable_name,
            } => {
                let mut msg = format!(
                    "Некорректный параметр #{} для метода '{}'",
                    param_index + 1,
                    method_name
                );

                if let Some(var) = variable_name {
                    msg.push_str(&format!(" переменной '{}'", var));
                }

                msg.push_str(&format!(": ожидается {}, получено", expected));

                if let Some(param_var) = param_variable_name {
                    msg.push_str(&format!(" переменная '{}' типа {}", param_var, actual));
                } else {
                    msg.push_str(&format!(" {}", actual));
                }

                msg
            }
            TypeErrorKind::NonExistentProperty {
                object_type,
                property_name,
                variable_name,
            } => {
                if let Some(var) = variable_name {
                    format!(
                        "Свойство '{}' не существует для переменной '{}' типа '{}'",
                        property_name, var, object_type
                    )
                } else {
                    self.format_brief()
                }
            }
            TypeErrorKind::SimpleTypeAsCollection {
                type_name,
                operation,
                variable_name,
            } => {
                if let Some(var) = variable_name {
                    format!(
                        "Переменная '{}' типа '{}' не является коллекцией, операция '{}' недоступна",
                        var, type_name, operation
                    )
                } else {
                    self.format_brief()
                }
            }
            TypeErrorKind::MethodNotAvailableInContext {
                method_name,
                object_type,
                variable_name,
                current_context,
                required_context,
            } => {
                if let Some(var) = variable_name {
                    format!(
                        "Метод '{}' переменной '{}' типа '{}' недоступен в контексте {:?}. Требуется: {:?}",
                        method_name, var, object_type, current_context, required_context
                    )
                } else {
                    format!(
                        "Метод '{}' типа '{}' недоступен в контексте {:?}. Требуется: {:?}",
                        method_name, object_type, current_context, required_context
                    )
                }
            }
            TypeErrorKind::UnknownMetadataObject {
                kind,
                name,
                suggestions,
                variable_name,
            } => {
                let kind_name = kind.to_russian_name();
                let mut msg = format!("{} \"{}\" не найден в конфигурации", kind_name, name);

                if let Some(var) = variable_name {
                    msg = format!("Переменная '{}': {}", var, msg);
                }

                if !suggestions.is_empty() {
                    msg.push_str(&format!(
                        ". Возможно, вы имели в виду: {}",
                        suggestions.join(", ")
                    ));
                }

                msg
            }
            // UnknownTypeAccess: Standard format matches Brief
            TypeErrorKind::UnknownTypeAccess { .. } => self.format_brief(),
            // UndeclaredVariable: Standard format matches Brief
            TypeErrorKind::UndeclaredVariable { .. } => self.format_brief(),
            // VarDeclarationAfterExecutable: Standard format matches Brief
            TypeErrorKind::VarDeclarationAfterExecutable { .. } => self.format_brief(),
            // UninitializedVariableUsage: Standard format matches Brief
            TypeErrorKind::UninitializedVariableUsage { .. } => self.format_brief(),
            TypeErrorKind::UnknownType { .. } => self.format_brief(),
            TypeErrorKind::InvalidStringConcatenation { .. } => self.format_brief(),
        }
    }

    /// MILESTONE 3.6 Phase 3: Detailed format - Standard + smart hints
    pub(crate) fn format_detailed(&self) -> String {
        let base = self.format_standard();
        let hint = self.generate_hint();

        if !hint.is_empty() {
            format!("{}\n\n{}", base, hint)
        } else {
            base
        }
    }

    /// MILESTONE 3.6 Phase 3: Generate smart hints
    pub(crate) fn generate_hint(&self) -> String {
        match self {
            TypeErrorKind::NonExistentMethod {
                object_type,
                method_name,
                ..
            } => {
                // Simple hint without fuzzy matching
                format!(
                    "\u{1F4A1} Подсказка: Проверьте правильность написания метода '{}' для типа '{}'. \
                    Возможно, метод называется по-другому или недоступен для текущего фасета.",
                    method_name, object_type
                )
            }
            TypeErrorKind::IncorrectParameterType {
                expected, actual, ..
            } => {
                format!(
                    "\u{1F4A1} Подсказка: Ожидается {}, но передано {}. \
                    Преобразуйте тип явно или используйте переменную правильного типа.",
                    expected, actual
                )
            }
            TypeErrorKind::NonExistentProperty {
                object_type,
                property_name,
                ..
            } => {
                format!(
                    "\u{1F4A1} Подсказка: Свойство '{}' не найдено для типа '{}'. \
                    Проверьте правильность имени свойства или используйте доступное свойство.",
                    property_name, object_type
                )
            }
            TypeErrorKind::SimpleTypeAsCollection {
                type_name,
                operation,
                ..
            } => {
                format!(
                    "\u{1F4A1} Подсказка: Тип '{}' не поддерживает операцию '{}'. \
                    Используйте коллекцию (Массив, Список, Соответствие) для этой операции.",
                    type_name, operation
                )
            }
            TypeErrorKind::MethodNotAvailableInContext {
                method_name,
                current_context,
                required_context,
                ..
            } => {
                if required_context == &ContextRequirements::ServerOnly {
                    format!(
                        "\u{1F4A1} Подсказка: Метод '{}' доступен только в серверном контексте. \
                        Используйте директиву &НаСервере или &НаСервереБезКонтекста, \
                        либо вызовите метод через серверную процедуру.",
                        method_name
                    )
                } else if matches!(current_context, CompilerDirective::OnClient) {
                    format!(
                        "\u{1F4A1} Подсказка: Метод '{}' недоступен в клиентском контексте. \
                        Переместите вызов в серверную процедуру или используйте альтернативный метод.",
                        method_name
                    )
                } else {
                    format!(
                        "\u{1F4A1} Подсказка: Метод '{}' требует контекст {:?}. Текущий контекст: {:?}. \
                        Измените директиву компиляции функции/процедуры.",
                        method_name, required_context, current_context
                    )
                }
            }
            TypeErrorKind::UnknownMetadataObject {
                kind, suggestions, ..
            } => {
                let kind_name = kind.to_russian_name();
                if suggestions.is_empty() {
                    format!(
                        "\u{1F4A1} Подсказка: {} не найден в загруженной конфигурации. \
                        Проверьте, что конфигурация загружена: BSL: Parse Configuration. \
                        Или исправьте имя объекта метаданных.",
                        kind_name
                    )
                } else {
                    "\u{1F4A1} Подсказка: Проверьте правильность написания имени. \
                        Используйте команду VSCode: BSL: Parse Configuration для загрузки метаданных.".to_string()
                }
            }
            TypeErrorKind::UnknownTypeAccess { variable_name, .. } => {
                if let Some(var_name) = variable_name {
                    format!(
                        "\u{1F4A1} Подсказка: Переменная '{}' не была инициализирована. \
                        Присвойте значение переменной перед обращением к её членам.",
                        var_name
                    )
                } else {
                    "\u{1F4A1} Подсказка: Переменная не была инициализирована. \
                        Присвойте значение переменной перед обращением к её членам."
                        .to_string()
                }
            }
            TypeErrorKind::UndeclaredVariable { variable_name, .. } => {
                format!(
                    "\u{1F4A1} Подсказка: Переменная '{}' не объявлена в текущей области видимости. \
                    Объявите переменную с помощью 'Перем {}' или присвойте ей значение перед использованием.",
                    variable_name, variable_name
                )
            }
            TypeErrorKind::VarDeclarationAfterExecutable { variable_name, .. } => {
                format!(
                    "\u{1F4A1} Подсказка: В 1С объявления переменных (Перем) должны располагаться \
                    в начале функции/процедуры, до любого исполняемого кода. \
                    Переместите 'Перем {}' в начало тела функции.",
                    variable_name
                )
            }
            TypeErrorKind::UninitializedVariableUsage { variable_name } => {
                format!(
                    "\u{1F4A1} Подсказка: Переменная '{}' объявлена, но не инициализирована. \
                    Присвойте значение переменной перед использованием: {} = <значение>;",
                    variable_name, variable_name
                )
            }
            TypeErrorKind::UnknownType {
                type_name,
                variable_name,
            } => {
                if let Some(var) = variable_name {
                    format!(
                        "\u{1F4A1} Подсказка: Тип '{}' указан в '{}' но отсутствует в TypeRepository. \
                        Проверьте опечатку или убедитесь, что тип загружен из Syntax Helper/конфигурации.",
                        type_name, var
                    )
                } else {
                    format!(
                        "\u{1F4A1} Подсказка: Тип '{}' отсутствует в TypeRepository. \
                        Проверьте опечатку или убедитесь, что тип загружен из Syntax Helper/конфигурации.",
                        type_name
                    )
                }
            }
            TypeErrorKind::InvalidStringConcatenation { .. } => {
                "\u{1F4A1} Подсказка: Приведите операнд к строке, например: Строка(<выражение>)."
                    .to_string()
            }
        }
    }
}
