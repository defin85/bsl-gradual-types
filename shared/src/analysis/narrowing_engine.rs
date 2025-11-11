//! Narrowing Engine
//!
//! Движок для сужения типов на основе control-flow анализа и type guards.
//!
//! Milestone 3.7: Advanced Type Narrowing

use crate::analysis::type_guards::{detect_type_guards, TypeGuard};
use crate::domain::flow_analysis::{ControlFlowGraph, FlowAnalysisContext};
use crate::domain::types::TypeResolution;
use std::collections::HashMap;

/// Контекст сужения типов для конкретного блока CFG
#[derive(Debug, Clone)]
pub struct NarrowingContext {
    /// Узауженные типы переменных в текущем блоке
    pub narrowed_types: HashMap<String, TypeResolution>,

    /// Type guards, активные в текущем блоке
    pub active_guards: Vec<TypeGuard>,

    /// Родительский контекст (для вложенных блоков)
    pub parent: Option<Box<NarrowingContext>>,
}

impl NarrowingContext {
    /// Создать новый пустой контекст
    pub fn new() -> Self {
        Self {
            narrowed_types: HashMap::new(),
            active_guards: Vec::new(),
            parent: None,
        }
    }

    /// Создать дочерний контекст (для вложенного блока)
    pub fn child(&self) -> Self {
        Self {
            narrowed_types: HashMap::new(),
            active_guards: Vec::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    /// Получить тип переменной с учётом сужения
    pub fn get_type(&self, variable: &str) -> Option<&TypeResolution> {
        // Сначала ищем в текущем контексте
        if let Some(ty) = self.narrowed_types.get(variable) {
            return Some(ty);
        }

        // Затем в родительском
        if let Some(parent) = &self.parent {
            return parent.get_type(variable);
        }

        None
    }

    /// Установить узауженный тип переменной
    pub fn set_type(&mut self, variable: String, resolution: TypeResolution) {
        self.narrowed_types.insert(variable, resolution);
    }

    /// Применить type guard к переменной
    pub fn apply_guard(&mut self, guard: TypeGuard, current_type: &TypeResolution) {
        let variable = guard.variable_name().to_string();
        let narrowed = guard.apply_narrowing(current_type);

        self.narrowed_types.insert(variable, narrowed);
        self.active_guards.push(guard);
    }

    /// Получить все активные guards
    pub fn get_active_guards(&self) -> &[TypeGuard] {
        &self.active_guards
    }

    /// Объединить с другим контекстом (для merge после if-then-else)
    pub fn merge(&mut self, other: &NarrowingContext, flow_ctx: &mut FlowAnalysisContext) {
        // Для каждой переменной из обоих контекстов
        let mut all_vars: Vec<String> = self
            .narrowed_types
            .keys()
            .chain(other.narrowed_types.keys())
            .cloned()
            .collect();
        all_vars.sort();
        all_vars.dedup();

        for var in all_vars {
            match (
                self.narrowed_types.get(&var),
                other.narrowed_types.get(&var),
            ) {
                (Some(self_type), Some(other_type)) => {
                    // Переменная есть в обоих контекстах — создаём union
                    let mut self_ctx = FlowAnalysisContext::new();
                    let mut other_ctx = FlowAnalysisContext::new();

                    self_ctx.set_variable(var.clone(), self_type.clone());
                    other_ctx.set_variable(var.clone(), other_type.clone());

                    self_ctx.merge(&other_ctx);

                    if let Some(merged) = self_ctx.get_variable(&var) {
                        self.narrowed_types.insert(var.clone(), merged.clone());
                        flow_ctx.set_variable(var, merged.clone());
                    }
                }
                (Some(self_type), None) => {
                    // Только в self — оставляем как есть
                    flow_ctx.set_variable(var, self_type.clone());
                }
                (None, Some(other_type)) => {
                    // Только в other — добавляем
                    self.narrowed_types.insert(var.clone(), other_type.clone());
                    flow_ctx.set_variable(var, other_type.clone());
                }
                (None, None) => unreachable!(),
            }
        }
    }
}

impl Default for NarrowingContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Движок сужения типов
pub struct NarrowingEngine {
    /// CFG для анализа
    cfg: ControlFlowGraph,

    /// Контексты сужения для каждого узла CFG
    contexts: HashMap<usize, NarrowingContext>,
}

impl NarrowingEngine {
    /// Создать новый движок с заданным CFG
    pub fn new(cfg: ControlFlowGraph) -> Self {
        Self {
            cfg,
            contexts: HashMap::new(),
        }
    }

    /// Применить сужение типов к переменной на основе условия
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// let engine = NarrowingEngine::new(cfg);
    /// let narrowed = engine.narrow_type(
    ///     &current_type,
    ///     "ТипЗнч(Параметр) = Тип(\"Число\")"
    /// );
    /// // narrowed теперь имеет тип Число
    /// ```
    pub fn narrow_type(&mut self, current: &TypeResolution, condition: &str) -> TypeResolution {
        // Обнаруживаем type guards в условии
        let guards = detect_type_guards(condition);

        if guards.is_empty() {
            // Нет type guards — возвращаем исходный тип
            return current.clone();
        }

        // Применяем первый найденный guard (в будущем можно комбинировать)
        if let Some(guard) = guards.first() {
            guard.apply_narrowing(current)
        } else {
            current.clone()
        }
    }

    /// Построить контексты сужения для всех узлов CFG
    ///
    /// Проходит по CFG и создаёт контексты сужения для каждого узла,
    /// распространяя информацию о типах через control flow.
    pub fn build_narrowing_contexts(&mut self, initial_context: FlowAnalysisContext) {
        use crate::domain::flow_analysis::{CfgNodeKind, EdgeKind};

        // Инициализируем контекст для entry узла
        if let Some(entry_node) = self.cfg.nodes().first() {
            let mut ctx = NarrowingContext::new();

            // Копируем начальные типы переменных из FlowAnalysisContext
            for (var, resolution) in initial_context.get_all_variables() {
                ctx.set_type(var.clone(), resolution.clone());
            }

            self.contexts.insert(entry_node.id, ctx);
        }

        // Проходим по всем узлам в топологическом порядке
        for node in self.cfg.nodes() {
            let node_ctx = self.contexts.get(&node.id).cloned().unwrap_or_default();

            match &node.kind {
                CfgNodeKind::Conditional { condition } => {
                    // Обнаруживаем type guards в условии
                    let guards = detect_type_guards(condition);

                    // Создаём контексты для true и false веток
                    for edge in self.cfg.edges() {
                        if edge.from == node.id {
                            let mut branch_ctx = node_ctx.child();

                            match edge.kind {
                                EdgeKind::ConditionalTrue => {
                                    // В true ветке применяем guards
                                    for guard in &guards {
                                        if let Some(current_type) =
                                            node_ctx.get_type(guard.variable_name())
                                        {
                                            branch_ctx.apply_guard(guard.clone(), current_type);
                                        }
                                    }
                                }
                                EdgeKind::ConditionalFalse => {
                                    // В false ветке можно применить инверсные guards
                                    // (пока не реализовано)
                                }
                                _ => {}
                            }

                            self.contexts.insert(edge.to, branch_ctx);
                        }
                    }
                }

                CfgNodeKind::Assignment { .. } => {
                    // При присваивании обновляем тип переменной
                    // (требует интеграции с Type Resolver)
                    let new_ctx = node_ctx.clone();

                    // Передаём контекст дальше
                    for edge in self.cfg.edges() {
                        if edge.from == node.id {
                            self.contexts.insert(edge.to, new_ctx.clone());
                        }
                    }
                }

                _ => {
                    // Для остальных узлов просто передаём контекст
                    for edge in self.cfg.edges() {
                        if edge.from == node.id {
                            self.contexts.insert(edge.to, node_ctx.clone());
                        }
                    }
                }
            }
        }
    }

    /// Получить контекст сужения для узла CFG
    pub fn get_context(&self, node_id: usize) -> Option<&NarrowingContext> {
        self.contexts.get(&node_id)
    }

    /// Получить CFG
    pub fn cfg(&self) -> &ControlFlowGraph {
        &self.cfg
    }

    /// Получить все контексты
    pub fn contexts(&self) -> &HashMap<usize, NarrowingContext> {
        &self.contexts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::flow_analysis::{CfgNode, CfgNodeKind, EdgeKind};
    use crate::domain::types::{ConcreteType, PlatformType, TypeResolution};

    #[test]
    fn test_narrowing_context_new() {
        let ctx = NarrowingContext::new();
        assert!(ctx.narrowed_types.is_empty());
        assert!(ctx.active_guards.is_empty());
        assert!(ctx.parent.is_none());
    }

    #[test]
    fn test_narrowing_context_set_get() {
        let mut ctx = NarrowingContext::new();
        let resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Строка".to_string(),
        }));

        ctx.set_type("x".to_string(), resolution.clone());
        assert!(ctx.get_type("x").is_some());
    }

