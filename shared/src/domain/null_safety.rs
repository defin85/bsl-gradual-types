//! Null Safety Analysis
//!
//! Анализ null safety через Control Flow Graph:
//! - Отслеживание nullable переменных
//! - Автоматический unwrap после проверок
//! - Предупреждения о потенциальных NPE

use crate::domain::flow_analysis::*;
use crate::domain::type_id::TypeId;
use crate::domain::types::{
    ConcreteType, PlatformType, ResolutionResult, SpecialType, TypeResolution,
};
use crate::ir::Span;
use std::collections::{HashMap, HashSet};

/// Null safety analyzer
pub struct NullSafetyAnalyzer {
    /// CFG для анализа потока управления
    cfg: ControlFlowGraph,
    /// Nullable переменные в каждом узле CFG
    nullable_vars: HashMap<usize, HashSet<TypeId>>,
    /// Non-null переменные после проверок
    non_null_vars: HashMap<usize, HashSet<TypeId>>,
}

/// Результат null safety анализа
#[derive(Debug, Clone, PartialEq)]
pub struct NullSafetyResult {
    pub warnings: Vec<NullSafetyWarning>,
    pub safe_operations: Vec<SafeOperation>,
}

/// Предупреждение о потенциальной проблеме с null
#[derive(Debug, Clone, PartialEq)]
pub struct NullSafetyWarning {
    pub kind: NullWarningKind,
    pub variable: String,
    pub line: Option<u32>,
    pub message: String,
    /// CFG node id, где была обнаружена проблема.
    pub node_id: usize,
    /// Span исходного кода (если доступен из CFG).
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullWarningKind {
    PossibleNullDereference, // Возможное обращение к Null
    UncheckedNull,           // Не проверено на Null перед использованием
    AlwaysNull,              // Переменная всегда Null
}

/// Безопасная операция после проверки
#[derive(Debug, Clone, PartialEq)]
pub struct SafeOperation {
    pub variable: String,
    pub unwrapped: bool,
    pub line: Option<u32>,
}

impl NullSafetyAnalyzer {
    pub fn new(cfg: ControlFlowGraph) -> Self {
        Self {
            cfg,
            nullable_vars: HashMap::new(),
            non_null_vars: HashMap::new(),
        }
    }

    /// Выполнить null safety анализ
    pub fn analyze(&mut self, context: &FlowAnalysisContext) -> NullSafetyResult {
        let mut warnings = Vec::new();
        let mut safe_operations = Vec::new();

        // Проходим по всем узлам CFG
        for node_id in 0..self.cfg.nodes().len() {
            let node = &self.cfg.nodes()[node_id];

            match &node.kind {
                CfgNodeKind::Assignment { variable, .. } => {
                    // Проверяем, nullable ли присваиваемое значение
                    if let Some(resolution) = context.get_variable(variable.as_str()) {
                        if self.is_nullish_resolution(resolution) {
                            self.nullable_vars
                                .entry(node_id)
                                .or_default()
                                .insert(TypeId::new(variable));
                        }
                    }
                }

                CfgNodeKind::Condition { variable } => {
                    if let Some(var_name) = self.extract_null_checked_variable(variable.as_str()) {
                        self.mark_non_null_in_successors(node_id, &var_name);
                        safe_operations.push(SafeOperation {
                            variable: var_name,
                            unwrapped: true,
                            line: None,
                        });
                    }
                }

                CfgNodeKind::Conditional { condition } | CfgNodeKind::LoopHeader { condition } => {
                    // v2 CFG использует:
                    // - `Conditional { condition }` для if,
                    // - `LoopHeader { condition }` для while/for/foreach.
                    //
                    // Для null-safety считаем оба варианта эквивалентом `Condition`.
                    if let Some(var_name) = self.extract_null_checked_variable(condition.as_str()) {
                        self.mark_non_null_in_successors(node_id, &var_name);
                        safe_operations.push(SafeOperation {
                            variable: var_name,
                            unwrapped: true,
                            line: None,
                        });
                    }
                }

                CfgNodeKind::MethodCall { object, .. } => {
                    // Проверяем, может ли object быть Null
                    if self.is_possibly_null(node_id, object.as_str(), context) {
                        warnings.push(NullSafetyWarning {
                            kind: NullWarningKind::PossibleNullDereference,
                            variable: object.clone(),
                            line: None,
                            message: format!(
                                "Переменная '{}' может быть Null при вызове метода",
                                object
                            ),
                            node_id,
                            span: self.cfg.node_span(node_id),
                        });
                    }
                }

                CfgNodeKind::PropertyAccess { object, .. } => {
                    // Проверяем, может ли object быть Null
                    if self.is_possibly_null(node_id, object.as_str(), context) {
                        warnings.push(NullSafetyWarning {
                            kind: NullWarningKind::PossibleNullDereference,
                            variable: object.clone(),
                            line: None,
                            message: format!(
                                "Переменная '{}' может быть Null при доступе к свойству",
                                object
                            ),
                            node_id,
                            span: self.cfg.node_span(node_id),
                        });
                    }
                }

                _ => {}
            }
        }

        NullSafetyResult {
            warnings,
            safe_operations,
        }
    }

