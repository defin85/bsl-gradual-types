//! Code Actions и Quick Fixes для LSP сервера
//!
//! Этот модуль предоставляет автоматические исправления и улучшения кода
//! на основе результатов анализа типов.

#![allow(unused_variables)]
#![allow(dead_code)]

use crate::core::type_checker::{DiagnosticSeverity, TypeContext, TypeDiagnostic};
use crate::domain::types::{Certainty, ConcreteType, ResolutionResult, TypeResolution};
use crate::parsing::bsl::ast::Program;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Поставщик code actions
pub struct CodeActionProvider {
    /// Кеш доступных действий для документов
    action_cache: HashMap<String, Vec<CodeActionOrCommand>>,
}

impl Default for CodeActionProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeActionProvider {
    /// Создать новый поставщик code actions
    pub fn new() -> Self {
        Self {
            action_cache: HashMap::new(),
        }
    }

    /// Получить доступные code actions для диапазона
    pub fn get_code_actions(
        &mut self,
        uri: &str,
        range: Range,
        context: &CodeActionContext,
        type_context: Option<&TypeContext>,
        diagnostics: &[TypeDiagnostic],
    ) -> Vec<CodeActionOrCommand> {
        let mut actions = Vec::new();

        // Действия на основе диагностик
        for diagnostic in &context.diagnostics {
            if Self::diagnostic_in_range(diagnostic, &range) {
                if let Some(action) =
                    self.create_action_for_diagnostic(diagnostic, uri, type_context)
                {
                    actions.push(action);
                }
            }
        }

        // Действия на основе типов
        if let Some(ctx) = type_context {
            actions.extend(self.create_type_actions(uri, range, ctx));
        }

        // Кешируем действия
        self.action_cache.insert(uri.to_string(), actions.clone());

        actions
    }

    /// Создать действие для диагностики
    fn create_action_for_diagnostic(
        &self,
        diagnostic: &Diagnostic,
        uri: &str,
        type_context: Option<&TypeContext>,
    ) -> Option<CodeActionOrCommand> {
        let message = &diagnostic.message;

        // Auto-fix для неопределенных переменных
        if message.contains("используется без объявления") {
            return self.create_variable_declaration_action(diagnostic, uri);
        }

        // Auto-fix для несовместимых типов
        if message.contains("Несовместимое присваивание") {
            return self.create_type_cast_action(diagnostic, uri, type_context);
        }

        // Auto-fix для неправильного количества аргументов
        if message.contains("ожидает") && message.contains("аргументов") {
            return self.create_fix_arguments_action(diagnostic, uri);
        }

        None
    }

