//! Simplified Flow Analyzer for current BSL AST
//!
//! Базовая реализация flow-sensitive анализа для текущей структуры AST

#![allow(deprecated)]

use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::{FlowAnalysisContext, TypeResolver};
use std::collections::HashMap;
use std::sync::Arc;

/// Упрощённый flow analyzer для работы с текущим AST
#[deprecated(
    note = "Устаревший экспериментальный анализатор. Используйте v2 pipeline (bsl-analysis-v2) + SemanticProgram.cfg."
)]
pub struct SimpleFlowAnalyzer {
    resolver: Arc<TypeResolver>,
}

impl SimpleFlowAnalyzer {
    pub fn new(resolver: Arc<TypeResolver>) -> Self {
        Self { resolver }
    }

    /// Анализировать код BSL и вернуть типы переменных
    pub fn analyze_code(&self, code: &str) -> FlowAnalysisResult {
        let mut context = FlowAnalysisContext::new();

        // Простой анализ на основе паттернов в тексте
        for line in code.lines() {
            let line = line.trim();

            // Паттерн присваивания: Перем x = значение
            if let Some((var_name, value)) = self.parse_assignment(line) {
                let var_type = self.infer_type_from_value(&value);
                context.set_variable(var_name, var_type);
            }

            // Паттерн условия: Если ... Тогда
            if line.starts_with("Если") || line.starts_with("If") {
                context.enter_scope();
            }

            // Конец условия: КонецЕсли
            if line.starts_with("КонецЕсли") || line.starts_with("EndIf") {
                context.exit_scope();
            }
        }

        FlowAnalysisResult {
            variables: context.get_all_variables().clone(),
            context,
        }
    }

    /// Парсинг простого присваивания
    fn parse_assignment(&self, line: &str) -> Option<(String, String)> {
        // Перем x = значение
        if (line.starts_with("Перем") || line.starts_with("Var")) && line.find('=').is_some() {
            // Используем split_whitespace для корректной работы с кириллицей
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 && parts[0] == "Перем" || parts[0] == "Var" {
                let var_name = parts[1].trim();
                // Всё после '=' - это значение
                let value_start = line.find('=').map(|pos| pos + 1)?;
                let value_part = &line[value_start..].trim();
                let value = value_part.trim_end_matches(';').trim();

                return Some((var_name.to_string(), value.to_string()));
            }
        }

        // x = значение (без Перем)
        if let Some(eq_pos) = line.find('=') {
            let var_part = line[..eq_pos].trim();
            let value_part = &line[eq_pos + 1..].trim();

            if !var_part.is_empty() && !value_part.is_empty() {
                let value = value_part.trim_end_matches(';').trim();
                return Some((var_part.to_string(), value.to_string()));
            }
        }

        None
    }

    /// Вывод типа из значения
    fn infer_type_from_value(&self, value: &str) -> TypeResolution {
        use bsl_shared::domain::types::{ConcreteType, PlatformType};

        // Числовой литерал
        if value.parse::<f64>().is_ok() {
            return TypeResolution::known(ConcreteType::Platform(PlatformType {
                name: "Число".to_string(),
            }));
        }

        // Строковый литерал
        if value.starts_with('"') && value.ends_with('"') {
            return TypeResolution::known(ConcreteType::Platform(PlatformType {
                name: "Строка".to_string(),
            }));
        }

        // Булев литерал
        if value == "Истина" || value == "Ложь" || value == "True" || value == "False" {
            return TypeResolution::known(ConcreteType::Platform(PlatformType {
                name: "Булево".to_string(),
            }));
        }

        // Вызов конструктора: Новый Массив()
        if value.starts_with("Новый") || value.starts_with("New") {
            let type_name = value
                .split_whitespace()
                .nth(1)
                .unwrap_or("Произвольный")
                .trim_end_matches("()")
                .trim_end_matches(';');

            return TypeResolution::known(ConcreteType::Platform(PlatformType {
                name: type_name.to_string(),
            }));
        }

        // Попытка разрешить через TypeResolver
        self.resolver.resolve_expression_sync(value)
    }
}

/// Результат упрощённого flow-анализа
#[derive(Debug, Clone)]
#[deprecated(
    note = "Устаревший экспериментальный результат. Используйте v2 pipeline (bsl-analysis-v2) + SemanticProgram.cfg."
)]
pub struct FlowAnalysisResult {
    /// Типы переменных (ключ: TypeId для регистронезависимого поиска)
    pub variables: HashMap<TypeId, TypeResolution>,

    /// Контекст flow-анализа
    pub context: FlowAnalysisContext,
}

#[cfg(test)]
#[path = "flow_analyzer_simple/tests.rs"]
mod tests;
