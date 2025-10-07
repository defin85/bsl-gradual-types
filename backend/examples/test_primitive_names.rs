use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;
use bsl_backend::data::loaders::syntax_helper::SyntaxNode;

fn main() {
    let mut parser = SyntaxHelperParser::new();

    println!("🚀 Парсинг Синтаксис-помощника...");
    parser.parse_syntax_helper("../examples/syntax_helper").unwrap();

    let db = parser.export_database();

    println!("\n🔍 Информация о примитивных типах:\n");

    // Список интересующих нас файлов
    let primitive_keys = [
        "examples/syntax_helper/rebuilt.shlang_ru/def_String",
        "examples/syntax_helper/rebuilt.shlang_ru/def_Number",
        "examples/syntax_helper/rebuilt.shlang_ru/def_Boolean",
        "examples/syntax_helper/rebuilt.shlang_ru/def_Null",
        "examples/syntax_helper/rebuilt.shlang_ru/def_Date",
    ];

    for key in &primitive_keys {
        if let Some(node) = db.nodes.get(*key) {
            if let SyntaxNode::Type(type_info) = node {
                println!("📄 Файл: {}", key);
                println!("   Русское имя: {}", type_info.identity.russian_name);
                println!("   Английское имя: {}", type_info.identity.english_name);
                println!("   Категория: {}", type_info.identity.category_path);
                println!("   Описание: {}",
                    type_info.documentation.type_description
                        .chars().take(100).collect::<String>()
                );
                println!("   Методы: {}", type_info.structure.methods.len());
                println!("   Свойства: {}", type_info.structure.properties.len());
                println!();
            }
        } else {
            println!("❌ Не найден: {}\n", key);
        }
    }
}
