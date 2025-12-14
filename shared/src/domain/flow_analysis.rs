//! Domain Layer: Flow-Sensitive Analysis
//!
//! Отслеживание изменений типов переменных в процессе выполнения кода
//! с учётом потока управления (control flow)

use crate::domain::type_id::TypeId;
use crate::domain::types::TypeResolution;
use std::collections::HashMap;

/// Контекст для flow-sensitive анализа
///
/// Отслеживает типы переменных на разных этапах выполнения кода
#[derive(Debug, Clone)]
pub struct FlowAnalysisContext {
    /// Текущее состояние типов переменных (ключ: TypeId для регистронезависимого поиска)
    variables: HashMap<TypeId, TypeResolution>,

    /// История изменений типов (для отладки и анализа)
    history: Vec<FlowEvent>,

    /// Глубина вложенности блоков (для if/while/for)
    scope_depth: usize,
}

impl FlowAnalysisContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            history: Vec::new(),
            scope_depth: 0,
        }
    }

    /// Установить тип переменной в текущем контексте
    pub fn set_variable(&mut self, name: impl Into<String>, resolution: TypeResolution) {
        let name_str = name.into();
        let type_id = TypeId::new(&name_str);
        self.history.push(FlowEvent::Assignment {
            variable: name_str, // История остаётся String
            resolution: resolution.clone(),
            scope_depth: self.scope_depth,
        });
        self.variables.insert(type_id, resolution);
    }

    /// Получить текущий тип переменной
    pub fn get_variable(&self, name: &str) -> Option<&TypeResolution> {
        let type_id = TypeId::new(name);
        self.variables.get(&type_id)
    }

    /// Войти в новый блок кода (if/while/for)
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
        self.history.push(FlowEvent::EnterScope {
            depth: self.scope_depth,
        });
    }

    /// Выйти из блока кода
    pub fn exit_scope(&mut self) {
        self.history.push(FlowEvent::ExitScope {
            depth: self.scope_depth,
        });
        if self.scope_depth > 0 {
            self.scope_depth -= 1;
        }
    }

    /// Создать ветку контекста для анализа условного блока (if-then-else)
    pub fn fork(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            history: self.history.clone(),
            scope_depth: self.scope_depth,
        }
    }

    /// Объединить два контекста после условного ветвления
    ///
    /// Создаёт union types для переменных, которые имеют разные типы в ветках
    pub fn merge(&mut self, other: &FlowAnalysisContext) {
        use crate::domain::types::{
            Certainty, ConcreteType, PlatformType, ResolutionResult, WeightedType,
        };

        for (var_id, other_resolution) in &other.variables {
            match self.variables.get(var_id) {
                Some(self_resolution) => {
                    // Переменная изменялась в обеих ветках — создаём union type
                    if self_resolution.result != other_resolution.result {
                        // Извлекаем ConcreteType из ResolutionResult
                        let self_concrete = match &self_resolution.result {
                            ResolutionResult::Concrete(t) => t.clone(),
                            ResolutionResult::Union(types) => {
                                // Берём первый тип из union
                                types.first().map(|wt| wt.type_.clone()).unwrap_or_else(|| {
                                    ConcreteType::Platform(PlatformType {
                                        name: "Произвольный".to_string(),
                                    })
                                })
                            }
                            ResolutionResult::Intersection(types) => {
                                // Берём первый тип из intersection
                                types.first().cloned().unwrap_or_else(|| {
                                    ConcreteType::Platform(PlatformType {
                                        name: "Произвольный".to_string(),
                                    })
                                })
                            }
                            ResolutionResult::Generic(gen) => {
                                // Используем базовый тип без параметров
                                ConcreteType::Platform(PlatformType {
                                    name: gen.base_type.clone(),
                                })
                            }
                            ResolutionResult::Nullable(t) => t.as_ref().clone(),
                            ResolutionResult::Dynamic => ConcreteType::Platform(PlatformType {
                                name: "Произвольный".to_string(),
                            }),
                        };

                        let other_concrete = match &other_resolution.result {
                            ResolutionResult::Concrete(t) => t.clone(),
                            ResolutionResult::Union(types) => {
                                types.first().map(|wt| wt.type_.clone()).unwrap_or_else(|| {
                                    ConcreteType::Platform(PlatformType {
                                        name: "Произвольный".to_string(),
                                    })
                                })
                            }
                            ResolutionResult::Intersection(types) => {
                                types.first().cloned().unwrap_or_else(|| {
                                    ConcreteType::Platform(PlatformType {
                                        name: "Произвольный".to_string(),
                                    })
                                })
                            }
                            ResolutionResult::Generic(gen) => {
                                ConcreteType::Platform(PlatformType {
                                    name: gen.base_type.clone(),
                                })
                            }
                            ResolutionResult::Nullable(t) => t.as_ref().clone(),
                            ResolutionResult::Dynamic => ConcreteType::Platform(PlatformType {
                                name: "Произвольный".to_string(),
                            }),
                        };

                        // Создаём union type
                        let union = vec![
                            WeightedType {
                                type_: self_concrete,
                                weight: 0.5,
                            },
                            WeightedType {
                                type_: other_concrete,
                                weight: 0.5,
                            },
                        ];

                        let merged_resolution = TypeResolution {
                            certainty: Certainty::InferredWeak,
                            result: ResolutionResult::Union(union),
                            source: crate::domain::types::ResolutionSource::Inferred,
                            metadata: crate::domain::types::ResolutionMetadata {
                                file: None,
                                line: None,
                                column: None,
                                notes: vec![format!(
                                    "Union type from conditional branches for variable: {}",
                                    var_id.display()
                                )],
                                uncertainty_reason: None,
                            },
                            active_facet: None,
                            available_facets: vec![],
                        };

                        self.variables.insert(var_id.clone(), merged_resolution);
                    }
                }
                None => {
                    // Переменная появилась только в другой ветке
                    self.variables
                        .insert(var_id.clone(), other_resolution.clone());
                }
            }
        }

        self.history.push(FlowEvent::MergeContexts {
            merged_variables: other
                .variables
                .keys()
                .map(|id| id.display().to_string())
                .collect(),
        });
    }

    /// Получить все текущие переменные
    pub fn get_all_variables(&self) -> &HashMap<TypeId, TypeResolution> {
        &self.variables
    }

    /// Получить историю событий (для отладки)
    pub fn get_history(&self) -> &Vec<FlowEvent> {
        &self.history
    }

    /// Получить текущую глубину вложенности
    pub fn get_scope_depth(&self) -> usize {
        self.scope_depth
    }
}

