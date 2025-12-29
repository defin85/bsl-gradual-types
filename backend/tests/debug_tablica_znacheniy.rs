//! Debug test для анализа проблемы с ТаблицаЗначенійТип

use tree_sitter::Parser;

#[test]
#[ignore = "Debug test uses a large fixture and is not part of CI"]
fn debug_tablica_znacheniy_ast() {
    let fixture_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/bsl/test_hover_milestone_2_11.bsl");
    let code = std::fs::read_to_string(&fixture_path).expect("Failed to read test file");

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("Failed to set language");

    let tree = parser.parse(&code, None).expect("Failed to parse");
    let root = tree.root_node();

    println!("\n=== SEARCHING LINE 26 (Кол = ТаблицаЗначенийТип.Количество) ===");
    search_line(&root, &code, 25); // Line 26 в 0-индексации = 25
}

fn search_line(node: &tree_sitter::Node, source: &str, target_line: usize) {
    if node.start_position().row <= target_line && node.end_position().row >= target_line {
        let indent = "  ".repeat(0);
        println!(
            "{}[{}] {} [{},{}..{},{}]",
            indent,
            node.id(),
            node.kind(),
            node.start_position().row,
            node.start_position().column,
            node.end_position().row,
            node.end_position().column
        );

        if node.child_count() > 0 {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                search_line(&child, source, target_line);
            }
        } else {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            println!("    Leaf text: {:?}", text);
        }
    }
}

#[allow(dead_code)]
fn search_node(node: &tree_sitter::Node, source: &str, pattern: &str) {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");

    if text.contains(pattern) {
        let indent = "  ".repeat(0);
        println!(
            "{}[{}] {} [{},{}..{},{}]",
            indent,
            node.id(),
            node.kind(),
            node.start_position().row,
            node.start_position().column,
            node.end_position().row,
            node.end_position().column
        );
        println!("  Text: {:?}", text.chars().take(100).collect::<String>());

        // Показываем дочерние узлы
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            print_tree(&child, source, 1);
        }
    } else {
        // Рекурсивно ищем в дочерних узлах
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            search_node(&child, source, pattern);
        }
    }
}

#[allow(dead_code)]
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
