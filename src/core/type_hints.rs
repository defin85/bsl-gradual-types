//! Type Hints и Inline Type Information
//!
//! Этот модуль предоставляет возможности для отображения типов прямо в коде
//! через LSP inlay hints и другие механизмы визуализации типов.

use tower_lsp::lsp_types::*;
use crate::core::types::{TypeResolution, ResolutionResult, ConcreteType, Certainty};
use crate::core::type_checker::TypeContext;
use crate::parser::ast::{Statement, Expression, Program};

/// Поставщик type hints для LSP
pub struct TypeHintsProvider {
    /// Настройки отображения hints
    pub settings: TypeHintsSettings,
}

/// Настройки type hints
#[derive(Debug, Clone)]
pub struct TypeHintsSettings {
    /// Показывать типы переменных
    pub show_variable_types: bool,
    /// Показывать типы возврата функций
    pub show_return_types: bool,
    /// Показывать типы параметров
    pub show_parameter_types: bool,
    /// Показывать Union типы развернуто
    pub show_union_details: bool,
    /// Минимальный уровень уверенности для показа
    pub min_certainty: f32,
    /// Максимальная длина hint
    pub max_hint_length: usize,
}

impl Default for TypeHintsSettings {
    fn default() -> Self {
        Self {
            show_variable_types: true,
            show_return_types: true,
            show_parameter_types: false, // Может быть слишком многословно
            show_union_details: true,
            min_certainty: 0.7, // Показывать только достаточно уверенные типы
            max_hint_length: 50,
        }
    }
}

impl TypeHintsProvider {
    /// Создать новый поставщик hints
    pub fn new(settings: TypeHintsSettings) -> Self {
        Self { settings }
    }
    
    /// Получить inlay hints для документа
    pub fn get_inlay_hints(
        &self,
        program: &Program,
        type_context: &TypeContext,
        range: Range,
    ) -> Vec<InlayHint> {
        let mut hints = Vec::new();
        
        // Анализируем statements в указанном диапазоне
        for statement in &program.statements {
            hints.extend(self.process_statement(statement, type_context, &range));
        }
        
        hints
    }
    
    /// Обработать statement для извлечения hints
    fn process_statement(
        &self,
        statement: &Statement,
        type_context: &TypeContext,
        range: &Range,
    ) -> Vec<InlayHint> {
        let mut hints = Vec::new();
        
        match statement {
            Statement::Assignment { target, value } => {
                hints.extend(self.process_assignment(target, value, type_context, range));
            }
            
            Statement::VarDeclaration { name, .. } => {
                if let Some(var_type) = type_context.variables.get(name) {
                    if self.should_show_hint(var_type) {
                        hints.push(self.create_variable_type_hint(name, var_type, 0, 0));
                    }
                }
            }
            
            Statement::FunctionDecl { name, body, .. } => {
                // Hint для типа возврата функции
                if self.settings.show_return_types {
                    if let Some(func_sig) = type_context.functions.get(name) {
                        if self.should_show_hint(&func_sig.return_type) {
                            hints.push(self.create_return_type_hint(name, &func_sig.return_type, 0, 0));
                        }
                    }
                }
                
                // Рекурсивно обрабатываем тело функции
                for stmt in body {
                    hints.extend(self.process_statement(stmt, type_context, range));
                }
            }
            
            Statement::If { condition, then_branch, else_if_branches, else_branch } => {
                // Hints для условий
                hints.extend(self.process_expression(condition, type_context, range));
                
                // Рекурсивно обрабатываем ветки
                for stmt in then_branch {
                    hints.extend(self.process_statement(stmt, type_context, range));
                }
                
                for (cond, branch) in else_if_branches {
                    hints.extend(self.process_expression(cond, type_context, range));
                    for stmt in branch {
                        hints.extend(self.process_statement(stmt, type_context, range));
                    }
                }
                
                if let Some(branch) = else_branch {
                    for stmt in branch {
                        hints.extend(self.process_statement(stmt, type_context, range));
                    }
                }
            }
            
            _ => {}
        }
        
        hints
    }
    