    /// Проверить, является ли переменная nullable в данном узле
    pub fn is_nullable_at(&self, node_id: usize, variable: &str) -> bool {
        let type_id = TypeId::new(variable);
        if let Some(non_null) = self.non_null_vars.get(&node_id) {
            if non_null.contains(&type_id) {
                return false; // Явно проверено на non-null
            }
        }

        if let Some(nullable) = self.nullable_vars.get(&node_id) {
            nullable.contains(&type_id)
        } else {
            false
        }
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    fn is_nullish_resolution(&self, resolution: &TypeResolution) -> bool {
        if resolution.result.is_nullable() {
            return true;
        }
        matches!(
            &resolution.result,
            ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Null))
                | ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Undefined))
        ) || matches!(&resolution.result, ResolutionResult::Concrete(ConcreteType::Platform(PlatformType { name })) if {
            let lower = name.to_lowercase();
            lower == "null" || lower == "неопределено" || lower == "undefined"
        })
    }

    /// Проверить, является ли условие проверкой на Null/Неопределено.
    #[cfg(test)]
    fn is_null_check(&self, condition: &str) -> bool {
        self.extract_null_checked_variable(condition).is_some()
    }

    fn extract_null_checked_variable(&self, condition: &str) -> Option<String> {
        let cond = condition.trim();

        fn extract_inside_parens(haystack: &str, needle: &str) -> Option<String> {
            let pos = haystack.find(needle)?;
            let after = &haystack[pos + needle.len()..];
            let open = after.find('(')?;
            let after_open = &after[open + 1..];
            let close = after_open.find(')')?;
            let inner = after_open[..close].trim();
            let var = inner.split(',').next().unwrap_or(inner).trim();
            (!var.is_empty()).then(|| var.to_string())
        }

        if let Some(v) = extract_inside_parens(cond, "ЗначениеЗаполнено") {
            return Some(v);
        }
        if let Some(v) = extract_inside_parens(cond, "ValueIsFilled") {
            return Some(v);
        }

        fn split_cmp<'a>(cond: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
            let (l, r) = cond.split_once(op)?;
            Some((l.trim(), r.trim()))
        }

        let (left, right) = split_cmp(cond, "<>")
            .or_else(|| split_cmp(cond, "!="))
            .unwrap_or(("", ""));

        if !left.is_empty() {
            let rhs = right.to_lowercase();
            let lhs = left.to_lowercase();

            let is_special = |s: &str| {
                s.contains("неопределено") || s.contains("undefined") || s.contains("null")
            };

            if is_special(&rhs) && !is_special(&lhs) {
                return left
                    .split_whitespace()
                    .last()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
            }

            if is_special(&lhs) && !is_special(&rhs) {
                return right
                    .split_whitespace()
                    .last()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
            }
        }

        None
    }

    /// Пометить переменную как non-null в последующих узлах
    fn mark_non_null_in_successors(&mut self, node_id: usize, variable: &str) {
        // Находим then-ветку (обычно первый successor)
        if let Some(edge) = self
            .cfg
            .edges()
            .iter()
            .find(|e| e.from == node_id && e.kind == EdgeKind::ConditionalTrue)
        {
            self.non_null_vars
                .entry(edge.to)
                .or_default()
                .insert(TypeId::new(variable));

            // Рекурсивно для всех последующих узлов в then-ветке
            self.propagate_non_null(edge.to, variable);
        }
    }

    /// Распространить информацию о non-null дальше по графу
    fn propagate_non_null(&mut self, node_id: usize, variable: &str) {
        let successors: Vec<_> = self
            .cfg
            .edges()
            .iter()
            .filter(|e| e.from == node_id && e.kind != EdgeKind::ConditionalFalse)
            .map(|e| e.to)
            .collect();

        for successor in successors {
            // Не распространяем, если переменная переприсваивается
            let node = &self.cfg.nodes()[successor];
            if let CfgNodeKind::Assignment {
                variable: assigned, ..
            } = &node.kind
            {
                if assigned == variable {
                    continue;
                }
            }

            self.non_null_vars
                .entry(successor)
                .or_default()
                .insert(TypeId::new(variable));

            // Рекурсивно
            self.propagate_non_null(successor, variable);
        }
    }

    /// Проверить, может ли переменная быть Null в данном узле
    fn is_possibly_null(
        &self,
        node_id: usize,
        variable: &str,
        context: &FlowAnalysisContext,
    ) -> bool {
        // Если явно помечена как non-null, то безопасно
        let type_id = TypeId::new(variable);
        if let Some(non_null) = self.non_null_vars.get(&node_id) {
            if non_null.contains(&type_id) {
                return false;
            }
        }

        // Проверяем тип переменной
        if let Some(resolution) = context.get_variable(variable) {
            self.is_nullish_resolution(resolution)
        } else {
            // Неизвестная переменная — не предупреждаем: слишком шумно для IDE.
            false
        }
    }
}

#[cfg(test)]
#[path = "null_safety/tests.rs"]
mod tests;
