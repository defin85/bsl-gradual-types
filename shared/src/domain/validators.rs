//! Type validation rules based on Balyuk & Popova (2021) research
//!
//! This module implements static type-checking validation inspired by:
//! "Static type-checking for programs developed on the platform 1C:Enterprise"
//! https://ceur-ws.org/Vol-2984/paper13.pdf
//!
//! Three main categories of errors detected:
//! 1. Incorrect parameter passing to methods
//! 2. Access to non-existent properties of objects
//! 3. Treating simple types as collections

use crate::domain::code_location::CompilerDirective;  // MILESTONE 3.11 Phase 3
use crate::domain::metadata_lookup::TypeMetadataLookup;
use crate::domain::runtime_context::ContextRequirements;  // MILESTONE 3.11 Phase 3
use crate::domain::types::{
    ConcreteType, DiagnosticSeverity, MetadataKind, SpecialType, TypeDiagnostic, TypeResolution,
    UncertaintyReason,
};
use crate::formatting::DetailLevel;  // MILESTONE 3.6 Phase 3
use crate::ir::Span;

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
        current_context: CompilerDirective,      // ✅ Type-safe context (OnClient, OnServer, etc.)
        required_context: ContextRequirements,   // ✅ Type-safe requirements (ServerOnly, Universal, etc.)
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


impl TypeErrorKind {
    /// MILESTONE 3.6 Phase 3: to_diagnostic теперь использует Brief формат по умолчанию (backward compatibility)
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

    /// MILESTONE 3.6 Phase 3: to_diagnostic с настраиваемым уровнем детализации
    pub fn to_diagnostic_with_detail(&self, span: Span, detail_level: DetailLevel) -> TypeDiagnostic {
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

    /// MILESTONE 3.11 Phase 3: Создать diagnostic с кастомной severity
    /// Используется для context warnings (Warning вместо Error)
    pub fn to_diagnostic_with_severity(&self, span: Span, detail_level: DetailLevel, severity: DiagnosticSeverity) -> TypeDiagnostic {
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

    /// MILESTONE 3.6 Phase 3: Brief формат - только тип ошибки (без имени переменной)
    fn format_brief(&self) -> String {
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
            TypeErrorKind::UnknownMetadataObject {
                kind,
                name,
                ..
            } => format!(
                "{} \"{}\" не найден в конфигурации",
                kind.to_russian_name(), name
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
            TypeErrorKind::UndeclaredVariable { variable_name, method_name, param_index } => {
                if let (Some(method), Some(idx)) = (method_name, param_index) {
                    format!("Необъявленная переменная '{}' в параметре #{} метода '{}'", variable_name, idx, method)
                } else {
                    format!("Необъявленная переменная '{}'", variable_name)
                }
            }
            TypeErrorKind::VarDeclarationAfterExecutable { variable_name, function_name } => {
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
        }
    }

    /// MILESTONE 3.6 Phase 3: Standard формат - тип + имя переменной
    fn format_standard(&self) -> String {
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
                    self.format_brief() // Fallback если variable_name отсутствует
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
                    msg.push_str(&format!(". Возможно, вы имели в виду: {}", suggestions.join(", ")));
                }

                msg
            }
            // UnknownTypeAccess: Standard формат совпадает с Brief
            TypeErrorKind::UnknownTypeAccess { .. } => self.format_brief(),
            // UndeclaredVariable: Standard формат совпадает с Brief
            TypeErrorKind::UndeclaredVariable { .. } => self.format_brief(),
            // VarDeclarationAfterExecutable: Standard формат совпадает с Brief
            TypeErrorKind::VarDeclarationAfterExecutable { .. } => self.format_brief(),
            // UninitializedVariableUsage: Standard формат совпадает с Brief
            TypeErrorKind::UninitializedVariableUsage { .. } => self.format_brief(),
        }
    }

    /// MILESTONE 3.6 Phase 3: Detailed формат - Standard + умные подсказки
    fn format_detailed(&self) -> String {
        let base = self.format_standard();
        let hint = self.generate_hint();

        if !hint.is_empty() {
            format!("{}\n\n{}", base, hint)
        } else {
            base
        }
    }