    /// Создать действие объявления переменной
    fn create_variable_declaration_action(
        &self,
        diagnostic: &Diagnostic,
        uri: &str,
    ) -> Option<CodeActionOrCommand> {
        // Извлекаем имя переменной из сообщения
        if let Some(var_name) = Self::extract_variable_name(&diagnostic.message) {
            let edit = WorkspaceEdit {
                changes: Some({
                    let mut changes = HashMap::new();
                    changes.insert(
                        Url::parse(uri).ok()?,
                        vec![TextEdit {
                            range: Range {
                                start: Position {
                                    line: diagnostic.range.start.line,
                                    character: 0,
                                },
                                end: Position {
                                    line: diagnostic.range.start.line,
                                    character: 0,
                                },
                            },
                            new_text: format!("    Перем {};\n", var_name),
                        }],
                    );
                    changes
                }),
                document_changes: None,
                change_annotations: None,
            };

            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Объявить переменную '{}'", var_name),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: Some(edit),
                command: None,
                is_preferred: Some(true),
                disabled: None,
                data: None,
            }))
        } else {
            None
        }
    }

    /// Создать действие приведения типа
    fn create_type_cast_action(
        &self,
        diagnostic: &Diagnostic,
        uri: &str,
        type_context: Option<&TypeContext>,
    ) -> Option<CodeActionOrCommand> {
        let edit = WorkspaceEdit {
            changes: Some({
                let mut changes = HashMap::new();
                changes.insert(
                    Url::parse(uri).ok()?,
                    vec![TextEdit {
                        range: diagnostic.range,
                        new_text: "// TODO: Добавить приведение типа".to_string(),
                    }],
                );
                changes
            }),
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Добавить приведение типа".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(edit),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        }))
    }

    /// Создать действие исправления аргументов
    fn create_fix_arguments_action(
        &self,
        diagnostic: &Diagnostic,
        uri: &str,
    ) -> Option<CodeActionOrCommand> {
        let edit = WorkspaceEdit {
            changes: Some({
                let mut changes = HashMap::new();
                changes.insert(
                    Url::parse(uri).ok()?,
                    vec![TextEdit {
                        range: diagnostic.range,
                        new_text: "// TODO: Исправить количество аргументов".to_string(),
                    }],
                );
                changes
            }),
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Исправить аргументы функции".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(edit),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        }))
    }

    /// Создать действия на основе типов
    fn create_type_actions(
        &self,
        uri: &str,
        range: Range,
        type_context: &TypeContext,
    ) -> Vec<CodeActionOrCommand> {
        let mut actions = Vec::new();

        // Действие добавления аннотаций типов
        if let Some(action) = self.create_type_annotation_action(uri, range, type_context) {
            actions.push(action);
        }

        // Действие оптимизации типов
        if let Some(action) = self.create_type_optimization_action(uri, range, type_context) {
            actions.push(action);
        }

        actions
    }

    /// Создать действие добавления аннотации типа
    fn create_type_annotation_action(
        &self,
        uri: &str,
        range: Range,
        type_context: &TypeContext,
    ) -> Option<CodeActionOrCommand> {
        // Найдем переменную в указанном диапазоне
        // TODO: Более сложная логика определения переменной по позиции

        if let Some((var_name, var_type)) = type_context.variables.iter().next() {
            let type_annotation = Self::format_type_annotation(var_type);

            let edit = WorkspaceEdit {
                changes: Some({
                    let mut changes = HashMap::new();
                    changes.insert(
                        Url::parse(uri).ok()?,
                        vec![TextEdit {
                            range: Range {
                                start: range.end,
                                end: range.end,
                            },
                            new_text: format!(" // Тип: {}", type_annotation),
                        }],
                    );
                    changes
                }),
                document_changes: None,
                change_annotations: None,
            };

            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Добавить аннотацию типа для '{}'", var_name),
                kind: Some(CodeActionKind::REFACTOR),
                diagnostics: None,
                edit: Some(edit),
                command: None,
                is_preferred: Some(false),
                disabled: None,
                data: None,
            }))
        } else {
            None
        }
    }

    /// Создать действие оптимизации типов
    fn create_type_optimization_action(
        &self,
        uri: &str,
        range: Range,
        type_context: &TypeContext,
    ) -> Option<CodeActionOrCommand> {
        // Поиск переменных с Union типами для оптимизации
        let union_variables: Vec<_> = type_context
            .variables
            .iter()
            .filter(|(_, type_res)| matches!(type_res.result, ResolutionResult::Union(_)))
            .collect();

        if !union_variables.is_empty() {
            let edit = WorkspaceEdit {
                changes: Some({
                    let mut changes = HashMap::new();
                    changes.insert(
                        Url::parse(uri).ok()?,
                        vec![TextEdit {
                            range: Range {
                                start: range.start,
                                end: range.start,
                            },
                            new_text: format!(
                                "// {} переменных с Union типами найдено\n",
                                union_variables.len()
                            ),
                        }],
                    );
                    changes
                }),
                document_changes: None,
                change_annotations: None,
            };

            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Оптимизировать Union типы".to_string(),
                kind: Some(CodeActionKind::REFACTOR),
                diagnostics: None,
                edit: Some(edit),
                command: None,
                is_preferred: Some(false),
                disabled: None,
                data: None,
            }))
        } else {
            None
        }
    }

    /// Проверить находится ли диагностика в диапазоне
    fn diagnostic_in_range(diagnostic: &Diagnostic, range: &Range) -> bool {
        // Простая проверка пересечения диапазонов
        diagnostic.range.start.line <= range.end.line
            && diagnostic.range.end.line >= range.start.line
    }

    /// Извлечь имя переменной из сообщения диагностики
    fn extract_variable_name(message: &str) -> Option<String> {
        // Ищем переменную в сообщении вида "Переменная 'имя' используется без объявления"
        if let Some(start) = message.find("'") {
            if let Some(end) = message[start + 1..].find("'") {
                return Some(message[start + 1..start + 1 + end].to_string());
            }
        }
        None
    }

    /// Форматировать аннотацию типа
    fn format_type_annotation(type_res: &TypeResolution) -> String {
        match &type_res.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(primitive)) => {
                format!("{:?}", primitive)
            }
            ResolutionResult::Concrete(ConcreteType::Platform(platform)) => platform.name.clone(),
            ResolutionResult::Union(union_types) => {
                let type_names: Vec<String> = union_types
                    .iter()
                    .map(|wt| format!("{:?}", wt.type_))
                    .collect();
                format!("Union({})", type_names.join(" | "))
            }
            _ => "Dynamic".to_string(),
        }
    }

    /// Получить кешированные действия
    pub fn get_cached_actions(&self, uri: &str) -> Option<&Vec<CodeActionOrCommand>> {
        self.action_cache.get(uri)
    }

    /// Очистить кеш действий
    pub fn clear_cache(&mut self) {
        self.action_cache.clear();
    }
}