    /// Обработать присваивание
    fn process_assignment(
        &self,
        target: &Expression,
        value: &Expression,
        type_context: &TypeContext,
        range: &Range,
    ) -> Vec<InlayHint> {
        let mut hints = Vec::new();
        
        if let Expression::Identifier(var_name) = target {
            if let Some(var_type) = type_context.variables.get(var_name) {
                if self.should_show_hint(var_type) {
                    hints.push(self.create_assignment_type_hint(var_name, var_type, 0, 0));
                }
            }
        }
        
        // Обрабатываем выражение справа
        hints.extend(self.process_expression(value, type_context, range));
        
        hints
    }
    
    /// Обработать выражение
    fn process_expression(
        &self,
        expression: &Expression,
        type_context: &TypeContext,
        range: &Range,
    ) -> Vec<InlayHint> {
        let mut hints = Vec::new();
        
        match expression {
            Expression::Call { function, args } => {
                // Hints для вызовов функций
                if let Expression::Identifier(func_name) = &**function {
                    if let Some(func_sig) = type_context.functions.get(func_name) {
                        if self.settings.show_return_types && self.should_show_hint(&func_sig.return_type) {
                            hints.push(self.create_function_call_hint(func_name, &func_sig.return_type, 0, 0));
                        }
                    }
                }
                
                // Рекурсивно обрабатываем аргументы
                for arg in args {
                    hints.extend(self.process_expression(arg, type_context, range));
                }
            }
            
            Expression::Binary { left, right, .. } => {
                hints.extend(self.process_expression(left, type_context, range));
                hints.extend(self.process_expression(right, type_context, range));
            }
            
            Expression::Unary { operand, .. } => {
                hints.extend(self.process_expression(operand, type_context, range));
            }
            
            _ => {}
        }
        
        hints
    }
    
    /// Проверить нужно ли показывать hint для типа
    fn should_show_hint(&self, type_res: &TypeResolution) -> bool {
        // Проверяем уровень уверенности
        
        
        // Для тестов показываем все типы
        match type_res.certainty {
            Certainty::Known => true,
            Certainty::Inferred(conf) => conf >= self.settings.min_certainty,
            Certainty::Unknown => false,
        }
    }
    
    /// Создать hint для типа переменной
    fn create_variable_type_hint(
        &self,
        var_name: &str,
        var_type: &TypeResolution,
        line: u32,
        character: u32,
    ) -> InlayHint {
        let type_text = self.format_type_hint(var_type);
        
        InlayHint {
            position: Position { line, character },
            label: InlayHintLabel::String(format!(": {}", type_text)),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: Some(InlayHintTooltip::String(format!(
                "Тип переменной '{}' выведен как: {}",
                var_name, 
                self.format_detailed_type(var_type)
            ))),
            padding_left: Some(false),
            padding_right: Some(true),
            data: None,
        }
    }
    
    /// Создать hint для присваивания
    fn create_assignment_type_hint(
        &self,
        var_name: &str,
        var_type: &TypeResolution,
        line: u32,
        character: u32,
    ) -> InlayHint {
        let type_text = self.format_type_hint(var_type);
        
        InlayHint {
            position: Position { line, character },
            label: InlayHintLabel::String(format!(" // {}", type_text)),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: Some(InlayHintTooltip::String(format!(
                "Присваивание переменной '{}' типа: {}",
                var_name,
                self.format_detailed_type(var_type)
            ))),
            padding_left: Some(true),
            padding_right: Some(false),
            data: None,
        }
    }
    
