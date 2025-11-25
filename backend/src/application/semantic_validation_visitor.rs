//! Semantic Validation Visitor
use bsl_shared::domain::resolver::{TypeResolver, ValidationResult, ValidationResultV2};
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{Certainty, ConcreteType, ConfigurationType, DiagnosticSeverity, FacetKind, MetadataKind, ResolutionResult, TypeDiagnostic};
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::domain::RuntimeExecutionContext;  // MILESTONE 3.11 Phase 3
use bsl_shared::formatting::DetailLevel;  // MILESTONE 3.6 Phase 3
use bsl_shared::ir::{
    FlowContext, SemanticNode, SemanticNodeKind, SemanticProgram, SemanticVisitor, Span,
};

pub struct SemanticValidationVisitor<'a> {
    validator: &'a TypeValidator<'a>,
    resolver: &'a TypeResolver,
    signature_index: &'a SignatureIndex,
    errors: Vec<TypeDiagnostic>,
    #[allow(dead_code)]
    program: &'a SemanticProgram,
    detail_level: DetailLevel,  // MILESTONE 3.6 Phase 3
    current_execution_context: RuntimeExecutionContext,  // MILESTONE 3.11 Phase 3
}

impl<'a> SemanticValidationVisitor<'a> {
    pub fn new(
        validator: &'a TypeValidator<'a>,
        program: &'a SemanticProgram,
        resolver: &'a TypeResolver,
        signature_index: &'a SignatureIndex,
    ) -> Self {
        Self {
            validator,
            resolver,
            signature_index,
            errors: Vec::new(),
            program,
            detail_level: DetailLevel::Full,  // Default для backward compatibility
            current_execution_context: RuntimeExecutionContext::new(),  // MILESTONE 3.11 Phase 3
        }
    }

    /// MILESTONE 3.6 Phase 3: Создать visitor с настраиваемым уровнем детализации
    pub fn with_detail_level(
        validator: &'a TypeValidator<'a>,
        program: &'a SemanticProgram,
        resolver: &'a TypeResolver,
        signature_index: &'a SignatureIndex,
        detail_level: DetailLevel,
    ) -> Self {
        Self {
            validator,
            resolver,
            signature_index,
            errors: Vec::new(),
            program,
            detail_level,
            current_execution_context: RuntimeExecutionContext::new(),  // MILESTONE 3.11 Phase 3
        }
    }

    pub fn into_errors(self) -> Vec<TypeDiagnostic> {
        self.errors
    }

    /// MILESTONE 3.11 Phase 3: Валидация доступности метода в текущем контексте
    /// Возвращает Some(TypeErrorKind) если метод недоступен в текущем контексте
    fn validate_method_call_context(
        &self,
        receiver_type: &str,
        method_name: &str,
        variable_name: Option<String>,
        _span: bsl_shared::ir::Span,
    ) -> Option<bsl_shared::domain::validators::TypeErrorKind> {
        use bsl_shared::domain::validators::TypeErrorKind;

        // Найти метод в SignatureIndex
        if let Some(signature) = self.signature_index.find_method(receiver_type, method_name) {
            // Проверить доступность метода в текущем контексте
            if !self.current_execution_context.can_call_method(&signature.context_requirements) {
                return Some(TypeErrorKind::MethodNotAvailableInContext {
                    method_name: method_name.to_string(),
                    object_type: receiver_type.to_string(),
                    variable_name,
                    current_context: self.current_execution_context.current_directive,  // ✅ Type-safe
                    required_context: signature.context_requirements,                   // ✅ Type-safe
                });
            }
        }
        None
    }