    /// MILESTONE 3.6 Phase 3: Генерация умных подсказок
    fn generate_hint(&self) -> String {
        match self {
            TypeErrorKind::NonExistentMethod {
                object_type,
                method_name,
                ..
            } => {
                // Простая подсказка без fuzzy matching
                format!(
                    "💡 Подсказка: Проверьте правильность написания метода '{}' для типа '{}'. \
                    Возможно, метод называется по-другому или недоступен для текущего фасета.",
                    method_name, object_type
                )
            }
            TypeErrorKind::IncorrectParameterType {
                expected,
                actual,
                ..
            } => {
                format!(
                    "💡 Подсказка: Ожидается {}, но передано {}. \
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
                    "💡 Подсказка: Свойство '{}' не найдено для типа '{}'. \
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
                    "💡 Подсказка: Тип '{}' не поддерживает операцию '{}'. \
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
                use crate::domain::runtime_context::ContextRequirements;

                if required_context == &ContextRequirements::ServerOnly {
                    format!(
                        "💡 Подсказка: Метод '{}' доступен только в серверном контексте. \
                        Используйте директиву &НаСервере или &НаСервереБезКонтекста, \
                        либо вызовите метод через серверную процедуру.",
                        method_name
                    )
                } else if matches!(current_context, CompilerDirective::OnClient) {
                    format!(
                        "💡 Подсказка: Метод '{}' недоступен в клиентском контексте. \
                        Переместите вызов в серверную процедуру или используйте альтернативный метод.",
                        method_name
                    )
                } else {
                    format!(
                        "💡 Подсказка: Метод '{}' требует контекст {:?}. Текущий контекст: {:?}. \
                        Измените директиву компиляции функции/процедуры.",
                        method_name, required_context, current_context
                    )
                }
            }
            TypeErrorKind::UnknownMetadataObject {
                kind,
                suggestions,
                ..
            } => {
                let kind_name = kind.to_russian_name();
                if suggestions.is_empty() {
                    format!(
                        "💡 Подсказка: {} не найден в загруженной конфигурации. \
                        Проверьте, что конфигурация загружена: BSL: Parse Configuration. \
                        Или исправьте имя объекта метаданных.",
                        kind_name
                    )
                } else {
                    "💡 Подсказка: Проверьте правильность написания имени. \
                        Используйте команду VSCode: BSL: Parse Configuration для загрузки метаданных.".to_string()
                }
            }
            TypeErrorKind::UnknownTypeAccess {
                variable_name,
                ..
            } => {
                if let Some(var_name) = variable_name {
                    format!(
                        "💡 Подсказка: Переменная '{}' не была инициализирована. \
                        Присвойте значение переменной перед обращением к её членам.",
                        var_name
                    )
                } else {
                    "💡 Подсказка: Переменная не была инициализирована. \
                        Присвойте значение переменной перед обращением к её членам.".to_string()
                }
            }
            TypeErrorKind::UndeclaredVariable {
                variable_name,
                ..
            } => {
                format!(
                    "💡 Подсказка: Переменная '{}' не объявлена в текущей области видимости. \
                    Объявите переменную с помощью 'Перем {}' или присвойте ей значение перед использованием.",
                    variable_name, variable_name
                )
            }
            TypeErrorKind::VarDeclarationAfterExecutable {
                variable_name,
                ..
            } => {
                format!(
                    "💡 Подсказка: В 1С объявления переменных (Перем) должны располагаться \
                    в начале функции/процедуры, до любого исполняемого кода. \
                    Переместите 'Перем {}' в начало тела функции.",
                    variable_name
                )
            }
            TypeErrorKind::UninitializedVariableUsage { variable_name } => {
                format!(
                    "💡 Подсказка: Переменная '{}' объявлена, но не инициализирована. \
                    Присвойте значение переменной перед использованием: {} = <значение>;",
                    variable_name, variable_name
                )
            }
        }
    }
}

/// Валидатор типов на основе правил из статьи
pub struct TypeValidator<'a> {
    metadata_lookup: &'a TypeMetadataLookup,
}

impl<'a> TypeValidator<'a> {
    /// Создаёт новый валидатор с доступом к метаданным
    pub fn new(metadata_lookup: &'a TypeMetadataLookup) -> Self {
        Self { metadata_lookup }
    }

    /// Проверка существования метода у объекта (новый метод!)
    pub fn validate_method_exists(
        &self,
        object_resolution: &TypeResolution,
        method_name: &str,
    ) -> Option<TypeErrorKind> {
        let methods = self.metadata_lookup.get_methods(object_resolution);

        // Проверяем есть ли метод с таким именем (case-insensitive для кириллицы и латиницы)
        let method_exists = methods.iter().any(|m| {
            Self::names_equal_ignore_case(&m.name, method_name)
                || Self::names_equal_ignore_case(&m.english_name, method_name)
        });

        if !method_exists {
            // Получаем читаемое имя типа для сообщения об ошибке
            let type_name = Self::resolution_to_string(object_resolution);
            Some(TypeErrorKind::NonExistentMethod {
                object_type: type_name,
                method_name: method_name.to_string(),
                variable_name: None,  // MILESTONE 3.6 Phase 3: будет передаваться из visitor
            })
        } else {
            None
        }
    }