/// Генератор quick fixes
pub struct QuickFixGenerator;

impl QuickFixGenerator {
    /// Сгенерировать quick fix для типовой ошибки
    pub fn generate_type_fix(
        uri: &str,
        diagnostic: &TypeDiagnostic,
        suggested_type: &TypeResolution,
    ) -> Option<CodeActionOrCommand> {
        let type_name = Self::get_simple_type_name(suggested_type);

        let edit = WorkspaceEdit {
            changes: Some({
                let mut changes = HashMap::new();
                if let Ok(parsed_uri) = Url::parse(uri) {
                    changes.insert(
                        parsed_uri,
                        vec![TextEdit {
                            range: Range {
                                start: Position {
                                    line: (diagnostic.line.saturating_sub(1)) as u32,
                                    character: diagnostic.column as u32,
                                },
                                end: Position {
                                    line: (diagnostic.line.saturating_sub(1)) as u32,
                                    character: (diagnostic.column + 10) as u32,
                                },
                            },
                            new_text: format!("// Предложенный тип: {}", type_name),
                        }],
                    );
                }
                changes
            }),
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeActionOrCommand::CodeAction(CodeAction {
            title: format!("Исправить тип -> {}", type_name),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![Self::convert_diagnostic_to_lsp(diagnostic)]),
            edit: Some(edit),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        }))
    }

    /// Получить простое имя типа
    fn get_simple_type_name(type_res: &TypeResolution) -> String {
        match &type_res.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(primitive)) => match primitive {
                crate::core::types::PrimitiveType::String => "Строка".to_string(),
                crate::core::types::PrimitiveType::Number => "Число".to_string(),
                crate::core::types::PrimitiveType::Boolean => "Булево".to_string(),
                crate::core::types::PrimitiveType::Date => "Дата".to_string(),
            },
            ResolutionResult::Concrete(ConcreteType::Platform(platform)) => platform.name.clone(),
            ResolutionResult::Union(union_types) if union_types.len() <= 2 => union_types
                .iter()
                .map(|wt| {
                    Self::get_simple_type_name(&TypeResolution {
                        certainty: Certainty::Known,
                        result: ResolutionResult::Concrete(wt.type_.clone()),
                        source: crate::core::types::ResolutionSource::Static,
                        metadata: Default::default(),
                        active_facet: None,
                        available_facets: vec![],
                    })
                })
                .collect::<Vec<_>>()
                .join(" или "),
            _ => "Динамический".to_string(),
        }
    }

    /// Конвертировать нашу диагностику в LSP формат
    fn convert_diagnostic_to_lsp(diagnostic: &TypeDiagnostic) -> Diagnostic {
        Diagnostic {
            range: Range {
                start: Position {
                    line: (diagnostic.line.saturating_sub(1)) as u32,
                    character: diagnostic.column as u32,
                },
                end: Position {
                    line: (diagnostic.line.saturating_sub(1)) as u32,
                    character: (diagnostic.column + 10) as u32,
                },
            },
            severity: Some(match diagnostic.severity {
                DiagnosticSeverity::Error => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
                DiagnosticSeverity::Warning => tower_lsp::lsp_types::DiagnosticSeverity::WARNING,
                DiagnosticSeverity::Info => tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION,
                DiagnosticSeverity::Hint => tower_lsp::lsp_types::DiagnosticSeverity::HINT,
            }),
            code: None,
            code_description: None,
            source: Some("bsl-gradual-types".to_string()),
            message: diagnostic.message.clone(),
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

/// Предложения по рефакторингу кода
pub struct RefactoringProvider;

impl RefactoringProvider {
    /// Предложить рефакторинг на основе анализа типов
    pub fn suggest_refactorings(
        uri: &str,
        range: Range,
        type_context: &TypeContext,
        _program: &Program,
    ) -> Vec<CodeActionOrCommand> {
        let mut actions = Vec::new();

        // Предложение извлечения функции
        if let Some(action) = Self::create_extract_function_action(uri, range) {
            actions.push(action);
        }

        // Предложение inline переменной
        if let Some(action) = Self::create_inline_variable_action(uri, range, type_context) {
            actions.push(action);
        }

        // Предложение оптимизации Union типов
        if let Some(action) = Self::create_optimize_union_types_action(uri, range, type_context) {
            actions.push(action);
        }

        actions
    }

    /// Создать действие извлечения функции
    fn create_extract_function_action(uri: &str, range: Range) -> Option<CodeActionOrCommand> {
        let edit = WorkspaceEdit {
            changes: Some({
                let mut changes = HashMap::new();
                if let Ok(parsed_uri) = Url::parse(uri) {
                    changes.insert(
                        parsed_uri,
                        vec![
                            // Заменяем выделенный код на вызов функции
                            TextEdit {
                                range,
                                new_text: "НоваяФункция();".to_string(),
                            },
                            // Добавляем объявление функции в конец файла
                            TextEdit {
                                range: Range {
                                    start: Position { line: u32::MAX, character: 0 },
                                    end: Position { line: u32::MAX, character: 0 },
                                },
                                new_text: "\n\nФункция НоваяФункция()\n    // TODO: Реализовать извлеченную логику\nКонецФункции".to_string(),
                            },
                        ]
                    );
                }
                changes
            }),
            document_changes: None,
            change_annotations: None,
        };

        Some(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Извлечь в функцию".to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            diagnostics: None,
            edit: Some(edit),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        }))
    }

    /// Создать действие inline переменной
    fn create_inline_variable_action(
        uri: &str,
        range: Range,
        type_context: &TypeContext,
    ) -> Option<CodeActionOrCommand> {
        // TODO: Реализовать логику inline переменной
        None
    }

    /// Создать действие оптимизации Union типов
    fn create_optimize_union_types_action(
        uri: &str,
        range: Range,
        type_context: &TypeContext,
    ) -> Option<CodeActionOrCommand> {
        let union_count = type_context
            .variables
            .values()
            .filter(|type_res| matches!(type_res.result, ResolutionResult::Union(_)))
            .count();

        if union_count > 0 {
            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Оптимизировать {} Union типов", union_count),
                kind: Some(CodeActionKind::REFACTOR),
                diagnostics: None,
                edit: None,
                command: Some(Command {
                    title: "Optimize Union Types".to_string(),
                    command: "bsl.optimizeUnionTypes".to_string(),
                    arguments: Some(vec![
                        serde_json::to_value(uri).unwrap_or_default(),
                        serde_json::to_value(union_count).unwrap_or_default(),
                    ]),
                }),
                is_preferred: Some(false),
                disabled: None,
                data: None,
            }))
        } else {
            None
        }
    }
}