    /// Конвертирует ValidationResult в TypeDiagnostic (Milestone 3.10)
    fn validation_result_to_diagnostic(
        result: &ValidationResult,
        span: bsl_shared::ir::Span,
    ) -> Option<TypeDiagnostic> {
        match result {
            ValidationResult::Ok(_) => None,
            ValidationResult::NotFound => None, // Уже обработано в validate_method_exists
            ValidationResult::MissingRequiredParam {
                param_name,
                param_index,
            } => Some(TypeDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: format!(
                    "Недостаточно параметров: отсутствует обязательный параметр #{} '{}'",
                    param_index + 1,
                    param_name
                ),
                line: span.start_line,
                column: span.start_column,
                end_line: span.end_line,
                end_column: span.end_column,
            }),
            ValidationResult::TooManyArgs { expected, actual } => Some(TypeDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: format!(
                    "Слишком много параметров: ожидается {}, получено {}",
                    expected, actual
                ),
                line: span.start_line,
                column: span.start_column,
                end_line: span.end_line,
                end_column: span.end_column,
            }),
            ValidationResult::TypeMismatch {
                param_name,
                expected,
                actual,
            } => Some(TypeDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: format!(
                    "Некорректный тип параметра '{}': ожидается {}, получено {}",
                    param_name, expected, actual
                ),
                line: span.start_line,
                column: span.start_column,
                end_line: span.end_line,
                end_column: span.end_column,
            }),
        }
    }

    /// Конвертирует ValidationResultV2 в TypeDiagnostic (Milestone 3.13)
    /// Использует объектное сравнение типов с детальными причинами несовместимости
    fn validation_result_v2_to_diagnostic(
        result: &ValidationResultV2,
        span: Span,
    ) -> Option<TypeDiagnostic> {
        match result {
            ValidationResultV2::Ok(_) => None,
            ValidationResultV2::NotFound => None, // Обрабатывается отдельно в validate_method_exists
            ValidationResultV2::MissingRequiredParam { param_name, param_index } => {
                Some(TypeDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: format!(
                        "Отсутствует обязательный параметр '{}' (позиция {})",
                        param_name, param_index + 1
                    ),
                    line: span.start_line,
                    column: span.start_column,
                    end_line: span.end_line,
                    end_column: span.end_column,
                })
            }
            ValidationResultV2::TooManyArgs { expected, actual } => {
                Some(TypeDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: format!(
                        "Слишком много аргументов: ожидается {}, передано {}",
                        expected, actual
                    ),
                    line: span.start_line,
                    column: span.start_column,
                    end_line: span.end_line,
                    end_column: span.end_column,
                })
            }
            ValidationResultV2::TypeMismatch { param_name, param_index, expected, actual, reason } => {
                let msg = if reason.is_empty() {
                    format!(
                        "Параметр '{}' (позиция {}): ожидается {}, получено {}",
                        param_name, param_index + 1, expected, actual
                    )
                } else {
                    format!(
                        "Параметр '{}' (позиция {}): ожидается {}, получено {} ({})",
                        param_name, param_index + 1, expected, actual, reason
                    )
                };
                Some(TypeDiagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: msg,
                    line: span.start_line,
                    column: span.start_column,
                    end_line: span.end_line,
                    end_column: span.end_column,
                })
            }
        }
    }

    fn simple_resolution(type_name: &str) -> bsl_shared::domain::types::TypeResolution {
        use bsl_shared::domain::types::{
            PrimitiveType, ResolutionMetadata, ResolutionSource, TypeResolution,
        };

        // ✅ MILESTONE 3.10: Убираем Generic параметры для поиска методов
        // "Массив<?>" → "Массив"
        let clean_type_name = if let Some(idx) = type_name.find('<') {
            &type_name[..idx]
        } else {
            type_name
        };

        // ✅ MILESTONE 3.11: Попытка распарсить фасетный тип конфигурации
        if let Some((facet_kind, metadata_kind, object_name)) = Self::try_parse_faceted_type(clean_type_name) {
            return TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                    name: object_name,
                    kind: metadata_kind,
                    facet: Some(facet_kind),
                    attributes: vec![],
                    tabular_sections: vec![],
                })),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata::default(),
                active_facet: Some(facet_kind),
                available_facets: vec![],
            };
        }

        // Примитивные типы
        let result = match clean_type_name {
            "Число" | "Number" => {
                ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number))
            }
            "Строка" | "String" => {
                ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String))
            }
            "Булево" | "Boolean" => {
                ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Boolean))
            }
            "Дата" | "Date" => {
                ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Date))
            }
            _ => {
                use bsl_shared::domain::types::PlatformType;
                ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                    name: clean_type_name.to_string(),
                }))
            }
        };

        // ✅ MILESTONE 3.7 BUGFIX: Устанавливаем default фасет для Platform типов
        // Для объектов, созданных через `Новый`, используем Object фасет
        let active_facet = match &result {
            ResolutionResult::Concrete(ConcreteType::Platform(_)) => {
                Some(FacetKind::Object)
            }
            _ => None,
        };

        TypeResolution {
            certainty: Certainty::Known,
            result,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet,
            available_facets: vec![],
        }
    }

    /// Попытка распарсить фасетный тип конфигурации
    ///
    /// # Примеры
    /// - "СправочникМенеджер.Контрагенты" -> Some((Manager, Catalog, "Контрагенты"))
    /// - "ДокументОбъект.ЗаказКлиента" -> Some((Object, Document, "ЗаказКлиента"))
    /// - "Массив" -> None
    fn try_parse_faceted_type(type_name: &str) -> Option<(FacetKind, MetadataKind, String)> {
        // Проверяем наличие точки (признак конкретизированного типа)
        let dot_pos = type_name.find('.')?;

        let prefix = &type_name[..dot_pos];
        let object_name = &type_name[dot_pos + 1..];

        // Используем pattern-matching функции из SignatureIndex
        let facet_kind = SignatureIndex::get_facet_kind_from_prefix(prefix)?;
        let metadata_kind = SignatureIndex::get_metadata_kind_from_prefix(prefix)?;

        Some((facet_kind, metadata_kind, object_name.to_string()))
    }
}