impl Default for FlowAnalysisContext {
    fn default() -> Self {
        Self::new()
    }
}

/// События в процессе flow-анализа
#[derive(Debug, Clone)]
pub enum FlowEvent {
    /// Присваивание переменной
    Assignment {
        variable: String,
        resolution: TypeResolution,
        scope_depth: usize,
    },

    /// Вход в новый блок кода
    EnterScope { depth: usize },

    /// Выход из блока кода
    ExitScope { depth: usize },

    /// Объединение контекстов после ветвления
    MergeContexts { merged_variables: Vec<String> },

    /// Вызов метода (может изменить тип через reassignment)
    MethodCall {
        variable: String,
        method: String,
        result_type: Option<TypeResolution>,
    },
}

/// Базовый граф потока управления (CFG)
///
/// Упрощённая версия для отслеживания последовательности блоков кода
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// Узлы графа (базовые блоки)
    nodes: Vec<CfgNode>,

    /// Рёбра графа (переходы между блоками)
    edges: Vec<CfgEdge>,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Добавить узел в граф
    pub fn add_node(&mut self, node: CfgNode) -> usize {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    /// Добавить ребро между узлами
    pub fn add_edge(&mut self, from: usize, to: usize, kind: EdgeKind) {
        self.edges.push(CfgEdge { from, to, kind });
    }

    /// Получить все узлы
    pub fn nodes(&self) -> &[CfgNode] {
        &self.nodes
    }

    /// Получить все рёбра
    pub fn edges(&self) -> &[CfgEdge] {
        &self.edges
    }
}