    #[test]
    fn test_narrowing_context_child() {
        let mut parent = NarrowingContext::new();
        let resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Число".to_string(),
        }));

        parent.set_type("x".to_string(), resolution.clone());

        let child = parent.child();
        assert!(child.get_type("x").is_some()); // Должен найти в parent
    }

    #[test]
    fn test_narrowing_context_apply_guard() {
        let mut ctx = NarrowingContext::new();
        let current = TypeResolution::unknown();

        let guard = TypeGuard::TypeCheck {
            variable: "x".to_string(),
            expected_type: "Строка".to_string(),
        };

        ctx.apply_guard(guard.clone(), &current);

        assert_eq!(ctx.active_guards.len(), 1);
        assert!(ctx.get_type("x").is_some());
    }

    #[test]
    fn test_narrowing_engine_narrow_type() {
        let cfg = ControlFlowGraph::new();
        let mut engine = NarrowingEngine::new(cfg);

        let current = TypeResolution::unknown();
        let narrowed = engine.narrow_type(&current, "ТипЗнч(Параметр) = Тип(\"Число\")");

        // Должен сузить до Число
        if let crate::domain::types::ResolutionResult::Concrete(ConcreteType::Platform(pt)) =
            &narrowed.result
        {
            assert_eq!(pt.name, "Число");
        } else {
            panic!("Expected narrowed type to be Число");
        }
    }

    #[test]
    fn test_narrowing_engine_no_guards() {
        let cfg = ControlFlowGraph::new();
        let mut engine = NarrowingEngine::new(cfg);

        let current = TypeResolution::unknown();
        let narrowed = engine.narrow_type(&current, "x > 0"); // Нет type guards

        // Должен вернуть исходный тип
        assert_eq!(format!("{:?}", current), format!("{:?}", narrowed));
    }

    #[test]
    fn test_narrowing_engine_build_contexts() {
        let mut cfg = ControlFlowGraph::new();

        let entry_id = cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::Entry,
            context_in: None,
            context_out: None,
        });

        let cond_id = cfg.add_node(CfgNode {
            id: 1,
            kind: CfgNodeKind::Conditional {
                condition: "ТипЗнч(x) = Тип(\"Строка\")".to_string(),
            },
            context_in: None,
            context_out: None,
        });

        let then_id = cfg.add_node(CfgNode {
            id: 2,
            kind: CfgNodeKind::BasicBlock {
                statements: vec!["y = x.Length".to_string()],
            },
            context_in: None,
            context_out: None,
        });

        cfg.add_edge(entry_id, cond_id, EdgeKind::Unconditional);
        cfg.add_edge(cond_id, then_id, EdgeKind::ConditionalTrue);

        let mut engine = NarrowingEngine::new(cfg);

        let mut initial_ctx = FlowAnalysisContext::new();
        initial_ctx.set_variable(
            "x".to_string(),
            TypeResolution::unknown(), // Any
        );

        engine.build_narrowing_contexts(initial_ctx);

        // Проверяем, что в then ветке x имеет тип Строка
        if let Some(then_ctx) = engine.get_context(then_id) {
            if let Some(x_type) = then_ctx.get_type("x") {
                if let crate::domain::types::ResolutionResult::Concrete(ConcreteType::Platform(
                    pt,
                )) = &x_type.result
                {
                    assert_eq!(pt.name, "Строка");
                } else {
                    panic!("Expected x to be narrowed to Строка in then branch");
                }
            } else {
                panic!("Variable x should exist in then branch context");
            }
        } else {
            panic!("Then branch context should exist");
        }
    }

    #[test]
    fn test_narrowing_context_merge() {
        let mut ctx1 = NarrowingContext::new();
        let mut ctx2 = NarrowingContext::new();

        ctx1.set_type(
            "x".to_string(),
            TypeResolution::known(ConcreteType::Platform(PlatformType {
                name: "Строка".to_string(),
            })),
        );

        ctx2.set_type(
            "x".to_string(),
            TypeResolution::known(ConcreteType::Platform(PlatformType {
                name: "Число".to_string(),
            })),
        );

        let mut flow_ctx = FlowAnalysisContext::new();
        ctx1.merge(&ctx2, &mut flow_ctx);

        // После merge x должен иметь union type
        if let Some(merged_type) = flow_ctx.get_variable("x") {
            assert!(matches!(
                merged_type.result,
                crate::domain::types::ResolutionResult::Union(_)
            ));
        } else {
            panic!("Variable x should exist after merge");
        }
    }
}
