//! Simplified Flow Analyzer for current BSL AST
//!
//! Базовая реализация flow-sensitive анализа для текущей структуры AST

use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::{FlowAnalysisContext, TypeResolver};
use std::collections::HashMap;
use std::sync::Arc;

/// Упрощённый flow analyzer для работы с текущим AST
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
        if line.starts_with("Перем") || line.starts_with("Var") {
            if line.find('=').is_some() {
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
pub struct FlowAnalysisResult {
    /// Типы переменных
    pub variables: HashMap<String, TypeResolution>,

    /// Контекст flow-анализа
    pub context: FlowAnalysisContext,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::types::{ConcreteType, ResolutionResult};

    #[test]
    fn test_simple_variable_assignment() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repo));
        let analyzer = SimpleFlowAnalyzer::new(resolver);

        let code = r#"
Перем x = 42;
Перем y = "текст";
        "#;

        let result = analyzer.analyze_code(code);

        assert!(result.variables.contains_key("x"));
        assert!(result.variables.contains_key("y"));

        // Проверяем тип x (Число)
        if let Some(x_type) = result.variables.get("x") {
            if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &x_type.result {
                assert_eq!(pt.name, "Число");
            }
        }

        // Проверяем тип y (Строка)
        if let Some(y_type) = result.variables.get("y") {
            if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &y_type.result {
                assert_eq!(pt.name, "Строка");
            }
        }
    }

    #[test]
    fn test_constructor_call() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repo));
        let analyzer = SimpleFlowAnalyzer::new(resolver);

        let code = r#"
Перем массив = Новый Массив();
        "#;

        let result = analyzer.analyze_code(code);

        assert!(result.variables.contains_key("массив"));

        if let Some(arr_type) = result.variables.get("массив") {
            if let ResolutionResult::Concrete(ConcreteType::Platform(pt)) = &arr_type.result {
                assert_eq!(pt.name, "Массив");
            }
        }
    }

    #[test]
    fn test_scope_tracking() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let resolver = Arc::new(TypeResolver::new(repo));
        let analyzer = SimpleFlowAnalyzer::new(resolver);

        let code = r#"
Перем x = 1;
Если x > 0 Тогда
    Перем y = 2;
КонецЕсли;
        "#;

        let result = analyzer.analyze_code(code);

        // Проверяем, что scope depth корректно отслеживается
        assert_eq!(result.context.get_scope_depth(), 0);
    }
}