    /// Создать hint для типа возврата функции
    fn create_return_type_hint(
        &self,
        func_name: &str,
        return_type: &TypeResolution,
        line: u32,
        character: u32,
    ) -> InlayHint {
        let type_text = self.format_type_hint(return_type);
        
        InlayHint {
            position: Position { line, character },
            label: InlayHintLabel::String(format!(" -> {}", type_text)),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: Some(InlayHintTooltip::String(format!(
                "Функция '{}' возвращает: {}",
                func_name,
                self.format_detailed_type(return_type)
            ))),
            padding_left: Some(true),
            padding_right: Some(false),
            data: None,
        }
    }
    
    /// Создать hint для вызова функции
    fn create_function_call_hint(
        &self,
        func_name: &str,
        return_type: &TypeResolution,
        line: u32,
        character: u32,
    ) -> InlayHint {
        let type_text = self.format_type_hint(return_type);
        
        InlayHint {
            position: Position { line, character },
            label: InlayHintLabel::String(format!(" : {}", type_text)),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: Some(InlayHintTooltip::String(format!(
                "Результат вызова '{}()': {}",
                func_name,
                self.format_detailed_type(return_type)
            ))),
            padding_left: Some(true),
            padding_right: Some(false),
            data: None,
        }
    }
    
    /// Форматировать тип для краткого отображения в hint
    fn format_type_hint(&self, type_res: &TypeResolution) -> String {
        let base_text = match &type_res.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(primitive)) => {
                match primitive {
                    crate::core::types::PrimitiveType::String => "Строка",
                    crate::core::types::PrimitiveType::Number => "Число", 
                    crate::core::types::PrimitiveType::Boolean => "Булево",
                    crate::core::types::PrimitiveType::Date => "Дата",
                }.to_string()
            }
            ResolutionResult::Concrete(ConcreteType::Platform(platform)) => {
                platform.name.clone()
            }
            ResolutionResult::Concrete(ConcreteType::Configuration(config)) => {
                format!("{:?}.{}", config.kind, config.name)
            }
            ResolutionResult::Union(union_types) => {
                if self.settings.show_union_details && union_types.len() <= 3 {
                    let type_names: Vec<String> = union_types.iter()
                        .map(|wt| self.format_concrete_type(&wt.type_))
                        .collect();
                    type_names.join("|")
                } else {
                    format!("Union<{}>", union_types.len())
                }
            }
            ResolutionResult::Dynamic => "Dynamic".to_string(),
            ResolutionResult::Conditional(_) => "Conditional".to_string(),
            ResolutionResult::Contextual(_) => "Contextual".to_string(),
            ResolutionResult::Concrete(ConcreteType::Special(special)) => format!("{:?}", special),
            ResolutionResult::Concrete(ConcreteType::GlobalFunction(func)) => format!("Func({})", func.name),
        };
        
        // Добавляем индикатор уверенности
        let confidence_indicator = match type_res.certainty {
            Certainty::Known => "",
            Certainty::Inferred(conf) if conf >= 0.9 => "~",
            Certainty::Inferred(_) => "?",
            Certainty::Unknown => "❓",
        };
        
        let full_text = format!("{}{}", base_text, confidence_indicator);
        
        // Обрезаем если слишком длинный
        if full_text.len() > self.settings.max_hint_length {
            format!("{}...", &full_text[..self.settings.max_hint_length.saturating_sub(3)])
        } else {
            full_text
        }
    }
    
    /// Форматировать конкретный тип
    fn format_concrete_type(&self, concrete_type: &ConcreteType) -> String {
        match concrete_type {
            ConcreteType::Primitive(primitive) => {
                match primitive {
                    crate::core::types::PrimitiveType::String => "Str",
                    crate::core::types::PrimitiveType::Number => "Num",
                    crate::core::types::PrimitiveType::Boolean => "Bool",
                    crate::core::types::PrimitiveType::Date => "Date",
                }.to_string()
            }
            ConcreteType::Platform(platform) => {
                // Сокращаем длинные имена платформенных типов
                if platform.name.len() > 15 {
                    format!("{}...", &platform.name[..12])
                } else {
                    platform.name.clone()
                }
            }
            ConcreteType::Configuration(config) => {
                format!("{:?}.{}", config.kind, 
                    if config.name.len() > 10 { 
                        format!("{}...", &config.name[..7])
                    } else { 
                        config.name.clone() 
                    }
                )
            }
            ConcreteType::Special(special) => format!("{:?}", special),
            ConcreteType::GlobalFunction(func) => {
                format!("Func({})", 
                    if func.name.len() > 10 { 
                        format!("{}...", &func.name[..7])
                    } else { 
                        func.name.clone() 
                    }
                )
            }
        }
    }
    
    /// Форматировать детальную информацию о типе
    fn format_detailed_type(&self, type_res: &TypeResolution) -> String {
        match &type_res.result {
            ResolutionResult::Concrete(concrete) => {
                format!("{} (уверенность: {})", 
                    self.format_concrete_type_detailed(concrete),
                    self.format_certainty(&type_res.certainty)
                )
            }
            ResolutionResult::Union(union_types) => {
                let types_info: Vec<String> = union_types.iter()
                    .map(|wt| format!("{} ({}%)", 
                        self.format_concrete_type(&wt.type_),
                        (wt.weight * 100.0) as u32
                    ))
                    .collect();
                format!("Union: {}", types_info.join(", "))
            }
            ResolutionResult::Dynamic => {
                format!("Динамический тип (уверенность: {})",
                    self.format_certainty(&type_res.certainty))
            }
            ResolutionResult::Conditional(_) => "Условный тип".to_string(),
            ResolutionResult::Contextual(_) => "Контекстный тип".to_string(),
        }
    }
    
    /// Форматировать конкретный тип детально
    fn format_concrete_type_detailed(&self, concrete_type: &ConcreteType) -> String {
        match concrete_type {
            ConcreteType::Platform(platform) => {
                format!("{} (методы: {}, свойства: {})", 
                    platform.name, 
                    platform.methods.len(),
                    platform.properties.len()
                )
            }
            ConcreteType::Configuration(config) => {
                format!("{:?}.{} (атрибуты: {})", 
                    config.kind, 
                    config.name,
                    config.attributes.len()
                )
            }
            ConcreteType::GlobalFunction(func) => {
                format!("Глобальная функция {} (параметры: {})", 
                    func.name,
                    func.parameters.len()
                )
            }
            _ => format!("{:?}", concrete_type),
        }
    }
    
    /// Форматировать уровень уверенности
    fn format_certainty(&self, certainty: &Certainty) -> String {
        match certainty {
            Certainty::Known => "100%".to_string(),
            Certainty::Inferred(conf) => format!("{:.0}%", conf * 100.0),
            Certainty::Unknown => "неизвестно".to_string(),
        }
    }
}

