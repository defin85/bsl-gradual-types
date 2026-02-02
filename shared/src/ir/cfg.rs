//! Control Flow Graph (CFG) - граф потока управления
//!
//! Канонический CFG живёт в IR-слое и используется доменной логикой flow-sensitive анализа
//! (type narrowing / null-safety) поверх `SemanticProgram`.

use serde::{Deserialize, Serialize};

use super::span::Span;

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

    /// Span (byte offsets) в исходном коде для каждого CFG-узла.
    ///
    /// Индекс вектора совпадает с `CfgNodeId` / индексом в `nodes`.
    #[serde(default)]
    node_spans: Vec<Option<Span>>,

    /// Индекс IR-ноды (`SemanticProgram.nodes`) для каждого CFG-узла, если узел
    /// был построен из конкретного IR statement/expression.
    ///
    /// Индекс вектора совпадает с `CfgNodeId` / индексом в `nodes`.
    #[serde(default)]
    node_ir_node_indices: Vec<Option<usize>>,
}

/// Bias (смещение) для поиска CFG-узла по byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfgNodeAtByteOffsetBias {
    /// Ищет узел, span которого содержит `offset` (без эвристик).
    Exact,
    /// Предпочитает узел слева от позиции (например, completion на границе токена).
    PreferLeft,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_spans: Vec::new(),
            node_ir_node_indices: Vec::new(),
        }
    }

    /// Добавить узел в граф
    pub fn add_node(&mut self, node: CfgNode) -> usize {
        let id = self.nodes.len();
        self.nodes.push(node);
        self.node_spans.push(None);
        self.node_ir_node_indices.push(None);
        id
    }

    /// Добавить ребро между узлами
    pub fn add_edge(&mut self, from: usize, to: usize, kind: EdgeKind) {
        self.edges.push(CfgEdge { from, to, kind });
    }

    /// Установить span для узла (по id).
    pub fn set_node_span(&mut self, node_id: usize, span: Option<Span>) {
        if let Some(slot) = self.node_spans.get_mut(node_id) {
            *slot = span;
        }
    }

    /// Установить индекс IR-ноды (`SemanticProgram.nodes`) для CFG-узла.
    pub fn set_node_ir_node_index(&mut self, node_id: usize, ir_node_index: Option<usize>) {
        if let Some(slot) = self.node_ir_node_indices.get_mut(node_id) {
            *slot = ir_node_index;
        }
    }

    /// Получить span для CFG-узла.
    pub fn node_span(&self, node_id: usize) -> Option<Span> {
        self.node_spans.get(node_id).copied().flatten()
    }

    /// Получить индекс IR-ноды (`SemanticProgram.nodes`) для CFG-узла.
    pub fn node_ir_node_index(&self, node_id: usize) -> Option<usize> {
        self.node_ir_node_indices.get(node_id).copied().flatten()
    }

    /// Найти CFG-узел по byte offset в исходном тексте.
    ///
    /// Алгоритм детерминирован:
    /// - выбирает узел с самым узким span, содержащим позицию;
    /// - при равной длине span выбирает узел с меньшим `node_id`;
    /// - в режиме `PreferLeft` сначала пытается `offset - 1`, затем `offset`.
    pub fn node_at_byte_offset(
        &self,
        offset: u32,
        bias: CfgNodeAtByteOffsetBias,
    ) -> Option<CfgNodeId> {
        let find_at = |at: u32| {
            (0..self.nodes.len())
                .filter_map(|node_id| self.node_span(node_id).map(|span| (node_id, span)))
                .filter(|(_, span)| span.contains(at))
                .min_by_key(|(node_id, span)| (span.len(), *node_id))
                .map(|(node_id, _)| node_id)
        };

        match bias {
            CfgNodeAtByteOffsetBias::Exact => find_at(offset),
            CfgNodeAtByteOffsetBias::PreferLeft => offset
                .checked_sub(1)
                .and_then(find_at)
                .or_else(|| find_at(offset)),
        }
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

    #[test]
    fn test_node_at_byte_offset_most_specific_and_bias() {
        let mut cfg = ControlFlowGraph::new();

        let wide = cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::BasicBlock {
                statements: vec![],
            },
        });
        cfg.set_node_span(
            wide,
            Some(Span {
                start: 0,
                end: 10,
            }),
        );

        let narrow = cfg.add_node(CfgNode {
            id: 1,
            kind: CfgNodeKind::BasicBlock {
                statements: vec![],
            },
        });
        cfg.set_node_span(
            narrow,
            Some(Span {
                start: 0,
                end: 5,
            }),
        );

        assert_eq!(
            cfg.node_at_byte_offset(3, CfgNodeAtByteOffsetBias::Exact),
            Some(narrow)
        );
        assert_eq!(
            cfg.node_at_byte_offset(10, CfgNodeAtByteOffsetBias::Exact),
            None
        );
        assert_eq!(
            cfg.node_at_byte_offset(10, CfgNodeAtByteOffsetBias::PreferLeft),
            Some(wide)
        );
        assert_eq!(
            cfg.node_at_byte_offset(5, CfgNodeAtByteOffsetBias::PreferLeft),
            Some(narrow)
        );
    }
}
