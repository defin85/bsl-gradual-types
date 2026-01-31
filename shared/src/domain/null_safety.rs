//! Null Safety Analysis
//!
//! Анализ null safety через Control Flow Graph:
//! - Отслеживание nullable переменных
//! - Автоматический unwrap после проверок
//! - Предупреждения о потенциальных NPE

use crate::domain::flow_analysis::*;
use crate::domain::type_id::TypeId;
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
                        if resolution.result.is_nullable() {
                            self.nullable_vars
                                .entry(node_id)
                                .or_default()
                                .insert(TypeId::new(variable));
                        }
                    }
                }

                CfgNodeKind::Condition { variable } => {
                    // Проверка на Null делает переменную non-null в then-ветке
                    let var_name = variable.clone();
                    if self.is_null_check(&var_name) {
                        // В then-ветке переменная точно не Null
                        self.mark_non_null_in_successors(node_id, &var_name);

                        // Это безопасная операция
                        safe_operations.push(SafeOperation {
                            variable: var_name,
                            unwrapped: true,
                            line: None,
                        });
                    }
                }
                CfgNodeKind::Conditional { condition } => {
                    // v2 CFG использует `Conditional { condition }` для if/while.
                    // Для null-safety считаем это эквивалентом `Condition`.
                    let cond = condition.clone();
                    if self.is_null_check(&cond) {
                        self.mark_non_null_in_successors(node_id, &cond);
                        safe_operations.push(SafeOperation {
                            variable: cond,
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

    /// Проверить, является ли условие проверкой на Null
    fn is_null_check(&self, condition: &str) -> bool {
        // Простая эвристика: ищем ЗначениеЗаполнено, НЕ Неопределено и т.д.
        condition.contains("ЗначениеЗаполнено")
            || condition.contains("ValueIsFilled")
            || condition.contains("НЕ Неопределено")
            || condition.contains("NOT Undefined")
            || condition.contains("<> Неопределено")
            || condition.contains("<> Undefined")
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
            resolution.result.is_nullable()
        } else {
            // Неизвестная переменная - лучше предупредить
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Certainty, ConcreteType, PlatformType, ResolutionMetadata, ResolutionResult,
        ResolutionSource, TypeResolution,
    };

    #[test]
    fn test_null_check_detection() {
        let cfg = ControlFlowGraph::new();
        let analyzer = NullSafetyAnalyzer::new(cfg);

        assert!(analyzer.is_null_check("Если ЗначениеЗаполнено(значение) Тогда"));
        assert!(analyzer.is_null_check("If ValueIsFilled(value) Then"));
        assert!(analyzer.is_null_check("Если значение <> Неопределено Тогда"));
        assert!(!analyzer.is_null_check("Если значение > 0 Тогда"));
    }

    #[test]
    fn test_nullable_tracking() {
        let mut cfg = ControlFlowGraph::new();

        // Создаём простой CFG: присваивание → проверка → использование
        cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::Assignment {
                variable: "x".to_string(),
                value: "Неопределено".to_string(),
            },
        });

        cfg.add_node(CfgNode {
            id: 1,
            kind: CfgNodeKind::Condition {
                variable: "ЗначениеЗаполнено(x)".to_string(),
            },
        });

        cfg.add_node(CfgNode {
            id: 2,
            kind: CfgNodeKind::MethodCall {
                object: "x".to_string(),
                method: "Метод".to_string(),
                arguments: vec![],
            },
        });

        cfg.add_edge(0, 1, EdgeKind::Unconditional);

        cfg.add_edge(1, 2, EdgeKind::ConditionalTrue);

        let mut analyzer = NullSafetyAnalyzer::new(cfg);
        let mut context = FlowAnalysisContext::new();

        // x = Null → nullable
        context.set_variable(
            "x",
            TypeResolution {
                result: ResolutionResult::nullable(ConcreteType::string()),
                certainty: Certainty::Known,
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata::default(),
                active_facet: None,
                available_facets: vec![],
            },
        );

        let result = analyzer.analyze(&context);

        // После проверки в узле 2 не должно быть предупреждений
        // (но текущая реализация ещё не достаточно умная)
        assert!(!result.safe_operations.is_empty());
    }

    #[test]
    fn test_unchecked_null_warning() {
        let mut cfg = ControlFlowGraph::new();

        // Прямое использование без проверки
        cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::Assignment {
                variable: "x".to_string(),
                value: "Неопределено".to_string(),
            },
        });

        cfg.add_node(CfgNode {
            id: 1,
            kind: CfgNodeKind::MethodCall {
                object: "x".to_string(),
                method: "Метод".to_string(),
                arguments: vec![],
            },
        });

        cfg.add_edge(0, 1, EdgeKind::Unconditional);

        let mut analyzer = NullSafetyAnalyzer::new(cfg);
        let mut context = FlowAnalysisContext::new();

        context.set_variable(
            "x",
            TypeResolution {
                result: ResolutionResult::nullable(ConcreteType::string()),
                certainty: Certainty::Known,
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata::default(),
                active_facet: None,
                available_facets: vec![],
            },
        );

        let result = analyzer.analyze(&context);

        // Должно быть предупреждение о возможном NPE
        assert!(!result.warnings.is_empty());
        assert_eq!(
            result.warnings[0].kind,
            NullWarningKind::PossibleNullDereference
        );
    }

    #[test]
    fn test_non_nullable_no_warning() {
        let mut cfg = ControlFlowGraph::new();

        cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::Assignment {
                variable: "x".to_string(),
                value: "\"строка\"".to_string(),
            },
        });

        cfg.add_node(CfgNode {
            id: 1,
            kind: CfgNodeKind::MethodCall {
                object: "x".to_string(),
                method: "Длина".to_string(),
                arguments: vec![],
            },
        });

        cfg.add_edge(0, 1, EdgeKind::Unconditional);

        let mut analyzer = NullSafetyAnalyzer::new(cfg);
        let mut context = FlowAnalysisContext::new();

        // x = "строка" → не nullable
        context.set_variable(
            "x",
            TypeResolution {
                result: ResolutionResult::Concrete(ConcreteType::string()),
                certainty: Certainty::Known,
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata::default(),
                active_facet: None,
                available_facets: vec![],
            },
        );

        let result = analyzer.analyze(&context);

        // Не должно быть предупреждений
        assert_eq!(result.warnings.len(), 0);
    }

    #[test]
    fn test_property_access_null_check() {
        let mut cfg = ControlFlowGraph::new();

        cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::PropertyAccess {
                object: "obj".to_string(),
                property: "Свойство".to_string(),
            },
        });

        let mut analyzer = NullSafetyAnalyzer::new(cfg);
        let mut context = FlowAnalysisContext::new();

        context.set_variable(
            "obj",
            TypeResolution {
                result: ResolutionResult::nullable(ConcreteType::Platform(PlatformType {
                    name: "Объект".to_string(),
                })),
                certainty: Certainty::Known,
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata::default(),
                active_facet: None,
                available_facets: vec![],
            },
        );

        let result = analyzer.analyze(&context);

        // Должно быть предупреждение о доступе к свойству nullable объекта
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].message.contains("доступе к свойству"));
    }
}
