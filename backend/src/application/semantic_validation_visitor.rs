//! Semantic Validation Visitor
use bsl_shared::domain::resolver::{TypeResolver, ValidationResult, ValidationResultV2};
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{DiagnosticSeverity, MetadataKind, TypeDiagnostic};
use bsl_shared::domain::validators::{TypeValidator, TypeErrorKind};
use bsl_shared::domain::RuntimeExecutionContext;  // MILESTONE 3.11 Phase 3
use bsl_shared::formatting::DetailLevel;  // MILESTONE 3.6 Phase 3
use bsl_shared::ir::{
    FlowContext, MemberAccessKind, SemanticNode, SemanticNodeKind, SemanticProgram, SemanticVisitor, Span,
};

// === MILESTONE 3.16: Helper функции для детекции коллекций метаданных ===

/// Маппинг имён коллекций метаданных на MetadataKind
/// (русское имя, английское имя, MetadataKind)
///
/// Единый источник истины для конвертации имён коллекций в MetadataKind.
/// Используется в is_metadata_collection_name() и collection_name_to_metadata_kind().
static METADATA_COLLECTIONS: &[(&str, &str, MetadataKind)] = &[
    ("Справочники", "Catalogs", MetadataKind::Catalog),
    ("Документы", "Documents", MetadataKind::Document),
    ("Перечисления", "Enums", MetadataKind::Enum),
    ("РегистрыСведений", "InformationRegisters", MetadataKind::InformationRegister),
    ("РегистрыНакопления", "AccumulationRegisters", MetadataKind::AccumulationRegister),
    ("РегистрыБухгалтерии", "AccountingRegisters", MetadataKind::AccountingRegister),
    ("РегистрыРасчета", "CalculationRegisters", MetadataKind::CalculationRegister),
    ("Отчеты", "Reports", MetadataKind::Report),
    ("Обработки", "DataProcessors", MetadataKind::DataProcessor),
    ("ПланыСчетов", "ChartsOfAccounts", MetadataKind::ChartOfAccounts),
    ("ПланыВидовХарактеристик", "ChartsOfCharacteristicTypes", MetadataKind::ChartOfCharacteristicTypes),
    ("ПланыВидовРасчета", "ChartsOfCalculationTypes", MetadataKind::ChartOfCalculationTypes),
    ("БизнесПроцессы", "BusinessProcesses", MetadataKind::BusinessProcess),
    ("Задачи", "Tasks", MetadataKind::Task),
];

/// Проверяет, является ли имя коллекцией метаданных
///
/// # Примеры
/// - "Справочники" / "Catalogs" → true
/// - "Документы" / "Documents" → true
/// - "Массив" → false
fn is_metadata_collection_name(name: &str) -> bool {
    METADATA_COLLECTIONS.iter().any(|(ru, en, _)| *ru == name || *en == name)
}

/// Конвертирует имя коллекции метаданных в MetadataKind
///
/// # Примеры
/// - "Справочники" → Some(MetadataKind::Catalog)
/// - "Документы" → Some(MetadataKind::Document)
/// - "Массив" → None
fn collection_name_to_metadata_kind(name: &str) -> Option<MetadataKind> {
    METADATA_COLLECTIONS
        .iter()
        .find(|(ru, en, _)| *ru == name || *en == name)
        .map(|(_, _, kind)| *kind)
}

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

    /// MILESTONE 3.16: Валидация доступа к члену коллекции метаданных
    ///
    /// Проверяет существование объекта метаданных при обращении вида:
    /// `Справочники.Контрагенты`, `Документы.ЗаказПокупателя` и т.д.
    ///
    /// # Параметры
    ///
    /// * `object_type` - тип объекта (например, "Справочники")
    /// * `member_name` - имя члена (например, "Контрагенты")
    /// * `variable_name` - имя переменной (для диагностики)
    ///
    /// # Возвращает
    ///
    /// `Some(TypeErrorKind)` если объект не найден, `None` иначе
    fn validate_metadata_member_access(
        &self,
        object_type: &str,
        member_name: &str,
        variable_name: Option<String>,
    ) -> Option<TypeErrorKind> {
        // Проверяем, является ли object_type коллекцией метаданных
        if !is_metadata_collection_name(object_type) {
            return None;
        }

        // Получаем вид метаданных
        let kind = match collection_name_to_metadata_kind(object_type) {
            Some(k) => k,
            None => return None,
        };

        // Используем метод из TypeValidator для валидации
        self.validator.validate_metadata_object_exists(kind, member_name, variable_name)
    }

    /// Конвертирует ValidationResult в TypeDiagnostic (Milestone 3.10)
    /// TODO: Использовать в будущем для детальной диагностики параметров
    #[allow(dead_code)]
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

    // Phase 4: simple_resolution() и try_parse_faceted_type() удалены
    // IR теперь хранит TypeResolution напрямую, а metadata_lookup корректно
    // обрабатывает Generic типы и фасеты без дополнительной конвертации
}

