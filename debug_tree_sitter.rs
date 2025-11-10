use tree_sitter::{Parser, Language};

extern "C" { fn tree_sitter_bsl() -> Language; }

fn main() {
    let code = r#"
Функция Тест()
    Кол = ТаблицаТип.Количество();
КонецФункции
"#;

    let mut parser = Parser::new();
    let language = unsafe { tree_sitter_bsl() };
    parser.set_language(&language).unwrap();
    
    let tree = parser.parse(code, None).unwrap();
    let root = tree.root_node();
    
    println!("=== TREE-SITTER AST ===");
    print_tree(&root, code, 0);
}

fn print_tree(node: &tree_sitter::Node, source: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let preview = if text.len() > 40 {
        format!("{}...", &text[..40])
    } else {
        text.to_string()
    };
    
    println!("{}{} [{}-{}] \"{}\"", 
        indent, 
        node.kind(),
        node.start_position().row,
        node.end_position().row,
        preview.replace('\n', "\\n")
    );
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(&child, source, depth + 1);
    }
}
