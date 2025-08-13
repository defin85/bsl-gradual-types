//! Тестовый парсер только для языковой справки

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!("=== Тестирование парсера языковой справки ===\n");
    
    let lang_path = "examples/syntax_helper/rebuilt.shlang_ru.zip";
    
    if !Path::new(lang_path).exists() {
        anyhow::bail!("Файл справки по языку не найден: {}", lang_path);
    }
    
    // Создаём парсер только с языковой справкой
    let mut parser = SyntaxHelperParser::new()
        .with_lang_archive(lang_path);
    
    // Запускаем парсинг
    println!("Запускаем парсинг...");
    parser.parse()?;
    
    // Получаем базу знаний
    let database = parser.database();
    
    // Выводим ключевые слова
    println!("\n📊 Найдено ключевых слов: {}", database.keywords.len());
    println!("\nПримеры ключевых слов по категориям:\n");
    
    // Группируем по категориям
    use std::collections::HashMap;
    let mut by_category: HashMap<String, Vec<String>> = HashMap::new();
    
    for keyword in &database.keywords {
        let category = format!("{:?}", keyword.category);
        let entry = format!("{} / {}", keyword.russian, keyword.english);
        by_category.entry(category).or_default().push(entry);
    }
    
    // Выводим по категориям
    for (category, keywords) in by_category {
        println!("{}:", category);
        for (i, keyword) in keywords.iter().enumerate() {
            if i >= 5 {
                println!("  ... и ещё {}", keywords.len() - 5);
                break;
            }
            println!("  - {}", keyword);
        }
        println!();
    }
    
    Ok(())
}