/// Поставщик semantic tokens для подсветки
pub struct SemanticTokensProvider {
    /// Настройки semantic highlighting
    pub settings: SemanticHighlightingSettings,
}

/// Настройки semantic highlighting
#[derive(Debug, Clone)]
pub struct SemanticHighlightingSettings {
    /// Подсвечивать переменные по типам
    pub highlight_variables_by_type: bool,
    /// Подсвечивать функции по сигнатурам
    pub highlight_functions: bool,
    /// Подсвечивать Union типы специально
    pub highlight_union_types: bool,
    /// Подсвечивать неопределенные типы
    pub highlight_unknown_types: bool,
}

impl Default for SemanticHighlightingSettings {
    fn default() -> Self {
        Self {
            highlight_variables_by_type: true,
            highlight_functions: true,
            highlight_union_types: true,
            highlight_unknown_types: true,
        }
    }
}

impl SemanticTokensProvider {
    /// Создать новый поставщик semantic tokens
    pub fn new(settings: SemanticHighlightingSettings) -> Self {
        Self { settings }
    }
    
    /// Получить semantic tokens для документа
    pub fn get_semantic_tokens(
        &self,
        program: &Program,
        type_context: &TypeContext,
    ) -> SemanticTokens {
        let mut tokens = Vec::new();
        
        // Обрабатываем программу для извлечения токенов
        for statement in &program.statements {
            tokens.extend(self.process_statement_for_tokens(statement, type_context));
        }
        
        SemanticTokens {
            result_id: None,
            data: tokens.into_iter().map(|t| tower_lsp::lsp_types::SemanticToken {
                delta_line: t.delta_line,
                delta_start: t.delta_start,
                length: t.length,
                token_type: t.token_type,
                token_modifiers_bitset: t.token_modifiers_bitset,
            }).collect(),
        }
    }
    
