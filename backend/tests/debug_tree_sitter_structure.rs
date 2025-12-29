//! Diagnostic test to inspect tree-sitter AST structure

use tree_sitter::Parser;

#[test]
#[ignore = "Debug test uses a large fixture and is not part of CI"]
fn debug_tree_sitter_ast_structure() {
    let fixture_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/bsl/test_hover_milestone_2_11.bsl");
    let code = std::fs::read_to_string(&fixture_path).expect("Failed to read test file");

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("Failed to set language");

    let tree = parser.parse(&code, None).expect("Failed to parse");
    let root = tree.root_node();

    println!("\n=== TREE-SITTER AST STRUCTURE ===");
    println!("Root kind: {}", root.kind());
    println!("Root children count: {}", root.child_count());

    println!("\n=== ROOT LEVEL CHILDREN ===");
    let mut cursor = root.walk();
    for (i, child) in root.children(&mut cursor).enumerate() {
        let text = &code[child.byte_range()];
        let preview = if text.len() > 50 {
            format!("{}...", &text[..50])
        } else {
            text.to_string()
        };

        println!(
            "\n[{}] {} (line {}-{})",
            i,
            child.kind(),
            child.start_position().row,
            child.end_position().row
        );
        println!("    Text: {:?}", preview);

        if child.kind() == "function_definition" || child.kind() == "procedure_definition" {
            println!("    Function/Procedure children:");
            let mut func_cursor = child.walk();
            for (j, func_child) in child.children(&mut func_cursor).enumerate() {
                if func_child.kind() == "var_definition" || func_child.kind() == "var_statement" {
                    let var_text = &code[func_child.byte_range()];
                    println!(
                        "      [{}] {} - {:?}",
                        j,
                        func_child.kind(),
                        var_text.lines().next().unwrap_or("")
                    );
                }
            }
        }
    }
}
