//! Отладка Execute statement

use tree_sitter::{Parser, TreeCursor};

fn print_tree(cursor: &mut TreeCursor, source: &str, indent: usize) {
    let node = cursor.node();
    let kind = node.kind();
    let text = &source[node.byte_range()];

    let display_text = if text.len() > 80 {
        format!("{}...", &text[..80])
    } else {
        text.to_string()
    };

    println!(
        "{:indent$}{} | '{}' | range: {}..{}",
        "",
        kind,
        display_text.replace('\n', "\\n"),
        node.start_byte(),
        node.end_byte(),
        indent = indent * 2
    );

    if cursor.goto_first_child() {
        print_tree(cursor, source, indent + 1);
        cursor.goto_parent();
    }

    if cursor.goto_next_sibling() {
        print_tree(cursor, source, indent);
    }
}

fn main() {
    let code = "Выполнить(\"Сообщить('Динамический код')\");";

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("Failed to set language");

    println!("Code: {}", code);
    println!("{}", "-".repeat(60));

    let tree = parser.parse(code, None).expect("Failed to parse");
    let mut cursor = tree.walk();

    print_tree(&mut cursor, code, 0);
}