    /// Обработать statement для semantic tokens
    fn process_statement_for_tokens(
        &self,
        statement: &Statement,
        type_context: &TypeContext,
    ) -> Vec<SemanticToken> {
        let mut tokens = Vec::new();
        
        match statement {
            Statement::Assignment { target, .. } => {
                if let Expression::Identifier(var_name) = target {
                    if let Some(var_type) = type_context.variables.get(var_name) {
                        tokens.push(self.create_variable_token(var_name, var_type, 0, 0));
                    }
                }
            }
            
            Statement::VarDeclaration { name, .. } => {
                if let Some(var_type) = type_context.variables.get(name) {
                    tokens.push(self.create_variable_declaration_token(name, var_type, 0, 0));
                }
            }
            
            Statement::FunctionDecl { name, .. } => {
                if let Some(func_sig) = type_context.functions.get(name) {
                    tokens.push(self.create_function_token(name, func_sig, 0, 0));
                }
            }
            
            _ => {}
        }
        
        tokens
    }
    
    /// Создать токен для переменной
    fn create_variable_token(
        &self,
        var_name: &str,
        var_type: &TypeResolution,
        line: u32,
        start_char: u32,
    ) -> SemanticToken {
        let token_type = self.get_variable_token_type(var_type);
        let modifiers = self.get_variable_modifiers(var_type);
        
        SemanticToken {
            delta_line: line,
            delta_start: start_char,
            length: var_name.len() as u32,
            token_type,
            token_modifiers_bitset: modifiers,
        }
    }
    
    /// Создать токен для объявления переменной
    fn create_variable_declaration_token(
        &self,
        var_name: &str,
        var_type: &TypeResolution,
        line: u32,
        start_char: u32,
    ) -> SemanticToken {
        SemanticToken {
            delta_line: line,
            delta_start: start_char,
            length: var_name.len() as u32,
            token_type: SEMANTIC_TOKEN_TYPE_VARIABLE,
            token_modifiers_bitset: if matches!(var_type.certainty, Certainty::Unknown) {
                1 << SEMANTIC_TOKEN_MODIFIER_DEPRECATED
            } else {
                0
            },
        }
    }
    
    /// Создать токен для функции
    fn create_function_token(
        &self,
        func_name: &str,
        func_sig: &crate::core::type_checker::FunctionSignature,
        line: u32,
        start_char: u32,
    ) -> SemanticToken {
        SemanticToken {
            delta_line: line,
            delta_start: start_char,
            length: func_name.len() as u32,
            token_type: SEMANTIC_TOKEN_TYPE_FUNCTION,
            token_modifiers_bitset: if func_sig.exported {
                1 << SEMANTIC_TOKEN_MODIFIER_STATIC
            } else {
                0
            },
        }
    }
    
