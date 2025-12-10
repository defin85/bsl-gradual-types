use bsl_backend::data::adapters::converters::convert_syntax_helper_to_raw;
use bsl_backend::data::loaders::syntax_helper::SyntaxHelperLoader;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use std::sync::Arc;

fn main() {
    println!("🚀 Тест: Примитивные типы в Repository\n");

    // 1. Парсим синтаксис-помощник
    let mut parser = SyntaxHelperLoader::new();
    parser
        .parse_syntax_helper("../examples/syntax_helper")
        .unwrap();
    let db = parser.export_database();
    println!("✅ Парсинг завершён: {} узлов", db.nodes.len());

    // 2. Конвертируем в RawTypeData
    let raw_types = convert_syntax_helper_to_raw(&db);
    println!("✅ Конвертировано в RawTypeData: {} типов", raw_types.len());

    // 3. Загружаем в Repository
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository.load_types(raw_types).unwrap();
    let stats = repository.get_stats();
    println!("✅ Загружено в Repository: {} типов\n", stats.total_types);

    // 4. Проверяем наличие примитивных типов
    println!("🔍 Поиск примитивных типов в Repository:\n");

    let primitive_names = [
        "Строка",
        "String",
        "Число",
        "Number",
        "Булево",
        "Boolean",
        "Дата",
        "Date",
        "Null",
    ];

    for name in &primitive_names {
        match repository.find_type(name) {
            Some(raw_type) => {
                println!("✅ Найден: {}", name);
                println!(
                    "   Полное имя: {} ({})",
                    raw_type.name, raw_type.english_name
                );
                println!(
                    "   Методы: {}, Свойства: {}",
                    raw_type.methods.len(),
                    raw_type.properties.len()
                );
                println!();
            }
            None => {
                println!("❌ НЕ найден: {}\n", name);
            }
        }
    }

    // 5. Вывод первых 20 типов для проверки
    println!("\n📋 Первые 20 типов в Repository:");
    let all_types = repository.get_all_types();
    for (i, raw_type) in all_types.iter().take(20).enumerate() {
        println!("{}. {} ({})", i + 1, raw_type.name, raw_type.english_name);
    }
}