impl Default for ControlFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Узел графа потока управления (базовый блок)
#[derive(Debug, Clone)]
pub struct CfgNode {
    /// ID узла
    pub id: usize,

    /// Тип узла
    pub kind: CfgNodeKind,

    /// Контекст типов на входе в узел
    pub context_in: Option<FlowAnalysisContext>,

    /// Контекст типов на выходе из узла
    pub context_out: Option<FlowAnalysisContext>,
}

/// Типы узлов CFG
#[derive(Debug, Clone)]
pub enum CfgNodeKind {
    /// Начало программы/функции
    Entry,

    /// Конец программы/функции
    Exit,

    /// Последовательность операторов
    BasicBlock { statements: Vec<String> },

    /// Условное ветвление (if-then-else)
    Conditional { condition: String },

    /// Начало цикла
    LoopHeader { condition: String },

    /// Тело цикла
    LoopBody,

    // === Новые варианты для null safety и type inference (Milestone 2.3) ===
    /// Присваивание переменной
    Assignment { variable: String, value: String },

    /// Вызов метода объекта
    MethodCall {
        object: String,
        method: String,
        arguments: Vec<String>,
    },

    /// Доступ к свойству объекта
    PropertyAccess { object: String, property: String },

    /// Проверка условия (для null safety)
    Condition { variable: String },
}

/// Ребро графа потока управления
#[derive(Debug, Clone)]
pub struct CfgEdge {
    /// От какого узла
    pub from: usize,

    /// К какому узлу
    pub to: usize,

    /// Тип перехода
    pub kind: EdgeKind,
}

/// Типы рёбер CFG
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeKind {
    /// Безусловный переход
    Unconditional,

    /// Условный переход (true ветка)
    ConditionalTrue,

    /// Условный переход (false ветка)
    ConditionalFalse,

    /// Переход в начало цикла
    LoopBack,

    /// Выход из цикла
    LoopExit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{ConcreteType, PlatformType, TypeResolution};

    #[test]
    fn test_flow_context_set_get() {
        let mut ctx = FlowAnalysisContext::new();
        let resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Строка".to_string(),
        }));

        ctx.set_variable("x".to_string(), resolution.clone());
        assert!(ctx.get_variable("x").is_some());
    }

    #[test]
    fn test_flow_context_scope() {
        let mut ctx = FlowAnalysisContext::new();
        assert_eq!(ctx.scope_depth, 0);

        ctx.enter_scope();
        assert_eq!(ctx.scope_depth, 1);

        ctx.exit_scope();
        assert_eq!(ctx.scope_depth, 0);
    }

    #[test]
    fn test_flow_context_fork() {
        let mut ctx = FlowAnalysisContext::new();
        let resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Строка".to_string(),
        }));

        ctx.set_variable("x".to_string(), resolution.clone());

        let forked = ctx.fork();
        assert!(forked.get_variable("x").is_some());
    }

    #[test]
    fn test_cfg_creation() {
        let mut cfg = ControlFlowGraph::new();

        let entry = cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::Entry,
            context_in: None,
            context_out: None,
        });

        let block = cfg.add_node(CfgNode {
            id: 1,
            kind: CfgNodeKind::BasicBlock {
                statements: vec!["x = 42".to_string()],
            },
            context_in: None,
            context_out: None,
        });

        cfg.add_edge(entry, block, EdgeKind::Unconditional);

        assert_eq!(cfg.nodes().len(), 2);
        assert_eq!(cfg.edges().len(), 1);
    }
}
