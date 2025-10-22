//! Исследование node kinds из tree-sitter-bsl
//!
//! Этот пример парсит BSL код и выводит все типы узлов AST

use tree_sitter::Parser;

fn main() {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("Failed to load BSL grammar");

    // Тестовый BSL код с различными конструкциями
    let test_code = r#"
Перем ГлобальнаяПеременная;

Функция ТестоваяФункция(Параметр1, Параметр2)
    Перем Результат;
    Результат = Параметр1 + Параметр2;
    Возврат Результат;
КонецФункции

Процедура ТестоваяПроцедура()
    Перем Счетчик;

    Если Истина Тогда
        Сообщить("Истина");
    Иначе
        Сообщить("Ложь");
    КонецЕсли;

    Для Счетчик = 1 По 10 Цикл
        Сообщить(Счетчик);
    КонецЦикла;

    Пока Счетчик > 0 Цикл
        Счетчик = Счетчик - 1;
    КонецЦикла;

    Попытка
        ВызватьМетод();
    Исключение
        Сообщить(ОписаниеОшибки());
    КонецПопытки;
КонецПроцедуры
"#;

    println!("=== Parsing BSL code ===\n{}\n", test_code);

    let tree = parser.parse(test_code, None).expect("Parsing failed");
    let root = tree.root_node();

    println!("=== Tree-sitter AST node kinds ===\n");
    print_node_kinds(&root, test_code, 0);

    println!("\n=== Summary of unique node kinds ===");
    let mut kinds = collect_node_kinds(&root);
    kinds.sort();
    kinds.dedup();

    for kind in kinds {
        println!("  - {}", kind);
    }
}

fn print_node_kinds(node: &tree_sitter::Node, source: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let text = if node.child_count() == 0 {
        let node_text = &source[node.byte_range()];
        format!(
            " => \"{}\"",
            node_text
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(40)
                .collect::<String>()
        )
    } else {
        String::new()
    };

    println!("{}{}{}", indent, node.kind(), text);

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_node_kinds(&child, source, depth + 1);
    }
}

fn collect_node_kinds(node: &tree_sitter::Node) -> Vec<String> {
    let mut kinds = vec![node.kind().to_string()];

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        kinds.extend(collect_node_kinds(&child));
    }

    kinds
}
