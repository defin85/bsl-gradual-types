//! Отладочный пример для изучения AST tree-sitter-bsl
//! Показывает полную структуру дерева для BSL кода

use tree_sitter::{Parser, TreeCursor};

fn print_tree(cursor: &mut TreeCursor, source: &str, indent: usize) {
    let node = cursor.node();
    let kind = node.kind();
    let text = &source[node.byte_range()];

    // Ограничиваем длину текста для читаемости
    let display_text = if text.len() > 50 {
        format!("{}...", &text[..50])
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
    let test_cases = vec![
        ("Число", "Число = 42;"),
        ("Строка", "Текст = \"Привет\";"),
        ("Булево", "Флаг = Истина;"),
        ("Бинарное выражение", "Результат = 10 + 20;"),
        ("Новый объект", "Массив = Новый Массив;"),
        ("Вызов функции", "Результат = Функция(1, 2);"),
        ("Property access", "Значение = Объект.Свойство;"),
        ("Процедура", "Процедура Тест()\nКонецПроцедуры"),
        (
            "Return с значением",
            "Функция Тест()\n    Возврат 42;\nКонецФункции",
        ),
        ("Execute", "Выполнить(\"Код\");"),
        ("RaiseError", "ВызватьИсключение \"Ошибка\";"),
    ];

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("Failed to set language");

    for (name, code) in test_cases {
        println!("\n{}", "=".repeat(60));
        println!("Test case: {}", name);
        println!("Code: {}", code);
        println!("{}", "-".repeat(60));

        let tree = parser.parse(code, None).expect("Failed to parse");
        let mut cursor = tree.walk();

        print_tree(&mut cursor, code, 0);
    }
}