    /// Получить тип токена для переменной на основе её типа
    fn get_variable_token_type(&self, var_type: &TypeResolution) -> u32 {
        match &var_type.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(_)) => SEMANTIC_TOKEN_TYPE_VARIABLE,
            ResolutionResult::Concrete(ConcreteType::Platform(_)) => SEMANTIC_TOKEN_TYPE_CLASS,
            ResolutionResult::Union(_) => SEMANTIC_TOKEN_TYPE_TYPE,
            _ => SEMANTIC_TOKEN_TYPE_VARIABLE,
        }
    }
    
    /// Получить модификаторы для переменной
    fn get_variable_modifiers(&self, var_type: &TypeResolution) -> u32 {
        let mut modifiers = 0;
        
        // Отмечаем неопределенные типы
        if matches!(var_type.certainty, Certainty::Unknown) {
            modifiers |= 1 << SEMANTIC_TOKEN_MODIFIER_DEPRECATED;
        }
        
        // Отмечаем Union типы
        if matches!(var_type.result, ResolutionResult::Union(_)) {
            modifiers |= 1 << SEMANTIC_TOKEN_MODIFIER_ABSTRACT;
        }
        
        modifiers
    }
    
    /// Кодировать токены в формат LSP
    #[allow(dead_code)]
    fn encode_tokens(&self, tokens: Vec<SemanticToken>) -> Vec<u32> {
        let mut encoded = Vec::new();
        
        for token in tokens {
            encoded.push(token.delta_line);
            encoded.push(token.delta_start);
            encoded.push(token.length);
            encoded.push(token.token_type);
            encoded.push(token.token_modifiers_bitset);
        }
        
        encoded
    }
}

/// Semantic token constants (упрощенная версия)
const SEMANTIC_TOKEN_TYPE_VARIABLE: u32 = 0;
const SEMANTIC_TOKEN_TYPE_FUNCTION: u32 = 1;
const SEMANTIC_TOKEN_TYPE_CLASS: u32 = 2;
const SEMANTIC_TOKEN_TYPE_TYPE: u32 = 3;

const SEMANTIC_TOKEN_MODIFIER_STATIC: u32 = 0;
const SEMANTIC_TOKEN_MODIFIER_DEPRECATED: u32 = 1;
const SEMANTIC_TOKEN_MODIFIER_ABSTRACT: u32 = 2;

/// Semantic token для internal использования
#[derive(Debug, Clone)]
pub struct SemanticToken {
    pub delta_line: u32,
    pub delta_start: u32,
    pub length: u32,
    pub token_type: u32,
    pub token_modifiers_bitset: u32,
}

/// Интеграция type hints в LSP сервер
pub struct TypeHintsIntegration {
    provider: TypeHintsProvider,
    semantic_provider: SemanticTokensProvider,
}

impl Default for TypeHintsIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeHintsIntegration {
    /// Создать новую интеграцию
    pub fn new() -> Self {
        Self {
            provider: TypeHintsProvider::new(TypeHintsSettings::default()),
            semantic_provider: SemanticTokensProvider::new(SemanticHighlightingSettings::default()),
        }
    }
    
    /// Создать с настройками
    pub fn with_settings(
        hints_settings: TypeHintsSettings,
        semantic_settings: SemanticHighlightingSettings,
    ) -> Self {
        Self {
            provider: TypeHintsProvider::new(hints_settings),
            semantic_provider: SemanticTokensProvider::new(semantic_settings),
        }
    }
    
    /// Обработать запрос inlay hints
    pub fn handle_inlay_hints_request(
        &self,
        params: InlayHintParams,
        program: &Program,
        type_context: &TypeContext,
    ) -> Vec<InlayHint> {
        self.provider.get_inlay_hints(program, type_context, params.range)
    }
    
    /// Обработать запрос semantic tokens
    pub fn handle_semantic_tokens_request(
        &self,
        program: &Program,
        type_context: &TypeContext,
    ) -> SemanticTokens {
        self.semantic_provider.get_semantic_tokens(program, type_context)
    }
    