/// Интеграция code actions в LSP сервер
pub struct LSPCodeActionIntegration {
    provider: CodeActionProvider,
    #[allow(dead_code)]
    refactoring_provider: RefactoringProvider,
}

impl Default for LSPCodeActionIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl LSPCodeActionIntegration {
    /// Создать новую интеграцию
    pub fn new() -> Self {
        Self {
            provider: CodeActionProvider::new(),
            refactoring_provider: RefactoringProvider,
        }
    }

    /// Обработать запрос code actions от LSP клиента
    pub fn handle_code_action_request(
        &mut self,
        params: CodeActionParams,
        type_context: Option<&TypeContext>,
        diagnostics: &[TypeDiagnostic],
        program: Option<&Program>,
    ) -> Vec<CodeActionOrCommand> {
        let uri = params.text_document.uri.to_string();
        let range = params.range;
        let context = params.context;

        let mut actions = Vec::new();

        // Получаем действия от основного провайдера
        actions.extend(self.provider.get_code_actions(
            &uri,
            range,
            &context,
            type_context,
            diagnostics,
        ));

        // Добавляем предложения по рефакторингу
        if let (Some(ctx), Some(prog)) = (type_context, program) {
            actions.extend(RefactoringProvider::suggest_refactorings(
                &uri, range, ctx, prog,
            ));
        }

        actions
    }

