use super::*;

fn utf16_position(content: &str, marker: &str) -> (u32, u32) {
    let byte_index = content
        .find(marker)
        .unwrap_or_else(|| panic!("Marker not found: {}", marker));
    let before = &content[..byte_index];
    let line = before.lines().count().saturating_sub(1) as u32;
    let last_line = before.lines().last().unwrap_or("");
    let col = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    (line, col)
}

#[test]
fn member_access_receiver_chain_from_method_call() {
    let content = concat!(
        "Procedure Test()\n",
        "    Table.Columns.Add().\n",
        "EndProcedure\n",
    );
    let parse_result = bsl_syntax::parse_fast(content).expect("parse");
    let (line, dot_col) = utf16_position(content, "Add().");
    let column = dot_col + "Add().".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let chain =
        extract_member_access_receiver_chain(content, line, column, &parse_result).expect("chain");
    let name_chain = chain.to_name_chain().expect("name chain");
    assert_eq!(
        name_chain,
        vec![
            "Table".to_string(),
            "Columns".to_string(),
            "Add".to_string()
        ]
    );
}

#[test]
fn member_access_receiver_chain_supports_call_index_property_chain() {
    let content = concat!(
        "Procedure Test()\n",
        "    a.b().c[d].e.\n",
        "EndProcedure\n",
    );
    let parse_result = bsl_syntax::parse_fast(content).expect("parse");
    let (line, dot_col) = utf16_position(content, "e.");
    let column = dot_col + "e.".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;
    let chain =
        extract_member_access_receiver_chain(content, line, column, &parse_result).expect("chain");

    assert_eq!(chain.head, ReceiverChainHead::Identifier("a".to_string()));
    assert_eq!(
        chain.segments,
        vec![
            ReceiverChainSegment {
                kind: ReceiverChainSegmentKind::Call,
                name: Some("b".to_string()),
            },
            ReceiverChainSegment {
                kind: ReceiverChainSegmentKind::Property,
                name: Some("c".to_string()),
            },
            ReceiverChainSegment {
                kind: ReceiverChainSegmentKind::Index,
                name: None,
            },
            ReceiverChainSegment {
                kind: ReceiverChainSegmentKind::Property,
                name: Some("e".to_string()),
            },
        ]
    );
}

#[test]
fn completion_target_contract_for_member_access() {
    let content = concat!(
        "Procedure Test()\n",
        "    Table.Columns.Add().\n",
        "EndProcedure\n",
    );
    let parse_result = bsl_syntax::parse_fast(content).expect("parse");
    let (line, dot_col) = utf16_position(content, "Add().");
    let column = dot_col + "Add().".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let target = extract_completion_target_for_member_access(content, line, column, &parse_result)
        .expect("target");

    assert_eq!(target.kind, CompletionTargetKind::MemberAccess);
    assert!(target.receiver_expression.is_some());
    assert_eq!(
        target
            .receiver
            .expect("receiver")
            .to_name_chain()
            .expect("name chain"),
        vec![
            "Table".to_string(),
            "Columns".to_string(),
            "Add".to_string()
        ]
    );
}

#[test]
fn member_access_receiver_chain_supports_global_call_receiver() {
    let content = concat!("Procedure Test()\n", "    Make().\n", "EndProcedure\n",);
    let parse_result = bsl_syntax::parse_fast(content).expect("parse");
    let (line, dot_col) = utf16_position(content, "Make().");
    let column = dot_col + "Make().".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let chain =
        extract_member_access_receiver_chain(content, line, column, &parse_result).expect("chain");

    assert_eq!(chain.head, ReceiverChainHead::Call("Make".to_string()));
    assert!(chain.segments.is_empty());
    assert!(chain.to_name_chain().is_none());
}

#[test]
fn member_access_receiver_chain_supports_parenthesized_receiver() {
    let content = concat!("Procedure Test()\n", "    (a).\n", "EndProcedure\n",);
    let parse_result = bsl_syntax::parse_fast(content).expect("parse");
    let (line, dot_col) = utf16_position(content, "(a).");
    let column = dot_col + "(a).".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let chain =
        extract_member_access_receiver_chain(content, line, column, &parse_result).expect("chain");

    assert_eq!(chain.head, ReceiverChainHead::Identifier("a".to_string()));
    assert!(chain.segments.is_empty());
}

#[test]
fn member_access_receiver_chain_supports_new_expression_receiver() {
    let content = concat!("Procedure Test()\n", "    New Table().\n", "EndProcedure\n",);
    let parse_result = bsl_syntax::parse_fast(content).expect("parse");
    let (line, dot_col) = utf16_position(content, "Table().");
    let column = dot_col + "Table().".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32;

    let chain =
        extract_member_access_receiver_chain(content, line, column, &parse_result).expect("chain");

    assert_eq!(
        chain.head,
        ReceiverChainHead::ExplicitType("Table".to_string())
    );
    assert!(chain.segments.is_empty());
    assert_eq!(
        chain.to_name_chain().expect("name chain"),
        vec!["Table".to_string()]
    );
}