impl<'a> SemanticVisitor for SemanticValidationVisitor<'a> {
    fn visit_node(&mut self, node: &SemanticNode, _context: &mut FlowContext) {
        match &node.kind {
            SemanticNodeKind::FunctionCall {
                function_name,
                object_name,
                object_type: Some(obj_type),
                arg_types,
                ..
            } => {
                let resolution = Self::simple_resolution(obj_type);

                // 1. ✅ MILESTONE 3.6 Phase 3: Проверяем существование метода с передачей variable_name
                if let Some(error_kind) = self
                    .validator
                    .validate_method_exists_with_variable(
                        &resolution,
                        function_name,
                        object_name.clone(),  // Передаём имя переменной
                    )
                {
                    let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                    self.errors.push(diagnostic);
                    return; // Нет смысла проверять параметры если метод не существует
                }

                // 1.5. ✅ MILESTONE 3.11 Phase 3: Проверяем доступность метода в текущем контексте
                if let Some(error_kind) = self.validate_method_call_context(
                    obj_type,
                    function_name,
                    object_name.clone(),
                    node.span,
                ) {
                    // Context warnings используют WARNING severity, а не Error
                    let diagnostic = error_kind.to_diagnostic_with_severity(
                        node.span,
                        self.detail_level,
                        DiagnosticSeverity::Warning
                    );
                    self.errors.push(diagnostic);
                    // НЕ return - продолжаем проверку параметров
                }

                // 2. ✅ MILESTONE 3.13: Проверяем типы параметров с объектным сравнением (v2)
                let validation_result = self.resolver.validate_call_v2(
                    Some(obj_type),
                    function_name,
                    arg_types,
                    self.signature_index,
                );

                // Конвертируем ValidationResultV2 в TypeDiagnostic
                if let Some(diagnostic) = Self::validation_result_v2_to_diagnostic(&validation_result, node.span) {
                    self.errors.push(diagnostic);
                }
            }
            SemanticNodeKind::MemberAccess {
                object_name,
                object_type,
                member_name,
                is_method: false,
                ..
            } => {
                let resolution = Self::simple_resolution(object_type);
                // ✅ MILESTONE 3.6 Phase 3: Передаём имя переменной
                if let Some(error_kind) = self
                    .validator
                    .validate_property_exists_with_variable(
                        &resolution,
                        member_name,
                        object_name.clone(),  // Передаём имя переменной
                    )
                {
                    let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                    self.errors.push(diagnostic);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    use bsl_shared::ir::{SemanticNode, SemanticNodeKind, Span};

    #[test]
    fn test_visitor_detects_nonexistent_method() {
        use std::sync::Arc;
        let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
        let metadata = TypeMetadataLookup::new(repository.clone());
        let validator = TypeValidator::new(&metadata);
        let resolver = TypeResolver::new(repository);
        let signature_index = SignatureIndex::new();
        let mut program = SemanticProgram::new();

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::FunctionCall {
                function_name: "НесуществующийМетод".to_string(),
                object_name: Some("МассивДанных".to_string()),
                object_type: Some("Массив".to_string()),
                arg_types: vec![],
            },
            span: Span::new(5, 10, 5, 40),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
        let mut context = FlowContext::new(program.symbols.root_scope);
        visitor.visit_node(&program.nodes[0], &mut context);

        let errors = visitor.into_errors();
        assert!(
            !errors.is_empty(),
            "Должна быть ошибка для несуществующего метода"
        );
        assert!(errors[0].message.contains("НесуществующийМетод"));
    }

    #[test]
    fn test_visitor_detects_nonexistent_property() {
        use std::sync::Arc;
        let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
        let metadata = TypeMetadataLookup::new(repository.clone());
        let validator = TypeValidator::new(&metadata);
        let resolver = TypeResolver::new(repository);
        let signature_index = SignatureIndex::new();
        let mut program = SemanticProgram::new();

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::MemberAccess {
                object_name: Some("МассивДанных".to_string()),
                object_type: "Массив".to_string(),
                member_name: "НесуществующееСвойство".to_string(),
                is_method: false,
            },
            span: Span::new(3, 5, 3, 35),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
        let mut context = FlowContext::new(program.symbols.root_scope);
        visitor.visit_node(&program.nodes[0], &mut context);

        let errors = visitor.into_errors();
        assert!(
            !errors.is_empty(),
            "Должна быть ошибка для несуществующего свойства"
        );
        assert!(errors[0].message.contains("НесуществующееСвойство"));
    }
}
