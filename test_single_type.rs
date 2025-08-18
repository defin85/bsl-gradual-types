use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;
use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxNode;

fn main() -> anyhow::Result<()> {
    println!("🔍 Проверка загрузки методов и свойств типа ТаблицаЗначений");
    
    let mut parser = SyntaxHelperParser::new();
    
    if std::path::Path::new("examples/syntax_helper/rebuilt.shcntx_ru").exists() {
        parser.parse_directory("examples/syntax_helper/rebuilt.shcntx_ru")?;
    } else {
        println!("❌ Директория справки не найдена");
        return Ok(());
    }
    
    let database = parser.export_database();
    
    println!("📊 Всего узлов в базе: {}", database.nodes.len());
    
    // Ищем ТаблицаЗначений
    for (path, node) in &database.nodes {
        if let SyntaxNode::Type(type_info) = node {
            if type_info.identity.russian_name == "ТаблицаЗначений" {
                println!("\n✅ Найден тип ТаблицаЗначений:");
                println!("   📍 Путь: {}", path);
                println!("   🇷🇺 Русское название: {}", type_info.identity.russian_name);
                println!("   🇬🇧 Английское название: {}", type_info.identity.english_name);
                println!("   📂 Категория: {}", type_info.identity.category_path);
                println!("   📝 Описание: {}", type_info.documentation.type_description);
                println!("   🔧 Методов: {}", type_info.structure.methods.len());
                println!("   📋 Свойств: {}", type_info.structure.properties.len());
                
                if !type_info.structure.methods.is_empty() {
                    println!("   🔧 Методы (первые 5):");
                    for method in type_info.structure.methods.iter().take(5) {
                        println!("      - {}", method);
                    }
                }
                
                if !type_info.structure.properties.is_empty() {
                    println!("   📋 Свойства:");
                    for property in &type_info.structure.properties {
                        println!("      - {}", property);
                    }
                }
                
                return Ok(());
            }
        }
    }
    
    println!("❌ ТаблицаЗначений не найдена");
    
    // Покажем несколько типов для отладки
    println!("\n🔍 Первые 5 типов в базе:");
    let mut count = 0;
    for (path, node) in &database.nodes {
        if let SyntaxNode::Type(type_info) = node {
            println!("   {}. {} / {} (методов: {}, свойств: {})", 
                count + 1,
                type_info.identity.russian_name,
                type_info.identity.english_name,
                type_info.structure.methods.len(),
                type_info.structure.properties.len()
            );
            count += 1;
            if count >= 5 { break; }
        }
    }
    
    Ok(())
}