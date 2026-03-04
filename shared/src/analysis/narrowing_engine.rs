//! Narrowing Engine
//!
//! Движок для сужения типов на основе control-flow анализа и type guards.
//!
//! Milestone 3.7: Advanced Type Narrowing

use crate::analysis::type_guards::{detect_type_guards, TypeGuard};
use crate::domain::flow_analysis::{ControlFlowGraph, FlowAnalysisContext};
use crate::domain::type_id::TypeId;
use crate::domain::types::TypeResolution;
use std::collections::HashMap;

/// Контекст сужения типов для конкретного блока CFG
#[derive(Debug, Clone)]
pub struct NarrowingContext {
    /// Сузуженные типы переменных в текущем блоке
    pub narrowed_types: HashMap<TypeId, TypeResolution>,

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
        let type_id = TypeId::new(variable);

        // Сначала ищем в текущем контексте
        if let Some(ty) = self.narrowed_types.get(&type_id) {
            return Some(ty);
        }

        // Затем в родительском
        if let Some(parent) = &self.parent {
            return parent.get_type(variable);
        }

        None
    }

    /// Установить сузуженный тип переменной
    pub fn set_type(&mut self, variable: &str, resolution: TypeResolution) {
        self.narrowed_types
            .insert(TypeId::new(variable), resolution);
    }

    /// Применить type guard к переменной
    pub fn apply_guard(&mut self, guard: TypeGuard, current_type: &TypeResolution) {
        let variable = guard.variable_name();
        let narrowed = guard.apply_narrowing(current_type);

        self.narrowed_types.insert(TypeId::new(variable), narrowed);
        self.active_guards.push(guard);
    }

    /// Получить все активные guards
    pub fn get_active_guards(&self) -> &[TypeGuard] {
        &self.active_guards
    }

    /// Объединить с другим контекстом (для merge после if-then-else)
    pub fn merge(&mut self, other: &NarrowingContext, flow_ctx: &mut FlowAnalysisContext) {
        // Для каждой переменной из обоих контекстов
        let mut all_vars: Vec<TypeId> = self
            .narrowed_types
            .keys()
            .chain(other.narrowed_types.keys())
            .cloned()
            .collect();
        all_vars.sort_by(|a, b| a.normalized().cmp(b.normalized()));
        all_vars.dedup();

        for var_id in all_vars {
            match (
                self.narrowed_types.get(&var_id),
                other.narrowed_types.get(&var_id),
            ) {
                (Some(self_type), Some(other_type)) => {
                    // Переменная есть в обоих контекстах — создаём union
                    let mut self_ctx = FlowAnalysisContext::new();
                    let mut other_ctx = FlowAnalysisContext::new();

                    self_ctx.set_variable(var_id.display(), self_type.clone());
                    other_ctx.set_variable(var_id.display(), other_type.clone());

                    self_ctx.merge(&other_ctx);

                    if let Some(merged) = self_ctx.get_variable(var_id.display()) {
                        self.narrowed_types.insert(var_id.clone(), merged.clone());
                        flow_ctx.set_variable(var_id.display(), merged.clone());
                    }
                }
                (Some(self_type), None) => {
                    // Только в self — оставляем как есть
                    flow_ctx.set_variable(var_id.display(), self_type.clone());
                }
                (None, Some(other_type)) => {
                    // Только в other — добавляем
                    self.narrowed_types
                        .insert(var_id.clone(), other_type.clone());
                    flow_ctx.set_variable(var_id.display(), other_type.clone());
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
    /// ```rust,no_run
    /// # use bsl_shared::analysis::NarrowingEngine;
    /// # use bsl_shared::domain::flow_analysis::ControlFlowGraph;
    /// # use bsl_shared::domain::types::TypeResolution;
    /// let cfg = ControlFlowGraph::new();
    /// let mut engine = NarrowingEngine::new(cfg);
    /// let current_type = TypeResolution::unknown();
    /// let narrowed = engine.narrow_type(
    ///     &current_type,
    ///     "ТипЗнч(Параметр) = Тип(\"Число\")"
    /// );
    /// // narrowed теперь имеет тип Число
    /// # let _ = narrowed;
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

        // Инициализируем контекст для всех entry-узлов (CFG может содержать несколько компонентов).
        let mut seeded = false;
        for node in self.cfg.nodes() {
            if !matches!(node.kind, CfgNodeKind::Entry) {
                continue;
            }
            let mut ctx = NarrowingContext::new();
            for (var_id, resolution) in initial_context.get_all_variables() {
                ctx.set_type(var_id.display(), resolution.clone());
            }
            self.contexts.insert(node.id, ctx);
            seeded = true;
        }
        if !seeded {
            // Fallback для CFG без явного Entry.
            if let Some(entry_node) = self.cfg.nodes().first() {
                let mut ctx = NarrowingContext::new();
                for (var_id, resolution) in initial_context.get_all_variables() {
                    ctx.set_type(var_id.display(), resolution.clone());
                }
                self.contexts.insert(entry_node.id, ctx);
            }
        }

        // Проходим по всем узлам в топологическом порядке
        for node in self.cfg.nodes() {
            let node_ctx = self.contexts.get(&node.id).cloned().unwrap_or_default();

            match &node.kind {
                CfgNodeKind::Conditional { condition } | CfgNodeKind::LoopHeader { condition } => {
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
#[path = "narrowing_engine/tests.rs"]
mod tests;
