use bsl_backend::data::loaders::syntax_helper::SyntaxHelperLoader;

fn main() {
    let mut parser = SyntaxHelperLoader::new();

    // Парсим оба каталога
    println!("🚀 Парсинг Синтаксис-помощника...");
    parser
        .parse_syntax_helper("../examples/syntax_helper")
        .unwrap();

    let db = parser.export_database();

    println!("\n📊 Статистика:");
    println!("Всего узлов: {}", db.nodes.len());

    // Ищем примитивные типы
    let primitive_keywords = [
        "String",
        "Строка",
        "Number",
        "Число",
        "Boolean",
        "Булево",
        "Date",
        "Дата",
        "Null",
        "Неопределено",
        "def_String",
        "def_Number",
        "def_Boolean",
    ];

    println!("\n🔍 Поиск примитивных типов:");
    for keyword in &primitive_keywords {
        let found: Vec<_> = db
            .nodes
            .iter()
            .filter(|(key, _)| key.to_lowercase().contains(&keyword.to_lowercase()))
            .collect();

        if !found.is_empty() {
            println!("\n✅ Найдено для '{}':", keyword);
            for (key, node) in found.iter().take(3) {
                println!("  - Ключ: {}, Тип: {:?}", key, node_type_name(node));
            }
        }
    }

    // Вывод всех ключей содержащих "def_"
    println!("\n📋 Все узлы с 'def_':");
    let def_nodes: Vec<_> = db
        .nodes
        .iter()
        .filter(|(key, _)| key.starts_with("def_"))
        .collect();

    for (key, node) in def_nodes.iter().take(20) {
        println!("  - {}: {:?}", key, node_type_name(node));
    }
}

fn node_type_name(node: &bsl_backend::data::loaders::syntax_helper::SyntaxNode) -> &'static str {
    use bsl_backend::data::loaders::syntax_helper::SyntaxNode;
    match node {
        SyntaxNode::Category(_) => "Category",
        SyntaxNode::Type(_) => "Type",
        SyntaxNode::Method(_) => "Method",
        SyntaxNode::Property(_) => "Property",
        SyntaxNode::Constructor(_) => "Constructor",
        SyntaxNode::GlobalFunction(_) => "GlobalFunction",
    }
}