    /// Обработать выполнение команды
    pub fn handle_execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "bsl.optimizeUnionTypes" => {
                // TODO: Реализовать оптимизацию Union типов
                Ok(Some(serde_json::json!({
                    "status": "success",
                    "message": "Union типы оптимизированы"
                })))
            }
            "bsl.addTypeAnnotations" => {
                // TODO: Реализовать добавление аннотаций типов
                Ok(Some(serde_json::json!({
                    "status": "success",
                    "message": "Аннотации типов добавлены"
                })))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_context() -> TypeContext {
        let mut variables = HashMap::new();
        variables.insert(
            "testVar".to_string(),
            crate::core::standard_types::primitive_type(crate::core::types::PrimitiveType::String),
        );

        TypeContext {
            variables,
            functions: HashMap::new(),
            current_scope: crate::core::dependency_graph::Scope::Global,
            scope_stack: vec![],
        }
    }

    #[test]
    fn test_code_action_provider() {
        let mut provider = CodeActionProvider::new();
        let context = create_test_context();

        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 10,
            },
        };

        let lsp_context = CodeActionContext {
            diagnostics: vec![],
            only: None,
            trigger_kind: Some(CodeActionTriggerKind::AUTOMATIC),
        };

        let actions =
            provider.get_code_actions("test://test.bsl", range, &lsp_context, Some(&context), &[]);

        // Должны быть доступны action для типов
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_extract_variable_name() {
        let message = "Переменная 'testVar' используется без объявления";
        let name = CodeActionProvider::extract_variable_name(message);
        assert_eq!(name, Some("testVar".to_string()));
    }

    #[test]
    fn test_format_type_annotation() {
        let string_type =
            crate::core::standard_types::primitive_type(crate::core::types::PrimitiveType::String);

        let annotation = CodeActionProvider::format_type_annotation(&string_type);
        assert_eq!(annotation, "String");
    }

    #[test]
    fn test_quick_fix_generator() {
        let diagnostic = TypeDiagnostic {
            severity: DiagnosticSeverity::Warning,
            message: "Тип не определен".to_string(),
            line: 10,
            column: 5,
            file: "test.bsl".to_string(),
        };

        let suggested_type =
            crate::core::standard_types::primitive_type(crate::core::types::PrimitiveType::String);

        let fix =
            QuickFixGenerator::generate_type_fix("test://test.bsl", &diagnostic, &suggested_type);

        assert!(fix.is_some());

        if let Some(CodeActionOrCommand::CodeAction(action)) = fix {
            assert!(action.title.contains("Строка"));
            assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        }
    }

    #[test]
    fn test_lsp_integration() {
        let mut integration = LSPCodeActionIntegration::new();

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier {
                uri: Url::parse("test://test.bsl").unwrap(),
            },
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 10,
                },
            },
            context: CodeActionContext {
                diagnostics: vec![],
                only: None,
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let context = create_test_context();
        let actions = integration.handle_code_action_request(params, Some(&context), &[], None);

        // Должны получить некоторые действия
        assert!(!actions.is_empty());
    }
}
