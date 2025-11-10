//! Debug test для анализа AST вызова метода

use tree_sitter::Parser;

#[test]
fn debug_method_call_tree_structure() {
    let code = r#"
Функция Тест()
    ТаблицаЗначенійТип = Новый ТаблицаЗначеній;
    Кол = ТаблицаЗначенійТип.Количество();
КонецФункції
"#;

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("Failed to set language");

    let tree = parser.parse(code, None).expect("Failed to parse");
    let root = tree.root_node();

    println!("\n=== TREE-SITTER AST FOR METHOD CALL ===");
    print_tree(&root, code, 0);
}

fn print_tree(node: &tree_sitter::Node, source: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let preview = if text.chars().count() > 50 {
        let truncated: String = text.chars().take(50).collect();
        format!("{}...", truncated.replace('\n', "\\n"))
    } else {
        text.replace('\n', "\\n")
    };

    println!(
        "{}[{}] {} [{},{}..{},{}] \"{}\"",
        indent,
        node.id(),
        node.kind(),
        node.start_position().row,
        node.start_position().column,
        node.end_position().row,
        node.end_position().column,
        preview
    );

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(&child, source, depth + 1);
    }
}