impl<'a> SemanticVisitor for SemanticValidationVisitor<'a> {
    fn visit_node(&mut self, node: &SemanticNode, _context: &mut FlowContext) {
        match &node.kind {
            // Context-Aware валидация: обновляем директиву при входе в функцию/процедуру
            SemanticNodeKind::FunctionDeclaration { compiler_directive, name, .. } => {
                // Обновляем runtime контекст на основе директивы из AST
                tracing::debug!(
                    "FunctionDeclaration '{}': compiler_directive = {:?}",
                    name, compiler_directive
                );
                if let Some(directive) = compiler_directive {
                    self.current_execution_context.current_directive = *directive;
                    self.current_execution_context.in_function = Some(name.clone());
                } else {
                    // Нет директивы = Unknown контекст
                    self.current_execution_context.current_directive = bsl_shared::domain::CompilerDirective::Unknown;
                    self.current_execution_context.in_function = Some(name.clone());
                }
            }
            SemanticNodeKind::ProcedureDeclaration { compiler_directive, name, .. } => {
                // Обновляем runtime контекст на основе директивы из AST
                tracing::debug!(
                    "ProcedureDeclaration '{}': compiler_directive = {:?}",
                    name, compiler_directive
                );
                if let Some(directive) = compiler_directive {
                    self.current_execution_context.current_directive = *directive;
                    self.current_execution_context.in_function = Some(name.clone());
                } else {
                    // Нет директивы = Unknown контекст
                    self.current_execution_context.current_directive = bsl_shared::domain::CompilerDirective::Unknown;
                    self.current_execution_context.in_function = Some(name.clone());
                }
            }
            SemanticNodeKind::FunctionCall {
                function_name,
                object_name,
                // Phase 3: object_type теперь TypeResolution
                object_type: Some(obj_type),
                arg_types,
                ..
            } => {
                // MILESTONE 5.1: Генерируем ошибку для Unknown типов
                if obj_type.is_unknown() {
                    let error_kind = TypeErrorKind::UnknownTypeAccess {
                        variable_name: object_name.clone(),
                        member_name: function_name.clone(),
                    };
                    let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                    self.errors.push(diagnostic);
                    return;
                }

                // Phase 4: obj_type уже TypeResolution — используем напрямую
                // metadata_lookup.get_methods() уже обрабатывает Generic и фасеты корректно

                // 1. MILESTONE 3.6 Phase 3: Проверяем существование метода с передачей variable_name
                if let Some(error_kind) = self
                    .validator
                    .validate_method_exists_with_variable(
                        obj_type,  // Phase 4: Прямое использование TypeResolution
                        function_name,
                        object_name.clone(),  // Передаём имя переменной
                    )
                {
                    let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                    self.errors.push(diagnostic);
                    return; // Нет смысла проверять параметры если метод не существует
                }

                // 1.5. MILESTONE 3.11 Phase 3: Проверяем доступность метода в текущем контексте
                // Phase 3: Передаём type_name() вместо String
                if let Some(error_kind) = self.validate_method_call_context(
                    &obj_type.type_name(),
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

                // 2. MILESTONE 3.13: Проверяем типы параметров с объектным сравнением (v2)
                // Phase 3: Конвертируем Vec<TypeResolution> → Vec<String> для validate_call_v2
                let arg_types_str: Vec<String> = arg_types.iter()
                    .map(|tr| tr.type_name())
                    .collect();

                let validation_result = self.resolver.validate_call_v2(
                    Some(&obj_type.type_name()),
                    function_name,
                    &arg_types_str,
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
                access_kind: MemberAccessKind::Property,
                ..
            } => {
                // ✅ MILESTONE 3.16: Валидация доступа к объектам метаданных
                // Проверяем конструкции вида: Справочники.Контрагенты, Документы.ЗаказПокупателя
                // ВАЖНО: object_name содержит оригинальное имя ("Документы"),
                //        object_type содержит трансформированный тип ("ДокументМенеджер.ЗаказКлиента")
                // Phase 3: object_type теперь TypeResolution
                tracing::debug!(
                    "🔍 MemberAccess: object_name={:?}, object_type={}, member_name={}",
                    object_name, object_type.type_name(), member_name
                );
                if let Some(collection_name) = object_name {
                    tracing::debug!("🔍 Checking if '{}' is metadata collection: {}", collection_name, is_metadata_collection_name(collection_name));
                    if is_metadata_collection_name(collection_name) {
                        // Это обращение к коллекции метаданных - валидируем объект
                        if let Some(error_kind) = self.validate_metadata_member_access(
                            collection_name,
                            member_name,
                            Some(collection_name.clone()),
                        ) {
                            let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                            self.errors.push(diagnostic);
                        }
                        // Независимо от результата, не проверяем свойства для коллекций метаданных
                        // т.к. это не обычный доступ к свойству объекта
                        return;
                    }
                }

                // Phase 4: object_type уже TypeResolution — используем напрямую
                // metadata_lookup.get_properties() уже обрабатывает Generic и фасеты корректно
                // MILESTONE 5.1: Генерируем ошибку для Unknown типов
                if object_type.is_unknown() {
                    let error_kind = TypeErrorKind::UnknownTypeAccess {
                        variable_name: object_name.clone(),
                        member_name: member_name.clone(),
                    };
                    let diagnostic = error_kind.to_diagnostic_with_detail(node.span, self.detail_level);
                    self.errors.push(diagnostic);
                    return;
                }

                // ✅ MILESTONE 3.6 Phase 3: Передаём имя переменной
                if let Some(error_kind) = self
                    .validator
                    .validate_property_exists_with_variable(
                        object_type,  // Phase 4: Прямое использование TypeResolution
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
    use bsl_shared::domain::repository::TypeRepository;  // MILESTONE 3.16: Import trait for load_types
    use bsl_shared::domain::types::FacetKind;  // Phase 4: Moved from main imports (used only in tests)
    use bsl_shared::ir::{SemanticNode, SemanticNodeKind, Span};

    #[test]
    fn test_visitor_detects_nonexistent_method() {
        use std::sync::Arc;
        use bsl_shared::domain::types::TypeResolution;
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
                // Phase 3: object_type теперь TypeResolution
                object_type: Some(TypeResolution::explicit("Массив")),
                // Phase 3: arg_types теперь Vec<TypeResolution>
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
        use bsl_shared::domain::types::TypeResolution;
        let repository = Arc::new(bsl_shared::domain::repository::InMemoryTypeRepository::new());
        let metadata = TypeMetadataLookup::new(repository.clone());
        let validator = TypeValidator::new(&metadata);
        let resolver = TypeResolver::new(repository);
        let signature_index = SignatureIndex::new();
        let mut program = SemanticProgram::new();

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::MemberAccess {
                object_name: Some("МассивДанных".to_string()),
                // Phase 3: object_type теперь TypeResolution
                object_type: TypeResolution::explicit("Массив"),
                member_name: "НесуществующееСвойство".to_string(),
                access_kind: MemberAccessKind::Property,
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

    // === MILESTONE 3.16: Тесты валидации объектов метаданных ===

    #[test]
    fn test_is_metadata_collection_name() {
        // Русские названия
        assert!(is_metadata_collection_name("Справочники"));
        assert!(is_metadata_collection_name("Документы"));
        assert!(is_metadata_collection_name("РегистрыСведений"));
        assert!(is_metadata_collection_name("Перечисления"));

        // Английские названия
        assert!(is_metadata_collection_name("Catalogs"));
        assert!(is_metadata_collection_name("Documents"));
        assert!(is_metadata_collection_name("InformationRegisters"));
        assert!(is_metadata_collection_name("Enums"));

        // Не коллекции метаданных
        assert!(!is_metadata_collection_name("Массив"));
        assert!(!is_metadata_collection_name("ТаблицаЗначений"));
        assert!(!is_metadata_collection_name("Строка"));
    }

    #[test]
    fn test_collection_name_to_metadata_kind() {
        // Русские названия
        assert_eq!(collection_name_to_metadata_kind("Справочники"), Some(MetadataKind::Catalog));
        assert_eq!(collection_name_to_metadata_kind("Документы"), Some(MetadataKind::Document));
        assert_eq!(collection_name_to_metadata_kind("РегистрыСведений"), Some(MetadataKind::InformationRegister));
        assert_eq!(collection_name_to_metadata_kind("РегистрыНакопления"), Some(MetadataKind::AccumulationRegister));

        // Английские названия
        assert_eq!(collection_name_to_metadata_kind("Catalogs"), Some(MetadataKind::Catalog));
        assert_eq!(collection_name_to_metadata_kind("Documents"), Some(MetadataKind::Document));

        // Неизвестные
        assert_eq!(collection_name_to_metadata_kind("Массив"), None);
        assert_eq!(collection_name_to_metadata_kind("Unknown"), None);
    }

    #[test]
    fn test_visitor_validates_metadata_object_when_config_loaded() {
        use std::sync::Arc;
        use bsl_shared::domain::repository::InMemoryTypeRepository;
        use bsl_shared::domain::types::{RawTypeData, RawDataSource, TypeResolution};

        // Создаём репозиторий с конфигурационными типами
        let repository = Arc::new(InMemoryTypeRepository::new());

        // Добавляем справочник "Контрагенты"
        let catalog = RawTypeData {
            name: "Справочники.Контрагенты".to_string(),
            english_name: "Catalogs.Contractors".to_string(),
            description: "Справочник контрагентов".to_string(),
            category: "Справочники".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![FacetKind::Manager, FacetKind::Object],
            kind: Some(MetadataKind::Catalog),
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };
        repository.load_types(vec![catalog]).unwrap();

        let metadata = TypeMetadataLookup::new(repository.clone());
        let validator = TypeValidator::new(&metadata);
        let resolver = TypeResolver::new(repository);
        let signature_index = SignatureIndex::new();
        let mut program = SemanticProgram::new();

        // Тестируем обращение к несуществующему справочнику
        // Справочники.НесуществующийСправочник
        // ВАЖНО: object_name должен быть Some("Справочники") - так формируется в ast_to_ir.rs
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::MemberAccess {
                object_name: Some("Справочники".to_string()),
                // Phase 3: object_type теперь TypeResolution
                object_type: TypeResolution::explicit("СправочникМенеджер"),
                member_name: "НесуществующийСправочник".to_string(),
                access_kind: MemberAccessKind::Property,
            },
            span: Span::new(1, 0, 1, 35),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
        let mut context = FlowContext::new(program.symbols.root_scope);
        visitor.visit_node(&program.nodes[0], &mut context);

        let errors = visitor.into_errors();
        assert!(
            !errors.is_empty(),
            "Должна быть ошибка для несуществующего справочника"
        );
        assert!(errors[0].message.contains("Справочник"));
        assert!(errors[0].message.contains("не найден"));
    }

    #[test]
    fn test_visitor_no_error_for_existing_metadata_object() {
        use std::sync::Arc;
        use bsl_shared::domain::repository::InMemoryTypeRepository;
        use bsl_shared::domain::types::{RawTypeData, RawDataSource, TypeResolution};

        let repository = Arc::new(InMemoryTypeRepository::new());

        // Добавляем справочник "Контрагенты"
        let catalog = RawTypeData {
            name: "Справочники.Контрагенты".to_string(),
            english_name: "Catalogs.Contractors".to_string(),
            description: "Справочник контрагентов".to_string(),
            category: "Справочники".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![FacetKind::Manager, FacetKind::Object],
            kind: Some(MetadataKind::Catalog),
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };
        repository.load_types(vec![catalog]).unwrap();

        let metadata = TypeMetadataLookup::new(repository.clone());
        let validator = TypeValidator::new(&metadata);
        let resolver = TypeResolver::new(repository);
        let signature_index = SignatureIndex::new();
        let mut program = SemanticProgram::new();

        // Тестируем обращение к существующему справочнику
        // Справочники.Контрагенты
        // ВАЖНО: object_name должен быть Some("Справочники") - так формируется в ast_to_ir.rs
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::MemberAccess {
                object_name: Some("Справочники".to_string()),
                // Phase 3: object_type теперь TypeResolution
                object_type: TypeResolution::explicit("СправочникМенеджер"),
                member_name: "Контрагенты".to_string(),
                access_kind: MemberAccessKind::Property,
            },
            span: Span::new(1, 0, 1, 25),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
        let mut context = FlowContext::new(program.symbols.root_scope);
        visitor.visit_node(&program.nodes[0], &mut context);

        let errors = visitor.into_errors();
        assert!(
            errors.is_empty(),
            "Не должно быть ошибок для существующего справочника"
        );
    }

    #[test]
    fn test_visitor_no_error_when_config_not_loaded() {
        use std::sync::Arc;
        use bsl_shared::domain::repository::InMemoryTypeRepository;
        use bsl_shared::domain::types::TypeResolution;

        // Репозиторий БЕЗ конфигурационных типов
        let repository = Arc::new(InMemoryTypeRepository::new());
        let metadata = TypeMetadataLookup::new(repository.clone());
        let validator = TypeValidator::new(&metadata);
        let resolver = TypeResolver::new(repository);
        let signature_index = SignatureIndex::new();
        let mut program = SemanticProgram::new();

        // Тестируем обращение к несуществующему справочнику
        // Когда конфигурация не загружена, ошибка не должна появляться (graceful degradation)
        // ВАЖНО: object_name должен быть Some("Справочники") для прохождения через is_metadata_collection_name
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::MemberAccess {
                object_name: Some("Справочники".to_string()),
                // Phase 3: object_type теперь TypeResolution
                object_type: TypeResolution::explicit("СправочникМенеджер"),
                member_name: "НесуществующийСправочник".to_string(),
                access_kind: MemberAccessKind::Property,
            },
            span: Span::new(1, 0, 1, 35),
            scope_id: program.symbols.root_scope,
        });

        let mut visitor = SemanticValidationVisitor::new(&validator, &program, &resolver, &signature_index);
        let mut context = FlowContext::new(program.symbols.root_scope);
        visitor.visit_node(&program.nodes[0], &mut context);

        let errors = visitor.into_errors();
        // Когда конфигурация не загружена, пропускаем валидацию
        // Но может быть ошибка "свойство не существует" для типа "Справочники"
        // Это ожидаемое поведение - graceful degradation
        assert!(
            errors.is_empty() || !errors[0].message.contains("не найден в конфигурации"),
            "Не должно быть ошибки 'не найден в конфигурации' когда конфигурация не загружена"
        );
    }
}
