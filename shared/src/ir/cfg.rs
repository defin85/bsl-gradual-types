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

/// Bias/предпочтение при выборе CFG-узла по byte offset.
///
/// Нужен для стабильного поведения на границах токенов (например, completion на `.`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeAtByteOffsetBias {
    /// Точный выбор по `offset`: берём самый специфичный (минимальный span), содержащий позицию.
    Exact,
    /// Предпочесть контекст слева от позиции (например, если курсор стоит сразу после `.`).
    PreferLeft,
    /// Предпочесть контекст справа от позиции (резерв для будущего).
    PreferRight,
}

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

    /// Найти CFG-узел по позиции (UTF-8 byte offset) с учётом bias.
    ///
    /// Алгоритм:
    /// 1) Находит все узлы, span которых содержит offset.
    /// 2) Выбирает “самый специфичный” узел (с минимальным span.len()).
    /// 3) При равенстве выбирает детерминированно по (span.start, node_id).
    ///
    /// Для `PreferLeft`/`PreferRight` пытается небольшое окно вокруг позиции, чтобы
    /// устойчиво обрабатывать границы токенов (например, completion на `.`).
    pub fn node_at_byte_offset(
        &self,
        byte_offset: u32,
        bias: NodeAtByteOffsetBias,
    ) -> Option<CfgNodeId> {
        let find_at = |offset: u32| {
            (0..self.nodes.len())
                .filter_map(|node_id| self.node_span(node_id).map(|span| (node_id, span)))
                .filter(|(_, span)| span.contains(offset))
                .min_by_key(|(node_id, span)| (span.len(), span.start, *node_id))
                .map(|(node_id, _)| node_id)
        };

        const WINDOW: u32 = 32;

        match bias {
            NodeAtByteOffsetBias::Exact => find_at(byte_offset),
            NodeAtByteOffsetBias::PreferLeft => {
                for delta in 0..=WINDOW {
                    if let Some(offset) = byte_offset.checked_sub(delta) {
                        if let Some(node_id) = find_at(offset) {
                            return Some(node_id);
                        }
                    }
                }
                None
            }
            NodeAtByteOffsetBias::PreferRight => {
                for delta in 0..=WINDOW {
                    if let Some(offset) = byte_offset.checked_add(delta) {
                        if let Some(node_id) = find_at(offset) {
                            return Some(node_id);
                        }
                    }
                }
                None
            }
        }
    }

    /// Получить индекс IR-ноды (`SemanticProgram.nodes`) для CFG-узла.
    pub fn node_ir_node_index(&self, node_id: usize) -> Option<usize> {
        self.node_ir_node_indices.get(node_id).copied().flatten()
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
    fn test_node_at_byte_offset_picks_most_specific_span() {
        let mut cfg = ControlFlowGraph::new();
        let entry = cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::Entry,
        });
        let wide = cfg.add_node(CfgNode {
            id: 1,
            kind: CfgNodeKind::BasicBlock {
                statements: vec!["wide".to_string()],
            },
        });
        let narrow = cfg.add_node(CfgNode {
            id: 2,
            kind: CfgNodeKind::BasicBlock {
                statements: vec!["narrow".to_string()],
            },
        });

        cfg.set_node_span(wide, Some(Span::new(0, 10)));
        cfg.set_node_span(narrow, Some(Span::new(2, 3)));
        cfg.add_edge(entry, wide, EdgeKind::Unconditional);

        let node = cfg
            .node_at_byte_offset(2, NodeAtByteOffsetBias::Exact)
            .expect("node");
        assert_eq!(node, narrow);
    }

    #[test]
    fn test_node_at_byte_offset_prefer_left_handles_end_boundary() {
        let mut cfg = ControlFlowGraph::new();
        let block = cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::BasicBlock {
                statements: vec!["x".to_string()],
            },
        });
        cfg.set_node_span(block, Some(Span::new(0, 10)));

        // Span.contains(10) == false (end exclusive), но PreferLeft должен найти узел по offset=9.
        let node = cfg
            .node_at_byte_offset(10, NodeAtByteOffsetBias::PreferLeft)
            .expect("node");
        assert_eq!(node, block);
    }
}