    /// Обновить настройки
    pub fn update_settings(
        &mut self,
        hints_settings: Option<TypeHintsSettings>,
        semantic_settings: Option<SemanticHighlightingSettings>,
    ) {
        if let Some(settings) = hints_settings {
            self.provider.settings = settings;
        }
        
        if let Some(settings) = semantic_settings {
            self.semantic_provider.settings = settings;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    fn create_test_context() -> TypeContext {
        let mut variables = HashMap::new();
        variables.insert("stringVar".to_string(), crate::core::standard_types::primitive_type(
            crate::core::types::PrimitiveType::String
        ));
        variables.insert("numberVar".to_string(), crate::core::standard_types::primitive_type(
            crate::core::types::PrimitiveType::Number
        ));
        
        let mut functions = HashMap::new();
        functions.insert("TestFunc".to_string(), crate::core::type_checker::FunctionSignature {
            params: vec![("param1".to_string(), crate::core::standard_types::primitive_type(
                crate::core::types::PrimitiveType::String
            ))],
            return_type: crate::core::standard_types::primitive_type(
                crate::core::types::PrimitiveType::Number
            ),
            exported: true,
        });
        
        TypeContext {
            variables,
            functions,
            current_scope: crate::core::dependency_graph::Scope::Global,
            scope_stack: vec![],
        }
    }
    
    #[test]
    fn test_type_hints_provider() {
        let provider = TypeHintsProvider::new(TypeHintsSettings::default());
        let context = create_test_context();
        
        // Создаем простую программу для тестирования
        let program = Program {
            statements: vec![
                Statement::VarDeclaration {
                    name: "stringVar".to_string(),
                    export: false,
                    value: None,
                }
            ],
        };
        
        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 10, character: 0 },
        };
        
        let hints = provider.get_inlay_hints(&program, &context, range);
        
        // Должен быть hint для переменной
        assert!(!hints.is_empty());
        
        let hint = &hints[0];
        assert_eq!(hint.kind, Some(InlayHintKind::TYPE));
        // Проверяем содержимое hint label
        match &hint.label {
            InlayHintLabel::String(text) => assert!(text.contains("Строка")),
            _ => panic!("Expected string label"),
        }
    }
    
    #[test]
    fn test_format_type_hint() {
        let provider = TypeHintsProvider::new(TypeHintsSettings::default());
        
        let string_type = crate::core::standard_types::primitive_type(
            crate::core::types::PrimitiveType::String
        );
        
        let hint_text = provider.format_type_hint(&string_type);
        assert_eq!(hint_text, "Строка");
        
        // Тест с Union типом
        let union_type = crate::core::union_types::UnionTypeManager::from_concrete_types(vec![
            ConcreteType::Primitive(crate::core::types::PrimitiveType::String),
            ConcreteType::Primitive(crate::core::types::PrimitiveType::Number),
        ]);
        
        let union_hint = provider.format_type_hint(&union_type);
        assert!(union_hint.contains("Str") || union_hint.contains("Union"));
    }
    
    #[test]
    fn test_semantic_tokens_provider() {
        let provider = SemanticTokensProvider::new(SemanticHighlightingSettings::default());
        let context = create_test_context();
        
        let program = Program {
            statements: vec![
                Statement::VarDeclaration {
                    name: "stringVar".to_string(), // Используем переменную из контекста
                    export: false,
                    value: None,
                }
            ],
        };
        
        let tokens = provider.get_semantic_tokens(&program, &context);
        
        // Должны быть semantic tokens
        assert!(!tokens.data.is_empty());
        assert_eq!(tokens.data.len() % 5, 0); // Должно быть кратно 5 (формат LSP)
    }
    
    #[test]
    fn test_type_hints_integration() {
        let integration = TypeHintsIntegration::new();
        let context = create_test_context();
        
        let program = Program {
            statements: vec![
                Statement::Assignment {
                    target: Expression::Identifier("stringVar".to_string()),
                    value: Expression::String("test".to_string()),
                }
            ],
        };
        
        let params = InlayHintParams {
            text_document: TextDocumentIdentifier {
                uri: Url::parse("test://test.bsl").unwrap(),
            },
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 10, character: 0 },
            },
            work_done_progress_params: Default::default(),
        };
        
        let hints = integration.handle_inlay_hints_request(params, &program, &context);
        
        // Должны получить type hints
        assert!(!hints.is_empty());
    }
}