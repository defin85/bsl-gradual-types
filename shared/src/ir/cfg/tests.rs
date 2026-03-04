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