    /// MILESTONE 3.6 Phase 3: версия с передачей variable_name
    pub fn validate_method_exists_with_variable(
        &self,
        object_resolution: &TypeResolution,
        method_name: &str,
        variable_name: Option<String>,
    ) -> Option<TypeErrorKind> {
        let methods = self.metadata_lookup.get_methods(object_resolution);

        tracing::info!(
            "🔍 validate_method: method='{}', type='{}', active_facet={:?}, found {} methods",
            method_name,
            object_resolution.type_name(),
            object_resolution.active_facet,
            methods.len()
        );

        let method_exists = methods.iter().any(|m| {
            Self::names_equal_ignore_case(&m.name, method_name)
                || Self::names_equal_ignore_case(&m.english_name, method_name)
        });

        if !method_exists {
            let type_name = Self::resolution_to_string(object_resolution);
            tracing::warn!(
                "❌ Method '{}' NOT found for type '{}' (active_facet={:?})",
                method_name,
                type_name,
                object_resolution.active_facet
            );
            Some(TypeErrorKind::NonExistentMethod {
                object_type: type_name,
                method_name: method_name.to_string(),
                variable_name,
            })
        } else {
            None
        }
    }

    /// Проверка существования свойства у объекта (обновлённый метод)
    pub fn validate_property_exists(
        &self,
        object_resolution: &TypeResolution,
        property_name: &str,
    ) -> Option<TypeErrorKind> {
        let properties = self.metadata_lookup.get_properties(object_resolution);

        // Проверяем есть ли свойство с таким именем (case-insensitive)
        let property_exists = properties
            .iter()
            .any(|p| Self::names_equal_ignore_case(&p.name, property_name));

        if !property_exists {
            let type_name = Self::resolution_to_string(object_resolution);
            Some(TypeErrorKind::NonExistentProperty {
                object_type: type_name,
                property_name: property_name.to_string(),
                variable_name: None,  // MILESTONE 3.6 Phase 3: будет передаваться из visitor
            })
        } else {
            None
        }
    }

    /// MILESTONE 3.6 Phase 3: версия с передачей variable_name
    pub fn validate_property_exists_with_variable(
        &self,
        object_resolution: &TypeResolution,
        property_name: &str,
        variable_name: Option<String>,
    ) -> Option<TypeErrorKind> {
        let properties = self.metadata_lookup.get_properties(object_resolution);

        let property_exists = properties
            .iter()
            .any(|p| Self::names_equal_ignore_case(&p.name, property_name));

        if !property_exists {
            let type_name = Self::resolution_to_string(object_resolution);
            Some(TypeErrorKind::NonExistentProperty {
                object_type: type_name,
                property_name: property_name.to_string(),
                variable_name,
            })
        } else {
            None
        }
    }

    /// Case-insensitive сравнение строк (работает с кириллицей и латиницей)
    fn names_equal_ignore_case(a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.chars()
            .zip(b.chars())
            .all(|(ca, cb)| ca.to_lowercase().eq(cb.to_lowercase()))
    }

    /// MILESTONE 3.16: Валидация существования объекта метаданных
    ///
    /// # Параметры
    ///
    /// * `kind` - вид метаданных (Catalog, Document, etc.)
    /// * `name` - имя объекта (например, "Контрагенты")
    /// * `variable_name` - имя переменной (для диагностического сообщения)
    ///
    /// # Возвращает
    ///
    /// `Some(TypeErrorKind::UnknownMetadataObject)` если объект не найден,
    /// `None` если объект существует или конфигурация не загружена
    ///
    /// # Пример
    ///
    /// ```ignore
    /// // Валидация: Справочники.Контрогенты (опечатка)
    /// if let Some(error) = validator.validate_metadata_object_exists(
    ///     MetadataKind::Catalog,
    ///     "Контрогенты",
    ///     Some("спр".to_string()),
    /// ) {
    ///     // error = UnknownMetadataObject { kind: Catalog, name: "Контрогенты", suggestions: ["Контрагенты"], ... }
    /// }
    /// ```
    pub fn validate_metadata_object_exists(
        &self,
        kind: MetadataKind,
        name: &str,
        variable_name: Option<String>,
    ) -> Option<TypeErrorKind> {
        // Graceful degradation: если конфигурация не загружена, не показываем ошибку
        if !self.metadata_lookup.is_configuration_loaded() {
            return None;
        }

        // Проверяем существование объекта метаданных
        if self.metadata_lookup.exists_metadata_object(kind, name) {
            return None; // Объект найден
        }

        // Объект не найден - генерируем предложения
        let suggestions = self.metadata_lookup.suggest_similar_names(kind, name, 3);

        Some(TypeErrorKind::UnknownMetadataObject {
            kind,
            name: name.to_string(),
            suggestions,
            variable_name,
        })
    }

