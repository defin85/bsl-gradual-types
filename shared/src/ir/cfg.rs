//! Control Flow Graph (CFG) - граф потока управления
//!
//! Используется для flow-sensitive анализа типов.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Идентификатор узла CFG
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CfgNodeId(pub usize);

/// Граф потока управления (для flow-sensitive анализа)
///
/// CFG представляет все возможные пути выполнения программы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFlowGraph {
    pub nodes: HashMap<CfgNodeId, CfgNode>,
    pub edges: HashMap<CfgNodeId, Vec<CfgNodeId>>,
    pub entry: CfgNodeId,
    pub exit: CfgNodeId,
}

/// Узел графа потока управления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CfgNode {
    /// Обычный statement
    Statement { semantic_node_id: usize },

    /// Точка ветвления (if, while condition)
    Branch {
        condition_node_id: usize,
        true_branch: CfgNodeId,
        false_branch: CfgNodeId,
    },

    /// Слияние путей выполнения
    Merge,

    /// Точка выхода из функции
    Exit,
}
