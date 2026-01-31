//! Control Flow Graph (CFG) - граф потока управления
//!
//! Канонический CFG живёт в IR-слое и используется доменной логикой flow-sensitive анализа
//! (type narrowing / null-safety) поверх `SemanticProgram`.

use serde::{Deserialize, Serialize};

/// Идентификатор узла CFG
///
/// В каноническом CFG node id совпадает с индексом узла в `ControlFlowGraph.nodes`.
pub type CfgNodeId = usize;

/// Граф потока управления (для flow-sensitive анализа)
///
/// Упрощённая версия для отслеживания последовательности блоков кода.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfgNode {
    /// ID узла (для удобства отладки; должен совпадать с индексом в `ControlFlowGraph.nodes`)
    pub id: usize,

    /// Тип узла
    pub kind: CfgNodeKind,
}

/// Типы узлов CFG
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    // === Варианты для null safety и type narrowing ===
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfgEdge {
    /// От какого узла
    pub from: usize,

    /// К какому узлу
    pub to: usize,

    /// Тип перехода
    pub kind: EdgeKind,
}

/// Типы рёбер CFG
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    #[test]
    fn test_cfg_creation() {
        let mut cfg = ControlFlowGraph::new();

        let entry = cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::Entry,
        });

        let block = cfg.add_node(CfgNode {
            id: 1,
            kind: CfgNodeKind::BasicBlock {
                statements: vec!["x = 42".to_string()],
            },
        });

        cfg.add_edge(entry, block, EdgeKind::Unconditional);

        assert_eq!(cfg.nodes().len(), 2);
        assert_eq!(cfg.edges().len(), 1);
    }
}