    /// MILESTONE 3.16: Validate type resolution and extract error if appropriate
    ///
    /// This method checks the `uncertainty_reason` field of a TypeResolution
    /// and generates appropriate validation errors.
    ///
    /// # Behavior
    ///
    /// - If `uncertainty_reason` is `MetadataObjectNotFound` and configuration is loaded,
    ///   returns `TypeErrorKind::UnknownMetadataObject` with suggestions
    /// - If `uncertainty_reason` is `ConfigurationNotLoaded`, returns `None` (graceful degradation)
    /// - Otherwise returns `None`
    ///
    /// # Example
    ///
    /// ```ignore
    /// let resolution = resolver.resolve_expression_sync("Справочники.Контрогенты");
    /// if let Some(error) = validator.validate_from_resolution(&resolution) {
    ///     // error = UnknownMetadataObject { kind: Catalog, name: "Контрогенты", ... }
    /// }
    /// ```
    pub fn validate_from_resolution(&self, resolution: &TypeResolution) -> Option<TypeErrorKind> {
        // Check if there's an uncertainty reason in the resolution metadata
        if let Some(reason) = &resolution.metadata.uncertainty_reason {
            match reason {
                UncertaintyReason::MetadataObjectNotFound { kind, name } => {
                    // Generate suggestions for similar names
                    let suggestions = self.metadata_lookup.suggest_similar_names(*kind, name, 3);

                    Some(TypeErrorKind::UnknownMetadataObject {
                        kind: *kind,
                        name: name.clone(),
                        suggestions,
                        variable_name: None, // Will be filled by the caller if needed
                    })
                }
                UncertaintyReason::ConfigurationNotLoaded => {
                    // Graceful degradation: don't report errors when config is not loaded
                    None
                }
                UncertaintyReason::Other(_) => {
                    // Other reasons don't generate validation errors
                    None
                }
                UncertaintyReason::UndeclaredVariable { name } => {
                    // Undeclared variable errors are handled separately in visitor
                    // to provide better context (method name, param index)
                    Some(TypeErrorKind::UndeclaredVariable {
                        variable_name: name.clone(),
                        method_name: None,
                        param_index: None,
                    })
                }
            }
        } else {
            None
        }
    }

    /// Проверка операций с коллекциями
    pub fn validate_collection_operation(
        type_resolution: &TypeResolution,
        operation: &str,
    ) -> Option<TypeErrorKind> {
        use crate::domain::types::ResolutionResult;

        match &type_resolution.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(prim)) => {
                // Примитивные типы нельзя использовать как коллекции
                Some(TypeErrorKind::SimpleTypeAsCollection {
                    type_name: prim.display_name().to_string(),
                    operation: operation.to_string(),
                    variable_name: None,  // MILESTONE 3.6 Phase 3: будет передаваться из visitor
                })
            }
            ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Undefined))
            | ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Null)) => {
                Some(TypeErrorKind::SimpleTypeAsCollection {
                    type_name: "Неопределено".to_string(),
                    operation: operation.to_string(),
                    variable_name: None,  // MILESTONE 3.6 Phase 3: будет передаваться из visitor
                })
            }
            _ => None,
        }
    }

    // Вспомогательные методы

    fn resolution_to_string(resolution: &TypeResolution) -> String {
        use crate::domain::types::ResolutionResult;

        match &resolution.result {
            ResolutionResult::Concrete(concrete) => concrete.to_string(),
            ResolutionResult::Union(types) => {
                let type_names: Vec<_> = types.iter().map(|wt| wt.type_.to_string()).collect();
                format!(
                    "({}) | вероятность неопределённости",
                    type_names.join(" | ")
                )
            }
            ResolutionResult::Intersection(types) => {
                let type_names: Vec<_> = types.iter().map(|t| t.to_string()).collect();
                format!("({})", type_names.join(" & "))
            }
            ResolutionResult::Generic(gen) => {
                if gen.type_params.is_empty() {
                    gen.base_type.clone()
                } else {
                    let params: Vec<_> = gen.type_params.iter().map(|t| t.to_string()).collect();
                    format!("{}<{}>", gen.base_type, params.join(", "))
                }
            }
            ResolutionResult::Nullable(inner) => {
                format!("{} | Null", inner)
            }
            ResolutionResult::Dynamic => "Произвольный".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{
        Certainty, PrimitiveType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    };


    #[test]
    fn test_simple_type_as_collection() {
        let resolution = TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let error = TypeValidator::validate_collection_operation(&resolution, "Добавить");

        assert!(error.is_some());
        assert!(matches!(
            error.unwrap(),
            TypeErrorKind::SimpleTypeAsCollection { .. }
        ));
    }

}
